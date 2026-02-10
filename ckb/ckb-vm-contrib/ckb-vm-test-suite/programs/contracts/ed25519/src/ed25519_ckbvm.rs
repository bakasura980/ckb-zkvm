#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]

#[cfg(test)]
extern crate alloc;

#[cfg(not(test))]
ckb_std::entry!(program_entry);
// By default, the following heap configuration is used:
// * 16KB fixed heap
// * 1.2MB(rounded up to be 16-byte aligned) dynamic heap
// * Minimal memory block in dynamic heap is 64 bytes
// For more details, please refer to ckb-std's default_alloc macro
// and the buddy-alloc alloc implementation.
ckb_std::default_alloc!(16384, 1258306, 64);

mod ed25519;
use ed25519::execute;

pub fn program_entry() -> i8 {
    let args = ckb_std::env::argv();
    let n = if args.len() == 0 { 1 } else { args[0].to_str().unwrap().parse().unwrap() };
    for _ in 0..n {
        assert_eq!(execute(), 0);
    }
    0
}
