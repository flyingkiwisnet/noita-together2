//! Optional C2 lane on the existing scheduler/native copy transport.
use crate::{protocol_c2::{self as p,wire},c1_journal::{Journal,Record},g3_social_stream::Exchange};
use serde::Deserialize;
use serde_json::{json,Value};
use std::{sync::{Arc,Mutex},time::{Duration,Instant}};
pub type Context=crate::c1_work::Context;
#[derive(Clone,Debug,Deserialize)]
#[serde(tag="kind",rename_all="snake_case",deny_unknown_fields)]
pub enum Input{#[serde(alias="c2_completion")]C2{state:Option<wire::State>,arena_bind:Option<wire::ArenaBind>,near:Option<wire::Near>,publish:Option<wire::Publish>,apply:Option<wire::Receipt>,receipt:Option<wire::Receipt>}}
impl Input{
 pub fn protected_allowed(&self,completion:bool)->bool{let Self::C2{state,arena_bind,publish,apply,near,receipt}=self;
  state.is_some()||arena_bind.is_some()||completion&&near.is_none()&&apply.is_none()&&(publish.as_ref().is_some_and(|v|v.payload.as_ref().is_some_and(|v|v.completed.is_some()))||receipt.as_ref().is_some_and(|v|v.sequence.as_deref()==Some("0")))
 }
 pub fn decode(bytes:&[u8])->Option<Self>{
  if bytes.len()>4096{return None}let v:Self=serde_json::from_slice(bytes).ok()?;let Self::C2{state,arena_bind,near,publish,apply,receipt}=&v;
  if [state.is_some(),arena_bind.is_some(),near.is_some(),publish.is_some(),apply.is_some(),receipt.is_some()].iter().filter(|x|**x).count()!=1{return None}
  macro_rules! check{($value:expr,$ty:ty,$name:literal)=>{if let Some(x)=$value{p::typed::<$ty>($name,&serde_json::to_vec(x).ok()?).ok()?;}}}
  check!(state,wire::State,"State");check!(arena_bind,wire::ArenaBind,"ArenaBind");check!(near,wire::Near,"Near");check!(publish,wire::Publish,"Publish");check!(receipt,wire::Receipt,"Receipt");
  if let Some(v)=apply{if v.state.is_some()||v.reason.is_some(){return None}uuid::Uuid::parse_str(v.event_id.as_deref()?).ok()?;let n=v.sequence.as_deref()?.parse::<u64>().ok()?;if n==0||n.to_string()!=v.sequence.as_deref()?{return None}}
  if state.as_ref().is_some_and(|v|v.progression.is_none())||arena_bind.as_ref().is_some_and(|v|v.progression.is_none()||v.arena_token.as_ref().is_none_or(|v|uuid::Uuid::parse_str(v).is_err())){return None}
  if let Some(v)=near{v.near?;for id in [&v.encounter_id,&v.arena_token,&v.readiness_epoch]{uuid::Uuid::parse_str(id.as_deref()?).ok()?;}}
  if let Some(v)=publish{v.progression?;uuid::Uuid::parse_str(v.event_id.as_deref()?).ok()?;v.rules_revision?;}
  if let Some(v)=receipt{if !matches!(v.state.as_deref(),Some("applied"|"skipped"|"uncertain")){return None}}
  Some(v)
 }
}
fn world(c:&Context)->String{use sha2::{Digest,Sha256};let s=&c.binding.scope;Sha256::digest(serde_json::to_vec(&json!([c.binding.service_origin,c.binding.service_id,c.binding.principal_id,s.room_id,s.run_id,s.run_epoch,s.world_token])).unwrap()).iter().map(|b|format!("{b:02x}")).collect()}
fn lobby(c:&Context)->crate::g3_journal::LobbyIdentity{crate::g3_journal::LobbyIdentity{service_origin:c.binding.service_origin.clone(),service_id:c.binding.service_id.clone(),principal_id:c.binding.principal_id.clone(),room_id:c.binding.scope.room_id.clone().unwrap_or_default()}}
fn request(c:&Context)->wire::Request{wire::Request{major:Some(1),revision:Some(p::REVISION.into()),service_id:Some(c.binding.service_id.clone()),service_epoch:Some(c.binding.service_epoch.clone()),session_epoch:Some(c.binding.session_epoch.clone()),request_id:Some(uuid::Uuid::new_v4().to_string()),room_id:c.binding.scope.room_id.clone(),scope:serde_json::from_value(serde_json::to_value(&c.binding.scope).unwrap()).ok(),owner_generation:Some(c.ownership_generation),..Default::default()}}
#[derive(Default)]struct State{valid:bool,world:String,owner:String,principal:String,world_token:String,progression:Option<u32>,snapshot:Option<wire::Snapshot>,journal:Option<Journal>,event:Option<Value>,permit:Option<String>,accepted:std::collections::VecDeque<String>,blocked:bool,status:String,polled:Option<Instant>,signal:Option<(u64,u64)>,page:usize,retired_next:Option<Instant>,bound:Option<wire::ArenaBind>}
impl State{
 fn response(&mut self,r:&wire::Response)->Result<(),String>{
  if let Some(incoming)=&r.snapshot{
   // Mutation receipts may carry room-only metadata. Absence of encounter is
   // not a reset of this explicitly selected progression.
   let mut projected=incoming.clone();if projected.encounter.is_none(){projected.encounter=self.snapshot.as_ref().and_then(|v|v.encounter.clone());}let snapshot=&projected;
   if let Some(old)=&self.snapshot{
    let next=snapshot.sequence.as_deref().ok_or("C2 snapshot sequence")?.parse::<u64>().map_err(|_|"C2 sequence")?;
    let previous=old.sequence.as_deref().unwrap_or("0").parse::<u64>().unwrap_or(0);if next<previous{return Err("C2 snapshot regressed".into())}
    if next==previous{let mut before=old.clone();before.encounter=None;let mut after=incoming.clone();after.encounter=None;if serde_json::to_value(before).unwrap()!=serde_json::to_value(after).unwrap(){return Err("C2 contradictory room snapshot".into())}}
    if let(Some(before),Some(after))=(&old.encounter,&incoming.encounter){
     if before.encounter_id!=after.encounter_id||before.progression!=after.progression{return Err("C2 encounter identity changed".into())}
     let prior=before.revision.as_deref().ok_or("C2 encounter revision")?.parse::<u64>().map_err(|_|"C2 encounter revision")?;let revision=after.revision.as_deref().ok_or("C2 encounter revision")?.parse::<u64>().map_err(|_|"C2 encounter revision")?;
     if revision<prior||revision==prior&&serde_json::to_value(before).unwrap()!=serde_json::to_value(after).unwrap(){return Err("C2 encounter projection regressed or contradicted".into())}
     let rank=|stage:Option<&str>|match stage{Some("locked")=>0,Some("unlocked")=>1,Some("battle")=>2,_=>-1};if rank(after.stage.as_deref())<rank(before.stage.as_deref()){return Err("C2 encounter stage regressed".into())}
    }
   }
   self.valid=true;self.snapshot=Some(snapshot.clone());self.polled=Some(Instant::now());
  }Ok(())
 }
 fn view(&self)->Value{
  let mut snapshot=serde_json::to_value(&self.snapshot).unwrap();let members=snapshot["encounter"]["participants"].as_array().cloned().unwrap_or_default();let pages=members.len().div_ceil(4).max(1);let page=self.page%pages;
  if snapshot["encounter"].is_object(){snapshot["encounter"]["participants"]=json!(members.iter().skip(page*4).take(4).cloned().collect::<Vec<_>>());}
  let completions=snapshot["completions"].as_array().cloned().unwrap_or_default();let completion_pages=completions.len().div_ceil(4).max(1);let completion_page=self.page%completion_pages;
  if snapshot.is_object(){snapshot["completions"]=json!(completions.iter().skip(completion_page*4).take(4).cloned().collect::<Vec<_>>());}
  json!({"enabled":true,"world":self.world,"principal":self.principal,"worldToken":self.world_token,"progression":self.progression,"snapshot":snapshot,"participantsPage":page,"participantsPages":pages,"participantsTotal":members.len(),"completionsPage":completion_page,"completionsPages":completion_pages,"completionsTotal":completions.len(),"bound":self.bound,"event":self.event,"permit":self.permit,"localAccepted":self.accepted,"initialized":self.valid&&self.snapshot.is_some(),"status":self.status})
 }
}
#[derive(Default)]pub struct Work{state:State,task:Option<tokio::task::JoinHandle<State>>,next:Option<Instant>,view:Value}
impl Work{
 pub fn view(&self)->Value{if self.task.is_some(){self.view.clone()}else{self.state.view()}}
 pub async fn shutdown(&mut self){if let Some(task)=self.task.take(){if let Ok(state)=task.await{self.state=state}}}
 pub fn retired_due(&mut self,journal:&crate::g3_journal::SharedJournal)->bool{
  if self.task.is_some()||self.state.retired_next.is_some_and(|t|Instant::now()<t){return false}self.state.retired_next=Some(Instant::now()+Duration::from_secs(2));
  if self.state.journal.is_none(){match Journal::open(&journal.c2_path()){Ok(j)=>self.state.journal=Some(j),Err(e)=>{self.state.status=e;return false}}}
  let local=self.state.journal.as_mut().unwrap();if let Err(e)=local.retire_deleted(&journal.deleted_lobbies()){self.state.status=e;return false}local.all_rows().iter().any(|r|r.sequence=="0"&&r.state=="pending")
 }
 pub async fn retired_tick(&mut self,recovery:Option<(crate::g3_http::RecoveryOwner,crate::g3_journal::SharedJournal)>){
  if self.task.is_some(){return}let Some((mut owner,journal))=recovery else{return};
  if self.state.journal.is_none(){match Journal::open(&journal.c2_path()){Ok(j)=>self.state.journal=Some(j),Err(e)=>{self.state.status=e;return}}}
  if !self.state.journal.as_ref().unwrap().all_rows().iter().any(|r|r.sequence=="0"&&r.state=="pending"){return}
  self.state.retired_next=Some(Instant::now()+Duration::from_secs(2));self.view=self.state.view();let mut state=std::mem::take(&mut self.state);let runtime=tokio::runtime::Handle::current();
  self.task=Some(tokio::task::spawn_blocking(move||runtime.block_on(async move{
   let local=state.journal.as_mut().unwrap();if let Err(e)=local.retire_deleted(&journal.deleted_lobbies()){state.status=e;return state}
   for mut row in local.all_rows(){if row.sequence!="0"||row.state!="pending"{continue}let Some(original)=row.source.clone()else{continue};
    let result=owner.c2_source_outcome(&row.lobby,original.clone(),None).await;
    let result=match result{Ok(v)if v.committed==Some(true)&&v.disposition.as_deref()==Some("pending")=>owner.c2_source_outcome(&row.lobby,original,Some(wire::SourceReceipt{state:Some(row.reason.clone()),reason:None})).await,other=>other};
    match result{Ok(v)=>{if v.committed!=Some(true){row.state="unresolved".into();state.status="C2 acceptance absent; original will not replay.".into();}else if v.disposition.as_deref()==Some("uncertain")||row.reason=="uncertain"{row.state="uncertain".into();state.blocked=true;state.status="C2 saved source outcome uncertain; no effect will replay.".into();}else if v.disposition.as_deref()==Some(&row.reason){row.state="acknowledged".into();}else{state.status="C2 source disposition contradicts saved original.".into();continue}if let Err(e)=local.put(row){state.status=e;}},Err(e)=>state.status=e}
   }state
  })));
 }
 pub async fn collect(&mut self){if self.task.as_ref().is_some_and(|t|t.is_finished()){if let Ok(s)=self.task.take().unwrap().await{self.state=s;}self.next=Some(Instant::now()+Duration::from_millis(200));}}
 pub fn source_recovery_needed(&self)->bool{self.task.is_none()&&self.state.journal.as_ref().is_some_and(|j|j.all_rows().iter().any(|r|r.sequence=="0"&&r.state=="pending"&&r.original.get("progression").and_then(Value::as_u64)!=self.state.progression.map(u64::from)))}
 pub async fn tick(&mut self,context:Option<Context>,stream:Arc<Mutex<Exchange>>,recovery:Option<crate::g3_http::RecoveryOwner>){
  self.collect().await;
  let Some(c)=context else{self.state.valid=false;self.state.polled=None;return};if self.task.is_some(){return}
  let key=world(&c);if self.state.world!=key{self.state=State{world:key,principal:c.binding.principal_id.clone(),world_token:c.binding.scope.world_token.clone().unwrap_or_default(),..Default::default()};}
  let owner=format!("{:?}:{}:{}",c.binding.context,c.ownership_generation,c.completion);if self.state.owner!=owner{self.state.owner=owner;self.state.valid=false;self.state.polled=None;self.state.event=None;self.state.permit=None;self.state.accepted.clear();}
  self.state.page=self.state.page.wrapping_add(1);let view=self.state.view();if c.owner.active().is_ok(){stream.lock().unwrap().c2_view_for(c.binding.context,view,c.bootstrap);}
  let input=stream.lock().unwrap().c2_input(c.bootstrap,c.completion);let signal=c.events.as_ref().and_then(|e|e.lock().unwrap().c2_signal(c.binding.context));
  let changed=self.state.signal!=signal;self.state.signal=signal;
  if input.is_none()&&!changed&&self.next.is_some_and(|t|Instant::now()<t){return}
  if input.is_none()&&!changed&&self.state.polled.is_some_and(|at|at.elapsed()<Duration::from_secs(1))&&self.state.event.is_none(){return}
  self.view=self.state.view();let state=std::mem::take(&mut self.state);let runtime=tokio::runtime::Handle::current();self.task=Some(tokio::task::spawn_blocking(move||runtime.block_on(async move{
   let mut c=c;let mut s=state;let mut recovery=recovery;let result=step(&mut c,&mut s,input,&stream,&mut recovery).await;if let Err(e)=result{s.status=e;}
   if c.owner.active().is_ok(){stream.lock().unwrap().c2_view_for(c.binding.context,s.view(),c.bootstrap);}s
  })));
 }
}
async fn step(c:&mut Context,s:&mut State,input:Option<(u64,Input,bool)>,stream:&Arc<Mutex<Exchange>>,recovery:&mut Option<crate::g3_http::RecoveryOwner>)->Result<(),String>{
 if s.journal.is_none()&&!c.bootstrap{s.journal=Some(Journal::open(&c.consumer.journal.c2_path())?)}
 if let Some(j)=s.journal.as_mut(){j.retire_deleted(&c.consumer.journal.deleted_lobbies())?;}
 if c.consumer.journal.lobby_deleted(&lobby(c)){s.event=None;s.permit=None;s.status="Lobby deleted; C2 work removed.".into();return Ok(())}
 if let Some((seq,input,lane))=input{
  if c.bootstrap&&!input.protected_allowed(c.completion){return Err("C2 protected mutation refused".into())}
  let Input::C2{state,arena_bind,near,publish,apply,receipt}=input;
  let mut q=request(c);
  if let Some(v)=state{if s.progression!=v.progression{s.snapshot=None;s.bound=None;}s.progression=v.progression;q.state=Some(v);let r=c.owner.c2(q).await?;s.response(&r)?;}
  else if let Some(v)=arena_bind{q.arena_bind=Some(v.clone());let r=c.owner.c2(q).await?;s.response(&r)?;s.bound=Some(v);}
  else if let Some(v)=near{
   q.near=Some(v.clone());match c.owner.c2(q).await{Ok(r)=>s.response(&r)?,Err(error)=>{
    if error!="CONFLICT"{return Err(error)}let mut refresh=request(c);refresh.state=Some(wire::State{progression:s.progression});let r=c.owner.c2(refresh).await?;s.response(&r)?;
    let current=s.snapshot.as_ref().and_then(|x|x.encounter.as_ref()).and_then(|x|x.participants.as_ref()).and_then(|v|v.iter().find(|m|m.principal_id.as_deref()==Some(&s.principal)&&m.world_token.as_deref()==Some(&s.world_token))).and_then(|m|m.readiness_epoch.as_ref());
    if current==v.readiness_epoch.as_ref(){return Err(error)}
   }}
  }
  else{
   if s.journal.is_none(){s.journal=Some(Journal::open(&c.consumer.journal.c2_path())?)}
   let journal=s.journal.as_mut().unwrap();
   let accepted=publish.as_ref().and_then(|v|v.event_id.clone()).map(|id|format!("{id}:0")).or_else(||receipt.as_ref().and_then(|r|r.event_id.as_ref().zip(r.sequence.as_ref())).map(|(id,seq)|format!("{id}:{seq}")));
   if !accepted.as_ref().is_some_and(|id|s.accepted.contains(id)){
    if let Some(mut v)=publish{
     let disposition=v.sender_disposition.take().unwrap_or_else(||"uncertain".into());v.sender_disposition=Some("pending".into());let id=v.event_id.clone().ok_or("C2 original ID")?;let original=serde_json::to_value(&v).unwrap();
     let source=journal.get(&s.world,&id,"0").and_then(|r|r.source).or_else(||Some(json!({"roomId":c.binding.scope.room_id,"runId":c.binding.scope.run_id,"worldToken":c.binding.scope.world_token,"configRevision":c.binding.scope.config_revision,"publish":original})));
     journal.put(Record{ordinal:0,lobby:lobby(c),world:s.world.clone(),id,sequence:"0".into(),source,state:"pending".into(),reason:disposition,original})?;
    }else if let Some(v)=apply{
     let mut row=journal.get(&s.world,v.event_id.as_deref().ok_or("C2 apply ID")?,v.sequence.as_deref().ok_or("C2 sequence")?).ok_or("C2 apply without delivery")?;
     if row.state=="pending"{row.state="applying".into();journal.put(row)?;s.permit=Some(uuid::Uuid::new_v4().to_string());}
    }else if let Some(v)=receipt{
     let mut row=journal.get(&s.world,v.event_id.as_deref().ok_or("C2 receipt ID")?,v.sequence.as_deref().ok_or("C2 sequence")?).ok_or("C2 receipt without delivery")?;
     let state=v.state.ok_or("C2 receipt state")?;if !matches!(row.state.as_str(),"pending"|"applying")&&row.state!=state{return Err("C2 contradictory receipt".into())}if row.state=="pending"&&state!="uncertain"{return Err("C2 effect not issued".into())}
     row.state=state;row.reason=v.reason.unwrap_or_default();journal.put(row)?;s.permit=None;
    }
    if let Some(id)=accepted{s.accepted.push_back(id);while s.accepted.len()>32{s.accepted.pop_front();}}
   }
  }
  stream.lock().unwrap().acknowledge_c1(c.binding.context,seq,lane);
 }
 let mut q=request(c);if let Some(progression)=s.progression{q.state=Some(wire::State{progression:Some(progression)});}else{q.rules_get=Some(Default::default());}
 s.response(&c.owner.c2(q).await?)?;
 if c.bootstrap&&!c.completion{return Ok(())}
 let Some(journal)=s.journal.as_mut()else{return Ok(())};let rows=journal.rows(&s.world);
 for mut row in rows{
  let mut q=request(c);
  if row.sequence=="0"{
   if row.original.get("progression").and_then(Value::as_u64)!=s.progression.map(u64::from){
    if row.state!="pending"{continue}let Some(owner)=recovery.as_mut()else{continue};let original=row.source.clone().ok_or("C2 old-progression source absent")?;
    let result=owner.c2_source_outcome(&row.lobby,original.clone(),None).await?;
    let result=if result.committed==Some(true)&&result.disposition.as_deref()==Some("pending"){owner.c2_source_outcome(&row.lobby,original,Some(wire::SourceReceipt{state:Some(row.reason.clone()),reason:None})).await?}else{result};
    row.state=if result.committed!=Some(true){"unresolved"}else if result.disposition.as_deref()==Some("uncertain")||row.reason=="uncertain"{"quarantined"}else if result.disposition.as_deref()==Some(&row.reason){"acknowledged"}else{return Err("C2 old-progression source disposition differs".into())}.into();
    s.journal.as_mut().unwrap().put(row)?;continue
   }
   if row.state!="pending"{if matches!(row.state.as_str(),"uncertain"|"quarantined"){s.blocked=true;}continue}q.publish=Some(serde_json::from_value(row.original.clone()).map_err(|_|"C2 original encoding")?);let r=c.owner.c2(q).await?;s.response(&r)?;
   if let Some(disposition)=r.event.as_ref().and_then(|v|v.sender_disposition.as_deref()){if disposition!="pending"&&disposition!=row.reason{if row.reason=="uncertain"{row.state="quarantined".into();s.blocked=true;s.status="C2 saved source outcome uncertain; original will not replay.".into();s.journal.as_mut().unwrap().put(row)?;continue}return Err("C2 source disposition contradicts saved original".into())}}
   let mut q=request(c);q.receipt=Some(wire::Receipt{event_id:Some(row.id.clone()),sequence:Some("0".into()),state:Some(row.reason.clone()),reason:None});s.response(&c.owner.c2(q).await?)?;if row.reason=="uncertain"{s.blocked=true;s.status="C2 source save outcome uncertain; no effect will replay.".into();row.state="quarantined".into();}else{row.state="acknowledged".into();}s.journal.as_mut().unwrap().put(row)?;
  }else if matches!(row.state.as_str(),"applied"|"skipped"|"uncertain"){
   q.receipt=Some(wire::Receipt{event_id:Some(row.id.clone()),sequence:Some(row.sequence.clone()),state:Some(row.state.clone()),reason:(row.state=="skipped").then(||row.reason.clone())});s.response(&c.owner.c2(q).await?)?;
   if row.state=="uncertain"{s.blocked=true;s.event=None;s.permit=None;s.status="C2 effect uncertain; it will not replay.".into();row.state="quarantined".into();s.journal.as_mut().unwrap().put(row)?;continue}
   row.state="acknowledged".into();s.journal.as_mut().unwrap().put(row)?;s.event=None;s.permit=None;
  }else if row.state=="applying"&&s.permit.is_none(){row.state="uncertain".into();row.reason="ambiguous_restore".into();s.journal.as_mut().unwrap().put(row)?;return Ok(())}
 }
 if !c.completion&&s.event.is_none(){
  let mut q=request(c);q.deliveries=Some(wire::Deliveries{limit:Some(1)});let r=c.owner.c2(q).await?;s.response(&r)?;
  if let Some(event)=r.deliveries.as_ref().and_then(|v|v.first()){
   let id=event.event_id.clone().ok_or("C2 delivery ID")?;let sequence=event.sequence.clone().ok_or("C2 delivery sequence")?;let value=serde_json::to_value(event).unwrap();let j=s.journal.as_mut().unwrap();
   if j.get(&s.world,&id,&sequence).is_none(){j.put(Record{ordinal:0,lobby:lobby(c),world:s.world.clone(),id,sequence,source:None,state:"pending".into(),reason:String::new(),original:value.clone()})?;}s.event=Some(value);
  }
 }
 if !s.blocked{s.status.clear();}Ok(())
}

#[cfg(test)]mod tests{
 use super::*;
 #[test]fn c2_retired_poll_needs_an_original_and_is_bounded_before_auth_owner_creation(){
  let dir=std::env::temp_dir().join(format!("nt2-c2-retired-{}",uuid::Uuid::new_v4()));std::fs::create_dir(&dir).unwrap();let journal=crate::g3_journal::SharedJournal::new(crate::g3_journal::Journal::open(&dir.join("transfers.jsonl")).unwrap());let mut work=Work::default();
  assert!(!work.retired_due(&journal));assert!(work.state.retired_next.unwrap()>Instant::now());
  let row=Record{ordinal:0,source:Some(json!({})),lobby:crate::g3_journal::LobbyIdentity{service_origin:"https://fixture.invalid".into(),service_id:"fixture".into(),principal_id:"principal".into(),room_id:"room".into()},world:"w".repeat(64),id:uuid::Uuid::new_v4().to_string(),sequence:"0".into(),state:"pending".into(),reason:"uncertain".into(),original:json!({"progression":0})};
  work.state.journal.as_mut().unwrap().put(row.clone()).unwrap();assert!(!work.retired_due(&journal));work.state.retired_next=Some(Instant::now()-Duration::from_millis(1));assert!(work.retired_due(&journal));assert!(!work.retired_due(&journal));work.state.progression=Some(0);assert!(!work.source_recovery_needed());work.state.progression=Some(1);assert!(work.source_recovery_needed());let mut done=row;done.state="quarantined".into();work.state.journal.as_mut().unwrap().put(done).unwrap();assert!(!work.source_recovery_needed());
  drop(work);drop(journal);assert!(dir.canonicalize().unwrap().starts_with(std::env::temp_dir().canonicalize().unwrap()));std::fs::remove_dir_all(dir).unwrap();
 }

 #[test]fn c2_room_only_receipt_snapshot_retains_scoped_encounter_and_rejects_regression(){
  let mut s=State::default();let encounter=wire::Encounter{encounter_id:Some(uuid::Uuid::new_v4().to_string()),progression:Some(0),stage:Some("unlocked".into()),revision:Some("10".into()),..Default::default()};
  s.response(&wire::Response{snapshot:Some(wire::Snapshot{sequence:Some("10".into()),encounter:Some(encounter),..Default::default()}),..Default::default()}).unwrap();
  s.response(&wire::Response{snapshot:Some(wire::Snapshot{sequence:Some("10".into()),..Default::default()}),..Default::default()}).unwrap();assert_eq!(s.snapshot.as_ref().unwrap().encounter.as_ref().unwrap().stage.as_deref(),Some("unlocked"));
  assert!(s.response(&wire::Response{snapshot:Some(wire::Snapshot{sequence:Some("9".into()),..Default::default()}),..Default::default()}).is_err());
  s.response(&wire::Response{snapshot:Some(wire::Snapshot{sequence:Some("11".into()),..Default::default()}),..Default::default()}).unwrap();let mut full=s.snapshot.clone().unwrap();full.encounter.as_mut().unwrap().stage=Some("battle".into());full.encounter.as_mut().unwrap().revision=Some("11".into());s.response(&wire::Response{snapshot:Some(full),..Default::default()}).unwrap();assert_eq!(s.snapshot.as_ref().unwrap().encounter.as_ref().unwrap().stage.as_deref(),Some("battle"));

 }

 #[test]fn c2_readonly_bootstrap_closed_inputs_and_readiness_epoch(){
  assert!(Input::decode(br#"{"kind":"c2_completion","state":{"progression":0}}"#).unwrap().protected_allowed(false));
  for value in [br#"{"kind":"c2","state":{}}"#.as_slice(),br#"{"kind":"c2","state":{"progression":0},"near":{}}"#,br#"{"kind":"c2","near":{"near":true}}"#,br#"{"kind":"c2","publish":{"progression":0}}"#,br#"{"kind":"c2","state":{"progression":-1}}"#]{assert!(Input::decode(value).is_none());}
  let id="11111111-1111-4111-8111-111111111111";
  let near=json!({"kind":"c2","near":{"encounterId":id,"arenaToken":id,"near":true,"readinessEpoch":id}});let input=Input::decode(&serde_json::to_vec(&near).unwrap()).unwrap();assert!(!input.protected_allowed(false));
  let apply=json!({"kind":"c2","apply":{"eventId":id,"sequence":"1"}});assert!(Input::decode(&serde_json::to_vec(&apply).unwrap()).is_some());
 }
 #[test]fn c2_snapshot_pages_keep_native_bound_and_cover_all_members_completions(){
  let mut s=State{world:"a".repeat(64),..Default::default()};let member=wire::Participant{principal_id:Some("p".repeat(36)),world_token:Some("w".repeat(36)),display_name:Some("<&\"".repeat(24)),status:Some("disconnected".into()),readiness_epoch:Some(uuid::Uuid::new_v4().to_string()),..Default::default()};
  s.snapshot=Some(wire::Snapshot{sequence:Some("1".into()),encounter:Some(wire::Encounter{participants:Some(vec![member;32]),..Default::default()}),completions:Some(vec![wire::Completion{display_name:Some("<&\"".repeat(24)),event_id:Some(uuid::Uuid::new_v4().to_string()),..Default::default()};32]),..Default::default()});
  let mut count=0;for page in 0..8{s.page=page;let v=s.view();count+=v["snapshot"]["encounter"]["participants"].as_array().unwrap().len();assert!(serde_json::to_vec(&v).unwrap().len()<8192);assert_eq!(v["participantsPages"],8);}assert_eq!(count,32);
 }
}
