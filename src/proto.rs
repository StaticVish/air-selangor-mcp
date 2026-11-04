pub mod v1 {
    include!(concat!(env!("OUT_DIR"), "/airselangor.v1.rs"));
}

pub const PROTO_SCHEMA: &str = include_str!("../proto/air_selangor.proto");
