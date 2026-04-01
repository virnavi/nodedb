use std::sync::Arc;

use futures_util::StreamExt;
use nodedb_crypto::NodeIdentity;
use nodedb_transport::connection::{
    encode_wire_message, decode_wire_message, PeerConnection, PeerReceiver,
};
use nodedb_transport::connection_pool::ConnectionPool;
use nodedb_transport::credential::CredentialStore;
use nodedb_transport::engine::TransportEngine;
use nodedb_transport::error::TransportError;
use nodedb_transport::query_handler::QueryHandler;
use nodedb_transport::tls;
use nodedb_transport::types::{
    FederatedQueryPolicy, FederatedQueryResponse, GossipConfig, PeerAcceptance,
    TransportConfig, WireMessage, WireMessageType,
};
use tokio::net::TcpListener;

/// Helper: start a TLS WebSocket server that performs the acceptor handshake
/// and returns the peer connection details.
async fn start_test_server(
    listener: TcpListener,
    identity: Arc<NodeIdentity>,
    credential_store: Arc<CredentialStore>,
) -> (PeerConnection, PeerReceiver) {
    let cert_key = tls::generate_self_signed_cert().unwrap();
    let server_config = tls::build_server_tls_config(&cert_key).unwrap();
    let tls_acceptor = tls::build_tls_acceptor(server_config);

    let (tcp_stream, _addr) = listener.accept().await.unwrap();
    let tls_stream = tls_acceptor.accept(tcp_stream).await.unwrap();
    let mut ws_stream = tokio_tungstenite::accept_async(tls_stream).await.unwrap();

    let result =
        nodedb_transport::handshake::handshake_acceptor(
            &mut ws_stream,
            &identity,
            "wss://127.0.0.1:0",
            &credential_store,
            "",
            "",
            None,
        )
        .await
        .unwrap();

    let (sink, stream) = ws_stream.split();
    let conn = PeerConnection::new_server(
        result.peer_public.peer_id.clone(),
        result.peer_endpoint,
        sink,
        Some(result.shared_key),
    );
    (conn, PeerReceiver::Server(stream))
}

/// Helper: connect as a client, perform initiator handshake.
async fn connect_test_client(
    addr: &str,
    identity: Arc<NodeIdentity>,
) -> (PeerConnection, PeerReceiver) {
    let tls_config = tls::build_client_tls_config().unwrap();
    let connector = tokio_tungstenite::Connector::Rustls(tls_config);

    let url = format!("wss://{}", addr);
    let (mut ws_stream, _) =
        tokio_tungstenite::connect_async_tls_with_config(&url, None, false, Some(connector))
            .await
            .unwrap();

    let result =
        nodedb_transport::handshake::handshake_initiator(
            &mut ws_stream,
            &identity,
            "wss://127.0.0.1:0",
            "",
            "",
        )
        .await
        .unwrap();

    let (sink, stream) = ws_stream.split();
    let conn = PeerConnection::new_client(
        result.peer_public.peer_id.clone(),
        format!("wss://{}", addr),
        sink,
        Some(result.shared_key),
    );
    (conn, PeerReceiver::Client(stream))
}

