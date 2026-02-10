//! Optimized SHA-512 implementation for CKB-VM with Rust bindings.
//!
//! This crate provides a Rust interface to an optimized SHA-512 implementation
//! designed for CKB-VM (RISC-V).
//!
//! # Example
//!
//! ```
//! use ckb_opt_sha512::{Sha512, sha512};
//!
//! // One-shot hashing
//! let hash = sha512(b"hello world");
//! assert_eq!(hash.len(), 64);
//!
//! // Incremental hashing
//! let mut hasher = Sha512::new();
//! hasher.update(b"hello ");
//! hasher.update(b"world");
//! let hash = hasher.finalize();
//! ```

#![no_std]

pub const SHA512_BLOCK_SIZE: usize = 64;
pub type Digest = [u8; SHA512_BLOCK_SIZE];

mod ffi {
    use core::ffi::{c_uchar, c_ulonglong};

    #[repr(C, align(16))]
    pub struct Sha512Ctx {
        pub data: [c_uchar; 128],
        pub datalen: c_ulonglong,
        pub bitlen: u128,
        pub state: [c_ulonglong; 8],
    }

    #[allow(improper_ctypes)]
    extern "C" {
        pub fn sha512_init(ctx: *mut Sha512Ctx);
        pub fn sha512_update(ctx: *mut Sha512Ctx, data: *const c_uchar, len: c_ulonglong);
        pub fn sha512_final(ctx: *mut Sha512Ctx, hash: *mut c_uchar);
    }
}

/// SHA-512 hasher.
pub struct Sha512 {
    ctx: ffi::Sha512Ctx,
}

impl Default for Sha512 {
    fn default() -> Self {
        Self::new()
    }
}

impl Sha512 {
    /// Creates a new SHA-512 hasher instance.
    #[inline]
    pub fn new() -> Self {
        let mut ctx = ffi::Sha512Ctx { data: [0u8; 128], datalen: 0, bitlen: 0, state: [0u64; 8] };
        unsafe {
            ffi::sha512_init(&mut ctx);
        }
        Self { ctx }
    }

    /// Updates the hasher with input data.
    #[inline]
    pub fn update(&mut self, data: &[u8]) {
        unsafe {
            ffi::sha512_update(&mut self.ctx, data.as_ptr(), data.len() as u64);
        }
    }

    /// Finalizes the hash computation and returns the digest.
    #[inline]
    pub fn finalize(mut self) -> Digest {
        let mut hash = [0u8; SHA512_BLOCK_SIZE];
        unsafe {
            ffi::sha512_final(&mut self.ctx, hash.as_mut_ptr());
        }
        hash
    }
}

/// Computes the SHA-512 hash of the input data.
#[inline]
pub fn sha512(data: &[u8]) -> Digest {
    let mut hasher = Sha512::new();
    hasher.update(data);
    hasher.finalize()
}
