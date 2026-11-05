//! Separately versioned SERVER-owned social wire; no capability grant.
#![allow(dead_code)]
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::sync::LazyLock;
// Generated from canonical inputs by build.rs; discovery and requests share it.
pub const REVISION:&str=env!("NT2_G3_SOCIAL_REVISION");
pub const SCHEMA_SHA:&str=env!("NT2_G3_SOCIAL_SCHEMA_SHA");
pub const POLICY_SHA:&str=env!("NT2_G3_SOCIAL_POLICY_SHA");
pub const POLICY:&str=env!("NT2_G3_SOCIAL_POLICY");
pub mod wire{include!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../packages/contracts/g3-social/g3-social.rs"));}
static SCHEMA:LazyLock<Value>=LazyLock::new(||serde_json::from_str(include_str!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../packages/contracts/g3-social/schema.json"))).unwrap());
const GOLDENS:&str=include_str!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../packages/contracts/g3-social/golden.json"));
include!("protocol_codec.rs");
