//! Consumer of the canonical Server-owned D1 contract.
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::sync::LazyLock;
pub const REVISION:&str="g3-stats-1";
pub mod wire{include!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../packages/contracts/g3-stats/g3-stats.rs"));}
static SCHEMA:LazyLock<Value>=LazyLock::new(||serde_json::from_str(include_str!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../packages/contracts/g3-stats/schema.json"))).expect("canonical statistics schema"));
const GOLDENS:&str=include_str!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../packages/contracts/g3-stats/golden.json"));
include!("protocol_codec.rs");
#[cfg(test)]mod d1_golden_tests{use super::*;#[test]fn d1_generated_golden_codecs_match(){let fixtures:Value=serde_json::from_str(include_str!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../packages/contracts/g3-stats/golden.json"))).unwrap();for row in fixtures["fixtures"].as_array().unwrap(){let kind=row["type"].as_str().unwrap();let hex=row["hex"].as_str().unwrap();let bytes:Vec<u8>=(0..hex.len()).step_by(2).map(|i|u8::from_str_radix(&hex[i..i+2],16).unwrap()).collect();assert_eq!(binary_value(kind,&bytes).unwrap(),row["json"]);assert_eq!(json_value(kind,&serde_json::to_vec(&row["json"]).unwrap()).unwrap(),row["json"]);}}}
