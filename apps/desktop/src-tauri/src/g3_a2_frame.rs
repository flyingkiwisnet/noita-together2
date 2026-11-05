//! Normalized A2 display data. Uses the existing native frame/authority envelope.
use crate::protocol_g3_social::wire;
use std::collections::BTreeMap;
#[path="g3_live_time.rs"]mod time;
#[derive(Clone,Debug)]pub struct Player{pub actor:String,pub name:String,pub state:String,pub ghost:String}
#[derive(Clone,Debug,PartialEq)]pub struct Hud{pub actor:String,pub revision:u32,pub hp:Option<u32>,pub max_hp:Option<u32>,pub sampo:Option<bool>,pub biome:String}
pub enum Command{Players{run:String,local:String,players:Vec<Player>},Hud(Hud),Chat{id:String,name:String,text:String},Outcome{id:String,status:u8,reason:String,draft:String},Dismissed(Vec<String>)}
fn text(out:&mut Vec<u8>,s:&str,max:usize)->Result<(),String>{if s.len()>max||s.chars().any(char::is_control){return Err("A2 native literal".into())}out.extend_from_slice(&(s.len() as u16).to_le_bytes());out.extend(s.as_bytes());Ok(())}
pub fn encode(out:&mut Vec<u8>,c:&Command,_now:f64)->Result<(),String>{match c{
 Command::Players{run,local,players}=>{if players.len()>32{return Err("A2 roster capacity".into())}out.push(10);text(out,run,512)?;text(out,local,128)?;out.push(players.len() as u8);for p in players{text(out,&p.actor,128)?;text(out,&p.name,396)?;out.push(match p.state.as_str(){"playing"=>1,"waiting"=>2,"disconnected"=>3,"respawning"=>4,_=>return Err("A2 roster state".into())});text(out,&p.ghost,32)?;}},
 Command::Hud(h)=>{if h.revision==0{return Err("HUD state revision".into())}out.push(11);text(out,&h.actor,128)?;out.extend(h.revision.to_le_bytes());out.extend(h.hp.unwrap_or(0).to_le_bytes());out.extend(h.max_hp.unwrap_or(0).to_le_bytes());out.push(u8::from(h.hp.is_some()&&h.max_hp.is_some())|if h.sampo.is_some(){2}else{0}|if h.sampo==Some(true){4}else{0});text(out,&h.biome,128)?;},
 Command::Chat{id,name,text:message}=>{out.push(12);text(out,id,160)?;text(out,name,396)?;text(out,message,4096)?;},
 Command::Outcome{id,status,reason,draft}=>{if *status>2{return Err("A2 outcome state".into())}out.push(13);text(out,id,32)?;out.push(*status);text(out,reason,396)?;text(out,draft,396)?;},
 Command::Dismissed(ids)=>{if ids.len()>256{return Err("A2 dismissal capacity".into())}out.push(14);out.extend((ids.len() as u16).to_le_bytes());for id in ids{text(out,id,20)?;}}
 }Ok(())}
