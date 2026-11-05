//! Read-only replacement configuration. A's owner is never passed to the local worker.
use super::*;
use nt2_desktop_g0::{startup_local::Scope,provisional_local::Preview,protocol_waiting::{self as protocol,wire as w},connection_time::{Stamp,Deadline}};

pub(super) fn decode_preview(scope:&Scope,config:&wire::Config,desktop:&Session,request:&w::Request,response:&w::Response,stamp:Stamp)->Result<Preview,String>{
 protocol::validate("Response",response)?;
 if response.major!=Some(1)||response.revision.as_deref()!=Some(protocol::REVISION)||response.service_id.as_deref()!=Some(&scope.service)||response.service_epoch.as_deref()!=Some(&scope.service_epoch)||response.request_id!=request.request_id||response.error.is_some()
  ||response.credentials.is_some()||response.pending_proof.is_some()||response.waiting_proof.is_some()||response.resume_proof.is_some()||response.canonical_admission.is_some()||response.pending.is_some()||response.waiting.is_some()||response.entry.is_some(){return Err("Read-only entry preview returned a different scope or authority".into())}
 let preview=response.entry_preview.as_ref().ok_or("Entry configuration preview absent")?;let entry=preview.scope.as_ref().ok_or("Entry preview scope absent")?;let sample=preview.sample.as_ref().ok_or("Entry preview correlation absent")?;
 let configuration=serde_json::to_value(config).map_err(|_|"Entry frozen configuration encoding")?;
 let digest=nt2_desktop_g0::protocol_connection::canonical_fingerprint(&serde_json::to_vec(&configuration).map_err(|_|"Entry config encoding")?)?;
 if preview.may_play!=Some(false)||entry.service_id.as_deref()!=Some(&scope.service)||entry.service_epoch.as_deref()!=Some(&scope.service_epoch)||entry.account_id.as_deref()!=Some(&desktop.account)||entry.device_id.as_deref()!=Some(scope.device.to_string().as_str())||entry.room_id.as_deref()!=Some(&scope.room)||entry.run_id.as_deref()!=Some(&scope.run)||entry.run_epoch.as_deref()!=Some(&scope.run_epoch)||entry.config_revision.map(u64::from)!=Some(scope.config)||entry.config_digest.as_deref()!=Some(&digest)
  ||sample.channel.as_deref()!=Some("entry-http")||sample.origin_request_id!=request.request_id||sample.credential_id.as_deref()!=Some(admission::token_id(desktop.access.as_str())?.as_str())||sample.sample_key.as_ref().is_none_or(|v|v.len()!=64||!v.bytes().all(|b|b.is_ascii_hexdigit()&&!b.is_ascii_uppercase())){return Err("Entry preview run/config/actor/request correlation differs".into())}
 let resolved=seed_policy::resolved(config)?;if entry.seed_policy.as_deref()!=Some(&resolved.name)||entry.randomize_loot!=Some(resolved.loot){return Err("Entry preview seed/loot configuration differs".into())}
 let (seed,policy)=match entry.seed_policy.as_deref(){Some("individual-random") if entry.actual_seed.is_none()&&entry.randomize_loot==Some(false)=>(0,2),Some("shared-fixed"|"shared-random")=>{
  let actual=entry.actual_seed.as_deref().ok_or("Shared entry seed absent")?;let seed=actual.parse::<u32>().map_err(|_|"Shared entry seed bound")?;
  if seed.to_string()!=actual||config.seed!=Some(seed){return Err("Shared entry frozen seed differs".into())}(seed,if entry.randomize_loot==Some(true){5}else{1})},_=>return Err("Entry seed policy unsupported".into())};
 let now=Instant::now();let mut review=Deadline::default();let until=review.install(stamp,preview.review_remaining_ms.ok_or("Entry preview review absent")?,3600000,3600000,now)?;
 let v:nt2_desktop_g0::protocol_connection::wire::ConnectionCredential=serde_json::from_value(serde_json::to_value(response.credential_validity.as_ref().ok_or("Entry preview desktop validity absent")?).map_err(|_|"Entry validity encoding")?).map_err(|_|"Entry validity shape")?;
 let mut leases=std::collections::BTreeMap::new();let until=until.min(stamp.started+Duration::from_secs(30)).min(admission::credential_sample(&mut leases,&scope.origin,&scope.service,desktop,&v,request.request_id.as_deref().ok_or("Entry preview request absent")?,stamp)?);
 if until<=now{return Err("Entry preview expired in flight".into())}
 let mut local=scope.clone();local.config_digest=digest;local.preparation.clear();
 Ok(Preview{scope:local,configuration,base:serde_json::to_value(entry).map_err(|_|"Entry immutable scope encoding")?,seed,policy,until,sample:stamp})
}

