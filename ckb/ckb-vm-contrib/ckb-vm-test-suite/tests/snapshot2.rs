use ckb_vm_test_suite::*;

#[test]
pub fn test_ed25519() {
    run_snapshot2(BINARY_PATH_ED25519);
}

#[test]
pub fn test_k256_ecdsa() {
    run_snapshot2(BINARY_PATH_K256_ECDSA);
}

#[test]
pub fn test_k256_schnorr() {
    run_snapshot2(BINARY_PATH_K256_SCHNORR);
}

#[test]
pub fn test_p256() {
    run_snapshot2(BINARY_PATH_P256);
}

#[test]
pub fn test_rsa() {
    run_snapshot2(BINARY_PATH_RSA);
}

#[test]
pub fn test_secp256k1_ecdsa() {
    run_snapshot2(BINARY_PATH_SECP256K1_ECDSA);
}

#[test]
pub fn test_secp256k1_schnorr() {
    run_snapshot2(BINARY_PATH_SECP256K1_SCHNORR);
}

#[test]
pub fn test_sphincsplus_ref() {
    run_snapshot2(BINARY_PATH_SPHINCSPLUS_REF);
}
