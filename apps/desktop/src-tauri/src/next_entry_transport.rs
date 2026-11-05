//! Provisional entry transport. HTTP and protected I/O run outside Core/lifecycle locks.
use super::*;
use nt2_desktop_g0::{provisional_local::{self as local,Worker,Phase,Record},startup_local as startup,waiting_local as journal,protocol_waiting::{self as protocol,wire as w}};
#[derive(Default)]pub(super) struct Transport{
 pub(super) selected:bool,pub(super) holding:bool,context:String,task:Option<tokio::task::JoinHandle<Result<Completed,String>>>,
 next:Option<Instant>,preview_next:Option<Instant>,pub(super) error:Option<String>,fence:Arc<AtomicU64>,promoted:Option<uuid::Uuid>,offline_world:Option<uuid::Uuid>,selector:String,
 pub(super) lineage_epoch:u64,
}
#[cfg(test)]#[path="next_entry_transport_test.rs"]mod tests;
impl Drop for Transport{fn drop(&mut self){self.fence.fetch_add(1,Ordering::AcqRel);if let Some(t)=&self.task{t.abort()}}}
impl Transport{pub(super) fn renewal_fence(&mut self){self.fence.fetch_add(1,Ordering::AcqRel);if let Some(t)=self.task.take(){t.abort()}self.next=None;}}
impl Transport{pub(super) fn reset_preview_clock(&mut self){self.preview_next=None;}}
struct Completed{world:uuid::Uuid,marker:Option<startup::WaitingMarker>,contact:Option<crate::pipe::rejoin::Contact>}
struct SensitiveRequest(w::Request);
impl std::ops::Deref for SensitiveRequest{type Target=w::Request;fn deref(&self)->&w::Request{&self.0}}
impl std::ops::DerefMut for SensitiveRequest{fn deref_mut(&mut self)->&mut w::Request{&mut self.0}}
impl Drop for SensitiveRequest{fn drop(&mut self){if let Some(s)=self.0.entry_activate.as_mut().and_then(|p|p.recovery_proof.as_mut()){s.zeroize()}if let Some(s)=self.0.entry_cancel.as_mut().and_then(|p|p.recovery_proof.as_mut()){s.zeroize()}}}
fn check(f:&AtomicU64,n:u64)->Result<(),String>{if f.load(Ordering::Acquire)!=n{Err("Entry authentication/contact retired; exact original retained".into())}else{Ok(())}}
fn envelope(scope:&startup::Scope,d:&Session)->w::Request{w::Request{major:Some(1),revision:Some(protocol::REVISION.into()),service_id:Some(scope.service.clone()),service_epoch:Some(scope.service_epoch.clone()),session_epoch:Some(d.epoch.clone()),request_id:Some(uuid::Uuid::new_v4().to_string()),..Default::default()}}
pub(super) async fn settle(worker:&Worker)->Result<local::View,String>{let until=Instant::now()+Duration::from_secs(3);loop{let(busy,view,error)=worker.view();if !busy{if let Some(e)=error{return Err(e)}return view.ok_or("Entry worker has no original".into())}if Instant::now()>=until{return Err("Entry protected write remains uncertain".into())}tokio::time::sleep(Duration::from_millis(2)).await;}}
async fn read<S:CredentialStore+'static>(store:Arc<S>,writer:Arc<Mutex<()>>,scope:startup::Scope,world:Option<uuid::Uuid>)->Result<Option<Record>,String>{
 tokio::task::spawn_blocking(move||{let _guard=writer.lock().unwrap();let adapter=rejoin::ResumeStore(store);if let Some(world)=world{local::load(&adapter,&scope,world)}else{local::current(&adapter,&scope)}}).await.map_err(|_|"Entry original read canceled")?
}
fn sanitized(r:&w::Response)->Result<(),String>{if r.credentials.is_some()||r.pending_proof.is_some()||r.waiting_proof.is_some()||r.resume_proof.is_some(){Err("Sanitized entry lookup unexpectedly returned authority".into())}else{Ok(())}}
fn validate_outcome(r:&w::Response,o:&w::EntryOriginal)->Result<(),String>{sanitized(r)?;if r.canonical_admission.is_some()||r.participant.is_some()||r.entry.is_some()||r.pending.is_some()||r.waiting.is_some(){return Err("Entry outcome was not sanitized".into())}let actual=r.entry_outcome.as_ref().ok_or("Entry exact original outcome absent")?;
 if serde_json::to_value(&actual.original).ok()!=serde_json::to_value(Some(o)).ok()||!matches!(actual.original_outcome.as_deref(),Some("committed"|"unresolved"))||actual.original_outcome.as_deref()==Some("committed")&&actual.coverage.as_deref()!=Some("exact_durable_record"){return Err("Entry original outcome differs".into())}Ok(())
}
fn observation(record:&Record,report:&native_report::Latest)->Result<w::GameObservation,String>{
 if report.observed_at.elapsed()>=Duration::from_secs(6)||!report.observation.complete||!report.observation.loaded||!report.observation.protected{return Err("Entry needs current complete protected loaded inspection".into())}
 Ok(w::GameObservation{attachment_id:None,game_instance_id:Some(record.game_instance().to_string()),world_token:Some(record.world().simple().to_string()),observation_revision:Some(report.sequence.try_into().map_err(|_|"Entry observation revision bound")?),actual_seed:report.observation.actual_seed.clone(),active_mods:Some(report.observation.mods.clone()),inventory_complete:Some(true),beta:Some(match report.observation.beta{Some(true)=>"true",Some(false)=>"false",None=>"unknown"}.into()),protected:Some(true),loaded:Some(true),seed_status:Some("known".into()),collector_failure:None,mod_version:Some(report.observation.mod_version.clone()),bridge_version:Some(report.observation.bridge_version.clone()),compatibility_provenance:Some("game-thread-client-observation".into())})
}
fn validate_activation_samples(r:&w::Response,request:&w::Request,desktop:&Session,scope:&startup::Scope,stamp:nt2_desktop_g0::connection_time::Stamp)->Result<(),String>{
 for sample in [r.pending.as_ref().and_then(|p|p.sample.as_ref()),r.waiting.as_ref().and_then(|p|p.sample.as_ref())].into_iter().flatten(){if sample.origin_request_id!=request.request_id||sample.credential_id.as_deref()!=Some(admission::token_id(desktop.access.as_str())?.as_str())||sample.sample_key.as_ref().is_none_or(|v|v.len()!=64)||!matches!(sample.channel.as_deref(),Some("pending-http"|"waiting-http")){return Err("Entry activation desktop sample differs".into())}}
 if let Some(credentials)=&r.credentials{
  let game=Session::read(serde_json::from_value(serde_json::to_value(credentials).map_err(|_|"Entry credentials conversion")?).map_err(|_|"Entry credentials shape")?,&scope.device.to_string(),"game")?;
  let owner=r.waiting.as_ref().and_then(|w|w.owner.as_ref()).ok_or("Entry current credential owner absent")?;
  if game.account!=scope.account||owner.desktop_id.as_deref()!=Some(&desktop.id)||owner.desktop_session_epoch.as_deref()!=Some(&desktop.epoch)||owner.game_session_id.as_deref()!=Some(&game.id)||owner.game_session_epoch.as_deref()!=Some(&game.epoch)||owner.attachment_id.as_deref()!=Some(&game.attachment)||owner.game_instance_id!=request.entry_activate.as_ref().and_then(|a|a.observation.as_ref()).and_then(|o|o.game_instance_id.clone()){return Err("Entry returned game owner differs".into())}
  let validity=serde_json::from_value(serde_json::to_value(r.credential_validity.as_ref().ok_or("Entry game validity absent")?).map_err(|_|"Entry validity conversion")?).map_err(|_|"Entry validity shape")?;
  admission::credential_sample(&mut Default::default(),&scope.origin,&scope.service,&game,&validity,request.request_id.as_deref().unwrap(),stamp)?;
 }Ok(())
}
// The activation credentials/admission are deliberately not installed. Fresh
// common recovery rechecks current ownership, inspection and credential leases.
fn discard_authority(r:&mut w::Response){if let Some(c)=r.credentials.as_mut(){if let Some(s)=&mut c.access_token{s.zeroize()}if let Some(s)=&mut c.refresh_secret{s.zeroize()}}if let Some(s)=&mut r.resume_proof{s.zeroize()}r.credentials=None;r.resume_proof=None;r.canonical_admission=None;r.participant=None;}

