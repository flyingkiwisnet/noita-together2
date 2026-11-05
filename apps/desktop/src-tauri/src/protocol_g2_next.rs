//! Published draft-4 candidate consumer; real-engine gate remains separate.
#![allow(dead_code)]
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::sync::LazyLock;
pub const REVISION: &str = "g2-draft-5";
pub const REVIEWED_SCHEMA_SHA256: &str =
    "93143bf9d249ad4ae10ed3b24b6dc9ef678ab96aceb2958b32854540984d8e44";
pub mod wire {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../packages/contracts/g2-next/g2-next.rs"
    ));
}
static SCHEMA: LazyLock<Value> = LazyLock::new(|| {
    serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../packages/contracts/g2-next/schema.json"
    )))
    .expect("canonical candidate schema")
});
const GOLDENS: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../packages/contracts/g2-next/golden.json"
));
include!("protocol_codec.rs");
/// Share the existing duplicate-key/trailing-data rejection with separately
/// declared canonical metadata consumers, without relaxing the G2 codec.
pub(crate) fn strict_json(bytes:&[u8])->Result<Value,String>{
    if bytes.len()>65536{return Err("JSON response exceeds bound".into())}
    let mut decoder=serde_json::Deserializer::from_slice(bytes);
    let value=<StrictValue as serde::Deserialize>::deserialize(&mut decoder).map_err(|_|"Invalid unique JSON object")?.0;
    decoder.end().map_err(|_|"Trailing JSON data")?;Ok(value)
}
pub const GOLD_CAPABILITIES:&[&str]=&["core.seed","core.ghost","core.hud","core.poi","core.seed-policy.v1","core.randomize-loot.v1","core.emotes","core.cosmetics","core.gold"];
pub const ITEM_CAPABILITIES:&[&str]=&["core.seed","core.ghost","core.hud","core.poi","core.seed-policy.v1","core.randomize-loot.v1","core.emotes","core.cosmetics","core.gold","core.items"];
pub const C2_CAPABILITIES:&[&str]=&["core.seed","core.ghost","core.hud","core.poi","core.seed-policy.v1","core.randomize-loot.v1","core.emotes","core.cosmetics","core.gold","core.items","core.world-rules","core.hourglass"];
pub const RACE_CAPABILITIES:&[&str]=&["core.seed","core.ghost","core.hud","core.poi","core.seed-policy.v1","core.randomize-loot.v1","core.emotes","core.cosmetics","core.gold","core.items","core.world-rules","core.hourglass","race.goals.v1"];
pub fn info(bytes: &[u8]) -> Result<wire::Info, ProtocolError> {
    // Canonical C1 metadata is a separately declared extension to legacy Info.
    // Validate that exact declaration, then retain the strict legacy codec.
    if bytes.len()>65536{return Err(ProtocolError::TooLarge);}
    let mut decoder=serde_json::Deserializer::from_slice(bytes);
    let mut value=<StrictValue as serde::Deserialize>::deserialize(&mut decoder).map_err(|_|ProtocolError::InvalidMessage)?.0;
    decoder.end().map_err(|_|ProtocolError::InvalidMessage)?;
    // Separately declared development availability is metadata, never effect
    // authority. Consume only the exact canonical extension before the strict
    // legacy Info decoder; unknown fields/features still fail closed.
    if let Some(features)=value.as_object_mut().and_then(|v|v.remove("developmentFeatures")){
        let catalogue:Value=serde_json::from_str(include_str!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../packages/contracts/g3/testing-starting-loadout.json"))).map_err(|_|ProtocolError::InvalidMessage)?;
        let key=catalogue["settingKey"].as_str().ok_or(ProtocolError::InvalidMessage)?;
        let expected=serde_json::json!({"testingStartingLoadout":{"available":true,"settingKey":key,"default":catalogue["default"]}});
        if features!=expected||value["compatibility"]["settingKeys"].as_array().is_none_or(|v|v.iter().filter(|v|v.as_str()==Some(key)).count()!=1){return Err(ProtocolError::InvalidMessage)}
    }
    if let Some(declaration)=value.as_object_mut().and_then(|v|v.remove("roomSemantics")){
        let expected:&[&str]=match declaration["kind"].as_str(){Some("core-seed")=>&["core.seed"],Some("core-ghost")=>&["core.seed","core.ghost"],Some("core-social")=>&["core.seed","core.ghost","core.hud","core.poi"],Some("core-social-seed-loot")=>&["core.seed","core.ghost","core.hud","core.poi","core.seed-policy.v1","core.randomize-loot.v1"],Some("core-social-controls-seed-loot")=>&["core.seed","core.ghost","core.hud","core.poi","core.seed-policy.v1","core.randomize-loot.v1","core.emotes","core.cosmetics"],Some("core-gold-social-controls-seed-loot")=>GOLD_CAPABILITIES,Some("core-items-gold-social-controls-seed-loot")=>ITEM_CAPABILITIES,Some("core-world-rules-hourglass-items-gold-social-controls-seed-loot")=>C2_CAPABILITIES,_=>return Err(ProtocolError::InvalidMessage)};
        let exact=|v:&Value,wanted:&[&str]|v.as_array().is_some_and(|a|a.len()==wanted.len()&&wanted.iter().all(|s|a.iter().filter(|v|v.as_str()==Some(s)).count()==1));
        let supported=value["compatibility"]["capabilities"].as_array().ok_or(ProtocolError::InvalidMessage)?;
        if declaration.as_object().is_none_or(|d|d.len()!=3)||declaration["revision"]!="g3-room-semantics-1"||!exact(&declaration["requiredCapabilities"],expected)
            ||supported.len()>32||supported.iter().any(|v|v.as_str().is_none())||supported.iter().filter_map(Value::as_str).collect::<std::collections::BTreeSet<_>>().len()!=supported.len()
            ||!expected.iter().all(|cap|supported.iter().any(|v|v.as_str()==Some(cap)))||expected.len()>1&&!exact(&value["compatibility"]["capabilities"],expected){return Err(ProtocolError::InvalidMessage);}
    }
    typed("Info",&serde_json::to_vec(&value).map_err(|_|ProtocolError::InvalidMessage)?)
}
#[cfg(test)]#[test]fn a3_room_semantics_accepts_shipped_eight_cap_profile_and_rejects_substitutions(){
    let shipped:Value=serde_json::from_str("{}").unwrap();
    let caps=shipped["compatibility"]["capabilities"].clone();assert_eq!(caps.as_array().unwrap().len(),8);
    let base=serde_json::json!({"major":1,"revision":REVISION,"serviceId":shipped["serviceId"],"serviceEpoch":"consumer-fixture","compatibility":shipped["compatibility"],"profile":shipped["buildRegistry"],"roomSemantics":{"revision":"g3-room-semantics-1","kind":"core-social-controls-seed-loot","requiredCapabilities":caps}});
    assert!(info(&serde_json::to_vec(&base).unwrap()).is_ok(),"Actual shipped normal8-cap metadata must admit its exact A3 declaration");
    let mut reordered=base.clone();reordered["compatibility"]["capabilities"].as_array_mut().unwrap().reverse();assert!(info(&serde_json::to_vec(&reordered).unwrap()).is_ok(),"Existing exact-set semantics do not depend on wire order");
    for field in ["requiredCapabilities","capabilities"]{for index in 0..8{
        let parent=if field=="requiredCapabilities"{"roomSemantics"}else{"compatibility"};
        let mut missing=base.clone();missing[parent][field].as_array_mut().unwrap().remove(index);assert!(info(&serde_json::to_vec(&missing).unwrap()).is_err(),"Missing {field} entry{index}");
        let mut duplicate=base.clone();duplicate[parent][field][index]=duplicate[parent][field][(index+1)%8].clone();assert!(info(&serde_json::to_vec(&duplicate).unwrap()).is_err(),"Substituted duplicate {field} entry{index}");
    }let parent=if field=="requiredCapabilities"{"roomSemantics"}else{"compatibility"};let mut extra=base.clone();extra[parent][field].as_array_mut().unwrap().push(serde_json::json!("service-only"));assert!(info(&serde_json::to_vec(&extra).unwrap()).is_err(),"Unexpected {field} cannot broaden A3");}
    for kind in ["core-social-seed-loot","unknown"]{let mut bad=base.clone();bad["roomSemantics"]["kind"]=kind.into();assert!(info(&serde_json::to_vec(&bad).unwrap()).is_err());}
    for fault in 0..3{let mut bad=base.clone();match fault{0=>bad["roomSemantics"]["revision"]="unknown".into(),1=>bad["roomSemantics"]["extra"]=true.into(),_=>bad["roomSemantics"]["requiredCapabilities"][0]=Value::Null};assert!(info(&serde_json::to_vec(&bad).unwrap()).is_err());}
}
#[cfg(test)]
#[test]
fn reviewed_schema_pin() {
    use sha2::{Digest, Sha256};
    let hash = Sha256::digest(include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../packages/contracts/g2-next/schema.json"
    )));
    assert_eq!(
        hash.iter().map(|b| format!("{b:02x}")).collect::<String>(),
        REVIEWED_SCHEMA_SHA256
    );
}

