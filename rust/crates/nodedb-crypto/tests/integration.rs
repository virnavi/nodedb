use nodedb_crypto::{NodeIdentity, PublicIdentity, seal_envelope, open_envelope};

#[test]
fn identity_generation_unique() {
    let id1 = NodeIdentity::generate();
    let id2 = NodeIdentity::generate();
    assert_ne!(id1.peer_id(), id2.peer_id());
}

#[test]
fn identity_from_bytes_deterministic() {
    let id1 = NodeIdentity::generate();
    let bytes = id1.signing_key_bytes();
    let id2 = NodeIdentity::from_signing_key_bytes(&bytes).unwrap();
    let id3 = NodeIdentity::from_signing_key_bytes(&bytes).unwrap();
    assert_eq!(id2.peer_id(), id3.peer_id());
    assert_eq!(id1.peer_id(), id2.peer_id());
}

#[test]
fn sign_verify_roundtrip() {
    let id = NodeIdentity::generate();
    let public = id.to_public();
    let data = b"test data for signing";
    let sig = id.sign(data);
    public.verify(data, &sig).unwrap();
}

#[test]
fn cross_identity_verification_fails() {
    let alice = NodeIdentity::generate();
    let bob = NodeIdentity::generate();
    let sig = alice.sign(b"data");
    assert!(bob.to_public().verify(b"data", &sig).is_err());
}

#[test]
fn envelope_roundtrip_between_identities() {
    let alice = NodeIdentity::generate();
    let bob = NodeIdentity::generate();

    let message = b"confidential federation payload";
    let envelope = seal_envelope(&alice, &bob.to_public(), message).unwrap();

    // Bob decrypts
    let plaintext = open_envelope(&bob, &envelope).unwrap();
    assert_eq!(plaintext, message);

    // Alice cannot decrypt her own message to Bob
    // (she used Bob's public key, not her own)
    let result = open_envelope(&alice, &envelope);
    assert!(result.is_err());
}

#[test]
fn public_identity_from_raw_bytes() {
    let id = NodeIdentity::generate();
    let bytes = id.verifying_key_bytes();
    let pi = PublicIdentity::from_public_key_bytes(&bytes).unwrap();
    assert_eq!(pi.peer_id, id.peer_id());

    // Verify signature using reconstructed public identity
    let sig = id.sign(b"test");
    pi.verify(b"test", &sig).unwrap();
}

#[test]
fn invalid_public_key_bytes() {
    assert!(PublicIdentity::from_public_key_bytes(&[0u8; 16]).is_err());
}

#[test]
fn envelope_sender_key_matches() {
    let alice = NodeIdentity::generate();
    let bob = NodeIdentity::generate();
    let env = seal_envelope(&alice, &bob.to_public(), b"test").unwrap();
    assert_eq!(env.sender_public_key, alice.verifying_key_bytes().to_vec());
}