pub fn roster(response:&wire::Response)->Result<Vec<Player>,String>{
 if response.error.is_some()||response.roster_complete!=Some(true){return Err("Incomplete authenticated roster".into())}
 let mut result=Vec::new();let mut seen=std::collections::BTreeSet::new();for p in response.players.as_deref().unwrap_or(&[]){
  let actor=p.player_id.clone().ok_or("Missing stable player")?;if p.world_token.as_ref()!=Some(&actor)||!seen.insert(actor.clone())||result.len()>=32{return Err("Foreign/duplicate player identity".into())}
  let name=p.display_name.clone().ok_or("Missing authenticated player name")?;if name.len()>396||name.chars().any(char::is_control){return Err("Player name literal".into())}
  let ghost=match(p.principal_id.as_ref(),p.game_session_id.as_ref(),p.world_token.as_ref()){(Some(a),Some(b),Some(c))=>{use sha2::{Digest,Sha256};Sha256::digest(serde_json::to_vec(&(a,b,c)).unwrap())[..16].iter().map(|b|format!("{b:02x}")).collect()},_=>String::new()};
  let state=p.state.clone().ok_or("Missing player state")?;if !["playing","waiting","disconnected","respawning"].contains(&state.as_str())||ghost.is_empty(){return Err("Invalid authenticated player state/identity".into())}
  result.push(Player{actor,name,state,ghost});
 }result.sort_by(|a,b|a.name.to_lowercase().cmp(&b.name.to_lowercase()).then_with(||a.actor.cmp(&b.actor)));Ok(result)
}
/// Retained HUD validity is canonical roster/world identity and revision, not age.
pub fn hud(scope:&wire::Scope,players:&[wire::Player],p:&wire::Projection)->Result<Hud,String>{
 let sender=p.scope.as_ref().ok_or("Missing HUD scope")?;let player=players.iter().find(|a|matches!(a.state.as_deref(),Some("playing"|"waiting"|"respawning"))&&a.game_session_id==p.game_session_id&&a.principal_id==p.principal_id&&a.world_token==sender.world_token).ok_or("HUD outside authenticated roster")?;
 if sender.room_id!=scope.room_id||sender.run_id!=scope.run_id||sender.run_epoch!=scope.run_epoch||sender.config_revision!=scope.config_revision||p.provenance.as_deref()!=Some("validated-client-report"){return Err("Foreign HUD scope/provenance".into())}
 let revision=p.hud_state_revision.filter(|n|*n>0).ok_or("HUD state revision")?;
 if p.event_id.is_some()||p.sequence.is_some()||p.expires_at.is_some(){return Err("Transient fields on retained HUD".into())}
 time::timestamp(p.captured_at.as_deref().ok_or("HUD capture")?)?; // Shape only, not a liveness clock.
 let value=p.hud.as_ref().ok_or("Missing HUD value")?;let known=value.hp_known==Some(true);let hp=value.hp_milli.unwrap_or(0);let max=value.max_hp_milli.unwrap_or(0);
 if known&&(max==0||max>i32::MAX as u32||hp>i32::MAX as u32){return Err("HUD HP bounds".into())}
 let biome=value.biome.clone().unwrap_or_default();if biome.len()>128||biome.chars().any(char::is_control){return Err("HUD biome bounds".into())}
 Ok(Hud{actor:player.player_id.clone().ok_or("HUD player identity")?,revision,hp:known.then_some(hp),max_hp:known.then_some(max),sampo:(value.sampo_known==Some(true)).then_some(value.sampo.unwrap_or(false)),biome})
}
#[derive(Default)]pub struct HudReplay{actors:BTreeMap<String,Hud>}
impl HudReplay{
 /// Revisions belong to stable run/world actors and survive their session change.
 /// Work owns the recipient scope/generation fence; actual departure retires floors.
 pub fn roster(&mut self,players:&[wire::Player]){self.actors.retain(|actor,_|players.iter().any(|p|p.player_id.as_ref()==Some(actor)));}
 pub fn accept(&mut self,value:Hud)->Result<Option<Hud>,String>{
  if let Some(old)=self.actors.get(&value.actor){
   if value.revision<old.revision{return Ok(None)}
   if value.revision==old.revision&&value!=*old{return Err("Changed retained HUD revision".into())}
  }else if self.actors.len()>=32{return Err("HUD replay capacity".into())}
  self.actors.insert(value.actor.clone(),value.clone());Ok(Some(value))
 }
}
#[cfg(test)]mod tests{
 use super::*;
 fn scope()->wire::Scope{let raw:serde_json::Value=serde_json::from_slice(include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../packages/contracts/g3-social/replay-policy.json"))).unwrap();serde_json::from_value(raw["vectors"][0]["scope"].clone()).unwrap()}
 fn player(s:&wire::Scope)->wire::Player{wire::Player{player_id:s.world_token.clone(),world_token:s.world_token.clone(),principal_id:Some(uuid::Uuid::new_v4().to_string()),game_session_id:Some(uuid::Uuid::new_v4().to_string()),display_name:Some("Player".into()),state:Some("playing".into()),inventory_revision:Some(0),held_wand_sprite_id:Some(String::new()),..Default::default()}}
 fn projection(s:&wire::Scope,a:&wire::Player,revision:u32)->wire::Projection{wire::Projection{scope:Some(s.clone()),principal_id:a.principal_id.clone(),game_session_id:a.game_session_id.clone(),hud_state_revision:Some(revision),provenance:Some("validated-client-report".into()),captured_at:Some("1970-01-01T00:00:01.000Z".into()),hud:Some(wire::Hud{hp_known:Some(true),hp_milli:Some(900),max_hp_milli:Some(1000),..Default::default()}),..Default::default()}}
 #[test]fn a2_retained_hud_has_no_display_lifetime_and_zero_is_valid(){
  let s=scope();let a=player(&s);let mut p=projection(&s,&a,1);let h=hud(&s,&[a.clone()],&p).unwrap();let mut early=vec![];let mut late=vec![];encode(&mut early,&Command::Hud(h.clone()),2.).unwrap();encode(&mut late,&Command::Hud(h),3600.).unwrap();assert_eq!(early,late,"Connected information is state, not a larger TTL");
  p.hud.as_mut().unwrap().hp_milli=Some(0);assert_eq!(hud(&s,&[a.clone()],&p).unwrap().hp,Some(0));p.hud.as_mut().unwrap().hp_known=Some(false);assert_eq!(hud(&s,&[a],&p).unwrap().hp,None);
 }
 #[test]fn a2_retained_waiting_disconnect_departure_and_old_world_fences(){
  let s=scope();let mut a=player(&s);let p=projection(&s,&a,1);a.state=Some("waiting".into());assert!(hud(&s,&[a.clone()],&p).is_ok());a.state=Some("respawning".into());assert!(hud(&s,&[a.clone()],&p).is_ok());let mut bytes=Vec::new();encode(&mut bytes,&Command::Players{run:"run".into(),local:"local".into(),players:vec![Player{actor:"actor".into(),name:"Name".into(),state:"respawning".into(),ghost:"0".repeat(32)}]},0.).unwrap();assert!(bytes.contains(&4));a.state=Some("disconnected".into());assert!(hud(&s,&[a],&p).is_err());assert!(hud(&s,&[],&p).is_err());let mut other=s.clone();other.run_id=Some(uuid::Uuid::new_v4().to_string());assert!(hud(&other,&[player(&s)],&p).is_err());
 }
 #[test]fn a2_retained_revision_survives_same_world_session_replacement(){
  let s=scope();let mut a=player(&s);let p=projection(&s,&a,50);let mut replay=HudReplay::default();replay.roster(&[a.clone()]);let first=hud(&s,&[a.clone()],&p).unwrap();assert!(replay.accept(first.clone()).unwrap().is_some());assert!(replay.accept(first).unwrap().is_some(),"Unchanged canonical state can restore a cold consumer");
  a.game_session_id=Some(uuid::Uuid::new_v4().to_string());replay.roster(&[a.clone()]);assert!(hud(&s,&[a.clone()],&p).is_err());let lower=projection(&s,&a,49);assert!(replay.accept(hud(&s,&[a.clone()],&lower).unwrap()).unwrap().is_none());let next=projection(&s,&a,51);assert!(replay.accept(hud(&s,&[a.clone()],&next).unwrap()).unwrap().is_some());let mut substituted=next.clone();substituted.hud.as_mut().unwrap().hp_milli=Some(0);assert!(replay.accept(hud(&s,&[a],&substituted).unwrap()).is_err());
 }
 #[test]fn a2_retained_rejects_missing_revision_transient_shape_and_bad_hp(){
  let s=scope();let a=player(&s);let mut p=projection(&s,&a,1);p.hud_state_revision=None;assert!(hud(&s,&[a.clone()],&p).is_err());p.hud_state_revision=Some(1);p.sequence=Some(1);assert!(hud(&s,&[a.clone()],&p).is_err());p.sequence=None;p.hud.as_mut().unwrap().max_hp_milli=Some(0);assert!(hud(&s,&[a],&p).is_err());
 }
}
