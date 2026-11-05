//! Canonical G1 JSON adapter. Validation uses the server-owned schema; no private wire types.
#![allow(dead_code)]
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::sync::LazyLock;

pub mod wire {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../packages/contracts/g1/g1.rs"
    ));
}
static SCHEMA: LazyLock<Value> = LazyLock::new(|| {
    serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../packages/contracts/g1/schema.json"
    )))
    .expect("canonical compiled schema")
});
const GOLDENS: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../packages/contracts/g1/golden.json"
));
include!("protocol_codec.rs");