#[cfg(all(test,feature="g3-rejoin"))]
#[test]fn b1_room_semantics_consumes_server_public_info_and_rejects_capability_substitutions(){
    let approval:Value=serde_json::from_slice(include_bytes!(env!("NT2_REJOIN_RUNTIME"))).unwrap();
    if !approval["compatibility"]["capabilities"].as_array().unwrap().iter().any(|c|c=="core.gold"){return}
    let repo=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).ancestors().nth(3).unwrap();
    let mut command=std::process::Command::new("C:/Program Files/nodejs/node.exe");
    #[cfg(windows)]{use std::os::windows::process::CommandExt;command.creation_flags(0x08000000);}
    let output=command.current_dir(repo).arg("apps/desktop/tools/b1-public-info-fixture.mjs").env("NT2_B1_INFO_RUNTIME",env!("NT2_REJOIN_RUNTIME")).output().unwrap();
    assert!(output.status.success(),"{}",String::from_utf8_lossy(&output.stderr));
    let base:Value=serde_json::from_slice(&output.stdout).unwrap();
    let current=approval["compatibility"]["capabilities"].as_array().unwrap().iter().any(|c|c=="core.world-rules");
    let(kind,count)=if current{("core-world-rules-hourglass-items-gold-social-controls-seed-loot",12)}else{("core-gold-social-controls-seed-loot",9)};
    assert_eq!(base["roomSemantics"]["kind"],kind);
    assert_eq!(base["compatibility"]["capabilities"].as_array().unwrap().len(),count);
    let consumed=info(&output.stdout).expect("Actual Server public Info envelope and approved capability declaration must be supported");
    assert_eq!(consumed.capabilities.unwrap().len(),count);
    assert_eq!(consumed.profile.unwrap().id.as_deref(),approval["buildRegistry"]["id"].as_str());
    for (parent,field) in [("roomSemantics","requiredCapabilities"),("compatibility","capabilities")]{
        for index in 0..count{
            let mut missing=base.clone();missing[parent][field].as_array_mut().unwrap().remove(index);assert!(info(&serde_json::to_vec(&missing).unwrap()).is_err());
            let mut duplicate=base.clone();duplicate[parent][field][index]=duplicate[parent][field][(index+1)%count].clone();assert!(info(&serde_json::to_vec(&duplicate).unwrap()).is_err());
        }
        let mut extra=base.clone();extra[parent][field].as_array_mut().unwrap().push(serde_json::json!("service-only"));assert!(info(&serde_json::to_vec(&extra).unwrap()).is_err());
        let mut reordered=base.clone();reordered[parent][field].as_array_mut().unwrap().reverse();assert!(info(&serde_json::to_vec(&reordered).unwrap()).is_ok());
    }
    for kind in ["core-social-controls-seed-loot","unknown"]{let mut wrong=base.clone();wrong["roomSemantics"]["kind"]=kind.into();assert!(info(&serde_json::to_vec(&wrong).unwrap()).is_err());}
    let mut unknown=base.clone();unknown["roomSemantics"]["unexpected"]=true.into();assert!(info(&serde_json::to_vec(&unknown).unwrap()).is_err());
}

