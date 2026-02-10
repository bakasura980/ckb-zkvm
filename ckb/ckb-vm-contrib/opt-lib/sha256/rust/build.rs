use std::env;

fn main() {
    let target = env::var("TARGET").unwrap();

    let mut builder = cc::Build::new();
    builder.file("src/sha256.c").include("src").opt_level(3);

    if target.contains("riscv64") {
        builder.flag("-march=rv64imc_zba_zbb_zbc_zbs");
    }

    builder.compile("sha256");

    println!("cargo:rerun-if-changed=src/sha256.c");
    println!("cargo:rerun-if-changed=src/sha256.h");
    println!("cargo:rerun-if-changed=build.rs");
}
