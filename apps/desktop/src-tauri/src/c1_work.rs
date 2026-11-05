//! C1 optional work on the ordinary scheduler, off Core and native locks.
use crate::{protocol_c1::{self as p,wire},c1_journal::{Journal,Record},g3_social_stream::Exchange};
use serde::Deserialize;
use serde_json::{json,Value};
use std::{sync::{Arc,Mutex},time::{Duration,Instant}};
pub struct Context{pub transport:crate::g3_bank::Context,pub ownership_generation:u32,pub bootstrap:bool,pub completion:bool}
impl std::ops::Deref for Context{type Target=crate::g3_bank::Context;fn deref(&self)->&Self::Target{&self.transport}}
impl std::ops::DerefMut for Context{fn deref_mut(&mut self)->&mut Self::Target{&mut self.transport}}
#[derive(Clone,Debug,Deserialize)]
#[serde(tag="kind",rename_all="snake_case",deny_unknown_fields)]
pub enum Input{#[serde(alias="c1_completion")]C1{publish:Option<wire::Publish>,apply:Option<wire::Receipt>,receipt:Option<wire::Receipt>,life:Option<wire::LifeUpdate>}}
impl Input{
 pub fn completion_allowed(&self)->bool{let Self::C1{publish,apply,receipt,life}=self;apply.is_none()&&(publish.as_ref().is_some_and(|p|p.payload.as_ref().is_some_and(|p|p.death.is_some()))||receipt.is_some()||life.is_some())}
 pub fn decode(b:&[u8])->Option<Self>{
  if b.len()>4096{return None}let value:Self=serde_json::from_slice(b).ok()?;
  let Self::C1{publish,apply,receipt,life}=&value;
  if [publish.is_some(),apply.is_some(),receipt.is_some(),life.is_some()].iter().filter(|v|**v).count()!=1{return None}
  if let Some(v)=publish{p::typed::<wire::Publish>("Publish",&serde_json::to_vec(v).ok()?).ok()?;uuid::Uuid::parse_str(v.event_id.as_deref()?).ok()?;}
  for v in [apply,receipt].into_iter().flatten(){uuid::Uuid::parse_str(v.event_id.as_deref()?).ok()?;let n=v.sequence.as_deref()?.parse::<u64>().ok()?;if n==0||n.to_string()!=v.sequence.as_deref()?{return None}}
  if let Some(v)=life{uuid::Uuid::parse_str(v.death_event_id.as_deref()?).ok()?;if v.state.as_deref()!=Some("playing"){return None}}
  if let Some(v)=apply{if v.state.is_some()||v.reason.is_some(){return None}}
  Some(value)
 }
}
pub(crate) fn world(c:&Context)->String{
 use sha2::{Digest,Sha256};let s=&c.binding.scope;
 let bytes=serde_json::to_vec(&json!([c.binding.service_origin,c.binding.service_id,c.binding.principal_id,s.room_id,s.run_id,s.run_epoch,s.world_token])).unwrap();
 Sha256::digest(bytes).iter().map(|b|format!("{b:02x}")).collect()
}
fn lobby(c:&Context)->crate::g3_journal::LobbyIdentity{crate::g3_journal::LobbyIdentity{service_origin:c.binding.service_origin.clone(),service_id:c.binding.service_id.clone(),principal_id:c.binding.principal_id.clone(),room_id:c.binding.scope.room_id.clone().unwrap_or_default()}}
fn request(c:&Context)->wire::Request{wire::Request{major:Some(1),revision:Some(p::REVISION.into()),service_id:Some(c.binding.service_id.clone()),service_epoch:Some(c.binding.service_epoch.clone()),session_epoch:Some(c.binding.session_epoch.clone()),request_id:Some(uuid::Uuid::new_v4().to_string()),room_id:c.binding.scope.room_id.clone(),scope:serde_json::from_value(serde_json::to_value(&c.binding.scope).unwrap()).ok(),owner_generation:Some(c.ownership_generation),..Default::default()}}
fn completion_order(mut rows:Vec<Record>)->Vec<Record>{
 let lives:Vec<_>=rows.iter().filter(|r|r.sequence=="life").cloned().collect();rows.retain(|r|r.sequence!="life");
 for life in lives.into_iter().rev(){let index=rows.iter().position(|r|r.sequence=="0"&&r.id==life.id).map(|i|i+1).unwrap_or(0);rows.insert(index,life);}rows
}
fn completion_record(row:&Record)->bool{(row.sequence=="0"&&row.original.get("payload").and_then(|p|p.get("death")).is_some())||row.sequence=="life"||row.sequence!="0"&&matches!(row.state.as_str(),"applied"|"skipped"|"uncertain")}
#[derive(Debug,PartialEq)]pub(crate) enum RetiredDisposition{Acknowledged,Unresolved,Uncertain}
pub(crate) fn finish_retired_source(local:&mut Journal,mut row:Record,result:Result<wire::SourceOutcome,String>)->Result<RetiredDisposition,String>{
 let value=result?;let disposition=match value.outcome.as_deref(){
  Some("unresolved")=>{row.state="unresolved".into();RetiredDisposition::Unresolved},
  Some("committed")=>{
   let actual=value.source_disposition.as_deref().ok_or("C1 retired source disposition absent")?;
   if actual=="pending"{return Err("C1 retired source receipt remains pending".into())}
   if actual==row.reason{row.state="acknowledged".into();RetiredDisposition::Acknowledged}
   else{row.state="uncertain".into();row.reason="source_save_outcome_unknown".into();RetiredDisposition::Uncertain}
  },_=>return Err("C1 retired source outcome absent".into())
 };local.put(row)?;Ok(disposition)
}
#[derive(Default)]struct State{world:String,snapshot:Option<wire::RuleSnapshot>,members:Vec<wire::Member>,event:Option<Value>,permit:Option<String>,journal:Option<Journal>,signal:Option<(u64,u64)>,dirty:bool,status:String,initialized:bool,life:Option<wire::LifeUpdate>,high_water:Option<String>,completed:std::collections::BTreeSet<String>,uncertain_reported:std::collections::BTreeSet<String>,owner_key:String,native_accepted:std::collections::VecDeque<String>,blocked:bool,source_uncertain:bool,retired_next:Option<Instant>,context_mode:u8,policy_at:Option<Instant>}
impl State{
 // Owner/world changes are invalidated before this call. A short same-owner
 // handoff to read-only bootstrap may retain freshly validated rules; effects
 // are always withdrawn. This does not renew policy time or game authority.
 fn change_mode(&mut self,mode:u8,owner_active:bool){
  if self.context_mode==mode{return}
  if self.context_mode==0&&mode!=0{self.initialized=mode==1&&owner_active&&self.initialized&&self.snapshot.is_some()&&self.policy_at.is_some_and(|at|Instant::now()<at);self.event=None;self.permit=None;self.dirty=true;}
  self.context_mode=mode;
 }
 fn policy_stale(&self)->bool{!self.initialized||self.context_mode==2&&self.policy_at.is_none_or(|at|Instant::now()>=at)||(self.event.is_some()||self.blocked)&&self.signal.is_some_and(|(_,sequence)|sequence>self.high_water.as_ref().and_then(|v|v.parse::<u64>().ok()).unwrap_or(0))}
 fn view(&self)->Value{json!({"enabled":true,"world":self.world,"snapshot":self.snapshot,"memberCount":self.members.len(),"event":self.event,"permit":self.permit,"initialized":self.initialized,"sequence":self.high_water,"status":self.status,"localAccepted":self.native_accepted})}
 fn response(&mut self,r:&wire::Response)->Result<(),String>{
  if let Some(sequence)=&r.sequence{let value=sequence.parse::<u64>().map_err(|_|"C1 response sequence")?;if self.high_water.as_ref().is_some_and(|old|old.parse::<u64>().unwrap_or(0)>value){return Err("C1 sequence regressed".into())}self.high_water=Some(sequence.clone());}
  if let Some(snapshot)=&r.snapshot{
   let next=snapshot.revision.ok_or("C1 rules revision")?;
   if let Some(old)=&self.snapshot{if next<old.revision.unwrap_or(0){return Err("C1 old rules response".into())}if next==old.revision.unwrap_or(0)&&serde_json::to_value(old).unwrap()!=serde_json::to_value(snapshot).unwrap(){return Err("C1 contradictory snapshot".into())}}
   self.snapshot=Some(snapshot.clone());self.initialized=true;self.policy_at=Some(Instant::now()+Duration::from_secs(2));
  }
  if let Some(members)=&r.members{let mut unique=std::collections::BTreeSet::new();for member in members{if !unique.insert(member.principal_id.as_deref().ok_or("C1 member identity")?){return Err("C1 duplicate membership".into())}}if r.member_count!=u32::try_from(members.len()).ok(){return Err("C1 roster count mismatch".into())}self.members=members.clone();}
  Ok(())
 }
}
#[derive(Default)]pub struct Work{state:State,task:Option<tokio::task::JoinHandle<State>>,next:Option<Instant>,view:Value}
impl Work{
 pub async fn shutdown(&mut self){if let Some(task)=self.task.take(){if let Ok(state)=task.await{self.state=state}}}
 pub fn view(&self)->Value{if self.task.is_some(){self.view.clone()}else{self.state.view()}}
 pub async fn retired_tick(&mut self,recovery:Option<(crate::g3_http::RecoveryOwner,crate::g3_journal::SharedJournal)>){
  if self.task.is_some()||self.next.is_some_and(|at|Instant::now()<at)||self.state.retired_next.is_some_and(|at|Instant::now()<at){return}let Some((mut owner,journal))=recovery else{return};
  if self.state.journal.as_ref().is_none_or(|j|!j.all_rows().iter().any(|r|r.sequence=="0"&&!matches!(r.state.as_str(),"uncertain"|"unresolved")&&r.original.get("payload").and_then(|p|p.get("death")).is_some())){return}
  self.state.retired_next=Some(Instant::now()+Duration::from_secs(2));self.view=self.state.view();let mut state=std::mem::take(&mut self.state);let runtime=tokio::runtime::Handle::current();
  self.task=Some(tokio::task::spawn_blocking(move||runtime.block_on(async move{
   if let Some(local)=state.journal.as_mut(){if let Err(error)=local.retire_deleted(&journal.deleted_lobbies()){state.status=error;return state;}for row in local.all_rows(){
    if row.sequence!="0"||matches!(row.state.as_str(),"uncertain"|"unresolved")||row.original.get("payload").and_then(|p|p.get("death")).is_none(){continue}let Some(source)=row.source.clone()else{continue};
    let result=owner.c1_source_outcome(&row.lobby,source,None).await;
    let result=match result{
     Ok(value)if value.outcome.as_deref()==Some("committed")&&value.source_disposition.as_deref()==Some("pending")=>{
      let receipt=wire::SourceDisposition{state:Some(row.reason.clone()),reason:None};owner.c1_source_outcome(&row.lobby,row.source.clone().unwrap(),Some(receipt)).await
     },other=>other
    };
    match finish_retired_source(local,row,result){
     Ok(RetiredDisposition::Acknowledged)=>state.status="Retired death report completed; saved effect remains unverified.".into(),
     Ok(RetiredDisposition::Unresolved)=>state.status="Death acceptance remains unresolved; no effect will replay.".into(),
     Ok(RetiredDisposition::Uncertain)=>{state.source_uncertain=true;state.status="Earlier shared-event save outcome is uncertain; it will not replay.".into();},
     Err(error)=>state.status=error,
    }
   }}state
  })));
 }
 pub async fn tick(&mut self,context:Option<Context>,stream:Arc<Mutex<Exchange>>){
  if self.task.as_ref().is_some_and(|t|t.is_finished()){if let Ok(state)=self.task.take().unwrap().await{self.state=state}self.next=Some(Instant::now()+Duration::from_millis(200));}
  let Some(c)=context else{self.state.initialized=false;self.state.policy_at=None;self.state.dirty=true;return};if self.task.is_some(){return}
  let owner_key=serde_json::to_string(&json!([c.binding.context,c.binding.scope.attachment_id,c.binding.session_epoch,c.ownership_generation])).unwrap();
  if owner_key!=self.state.owner_key{self.state.owner_key=owner_key;self.state.initialized=false;self.state.event=None;self.state.permit=None;self.state.dirty=true;}
  let key=world(&c);if key!=self.state.world{self.state.world=key;self.state.snapshot=None;self.state.initialized=false;self.state.event=None;self.state.permit=None;self.state.signal=None;self.state.high_water=None;self.state.completed.clear();self.state.uncertain_reported.clear();self.state.native_accepted.clear();self.state.blocked=false;self.state.source_uncertain=false;self.state.dirty=true;}
  let mode=if c.completion{2}else if c.bootstrap{1}else{0};
  self.state.change_mode(mode,c.owner.active().is_ok());
  let mut input=if c.bootstrap&&!c.completion{None}else{stream.lock().unwrap().c1_input(c.completion)};
  if let Some((seq,value,lane))=&input{
   let Input::C1{publish,receipt,life,..}=value;let key=publish.as_ref().and_then(|p|p.event_id.as_ref()).map(|id|format!("{id}:0")).or_else(||life.as_ref().and_then(|p|p.death_event_id.as_ref()).map(|id|format!("{id}:life"))).or_else(||receipt.as_ref().and_then(|p|p.event_id.as_ref().zip(p.sequence.as_ref())).map(|(id,sequence)|format!("{id}:{sequence}")));
   if key.is_some_and(|key|self.state.native_accepted.contains(&key)||self.state.completed.contains(&key)){stream.lock().unwrap().acknowledge_c1(c.binding.context,*seq,*lane);input=None;}
  }
  let signal=c.events.as_ref().and_then(|e|e.lock().unwrap().c1_signal(c.binding.context));
  if signal!=self.state.signal{self.state.signal=signal;self.state.dirty=true;}
  publish(&c,&self.state,&stream);
  if input.is_none()&&!self.state.dirty&&!self.state.policy_stale()&&(c.bootstrap||self.state.blocked||self.state.event.is_none()){return}
  if input.is_none()&&self.next.is_some_and(|t|Instant::now()<t){return}
  self.view=self.state.view();let state=std::mem::take(&mut self.state);let runtime=tokio::runtime::Handle::current();
  self.task=Some(tokio::task::spawn_blocking(move||runtime.block_on(run(c,state,input,stream))));
 }
}
fn publish(c:&Context,s:&State,stream:&Arc<Mutex<Exchange>>){if c.owner.active().is_err(){return}let mut x=stream.lock().unwrap();if c.bootstrap{x.c1_bootstrap_view(c.binding.context,c.ownership_generation,s.view());}else{x.c1_view_for(c.binding.context,s.view());}}
async fn run(mut c:Context,mut s:State,input:Option<(u64,Input,bool)>,stream:Arc<Mutex<Exchange>>)->State{
 let result=step(&mut c,&mut s,input.clone(),&stream).await;
 if let Err(error)=result{s.status=error;s.dirty=true;if c.owner.active().is_err(){s.initialized=false;s.policy_at=None;}}
 publish(&c,&s,&stream);s
}
async fn step(c:&mut Context,s:&mut State,input:Option<(u64,Input,bool)>,stream:&Arc<Mutex<Exchange>>)->Result<(),String>{
 c.owner.active()?;
 if c.bootstrap&&!c.completion{
  // This path cannot open a journal, accept an original, fetch a delivery,
  // issue an effect permit, acknowledge input, or change lifecycle authority.
  let mut r=request(c);r.rules_get=Some(Default::default());let reply=c.owner.c1(r).await?;
  s.response(&reply)?;s.dirty=false;if !s.blocked&&!s.source_uncertain{s.status.clear();}return Ok(())
 }
 if s.journal.is_none(){s.journal=Some(Journal::open(&c.consumer.journal.c1_path())?);}
 s.journal.as_mut().unwrap().retire_deleted(&c.consumer.journal.deleted_lobbies())?;
 if c.consumer.journal.lobby_deleted(&lobby(c)){s.event=None;s.permit=None;s.status="Lobby deleted; shared player events removed.".into();return Ok(())}
 if let Some((seq,input,lane))=input{
  if c.completion&&!input.completion_allowed(){return Err("C1 protected input refused".into())}
  let Input::C1{publish,apply,receipt,life}=input;
  let acceptance=publish.as_ref().and_then(|p|p.event_id.as_ref()).map(|id|format!("{id}:0")).or_else(||life.as_ref().and_then(|l|l.death_event_id.as_ref()).map(|id|format!("{id}:life"))).or_else(||receipt.as_ref().and_then(|p|p.event_id.as_ref().zip(p.sequence.as_ref())).map(|(id,sequence)|format!("{id}:{sequence}")));
  if acceptance.as_ref().is_some_and(|key|s.native_accepted.contains(key)){stream.lock().unwrap().acknowledge_c1(c.binding.context,seq,lane);return Ok(())}
  let journal=s.journal.as_mut().unwrap();
  if let Some(mut v)=publish{
   let disposition=v.sender_disposition.take().unwrap_or_else(||"uncertain".into());v.sender_disposition=Some("pending".into());
   let id=v.event_id.clone().ok_or("C1 original ID")?;let original=serde_json::to_value(&v).unwrap();
   let source=journal.get(&s.world,&id,"0").and_then(|r|r.source).or_else(||Some(json!({"roomId":c.binding.scope.room_id,"runId":c.binding.scope.run_id,"worldToken":c.binding.scope.world_token,"configRevision":c.binding.scope.config_revision,"publish":original})));
   journal.put(Record{ordinal:0,source,lobby:lobby(c),world:s.world.clone(),id,sequence:"0".into(),state:"pending".into(),reason:disposition,original})?;
  }else if let Some(v)=apply{
   let id=v.event_id.as_deref().ok_or("C1 apply ID")?;let sequence=v.sequence.as_deref().ok_or("C1 apply sequence")?;
   let mut row=journal.get(&s.world,id,sequence).ok_or("C1 apply has no original")?;
   if row.state=="pending"{row.state="applying".into();journal.put(row)?;s.permit=Some(uuid::Uuid::new_v4().to_string());}
  }else if let Some(v)=receipt{
   let completed=format!("{}:{}",v.event_id.as_deref().unwrap_or_default(),v.sequence.as_deref().unwrap_or_default());
   if s.completed.contains(&completed){stream.lock().unwrap().acknowledge_c1(c.binding.context,seq,lane);return Ok(())}
   let mut row=journal.get(&s.world,v.event_id.as_deref().ok_or("C1 receipt ID")?,v.sequence.as_deref().ok_or("C1 receipt sequence")?).ok_or("C1 receipt has no original")?;
   if !matches!(v.state.as_deref(),Some("applied"|"skipped"|"uncertain")){return Err("C1 receipt state".into())}
   if row.state=="pending"&&v.state.as_deref()!=Some("uncertain")||!matches!(row.state.as_str(),"applying"|"pending")&&row.state!=v.state.as_deref().unwrap(){return Err("C1 contradictory effect result".into())}
   row.state=v.state.unwrap();row.reason=v.reason.unwrap_or_default();journal.put(row)?;s.permit=None;
  }else if let Some(v)=life{
   journal.put(Record{ordinal:0,source:None,lobby:lobby(c),world:s.world.clone(),id:v.death_event_id.clone().ok_or("C1 life death ID")?,sequence:"life".into(),state:"pending".into(),reason:String::new(),original:serde_json::to_value(v).unwrap()})?;
  }
  if let Some(key)=acceptance{s.native_accepted.push_back(key);while s.native_accepted.len()>32{s.native_accepted.pop_front();}}
  // ACK only after local fsync. The game thread never waits for an RPC.
  stream.lock().unwrap().acknowledge_c1(c.binding.context,seq,lane);s.dirty=true;
 }
 if s.policy_stale(){
  let mut r=request(c);r.rules_get=Some(Default::default());let reply=c.owner.c1(r).await?;s.response(&reply)?;
 }
 let rows=completion_order(s.journal.as_ref().unwrap().rows(&s.world));
 for mut row in rows{
  if c.completion&&!completion_record(&row){continue}
  if s.blocked&&row.sequence!="0"&&row.sequence!="life"&&row.state!="uncertain"{continue}
  if row.sequence=="life"{
   let mut r=request(c);r.life=Some(serde_json::from_value(row.original.clone()).map_err(|_|"C1 life encoding")?);let reply=c.owner.c1(r).await?;s.response(&reply)?;row.state="acknowledged".into();s.journal.as_mut().unwrap().put(row)?;
  }else if row.sequence=="0"{
   if row.state=="uncertain"{s.source_uncertain=true;continue}
   let mut r=request(c);r.publish=Some(serde_json::from_value(row.original.clone()).map_err(|_|"C1 original encoding")?);
   let response=c.owner.c1(r).await?;s.response(&response)?;
   if response.event.as_ref().and_then(|e|e.sender_disposition.as_deref()).is_some_and(|state|state!="pending"&&state!=row.reason){
    row.state="uncertain".into();row.reason="source_save_outcome_unknown".into();s.journal.as_mut().unwrap().put(row)?;s.source_uncertain=true;continue
   }
   let mut r=request(c);r.receipt=Some(wire::Receipt{event_id:Some(row.id.clone()),sequence:Some("0".into()),state:Some(row.reason.clone()),reason:None});let reply=c.owner.c1(r).await?;s.response(&reply)?;
   if row.reason=="uncertain"{s.source_uncertain=true;}row.state="acknowledged".into();s.journal.as_mut().unwrap().put(row)?;
  }else if matches!(row.state.as_str(),"applied"|"skipped"|"uncertain"){
   let mut r=request(c);r.receipt=Some(wire::Receipt{event_id:Some(row.id.clone()),sequence:Some(row.sequence.clone()),state:Some(row.state.clone()),reason:(row.state=="skipped").then(||row.reason.clone())});
   if row.state=="uncertain"{
    let key=format!("{}:{}",row.id,row.sequence);if !s.uncertain_reported.contains(&key){if let Ok(reply)=c.owner.c1(r).await{s.response(&reply)?;s.uncertain_reported.insert(key);}}
    s.blocked=true;s.event=None;s.permit=None;s.status="C1 effect uncertain; do not repeat. Inspect the same world's save.".into();continue
   }
   let reply=c.owner.c1(r).await?;s.response(&reply)?;
   if row.state!="uncertain"{s.completed.insert(format!("{}:{}",row.id,row.sequence));if s.completed.len()>32{let first=s.completed.first().unwrap().clone();s.completed.remove(&first);}row.state="acknowledged".into();s.journal.as_mut().unwrap().put(row)?;s.event=None;s.permit=None;}else{s.status="C1 effect uncertain; do not repeat. Inspect the same world's save.".into();return Ok(())}
  }else if row.state=="applying"&&s.permit.is_none(){
   row.state="uncertain".into();row.reason="ambiguous_restore".into();s.journal.as_mut().unwrap().put(row)?;s.dirty=true;return Ok(())
  }
 }
 if c.completion{s.event=None;s.permit=None;s.dirty=false;return Ok(())}
 if let Some(life)=s.life.clone(){let mut r=request(c);r.life=Some(life);let reply=c.owner.c1(r).await?;s.response(&reply)?;s.life=None;}
 if s.event.is_none()&&!s.blocked{
  let mut r=request(c);r.deliveries=Some(wire::Page{limit:Some(1)});let reply=c.owner.c1(r).await?;s.response(&reply)?;
  if let Some(event)=reply.deliveries.as_ref().and_then(|v|v.first()){
   let id=event.event_id.clone().ok_or("C1 delivery ID")?;let sequence=event.sequence.clone().ok_or("C1 delivery sequence")?;
   let original=serde_json::to_value(event).unwrap();let journal=s.journal.as_mut().unwrap();
   if journal.get(&s.world,&id,&sequence).is_none(){journal.put(Record{ordinal:0,source:None,lobby:lobby(c),world:s.world.clone(),id,sequence,state:"pending".into(),reason:String::new(),original:original.clone()})?;}
   s.event=Some(original);
  }
  s.dirty=reply.more==Some(true);
 }else{s.dirty=false;}
 if s.source_uncertain{s.status="Earlier shared-event save outcome is uncertain; it will not replay.".into();}else if !s.blocked{s.status.clear();}Ok(())
}

#[cfg(test)]mod tests{
 use super::*;
 #[test]fn c1_authority_gap_invalidates_policy_without_erasing_retained_effect(){tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async{
  let mut work=Work::default();work.state.initialized=true;work.state.policy_at=Some(Instant::now()+Duration::from_secs(2));work.state.event=Some(json!({"eventId":"retained"}));
  work.tick(None,Arc::new(Mutex::new(Exchange::default()))).await;
  assert!(!work.state.initialized);assert!(work.state.policy_stale());assert!(work.state.dirty);assert_eq!(work.state.event.as_ref().unwrap()["eventId"],"retained");
 });}
 #[test]fn c1_same_owner_bootstrap_keeps_only_current_readonly_policy_and_withdraws_effects(){
  for case in 0..5{let mut s=State::default();s.initialized=true;s.snapshot=Some(wire::RuleSnapshot::default());s.policy_at=Some(Instant::now()+Duration::from_secs(2));s.event=Some(json!({"eventId":"never-replay"}));s.permit=Some("never-reuse".into());let original_time=s.policy_at;
   if case==1{s.policy_at=Some(Instant::now())}if case==2{s.snapshot=None}if case==4{s.initialized=false}
   s.change_mode(1,case!=3);assert_eq!(s.initialized,case==0);assert!(s.event.is_none()&&s.permit.is_none()&&s.dirty);if case==0{assert_eq!(s.policy_at,original_time,"A metadata handoff cannot refresh policy age");s.change_mode(0,true);s.change_mode(2,true);assert!(!s.initialized,"Protected completion cannot inherit gameplay policy");}
  }
 }
 #[test]fn c1_life_completion_precedes_post_respawn_pickups_after_its_death_original(){
  let row=|id:&str,sequence:&str,ordinal:u64|Record{ordinal,source:None,lobby:crate::g3_journal::LobbyIdentity{service_origin:"https://fixture.invalid".into(),service_id:"service".into(),principal_id:"principal".into(),room_id:"room".into()},world:"a".repeat(64),id:id.into(),sequence:sequence.into(),state:"pending".into(),reason:String::new(),original:json!({})};
  let ordered=completion_order(vec![row("before","0",1),row("death","0",2),row("after","0",3),row("death","life",4)]);assert_eq!(ordered.iter().map(|r|(r.id.as_str(),r.sequence.as_str())).collect::<Vec<_>>(),vec![("before","0"),("death","0"),("death","life"),("after","0")]);
  let ordered=completion_order(vec![row("after","0",3),row("death","life",4)]);assert_eq!(ordered[0].sequence,"life");
 }
 #[test]fn c1_local_permit_and_receipt_do_not_poll_rules_without_new_policy_evidence(){
  let mut s=State::default();assert!(s.policy_stale());s.initialized=true;s.dirty=true;s.signal=Some((1,2));s.high_water=Some("2".into());s.event=Some(json!({"payload":{"heart":{}}}));assert!(!s.policy_stale());
  s.signal=Some((1,3));assert!(s.policy_stale());s.high_water=Some("3".into());assert!(!s.policy_stale());s.event=None;s.signal=Some((1,4));assert!(!s.policy_stale(),"The next delivery fetch already carries the policy/projection");s.blocked=true;assert!(s.policy_stale(),"A blocked effect must still receive live rules");s.blocked=false;s.context_mode=2;s.policy_at=Some(Instant::now());assert!(s.policy_stale(),"Protected post-entry mode must refresh live policy without a gameplay WSS owner");s.context_mode=1;assert!(!s.policy_stale(),"Initial bootstrap does not start a background poll");
 }
 #[test]fn c1_protected_replay_never_publishes_pickups_or_issues_unissued_effects(){
  let mut row:Record=serde_json::from_value(json!({"ordinal":1,"source":null,"lobby":{"service_origin":"https://fixture.invalid","service_id":"service","principal_id":"principal","room_id":"room"},"world":"a".repeat(64),"id":"11111111-1111-4111-8111-111111111111","sequence":"0","state":"pending","reason":"applied","original":{"payload":{"heart":{}}}})).unwrap();
  for payload in [json!({"heart":{}}),json!({"perk":{"perkId":"EXTRA_MONEY"}}),json!({"orb":{"orbId":0}}),json!({"fungal":{"fromMaterials":["water"],"toMaterial":"oil"}})]{row.original=json!({"payload":payload});assert!(!completion_record(&row));}
  row.original=json!({"payload":{"death":{"state":"respawning","cause":"normal"}}});assert!(completion_record(&row));
  row.sequence="1".into();for state in ["pending","applying"]{row.state=state.into();assert!(!completion_record(&row));}for state in ["applied","skipped","uncertain"]{row.state=state.into();assert!(completion_record(&row));}
  row.sequence="life".into();row.state="pending".into();assert!(completion_record(&row));
 }
 #[test]fn c1_local_input_has_one_closed_command_and_canonical_original(){
  let source=json!({"kind":"c1","publish":{"eventId":"11111111-1111-4111-8111-111111111111","rulesRevision":1,"payload":{"heart":{}},"senderDisposition":"applied"}});
  assert!(Input::decode(&serde_json::to_vec(&source).unwrap()).is_some());
  for body in [json!({"kind":"c1"}),json!({"kind":"c1","life":{"deathEventId":"bad","state":"playing"}}),json!({"kind":"c1","life":{"deathEventId":"11111111-1111-4111-8111-111111111111","state":"respawning"}}),json!({"kind":"c1","apply":{"eventId":"11111111-1111-4111-8111-111111111111","sequence":"01"}}),json!({"kind":"c1","apply":{"eventId":"11111111-1111-4111-8111-111111111111","sequence":"1","state":"applied"}})]{assert!(Input::decode(&serde_json::to_vec(&body).unwrap()).is_none());}
  let mut two=source.clone();two["life"]=json!({"deathEventId":"11111111-1111-4111-8111-111111111111","state":"playing"});assert!(Input::decode(&serde_json::to_vec(&two).unwrap()).is_none());
 }
}
