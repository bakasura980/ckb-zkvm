#[macro_use]
extern crate criterion;

use ckb_vm_test_suite::{
    BINARY_PATH_ED25519, BINARY_PATH_K256_ECDSA, BINARY_PATH_K256_SCHNORR, BINARY_PATH_P256, BINARY_PATH_RSA,
    BINARY_PATH_SECP256K1_ECDSA, BINARY_PATH_SECP256K1_SCHNORR, BINARY_PATH_SPHINCSPLUS_REF, NATIVE_PATH_ED25519,
    NATIVE_PATH_K256_ECDSA, NATIVE_PATH_K256_SCHNORR, NATIVE_PATH_P256, NATIVE_PATH_RSA, NATIVE_PATH_SECP256K1_ECDSA,
    NATIVE_PATH_SECP256K1_SCHNORR, NATIVE_PATH_SPHINCSPLUS_REF, run_asm, run_interpret, run_mop, run_native,
};
use criterion::Criterion;
use std::fs;

fn asm_ed25519(c: &mut Criterion) {
    c.bench_function("asm_ed25519", |b| {
        let buffer = fs::read(BINARY_PATH_ED25519).unwrap().into();
        b.iter(|| run_asm(&buffer));
    });
}

fn asm_k256_ecdsa(c: &mut Criterion) {
    c.bench_function("asm_k256_ecdsa", |b| {
        let buffer = fs::read(BINARY_PATH_K256_ECDSA).unwrap().into();
        b.iter(|| run_asm(&buffer));
    });
}

fn asm_k256_schnorr(c: &mut Criterion) {
    c.bench_function("asm_k256_schnorr", |b| {
        let buffer = fs::read(BINARY_PATH_K256_SCHNORR).unwrap().into();
        b.iter(|| run_asm(&buffer));
    });
}

fn asm_p256(c: &mut Criterion) {
    c.bench_function("asm_p256", |b| {
        let buffer = fs::read(BINARY_PATH_P256).unwrap().into();
        b.iter(|| run_asm(&buffer));
    });
}

fn asm_rsa(c: &mut Criterion) {
    c.bench_function("asm_rsa", |b| {
        let buffer = fs::read(BINARY_PATH_RSA).unwrap().into();
        b.iter(|| run_asm(&buffer));
    });
}

fn asm_secp256k1_ecdsa(c: &mut Criterion) {
    c.bench_function("asm_secp256k1_ecdsa", |b| {
        let buffer = fs::read(BINARY_PATH_SECP256K1_ECDSA).unwrap().into();
        b.iter(|| run_asm(&buffer));
    });
}

fn asm_secp256k1_schnorr(c: &mut Criterion) {
    c.bench_function("asm_secp256k1_schnorr", |b| {
        let buffer = fs::read(BINARY_PATH_SECP256K1_SCHNORR).unwrap().into();
        b.iter(|| run_asm(&buffer));
    });
}

fn asm_sphincsplus_ref(c: &mut Criterion) {
    c.bench_function("asm_sphincsplus_ref", |b| {
        let buffer = fs::read(BINARY_PATH_SPHINCSPLUS_REF).unwrap().into();
        b.iter(|| run_asm(&buffer));
    });
}

fn interpret_ed25519(c: &mut Criterion) {
    c.bench_function("interpret_ed25519", |b| {
        let buffer = fs::read(BINARY_PATH_ED25519).unwrap().into();
        b.iter(|| run_interpret(&buffer));
    });
}

fn interpret_k256_ecdsa(c: &mut Criterion) {
    c.bench_function("interpret_k256_ecdsa", |b| {
        let buffer = fs::read(BINARY_PATH_K256_ECDSA).unwrap().into();
        b.iter(|| run_interpret(&buffer));
    });
}

fn interpret_k256_schnorr(c: &mut Criterion) {
    c.bench_function("interpret_k256_schnorr", |b| {
        let buffer = fs::read(BINARY_PATH_K256_SCHNORR).unwrap().into();
        b.iter(|| run_interpret(&buffer));
    });
}

fn interpret_p256(c: &mut Criterion) {
    c.bench_function("interpret_p256", |b| {
        let buffer = fs::read(BINARY_PATH_P256).unwrap().into();
        b.iter(|| run_interpret(&buffer));
    });
}

fn interpret_rsa(c: &mut Criterion) {
    c.bench_function("interpret_rsa", |b| {
        let buffer = fs::read(BINARY_PATH_RSA).unwrap().into();
        b.iter(|| run_interpret(&buffer));
    });
}

fn interpret_secp256k1_ecdsa(c: &mut Criterion) {
    c.bench_function("interpret_secp256k1_ecdsa", |b| {
        let buffer = fs::read(BINARY_PATH_SECP256K1_ECDSA).unwrap().into();
        b.iter(|| run_interpret(&buffer));
    });
}

fn interpret_secp256k1_schnorr(c: &mut Criterion) {
    c.bench_function("interpret_secp256k1_schnorr", |b| {
        let buffer = fs::read(BINARY_PATH_SECP256K1_SCHNORR).unwrap().into();
        b.iter(|| run_interpret(&buffer));
    });
}

