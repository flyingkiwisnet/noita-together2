//! Consumer validation and immutable digest for the SERVER-owned item contract.
//! No game effect, credential, reservation or compensation authority lives here.
use crate::protocol_g3::{self,wire};
use serde::{Serialize,ser::{SerializeMap,SerializeSeq}};
use serde_json::{json,Value};
use std::{collections::{BTreeMap,BTreeSet},sync::LazyLock};
pub const MAX_DESCRIPTOR:usize=131072;
pub const MAX_PENDING:usize=256;
#[derive(Debug,Clone,PartialEq,Eq,serde::Serialize,serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Original{pub kind:String,pub payload_json:String}
struct Catalogue{revision:String,actions:BTreeSet<String>,materials:BTreeSet<String>,sprites:BTreeSet<String>,objects:BTreeMap<String,bool>}
static CATALOGUE:LazyLock<Catalogue>=LazyLock::new(||{
 let v:Value=serde_json::from_str(include_str!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../packages/contracts/g3-items/catalogue.json"))).unwrap();
 let ids=|key:&str|v[key].as_array().unwrap().iter().map(|r|r["id"].as_str().unwrap().into()).collect();
 Catalogue{revision:v["revision"].as_str().unwrap().into(),actions:ids("actions"),materials:ids("materials"),sprites:ids("sprites"),objects:v["objects"].as_array().unwrap().iter().map(|r|(r["id"].as_str().unwrap().into(),r["available"]==true)).collect()}
});
struct Sorted<'a>(&'a Value);
impl Serialize for Sorted<'_>{fn serialize<S:serde::Serializer>(&self,s:S)->Result<S::Ok,S::Error>{match self.0{
 Value::Object(map)=>{let mut keys:Vec<_>=map.keys().collect();keys.sort();let mut out=s.serialize_map(Some(keys.len()))?;for key in keys{out.serialize_entry(key,&Sorted(&map[key]))?;}out.end()},
 Value::Array(array)=>{let mut out=s.serialize_seq(Some(array.len()))?;for value in array{out.serialize_element(&Sorted(value))?;}out.end()},v=>v.serialize(s),
}}}
pub fn catalogue_revision()->&'static str{&CATALOGUE.revision}
pub fn canonical(value:&Value)->Result<Vec<u8>,String>{serde_json::to_vec(&Sorted(value)).map_err(|_|"Item canonical encoding".into())}
fn signed(value:Option<&String>)->Result<i32,String>{value.ok_or("Missing item integer")?.parse().map_err(|_|"Invalid item integer".into())}
fn float(value:Option<&String>,limit:f64)->Result<f64,String>{let n:f64=value.ok_or("Missing item number")?.parse().map_err(|_|"Invalid item number")?;if !n.is_finite()||n.abs()>limit{return Err("Invalid item number".into())}Ok(n)}
fn spell(value:&wire::ItemSpell,current:bool)->Result<(),String>{if value.action_id.as_ref().is_none_or(|id|id.is_empty()||current&&!CATALOGUE.actions.contains(id))||signed(value.uses_remaining.as_ref())?< -1{return Err("Unsupported item spell".into())}Ok(())}
fn cards(values:&[wire::ItemCard],max:usize,current:bool)->Result<(),String>{if values.len()>max{return Err("Item card capacity".into())}let mut previous=None;for card in values{let slot=card.slot.ok_or("Missing card slot")?;if slot>i32::MAX as u32||previous.is_some_and(|p|p>=slot){return Err("Invalid ordered card slots".into())}previous=Some(slot);spell(card.spell.as_ref().ok_or("Missing card spell")?,current)?;}Ok(())}
pub fn descriptor(value:&wire::ItemDescriptor)->Result<wire::ItemDescriptor,String>{descriptor_checked(value,true)}
// Retained validation proves bounded historical structure, never current effect eligibility.
fn descriptor_checked(value:&wire::ItemDescriptor,current:bool)->Result<wire::ItemDescriptor,String>{
 let bytes=serde_json::to_vec(value).map_err(|_|"Item encoding")?;if bytes.len()>MAX_DESCRIPTOR{return Err("Item descriptor size".into())}
 let mut value:wire::ItemDescriptor=protocol_g3::typed("ItemDescriptor",&bytes).map_err(|_|"Invalid canonical item descriptor")?;
 if value.catalogue_revision.as_ref().is_none_or(|id|id.is_empty()){return Err("Missing item catalogue".into())}
 if current&&value.catalogue_revision.as_deref()!=Some(&CATALOGUE.revision){return Err("Unsupported item catalogue".into())}
 if [value.spell.is_some(),value.wand.is_some(),value.container.is_some(),value.object.is_some()].into_iter().filter(|v|*v).count()!=1{return Err("Invalid item category".into())}
 if let Some(s)=&value.spell{spell(s,current)?;}
 if let Some(w)=&mut value.wand{
  if w.named.is_none()||w.shuffle_deck_when_empty.is_none()||w.ui_name.as_ref().is_none_or(|s|s.len()>256)||w.sprite_id.as_ref().is_none_or(|id|id.is_empty()||current&&!CATALOGUE.sprites.contains(id)){return Err("Unsupported wand appearance".into())}
  for v in [&w.grip_x,&w.grip_y,&w.tip_x,&w.tip_y,&w.mana_max,&w.mana_charge_speed,&w.spread_degrees,&w.speed_multiplier,&w.fire_rate_wait]{float(v.as_ref(),f32::MAX as f64)?;}
  signed(w.reload_time.as_ref())?;signed(w.actions_per_round.as_ref())?;let capacity=signed(w.deck_capacity.as_ref())?;
  let deck=w.deck.get_or_insert_with(Vec::new);cards(deck,1024,current)?;let permanent=w.permanent.get_or_insert_with(Vec::new);cards(permanent,256,current)?;
  capacity.checked_add(permanent.len()as i32).ok_or("Wand capacity overflow")?;
 }
 if let Some(c)=&mut value.container{
  if !matches!(c.container_type.as_deref(),Some("potion"|"powder"))||c.color.as_ref().is_none_or(|c|c.parse::<u32>().is_err()){return Err("Invalid container type/colour".into())}
  let contents=c.contents.get_or_insert_with(Vec::new);if contents.len()>1024{return Err("Material count capacity".into())}let mut seen=BTreeSet::new();
  for material in contents.iter(){let id=material.material_id.as_ref().ok_or("Missing material")?;if id.is_empty()||current&&!CATALOGUE.materials.contains(id)||!seen.insert(id){return Err("Unsupported or duplicate material".into())}if float(material.amount.as_ref(),f64::MAX)?<=0.0{return Err("Invalid material quantity".into())}}
 }
 if let Some(o)=&value.object{if o.object_id.as_ref().is_none_or(|id|id.is_empty())||current&&o.object_id.as_ref().and_then(|id|CATALOGUE.objects.get(id))!=Some(&true){return Err("Object unavailable in this game build".into())}}
 Ok(value)
}
pub fn validate_original(original:&Original)->Result<Value,String>{original_checked(original,true)}
pub(crate) fn retained_original(original:&Original)->Result<Value,String>{original_checked(original,false)}
fn original_checked(original:&Original,current:bool)->Result<Value,String>{
 if original.payload_json.len()>MAX_DESCRIPTOR{return Err("Item original payload size".into())}
 match original.kind.as_str(){
  "deposit"=>{let item=protocol_g3::typed("ItemDescriptor",original.payload_json.as_bytes()).map_err(|_|"Invalid item original")?;serde_json::to_value(descriptor_checked(&item,current)?).map_err(|_|"Item original encoding".into())},
  "claim"=>{let payload:Value=serde_json::from_str(&original.payload_json).map_err(|_|"Invalid claim original")?;let id=payload["itemId"].as_str().ok_or("Missing original item ID")?;let revision=payload["expectedRevision"].as_str().ok_or("Missing original item revision")?;if payload.as_object().is_none_or(|m|m.len()!=2)||uuid::Uuid::parse_str(id).is_err()||revision.parse::<u64>().is_err()||revision.is_empty()||revision.starts_with('0')&&revision!="0"{return Err("Invalid claim identity/revision".into())}Ok(payload)},
  _=>Err("Invalid item original direction".into()),
 }
}
pub(crate) fn retained_digest_bytes(principal:&str,scope:&wire::Scope,original:&Original)->Result<Vec<u8>,String>{canonical(&json!({"v":1,"principalId":principal,"scope":scope,"kind":format!("item.{}",original.kind),"payload":retained_original(original)?}))}
pub fn digest_bytes(principal:&str,scope:&wire::Scope,original:&Original)->Result<Vec<u8>,String>{canonical(&json!({"v":1,"principalId":principal,"scope":scope,"kind":format!("item.{}",original.kind),"payload":validate_original(original)?}))}
pub fn digest(principal:&str,scope:&wire::Scope,original:&Original)->Result<String,String>{use sha2::{Digest,Sha256};Ok(Sha256::digest(digest_bytes(principal,scope,original)?).iter().map(|b|format!("{b:02x}")).collect())}
pub fn operation(original:&crate::g3_journal::Original,op:&wire::ItemOperation)->Result<(),String>{
 let payload=original.item.as_ref().ok_or("Not an item original")?;let scope:wire::Scope=protocol_g3::typed("Scope",original.scope_json.as_bytes()).map_err(|_|"Invalid item scope")?;
 if op.operation_id.as_deref()!=Some(&original.operation_id)||op.principal_id.as_deref()!=Some(&original.principal_id)||op.direction.as_deref()!=Some(&payload.kind)||canonical(&serde_json::to_value(&op.original_scope).unwrap())?!=canonical(&serde_json::to_value(&scope).unwrap())?||op.payload_digest.as_deref()!=Some(digest(&original.principal_id,&scope,payload)?.as_str())||op.delivery_id.as_ref().is_none_or(|s|uuid::Uuid::parse_str(s).is_err())||op.item_id.as_ref().is_none_or(|s|uuid::Uuid::parse_str(s).is_err())||op.save_persistence.as_deref()!=Some("unproven")||!matches!(op.state.as_deref(),Some("awaiting_local"|"reserved"|"uncertain"|"completed"|"cancelled")){return Err("Item status differs from immutable original".into())}
 let item=descriptor(op.item.as_ref().ok_or("Item operation descriptor absent")?)?;
 if payload.kind=="deposit"{if canonical(&serde_json::to_value(item).unwrap())?!=canonical(&validate_original(payload)?)?{return Err("Item deposit payload changed".into())}}
 else if validate_original(payload)?["itemId"].as_str()!=op.item_id.as_deref(){return Err("Claim item identity changed".into())}
 Ok(())
}
#[cfg(test)]mod tests{use super::*;
 #[test]fn startup_retained_shape_never_grants_catalogue_eligibility(){
  let vectors:Value=serde_json::from_str(include_str!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../packages/contracts/g3-items/operation-digest.json"))).unwrap();
  let mut wand=vectors["vectors"][0]["original"]["payload"].clone();wand["wand"]["spriteId"]=json!("retired_sprite");
  let spell=json!({"catalogueRevision":catalogue_revision(),"spell":{"actionId":"RETIRED_ACTION","usesRemaining":"-1"}});
  let container=json!({"catalogueRevision":catalogue_revision(),"container":{"containerType":"potion","color":"4294967295","contents":[{"materialId":"retired_material","amount":"5"}]}});
  let object=json!({"catalogueRevision":catalogue_revision(),"object":{"objectId":"retired_object"}});
  for mut payload in [wand,spell,container,object]{
   for revision in ["nt2-items-base-1",catalogue_revision()]{payload["catalogueRevision"]=json!(revision);let o=Original{kind:"deposit".into(),payload_json:payload.to_string()};assert!(retained_original(&o).is_ok());assert!(validate_original(&o).is_err());}
  }
  for payload in [json!({"spell":{"actionId":"OLD","usesRemaining":"-1"}}),json!({"catalogueRevision":"old","object":{}}),json!({"catalogueRevision":"old","spell":{"actionId":"OLD","usesRemaining":"-2"}}),json!({"catalogueRevision":"old","container":{"containerType":"potion","color":"1","contents":[{"materialId":"old","amount":"NaN"}]}})]{
   assert!(retained_original(&Original{kind:"deposit".into(),payload_json:payload.to_string()}).is_err());
  }
 }
 #[test]fn item_original_digest_matches_server_utf8_vectors(){let vectors:Value=serde_json::from_str(include_str!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../packages/contracts/g3-items/operation-digest.json"))).unwrap();for vector in vectors["vectors"].as_array().unwrap(){let o=&vector["original"];let scope=serde_json::from_value(o["scope"].clone()).unwrap();let original=Original{kind:vector["direction"].as_str().unwrap().into(),payload_json:o["payload"].to_string()};let bytes=digest_bytes(o["principalId"].as_str().unwrap(),&scope,&original).unwrap();assert_eq!(std::str::from_utf8(&bytes).unwrap(),vector["canonicalJson"].as_str().unwrap());assert_eq!(digest(o["principalId"].as_str().unwrap(),&scope,&original).unwrap(),vector["sha256"]);}}
}