#[tokio::test]
async fn two_peers_handshake_and_exchange_messages() {
    let id_server = Arc::new(NodeIdentity::generate());
    let id_client = Arc::new(NodeIdentity::generate());
    let cred_store = Arc::new(CredentialStore::accept_all());

    // Bind to a random port
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap().to_string();

    // Run server and client concurrently
    let server_id = id_server.clone();
    let server_cred = cred_store.clone();
    let server_handle = tokio::spawn(async move {
        start_test_server(listener, server_id, server_cred).await
    });

    let client_id = id_client.clone();
    let addr_clone = addr.clone();
    let client_handle = tokio::spawn(async move {
        connect_test_client(&addr_clone, client_id).await
    });

    let (server_conn, mut server_receiver) = server_handle.await.unwrap();
    let (client_conn, mut client_receiver) = client_handle.await.unwrap();

    // Verify peer IDs match
    assert_eq!(server_conn.peer_id, id_client.peer_id());
    assert_eq!(client_conn.peer_id, id_server.peer_id());

    // Verify shared keys are equal
    assert_eq!(server_conn.shared_key, client_conn.shared_key);
    assert!(server_conn.shared_key.is_some());

    // Client sends a message to server
    let msg = WireMessage {
        version: 1,
        msg_id: "msg-1".to_string(),
        msg_type: WireMessageType::Ping,
        sender_id: id_client.peer_id().to_string(),
        payload: b"hello from client".to_vec(),
    };
    client_conn.send_message(&msg).await.unwrap();

    // Server receives it
    let received = nodedb_transport::connection::recv_message(&mut server_receiver)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(received.msg_id, "msg-1");
    assert_eq!(received.msg_type, WireMessageType::Ping);
    assert_eq!(received.payload, b"hello from client");

    // Server sends a response
    let reply = WireMessage {
        version: 1,
        msg_id: "msg-2".to_string(),
        msg_type: WireMessageType::Pong,
        sender_id: id_server.peer_id().to_string(),
        payload: b"hello from server".to_vec(),
    };
    server_conn.send_message(&reply).await.unwrap();

    // Client receives it
    let received = nodedb_transport::connection::recv_message(&mut client_receiver)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(received.msg_id, "msg-2");
    assert_eq!(received.msg_type, WireMessageType::Pong);
    assert_eq!(received.payload, b"hello from server");
}

#[tokio::test]
async fn peer_rejection_via_credential_store() {
    let id_server = Arc::new(NodeIdentity::generate());
    let id_client = Arc::new(NodeIdentity::generate());

    // Credential store that rejects all peers
    let cred_store = Arc::new(CredentialStore::new(|_| PeerAcceptance::Reject));

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap().to_string();

    let cert_key = tls::generate_self_signed_cert().unwrap();
    let server_config = tls::build_server_tls_config(&cert_key).unwrap();
    let tls_acceptor = tls::build_tls_acceptor(server_config);

    // Server side
    let server_handle = tokio::spawn(async move {
        let (tcp_stream, _) = listener.accept().await.unwrap();
        let tls_stream = tls_acceptor.accept(tcp_stream).await.unwrap();
        let mut ws_stream = tokio_tungstenite::accept_async(tls_stream).await.unwrap();

        let result = nodedb_transport::handshake::handshake_acceptor(
            &mut ws_stream,
            &id_server,
            "wss://127.0.0.1:0",
            &cred_store,
            "",
            "",
            None,
        )
        .await;

        // Server should report PeerRejected
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, nodedb_transport::TransportError::PeerRejected(_)),
            "expected PeerRejected, got: {:?}",
            err
        );
    });

    // Client side
    let client_handle = tokio::spawn(async move {
        let tls_config = tls::build_client_tls_config().unwrap();
        let connector = tokio_tungstenite::Connector::Rustls(tls_config);
        let url = format!("wss://{}", addr);
        let (mut ws_stream, _) =
            tokio_tungstenite::connect_async_tls_with_config(&url, None, false, Some(connector))
                .await
                .unwrap();

        let result = nodedb_transport::handshake::handshake_initiator(
            &mut ws_stream,
            &id_client,
            "wss://127.0.0.1:0",
            "",
            "",
        )
        .await;

        // Client should see PeerRejected
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, nodedb_transport::TransportError::PeerRejected(_)),
            "expected PeerRejected, got: {:?}",
            err
        );
    });

    server_handle.await.unwrap();
    client_handle.await.unwrap();
}

