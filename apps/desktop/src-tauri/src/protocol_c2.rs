//! Consumer of the Server-owned, separately versioned C2 family.
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::sync::LazyLock;
pub const REVISION:&str="g3-c2-1";
pub mod wire{include!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../packages/contracts/g3-c2/g3-c2.rs"));}
static SCHEMA:LazyLock<Value>=LazyLock::new(||serde_json::from_str(include_str!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../packages/contracts/g3-c2/schema.json"))).expect("canonical C2 schema"));
const GOLDENS:&str=include_str!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../packages/contracts/g3-c2/golden.json"));
include!("protocol_codec.rs");


# [cfg(test)]mod c2_golden_tests{use super::*;
 #[test]fn c2_generated_golden_codecs_match(){let fixtures:Value=serde_json::from_str(GOLDENS).unwrap();for row in fixtures["fixtures"].as_array().unwrap(){let kind=row["type"].as_str().unwrap();let hex=row["hex"].as_str().unwrap();let bytes:Vec<u8>=(0..hex.len()).step_by(2).map(|i|u8::from_str_radix(&hex[i..i+2],16).unwrap()).collect();assert_eq!(binary_value(kind,&bytes).unwrap(),row["json"]);assert_eq!(json_value(kind,&serde_json::to_vec(&row["json"]).unwrap()).unwrap(),row["json"]);}}
}
