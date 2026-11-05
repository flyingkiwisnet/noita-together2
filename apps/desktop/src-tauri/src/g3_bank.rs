//! Bounded bank transport/journal worker. Wallet mutation belongs to the game
//! thread; this worker only supplies an ephemeral, flushed effect permission.
use crate::{g3_http::{HttpOwner,RecoveryOwner,OriginalStatus},g3_local_consumer::{Binding,LocalConsumer},g3_journal::{EffectPermit,Phase,SharedJournal,LobbyIdentity},protocol_g3::{self,wire},g3_social_stream::Exchange};
use serde::Deserialize;
use serde_json::{json,Value};
use std::{collections::BTreeMap,sync::{Arc,Mutex},time::{Instant,Duration}};

#[path="g3_item_bank.rs"]mod items;
#[path="g3_bank_cache.rs"]mod cache;
pub struct Context{pub binding:Binding,pub consumer:LocalConsumer,pub owner:HttpOwner,pub events:Option<Arc<Mutex<crate::g3_runtime::Exchange>>>,pub items:bool,pub item_players:Value}
fn same_bank_world(a:&wire::Scope,b:&wire::Scope)->bool{a.room_id==b.room_id&&a.run_id==b.run_id&&a.run_epoch==b.run_epoch&&a.config_revision==b.config_revision&&a.world_token==b.world_token}
impl Context{
 fn signal(&self)->Option<(u64,u64)>{self.events.as_ref()?.lock().unwrap().bank_signal(self.binding.context)}
 fn lobby(&self)->Option<LobbyIdentity>{Some(LobbyIdentity{service_origin:self.binding.service_origin.clone(),service_id:self.binding.service_id.clone(),principal_id:self.binding.principal_id.clone(),room_id:self.binding.scope.room_id.clone()?})}
 fn can_publish(&self)->bool{self.owner.active().is_ok()&&!self.lobby().is_some_and(|l|self.consumer.journal.lobby_deleted(&l))}
 fn request(&self)->wire::Request{wire::Request{major:Some(1),revision:Some(protocol_g3::REVISION.into()),service_id:Some(self.binding.service_id.clone()),service_epoch:Some(self.binding.service_epoch.clone()),session_epoch:Some(self.binding.session_epoch.clone()),request_id:Some(uuid::Uuid::new_v4().to_string()),scope:Some(self.binding.scope.clone()),..Default::default()}}
 fn current_original(&self,p:&crate::g3_journal::Pending)->bool{
  let o=&p.record.original;if o.service_origin!=self.binding.service_origin||o.service_id!=self.binding.service_id||o.principal_id!=self.binding.principal_id{return false}
  serde_json::from_str::<wire::Scope>(&o.scope_json).ok().is_some_and(|scope|same_bank_world(&scope,&self.binding.scope))
 }
 fn original(&self,id:&str)->Result<wire::Request,String>{
  let p=self.consumer.journal.pending().find(|p|p.record.original.operation_id==id).ok_or("Unknown original")?;
  if !self.current_original(&p){return Err("Original belongs to another bank world".into())}
  let o=p.record.original;if o.service_origin!=self.binding.service_origin||o.service_id!=self.binding.service_id||o.principal_id!=self.binding.principal_id{return Err("Original belongs to another account".into())}
  let mut r=self.request();r.current_scope=r.scope.take();r.scope=Some(protocol_g3::typed("Scope",o.scope_json.as_bytes()).map_err(|_|"Invalid original scope")?);Ok(r)
 }
 async fn call(&mut self,r:wire::Request)->Result<wire::Response,String>{self.owner.call(serde_json::to_vec(&r).map_err(|_|"Bank encoding")?).await}
}
#[derive(Debug,Clone,Deserialize)]
#[serde(tag="kind",rename_all="snake_case",deny_unknown_fields)]
pub enum Input{
 Submit{id:String,direction:String,amount:u32},
 Prepare{id:String,digest:String,attempt:String},
 Result{id:String,digest:String,attempt:String,applied:bool},
 ItemChunk{id:String,channel:String,offset:usize,total:usize,data:String},
 ItemCopied{id:String,channel:String,offset:usize},
 ItemClaim{id:String,item_id:String,revision:String},
 ItemPrepare{id:String,digest:String,attempt:String},
 ItemResult{id:String,digest:String,attempt:String,applied:bool},
 ItemQuery{query:wire::ItemBank},
 ItemInspect{item_id:String},
 InventoryInspect{actor:String,slot:u32},
}
impl Input{pub fn decode(bytes:&[u8])->Result<Self,String>{
 if bytes.is_empty()||bytes.len()>4096{return Err("Bank input size".into())}let value:Self=serde_json::from_slice(bytes).map_err(|_|"Bank input format")?;
 if !matches!(&value,Self::Submit{..}|Self::Prepare{..}|Self::Result{..}){items::validate_input(&value)?;return Ok(value)}
 let(id,digest,attempt)=match &value{Self::Submit{id,direction,amount}=>{if !matches!(direction.as_str(),"deposit"|"withdraw")||!(1..=1_000_000).contains(amount){return Err("Bank amount/direction".into())}(id,None,None)},Self::Prepare{id,digest,attempt}|Self::Result{id,digest,attempt,..}=>(id,Some(digest),Some(attempt)),_=>unreachable!()};
 if uuid::Uuid::parse_str(id).is_err()||digest.is_some_and(|s|s.len()!=64||!s.bytes().all(|b|b.is_ascii_hexdigit()&&!b.is_ascii_uppercase()))||attempt.is_some_and(|s|s.len()!=32||!s.bytes().all(|b|b.is_ascii_hexdigit())){return Err("Bank input identity".into())}Ok(value)
}}
struct Permission{permit:EffectPermit,attempt:String,digest:String}
// Optional lanes schedule from completion, never catch up missed ticks.
#[derive(Default)]struct PollSchedule{next:Option<Instant>,failures:u8}
impl PollSchedule{
 fn ready(&self,now:Instant)->bool{self.next.is_none_or(|at|now>=at)}
 fn complete(&mut self,now:Instant,success:bool,base_ms:u64){
  self.failures=if success{0}else{self.failures.saturating_add(1).min(4)};
  let delay=base_ms.saturating_mul(1u64<<self.failures).min(base_ms.max(8000));
  // Independent per-completion jitter avoids synchronized client retries.
  let jitter=u64::from(uuid::Uuid::new_v4().as_bytes()[0]);
  self.next=Some(now+Duration::from_millis(delay+jitter));
 }
 fn request_now(&mut self){self.next=None;}
}
// A live, exactly current-scope accepted original is waiting for its local
// offer/prepare handshake, not recovery. Prepare still fetches fresh authority.
// Restored, changed-scope, unknown and ambiguous originals keep status recovery.
fn known_live_unissued(c:&Context,p:&crate::g3_journal::Pending,allowed:Option<bool>,state:Option<&str>)->bool{
 !p.restored&&p.record.phase==Phase::ServerAccepted&&allowed==Some(true)&&matches!(state,Some("awaiting_local"|"reserved"))&&serde_json::from_str::<wire::Scope>(&p.record.original.scope_json).ok().is_some_and(|scope|serde_json::to_vec(&scope).unwrap()==serde_json::to_vec(&c.binding.scope).unwrap())
}
fn terminal_operation(state:Option<&str>)->bool{matches!(state,Some("completed"|"uncertain"|"cancelled"))}
// A durable prefix before EffectIntent can query its original promptly after
// reopen. Status is read-only; fresh prepare remains the only effect permit.
fn historical_original(p:&crate::g3_journal::Pending)->bool{matches!(p.record.phase,Phase::EffectIntent|Phase::Uncertain)||p.restored&&!matches!(p.record.phase,Phase::Requested|Phase::ServerAccepted)}
#[derive(Default)]struct State{
 items:items::State,history_poll:PollSchedule,recovery_passes:u8,cache:cache::Cache,bank_poll:PollSchedule,recovery_poll:PollSchedule,refresh_status:Option<String>,mailbox_passes:u8,item_turn:bool,recovery_turn:bool,
 lobby:Option<LobbyIdentity>,
 world:String,attachment:String,cursor:String,after:String,operations:BTreeMap<String,wire::Operation>,
 permissions:BTreeMap<String,Permission>,submission:Option<Value>,status:String,balance:Option<u32>,version:u64,enabled:bool,notices:Vec<Value>,effect:Option<Value>,fresh:bool,scan:usize,effect_scan:usize,
}
impl State{
 fn refresh_failed(&mut self,error:&str){self.refresh_status=Some(refresh_message(error).into());if error!="CAPACITY"{self.fresh=false;self.cursor.clear();self.notices.clear();}}
 fn view(&self)->Vec<u8>{
  let queued:u64=self.operations.values().filter(|o|o.direction.as_deref()==Some("withdraw")&&o.state.as_deref()==Some("reserved")).map(|o|u64::from(o.amount.unwrap_or(0))).sum();
  serde_json::to_vec(&json!({"run":self.world,"balance":self.balance,"version":self.version.to_string(),"enabled":self.enabled,"fresh":self.fresh,"queued":queued,"status":self.status,"operation_status":self.status,"refresh_status":self.refresh_status,"submission":self.submission,"effect":self.effect,"notices":self.notices,"items":self.items.view()})).ok().filter(|v|v.len()<=8192).unwrap_or_default()
 }
}
#[derive(Default)]pub struct Work{state:State,task:Option<tokio::task::JoinHandle<State>>,next:Option<Instant>,journal:Option<SharedJournal>,task_lobby:Option<LobbyIdentity>,last_view:Value,last_world:String}
impl Work{
 pub async fn shutdown(&mut self){if let Some(task)=self.task.take(){let _=task.await;}}
 fn state_view(&self)->Value{json!({"pending":self.task.is_some(),"items":self.state.items.diagnostics(),"balance":self.state.balance,"version":self.state.version,"status":self.state.status,"operations":self.state.operations.len(),"permissions":self.state.permissions.len(),"fresh":self.state.fresh,"operation_statuses":self.state.operations.iter().rev().take(32).map(|(id,o)|json!({"operation_id":id,"server_status":o.state})).collect::<Vec<_>>()})}
 pub fn view(&self)->Value{if self.task.is_some(){self.last_view.clone()}else{self.state_view()}}
 pub fn view_for(&self,binding:Option<&Binding>)->Value{let Some(binding)=binding else{return Value::Null};let key=world_key(binding);if self.task.is_some(){if key==self.last_world{return self.last_view.clone()}}else if key==self.state.world{return self.state_view()}Value::Null}
 /// Called by the ordinary200ms scheduler. The file owner and HTTP exchange
 /// travel to a bounded blocking worker, never under a lifecycle/native lock.
 pub async fn tick(&mut self,context:Option<Context>,stream:Arc<Mutex<Exchange>>){
  if self.task.as_ref().is_some_and(|t|t.is_finished()){
   if let Ok(state)=self.task.take().unwrap().await{self.state=state;}self.task_lobby=None;
   self.next=Some(Instant::now());
   #[cfg(feature="g3-bank-latency-fixture")]if context.as_ref().is_some_and(|c|crate::g3_http::bank_timing::prior_cooldown(&c.binding)){self.next=Some(Instant::now()+Duration::from_millis(300));}
  }
  if let Some(context)=&context{self.journal=Some(context.consumer.journal.clone());}
  let deleted=self.journal.as_ref().is_some_and(|j|self.state.lobby.as_ref().is_some_and(|l|j.lobby_deleted(l))||self.task_lobby.as_ref().is_some_and(|l|j.lobby_deleted(l)));
  if deleted{self.state=State{status:"Lobby deleted; transfers removed.".into(),..Default::default()};stream.lock().unwrap().clear_deleted_bank();}
  if let Some(j)=&self.journal{prune_state(&mut self.state,j);}
  let Some(context)=context else{self.state.fresh=false;self.state.cursor.clear();self.state.notices.clear();return};
  if context.lobby().is_some_and(|l|context.consumer.journal.lobby_deleted(&l)){stream.lock().unwrap().clear_deleted_bank();return}
  if self.task.is_some(){return}
  let key=world_key(&context.binding);
  if self.state.world!=key{self.state=State{world:key,lobby:context.lobby(),..Default::default()};}
  let view=self.state.view();if !view.is_empty(){stream.lock().unwrap().bank_view(view);}
  let input=stream.lock().unwrap().bank_input();
  #[cfg(feature="g3-bank-latency-fixture")]
  if crate::g3_http::bank_timing::prior_cooldown(&context.binding)&&self.next.is_some_and(|at|Instant::now()<at){return}
  if input.is_none()&&self.next.is_some_and(|at|Instant::now()<at){return}
 self.last_view=self.state_view();self.last_view["pending"]=json!(true);self.last_world=self.state.world.clone();let state=std::mem::take(&mut self.state);let runtime=tokio::runtime::Handle::current();
  self.task_lobby=context.lobby();self.task=Some(tokio::task::spawn_blocking(move||runtime.block_on(run(context,state,input,stream))));
 }
}
fn prune_state(s:&mut State,j:&SharedJournal){
 s.items.prune(j);
 let ids:std::collections::BTreeSet<_>=j.pending().map(|p|p.record.original.operation_id).collect();
 s.operations.retain(|id,_|ids.contains(id));s.permissions.retain(|id,_|ids.contains(id));
 if s.submission.as_ref().and_then(|v|v["id"].as_str()).is_some_and(|id|!ids.contains(id)){s.submission=None;}
 if s.effect.as_ref().and_then(|v|v["id"].as_str()).is_some_and(|id|!ids.contains(id)){s.effect=None;}
}
/// Existing desktop original-status seam: one readonly call at a time, without
/// a game/lobby binding. Only typed authenticated ROOM_DELETED mutates storage.
// This owner can only read original status or accept typed lobby-deletion proof.
// Active bank/effect workers keep their separate same-world fence.
fn cleanup_originals(owner:&RecoveryOwner,journal:&SharedJournal)->BTreeMap<LobbyIdentity,String>{journal.pending().filter(|p|owner.owns_original(&p.record.original)).filter_map(|p|LobbyIdentity::from_original(&p.record.original).ok().map(|key|(key,p.record.original.operation_id))).collect()}
#[derive(Default)]struct Probe{next:Option<Instant>,failures:u8}
#[derive(Default)]pub struct Cleanup{task:Option<tokio::task::JoinHandle<(LobbyIdentity,Result<usize,String>)>>,next:Option<Instant>,owner_key:String,task_world:String,probes:BTreeMap<LobbyIdentity,Probe>,removed:usize,checked:usize,last_error:Option<String>}
impl Cleanup{
 pub async fn shutdown(&mut self){if let Some(task)=self.task.take(){let _=task.await;}}
 pub fn view(&self)->Value{json!({"pending":self.task.is_some(),"removed":self.removed,"checked":self.checked,"last_error":self.last_error})}
 pub async fn tick(&mut self,context:Option<(RecoveryOwner,SharedJournal,Binding)>){
  let Some((mut owner,journal,selected))=context else{self.owner_key.clear();self.probes.clear();self.next=None;self.last_error=None;return};
  let key=format!("{}:{}",owner.cleanup_owner_key(),world_key(&selected));
  if self.owner_key!=key{self.owner_key=key;self.probes.clear();self.next=None;self.last_error=None;self.removed=0;self.checked=0;}
  if self.task.as_ref().is_some_and(|t|t.is_finished()){
   if let Ok((key,result))=self.task.take().unwrap().await{if self.task_world==world_key(&selected){self.completed(key,result,Instant::now());self.next=Some(Instant::now()+Duration::from_secs(1));}}
  }
  if self.task.is_some()||self.next.is_some_and(|at|Instant::now()<at){return}
  let originals=cleanup_originals(&owner,&journal);
  let Some((key,id))=self.select(originals,Instant::now()) else{return};
  self.task_world=world_key(&selected);self.task=Some(tokio::spawn(async move{let result=async{match owner.status_or_deleted(&journal,&id).await?{
   OriginalStatus::Operation(_)|OriginalStatus::ItemOperation(_)=>Ok(0),OriginalStatus::LobbyDeleted(proof)=>tokio::task::spawn_blocking(move||journal.retire_lobby(proof)).await.map_err(|_|"Lobby cleanup worker failed")?,
  }}.await;(key,result)}));
 }
 fn select(&mut self,originals:BTreeMap<LobbyIdentity,String>,now:Instant)->Option<(LobbyIdentity,String)>{
  self.probes.retain(|key,_|originals.contains_key(key));
  for key in originals.keys(){self.probes.entry(key.clone()).or_default();}
  let key=self.probes.iter().filter(|(_,p)|p.next.is_none_or(|at|at<=now)).min_by_key(|(_,p)|p.next).map(|(key,_)|key.clone())?;
  Some((key.clone(),originals[&key].clone()))
 }
 fn completed(&mut self,key:LobbyIdentity,result:Result<usize,String>,now:Instant){
  self.checked=self.checked.saturating_add(1);self.last_error=result.as_ref().err().cloned();
  let probe=self.probes.entry(key).or_default();
  let seconds=match result{Ok(count)=>{self.removed=self.removed.saturating_add(count);probe.failures=0;30},Err(_)=>{probe.failures=probe.failures.saturating_add(1).min(7);(5u64<<u32::from(probe.failures-1)).min(300)}};
  probe.next=Some(now+Duration::from_secs(seconds));
 }
}
fn world_key(b:&Binding)->String{use sha2::{Digest,Sha256};let raw=serde_json::to_string(&(&b.service_origin,&b.service_id,&b.service_epoch,&b.principal_id,&b.scope.room_id,&b.scope.run_id,&b.scope.run_epoch,&b.scope.config_revision,&b.scope.world_token)).unwrap();Sha256::digest(raw.as_bytes()).iter().map(|b|format!("{b:02x}")).collect()}
fn refresh_message(error:&str)->&'static str{match error{"CAPACITY"=>"Bank refresh busy; retrying.","FORBIDDEN"|"POLICY_DISABLED"=>"Bank refresh unavailable.",_=>"Bank refresh unavailable; cached data shown."}}
fn mailbox_background_due(passes:&mut u8,has_input:bool)->bool{if !has_input{*passes=0;true}else{*passes=passes.saturating_add(1);if *passes>=4{*passes=0;true}else{false}}}
fn message(error:&str)->&'static str{match error{"INSUFFICIENT_FUNDS"=>"Not enough gold in the bank.","CAPACITY"=>"Gold service is busy. Recovering the original request.","BANK_FULL"=>"Gold bank is full. Original retained.","FORBIDDEN"=>"Gold transfers are disabled or unavailable.",_=>"Bank unavailable. Original operation retained for recovery."}}
async fn status(c:&mut Context,s:&mut State,id:&str)->Result<wire::Operation,String>{let mut r=c.original(id)?;r.gold_status=Some(wire::GoldStatus{operation_id:Some(id.into())});let result=c.call(r).await;s.recovery_poll.complete(Instant::now(),result.is_ok(),600);let op=result?.operation.ok_or("Missing bank operation")?;c.consumer.journal.validate_status(&op)?;s.operations.insert(id.into(),op.clone());if s.submission.as_ref().is_some_and(|v|v["id"]==id){s.status=match op.state.as_deref(){Some("completed")=>"Gold transfer completed in this session.",Some("uncertain")=>"Earlier gold transfer uncertain; no automatic replay.",Some("awaiting_local")=>"Deposit reserved; waiting for wallet debit.",Some("reserved")=>"Gold queued. Collect at the outlet.",_=>"Original gold request resolved."}.into();}if s.submission.as_ref().is_some_and(|v|v["id"]==id&&v["status"]!="accepted"){let state=match op.state.as_deref(){Some("cancelled")=>"refused",Some("uncertain")=>"unresolved",_=>"accepted"};s.submission=Some(json!({"id":id,"status":state,"direction":op.direction}));}Ok(op)}
fn gold_permanent_intent(error:&str)->bool{matches!(error,"BANK_FULL"|"INSUFFICIENT_FUNDS"|"FORBIDDEN"|"INVALID_MESSAGE"|"CONFLICT")}
fn gold_intent_result(c:&mut Context,s:&mut State,id:&str,result:Result<wire::Response,String>)->Result<wire::Response,String>{
 if let Err(error)=&result{
  // A service/storage refusal cannot erase a known acceptance or local effect.
  let unissued=c.consumer.journal.pending().find(|p|p.record.original.operation_id==id).is_some_and(|p|p.record.phase==Phase::Requested&&p.record.delivery.is_none())&&!s.operations.contains_key(id);
  let refused=gold_permanent_intent(error)&&unissued;if refused{c.consumer.journal.refuse(id)?;}
  if let Some(v)=s.submission.as_mut().filter(|v|v["id"]==id){v["status"]=json!(if refused{"refused"}else{"unresolved"});}
 }
 result
}
async fn recover_gold_original(c:&mut Context,s:&mut State,p:&crate::g3_journal::Pending)->Result<(),String>{
 let id=&p.record.original.operation_id;
 match status(c,s,id).await{Ok(_)=>Ok(()),Err(error)=>{
  if error!="NOT_FOUND"||p.record.phase!=Phase::Requested||p.restored||!protocol_g3::typed::<wire::Scope>("Scope",p.record.original.scope_json.as_bytes()).ok().is_some_and(|scope|serde_json::to_vec(&scope).unwrap()==serde_json::to_vec(&c.binding.scope).unwrap()){return Err(error)}
  let req=c.consumer.begin_gold_with_id(&p.record.original.direction,p.record.original.amount,c.consumer.generation(),id.clone())?;
  let result=c.owner.submit_gold(&mut c.consumer,req).await;let response=gold_intent_result(c,s,id,result)?;
  let op=response.operation.ok_or("Missing recovered gold operation")?;s.operations.insert(id.clone(),op);
  if let Some(v)=s.submission.as_mut().filter(|v|v["id"].as_str()==Some(id.as_str())){v["status"]=json!("accepted");v["direction"]=json!(p.record.original.direction);}
  Ok(())
 }}
}
async fn action(c:&mut Context,s:&mut State,input:Input)->Result<(),String>{match input{
 Input::Submit{id,direction,amount}=>{
  // The ID originates once at the click and survives every mailbox/reply retry.
  if let Some(p)=c.consumer.journal.pending().find(|p|p.record.original.operation_id==id){
   if p.record.original.item.is_some()||p.record.original.direction!=direction||p.record.original.amount!=amount{return Err("CONFLICT".into())}
   if p.record.phase==Phase::Refused{s.submission=Some(json!({"id":id,"status":"refused","direction":direction}));return Ok(())}
   s.submission=Some(json!({"id":id,"status":"pending","direction":direction}));status(c,s,&id).await?;return Ok(())
  }
  let request=c.consumer.begin_gold_with_id(&direction,amount,c.consumer.generation(),id.clone())?;
  s.submission=Some(json!({"id":id,"status":"pending","direction":direction}));
  let result=c.owner.submit_gold(&mut c.consumer,request).await;let response=gold_intent_result(c,s,&id,result)?;let op=response.operation.ok_or("Missing bank operation")?;
  s.operations.insert(id.clone(),op);s.submission=Some(json!({"id":id,"status":"accepted","direction":direction}));s.status=if direction=="withdraw"{"Stay on the left wing altar in the holy mountain to receive your gold"}else{"Deposit accepted; waiting for wallet debit."}.into();
 },
 Input::Prepare{id,digest,attempt}=>{
  if let Some(p)=s.permissions.get(&id){if p.attempt!=attempt||p.digest!=digest{return Err("Effect permission conflict".into())}return Ok(())}
  let op=status(c,s,&id).await?;if op.effect_allowed!=Some(true)||op.payload_digest.as_deref()!=Some(&digest){return Err("Effect permission refused".into())}
  c.owner.active()?;c.consumer.journal.recover_unissued(&op)?;let permit=c.consumer.journal.prepare_effect(&id)?;
  s.permissions.insert(id.clone(),Permission{permit,attempt:attempt.clone(),digest:digest.clone()});
  s.effect=Some(json!({"id":id,"digest":digest,"amount":op.amount,"direction":op.direction,"stage":"permit","attempt":attempt}));
 },
 Input::Result{id,digest,attempt,applied}=>{
  let p=s.permissions.get(&id).ok_or("No in-session effect permission")?;if p.attempt!=attempt||p.digest!=digest{return Err("Effect result conflict".into())}
  let p=s.permissions.remove(&id).unwrap();c.consumer.journal.finish_session_effect(p.permit,applied)?;s.effect=None;
  send_receipt(c,s,&id,applied).await?;
 },_=>return Err("Not a gold input".into())
}Ok(())}
async fn send_receipt(c:&mut Context,s:&mut State,id:&str,applied:bool)->Result<(),String>{
 let p=c.consumer.journal.pending().find(|p|p.record.original.operation_id==id).ok_or("Missing original")?;let d=p.record.delivery.ok_or("Missing delivery")?;
 let outcome=if !applied{"uncertain"}else if p.record.original.direction=="deposit"{"escrowed"}else{"applied"};
 // Stable receipt identity across a lost committed reply; not a new operation.
 let receipt_id=id.to_owned();
 // This digest identifies the client-reported session outcome. It explicitly
 // does not assert persistence in a loaded Noita save, and is stable on retry.
 use sha2::{Digest,Sha256};let evidence=serde_json::to_vec(&json!({"revision":1,"operationId":id,"deliveryId":d.delivery_id,"payloadDigest":d.payload_digest,"outcome":outcome,"provenance":"client-reported-session-outcome","savePersistence":"unproven"})).map_err(|_|"Receipt evidence encoding")?;
 let evidence_digest=Sha256::digest(&evidence).iter().map(|b|format!("{b:02x}")).collect();
 let mut r=c.original(id)?;r.gold_receipt=Some(wire::GoldReceipt{operation_id:Some(id.into()),delivery_id:Some(d.delivery_id),payload_digest:Some(d.payload_digest.clone()),receipt_id:Some(receipt_id),outcome:Some(outcome.into()),evidence_digest:Some(evidence_digest)});
 let result=c.call(r).await;s.recovery_poll.complete(Instant::now(),result.is_ok(),600);let op=result?.operation.ok_or("Missing receipt operation")?;c.consumer.journal.validate_status(&op)?;s.operations.insert(id.into(),op);s.cache.request();s.bank_poll.request_now();if s.submission.as_ref().is_some_and(|v|v["id"]==id){s.status=if applied{"Gold transfer completed in this session."}else{"Wallet result is uncertain. This operation will not replay."}.into();}Ok(())
}
fn accept_gold_page(s:&mut State,journal:&SharedJournal,bank:wire::GoldBankSnapshot){
   let first=s.balance.is_none();
   let version=bank.balance_version.as_deref().and_then(|v|v.parse::<u64>().ok());
   if let Some(version)=version.filter(|v|*v>=s.version){s.refresh_status=None;s.version=version;s.balance=bank.available_balance;s.enabled=bank.send_gold_enabled==Some(true);s.fresh=true;
    if s.status.is_empty(){s.status=if s.enabled{"Ready."}else{"Send Gold is disabled."}.into();}
   }
   s.cursor=bank.notice_sequence.unwrap_or_default();s.after=bank.next_operation_id.unwrap_or_default();
   s.notices=bank.notices.unwrap_or_default().into_iter().filter(|_|!first).map(|n|json!({"id":n.sequence,"text":format!("{} deposited {} gold to the bank",n.display_name.unwrap_or_else(||"Player".into()),n.amount.unwrap_or(0))})).collect();
   for op in bank.operations.unwrap_or_default(){if let Some(id)=&op.operation_id{if journal.validate_status(&op).is_ok(){s.operations.insert(id.clone(),op);}}}
}
fn display_ready(c:&Context,cache:&mut cache::Cache,schedule:&mut PollSchedule)->bool{
 let now=Instant::now();let signal=c.signal();if let Some((generation,revision))=signal{if cache.invalidate(generation,revision,now){schedule.request_now();}}
 let dirty=cache.ready(now);
 if dirty&&schedule.failures==0{schedule.request_now();}
 schedule.ready(now)&&(dirty||signal.is_none())
}
fn publish_bank(c:&Context,s:&State,stream:&Arc<Mutex<Exchange>>){let view=s.view();if c.can_publish()&&!view.is_empty(){stream.lock().unwrap().bank_view_for(c.binding.context,view);}}
async fn pump_gold(c:&mut Context,s:&mut State,stream:&Arc<Mutex<Exchange>>){
 let pending:Vec<_>=c.consumer.journal.pending().filter(|p|p.record.original.item.is_none()&&p.record.phase!=Phase::Refused&&!s.operations.get(&p.record.original.operation_id).is_some_and(|op|known_live_unissued(c,p,op.effect_allowed,op.state.as_deref()))&&!s.permissions.contains_key(&p.record.original.operation_id)&&s.operations.get(&p.record.original.operation_id).is_none_or(|op|!terminal_operation(op.state.as_deref()))&&c.original(&p.record.original.operation_id).is_ok()).collect();
 let (history,live):(Vec<_>,Vec<_>)=pending.into_iter().partition(|p|historical_original(p));
 let history_turn=!history.is_empty()&&s.history_poll.ready(Instant::now())&&(live.is_empty()||s.recovery_passes>=8);
 let pending=if history_turn{history}else{live};
 let page_ready=display_ready(c,&mut s.cache,&mut s.bank_poll);
 let recovery_ready=(history_turn||s.recovery_poll.ready(Instant::now()))&&!pending.is_empty();
 let recover=recovery_ready&&(!page_ready||!s.recovery_turn);
 if page_ready&&!recover{
 s.recovery_turn=false;s.refresh_status=Some("Refreshing bank...".into());publish_bank(c,s,stream);
 let mut request=c.request();request.gold_bank=Some(wire::GoldBank{after_notice_sequence:(!s.cursor.is_empty()).then(||s.cursor.clone()),after_operation_id:(!s.after.is_empty()).then(||s.after.clone()),limit:Some(8)});
 let result=c.call(request).await;s.bank_poll.complete(Instant::now(),result.is_ok(),if result.is_ok()||c.signal().is_none()&&s.cache.revision().is_some()&&!s.cache.ready(Instant::now()){30000}else{600});
 match result{
  Ok(response)=>{s.cache.accept(response.bank_revision.as_deref().and_then(|v|v.parse().ok()).unwrap_or(0));if let Some(bank)=response.bank{
   accept_gold_page(s,&c.consumer.journal,bank);
  }},Err(e)=>{s.refresh_failed(&e);}
 }
 }
 // A bounded round-robin original recovery step, independent of bank pages.

 if recover{s.recovery_turn=true;s.scan%=pending.len();let p=&pending[s.scan];s.scan+=1;let id=&p.record.original.operation_id;
  if history_turn{
   let mut r=c.original(id).unwrap();r.gold_status=Some(wire::GoldStatus{operation_id:Some(id.clone())});let result=c.call(r).await;
   s.history_poll.complete(Instant::now(),result.is_ok(),30000);s.recovery_passes=0;
   if let Ok(response)=result{if let Some(op)=response.operation{if c.consumer.journal.validate_status(&op).is_ok(){s.operations.insert(id.clone(),op);}}}
  }else if p.record.phase==Phase::AppliedSession&&!p.restored{if s.operations.get(id).is_none_or(|o|o.state.as_deref()!=Some("completed")){let _=send_receipt(c,s,id,true).await;}}
  else if p.record.phase==Phase::Requested||p.record.phase==Phase::ServerAccepted{
   if let Err(error)=recover_gold_original(c,s,p).await{if s.submission.as_ref().is_some_and(|v|v["id"]==*id){s.status=message(&error).into();}}
  }else if !s.permissions.contains_key(id)&&matches!(p.record.phase,Phase::EffectIntent|Phase::AppliedSession|Phase::Uncertain){let _=status(c,s,id).await;}
  if !history_turn{s.recovery_passes=s.recovery_passes.saturating_add(1);}
 }
 offer_ready_gold(c,s);
}
fn offer_ready_gold(c:&Context,s:&mut State){
 if s.permissions.is_empty(){s.effect=None;if s.enabled&&s.fresh{let operations:Vec<_>=s.operations.iter().collect();let len=operations.len();for offset in 0..len{let index=(s.effect_scan+offset)%len;let(id,op)=operations[index];
  if op.effect_allowed!=Some(true){continue}let Some(p)=c.consumer.journal.pending().find(|p|p.record.original.operation_id==*id)else{continue};
  if !c.current_original(&p)||!matches!(p.record.phase,Phase::Requested|Phase::ServerAccepted)||!matches!(op.state.as_deref(),Some("awaiting_local"|"reserved")){continue}
  s.effect=Some(json!({"id":id,"digest":op.payload_digest,"direction":op.direction,"amount":op.amount,"stage":"offer"}));s.effect_scan=(index+1)%len;break
 }}}
}

