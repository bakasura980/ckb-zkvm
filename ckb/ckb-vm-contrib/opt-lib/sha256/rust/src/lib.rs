//! Optimized SHA-256 implementation for CKB-VM with Rust bindings.
//!
//! This crate provides a Rust interface to an optimized SHA-256 implementation
//! designed for CKB-VM (RISC-V).
//!
//! # Example
//!
//! ```
//! use ckb_opt_sha256::{Sha256, sha256};
//!
//! // One-shot hashing
//! let hash = sha256(b"hello world");
//! assert_eq!(hash.len(), 32);
//!
//! // Incremental hashing
//! let mut hasher = Sha256::new();
//! hasher.update(b"hello ");
//! hasher.update(b"world");
//! let hash = hasher.finalize();
//! ```

#![no_std]

pub const SHA256_BLOCK_SIZE: usize = 32;
pub type Digest = [u8; SHA256_BLOCK_SIZE];

mod ffi {
    use core::ffi::{c_uchar, c_uint, c_ulonglong};

    #[repr(C)]
    pub struct Sha256Ctx {
        pub data: [c_uchar; 64],
        pub datalen: c_uint,
        pub bitlen: c_ulonglong,
        pub state: [c_uint; 8],
    }

    extern "C" {
        pub fn sha256_init(ctx: *mut Sha256Ctx);
        pub fn sha256_update(ctx: *mut Sha256Ctx, data: *const c_uchar, len: c_ulonglong);
        pub fn sha256_final(ctx: *mut Sha256Ctx, hash: *mut c_uchar);
    }
}

/// SHA-256 hasher.
pub struct Sha256 {
    ctx: ffi::Sha256Ctx,
}

impl Default for Sha256 {
    fn default() -> Self {
        Self::new()
    }
}

impl Sha256 {
    /// Creates a new SHA-256 hasher instance.
    #[inline]
    pub fn new() -> Self {
        let mut ctx = ffi::Sha256Ctx { data: [0u8; 64], datalen: 0, bitlen: 0, state: [0u32; 8] };
        unsafe {
            ffi::sha256_init(&mut ctx);
        }
        Self { ctx }
    }

    /// Updates the hasher with input data.
    #[inline]
    pub fn update(&mut self, data: &[u8]) {
        unsafe {
            ffi::sha256_update(&mut self.ctx, data.as_ptr(), data.len() as u64);
        }
    }

    /// Finalizes the hash computation and returns the digest.
    #[inline]
    pub fn finalize(mut self) -> Digest {
        let mut hash = [0u8; SHA256_BLOCK_SIZE];
        unsafe {
            ffi::sha256_final(&mut self.ctx, hash.as_mut_ptr());
        }
        hash
    }
}

/// Computes the SHA-256 hash of the input data.
#[inline]
pub fn sha256(data: &[u8]) -> Digest {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize()
}
