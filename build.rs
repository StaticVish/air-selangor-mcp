fn main() {
    println!("cargo:rerun-if-changed=proto/air_selangor.proto");
    let mut config = prost_build::Config::new();
    config.type_attribute(".", "#[derive(serde::Serialize, serde::Deserialize)]");
    config
        .compile_protos(&["proto/air_selangor.proto"], &["proto/"])
        .expect("Failed to compile protobuf definitions");
}
