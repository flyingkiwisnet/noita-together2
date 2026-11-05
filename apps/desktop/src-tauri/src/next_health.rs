//! Typed independent connection worker. No mutable Core or optional UI state is
//! cloned into this lane. Coordinator commits are scoped and contain no I/O.
use super::*;
use nt2_desktop_g0::{protocol_connection::{self as connection,wire as c},connection_time::Stamp};
use nt2_desktop_g0::protocol_rejoin::wire as resume;
#[cfg(test)]#[path="next_health_owner_test.rs"]mod owner_tests;
#[derive(Clone)]struct Original {id:String,body:zeroize::Zeroizing<Vec<u8>>,stamp:Stamp,endpoint:&'static str,session:Session,anonymous:bool}
#[derive(Clone)]pub(super) struct Active {
 ctx:admission::Context,ticket:lifecycle::Ticket,nonce:u64,generation:u64,key:String,device:uuid::Uuid,profile:Option<ServiceProfile>,remember:bool,
 reported:u64,report:Option<Original>,report_source:Option<native_report::Latest>,acknowledged:Option<native_report::Latest>,renew:Option<Original>,refresh:Option<Original>,refresh_commit:Option<RefreshCommit>,refresh_storage_error:Option<String>,refresh_retry_at:Option<Instant>,capture_done:bool,force_renew:bool,force_refresh:bool,not_before:Option<Instant>,
}
pub(super) struct Checkpoint {nonce:u64,generation:u64,key:String,ticket:lifecycle::Ticket,revision:u64,output:Option<Output>}
#[derive(Clone)]struct RefreshCommit {session:Session,profile:Option<ServiceProfile>,remember:bool}
struct RefreshReady {owner:Active,session:Session,persist_error:Option<String>}
struct WaitingReady {owner:Active,marker:Result<nt2_desktop_g0::startup_local::WaitingMarker,String>}
struct Output {desktop:Session,game:Session,reported:u64,entered:bool,offered:bool,error:Option<String>,capture:Option<CaptureReply>}
struct CaptureReply {ticket:lifecycle::Ticket,response:resume::Response}
impl Drop for CaptureReply{fn drop(&mut self){if let Some(proof)=self.response.resume_proof.as_mut(){proof.zeroize();}}}
impl Clone for CaptureReply{fn clone(&self)->Self{Self{ticket:self.ticket.clone(),response:self.response.clone()}}}
pub struct HealthRunner<S:CredentialStore>{
 lineage:Arc<nt2_desktop_g0::provisional_local::Lineage>,
 store:Arc<S>,writer:Arc<Mutex<()>>,http:reqwest::Client,
 coordinator:Arc<Mutex<lifecycle::Coordinator>>,updates:Arc<Mutex<Option<Checkpoint>>>,nonce:Arc<AtomicU64>,generation:Arc<AtomicU64>,pending:Arc<Mutex<Option<handoff::Pending>>>,
 publisher:Option<nt2_desktop_g0::g3_runtime::HealthPublisher>,social:nt2_desktop_g0::g3_social::CredentialFeed,next:Instant,revision:u64,
 refresh:Option<tokio::task::JoinHandle<Result<RefreshReady,String>>>,presence:Option<tokio::task::JoinHandle<Result<(),String>>>,capture:Option<tokio::task::JoinHandle<Result<CaptureReply,String>>>,capture_ready:Option<CaptureReply>,
 waiting:Option<tokio::task::JoinHandle<WaitingReady>>,waiting_retry:Instant,waiting_error:Option<String>,waiting_scope:Option<(lifecycle::Ticket,u64,u64)>,
}
fn seq(coordinator:&Arc<Mutex<lifecycle::Coordinator>>)->Result<Stamp,String>{let mut c=coordinator.lock().unwrap();c.request_sequence=c.request_sequence.checked_add(1).ok_or("Request sequence exhausted")?;Ok(Stamp{sequence:c.request_sequence,started:Instant::now()})}
fn envelope(ctx:&admission::Context,game:bool)->wire::Request{wire::Request{major:Some(1),revision:Some(protocol::REVISION.into()),service_id:Some(ctx.issuer.service.clone()),service_epoch:Some(ctx.epoch.clone()),session_epoch:Some(if game{ctx.game.epoch.clone()}else{ctx.desktop.epoch.clone()}),request_id:Some(uuid::Uuid::new_v4().to_string()),room_id:Some(ctx.room.clone()),run_id:Some(ctx.run.clone()),run_epoch:Some(ctx.run_epoch.clone()),config_revision:Some(ctx.config),..Default::default()}}
fn rejoin_envelope(ctx:&admission::Context)->resume::Request{resume::Request{major:Some(1),revision:Some(nt2_desktop_g0::protocol_rejoin::REVISION.into()),service_id:Some(ctx.issuer.service.clone()),service_epoch:Some(ctx.epoch.clone()),session_epoch:Some(ctx.game.epoch.clone()),request_id:Some(uuid::Uuid::new_v4().to_string()),room_id:Some(ctx.room.clone()),run_id:Some(ctx.run.clone()),participant_id:ctx.participant.clone(),world_token:Some(ctx.world.clone()),config_revision:Some(ctx.config),generation:ctx.ownership,..Default::default()}}
fn original<T:Serialize>(request:&T,id:&str,session:&Session,endpoint:&'static str,coordinator:&Arc<Mutex<lifecycle::Coordinator>>)->Result<Original,String>{Ok(Original{id:id.into(),body:zeroize::Zeroizing::new(serde_json::to_vec(request).map_err(|_|"Request encoding failed")?),stamp:seq(coordinator)?,endpoint,session:session.clone(),anonymous:serde_json::to_value(request).ok().is_some_and(|v|v.get("refresh").is_some())})}
async fn call(http:&reqwest::Client,ctx:&admission::Context,job:&Original,current:&Session,until:Instant)->Result<zeroize::Zeroizing<Vec<u8>>,String>{
 if current.id!=job.session.id||current.epoch!=job.session.epoch||current.account!=job.session.account{return Err("Original worker authentication scope changed".into());}
 let body=job.body.to_vec();let url=format!("{}{}",ctx.issuer.origin,job.endpoint);
 tokio::time::timeout_at(tokio::time::Instant::from_std(until),async{
  let mut call=http.post(&url).header("content-type","application/json").body(body);if !job.anonymous{call=call.bearer_auth(current.access.as_str());}let mut response=call.send().await.map_err(|_|"Candidate connection unavailable")?;
  if response.url().as_str()!=url{return Err("Worker redirect refused".into());}if !response.status().is_success(){return Err(if response.status().as_u16()==403{"FORBIDDEN"}else if response.status().as_u16()==401{"UNAUTHENTICATED"}else{"SERVICE_UNAVAILABLE"}.into());}
  let mut bytes=zeroize::Zeroizing::new(Vec::new());while let Some(chunk)=response.chunk().await.map_err(|_|"Candidate response unavailable")?{if bytes.len()+chunk.len()>65536{return Err("Worker response capacity".into());}bytes.extend_from_slice(&chunk);}Ok(bytes)
 }).await.map_err(|_|"Whole health attempt deadline".to_string())?
}
fn verify(value:&Value,ctx:&admission::Context,id:&str)->Result<(),String>{
 if value["serviceId"].as_str()!=Some(&ctx.issuer.service)||value["requestId"].as_str()!=Some(id){return Err("Worker response scope changed".into());}
 if value["serviceEpoch"].as_str()!=Some(&ctx.epoch){return Err("Worker service epoch changed".into());}
 if let Some(error)=value.get("error"){if error["retryAfterMs"].as_u64().is_some_and(|v|v>60000){return Err("Invalid server backoff".into());}return Err(error["code"].as_str().unwrap_or("Worker request refused").into());}Ok(())
}
fn lobby(bytes:&[u8],ctx:&admission::Context,id:&str)->Result<wire::Response,String>{let v:Value=serde_json::from_slice(bytes).map_err(|_|"Invalid worker reply")?;verify(&v,ctx,id)?;protocol::response(bytes).map_err(|_|"Invalid canonical lobby response".into())}
fn convert<T:serde::de::DeserializeOwned,U:Serialize>(v:&U)->Result<T,String>{serde_json::from_value(serde_json::to_value(v).map_err(|_|"Canonical conversion failed")?).map_err(|_|"Canonical conversion failed".into())}
fn movement_binding(ctx:&admission::Context,owner:(crate::local_owner::Id,[u8;16]))->nt2_desktop_g0::g3_local_consumer::Binding{
 let scope=nt2_desktop_g0::protocol_g3::wire::Scope{room_id:Some(ctx.room.clone()),run_id:Some(ctx.run.clone()),run_epoch:Some(ctx.run_epoch.clone()),config_revision:Some(ctx.config),attachment_id:Some(ctx.game.attachment.clone()),world_token:Some(ctx.world.clone())};
 nt2_desktop_g0::g3_local_consumer::Binding{context:owner.1,service_origin:ctx.issuer.origin.clone(),service_id:ctx.issuer.service.clone(),service_epoch:ctx.epoch.clone(),session_epoch:ctx.game.epoch.clone(),principal_id:ctx.game.account.clone(),scope}
}
impl<S:CredentialStore> Core<S>{
 pub(super) fn health_identity(&self)->Option<String>{let g=self.game.as_ref()?;let d=self.desktop.as_ref()?;let r=self.room.as_ref()?;Some(json!([self.base,self.service,self.epoch,d.id,d.epoch,d.account,g.session.id,g.session.epoch,g.session.attachment,g.instance,g.owner,r.run_id,r.run_epoch,r.config_revision]).to_string())}
 pub(super) fn cancel_health(&mut self){self.health_nonce.fetch_add(1,Ordering::AcqRel);*self.health_updates.lock().unwrap()=None;self.lifecycle.lock().unwrap().active=None;}
 pub(super) fn health_active(&self)->bool{self.health_enabled&&self.health_updates.lock().unwrap().as_ref().is_some_and(|p|p.nonce==self.health_nonce.load(Ordering::Acquire))}
 pub(super) fn publish_health(&mut self){
  if !self.health_enabled||self.entry.holding{return;}let Some(key)=self.health_identity()else{return};let Ok(ctx)=self.connection_context()else{return};let Some(ticket)=self.lifecycle_owner_ticket()else{return};
  let mut coordinator=self.lifecycle.lock().unwrap();let nonce=self.health_nonce.load(Ordering::Acquire);let generation=self.generation.load(Ordering::Acquire);
  if coordinator.authority.terminal{return;}
  if let Some(active)=coordinator.active.as_mut().filter(|a|a.ticket==ticket&&a.nonce==nonce){
   active.profile=self.profile.clone();active.remember=self.remember;
   // Root owns world/profile actions; verified token replacements remain owned by
   // the refresh lane. Projection polling cannot overwrite fresher credentials.
   if ctx.recovered{active.capture_done=true;active.ctx.recovered=true;active.ctx.participant=ctx.participant;active.ctx.ownership=ctx.ownership;active.ctx.original_start=ctx.original_start;}return;
  }
  coordinator.active=Some(Active{ctx,ticket:ticket.clone(),nonce,generation,key:key.clone(),device:self.device,profile:self.profile.clone(),remember:self.remember,reported:self.game.as_ref().unwrap().reported,report:None,report_source:None,acknowledged:None,renew:None,refresh:None,refresh_commit:None,refresh_storage_error:None,refresh_retry_at:None,capture_done:self.resume_intent.association().is_some(),force_renew:false,force_refresh:false,not_before:None});
  *self.health_updates.lock().unwrap()=Some(Checkpoint{nonce,generation,key,ticket,revision:0,output:None});self.health_seen=0;
 }
 pub fn sync_health(&mut self){
  let output={let coordinator=self.lifecycle.lock().unwrap();let mut slot=self.health_updates.lock().unwrap();let Some(p)=slot.as_mut()else{return};if p.nonce!=self.health_nonce.load(Ordering::Acquire)||p.generation!=self.generation.load(Ordering::Acquire)||Some(&p.key)!=self.health_identity().as_ref()||!coordinator.accepts(&p.ticket)||p.revision<=self.health_seen{return;}self.health_seen=p.revision;p.output.take()};
  self.publish_waiting_metadata();
  let Some(mut output)=output else{return};let Some(game)=self.game.as_mut()else{return};
  if game.session.id!=output.game.id||game.session.epoch!=output.game.epoch||game.session.account!=output.game.account{return;}
  game.session=output.game.clone();game.reported=output.reported;game.entered|=output.entered;game.offered=output.offered;self.desktop=Some(output.desktop.clone());self.last_service_observation=output.reported;
  if self.lifecycle.lock().unwrap().authority.terminal{
   self.resume_intent.terminal();self.cancel_health();self.g3_pause();self.native_stage="ended";self.status_error=output.error.take();if let Err(error)=self.persist_recovery(){self.status_error=Some(error);}return;
  }
  if output.entered{let local=self.pending.lock().unwrap().clone();if let Some(local)=local{self.complete_verified_setup_entry(&local);}
   if self.resume_intent.phase()==nt2_desktop_g0::rejoin_local::Phase::Inspecting{if self.resume_intent.release(self.resume_intent.generation(),true).is_ok(){if let Err(error)=self.persist_recovery(){self.status_error=Some(error);}}}
  }
  let mut capture_error=None;if let Some(mut capture)=output.capture.take(){if let Err(error)=self.adopt_selected_association(std::mem::take(&mut capture.response)){capture_error=Some(error);}}
  if output.error.as_deref()==Some("Worker service epoch changed"){self.rejoin_transport.rediscover=true;}
  self.native_stage=if output.entered{"playing"}else if output.offered{"host-authorized-awaiting-local-entry"}else{"checking-report"};self.status_error=capture_error.or(output.error.take());self.publish_health();
 }
 pub(super) fn record_health_entry(&mut self){} // Entry belongs to the coordinator's actual native sample.
 pub fn health_runner(&mut self)->HealthRunner<S>{self.health_enabled=true;self.publish_health();HealthRunner{lineage:self.replacement_lineage.clone(),store:self.store.clone(),writer:self.credential_writer.clone(),http:self.http.clone(),coordinator:self.lifecycle.clone(),updates:self.health_updates.clone(),nonce:self.health_nonce.clone(),generation:self.generation.clone(),pending:self.pending.clone(),publisher:self.g3.as_ref().map(|g|g.health_publisher()),social:self.social_credentials.clone(),next:Instant::now(),revision:0,refresh:None,presence:None,capture:None,capture_ready:None,waiting:None,waiting_retry:Instant::now(),waiting_error:None,waiting_scope:None}}
 /// Non-secret candidate egress only. No renderer/native ABI is frozen here.
 pub(super) fn waiting_startup_marker(&self)->Option<nt2_desktop_g0::startup_local::WaitingMarker>{let c=self.lifecycle.lock().unwrap();let a=c.active.as_ref()?;if a.nonce!=self.health_nonce.load(Ordering::Acquire)||a.generation!=self.generation.load(Ordering::Acquire)||!c.accepts(&a.ticket)||c.authority.terminal{return None}c.waiting_metadata.clone()}
 pub(super) fn transient_health_error(error:&str)->bool{matches!(error,"Candidate connection unavailable"|"Candidate HTTP refused"|"Candidate response unavailable"|"CAPACITY"|"RATE_LIMITED"|"SERVICE_UNAVAILABLE"|"Whole health attempt deadline"|"Scoped health server backoff")}
 pub(super) fn protect_health_error(&mut self,error:&str){if !Self::transient_health_error(error){self.g3_pause();if let Some(p)=self.pending.lock().unwrap().as_mut(){p.release=false;p.authority_until=Instant::now();}}}
}
impl<S:CredentialStore+'static> HealthRunner<S>{
 fn health_reply(&self,a:&Active,bytes:&[u8],id:&str)->Result<(),String>{
  let value:Value=serde_json::from_slice(bytes).map_err(|_|"Invalid health reply")?;let result=verify(&value,&a.ctx,id);
  if result.is_err()&&value["serviceId"].as_str()==Some(&a.ctx.issuer.service)&&value["serviceEpoch"].as_str()==Some(&a.ctx.epoch)&&value["requestId"].as_str()==Some(id){if let Some(ms)=value["error"]["retryAfterMs"].as_u64().filter(|v|*v>0&&*v<=60000){let next=Instant::now()+Duration::from_millis(ms);let mut c=self.coordinator.lock().unwrap();if c.accepts(&a.ticket){if let Some(active)=c.active.as_mut(){active.not_before=Some(active.not_before.map(|old|old.max(next)).unwrap_or(next));}}}}
  result
 }
 fn current(&self,a:&Active)->bool{a.nonce==self.nonce.load(Ordering::Acquire)&&a.generation==self.generation.load(Ordering::Acquire)&&self.coordinator.lock().unwrap().accepts(&a.ticket)}
 #[cfg(test)]pub(super) fn renew_for_controlled_clock(&mut self){if let Some(a)=self.coordinator.lock().unwrap().active.as_mut(){a.force_renew=true;}}
 #[cfg(test)]pub(super) fn refresh_for_controlled_store(&mut self){if let Some(a)=self.coordinator.lock().unwrap().active.as_mut(){a.force_refresh=true;}}
 #[cfg(test)]pub(super) fn refresh_pending_for_controlled_store(&self)->bool{self.refresh.is_some()}
 #[cfg(test)]pub(super) fn refresh_commit_pending_for_test(&self)->bool{self.coordinator.lock().unwrap().active.as_ref().is_some_and(|a|a.refresh_commit.is_some())}
 fn sample(&self,a:&Active,s:&Session,v:&c::ConnectionCredential,job:&Original)->Result<Instant,String>{let mut coordinator=self.coordinator.lock().unwrap();if !coordinator.accepts(&a.ticket){return Err("Retired worker".into());}admission::credential_sample(&mut coordinator.credentials,&a.ctx.issuer.origin,&a.ctx.issuer.service,s,v,&job.id,job.stamp)}
 fn expiring(&self,a:&Active,s:&Session)->bool{let Ok(key)=admission::credential_key(&a.ctx.issuer.origin,&a.ctx.issuer.service,s)else{return true};self.coordinator.lock().unwrap().credentials.get(&key).is_none_or(|l|l.deadline.remaining(Instant::now())<=Duration::from_secs(60))}
 async fn work(&self,a:&mut Active,local:&handoff::Pending,until:Instant)->Result<(),String>{
  if a.not_before.is_some_and(|at|Instant::now()<at){return Err("Scoped health server backoff".into());}
  if a.force_renew||a.renew.is_some()||self.expiring(a,&a.ctx.game){
   if a.renew.is_none(){let mut request=envelope(&a.ctx,false);request.game_renew=Some(wire::GameRenew{attachment_id:Some(a.ctx.game.attachment.clone()),game_instance_id:Some(a.ctx.instance.clone()),world_token:Some(a.ctx.world.clone()),game_session_id:Some(a.ctx.game.id.clone()),game_session_epoch:Some(a.ctx.game.epoch.clone())});a.renew=Some(original(&request,request.request_id.as_deref().unwrap(),&a.ctx.desktop,"/v1/rpc",&self.coordinator)?);}
   let job=a.renew.as_ref().unwrap();let mut response=call(&self.http,&a.ctx,job,&a.ctx.desktop,until).await.and_then(|bytes|{lobby(&bytes,&a.ctx,&job.id)?;Ok(bytes)});
   if response.as_ref().err().is_some_and(|e|matches!(e.as_str(),"UNAUTHENTICATED"|"EXPIRED")){
    let current=self.coordinator.lock().unwrap().active.as_ref().filter(|current|current.ticket==a.ticket&&current.nonce==a.nonce&&current.generation==a.generation).map(|current|current.ctx.desktop.clone());
    if let Some(current)=current.filter(|current|current.access.as_str()!=a.ctx.desktop.access.as_str()){a.ctx.desktop=current;response=call(&self.http,&a.ctx,job,&a.ctx.desktop,until).await;}
   }
   let bytes=response?;self.health_reply(a,&bytes,&job.id)?;let mut reply=lobby(&bytes,&a.ctx,&job.id)?;let session=Session::read(reply.credentials.take().ok_or("Missing renewed game")?,&a.device.to_string(),"game")?;
   if session.id!=a.ctx.game.id||session.epoch!=a.ctx.game.epoch||session.account!=a.ctx.game.account||session.attachment!=a.ctx.game.attachment{return Err("Renewed worker game changed".into());}
   let validity:c::ConnectionCredential=convert(reply.credential_validity.as_ref().ok_or("Missing game credential validity")?)?;self.sample(a,&session,&validity,job)?;a.ctx.game=session;a.renew=None;a.force_renew=false;
   self.social.renew(&a.ctx.game.id,&a.ctx.game.epoch,&a.ctx.game.account,&a.ctx.game.attachment,local.owner.ok_or("No native owner")?.1,zeroize::Zeroizing::new(a.ctx.game.access.as_str().to_string()))?;
  }
  if let Some(report)=local.observation.as_ref(){
   if report.context!=local.owner.ok_or("No native owner")?.1{return Err("Native report owner changed".into());}
   let enrollment_holds_same_report=self.waiting.is_some()&&a.acknowledged.as_ref().is_some_and(|ack|ack.digest==report.digest&&ack.context==report.context&&ack.observation.loaded==report.observation.loaded&&ack.observation.complete==report.observation.complete&&ack.observation.protected==report.observation.protected);
   if report.sequence>a.reported&&!enrollment_holds_same_report||a.report.is_some(){
    if a.report.is_none(){a.report_source=Some(report.clone());let v=wire::GameObservation{attachment_id:Some(a.ctx.game.attachment.clone()),game_instance_id:Some(a.ctx.instance.clone()),world_token:Some(a.ctx.world.clone()),observation_revision:Some(report.sequence.try_into().map_err(|_|"Observation sequence exhausted")?),actual_seed:report.observation.actual_seed.clone(),active_mods:Some(report.observation.mods.clone()),inventory_complete:Some(report.observation.complete),beta:Some(match report.observation.beta{Some(true)=>"true",Some(false)=>"false",None=>"unknown"}.into()),protected:Some(report.observation.protected),loaded:Some(report.observation.loaded),seed_status:Some(if report.observation.actual_seed.is_some(){"known"}else{"unknown"}.into()),collector_failure:report.observation.error.clone(),mod_version:Some(report.observation.mod_version.clone()),bridge_version:Some(report.observation.bridge_version.clone()),compatibility_provenance:Some("game-thread-client-observation".into())};
     if a.ctx.recovered{let mut request=rejoin_envelope(&a.ctx);request.observe=Some(convert(&v)?);a.report=Some(original(&request,request.request_id.as_deref().unwrap(),&a.ctx.game,"/v3/rejoin/rpc",&self.coordinator)?);}else{let mut request=envelope(&a.ctx,true);request.game_observation=Some(v);a.report=Some(original(&request,request.request_id.as_deref().unwrap(),&a.ctx.game,"/v1/rpc",&self.coordinator)?);}
    }
    let job=a.report.as_ref().unwrap();let bytes=call(&self.http,&a.ctx,job,&a.ctx.game,until).await?;self.health_reply(a,&bytes,&job.id)?;let value:Value=serde_json::from_slice(&bytes).map_err(|_|"Invalid inspection reply")?;verify(&value,&a.ctx,&job.id)?;
    let submitted:Value=serde_json::from_slice(&job.body).map_err(|_|"Invalid original inspection")?;a.reported=submitted[if a.ctx.recovered{"observe"}else{"gameObservation"}]["observationRevision"].as_u64().ok_or("Missing original report revision")?;a.report=None;a.acknowledged=a.report_source.take();
   }
  }
  if a.reported==0{return Ok(());}
  // Pulse remains the existing reliable authority renewal; the following read
  // supplies the single canonical decision for both initial and resumed worlds.
  if a.ctx.recovered{let mut r=rejoin_envelope(&a.ctx);r.pulse=Some(resume::Empty{});let job=original(&r,r.request_id.as_deref().unwrap(),&a.ctx.game,"/v3/rejoin/rpc",&self.coordinator)?;let bytes=call(&self.http,&a.ctx,&job,&a.ctx.game,until).await?;self.health_reply(a,&bytes,&job.id)?;verify(&serde_json::from_slice(&bytes).map_err(|_|"Invalid pulse")?,&a.ctx,&job.id)?;}else{let mut r=envelope(&a.ctx,true);r.game_pulse=Some(wire::GamePulse{attachment_id:Some(a.ctx.game.attachment.clone()),game_instance_id:Some(a.ctx.instance.clone()),world_token:Some(a.ctx.world.clone())});let job=original(&r,r.request_id.as_deref().unwrap(),&a.ctx.game,"/v1/rpc",&self.coordinator)?;let bytes=call(&self.http,&a.ctx,&job,&a.ctx.game,until).await?;self.health_reply(a,&bytes,&job.id)?;let _=lobby(&bytes,&a.ctx,&job.id)?;}
  let pre_start={let co=self.coordinator.lock().unwrap();!a.ctx.recovered&&!co.authority.offered&&!co.authority.entered};
  if pre_start{
   // A valid staging owner has no canonical admission yet. The scoped game
   // status distinguishes that wait from refusal. Poll again on every cycle;
   // only the normal admission consumer below may grant actual later Start.
   let mut r=envelope(&a.ctx,true);r.room_get=Some(wire::Empty{});let job=original(&r,r.request_id.as_deref().unwrap(),&a.ctx.game,"/v1/rpc",&self.coordinator)?;
   let bytes=call(&self.http,&a.ctx,&job,&a.ctx.game,until).await?;self.health_reply(a,&bytes,&job.id)?;let reply=lobby(&bytes,&a.ctx,&job.id)?;
   if initial_waiting_status(&a.ctx,&reply)?{
    let validity:c::ConnectionCredential=convert(reply.credential_validity.as_ref().ok_or("Missing staging credential sample")?)?;
    let until=self.sample(a,&a.ctx.game,&validity,&job)?;
    let mut p=self.pending.lock().unwrap();let current=p.as_mut().ok_or("Native owner disappeared")?;
    if !self.current(a)||current.owner!=local.owner||current.run!=local.run||current.epoch!=local.epoch||current.config!=local.config||current.generation.as_ref().is_none_or(|(g,n)|g.load(Ordering::Acquire)!=*n)||current.last_seen.is_none_or(|at|at.elapsed()>Duration::from_secs(12)){return Err("Late staging owner retired".into())}
    if until<=Instant::now(){return Err("Staging credential lease expired".into())}
    // Protected collection continuity only, capped by the sampled credential.
    // Native status cannot become playing while release remains false.
    current.release=false;current.authority_until=(Instant::now()+Duration::from_secs(6)).min(until);return Ok(())
   }
  }
  let r=c::Request{major:Some(1),revision:Some(connection::REVISION.into()),service_id:Some(a.ctx.issuer.service.clone()),service_epoch:Some(a.ctx.epoch.clone()),session_epoch:Some(a.ctx.game.epoch.clone()),request_id:Some(uuid::Uuid::new_v4().to_string()),admission:Some(c::AdmissionRead{room_id:Some(a.ctx.room.clone()),participant_id:a.ctx.participant.clone()}),..Default::default()};let job=original(&r,r.request_id.as_deref().unwrap(),&a.ctx.game,connection::ENDPOINT,&self.coordinator)?;let bytes=call(&self.http,&a.ctx,&job,&a.ctx.game,until).await?;self.health_reply(a,&bytes,&job.id)?;let reply=connection::response(&bytes)?;verify(&serde_json::from_slice(&bytes).map_err(|_|"Invalid admission envelope")?,&a.ctx,&job.id)?;
  let credential_until=self.sample(a,&a.ctx.game,reply.credential_validity.as_ref().ok_or("Missing current credential sample")?,&job)?;let admission=reply.admission.ok_or("Missing canonical admission")?;
  let mut pending=self.pending.lock().unwrap();let current=pending.as_mut().ok_or("Native owner disappeared")?;if current.owner!=local.owner||current.run!=local.run||current.epoch!=local.epoch||current.config!=local.config||current.generation.as_ref().is_none_or(|(f,g)|f.load(Ordering::Acquire)!=*g){return Err("Late admission owner retired".into());}
  let mut coordinator=self.coordinator.lock().unwrap();if !coordinator.accepts(&a.ticket){return Err("Late admission ticket retired".into());}
  let credential=admission::token_id(a.ctx.game.access.as_str())?;let result=coordinator.authority.install(admission,&a.ctx,current,&job.id,&credential,job.stamp,credential_until);
  match result{Ok(until)=>{current.release=true;current.authority_until=until;},Err(error)=>{if coordinator.authority.terminal||!coordinator.authority.offered{current.release=false;}return Err(error);}}
  Ok(())
 }
 pub async fn pump(&mut self)->Result<(),String>{
  if Instant::now()<self.next{return Ok(());}self.next=Instant::now()+Duration::from_secs(2);
  let Some(mut a)=self.coordinator.lock().unwrap().active.clone()else{return Ok(())};if !self.current(&a){return Ok(());}
  if self.waiting_scope.as_ref().is_none_or(|(ticket,nonce,generation)|ticket!=&a.ticket||*nonce!=a.nonce||*generation!=a.generation){self.waiting_error=None;self.waiting_retry=Instant::now();self.waiting_scope=Some((a.ticket.clone(),a.nonce,a.generation));}
  let _refresh_error=self.finish_refresh().await.err();if let Some(current)=self.coordinator.lock().unwrap().active.as_ref().filter(|current|current.ticket==a.ticket){a.ctx.desktop=current.ctx.desktop.clone();a.refresh=current.refresh.clone();a.force_refresh=current.force_refresh;a.refresh_commit=current.refresh_commit.clone();a.refresh_storage_error=current.refresh_storage_error.clone();a.refresh_retry_at=current.refresh_retry_at;}
  self.start_refresh(&a)?;self.start_presence(&a)?;
  let Some(local)=self.pending.lock().unwrap().clone()else{return Ok(())};if local.last_seen.is_none_or(|v|v.elapsed()>Duration::from_secs(12)){if let Some(publisher)=&self.publisher{publisher.pause();}return Err("Native contact lost".into());}
  self.finish_waiting().await;self.start_waiting(&a,&local)?;
  if let (Some(publisher),Some(owner))=(&self.publisher,local.owner){if publisher.take_live_renewal(&movement_binding(&a.ctx,owner),a.ctx.game.access.as_str(),owner.0,a.generation){a.force_renew=true;}}
  {let mut coordinator=self.coordinator.lock().unwrap();if !coordinator.accepts(&a.ticket){return Ok(());}coordinator.authority.native_entry(&local);}
  let result=self.work(&mut a,&local,Instant::now()+Duration::from_secs(3)).await;
  if !self.current(&a){return Err("Client action cancelled".into());}
  let (entered,offered)={let mut coordinator=self.coordinator.lock().unwrap();if !coordinator.accepts(&a.ticket){return Err("Client action cancelled".into());}let entered=coordinator.authority.entered;let offered=coordinator.authority.offered;
   if let Some(active)=coordinator.active.as_mut(){active.ctx.game=a.ctx.game.clone();active.report=a.report.take();active.report_source=a.report_source.take();active.acknowledged=a.acknowledged.clone();active.renew=a.renew.take();active.reported=a.reported;active.force_renew=a.force_renew;if result.as_ref().err().is_some_and(|e|matches!(e.as_str(),"UNAUTHENTICATED"|"EXPIRED"|"REAUTH_REQUIRED")){active.force_renew=true;active.force_refresh=true;}} (entered,offered)};
  if result.as_ref().err().is_some_and(|e|!Core::<S>::transient_health_error(e)&&e!="Current native report awaiting matching canonical inspection"){
   if let Some(p)=self.pending.lock().unwrap().as_mut(){if p.owner==local.owner{p.release=false;}}if let Some(publisher)=&self.publisher{publisher.pause();}
  }
  if self.capture.as_ref().is_some_and(|task|task.is_finished()){
   if let Ok(Ok(reply))=self.capture.take().unwrap().await{self.capture_ready=Some(reply);}
  }
  if self.capture_ready.as_ref().is_some_and(|r|r.ticket!=a.ticket){self.capture_ready=None;}
  if a.capture_done{self.capture_ready=None;}else if entered&&self.capture.is_none()&&self.capture_ready.is_none(){self.start_capture(&a)?;}
  self.publish_movement(&a.ctx,&local,entered)?;
  self.revision=self.revision.checked_add(1).ok_or("Health revision exhausted")?;
  let storage_notice=self.coordinator.lock().unwrap().active.as_ref().and_then(|a|a.refresh_storage_error.clone()).or_else(||self.waiting_error.clone());
  let terminal=self.coordinator.lock().unwrap().authority.terminal;let notice=if terminal{result.as_ref().err().cloned().or(storage_notice)}else{storage_notice.or_else(||result.as_ref().err().cloned())};
  let mut slot=self.updates.lock().unwrap();if let Some(p)=slot.as_mut().filter(|p|p.ticket==a.ticket&&p.nonce==a.nonce&&p.generation==a.generation){p.revision=self.revision;p.output=Some(Output{desktop:a.ctx.desktop,game:a.ctx.game,reported:a.reported,entered,offered,error:notice,capture:self.capture_ready.clone()});}
  result
 }
 fn publish_movement(&self,ctx:&admission::Context,local:&handoff::Pending,entered:bool)->Result<(),String>{
  let Some(publisher)=&self.publisher else{return Ok(())};let p=self.pending.lock().unwrap().clone().ok_or("No current native owner")?;
  if !entered||!p.release||p.authority_until<=Instant::now()||p.owner!=local.owner||p.observation.as_ref().is_none_or(|r|r.observation.protected||!r.observation.complete||!r.observation.loaded){publisher.pause();return Ok(());}
  let owner=p.owner.ok_or("Missing native owner")?;let binding=movement_binding(ctx,owner);let scope=binding.scope.clone();
  publisher.bind_live_scope(nt2_desktop_g0::protocol_g3_live::wire::Scope{principal_id:Some(ctx.game.account.clone()),session_id:Some(ctx.game.id.clone()),session_epoch:Some(ctx.game.epoch.clone()),room_id:scope.room_id,run_id:scope.run_id,run_epoch:scope.run_epoch,config_revision:scope.config_revision,attachment_id:scope.attachment_id,world_token:scope.world_token});
  publisher.authorize(binding,zeroize::Zeroizing::new(ctx.game.access.as_str().into()),owner.0,p.authority_until,self.http.clone(),local.generation.as_ref().ok_or("Missing generation")?.1)
 }
 fn start_capture(&mut self,a:&Active)->Result<(),String>{
  let r=resume::Request{major:Some(1),revision:Some(nt2_desktop_g0::protocol_rejoin::REVISION.into()),service_id:Some(a.ctx.issuer.service.clone()),service_epoch:Some(a.ctx.epoch.clone()),session_epoch:Some(a.ctx.desktop.epoch.clone()),request_id:Some(uuid::Uuid::new_v4().to_string()),room_id:Some(a.ctx.room.clone()),status:Some(resume::Empty{}),..Default::default()};
  let job=original(&r,r.request_id.as_deref().unwrap(),&a.ctx.desktop,"/v3/rejoin/rpc",&self.coordinator)?;let ctx=a.ctx.clone();let ticket=a.ticket.clone();let http=self.http.clone();
  self.capture=Some(tokio::spawn(async move{let bytes=call(&http,&ctx,&job,&ctx.desktop,Instant::now()+Duration::from_secs(3)).await?;let value:Value=serde_json::from_slice(&bytes).map_err(|_|"Invalid selected status")?;verify(&value,&ctx,&job.id)?;let reply=nt2_desktop_g0::protocol_rejoin::response(&bytes)?;if reply.credentials.is_some(){return Err("Unexpected credentials in selected status".into());}Ok(CaptureReply{ticket,response:reply})}));Ok(())
 }
 fn start_presence(&mut self,a:&Active)->Result<(),String>{
  if self.presence.as_ref().is_some_and(|p|!p.is_finished()){return Ok(());}self.presence.take();let mut request=envelope(&a.ctx,false);request.presence=Some(wire::Presence{present:Some(true)});let job=original(&request,request.request_id.as_deref().unwrap(),&a.ctx.desktop,"/v1/rpc",&self.coordinator)?;let ctx=a.ctx.clone();let http=self.http.clone();self.presence=Some(tokio::spawn(async move{let bytes=call(&http,&ctx,&job,&ctx.desktop,Instant::now()+Duration::from_secs(3)).await?;let _=lobby(&bytes,&ctx,&job.id)?;Ok(())}));Ok(())
 }
 async fn finish_waiting(&mut self){
  if !self.waiting.as_ref().is_some_and(|t|t.is_finished()){return}
  match self.waiting.take().unwrap().await{
   Ok(WaitingReady{owner,marker}) if self.current(&owner)=>{match marker{Ok(marker)=>{let mut c=self.coordinator.lock().unwrap();if c.accepts(&owner.ticket){c.waiting_metadata=Some(marker);self.waiting_error=None;}},Err(reason)=>{self.waiting_error=Some(format!("Save & Quit recovery is not ready: {}",reason.chars().take(180).collect::<String>()));self.waiting_retry=Instant::now()+Duration::from_secs(2);}}},
   _=>{},
  }
 }
 fn start_waiting(&mut self,a:&Active,local:&handoff::Pending)->Result<(),String>{
  if self.waiting.is_some()||Instant::now()<self.waiting_retry||a.ctx.recovered||!local.loaded||!local.accepted{return Ok(())}
  let Some(bundle)=local.startup_bundle.clone()else{return Ok(())};let Some(owner)=local.owner else{return Ok(())};let Some(actual)=local.actual_seed else{return Ok(())};let Some(ack)=a.acknowledged.clone()else{return Ok(())};
  if !ack.observation.loaded||!ack.observation.protected||!ack.observation.complete||ack.context!=owner.1||ack.observation.actual_seed.as_deref()!=Some(actual.to_string().as_str())||!local.observation.as_ref().is_some_and(|r|r.context==owner.1&&r.digest==ack.digest&&r.observation.loaded&&r.observation.protected&&r.observation.complete){return Ok(())}
  let scope=bundle.scope();
  if bundle.generation()!=a.generation||scope.origin!=a.ctx.issuer.origin||scope.service!=a.ctx.issuer.service||scope.service_epoch!=a.ctx.epoch||scope.account!=a.ctx.game.account||scope.device!=a.device||scope.desktop!=a.ctx.desktop.id||scope.desktop_epoch!=a.ctx.desktop.epoch||scope.room!=a.ctx.room||scope.run!=a.ctx.run||scope.run_epoch!=a.ctx.run_epoch||scope.config!=u64::from(a.ctx.config)||hex(&owner.1)!=a.ctx.world{return Ok(())}
  if self.coordinator.lock().unwrap().waiting_metadata.as_ref().is_some_and(|m|m.bundle_digest==bundle.digest()){return Ok(())}
  let Some(path)=local.claim_path.clone()else{return Ok(())};let Some(attach)=local.startup_attach.clone()else{return Ok(())};
  if !self.lineage.block("canonical waiting enrollment"){return Ok(())}
  let store=self.store.clone();let writer=self.writer.clone();let coordinator=self.coordinator.clone();let nonce=self.nonce.clone();let generation=self.generation.clone();let a=a.clone();let http=self.http.clone();
  self.waiting=Some(tokio::spawn(async move{let result=waiting::enroll(store,writer,coordinator,nonce,generation,http,a.clone(),bundle,owner,actual,path,attach,ack).await;WaitingReady{owner:a,marker:result}}));Ok(())
 }
 fn start_refresh(&mut self,a:&Active)->Result<(),String>{
  if self.refresh.is_some(){return Ok(());}
  if let Some(commit)=a.refresh_commit.clone(){
   if commit.profile!=a.profile||commit.remember!=a.remember{
    let mut c=self.coordinator.lock().unwrap();if refresh_current(&c,a,&self.nonce,&self.generation){if let Some(active)=c.active.as_mut(){active.refresh_commit=None;active.refresh=None;active.refresh_retry_at=None;active.force_refresh=false;active.refresh_storage_error=Some("Credential storage cancelled after profile or remember choice changed".into());}}return Ok(());
   }
   if a.refresh_retry_at.is_some_and(|at|Instant::now()<at){return Ok(());}
   let store=self.store.clone();let writer=self.writer.clone();let coordinator=self.coordinator.clone();let nonce=self.nonce.clone();let generation=self.generation.clone();let a=a.clone();
   self.refresh=Some(tokio::spawn(async move{commit_refresh(store,writer,coordinator,nonce,generation,a,commit.session).await}));return Ok(());
  }
  if !a.force_refresh&&!self.expiring(a,&a.ctx.desktop){return Ok(());}let mut a=a.clone();
  let job=if let Some(job)=a.refresh.clone(){job}else{let mut request=envelope(&a.ctx,false);request.session_epoch=None;request.refresh=Some(wire::Refresh{secret:Some(a.ctx.desktop.refresh.as_str().into()),device_id:Some(a.device.to_string())});original(&request,request.request_id.as_deref().unwrap(),&a.ctx.desktop,"/v1/rpc",&self.coordinator)?};
  a.refresh=Some(job.clone());{let mut coordinator=self.coordinator.lock().unwrap();if !coordinator.accepts(&a.ticket){return Ok(());}if let Some(active)=coordinator.active.as_mut().filter(|current|current.nonce==a.nonce&&current.generation==a.generation){active.refresh=Some(job.clone());}else{return Ok(());}}
  let store=self.store.clone();let writer=self.writer.clone();let http=self.http.clone();let nonce=self.nonce.clone();let generation=self.generation.clone();let coordinator=self.coordinator.clone();
  self.refresh=Some(tokio::spawn(async move{
   // The immutable network original owns its secret/request pair, including
   // ambiguous retries. Never reconstruct it from a replacement credential.
   let request:wire::Request=serde_json::from_slice(&job.body).map_err(|_|"Invalid original refresh")?;
   let secret=request.refresh.and_then(|r|r.secret).ok_or("Missing original refresh secret")?;
   let record=Remembered{revision:protocol::REVISION.into(),device:a.device.to_string(),refresh:secret,request_id:job.id.clone(),started:now()};
   persist_refresh(store.clone(),writer.clone(),coordinator.clone(),nonce.clone(),generation.clone(),a.clone(),record).await?;
   let bytes=call(&http,&a.ctx,&job,&a.ctx.desktop,Instant::now()+Duration::from_secs(3)).await?;let mut reply=lobby(&bytes,&a.ctx,&job.id)?;
   let session=Session::read(reply.credentials.take().ok_or("Missing refreshed desktop")?,&a.device.to_string(),"desktop")?;
   if session.id!=a.ctx.desktop.id||session.epoch!=a.ctx.desktop.epoch||session.account!=a.ctx.desktop.account{return Err("Refreshed desktop owner changed".into());}
   let validity:c::ConnectionCredential=convert(reply.credential_validity.as_ref().ok_or("Missing refresh credential validity")?)?;
    {let mut c=coordinator.lock().unwrap();if !refresh_current(&c,&a,&nonce,&generation){return Err("Refresh owner retired".into());}admission::credential_sample(&mut c.credentials,&a.ctx.issuer.origin,&a.ctx.issuer.service,&session,&validity,&job.id,job.stamp)?;
     // Refresh revokes the prior desktop access token. Publish the verified RAM
     // credential before slow storage, retaining the exact recovery original.
     if let Some(active)=c.active.as_mut(){active.ctx.desktop=session.clone();active.force_refresh=false;active.refresh_commit=Some(RefreshCommit{session:session.clone(),profile:a.profile.clone(),remember:a.remember});active.refresh_storage_error=Some("Credential update awaiting protected storage".into());}}
   commit_refresh(store,writer,coordinator,nonce,generation,a,session).await
  }));Ok(())
 }
 async fn finish_refresh(&mut self)->Result<(),String>{
  if !self.refresh.as_ref().is_some_and(|p|p.is_finished()){return Ok(());}
  let result=self.refresh.take().unwrap().await.map_err(|_|"Refresh worker cancelled")?;let Ok(RefreshReady{owner:a,session,persist_error})=result else{return Ok(())};
  let mut coordinator=self.coordinator.lock().unwrap();if !refresh_current(&coordinator,&a,&self.nonce,&self.generation){return Ok(());}
  if let Some(active)=coordinator.active.as_mut(){active.ctx.desktop=session;active.force_refresh=false;
   if persist_error.is_some(){active.refresh_storage_error=Some("Credential update could not be saved; protected storage retry pending".into());active.refresh_retry_at=Some(Instant::now()+Duration::from_secs(2));}
   else{active.refresh=None;active.refresh_commit=None;active.refresh_retry_at=None;active.refresh_storage_error=None;}}Ok(())
 }
}
fn initial_waiting_status(ctx:&admission::Context,reply:&wire::Response)->Result<bool,String>{
 let room=reply.room.as_ref().ok_or("Missing scoped game room")?;let status=reply.game_status.as_ref().ok_or("Missing scoped game status")?;
 if room.room_id.as_deref()!=Some(&ctx.room)||room.run_id.as_deref()!=Some(&ctx.run)||room.run_epoch.as_deref()!=Some(&ctx.run_epoch)||room.config_revision!=Some(ctx.config){return Err("Scoped game room changed".into())}
 if room.state.as_deref()==Some("staging")&&status.phase.as_deref()==Some("hold")&&status.reason.as_deref()==Some("awaiting_final_start"){
  if !room.members.as_ref().is_some_and(|members|members.iter().any(|m|m.desktop_id.as_deref()==Some(&ctx.desktop.id)&&m.account_id.as_deref()==Some(&ctx.game.account)&&m.consented_config_revision==Some(ctx.config)&&m.included==Some(true)&&m.ready==Some(true))){return Err("Current staging inspection is not ready".into())}
  return Ok(true)
 }
 if room.state.as_deref()==Some("running")&&matches!(status.phase.as_deref(),Some("authorized"|"playing"|"restoring")){return Ok(false)}
 Err("Current game is not awaiting Start or admitted; world remains held".into())
}
// Known HTTP success is retained separately from an ambiguous network original.
// A storage-only retry never sends HTTP or replaces the protected original journal.
async fn commit_refresh<S:CredentialStore+'static>(store:Arc<S>,writer:Arc<Mutex<()>>,coordinator:Arc<Mutex<lifecycle::Coordinator>>,nonce:Arc<AtomicU64>,generation:Arc<AtomicU64>,a:Active,session:Session)->Result<RefreshReady,String>{
 let record=Remembered{revision:protocol::REVISION.into(),device:a.device.to_string(),refresh:session.refresh.as_str().into(),request_id:String::new(),started:0};
 let persist_error=persist_refresh(store,writer,coordinator,nonce,generation,a.clone(),record).await.err();
 Ok(RefreshReady{owner:a,session,persist_error})
}
// Both journal and final commit use the same serialized blocking writer. Neither
// lock contention nor CredentialStore I/O runs on the health-pump task.
fn refresh_current(c:&lifecycle::Coordinator,a:&Active,nonce:&AtomicU64,generation:&AtomicU64)->bool{
 a.nonce==nonce.load(Ordering::Acquire)&&a.generation==generation.load(Ordering::Acquire)&&c.accepts(&a.ticket)&&c.active.as_ref().is_some_and(|current|current.nonce==a.nonce&&current.generation==a.generation&&current.profile==a.profile&&current.remember==a.remember)
}
async fn persist_refresh<S:CredentialStore+'static>(store:Arc<S>,writer:Arc<Mutex<()>>,coordinator:Arc<Mutex<lifecycle::Coordinator>>,nonce:Arc<AtomicU64>,generation:Arc<AtomicU64>,a:Active,record:Remembered)->Result<(),String>{
 tokio::task::spawn_blocking(move||{
  let _guard=writer.lock().unwrap();
  if !refresh_current(&coordinator.lock().unwrap(),&a,&nonce,&generation){return Err("Refresh owner retired before persistence".into());}
  if a.remember{let profile=a.profile.as_ref().ok_or("Missing scoped refresh profile")?;let bytes=zeroize::Zeroizing::new(serde_json::to_vec(&record).map_err(|_|"Refresh persistence encoding failed")?);store.save(profile,&SecretBlob::new(bytes.to_vec()).map_err(|_|"Refresh persistence capacity")?).map_err(|e|e.to_string())?;}
  if !refresh_current(&coordinator.lock().unwrap(),&a,&nonce,&generation){return Err("Refresh owner retired during persistence".into());}Ok(())
 }).await.map_err(|_|"Refresh writer task failed".to_string())?
}
impl<S:CredentialStore> Drop for HealthRunner<S>{fn drop(&mut self){if let Some(task)=self.refresh.take(){task.abort();}if let Some(task)=self.presence.take(){task.abort();}if let Some(task)=self.capture.take(){task.abort();}if let Some(task)=self.waiting.take(){task.abort();}}}