#[cfg(test)]#[test]fn c1_semantic_metadata_extension_stays_strict(){
    let base=serde_json::json!({"major":1,"revision":REVISION,"serviceId":"fixture","serviceEpoch":"epoch","compatibility":{"capabilities":["core.seed","core.ghost","core.hud","core.poi"]},"roomSemantics":{"revision":"g3-room-semantics-1","kind":"core-social","requiredCapabilities":["core.seed","core.ghost","core.hud","core.poi"]}});
    assert!(info(&serde_json::to_vec(&base).unwrap()).is_ok());
    for fault in 0..6{
        let mut bad=base.clone();match fault{
            0=>bad["roomSemantics"]["revision"]="unknown".into(),
            1=>bad["roomSemantics"]["invented"]=true.into(),
            2=>bad["roomSemantics"]["requiredCapabilities"]=serde_json::json!(["core.seed","core.seed","core.hud","core.poi"]),
            3=>bad["compatibility"]["capabilities"]=serde_json::json!(["core.seed","core.ghost","core.hud","core.poi","extra"]),
            4=>bad["invented"]=true.into(),
            _=>bad["roomSemantics"]=Value::Null,
        }assert!(info(&serde_json::to_vec(&bad).unwrap()).is_err());
    }
    let mut legacy=base.clone();legacy.as_object_mut().unwrap().remove("roomSemantics");assert!(info(&serde_json::to_vec(&legacy).unwrap()).is_ok());
    let text=serde_json::to_string(&base).unwrap();let duplicate=text.replacen("\"kind\":\"core-social\"","\"kind\":\"core-social\",\"kind\":\"core-social\"",1);assert!(info(duplicate.as_bytes()).is_err());
    let mut seed_loot=base.clone();seed_loot["roomSemantics"]["kind"]="core-social-seed-loot".into();
    let caps=serde_json::json!(["core.seed","core.ghost","core.hud","core.poi","core.seed-policy.v1","core.randomize-loot.v1"]);
    seed_loot["roomSemantics"]["requiredCapabilities"]=caps.clone();seed_loot["compatibility"]["capabilities"]=caps;
    assert!(info(&serde_json::to_vec(&seed_loot).unwrap()).is_ok());
    for field in ["roomSemantics","compatibility"]{let mut bad=seed_loot.clone();let name=if field=="roomSemantics"{"requiredCapabilities"}else{"capabilities"};bad[field][name].as_array_mut().unwrap().pop();assert!(info(&serde_json::to_vec(&bad).unwrap()).is_err());}
}


