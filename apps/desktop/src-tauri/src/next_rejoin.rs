//! Core local seams; Server status/transport orchestration is composed in RJ2.
use super::*;
use nt2_desktop_g0::{rejoin_local as local,secrets as protected};
use nt2_desktop_g0::protocol_rejoin::{self as protocol,wire as resume};
#[derive(Clone,Default)]
pub(super) struct Transport {
 pub(super) contact:Option<crate::pipe::rejoin::Contact>,
 pub(super) generation:u32,instance:Option<uuid::Uuid>,awaiting_fresh:bool,pub(super) rediscover:bool,next_health:Option<Instant>,next_presence:Option<Instant>,observation:Option<zeroize::Zeroizing<Vec<u8>>>,pub(super) offline:Option<zeroize::Zeroizing<Vec<u8>>>,
 pub(super) restored_original:bool,next_outcome:Option<Instant>,next_owner_status:Option<Instant>,not_before:Option<Instant>,pub(super) offline_scope:Option<lifecycle::OriginalScope>,pub(super) resume_scope:Option<lifecycle::OriginalScope>,resume_uncertain:Option<zeroize::Zeroizing<Vec<u8>>>,
}
pub(super) struct Parked {pub(super) intent:local::ResumeIntent,pub(super) transport:Transport}
pub(super) struct ResumeStore<S:CredentialStore>(pub(super) Arc<S>);
fn protected_error(e:crate::secrets::StoreError)->protected::StoreError{match e{
 crate::secrets::StoreError::Windows{operation,code,blob_bytes}=>protected::StoreError::Windows{operation,code,blob_bytes},
 crate::secrets::StoreError::InvalidProfile=>protected::StoreError::InvalidProfile,crate::secrets::StoreError::InvalidSecret=>protected::StoreError::InvalidSecret,crate::secrets::StoreError::Corrupt=>protected::StoreError::Corrupt,crate::secrets::StoreError::Unavailable=>protected::StoreError::Unavailable,
}}
impl<S:CredentialStore> protected::CredentialStore for ResumeStore<S>{
 fn save(&self,p:&protected::ServiceProfile,b:&protected::SecretBlob)->Result<(),protected::StoreError>{
  let p=ServiceProfile::new(p.origin(),p.service_id(),p.profile_id()).map_err(|_|protected::StoreError::InvalidProfile)?;
  let b=SecretBlob::new(b.bytes().to_vec()).map_err(|_|protected::StoreError::InvalidSecret)?;
  self.0.save(&p,&b).map_err(protected_error)
 }
 fn load(&self,p:&protected::ServiceProfile)->Result<Option<protected::SecretBlob>,protected::StoreError>{
  let p=ServiceProfile::new(p.origin(),p.service_id(),p.profile_id()).map_err(|_|protected::StoreError::InvalidProfile)?;
  self.0.load(&p).map_err(protected_error)?.map(|b|protected::SecretBlob::new(b.bytes().to_vec())).transpose()
 }
 fn delete(&self,p:&protected::ServiceProfile)->Result<(),protected::StoreError>{
  let p=ServiceProfile::new(p.origin(),p.service_id(),p.profile_id()).map_err(|_|protected::StoreError::InvalidProfile)?;
  self.0.delete(&p).map_err(protected_error)
 }
}
impl<S:CredentialStore> Core<S>{
 async fn reconcile_health_owner(&mut self)->Result<(),String>{
  if self.lifecycle.lock().unwrap().authority.authority.live(Instant::now()){return Ok(());}
  if self.rejoin_transport.next_owner_status.is_some_and(|t|Instant::now()<t){return Ok(());}
  self.rejoin_transport.next_owner_status=Some(Instant::now()+Duration::from_secs(2));
  let Some(a)=self.resume_intent.association().cloned()else{return Ok(())};
  let mut request=self.rejoin_request(false)?;request.status=Some(resume::Empty{});
  let bytes=serde_json::to_vec(&request).map_err(|_|"Owner status encoding failed")?;
  let response=tokio::time::timeout(Duration::from_secs(3),self.rejoin_rpc(&bytes,false)).await.map_err(|_|"Owner status deadline")??;
  let participant=response.participant.as_ref().ok_or("Missing scoped owner status")?;
  if !participant.association.as_ref().is_some_and(|v|a.matches_canonical(v)){return Err("Owner status association differs".into());}
  let generation=participant.generation.filter(|v|*v>0).ok_or("Missing scoped owner generation")?;
  if participant.return_eligible==Some(false){
   self.cancel_health();self.lifecycle.lock().unwrap().retire_owner();self.g3_pause();
   self.resume_intent.terminal();self.native_diagnostics.lock().unwrap().rejoin.terminal=true;
   if let Some(p)=self.pending.lock().unwrap().as_mut(){p.release=false;p.authority_until=Instant::now();}
   self.persist_recovery()?;return Ok(());
  }
  if participant.return_eligible!=Some(true){return Err("Owner return eligibility unavailable".into());}
  if participant.state.as_deref()==Some("active")&&generation==self.rejoin_transport.generation{return Ok(());}
  if !matches!(participant.state.as_deref(),Some("active"|"reconnecting")){return Err("Owner status is not a resumable disposition".into());}
  // Only this exact authoritative disposition retires the expired owner. A
  // generic permission/network error leaves the same owner eligible to heal.
  self.cancel_health();self.lifecycle.lock().unwrap().retire_owner();self.g3_pause();self.game=None;
  self.rejoin_transport.generation=generation;self.rejoin_transport.observation=None;
  self.resume_intent.transient_loss(Instant::now());
  if let Some(p)=self.pending.lock().unwrap().as_mut(){p.release=false;p.authority_until=Instant::now();}
  Ok(())
 }
 /// Caller has decoded an authenticated canonical status for the selected owner.
 /// This returns only bounded non-secret metadata; Lua receives no resume proof.
 pub(crate) fn remember_selected_world(&mut self,a:local::Association,proof:protected::SecretBlob)->Result<Vec<u8>,String>{
  self.resume_intent.validate_selected(&a)?;let desktop=self.desktop.as_ref().ok_or("Sign in before recording saved association")?;
  let p=self.pending.lock().unwrap();let pending=p.as_ref().ok_or("No admitted native world")?;
  if a.origin!=self.base||a.service!=self.service||a.account.to_string()!=desktop.account||a.run.to_string()!=pending.run||u64::from(a.config_revision)!=pending.config
   ||!pending.loaded||!pending.accepted||!self.game.as_ref().is_some_and(|g|g.entered)
   ||pending.actual_seed.map(|s|s.to_string()).as_deref()!=Some(&a.actual_seed)
   ||self.waiting_world().map(String::from).or_else(||pending.owner.map(|(_,token)|hex(&token))).as_deref()!=Some(&a.world_token){return Err("Association is not this actually admitted selected world".into())}
  let room=self.room.as_ref().ok_or("Missing frozen room configuration")?;
  if room.room_id.as_deref()!=Some(a.room.to_string().as_str()){return Err("Saved association differs from room".into())}
  let config=room.config.as_ref().ok_or("Missing frozen configuration")?;use sha2::Digest;
  if format!("{:x}",sha2::Sha256::digest(serde_json::to_vec(config).map_err(|_|"Invalid frozen config")?))!=a.frozen_config_digest{return Err("Saved configuration digest differs".into())}
  drop(p);let record=local::remember_selected(&ResumeStore(self.store.clone()),&mut self.resume_intent,a,&proof)?;
  self.store.save(&self.rejoin_index_profile()?,&SecretBlob::new(record.clone()).map_err(|_|"Invalid association index")?).map_err(|_|"Association index storage unavailable")?;Ok(record)
 }
 pub(crate) fn offer_saved_world(&mut self,metadata:&[u8])->Result<(),String>{
  let a=local::Association::decode(metadata)?;
  if a.origin!=self.base||a.service!=self.service||self.desktop.as_ref().is_some_and(|d|d.account!=a.account.to_string()){return Err("Saved world belongs to another selected account/service".into())}
  let proof=local::recall(&ResumeStore(self.store.clone()),&a).map_err(|e|e.to_string())?;
  self.resume_intent.offer(a,proof.is_some())
 }
 pub(super) fn park_foreign_recovery(&mut self)->Result<(),String>{
  self.restore_parked_recovery()?;
  let issuer=self.lifecycle_issuer();
  let matches=|a:&local::Association|issuer.as_ref().is_some_and(|i|i.origin==a.origin&&i.service==a.service&&i.account==a.account);
  if self.resume_intent.association().is_some_and(|a|!matches(a)){
   if self.parked_recovery.len()>=16{return Err("Parked recovery capacity reached; resolve an original account before switching again".into());}
   self.persist_recovery()?;
   let intent=std::mem::take(&mut self.resume_intent);let mut transport=std::mem::take(&mut self.rejoin_transport);
   transport.contact=None;transport.observation=None;transport.rediscover=false;
   self.parked_recovery.push(Parked{intent,transport});self.native_diagnostics.lock().unwrap().rejoin=Default::default();
  }
  if self.resume_intent.association().is_none()&&!self.lifecycle.lock().unwrap().fresh_intent(){
   if let Some(index)=self.parked_recovery.iter().position(|p|p.intent.association().is_some_and(matches)){
    let p=self.parked_recovery.remove(index);self.resume_intent=p.intent;self.rejoin_transport=p.transport;
   }
  }Ok(())
 }
 pub(super) fn park_for_fresh_room(&mut self,room:&str)->Result<(),String>{
  self.park_foreign_recovery()?;
  if self.pending.lock().unwrap().as_ref().is_some_and(|p|p.owner.is_some()){return Ok(());}
  if self.resume_intent.association().is_none_or(|a|a.room==room){return Ok(());}
  if self.parked_recovery.len()>=16{return Err("Parked recovery capacity reached; resolve an original world first".into());}
  self.persist_recovery()?;
  let intent=std::mem::take(&mut self.resume_intent);let mut transport=std::mem::take(&mut self.rejoin_transport);
  transport.contact=None;transport.observation=None;transport.rediscover=false;
  self.parked_recovery.push(Parked{intent,transport});
  if let Some(issuer)=self.lifecycle_issuer(){self.lifecycle.lock().unwrap().fresh(issuer,room.into());}
  self.native_diagnostics.lock().unwrap().rejoin=Default::default();self.persist_recovery()
 }
 // Called only for an explicit Join after a correlated current staging run pair.
 // Pure historical selection is not an unresolved queue; its proof stays keyed
 // by saved identity. Outstanding operations are parked verbatim, never deleted.
 pub(super) fn park_completed_run_recovery(&mut self,room:&str,run:&str,epoch:&str)->Result<(),String>{
  if uuid::Uuid::parse_str(run).is_err()||uuid::Uuid::parse_str(epoch).is_err(){return Err("Complete current run identity required".into())}
  if self.resume_intent.association().is_none_or(|a|a.room!=room||a.run==run){return Ok(())}
  let queued=self.resume_intent.original_operation().is_some()||self.rejoin_transport.offline.is_some()||self.rejoin_transport.resume_uncertain.is_some()
   ||(self.resume_intent.phase()==local::Phase::Offline&&self.resume_intent.association().is_some_and(|a|local::retired_proof(&ResumeStore(self.store.clone()),a).map_or(true,|p|p.is_some())));
  if queued&&self.parked_recovery.len()>=16{return Err("Resolve original operation capacity before another run".into())}
  self.persist_recovery()?;let intent=std::mem::take(&mut self.resume_intent);let mut transport=std::mem::take(&mut self.rejoin_transport);transport.contact=None;transport.observation=None;
  if queued{self.parked_recovery.push(Parked{intent,transport});}
  if let Some(issuer)=self.lifecycle_issuer(){self.lifecycle.lock().unwrap().fresh(issuer,room.into());}
  self.native_diagnostics.lock().unwrap().rejoin=Default::default();Ok(())
 }
 fn select_parked_world(&mut self,a:&local::Association)->Result<bool,String>{
  let Some(index)=self.parked_recovery.iter().position(|p|p.intent.association()==Some(a))else{return Ok(false)};
  self.persist_recovery()?;
  let selected=self.parked_recovery.remove(index);let old_intent=std::mem::replace(&mut self.resume_intent,selected.intent);let old_transport=std::mem::replace(&mut self.rejoin_transport,selected.transport);
  let queued_offline=old_intent.phase()==local::Phase::Offline&&old_intent.association().is_some_and(|world|local::retired_proof(&ResumeStore(self.store.clone()),world).map_or(true,|proof|proof.is_some()));
  if old_intent.association().is_some()&&(queued_offline||old_intent.original_operation().is_some()||old_transport.offline.is_some()||old_transport.resume_uncertain.is_some()){self.parked_recovery.push(Parked{intent:old_intent,transport:old_transport});}
  // Positive routing of this exact saved world restores its original envelope,
  // rather than creating a second operation beside a hidden parked one.
  self.cancel_health();self.lifecycle.lock().unwrap().saved(lifecycle::StableWorld::saved(a));self.g3_pause();self.game=None;
  if let Some(p)=self.pending.lock().unwrap().as_mut(){p.release=false;p.authority_until=Instant::now();}
  self.persist_recovery()?;Ok(true)
 }
 pub(super) fn fresh_join_recovery_blocker(&mut self)->Result<Option<&'static str>,String>{
  self.park_foreign_recovery()?;
  let retirement_pending=self.rejoin_transport.offline.is_some()
   ||(self.resume_intent.phase()==local::Phase::Offline&&self.resume_intent.association().is_some_and(|a|
    local::retired_proof(&ResumeStore(self.store.clone()),a).map_or(true,|p|p.is_some())));
  if retirement_pending{return Ok(Some("Previous offline request is still being confirmed; fresh lobby setup has not started"))}
  if self.resume_intent.original_operation().is_some(){return Ok(Some("Previous recovery request outcome is unresolved; fresh lobby setup has not started"))}
  Ok(None)
 }
 pub fn rejoin_previous(&mut self)->Result<(),String>{
  if self.lc1{return self.lc1_choose(false)}
  if self.fresh_setup_owns_world(){return Err("Fresh lobby setup owns this session; saved-world recovery has not started".into())}
  let a=self.resume_intent.association().ok_or("No known saved world")?;
  if a.origin!=self.base||a.service!=self.service||self.desktop.as_ref().is_some_and(|d|d.account!=a.account){return Err("Sign in to the original saved-world account/service".into())}
  let world=lifecycle::StableWorld::saved(a);self.resume_intent.choose_rejoin()?;self.lifecycle.lock().unwrap().saved(world);Ok(())
 }
 pub fn continue_offline(&mut self)->Result<(),String>{
  if self.lc1{return self.lc1_choose(true)}
  if self.offline_provisional()?{return Ok(())}
  let local_pending=self.pending.lock().unwrap().clone();let metadata=self.native_diagnostics.lock().unwrap().rejoin.metadata.clone();
  let waiting=local_pending.as_ref().and_then(|p|p.startup_bundle.clone().zip(p.actual_seed)).and_then(|(b,actual)|{let m=metadata.as_ref().and_then(|(_,_,bytes)|nt2_desktop_g0::startup_local::WaitingMarker::decode(bytes).ok()).filter(|m|m.local_world==b.world()&&m.bundle_digest==b.digest()&&m.actual_seed==actual.to_string()).or_else(||b.marker(actual).ok());m.map(|m|(b,m))});
  // Keep the protected proof until canonical offline retirement is acknowledged.
  // Local terminal intent stops resume immediately; it never means RPC success.
  if let Some(a)=self.resume_intent.association(){local::retire_local(&ResumeStore(self.store.clone()),a).map_err(|e|e.to_string())?;}
  if let Some(bytes)=self.resume_intent.original_operation(){self.rejoin_transport.resume_uncertain=Some(zeroize::Zeroizing::new(bytes.to_vec()));}
  self.lifecycle.lock().unwrap().offline();
  self.resume_intent.offline();self.revoke_native();self.retire_waiting_local()?;
  if let Some((bundle,marker))=waiting{
   let scope=bundle.scope();if scope.origin==self.base&&scope.service==self.service&&scope.device==self.device&&self.desktop.as_ref().is_some_and(|d|d.account==scope.account){
    let selected=self.select_waiting_for_retirement(scope,&marker);let retirement=if selected.is_ok(){self.retire_waiting_local()}else{selected};
    let _writer=self.credential_writer.lock().unwrap();nt2_desktop_g0::startup_local::retire(&ResumeStore(self.store.clone()),scope,&marker)?;retirement?;
   }
  }
  {let mut d=self.native_diagnostics.lock().unwrap();d.rejoin.offline=true;d.rejoin.lookup=5;}
  self.persist_recovery()?;Ok(())
 }
}
impl<S:CredentialStore> Core<S>{
 async fn rediscover_rejoin_issuer(&mut self)->Result<(),String>{
  // No credential is sent until HTTPS discovery confirms this exact issuer.
  let mut response=self.http.get(format!("{}/v1/info",self.base)).send().await.map_err(|_|"Candidate connection unavailable")?.error_for_status().map_err(|_|"Candidate HTTP refused")?;
  let mut bytes=Vec::new();while let Some(chunk)=response.chunk().await.map_err(|_|"Candidate response unavailable")?{if bytes.len()+chunk.len()>65536{return Err("Recovery discovery exceeds bound".into())}bytes.extend_from_slice(&chunk);}
  let info=super::protocol::info(&bytes).map_err(|_|"Recovery issuer contract differs")?;
  if info.service_id.as_deref()!=Some(&self.service)||info.major!=Some(1)||info.revision.as_deref()!=Some(super::protocol::REVISION){return Err("Saved recovery issuer differs; sign in must be reviewed".into())}
  let epoch=required(info.service_epoch.clone())?;
  {
   self.cancel_health();self.lifecycle.lock().unwrap().retire_owner();
   self.generation.fetch_add(1,Ordering::AcqRel);self.g3_pause();
   if let Some(p)=self.pending.lock().unwrap().as_mut(){p.release=false;p.authority_until=Instant::now();}
   self.epoch=epoch;self.info=Some(info);self.game=None;self.g3_release=None;self.rejoin_transport.observation=None;self.rejoin_transport.awaiting_fresh=false;
   self.resume_intent.transient_loss(Instant::now());
  }
  self.rotate().await?;self.rejoin_transport.rediscover=false;Ok(())
 }
 #[cfg(test)]
 pub(super) fn rejoin_retirement_verified(&self)->bool{
  self.resume_intent.association().is_some_and(|a|local::is_retired(&ResumeStore(self.store.clone()),a).unwrap()&&local::recall(&ResumeStore(self.store.clone()),a).unwrap().is_none()&&local::retired_proof(&ResumeStore(self.store.clone()),a).unwrap().is_none())
 }
 #[cfg(test)]
 pub(super) fn rejoin_due_for_controlled_clock(&mut self){self.rejoin_transport.next_health=None;self.rejoin_transport.next_presence=None;}
 pub(super) fn rejoin_index_profile(&self)->Result<ServiceProfile,String>{
  let account=&self.desktop.as_ref().ok_or("Sign in first")?.account;use sha2::Digest;
  let bytes=sha2::Sha256::digest(serde_json::to_vec(&("nt2-rejoin-index-v1",self.device,account)).map_err(|_|"Invalid association index scope")?);
  ServiceProfile::new(&self.base,&self.service,uuid::Uuid::from_bytes(bytes[..16].try_into().unwrap())).map_err(|_|"Invalid association index profile".into())
 }
 fn rejoin_request(&self,game:bool)->Result<resume::Request,String>{
  let session=if game{self.game.as_ref().map(|g|g.session.clone())}else{self.desktop.clone()}.ok_or("Sign in first")?;
  let a=self.resume_intent.association();
  Ok(resume::Request{major:Some(1),revision:Some(protocol::REVISION.into()),service_id:Some(self.service.clone()),service_epoch:Some(self.epoch.clone()),session_epoch:Some(session.epoch.clone()),request_id:Some(uuid::Uuid::new_v4().to_string()),room_id:a.map(|a|a.room.clone()),run_id:a.map(|a|a.run.clone()),participant_id:a.map(|a|a.participant.clone()),world_token:a.map(|a|a.world_token.clone()),config_revision:a.map(|a|a.config_revision),generation:Some(self.rejoin_transport.generation),..Default::default()})
 }
 async fn rejoin_rpc(&mut self,bytes:&[u8],game:bool)->Result<resume::Response,String>{
  if bytes.is_empty()||bytes.len()>32768{return Err("Invalid bounded recovery request".into())}
  let request:resume::Request=serde_json::from_slice(bytes).map_err(|_|"Invalid recovery request")?;
  let session=if game{self.game.as_ref().map(|g|g.session.clone())}else{self.desktop.clone()}.ok_or("Sign in first")?;
  let issuer=self.lifecycle_issuer().ok_or("Sign in to the original recovery account")?;
  if request.service_id.as_deref()!=Some(&issuer.service)||request.service_epoch.as_deref()!=Some(&self.epoch)||request.session_epoch.as_deref()!=Some(&session.epoch){return Err("Original recovery operation parked: authentication changed".into());}
  // Initial capture is a proof-free current-desktop status query. It does not
  // inherit a historical association and is not an original mutation replay.
  let selected_capture=!game&&request.status.is_some()&&request.resume.is_none()&&request.offline.is_none()&&request.observe.is_none()&&request.pulse.is_none()&&request.review.is_none()&&request.list.is_none()
   &&request.run_id.is_none()&&request.participant_id.is_none()&&request.world_token.is_none()&&request.config_revision.is_none()&&request.generation.is_none()
   &&self.rejoin_transport.contact.is_none()&&request.room_id.is_some()&&request.room_id==self.room.as_ref().and_then(|r|r.room_id.clone())
   &&self.game.as_ref().is_some_and(|g|g.entered)&&self.pending.lock().unwrap().as_ref().is_some_and(|p|self.native_authority_valid(p));
  if !selected_capture{
   let a=self.resume_intent.association().ok_or("Recovery world unavailable")?;
   if a.origin!=issuer.origin||a.service!=issuer.service||a.account!=issuer.account||request.room_id.as_deref()!=Some(&a.room)||request.config_revision!=Some(a.config_revision)||request.run_id.as_deref()!=Some(&a.run)||request.participant_id.as_deref()!=Some(&a.participant)||request.world_token.as_deref()!=Some(&a.world_token){return Err("Original recovery operation parked: destination/account/world changed".into());}
   let original=if request.offline.is_some(){&mut self.rejoin_transport.offline_scope}else{&mut self.rejoin_transport.resume_scope};
   if request.offline.is_some()||request.resume.is_some(){
    let id=request.request_id.as_deref().ok_or("Missing original operation identity")?;
    let scope=original.get_or_insert_with(||lifecycle::OriginalScope::capture(a,&self.epoch,&session.id,&session.epoch,id,bytes));
    if !scope.permits(&issuer,&self.epoch,&session.id,&session.epoch,id,bytes,a){return Err("Original recovery operation parked: authenticated outcome reconciliation required".into());}
   }
  }
  if request.offline.is_some()||request.resume.is_some(){self.persist_recovery()?;}
  let stamp=self.lifecycle.lock().unwrap().request_stamp(&json!([self.base,self.service,self.epoch,session.id,session.epoch,"rejoin"]).to_string(),request.request_id.as_deref().ok_or("Missing original timing identity")?)?;
  let generation=self.generation.load(Ordering::Acquire);
  let mut response=self.http.post(format!("{}{}",self.base,protocol::ENDPOINT)).header("content-type","application/json").bearer_auth(session.access.as_str()).body(bytes.to_vec()).send().await.map_err(|_|"Candidate connection unavailable")?.error_for_status().map_err(|_|"Candidate HTTP refused")?;
  let mut body=zeroize::Zeroizing::new(Vec::new());
  while let Some(chunk)=response.chunk().await.map_err(|_|"Candidate response unavailable")?{if body.len()+chunk.len()>65536{return Err("Recovery response exceeds bound".into())}body.extend_from_slice(&chunk);}
  if generation!=self.generation.load(Ordering::Acquire)||self.health_ticket.is_some_and(|ticket|ticket!=self.health_nonce.load(Ordering::Acquire)){return Err("Client action cancelled".into())}
  let value=protocol::response(&body)?;
  if value.request_id!=request.request_id||value.service_id.as_deref()!=Some(&self.service){return Err("Recovery response context changed".into())}
  if value.service_epoch.as_deref()!=Some(&self.epoch){self.rejoin_transport.rediscover=true;return Err("Recovery service epoch changed".into())}
  if let Some(error)=&value.error{let delay=error.retry_after_ms.unwrap_or(0);if delay>60000{return Err("Invalid recovery server backoff".into());}if delay>0{let next=Instant::now()+Duration::from_millis(u64::from(delay));self.rejoin_transport.not_before=Some(self.rejoin_transport.not_before.map(|old|old.max(next)).unwrap_or(next));}let code=error.code.clone().unwrap_or("Recovery refused".into());if matches!(code.as_str(),"STALE_EPOCH"|"UNAUTHENTICATED")&&!self.health_active(){self.rejoin_transport.rediscover=true;}return Err(code)}
  if let Some(v)=value.credential_validity.as_ref(){
   let issued=if let Some(credentials)=value.credentials.as_ref(){let credentials:wire::Credentials=serde_json::from_value(serde_json::to_value(credentials).map_err(|_|"Invalid issued recovery credential")?).map_err(|_|"Invalid issued recovery credential")?;Session::read(credentials,&self.device.to_string(),"game")?}else{session};
   let v:nt2_desktop_g0::protocol_connection::wire::ConnectionCredential=serde_json::from_value(serde_json::to_value(v).map_err(|_|"Invalid recovery validity")?).map_err(|_|"Invalid recovery validity")?;
   admission::credential_sample(&mut self.lifecycle.lock().unwrap().credentials,&self.base,&self.service,&issued,&v,request.request_id.as_deref().unwrap(),stamp)?;
  }Ok(value)
 }
 pub(super) fn rejoin_intended_world(&self)->Option<String>{self.rejoin_transport.contact.as_ref()?;self.resume_intent.association().map(|a|a.world_token.clone())}
 pub(super) fn rejoin_authority_valid(&self,p:&handoff::Pending)->bool{
  let Some(contact)=self.rejoin_transport.contact.as_ref() else{return false};
  let Some(a)=self.resume_intent.association() else{return false};
  !matches!(self.resume_intent.phase(),local::Phase::Terminal|local::Phase::Offline)&&!self.native_revoked&&self.rejoin_transport.generation>0
   &&p.owner==Some((contact.pid,contact.context))&&p.run==a.run&&p.config==u64::from(a.config_revision)&&p.actual_seed==Some(contact.actual)
   &&p.generation.as_ref().is_some_and(|(f,g)|Arc::ptr_eq(f,&self.generation)&&f.load(Ordering::Acquire)==*g)
   &&self.game.as_ref().is_some_and(|g|g.owner==(contact.pid,contact.context)&&g.session.account==a.account)
 }
 pub(super) async fn capture_selected_association(&mut self)->Result<(),String>{
  if !self.game.as_ref().is_some_and(|g|g.entered){return Ok(())}
  if self.resume_intent.association().is_some_and(|a|self.game.as_ref().is_some_and(|g|a.world_token==hex(&g.owner.1))){return Ok(())}
  let mut request=self.rejoin_request(false)?;
  // Initial selected capture is scoped by the authenticated current desktop,
  // never by an older saved-world association imported from the local index.
  request.room_id=self.room.as_ref().and_then(|r|r.room_id.clone());
  request.run_id=None;request.participant_id=None;request.world_token=None;
  request.config_revision=None;request.generation=None;request.status=Some(resume::Empty{});
  let result=self.rejoin_rpc(&serde_json::to_vec(&request).map_err(|_|"Status encoding failed")?,false).await?;
  self.adopt_selected_association(result)
 }
 pub(super) fn adopt_selected_association(&mut self,mut result:resume::Response)->Result<(),String>{
  let participant=result.participant.as_ref().ok_or("Missing selected participant")?;
  let remote=participant.association.as_ref().ok_or("Missing selected association")?;
  let room=self.room.as_ref().ok_or("Missing selected room")?;
  let config=room.config.as_ref().ok_or("Missing selected config")?;use sha2::Digest;
  let actual=required(remote.actual_seed.clone())?;
  let a=local::Association{version:1,local_world:self.waiting_local_world().unwrap_or_else(uuid::Uuid::new_v4),origin:self.base.clone(),service:self.service.clone(),run:required(remote.run_id.clone())?,participant:required(remote.participant_id.clone())?,room:required(remote.room_id.clone())?,world_token:required(remote.world_token.clone())?,account:required(remote.account_id.clone())?,original_start:required(remote.original_start_id.clone())?,config_revision:remote.config_revision.ok_or("Missing saved revision")?,actual_seed:actual.clone(),seed_policy:required(remote.seed_policy.clone())?,loot_policy:if remote.seed_policy.as_deref()==Some("individual-random"){"individual:0".into()}else{format!("shared:{}:{actual}",u8::from(remote.randomize_loot==Some(true)))},frozen_config_digest:format!("{:x}",sha2::Sha256::digest(serde_json::to_vec(config).map_err(|_|"Config digest failed")?))};
  if !a.matches_canonical(remote)||participant.return_eligible!=Some(true)||participant.state.as_deref()!=Some("active"){return Err("Selected participant is not currently eligible".into())}
  let proof=zeroize::Zeroizing::new(result.resume_proof.take().ok_or("Missing selected resume proof")?);
  let record=self.remember_selected_world(a.clone(),protected::SecretBlob::new(proof.as_bytes().to_vec()).map_err(|_|"Invalid protected resume proof")?)?;
  let owner=self.game.as_ref().ok_or("Selected game changed")?.owner;
  let mut diag=self.native_diagnostics.lock().unwrap();diag.rejoin.waiting_offered=None;diag.rejoin.waiting_context=None;diag.rejoin.metadata=Some((owner.0,owner.1,record));diag.rejoin.offered=Some((owner.0,a));diag.rejoin.offered_context=Some(owner.1);diag.rejoin.lookup=2;diag.rejoin.terminal=false;diag.rejoin.offline=false;
  self.rejoin_transport.generation=participant.generation.ok_or("Missing selected ownership generation")?;
  let contact=crate::pipe::rejoin::Contact{pid:owner.0,context:owner.1,association:*diag.rejoin.offered.as_ref().unwrap().1.local_world.as_bytes(),actual:diag.rejoin.offered.as_ref().unwrap().1.actual_seed.parse().map_err(|_|"Invalid selected actual seed")?};
  diag.rejoin.contact=Some(contact.clone());self.rejoin_transport.contact=Some(contact);self.waiting=Default::default();Ok(())
 }
 pub(super) async fn rejoin_tick(&mut self)->Result<(),String>{
  tokio::time::timeout(Duration::from_secs(3),self.rejoin_tick_inner()).await.map_err(|_|"Whole recovery attempt deadline".to_string())?
 }
 async fn rejoin_tick_inner(&mut self)->Result<(),String>{
  self.park_foreign_recovery()?;
  if self.desktop.is_none(){return Ok(())}
  // Explicit fresh Join owns this lane until its original world enters. Late
  // old saved-world frames cannot replace its grant, including after arm.
  // Known Continue outside this fresh setup scope uses the normal path below.
  if self.fresh_setup_owns_world()&&!self.native_diagnostics.lock().unwrap().rejoin.offline{
   if let Some(reason)=self.fresh_join_recovery_blocker()?{return Err(reason.into())}
   let mut diag=self.native_diagnostics.lock().unwrap();
   if diag.rejoin.offered.is_some()||diag.rejoin.contact.is_some(){diag.rejoin=Default::default();}
   return Ok(())
  }
  if self.rejoin_transport.rediscover{self.rediscover_rejoin_issuer().await?;}
  let initial_world=self.rejoin_transport.contact.is_none()&&(self.preparation.is_some()||self.game.is_some());
  // A fresh preparation supersedes only an unchosen historical offer. Do not
  // erase selected recovery, terminal retirement or an uncertain operation.
  if initial_world&&self.resume_intent.phase()==local::Phase::Choice {
   self.resume_intent=local::ResumeIntent::default();
  }
  if !initial_world&&self.resume_intent.association().is_none(){
   if let Some(record)=self.store.load(&self.rejoin_index_profile()?).map_err(|_|"Saved association index unavailable")?{
    let a=local::Association::decode(record.bytes())?;
    if a.origin==self.base&&a.service==self.service&&self.desktop.as_ref().is_some_and(|d|d.account==a.account){
     if local::recall(&ResumeStore(self.store.clone()),&a).map_err(|e|e.to_string())?.is_some(){self.resume_intent.offer(a,true)?;}
    }
   }
  }
  // Initial selected world is captured by the real authenticated status path.
  if self.rejoin_transport.contact.is_none()&&!self.health_active(){self.capture_selected_association().await?;}
  let (offered,mut contact,offline,loaded_saved)={let mut diag=self.native_diagnostics.lock().unwrap();(diag.rejoin.offered.clone(),diag.rejoin.contact.clone(),diag.rejoin.offline,std::mem::take(&mut diag.rejoin.loaded_saved))};
  if let Some((_,a))=&offered{
   if a.origin!=self.base||a.service!=self.service||self.desktop.as_ref().is_none_or(|d|d.account!=a.account){self.native_diagnostics.lock().unwrap().rejoin.lookup=0;return Ok(())}
   if self.resume_intent.association()!=Some(a){
    if !self.select_parked_world(a)?{
     let proof=local::recall(&ResumeStore(self.store.clone()),a).map_err(|e|e.to_string())?;
     if proof.is_none(){self.native_diagnostics.lock().unwrap().rejoin.lookup=if local::is_retired(&ResumeStore(self.store.clone()),a).map_err(|e|e.to_string())?{5}else{0};return Ok(())}
     self.resume_intent.offer(a.clone(),true)?;
    }
   }
   if loaded_saved&&self.resume_intent.phase()==local::Phase::Playing {
    let proof=local::recall(&ResumeStore(self.store.clone()),a).map_err(|e|e.to_string())?;
    self.resume_intent.offer(a.clone(),proof.is_some())?;
    self.revoke_native();self.rejoin_transport.contact=None;self.rejoin_transport.observation=None;
    let mut diag=self.native_diagnostics.lock().unwrap();diag.rejoin.contact=None;diag.rejoin.admitted=None;contact=None;
   }
   self.native_diagnostics.lock().unwrap().rejoin.lookup=match self.resume_intent.phase(){local::Phase::Choice=>1,local::Phase::Terminal=>3,local::Phase::Offline=>5,_=>2};
  }
  if offline{self.continue_offline()?;}
  if self.resume_intent.phase()==local::Phase::Offline{return self.send_rejoin_offline().await}
  let Some(contact)=contact else{return Ok(())};
  if matches!(self.resume_intent.phase(),local::Phase::Terminal|local::Phase::Offline){self.native_diagnostics.lock().unwrap().rejoin.terminal=true;return Ok(())}
  if self.resume_intent.phase()==local::Phase::Choice{self.resume_intent.choose_rejoin()?;}
  let a=self.resume_intent.association().ok_or("Missing retained association")?.clone();
  if a.local_world.as_bytes()!=&contact.association||a.actual_seed!=contact.actual.to_string(){return Err("Current native world differs from saved association".into())}
  if self.health_active()&&self.rejoin_transport.contact.as_ref()==Some(&contact){
   self.reconcile_health_owner().await?;
   if self.resume_intent.phase()==local::Phase::Terminal{return Ok(());}
   if self.health_active(){return Ok(());}
  }
  if self.rejoin_transport.awaiting_fresh&&self.rejoin_transport.contact.as_ref()==Some(&contact){return Ok(())}
  if self.rejoin_transport.contact.as_ref()!=Some(&contact){
   {let mut coordinator=self.lifecycle.lock().unwrap();coordinator.saved(lifecycle::StableWorld::saved(&a));coordinator.retire_owner();}
   // A new native owner cancels the old health lane before any network await.
   self.health_nonce.fetch_add(1,Ordering::AcqRel);*self.health_updates.lock().unwrap()=None;
   self.g3_retire();self.game=None;self.preparation=None;self.native_revoked=false;
   self.resume_intent.transient_loss(Instant::now());self.rejoin_transport.contact=Some(contact.clone());self.rejoin_transport.awaiting_fresh=false;self.rejoin_transport.next_health=None;self.rejoin_transport.observation=None;
  }
  if self.game.is_none(){
   if self.rejoin_transport.not_before.is_some_and(|at|Instant::now()<at){return Err("Waiting for scoped recovery server backoff".into());}
   if self.desktop.as_ref().is_some_and(|d|self.session_expiring(d,60000)){self.rotate().await?;}
   if !self.resume_intent.retry_due(Instant::now()){return Ok(())}
   if self.resume_intent.original_operation().is_some()&&(self.rejoin_transport.restored_original||self.rejoin_transport.resume_scope.is_some()){
    if !self.reconcile_original_operation("resume").await?{return Err("Original resume outcome remains unresolved; request is safely parked".into());}
   }
   if self.resume_intent.original_operation().is_some(){
    let mut status=self.rejoin_request(false)?;status.status=Some(resume::Empty{});
    let response=self.rejoin_rpc(&serde_json::to_vec(&status).map_err(|_|"Status encoding failed")?,false).await?;
    let participant=response.participant.as_ref().ok_or("Missing current resume status")?;
    if !participant.association.as_ref().is_some_and(|v|a.matches_canonical(v)){return Err("Resume status association changed".into())}
    if participant.return_eligible!=Some(true){self.resume_intent.terminal();self.native_diagnostics.lock().unwrap().rejoin.terminal=true;return Ok(())}
    if participant.state.as_deref()==Some("reconnecting"){
     self.rejoin_transport.generation=participant.generation.ok_or("Missing current generation")?;
     self.resume_intent.reconcile_lost_generation()?;self.rejoin_transport.resume_scope=None;
    }
   }
   if self.resume_intent.original_operation().is_none(){
    let proof=local::recall(&ResumeStore(self.store.clone()),&a).map_err(|e|e.to_string())?.ok_or("Missing protected resume proof")?;
    let instance=uuid::Uuid::new_v4();let desktop=self.desktop.as_ref().unwrap();
    let encoded=zeroize::Zeroizing::new(local::resume_request(&a,&proof,&self.epoch,&desktop.epoch,&uuid::Uuid::new_v4().to_string(),instance)?);
    let mut request:resume::Request=serde_json::from_slice(&encoded).map_err(|_|"Resume encoding failed")?;request.generation=Some(self.rejoin_transport.generation);
    let bytes=serde_json::to_vec(&request).map_err(|_|"Resume encoding failed")?;
    if let Some(proof)=request.resume.as_mut().and_then(|r|r.resume_proof.as_mut()){proof.zeroize();}
    self.resume_intent.bind_operation(self.resume_intent.generation(),bytes)?;self.rejoin_transport.instance=Some(instance);
   }
   let original=zeroize::Zeroizing::new(self.resume_intent.original_operation().unwrap().to_vec());
   let mut result=match self.rejoin_rpc(&original,false).await{Ok(v)=>v,Err(error)=>{
    if error=="OWNER_ACTIVE"{self.resume_intent.reconcile_lost_generation()?;self.rejoin_transport.resume_scope=None;self.persist_recovery()?;}
    self.resume_intent.transient_loss(Instant::now());return Err(error)}};
   let req:resume::Request=serde_json::from_slice(&original).map_err(|_|"Invalid retained request")?;
   let desktop=self.desktop.as_ref().unwrap();let generation=local::validate_admission(&a,&result,&self.epoch,req.request_id.as_deref().unwrap(),&desktop.id,&self.device.to_string(),self.rejoin_transport.generation)?;
   let credentials:wire::Credentials=serde_json::from_value(serde_json::to_value(result.credentials.take().ok_or("Missing resumed credentials")?).map_err(|_|"Credential encoding failed")?).map_err(|_|"Invalid resumed credentials")?;
   let session=Session::read(credentials,&self.device.to_string(),"game")?;
   let participant=result.participant.ok_or("Missing resumed participant")?;
   let mut room_request=self.request(false);room_request.room_get=Some(wire::Empty{});room_request.room_id=Some(a.room.clone());
   let room_response=self.rpc(&room_request,false,false).await?;self.absorb(&room_response);
   self.native_revoked=false;self.rejoin_transport.generation=generation;
   self.game=Some(Game{session,instance:self.rejoin_transport.instance.ok_or("Missing original resume instance")?.to_string(),owner:(contact.pid,contact.context),reported:0,report_retry:None,renew_retry:None,permit:None,entered:false,offered:false});
   let fence=self.generation.fetch_add(1,Ordering::AcqRel)+1;
   let mut p=handoff::new(if a.seed_policy=="individual-random"{0}else{contact.actual.try_into().map_err(|_|"Shared seed overflow")?},a.run.clone(),u64::from(a.config_revision));
   p.epoch=required(participant.run_epoch)?;p.world_policy=if a.seed_policy=="individual-random"{2}else if a.loot_policy.starts_with("shared:1:"){5}else{1};p.actual_seed=Some(contact.actual);p.owner=Some((contact.pid,contact.context));p.accepted=true;p.native_v2=true;p.last_seen=Some(Instant::now());p.authority_until=Instant::now()+Duration::from_secs(6);p.generation=Some((self.generation.clone(),fence));
   *self.pending.lock().unwrap()=Some(p);self.native_diagnostics.lock().unwrap().rejoin.admitted=Some(contact);
   self.resume_intent.admitted(self.resume_intent.generation())?;
  }
  self.rejoin_playing_tick().await
 }
 async fn send_rejoin_offline(&mut self)->Result<(),String>{
  let Some(a)=self.resume_intent.association().cloned() else{return Ok(())};
  if self.rejoin_transport.offline.is_some(){
   if self.rejoin_transport.restored_original||self.rejoin_transport.offline_scope.as_ref().is_some_and(|s|self.desktop.as_ref().is_some_and(|d|!s.same_auth(&self.epoch,&d.id,&d.epoch))){
    if self.reconcile_original_operation("offline").await?{return Ok(());}
    return Err("Original offline outcome remains unresolved; request is safely parked".into());
   }
  }
  if self.rejoin_transport.offline.is_none()&&self.rejoin_transport.generation==0{
   let mut request=self.rejoin_request(false)?;request.status=Some(resume::Empty{});
   let status=self.rejoin_rpc(&serde_json::to_vec(&request).map_err(|_|"Offline status encoding failed")?,false).await?;
   let participant=status.participant.ok_or("Missing original offline participation")?;
   if !participant.association.as_ref().is_some_and(|v|a.matches_canonical(v)){return Err("Offline status differs from original world".into())}
   self.rejoin_transport.generation=participant.generation.filter(|g|*g>0).ok_or("Missing current offline generation")?;
   if participant.return_eligible==Some(false){local::forget(&ResumeStore(self.store.clone()),&a).map_err(|e|e.to_string())?;return Ok(())}
  }
  if self.rejoin_transport.offline.is_none(){
   let Some(proof)=local::retired_proof(&ResumeStore(self.store.clone()),&a).map_err(|e|e.to_string())? else{return Ok(())};
   let mut request=self.rejoin_request(false)?;request.offline=Some(resume::Offline{resume_proof:Some(std::str::from_utf8(proof.bytes()).map_err(|_|"Invalid offline proof")?.into())});
   self.rejoin_transport.offline=Some(zeroize::Zeroizing::new(serde_json::to_vec(&request).map_err(|_|"Offline encoding failed")?));
   use zeroize::Zeroize;if let Some(secret)=request.offline.as_mut().and_then(|v|v.resume_proof.as_mut()){secret.zeroize();}
  }
  let bytes=zeroize::Zeroizing::new(self.rejoin_transport.offline.as_ref().unwrap().to_vec());
  match self.rejoin_rpc(&bytes,false).await{
   Ok(response)=>{
    if response.participant.as_ref().is_none_or(|p|p.state.as_deref()!=Some("offline")||p.return_eligible!=Some(false)||!p.association.as_ref().is_some_and(|v|a.matches_canonical(v))){return Err("Offline retirement acknowledgement is not this participation".into())}
   }
   Err(error)=>{
   if error=="STALE_EPOCH"{return Err("Original offline operation parked: authenticated outcome reconciliation required".into())}
   if error!="RETURN_INELIGIBLE"{return Err(error)}
   // Existing canonical End/removal already forbids shared return. This is
   // terminal participation evidence, never a fabricated offline RPC receipt.
   let mut request=self.rejoin_request(false)?;request.status=Some(resume::Empty{});
   let status=self.rejoin_rpc(&serde_json::to_vec(&request).map_err(|_|"Terminal status encoding failed")?,false).await?;
   if status.participant.as_ref().is_none_or(|p|p.return_eligible!=Some(false)||!p.association.as_ref().is_some_and(|v|a.matches_canonical(v))){return Err("Offline retirement remains uncertain".into())}
  }}
  local::forget(&ResumeStore(self.store.clone()),&a).map_err(|e|e.to_string())?;self.rejoin_transport.offline=None;self.rejoin_transport.offline_scope=None;self.rejoin_transport.resume_uncertain=None;self.rejoin_transport.resume_scope=None;self.persist_recovery()?;Ok(())
 }
 pub(super) async fn rejoin_playing_tick(&mut self)->Result<(),String>{
  #[cfg(feature="g3-rejoin")]
  if self.health_enabled{self.publish_health();return Ok(());}
        #[cfg(all(feature="g3-rejoin",not(test)))]{return Err("Independent connection health worker is not running".into());}
  #[cfg(any(not(feature="g3-rejoin"),test))]{
  if self.rejoin_transport.next_health.is_some_and(|next|Instant::now()<next){return Ok(())}
  self.rejoin_transport.next_health=Some(Instant::now()+Duration::from_millis(500));
  if self.health_ticket.is_none()&&self.rejoin_transport.next_presence.is_none_or(|next|Instant::now()>=next){
   let mut presence=self.request(false);presence.presence=Some(wire::Presence{present:Some(true)});
   self.rpc(&presence,false,false).await?;self.rejoin_transport.next_presence=Some(Instant::now()+Duration::from_secs(2));
  }
  self.renew_game().await?;
  let local=self.pending.lock().unwrap().clone().ok_or("Recovering native contact disappeared")?;
  if !self.rejoin_authority_valid(&local){return Err("Recovering native authority changed".into())}
  if local.last_seen.is_none_or(|seen|seen.elapsed()>Duration::from_secs(12)){self.game=None;self.rejoin_transport.awaiting_fresh=true;self.resume_intent.transient_loss(Instant::now());return Ok(())}
  if !local.loaded{return Ok(())}
  let Some(report)=local.observation.as_ref() else{return Ok(())};
  if report.sequence>self.game.as_ref().unwrap().reported||self.rejoin_transport.observation.is_some(){
   if self.rejoin_transport.observation.is_none(){
    let game=self.game.as_ref().unwrap();let observation=&report.observation;let mut request=self.rejoin_request(true)?;
    request.observe=Some(resume::GameObservation{attachment_id:Some(game.session.attachment.clone()),game_instance_id:Some(game.instance.clone()),world_token:self.rejoin_intended_world(),observation_revision:Some(report.sequence.try_into().map_err(|_|"Observation generation exhausted")?),actual_seed:observation.actual_seed.clone(),active_mods:Some(observation.mods.clone()),inventory_complete:Some(observation.complete),beta:Some(match observation.beta{Some(true)=>"true",Some(false)=>"false",None=>"unknown"}.into()),protected:Some(observation.protected),loaded:Some(observation.loaded),seed_status:Some(if observation.actual_seed.is_some(){"known"}else{"unknown"}.into()),collector_failure:observation.error.clone(),mod_version:Some(observation.mod_version.clone()),bridge_version:Some(observation.bridge_version.clone()),compatibility_provenance:Some("game-thread-client-observation".into())});
    self.rejoin_transport.observation=Some(zeroize::Zeroizing::new(serde_json::to_vec(&request).map_err(|_|"Observation encoding failed")?));
   }
   let original=zeroize::Zeroizing::new(self.rejoin_transport.observation.as_ref().unwrap().to_vec());let submitted:resume::Request=serde_json::from_slice(&original).map_err(|_|"Invalid original observation")?;
   let response=self.rejoin_rpc(&original,true).await?;
   self.game.as_mut().unwrap().reported=u64::from(submitted.observe.unwrap().observation_revision.unwrap());self.rejoin_transport.observation=None;
   self.accept_rejoin_response(response,&local)?;
  }
  let mut request=self.rejoin_request(true)?;request.pulse=Some(resume::Empty{});
  let response=self.rejoin_rpc(&serde_json::to_vec(&request).map_err(|_|"Pulse encoding failed")?,true).await?;
  self.accept_rejoin_response(response,&local)?;self.g3_sync()?;Ok(())
  }
 }
 #[cfg(any(not(feature="g3-rejoin"),test))]
 fn accept_rejoin_response(&mut self,response:resume::Response,local:&handoff::Pending)->Result<(),String>{
  let a=self.resume_intent.association().ok_or("No retained association")?;
  let participant=response.participant.as_ref().ok_or("Missing current recovery participant")?;
  let game=self.game.as_ref().ok_or("No current resumed game")?;
  if !participant.association.as_ref().is_some_and(|v|a.matches_canonical(v))||participant.generation!=Some(self.rejoin_transport.generation)||participant.game_session_id.as_deref()!=Some(&game.session.id)||participant.attachment_id.as_deref()!=Some(&game.session.attachment)||participant.service_epoch.as_deref()!=Some(&self.epoch)||participant.run_epoch.as_deref()!=Some(&local.epoch){return Err("Recovery generation changed".into())}
  if participant.return_eligible!=Some(true){self.resume_intent.terminal();self.native_diagnostics.lock().unwrap().rejoin.terminal=true;return Ok(())}
  let release:Option<wire::GameRelease>=response.release.map(|r|serde_json::from_value(serde_json::to_value(r).unwrap()).map_err(|_|"Invalid recovery release")).transpose()?;
  let shared=self.pending.clone();let mut slot=shared.lock().unwrap();let p=slot.as_mut().ok_or("Recovery native owner disappeared")?;
  if p.owner!=local.owner||!self.rejoin_authority_valid(p){return Err("Recovery owner changed during request".into())}
  p.release=false;
  if let Some(r)=release{
   self.validate_rejoin_release(&r,p)?;let game=self.game.as_mut().unwrap();game.offered=true;
   if p.observation.as_ref().is_some_and(|v|!v.observation.protected){game.entered=true;}
   game.permit=Some(r.clone());self.g3_release=Some(r);p.release=true;
   if self.resume_intent.phase()==local::Phase::Inspecting{self.resume_intent.release(self.resume_intent.generation(),true)?;self.rejoin_transport.resume_scope=None;}
  }
  // Canonical presence is thirty seconds; gameplay gets only a short local lease.
  p.authority_until=Instant::now()+Duration::from_secs(6);Ok(())
 }
 #[cfg(any(not(feature="g3-rejoin"),test))]
 pub(super) fn validate_rejoin_release(&self,r:&wire::GameRelease,p:&handoff::Pending)->Result<(),String>{
  let a=self.resume_intent.association().ok_or("No saved association")?;let g=self.game.as_ref().ok_or("No recovery game")?;let d=self.desktop.as_ref().ok_or("No recovery desktop")?;let report=p.observation.as_ref().ok_or("No actual recovery inspection")?;
  if !self.rejoin_authority_valid(p)||r.contract_revision.as_deref()!=Some(super::protocol::REVISION)||r.service_id.as_deref()!=Some(&self.service)||r.service_epoch.as_deref()!=Some(&self.epoch)||r.room_id.as_deref()!=Some(&a.room)||r.run_id.as_deref()!=Some(&a.run)||r.run_epoch.as_deref()!=Some(&p.epoch)||r.config_revision!=Some(a.config_revision)||r.desktop_id.as_deref()!=Some(&d.id)||r.desktop_session_epoch.as_deref()!=Some(&d.epoch)||r.game_session_id.as_deref()!=Some(&g.session.id)||r.game_session_epoch.as_deref()!=Some(&g.session.epoch)||r.attachment_id.as_deref()!=Some(&g.session.attachment)||r.game_instance_id.as_deref()!=Some(&g.instance)||r.world_token.as_deref()!=Some(&a.world_token)||r.start_request_id.as_deref()!=Some(&a.original_start)||r.actual_seed.as_deref()!=Some(&a.actual_seed)||r.seed_policy.as_deref()!=Some(&a.seed_policy)||r.randomize_loot!=Some(a.loot_policy.starts_with("shared:1:"))||r.content_digest.as_deref()!=Some(&hex(&report.digest))||r.observation_revision.map(u64::from)!=Some(report.sequence)||report.sequence>g.reported||!report.observation.complete||!report.observation.loaded||!p.loaded||!p.accepted||r.phase.as_deref()!=Some(if report.observation.protected{"authorized"}else{"playing"})||timestamp(r.release_expires_at.as_deref().ok_or("Missing current lease")?)?<=now(){return Err("Release differs from current inspected recovery owner".into())}Ok(())
 }
}

