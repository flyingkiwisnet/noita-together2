//! The sole CR2 initial/recovered admission consumer. Native facts remain distinct
//! from canonical authorization; UTC fields are immutable history, not deadlines.
use super::*;
use nt2_desktop_g0::{protocol_connection::wire as c,connection_time::{Deadline,Stamp}};
#[derive(Clone)]pub(super) struct Context {
 pub issuer:lifecycle::Issuer,pub epoch:String,pub desktop:Session,pub game:Session,pub instance:String,
 pub room:String,pub run:String,pub run_epoch:String,pub config:u32,pub world:String,pub participant:Option<String>,
 pub ownership:Option<u32>,pub original_start:Option<String>,pub seed_policy:String,pub loot:bool,pub recovered:bool,
}
#[derive(Clone,Default)]pub(super) struct State {
 pub admission:Option<c::ConnectionAdmission>,pub authority:Deadline,pub review_until:Option<Instant>,pub entered:bool,
 immutable:Option<Vec<u8>>,offered_at:Option<Instant>,pub offered:bool,pub reason:Option<String>,pub terminal:bool,
}
#[derive(Clone)]pub(super) struct CredentialLease{pub deadline:Deadline,pub sample_key:String,pub sequence:u64,pub binding:String,pub origin_request:String}
fn bounded_key(v:Option<&str>)->bool{v.is_some_and(|v|v.len()==64&&v.bytes().all(|b|b.is_ascii_hexdigit()&&!b.is_ascii_uppercase()))}
pub(super) fn token_id(token:&str)->Result<String,String>{nt2_desktop_g0::protocol_connection::credential_id(token)}
pub(super) fn credential_key(origin:&str,service:&str,s:&Session)->Result<String,String>{Ok(json!([origin,service,s.account,s.id,s.epoch,token_id(s.access.as_str())?]).to_string())}
pub(super) fn credential_sample(leases:&mut std::collections::BTreeMap<String,CredentialLease>,origin:&str,service:&str,s:&Session,v:&c::ConnectionCredential,request:&str,stamp:Stamp)->Result<Instant,String>{
 let id=token_id(s.access.as_str())?;
 if v.principal_id.as_deref()!=Some(&s.account)||v.session_id.as_deref()!=Some(&s.id)||v.session_epoch.as_deref()!=Some(&s.epoch)||v.credential_id.as_deref()!=Some(&id)||v.origin_request_id.as_deref()!=Some(request)||!bounded_key(v.sample_key.as_deref())||v.access_remaining_ms.is_none_or(|v|v>900000){return Err("Credential validity differs from returned/authenticated credential".into());}
 let key=credential_key(origin,service,s)?;let binding=json!([origin,service,s.account,s.id,s.epoch]).to_string();
 if leases.values().any(|l|l.binding==binding&&l.sequence>stamp.sequence){return Err("Late credential sample retired".into());}
 let original_until=leases.values().filter(|l|l.binding==binding&&l.origin_request==request).filter_map(|l|l.deadline.until()).min();
 if leases.len()>=64&&!leases.contains_key(&key){if let Some(old)=leases.iter().min_by_key(|(_,v)|v.sequence).map(|(k,_)|k.clone()){leases.remove(&old);}}
 let lease=leases.entry(key).or_insert(CredentialLease{deadline:Deadline::default(),sample_key:String::new(),sequence:0,binding:binding.clone(),origin_request:request.into()});
 if lease.sample_key==v.sample_key.as_deref().unwrap_or("")&&lease.deadline.until().is_some(){return Ok(lease.deadline.until().unwrap());}
 let old_until=lease.deadline.until();let until=lease.deadline.install(stamp,v.access_remaining_ms.ok_or("Missing credential duration")?,900000,900000,Instant::now())?;if let Some(old)=old_until.into_iter().chain(original_until).min(){lease.deadline.shorten(old);}lease.sequence=stamp.sequence;lease.origin_request=request.into();lease.sample_key=required(v.sample_key.clone())?;Ok(lease.deadline.until().unwrap_or(until))
}
#[cfg(test)]mod lc1_tests{use super::*;#[test]fn lc1_higher_canonical_owner_requires_stable_world_start_and_new_validated_lease(){
 let old:c::ConnectionAdmission=serde_json::from_value(json!({"generation":1,"serviceId":"service","serviceEpoch":"epoch","participantId":"participant","scope":{"principalId":"account","roomId":"room","runId":"run","runEpoch":"run-epoch","configRevision":1,"worldToken":"world","sessionId":"old-game"},"originalStart":{"requestId":"original-start"}})).unwrap();let mut state=State{admission:Some(old.clone()),immutable:Some(vec![1]),entered:true,offered:true,..Default::default()};assert_eq!(state.for_generation(&old).unwrap().immutable,Some(vec![1]),"Same generation keeps immutable freeze");let mut next=old.clone();next.generation=Some(2);next.scope.as_mut().unwrap().session_id=Some("new-game".into());let candidate=state.for_generation(&next).unwrap();assert!(candidate.admission.is_none()&&candidate.immutable.is_none()&&!candidate.entered&&!candidate.offered&&!candidate.authority.live(Instant::now()),"New generation alone grants no authority");let mut changed=next.clone();changed.original_start.as_mut().unwrap().request_id=Some("other-start".into());assert!(state.for_generation(&changed).is_err());changed=next.clone();changed.scope.as_mut().unwrap().world_token=Some("other-world".into());assert!(state.for_generation(&changed).is_err());changed=next.clone();changed.service_epoch=Some("other-service-epoch".into());assert!(state.for_generation(&changed).is_err());changed=next.clone();changed.generation=Some(0);assert!(state.for_generation(&changed).is_err());state.terminal=true;assert!(state.for_generation(&next).is_err());
}}
impl State{
 // LC1 may obtain a newer canonical owner while retaining the same local
 // native conversation. Validate its new admission from an empty lease, while
 // preserving stable world/participant/original-Start identity across owners.
 pub(super) fn for_generation(&self,next:&c::ConnectionAdmission)->Result<Self,String>{
  if self.terminal{return Err("Canonical participation already terminal for this owner".into())}let Some(old)=&self.admission else{return Ok(self.clone())};let previous=old.generation.ok_or("Previous canonical generation absent")?;let current=next.generation.ok_or("Current canonical generation absent")?;if current<previous{return Err("Canonical owner generation regressed".into())}if current==previous{return Ok(self.clone())}
  let stable=|a:&c::ConnectionAdmission|json!([a.service_id,a.service_epoch,a.participant_id,a.scope.as_ref().map(|s|(&s.principal_id,&s.room_id,&s.run_id,&s.run_epoch,s.config_revision,&s.world_token)),a.original_start]);if stable(old)!=stable(next){return Err("Canonical owner changed stable world or original Start".into())}Ok(Self::default())
 }
 pub fn native_entry(&mut self,p:&handoff::Pending){
  if self.offered&&p.release&&self.authority.live(Instant::now())&&p.observation.as_ref().is_some_and(|r|self.offered_at.is_some_and(|at|r.observed_at>=at)&&self.admission.as_ref().and_then(|a|a.current_report.as_ref()).is_some_and(|report|report.content_digest.as_deref()==Some(&hex(&r.digest)))&&r.context==p.owner.map(|v|v.1).unwrap_or_default()&&!r.observation.protected&&r.observation.complete&&r.observation.loaded&&p.loaded&&p.accepted&&p.matches_seed(r.observation.actual_seed.as_deref())){self.entered=true;}
 }
 pub fn install(&mut self,a:c::ConnectionAdmission,ctx:&Context,p:&handoff::Pending,request:&str,credential:&str,stamp:Stamp,credential_until:Instant)->Result<Instant,String>{
  nt2_desktop_g0::protocol_connection::validate("ConnectionAdmission",&a)?;
  if self.terminal{return Err("Canonical participation already terminal for this owner".into());}
  let scope=a.scope.as_ref().ok_or("Missing admitted world scope")?;let owner=a.owner.as_ref().ok_or("Missing admitted owner")?;let sample=a.sample.as_ref().ok_or("Missing admission sample")?;let generation=a.generation.filter(|v|*v>0).ok_or("Missing owner generation")?;
  if a.service_id.as_deref()!=Some(&ctx.issuer.service)||a.service_epoch.as_deref()!=Some(&ctx.epoch)||scope.principal_id.as_deref()!=Some(&ctx.game.account)||scope.session_id.as_deref()!=Some(&ctx.game.id)||scope.session_epoch.as_deref()!=Some(&ctx.game.epoch)||scope.room_id.as_deref()!=Some(&ctx.room)||scope.run_id.as_deref()!=Some(&ctx.run)||scope.run_epoch.as_deref()!=Some(&ctx.run_epoch)||scope.config_revision!=Some(ctx.config)||scope.world_token.as_deref()!=Some(&ctx.world)||scope.attachment_id.as_deref()!=Some(&ctx.game.attachment)
   ||owner.desktop_id.as_deref()!=Some(&ctx.desktop.id)||owner.desktop_session_epoch.as_deref()!=Some(&ctx.desktop.epoch)||owner.game_session_id.as_deref()!=Some(&ctx.game.id)||owner.game_session_epoch.as_deref()!=Some(&ctx.game.epoch)||owner.attachment_id.as_deref()!=Some(&ctx.game.attachment)||owner.game_instance_id.as_deref()!=Some(&ctx.instance)||ctx.game.account!=ctx.issuer.account
   ||ctx.participant.as_ref().is_some_and(|id|a.participant_id.as_ref()!=Some(id))||ctx.ownership.is_some_and(|g|g!=generation)||a.participant_id.as_ref().is_none_or(String::is_empty)
   ||sample.channel.as_deref()!=Some("http")||sample.origin_request_id.as_deref()!=Some(request)||sample.credential_id.as_deref()!=Some(credential)||!bounded_key(sample.sample_key.as_deref())
   ||p.run!=ctx.run||p.epoch!=ctx.run_epoch||p.config!=u64::from(ctx.config)||p.owner.is_none()||!p.accepted||!p.loaded||p.last_seen.is_none_or(|v|v.elapsed()>Duration::from_secs(12)){return Err("Admission differs from current world/owner/request".into());}
  let decision=a.decision.as_ref().ok_or("Missing admission decision")?;
  if decision.remaining_ms.is_none_or(|v|v>30000)||decision.initial_review_remaining_ms.is_none_or(|v|v>3600000){return Err("Invalid canonical duration bounds".into());}
  self.reason=decision.reason.clone();
  if decision.terminal==Some(true){self.terminal=true;self.offered=false;self.authority.shorten(Instant::now());self.admission=Some(a);return Err("Canonical participation terminal".into());}
  if decision.authorized!=Some(true){self.offered=false;self.admission=Some(a);return Err("Canonical inspection/authorization held".into());}
  let report=p.observation.as_ref().ok_or("No actual native inspection")?;let current=a.current_report.as_ref().ok_or("Missing current canonical report")?;
  timestamp(current.captured_at.as_deref().ok_or("Missing current inspection capture")?)?;
  if self.admission.as_ref().and_then(|a|a.current_report.as_ref()).is_some_and(|old|old.revision==current.revision&&old.captured_at!=current.captured_at){return Err("Current inspection capture changed without a new revision".into());}
  if report.context!=p.owner.unwrap().1||!report.observation.complete||!report.observation.loaded||!p.matches_seed(report.observation.actual_seed.as_deref())||current.complete!=Some(true)||current.loaded!=Some(true)||current.content_valid!=Some(true)||current.revision.map(u64::from)!=Some(report.sequence)||current.content_digest.as_deref()!=Some(&hex(&report.digest))||current.protected!=Some(report.observation.protected){return Err("Current native report awaiting matching canonical inspection".into());}
  let original=a.original_start.as_ref().ok_or("Missing original Start")?;let immutable=a.immutable.as_ref().ok_or("Missing immutable admission")?;let selected=immutable.selected_inspection.as_ref().ok_or("Missing selected inspection")?;let original_selected=original.selected_inspection.as_ref().ok_or("Missing original selection")?;
  if original.request_id.as_ref().is_none_or(String::is_empty)||ctx.original_start.as_ref().is_some_and(|id|original.request_id.as_ref()!=Some(id))||immutable.authorization_id.as_ref().is_none_or(String::is_empty)||immutable.start_request_id.as_ref().is_none_or(String::is_empty)
   ||selected.world_token.as_deref()!=Some(&ctx.world)||selected.desktop_id.as_deref()!=Some(&ctx.desktop.id)||selected.game_session_id.as_deref()!=Some(&ctx.game.id)||selected.game_session_epoch.as_deref()!=Some(&ctx.game.epoch)||selected.desktop_session_epoch.as_deref()!=Some(&ctx.desktop.epoch)||selected.attachment_id.as_deref()!=Some(&ctx.game.attachment)||selected.game_instance_id.as_deref()!=Some(&ctx.instance)
   ||selected.content_digest.as_deref()!=Some(&hex(&report.digest))||selected.observation_revision.is_none_or(|v|u64::from(v)<p.invalidated_observation||u64::from(v)>report.sequence)||selected.actual_seed!=report.observation.actual_seed||selected.seed_policy.as_deref()!=Some(&ctx.seed_policy)||selected.randomize_loot!=Some(ctx.loot)
   ||original_selected.world_token.as_deref()!=Some(&ctx.world)||original_selected.actual_seed!=report.observation.actual_seed||original_selected.content_digest.as_deref()!=Some(&hex(&report.digest)) {return Err("Immutable admission/Start differs from inspected world".into());}
  // UTC is checked only for internal historical ordering, never versus client now.
  if timestamp(selected.captured_at.as_deref().ok_or("Missing selected capture")?)?>timestamp(immutable.authorized_at.as_deref().ok_or("Missing authorization history")?)?{return Err("Invalid immutable inspection history".into());}
  let frozen=serde_json::to_vec(immutable).map_err(|_|"Immutable encoding failed")?;if self.immutable.as_ref().is_some_and(|v|v!=&frozen){return Err("Immutable admission changed within owner generation".into());}
  self.native_entry(p);
  let mut until=self.authority.install(stamp,decision.remaining_ms.unwrap(),30000,12000,Instant::now())?.min(credential_until);
  if generation==1&&!self.entered{let review=stamp.started.checked_add(Duration::from_millis(u64::from(decision.initial_review_remaining_ms.unwrap()))).ok_or("Review deadline overflow")?;self.review_until=Some(self.review_until.map(|old|old.min(review)).unwrap_or(review));until=until.min(self.review_until.unwrap());}
  self.authority.shorten(until);
  if until<=Instant::now(){return Err("Canonical authority duration exhausted".into());}
  self.immutable=Some(frozen);if !self.offered{self.offered_at=Some(Instant::now());}self.offered=true;self.terminal=false;self.admission=Some(a);Ok(until)
 }
}
impl<S:CredentialStore> Core<S>{
 pub(super) fn connection_context(&self)->Result<Context,String>{
  if self.lc1{return self.world.ctx.clone().ok_or("Current lifecycle connection absent".into())}
  let game=self.game.as_ref().ok_or("No current game")?;let desktop=self.desktop.as_ref().ok_or("No desktop")?;let p=self.pending.lock().unwrap().clone().ok_or("No current native world")?;let room=self.room.as_ref().ok_or("No current room")?;let policy=seed_policy::resolved(room.config.as_ref().ok_or("No frozen config")?)?;
  let a=self.resume_intent.association().filter(|a|a.origin==self.base&&a.service==self.service&&a.account==desktop.account&&a.run==p.run&&self.rejoin_transport.contact.is_some());
  Ok(Context{issuer:self.lifecycle_issuer().ok_or("No issuer")?,epoch:self.epoch.clone(),desktop:desktop.clone(),game:game.session.clone(),instance:game.instance.clone(),room:required(room.room_id.clone())?,run:p.run,run_epoch:p.epoch,config:p.config.try_into().map_err(|_|"Invalid configuration")?,world:self.waiting_world().map(String::from).or_else(||a.map(|a|a.world_token.clone())).unwrap_or_else(||hex(&game.owner.1)),participant:a.map(|a|a.participant.clone()),ownership:if self.rejoin_transport.generation>0{Some(self.rejoin_transport.generation)}else{None},original_start:a.map(|a|a.original_start.clone()),seed_policy:policy.name,loot:policy.loot,recovered:self.rejoin_transport.contact.is_some()})
 }
 pub(super) fn session_expiring(&self,s:&Session,within_ms:u64)->bool{
  #[cfg(feature="g3-rejoin")]{let Ok(key)=credential_key(&self.base,&self.service,s)else{return true};return self.lifecycle.lock().unwrap().credentials.get(&key).is_none_or(|l|l.deadline.remaining(Instant::now())<=Duration::from_millis(within_ms));}
  #[cfg(not(feature="g3-rejoin"))]{s.expires<=now().saturating_add(within_ms)}
 }
}

