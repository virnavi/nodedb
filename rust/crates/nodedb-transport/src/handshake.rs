use futures_util::{SinkExt, StreamExt};
use nodedb_crypto::{NodeIdentity, PublicIdentity};
use rand::RngCore;
use tokio_tungstenite::tungstenite::Message;

use crate::credential::CredentialStore;
use crate::error::TransportError;
use crate::pairing::PairingStore;
use crate::types::{PeerAcceptance, WireMessage, WireMessageType};

/// Hello payload sent by the initiator.
#[derive(serde::Serialize, serde::Deserialize)]
pub struct HelloPayload {
    pub peer_id: String,
    pub public_key_bytes: Vec<u8>,
    pub endpoint: String,
    /// Random 32-byte nonce for key-ownership proof. Empty for legacy peers.
    #[serde(default)]
    pub nonce: Vec<u8>,
    /// Ed25519 signature of the nonce (64 bytes). Empty for legacy peers.
    #[serde(default)]
    pub signature: Vec<u8>,
    /// User ID of the connecting user (UUID). Empty for legacy peers.
    #[serde(default)]
    pub user_id: String,
    /// Human-readable device name. Empty for legacy peers.
    #[serde(default)]
    pub device_name: String,
}

/// HelloAck payload sent by the acceptor.
#[derive(serde::Serialize, serde::Deserialize)]
pub struct HelloAckPayload {
    pub peer_id: String,
    pub public_key_bytes: Vec<u8>,
    pub accepted: bool,
    /// Random 32-byte nonce for key-ownership proof. Empty for legacy peers.
    #[serde(default)]
    pub nonce: Vec<u8>,
    /// Ed25519 signature of the nonce (64 bytes). Empty for legacy peers.
    #[serde(default)]
    pub signature: Vec<u8>,
    /// User ID of the accepting user (UUID). Empty for legacy peers.
    #[serde(default)]
    pub user_id: String,
    /// Human-readable device name. Empty for legacy peers.
    #[serde(default)]
    pub device_name: String,
    /// If true, this is a new pairing request that needs user confirmation.
    #[serde(default)]
    pub pairing_required: bool,
}

/// Result of a successful handshake from the initiator side.
#[derive(Debug)]
pub struct HandshakeResult {
    pub peer_public: PublicIdentity,
    pub shared_key: [u8; 32],
    pub peer_user_id: String,
    pub peer_device_name: String,
}

/// Result of a successful handshake from the acceptor side.
#[derive(Debug)]
pub struct AcceptorHandshakeResult {
    pub peer_public: PublicIdentity,
    pub peer_endpoint: String,
    pub shared_key: [u8; 32],
    pub peer_user_id: String,
    pub peer_device_name: String,
}

/// Generate a cryptographically random 32-byte nonce.
fn generate_nonce() -> [u8; 32] {
    let mut nonce = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut nonce);
    nonce
}

/// Verify a peer's Ed25519 signature over a nonce.
///
/// Returns the constructed `PublicIdentity` on success.
fn verify_peer_signature(
    public_key_bytes: &[u8],
    nonce: &[u8],
    signature: &[u8],
    peer_id: &str,
) -> Result<(), TransportError> {
    let peer_public = PublicIdentity {
        peer_id: peer_id.to_string(),
        public_key_bytes: public_key_bytes.to_vec(),
    };
    peer_public
        .verify(nonce, signature)
        .map_err(|e| TransportError::Handshake(format!("peer signature verification failed: {}", e)))
}