#[tokio::test]
async fn connection_pool_manages_peers() {
    let id_server = Arc::new(NodeIdentity::generate());
    let id_client = Arc::new(NodeIdentity::generate());
    let cred_store = Arc::new(CredentialStore::accept_all());
    let pool = Arc::new(ConnectionPool::new());

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap().to_string();

    let sid = id_server.clone();
    let sc = cred_store.clone();
    let sp = pool.clone();
    let server_handle = tokio::spawn(async move {
        let (conn, receiver) = start_test_server(listener, sid, sc).await;
        let peer_id = conn.peer_id.clone();
        sp.add(conn, receiver);
        peer_id
    });

    let cid = id_client.clone();
    let cp = pool.clone();
    let addr2 = addr.clone();
    let client_handle = tokio::spawn(async move {
        let (conn, receiver) = connect_test_client(&addr2, cid).await;
        let peer_id = conn.peer_id.clone();
        cp.add(conn, receiver);
        peer_id
    });

    let server_peer_id = server_handle.await.unwrap();
    let client_peer_id = client_handle.await.unwrap();

    // Pool should have 2 connections
    assert_eq!(pool.peer_count(), 2);
    assert!(pool.get(&server_peer_id).is_some());
    assert!(pool.get(&client_peer_id).is_some());

    let ids = pool.connected_peer_ids();
    assert_eq!(ids.len(), 2);

    // Send a message from pool to client-side peer
    let msg = WireMessage {
        version: 1,
        msg_id: "pool-msg".to_string(),
        msg_type: WireMessageType::Ping,
        sender_id: "test".to_string(),
        payload: b"via pool".to_vec(),
    };
    pool.send(&server_peer_id, &msg).await.unwrap();

    // The message should arrive in the pool's incoming channel (received by the read loop)
    let (_from_peer, received) = pool.recv().await.unwrap();
    assert_eq!(received.msg_id, "pool-msg");

    // Remove peer
    pool.remove(&server_peer_id);
    assert_eq!(pool.peer_count(), 1);
    assert!(pool.get(&server_peer_id).is_none());
}

#[tokio::test]
async fn encrypted_message_roundtrip() {
    let id_a = Arc::new(NodeIdentity::generate());
    let id_b = Arc::new(NodeIdentity::generate());
    let cred_store = Arc::new(CredentialStore::accept_all());

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap().to_string();

    let ia = id_a.clone();
    let sc = cred_store.clone();
    let server_handle = tokio::spawn(async move {
        start_test_server(listener, ia, sc).await
    });

    let ib = id_b.clone();
    let addr2 = addr.clone();
    let client_handle = tokio::spawn(async move {
        connect_test_client(&addr2, ib).await
    });

    let (server_conn, mut server_receiver) = server_handle.await.unwrap();
    let (client_conn, _client_receiver) = client_handle.await.unwrap();

    let shared_key = server_conn.shared_key.unwrap();

    // Encrypt a payload with the shared key
    let plaintext = b"secret federated query data";
    let (nonce, ciphertext) =
        nodedb_crypto::encryption::aes_256_gcm_encrypt(&shared_key, plaintext).unwrap();

    // Send encrypted payload as a WireMessage
    let msg = WireMessage {
        version: 1,
        msg_id: "enc-1".to_string(),
        msg_type: WireMessageType::QueryRequest,
        sender_id: id_b.peer_id().to_string(),
        payload: [nonce.as_slice(), ciphertext.as_slice()].concat(),
    };
    client_conn.send_message(&msg).await.unwrap();

    // Server receives and decrypts
    let received = nodedb_transport::connection::recv_message(&mut server_receiver)
        .await
        .unwrap()
        .unwrap();

    let recv_nonce = &received.payload[..12];
    let recv_ciphertext = &received.payload[12..];
    let decrypted =
        nodedb_crypto::encryption::aes_256_gcm_decrypt(&shared_key, recv_nonce, recv_ciphertext)
            .unwrap();

    assert_eq!(decrypted, plaintext);
}

#[test]
fn encode_decode_wire_message_roundtrip() {
    let msg = WireMessage {
        version: 1,
        msg_id: "test-1".to_string(),
        msg_type: WireMessageType::GossipPeerList,
        sender_id: "peer-a".to_string(),
        payload: vec![10, 20, 30],
    };
    let encoded = encode_wire_message(&msg).unwrap();
    assert_eq!(encoded[0], 1); // version byte
    let decoded = decode_wire_message(&encoded).unwrap();
    assert_eq!(decoded.msg_id, "test-1");
    assert_eq!(decoded.msg_type, WireMessageType::GossipPeerList);
    assert_eq!(decoded.payload, vec![10, 20, 30]);
}

