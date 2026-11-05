//! Item lane of the existing optional bank worker. No engine calls or second
//! lifecycle owner. Descriptors travel as bounded, acknowledged mailbox copies.
use super::*;
use base64::{Engine,engine::general_purpose::STANDARD};
use std::collections::VecDeque;
const CHUNK:usize=2304;
include!("g3_item_batch.rs");
fn uuid(s:&str)->bool{uuid::Uuid::parse_str(s).is_ok()}
fn identity(s:&str)->bool{!s.is_empty()&&s.len()<=80&&s.bytes().all(|b|b.is_ascii_alphanumeric()||b==b'-'||b==b'_')}
pub(super) fn validate_input(v:&Input)->Result<(),String>{let valid=match v{
 Input::ItemChunk{id,channel,offset,total,data}=>identity(id)&&matches!(channel.as_str(),"deposit"|"inventory")&&*total>0&&*total<=crate::g3_items::MAX_DESCRIPTOR&&offset<total&&data.len()<=3072&&STANDARD.decode(data).is_ok_and(|b|!b.is_empty()&&b.len()<=CHUNK&&b.len()<=total-offset&&STANDARD.encode(&b)==*data),
 Input::ItemCopied{id,channel,offset}=>identity(id)&&channel.len()<=24&&*offset<crate::g3_items::MAX_DESCRIPTOR,
 Input::ItemClaim{id,item_id,revision}=>uuid(id)&&uuid(item_id)&&revision.parse::<u64>().is_ok(),
 Input::ItemPrepare{id,digest,attempt}|Input::ItemResult{id,digest,attempt,..}=>uuid(id)&&digest.len()==64&&digest.bytes().all(|b|b.is_ascii_digit()||(b'a'..=b'f').contains(&b))&&attempt.len()==32&&attempt.bytes().all(|b|b.is_ascii_hexdigit()),
 Input::ItemQuery{query}=>serde_json::to_vec(query).ok().and_then(|b|protocol_g3::typed::<wire::ItemBank>("ItemBank",&b).ok()).is_some()&&query.search.as_ref().is_none_or(|s|s.chars().count()<=32),
 Input::ItemInspect{item_id}=>uuid(item_id),Input::InventoryInspect{actor,slot}=>uuid(actor)&&*slot<8,_=>false,
 };if valid{Ok(())}else{Err("Invalid item mailbox input".into())}}
