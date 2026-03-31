use rc_crypto::keypair::Keypair;
use rc_primitives::types::Address;

// ─── Key Generation ────────────────────────────────────────────────────────

#[test]
fn test_keypair_generate_unique() {
    let kp1 = Keypair::generate();
    let kp2 = Keypair::generate();
    // Два случайных ключа не должны совпадать (вероятность 1/2^256)
    assert_ne!(kp1.public.as_bytes(), kp2.public.as_bytes());
}

#[test]
fn test_address_from_keypair() {
    let kp = Keypair::generate();
    let addr = kp.address();
    // Адрес должен быть 20 байт (не ZERO)
    assert_ne!(addr, Address::ZERO);
}

#[test]
fn test_same_keypair_same_address() {
    let kp = Keypair::generate();
    let addr1 = kp.address();
    let addr2 = kp.public.to_address();
    assert_eq!(addr1, addr2);
}

// ─── Signing & Verification ─────────────────────────────────────────────────

#[test]
fn test_sign_and_verify_ok() {
    let kp = Keypair::generate();
    let msg = b"transfer 100 RSC to Alice";
    let sig = kp.sign(msg);
    assert!(kp.public.verify(msg, &sig).is_ok());
}

#[test]
fn test_tampered_message_fails_verification() {
    let kp = Keypair::generate();
    let msg = b"transfer 100 RSC to Alice";
    let tampered = b"transfer 999 RSC to Alice";
    let sig = kp.sign(msg);
    assert!(kp.public.verify(tampered, &sig).is_err());
}

#[test]
fn test_wrong_key_fails_verification() {
    let kp1 = Keypair::generate();
    let kp2 = Keypair::generate();
    let msg = b"hello";
    let sig = kp1.sign(msg);
    // Подпись kp1 не может быть верифицирована публичным ключом kp2
    assert!(kp2.public.verify(msg, &sig).is_err());
}

#[test]
fn test_signing_is_deterministic() {
    // Ed25519 детерминирован: одинаковый ключ + сообщение → одинаковая подпись
    let kp = Keypair::generate();
    let msg = b"deterministic signing test";
    let sig1 = kp.sign(msg);
    let sig2 = kp.sign(msg);
    assert_eq!(sig1.as_bytes(), sig2.as_bytes());
}

#[test]
fn test_signature_from_bytes_roundtrip() {
    let kp = Keypair::generate();
    let msg = b"roundtrip test";
    let sig = kp.sign(msg);

    let bytes = sig.to_vec();
    let sig_back = rc_crypto::Signature::from_bytes(&bytes).expect("valid sig bytes");
    assert!(kp.public.verify(msg, &sig_back).is_ok());
}

#[test]
fn test_invalid_signature_bytes_rejected() {
    let bad_bytes = vec![0u8; 32]; // слишком короткие (нужно 64)
    assert!(rc_crypto::Signature::from_bytes(&bad_bytes).is_err());
}

#[test]
fn test_public_key_from_bytes_roundtrip() {
    let kp = Keypair::generate();
    let bytes = kp.public.as_bytes();
    let pub_key = rc_crypto::keypair::PublicKey::from_bytes(bytes).expect("valid pubkey");
    assert_eq!(kp.public.as_bytes(), pub_key.as_bytes());
}