#[tokio::test]
async fn transport_engine_lifecycle() {
    // Find free ports
    let listener_a = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr_a = listener_a.local_addr().unwrap().to_string();
    drop(listener_a);

    let listener_b = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr_b = listener_b.local_addr().unwrap().to_string();
    drop(listener_b);

    let id_a = NodeIdentity::generate();
    let id_b = NodeIdentity::generate();

    // Start engine A (no mDNS in tests, use seed peers)
    let config_a = TransportConfig {
        storage_path: String::new(),
        listen_addr: addr_a.clone(),
        tls_cert_pem: None,
        tls_key_pem: None,
        seed_peers: vec![],
        mdns_enabled: false,
        gossip: GossipConfig {
            interval_seconds: 60, // long interval, we test manually
            fan_out: 3,
            ttl: 5,
        },
        query_policy: FederatedQueryPolicy::QueryPeersOnMiss,
        mesh: None,
        trusted_peer_keys: vec![],
        require_pairing: false,
        user_id: String::new(),
        device_name: String::new(),
    };

    let engine_a = TransportEngine::start(config_a, id_a, None, None, None)
        .await
        .unwrap();

    // Start engine B
    let config_b = TransportConfig {
        storage_path: String::new(),
        listen_addr: addr_b.clone(),
        tls_cert_pem: None,
        tls_key_pem: None,
        seed_peers: vec![format!("wss://{}", addr_a)],
        mdns_enabled: false,
        gossip: GossipConfig {
            interval_seconds: 60,
            fan_out: 3,
            ttl: 5,
        },
        query_policy: FederatedQueryPolicy::QueryPeersOnMiss,
        mesh: None,
        trusted_peer_keys: vec![],
        require_pairing: false,
        user_id: String::new(),
        device_name: String::new(),
    };

    let engine_b = TransportEngine::start(config_b, id_b, None, None, None)
        .await
        .unwrap();

    // Give server A a moment to start listening
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // B connects to A
    let peer_id = engine_b
        .connect_to_peer(&format!("wss://{}", addr_a))
        .await
        .unwrap();
    assert_eq!(peer_id, engine_a.identity().peer_id());

    // B should have 1 connected peer
    assert_eq!(engine_b.connected_peer_count(), 1);
    assert_eq!(engine_b.connected_peer_ids(), vec![engine_a.identity().peer_id().to_string()]);

    // A should also have 1 connected peer (accepted by server)
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    assert_eq!(engine_a.connected_peer_count(), 1);

    // Clean shutdown
    engine_a.shutdown();
    engine_b.shutdown();
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
}

#[tokio::test]
async fn transport_engine_with_audit() {
    let dir = tempfile::TempDir::new().unwrap();
    let storage = Arc::new(
        nodedb_storage::StorageEngine::open(&dir.path().join("db")).unwrap(),
    );

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap().to_string();
    drop(listener);

    let config = TransportConfig {
        storage_path: String::new(),
        listen_addr: addr,
        tls_cert_pem: None,
        tls_key_pem: None,
        seed_peers: vec![],
        mdns_enabled: false,
        gossip: GossipConfig::default(),
        query_policy: FederatedQueryPolicy::LocalOnly,
        mesh: None,
        trusted_peer_keys: vec![],
        require_pairing: false,
        user_id: String::new(),
        device_name: String::new(),
    };

    let identity = NodeIdentity::generate();
    let engine = TransportEngine::start(config, identity, Some(storage), None, None)
        .await
        .unwrap();

    // Audit log should be available
    let audit = engine.audit_log().unwrap();
    assert_eq!(audit.count(), 0);

    // Append an audit entry
    let entry = nodedb_transport::types::NodeShareAuditEntry {
        id: 0,
        timestamp: chrono::Utc::now(),
        peer_id: "test-peer".to_string(),
        action: "query_response".to_string(),
        collection: Some("users".to_string()),
        record_count: 10,
        content_hash: "abc".to_string(),
    };
    let stored = audit.append(entry).unwrap();
    assert_eq!(stored.id, 1);
    assert_eq!(audit.count(), 1);

    // LocalOnly query returns None
    let result = engine.query(vec![1, 2, 3], 5).await.unwrap();
    assert!(result.is_none());

    engine.shutdown();
}

/// A test QueryHandler that echoes back the query_type and query_data.
struct EchoQueryHandler;

impl QueryHandler for EchoQueryHandler {
    fn handle_query(
        &self,
        query_type: &str,
        query_data: &[u8],
        _origin_peer_id: &str,
    ) -> Result<Vec<u8>, TransportError> {
        // Return a simple response: { query_type, data_len }
        let response = rmpv::Value::Map(vec![
            (
                rmpv::Value::String("query_type".into()),
                rmpv::Value::String(query_type.into()),
            ),
            (
                rmpv::Value::String("data_len".into()),
                rmpv::Value::Integer(rmpv::Integer::from(query_data.len() as i64)),
            ),
        ]);
        rmp_serde::to_vec(&response)
            .map_err(|e| TransportError::Serialization(e.to_string()))
    }
}

