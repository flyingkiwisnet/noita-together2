//! Bounded game-thread data, carried by the existing optional native mailbox.
use crate::protocol_g3_social::{self,wire};
#[derive(Clone,Debug,serde::Deserialize,serde::Serialize)]
#[serde(tag="kind",rename_all="camelCase",deny_unknown_fields)]
pub enum Input{
 Hud{hud:wire::Hud},
 Poi{id:String,text:String,x:i32,y:i32},
 Dismiss{sequence:String},
 Emote{id:String,emote:wire::Emote},
 Cosmetic{id:String,cosmetic:wire::Cosmetic},
}
#[cfg(test)]mod tests{use super::*;
 #[test]fn a3_bounded_native_intents_catalogue_and_authority_rejection(){
  let id=uuid::Uuid::new_v4().simple().to_string();for emote in crate::g3_social_projection::EMOTE_IDS{for active in [true,false]{assert!(Input::decode(&serde_json::to_vec(&serde_json::json!({"kind":"emote","id":id,"emote":{"emoteId":emote,"active":active}})).unwrap()).is_ok());}}
  for robe in crate::g3_social_projection::ROBE_IDS{assert!(Input::decode(&serde_json::to_vec(&serde_json::json!({"kind":"cosmetic","id":id,"cosmetic":{"robeId":robe,"hat":true,"amulet":false,"gem":true}})).unwrap()).is_ok());}
  for bad in [serde_json::json!({"kind":"emote","id":id,"emote":{"emoteId":"untrusted/path","active":true}}),serde_json::json!({"kind":"emote","id":id,"emote":{"emoteId":"wave","active":true},"authority":true}),serde_json::json!({"kind":"cosmetic","id":id,"cosmetic":{"robeId":"blue","hat":true}})]{assert!(Input::decode(&serde_json::to_vec(&bad).unwrap()).is_err(),"Accepted malformed local intent: {bad}");}
 }
 #[test]fn a2_production_capture_json_decodes_known_and_unknown_hud(){
  // Same bytes emitted by a2-capture-wire-test.lua using the production
  // capture and FFI encoder under Noita's Lua DLL (engine facts injected).
  let Input::Hud{hud}=Input::decode(br#"{"kind":"hud","hud":{"hpKnown":true,"sampoKnown":true,"hpMilli":4000,"maxHpMilli":4000,"sampo":false,"biome":"$biome_coalmine"}}"#).unwrap()else{panic!("HUD input")};
  assert_eq!(hud.hp_known,Some(true));assert_eq!(hud.hp_milli,Some(4000));assert_eq!(hud.max_hp_milli,Some(4000));assert_eq!(hud.sampo,Some(false));assert_eq!(hud.biome.as_deref(),Some("$biome_coalmine"));
  let Input::Hud{hud}=Input::decode(br#"{"kind":"hud","hud":{"hpKnown":false,"sampoKnown":false}}"#).unwrap()else{panic!("unknown HUD input")};assert_eq!(hud.hp_known,Some(false));assert!(hud.hp_milli.is_none()&&hud.biome.is_none());
 }
 #[test]fn a2_unicode_byte_codepoint_controls_and_strict_identity(){
  let id=uuid::Uuid::new_v4().simple().to_string();let bytes=|text:&str|serde_json::to_vec(&serde_json::json!({"kind":"poi","id":id,"text":text,"x":-100,"y":200})).unwrap();
  assert!(Input::decode(&bytes(&"🧙".repeat(99))).is_ok());assert!(Input::decode(&bytes(&"🧙".repeat(100))).is_err());for text in ["","  ","a\n","a\u{7f}","a\u{85}"]{assert!(Input::decode(&bytes(text)).is_err());}
  assert!(Input::decode(b"{\"kind\":\"dismiss\",\"sequence\":\"01\"}").is_err());assert!(Input::decode(b"{\"kind\":\"dismiss\",\"sequence\":\"1\",\"authority\":true}").is_err());let mut bad=bytes("valid");bad[0]=255;assert!(Input::decode(&bad).is_err());
 }
}
impl Input{
 pub fn decode(bytes:&[u8])->Result<Self,String>{
  if bytes.is_empty()||bytes.len()>4096{return Err("Native social input bound".into())}
  let value:Self=serde_json::from_slice(bytes).map_err(|_|"Native social input shape")?;
  match &value{
   Self::Hud{hud}=>{protocol_g3_social::typed::<wire::Hud>("Hud",&serde_json::to_vec(hud).unwrap()).map_err(|_|"Native HUD facts")?;},
   Self::Poi{id,text,..}=>{if id.len()!=32||uuid::Uuid::parse_str(id).is_err()||text.is_empty()||text.len()>396||text.chars().count()>99||text.chars().any(char::is_control)||text.trim().is_empty(){return Err("Native POI literal/identity".into())}},
   Self::Dismiss{sequence}=>{if sequence.parse::<u64>().ok().filter(|n|*n>0).is_none_or(|n|n.to_string()!=*sequence){return Err("Native dismissal sequence".into())}}
   Self::Emote{id,emote}=>{if id.len()!=32||uuid::Uuid::parse_str(id).is_err(){return Err("Native emote identity".into())}let raw:serde_json::Value=serde_json::from_slice(bytes).map_err(|_|"Native emote JSON")?;if emote.emote_id.as_deref().is_none_or(|id|!crate::g3_social_projection::EMOTE_IDS.contains(&id))||emote.active.is_none()||raw["emote"].as_object().is_none_or(|v|v.len()!=2){return Err("Native emote shape".into())}protocol_g3_social::typed::<wire::Emote>("Emote",&serde_json::to_vec(&raw["emote"]).unwrap()).map_err(|_|"Native emote facts")?;},
   Self::Cosmetic{id,cosmetic}=>{if id.len()!=32||uuid::Uuid::parse_str(id).is_err(){return Err("Native cosmetic identity".into())}let raw:serde_json::Value=serde_json::from_slice(bytes).map_err(|_|"Native cosmetic JSON")?;if cosmetic.robe_id.as_deref().is_none_or(|id|!crate::g3_social_projection::ROBE_IDS.contains(&id))||cosmetic.hat.is_none()||cosmetic.amulet.is_none()||cosmetic.gem.is_none()||raw["cosmetic"].as_object().is_none_or(|v|v.len()!=4){return Err("Native cosmetic shape".into())}protocol_g3_social::typed::<wire::Cosmetic>("Cosmetic",&serde_json::to_vec(&raw["cosmetic"]).unwrap()).map_err(|_|"Native cosmetic facts")?;}
  }Ok(value)
 }
}