#[derive(Serialize,Deserialize)]#[serde(deny_unknown_fields)]
struct DurableRecovery {
 version:u32,association:local::Association,offline_intent:bool,generation:u32,instance:Option<uuid::Uuid>,
 offline:Option<Vec<u8>>,resume:Option<Vec<u8>>,resume_uncertain:Option<Vec<u8>>,
 offline_scope:Option<lifecycle::OriginalScope>,resume_scope:Option<lifecycle::OriginalScope>,
}

#[cfg(test)]mod cr2_tests{
 use super::*;
 #[test]fn cr2_original_journal_core_restart_account_switch_and_unrelated_setup(){
  let(mut c,_,_)=super::super::tests::release_case();c.clear_game();c.base="https://journal-fixture.invalid".into();
  let a=local::Association{version:1,local_world:uuid::Uuid::new_v4(),origin:c.base.clone(),service:c.service.clone(),account:c.desktop.as_ref().unwrap().account.clone(),room:uuid::Uuid::new_v4().to_string(),run:uuid::Uuid::new_v4().to_string(),participant:uuid::Uuid::new_v4().to_string(),world_token:"1".repeat(32),original_start:uuid::Uuid::new_v4().to_string(),config_revision:1,actual_seed:"424242".into(),seed_policy:"shared-fixed".into(),loot_policy:"shared:0:424242".into(),frozen_config_digest:"a".repeat(64)};
  c.resume_intent.offer(a.clone(),true).unwrap();c.resume_intent.choose_rejoin().unwrap();c.resume_intent.world_loaded(&a).unwrap();
  let desktop=c.desktop.as_ref().unwrap();let id=uuid::Uuid::new_v4().to_string();let instance=uuid::Uuid::new_v4();let bytes=zeroize::Zeroizing::new(local::resume_request(&a,&protected::SecretBlob::new(vec![b'A';43]).unwrap(),&c.epoch,&desktop.epoch,&id,instance).unwrap());let mut request:resume::Request=serde_json::from_slice(&bytes).unwrap();request.generation=Some(1);let original=serde_json::to_vec(&request).unwrap();if let Some(p)=request.resume.as_mut().and_then(|r|r.resume_proof.as_mut()){p.zeroize();}
  c.rejoin_transport.resume_scope=Some(lifecycle::OriginalScope::capture(&a,&c.epoch,&desktop.id,&desktop.epoch,&id,&original));c.rejoin_transport.generation=1;c.rejoin_transport.instance=Some(instance);c.resume_intent.bind_operation(c.resume_intent.generation(),original.clone()).unwrap();c.persist_recovery().unwrap();
  let mut restarted=Core::new(c.store.as_ref().clone(),c.device,Arc::new(Mutex::new(None)),c.claims_root.clone());restarted.base=c.base.clone();restarted.service=c.service.clone();restarted.epoch=c.epoch.clone();restarted.desktop=c.desktop.clone();restarted.park_foreign_recovery().unwrap();assert_eq!(restarted.resume_intent.original_operation(),Some(original.as_slice()));assert!(restarted.rejoin_transport.restored_original);
  let scope=restarted.rejoin_transport.resume_scope.as_ref().unwrap();let old=restarted.desktop.clone().unwrap();let mut foreign=restarted.lifecycle_issuer().unwrap();foreign.origin="https://other.invalid".into();assert!(!scope.permits(&foreign,&c.epoch,&old.id,&old.epoch,&id,&original,&a));assert_eq!(scope.operation(&a,c.device,"resume",&original).unwrap().original_owner_generation,Some(1));
  restarted.park_for_fresh_room(&uuid::Uuid::new_v4().to_string()).unwrap();assert!(restarted.fresh_join_recovery_blocker().unwrap().is_none());assert_eq!(restarted.parked_recovery.len(),1);assert_eq!(restarted.parked_recovery[0].intent.original_operation(),Some(original.as_slice()));
  assert!(restarted.select_parked_world(&a).unwrap());assert_eq!(restarted.resume_intent.original_operation(),Some(original.as_slice()));assert!(restarted.parked_recovery.is_empty());restarted.park_for_fresh_room(&uuid::Uuid::new_v4().to_string()).unwrap();
  restarted.desktop.as_mut().unwrap().account=uuid::Uuid::new_v4().to_string();restarted.park_foreign_recovery().unwrap();assert!(restarted.resume_intent.association().is_none());
  restarted.desktop=Some(old);restarted.lifecycle.lock().unwrap().identity_changed();restarted.park_foreign_recovery().unwrap();assert_eq!(restarted.resume_intent.original_operation(),Some(original.as_slice()));
  // More than five transient reconnects retain the same immutable original.
  for _ in 0..32{let at=Instant::now();restarted.resume_intent.transient_loss(at);assert!(!restarted.resume_intent.retry_due(at));assert!(restarted.resume_intent.retry_due(at+Duration::from_secs(10)));assert_eq!(restarted.resume_intent.original_operation(),Some(original.as_slice()));}
  local::remember(&ResumeStore(c.store.clone()),&a,&protected::SecretBlob::new(vec![b'A';43]).unwrap()).unwrap();c.continue_offline().unwrap();let mut offline=c.rejoin_request(false).unwrap();offline.offline=Some(resume::Offline{resume_proof:Some("A".repeat(43))});let offline_bytes=serde_json::to_vec(&offline).unwrap();let desktop=c.desktop.as_ref().unwrap();c.rejoin_transport.offline_scope=Some(lifecycle::OriginalScope::capture(&a,&c.epoch,&desktop.id,&desktop.epoch,offline.request_id.as_deref().unwrap(),&offline_bytes));c.rejoin_transport.offline=Some(zeroize::Zeroizing::new(offline_bytes.clone()));c.persist_recovery().unwrap();
  let mut offline_restart=Core::new(c.store.as_ref().clone(),c.device,Arc::new(Mutex::new(None)),c.claims_root.clone());offline_restart.base=c.base.clone();offline_restart.service=c.service.clone();offline_restart.epoch=c.epoch.clone();offline_restart.desktop=c.desktop.clone();offline_restart.park_foreign_recovery().unwrap();assert_eq!(offline_restart.resume_intent.phase(),local::Phase::Offline);assert_eq!(offline_restart.rejoin_transport.offline.as_ref().unwrap().as_slice(),offline_bytes.as_slice());assert_eq!(offline_restart.rejoin_transport.resume_uncertain.as_ref().unwrap().as_slice(),original.as_slice());assert_eq!(offline_restart.rejoin_transport.offline_scope.as_ref().unwrap().operation(&a,c.device,"offline",&offline_bytes).unwrap().original_owner_generation,Some(1));
 }
}
impl Drop for DurableRecovery{fn drop(&mut self){use zeroize::Zeroize;for bytes in [&mut self.offline,&mut self.resume,&mut self.resume_uncertain].into_iter().flatten(){bytes.zeroize();}}}
impl<S:CredentialStore> Core<S>{
 fn parking_profile(&self,a:&local::Association)->Result<protected::ServiceProfile,String>{
  let digest=Sha256::digest(serde_json::to_vec(&("nt2-parked-recovery-v1",self.device,&a.account)).map_err(|_|"Parking scope failed")?);
  protected::ServiceProfile::new(&a.origin,&a.service,uuid::Uuid::from_bytes(digest[..16].try_into().unwrap())).map_err(|_|"Invalid parking issuer".into())
 }
 pub(super) fn persist_recovery(&self)->Result<(),String>{
  let Some(a)=self.resume_intent.association() else{return Ok(())};let profile=self.parking_profile(a)?;let mut records=vec![];
  for (intent,t) in std::iter::once((&self.resume_intent,&self.rejoin_transport)).chain(self.parked_recovery.iter().map(|p|(&p.intent,&p.transport))){
   let Some(world)=intent.association().filter(|w|w.origin==a.origin&&w.service==a.service&&w.account==a.account) else{continue};
   let queued_offline=intent.phase()==local::Phase::Offline&&local::retired_proof(&ResumeStore(self.store.clone()),world).map_err(|e|e.to_string())?.is_some();
   if !queued_offline&&t.offline.is_none()&&intent.original_operation().is_none()&&t.resume_uncertain.is_none(){continue;}
   let record=DurableRecovery{version:1,association:world.clone(),offline_intent:intent.phase()==local::Phase::Offline,generation:t.generation,instance:t.instance,offline:t.offline.as_ref().map(|b|b.to_vec()),resume:intent.original_operation().map(|b|b.to_vec()),resume_uncertain:t.resume_uncertain.as_ref().map(|b|b.to_vec()),offline_scope:t.offline_scope.clone(),resume_scope:t.resume_scope.clone()};
   records.push(zeroize::Zeroizing::new(serde_json::to_vec(&record).map_err(|_|"Parking encoding failed")?));
  }
  nt2_desktop_g0::connection_journal::save(&ResumeStore(self.store.clone()),&profile,&records).map_err(|e|e.to_string())
 }
 fn restore_parked_recovery(&mut self)->Result<(),String>{
  let Some(issuer)=self.lifecycle_issuer() else{return Ok(())};if self.parking_loaded.as_ref()==Some(&issuer){return Ok(())}
  let profile_a=local::Association{version:1,local_world:uuid::Uuid::new_v4(),origin:issuer.origin.clone(),service:issuer.service.clone(),account:issuer.account.clone(),run:String::new(),participant:String::new(),room:String::new(),world_token:String::new(),original_start:String::new(),config_revision:0,actual_seed:String::new(),seed_policy:String::new(),loot_policy:String::new(),frozen_config_digest:String::new()};
  let profile=self.parking_profile(&profile_a)?;let records=nt2_desktop_g0::connection_journal::load(&ResumeStore(self.store.clone()),&profile).map_err(|e|e.to_string())?;
  for bytes in records{
   let mut r:DurableRecovery=serde_json::from_slice(&bytes).map_err(|_|"Corrupt parked recovery")?;r.association.validate()?;
   if r.version!=1||r.association.origin!=issuer.origin||r.association.service!=issuer.service||r.association.account!=issuer.account{return Err("Parked recovery issuer/account mismatch".into());}
   if self.resume_intent.association()==Some(&r.association)||self.parked_recovery.iter().any(|p|p.intent.association()==Some(&r.association)){continue;}
   if self.parked_recovery.len()>=16{return Err("Parked recovery capacity reached".into());}
   let mut intent=local::ResumeIntent::default();intent.offer(r.association.clone(),true)?;
   if r.offline_intent{intent.offline();}else{intent.choose_rejoin()?;intent.world_loaded(&r.association)?;if let Some(bytes)=r.resume.take(){intent.bind_operation(intent.generation(),bytes)?;}}
   let transport=Transport{restored_original:true,generation:r.generation,instance:r.instance,offline:r.offline.take().map(zeroize::Zeroizing::new),resume_uncertain:r.resume_uncertain.take().map(zeroize::Zeroizing::new),offline_scope:r.offline_scope.take(),resume_scope:r.resume_scope.take(),..Default::default()};
   self.parked_recovery.push(Parked{intent,transport});
  }
  self.parking_loaded=Some(issuer);Ok(())
 }
}