#[tokio::test]
async fn federated_query_with_query_handler() {
    // Peer A: has a QueryHandler, will answer queries
    let id_a = NodeIdentity::generate();
    let id_b = NodeIdentity::generate();

    let listener_a = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr_a = listener_a.local_addr().unwrap().to_string();
    drop(listener_a);

    let listener_b = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr_b = listener_b.local_addr().unwrap().to_string();
    drop(listener_b);

    let config_a = TransportConfig {
        storage_path: String::new(),
        listen_addr: addr_a.clone(),
        tls_cert_pem: None,
        tls_key_pem: None,
        seed_peers: vec![],
        mdns_enabled: false,
        gossip: GossipConfig { interval_seconds: 60, fan_out: 3, ttl: 5 },
        query_policy: FederatedQueryPolicy::QueryPeersAlways,
        mesh: None,
        trusted_peer_keys: vec![],
        require_pairing: false,
        user_id: String::new(),
        device_name: String::new(),
    };

    let handler_a: Arc<dyn QueryHandler> = Arc::new(EchoQueryHandler);
    let engine_a = TransportEngine::start(config_a, id_a, None, None, Some(handler_a))
        .await
        .unwrap();

    // Peer B: will send a query to Peer A
    let config_b = TransportConfig {
        storage_path: String::new(),
        listen_addr: addr_b.clone(),
        tls_cert_pem: None,
        tls_key_pem: None,
        seed_peers: vec![],
        mdns_enabled: false,
        gossip: GossipConfig { interval_seconds: 60, fan_out: 3, ttl: 5 },
        query_policy: FederatedQueryPolicy::QueryPeersAlways,
        mesh: None,
        trusted_peer_keys: vec![],
        require_pairing: false,
        user_id: String::new(),
        device_name: String::new(),
    };

    let engine_b = TransportEngine::start(config_b, id_b, None, None, None)
        .await
        .unwrap();

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // B connects to A
    engine_b
        .connect_to_peer(&format!("wss://{}", addr_a))
        .await
        .unwrap();

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // B sends a federated query envelope
    let envelope = nodedb_transport::types::FederatedQueryEnvelope {
        query_type: "nosql".to_string(),
        query_id: "test-q-1".to_string(),
        origin_peer_id: engine_b.identity().peer_id().to_string(),
        ttl: 3,
        query_data: vec![10, 20, 30],
        visited: vec![engine_b.identity().peer_id().to_string()],
    };
    let envelope_bytes = rmp_serde::to_vec(&envelope).unwrap();

    let results = engine_b
        .query_all(envelope_bytes, 5)
        .await
        .unwrap();

    // Should get one response from Peer A
    assert_eq!(results.len(), 1);

    // Parse the response
    let resp: FederatedQueryResponse = rmp_serde::from_slice(&results[0]).unwrap();
    assert!(resp.success);
    assert_eq!(resp.responder_peer_id, engine_a.identity().peer_id());

    // Parse the result data
    let result_val: rmpv::Value = rmp_serde::from_slice(&resp.result_data).unwrap();
    let qt = result_val.as_map().unwrap().iter()
        .find(|(k, _)| k.as_str() == Some("query_type"))
        .unwrap().1.as_str().unwrap();
    assert_eq!(qt, "nosql");

    engine_a.shutdown();
    engine_b.shutdown();
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
}