/// Perform the initiator side of the handshake.
/// Sends Hello, waits for HelloAck, derives shared secret.
pub async fn handshake_initiator<S>(
    stream: &mut S,
    identity: &NodeIdentity,
    my_endpoint: &str,
    user_id: &str,
    device_name: &str,
) -> Result<HandshakeResult, TransportError>
where
    S: SinkExt<Message, Error = tokio_tungstenite::tungstenite::Error>
        + StreamExt<Item = Result<Message, tokio_tungstenite::tungstenite::Error>>
        + Unpin,
{
    // Generate nonce and sign it to prove key ownership
    let my_nonce = generate_nonce();
    let my_signature = identity.sign(&my_nonce);

    // Send Hello
    let hello_payload = HelloPayload {
        peer_id: identity.peer_id().to_string(),
        public_key_bytes: identity.to_public().public_key_bytes.clone(),
        endpoint: my_endpoint.to_string(),
        nonce: my_nonce.to_vec(),
        signature: my_signature,
        user_id: user_id.to_string(),
        device_name: device_name.to_string(),
    };
    let payload_bytes = rmp_serde::to_vec(&hello_payload)
        .map_err(|e| TransportError::Serialization(e.to_string()))?;

    let hello = WireMessage {
        version: 1,
        msg_id: uuid::Uuid::new_v4().to_string(),
        msg_type: WireMessageType::Hello,
        sender_id: identity.peer_id().to_string(),
        payload: payload_bytes,
    };

    let encoded = crate::connection::encode_wire_message(&hello)?;
    stream
        .send(Message::Binary(encoded))
        .await
        .map_err(|e| TransportError::Handshake(format!("send Hello: {}", e)))?;

    // Wait for HelloAck
    let ack_msg = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        read_next_binary(stream),
    )
    .await
    .map_err(|_| TransportError::Timeout("HelloAck timeout".into()))?
    .map_err(|e| TransportError::Handshake(format!("recv HelloAck: {}", e)))?;

    let wire_msg = crate::connection::decode_wire_message(&ack_msg)?;
    if wire_msg.msg_type != WireMessageType::HelloAck {
        return Err(TransportError::Handshake(format!(
            "expected HelloAck, got {:?}",
            wire_msg.msg_type
        )));
    }

    let ack: HelloAckPayload = rmp_serde::from_slice(&wire_msg.payload)
        .map_err(|e| TransportError::Serialization(e.to_string()))?;

    if ack.pairing_required {
        return Err(TransportError::PairingRequired(ack.peer_id));
    }

    if !ack.accepted {
        return Err(TransportError::PeerRejected(ack.peer_id));
    }

    // Verify acceptor's signature if present (non-legacy peer)
    if !ack.nonce.is_empty() && !ack.signature.is_empty() {
        verify_peer_signature(&ack.public_key_bytes, &ack.nonce, &ack.signature, &ack.peer_id)?;
    }

    let peer_public = PublicIdentity {
        peer_id: ack.peer_id,
        public_key_bytes: ack.public_key_bytes,
    };

    // Derive shared secret
    let shared_key = derive_shared_key(identity, &peer_public)?;

    Ok(HandshakeResult {
        peer_public,
        shared_key,
        peer_user_id: ack.user_id,
        peer_device_name: ack.device_name,
    })
}