impl<S:CredentialStore> Core<S>{
 async fn reconcile_original_operation(&mut self,kind:&str)->Result<bool,String>{
  use nt2_desktop_g0::protocol_connection::{self as cp,wire as c};
  if self.rejoin_transport.next_outcome.is_some_and(|at|Instant::now()<at){return Ok(false);}
  let a=self.resume_intent.association().ok_or("No parked world")?.clone();let desktop=self.desktop.as_ref().ok_or("Sign in to original account")?.clone();
  if a.origin!=self.base||a.service!=self.service||a.account!=desktop.account{return Err("Parked original belongs to another issuer/account".into());}
  let (bytes,scope)=if kind=="offline"{(self.rejoin_transport.offline.as_ref().map(|b|b.as_slice()),self.rejoin_transport.offline_scope.as_ref())}else{(self.resume_intent.original_operation(),self.rejoin_transport.resume_scope.as_ref())};
  let original=scope.ok_or("Missing protected original authentication scope")?.operation(&a,self.device,kind,bytes.ok_or("Missing protected original bytes")?)?;
  let request=c::Request{major:Some(1),revision:Some(cp::REVISION.into()),service_id:Some(self.service.clone()),service_epoch:Some(self.epoch.clone()),session_epoch:Some(desktop.epoch.clone()),request_id:Some(uuid::Uuid::new_v4().to_string()),outcome:Some(original.clone()),..Default::default()};
  let generation=self.generation.load(Ordering::Acquire);let url=format!("{}{}",self.base,cp::ENDPOINT);let body=zeroize::Zeroizing::new(serde_json::to_vec(&request).map_err(|_|"Outcome request encoding failed")?);
  let reply=tokio::time::timeout(Duration::from_secs(3),async{
   let mut response=self.http.post(&url).header("content-type","application/json").bearer_auth(desktop.access.as_str()).body(body.to_vec()).send().await.map_err(|_|"Outcome connection unavailable")?;if response.url().as_str()!=url||!response.status().is_success(){return Err("Outcome HTTP refused".to_string());}let mut bytes=zeroize::Zeroizing::new(Vec::new());while let Some(chunk)=response.chunk().await.map_err(|_|"Outcome response interrupted")?{if bytes.len()+chunk.len()>65536{return Err("Outcome response capacity".into());}bytes.extend_from_slice(&chunk);}cp::response(&bytes)
  }).await.map_err(|_|"Outcome whole-attempt deadline")??;
  if generation!=self.generation.load(Ordering::Acquire)||self.desktop.as_ref().is_none_or(|d|d.id!=desktop.id||d.epoch!=desktop.epoch||d.account!=a.account)||reply.request_id!=request.request_id||reply.service_id.as_deref()!=Some(&self.service)||reply.service_epoch.as_deref()!=Some(&self.epoch){return Err("Late outcome account/service result retired".into());}
  if let Some(error)=reply.error{if error.retry_after_ms.is_some_and(|v|v>60000){return Err("Invalid outcome backoff".into());}self.rejoin_transport.next_outcome=Some(Instant::now()+Duration::from_millis(u64::from(error.retry_after_ms.unwrap_or(2000).max(2000))));return Err(error.code.unwrap_or("Outcome refused".into()));}
  let outcome=reply.outcome.ok_or("Missing original outcome")?;if serde_json::to_vec(&outcome.original).ok()!=serde_json::to_vec(&Some(original)).ok()||outcome.retry_after_ms.is_some_and(|v|v>60000){return Err("Outcome differs from exact original identity".into());}
  self.rejoin_transport.next_outcome=Some(Instant::now()+Duration::from_millis(u64::from(outcome.retry_after_ms.unwrap_or(2000).max(2000))));
  self.last_original_outcome=Some(json!({"kind":kind,"original_outcome":outcome.original_outcome,"coverage":outcome.coverage,"current_disposition":outcome.current_disposition,"reason":outcome.current_reason}));
  let committed=outcome.original_outcome.as_deref()==Some("committed")&&outcome.coverage.as_deref()==Some("exact_durable_record")&&outcome.effect_id.as_ref().is_some_and(|s|!s.is_empty());
  let terminal=matches!(outcome.current_disposition.as_deref(),Some("terminal"|"offline"|"ended"|"removed"));
  if terminal{
   // Current disposition stops this world; it does not invent an old commit.
   local::retire_local(&ResumeStore(self.store.clone()),&a).map_err(|e|e.to_string())?;local::forget(&ResumeStore(self.store.clone()),&a).map_err(|e|e.to_string())?;
   self.resume_intent.terminal();self.rejoin_transport.offline=None;self.rejoin_transport.resume_uncertain=None;self.rejoin_transport.offline_scope=None;self.rejoin_transport.resume_scope=None;self.rejoin_transport.restored_original=false;self.native_diagnostics.lock().unwrap().rejoin.terminal=true;self.persist_recovery()?;return Ok(true);
  }
  if committed&&kind=="resume"{
   self.resume_intent.reconcile_lost_generation()?;self.rejoin_transport.resume_scope=None;self.rejoin_transport.restored_original=false;self.rejoin_transport.generation=outcome.current_generation.filter(|g|*g>0).ok_or("Missing current ownership generation")?;self.persist_recovery()?;
   // The normal new resume still meets OWNER_ACTIVE/lease and fresh inspection
   // fences. Sanitized status never supplies or resurrects old game credentials.
   return Ok(true);
  }
  Ok(false)
 }
}