#[tokio::test]
async fn federated_query_ttl_zero_rejected() {
    let id_a = NodeIdentity::generate();
    let id_b = NodeIdentity::generate();

    let listener_a = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr_a = listener_a.local_addr().unwrap().to_string();
    drop(listener_a);

    let listener_b = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr_b = listener_b.local_addr().unwrap().to_string();
    drop(listener_b);

    let config_a = TransportConfig {
        storage_path: String::new(),
        listen_addr: addr_a.clone(),
        tls_cert_pem: None,
        tls_key_pem: None,
        seed_peers: vec![],
        mdns_enabled: false,
        gossip: GossipConfig { interval_seconds: 60, fan_out: 3, ttl: 5 },
        query_policy: FederatedQueryPolicy::QueryPeersAlways,
        mesh: None,
        trusted_peer_keys: vec![],
        require_pairing: false,
        user_id: String::new(),
        device_name: String::new(),
    };

    let handler: Arc<dyn QueryHandler> = Arc::new(EchoQueryHandler);
    let engine_a = TransportEngine::start(config_a, id_a, None, None, Some(handler))
        .await
        .unwrap();

    let config_b = TransportConfig {
        storage_path: String::new(),
        listen_addr: addr_b,
        tls_cert_pem: None,
        tls_key_pem: None,
        seed_peers: vec![],
        mdns_enabled: false,
        gossip: GossipConfig { interval_seconds: 60, fan_out: 3, ttl: 5 },
        query_policy: FederatedQueryPolicy::QueryPeersAlways,
        mesh: None,
        trusted_peer_keys: vec![],
        require_pairing: false,
        user_id: String::new(),
        device_name: String::new(),
    };

    let engine_b = TransportEngine::start(config_b, id_b, None, None, None)
        .await
        .unwrap();

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    engine_b
        .connect_to_peer(&format!("wss://{}", addr_a))
        .await
        .unwrap();

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // Send with TTL=0
    let envelope = nodedb_transport::types::FederatedQueryEnvelope {
        query_type: "nosql".to_string(),
        query_id: "test-q-ttl0".to_string(),
        origin_peer_id: engine_b.identity().peer_id().to_string(),
        ttl: 0,
        query_data: vec![1, 2, 3],
        visited: vec![],
    };
    let envelope_bytes = rmp_serde::to_vec(&envelope).unwrap();

    let results = engine_b
        .query_all(envelope_bytes, 3)
        .await
        .unwrap();

    // Should get a response with success=false (TTL expired)
    assert_eq!(results.len(), 1);
    let resp: FederatedQueryResponse = rmp_serde::from_slice(&results[0]).unwrap();
    assert!(!resp.success);
    assert_eq!(resp.error_message.unwrap(), "TTL expired");

    engine_a.shutdown();
    engine_b.shutdown();
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
}

/// Test that a query with the receiving peer's ID in the visited set
/// returns an empty success response (loop prevention).
#[tokio::test]
async fn federated_query_visited_loop_prevention() {
    let _ = rustls::crypto::ring::default_provider().install_default();

    let id_a = NodeIdentity::generate();
    let id_b = NodeIdentity::generate();
    let peer_a_id = id_a.peer_id().to_string();

    let listener_a = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr_a = listener_a.local_addr().unwrap().to_string();
    drop(listener_a);

    let listener_b = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr_b = listener_b.local_addr().unwrap().to_string();
    drop(listener_b);

    let config_a = TransportConfig {
        storage_path: String::new(),
        listen_addr: addr_a.clone(),
        tls_cert_pem: None,
        tls_key_pem: None,
        seed_peers: vec![],
        mdns_enabled: false,
        gossip: GossipConfig { interval_seconds: 60, fan_out: 3, ttl: 5 },
        query_policy: FederatedQueryPolicy::QueryPeersAlways,
        mesh: None,
        trusted_peer_keys: vec![],
        require_pairing: false,
        user_id: String::new(),
        device_name: String::new(),
    };

    let handler_a: Arc<dyn QueryHandler> = Arc::new(EchoQueryHandler);
    let engine_a = TransportEngine::start(config_a, id_a, None, None, Some(handler_a))
        .await
        .unwrap();

    let config_b = TransportConfig {
        storage_path: String::new(),
        listen_addr: addr_b.clone(),
        tls_cert_pem: None,
        tls_key_pem: None,
        seed_peers: vec![],
        mdns_enabled: false,
        gossip: GossipConfig { interval_seconds: 60, fan_out: 3, ttl: 5 },
        query_policy: FederatedQueryPolicy::QueryPeersAlways,
        mesh: None,
        trusted_peer_keys: vec![],
        require_pairing: false,
        user_id: String::new(),
        device_name: String::new(),
    };

    let engine_b = TransportEngine::start(config_b, id_b, None, None, None)
        .await
        .unwrap();

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    engine_b
        .connect_to_peer(&format!("wss://{}", addr_a))
        .await
        .unwrap();

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // Send envelope with Peer A's ID already in visited — should trigger loop prevention
    let envelope = nodedb_transport::types::FederatedQueryEnvelope {
        query_type: "nosql".to_string(),
        query_id: "test-q-loop".to_string(),
        origin_peer_id: engine_b.identity().peer_id().to_string(),
        ttl: 3,
        query_data: vec![10, 20, 30],
        visited: vec![
            engine_b.identity().peer_id().to_string(),
            peer_a_id, // Peer A is already "visited"
        ],
    };
    let envelope_bytes = rmp_serde::to_vec(&envelope).unwrap();

    let results = engine_b.query_all(envelope_bytes, 3).await.unwrap();

    // Peer A should return empty success (loop prevention)
    assert_eq!(results.len(), 1);
    let resp: FederatedQueryResponse = rmp_serde::from_slice(&results[0]).unwrap();
    assert!(resp.success);
    assert!(resp.result_data.is_empty(), "visited peer should return empty result");

    engine_a.shutdown();
    engine_b.shutdown();
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
}

