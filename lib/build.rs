//! Build script for protobuf code generation.
//!
//! Generates Rust types from proto/trace.proto at compile time.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Rerun if proto file changes
    println!("cargo:rerun-if-changed=proto/trace.proto");

    // Configure prost-build
    let mut config = prost_build::Config::new();

    // Add serde derives for JSON serialization (useful for debugging)
    config.type_attribute(".", "#[derive(serde::Serialize, serde::Deserialize)]");

    // Generate code from proto file
    config.compile_protos(&["proto/trace.proto"], &["proto/"])?;

    Ok(())
}