async fn step<S:CredentialStore+'static>(store:Arc<S>,writer:Arc<Mutex<()>>,http:reqwest::Client,desktop:Session,worker:Arc<Worker>,pending:Arc<Mutex<Option<handoff::Pending>>>,view:local::View,contact:Option<crate::pipe::rejoin::Contact>,report:Option<native_report::Latest>,game_version:String,capabilities:Vec<String>,fence:Arc<AtomicU64>,expected:u64,stamp:nt2_desktop_g0::connection_time::Stamp)->Result<Completed,String>{
 let transport=view.scope.clone();let mut record=read(store.clone(),writer.clone(),view.scope.clone(),Some(view.world)).await?.ok_or("Entry original absent")?;check(&fence,expected)?;
 let detached=matches!(view.phase,Phase::Detached|Phase::CancelSubmitted|Phase::Cancelled);
 if detached&&record.activation_original().is_none(){return Ok(Completed{world:view.world,marker:None,contact:None})}
 let actual=contact.as_ref().map(|c|c.actual.to_string()).or_else(||record.canonical().map(|c|c.marker.actual_seed.clone())).or_else(||detached.then(||"0".into())).ok_or("Loaded entry actual seed absent")?;
 if !detached&&record.canonical().is_some(){return promote(store,writer,record,contact,fence,expected).await}
 // Exact outcome first on every uncertain original. Unresolved is never taken
 // as permission to allocate another B; only the same sealed identity can retry.
 if let Some(original)=if detached{record.cancellation_metadata().or_else(||record.activation_metadata())}else{record.activation_metadata()}{let mut q=envelope(&transport,&desktop);q.entry_outcome=Some(original.clone());check(&fence,expected)?;let reply=waiting::rpc(&http,&transport,&desktop,&q).await.map_err(|e|e.code)?;validate_outcome(&reply,&original)?;}
 let mut status=envelope(&transport,&desktop);status.entry_status=Some(w::EntryLookup{scope:Some(record.entry_scope()?),identity:Some(record.entry_identity())});check(&fence,expected)?;
 let current=waiting::rpc(&http,&transport,&desktop,&status).await.map_err(|e|e.code)?;sanitized(&current)?;check(&fence,expected)?;
 let state=current.entry.as_ref().and_then(|e|e.state.as_deref()).ok_or("Entry disposition absent")?;
 if !detached&&state=="activated"{worker.confirm_current(current,actual.clone(),status,false)?;settle(&worker).await?;let record=read(store.clone(),writer.clone(),view.scope.clone(),Some(view.world)).await?.ok_or("Confirmed entry absent")?;return promote(store,writer,record,contact,fence,expected).await}
 if !detached&&state=="cancelled"{return Err("Entry B was canonically cancelled; world remains held".into())}
 let existing=record.operation_for_service(detached,&desktop.id,&desktop.epoch,&transport.service_epoch);
 let mut request=SensitiveRequest(if let Some(bytes)=existing{serde_json::from_slice::<w::Request>(&bytes).map_err(|_|"Entry sealed operation corrupt")?}else{
  let mut q=envelope(&transport,&desktop);
  if detached{q.entry_cancel=Some(w::EntryCancel{scope:Some(record.entry_scope()?),identity:Some(record.entry_identity()),recovery_proof:Some(record.future_secret().as_str().into()),world_offline:Some(true)})}
  else if let Some(bytes)=record.activation_original(){let original:w::Request=serde_json::from_slice(&bytes).map_err(|_|"Entry original activation corrupt")?;q.entry_activate=original.entry_activate;}
  else{let report=report.as_ref().ok_or("Entry actual protected report absent")?;q.entry_activate=Some(w::EntryActivate{scope:Some(record.entry_scope()?),identity:Some(record.entry_identity()),recovery_proof:Some(record.future_secret().as_str().into()),game_version:Some(game_version),mod_version:Some(report.observation.mod_version.clone()),bridge_version:Some(report.observation.bridge_version.clone()),capabilities:Some(capabilities),observation:Some(observation(&record,report)?)})}
  let bytes=serde_json::to_vec(&q).map_err(|_|"Entry original encoding")?;if detached{worker.seal_cancel(view.world,bytes,desktop.id.clone())?}else{worker.seal_activation(view.world,bytes,desktop.id.clone())?};settle(&worker).await?;
  record=read(store.clone(),writer.clone(),view.scope.clone(),Some(view.world)).await?.ok_or("Entry sealed original absent")?;let sealed=record.operation_for_service(detached,&desktop.id,&desktop.epoch,&transport.service_epoch).ok_or("Entry original not durably sealed")?;serde_json::from_slice(&sealed).map_err(|_|"Entry sealed original corrupt")?
 });
 if detached&&state=="cancelled"{worker.confirm_current(current,actual,status,true)?;settle(&worker).await?;return Ok(Completed{world:view.world,marker:None,contact:None})}
 check(&fence,expected)?;let latest=worker.view().1.ok_or("Entry local owner absent")?;if !detached&&matches!(latest.phase,Phase::Detached|Phase::CancelSubmitted|Phase::Cancelled){return Err("Entry detached before activation send; cancellation pending".into())}
 if !detached&&!pending.lock().unwrap().as_ref().is_some_and(|p|contact.as_ref().is_some_and(|c|p.owner==Some((c.pid,c.context)))&&p.loaded&&p.accepted&&p.last_seen.is_some_and(|at|at.elapsed()<Duration::from_secs(12))&&p.generation.as_ref().is_some_and(|(g,n)|g.load(Ordering::Acquire)==*n)){return Err("Entry native contact closed before send; original retained".into())}
 let mut response=waiting::rpc(&http,&transport,&desktop,&request).await.map_err(|e|e.code)?;check(&fence,expected)?;
 if !detached{validate_activation_samples(&response,&request,&desktop,record.scope(),stamp)?}discard_authority(&mut response);
 worker.confirm_current(response,actual,request.clone(),detached)?;if let Some(p)=request.entry_activate.as_mut().and_then(|p|p.recovery_proof.as_mut()){p.zeroize()}if let Some(p)=request.entry_cancel.as_mut().and_then(|p|p.recovery_proof.as_mut()){p.zeroize()}settle(&worker).await?;
 if detached{return Ok(Completed{world:view.world,marker:None,contact:None})}
 let record=read(store.clone(),writer.clone(),view.scope,Some(view.world)).await?.ok_or("Activated entry original absent")?;promote(store,writer,record,contact,fence,expected).await
}
async fn promote<S:CredentialStore+'static>(store:Arc<S>,writer:Arc<Mutex<()>>,record:Record,contact:Option<crate::pipe::rejoin::Contact>,fence:Arc<AtomicU64>,expected:u64)->Result<Completed,String>{
 tokio::task::spawn_blocking(move||{let _guard=writer.lock().unwrap();check(&fence,expected)?;if record.phase()!=Phase::ActivationReconciled{return Err("Detached entry cannot promote; retirement pending".into())}let c=record.canonical().ok_or("Entry canonical receipt absent")?;let adapter=rejoin::ResumeStore(store);let marker=startup::promote_provisional(&adapter,&record)?;
  if journal::load_saved_selector(&adapter,&c.scope,&marker)?.is_none(){let mut j=journal::Journal::new(marker.clone());j.intent=c.waiting.clone();j.proof=c.waiting_proof.clone();j.generation=c.waiting_generation;j.current_attachment=c.attachment.clone();journal::save(&adapter,&c.scope,&j)?;}
  check(&fence,expected)?;Ok(Completed{world:record.world(),marker:Some(marker),contact})}).await.map_err(|_|"Entry promotion worker canceled")?
}

