//! Optional read-only observations. Reuses journal ownership and authenticated work.
use crate::{c1_journal::{Journal,Record},c1_work::Context,g3_social_stream::Exchange,protocol_stats::{self as p,wire}};
use serde::Deserialize;
use serde_json::{json,Value};
use std::{collections::VecDeque,sync::{Arc,Mutex},time::{Duration,Instant}};
#[derive(Clone,Deserialize)]#[serde(deny_unknown_fields)]
pub struct Input{kind:String,world:String,observation:wire::Observe}
impl Input{pub fn decode(bytes:&[u8])->Option<Self>{
 if bytes.len()>4096{return None}let v:Self=serde_json::from_slice(bytes).ok()?;
 if v.kind!="d1"||!crate::debug_diagnostics::hex(&v.world,64){return None}
 p::typed::<wire::Observe>("Observe",&serde_json::to_vec(&v.observation).ok()?).ok()?;
 uuid::Uuid::parse_str(v.observation.event_id.as_deref()?).ok()?;v.observation.progression?;
 if !matches!(v.observation.kind.as_deref(),Some("heart"|"orb"|"steve"|"death"|"win"))||v.observation.kind.as_deref()==Some("orb")&&v.observation.orb_id.is_none(){return None}Some(v)
}}
#[derive(Default)]struct State{world:String,journal:Option<Journal>,accepted:VecDeque<(String,Value)>,status:String}
impl State{fn view(&self)->Value{json!({"world":self.world,"accepted":self.accepted.iter().rev().take(32).map(|(id,_)|id).collect::<Vec<_>>(),"queued":self.journal.as_ref().map(|j|j.rows(&self.world).len()).unwrap_or(0),"status":self.status})}}
#[derive(Default)]pub struct Work{state:State,task:Option<tokio::task::JoinHandle<State>>,next:Option<Instant>,failures:u32}
impl Work{
 pub async fn shutdown(&mut self){if let Some(t)=self.task.take(){let _=t.await;}}
 pub async fn tick(&mut self,context:Option<Context>,stream:Arc<Mutex<Exchange>>){
  if self.task.as_ref().is_some_and(|t|t.is_finished()){if let Ok(s)=self.task.take().unwrap().await{self.state=s;}self.failures=if self.state.status.is_empty(){0}else{self.failures.saturating_add(1).min(4)};self.next=Some(Instant::now()+Duration::from_millis((500u64<<self.failures)+u64::from(uuid::Uuid::new_v4().as_bytes()[0])));}
  let Some(c)=context.filter(|c|!c.bootstrap&&c.owner.active().is_ok())else{return};if self.task.is_some(){return}
  let key=crate::c1_work::world(&c);if self.state.world!=key{self.state.world=key;self.state.accepted.clear();self.state.status.clear();self.next=None;self.failures=0;}
  stream.lock().unwrap().d1_view_for(c.binding.context,self.state.view());
  let input=stream.lock().unwrap().d1_input();if self.next.is_some_and(|at|Instant::now()<at){return}
  let mut state=std::mem::take(&mut self.state);let runtime=tokio::runtime::Handle::current();
  self.task=Some(tokio::task::spawn_blocking(move||runtime.block_on(async move{
   let mut c=c;let seq=input.as_ref().map(|(serial,seq,_)|(*serial,*seq));let result=step(&mut c,&mut state,input,&stream).await;if let Some((serial,seq))=seq{stream.lock().unwrap().acknowledge_d1(serial,seq);}state.status=result.err().unwrap_or_default();if c.owner.active().is_ok(){stream.lock().unwrap().d1_view_for(c.binding.context,state.view());}state
  })));
 }
}
async fn step(c:&mut Context,s:&mut State,input:Option<(u64,u64,Input)>,_stream:&Arc<Mutex<Exchange>>)->Result<(),String>{
 c.owner.active()?;if s.journal.is_none(){s.journal=Some(Journal::open(&c.consumer.journal.c1_path().with_file_name("d1-observations.jsonl"))?);}
 let j=s.journal.as_mut().unwrap();j.retire_deleted(&c.consumer.journal.deleted_lobbies())?;
 if let Some((_,sequence,input))=input{
  if input.world!=s.world{return Err("Statistics world changed".into())}
  let id=input.observation.event_id.clone().unwrap();let original=serde_json::to_value(input.observation).unwrap();
  if let Some(pos)=s.accepted.iter().position(|(key,_)|*key==id){if s.accepted[pos].1!=original{return Err("Statistics original changed".into())}let old=s.accepted.remove(pos).unwrap();s.accepted.push_back(old);}
  else{j.put(Record{ordinal:0,source:None,lobby:crate::g3_journal::LobbyIdentity{service_origin:c.binding.service_origin.clone(),service_id:c.binding.service_id.clone(),principal_id:c.binding.principal_id.clone(),room_id:c.binding.scope.room_id.clone().ok_or("Statistics room missing")?},world:s.world.clone(),id,sequence:"stats".into(),state:"pending".into(),reason:"observed_applied".into(),original})?;}
  c.owner.active()?;let _=sequence;
 }
 let Some(mut row)=j.rows(&s.world).into_iter().find(|r|r.sequence=="stats")else{return Ok(())};
 let request=wire::Request{major:Some(1),revision:Some(p::REVISION.into()),service_id:Some(c.binding.service_id.clone()),service_epoch:Some(c.binding.service_epoch.clone()),session_epoch:Some(c.binding.session_epoch.clone()),request_id:Some(uuid::Uuid::new_v4().to_string()),scope:Some(serde_json::from_value(serde_json::to_value(&c.binding.scope).unwrap()).map_err(|_|"Statistics scope")?),owner_generation:Some(c.ownership_generation),observe:Some(serde_json::from_value(row.original.clone()).map_err(|_|"Statistics original")?),..Default::default()};
 c.owner.statistics(request).await?;row.state="acknowledged".into();j.put(row.clone())?;s.accepted.push_back((row.id,row.original));while s.accepted.len()>64{s.accepted.pop_front();}Ok(())
}
#[cfg(test)]mod tests{use super::*;
 fn observation()->Value{json!({"eventId":"11111111-1111-4111-8111-111111111111","kind":"heart","progression":0})}
 #[test]fn d1_observation_input_is_closed_and_canonical(){
  let base=json!({"kind":"d1","world":"a".repeat(64),"observation":observation()});assert!(Input::decode(&serde_json::to_vec(&base).unwrap()).is_some());
  for field in ["count","target","token"]{let mut bad=base.clone();bad["observation"][field]=json!(1);assert!(Input::decode(&serde_json::to_vec(&bad).unwrap()).is_none());}
  for bad in [json!({"kind":"d1","world":"bad","observation":observation()}),json!({"kind":"d1","world":"a".repeat(64),"observation":{"eventId":"11111111-1111-4111-8111-111111111111","kind":"enemy","progression":0}}),json!({"kind":"d1","world":"a".repeat(64),"observation":{"eventId":"11111111-1111-4111-8111-111111111111","kind":"orb","progression":0}})]{assert!(Input::decode(&serde_json::to_vec(&bad).unwrap()).is_none());}
 }
 #[test]fn d1_saved_observation_retries_without_ambiguous_effect_replay(){
  let root=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().parent().unwrap().parent().unwrap().join("spikes/noita-entry/.build/d1-client-20261104").join(uuid::Uuid::new_v4().to_string());std::fs::create_dir_all(&root).unwrap();let path=root.join("observations.jsonl");
  let row=Record{ordinal:0,source:None,lobby:crate::g3_journal::LobbyIdentity{service_origin:"https://fixture.invalid".into(),service_id:"service".into(),principal_id:"player".into(),room_id:"room".into()},world:"a".repeat(64),id:"11111111-1111-4111-8111-111111111111".into(),sequence:"stats".into(),state:"pending".into(),reason:"observed_applied".into(),original:observation()};
  {let mut j=Journal::open(&path).unwrap();j.put(row.clone()).unwrap();j.put(row.clone()).unwrap();let mut changed=row.clone();changed.original["kind"]=json!("steve");assert!(j.put(changed).is_err());}
  let mut j=Journal::open(&path).unwrap();let saved=j.rows(&row.world);assert_eq!(saved.len(),1);assert_eq!(saved[0].original,row.original);assert_eq!(saved[0].reason,"observed_applied");assert_eq!(saved[0].state,"pending");let mut done=row;done.state="acknowledged".into();j.put(done.clone()).unwrap();assert!(j.rows(&done.world).is_empty());drop(j);std::fs::remove_dir_all(root).unwrap();
 }
}