struct Incoming{total:usize,bytes:Vec<u8>}
struct Outgoing{id:String,channel:String,bytes:Vec<u8>,offset:usize}
#[derive(Default)]pub(super) struct State{
 pending_bank:Option<(String,Value,String)>,submissions:Vec<Value>,intents:BTreeMap<String,Vec<u8>>,intent_due:Option<Instant>,cache:cache::Cache,refresh_status:Option<String>,recovery_turn:bool,prepares:BTreeMap<String,(String,String,PollSchedule)>,history_schedule:PollSchedule,recovery_passes:u8,offer_scan:usize,page_schedule:PollSchedule,recovery_schedule:PollSchedule,enabled:bool,fresh:bool,status:String,intent_error:Option<Value>,receipt_error:Option<Value>,receipt_success:Option<Value>,receipt_attempt:Option<Value>,bank_poll:Option<Value>,receipt_failures:u64,recovery_streak:u8,players_key:String,payloads:std::collections::BTreeSet<String>,submission:Option<Value>,effect:Option<Value>,operations:BTreeMap<String,wire::ItemOperation>,permissions:BTreeMap<String,Permission>,incoming:BTreeMap<(String,String),Incoming>,outgoing:VecDeque<Outgoing>,copied:Option<Value>,query:wire::ItemBank,query_key:String,notice:String,after:String,scan:usize,poll:usize,bank:Option<Value>,
}
impl State{
 pub(super) fn metadata_error(&mut self,error:&str){self.refresh_status=Some(if error=="CAPACITY"{refresh_message(error)}else{"Bank metadata unavailable; retrying."}.into());}
 fn refresh_failed(&mut self,error:&str){self.refresh_status=Some(refresh_message(error).into());if error!="CAPACITY"{self.fresh=false;}}
 pub(super) fn diagnostics(&self)->Value{json!({"intent_failure":self.intent_error,"receipt_failure":self.receipt_error,"receipt_success":self.receipt_success,"receipt_attempt":self.receipt_attempt,"receipt_failures":self.receipt_failures,"bank_poll":self.bank_poll,"outgoing":self.outgoing.len(),"poll":self.poll,"status":self.status,"prepare_waiting":self.prepares.len(),"permissions":self.permissions.len(),"operation_statuses":self.operations.iter().rev().take(32).map(|(id,o)|json!({"operation_id":id,"server_status":o.state})).collect::<Vec<_>>()})}
 fn intent_failed(&mut self,id:&str,kind:&str,error:&str){
  let code=if matches!(error,"FORBIDDEN"|"CAPACITY"|"INVALID_MESSAGE"|"BANK_FULL"|"POLICY_DISABLED"|"CLAIM_LOST"|"UNSUPPORTED_CONTENT"|"INCOMPATIBLE_CATALOGUE"|"STALE_EPOCH"|"NOT_FOUND"|"CONFLICT"|"SERVICE_UNAVAILABLE"){error}else{"Unclassified item intent failure"};
  self.intent_error=Some(json!({"stage":"itemIntent","operation_id":id,"direction":kind,"code":code}));
 }
 fn receipt_failed(&mut self,id:&str,error:&str){
  let lower=error.to_ascii_lowercase();let code=if error.len()<=160&&!error.chars().any(char::is_control)&&!["token","secret","password","authorization","bearer","proof"].iter().any(|word|lower.contains(word)){error}else{"Unclassified item receipt failure"};
  self.receipt_failures=self.receipt_failures.saturating_add(1);self.receipt_error=Some(json!({"stage":"itemReceipt","operation_id":id,"code":code,"count":self.receipt_failures,"resolved":false}));self.foreground_error(id,error);
 }
 pub(super) fn reattach(&mut self){self.pending_bank=None;self.submissions.clear();self.intents.clear();self.intent_due=None;self.prepares.clear();self.cache.request();self.page_schedule.request_now();self.recovery_schedule.request_now();self.permissions.clear();self.effect=None;self.incoming.clear();self.outgoing.clear();self.copied=None;self.fresh=false;self.query_key.clear();self.notice.clear();self.payloads.clear();self.players_key.clear();}
 pub(super) fn foreground_error(&mut self,id:&str,e:&str){if self.submission.as_ref().is_some_and(|v|v["id"]==id){self.error(e);}}
 pub(super) fn error(&mut self,e:&str){self.status=match e{"FORBIDDEN"|"POLICY_DISABLED"=>"This item category is disabled or unavailable.","CAPACITY"=>"Item service is busy. Waiting to resolve the original request.","BANK_FULL"=>"Item bank is full. Original retained.","CONFLICT"|"CLAIM_LOST"|"NOT_FOUND"=>"Item is no longer available. Refresh the bank.",_=>"Item result unresolved. Original retained; do not repeat the transfer."}.into();}
 pub(super) fn prune(&mut self,j:&SharedJournal){let ids:std::collections::BTreeSet<_>=j.pending().filter(|p|p.record.original.item.is_some()).map(|p|p.record.original.operation_id).collect();self.operations.retain(|id,_|ids.contains(id));self.prepares.retain(|id,_|ids.contains(id));self.payloads.retain(|id|ids.contains(id));self.permissions.retain(|id,_|ids.contains(id));if self.effect.as_ref().and_then(|v|v["id"].as_str()).is_some_and(|id|!ids.contains(id)){self.effect=None;}if self.submission.as_ref().and_then(|v|v["id"].as_str()).is_some_and(|id|!ids.contains(id)){self.submission=None;}for diagnostic in [&mut self.intent_error,&mut self.receipt_error,&mut self.receipt_success,&mut self.receipt_attempt]{if diagnostic.as_ref().and_then(|v|v["operation_id"].as_str()).is_some_and(|id|!ids.contains(id)){*diagnostic=None;}}}
 fn effects(&self)->Vec<Value>{let mut effects:Vec<_>=self.permissions.iter().map(|(id,p)|json!({"id":id,"digest":p.digest,"attempt":p.attempt,"direction":self.operations.get(id).and_then(|o|o.direction.as_deref()),"stage":"permit"})).collect();if let Some(offer)=self.effect.as_ref().filter(|e|e["stage"]=="offer"){if effects.len()<4{effects.insert(0,offer.clone());}}effects}
 pub(super) fn view(&self)->Value{let mut diagnostics=self.diagnostics();diagnostics.as_object_mut().unwrap().remove("operation_statuses");let chunk=self.outgoing.front().map(|o|json!({"id":o.id,"channel":o.channel,"offset":o.offset,"total":o.bytes.len(),"data":STANDARD.encode(&o.bytes[o.offset..(o.offset+CHUNK).min(o.bytes.len())])}));json!({"enabled":self.enabled,"fresh":self.fresh,"status":self.status,"operation_status":self.status,"refresh_status":self.refresh_status,"submission":self.submission,"submissions":self.submissions,"effect":self.effect,"effects":self.effects(),"chunk":chunk,"copied":self.copied,"diagnostics":diagnostics,"queued":self.operations.values().filter(|o|o.direction.as_deref()==Some("claim")&&o.state.as_deref()==Some("reserved")).count()})}
 fn queue(&mut self,id:String,channel:&str,value:Value)->Result<(),String>{let bytes=serde_json::to_vec(&value).map_err(|_|"Item copy encoding")?;if bytes.len()>crate::g3_items::MAX_DESCRIPTOR{return Err("Item copy size".into())}if self.outgoing.iter().any(|o|o.id==id&&o.channel==channel){return Ok(())}if self.outgoing.len()>=4{return Err("Item copy busy".into())}self.outgoing.push_back(Outgoing{id,channel:channel.into(),bytes,offset:0});Ok(())}
}
fn recovered_submission(s:&mut State,op:&wire::ItemOperation){
 if matches!(op.state.as_deref(),Some("awaiting_local"|"reserved"))&&s.submission.as_ref().is_some_and(|v|v["id"].as_str()==op.operation_id.as_deref()&&matches!(v["status"].as_str(),Some("pending"|"unresolved"|"accepted"))){
  s.submission.as_mut().unwrap()["status"]=json!("accepted");
  s.status=if op.direction.as_deref()==Some("claim"){"Item queued. Collect at the left wing outlet."}else{"Deposit reserved; original still local until escrow."}.into();
 }
}
async fn status(c:&mut Context,s:&mut State,id:&str)->Result<wire::ItemOperation,String>{let result=status_inner(c,s,id).await;s.recovery_schedule.complete(Instant::now(),result.is_ok(),600);result}
async fn status_inner(c:&mut Context,s:&mut State,id:&str)->Result<wire::ItemOperation,String>{let mut r=c.original(id)?;r.item_status=Some(wire::ItemStatus{operation_id:Some(id.into())});let op=c.call(r).await?.item_operation.ok_or("Missing item status")?;c.consumer.journal.validate_item_status(&op)?;if c.consumer.journal.pending().any(|p|p.record.original.operation_id==id&&!p.restored&&matches!(p.record.phase,Phase::Requested|Phase::ServerAccepted)){recovered_submission(s,&op);}s.operations.insert(id.into(),op.clone());Ok(op)}
fn permanent_refusal(error:&str)->bool{matches!(error,"FORBIDDEN"|"INVALID_MESSAGE"|"BANK_FULL"|"POLICY_DISABLED"|"CLAIM_LOST"|"UNSUPPORTED_CONTENT"|"INCOMPATIBLE_CATALOGUE"|"STALE_EPOCH")}
fn intent_result(c:&mut Context,s:&mut State,id:&str,kind:&str,result:Result<wire::Response,String>)->Result<wire::Response,String>{
 if let Err(error)=&result{
  s.intent_failed(id,kind,error);
  if permanent_refusal(error){c.consumer.journal.refuse(id)?;}
  if let Some(submission)=s.submission.as_mut().filter(|v|v["id"]==id){submission["status"]=json!(if permanent_refusal(error){"refused"}else{"unresolved"});}
 }
 result
}
async fn retry_unissued_with<F>(c:&mut Context,s:&mut State,p:&crate::g3_journal::Pending,error:&str,send:F)->Result<(),String>
where F:for<'a>FnOnce(&'a mut HttpOwner,&'a mut LocalConsumer,Vec<u8>)->std::pin::Pin<Box<dyn std::future::Future<Output=Result<wire::Response,String>>+Send+'a>>{
 if error!="NOT_FOUND"||p.record.phase!=Phase::Requested||p.restored||!serde_json::from_str::<wire::Scope>(&p.record.original.scope_json).ok().is_some_and(|scope|serde_json::to_vec(&scope).unwrap()==serde_json::to_vec(&c.binding.scope).unwrap()){return Ok(())}
 if let Some(original)=p.record.original.item.clone(){
  let id=&p.record.original.operation_id;let kind=original.kind.clone();let req=c.consumer.begin_item_with_id(original,c.consumer.generation(),id.clone())?;
  let response=send(&mut c.owner,&mut c.consumer,req).await;let response=intent_result(c,s,id,&kind,response)?;
  if let Some(op)=response.item_operation{recovered_submission(s,&op);s.operations.insert(id.clone(),op);}
 }
 Ok(())
}
async fn submit(c:&mut Context,s:&mut State,id:String,kind:&str,payload:Value)->Result<(),String>{
 let original=crate::g3_items::Original{kind:kind.into(),payload_json:String::from_utf8(crate::g3_items::canonical(&payload)?).map_err(|_|"Item encoding")?};crate::g3_items::validate_original(&original)?;
 if let Some(p)=c.consumer.journal.pending().find(|p|p.record.original.operation_id==id){if p.record.original.item.as_ref()!=Some(&original){return Err("CONFLICT".into())}if p.record.phase==Phase::Refused{s.submission=Some(json!({"id":id,"direction":kind,"status":"refused"}));return Err("FORBIDDEN".into())}// Exact repeated copy is already flushed; the scheduled recovery owner alone retries/status-polls it.
 return Ok(())}
 s.submission=Some(json!({"id":id,"status":"pending","direction":kind}));
 let req=match c.consumer.begin_item_with_id(original,c.consumer.generation(),id.clone()){Ok(req)=>req,Err(error)=>{s.intent_failed(&id,kind,&error);s.submission.as_mut().unwrap()["status"]=json!(if error=="CAPACITY"||permanent_refusal(&error){"refused"}else{"unresolved"});return Err(error)}};
 if kind=="deposit"{
  if s.intents.len()>=4{return Err("CAPACITY".into())}
  s.intents.insert(id,req);if s.intent_due.is_none(){s.intent_due=Some(Instant::now()+Duration::from_millis(150));}return Ok(())
 }
 let result=c.owner.submit_item(&mut c.consumer,req).await;let response=intent_result(c,s,&id,kind,result)?;
 let op=response.item_operation.ok_or("Missing item operation")?;s.operations.insert(id.clone(),op);s.submission=Some(json!({"id":id,"status":"accepted","direction":kind}));s.query_key.clear();s.cache.request();s.page_schedule.request_now();s.status=if kind=="claim"{"Item queued. Collect at the left wing outlet."}else{"Deposit reserved; original still local until escrow."}.into();Ok(())
}
async fn receipt(c:&mut Context,s:&mut State,id:&str,applied:bool)->Result<(),String>{
 let result=receipt_inner(c,s,id,applied).await;s.recovery_schedule.complete(Instant::now(),result.is_ok(),600);
 if let Err(error)=&result{s.receipt_failed(id,error);}else if let Some(error)=&mut s.receipt_error{if error["operation_id"]==id{error["resolved"]=json!(true);}}
 result
}
async fn receipt_inner(c:&mut Context,s:&mut State,id:&str,applied:bool)->Result<(),String>{
 let p=c.consumer.journal.pending().find(|p|p.record.original.operation_id==id).ok_or("Unknown item original")?;let d=p.record.delivery.ok_or("Missing item delivery")?;let outcome=if !applied{"uncertain"}else if p.record.original.direction=="deposit"{"escrowed"}else{"applied"};
 use sha2::{Digest,Sha256};let evidence=Sha256::digest(serde_json::to_vec(&json!({"revision":1,"operationId":id,"deliveryId":d.delivery_id,"payloadDigest":d.payload_digest,"outcome":outcome,"provenance":"client-reported-session-outcome","savePersistence":"unproven"})).unwrap()).iter().map(|b|format!("{b:02x}")).collect();
 let mut r=c.original(id)?;r.item_receipt=Some(wire::ItemReceipt{operation_id:Some(id.into()),delivery_id:Some(d.delivery_id),payload_digest:Some(d.payload_digest),receipt_id:Some(id.into()),outcome:Some(outcome.into()),evidence_digest:Some(evidence)});s.receipt_attempt=Some(json!({"operation_id":id,"receipt_id":r.item_receipt.as_ref().unwrap().receipt_id,"outcome":r.item_receipt.as_ref().unwrap().outcome}));let op=c.call(r).await?.item_operation.ok_or("Missing item receipt")?;c.consumer.journal.validate_item_status(&op)?;s.receipt_success=Some(json!({"operation_id":id,"state":op.state,"bank_revision":op.bank_revision}));s.operations.insert(id.into(),op);s.query_key.clear();s.cache.request();s.page_schedule.request_now();if s.submission.as_ref().is_some_and(|v|v["id"]==id){s.status=if applied{"Item transfer completed in this session."}else{"Item effect uncertain; no automatic replay."}.into();}Ok(())
}
pub(super) async fn action(c:&mut Context,s:&mut State,input:Input)->Result<(),String>{
 if !c.items{return Err("FORBIDDEN".into())}match input{
 Input::ItemCopied{id,channel,offset}=>{if let Some(o)=s.outgoing.front_mut(){if o.id==id&&o.channel==channel&&o.offset==offset{o.offset=(o.offset+CHUNK).min(o.bytes.len());if o.offset==o.bytes.len(){s.outgoing.pop_front();}else if s.outgoing.len()>1{let partial=s.outgoing.pop_front().unwrap();s.outgoing.push_back(partial);}}}},
 Input::ItemChunk{id,channel,offset,total,data}=>{
  let bytes=STANDARD.decode(data).map_err(|_|"Invalid item chunk")?;let key=(channel.clone(),id.clone());
  if !s.incoming.contains_key(&key){if offset!=0||s.incoming.len()>=4{return Err("Item chunks busy".into())}s.incoming.insert(key.clone(),Incoming{total,bytes:Vec::with_capacity(total)});}
  let entry=s.incoming.get_mut(&key).unwrap();if entry.total!=total{return Err("Item chunk conflict".into())}if offset<entry.bytes.len(){if entry.bytes.get(offset..offset+bytes.len())!=Some(bytes.as_slice()){return Err("Item chunk conflict".into())}}else{if offset!=entry.bytes.len(){return Err("Item chunk gap".into())}entry.bytes.extend_from_slice(&bytes);}
  s.copied=Some(json!({"id":id,"channel":channel,"offset":offset}));
  if entry.bytes.len()==total{let data=s.incoming.remove(&key).unwrap().bytes;
   if channel=="deposit"{if !uuid(&id){return Err("Invalid deposit identity".into())}let item=protocol_g3::typed::<wire::ItemDescriptor>("ItemDescriptor",&data).map_err(|_|"Invalid deposit descriptor")?;submit(c,s,id,"deposit",serde_json::to_value(crate::g3_items::descriptor(&item)?).unwrap()).await?;}
   else{let inventory=protocol_g3::typed::<wire::InventoryPublish>("InventoryPublish",&data).map_err(|_|"Invalid inventory snapshot")?;if inventory.wands.as_ref().is_none_or(|v|v.len()>8){return Err("Inventory bound".into())}for slot in inventory.wands.as_deref().unwrap_or(&[]){if slot.slot.is_none_or(|v|v>=8){return Err("Inventory slot".into())}if let Some(w)=&slot.wand{crate::g3_items::descriptor(&wire::ItemDescriptor{catalogue_revision:Some(crate::g3_items::catalogue_revision().into()),wand:Some(w.clone()),..Default::default()})?;}}let mut r=c.request();r.inventory_publish=Some(inventory);c.call(r).await?;}
  }
 },
 Input::ItemClaim{id,item_id,revision}=>submit(c,s,id,"claim",json!({"itemId":item_id,"expectedRevision":revision})).await?,
 Input::ItemQuery{mut query}=>{query.after_notice_sequence=None;query.after_operation_id=None;s.query=query;s.query_key.clear();s.poll=0;s.cache.request();s.page_schedule.request_now();},
 Input::ItemInspect{item_id}=>{let mut r=c.request();r.item_inspect=Some(wire::ItemInspect{item_id:Some(item_id.clone())});let detail=c.call(r).await?.item_detail.ok_or("Missing item detail")?;if let Some(item)=&detail.item{crate::g3_items::descriptor(item)?;}s.queue(item_id,"detail",serde_json::to_value(detail).unwrap())?;},
 Input::InventoryInspect{actor,slot}=>{let mut r=c.request();r.inventory_inspect=Some(wire::InventoryInspect{game_session_id:Some(actor.clone()),slot:Some(slot)});let view=c.call(r).await?.inventory.ok_or("Missing inventory preview")?;s.queue(format!("{actor}-{slot}-{}",view.revision.unwrap_or(0)),"preview",serde_json::to_value(view).unwrap())?;},
 Input::ItemPrepare{id,digest,attempt}=>{if let Some(p)=s.permissions.get(&id){if p.attempt!=attempt||p.digest!=digest{return Err("Item permit conflict".into())}return Ok(())}
  c.original(&id)?;
  if !c.consumer.journal.pending().find(|p|p.record.original.operation_id==id).is_some_and(|p|matches!(p.record.phase,Phase::Requested|Phase::ServerAccepted)){return Err("Earlier item effect unresolved".into())}
  if !s.prepares.contains_key(&id)&&s.prepares.len()+s.permissions.len()>=4{return Err("CAPACITY".into())}
  let retry=s.prepares.entry(id.clone()).or_insert_with(||(digest.clone(),attempt.clone(),PollSchedule::default()));
  if retry.0!=digest||retry.1!=attempt{return Err("Item permit conflict".into())}if !retry.2.ready(Instant::now()){return Ok(())}
  let result=status_inner(c,s,&id).await;s.prepares.get_mut(&id).unwrap().2.complete(Instant::now(),result.is_ok(),600);let op=result?;
  if op.effect_allowed!=Some(true)||op.payload_digest.as_deref()!=Some(&digest){return Err("Item effect refused".into())}c.owner.active()?;c.consumer.journal.recover_item_unissued(&op)?;let permit=c.consumer.journal.prepare_effect(&id)?;
  s.prepares.remove(&id);s.permissions.insert(id.clone(),Permission{permit,attempt:attempt.clone(),digest:digest.clone()});s.effect=Some(json!({"id":id,"digest":digest,"direction":op.direction,"stage":"permit","attempt":attempt}));},
 Input::ItemResult{id,digest,attempt,applied}=>{let p=s.permissions.get(&id).ok_or("No item permission")?;if p.digest!=digest||p.attempt!=attempt{return Err("Item result conflict".into())}let p=s.permissions.remove(&id).unwrap();c.consumer.journal.finish_session_effect(p.permit,applied)?;s.effect=None;receipt(c,s,&id,applied).await?;},_=>return Err("Invalid item action".into()),
 }Ok(())
}
fn pending_receipt_outcome(p:&crate::g3_journal::Pending,s:&State)->Option<bool>{
 if p.restored{return None}let state=s.operations.get(&p.record.original.operation_id).and_then(|o|o.state.as_deref());
 match p.record.phase{Phase::AppliedSession if !terminal_operation(state)=>Some(true),Phase::Uncertain if !matches!(state,Some("uncertain"|"completed"|"cancelled"))=>Some(false),_=>None}
}
fn accept_page(s:&mut State,mut bank:wire::ItemBankSnapshot){if s.bank.is_none(){bank.notices=None;}s.bank_poll=Some(json!({"poll":s.poll,"revision":bank.revision,"total":bank.total}));s.refresh_status=None;s.fresh=true;s.notice=bank.notice_sequence.clone().unwrap_or_default();s.after=bank.next_operation_id.take().unwrap_or_default();bank.operations=None;let mut value=serde_json::to_value(bank).unwrap();value["query"]=serde_json::to_value(&s.query).unwrap();let key=crate::g3_items::canonical(&value).unwrap_or_default();let key=String::from_utf8(key).unwrap_or_default();if s.query_key!=key{s.bank=Some(value.clone());s.pending_bank=Some((uuid::Uuid::new_v4().to_string(),value,key));}queue_cached_page(s);}
#[cfg(test)]pub(super) async fn pump(c:&mut Context,s:&mut State){pump_with_publication(c,s,None).await;}
pub(super) async fn pump_with_publication(c:&mut Context,s:&mut State,publication:Option<(&Arc<Mutex<Exchange>>,Value)>){
 s.enabled=c.items;if !c.items{s.fresh=false;return}s.poll+=1;
 let players_key=c.item_players.to_string();if s.players_key!=players_key{if s.queue(uuid::Uuid::new_v4().to_string(),"players",c.item_players.clone()).is_ok(){s.players_key=players_key;}}
 let pending:Vec<_>=c.consumer.journal.pending().filter(|p|p.record.original.item.is_some()&&c.original(&p.record.original.operation_id).is_ok()).collect();
 let receipts:Vec<_>=pending.iter().filter_map(|p|pending_receipt_outcome(p,s).map(|outcome|(p,outcome))).collect();
 let statuses:Vec<_>=pending.iter().filter(|p|!s.intents.contains_key(&p.record.original.operation_id)&&p.record.phase!=Phase::Refused&&!s.operations.get(&p.record.original.operation_id).is_some_and(|op|known_live_unissued(c,p,op.effect_allowed,op.state.as_deref()))&&pending_receipt_outcome(p,s).is_none()&&!s.permissions.contains_key(&p.record.original.operation_id)&&s.operations.get(&p.record.original.operation_id).is_none_or(|o|!terminal_operation(o.state.as_deref()))).collect();
 let (history,statuses):(Vec<_>,Vec<_>)=statuses.into_iter().partition(|p|historical_original(p));
 let history_turn=!history.is_empty()&&s.history_schedule.ready(Instant::now())&&(statuses.is_empty()&&receipts.is_empty()||s.recovery_passes>=8);
 let statuses=if history_turn{history}else{statuses};
 // One periodic recovery RPC per pass. Acknowledging an effect already made
 // takes precedence over terminal history, with a status turn after3 retries.
 if !s.intents.is_empty()&&s.intent_due.is_none_or(|at|Instant::now()>=at){flush_intents(c,s).await;offer_ready(c,s);return}
 let recovery_ready=history_turn||s.recovery_schedule.ready(Instant::now());
 let page_ready=display_ready(c,&mut s.cache,&mut s.page_schedule);
 let recovery_turn=recovery_ready&&(!receipts.is_empty()||!statuses.is_empty())&&(!page_ready||!s.recovery_turn);
 if recovery_turn{s.recovery_turn=true;}
 let receipt_turn=recovery_turn&&!history_turn&&!receipts.is_empty()&&(statuses.is_empty()||s.recovery_streak<3);
 if receipt_turn{s.scan%=receipts.len();let(p,outcome)=receipts[s.scan];let id=&p.record.original.operation_id;s.scan+=1;s.recovery_streak=s.recovery_streak.saturating_add(1).min(3);let _=receipt(c,s,id,outcome).await;}
 if page_ready&&!recovery_turn{s.recovery_turn=false;s.refresh_status=Some("Refreshing item bank...".into());
  if let Some((stream,mut base))=publication.filter(|_|c.can_publish()){base["items"]=s.view();if let Ok(bytes)=serde_json::to_vec(&base){stream.lock().unwrap().bank_view_for(c.binding.context,bytes);}}
  let mut r=c.request();let mut query=s.query.clone();query.after_notice_sequence=(!s.notice.is_empty()).then(||s.notice.clone());query.after_operation_id=(!s.after.is_empty()).then(||s.after.clone());r.item_bank=Some(query);
  let result=c.call(r).await;s.page_schedule.complete(Instant::now(),result.is_ok(),if result.is_ok()||c.signal().is_none()&&s.cache.revision().is_some()&&!s.cache.ready(Instant::now()){30000}else{600});
  match result{Ok(response)=>{s.cache.accept(response.bank_revision.as_deref().and_then(|v|v.parse().ok()).unwrap_or(0));if let Some(bank)=response.item_bank{accept_page(s,bank);}},Err(e)=>{let code=match e.as_str(){"CAPACITY"|"FORBIDDEN"|"NOT_FOUND"|"STALE_EPOCH"|"SERVICE_UNAVAILABLE"|"INVALID_MESSAGE"|"INCOMPATIBLE_CATALOGUE"=>e.as_str(),_=>"Unclassified bank poll failure"};s.bank_poll=Some(json!({"poll":s.poll,"error":code}));s.refresh_failed(&e);s.notice.clear();}}
 }
 if recovery_turn&&history_turn{s.scan%=statuses.len();let id=&statuses[s.scan].record.original.operation_id;s.scan+=1;let result=status_inner(c,s,id).await;s.history_schedule.complete(Instant::now(),result.is_ok(),30000);s.recovery_passes=0;}
 else if recovery_turn&&!receipt_turn&&statuses.len()>1{s.recovery_streak=0;recover_statuses(c,s,&statuses).await;}
 else if recovery_turn&&!receipt_turn&&!statuses.is_empty(){s.scan%=statuses.len();let p=statuses[s.scan];s.scan+=1;s.recovery_streak=0;let id=&p.record.original.operation_id;
  match status(c,s,id).await{Ok(_)=>{},Err(e)=>{
   if e=="NOT_FOUND"{if let Err(error)=retry_unissued_with(c,s,p,&e,|owner,consumer,req|Box::pin(owner.submit_item(consumer,req))).await{s.foreground_error(id,&error);}}
   else{s.foreground_error(id,&e);}
  }}
 }
 if recovery_turn&&!history_turn{s.recovery_passes=s.recovery_passes.saturating_add(1);}
 offer_ready(c,s);
}
fn queue_cached_page(s:&mut State){
 if let Some((id,value,key))=s.pending_bank.take(){if s.queue(id.clone(),"bank",value.clone()).is_ok(){s.query_key=key;}else{s.pending_bank=Some((id,value,key));}}
}
pub(super) fn offer_ready(c:&Context,s:&mut State){
 let pending:Vec<_>=c.consumer.journal.pending().filter(|p|p.record.original.item.is_some()&&c.current_original(p)).collect();s.effect=None;
 if s.permissions.len()<4{for offset in 0..pending.len(){let index=(s.offer_scan+offset)%pending.len();let p=&pending[index];let id=&p.record.original.operation_id;
  if !matches!(p.record.phase,Phase::Requested|Phase::ServerAccepted)||s.permissions.contains_key(id){continue}
  if let Some(op)=s.operations.get(id).filter(|o|o.effect_allowed==Some(true)&&matches!(o.state.as_deref(),Some("awaiting_local"|"reserved"))).cloned(){
   if let Some(item)=&op.item{if s.payloads.contains(id)||s.queue(id.clone(),"effect_item",serde_json::to_value(item).unwrap()).is_ok(){s.payloads.insert(id.clone());s.effect=Some(json!({"id":id,"digest":op.payload_digest,"direction":op.direction,"stage":"offer"}));s.offer_scan=(index+1)%pending.len();break}}
  }
 }}
 if s.effect.is_none(){s.effect=s.effects().into_iter().next();}
 queue_cached_page(s);
}

