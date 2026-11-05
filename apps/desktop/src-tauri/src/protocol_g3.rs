//! Provisional server-owned G3 consumer; not connected to accepted G2 runtime yet.
#![allow(dead_code)]
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::sync::LazyLock;
pub const REVISION: &str = "g3-draft-4";
pub const PROVISIONAL_SCHEMA_SHA256: &str =
    "60681a8f53a36791dc945d18586296703f95401befe7b7b9cddfbd9881b0a534";
pub mod wire {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../packages/contracts/g3/g3.rs"
    ));
}
static SCHEMA: LazyLock<Value> = LazyLock::new(|| {
    serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../packages/contracts/g3/schema.json"
    )))
    .expect("canonical G3 schema")
});
const GOLDENS: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../packages/contracts/g3/golden.json"
));
include!("protocol_codec.rs");
#[test]
fn provisional_g3_schema_pin() {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../packages/contracts/g3/schema.json"
    )));
    assert_eq!(
        digest
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>(),
        PROVISIONAL_SCHEMA_SHA256
    );
}