#[cfg(test)]#[test]fn testing_loadout_public_info_extension_is_exact_and_legacy_stays_strict(){
    let runtime:Value=serde_json::from_str("{}").unwrap();
    let generated=serde_json::json!({"major":1,"revision":REVISION,"serviceId":runtime["serviceId"],"serviceEpoch":"public-info-fixture","compatibility":runtime["compatibility"],"capabilities":runtime["compatibility"]["capabilities"],"profile":runtime["buildRegistry"],"roomSemantics":{"revision":"g3-room-semantics-1","kind":"core-world-rules-hourglass-items-gold-social-controls-seed-loot","requiredCapabilities":C2_CAPABILITIES},"developmentFeatures":{"testingStartingLoadout":{"available":true,"settingKey":"NT_testing_mode","default":false}}});
    let bytes=std::env::var("NT2_TESTING_INFO_FIXTURE").map(|p|std::fs::read(p).unwrap()).unwrap_or_else(|_|serde_json::to_vec(&generated).unwrap());
    let base=serde_json::from_slice::<Value>(&bytes).unwrap();assert!(info(&bytes).is_ok(),"Actual public Info must be consumed before authentication");
    let mut original_strict=base.clone();original_strict.as_object_mut().unwrap().remove("roomSemantics");assert!(typed::<wire::Info>("Info",&serde_json::to_vec(&original_strict).unwrap()).is_err(),"Reproduce original legacy codec rejection of developmentFeatures");
    let mut legacy=base.clone();legacy.as_object_mut().unwrap().remove("developmentFeatures");assert!(info(&serde_json::to_vec(&legacy).unwrap()).is_ok());
    for fault in 0..12{
        let mut bad=base.clone();match fault{
            0=>bad["developmentFeatures"]["unknownFeature"]=serde_json::json!({}),
            1=>bad["developmentFeatures"]["testingStartingLoadout"]["extra"]=true.into(),
            2=>bad["developmentFeatures"]["testingStartingLoadout"]["available"]=false.into(),
            3=>bad["developmentFeatures"]["testingStartingLoadout"]["default"]=true.into(),
            4=>bad["developmentFeatures"]["testingStartingLoadout"]["default"]="false".into(),
            5=>bad["developmentFeatures"]["testingStartingLoadout"]["settingKey"]="other".into(),
            6=>bad["compatibility"]["settingKeys"].as_array_mut().unwrap().retain(|v|v!="NT_testing_mode"),
            7=>bad["compatibility"]["settingKeys"].as_array_mut().unwrap().push("NT_testing_mode".into()),
            8=>bad["unexpectedInfoField"]=true.into(),
            9=>bad["roomSemantics"]["requiredCapabilities"].as_array_mut().unwrap().pop().map(|_|()).unwrap(),
            10=>bad["major"]="2".into(),
            _=>bad["developmentFeatures"]=Value::Null,
        }
        assert!(info(&serde_json::to_vec(&bad).unwrap()).is_err(),"Malformed Info variant {fault}");
    }
    let raw=serde_json::to_string(&base).unwrap();let duplicate=raw.replacen(r#""available":true"#,r#""available":true,"available":true"#,1);assert_ne!(raw,duplicate);assert!(info(duplicate.as_bytes()).is_err());
    let mut trailing=raw.into_bytes();trailing.extend_from_slice(b"{}");assert!(info(&trailing).is_err());assert!(info(&vec![b' ';65537]).is_err());
}
