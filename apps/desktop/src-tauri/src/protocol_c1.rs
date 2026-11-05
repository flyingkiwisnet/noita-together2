//! Consumer of the Server-owned, separately versioned C1 family.
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::sync::LazyLock;
pub const REVISION:&str="g3-c1-1";
pub mod wire{include!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../packages/contracts/g3-c1/g3-c1.rs"));}
static SCHEMA:LazyLock<Value>=LazyLock::new(||serde_json::from_str(include_str!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../packages/contracts/g3-c1/schema.json"))).expect("canonical C1 schema"));
const GOLDENS:&str=include_str!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../packages/contracts/g3-c1/golden.json"));
include!("protocol_codec.rs");

#[cfg(test)]mod c1_tests{
 use super::*;
 #[test]fn canonical_rules_preserve_exact_names(){
  let fixtures:Value=serde_json::from_str(include_str!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../packages/contracts/g3-c1/fixture-inputs.json"))).unwrap();
  let original=fixtures.as_array().unwrap().iter().find(|f|f["type"]=="RuleSnapshot").unwrap()["json"].clone();
  let rules:wire::RuleSnapshot=typed("RuleSnapshot",&serde_json::to_vec(&original).unwrap()).unwrap();
  assert_eq!(serde_json::to_value(&rules).unwrap(),original);
  assert_eq!(rules.rules.unwrap().nt_sync_hearts,Some(true));
 }
}