#[cfg(test)]pub(super) mod cr2_tests{
 use super::*;
 pub(crate) fn fixture()->(State,Context,handoff::Pending,c::ConnectionAdmission,Stamp){
  let v:Value=serde_json::from_str(include_str!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../packages/contracts/g3-connection/golden.json"))).unwrap();
  let a:c::ConnectionAdmission=serde_json::from_value(v["fixtures"].as_array().unwrap().iter().find(|v|v["type"]=="ConnectionAdmission").unwrap()["json"].clone()).unwrap();let s=a.scope.as_ref().unwrap();let o=a.owner.as_ref().unwrap();let selected=a.immutable.as_ref().unwrap().selected_inspection.as_ref().unwrap();
  let session=Session{access:SecretString::from("unused".to_string()),refresh:SecretString::from(String::new()),id:required(s.session_id.clone()).unwrap(),epoch:required(s.session_epoch.clone()).unwrap(),account:required(s.principal_id.clone()).unwrap(),attachment:required(s.attachment_id.clone()).unwrap(),principal:wire::Principal::default(),expires:0};let mut desktop=session.clone();desktop.id=required(o.desktop_id.clone()).unwrap();desktop.epoch=required(o.desktop_session_epoch.clone()).unwrap();
  let ctx=Context{issuer:lifecycle::Issuer{origin:"https://fixture.invalid".into(),service:required(a.service_id.clone()).unwrap(),account:session.account.clone()},epoch:required(a.service_epoch.clone()).unwrap(),desktop,game:session,instance:required(o.game_instance_id.clone()).unwrap(),room:required(s.room_id.clone()).unwrap(),run:required(s.run_id.clone()).unwrap(),run_epoch:required(s.run_epoch.clone()).unwrap(),config:s.config_revision.unwrap(),world:required(s.world_token.clone()).unwrap(),participant:a.participant_id.clone(),ownership:a.generation,original_start:a.original_start.as_ref().unwrap().request_id.clone(),seed_policy:required(selected.seed_policy.clone()).unwrap(),loot:selected.randomize_loot.unwrap(),recovered:false};
  let mut p=handoff::new(selected.actual_seed.as_ref().unwrap().parse().unwrap(),ctx.run.clone(),u64::from(ctx.config));p.epoch=ctx.run_epoch.clone();p.owner=Some((nt2_local_ws::owner::Id::OsProcess(77),[5;16]));p.loaded=true;p.accepted=true;p.last_seen=Some(Instant::now());let digest=selected.content_digest.as_ref().unwrap().as_bytes().chunks_exact(2).map(|b|u8::from_str_radix(std::str::from_utf8(b).unwrap(),16).unwrap()).collect::<Vec<_>>().try_into().unwrap();
  p.observation=Some(native_report::Latest{context:[5;16],sequence:u64::from(a.current_report.as_ref().unwrap().revision.unwrap()),digest,observed_at:Instant::now(),observation:native_report::Observation{actual_seed:selected.actual_seed.clone(),mods:vec![],complete:true,error:None,beta:Some(false),loaded:true,protected:true,mod_version:"fixture".into(),bridge_version:"fixture".into(),..Default::default()}});
  (State::default(),ctx,p,a,Stamp{sequence:1,started:Instant::now()-Duration::from_millis(300)})
 }
 fn install(state:&mut State,ctx:&Context,p:&handoff::Pending,a:c::ConnectionAdmission,stamp:Stamp)->Result<Instant,String>{let sample=a.sample.as_ref().unwrap();let request=sample.origin_request_id.clone().unwrap();let credential=sample.credential_id.clone().unwrap();state.install(a,ctx,p,&request,&credential,stamp,Instant::now()+Duration::from_secs(900))}
 #[test]fn cr2_shared_admission_post_start_same_content_actual_entry_and_immutable_fence(){
  let(mut state,ctx,mut p,a,stamp)=fixture();let until=install(&mut state,&ctx,&p,a.clone(),stamp).unwrap();assert_eq!(until,stamp.started+Duration::from_secs(12));assert!(!state.entered);assert!(state.offered);
  let mut later=a.clone();p.release=true;p.observation.as_mut().unwrap().sequence+=1;p.observation.as_mut().unwrap().observation.protected=false;p.observation.as_mut().unwrap().observed_at=Instant::now();state.native_entry(&p);assert!(state.entered);
  later.current_report.as_mut().unwrap().revision=Some(p.observation.as_ref().unwrap().sequence as u32);later.current_report.as_mut().unwrap().protected=Some(false);later.current_report.as_mut().unwrap().captured_at=Some("2099-01-01T00:00:00Z".into());
  let next=Stamp{sequence:2,started:Instant::now()};install(&mut state,&ctx,&p,later.clone(),next).unwrap();
  later.immutable.as_mut().unwrap().authorization_id=Some(uuid::Uuid::new_v4().to_string());assert!(install(&mut state,&ctx,&p,later,Stamp{sequence:3,started:Instant::now()}).unwrap_err().contains("Immutable admission changed"));
 }
 #[test]fn cr2_history_clock_offsets_duplicate_delayed_wrong_scope_terminal_and_resumed_review(){
  for year in ["1970","2099"]{let(mut state,ctx,p,mut a,stamp)=fixture();a.immutable.as_mut().unwrap().authorized_at=Some(format!("{year}-01-02T00:00:00Z"));a.immutable.as_mut().unwrap().selected_inspection.as_mut().unwrap().captured_at=Some(format!("{year}-01-01T00:00:00Z"));let end=install(&mut state,&ctx,&p,a.clone(),stamp).unwrap();assert_eq!(install(&mut state,&ctx,&p,a.clone(),stamp).unwrap(),end);
   let mut wrong=a.clone();wrong.owner.as_mut().unwrap().game_session_epoch=Some(uuid::Uuid::new_v4().to_string());assert!(install(&mut state,&ctx,&p,wrong,stamp).is_err());
   let mut delayed=state.clone();assert!(install(&mut delayed,&ctx,&p,a.clone(),Stamp{sequence:2,started:Instant::now()-Duration::from_secs(13)}).is_err());
   let mut resumed=State::default();let mut resumed_ctx=ctx.clone();resumed_ctx.ownership=Some(2);resumed_ctx.recovered=true;let mut r=a.clone();r.generation=Some(2);r.decision.as_mut().unwrap().initial_review_remaining_ms=Some(0);assert!(install(&mut resumed,&resumed_ctx,&p,r,stamp).is_ok());
   a.decision.as_mut().unwrap().terminal=Some(true);assert!(install(&mut state,&ctx,&p,a.clone(),stamp).is_err());a.decision.as_mut().unwrap().terminal=Some(false);assert!(install(&mut state,&ctx,&p,a,stamp).is_err());
  }
 }
 #[test]fn cr2_changed_token_duplicate_request_never_extends_credential(){
  use base64::Engine;let(_,ctx,_,_,stamp)=fixture();let mut session=ctx.game;let mut leases=std::collections::BTreeMap::new();let mut end=None;
  for (n,jti) in ["first","second"].into_iter().enumerate(){session.access=SecretString::from(format!("e30.{}.x",base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(json!({"jti":jti}).to_string())));let v=c::ConnectionCredential{principal_id:Some(session.account.clone()),session_id:Some(session.id.clone()),session_epoch:Some(session.epoch.clone()),credential_id:Some(jti.into()),origin_request_id:Some("same-original".into()),sample_key:Some(format!("{:064x}",n+1)),access_remaining_ms:Some(if n==0{5000}else{900000})};let until=credential_sample(&mut leases,"https://fixture.invalid","service",&session,&v,"same-original",stamp).unwrap();if let Some(end)=end{assert_eq!(until,end);}end=Some(until);}
 }
 #[cfg(feature="g3-rejoin")]
 #[test]fn cr2_preparation_remaining_uses_verified_service_anchor_at_large_clock_offsets(){
  for year in ["1970","2099"]{let(core,_,_)=super::super::tests::release_case();let mut prep=core.preparation.clone().unwrap();prep.expires_at=Some(format!("{year}-01-01T00:10:00Z"));prep.review_expires_at=Some(format!("{year}-01-01T00:30:00Z"));let stamp=Stamp{sequence:1,started:Instant::now()-Duration::from_secs(3)};
   core.lifecycle.lock().unwrap().service_time=Some(ServiceTime{scope:json!([core.base,core.service,core.epoch,core.desktop.as_ref().unwrap().account]).to_string(),stamp,upper_ms:timestamp(&format!("{year}-01-01T00:00:00Z")).unwrap()});let remaining=core.preparation_remaining(&prep,false).unwrap();assert!((596000..=597000).contains(&remaining));let review=core.preparation_remaining(&prep,true).unwrap();assert!((1796000..=1797000).contains(&review));
   // A later sample cannot lengthen the fixed review for the same preparation.
   core.lifecycle.lock().unwrap().service_time.as_mut().unwrap().upper_ms=0;assert!(core.preparation_remaining(&prep,true).unwrap()<=review);
  }
 }
}