#[cfg(test)]mod cr2_tests{
 use super::*;
 #[test]fn se1_current_health_keeps_entered_world_after_initial_review_expiry(){tauri::async_runtime::block_on(async{
  use base64::Engine;use super::super::setup::tests::{network,Reply};
  let(mut core,_,_)=super::super::tests::release_case();let(_,mut ctx,mut p,mut admission,stamp)=super::super::admission::cr2_tests::fixture();let jti="se1-current-game";
  ctx.game.access=SecretString::from(format!("fixture.{}.unsigned",base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(json!({"jti":jti}).to_string())));
  admission.sample.as_mut().unwrap().credential_id=Some(jti.into());let original=admission.clone();p.release=true;p.observation.as_mut().unwrap().observation.protected=false;p.observation.as_mut().unwrap().sequence+=1;admission.current_report.as_mut().unwrap().revision=Some(p.observation.as_ref().unwrap().sequence as u32);admission.current_report.as_mut().unwrap().protected=Some(false);admission.current_report.as_mut().unwrap().captured_at=Some("2099-01-01T00:00:00Z".into());admission.decision.as_mut().unwrap().initial_review_remaining_ms=Some(0);
  let validity=c::ConnectionCredential{principal_id:Some(ctx.game.account.clone()),session_id:Some(ctx.game.id.clone()),session_epoch:Some(ctx.game.epoch.clone()),credential_id:Some(jti.into()),origin_request_id:Some("retagged".into()),sample_key:Some("b".repeat(64)),access_remaining_ms:Some(900000)};
  let(origin,server)=network(vec![Reply::Data(json!({})),Reply::Data(json!({"admission":admission,"credentialValidity":validity}))]);ctx.issuer.origin=origin;
  core.preparation.as_mut().unwrap().review_expires_at=Some("2000-01-01T00:00:00Z".into());core.room.as_mut().unwrap().review_expires_at=Some("2000-01-01T00:00:00Z".into());
  let runner=core.health_runner();let mut a=runner.coordinator.lock().unwrap().active.clone().unwrap();a.ctx=ctx.clone();a.reported=p.observation.as_ref().unwrap().sequence;a.capture_done=true;
  let world=lifecycle::StableWorld{issuer:ctx.issuer.clone(),room:ctx.room.clone(),run:ctx.run.clone(),participant:ctx.participant.clone().unwrap_or_default(),world:ctx.world.clone()};
  {let mut co=runner.coordinator.lock().unwrap();co.saved(world.clone());a.ticket=co.bind(lifecycle::Owner{world,native:p.owner.unwrap(),session:ctx.desktop.id.clone(),generation:a.generation}).unwrap();co.active=Some(a.clone());co.request_sequence=1;
   let sample=original.sample.as_ref().unwrap();co.authority.install(original.clone(),&ctx,&{let mut prior=p.clone();prior.release=false;prior.observation.as_mut().unwrap().sequence-=1;prior.observation.as_mut().unwrap().observation.protected=true;prior},sample.origin_request_id.as_deref().unwrap(),jti,stamp,Instant::now()+Duration::from_secs(900)).unwrap();p.observation.as_mut().unwrap().observed_at=Instant::now();co.authority.native_entry(&p);assert!(co.authority.entered);
   let mut initial=validity.clone();initial.origin_request_id=Some("initial-current-game".into());admission::credential_sample(&mut co.credentials,&ctx.issuer.origin,&ctx.issuer.service,&ctx.game,&initial,"initial-current-game",nt2_desktop_g0::connection_time::Stamp{sequence:1,started:Instant::now()}).unwrap();}
  p.generation=Some((core.generation.clone(),a.generation));*runner.pending.lock().unwrap()=Some(p.clone());runner.work(&mut a,&p,Instant::now()+Duration::from_secs(3)).await.unwrap();
  let co=runner.coordinator.lock().unwrap();assert!(co.authority.entered&&co.authority.offered&&!co.authority.terminal);drop(co);assert!(runner.pending.lock().unwrap().as_ref().unwrap().release);let calls=server.join().unwrap();assert_eq!(calls.len(),2);assert!(calls[0]["gamePulse"].is_object()&&calls[1]["admission"].is_object());assert!(calls.iter().all(|q|q["preparationRenew"].is_null()&&q["gameRenew"].is_null()&&q["prepare"].is_null()));
 });}
 #[test]fn st1_waiting_writer_late_owner_cannot_save_or_publish(){tauri::async_runtime::block_on(async{
  for fault in 0..3{
   let(mut core,mut p,_)=super::super::tests::release_case();core.base="https://fixture.invalid".into();let mut runner=core.health_runner();let mut a=runner.coordinator.lock().unwrap().active.clone().unwrap();let owner=p.owner.unwrap();let actual=p.actual_seed.unwrap_or(u64::from(p.seed));
   let root=std::env::temp_dir().join(format!("nt2-st1-fence-{}",uuid::Uuid::new_v4()));std::fs::create_dir_all(&root).unwrap();let mut claim=owner.0.claim_prefix();claim.extend_from_slice(&owner.1);let path=root.join("claim");std::fs::write(&path,claim).unwrap();p.claim_path=Some(path);p.preparation_id=Some("original-preparation".into());p.loaded=true;p.accepted=true;p.actual_seed=Some(actual);
   let scope=nt2_desktop_g0::startup_local::Scope{origin:a.ctx.issuer.origin.clone(),service:a.ctx.issuer.service.clone(),service_epoch:a.ctx.epoch.clone(),account:a.ctx.game.account.clone(),device:a.device,desktop:a.ctx.desktop.id.clone(),desktop_epoch:a.ctx.desktop.epoch.clone(),room:a.ctx.room.clone(),run:a.ctx.run.clone(),run_epoch:a.ctx.run_epoch.clone(),config:u64::from(a.ctx.config),config_digest:"a".repeat(64),preparation:"original-preparation".into()};
   let bundle=Arc::new(nt2_desktop_g0::startup_local::Prepared::stage(scope,p.seed,p.world_policy,a.generation,Instant::now()+Duration::from_secs(60)).unwrap());p.startup_bundle=Some(bundle.clone());
   let observation=native_report::Observation{actual_seed:Some(actual.to_string()),mods:vec!["noita_together2".into()],complete:true,error:None,beta:None,loaded:true,protected:true,mod_version:"fixture".into(),bridge_version:"fixture".into(),..Default::default()};let digest=observation.content_digest().unwrap();p.observation=Some(native_report::Latest{context:owner.1,sequence:1,observation,digest,observed_at:Instant::now()});
   runner.start_waiting(&a,&p).unwrap();assert!(runner.waiting.is_none(),"No enrollment before service acknowledgement");
   a.acknowledged=p.observation.clone();p.startup_attach=Some(nt2_desktop_g0::startup_local::AttachEvidence{request_id:uuid::Uuid::new_v4().to_string(),fingerprint:"a".repeat(64),session_id:a.ctx.desktop.id.clone(),session_epoch:a.ctx.desktop.epoch.clone(),service_epoch:a.ctx.epoch.clone(),preparation:"original-preparation".into(),attachment:a.ctx.game.attachment.clone(),instance:a.ctx.instance.clone()});
   let writer=runner.writer.clone();let held=writer.lock().unwrap();runner.start_waiting(&a,&p).unwrap();assert!(runner.waiting.is_some());
   match fault{0=>{core.cancel_health();},1=>{core.generation.fetch_add(1,Ordering::AcqRel);},_=>{core.lifecycle.lock().unwrap().retire_owner();}}
   drop(held);let until=Instant::now()+Duration::from_secs(3);while !runner.waiting.as_ref().unwrap().is_finished(){assert!(Instant::now()<until);tokio::task::yield_now().await;}runner.finish_waiting().await;
   assert!(core.waiting_startup_marker().is_none());assert!(runner.waiting_error.is_none());let marker=bundle.marker(actual).unwrap();assert!(nt2_desktop_g0::startup_local::recall(&super::super::rejoin::ResumeStore(core.store.clone()),&a.ctx.issuer.origin,&a.ctx.issuer.service,&a.ctx.game.account,a.device,&marker).unwrap().is_none());std::fs::remove_dir_all(root).unwrap();
  }
 });}
 #[test]fn cr2_r3_pending_commit_rechecks_owner_profile_and_policy(){tauri::async_runtime::block_on(async{
  for fault in 0..3{
   let(mut core,_,_)=super::super::tests::release_case();let profile=ServiceProfile::new("https://pending-fixture.invalid","service",uuid::Uuid::new_v4()).unwrap();core.profile=Some(profile.clone());core.remember=true;let mut runner=core.health_runner();
   let original=SecretBlob::new(b"exact synthetic original journal".to_vec()).unwrap();core.store.save(&profile,&original).unwrap();
   {let mut c=runner.coordinator.lock().unwrap();let a=c.active.as_mut().unwrap();a.refresh_commit=Some(RefreshCommit{session:a.ctx.desktop.clone(),profile:a.profile.clone(),remember:true});a.refresh_storage_error=Some("Credential update could not be saved; protected storage retry pending".into());}
   let a=runner.coordinator.lock().unwrap().active.clone().unwrap();let writer=runner.writer.clone();let held=writer.lock().unwrap();runner.start_refresh(&a).unwrap();assert!(runner.refresh.is_some());
   match fault{0=>{core.remember=false;core.publish_health();},1=>{core.profile=Some(ServiceProfile::new(profile.origin(),profile.service_id(),uuid::Uuid::new_v4()).unwrap());core.publish_health();},_=>{core.cancel_health();core.lifecycle.lock().unwrap().retire_owner();}}
   drop(held);while !runner.refresh.as_ref().unwrap().is_finished(){tokio::task::yield_now().await;}runner.finish_refresh().await.unwrap();
   let active=runner.coordinator.lock().unwrap().active.clone();if let Some(a)=active{runner.start_refresh(&a).unwrap();assert!(runner.refresh.is_none());assert!(runner.coordinator.lock().unwrap().active.as_ref().unwrap().refresh_commit.is_none());}
   assert_eq!(core.store.load(&profile).unwrap().unwrap().bytes(),original.bytes());
  }
 });}
 #[test]fn cr2_correction_queued_writer_rechecks_policy_profile_and_retirement(){tauri::async_runtime::block_on(async{
  for fault in 0..3{
   let(mut core,_,_)=super::super::tests::release_case();let profile=ServiceProfile::new("https://writer-fixture.invalid","service",uuid::Uuid::new_v4()).unwrap();core.profile=Some(profile.clone());core.remember=true;let runner=core.health_runner();let a=runner.coordinator.lock().unwrap().active.clone().unwrap();let record=Remembered{revision:protocol::REVISION.into(),device:a.device.to_string(),refresh:"A".repeat(43),request_id:String::new(),started:0};
   let held=runner.writer.lock().unwrap();let store=runner.store.clone();let writer=runner.writer.clone();let coordinator=runner.coordinator.clone();let nonce=runner.nonce.clone();let generation=runner.generation.clone();let queued=tokio::spawn(async move{persist_refresh(store,writer,coordinator,nonce,generation,a,record).await});tokio::task::yield_now().await;
   match fault{0=>{core.remember=false;core.publish_health();},1=>{core.profile=Some(ServiceProfile::new(profile.origin(),profile.service_id(),uuid::Uuid::new_v4()).unwrap());core.publish_health();},_=>{core.cancel_health();core.lifecycle.lock().unwrap().retire_owner();}}
   drop(held);assert!(queued.await.unwrap().is_err());assert!(core.store.load(&profile).unwrap().is_none());
  }
 });}
 #[test]fn cr2_late_typed_refresh_cannot_publish_or_persist_after_retirement(){tauri::async_runtime::block_on(async{
  let(mut core,_,_)=super::super::tests::release_case();let mut runner=core.health_runner();let profile=ServiceProfile::new("https://refresh-fixture.invalid","service",uuid::Uuid::new_v4()).unwrap();core.profile=Some(profile.clone());core.remember=true;core.publish_health();let a=runner.coordinator.lock().unwrap().active.clone().unwrap();assert_eq!(a.profile.as_ref().unwrap().origin(),profile.origin());assert!(a.remember);let mut request=envelope(&a.ctx,false);request.presence=Some(wire::Presence{present:Some(true)});let job=original(&request,request.request_id.as_deref().unwrap(),&a.ctx.desktop,"/v1/rpc",&runner.coordinator).unwrap();let desktop=core.desktop.as_ref().unwrap().id.clone();let before=core.store.load(&profile).unwrap().map(|b|b.bytes().to_vec());
  let session=a.ctx.desktop.clone();runner.refresh=Some(tokio::spawn(async move{let _=job;Ok(RefreshReady{owner:a,session,persist_error:None})}));
  core.cancel_health();core.lifecycle.lock().unwrap().retire_owner();while !runner.refresh.as_ref().unwrap().is_finished(){tokio::task::yield_now().await;}runner.finish_refresh().await.unwrap();assert_eq!(core.desktop.as_ref().unwrap().id,desktop);assert_eq!(core.store.load(&profile).unwrap().map(|b|b.bytes().to_vec()),before);assert!(runner.coordinator.lock().unwrap().active.is_none());
 });}
 #[test]fn cr2_health_calls_share_one_complete_attempt_deadline(){tauri::async_runtime::block_on(async{
  use tokio::io::{AsyncReadExt,AsyncWriteExt};let listener=tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();let origin=format!("http://{}",listener.local_addr().unwrap());
  let server=tokio::spawn(async move{for _ in 0..2{let(mut socket,_)=listener.accept().await.unwrap();let mut bytes=vec![0;4096];let _=socket.read(&mut bytes).await;tokio::time::sleep(Duration::from_millis(90)).await;let _=socket.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}").await;}});
  let(mut core,_,_)=super::super::tests::release_case();let runner=core.health_runner();let mut a=runner.coordinator.lock().unwrap().active.clone().unwrap();a.ctx.issuer.origin=origin;let mut request=envelope(&a.ctx,false);request.presence=Some(wire::Presence{present:Some(true)});let job=original(&request,request.request_id.as_deref().unwrap(),&a.ctx.desktop,"/v1/rpc",&runner.coordinator).unwrap();let until=Instant::now()+Duration::from_millis(150);
  call(&runner.http,&a.ctx,&job,&a.ctx.desktop,until).await.unwrap();assert_eq!(call(&runner.http,&a.ctx,&job,&a.ctx.desktop,until).await.unwrap_err(),"Whole health attempt deadline");server.await.unwrap();
 });}
 #[test]fn cr2_transient_health_failure_preserves_but_never_extends_current_lease(){tauri::async_runtime::block_on(async{
  let(mut core,_,_)=super::super::tests::release_case();let mut runner=core.health_runner();let until=Instant::now()+Duration::from_secs(12);{let mut p=core.pending.lock().unwrap();p.as_mut().unwrap().release=true;p.as_mut().unwrap().authority_until=until;}
  let listener=std::net::TcpListener::bind("127.0.0.1:0").unwrap();let origin=format!("http://{}",listener.local_addr().unwrap());drop(listener);
  {let mut c=runner.coordinator.lock().unwrap();c.active.as_mut().unwrap().ctx.issuer.origin=origin;c.authority.authority.install(Stamp{sequence:1,started:Instant::now()},12000,30000,12000,Instant::now()).unwrap();}
  assert_eq!(runner.pump().await.unwrap_err(),"Candidate connection unavailable");let p=core.pending.lock().unwrap();assert!(p.as_ref().unwrap().release);assert_eq!(p.as_ref().unwrap().authority_until,until);assert!(runner.coordinator.lock().unwrap().authority.authority.live(Instant::now()));
 });}
 #[test]fn cr2_scoped_terminal_checkpoint_stops_worker_and_cannot_be_republished(){
  let(mut core,_,_)=super::super::tests::release_case();let runner=core.health_runner();let a=runner.coordinator.lock().unwrap().active.clone().unwrap();runner.coordinator.lock().unwrap().authority.terminal=true;
  {let mut slot=runner.updates.lock().unwrap();let p=slot.as_mut().unwrap();p.revision=1;p.output=Some(Output{desktop:a.ctx.desktop,game:a.ctx.game,reported:0,entered:false,offered:false,error:Some("Canonical participation terminal".into()),capture:None});}
  core.sync_health();assert!(!core.health_active());core.publish_health();assert!(runner.coordinator.lock().unwrap().active.is_none());assert_eq!(core.native_stage,"ended");
 }
 #[test]fn cr2_health_backoff_is_bounded_scoped_and_preserves_authority(){
  let(mut core,_,_)=super::super::tests::release_case();let runner=core.health_runner();let a=runner.coordinator.lock().unwrap().active.clone().unwrap();let until=core.pending.lock().unwrap().as_ref().unwrap().authority_until;
  let response=json!({"serviceId":a.ctx.issuer.service,"serviceEpoch":a.ctx.epoch,"requestId":"request","error":{"code":"CAPACITY","retryAfterMs":15000}});assert_eq!(runner.health_reply(&a,&serde_json::to_vec(&response).unwrap(),"request").unwrap_err(),"CAPACITY");let next=runner.coordinator.lock().unwrap().active.as_ref().unwrap().not_before.unwrap();assert!(next>=Instant::now()+Duration::from_secs(14));assert_eq!(core.pending.lock().unwrap().as_ref().unwrap().authority_until,until);
  let mut wrong=response.clone();wrong["requestId"]=json!("other");wrong["error"]["retryAfterMs"]=json!(60000);assert!(runner.health_reply(&a,&serde_json::to_vec(&wrong).unwrap(),"request").is_err());assert_eq!(runner.coordinator.lock().unwrap().active.as_ref().unwrap().not_before,Some(next));
  let mut malformed=response;malformed["error"]["retryAfterMs"]=json!(60001);assert_eq!(runner.health_reply(&a,&serde_json::to_vec(&malformed).unwrap(),"request").unwrap_err(),"Invalid server backoff");
 }
}

#[path="next_health_waiting.rs"] mod waiting;
