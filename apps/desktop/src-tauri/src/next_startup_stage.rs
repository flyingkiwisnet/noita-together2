//! Independent preparation reservation/staging worker. No full Core clone and
//! no disk/network work under the UI/Core, pending or coordinator mutex.
use super::*;use nt2_desktop_g0::{startup_local::Prepared,startup_pending as local,protocol_waiting::{self as protocol,wire as w}};
#[derive(Default)]pub(super) struct Runner{pub(super) config:Option<Value>,task:Option<tokio::task::JoinHandle<Result<Ready,String>>>,key:String,next:Option<Instant>,attach:Option<tokio::task::JoinHandle<Result<(String,nt2_desktop_g0::startup_local::WaitingMarker),String>>>,attach_key:String,attach_saved:String}
struct Ready{bundle:Arc<Prepared>,key:String}
impl Drop for Runner{fn drop(&mut self){if let Some(t)=&self.task{t.abort()}if let Some(t)=&self.attach{t.abort()}}}
fn identity(p:&handoff::Pending)->Option<String>{let b=p.startup_bundle.as_ref()?;Some(json!([b.scope(),p.seed,p.world_policy,p.generation.as_ref()?.1]).to_string())}
fn current(p:&Arc<Mutex<Option<handoff::Pending>>>,key:&str)->bool{p.lock().unwrap().as_ref().is_some_and(|p|!p.startup_ready&&p.owner.is_none()&&identity(p).as_deref()==Some(key)&&p.generation.as_ref().is_some_and(|(g,n)|g.load(Ordering::Acquire)==*n))}
impl<S:CredentialStore> Core<S>{
 pub(super) async fn seal_startup_attach(&mut self,p:&handoff::Pending,retry:&Retry)->Result<bool,String> where S:'static{
  let Some(bundle)=p.startup_bundle.clone()else{return Ok(true)};if !p.startup_ready{return Err("Startup canonical reservation was not sealed".into())}
  let owner=p.owner.ok_or("Startup claim owner absent")?;let request_id=retry.request.request_id.clone().ok_or("Original attach ID absent")?;let key=json!([bundle.digest(),owner,request_id]).to_string();
  if self.startup_stage.attach.as_ref().is_some_and(|t|t.is_finished()){
   let (saved,marker)=self.startup_stage.attach.take().unwrap().await.map_err(|_|"Attach seal worker canceled")??;
   if saved==key&&self.pending.lock().unwrap().as_ref().is_some_and(|p|p.owner==Some(owner)&&p.startup_bundle.as_ref().is_some_and(|b|b.digest()==bundle.digest())&&p.generation.as_ref().is_some_and(|(g,n)|g.load(Ordering::Acquire)==*n)){
    self.startup_stage.attach_saved=saved;let mut d=self.native_diagnostics.lock().unwrap();if d.rejoin.metadata.as_ref().is_none_or(|(_,_,b)|nt2_desktop_g0::rejoin_local::Association::decode(b).is_err()){d.rejoin.metadata=Some((owner.0,owner.1,marker.encode()?));}
   }
  }
  if self.startup_stage.attach_saved==key{return Ok(true)}
  if self.startup_stage.attach.is_some(){return Ok(false)}
  let path=p.claim_path.clone().ok_or("Startup claim path absent")?;let actual=p.actual_seed.ok_or("Startup actual seed not yet observed")?;
  let request=serde_json::from_value(serde_json::to_value(&retry.request).map_err(|_|"Attach original encoding")?).map_err(|_|"Attach original shape")?;let original=local::AttachOriginal::capture(bundle.scope(),request,&bundle.scope().desktop)?;
  let store=self.store.clone();let writer=self.credential_writer.clone();let pending=self.pending.clone();let expected=p.generation.clone().ok_or("Startup generation absent")?;self.startup_stage.attach_key=key.clone();
  self.startup_stage.attach=Some(tokio::task::spawn_blocking(move||{
   let valid=||pending.lock().unwrap().as_ref().is_some_and(|p|p.owner==Some(owner)&&p.startup_bundle.as_ref().is_some_and(|b|b.digest()==bundle.digest())&&expected.0.load(Ordering::Acquire)==expected.1);
   let _guard=writer.lock().unwrap();if !valid(){return Err("Startup attach owner retired before seal".into())}let adapter=rejoin::ResumeStore(store);let mut r=local::load(&adapter,bundle.scope(),bundle.world())?.ok_or("Startup reservation missing before attach")?;if !r.ready(){return Err("Startup proof unavailable before attach".into())}
   if let Some(old)=&r.attach{if serde_json::to_value(&old.original).ok()!=serde_json::to_value(&original.original).ok(){return Err("Exact original attachment differs; outcome required".into())}}else{r.attach=Some(original);local::save(&adapter,&r)?}
   let claim=std::fs::read(&path).map_err(|_|"Startup claim witness unavailable")?;let marker=nt2_desktop_g0::startup_local::persist_pending(&adapter,&bundle,actual,owner.0,owner.1,&claim)?;if !valid(){return Err("Startup attach owner retired during seal".into())}Ok((key,marker))
  }));Ok(false)
 }
 pub(super) async fn poll_startup_stage(&mut self) where S:'static{
  if self.startup_stage.task.as_ref().is_some_and(|t|t.is_finished()){
   match self.startup_stage.task.take().unwrap().await{Ok(Ok(ready))=>{let mut slot=self.pending.lock().unwrap();if let Some(p)=slot.as_mut().filter(|p|identity(p).as_deref()==Some(&ready.key)&&p.owner.is_none()&&p.generation.as_ref().is_some_and(|(g,n)|g.load(Ordering::Acquire)==*n)){p.expires=p.expires.min(ready.bundle.until());p.authority_until=(Instant::now()+Duration::from_secs(12)).min(p.expires);p.startup_bundle=Some(ready.bundle);p.startup_ready=true;self.native_stage="ready-for-noita";self.native_failure=None;}},Ok(Err(e)) if self.pending.lock().unwrap().as_ref().is_some_and(|p|identity(p).as_deref()==Some(&self.startup_stage.key))=>{self.native_failure=Some(format!("Startup is not ready: {e}"));self.native_stage="startup-staging-failed";self.startup_stage.next=Some(Instant::now()+Duration::from_secs(2));},_=>{}}
  }
  let Some(p)=self.pending.lock().unwrap().clone()else{return};if p.startup_ready||p.owner.is_some(){return}let Some(key)=identity(&p)else{return};if self.startup_stage.key!=key{self.startup_stage.task.take().map(|t|t.abort());self.startup_stage.key=key.clone();self.startup_stage.next=None;}
  if self.startup_stage.task.is_some()||self.startup_stage.next.is_some_and(|at|Instant::now()<at){return}
  let Some(config)=self.startup_stage.config.clone()else{return};let Some(candidate)=p.startup_bundle.clone()else{return};let Some(desktop)=self.desktop.clone()else{return};let Some(path)=p.claim_path.clone()else{return};let store=self.store.clone();let writer=self.credential_writer.clone();let pending=self.pending.clone();let http=self.http.clone();let coordinator=self.lifecycle.clone();self.native_stage="reserving-startup";
  self.startup_stage.task=Some(tokio::spawn(async move{stage(store,writer,pending,http,coordinator,desktop,candidate,config,path,p.expires,key).await}));
 }
}
#[cfg(test)]mod tests{
 use super::*;use std::sync::atomic::AtomicBool;
 #[derive(Clone,Default)]struct SlowStore{inner:super::super::tests::MemoryStore,entered:Arc<AtomicBool>,release:Arc<AtomicBool>}
 impl CredentialStore for SlowStore{
  fn save(&self,p:&ServiceProfile,b:&SecretBlob)->Result<(),crate::secrets::StoreError>{self.entered.store(true,Ordering::Release);while !self.release.load(Ordering::Acquire){std::thread::sleep(Duration::from_millis(2))}self.inner.save(p,b)}
  fn load(&self,p:&ServiceProfile)->Result<Option<SecretBlob>,crate::secrets::StoreError>{self.inner.load(p)}
  fn delete(&self,p:&ServiceProfile)->Result<(),crate::secrets::StoreError>{self.inner.delete(p)}
 }
 #[test]fn startup_repair_slow_staging_does_not_hold_core_status_cancel_or_expose_seed(){tauri::async_runtime::block_on(async{
  let(base,mut p,_)=super::super::tests::release_case();let store=SlowStore::default();let root=std::env::temp_dir().join(format!("nt2-stage-{}",uuid::Uuid::new_v4()));let mut c=Core::new(store.clone(),uuid::Uuid::new_v4(),Arc::new(Mutex::new(None)),root.clone());c.base="https://stage.invalid".into();c.service=base.service;c.epoch=base.epoch;c.desktop=base.desktop;c.room=base.room;c.preparation=base.preparation;c.generation.store(7,Ordering::Release);
  let config=serde_json::to_value(c.room.as_ref().unwrap().config.as_ref().unwrap()).unwrap();let scope=nt2_desktop_g0::startup_local::Scope{origin:c.base.clone(),service:c.service.clone(),service_epoch:c.epoch.clone(),account:c.desktop.as_ref().unwrap().account.clone(),device:c.device,desktop:c.desktop.as_ref().unwrap().id.clone(),desktop_epoch:c.desktop.as_ref().unwrap().epoch.clone(),room:c.room.as_ref().unwrap().room_id.clone().unwrap(),run:p.run.clone(),run_epoch:p.epoch.clone(),config:p.config,config_digest:nt2_desktop_g0::protocol_connection::canonical_fingerprint(&serde_json::to_vec(&config).unwrap()).unwrap(),preparation:c.preparation.as_ref().unwrap().preparation_id.clone().unwrap()};
  p.owner=None;p.startup_ready=false;p.startup_bundle=Some(Arc::new(Prepared::stage(scope,p.seed,p.world_policy,7,Instant::now()+Duration::from_secs(60)).unwrap()));p.claim_path=Some(root.join("claim"));p.generation=Some((c.generation.clone(),7));*c.pending.lock().unwrap()=Some(p);c.startup_stage.config=Some(config);
  c.poll_startup_stage().await;let until=Instant::now()+Duration::from_secs(3);while !store.entered.load(Ordering::Acquire){assert!(Instant::now()<until);tokio::task::yield_now().await;}
  let began=Instant::now();assert_eq!(c.view()["native_armed"],false);let _status=c.view();c.clear_game();assert!(began.elapsed()<Duration::from_millis(50),"Slow protected persistence blocked Core status/cancel");store.release.store(true,Ordering::Release);
  while !c.startup_stage.task.as_ref().unwrap().is_finished(){assert!(Instant::now()<until);tokio::task::yield_now().await;}c.poll_startup_stage().await;assert!(c.pending.lock().unwrap().is_none());assert_ne!(c.native_stage,"ready-for-noita");assert!(!root.join("claim").exists());
 });}
}
async fn stage<S:CredentialStore+'static>(store:Arc<S>,writer:Arc<Mutex<()>>,pending:Arc<Mutex<Option<handoff::Pending>>>,http:reqwest::Client,coordinator:Arc<Mutex<lifecycle::Coordinator>>,desktop:Session,candidate:Arc<Prepared>,config:Value,path:std::path::PathBuf,until:Instant,key:String)->Result<Ready,String>{
 let scope=candidate.scope().clone();let (bundle,mut record)={let(store,writer,pending,scope,key)=(store.clone(),writer.clone(),pending.clone(),scope.clone(),key.clone());tokio::task::spawn_blocking(move||{
  let _guard=writer.lock().unwrap();if !current(&pending,&key){return Err("Staging attempt retired before persistence".into())}if path.try_exists().map_err(|_|"Consumed original journal unavailable")?{return Err("Already locally consumed; pending recovery/outcome required".into())}
  let adapter=rejoin::ResumeStore(store);let bundle=Prepared::stage_durable(&adapter,scope.clone(),candidate.seed(),candidate.policy(),candidate.generation(),until,config)?;
  let mut r=w::Request{major:Some(1),revision:Some(protocol::REVISION.into()),service_id:Some(scope.service.clone()),service_epoch:Some(scope.service_epoch.clone()),session_epoch:Some(scope.desktop_epoch.clone()),request_id:Some(uuid::Uuid::new_v4().to_string()),..Default::default()};let context=bundle.world().simple().to_string();r.reserve=Some(w::Reserve{preparation_id:Some(scope.preparation.clone()),world_token:Some(context.clone()),context_id:Some(context)});
  if !current(&pending,&key){return Err("Staging attempt retired before reserve original persistence".into())}
  let record=match local::load(&adapter,&scope,bundle.world())?{Some(r)=>r,None=>local::Record{scope:scope.clone(),world:bundle.world(),original_session:scope.desktop.clone(),fingerprint:nt2_desktop_g0::protocol_connection::canonical_fingerprint(&serde_json::to_vec(&r).map_err(|_|"Reserve original encoding")?)?,request:r,intent:None,proof:None,generation:0,retired:false,operation:None,attach:None}};
  local::save(&adapter,&record)?;if !current(&pending,&key){return Err("Staging attempt retired during persistence".into())}Ok::<_,String>((bundle,record))
 }).await.map_err(|_|"Staging persistence worker canceled")??};
 if record.retired{return Err("Startup reservation explicitly retired".into())}
 let request=record.request.clone();let id=request.request_id.as_deref().ok_or("Missing exact reserve request")?;
 let _=id;
 if !current(&pending,&key){return Err("Staging attempt retired before reserve".into())}
 // Resolve the exact durable original through current authentication. The
 // outcome supplies identity only. A separate current status must provide the
 // proof/current issued state and a newly correlated validity sample.
 let mut status=request.clone();let pending_id=if record.ready(){record.intent.as_ref().and_then(|i|i.pending_id.clone())}else{
  let mut query=request.clone();query.request_id=Some(uuid::Uuid::new_v4().to_string());query.reserve=None;query.pending_outcome=Some(record.original());
  let response=super::waiting::rpc(&http,&scope,&desktop,&query).await.map_err(|e|e.code)?;
  if response.credentials.is_some()||response.pending_proof.is_some()||response.waiting_proof.is_some()||response.resume_proof.is_some()||response.canonical_admission.is_some(){return Err("Pending original outcome unexpectedly carried authority".into())}
  let outcome=response.pending_outcome.as_ref().ok_or("Pending exact original outcome absent")?;
  if serde_json::to_value(&outcome.original).ok()!=serde_json::to_value(Some(record.original())).ok(){return Err("Pending outcome original differs".into())}
  match outcome.original_outcome.as_deref(){Some("committed") if outcome.coverage.as_deref()==Some("exact_durable_record")=>Some(outcome.pending_id.clone().ok_or("Pending original committed identity absent")?),Some("unresolved")=>None,_=>return Err("Pending original remains uncertain".into())}
 };
 if let Some(pending_id)=pending_id{status.request_id=Some(uuid::Uuid::new_v4().to_string());status.reserve=None;status.pending_id=Some(pending_id);status.pending_status=Some(w::Empty{});}
 if desktop.id!=record.original_session||desktop.epoch!=scope.desktop_epoch{return Err("Missing original reservation proof after authentication replacement".into())}
 let stamp=coordinator.lock().unwrap().request_stamp(&json!([scope.origin,scope.service,scope.desktop,scope.desktop_epoch,"startup-reserve-status"]).to_string(),status.request_id.as_deref().unwrap())?;
 let response=super::waiting::rpc(&http,&scope,&desktop,&status).await.map_err(|e|e.code)?;let state=response.pending.as_ref().ok_or("Missing pending reservation state")?;
 if state.may_play!=Some(false)||state.state.as_deref()!=Some("pending")||state.preparation_state.as_deref()!=Some("issued")||response.credentials.is_some()||response.canonical_admission.is_some()||response.waiting.is_some()||response.participant.is_some()||response.resume_proof.is_some(){return Err("Pending reservation cannot carry load/Start/play authority".into())}
 let sample=state.sample.as_ref().ok_or("Pending reservation sample absent")?;if sample.channel.as_deref()!=Some("pending-http")||sample.origin_request_id!=status.request_id||sample.credential_id.as_deref()!=Some(&admission::token_id(desktop.access.as_str())?)||sample.sample_key.as_ref().is_none_or(|s|s.len()!=64||!s.bytes().all(|c|c.is_ascii_hexdigit()&&!c.is_ascii_uppercase())){return Err("Pending reservation sample differs from current request/credential".into())}
 let mut deadline=nt2_desktop_g0::connection_time::Deadline::default();let preparation=deadline.install(stamp,state.preparation_remaining_ms.ok_or("Pending preparation duration absent")?,600000,600000,Instant::now())?;let mut review=nt2_desktop_g0::connection_time::Deadline::default();let review=review.install(stamp,state.review_remaining_ms.ok_or("Pending review duration absent")?,3600000,3600000,Instant::now())?;let until=until.min(preparation).min(review);if until<=Instant::now(){return Err("Pending reservation expired during staging".into())}
 let intent=state.intent.as_ref().ok_or("Pending intent absent")?;let frozen:wire::Config=serde_json::from_value(serde_json::from_slice::<Value>(&bundle.snapshot()?).map_err(|_|"Complete startup snapshot shape")?["config"].clone()).map_err(|_|"Frozen startup configuration shape")?;let policy=seed_policy::resolved(&frozen)?;
 if intent.seed_policy.as_deref()!=Some(&policy.name)||intent.randomize_loot!=Some(policy.loot)||(bundle.policy()==2&&intent.actual_seed.is_some())||(bundle.policy()!=2&&intent.actual_seed.as_deref()!=Some(bundle.seed().to_string().as_str())){return Err("Pending seed/loot differs from immutable frozen configuration".into())}
 let validity:nt2_desktop_g0::protocol_connection::wire::ConnectionCredential=serde_json::from_value(serde_json::to_value(response.credential_validity.as_ref().ok_or("Pending current credential duration absent")?).map_err(|_|"Pending credential encoding")?).map_err(|_|"Pending credential shape")?;
 let mut credentials=std::collections::BTreeMap::new();let credential_until=admission::credential_sample(&mut credentials,&scope.origin,&scope.service,&desktop,&validity,status.request_id.as_deref().unwrap(),stamp)?;let until=until.min(credential_until);if until<=Instant::now(){return Err("Pending credential expired during staging".into())}
 record.intent=state.intent.clone();record.generation=state.generation.filter(|g|*g>0).ok_or("Pending generation absent")?;record.proof=response.pending_proof.or(record.proof.take());record.validate()?;if !record.ready(){return Err("Pending proof not durably ready; original retained".into())}
 let scope=scope.clone();let ready=tokio::task::spawn_blocking(move||{let _guard=writer.lock().unwrap();if !current(&pending,&key){return Err("Staging attempt retired before proof commit".into())}local::save(&rejoin::ResumeStore(store),&record)?;if !current(&pending,&key){return Err("Staging attempt retired during proof commit".into())}let bundle=bundle.with_deadline(until);let _=scope;Ok::<_,String>(Ready{bundle:Arc::new(bundle),key})}).await.map_err(|_|"Staging proof worker canceled")??;Ok(ready)
}