impl<S:CredentialStore> Core<S>{
 pub(super) fn record_credential_reply(&self,r:&wire::Response,request:&str,stamp:Stamp,game:bool,override_token:Option<&SecretString>)->Result<(),String>{
  if self.untimed_refresh.as_deref()==Some(request){return Ok(());}
  let Some(v)=r.credential_validity.as_ref()else{return Ok(())};
  let mut session=if let Some(credentials)=&r.credentials{Session::read(credentials.clone(),&self.device.to_string(),credentials.kind.as_deref().ok_or("Missing credential kind")?)?}else{if game{self.game.as_ref().map(|g|g.session.clone())}else{self.desktop.clone()}.ok_or("Missing authenticated credential")?};
  if let Some(token)=override_token{if r.credentials.is_none(){
   use base64::Engine;let bytes=zeroize::Zeroizing::new(base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(token.as_str().split('.').nth(1).ok_or("Missing override credential identity")?).map_err(|_|"Invalid override identity")?);let claims:Value=serde_json::from_slice(&bytes).map_err(|_|"Invalid override identity")?;
   if claims["sub"].as_str()!=Some(&session.account){return Err("Override credential account changed".into());}
   session.id=claims["sid"].as_str().ok_or("Missing override session")?.into();session.epoch=claims["sessionEpoch"].as_str().ok_or("Missing override epoch")?.into();session.access=token.clone();session.expires=claims["exp"].as_u64().ok_or("Missing override expiry history")?.checked_mul(1000).ok_or("Invalid override expiry")?;
  }}
  let v:c::ConnectionCredential=serde_json::from_value(serde_json::to_value(v).map_err(|_|"Credential conversion failed")?).map_err(|_|"Credential conversion failed")?;
  let mut coordinator=self.lifecycle.lock().unwrap();
  let until=credential_sample(&mut coordinator.credentials,&self.base,&self.service,&session,&v,request,stamp)?;
  if until>Instant::now()&&coordinator.service_time.as_ref().is_none_or(|old|old.stamp.sequence<stamp.sequence){
   coordinator.service_time=Some(ServiceTime{scope:json!([self.base,self.service,self.epoch,session.account]).to_string(),stamp,upper_ms:session.expires.saturating_sub(u64::from(v.access_remaining_ms.unwrap()))});
  }Ok(())
 }
 pub(super) fn validate_current_admission(&self,p:&handoff::Pending)->Result<(),String>{
  let c=self.lifecycle.lock().unwrap();let a=&c.authority;
  if !a.offered||a.terminal||!a.authority.live(Instant::now())||p.authority_until<=Instant::now()||!p.release{return Err("Current canonical authority unavailable".into());}
  let admitted=a.admission.as_ref().ok_or("No canonical admission")?;let report=p.observation.as_ref().ok_or("No current report")?;let current=admitted.current_report.as_ref().ok_or("No current canonical report")?;
  if !report.observation.complete||!report.observation.loaded||current.content_digest.as_deref()!=Some(&hex(&report.digest))||!p.matches_seed(report.observation.actual_seed.as_deref()){return Err("Current content differs from admission".into());}Ok(())
 }
 #[cfg(feature="g3-rejoin")]
 pub(super) fn lc1_live_movement_ready(&self,p:&handoff::Pending)->bool{self.lc1_projection_ready(p,true)}
 #[cfg(feature="g3-rejoin")]
 pub(super) fn lc1_c1_metadata_ready(&self,p:&handoff::Pending)->bool{self.lc1_projection_ready(p,false)}
 #[cfg(feature="g3-rejoin")]
 pub(super) fn lc1_c1_completion_ready(&self,p:&handoff::Pending)->bool{self.lc1_c1_metadata_ready(p)&&self.game.as_ref().is_some_and(|g|g.entered)&&self.lifecycle.lock().unwrap().authority.entered}
 #[cfg(feature="g3-rejoin")]
 fn lc1_projection_ready(&self,p:&handoff::Pending,gameplay:bool)->bool{
  // A protected canonical ACK can release local Controls. Movement waits for
  // the later Server-entered/unprotected ACK, retaining its original lease.
  if self.validate_current_admission(p).is_err(){return false}
  let (Some(game),Some(desktop),Some(record),Some(report))=(self.game.as_ref(),self.desktop.as_ref(),self.world.record.as_ref(),p.observation.as_ref())else{return false};
  if gameplay&&(!game.entered||report.observation.protected)||p.owner!=Some(game.owner)||report.context!=game.owner.1||p.run!=record.identity.run||p.epoch!=record.identity.run_epoch||p.config!=record.identity.config{return false}
  let lifecycle=self.lifecycle.lock().unwrap();let authority=&lifecycle.authority;
  let Some(a)=authority.admission.as_ref()else{return false};let (Some(scope),Some(owner),Some(current),Some(decision))=(a.scope.as_ref(),a.owner.as_ref(),a.current_report.as_ref(),a.decision.as_ref())else{return false};
  let world=record.world.to_string();let instance=uuid::Uuid::from_bytes(game.owner.1).to_string();
  let Some(generations)=self.world.admitted_generations.as_ref()else{return false};
  if !self.world.coordinator.current(&generations.ticket)||generations.ticket.native!=game.owner||generations.ticket.world!=record.world||generations.lifecycle==0||generations.lifecycle!=record.canonical_generation||Some(generations.participant)!=a.generation.filter(|g|*g>0){return false}
  if gameplay&&!authority.entered||a.service_id.as_deref()!=Some(&self.service)||a.service_epoch.as_deref()!=Some(&self.epoch)
   ||scope.principal_id.as_deref()!=Some(&game.session.account)||scope.session_id.as_deref()!=Some(&game.session.id)||scope.session_epoch.as_deref()!=Some(&game.session.epoch)||scope.attachment_id.as_deref()!=Some(&game.session.attachment)
   ||scope.room_id.as_deref()!=Some(&record.identity.room)||scope.run_id.as_deref()!=Some(&p.run)||scope.run_epoch.as_deref()!=Some(&p.epoch)||scope.config_revision.map(u64::from)!=Some(p.config)||scope.world_token.as_deref()!=Some(&world)
   ||owner.desktop_id.as_deref()!=Some(&desktop.id)||owner.desktop_session_epoch.as_deref()!=Some(&desktop.epoch)||owner.game_session_id.as_deref()!=Some(&game.session.id)||owner.game_session_epoch.as_deref()!=Some(&game.session.epoch)||owner.attachment_id.as_deref()!=Some(&game.session.attachment)||owner.game_instance_id.as_deref()!=Some(&instance)
   ||decision.authorized!=Some(true)||gameplay&&decision.playing!=Some(true)||decision.terminal==Some(true)||gameplay&&current.protected!=Some(false)||current.complete!=Some(true)||current.loaded!=Some(true)||current.content_valid!=Some(true){return false}
  // New same-content local1s reports do not invalidate an installed playing
  // acknowledgement. Never require the exact newest in-flight sequence.
  current.revision.filter(|r|*r>0).map(u64::from).is_some_and(|revision|revision>=p.invalidated_observation&&revision<=report.sequence)
 }
}