async fn consume(c:&mut Context,s:&mut State,seq:u64,input:Input,stream:&Arc<Mutex<Exchange>>){
  #[cfg(feature="g3-bank-latency-fixture")]let timing_label=match &input{Input::ItemChunk{id,channel,..}=>(format!("input:{channel}"),Some(id.clone())),Input::ItemCopied{id,channel,..}=>(format!("ack:{channel}"),Some(id.clone())),Input::ItemPrepare{id,..}=>("prepare:item".into(),Some(id.clone())),Input::ItemResult{id,..}=>("result:item".into(),Some(id.clone())),Input::Submit{id,..}=>("submit:gold".into(),Some(id.clone())),Input::Prepare{id,..}=>("prepare:gold".into(),Some(id.clone())),Input::Result{id,..}=>("result:gold".into(),Some(id.clone())),_=>("query:metadata".into(),None)};
  #[cfg(feature="g3-bank-latency-fixture")]crate::g3_http::bank_timing::stage(&c.binding,&timing_label.0,timing_label.1.as_deref());
  if matches!(&input,Input::Submit{..}|Input::Result{..}){s.cache.request();s.bank_poll.request_now();}
  if !matches!(&input,Input::Submit{..}|Input::Prepare{..}|Input::Result{..}){
   let original=match &input{Input::ItemClaim{id,..}|Input::ItemPrepare{id,..}|Input::ItemResult{id,..}=>Some(id.clone()),Input::ItemChunk{id,channel,..}if channel=="deposit"=>Some(id.clone()),_=>None};
   if let Err(e)=items::action(c,&mut s.items,input).await{if let Some(id)=original{s.items.foreground_error(&id,&e);}else{s.items.metadata_error(&e);}}
  }else{
  let id=match &input{Input::Submit{id,..}|Input::Prepare{id,..}|Input::Result{id,..}=>id.clone(),_=>unreachable!()};
  if let Err(e)=action(c,s,input).await{if s.submission.as_ref().is_some_and(|v|v["id"]==id){s.status=message(&e).into();}if s.submission.as_ref().is_some_and(|v|v["id"]==id&&v["status"]!="accepted"&&v["status"]!="refused"){s.submission=Some(json!({"id":id,"status":"unresolved"}));}}
  }
  #[cfg(feature="g3-bank-latency-fixture")]crate::g3_http::bank_timing::stage(&c.binding,&format!("done:{}",timing_label.0),timing_label.1.as_deref());
  stream.lock().unwrap().acknowledge_bank(c.binding.context,seq);
}
async fn run(mut c:Context,mut s:State,input:Option<(u64,Input)>,stream:Arc<Mutex<Exchange>>)->State{
 let current=world_key(&c.binding);if s.world!=current{s=State{world:current,lobby:c.lobby(),..Default::default()};}
 if c.lobby().is_some_and(|l|c.consumer.journal.lobby_deleted(&l)){return State{lobby:c.lobby(),..Default::default()}}
 let attachment=c.binding.scope.attachment_id.clone().unwrap_or_default();if s.attachment!=attachment{s.cache.request();s.bank_poll.request_now();s.recovery_poll.request_now();s.fresh=false;s.cursor.clear();s.notices.clear();s.permissions.clear();s.effect=None;s.items.reattach();s.attachment=attachment;}
 // Keep the latest bank page intact until another page replaces it. Optional
 // mailbox/C1 work must not erase notices before the native lane copies it.
 let had_input=input.is_some();let deadline=Instant::now()+Duration::from_millis(80);let wake=stream.lock().unwrap().bank_wake.clone();let mut current_input=input;let mut drained=0u8;
 // Drain up to eight mailbox frames /32KiB /80ms. New frames wake this owner;
 // no per-frame sleep and no disk/network work under the exchange lock.
 for _ in 0..8{
  let Some((seq,input))=current_input.take()else{break};
  let _=futures_util::FutureExt::now_or_never(wake.notified());
  consume(&mut c,&mut s,seq,input,&stream).await;drained+=1;
  offer_ready_gold(&c,&mut s);items::offer_ready(&c,&mut s.items);publish_bank(&c,&s,&stream);
  if Instant::now()>=deadline{break}
  current_input=stream.lock().unwrap().bank_input();
  if current_input.is_none(){
   if tokio::time::timeout(deadline.saturating_duration_since(Instant::now()),wake.notified()).await.is_err(){break}
   current_input=stream.lock().unwrap().bank_input();
  }
 }
 if had_input{s.mailbox_passes=s.mailbox_passes.saturating_add(drained.saturating_sub(1));}
 if mailbox_background_due(&mut s.mailbox_passes,had_input){
  if c.items&&s.item_turn{let base:Value=serde_json::from_slice(&s.view()).unwrap_or(json!({}));items::pump_with_publication(&mut c,&mut s.items,Some((&stream,base))).await;}else{pump_gold(&mut c,&mut s,&stream).await;}
  s.item_turn=!s.item_turn;
 }
 items::offer_ready(&c,&mut s.items);

 prune_state(&mut s,&c.consumer.journal);s
}
#[cfg(test)]mod tests{
 use super::*;
 include!("g3_bank_delete_test.rs");
 include!("g3_bank_capacity_test.rs");
 include!("g3_bank_refresh_test.rs");
 include!("g3_bank_isolation_test.rs");
 #[test]fn bank_optional_polling_backs_off_without_catchup_and_explicit_refresh_bypasses_wait(){
  let now=Instant::now();let mut lane=PollSchedule::default();assert!(lane.ready(now));
  lane.complete(now,true,2000);assert!(!lane.ready(now+Duration::from_millis(1999)));assert!(lane.ready(now+Duration::from_millis(2255)));
  for n in 1..=8{lane.complete(now,false,600);let delay=(600u64*(1u64<<n.min(4))).min(8000);assert!(!lane.ready(now+Duration::from_millis(delay-1)));assert!(lane.ready(now+Duration::from_millis(delay+255)));}
  let later=now+Duration::from_secs(60);assert!(lane.ready(later));lane.complete(later,true,2000);assert!(!lane.ready(later),"Missed ticks never produce catch-up requests");assert_eq!(lane.failures,0);
  lane.request_now();assert!(lane.ready(later),"A user refresh does not wait for background backoff");
  for state in ["completed","uncertain","cancelled"]{assert!(terminal_operation(Some(state)));}
  for state in [None,Some("reserved"),Some("awaiting_local")]{assert!(!terminal_operation(state));}
 }
 #[test]fn b1_input_has_bounded_numeric_identity_and_exact_variants(){
  let id=uuid::Uuid::new_v4().to_string();
  for amount in [1,1_000_000]{assert!(Input::decode(serde_json::to_string(&json!({"kind":"submit","id":id,"amount":amount,"direction":"deposit"})).unwrap().as_bytes()).is_ok());}
  for amount in [json!(0),json!(1_000_001),json!(-1),json!(1.1),json!("5")]{assert!(Input::decode(serde_json::to_string(&json!({"kind":"submit","id":id,"amount":amount,"direction":"withdraw"})).unwrap().as_bytes()).is_err());}
  for bad in [json!({"kind":"submit","id":id,"amount":1,"direction":"items"}),json!({"kind":"submit","id":id,"amount":1,"direction":"deposit","scope":"forged"}),json!({"kind":"prepare","id":id,"digest":"a".repeat(64),"attempt":"bad"})]{assert!(Input::decode(serde_json::to_vec(&bad).unwrap().as_slice()).is_err());}
 }
 #[test]fn b1_unavailable_balance_is_not_zero_and_view_is_bounded(){let state=State::default();let view:Value=serde_json::from_slice(&state.view()).unwrap();assert!(view["balance"].is_null());assert_eq!(view["fresh"],false);assert!(state.view().len()<=8192);}
}
