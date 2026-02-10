fn main() {
    std::process::Command::new("sh").args(&["build.sh"]).status().unwrap();

    let spike_sys_path = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let spike_sys_path = std::path::Path::new(&spike_sys_path);

    println!("cargo:rustc-link-search=native={}", spike_sys_path.join("target").to_str().unwrap());
    println!("cargo:rustc-link-search=native={}", spike_sys_path.join("riscv-isa-sim/build").to_str().unwrap());

    println!("cargo:rustc-link-lib=static=riscv");
    println!("cargo:rustc-link-lib=static=softfloat");
    println!("cargo:rustc-link-lib=static=disasm");

    println!("cargo:rustc-link-lib=dylib=stdc++");
}
