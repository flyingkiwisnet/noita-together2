//! Scoped local social operation/dismissal journal. No credentials or authority.
//! Disk work runs outside Core/native locks. Unknown replies retain originals.
use crate::{g3_local_consumer::Binding,protocol_g3_social::wire};
use std::{collections::{BTreeMap,BTreeSet},path::{Path,PathBuf},sync::Mutex,io::{Read,Write}};
#[derive(Clone,serde::Serialize,serde::Deserialize)]#[serde(deny_unknown_fields)]
pub struct Operation{pub publish:wire::Publish,pub text:String,pub status:u8,pub reason:String,#[serde(default)]pub uncertain:bool}
impl Operation{
 pub fn matches(&self,text:&str,x:i32,y:i32)->bool{self.text==text&&self.publish.poi.as_ref().is_some_and(|p|p.text.as_deref()==Some(text)&&p.x_milli.as_deref()==Some(x.to_string().as_str())&&p.y_milli.as_deref()==Some(y.to_string().as_str()))}
}
#[cfg(test)]mod tests{use super::*;
 fn binding()->Binding{let raw:serde_json::Value=serde_json::from_slice(include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../packages/contracts/g3-social/replay-policy.json"))).unwrap();Binding{context:[1;16],service_origin:"https://127.0.0.1:4358".into(),service_id:"service".into(),service_epoch:"epoch".into(),session_epoch:"session".into(),principal_id:uuid::Uuid::new_v4().to_string(),scope:serde_json::from_value(raw["vectors"][0]["scope"].clone()).unwrap()}}
 #[test]fn a3_cold_original_uncertainty_pending_kind_and_capacity_are_transactional(){
  let root=Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../spikes/noita-entry/.build").join(format!("a3-journal-{}",uuid::Uuid::new_v4()));let b=binding();let store=Store::new(&root,&b);let id=uuid::Uuid::new_v4().simple().to_string();let p=wire::Publish{event_id:Some(uuid::Uuid::new_v4().to_string()),sequence:Some(1),emote:Some(wire::Emote{emote_id:Some("wave".into()),active:Some(false)}),..Default::default()};let expected=serde_json::to_vec(&p).unwrap();store.update(|s|{s.social_owner="same-current-owner".into();s.social.insert(id.clone(),SocialOperation{publish:p,status:0,reason:String::new(),uncertain:true});s.social_pending.insert("emote".into(),id.clone());s.dismissed.insert(1);Ok(())}).unwrap();let cold=Store::new(&root,&b).read().unwrap();assert_eq!(serde_json::to_vec(&cold.social[&id].publish).unwrap(),expected);assert!(cold.social[&id].uncertain);assert_eq!(cold.social_pending["emote"],id);assert!(store.update(|s|{s.social_pending.insert("cosmetic".into(),id.clone());Ok(())}).is_err());assert!(!Store::new(&root,&b).read().unwrap().social_pending.contains_key("cosmetic"));
  assert!(store.update(|s|{for _ in 0..128{s.social.insert(uuid::Uuid::new_v4().simple().to_string(),cold.social[&id].clone());}Ok(())}).is_err());assert_eq!(store.read().unwrap().social.len(),1);store.update(|s|{s.social_owner="replacement-owner".into();s.social.clear();s.social_pending.clear();Ok(())}).unwrap();let next=Store::new(&root,&b).read().unwrap();assert!(next.social.is_empty());assert!(next.dismissed.contains(&1),"Transient reset preserves A2 journal state");std::fs::remove_dir_all(root).unwrap();
 }
 #[test]fn a2_journal_rebind_preserves_original_uncertainty_dismissal_and_isolates_world(){
  let root=Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../spikes/noita-entry/.build").join(format!("nt2-a2-{}",uuid::Uuid::new_v4()));let mut b=binding();let store=Store::new(&root,&b);let id=uuid::Uuid::new_v4().simple().to_string();let original=wire::Publish{event_id:Some(uuid::Uuid::new_v4().to_string()),sequence:Some(7),poi:Some(wire::Poi{text:Some("雪🧙".into()),x_milli:Some("1000".into()),y_milli:Some("-2000".into())}),..Default::default()};let expected=serde_json::to_vec(&original).unwrap();
  store.update(|s|{s.operations.insert(id.clone(),Operation{publish:original,text:"雪🧙".into(),status:0,reason:String::new(),uncertain:true});s.pending=Some(id.clone());s.dismissed.insert(256);s.chat_high=100;Ok(())}).unwrap();
  b.context=[2;16];b.session_epoch="new-session".into();b.scope.attachment_id=Some(uuid::Uuid::new_v4().to_string());let resumed=Store::new(&root,&b).read().unwrap();let op=&resumed.operations[&id];assert_eq!(serde_json::to_vec(&op.publish).unwrap(),expected);assert!(op.uncertain&&op.matches("雪🧙",1000,-2000));assert!(!op.matches("changed",1000,-2000));assert!(!op.matches("雪🧙",1001,-2000));assert!(resumed.dismissed.contains(&256));assert_eq!(resumed.chat_high,100);
  b.scope.world_token=Some(uuid::Uuid::new_v4().to_string());assert!(Store::new(&root,&b).read().unwrap().operations.is_empty());assert!(store.update(|s|{s.dismissed.insert(0);Ok(())}).is_err());assert!(!store.read().unwrap().dismissed.contains(&0));std::fs::remove_dir_all(root).unwrap();
 }
}
#[derive(Clone,Default,serde::Serialize,serde::Deserialize)]#[serde(deny_unknown_fields)]
pub struct State{pub scope:String,pub operations:BTreeMap<String,Operation>,pub dismissed:BTreeSet<u64>,pub chat_high:u32,pub pending:Option<String>,#[serde(default)]pub social_owner:String,#[serde(default)]pub social:BTreeMap<String,SocialOperation>,#[serde(default)]pub social_pending:BTreeMap<String,String>}
#[derive(Clone,serde::Serialize,serde::Deserialize)]#[serde(deny_unknown_fields)]
pub struct SocialOperation{pub publish:wire::Publish,pub status:u8,pub reason:String,pub uncertain:bool}
pub struct Store{path:PathBuf,scope:String,state:Mutex<Option<State>>}
fn validate(state:&State)->Result<(),String>{
 if state.social.len()>128||state.social_owner.len()>1024||state.social_pending.len()>2{return Err("A3 journal capacity".into())}
 for(kind,id)in &state.social_pending{if !["emote","cosmetic"].contains(&kind.as_str())||state.social.get(id).is_none_or(|op|op.status!=0||op.publish.emote.is_some()!=(kind=="emote")){return Err("A3 pending identity".into())}}
 for(id,op)in &state.social{if id.len()!=32||uuid::Uuid::parse_str(id).is_err()||op.status>2||op.reason.len()>396||op.reason.chars().any(char::is_control)||op.publish.event_id.is_none()||op.publish.sequence.is_none_or(|n|n==0)||op.publish.hud.is_some()||op.publish.poi.is_some()||op.publish.emote.is_some()==op.publish.cosmetic.is_some(){return Err("A3 journal operation".into())}crate::protocol_g3_social::typed::<wire::Publish>("Publish",&serde_json::to_vec(&op.publish).unwrap()).map_err(|_|"A3 original shape")?;}
 if state.operations.len()>288||state.dismissed.len()>256||state.dismissed.contains(&0)||state.pending.as_ref().is_some_and(|id|!state.operations.contains_key(id)){return Err("Social journal capacity/identity".into())}
 for(id,op)in &state.operations{
  crate::g3_social_input::Input::decode(&serde_json::to_vec(&serde_json::json!({"kind":"poi","id":id,"text":op.text,"x":0,"y":0})).unwrap())?;
  crate::protocol_g3_social::typed::<wire::Publish>("Publish",&serde_json::to_vec(&op.publish).unwrap()).map_err(|_|"Social journal original shape")?;
  if op.status>2||op.reason.len()>396||op.reason.chars().any(char::is_control)||op.publish.event_id.is_none()||op.publish.sequence.is_none_or(|s|s==0)||op.publish.poi.as_ref().and_then(|p|p.text.as_ref())!=Some(&op.text)||op.publish.hud.is_some()||op.publish.emote.is_some()||op.publish.cosmetic.is_some(){return Err("Social journal operation".into())}
 }Ok(())
}
fn ordinary(path:&Path)->Result<(),String>{if let Ok(m)=std::fs::symlink_metadata(path){
 #[cfg(windows)]{use std::os::windows::fs::MetadataExt;if m.file_attributes()&0x400!=0{return Err("Social journal link refused".into())}}
 if m.file_type().is_symlink(){return Err("Social journal link refused".into())}
 }Ok(())}
impl Store{
 pub fn new(root:&Path,binding:&Binding)->Self{use sha2::{Digest,Sha256};let scope=serde_json::to_string(&serde_json::json!([binding.service_origin,binding.service_id,binding.principal_id,binding.scope.room_id,binding.scope.run_id,binding.scope.run_epoch,binding.scope.config_revision,binding.scope.world_token])).unwrap();let digest:String=Sha256::digest(scope.as_bytes()).iter().map(|b|format!("{b:02x}")).collect();Self{path:root.join(format!("a2-{digest}.json")),scope,state:Mutex::new(None)}}
 fn load(&self)->Result<State,String>{
  ordinary(self.path.parent().ok_or("Social journal parent")?)?;ordinary(&self.path)?;
  if !self.path.exists(){return Ok(State{scope:self.scope.clone(),..Default::default()})}
  let mut bytes=Vec::new();std::fs::File::open(&self.path).map_err(|_|"Social journal open")?.take(2097153).read_to_end(&mut bytes).map_err(|_|"Social journal read")?;
  if bytes.len()>2097152{return Err("Social journal capacity".into())}let state:State=serde_json::from_slice(&bytes).map_err(|_|"Social journal shape")?;
  if state.scope!=self.scope{return Err("Social journal scope".into())}validate(&state)?;Ok(state)
 }
 pub fn read(&self)->Result<State,String>{let mut slot=self.state.lock().map_err(|_|"Social journal lock")?;if slot.is_none(){*slot=Some(self.load()?)}Ok(slot.as_ref().unwrap().clone())}
 pub fn update<T>(&self,apply:impl FnOnce(&mut State)->Result<T,String>)->Result<T,String>{
  let mut slot=self.state.lock().map_err(|_|"Social journal lock")?;let mut state=if let Some(s)=&*slot{s.clone()}else{self.load()?};let result=apply(&mut state)?;
  while state.operations.len()>288{let key=state.operations.iter().find(|(id,op)|op.status==2&&state.pending.as_ref()!=Some(*id)).map(|(k,_)|k.clone()).ok_or("Social original capacity")?;state.operations.remove(&key);}
  validate(&state)?;let bytes=serde_json::to_vec(&state).map_err(|_|"Social journal encoding")?;if bytes.len()>2097152{return Err("Social journal capacity".into())}
  let root=self.path.parent().ok_or("Social journal parent")?;ordinary(root)?;std::fs::create_dir_all(root).map_err(|_|"Social journal directory")?;ordinary(&self.path)?;
  let temp=root.join(format!(".a2-{}.tmp",uuid::Uuid::new_v4()));let mut file=std::fs::OpenOptions::new().create_new(true).write(true).open(&temp).map_err(|_|"Social journal stage")?;
  file.write_all(&bytes).and_then(|_|file.sync_all()).map_err(|_|"Social journal flush")?;drop(file);
  std::fs::rename(&temp,&self.path).map_err(|e|format!("Social journal commit: {e}"))?;*slot=Some(state);Ok(result)
 }
}