#[cfg(test)]mod tests{use super::*;
 include!("g3_item_batch_test.rs");
 include!("g3_item_isolation_test.rs");
 #[test]fn inventory_validates_wands_against_current_canonical_revision(){tauri::async_runtime::block_on(async{
  let(mut c,mut s,dir,_,_)=claim_context();let vectors:Value=serde_json::from_str(include_str!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../packages/contracts/g3-items/operation-digest.json"))).unwrap();let mut descriptor=vectors["vectors"][0]["original"]["payload"].clone();descriptor["catalogueRevision"]=json!(crate::g3_items::catalogue_revision());
  let body=serde_json::to_vec(&json!({"revision":1,"heldSlot":"0","wands":[{"slot":0,"wand":descriptor["wand"]}]})).unwrap();assert!(body.len()<CHUNK);let seen=Arc::new(std::sync::atomic::AtomicU64::new(0));let observed=seen.clone();
  c.owner.test_transport(move|bytes|{let q=protocol_g3::request(bytes).unwrap();assert_eq!(q.inventory_publish.as_ref().unwrap().wands.as_ref().unwrap().len(),1);observed.fetch_add(1,std::sync::atomic::Ordering::AcqRel);Ok(serde_json::to_vec(&wire::Response{major:Some(1),revision:Some(protocol_g3::REVISION.into()),service_id:q.service_id,service_epoch:q.service_epoch,request_id:q.request_id,..Default::default()}).unwrap())});
  action(&mut c,&mut s,Input::ItemChunk{id:"inventory-1".into(),channel:"inventory".into(),offset:0,total:body.len(),data:STANDARD.encode(&body)}).await.unwrap();assert_eq!(seen.load(std::sync::atomic::Ordering::Acquire),1);assert!(s.permissions.is_empty());assert_eq!(c.consumer.journal.pending().count(),1);drop(c);std::fs::remove_file(dir.join("originals.jsonl")).unwrap();std::fs::remove_dir(dir).unwrap();
 });}
 #[test]fn item_duplicate_flushed_copy_does_not_poll_or_reissue_and_keeps_original(){tauri::async_runtime::block_on(async{
  let(mut c,mut s,dir,id,_)=claim_context();let payload:Value=serde_json::from_str(&c.consumer.journal.pending().next().unwrap().record.original.item.unwrap().payload_json).unwrap();let calls=Arc::new(std::sync::atomic::AtomicU64::new(0));let seen=calls.clone();c.owner.test_transport(move|_|{seen.fetch_add(1,std::sync::atomic::Ordering::AcqRel);Err("Unexpected copy replay RPC".into())});
  for _ in 0..8{submit(&mut c,&mut s,id.clone(),"claim",payload.clone()).await.unwrap();}assert_eq!(calls.load(std::sync::atomic::Ordering::Acquire),0);assert_eq!(c.consumer.journal.pending().count(),1);assert_eq!(c.consumer.journal.pending().next().unwrap().record.phase,Phase::Requested);assert!(s.permissions.is_empty());
  c.consumer.journal.refuse(&id).unwrap();assert_eq!(submit(&mut c,&mut s,id,"claim",payload).await.unwrap_err(),"FORBIDDEN");
  drop(c);std::fs::remove_file(dir.join("originals.jsonl")).unwrap();std::fs::remove_dir(dir).unwrap();
 });}

 #[test]fn item_page_capacity_preserves_confirmed_display_but_outage_revokes_freshness_without_permission(){
  let mut s=State::default();s.metadata_error("CAPACITY");assert!(s.submission.is_none()&&!s.status.contains("original")&&s.effect.is_none());s.metadata_error("Late G3 response discarded");assert!(!s.status.contains("original"));s.refresh_failed("CAPACITY");assert!(!s.fresh&&s.bank.is_none()&&s.submission.is_none());assert!(!s.status.contains("original"));
  accept_page(&mut s,wire::ItemBankSnapshot{revision:Some("2".into()),items:Some(vec![]),total:Some(0),..Default::default()});assert!(s.fresh&&s.refresh_status.is_none());let cached=s.bank.clone();
  s.refresh_failed("CAPACITY");assert!(s.fresh&&s.bank==cached);assert!(s.permissions.is_empty()&&s.effect.is_none());
  s.refresh_failed("SERVICE_UNAVAILABLE");assert!(!s.fresh&&s.bank==cached);assert!(s.permissions.is_empty()&&s.effect.is_none());
  accept_page(&mut s,wire::ItemBankSnapshot{revision:Some("2".into()),items:Some(vec![]),total:Some(0),..Default::default()});assert!(s.fresh&&s.refresh_status.is_none());
 }
 #[test]fn item_copy_ack_rotates_large_page_without_starving_effect_descriptor_or_accepting_duplicate(){tauri::async_runtime::block_on(async{
  let(mut c,mut s,dir,_,_)=claim_context();s.queue("large".into(),"bank",json!({"text":"x".repeat(8000)})).unwrap();s.queue("effect".into(),"effect_item",json!({"value":1})).unwrap();
  action(&mut c,&mut s,Input::ItemCopied{id:"large".into(),channel:"bank".into(),offset:0}).await.unwrap();assert_eq!(s.view()["chunk"]["id"],"effect");
  action(&mut c,&mut s,Input::ItemCopied{id:"large".into(),channel:"bank".into(),offset:0}).await.unwrap();assert_eq!(s.view()["chunk"]["id"],"effect","Old duplicate cannot acknowledge another copy");
  action(&mut c,&mut s,Input::ItemCopied{id:"effect".into(),channel:"effect_item".into(),offset:0}).await.unwrap();assert_eq!(s.view()["chunk"]["offset"],CHUNK);assert!(s.permissions.is_empty());
  drop(c);std::fs::remove_file(dir.join("originals.jsonl")).unwrap();std::fs::remove_dir(dir).unwrap();
 });}

 #[test]fn item_repeated_prepare_respects_failure_backoff_without_granting_or_changing_attempt(){tauri::async_runtime::block_on(async{
  let(mut c,mut s,dir,id,_)=claim_context();let digest="a".repeat(64);let attempt="b".repeat(32);
  c.owner.guard(Arc::new(std::sync::atomic::AtomicU64::new(1)),1,Instant::now()-Duration::from_secs(1));
  let input=||Input::ItemPrepare{id:id.clone(),digest:digest.clone(),attempt:attempt.clone()};assert!(action(&mut c,&mut s,input()).await.is_err());assert_eq!(s.prepares[&id].2.failures,1);let next=s.prepares[&id].2.next;
  for _ in 0..20{action(&mut c,&mut s,input()).await.unwrap();}
  assert_eq!(s.prepares[&id].2.next,next);assert_eq!(s.prepares[&id].2.failures,1);assert!(s.permissions.is_empty()&&s.effect.is_none());assert_eq!(c.consumer.journal.pending().next().unwrap().record.phase,Phase::Requested);
  assert!(action(&mut c,&mut s,Input::ItemPrepare{id:id.clone(),digest:digest.clone(),attempt:"c".repeat(32)}).await.is_err());assert_eq!(s.prepares[&id].1,attempt);
  s.prepares.get_mut(&id).unwrap().2.request_now();assert!(action(&mut c,&mut s,input()).await.is_err());assert_eq!(s.prepares[&id].2.failures,2);assert!(s.permissions.is_empty());s.reattach();assert!(s.prepares.is_empty());
  drop(c);std::fs::remove_file(dir.join("originals.jsonl")).unwrap();std::fs::remove_dir(dir).unwrap();
 });}
 #[test]fn item_prepare_progresses_when_due_pumps_interleave_other_status_and_receipts(){tauri::async_runtime::block_on(async{
  let(mut c,mut s,dir,id,req)=claim_context();let mut requests=vec![(id.clone(),req)];
  for _ in 0..2{let other=uuid::Uuid::new_v4().to_string();let req=c.consumer.begin_item_with_id(crate::g3_items::Original{kind:"claim".into(),payload_json:json!({"itemId":uuid::Uuid::new_v4().to_string(),"expectedRevision":"2"}).to_string()},c.consumer.generation(),other.clone()).unwrap();requests.push((other,req));}
  let mut ops=BTreeMap::new();for(key,req)in &requests{let original=c.consumer.journal.pending().find(|p|p.record.original.operation_id==*key).unwrap().record.original;let digest=crate::g3_journal::operation_digest(&original).unwrap();let bytes=intent_reply(req,None,&c.binding.principal_id,&digest);let response:wire::Response=serde_json::from_slice(&bytes).unwrap();ops.insert(key.clone(),response.item_operation.unwrap());}
  // One different original needs an applied receipt; the other needs status.
  let receipt_id=requests[2].0.clone();let bytes=intent_reply(&requests[2].1,None,&c.binding.principal_id,ops[&receipt_id].payload_digest.as_deref().unwrap());
  let accepted=c.owner.submit_item_via(&mut c.consumer,requests[2].1.clone(),move|_,_|Box::pin(async move{Ok(bytes)})).await.unwrap();ops.insert(receipt_id.clone(),accepted.item_operation.unwrap());let permit=c.consumer.journal.prepare_effect(&receipt_id).unwrap();c.consumer.journal.finish_session_effect(permit,true).unwrap();s.operations.insert(receipt_id.clone(),ops[&receipt_id].clone());
  let primary=id.clone();let seen=Arc::new(Mutex::new(Vec::<(String,String)>::new()));let observed=seen.clone();let fail=Arc::new(std::sync::atomic::AtomicBool::new(true));let once=fail.clone();let digest=ops[&id].payload_digest.clone().unwrap();
  c.owner.test_transport(move|bytes|{
   let q=protocol_g3::request(bytes).unwrap();
   if let Some(batch)=&q.bank_batch{
    let entries=batch.entries.as_ref().unwrap().iter().enumerate().map(|(index,e)|{let key=e.item_status.as_ref().unwrap().operation_id.as_ref().unwrap();observed.lock().unwrap().push(("status".into(),key.clone()));wire::BankBatchEntryResult{index:Some(index as u32),operation_id:Some(key.clone()),item_operation:Some(ops[key].clone()),..Default::default()}}).collect();
    return Ok(serde_json::to_vec(&wire::Response{major:Some(1),revision:Some(protocol_g3::REVISION.into()),service_id:q.service_id,service_epoch:q.service_epoch,request_id:q.request_id,bank_batch:Some(wire::BankBatchResult{entries:Some(entries)}),..Default::default()}).unwrap())
   }
   let(kind,key)=if let Some(v)=&q.item_status{("status",v.operation_id.as_ref().unwrap())}else if let Some(v)=&q.item_receipt{("receipt",v.operation_id.as_ref().unwrap())}else{return Err("Unexpected focused transport command".into())};observed.lock().unwrap().push((kind.into(),key.clone()));
   let error=kind=="status"&&*key==primary&&once.swap(false,std::sync::atomic::Ordering::AcqRel);let mut op=ops[key].clone();if kind=="receipt"{op.state=Some("completed".into());op.local_outcome=Some("applied".into());op.effect_allowed=Some(false);}
   Ok(serde_json::to_vec(&wire::Response{major:Some(1),revision:Some(protocol_g3::REVISION.into()),service_id:q.service_id,service_epoch:q.service_epoch,request_id:q.request_id,item_operation:(!error).then_some(op),error:error.then_some(wire::Error{code:Some("CAPACITY".into()),..Default::default()}),..Default::default()}).unwrap())
  });
  s.page_schedule.next=Some(Instant::now()+Duration::from_secs(60));let attempt="b".repeat(32);let input=||Input::ItemPrepare{id:id.clone(),digest:digest.clone(),attempt:attempt.clone()};
  assert_eq!(action(&mut c,&mut s,input()).await.unwrap_err(),"CAPACITY");
  // Hold the logical delay until its explicit advance; no wall-clock sleeps.
  s.prepares.get_mut(&id).unwrap().2.next=Some(Instant::now()+Duration::from_secs(60));let prepare_due=s.prepares[&id].2.next;
  for _ in 0..6{s.recovery_schedule.request_now();pump(&mut c,&mut s).await;action(&mut c,&mut s,input()).await.unwrap();assert!(s.permissions.is_empty(),"A cached successful periodic status cannot grant a permit");assert_eq!(s.prepares[&id].2.next,prepare_due,"Periodic recovery must not postpone prepare eligibility");}
  let events=seen.lock().unwrap().clone();assert!(events.contains(&("receipt".into(),receipt_id)));assert!(events.contains(&("status".into(),requests[1].0.clone())),"Other status work gets a turn");
  let old_world=c.binding.scope.world_token.replace(uuid::Uuid::new_v4().to_string());s.prepares.get_mut(&id).unwrap().2.request_now();let calls=seen.lock().unwrap().len();assert!(action(&mut c,&mut s,input()).await.is_err(),"A substituted current world must not reach transport or permission");assert_eq!(seen.lock().unwrap().len(),calls);assert!(s.permissions.is_empty());c.binding.scope.world_token=old_world;
  // Model the independent prepare delay elapsing, then run a due pump FIRST.
  s.prepares.get_mut(&id).unwrap().2.request_now();s.recovery_schedule.request_now();pump(&mut c,&mut s).await;assert!(!s.recovery_schedule.ready(Instant::now()),"The old shared-slot condition would postpone this prepare again");assert!(s.prepares[&id].2.ready(Instant::now()));let calls=seen.lock().unwrap().len();action(&mut c,&mut s,input()).await.unwrap();
  assert_eq!(seen.lock().unwrap().len(),calls+1,"Prepare executes its own fresh validated status after the pump");assert_eq!(s.permissions.len(),1);assert_eq!(s.effect.as_ref().unwrap()["stage"],"permit");assert_eq!(c.consumer.journal.pending().find(|p|p.record.original.operation_id==id).unwrap().record.phase,Phase::EffectIntent);
  for _ in 0..6{s.recovery_schedule.request_now();pump(&mut c,&mut s).await;action(&mut c,&mut s,input()).await.unwrap();assert_eq!(s.permissions.len(),1);}
  for (key,d,a)in [(id.clone(),"c".repeat(64),attempt.clone()),(id.clone(),digest.clone(),"c".repeat(32)),(uuid::Uuid::new_v4().to_string(),digest.clone(),attempt.clone())]{assert!(action(&mut c,&mut s,Input::ItemPrepare{id:key,digest:d,attempt:a}).await.is_err());}assert_eq!(s.permissions.len(),1);
  s.reattach();assert!(s.permissions.is_empty()&&s.prepares.is_empty());assert!(action(&mut c,&mut s,input()).await.is_err(),"Reattach cannot recreate an already-issued permit");assert!(s.permissions.is_empty());
  drop(c);std::fs::remove_file(dir.join("originals.jsonl")).unwrap();std::fs::remove_dir(dir).unwrap();
 });}
 #[test]fn item_background_page_failures_coalesce_but_explicit_queries_refresh_immediately(){tauri::async_runtime::block_on(async{
  let(mut c,mut s,dir,_,_)=claim_context();
  c.owner.guard(Arc::new(std::sync::atomic::AtomicU64::new(1)),1,Instant::now()-Duration::from_secs(1));
  pump(&mut c,&mut s).await;if s.page_schedule.failures==0{pump(&mut c,&mut s).await;}let first=s.bank_poll.clone();let deadline=s.page_schedule.next;assert_eq!(s.page_schedule.failures,1);assert!(!s.fresh);
  for _ in 0..20{pump(&mut c,&mut s).await;}
  assert_eq!(s.bank_poll,first,"Idle scheduler ticks do not repeat a failed page");assert_eq!(s.page_schedule.next,deadline);
  action(&mut c,&mut s,Input::ItemQuery{query:wire::ItemBank{search:Some("wand".into()),..Default::default()}}).await.unwrap();
  assert!(s.page_schedule.ready(Instant::now()));pump(&mut c,&mut s).await;assert_eq!(s.page_schedule.failures,2,"An explicit query runs despite background backoff");assert_eq!(s.query.search.as_deref(),Some("wand"));assert!(s.permissions.is_empty());
  drop(c);std::fs::remove_file(dir.join("originals.jsonl")).unwrap();std::fs::remove_dir(dir).unwrap();
 });}
 fn claim_context()->(Context,State,std::path::PathBuf,String,Vec<u8>){
  let dir=std::env::temp_dir().join(format!("nt2-item-capacity-{}",uuid::Uuid::new_v4()));std::fs::create_dir(&dir).unwrap();
  let binding=Binding{context:[1;16],service_origin:"https://fixture.invalid".into(),service_id:"service".into(),service_epoch:uuid::Uuid::new_v4().to_string(),session_epoch:uuid::Uuid::new_v4().to_string(),principal_id:uuid::Uuid::new_v4().to_string(),scope:wire::Scope{room_id:Some(uuid::Uuid::new_v4().to_string()),run_id:Some(uuid::Uuid::new_v4().to_string()),run_epoch:Some(uuid::Uuid::new_v4().to_string()),config_revision:Some(1),attachment_id:Some(uuid::Uuid::new_v4().to_string()),world_token:Some(uuid::Uuid::new_v4().to_string())}};
  let mut consumer=LocalConsumer::new(crate::g3_journal::Journal::open(&dir.join("originals.jsonl")).unwrap());consumer.bind(binding.clone()).unwrap();let id=uuid::Uuid::new_v4().to_string();
  let req=consumer.begin_item_with_id(crate::g3_items::Original{kind:"claim".into(),payload_json:json!({"itemId":uuid::Uuid::new_v4().to_string(),"expectedRevision":"2"}).to_string()},consumer.generation(),id.clone()).unwrap();
  let owner=HttpOwner::new(binding.clone(),zeroize::Zeroizing::new("fixture-credential".into()),Arc::new(std::sync::atomic::AtomicU64::new(1))).unwrap();
  (Context{binding,consumer,owner,events:None,items:true,item_players:json!([])},State{submission:Some(json!({"id":id,"direction":"claim","status":"pending"})),..Default::default()},dir,id,req)
 }
 fn intent_reply(bytes:&[u8],error:Option<&str>,principal:&str,digest:&str)->Vec<u8>{
  let q=protocol_g3::request(bytes).unwrap();let operation=q.item_claim.as_ref().unwrap();
  let op=wire::ItemOperation{operation_id:operation.operation_id.clone(),principal_id:Some(principal.into()),original_scope:q.scope.clone(),direction:Some("claim".into()),item_id:operation.item_id.clone(),delivery_id:Some(uuid::Uuid::new_v4().to_string()),payload_digest:Some(digest.into()),state:Some("reserved".into()),item:Some(serde_json::from_value(json!({"catalogueRevision":"nt2-items-base-2","spell":{"actionId":"LIGHT_BULLET","usesRemaining":"-1"}})).unwrap()),effect_allowed:Some(true),save_persistence:Some("unproven".into()),bank_revision:Some("3".into()),..Default::default()};
  serde_json::to_vec(&wire::Response{major:Some(1),revision:Some(protocol_g3::REVISION.into()),service_id:q.service_id,service_epoch:q.service_epoch,request_id:q.request_id,item_operation:error.is_none().then_some(op),error:error.map(|code|wire::Error{code:Some(code.into()),..Default::default()}),..Default::default()}).unwrap()
 }
 #[test]fn cold_pre_effect_status_bypasses_history_delay_without_granting_or_replaying(){tauri::async_runtime::block_on(async{
  let(mut c,_,dir,id,req)=claim_context();let original=c.consumer.journal.pending().next().unwrap().record.original;let digest=crate::g3_journal::operation_digest(&original).unwrap();let op=protocol_g3::response(&intent_reply(&req,None,&c.binding.principal_id,&digest)).unwrap().item_operation.unwrap();c.consumer.journal.accept_item_operation(&op).unwrap();
  let reopen=|c:Context|{let Context{binding,consumer,owner,events,items,item_players}=c;drop(consumer);let journal=crate::g3_journal::Journal::open(&dir.join("originals.jsonl")).unwrap();let mut consumer=LocalConsumer::new(journal);consumer.bind(binding.clone()).unwrap();Context{binding,consumer,owner,events,items,item_players}};c=reopen(c);assert!(c.consumer.journal.pending().next().unwrap().restored);
  let count=Arc::new(std::sync::atomic::AtomicU64::new(0));let seen=count.clone();let reply=op.clone();c.owner.test_transport(move|bytes|{let q=protocol_g3::request(bytes).unwrap();assert!(q.item_status.is_some(),"Only status may run before a fresh explicit prepare");seen.fetch_add(1,std::sync::atomic::Ordering::AcqRel);Ok(serde_json::to_vec(&wire::Response{major:Some(1),revision:Some(protocol_g3::REVISION.into()),service_id:q.service_id,service_epoch:q.service_epoch,request_id:q.request_id,item_operation:Some(reply.clone()),..Default::default()}).unwrap())});
  let mut s=State::default();s.cache.accept(0);s.page_schedule.next=Some(Instant::now()+Duration::from_secs(60));s.history_schedule.next=s.page_schedule.next;pump(&mut c,&mut s).await;assert_eq!(count.load(std::sync::atomic::Ordering::Acquire),1);assert_eq!(s.effect.as_ref().unwrap()["stage"],"offer");assert!(s.permissions.is_empty());assert!(c.consumer.journal.prepare_effect(&id).is_err(),"Restored prefix still needs fresh prepare status");
  c.consumer.journal.recover_item_unissued(&op).unwrap();let _permit=c.consumer.journal.prepare_effect(&id).unwrap();c=reopen(c);let mut s=State::default();s.cache.accept(0);s.page_schedule.next=Some(Instant::now()+Duration::from_secs(60));s.history_schedule.next=s.page_schedule.next;pump(&mut c,&mut s).await;assert_eq!(count.load(std::sync::atomic::Ordering::Acquire),1,"Ambiguous effect stays on slow history lane");assert!(s.permissions.is_empty()&&s.effect.is_none());drop(c);std::fs::remove_file(dir.join("originals.jsonl")).unwrap();std::fs::remove_dir(dir).unwrap();
 });}
 #[test]fn item_applied_receipt_gets_a_turn_before_completed_history_without_starving_status_or_replaying_cold(){tauri::async_runtime::block_on(async{
  let(mut c,mut s,dir,first_id,first_req)=claim_context();let mut urgent=String::new();
  for index in 0..5{
   let(id,req)=if index==0{(first_id.clone(),first_req.clone())}else{let id=uuid::Uuid::new_v4().to_string();let req=c.consumer.begin_item_with_id(crate::g3_items::Original{kind:"claim".into(),payload_json:json!({"itemId":uuid::Uuid::new_v4().to_string(),"expectedRevision":"2"}).to_string()},c.consumer.generation(),id.clone()).unwrap();(id,req)};
   let original=c.consumer.journal.pending().find(|p|p.record.original.operation_id==id).unwrap().record.original;let digest=crate::g3_journal::operation_digest(&original).unwrap();let bytes=intent_reply(&req,None,&c.binding.principal_id,&digest);
   let response=c.owner.submit_item_via(&mut c.consumer,req,move|_,_|Box::pin(async move{Ok(bytes)})).await.unwrap();let mut op=response.item_operation.unwrap();let permit=c.consumer.journal.prepare_effect(&id).unwrap();c.consumer.journal.finish_session_effect(permit,true).unwrap();
   if index<4{op.state=Some("completed".into());op.effect_allowed=Some(false);c.consumer.journal.validate_item_status(&op).unwrap();}else{urgent=id.clone();}s.operations.insert(id,op);
  }
  s.scan=c.consumer.journal.pending().position(|p|s.operations[&p.record.original.operation_id].state.as_deref()==Some("completed")).unwrap();s.poll=1;
  // Controlled expired transport makes the actual selected receipt observable
  // without network/SQL. It is not evidence of the integration failure's cause.
  c.owner.guard(Arc::new(std::sync::atomic::AtomicU64::new(1)),1,Instant::now()-Duration::from_secs(1));pump(&mut c,&mut s).await;
  assert_eq!(s.diagnostics()["receipt_failure"]["operation_id"],urgent);assert_eq!(s.receipt_failures,1);assert!(s.permissions.is_empty()&&s.effect.is_none());
  let waiting=uuid::Uuid::new_v4().to_string();c.consumer.begin_item_with_id(crate::g3_items::Original{kind:"claim".into(),payload_json:json!({"itemId":uuid::Uuid::new_v4().to_string(),"expectedRevision":"2"}).to_string()},c.consumer.generation(),waiting).unwrap();s.recovery_streak=3;s.status="status turn not run".into();s.recovery_schedule.request_now();pump(&mut c,&mut s).await;assert_eq!(s.receipt_failures,1,"A page gets its fair turn between recoveries");s.recovery_schedule.request_now();pump(&mut c,&mut s).await;assert_eq!(s.receipt_failures,1,"An ordinary status gets the next bounded fair turn");assert_eq!(s.status,"status turn not run","Unrelated background status cannot replace current feedback");assert!(!s.recovery_schedule.ready(Instant::now()),"Ordinary status completed and scheduled independently");
  let Context{binding,consumer,owner,events,items,item_players}=c;drop(consumer);let journal=crate::g3_journal::Journal::open(&dir.join("originals.jsonl")).unwrap();let mut consumer=LocalConsumer::new(journal);consumer.bind(binding.clone()).unwrap();let mut c=Context{binding,consumer,owner,events,items,item_players};assert!(c.consumer.journal.pending().all(|p|p.restored));s.poll=4;pump(&mut c,&mut s).await;assert_eq!(s.receipt_failures,1,"Cold applied outcomes cannot produce an automatic positive receipt");assert!(s.permissions.is_empty()&&s.effect.is_none());
  drop(c);std::fs::remove_file(dir.join("originals.jsonl")).unwrap();std::fs::remove_dir(dir).unwrap();
 });}
 #[test]fn item_uncertain_receipt_retries_the_same_metadata_and_stops_on_ack_or_cold_reopen(){tauri::async_runtime::block_on(async{
  let(mut c,mut s,dir,id,req)=claim_context();let original=c.consumer.journal.pending().next().unwrap().record.original;let digest=crate::g3_journal::operation_digest(&original).unwrap();let bytes=intent_reply(&req,None,&c.binding.principal_id,&digest);let response=c.owner.submit_item_via(&mut c.consumer,req,move|_,_|Box::pin(async move{Ok(bytes)})).await.unwrap();s.operations.insert(id.clone(),response.item_operation.unwrap());let permit=c.consumer.journal.prepare_effect(&id).unwrap();c.consumer.journal.finish_session_effect(permit,false).unwrap();
  c.owner.guard(Arc::new(std::sync::atomic::AtomicU64::new(1)),1,Instant::now()-Duration::from_secs(1));s.poll=1;pump(&mut c,&mut s).await;let first=s.receipt_attempt.clone().unwrap();assert_eq!(first["operation_id"],id);assert_eq!(first["receipt_id"],id);assert_eq!(first["outcome"],"uncertain");assert_eq!(s.diagnostics()["receipt_failure"]["code"],"Retired G3 owner generation");let scheduled=s.recovery_schedule.next;pump(&mut c,&mut s).await;assert_eq!(s.receipt_failures,1,"Immediate optional ticks do not retry a failed receipt");assert_eq!(s.recovery_schedule.next,scheduled);s.recovery_schedule.request_now();pump(&mut c,&mut s).await;assert_eq!(s.receipt_attempt.as_ref().unwrap(),&first);assert_eq!(s.receipt_failures,2);assert!(c.consumer.journal.prepare_effect(&id).is_err());assert!(s.permissions.is_empty()&&s.effect.is_none());
  let mut acknowledged=s.operations[&id].clone();acknowledged.state=Some("uncertain".into());acknowledged.local_outcome=Some("uncertain".into());acknowledged.effect_allowed=Some(false);c.consumer.journal.validate_item_status(&acknowledged).unwrap();s.operations.insert(id.clone(),acknowledged);s.recovery_schedule.request_now();s.poll=4;pump(&mut c,&mut s).await;assert_eq!(s.receipt_failures,2,"Acknowledged uncertainty is not reported repeatedly");assert!(s.recovery_schedule.next.is_none(),"Acknowledged uncertainty performs no status RPC either");
  let Context{binding,consumer,owner,events,items,item_players}=c;drop(consumer);let journal=crate::g3_journal::Journal::open(&dir.join("originals.jsonl")).unwrap();let mut consumer=LocalConsumer::new(journal);consumer.bind(binding.clone()).unwrap();let mut c=Context{binding,consumer,owner,events,items,item_players};s.operations.get_mut(&id).unwrap().state=Some("reserved".into());s.receipt_attempt=None;s.poll=1;pump(&mut c,&mut s).await;assert!(s.receipt_attempt.is_none(),"Cold ambiguous outcomes remain status-only");assert_eq!(s.receipt_failures,2);assert!(c.consumer.journal.prepare_effect(&id).is_err());
  drop(c);std::fs::remove_file(dir.join("originals.jsonl")).unwrap();std::fs::remove_dir(dir).unwrap();
 });}
 #[test]fn item_capacity_recovery_reuses_flushed_original_and_cannot_grant_an_effect(){tauri::async_runtime::block_on(async{
  let(mut c,mut s,dir,id,req)=claim_context();let p=c.consumer.journal.pending().next().unwrap();let digest=crate::g3_journal::operation_digest(&p.record.original).unwrap();let principal=c.binding.principal_id.clone();
  let result=c.owner.submit_item_via(&mut c.consumer,req,|_,bytes|Box::pin(async move{Ok(intent_reply(&bytes,Some("CAPACITY"),"",""))})).await;
  assert_eq!(intent_result(&mut c,&mut s,&id,"claim",result).unwrap_err(),"CAPACITY");s.error("CAPACITY");s.error("NOT_FOUND");assert_eq!(s.submission.as_ref().unwrap()["status"],"unresolved");assert_eq!(c.consumer.journal.pending().next().unwrap().record.phase,Phase::Requested);
  let expected=id.clone();retry_unissued_with(&mut c,&mut s,&p,"NOT_FOUND",move|owner,consumer,req|Box::pin(async move{
   owner.submit_item_via(consumer,req,move|_,bytes|Box::pin(async move{let q=protocol_g3::request(&bytes).unwrap();assert_eq!(q.item_claim.as_ref().unwrap().operation_id.as_deref(),Some(expected.as_str()));Ok(intent_reply(&bytes,None,&principal,&digest))})).await
  })).await.unwrap();
  assert_eq!(c.consumer.journal.pending().count(),1);assert_eq!(c.consumer.journal.pending().next().unwrap().record.phase,Phase::ServerAccepted);assert_eq!(s.submission.as_ref().unwrap()["status"],"accepted");assert!(s.permissions.is_empty()&&s.effect.is_none(),"Transport acceptance is not an effect permit");
  s.submission=Some(json!({"id":uuid::Uuid::new_v4().to_string(),"status":"pending"}));let old=s.operations[&id].clone();recovered_submission(&mut s,&old);assert_eq!(s.submission.as_ref().unwrap()["status"],"pending","An older correlated result cannot acknowledge a different submission");
  drop(c);std::fs::remove_file(dir.join("originals.jsonl")).unwrap();std::fs::remove_dir(dir).unwrap();
 });}
 #[test]fn item_capacity_recovery_stops_on_permanent_refusal_and_never_reissues_restored_or_other_scope(){tauri::async_runtime::block_on(async{
  let(mut c,mut s,dir,id,_req)=claim_context();let p=c.consumer.journal.pending().next().unwrap();intent_result(&mut c,&mut s,&id,"claim",Err("CAPACITY".into())).unwrap_err();
  let calls=Arc::new(std::sync::atomic::AtomicU64::new(0));let mut restored=p.clone();restored.restored=true;
  for candidate in [&restored,&p]{let old=c.binding.scope.world_token.clone();if !candidate.restored{c.binding.scope.world_token=Some(uuid::Uuid::new_v4().to_string());}let calls=calls.clone();retry_unissued_with(&mut c,&mut s,candidate,"NOT_FOUND",move|_,_,_|Box::pin(async move{calls.fetch_add(1,std::sync::atomic::Ordering::AcqRel);Err("unexpected transport".into())})).await.unwrap();c.binding.scope.world_token=old;}
  assert_eq!(calls.load(std::sync::atomic::Ordering::Acquire),0);
  let error=retry_unissued_with(&mut c,&mut s,&p,"NOT_FOUND",|owner,consumer,req|Box::pin(async move{owner.submit_item_via(consumer,req,|_,bytes|Box::pin(async move{Ok(intent_reply(&bytes,Some("POLICY_DISABLED"),"",""))})).await})).await.unwrap_err();s.error(&error);
  assert_eq!(error,"POLICY_DISABLED");assert_eq!(s.submission.as_ref().unwrap()["status"],"refused");let refused=c.consumer.journal.pending().next().unwrap();assert_eq!(refused.record.phase,Phase::Refused);
  let spy=calls.clone();retry_unissued_with(&mut c,&mut s,&refused,"NOT_FOUND",move|_,_,_|Box::pin(async move{spy.fetch_add(1,std::sync::atomic::Ordering::AcqRel);Err("unexpected transport".into())})).await.unwrap();assert_eq!(calls.load(std::sync::atomic::Ordering::Acquire),0);assert!(s.permissions.is_empty());
  s.submission=None;let payload:Value=serde_json::from_str(&refused.record.original.item.unwrap().payload_json).unwrap();assert_eq!(submit(&mut c,&mut s,id.clone(),"claim",payload).await.unwrap_err(),"FORBIDDEN");assert_eq!(s.submission.as_ref().unwrap()["status"],"refused","A repeated native original keeps its terminal refusal visible");
  drop(c);std::fs::remove_file(dir.join("originals.jsonl")).unwrap();std::fs::remove_dir(dir).unwrap();
 });}
 #[test]fn item_receipt_real_seam_keeps_failure_and_applied_original_without_replay(){tauri::async_runtime::block_on(async{
  use crate::g3_journal::Journal;use std::sync::atomic::AtomicU64;
  let dir=std::env::temp_dir().join(format!("nt2-item-receipt-{}",uuid::Uuid::new_v4()));std::fs::create_dir(&dir).unwrap();let path=dir.join("originals.jsonl");
  let binding=Binding{context:[1;16],service_origin:"https://fixture.invalid".into(),service_id:"service".into(),service_epoch:"epoch".into(),session_epoch:"session".into(),principal_id:"principal".into(),scope:wire::Scope{room_id:Some("room".into()),run_id:Some("run".into()),run_epoch:Some("run-epoch".into()),config_revision:Some(1),attachment_id:Some("attachment".into()),world_token:Some("world".into())}};
  let mut consumer=LocalConsumer::new(Journal::open(&path).unwrap());consumer.bind(binding.clone()).unwrap();let id=uuid::Uuid::new_v4().to_string();let item=json!({"catalogueRevision":"nt2-items-base-2","spell":{"actionId":"LIGHT_BULLET","usesRemaining":"-1"}});consumer.begin_item_with_id(crate::g3_items::Original{kind:"deposit".into(),payload_json:item.to_string()},consumer.generation(),id.clone()).unwrap();let original=consumer.journal.pending().next().unwrap().record.original;
  consumer.journal.accept_item_operation(&wire::ItemOperation{operation_id:Some(id.clone()),principal_id:Some(binding.principal_id.clone()),original_scope:Some(binding.scope.clone()),direction:Some("deposit".into()),item_id:Some(uuid::Uuid::new_v4().to_string()),delivery_id:Some(uuid::Uuid::new_v4().to_string()),payload_digest:Some(crate::g3_journal::operation_digest(&original).unwrap()),item:Some(serde_json::from_value(item).unwrap()),state:Some("awaiting_local".into()),save_persistence:Some("unproven".into()),..Default::default()}).unwrap();let permit=consumer.journal.prepare_effect(&id).unwrap();consumer.journal.finish_session_effect(permit,true).unwrap();
  let fence=Arc::new(AtomicU64::new(1));let mut owner=HttpOwner::new(binding.clone(),zeroize::Zeroizing::new("fixture-credential".into()),fence.clone()).unwrap();owner.guard(fence,1,Instant::now()-Duration::from_secs(1));let mut c=Context{binding,consumer,owner,events:None,items:true,item_players:json!([])};let mut state=State::default();
  for count in 1..=2{assert_eq!(receipt(&mut c,&mut state,&id,true).await.unwrap_err(),"Retired G3 owner generation");assert_eq!(state.diagnostics()["receipt_failure"]["code"],"Retired G3 owner generation");assert_eq!(state.receipt_failures,count);assert_eq!(c.consumer.journal.pending().next().unwrap().record.phase,Phase::AppliedSession);assert!(c.consumer.journal.prepare_effect(&id).is_err());}
  drop(c);std::fs::remove_file(path).unwrap();std::fs::remove_dir(dir).unwrap();
 });}
 #[test]fn item_receipt_failure_retains_bounded_concrete_code_without_authority(){let mut s=State::default();let id=uuid::Uuid::new_v4().to_string();s.receipt_failed(&id,"INVALID_STATE");assert_eq!(s.diagnostics()["receipt_failure"]["code"],"INVALID_STATE");assert_eq!(s.diagnostics()["receipt_failure"]["operation_id"],id);assert!(s.permissions.is_empty()&&s.operations.is_empty());s.intent_failed(&id,"claim","CAPACITY");s.error("NOT_FOUND");assert_eq!(s.diagnostics()["intent_failure"]["code"],"CAPACITY");s.intent_failed(&id,"claim","bearer private credential");assert_eq!(s.diagnostics()["intent_failure"]["code"],"Unclassified item intent failure");s.error("FORBIDDEN");assert_eq!(s.diagnostics()["receipt_failure"]["code"],"INVALID_STATE","Unrelated bank failures do not erase receipt evidence");s.receipt_failed(&id,"Retired G3 owner generation");assert_eq!(s.diagnostics()["receipt_failure"]["code"],"Retired G3 owner generation");assert_eq!(s.receipt_failures,2);for bad in ["bearer private credential".to_owned(),"x".repeat(161),"line\ncontrol".into()]{s.receipt_failed(&id,&bad);assert_eq!(s.diagnostics()["receipt_failure"]["code"],"Unclassified item receipt failure");}assert!(serde_json::to_vec(&s.view()).unwrap().len()<4096);}
 #[test]fn item_chunks_are_bounded_and_output_advances_only_exact_copy(){let id=uuid::Uuid::new_v4().to_string();let mut s=State::default();s.queue(id.clone(),"detail",json!({"value":"x".repeat(6000)})).unwrap();let first=s.view();assert!(serde_json::to_vec(&first).unwrap().len()<4096);let o=s.outgoing.front_mut().unwrap();assert_eq!(o.offset,0);o.offset=CHUNK;assert_ne!(s.view()["chunk"]["offset"],first["chunk"]["offset"]);for offset in [0,2304]{assert!(validate_input(&Input::ItemChunk{id:id.clone(),channel:"deposit".into(),offset,total:6000,data:STANDARD.encode(vec![1;CHUNK])}).is_ok());}assert!(validate_input(&Input::ItemChunk{id,channel:"deposit".into(),offset:0,total:131073,data:"eA==".into()}).is_err());}}