#[cfg(test)]mod tests{
 use super::*;
 fn fixture()->(Scope,wire::Config,Session,w::Request,w::Response,Stamp){
  use base64::Engine;let uuid=||uuid::Uuid::new_v4().to_string();let account=uuid();let id=uuid();let epoch=uuid();let credential=uuid();
  let desktop=Session{access:SecretString::from(format!("fixture.{}.unsigned",base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(json!({"jti":credential}).to_string()))),refresh:SecretString::from("x".repeat(43)),id:id.clone(),epoch:epoch.clone(),account:account.clone(),attachment:String::new(),principal:wire::Principal::default(),expires:0};
  let config:wire::Config=serde_json::from_value(json!({"mode":"core","seed":424242,"seedPolicy":"shared-fixed","randomizeLoot":true,"requiredCapabilities":["core.seed-policy.v1","core.randomize-loot.v1"]})).unwrap();
  let digest=nt2_desktop_g0::protocol_connection::canonical_fingerprint(&serde_json::to_vec(&config).unwrap()).unwrap();
  let scope=Scope{origin:"https://entry.invalid".into(),service:"service".into(),service_epoch:uuid(),account:account.clone(),device:uuid::Uuid::new_v4(),desktop:id.clone(),desktop_epoch:epoch.clone(),room:uuid(),run:uuid(),run_epoch:uuid(),config:1,config_digest:digest.clone(),preparation:String::new()};
  let request=w::Request{major:Some(1),revision:Some(protocol::REVISION.into()),service_id:Some(scope.service.clone()),service_epoch:Some(scope.service_epoch.clone()),session_epoch:Some(epoch.clone()),request_id:Some(uuid()),entry_preview:Some(w::EntryPreviewRequest{room_id:Some(scope.room.clone()),base_preparation_id:None}),..Default::default()};
  let entry=w::EntryScope{service_id:Some(scope.service.clone()),service_epoch:Some(scope.service_epoch.clone()),account_id:Some(account.clone()),device_id:Some(scope.device.to_string()),room_id:Some(scope.room.clone()),run_id:Some(scope.run.clone()),run_epoch:Some(scope.run_epoch.clone()),config_revision:Some(1),config_digest:Some(digest),base_preparation_id:Some(uuid()),base_fingerprint:Some("a".repeat(64)),seed_policy:Some("shared-fixed".into()),randomize_loot:Some(true),actual_seed:Some("424242".into())};
  let sample=w::ConnectionSample{channel:Some("entry-http".into()),origin_request_id:request.request_id.clone(),credential_id:Some(credential.clone()),sample_key:Some("a".repeat(64))};
  let validity=w::ConnectionCredential{principal_id:Some(account),session_id:Some(id),session_epoch:Some(epoch),credential_id:Some(credential),origin_request_id:request.request_id.clone(),sample_key:Some("b".repeat(64)),access_remaining_ms:Some(5000)};
  let response=w::Response{major:Some(1),revision:Some(protocol::REVISION.into()),service_id:request.service_id.clone(),service_epoch:request.service_epoch.clone(),request_id:request.request_id.clone(),entry_preview:Some(w::EntryPreview{scope:Some(entry),sample:Some(sample),review_remaining_ms:Some(60000),may_play:Some(false)}),credential_validity:Some(validity),..Default::default()};
  (scope,config,desktop,request,response,Stamp{sequence:1,started:Instant::now()})
 }
 #[test]fn se1_entry_preview_binds_actor_current_run_config_and_request(){
  let(s,c,d,q,r,t)=fixture();let preview=decode_preview(&s,&c,&d,&q,&r,t).unwrap();assert!(preview.scope.preparation.is_empty());assert_eq!(preview.seed,424242);assert_eq!(preview.policy,5);
  assert!(preview.until<=t.started+Duration::from_secs(5));
  for fault in ["run","run-epoch","config","account","credential","request","authority","policy"]{
   let mut r=r.clone();match fault{
    "run"=>r.entry_preview.as_mut().unwrap().scope.as_mut().unwrap().run_id=Some(uuid::Uuid::new_v4().to_string()),
    "run-epoch"=>r.entry_preview.as_mut().unwrap().scope.as_mut().unwrap().run_epoch=None,
    "config"=>r.entry_preview.as_mut().unwrap().scope.as_mut().unwrap().config_digest=Some("c".repeat(64)),
    "account"=>r.entry_preview.as_mut().unwrap().scope.as_mut().unwrap().account_id=Some(uuid::Uuid::new_v4().to_string()),
    "credential"=>r.credential_validity.as_mut().unwrap().credential_id=Some("other".into()),
    "request"=>r.entry_preview.as_mut().unwrap().sample.as_mut().unwrap().origin_request_id=Some(uuid::Uuid::new_v4().to_string()),
    "authority"=>r.waiting_proof=Some("x".repeat(43)),
    "policy"=>r.entry_preview.as_mut().unwrap().scope.as_mut().unwrap().randomize_loot=Some(false),_=>unreachable!()}
   assert!(decode_preview(&s,&c,&d,&q,&r,t).is_err(),"{fault}");
  }
 }
 #[test]fn se1_entry_preview_elapsed_time_never_becomes_generation_permission(){
  let(s,c,d,q,r,mut t)=fixture();t.started-=Duration::from_secs(6);assert!(decode_preview(&s,&c,&d,&q,&r,t).is_err());
  let(s,c,d,q,mut r,t)=fixture();r.entry_preview.as_mut().unwrap().review_remaining_ms=Some(0);assert!(decode_preview(&s,&c,&d,&q,&r,t).is_err());
 }
}