/// Perform the acceptor side of the handshake.
/// Waits for Hello, checks acceptance (including pairing), sends HelloAck, derives shared secret.
pub async fn handshake_acceptor<S>(
    stream: &mut S,
    identity: &NodeIdentity,
    _my_endpoint: &str,
    credential_store: &CredentialStore,
    user_id: &str,
    device_name: &str,
    pairing_store: Option<&PairingStore>,
) -> Result<AcceptorHandshakeResult, TransportError>
where
    S: SinkExt<Message, Error = tokio_tungstenite::tungstenite::Error>
        + StreamExt<Item = Result<Message, tokio_tungstenite::tungstenite::Error>>
        + Unpin,
{
    // Wait for Hello
    let hello_data = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        read_next_binary(stream),
    )
    .await
    .map_err(|_| TransportError::Timeout("Hello timeout".into()))?
    .map_err(|e| TransportError::Handshake(format!("recv Hello: {}", e)))?;

    let wire_msg = crate::connection::decode_wire_message(&hello_data)?;
    if wire_msg.msg_type != WireMessageType::Hello {
        return Err(TransportError::Handshake(format!(
            "expected Hello, got {:?}",
            wire_msg.msg_type
        )));
    }

    let hello: HelloPayload = rmp_serde::from_slice(&wire_msg.payload)
        .map_err(|e| TransportError::Serialization(e.to_string()))?;

    // Verify initiator's signature if present (non-legacy peer)
    if !hello.nonce.is_empty() && !hello.signature.is_empty() {
        verify_peer_signature(
            &hello.public_key_bytes,
            &hello.nonce,
            &hello.signature,
            &hello.peer_id,
        )?;
    }

    let peer_public = PublicIdentity {
        peer_id: hello.peer_id.clone(),
        public_key_bytes: hello.public_key_bytes.clone(),
    };

    // Determine acceptance: pairing store takes priority, then credential store
    let (accepted, pairing_required) = if let Some(ps) = pairing_store {
        if ps.is_paired(&hello.peer_id) {
            // Paired peer — verify reconnect credentials match
            ps.verify_reconnect(&hello.peer_id, &hello.public_key_bytes, &hello.user_id)?;
            ps.touch_verified(&hello.peer_id)?;
            (true, false)
        } else {
            // Unknown peer — add to pending, require pairing
            ps.add_pending(crate::pairing::PendingPairingRequest {
                peer_id: hello.peer_id.clone(),
                public_key_bytes: hello.public_key_bytes.clone(),
                user_id: hello.user_id.clone(),
                device_name: hello.device_name.clone(),
                endpoint: hello.endpoint.clone(),
                received_at: chrono::Utc::now(),
            });
            (false, true)
        }
    } else {
        // No pairing store — fall back to credential store
        let acc = credential_store.should_accept(&peer_public) == PeerAcceptance::Accept;
        (acc, false)
    };

    // Generate nonce and sign it to prove our key ownership
    let my_nonce = generate_nonce();
    let my_signature = identity.sign(&my_nonce);

    // Send HelloAck
    let ack_payload = HelloAckPayload {
        peer_id: identity.peer_id().to_string(),
        public_key_bytes: identity.to_public().public_key_bytes.clone(),
        accepted,
        nonce: my_nonce.to_vec(),
        signature: my_signature,
        user_id: user_id.to_string(),
        device_name: device_name.to_string(),
        pairing_required,
    };
    let payload_bytes = rmp_serde::to_vec(&ack_payload)
        .map_err(|e| TransportError::Serialization(e.to_string()))?;

    let ack = WireMessage {
        version: 1,
        msg_id: uuid::Uuid::new_v4().to_string(),
        msg_type: WireMessageType::HelloAck,
        sender_id: identity.peer_id().to_string(),
        payload: payload_bytes,
    };

    let encoded = crate::connection::encode_wire_message(&ack)?;
    stream
        .send(Message::Binary(encoded))
        .await
        .map_err(|e| TransportError::Handshake(format!("send HelloAck: {}", e)))?;

    if pairing_required {
        return Err(TransportError::PairingRequired(hello.peer_id));
    }

    if !accepted {
        return Err(TransportError::PeerRejected(hello.peer_id));
    }

    // Derive shared secret
    let shared_key = derive_shared_key(identity, &peer_public)?;

    Ok(AcceptorHandshakeResult {
        peer_public,
        peer_endpoint: hello.endpoint,
        shared_key,
        peer_user_id: hello.user_id,
        peer_device_name: hello.device_name,
    })
}

/// Derive X25519 shared secret from Ed25519 identities.
fn derive_shared_key(
    my_identity: &NodeIdentity,
    peer_public: &PublicIdentity,
) -> Result<[u8; 32], TransportError> {
    let my_x25519 = my_identity.to_x25519_secret();
    let their_x25519 = peer_public
        .to_x25519_public()
        .map_err(|e| TransportError::Crypto(e))?;
    Ok(nodedb_crypto::encryption::derive_shared_secret(
        &my_x25519,
        &their_x25519,
    ))
}