impl<S:CredentialStore> Core<S>{
 pub(super) fn offline_provisional(&mut self)->Result<bool,String>{let bridge=self.native_diagnostics.lock().unwrap().provisional.clone();let Some(bridge)=bridge else{return Ok(false)};let Some(view)=bridge.worker.view().1 else{return Ok(false)};if self.entry.promoted==Some(view.world)||!self.entry.holding{return Ok(false)}bridge.detach_local()?;self.entry.error=Some("Local B detached; exact server retirement is reconciled separately".into());self.entry.next=None;Ok(true)}
 pub(super) fn entry_view(&self)->Value{let view=self.provisional.as_ref().and_then(|w|w.view().1);json!({"state":view.as_ref().map(|v|format!("{:?}",v.phase)),"error":self.entry.error,"retirement_pending":view.as_ref().is_some_and(|v|v.retirement_pending)})}
 pub(super) async fn entry_tick(&mut self)->Result<bool,String> where S:'static{
  self.sync_replacement_lineage();
  self.entry.holding=false;
  if !self.automatic_setup_enabled(){return Ok(false)}let Some(desktop)=self.desktop.clone()else{return Ok(false)};
  let context=self.preview_context();if self.entry.context!=context{self.entry.fence.fetch_add(1,Ordering::AcqRel);if let Some(task)=self.entry.task.take(){task.abort()}self.entry.context=context.clone();self.entry.next=None;self.entry.preview_next=None;
   let existing=self.provisional.as_ref().and_then(|w|w.view().1);if existing.as_ref().is_some_and(|v|v.scope.origin!=self.base||v.scope.service!=self.service||v.scope.account!=desktop.account||v.scope.device!=self.device){self.provisional=None;self.native_diagnostics.lock().unwrap().provisional=None;self.entry.promoted=None;self.entry.selected=false;}
   if self.provisional.is_none(){
    let selection=self.replacement_review_selection();let record=read(self.store.clone(),self.credential_writer.clone(),self.entry_lookup_scope(&desktop),None).await?;
    if context!=self.preview_context()||selection!=self.replacement_review_selection(){self.entry.context.clear();return Err("Entry source lookup retired with its setup selection; originals retained".into())}
    // Retention is independent of fresh-source eligibility. A resolved older
    // head stays protected, and explicit saved-marker recovery below can load
    // it. Uncertain activation/cancellation work is never filtered out here.
    if let Some(record)=record.filter(|r|r.phase()!=Phase::Staged&&r.phase()!=Phase::Cancelled&&!(r.phase()==Phase::ActivationReconciled&&self.fresh_setup_supersedes_entry(r.scope()))){let worker=Arc::new(Worker::coherent(Arc::new(rejoin::ResumeStore(self.store.clone())),self.credential_writer.clone(),self.replacement_lineage.clone()));worker.recover_selector(record.scope().clone(),record.world())?;self.native_diagnostics.lock().unwrap().provisional=Some(Arc::new(crate::pipe::entry::Bridge::new(worker.clone())));self.provisional=Some(worker);}
   }
  }
  let selector={let d=self.native_diagnostics.lock().unwrap();json!([d.rejoin.waiting_offered.as_ref().map(|(p,m)|(p,m)),d.rejoin.waiting_context,d.rejoin.offered.as_ref().map(|(p,a)|(p,&a.local_world)),d.rejoin.offered_context]).to_string()};
  if self.entry.task.is_some()&&self.entry.selector!=selector{self.entry.fence.fetch_add(1,Ordering::AcqRel);self.entry.task.take().unwrap().abort();self.entry.next=None;self.entry.error=Some("Saved entry changed; late original retained".into());}
  if self.entry.task.as_ref().is_some_and(|t|t.is_finished()){
   match self.entry.task.take().unwrap().await{Ok(Ok(done))=>{
    if let (Some(marker),Some(contact))=(done.marker,done.contact){let provisional=self.native_diagnostics.lock().unwrap().provisional.clone();let valid=self.provisional.as_ref().and_then(|w|w.view().1).is_some_and(|v|v.world==done.world&&v.phase==Phase::ActivationReconciled)&&provisional.as_ref().is_some_and(|b|b.pending.lock().unwrap().as_ref().is_some_and(|p|p.owner==Some((contact.pid,contact.context))&&p.last_seen.is_some_and(|at|at.elapsed()<Duration::from_secs(12))));
     if valid{self.replacement_lineage.resolve(done.world);self.cancel_health();self.park_setup_for_entry();let mut d=self.native_diagnostics.lock().unwrap();d.rejoin.metadata=Some((contact.pid,contact.context,marker.encode()?));d.rejoin.waiting_offered=Some((contact.pid,marker));d.rejoin.waiting_context=Some(contact.context);d.rejoin.contact=Some(contact.clone());d.rejoin.admitted=Some(contact);d.rejoin.offline=false;d.rejoin.terminal=false;self.entry.promoted=Some(done.world);}}
    self.entry.error=None;},Ok(Err(e))=>self.entry.error=Some(e),Err(_)=>self.entry.error=Some("Entry worker interrupted; original retained".into())}
   self.entry.next=Some(Instant::now()+Duration::from_secs(2));
  }
  let offered={let d=self.native_diagnostics.lock().unwrap();d.rejoin.waiting_offered.clone().filter(|(_,m)|m.kind=="nt2-provisional"&&self.entry.offline_world!=Some(m.local_world))};
  if let Some((_,marker))=&offered{if self.provisional.as_ref().and_then(|w|w.view().1).is_none_or(|v|v.world!=marker.local_world){
   if self.provisional.as_ref().is_some_and(|w|w.view().0){self.entry.holding=true;return Ok(true)}
   if self.entry.task.is_none(){let scope=self.entry_lookup_scope(&desktop);let record=read(self.store.clone(),self.credential_writer.clone(),scope,Some(marker.local_world)).await?.ok_or("Saved provisional journal absent")?;let receipt=record.canonical();if record.digest()!=marker.bundle_digest||receipt.as_ref().is_some_and(|c|c.marker.actual_seed!=marker.actual_seed)||record.scope().service_epoch!=self.epoch{return Err("Saved provisional selector differs from protected original".into())}
    let worker=Arc::new(Worker::coherent(Arc::new(rejoin::ResumeStore(self.store.clone())),self.credential_writer.clone(),self.replacement_lineage.clone()));worker.recover_selector(record.scope().clone(),record.world())?;self.native_diagnostics.lock().unwrap().provisional=Some(Arc::new(crate::pipe::entry::Bridge::new(worker.clone())));self.provisional=Some(worker);
   }self.entry.holding=true;return Ok(true)
  }}
  let bridge=self.native_diagnostics.lock().unwrap().provisional.clone();let view=self.provisional.as_ref().and_then(|w|w.view().1);
  if let (Some(bridge),Some(view))=(bridge,view){
   if let Some((_,marker))=&offered{let actual=marker.actual_seed.parse::<u64>().map_err(|_|"Saved provisional seed invalid")?;let loot=if view.policy==2{"individual:0".into()}else{format!("shared:{}:{actual}",u8::from(view.policy==5))};if marker.bundle_digest!=view.digest||marker.loot!=loot||view.policy!=2&&actual!=u64::from(view.seed){return Err("Saved provisional seed/loot binding differs".into())}self.native_diagnostics.lock().unwrap().rejoin.lookup=if matches!(view.phase,Phase::Detached|Phase::CancelSubmitted|Phase::Cancelled){5}else{2};}
   let restored_contact=self.native_diagnostics.lock().unwrap().rejoin.contact.clone();
   if let Some((pid,marker))=&offered{if let Some(contact)=restored_contact.filter(|c|c.pid==*pid&&c.association==*marker.local_world.as_bytes()&&c.actual.to_string()==marker.actual_seed){bridge.install_restored(&view,&contact);self.native_diagnostics.lock().unwrap().rejoin.admitted=Some(contact);}}
   let local=bridge.pending.lock().unwrap().clone();let contact=local.as_ref().and_then(|p|Some(crate::pipe::rejoin::Contact{pid:p.owner?.0,context:p.owner?.1,association:*view.world.as_bytes(),actual:p.actual_seed?}));let report=local.as_ref().and_then(|p|p.observation.clone()).filter(|r|r.observation.complete&&r.observation.loaded&&r.observation.protected&&r.observed_at.elapsed()<Duration::from_secs(6));
   let detached=matches!(view.phase,Phase::Detached|Phase::CancelSubmitted|Phase::Cancelled);let active=matches!(view.phase,Phase::Claimed|Phase::ActivationSubmitted|Phase::ActivationReconciled)&&self.entry.promoted!=Some(view.world)&&(local.as_ref().is_some_and(|p|p.last_seen.is_some_and(|at|at.elapsed()<Duration::from_secs(12)))||offered.is_some());
   if active{self.entry.holding=true;self.cancel_health();self.park_setup_for_entry();}
   if self.entry.promoted==Some(view.world)&&self.game.as_ref().is_some_and(|g|contact.as_ref().is_some_and(|c|g.owner!=(c.pid,c.context))){self.entry.holding=true;}
   if !self.provisional.as_ref().is_some_and(|w|w.view().0)&&self.entry.task.is_none()&&self.entry.next.is_none_or(|at|Instant::now()>=at)&&((active&&(report.is_some()||view.phase==Phase::ActivationReconciled))||detached&&view.retirement_pending){
    let worker=self.provisional.clone().unwrap();let scope=view.scope.clone();if scope.origin!=self.base||scope.service!=self.service||(!detached&&scope.service_epoch!=self.epoch)||scope.account!=desktop.account||scope.device!=self.device{return Err("Entry source belongs to another issuer/account/device".into())}
    let stamp={let mut c=self.lifecycle.lock().unwrap();c.request_sequence+=1;nt2_desktop_g0::connection_time::Stamp{sequence:c.request_sequence,started:Instant::now()}};
    let version=self.info.as_ref().and_then(|i|i.compatibility.as_ref()).and_then(|c|c.game_version.clone()).ok_or("Entry game compatibility absent")?;let expected=self.entry.fence.load(Ordering::Acquire);let mut transport_view=view.clone();if detached{transport_view.scope.service_epoch=self.epoch.clone();}
    self.entry.selector=selector.clone();self.entry.task=Some(tokio::spawn(step(self.store.clone(),self.credential_writer.clone(),self.http.clone(),desktop.clone(),worker,bridge.pending.clone(),transport_view,contact,report,version,self.local_game_capabilities(),self.entry.fence.clone(),expected,stamp)));
   }
   if active{return Ok(true)}if detached{self.entry.offline_world=Some(view.world);if !view.retirement_pending{self.replacement_lineage.resolve(view.world);self.provisional=None;self.native_diagnostics.lock().unwrap().provisional=None;}else{return Ok(false)}}
  }
  // Automatic passive configuration is refreshed independently of A health.
  // A first-time unconsumed canonical setup keeps its accepted startup lane.
  let run=self.room.as_ref().and_then(|r|r.run_id.as_deref());let known_a=self.entry.selected||self.pending.lock().unwrap().as_ref().is_some_and(|p|p.owner.is_some()&&Some(p.run.as_str())==run)||self.resume_intent.association().is_some_and(|a|Some(a.run.as_str())==run);
  if known_a&&self.entry.promoted.is_none_or(|world|self.waiting_local_world()==Some(world)&&self.game.is_some())&&self.entry.preview_next.is_none_or(|at|Instant::now()>=at){
   self.entry.preview_next=Some(Instant::now()+Duration::from_secs(10));
   if let Err(e)=self.refresh_passive_entry().await{self.entry.error=Some(e)}
  }Ok(false)
 }
 pub(super) async fn refresh_passive_entry(&mut self)->Result<(),String> where S:'static{
  let promoted_ready=|core:&Self|core.provisional.as_ref().and_then(|w|w.view().1).is_some_and(|v|core.entry.promoted==Some(v.world)&&v.phase==Phase::ActivationReconciled&&core.waiting_local_world()==Some(v.world)&&core.game.is_some());
  if !self.unused_replacement_slot()&&!promoted_ready(self){return Ok(())}
  if promoted_ready(self){self.replacement_lineage.resolve(self.provisional.as_ref().unwrap().view().1.unwrap().world);}
  if self.sync_replacement_lineage().is_none(){return Ok(())}
  if self.replacement_review_target().as_ref().and_then(|p|p.state.as_deref())!=Some("staging"){return Ok(())}
  let review=self.read_replacement_review().await?;
  // Native offer/claim may begin while the read is in flight. That original
  // wins; do not publish a new review or replace/rebase its protected record.
  if !self.unused_replacement_slot()&&!promoted_ready(self){return Err("Fresh entry began during automatic review; original retained".into())}
  if promoted_ready(self){self.provisional=None;self.native_diagnostics.lock().unwrap().provisional=None;}
  self.setup.preview=Some(review);self.prepare_replacement_preview().await?;self.entry.error=None;Ok(())
 }
 fn entry_lookup_scope(&self,d:&Session)->startup::Scope{startup::Scope{origin:self.base.clone(),service:self.service.clone(),service_epoch:self.epoch.clone(),account:d.account.clone(),device:self.device,desktop:d.id.clone(),desktop_epoch:d.epoch.clone(),room:String::new(),run:String::new(),run_epoch:String::new(),config:0,config_digest:String::new(),preparation:String::new()}}
}