#[derive(Clone)]pub(super) struct ServiceTime {pub scope:String,pub stamp:Stamp,pub upper_ms:u64}
impl<S:CredentialStore> Core<S>{
 pub(super) fn preparation_remaining(&self,prep:&wire::Preparation,review:bool)->Result<u64,String>{
  let expiry=if review{prep.review_expires_at.as_deref()}else{prep.expires_at.as_deref()}.ok_or("Missing preparation deadline")?;
  #[cfg(not(feature="g3-rejoin"))]{return Ok(timestamp(expiry)?.saturating_sub(now()));}
  #[cfg(feature="g3-rejoin")]{
   let mut c=self.lifecycle.lock().unwrap();let clock=c.service_time.as_ref().ok_or("Fresh authenticated timing sample required before preparation")?;
   let account=self.desktop.as_ref().ok_or("No preparation account")?;
   if clock.scope!=json!([self.base,self.service,self.epoch,account.account]).to_string(){return Err("Preparation timing issuer/account changed".into());}
   let end=clock.stamp.started.checked_add(Duration::from_millis(timestamp(expiry)?.saturating_sub(clock.upper_ms))).ok_or("Preparation deadline overflow")?;
   let key=json!([clock.scope,prep.preparation_id,review,if review{None}else{Some(expiry)}]).to_string();
   if c.preparations.len()>=64&&!c.preparations.contains_key(&key){return Err("Preparation timing capacity reached; explicitly reconnect".into());}
   let fixed=c.preparations.entry(key).or_insert(end);*fixed=(*fixed).min(end);
   Ok(fixed.saturating_duration_since(Instant::now()).as_millis().try_into().unwrap_or(u64::MAX))
  }
 }
}