/// Read the next binary message from the stream, skipping non-binary frames.
async fn read_next_binary<S>(stream: &mut S) -> Result<Vec<u8>, TransportError>
where
    S: StreamExt<Item = Result<Message, tokio_tungstenite::tungstenite::Error>> + Unpin,
{
    loop {
        match stream.next().await {
            Some(Ok(Message::Binary(data))) => return Ok(data),
            Some(Ok(Message::Close(_))) | None => {
                return Err(TransportError::Receive("connection closed during handshake".into()))
            }
            Some(Ok(_)) => continue, // skip ping/pong/text
            Some(Err(e)) => return Err(TransportError::Receive(e.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hello_payload_serde_roundtrip() {
        let payload = HelloPayload {
            peer_id: "peer1".to_string(),
            public_key_bytes: vec![1, 2, 3],
            endpoint: "wss://localhost:9400".to_string(),
            nonce: vec![],
            signature: vec![],
            user_id: "user-abc".to_string(),
            device_name: "My Phone".to_string(),
        };
        let bytes = rmp_serde::to_vec(&payload).unwrap();
        let decoded: HelloPayload = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.peer_id, "peer1");
        assert_eq!(decoded.public_key_bytes, vec![1, 2, 3]);
        assert_eq!(decoded.user_id, "user-abc");
        assert_eq!(decoded.device_name, "My Phone");
    }

    #[test]
    fn hello_ack_payload_serde_roundtrip() {
        let payload = HelloAckPayload {
            peer_id: "peer2".to_string(),
            public_key_bytes: vec![4, 5, 6],
            accepted: true,
            nonce: vec![],
            signature: vec![],
            user_id: "user-xyz".to_string(),
            device_name: "Server".to_string(),
            pairing_required: false,
        };
        let bytes = rmp_serde::to_vec(&payload).unwrap();
        let decoded: HelloAckPayload = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.peer_id, "peer2");
        assert!(decoded.accepted);
        assert_eq!(decoded.user_id, "user-xyz");
        assert!(!decoded.pairing_required);
    }

    #[test]
    fn hello_payload_backward_compat_no_nonce() {
        #[derive(serde::Serialize)]
        struct OldHelloPayload {
            peer_id: String,
            public_key_bytes: Vec<u8>,
            endpoint: String,
        }
        let old = OldHelloPayload {
            peer_id: "peer1".to_string(),
            public_key_bytes: vec![1, 2, 3],
            endpoint: "wss://localhost:9400".to_string(),
        };
        let bytes = rmp_serde::to_vec(&old).unwrap();
        let decoded: HelloPayload = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.peer_id, "peer1");
        assert!(decoded.nonce.is_empty());
        assert!(decoded.signature.is_empty());
    }

    #[test]
    fn hello_ack_payload_backward_compat_no_nonce() {
        #[derive(serde::Serialize)]
        struct OldHelloAckPayload {
            peer_id: String,
            public_key_bytes: Vec<u8>,
            accepted: bool,
        }
        let old = OldHelloAckPayload {
            peer_id: "peer2".to_string(),
            public_key_bytes: vec![4, 5, 6],
            accepted: true,
        };
        let bytes = rmp_serde::to_vec(&old).unwrap();
        let decoded: HelloAckPayload = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.peer_id, "peer2");
        assert!(decoded.accepted);
        assert!(decoded.nonce.is_empty());
        assert!(decoded.signature.is_empty());
    }

    #[test]
    fn hello_payload_signed_roundtrip() {
        let id = NodeIdentity::generate();
        let nonce = generate_nonce();
        let signature = id.sign(&nonce);
        let payload = HelloPayload {
            peer_id: id.peer_id().to_string(),
            public_key_bytes: id.to_public().public_key_bytes.clone(),
            endpoint: "wss://localhost:9400".to_string(),
            nonce: nonce.to_vec(),
            signature: signature.clone(),
            user_id: "uid-test".to_string(),
            device_name: "dev".to_string(),
        };
        let bytes = rmp_serde::to_vec(&payload).unwrap();
        let decoded: HelloPayload = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.nonce.len(), 32);
        assert_eq!(decoded.signature.len(), 64);
        assert_eq!(decoded.user_id, "uid-test");

        // Verify the signature is valid
        let peer = id.to_public();
        peer.verify(&decoded.nonce, &decoded.signature).unwrap();
    }

    #[test]
    fn hello_ack_payload_signed_roundtrip() {
        let id = NodeIdentity::generate();
        let nonce = generate_nonce();
        let signature = id.sign(&nonce);
        let payload = HelloAckPayload {
            peer_id: id.peer_id().to_string(),
            public_key_bytes: id.to_public().public_key_bytes.clone(),
            accepted: true,
            nonce: nonce.to_vec(),
            signature: signature.clone(),
            user_id: "uid-ack".to_string(),
            device_name: "srv".to_string(),
            pairing_required: false,
        };
        let bytes = rmp_serde::to_vec(&payload).unwrap();
        let decoded: HelloAckPayload = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.nonce.len(), 32);
        assert_eq!(decoded.signature.len(), 64);
        assert_eq!(decoded.user_id, "uid-ack");
        assert!(!decoded.pairing_required);

        let peer = id.to_public();
        peer.verify(&decoded.nonce, &decoded.signature).unwrap();
    }

    #[test]
    fn verify_peer_signature_valid() {
        let id = NodeIdentity::generate();
        let nonce = generate_nonce();
        let sig = id.sign(&nonce);
        let pub_id = id.to_public();
        verify_peer_signature(
            &pub_id.public_key_bytes,
            &nonce,
            &sig,
            &pub_id.peer_id,
        )
        .unwrap();
    }

    #[test]
    fn verify_peer_signature_wrong_key_fails() {
        let id1 = NodeIdentity::generate();
        let id2 = NodeIdentity::generate();
        let nonce = generate_nonce();
        let sig = id1.sign(&nonce); // signed by id1
        let pub2 = id2.to_public();
        let result = verify_peer_signature(
            &pub2.public_key_bytes,
            &nonce,
            &sig,
            &pub2.peer_id,
        );
        assert!(result.is_err());
    }

    #[test]
    fn verify_peer_signature_tampered_nonce_fails() {
        let id = NodeIdentity::generate();
        let nonce = generate_nonce();
        let sig = id.sign(&nonce);
        let mut tampered_nonce = nonce;
        tampered_nonce[0] ^= 0xff;
        let pub_id = id.to_public();
        let result = verify_peer_signature(
            &pub_id.public_key_bytes,
            &tampered_nonce,
            &sig,
            &pub_id.peer_id,
        );
        assert!(result.is_err());
    }

    #[test]
    fn derive_shared_key_works() {
        let id_a = NodeIdentity::generate();
        let id_b = NodeIdentity::generate();
        let key_ab = derive_shared_key(&id_a, &id_b.to_public()).unwrap();
        let key_ba = derive_shared_key(&id_b, &id_a.to_public()).unwrap();
        assert_eq!(key_ab, key_ba);
    }

    #[test]
    fn hello_payload_backward_compat_no_user_id() {
        // Old payload without user_id/device_name fields
        #[derive(serde::Serialize)]
        struct OldHello {
            peer_id: String,
            public_key_bytes: Vec<u8>,
            endpoint: String,
            nonce: Vec<u8>,
            signature: Vec<u8>,
        }
        let old = OldHello {
            peer_id: "peer-old".to_string(),
            public_key_bytes: vec![1, 2],
            endpoint: "wss://old:9400".to_string(),
            nonce: vec![],
            signature: vec![],
        };
        let bytes = rmp_serde::to_vec(&old).unwrap();
        let decoded: HelloPayload = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.peer_id, "peer-old");
        assert!(decoded.user_id.is_empty());
        assert!(decoded.device_name.is_empty());
    }

    #[test]
    fn hello_ack_payload_backward_compat_no_pairing_required() {
        // Old ack without user_id/device_name/pairing_required
        #[derive(serde::Serialize)]
        struct OldAck {
            peer_id: String,
            public_key_bytes: Vec<u8>,
            accepted: bool,
            nonce: Vec<u8>,
            signature: Vec<u8>,
        }
        let old = OldAck {
            peer_id: "peer-old-ack".to_string(),
            public_key_bytes: vec![3, 4],
            accepted: true,
            nonce: vec![],
            signature: vec![],
        };
        let bytes = rmp_serde::to_vec(&old).unwrap();
        let decoded: HelloAckPayload = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.peer_id, "peer-old-ack");
        assert!(decoded.accepted);
        assert!(decoded.user_id.is_empty());
        assert!(decoded.device_name.is_empty());
        assert!(!decoded.pairing_required);
    }

    #[test]
    fn hello_ack_pairing_required_roundtrip() {
        let payload = HelloAckPayload {
            peer_id: "pr-peer".to_string(),
            public_key_bytes: vec![5, 6],
            accepted: false,
            nonce: vec![],
            signature: vec![],
            user_id: "uid".to_string(),
            device_name: "dev".to_string(),
            pairing_required: true,
        };
        let bytes = rmp_serde::to_vec(&payload).unwrap();
        let decoded: HelloAckPayload = rmp_serde::from_slice(&bytes).unwrap();
        assert!(!decoded.accepted);
        assert!(decoded.pairing_required);
    }
}
