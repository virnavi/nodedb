use std::sync::Arc;

use futures_util::{SinkExt, StreamExt};
use tokio::sync::Mutex;
use tokio_tungstenite::tungstenite::Message;

use crate::error::TransportError;
use crate::types::WireMessage;

type WsSink = futures_util::stream::SplitSink<
    tokio_tungstenite::WebSocketStream<tokio_rustls::server::TlsStream<tokio::net::TcpStream>>,
    Message,
>;
type WsStream = futures_util::stream::SplitStream<
    tokio_tungstenite::WebSocketStream<tokio_rustls::server::TlsStream<tokio::net::TcpStream>>,
>;

type WsClientSink = futures_util::stream::SplitSink<
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
    Message,
>;
type WsClientStream = futures_util::stream::SplitStream<
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
>;

/// Unified sender that can write to either server or client WebSocket streams.
pub enum PeerSender {
    Server(Mutex<WsSink>),
    Client(Mutex<WsClientSink>),
}

/// Unified receiver that can read from either server or client WebSocket streams.
pub enum PeerReceiver {
    Server(WsStream),
    Client(WsClientStream),
}

/// A connection to a single peer, wrapping a split WebSocket stream.
pub struct PeerConnection {
    pub peer_id: String,
    pub endpoint: String,
    pub sender: Arc<PeerSender>,
    pub shared_key: Option<[u8; 32]>,
}

impl PeerConnection {
    /// Create from a server-accepted WebSocket (split already done).
    pub fn new_server(
        peer_id: String,
        endpoint: String,
        sink: WsSink,
        shared_key: Option<[u8; 32]>,
    ) -> Self {
        PeerConnection {
            peer_id,
            endpoint,
            sender: Arc::new(PeerSender::Server(Mutex::new(sink))),
            shared_key,
        }
    }

    /// Create from a client WebSocket connection.
    pub fn new_client(
        peer_id: String,
        endpoint: String,
        sink: WsClientSink,
        shared_key: Option<[u8; 32]>,
    ) -> Self {
        PeerConnection {
            peer_id,
            endpoint,
            sender: Arc::new(PeerSender::Client(Mutex::new(sink))),
            shared_key,
        }
    }

    /// Send a WireMessage over the WebSocket.
    pub async fn send_message(&self, msg: &WireMessage) -> Result<(), TransportError> {
        let payload = encode_wire_message(msg)?;
        let ws_msg = Message::Binary(payload);

        match self.sender.as_ref() {
            PeerSender::Server(sink) => {
                sink.lock()
                    .await
                    .send(ws_msg)
                    .await
                    .map_err(|e| TransportError::Send(e.to_string()))
            }
            PeerSender::Client(sink) => {
                sink.lock()
                    .await
                    .send(ws_msg)
                    .await
                    .map_err(|e| TransportError::Send(e.to_string()))
            }
        }
    }
}

/// Read the next WireMessage from a receiver stream.
pub async fn recv_message(receiver: &mut PeerReceiver) -> Result<Option<WireMessage>, TransportError> {
    let msg = match receiver {
        PeerReceiver::Server(stream) => stream.next().await,
        PeerReceiver::Client(stream) => stream.next().await,
    };

    match msg {
        Some(Ok(Message::Binary(data))) => {
            let wire_msg = decode_wire_message(&data)?;
            Ok(Some(wire_msg))
        }
        Some(Ok(Message::Close(_))) | None => Ok(None),
        Some(Ok(_)) => Ok(None), // ignore text, ping, pong at this level
        Some(Err(e)) => Err(TransportError::Receive(e.to_string())),
    }
}

/// Encode a WireMessage to binary: [version byte] [msgpack body]
pub fn encode_wire_message(msg: &WireMessage) -> Result<Vec<u8>, TransportError> {
    let mut buf = Vec::with_capacity(256);
    buf.push(msg.version);
    let body = rmp_serde::to_vec(msg)
        .map_err(|e| TransportError::Serialization(e.to_string()))?;
    buf.extend_from_slice(&body);
    Ok(buf)
}

/// Decode a binary frame into a WireMessage.
pub fn decode_wire_message(data: &[u8]) -> Result<WireMessage, TransportError> {
    if data.is_empty() {
        return Err(TransportError::Receive("empty message".to_string()));
    }
    // Skip version byte, decode the rest as msgpack
    let msg: WireMessage = rmp_serde::from_slice(&data[1..])
        .map_err(|e| TransportError::Serialization(e.to_string()))?;
    Ok(msg)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::WireMessageType;

    #[test]
    fn encode_decode_roundtrip() {
        let msg = WireMessage {
            version: 1,
            msg_id: "test-1".to_string(),
            msg_type: WireMessageType::Ping,
            sender_id: "peer1".to_string(),
            payload: vec![1, 2, 3],
        };
        let encoded = encode_wire_message(&msg).unwrap();
        assert_eq!(encoded[0], 1); // version byte
        let decoded = decode_wire_message(&encoded).unwrap();
        assert_eq!(decoded.msg_id, "test-1");
        assert_eq!(decoded.msg_type, WireMessageType::Ping);
        assert_eq!(decoded.payload, vec![1, 2, 3]);
    }

    #[test]
    fn decode_empty_fails() {
        assert!(decode_wire_message(&[]).is_err());
    }
}