impl<S:CredentialStore> Core<S>{
 // Attach, ordinary waiting enrollment and saved ownership recovery change
 // the canonical base. Inspection/presence and same-session renewal do not.
 pub(super) fn sync_replacement_lineage(&mut self)->Option<u64>{
  let pending=self.pending.lock().unwrap().clone();let metadata=self.lifecycle.lock().unwrap().waiting_metadata.clone();
  let saved={let d=self.native_diagnostics.lock().unwrap();d.rejoin.waiting_offered.as_ref().is_some_and(|(_,m)|m.kind!="nt2-provisional")&&d.rejoin.contact.as_ref().is_none_or(|c|self.game.as_ref().is_none_or(|g|g.owner!=(c.pid,c.context)))};
  let ready=!saved&&pending.as_ref().is_none_or(|p|p.owner.is_none()||if let Some(b)=&p.startup_bundle{p.startup_attach.as_ref().is_some_and(|a|self.game.as_ref().is_some_and(|g|Some(g.owner)==p.owner&&g.session.attachment==a.attachment))&&metadata.as_ref().is_some_and(|m|m.local_world==b.world()&&m.bundle_digest==b.digest())}else{self.game.as_ref().is_some_and(|g|Some(g.owner)==p.owner)});
  let key=json!([self.base,self.service,self.epoch,self.device,self.desktop.as_ref().map(|d|&d.account),self.replacement_review_target().map(|p|(p.room,p.run,p.config_revision)),pending.as_ref().map(|p|(&p.run,&p.epoch,p.config,&p.preparation_id,p.owner)),self.game.as_ref().map(|g|(&g.session.id,&g.session.epoch,&g.session.attachment,g.owner)),metadata.as_ref().map(|m|(m.local_world,&m.bundle_digest))]).to_string();
  self.replacement_lineage.update(key,ready);let epoch=self.replacement_lineage.snapshot();if self.entry.lineage_epoch!=epoch.unwrap_or(0){self.entry.lineage_epoch=epoch.unwrap_or(0);self.entry.reset_preview_clock();}epoch
 }
 /// A stale lifecycle context is a reason for a fresh authenticated read, not
 /// permission to relabel the old selected snapshot. This never absorbs a Room
 /// or changes membership, current A, selectors or the chosen run/config.
 pub(super) async fn read_replacement_review(&mut self)->Result<setup::Preview,String>{
  let lineage=self.sync_replacement_lineage().ok_or("Local attachment/enrollment/recovery is not settled")?;
  let (sequence,selection)=self.replacement_review_selection();let context=self.preview_context();let desktop=self.desktop.clone().ok_or("Replacement review requires authentication")?;
  let selected=self.replacement_review_target().ok_or("Replacement selection absent")?;
  if selected.sequence!=sequence||selected.state.as_deref()!=Some("staging")||selected.run.is_none(){return Err("Replacement selection is not a complete staging run".into())}
  let mut request=self.request(false);request.room_id=Some(selected.room.clone());request.expected_revision=None;request.config_revision=None;request.run_id=None;request.run_epoch=None;request.room_preview=Some(wire::Empty{});
  let response=self.rpc(&request,false,false).await?;
  if !self.replacement_lineage.current(lineage)||context!=self.preview_context()||self.replacement_review_selection()!=(sequence,selection)||self.desktop.as_ref().is_none_or(|d|d.access.as_str()!=desktop.access.as_str()){return Err("Late automatic replacement review retired".into())}
  let admission=response.admission.ok_or("Replacement lobby review absent")?;let room=response.room.ok_or("Replacement lobby snapshot absent")?;let config=room.config.ok_or("Replacement configuration absent")?;let run=room.run_id.zip(room.run_epoch);
  if admission.room_id.as_deref()!=Some(&selected.room)||room.room_id!=admission.room_id||room.revision!=admission.revision||room.config_revision!=admission.config_revision||run!=selected.run||room.config_revision!=Some(selected.config_revision)||serde_json::to_value(&config).ok()!=serde_json::to_value(&selected.config).ok()||room.state.as_deref()!=Some("staging")||run.as_ref().is_none_or(|(r,e)|uuid::Uuid::parse_str(r).is_err()||uuid::Uuid::parse_str(e).is_err()){return Err("Replacement lobby run/configuration changed; original worlds retained".into())}
  Ok(setup::Preview{room:selected.room,revision:admission.revision.ok_or("Replacement lobby revision absent")?,config_revision:selected.config_revision,config,run,state:room.state,context,sequence})
 }
 pub(super) fn unused_replacement_slot(&self)->bool{self.provisional.as_ref().is_none_or(|w|{let(busy,view,_)=w.view();!busy&&!w.fresh_started()&&view.is_none_or(|v|v.phase==nt2_desktop_g0::provisional_local::Phase::Staged)})}
 /// An explicit coherent lobby read selects configuration only. This operation
 /// does not call Join, reserve, recover, cancel or change the active A slot.
 pub(super) async fn prepare_replacement_preview(&mut self)->Result<(),String> where S:'static{
  let lineage=self.sync_replacement_lineage().ok_or("Local attachment/enrollment/recovery is not settled")?;
  if !self.unused_replacement_slot(){return Err("Original fresh entry retained; unused replacement slot required".into())}
  let selected=self.setup.preview.clone().ok_or("Review current lobby before replacement preview")?;
  let selection=self.replacement_review_selection();let desktop_token=self.desktop.as_ref().map(|d|d.access.clone());
  self.supported_policy(&selected.config)?;
  if selected.context!=self.preview_context()||selected.state.as_deref()!=Some("staging"){return Err("Replacement preview is not the current staging review".into())}
  let (run,run_epoch)=selected.run.clone().ok_or("Replacement preview requires complete current-run identity")?;
  let desktop=self.desktop.clone().ok_or("Entry preview requires current authentication")?;
  let scope=Scope{origin:self.base.clone(),service:self.service.clone(),service_epoch:self.epoch.clone(),account:desktop.account.clone(),device:self.device,desktop:desktop.id.clone(),desktop_epoch:desktop.epoch.clone(),room:selected.room.clone(),run,run_epoch,config:u64::from(selected.config_revision),config_digest:String::new(),preparation:String::new()};
  let mut request=w::Request{major:Some(1),revision:Some(protocol::REVISION.into()),service_id:Some(scope.service.clone()),service_epoch:Some(scope.service_epoch.clone()),session_epoch:Some(desktop.epoch.clone()),request_id:Some(uuid::Uuid::new_v4().to_string()),..Default::default()};
  request.entry_preview=Some(w::EntryPreviewRequest{room_id:Some(scope.room.clone()),base_preparation_id:None});
  let stamp=self.lifecycle.lock().unwrap().request_stamp(&json!([scope.origin,scope.service,desktop.id,desktop.epoch,"entry-preview"]).to_string(),request.request_id.as_deref().unwrap())?;
  let response=waiting::rpc(&self.http,&scope,&desktop,&request).await.map_err(|e|e.code)?;
  if !self.replacement_lineage.current(lineage)||selected.context!=self.preview_context()||self.replacement_review_selection()!=selection||self.desktop.as_ref().map(|d|d.access.as_str())!=desktop_token.as_ref().map(|t|t.as_str())||!self.unused_replacement_slot(){return Err("Late replacement preview retired; original fresh entry retained".into())}
  let preview=decode_preview(&scope,&selected.config,&desktop,&request,&response,stamp)?;
  self.stage_provisional_at(preview,Some(lineage))?;
  let worker=self.provisional.clone().ok_or("Entry worker absent")?;super::entry_transport::settle(&worker).await?;
  if !self.replacement_lineage.current(lineage)||selected.context!=self.preview_context(){return Err("Entry review retired during local staging".into())}
  let mut diagnostics=self.native_diagnostics.lock().unwrap();if diagnostics.provisional.is_none(){diagnostics.provisional=Some(Arc::new(crate::pipe::entry::Bridge::new(worker)));}Ok(())
 }
}