fn interpret_sphincsplus_ref(c: &mut Criterion) {
    c.bench_function("interpret_sphincsplus_ref", |b| {
        let buffer = fs::read(BINARY_PATH_SPHINCSPLUS_REF).unwrap().into();
        b.iter(|| run_interpret(&buffer));
    });
}

fn mop_ed25519(c: &mut Criterion) {
    c.bench_function("mop_ed25519", |b| {
        let buffer = fs::read(BINARY_PATH_ED25519).unwrap().into();
        b.iter(|| run_mop(&buffer));
    });
}

fn mop_k256_ecdsa(c: &mut Criterion) {
    c.bench_function("mop_k256_ecdsa", |b| {
        let buffer = fs::read(BINARY_PATH_K256_ECDSA).unwrap().into();
        b.iter(|| run_mop(&buffer));
    });
}

fn mop_k256_schnorr(c: &mut Criterion) {
    c.bench_function("mop_k256_schnorr", |b| {
        let buffer = fs::read(BINARY_PATH_K256_SCHNORR).unwrap().into();
        b.iter(|| run_mop(&buffer));
    });
}

fn mop_p256(c: &mut Criterion) {
    c.bench_function("mop_p256", |b| {
        let buffer = fs::read(BINARY_PATH_P256).unwrap().into();
        b.iter(|| run_mop(&buffer));
    });
}

fn mop_rsa(c: &mut Criterion) {
    c.bench_function("mop_rsa", |b| {
        let buffer = fs::read(BINARY_PATH_RSA).unwrap().into();
        b.iter(|| run_mop(&buffer));
    });
}

fn mop_secp256k1_ecdsa(c: &mut Criterion) {
    c.bench_function("mop_secp256k1_ecdsa", |b| {
        let buffer = fs::read(BINARY_PATH_SECP256K1_ECDSA).unwrap().into();
        b.iter(|| run_mop(&buffer));
    });
}

fn mop_secp256k1_schnorr(c: &mut Criterion) {
    c.bench_function("mop_secp256k1_schnorr", |b| {
        let buffer = fs::read(BINARY_PATH_SECP256K1_SCHNORR).unwrap().into();
        b.iter(|| run_mop(&buffer));
    });
}

fn mop_sphincsplus_ref(c: &mut Criterion) {
    c.bench_function("mop_sphincsplus_ref", |b| {
        let buffer = fs::read(BINARY_PATH_SPHINCSPLUS_REF).unwrap().into();
        b.iter(|| run_mop(&buffer));
    });
}

fn native_ed25519(c: &mut Criterion) {
    c.bench_function("native_ed25519", |b| {
        b.iter(|| run_native(NATIVE_PATH_ED25519));
    });
}

fn native_k256_ecdsa(c: &mut Criterion) {
    c.bench_function("native_k256_ecdsa", |b| {
        b.iter(|| run_native(NATIVE_PATH_K256_ECDSA));
    });
}

fn native_k256_schnorr(c: &mut Criterion) {
    c.bench_function("native_k256_schnorr", |b| {
        b.iter(|| run_native(NATIVE_PATH_K256_SCHNORR));
    });
}

fn native_p256(c: &mut Criterion) {
    c.bench_function("native_p256", |b| {
        b.iter(|| run_native(NATIVE_PATH_P256));
    });
}

fn native_rsa(c: &mut Criterion) {
    c.bench_function("native_rsa", |b| {
        b.iter(|| run_native(NATIVE_PATH_RSA));
    });
}

fn native_secp256k1_ecdsa(c: &mut Criterion) {
    c.bench_function("native_secp256k1_ecdsa", |b| {
        b.iter(|| run_native(NATIVE_PATH_SECP256K1_ECDSA));
    });
}

fn native_secp256k1_schnorr(c: &mut Criterion) {
    c.bench_function("native_secp256k1_schnorr", |b| {
        b.iter(|| run_native(NATIVE_PATH_SECP256K1_SCHNORR));
    });
}

fn native_sphincsplus_ref(c: &mut Criterion) {
    c.bench_function("native_sphincsplus_ref", |b| {
        b.iter(|| run_native(NATIVE_PATH_SPHINCSPLUS_REF));
    });
}

criterion_group!(
    benches,
    asm_ed25519,
    asm_k256_ecdsa,
    asm_k256_schnorr,
    asm_p256,
    asm_rsa,
    asm_secp256k1_ecdsa,
    asm_secp256k1_schnorr,
    asm_sphincsplus_ref,
    interpret_ed25519,
    interpret_k256_ecdsa,
    interpret_k256_schnorr,
    interpret_p256,
    interpret_rsa,
    interpret_secp256k1_ecdsa,
    interpret_secp256k1_schnorr,
    interpret_sphincsplus_ref,
    mop_ed25519,
    mop_k256_ecdsa,
    mop_k256_schnorr,
    mop_p256,
    mop_rsa,
    mop_secp256k1_ecdsa,
    mop_secp256k1_schnorr,
    mop_sphincsplus_ref,
    native_ed25519,
    native_k256_ecdsa,
    native_k256_schnorr,
    native_p256,
    native_rsa,
    native_secp256k1_ecdsa,
    native_secp256k1_schnorr,
    native_sphincsplus_ref,
);
criterion_main!(benches);