/// Test that a query with TTL=1 executes locally but does NOT forward
/// (TTL is decremented to 0 before forwarding check).
#[tokio::test]
async fn federated_query_ttl1_no_forward() {
    let _ = rustls::crypto::ring::default_provider().install_default();

    let id_a = NodeIdentity::generate();
    let id_b = NodeIdentity::generate();

    let listener_a = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr_a = listener_a.local_addr().unwrap().to_string();
    drop(listener_a);

    let listener_b = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr_b = listener_b.local_addr().unwrap().to_string();
    drop(listener_b);

    let config_a = TransportConfig {
        storage_path: String::new(),
        listen_addr: addr_a.clone(),
        tls_cert_pem: None,
        tls_key_pem: None,
        seed_peers: vec![],
        mdns_enabled: false,
        gossip: GossipConfig { interval_seconds: 60, fan_out: 3, ttl: 5 },
        query_policy: FederatedQueryPolicy::QueryPeersAlways,
        mesh: None,
        trusted_peer_keys: vec![],
        require_pairing: false,
        user_id: String::new(),
        device_name: String::new(),
    };

    let handler_a: Arc<dyn QueryHandler> = Arc::new(EchoQueryHandler);
    let engine_a = TransportEngine::start(config_a, id_a, None, None, Some(handler_a))
        .await
        .unwrap();

    let config_b = TransportConfig {
        storage_path: String::new(),
        listen_addr: addr_b.clone(),
        tls_cert_pem: None,
        tls_key_pem: None,
        seed_peers: vec![],
        mdns_enabled: false,
        gossip: GossipConfig { interval_seconds: 60, fan_out: 3, ttl: 5 },
        query_policy: FederatedQueryPolicy::QueryPeersAlways,
        mesh: None,
        trusted_peer_keys: vec![],
        require_pairing: false,
        user_id: String::new(),
        device_name: String::new(),
    };

    let engine_b = TransportEngine::start(config_b, id_b, None, None, None)
        .await
        .unwrap();

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    engine_b
        .connect_to_peer(&format!("wss://{}", addr_a))
        .await
        .unwrap();

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // Send with TTL=1: Peer A should decrement to 0, execute locally, NOT forward
    let envelope = nodedb_transport::types::FederatedQueryEnvelope {
        query_type: "nosql".to_string(),
        query_id: "test-q-ttl1".to_string(),
        origin_peer_id: engine_b.identity().peer_id().to_string(),
        ttl: 1,
        query_data: vec![10, 20, 30],
        visited: vec![engine_b.identity().peer_id().to_string()],
    };
    let envelope_bytes = rmp_serde::to_vec(&envelope).unwrap();

    let results = engine_b.query_all(envelope_bytes, 3).await.unwrap();

    // Should get a successful response (local execution, no forwarding)
    assert_eq!(results.len(), 1);
    let resp: FederatedQueryResponse = rmp_serde::from_slice(&results[0]).unwrap();
    assert!(resp.success);
    // EchoQueryHandler returns non-empty data
    assert!(!resp.result_data.is_empty(), "TTL=1 should still execute locally");

    engine_a.shutdown();
    engine_b.shutdown();
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
}
