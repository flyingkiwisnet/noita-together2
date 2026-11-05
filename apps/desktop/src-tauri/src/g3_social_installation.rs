//! Production social enablement requires a separately published, pinned server approval.
//! No supplemental approval exists yet; callers cannot supply their own approval bytes.
use serde_json::Value;
use sha2::{Digest,Sha256};
use std::path::Path;
const COMPILED_APPROVAL:Option<(&[u8],&str)>=None;
pub const GOLD_REQUIRED:&[&str]=&["gold_local.lua","gold_wallet.lua","bank_json.lua","bank_native.lua","bank_ui.lua","bank_mail.lua","bank_glow.lua","bank_outlet.xml","bank_mail.png","bank_outer.png","ui/bank.png","ui/fridge.png"];
pub const ITEM_REQUIRED:&[&str]=&["item_bridge.lua","item_local.lua","item_chunks.lua","item_descriptor.lua","item_construct.lua","item_game_api.lua","item_catalogue.lua","item_catalogue_data.lua","item_presenter.lua","item_bank_model.lua","item_inlet.xml","item_inlet_glow.lua","item_wand.xml","item_wand_sprite.xml","item_potion.xml","item_powder.xml"];
pub const C1_REQUIRED:&[&str]=&["mods/noita_together2/files/c1/append/checkpoint_mail.lua","mods/noita_together2/files/c1/append/fungal_shift.lua","mods/noita_together2/files/c1/append/heart_append.lua","mods/noita_together2/files/c1/append/heart_better_append.lua","mods/noita_together2/files/c1/append/orb_init.lua","mods/noita_together2/files/c1/append/orb_pickup.lua","mods/noita_together2/files/c1/append/perk.lua","mods/noita_together2/files/c1/append/perk_pickup.lua","mods/noita_together2/files/c1/append/status_effects/hearty_end.lua","mods/noita_together2/files/c1/append/status_effects/hearty_start.lua","mods/noita_together2/files/c1/entities/death_cam.xml","mods/noita_together2/files/c1/entities/forced_orb.xml","mods/noita_together2/files/c1/entities/perks/banned_perk.xml","mods/noita_together2/files/c1/items_gfx/banned_perks/glass_cannon.png","mods/noita_together2/files/c1/register.lua","mods/noita_together2/files/c1/scripts/ban_perks.lua","mods/noita_together2/files/c1/scripts/ban_perks_events.lua","mods/noita_together2/files/c1/scripts/bannable_perk_check.lua","mods/noita_together2/files/c1/scripts/checkpoint.lua","mods/noita_together2/files/c1/scripts/entity_translations.lua","mods/noita_together2/files/c1/scripts/fungal_receive.lua","mods/noita_together2/files/c1/scripts/orb_killer.lua","mods/noita_together2/files/c1/translations.csv","mods/noita_together2/files/g3/c1_gameplay.lua","mods/noita_together2/files/g3/c1_content.lua","mods/noita_together2/files/g3/c1_effects.lua","mods/noita_together2/files/g3/c1_local.lua","mods/noita_together2/files/g3/c1_hook_api.lua","mods/noita_together2/files/g3/c1_catalogue_data.lua"];
pub const C2_REQUIRED:&[&str]=&["mods/noita_together2/files/c2/append/boss_arena.lua","mods/noita_together2/files/c2/append/sampo_append.lua","mods/noita_together2/files/c2/append/snowcastle_cavern.lua","mods/noita_together2/files/c2/effects/hourglass_hastium.xml","mods/noita_together2/files/c2/effects/weak_ambrosia.xml","mods/noita_together2/files/c2/effects/weak_ambrosia_end.lua","mods/noita_together2/files/c2/effects/weak_ambrosia_start.lua","mods/noita_together2/files/c2/entities/hourglass_effect_trigger.xml","mods/noita_together2/files/c2/entities/sampo/disabled_sampo.png","mods/noita_together2/files/c2/entities/sampo/disabled_sampo.xml","mods/noita_together2/files/c2/entities/sampo/disabled_sampo_sprite.xml","mods/noita_together2/files/c2/entities/sampo/platform.xml","mods/noita_together2/files/c2/entities/sampo/sampo_aura.xml","mods/noita_together2/files/c2/hook_api.lua","mods/noita_together2/files/c2/hourglass.lua","mods/noita_together2/files/c2/local.lua","mods/noita_together2/files/c2/register.lua","mods/noita_together2/files/c2/scripts/hourglass_events.lua","mods/noita_together2/files/c2/scripts/sampo/sampo_aura.lua","mods/noita_together2/files/c2/scripts/sampo/sampo_aura_attractor_fix.lua","mods/noita_together2/files/c2/storage.lua","mods/noita_together2/files/c2/translations.csv","mods/noita_together2/files/c2/ui_gfx/status_indicators/nt_hourglass_protection_half.png","mods/noita_together2/files/c2/ui_gfx/status_indicators/nt_hourglass_speed_levitation.png","mods/noita_together2/files/c2/world.lua"];
const D1_REQUIRED:&[(&str,&[u8])]=&[
 ("mods/noita_together2/files/g3/d1_hook_api.lua",include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../mod/noita_together2/files/g3/d1_hook_api.lua"))),
 ("mods/noita_together2/files/g3/d1_local.lua",include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../mod/noita_together2/files/g3/d1_local.lua"))),
 ("mods/noita_together2/files/c1/append/steve_death.lua",include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../mod/noita_together2/files/c1/append/steve_death.lua"))),
];
const ITEM_ART:&[&str]=&["ui/item_bank/all.png","ui/item_bank/wands.png","ui/item_bank/spells.png","ui/item_bank/items.png","ui/item_bank/slot.png","ui/item_bank/back.png","ui/item_bank/back10.png","ui/item_bank/forward.png","ui/item_bank/forward10.png","ui/item_bank/icon_gun_shuffle-1.png","ui/item_bank/icon_gun_actions_per_round-1.png","ui/item_bank/icon_fire_rate_wait-1.png","ui/item_bank/icon_reload_time-1.png","ui/item_bank/icon_mana_max-1.png","ui/item_bank/icon_mana_charge_speed-1.png","ui/item_bank/icon_gun_capacity-1.png","ui/item_bank/icon_spread_degrees-1.png"];
/// Validate the declared B1 closure before reading/installing any runtime file.
/// Both capability paths must agree; checking only listed hashes cannot detect
/// a production dofile dependency which was omitted from the list itself.
const RACE_REQUIRED:&[(&str,&[u8])]=&[
 ("mods/noita_together2/files/race/append/boss.lua",include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../mod/noita_together2/files/race/append/boss.lua")).as_slice()),
 ("mods/noita_together2/files/race/append/orb.lua",include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../mod/noita_together2/files/race/append/orb.lua")).as_slice()),
 ("mods/noita_together2/files/race/catalogue.lua",include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../mod/noita_together2/files/race/catalogue.lua")).as_slice()),
 ("mods/noita_together2/files/race/hook_api.lua",include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../mod/noita_together2/files/race/hook_api.lua")).as_slice()),
 ("mods/noita_together2/files/race/local.lua",include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../mod/noita_together2/files/race/local.lua")).as_slice()),
 ("mods/noita_together2/files/race/panel.lua",include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../mod/noita_together2/files/race/panel.lua")).as_slice()),
 ("mods/noita_together2/files/race/player_death.lua",include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../mod/noita_together2/files/race/player_death.lua")).as_slice()),
 ("mods/noita_together2/files/race/register.lua",include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../mod/noita_together2/files/race/register.lua")).as_slice()),
 ("mods/noita_together2/files/race/sample.lua",include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../mod/noita_together2/files/race/sample.lua")).as_slice()),
 ("mods/noita_together2/files/race/sha256.lua",include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../mod/noita_together2/files/race/sha256.lua")).as_slice()),
 ("mods/noita_together2/files/race/storage.lua",include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../mod/noita_together2/files/race/storage.lua")).as_slice()),
];
pub fn verify_gold_closure(approval:&Value)->Result<(),String>{
 let has_gold=|caps:&Value|caps.as_array().is_some_and(|v|v.iter().any(|c|c=="core.gold"));
 let profile=&approval["profile"]["capabilities"];let compatibility=&approval["compatibility"]["capabilities"];
 if approval.get("b1Fixture").is_none()&&!has_gold(profile)&&!has_gold(compatibility){return Ok(())}
 let item=profile.as_array().is_some_and(|v|v.iter().any(|c|c=="core.items"))||compatibility.as_array().is_some_and(|v|v.iter().any(|c|c=="core.items"));
 let c2=profile.as_array().is_some_and(|v|v.iter().any(|c|c=="core.world-rules"||c=="core.hourglass"))||approval.get("c2Fixture").is_some();
 let race=profile.as_array().is_some_and(|v|v.iter().any(|v|v=="race.goals.v1"));
 let expected=if race{crate::protocol_g2_next::RACE_CAPABILITIES}else if c2{crate::protocol_g2_next::C2_CAPABILITIES}else if item{crate::protocol_g2_next::ITEM_CAPABILITIES}else{crate::protocol_g2_next::GOLD_CAPABILITIES};
 let exact=|caps:&Value|caps.as_array().is_some_and(|v|v.len()==expected.len()&&expected.iter().all(|cap|v.iter().filter(|v|v.as_str()==Some(cap)).count()==1));
 if !exact(profile)||!exact(compatibility){return Err("B1 profile and compatibility require the exact gold capabilities".into())}
 if race{
  if approval["fixtureOnly"]==true{if approval["raceFixture"]["revision"]!="race-first-release-private-1"||approval["raceFixture"]["installed"]!=false{return Err("Race fixture selector differs".into())}}else if approval["fixtureOnly"]!=false||approval["id"]!="nt2-g3-testing-loadout-dev-20261030"||approval.get("raceFixture").is_some(){return Err("Race requires the reviewed normal testing runtime".into())}
  if approval["fixtureOnly"]==false&&approval["compatibility"]["modes"]!=serde_json::json!(["core","race"]){return Err("Race normal modes differ from Core and Race".into())}
  for (path,bytes) in RACE_REQUIRED{
   let sha=format!("{:x}",Sha256::digest(bytes));let source=path.replacen("mods/","mod/",1);
   for key in ["runtimeMappings","requiredLocalFiles"]{if approval[key].as_array().is_none_or(|files|files.iter().filter(|f|f["path"]==*path&&f["sha256"]==sha&&(key!="runtimeMappings"||f["sourcePath"]==source)).count()!=1){return Err("Race dependency missing, aliased or changed".into())}}
  }
 }
 if c2{
  if approval["fixtureOnly"]==true{
   if approval["fixtureNative"]["private"]!=true||approval["c2Fixture"]!=serde_json::json!({"revision":"c2-world-private-1","installed":false,"injectedEngine":true,"serviceMocked":false,"normalProfilesChanged":false})||approval["c1Fixture"]["revision"]!="c1-player-private-1"{return Err("C2 requires the exact private uninstalled selector".into())}
  }else{
   let count=match approval["id"].as_str(){Some("nt2-g3-c2-world-dev-20261030")=>306,Some("nt2-g3-testing-loadout-dev-20261030")=>if race{324}else{313},_=>0};
   if count==0||approval["fixtureOnly"]!=false||approval["runtimeMappings"].as_array().is_none_or(|v|v.len()!=count)||approval["requiredLocalFiles"].as_array().is_none_or(|v|v.len()!=count)||["itemFixture","b1Fixture","a2Fixture","a3Fixture","c1Fixture","c2Fixture","raceFixture","fixtureNative"].iter().any(|key|approval.get(*key).is_some()){return Err("C2 requires an exact normal runtime without fixture selectors".into())}
   if matches!(count,313|324){
    for (path,bytes) in [("mods/noita_together2/files/startup/testing_loadout.lua",include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../mod/noita_together2/files/startup/testing_loadout.lua")).as_slice()),("mods/noita_together2/files/startup/testing-starting-loadout.json",include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../packages/contracts/g3/testing-starting-loadout.json")).as_slice()),("mods/noita_together2/files/rejoin/lobby_status.lua",include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../mod/noita_together2/files/rejoin/lobby_status.lua")).as_slice()),("mods/noita_together2/files/g3/ghost_labels.lua",include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../mod/noita_together2/files/g3/ghost_labels.lua")).as_slice())]{
     let sha=format!("{:x}",Sha256::digest(bytes));for key in ["runtimeMappings","requiredLocalFiles"]{if approval[key].as_array().is_none_or(|v|v.iter().filter(|v|v["path"]==path&&v["sha256"]==sha).count()!=1){return Err("Testing loadout dependency missing or changed".into())}}
    }
   }
  }
  if approval["fixtureOnly"]==false&&approval["id"]=="nt2-g3-testing-loadout-dev-20261030"{
   for (path,bytes) in D1_REQUIRED{
    let sha=format!("{:x}",Sha256::digest(bytes));
    let source=path.replacen("mods/","mod/",1);
    for key in ["runtimeMappings","requiredLocalFiles"]{if approval[key].as_array().is_none_or(|files|files.iter().filter(|f|f["path"]==*path&&f["sha256"]==sha&&(key!="runtimeMappings"||f["sourcePath"]==source)).count()!=1){return Err("D1 dependency missing, aliased or changed".into())}}
   }
  }
  let digest=Sha256::digest(include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../packages/contracts/g3-hourglass/catalogue.json"))).iter().map(|b|format!("{b:02x}")).collect::<String>();
  if approval["hourglassCatalogue"]["revision"]!="nt2-hourglass-builtins-1"||approval["hourglassCatalogue"]["sha256"]!=digest{return Err("C2 catalogue pin differs".into())}
  for path in C2_REQUIRED{let mappings=approval["runtimeMappings"].as_array().ok_or("C2 mappings")?;let files=approval["requiredLocalFiles"].as_array().ok_or("C2 closure")?;let mapped:Vec<_>=mappings.iter().filter(|m|m["path"]==*path).collect();let required:Vec<_>=files.iter().filter(|m|m["path"]==*path).collect();if mapped.len()!=1||required.len()!=1||mapped[0]["sha256"]!=required[0]["sha256"]{return Err(format!("Missing C2 dependency: {path}"))}}
 }
 let mappings=approval["runtimeMappings"].as_array().ok_or("Missing B1 runtime mappings")?;
 let files=approval["requiredLocalFiles"].as_array().ok_or("Missing B1 required files")?;
 for suffix in GOLD_REQUIRED.iter().chain(if item{ITEM_REQUIRED.iter()}else{[].iter()}).chain(if item{ITEM_ART.iter()}else{[].iter()}){
  let path=format!("mods/noita_together2/files/g3/{suffix}");
  let mapped:Vec<_>=mappings.iter().filter(|m|m["path"]==path).collect();let required:Vec<_>=files.iter().filter(|m|m["path"]==path).collect();
  if mapped.len()!=1||required.len()!=1||mapped[0]["sha256"]!=required[0]["sha256"]{return Err(format!("Missing/inconsistent B1 dependency: {suffix}"))}
 }
 if item{
  let catalogue:Value=serde_json::from_str(include_str!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../packages/contracts/g3-items/catalogue.json"))).map_err(|_|"Item catalogue unavailable")?;
  let catalogue_sha=Sha256::digest(include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../packages/contracts/g3-items/catalogue.json"))).iter().map(|b|format!("{b:02x}")).collect::<String>();
  if approval["fixtureOnly"]==true{
   if approval["itemFixture"]["revision"]!="items-private-1"||approval["fixtureNative"]["private"]!=true{return Err("Private item runtime classification differs".into())}
  }else{
   if approval.get("fixtureOnly").is_some_and(|v|v.as_bool()!=Some(false))||["itemFixture","b1Fixture","a2Fixture","a3Fixture","c1Fixture","fixtureNative"].iter().any(|key|approval.get(*key).is_some())||approval["sourceApproved"]!=true||approval["developmentOnly"]!=true||approval["installApproved"]!=false||approval["nativeArtifacts"].as_array().is_none_or(|v|v.len()!=3||v.iter().any(|a|a["private"]!=false||a["testFlags"]!=false)){return Err("Normal item runtime contains private/test selectors or unapproved native images".into())}
  }
  if approval["itemCatalogue"]["revision"]!=catalogue["revision"]||approval["itemCatalogue"]["sha256"]!=catalogue_sha{return Err("Item runtime catalogue differs from canonical source".into())}
  let expected=Sha256::digest(include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../packages/contracts/g3-items/catalogue.lua"))).iter().map(|b|format!("{b:02x}")).collect::<String>();
  if !mappings.iter().any(|m|m["path"]=="mods/noita_together2/files/g3/item_catalogue_data.lua"&&m["sha256"]==expected){return Err("Item runtime catalogue bytes differ".into())}
 }
 if mappings.iter().any(|m|m["path"]=="mods/noita_together2/files/c1/register.lua")||approval.get("c1Catalogue").is_some(){
  for path in C1_REQUIRED{let mapped:Vec<_>=mappings.iter().filter(|m|m["path"]==*path).collect();let required:Vec<_>=files.iter().filter(|m|m["path"]==*path).collect();if mapped.len()!=1||required.len()!=1||mapped[0]["sha256"]!=required[0]["sha256"]{return Err(format!("Missing C1 dependency: {path}"))}}
  let c:Value=serde_json::from_str(include_str!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../packages/contracts/g3-c1/catalogue.json"))).map_err(|_|"C1 catalogue")?;
  let digest=Sha256::digest(include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../packages/contracts/g3-c1/catalogue.json"))).iter().map(|b|format!("{b:02x}")).collect::<String>();
  let lua=Sha256::digest(include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../packages/contracts/g3-c1/catalogue.lua"))).iter().map(|b|format!("{b:02x}")).collect::<String>();
  if approval["c1Catalogue"]["revision"]!=c["revision"]||approval["c1Catalogue"]["sha256"]!=digest||!mappings.iter().any(|m|m["path"]=="mods/noita_together2/files/g3/c1_catalogue_data.lua"&&m["sha256"]==lua){return Err("C1 runtime catalogue differs from canonical source".into())}
 }
 Ok(())
}
#[cfg(not(any(feature="g3-c2-live",feature="g3-a1-runtime-fix",feature="g3-a1-runtime-diagnostic",feature="g3-a1-heartbeat-busy")))]
const DEVELOPMENT_BYTES:&[u8]=b"{}";
#[cfg(not(any(feature="g3-c2-live",feature="g3-a1-runtime-fix",feature="g3-a1-runtime-diagnostic",feature="g3-a1-heartbeat-busy")))]
pub const DEVELOPMENT_SHA:&str="b0ce032108612ff1d2540008ae9a96f746748243857d5d6efaa8537b7be934c0";
#[cfg(not(any(feature="g3-c2-live",feature="g3-a1-runtime-fix",feature="g3-a1-runtime-diagnostic",feature="g3-a1-heartbeat-busy")))]
pub const DEVELOPMENT_ID:&str="nt2-g3-native-social-development-2";
#[cfg(all(feature="g3-a1-runtime-fix",not(any(feature="g3-c2-live",feature="g3-a1-runtime-diagnostic",feature="g3-a1-heartbeat-busy"))))]
const DEVELOPMENT_BYTES:&[u8]=b"{}";
#[cfg(all(feature="g3-a1-runtime-fix",not(any(feature="g3-c2-live",feature="g3-a1-runtime-diagnostic",feature="g3-a1-heartbeat-busy"))))]
pub const DEVELOPMENT_SHA:&str="fe726f360170d2d738b91a5c95d0f787a5df87f0759b368d4c8d93dd193f0077";
#[cfg(all(feature="g3-a1-runtime-fix",not(any(feature="g3-c2-live",feature="g3-a1-runtime-diagnostic",feature="g3-a1-heartbeat-busy"))))]
pub const DEVELOPMENT_ID:&str="nt2-g3-native-social-development-3";
#[cfg(all(feature="g3-a1-runtime-diagnostic",not(any(feature="g3-c2-live",feature="g3-a1-heartbeat-busy"))))]
const DEVELOPMENT_BYTES:&[u8]=b"{}";
#[cfg(all(feature="g3-a1-runtime-diagnostic",not(any(feature="g3-c2-live",feature="g3-a1-heartbeat-busy"))))]
pub const DEVELOPMENT_SHA:&str="be6fec66dae95a141b09bceff5f3b22565ca7647958495507bcd441f014dde0b";
#[cfg(all(feature="g3-a1-runtime-diagnostic",not(any(feature="g3-c2-live",feature="g3-a1-heartbeat-busy"))))]
pub const DEVELOPMENT_ID:&str="nt2-g3-native-social-development-4";
#[cfg(all(feature="g3-a1-heartbeat-busy",not(feature="g3-c2-live")))]
const DEVELOPMENT_BYTES:&[u8]=b"{}";
#[cfg(all(feature="g3-a1-heartbeat-busy",not(feature="g3-c2-live")))]
pub const DEVELOPMENT_SHA:&str="758b597a0deace517b4891fe1c6460ed55696e736aa0c03123bc5f1139595217";
#[cfg(all(feature="g3-a1-heartbeat-busy",not(feature="g3-c2-live")))]
pub const DEVELOPMENT_ID:&str="nt2-g3-native-social-development-5";
#[cfg(all(feature="g3-c2-live",not(any(feature="g3-c2-potion-diagnostic",feature="g3-c2-potion-correction",feature="g3-seed-loot"))))]
const DEVELOPMENT_BYTES:&[u8]=b"{}";
#[cfg(all(feature="g3-c2-live",not(any(feature="g3-c2-potion-diagnostic",feature="g3-c2-potion-correction",feature="g3-seed-loot"))))]
pub const DEVELOPMENT_SHA:&str="ccd0d38d806e04f3e02df840cc524e958ca288e19f5de2473330912bd363accb";
#[cfg(all(feature="g3-c2-live",not(any(feature="g3-c2-potion-diagnostic",feature="g3-c2-potion-correction",feature="g3-seed-loot"))))]
pub const DEVELOPMENT_ID:&str="nt2-g3-native-c2-development-2";
#[cfg(feature="g3-c2-potion-diagnostic")]
include!("g3_potion_diagnostic_approval.rs");
#[cfg(feature="g3-c2-potion-correction")]
include!("g3_potion_correction_approval.rs");
#[cfg(all(feature="g3-c2-potion-diagnostic",feature="g3-c2-potion-correction"))]
compile_error!("Select one separate potion candidate");
#[cfg(all(feature="g3-seed-loot",not(feature="g3-loot01")))]
include!("g3_seed_loot_approval.rs");
#[cfg(all(feature="g3-loot01",not(feature="g3-rejoin")))]
include!("g3_loot01_approval.rs");
#[cfg(feature="g3-rejoin")]
const DEVELOPMENT_BYTES:&[u8]=include_bytes!(env!("NT2_REJOIN_RUNTIME"));
#[cfg(feature="g3-rejoin")]
pub const DEVELOPMENT_SHA:&str=env!("NT2_REJOIN_RUNTIME_SHA");
#[cfg(feature="g3-rejoin")]
pub const DEVELOPMENT_ID:&str=env!("NT2_REJOIN_RUNTIME_ID");
#[cfg(all(feature="g3-seed-loot",any(feature="g3-c2-potion-correction",feature="g3-c2-potion-diagnostic")))]
compile_error!("Select one separate seed/loot or potion candidate");
pub const SERVICE_ID:&str=if cfg!(any(feature="g3-c2-potion-correction",feature="g3-seed-loot")){"nt2-g3-c2-potion-correction"}else if cfg!(feature="g3-c2-potion-diagnostic"){"nt2-g3-c2-potion-diagnostic"}else if cfg!(feature="g3-c2-live"){"nt2-g3-native-c2-development"}else{"nt2-g3-native-social-development-a1"};
const NAMESPACE:&str=if cfg!(feature="g3-rejoin"){"g3_rejoin_"}else if cfg!(any(feature="g3-c2-potion-correction",feature="g3-seed-loot")){"g3_c2_potion_correction_"}else if cfg!(feature="g3-c2-potion-diagnostic"){"g3_c2_potion_"}else if cfg!(feature="g3-c2-live"){"g3_c2_"}else{"g3_social_a1_"};
const PIPE:&str=if cfg!(feature="g3-c2-live"){"nt2-g3-c2-dev-v1-"}else{"nt2-g3-social-dev-v1-"};
pub const MOD_VERSION:&str=if cfg!(feature="g3-c2-live"){"nt2-g3-c2-dev-1"}else{"nt2-g3-social-dev-1"};
pub const BRIDGE_VERSION:&str=if cfg!(feature="g3-c2-live"){"g3-c2-ipc-1"}else{"g3-social-transport-1"};
#[cfg(not(feature="g3-rejoin"))]
const FILE_COUNT:usize=if cfg!(feature="g3-loot01"){35}else if cfg!(feature="g3-seed-loot"){34}else if cfg!(feature="g3-c2-live"){22}else{20};
fn file_count()->usize{
 #[cfg(feature="g3-rejoin")]{serde_json::from_slice::<Value>(DEVELOPMENT_BYTES).ok().and_then(|a|a["requiredLocalFiles"].as_array().map(Vec::len)).unwrap_or(0)}
 #[cfg(not(feature="g3-rejoin"))]{FILE_COUNT}
}
fn gold_development_capability(approval:&Value)->bool{
 approval["compatibility"]["capabilities"].as_array().is_some_and(|v|v.iter().any(|c|c=="core.gold"))&&verify_gold_closure(approval).is_ok()
}
fn development_capabilities()->Value{let mut caps=vec!["core.seed","core.ghost","core.hud","core.poi"];if cfg!(feature="g3-seed-loot"){caps.extend(["core.seed-policy.v1","core.randomize-loot.v1"]);}
 if cfg!(feature="g3-rejoin")&&development_profile().ok().is_some_and(|p|p["compatibility"]["capabilities"].as_array().is_some_and(|v|v.iter().any(|v|v=="core.emotes"))){caps.extend(["core.emotes","core.cosmetics"]);}
 // The compiled pinned declaration and complete closure approve this feature
 // for both normal and private builds. A fixture marker is not a ship selector.
 if cfg!(feature="g3-rejoin")&&development_profile().ok().is_some_and(|p|gold_development_capability(&p)){caps.push("core.gold");}
 if cfg!(feature="g3-rejoin")&&development_profile().ok().is_some_and(|p|p["profile"]["capabilities"].as_array().is_some_and(|v|v.iter().any(|c|c=="core.items"))&&verify_gold_closure(&p).is_ok()){caps.push("core.items");}
 if cfg!(feature="g3-rejoin")&&development_profile().ok().is_some_and(|p|p["profile"]["capabilities"].as_array().is_some_and(|v|v.iter().any(|c|c=="core.world-rules"))&&verify_gold_closure(&p).is_ok()){caps.extend(["core.world-rules","core.hourglass"]);}
 if cfg!(feature="g3-rejoin")&&development_profile().ok().is_some_and(|p|p["profile"]["capabilities"].as_array().is_some_and(|v|v.iter().any(|c|c=="race.goals.v1"))&&verify_gold_closure(&p).is_ok()){caps.push("race.goals.v1");}
 serde_json::json!(caps)}
pub fn development_profile()->Result<Value,String>{let sha=Sha256::digest(DEVELOPMENT_BYTES).iter().map(|b|format!("{b:02x}")).collect::<String>();if sha!=DEVELOPMENT_SHA{return Err("Pinned external development runtime drift".into());}serde_json::from_slice(DEVELOPMENT_BYTES).map_err(|_|"Invalid development runtime".into())}
/// Explicit SOURCE development policy, never an installation/production approval.
pub fn verify_development(root:&Path,metadata:&Value)->Result<Verified,String>{
 verify_gold_closure(&development_profile()?)?;
 let approval=development_profile()?;if cfg!(feature="g3-c2-potion-correction") && (approval["diagnosticOnly"]!=false||approval["correctionCandidate"]!=true||approval["gameplaySemanticsChanged"]!=true){return Err("Correction candidate semantics refused".into());}if approval["id"]!=DEVELOPMENT_ID||approval["serviceId"]!=SERVICE_ID||approval["namespacePrefix"]!=NAMESPACE||approval["compatibility"]["capabilities"]!=development_capabilities()||approval["sourceApproved"]!=true||approval["developmentOnly"]!=true||approval["installApproved"]!=false||approval["packaged"]!=false||approval["actualNoitaEvidence"]!=false||approval["pipePrefix"]!=PIPE||approval["requiredLocalFiles"].as_array().is_none_or(|f|f.len()!=file_count()){return Err("External source-only development policy refused".into());}
 let read=|path:&Path|{
  #[cfg(any(feature="g3-c2-potion-correction",feature="g3-seed-loot"))]{crate::g3_local_testing::hash(root,path)}
  #[cfg(not(any(feature="g3-c2-potion-correction",feature="g3-seed-loot")))]{crate::g3_installation::hash(path)}
 };
 verify_policy(root,metadata,&approval,read,false)?;Ok(Verified{root:root.to_owned(),metadata:metadata.clone()})
}
#[cfg(not(feature="g3-c2-live"))]
const REQUIRED:&[&str]=&["init.lua","files/nt2_g3_social.dll","files/g3/social_native.lua","files/g3/social_dispatch.lua","files/g3/hud_poi_local.lua","files/g3/social_game_api.lua","files/g3/validation.lua","files/inspection.lua","files/g3/native_owner.lua","files/g3/movement.lua","files/g3/ghosts.lua","files/g3/game_api.lua","files/g3/poi.xml","files/g3/ui/hpbar_full.png","files/g3/ui/hpbar_empty.png","files/g3/ui/sampo.png","files/logging.xml","files/g3/ghost.xml","files/g3/poi_pickup.lua","files/g3/ui/poi.png"];
#[cfg(all(feature="g3-c2-live",not(feature="g3-seed-loot")))]
const REQUIRED:&[&str]=&["init.lua","files/nt2_g3_social.dll","files/g3/social_native.lua","files/g3/social_dispatch.lua","files/g3/hud_poi_local.lua","files/g3/social_game_api.lua","files/g3/validation.lua","files/inspection.lua","files/g3/native_owner.lua","files/g3/movement.lua","files/g3/ghosts.lua","files/g3/game_api.lua","files/g3/poi.xml","files/g3/ui/hpbar_full.png","files/g3/ui/hpbar_empty.png","files/g3/ui/sampo.png","files/logging.xml","files/g3/ghost.xml","files/g3/poi_pickup.lua","files/g3/ui/poi.png","files/nt2_g3_ipc.dll","files/nt2_g3_transport.dll"];
#[cfg(all(feature="g3-seed-loot",not(feature="g3-loot01")))]
const REQUIRED:&[&str]=&["init.lua","files/nt2_g3_social.dll","files/g3/social_native.lua","files/g3/social_dispatch.lua","files/g3/hud_poi_local.lua","files/g3/social_game_api.lua","files/g3/validation.lua","files/inspection.lua","files/g3/native_owner.lua","files/g3/movement.lua","files/g3/ghosts.lua","files/g3/game_api.lua","files/g3/poi.xml","files/g3/ui/hpbar_full.png","files/g3/ui/hpbar_empty.png","files/g3/ui/sampo.png","files/logging.xml","files/g3/ghost.xml","files/g3/poi_pickup.lua","files/g3/ui/poi.png","files/nt2_g3_ipc.dll","files/nt2_g3_transport.dll","files/loot/biome_scripts.lua","files/loot/butcher_randomseed.lua","files/loot/chest_random.lua","files/loot/chest_random_super.lua","files/loot/co_op_mail.lua","files/loot/generate_shop_item.lua","files/loot/gun_procedural.lua","files/loot/LICENSE","files/loot/perk_reroll.lua","files/loot/policy.lua","files/loot/preset_wands_random.lua","files/loot/register.lua"];
#[cfg(feature="g3-loot01")]
const REQUIRED:&[&str]=&["init.lua","files/nt2_g3_social.dll","files/g3/social_native.lua","files/g3/social_dispatch.lua","files/g3/hud_poi_local.lua","files/g3/social_game_api.lua","files/g3/validation.lua","files/inspection.lua","files/g3/native_owner.lua","files/g3/movement.lua","files/g3/ghosts.lua","files/g3/game_api.lua","files/g3/poi.xml","files/g3/ui/hpbar_full.png","files/g3/ui/hpbar_empty.png","files/g3/ui/sampo.png","files/logging.xml","files/g3/ghost.xml","files/g3/poi_pickup.lua","files/g3/ui/poi.png","files/nt2_g3_ipc.dll","files/nt2_g3_transport.dll","files/loot/biome_scripts.lua","files/loot/butcher_randomseed.lua","files/loot/chest_random.lua","files/loot/chest_random_super.lua","files/loot/co_op_mail.lua","files/loot/generate_shop_item.lua","files/loot/gun_procedural.lua","files/loot/LICENSE","files/loot/perk_reroll.lua","files/loot/policy.lua","files/loot/preset_wands_random.lua","files/loot/register.lua","files/loot/run-policy.txt"];
pub struct Verified{root:std::path::PathBuf,metadata:Value}
impl Verified{pub fn matches(&self,root:&Path,metadata:&Value)->bool{self.root==root&&self.metadata==*metadata}}
pub fn verify(root:&Path,metadata:&Value)->Result<Verified,String>{
 let (bytes,pin)=COMPILED_APPROVAL.ok_or("Supplemental social runtime approval is not published/pinned; enablement refused")?;
 let sha=Sha256::digest(bytes).iter().map(|b|format!("{b:02x}")).collect::<String>();if sha!=pin{return Err("Compiled social approval drift".into());}
 let approval:Value=serde_json::from_slice(bytes).map_err(|_|"Invalid compiled social approval")?;
 verify_using(root,metadata,&approval,crate::g3_installation::hash)?;
 Ok(Verified{root:root.to_owned(),metadata:metadata.clone()})
}
fn verify_using(root:&Path,metadata:&Value,approval:&Value,read:impl Fn(&Path)->Result<String,String>)->Result<(),String>{
 verify_policy(root,metadata,approval,read,true)
}
fn verify_policy(root:&Path,metadata:&Value,approval:&Value,read:impl Fn(&Path)->Result<String,String>,installation:bool)->Result<(),String>{
 if approval["fixtureOnly"]!=true&&(approval.get("c1Fixture").is_some()||approval.get("c2Fixture").is_some()){return Err("Normal runtime contains C1 fixture selector".into())}
 if approval["installApproved"]!=installation||approval["profile"]!=*metadata||metadata["modVersion"]!=MOD_VERSION||metadata["bridgeVersion"]!=BRIDGE_VERSION{return Err("Unreviewed/wrong social runtime metadata".into());}
 let caps=metadata["capabilities"].as_array().ok_or("Missing social capabilities")?;for cap in ["core.seed","core.ghost","core.hud","core.poi"]{if !caps.iter().any(|v|v==cap){return Err("Social capability not approved".into());}}
 let a3=caps.iter().any(|v|v=="core.emotes"||v=="core.cosmetics");
 if a3&&(!caps.iter().any(|v|v=="core.emotes")||!caps.iter().any(|v|v=="core.cosmetics")){return Err("Incomplete A3 capabilities".into())}
 let builds=metadata["supportedBuilds"].as_array().ok_or("Missing exact original build pairs")?;let exe=read(&root.join("noita.exe"))?;let lua=read(&root.join("lua51.dll"))?;
 if !builds.iter().any(|p|p["gameExeSha256"]==exe&&p["originalLuaSha256"]==lua){return Err("Social exact original build pair refused".into());}
 let files=approval["requiredLocalFiles"].as_array().ok_or("Missing complete social runtime closure")?;
 let normal_c1=approval["fixtureOnly"]!=true&&files.len()==281&&file_count()==281&&approval["runtimeMappings"].as_array().is_some_and(|m|m.len()==281)&&approval["c1Catalogue"].is_object()
  &&caps.iter().any(|c|c=="core.gold")&&caps.iter().any(|c|c=="core.items")&&verify_gold_closure(approval).is_ok();
 let normal_c2=approval["fixtureOnly"]==false&&matches!(file_count(),306|313|324)&&caps.iter().any(|c|c=="core.world-rules")&&verify_gold_closure(approval).is_ok();
 let limit=if normal_c2{file_count()}else if approval["fixtureOnly"]==true&&approval["fixtureNative"]["private"]==true&&approval["c1Fixture"]["revision"]=="c1-player-private-1"&&approval["c1Fixture"]["installed"]==false{512}else if normal_c1{281}else{256};
 if files.len()<file_count()||files.len()>limit{return Err("Incomplete/oversize social closure".into());}
 if cfg!(feature="g3-c2-live"){
  if files.len()!=file_count()||approval["configurationDirectory"]!="g3-c2-dev"||approval["cargoFeature"]!="g3-c2-live"||approval["live"]["enabled"]!=true||approval["live"]["endpoint"]!="/v3/live"||approval["live"]["subprotocol"]!="nt2.g3.live.v1"{return Err("C2 live composition policy refused".into());}
  for(path,field)in [("mods/noita_together2/files/nt2_g3_transport.dll","bridgeSha256"),("mods/noita_together2/init.lua","modSha256")]{if files.iter().find(|p|p["path"]==path).is_none_or(|p|p["sha256"]!=metadata[field]){return Err("C2 primary/entry identity differs from complete closure".into());}}
 }
 let mut seen=std::collections::BTreeSet::new();for file in files{let path=file["path"].as_str().ok_or("Invalid social mapping")?;
  if path.split('/').any(|p|p==".nt2-local")||!path.starts_with("mods/noita_together2/")||path.contains('\\')||path.split('/').any(|p|p.is_empty()||p=="."||p==".."||p.contains(':'))||!seen.insert(path){return Err("Unsafe/duplicate social mapping".into());}
  let expected=file["sha256"].as_str().filter(|v|v.len()==64&&v.bytes().all(|b|b.is_ascii_hexdigit()&&(!b.is_ascii_alphabetic()||b.is_ascii_lowercase()))).ok_or("Invalid social SHA")?;
  if read(&root.join(path))?!=expected{return Err("Changed social runtime dependency".into());}
 }
 if cfg!(feature="g3-rejoin")&&!seen.contains("mods/noita_together2/files/rejoin/local_world.lua"){return Err("Missing current rejoin world helper".into());}
 if cfg!(feature="g3-rejoin") {for path in ["settings.lua","mod.xml","files/startup/bundle.lua","files/startup/entry_observer.lua","files/startup/settings_scope.lua","files/startup/scope_registry.lua","files/startup/scope_new.lua","files/startup/scope_restart.lua","files/startup/scope_runtime.lua","files/startup/scope_menu.lua","files/startup/scope_invalid.lua","files/startup/scope_spent.lua","files/startup/scope_trace.lua","files/startup/waiting_candidate.lua"]{if !seen.contains(format!("mods/noita_together2/{path}").as_str()){return Err("Missing current SE1 startup dependency".into())}}}
 if a3{for path in ["social_catalogue.lua","social_controls.lua","social_preferences.lua","social_keys.lua","social_local.lua","social_menu.lua","social_visual.lua","social_restore.lua","mnee_bindings.lua","emotes/holder.xml","emotes/holder_local.xml"]{if !seen.contains(format!("mods/noita_together2/files/g3/{path}").as_str()){return Err("Missing A3 presentation dependency".into())}}}
 verify_gold_closure(approval)?;
 for path in REQUIRED{if !seen.contains(format!("mods/noita_together2/{path}").as_str()){return Err("Required social runtime dependency omitted".into());}}
 Ok(())
}
#[cfg(test)]mod tests{use super::*;
 #[test]fn c2_private_closure_requires_exact_caps_catalogue_selector_and_all_dependencies(){
  let p=development_profile().unwrap();if p.get("c2Fixture").is_none(){return}assert!(verify_gold_closure(&p).is_ok());
  for path in C2_REQUIRED{let mut bad=p.clone();bad["requiredLocalFiles"].as_array_mut().unwrap().retain(|v|v["path"]!=*path);assert!(verify_gold_closure(&bad).is_err(),"{path}");}
  for cap in ["core.world-rules","core.hourglass","race.goals.v1"]{let mut bad=p.clone();bad["profile"]["capabilities"].as_array_mut().unwrap().retain(|v|v!=cap);assert!(verify_gold_closure(&bad).is_err());}
  let mut bad=p.clone();bad["profile"]["capabilities"].as_array_mut().unwrap().push("extra".into());assert!(verify_gold_closure(&bad).is_err());
  let mut bad=p.clone();bad["hourglassCatalogue"]["sha256"]="0".repeat(64).into();assert!(verify_gold_closure(&bad).is_err());let mut bad=p.clone();bad["fixtureOnly"]=false.into();assert!(verify_gold_closure(&bad).is_err());
 }

 #[cfg(feature="g3-rejoin")]
 #[test]fn c2_normal_306_closure_rejects_private_missing_catalogue_caps_and_oversize(){
  let mut normal=development_profile().unwrap();assert_eq!(normal["requiredLocalFiles"].as_array().unwrap().len(),306);
  for key in ["itemFixture","b1Fixture","a2Fixture","a3Fixture","c1Fixture","c2Fixture","raceFixture","fixtureNative"]{normal.as_object_mut().unwrap().remove(key);}
  normal["id"]="nt2-g3-c2-world-dev-20261030".into();normal["fixtureOnly"]=false.into();normal["sourceApproved"]=true.into();normal["developmentOnly"]=true.into();normal["installApproved"]=false.into();
  for a in normal["nativeArtifacts"].as_array_mut().unwrap(){a["private"]=false.into();a["testFlags"]=false.into();}
  let root=Path::new("E:/C2-read-only-policy-test");
  let verify=|a:&Value|verify_policy(root,&a["profile"],a,|p|{
   if p==root.join("noita.exe"){return Ok(a["profile"]["supportedBuilds"][0]["gameExeSha256"].as_str().unwrap().into())}
   if p==root.join("lua51.dll"){return Ok(a["profile"]["supportedBuilds"][0]["originalLuaSha256"].as_str().unwrap().into())}
   a["requiredLocalFiles"].as_array().unwrap().iter().find(|f|p==root.join(f["path"].as_str().unwrap())).and_then(|f|f["sha256"].as_str()).map(String::from).ok_or("Unexpected resource".into())
  },false);
  verify(&normal).unwrap();assert_eq!(development_capabilities(),serde_json::json!(crate::protocol_g2_next::RACE_CAPABILITIES));
  for path in C2_REQUIRED{let mut bad=normal.clone();for key in ["requiredLocalFiles","runtimeMappings"]{bad[key].as_array_mut().unwrap().iter_mut().find(|v|v["path"]==*path).unwrap()["path"]="mods/noita_together2/files/c2/missing-test.lua".into();}assert!(verify(&bad).is_err(),"{path}");}
  for key in ["itemFixture","b1Fixture","a2Fixture","a3Fixture","c1Fixture","c2Fixture","raceFixture","fixtureNative"]{let mut bad=normal.clone();bad[key]=serde_json::json!({});assert!(verify(&bad).is_err(),"{key}");}
  for key in ["revision","sha256"]{let mut bad=normal.clone();bad["hourglassCatalogue"][key]="changed".into();assert!(verify(&bad).is_err());}
  for cap in ["core.world-rules","core.hourglass","race.goals.v1"]{let mut bad=normal.clone();bad["profile"]["capabilities"].as_array_mut().unwrap().retain(|v|v!=cap);assert!(verify(&bad).is_err());}
  for key in ["private","testFlags"]{let mut bad=normal.clone();bad["nativeArtifacts"][0][key]=true.into();assert!(verify(&bad).is_err());}
  let mut bad=normal.clone();bad["id"]="other-normal".into();assert!(verify(&bad).is_err());
  let mut bad=normal.clone();for key in ["requiredLocalFiles","runtimeMappings"]{bad[key].as_array_mut().unwrap().pop();}assert!(verify(&bad).is_err());
  let mut bad=normal.clone();for key in ["requiredLocalFiles","runtimeMappings"]{bad[key].as_array_mut().unwrap().push(serde_json::json!({"path":"mods/noita_together2/files/c2/extra.lua","sha256":"a".repeat(64)}));}assert!(verify(&bad).is_err());
 }
 #[cfg(feature="g3-rejoin")]
 #[test]fn testing_loadout_normal_324_closure_caps_and_negative_boundaries(){
  let mut normal=development_profile().unwrap();assert_eq!(normal["requiredLocalFiles"].as_array().unwrap().len(),324);
  for key in ["itemFixture","b1Fixture","a2Fixture","a3Fixture","c1Fixture","c2Fixture","raceFixture","fixtureNative"]{normal.as_object_mut().unwrap().remove(key);}
  normal["id"]="nt2-g3-testing-loadout-dev-20261030".into();normal["fixtureOnly"]=false.into();normal["sourceApproved"]=true.into();normal["developmentOnly"]=true.into();normal["installApproved"]=false.into();
  for a in normal["nativeArtifacts"].as_array_mut().unwrap(){a["private"]=false.into();a["testFlags"]=false.into();}
  let root=Path::new("E:/C2-read-only-policy-test");
  let verify=|a:&Value|verify_policy(root,&a["profile"],a,|p|{
   if p==root.join("noita.exe"){return Ok(a["profile"]["supportedBuilds"][0]["gameExeSha256"].as_str().unwrap().into())}
   if p==root.join("lua51.dll"){return Ok(a["profile"]["supportedBuilds"][0]["originalLuaSha256"].as_str().unwrap().into())}
   a["requiredLocalFiles"].as_array().unwrap().iter().find(|f|p==root.join(f["path"].as_str().unwrap())).and_then(|f|f["sha256"].as_str()).map(String::from).ok_or("Unexpected resource".into())
  },false);
  verify(&normal).unwrap();assert_eq!(development_capabilities(),serde_json::json!(crate::protocol_g2_next::RACE_CAPABILITIES));
  for modes in [serde_json::json!(["core"]),serde_json::json!(["core","race","nemesis"])]{let mut bad=normal.clone();bad["compatibility"]["modes"]=modes;assert!(verify(&bad).is_err());}
  for path in C2_REQUIRED{let mut bad=normal.clone();for key in ["requiredLocalFiles","runtimeMappings"]{bad[key].as_array_mut().unwrap().iter_mut().find(|v|v["path"]==*path).unwrap()["path"]="mods/noita_together2/files/c2/missing-test.lua".into();}assert!(verify(&bad).is_err(),"{path}");}
  for key in ["itemFixture","b1Fixture","a2Fixture","a3Fixture","c1Fixture","c2Fixture","raceFixture","fixtureNative"]{let mut bad=normal.clone();bad[key]=serde_json::json!({});assert!(verify(&bad).is_err(),"{key}");}
  for key in ["revision","sha256"]{let mut bad=normal.clone();bad["hourglassCatalogue"][key]="changed".into();assert!(verify(&bad).is_err());}
  for cap in ["core.world-rules","core.hourglass","race.goals.v1"]{let mut bad=normal.clone();bad["profile"]["capabilities"].as_array_mut().unwrap().retain(|v|v!=cap);assert!(verify(&bad).is_err());}
  for key in ["private","testFlags"]{let mut bad=normal.clone();bad["nativeArtifacts"][0][key]=true.into();assert!(verify(&bad).is_err());}
  for path in ["mods/noita_together2/files/startup/testing_loadout.lua","mods/noita_together2/files/startup/testing-starting-loadout.json","mods/noita_together2/files/rejoin/lobby_status.lua","mods/noita_together2/files/g3/ghost_labels.lua"]{let mut bad=normal.clone();for key in ["runtimeMappings","requiredLocalFiles"]{bad[key].as_array_mut().unwrap().iter_mut().find(|v|v["path"]==path).unwrap()["sha256"]="0".repeat(64).into();}assert!(verify(&bad).is_err());}
  let mut bad=normal.clone();bad["id"]="other-normal".into();assert!(verify(&bad).is_err());
  for (path,_) in D1_REQUIRED.iter().chain(RACE_REQUIRED.iter()){
   for field in ["path","sha256"]{let mut bad=normal.clone();for key in ["runtimeMappings","requiredLocalFiles"]{bad[key].as_array_mut().unwrap().iter_mut().find(|v|v["path"]==*path).unwrap()[field]=if field=="path"{"mods/noita_together2/files/g3/substitute.lua".into()}else{"0".repeat(64).into()};}assert!(verify(&bad).is_err(),"D1{path}/{field}");}
   let mut bad=normal.clone();bad["runtimeMappings"].as_array_mut().unwrap().iter_mut().find(|v|v["path"]==*path).unwrap()["sourcePath"]="mod/noita_together2/files/g3/substitute.lua".into();assert!(verify(&bad).is_err(),"D1 source alias{path}");
  }
  let mut bad=normal.clone();for key in ["requiredLocalFiles","runtimeMappings"]{bad[key].as_array_mut().unwrap().pop();}assert!(verify(&bad).is_err());
  let mut bad=normal.clone();for key in ["requiredLocalFiles","runtimeMappings"]{bad[key].as_array_mut().unwrap().push(serde_json::json!({"path":"mods/noita_together2/files/c2/extra.lua","sha256":"a".repeat(64)}));}assert!(verify(&bad).is_err());
 }
 #[cfg(feature="g3-rejoin")]
 #[test]fn c1_normal_281_closure_accepts_exact_policy_and_rejects_missing_private_catalogue_and_oversize(){
  // Policy/Noita file reads are injected. This grants no fixture installation.
  let mut normal=development_profile().unwrap();if normal.get("c1Catalogue").is_none(){return}
  for key in ["itemFixture","b1Fixture","a2Fixture","a3Fixture","c1Fixture","fixtureNative"]{normal.as_object_mut().unwrap().remove(key);}
  normal["fixtureOnly"]=false.into();normal["sourceApproved"]=true.into();normal["developmentOnly"]=true.into();normal["installApproved"]=false.into();
  for a in normal["nativeArtifacts"].as_array_mut().unwrap(){a["private"]=false.into();a["testFlags"]=false.into();}
  assert_eq!(normal["requiredLocalFiles"].as_array().unwrap().len(),281);assert_eq!(C1_REQUIRED.len(),29);
  let root=Path::new("E:/C1-read-only-policy-test");
  let verify=|a:&Value|verify_policy(root,&a["profile"],a,|p|{
   if p==root.join("noita.exe"){return Ok(a["profile"]["supportedBuilds"][0]["gameExeSha256"].as_str().unwrap().into())}
   if p==root.join("lua51.dll"){return Ok(a["profile"]["supportedBuilds"][0]["originalLuaSha256"].as_str().unwrap().into())}
   a["requiredLocalFiles"].as_array().unwrap().iter().find(|f|p==root.join(f["path"].as_str().unwrap())).and_then(|f|f["sha256"].as_str()).map(String::from).ok_or("Unexpected resource".into())
  },false);
  verify(&normal).unwrap();
  for path in C1_REQUIRED{let mut bad=normal.clone();for key in ["requiredLocalFiles","runtimeMappings"]{bad[key].as_array_mut().unwrap().iter_mut().find(|f|f["path"]==*path).unwrap()["path"]="mods/noita_together2/files/c1/omitted-test.lua".into();}assert!(verify(&bad).is_err(),"Missing C1 mapping accepted: {path}");}
  for key in ["revision","sha256"]{let mut bad=normal.clone();bad["c1Catalogue"][key]="changed".into();assert!(verify(&bad).is_err());}
  let mut bad=normal.clone();for key in ["requiredLocalFiles","runtimeMappings"]{bad[key].as_array_mut().unwrap().iter_mut().find(|f|f["path"]=="mods/noita_together2/files/g3/c1_catalogue_data.lua").unwrap()["sha256"]="a".repeat(64).into();}assert!(verify(&bad).is_err());
  for selector in ["itemFixture","b1Fixture","a2Fixture","a3Fixture","c1Fixture","fixtureNative"]{let mut bad=normal.clone();bad[selector]=serde_json::json!({"private":true,"revision":"c1-player-private-1","installed":false});assert!(verify(&bad).is_err(),"Normal C1 accepted {selector}");}
  for flag in ["private","testFlags"]{let mut bad=normal.clone();bad["nativeArtifacts"][0][flag]=true.into();assert!(verify(&bad).is_err());}
  for count in [282,512]{let mut bad=normal.clone();while bad["requiredLocalFiles"].as_array().unwrap().len()<count{let n=bad["requiredLocalFiles"].as_array().unwrap().len();let f=serde_json::json!({"path":format!("mods/noita_together2/files/c1/extra-{n}.lua"),"sha256":"a".repeat(64)});bad["requiredLocalFiles"].as_array_mut().unwrap().push(f.clone());bad["runtimeMappings"].as_array_mut().unwrap().push(f);}assert!(verify(&bad).is_err(),"Normal C1 oversized closure accepted");}
  assert!(verify_policy(root,&normal["profile"],&normal,|_|Ok("a".repeat(64)),false).is_err(),"Hash verification must remain active");
 }
 #[cfg(feature="g3-rejoin")]
 #[test]fn item_closure_requires_catalogue_assets_and_both_capability_paths(){
  let current=development_profile().unwrap();if current["itemFixture"]["revision"]!="items-private-1"{return}verify_gold_closure(&current).unwrap();
  for suffix in ITEM_REQUIRED.iter().chain(ITEM_ART.iter()){
   let path=format!("mods/noita_together2/files/g3/{suffix}");let mut bad=current.clone();bad["requiredLocalFiles"].as_array_mut().unwrap().retain(|v|v["path"]!=path);assert!(verify_gold_closure(&bad).is_err(),"Missing item source closure: {suffix}");
  }
  for field in ["profile","compatibility"]{let mut bad=current.clone();bad[field]["capabilities"].as_array_mut().unwrap().retain(|c|c!="core.items");assert!(verify_gold_closure(&bad).is_err());}
  let mut bad=current.clone();bad["itemCatalogue"]["sha256"]=serde_json::json!("a".repeat(64));assert!(verify_gold_closure(&bad).is_err());
 }
 #[cfg(feature="g3-rejoin")]
 #[test]fn item_normal_closure_accepts_no_fixture_marker_and_refuses_private_selectors(){
  let normal:Value=serde_json::from_str("{}").unwrap();verify_gold_closure(&normal).unwrap();
  for selector in ["itemFixture","b1Fixture","a2Fixture","a3Fixture","fixtureNative"]{let mut bad=normal.clone();bad[selector]=serde_json::json!({"revision":"items-private-1","private":true});assert!(verify_gold_closure(&bad).is_err(),"Normal runtime accepted {selector}");}
  for value in [serde_json::json!(true),serde_json::json!("true")]{let mut bad=normal.clone();bad["fixtureOnly"]=value;assert!(verify_gold_closure(&bad).is_err());}
  for flag in ["private","testFlags"]{let mut bad=normal.clone();bad["nativeArtifacts"][0][flag]=true.into();assert!(verify_gold_closure(&bad).is_err());}
  for field in ["revision","sha256"]{let mut bad=normal.clone();bad["itemCatalogue"][field]="changed".into();assert!(verify_gold_closure(&bad).is_err());}
  let mut bad=normal.clone();bad["runtimeMappings"].as_array_mut().unwrap().iter_mut().find(|m|m["path"]=="mods/noita_together2/files/g3/item_catalogue_data.lua").unwrap()["sha256"]="a".repeat(64).into();assert!(verify_gold_closure(&bad).is_err());
  let private=development_profile().unwrap();if private["fixtureOnly"]==true&&private["itemFixture"]["revision"]=="items-private-1"{verify_gold_closure(&private).unwrap();let mut missing=private.clone();missing.as_object_mut().unwrap().remove("itemFixture");assert!(verify_gold_closure(&missing).is_err());}
 }
 #[cfg(feature="g3-rejoin")]
 #[test]fn b1_closure_requires_gold_local_and_both_exact_capability_paths(){
  let current=development_profile().unwrap();if !current["compatibility"]["capabilities"].as_array().unwrap().iter().any(|c|c=="core.gold"){return}
  verify_gold_closure(&current).unwrap();
  for field in ["runtimeMappings","requiredLocalFiles"]{
   let mut bad=current.clone();let entry=bad[field].as_array_mut().unwrap().iter_mut().find(|m|m["path"]=="mods/noita_together2/files/g3/gold_local.lua").unwrap();entry["path"]="mods/noita_together2/files/g3/unused-b1.lua".into();
   assert!(verify_gold_closure(&bad).unwrap_err().contains("gold_local.lua"),"Missing actual production dofile dependency must fail before listed hashes are accepted");
  }
  for field in ["profile","compatibility"]{
   for index in 0..9{let mut bad=current.clone();bad[field]["capabilities"].as_array_mut().unwrap().remove(index);assert!(verify_gold_closure(&bad).is_err());}
   let mut extra=current.clone();extra[field]["capabilities"].as_array_mut().unwrap().push("service-only".into());assert!(verify_gold_closure(&extra).is_err());
  }
 }
 #[cfg(feature="g3-rejoin")]
 #[test]fn b1_normal_gold_selector_uses_pinned_complete_closure_without_fixture_marker(){
  let mut normal=development_profile().unwrap();if !normal["compatibility"]["capabilities"].as_array().unwrap().iter().any(|c|c=="core.gold"){return}
  let items=normal["compatibility"]["capabilities"].as_array().unwrap().iter().any(|c|c=="core.items");
  if items{normal=serde_json::from_str("{}").unwrap();}
  else{normal.as_object_mut().unwrap().remove("b1Fixture");normal.as_object_mut().unwrap().remove("fixtureNative");normal["fixtureOnly"]=false.into();}
  assert!(gold_development_capability(&normal),"Normal packaging must not require a private fixture marker");
  let expected=if items{crate::protocol_g2_next::ITEM_CAPABILITIES}else{crate::protocol_g2_next::GOLD_CAPABILITIES};assert_eq!(development_capabilities(),serde_json::json!(expected));
  let mut missing=normal.clone();missing["requiredLocalFiles"].as_array_mut().unwrap().retain(|f|f["path"]!="mods/noita_together2/files/g3/gold_local.lua");assert!(!gold_development_capability(&missing));
  let mut partial=normal.clone();partial["profile"]["capabilities"].as_array_mut().unwrap().retain(|c|c!="core.gold");assert!(!gold_development_capability(&partial));
  normal["compatibility"]["capabilities"].as_array_mut().unwrap().retain(|c|c!="core.gold");assert!(!gold_development_capability(&normal));
 }
 #[cfg(feature="g3-rejoin")]
 #[test]fn a3_closure_rejects_partial_capabilities_and_missing_saved_restoration_dependencies(){
  let approval=development_profile().unwrap();if !approval["profile"]["capabilities"].as_array().unwrap().iter().any(|c|c=="core.emotes"){return}let root=Path::new("E:/A3-source-policy-test");
  let verify=|a:&Value|verify_policy(root,&a["profile"],a,|path|{if path==root.join("noita.exe"){return Ok(a["profile"]["supportedBuilds"][0]["gameExeSha256"].as_str().unwrap().into())}if path==root.join("lua51.dll"){return Ok(a["profile"]["supportedBuilds"][0]["originalLuaSha256"].as_str().unwrap().into())}a["requiredLocalFiles"].as_array().unwrap().iter().find(|f|path==root.join(f["path"].as_str().unwrap())).and_then(|f|f["sha256"].as_str()).map(String::from).ok_or("Unexpected resource".into())},false);
  verify(&approval).unwrap();for cap in ["core.emotes","core.cosmetics"]{let mut bad=approval.clone();bad["profile"]["capabilities"].as_array_mut().unwrap().retain(|c|c!=cap);assert!(verify(&bad).is_err());}
  for suffix in ["social_restore.lua","emotes/holder_local.xml","emotes/holder.xml"]{let mut bad=approval.clone();let path=format!("mods/noita_together2/files/g3/{suffix}");let file=bad["requiredLocalFiles"].as_array_mut().unwrap().iter_mut().find(|f|f["path"]==path).unwrap();file["path"]=serde_json::json!("mods/noita_together2/files/g3/unused-test-resource.xml");assert!(verify(&bad).is_err(),"Required saved-restoration dependency omitted: {suffix}");}
 }
 #[cfg(feature="g3-seed-loot")]
 #[test]fn seedloot_u2_selector_isolated_from_accepted_potion(){
  let current=development_profile().unwrap();assert_eq!(current["requiredLocalFiles"].as_array().unwrap().len(),file_count());assert_eq!(current["compatibility"]["capabilities"],development_capabilities());
  let old:Value=serde_json::from_slice(b"{}").unwrap();
  assert_ne!(current["profile"],old["profile"]);assert!(verify_development(Path::new("invalid-must-refuse"),&old["profile"]).is_err());
 }
 #[cfg(feature="g3-loot01")]
 #[test]fn loot01_selector_refuses_previous_seedloot_profile(){
  let old:Value=serde_json::from_slice(b"{}").unwrap();
  assert_eq!(old["requiredLocalFiles"].as_array().unwrap().len(),34);
  assert!(verify_development(Path::new("invalid-refuse-before-read"),&old["profile"]).is_err());
  assert!(REQUIRED.contains(&"files/loot/run-policy.txt"));
 }
 #[cfg(feature="g3-c2-potion-diagnostic")]
 #[test]fn potion_selector_isolated_from_regular_approval(){
  let diagnostic=development_profile().unwrap();assert_eq!(diagnostic["serviceId"],"nt2-g3-c2-potion-diagnostic");assert_eq!(diagnostic["namespacePrefix"],"g3_c2_potion_");
  let regular:Value=serde_json::from_slice(b"{}").unwrap();
  assert_ne!(regular["profile"],diagnostic["profile"]);assert!(verify_development(Path::new("invalid-must-refuse-before-file-read"),&regular["profile"]).is_err());
 }
 #[cfg(any(feature="g3-c2-potion-correction",feature="g3-seed-loot"))]
 #[test]fn linked_testing_target_keeps_exact_original_pair_and_all_profile_hashes(){
  let canonical=Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..").canonicalize().unwrap();let repo=std::path::PathBuf::from(canonical.to_str().unwrap().trim_start_matches(r"\\?\"));
  let scratch=repo.join("spikes/noita-entry/.build").join(format!("g3-local-link-profile-{}",uuid::Uuid::new_v4()));let game=scratch.join("game");let target=scratch.join("testing/current/mod/noita_together2");std::fs::create_dir_all(game.join("mods")).unwrap();std::fs::create_dir_all(&target).unwrap();
  let approval=development_profile().unwrap();let mut files=vec![];
  for mapping in approval["runtimeMappings"].as_array().unwrap(){let relative=mapping["path"].as_str().unwrap().strip_prefix("mods/noita_together2/").unwrap();let file=target.join(relative);std::fs::create_dir_all(file.parent().unwrap()).unwrap();let bytes=std::fs::read(repo.join(mapping["sourcePath"].as_str().unwrap())).unwrap();std::fs::write(&file,&bytes).unwrap();files.push((file,bytes));}
  for name in ["noita.exe","lua51.dll"]{std::fs::copy(Path::new("C:/Noita").join(name),game.join(name)).unwrap();}
  assert!(std::process::Command::new("powershell.exe").args(["-NoProfile","-Command","New-Item -ItemType Junction -Path $env:NT2_TEST_LINK -Target $env:NT2_TEST_TARGET | Out-Null"]).env("NT2_TEST_LINK",game.join("mods/noita_together2")).env("NT2_TEST_TARGET",&target).status().unwrap().success());
  let verify=||verify_policy(&game,&approval["profile"],&approval,|p|crate::g3_local_testing::test_hash(&game,p,&target),false);
  verify().unwrap();assert!(verify_development(&game,&approval["profile"]).is_err(),"public production target remains literal, no scratch bypass");
  for(file,bytes)in files{let mut changed=bytes.clone();changed[0]^=1;std::fs::write(&file,changed).unwrap();assert!(verify().is_err());std::fs::write(file,bytes).unwrap();}
  let exe=std::fs::read(game.join("noita.exe")).unwrap();std::fs::write(game.join("noita.exe"),b"changed original").unwrap();assert!(verify().is_err());std::fs::write(game.join("noita.exe"),exe).unwrap();verify().unwrap();
 }
 #[cfg(feature="g3-c2-potion-correction")]
 #[test]fn correction_selector_refuses_regular_and_diagnostic_profiles(){
  let correction=development_profile().unwrap();assert_eq!(correction["serviceId"],"nt2-g3-c2-potion-correction");assert_eq!(correction["namespacePrefix"],"g3_c2_potion_correction_");
  assert_eq!(correction["diagnosticOnly"],false);assert_eq!(correction["correctionCandidate"],true);assert_eq!(correction["gameplaySemanticsChanged"],true);
  for bytes in [b"{}".as_slice(),b"{}".as_slice()]{
   let other:Value=serde_json::from_slice(bytes).unwrap();assert_ne!(other["profile"],correction["profile"]);assert!(verify_development(Path::new("invalid-refuse-before-read"),&other["profile"]).is_err());
  }
 }
 #[test]fn exact_a1_development_resource_closure_and_metadata_refusals(){
  let approval=development_profile().unwrap();assert_eq!(approval["id"],DEVELOPMENT_ID);assert_eq!(approval["serviceId"],SERVICE_ID);
  assert_eq!(approval["sourceApproved"],true);assert_eq!(approval["installApproved"],false);
  let repo=Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..").canonicalize().unwrap();
  let root=repo.join("spikes/noita-entry/.build/social-a1-verifier").join(uuid::Uuid::new_v4().to_string());std::fs::create_dir_all(&root).unwrap();
  let mut assets=vec![];for mapping in approval["runtimeMappings"].as_array().unwrap(){
   let target=root.join(mapping["path"].as_str().unwrap());std::fs::create_dir_all(target.parent().unwrap()).unwrap();
   let bytes=std::fs::read(repo.join(mapping["sourcePath"].as_str().unwrap())).unwrap();std::fs::write(&target,&bytes).unwrap();assets.push((target,bytes));
  }
  assert_eq!(assets.len(),file_count());
  for pair in approval["profile"]["supportedBuilds"].as_array().unwrap(){
   let read=|p:&Path|if p==root.join("noita.exe"){Ok(pair["gameExeSha256"].as_str().unwrap().into())}else if p==root.join("lua51.dll"){Ok(pair["originalLuaSha256"].as_str().unwrap().into())}else{crate::g3_installation::hash(p)};
   verify_policy(&root,&approval["profile"],&approval,read,false).unwrap();
   for (target,bytes) in &assets{let mut bad=bytes.clone();bad[0]^=1;std::fs::write(target,bad).unwrap();assert!(verify_policy(&root,&approval["profile"],&approval,read,false).is_err());std::fs::remove_file(target).unwrap();assert!(verify_policy(&root,&approval["profile"],&approval,read,false).is_err());std::fs::write(target,bytes).unwrap();}
   for field in ["id","bridgeSha256","modSha256","capabilities"]{let mut bad=approval["profile"].clone();bad[field]=serde_json::json!("forged");assert!(verify_policy(&root,&bad,&approval,read,false).is_err());}
   assert!(verify_using(&root,&approval["profile"],&approval,read).is_err());
  }
 }
 #[test]fn unpublished_or_unapproved_social_runtime_never_enables(){let metadata=serde_json::json!({"modVersion":"nt2-g3-social-dev-1","bridgeVersion":"g3-social-transport-1"});assert!(verify(Path::new("E:/fixture"),&metadata).is_err());assert!(verify_using(Path::new("E:/fixture"),&metadata,&serde_json::json!({"profile":metadata,"installApproved":false}),|_|panic!("must refuse before file access")).is_err());}
 #[cfg(not(feature="g3-c2-live"))]
 #[test]fn entire_local_social_closure_pairs_caps_metadata_and_each_tamper(){
  let repo=Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..").canonicalize().unwrap();let root=repo.join("spikes/noita-entry/.build/social-verifier").join(uuid::Uuid::new_v4().to_string());std::fs::create_dir_all(&root).unwrap();
  let pair=crate::g3_installation::profile().unwrap()["profile"]["supportedBuilds"][0].clone();let metadata=serde_json::json!({"modVersion":"nt2-g3-social-dev-1","bridgeVersion":"g3-social-transport-1","capabilities":["core.seed","core.ghost","core.hud","core.poi"],"supportedBuilds":[pair.clone()]});
  let mut files=vec![];let mut originals=vec![];for path in REQUIRED{let source=if *path=="init.lua"{repo.join("mod/noita_together2/init-g3-social.lua")}else if *path=="files/nt2_g3_social.dll"{repo.join("spikes/noita-entry/.build/g3-social/nt2_g3_social.dll")}else{repo.join("mod/noita_together2").join(path)};
   let target=root.join("mods/noita_together2").join(path);std::fs::create_dir_all(target.parent().unwrap()).unwrap();let bytes=std::fs::read(source).unwrap();std::fs::write(&target,&bytes).unwrap();files.push(serde_json::json!({"path":format!("mods/noita_together2/{path}"),"sha256":crate::g3_installation::hash(&target).unwrap()}));originals.push((target,bytes));}
  // Private verifier fixture only: no published approval, actual exe/Lua bytes or install grant.
  let approval=serde_json::json!({"profile":metadata,"installApproved":true,"requiredLocalFiles":files});let read=|p:&Path|if p==root.join("noita.exe"){Ok(pair["gameExeSha256"].as_str().unwrap().to_owned())}else if p==root.join("lua51.dll"){Ok(pair["originalLuaSha256"].as_str().unwrap().to_owned())}else{crate::g3_installation::hash(p)};
  assert!(verify_using(&root,&metadata,&approval,read).is_ok());assert!(verify(&root,&metadata).is_err());for (target,bytes) in originals{let mut bad=bytes.clone();bad[0]^=1;std::fs::write(&target,bad).unwrap();assert!(verify_using(&root,&metadata,&approval,read).is_err());std::fs::write(target,bytes).unwrap();}
  let mut bad=metadata.clone();bad["capabilities"]=serde_json::json!(["core.seed","core.ghost"]);assert!(verify_using(&root,&bad,&approval,read).is_err());let mut missing=approval.clone();missing["requiredLocalFiles"].as_array_mut().unwrap().pop();assert!(verify_using(&root,&metadata,&missing,read).is_err());
 }
}
