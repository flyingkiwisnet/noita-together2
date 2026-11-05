//! Early saved-world recovery. Pending proof is identity only; all current
//! authority continues through waiting and the existing health/admission owner.
use super::*;
use nt2_desktop_g0::startup_pending as local;
struct Evidence{scope:startup::Scope,intent:Option<w::PendingIntent>,proof:Option<String>,generation:u32,attach:Option<local::AttachOriginal>,ready:bool}
impl Evidence{fn ready(&self)->bool{self.ready}fn canonical(scope:&startup::Scope,record:&local::Record)->Self{Self{scope:scope.clone(),ready:record.ready(),intent:record.intent.clone(),proof:record.proof.clone(),generation:record.generation,attach:record.attach.clone()}}}
impl Drop for Evidence{fn drop(&mut self){use zeroize::Zeroize;if let Some(p)=&mut self.proof{p.zeroize()}}}
fn evidence<S:nt2_desktop_g0::secrets::CredentialStore>(store:&S,scope:&startup::Scope,world:uuid::Uuid)->Result<Evidence,String>{
 if let Some(record)=local::load(store,scope,world)?{return Ok(Evidence::canonical(scope,&record))}
 let record=nt2_desktop_g0::provisional_local::load(store,scope,world)?.ok_or("Pending original source absent")?;
 let canonical=record.canonical().ok_or("Provisional pending has no confirmed canonical intent")?;if canonical.scope!=*scope||canonical.marker.local_world!=world{return Err("Provisional canonical pending scope differs".into())}
 Ok(Evidence{scope:scope.clone(),intent:Some(canonical.pending.clone()),proof:Some(record.future_secret().as_str().into()),generation:canonical.pending_generation,attach:None,ready:!matches!(record.phase(),nt2_desktop_g0::provisional_local::Phase::Detached|nt2_desktop_g0::provisional_local::Phase::CancelSubmitted|nt2_desktop_g0::provisional_local::Phase::Cancelled)})
}
pub(super) fn has_source<S:nt2_desktop_g0::secrets::CredentialStore>(store:&S,scope:&startup::Scope,world:uuid::Uuid)->Result<bool,String>{
 if local::load(store,scope,world)?.is_some(){return Ok(true)}Ok(nt2_desktop_g0::provisional_local::load(store,scope,world)?.is_some_and(|r|r.canonical().is_some()))
}
pub(super) fn require_source<S:nt2_desktop_g0::secrets::CredentialStore>(store:&S,scope:&startup::Scope,world:uuid::Uuid)->Result<(),String>{let e=evidence(store,scope,world)?;if !e.ready(){return Err("Pending original proof retired or incomplete".into())}Ok(())}
pub(super) async fn retire<S:CredentialStore+'static>(store:Arc<S>,writer:Arc<Mutex<()>>,http:reqwest::Client,desktop:Session,nonce:Arc<AtomicU64>,expected:u64,mut l:Loaded)->Result<(),Failure>{
 let record={let(store,writer,s,world)=(store.clone(),writer.clone(),l.scope.clone(),l.journal.marker.local_world);tokio::task::spawn_blocking(move||{let _guard=writer.lock().unwrap();evidence(&rejoin::ResumeStore(store),&s,world)}).await.map_err(|_|"Pending retirement lookup canceled")??};
 if nonce.load(Ordering::Acquire)!=expected{return Err("Pending retirement authentication retired".into())}
 let original=record.intent.as_ref().ok_or("Pending retirement intent absent")?;
 if l.journal.retirement.is_none(){let mut query=envelope(&l.scope,&desktop);query.pending_id=original.pending_id.clone();query.pending_status=Some(w::Empty{});let response=rpc(&http,&l.scope,&desktop,&query).await?;let state=response.pending.as_ref().ok_or("Pending retirement current state absent")?;
  if state.may_play!=Some(false)||serde_json::to_value(&state.intent).ok()!=serde_json::to_value(&record.intent).ok(){return Err("Pending retirement current original differs".into())}
  let mut request=envelope(&l.scope,&desktop);request.pending_id=original.pending_id.clone();request.generation=Some(state.generation.filter(|g|*g>=record.generation).ok_or("Pending retirement generation regressed")?);request.pending_offline=Some(w::PendingOffline{pending_proof:record.proof.clone()});l.journal.retirement=Some(journal::Mutation::capture(&l.scope,request,&desktop.id)?);
  l={let(store,writer,nonce,l)=(store.clone(),writer.clone(),nonce.clone(),l.clone());tokio::task::spawn_blocking(move||{let _guard=writer.lock().unwrap();if nonce.load(Ordering::Acquire)!=expected{return Err("Pending retirement retired before commit".into())}journal::save_retired(&rejoin::ResumeStore(store),&l.scope,&l.journal)?;Ok::<_,String>(l)}).await.map_err(|_|"Pending retirement original writer canceled")??};
 }
 let op=l.journal.retirement.as_ref().ok_or("Pending retirement exact original absent")?;
 if nonce.load(Ordering::Acquire)!=expected{return Err("Pending retirement authentication retired before send".into())}
 if op.permits(&l.scope,&desktop.id,&desktop.epoch){let response=rpc(&http,&l.scope,&desktop,&op.request).await?;let state=response.pending.as_ref().ok_or("Pending retirement acknowledgment absent")?;if state.may_play!=Some(false)||state.state.as_deref()!=Some("terminal")||state.reason.as_deref()!=Some("explicit_offline")||serde_json::to_value(&state.intent).ok()!=serde_json::to_value(&record.intent).ok()||response.credentials.is_some(){return Err("Pending retirement acknowledgment differs".into())}}
 else{let mut query=envelope(&l.scope,&desktop);query.pending_outcome=Some(op.original.clone());let response=rpc(&http,&l.scope,&desktop,&query).await?;no_authority(&response)?;let outcome=response.pending_outcome.ok_or("Pending retirement outcome absent")?;if outcome.original_outcome.as_deref()!=Some("committed")||outcome.coverage.as_deref()!=Some("exact_durable_record")||outcome.pending_id!=original.pending_id||serde_json::to_value(&outcome.original).ok()!=serde_json::to_value(Some(&op.original)).ok(){return Err("Pending retirement remains uncertain".into())}}
 l.journal.retirement=None;l.journal.retirement_done=true;tokio::task::spawn_blocking(move||{let _guard=writer.lock().unwrap();if nonce.load(Ordering::Acquire)!=expected{return Err("Pending retirement retired before final commit".into())}let adapter=rejoin::ResumeStore(store);journal::save_retired(&adapter,&l.scope,&l.journal)?;journal::finish_retirement(&adapter,&l.scope,&l.journal.marker)}).await.map_err(|_|"Pending retirement commit canceled")??;Ok(())
}
fn no_authority(r:&w::Response)->Result<(),Failure>{if r.credentials.is_some()||r.resume_proof.is_some()||r.waiting_proof.is_some()||r.pending_proof.is_some()||r.canonical_admission.is_some(){return Err("Sanitized pending outcome unexpectedly carried authority".into())}Ok(())}
fn waiting_link(record:&Evidence,p:&w::PendingState,state:&w::WaitingState,marker:&startup::WaitingMarker)->Result<(),Failure>{
 let original=record.intent.as_ref().ok_or("Pending immutable intent absent")?;let i=state.intent.as_ref().ok_or("Promoted waiting intent absent")?;
 if p.may_play!=Some(false)||p.state.as_deref()!=Some("promoted")||serde_json::to_value(&p.intent).ok()!=serde_json::to_value(&record.intent).ok()||p.waiting_id.is_none()||p.waiting_id!=i.waiting_id||state.may_play!=Some(false)||state.state.as_deref()==Some("terminal")||i.pending_reservation_id.as_ref().is_some_and(|id|Some(id)!=original.pending_id.as_ref())||i.world_token!=original.world_token||i.context_id!=original.context_id||i.config_digest!=original.config_digest||i.seed_policy!=original.seed_policy||i.randomize_loot!=original.randomize_loot{return Err("Authenticated pending/waiting immutable linkage differs".into())}
 let mut preview=journal::Journal::new(marker.clone());preview.intent=state.intent.clone();preview.generation=state.generation.filter(|g|*g>0).ok_or("Promoted waiting generation absent")?;preview.current_attachment=state.owner.as_ref().and_then(|o|o.attachment_id.clone());preview.validate(&record.scope)?;Ok(())
}
fn exact_enrollment(i:&w::WaitingIntent,e:&w::Enroll)->bool{i.preparation_id==e.preparation_id&&i.original_attachment_id==e.attachment_id&&i.original_attach_request_id==e.original_attach_request_id&&i.original_attach_fingerprint==e.original_attach_fingerprint&&i.world_token==e.world_token&&i.original_game_instance_id==e.game_instance_id&&i.original_observation_revision==e.observation_revision&&i.content_digest==e.content_digest&&i.context_id==e.context_id}
fn promotion_proof(state:&w::WaitingState,response:&w::Response)->Result<Option<String>,Failure>{
 if state.state.as_deref()==Some("started"){
  if response.waiting_proof.is_some()||response.participant.is_none()||response.resume_proof.as_ref().is_none_or(|p|p.len()!=43)||response.canonical_admission.is_none(){return Err("Selected pending recovery authority/proof shape differs".into())}Ok(None)
 }else{let proof=response.waiting_proof.as_ref().filter(|p|p.len()==43).ok_or("Pending promotion proof absent; exact original retained")?;Ok(Some(proof.clone()))}
}
async fn save<S:CredentialStore+'static>(store:Arc<S>,writer:Arc<Mutex<()>>,c:Arc<Mutex<lifecycle::Coordinator>>,nonce:Arc<AtomicU64>,generation:Arc<AtomicU64>,expected:(u64,u64),ticket:lifecycle::Ticket,l:Loaded)->Result<Loaded,Failure>{
 Ok(tokio::task::spawn_blocking(move||{let _guard=writer.lock().unwrap();if !current(&c,&nonce,&generation,expected,&ticket){return Err("Pending owner retired before original commit".into())}journal::save(&rejoin::ResumeStore(store),&l.scope,&l.journal)?;if !current(&c,&nonce,&generation,expected,&ticket){return Err("Pending owner retired during original commit".into())}Ok::<_,String>(l)}).await.map_err(|_|"Pending original worker canceled")??)
}
pub(super) async fn recover_pending<S:CredentialStore+'static>(store:Arc<S>,writer:Arc<Mutex<()>>,c:Arc<Mutex<lifecycle::Coordinator>>,nonce:Arc<AtomicU64>,generation:Arc<AtomicU64>,expected:(u64,u64),ticket:lifecycle::Ticket,http:reqwest::Client,desktop:Session,device:uuid::Uuid,mut l:Loaded,contact:crate::pipe::rejoin::Contact,report:native_report::Latest,game_version:String,capabilities:Vec<String>)->Result<Recovered,Failure>{
 let record={let (store,writer,s,world)=(store.clone(),writer.clone(),l.scope.clone(),l.journal.marker.local_world);tokio::task::spawn_blocking(move||{let _guard=writer.lock().unwrap();evidence(&rejoin::ResumeStore(store),&s,world)}).await.map_err(|_|"Pending lookup worker canceled")??};
 if !record.ready(){return Err("Original pending proof unavailable or explicitly retired".into())}
 let original=record.intent.as_ref().ok_or("Original pending intent absent")?;
 let mut query=envelope(&l.scope,&desktop);query.pending_id=original.pending_id.clone();query.pending_status=Some(w::Empty{});let status=rpc(&http,&l.scope,&desktop,&query).await?;
 if status.credentials.is_some()||status.waiting_proof.is_some()||status.resume_proof.is_some()||status.canonical_admission.is_some(){return Err("Pending status unexpectedly carried load/play authority".into())}
 let state=status.pending.as_ref().ok_or("Current pending state absent")?;
 if state.may_play!=Some(false)||!matches!(state.state.as_deref(),Some("pending"|"promoted"))||serde_json::to_value(&state.intent).ok()!=serde_json::to_value(&record.intent).ok()||state.bound_actual_seed.as_ref().is_some_and(|s|s!=&l.journal.marker.actual_seed){return Err("Current pending original/seed is terminal or differs".into())}
 let pending_generation=state.generation.filter(|g|*g>=record.generation).ok_or("Pending generation regressed")?;
 // An original operation from another auth/contact is reconciled exactly. A
 // committed effect supplies no authority; a fresh protected recovery follows.
 if let Some(op)=l.journal.operation.as_ref().filter(|op|!op.permits(&l.scope,&desktop.id,&desktop.epoch)||op.request.pending_recover.as_ref().and_then(|r|r.context_id.as_deref())!=Some(hex(&contact.context).as_str())){
  let mut query=envelope(&l.scope,&desktop);
  if let Some(enroll)=&op.request.enroll{
   query.outcome=Some(op.original.clone());let response=rpc(&http,&l.scope,&desktop,&query).await?;if response.credentials.is_some()||response.waiting_proof.is_some()||response.pending_proof.is_some()||response.resume_proof.is_some(){return Err("Enrollment outcome unexpectedly carried credentials/proof".into())}let outcome=response.outcome.as_ref().ok_or("Exact uncertain enrollment outcome absent")?;
   if outcome.original_outcome.as_deref()!=Some("committed")||outcome.coverage.as_deref()!=Some("exact_durable_record")||serde_json::to_value(&outcome.original).ok()!=serde_json::to_value(Some(&op.original)).ok(){return Err("Exact enrollment remains explicitly uncertain".into())}
   let waiting=response.waiting.as_ref().ok_or("Exact enrollment current waiting state absent")?;waiting_link(&record,state,waiting,&l.journal.marker)?;if !exact_enrollment(waiting.intent.as_ref().unwrap(),enroll){return Err("Exact enrollment immutable original differs".into())}
   // Admission metadata in a started outcome is descriptive only; it is never
   // applied here. Fresh protected recovery below validates the current owner.
   if response.canonical_admission.is_some()&&waiting.state.as_deref()!=Some("started"){return Err("Staging enrollment outcome carried admission".into())}
   l.journal.intent=waiting.intent.clone();l.journal.generation=waiting.generation.filter(|g|*g>0).ok_or("Reconciled enrollment generation absent")?;l.journal.current_attachment=waiting.owner.as_ref().and_then(|o|o.attachment_id.clone());
  }else{
   query.pending_outcome=Some(op.original.clone());let response=rpc(&http,&l.scope,&desktop,&query).await?;no_authority(&response)?;let outcome=response.pending_outcome.as_ref().ok_or("Pending original outcome absent")?;
   if outcome.original_outcome.as_deref()!=Some("committed")||outcome.coverage.as_deref()!=Some("exact_durable_record")||serde_json::to_value(&outcome.original).ok()!=serde_json::to_value(Some(&op.original)).ok()||outcome.pending_id!=original.pending_id{return Err("Pending previous recovery remains explicitly uncertain".into())}
  }
  l.journal.operation=None;l=save(store.clone(),writer.clone(),c.clone(),nonce.clone(),generation.clone(),expected,ticket.clone(),l).await?;
 }
 if l.journal.operation.is_none(){
  let attachment=if let Some(waiting_id)=&state.waiting_id{
   // Never substitute an immutable original attachment after promotion.
   let mut query=envelope(&l.scope,&desktop);query.waiting_id=Some(waiting_id.clone());query.status=Some(w::Empty{});let result=rpc(&http,&l.scope,&desktop,&query).await?;
   if result.credentials.is_some(){return Err("Waiting owner lookup unexpectedly carried credentials".into())}let waiting=result.waiting.as_ref().ok_or("Promoted current waiting state absent")?;waiting_link(&record,state,waiting,&l.journal.marker)?;if result.canonical_admission.is_some()&&waiting.state.as_deref()!=Some("started"){return Err("Staging waiting lookup carried admission".into())}waiting.owner.as_ref().and_then(|o|o.attachment_id.clone())
  }else if let Some(attach)=&record.attach{
   let mut query=envelope(&l.scope,&desktop);query.pending_outcome=Some(attach.original.clone());let result=rpc(&http,&l.scope,&desktop,&query).await?;no_authority(&result)?;let outcome=result.pending_outcome.as_ref().ok_or("Original attach outcome absent")?;
   if serde_json::to_value(&outcome.original).ok()!=serde_json::to_value(Some(&attach.original)).ok(){return Err("Original attach outcome metadata differs".into())}
   match outcome.original_outcome.as_deref(){Some("committed") if outcome.coverage.as_deref()==Some("exact_consumed_attach_record")&&outcome.pending_id==original.pending_id=>Some(outcome.effect_id.clone().ok_or("Original committed attachment absent")?),Some("unresolved")=>None,_=>return Err("Original attach outcome remains uncertain".into())}
  }else{None};
  let mut r=envelope(&l.scope,&desktop);r.pending_id=original.pending_id.clone();r.generation=Some(pending_generation);r.pending_recover=Some(w::PendingRecover{pending_proof:record.proof.clone(),game_version:Some(game_version),mod_version:Some(report.observation.mod_version.clone()),bridge_version:Some(report.observation.bridge_version.clone()),capabilities:Some(capabilities),context_id:Some(hex(&contact.context)),observation:Some(w::GameObservation{attachment_id:attachment,game_instance_id:Some(uuid::Uuid::new_v4().to_string()),world_token:original.world_token.clone(),observation_revision:Some(report.sequence.try_into().map_err(|_|"Pending observation revision bound")?),actual_seed:Some(l.journal.marker.actual_seed.clone()),active_mods:Some(report.observation.mods.clone()),inventory_complete:Some(true),beta:Some(match report.observation.beta{Some(true)=>"true",Some(false)=>"false",None=>"unknown"}.into()),protected:Some(true),loaded:Some(true),seed_status:Some("known".into()),collector_failure:None,mod_version:Some(report.observation.mod_version.clone()),bridge_version:Some(report.observation.bridge_version.clone()),compatibility_provenance:Some("game-thread-client-observation".into())})});
  l.journal.operation=Some(journal::Mutation::capture(&l.scope,r,&desktop.id)?);l=save(store.clone(),writer.clone(),c.clone(),nonce.clone(),generation.clone(),expected,ticket.clone(),l).await?;
 }
 let request=l.journal.operation.as_ref().ok_or("Pending exact original absent")?.request.clone();let id=request.request_id.as_deref().ok_or("Pending original request ID absent")?;let stamp=c.lock().unwrap().request_stamp(&json!([l.scope.origin,l.scope.service,desktop.id,desktop.epoch,"pending-recover"]).to_string(),id)?;
 if !current(&c,&nonce,&generation,expected,&ticket){return Err("Pending owner retired before send".into())}
 let response=rpc(&http,&l.scope,&desktop,&request).await?;let pending=response.pending.as_ref().ok_or("Recovered pending identity absent")?;let waiting=response.waiting.as_ref().ok_or("Recovered canonical waiting identity absent")?;let intent=waiting.intent.as_ref().ok_or("Recovered waiting intent absent")?;
 waiting_link(&record,pending,waiting,&l.journal.marker)?;let _=intent;
 if pending.generation.is_none_or(|g|g<=request.generation.unwrap_or(0))||pending.bound_actual_seed.as_deref()!=Some(&l.journal.marker.actual_seed){return Err("Recovered pending generation/seed differs".into())}
 if l.journal.intent.is_some()&&serde_json::to_value(&l.journal.intent).ok()!=serde_json::to_value(&waiting.intent).ok(){return Err("Previously reconciled enrollment immutable intent changed".into())}
 l.journal.intent=waiting.intent.clone();
 if waiting.state.as_deref()==Some("started"){
  // No waiting proof is issued after Start. Retain the original pending proof
  // and exact operation; the strict selected admission path below validates
  // current ownership before any Game is published. A later quit can use the
  // same pending proof again until actual entry captures admitted proof.
 }
 l.journal.proof=promotion_proof(waiting,&response)?;
 complete_recovery(store,writer,c,nonce,generation,expected,ticket,desktop,device,l,contact,report,request,stamp,response).await
}
#[cfg(test)]mod tests{
 use super::*;
 fn waiting_link(record:&local::Record,p:&w::PendingState,state:&w::WaitingState,marker:&startup::WaitingMarker)->Result<(),Failure>{super::waiting_link(&Evidence::canonical(&record.scope,record),p,state,marker)}
 #[test]fn startup_repair_ordinary_enrollment_links_without_optional_pending_id_but_exact_original_is_required(){
  let(c,marker)=super::super::tests::parked();let adapter=rejoin::ResumeStore(c.store.clone());let s=startup::recall(&adapter,&c.base,&c.service,&c.desktop.as_ref().unwrap().account,c.device,&marker).unwrap().unwrap().original;let j=journal::load(&adapter,&s,&marker).unwrap().unwrap();let mut i=j.intent.clone().unwrap();let context=marker.local_world.simple().to_string();i.world_token=Some(context.clone());i.context_id=Some(context.clone());i.pending_reservation_id=None;
  let original=w::PendingIntent{pending_id:Some(uuid::Uuid::new_v4().to_string()),preparation_id:Some(s.preparation.clone()),service_id:Some(s.service.clone()),service_epoch:Some(s.service_epoch.clone()),room_id:Some(s.room.clone()),run_id:Some(s.run.clone()),run_epoch:Some(s.run_epoch.clone()),config_revision:Some(s.config as u32),config_digest:Some(s.config_digest.clone()),account_id:Some(s.account.clone()),device_id:Some(s.device.to_string()),original_desktop_id:Some(s.desktop.clone()),original_desktop_epoch:Some(s.desktop_epoch.clone()),world_token:Some(context.clone()),context_id:Some(context.clone()),seed_policy:i.seed_policy.clone(),randomize_loot:i.randomize_loot,actual_seed:i.actual_seed.clone()};
  let request=w::Request{major:Some(1),revision:Some(protocol::REVISION.into()),service_id:Some(s.service.clone()),service_epoch:Some(s.service_epoch.clone()),session_epoch:Some(s.desktop_epoch.clone()),request_id:Some(uuid::Uuid::new_v4().to_string()),reserve:Some(w::Reserve{preparation_id:Some(s.preparation.clone()),world_token:Some(context.clone()),context_id:Some(context)}),..Default::default()};let record=local::Record{scope:s.clone(),world:marker.local_world,original_session:s.desktop.clone(),fingerprint:nt2_desktop_g0::protocol_connection::canonical_fingerprint(&serde_json::to_vec(&request).unwrap()).unwrap(),request,intent:Some(original),proof:Some("p".repeat(43)),generation:1,retired:false,operation:None,attach:None};record.validate().unwrap();
  let pending=w::PendingState{intent:record.intent.clone(),state:Some("promoted".into()),may_play:Some(false),generation:Some(2),waiting_id:i.waiting_id.clone(),..Default::default()};let mut state=w::WaitingState{intent:Some(i.clone()),state:Some("waiting".into()),may_play:Some(false),generation:Some(2),owner:Some(w::ConnectionOwner{attachment_id:j.current_attachment.clone(),..Default::default()}),..Default::default()};assert!(waiting_link(&record,&pending,&state,&marker).is_ok());state.intent.as_mut().unwrap().pending_reservation_id=record.intent.as_ref().unwrap().pending_id.clone();assert!(waiting_link(&record,&pending,&state,&marker).is_ok());state.intent.as_mut().unwrap().pending_reservation_id=Some(uuid::Uuid::new_v4().to_string());assert!(waiting_link(&record,&pending,&state,&marker).is_err());state.intent=Some(i.clone());let mut wrong=pending.clone();wrong.waiting_id=Some(uuid::Uuid::new_v4().to_string());assert!(waiting_link(&record,&wrong,&state,&marker).is_err());state.intent.as_mut().unwrap().config_digest=Some("f".repeat(64));assert!(waiting_link(&record,&pending,&state,&marker).is_err());state.intent=Some(i.clone());state.intent.as_mut().unwrap().world_token=Some(uuid::Uuid::new_v4().simple().to_string());assert!(waiting_link(&record,&pending,&state,&marker).is_err());
  let mut enroll=w::Enroll{preparation_id:i.preparation_id.clone(),attachment_id:i.original_attachment_id.clone(),original_attach_request_id:i.original_attach_request_id.clone(),original_attach_fingerprint:i.original_attach_fingerprint.clone(),world_token:i.world_token.clone(),game_instance_id:i.original_game_instance_id.clone(),observation_revision:i.original_observation_revision,content_digest:i.content_digest.clone(),context_id:i.context_id.clone()};assert!(exact_enrollment(&i,&enroll));enroll.original_attach_fingerprint=Some("f".repeat(64));assert!(!exact_enrollment(&i,&enroll));
 }
 #[test]fn startup_repair_started_pending_bridge_never_fabricates_waiting_proof(){
  let mut state=w::WaitingState{state:Some("started".into()),..Default::default()};let mut response=w::Response{participant:Some(w::Participant::default()),resume_proof:Some("p".repeat(43)),canonical_admission:Some(w::ConnectionAdmission::default()),..Default::default()};assert!(promotion_proof(&state,&response).unwrap().is_none());response.waiting_proof=Some("p".repeat(43));assert!(promotion_proof(&state,&response).is_err());response.waiting_proof=None;response.resume_proof=None;assert!(promotion_proof(&state,&response).is_err());state.state=Some("waiting".into());assert!(promotion_proof(&state,&response).is_err());response.waiting_proof=Some("w".repeat(43));assert!(promotion_proof(&state,&response).unwrap().is_some());
  // These are proof-publication shape fragments only. Production then invokes
  // the strict original linkage, selected participant and admission validators;
  // the empty authority fragments above could never publish a Game/release.
 }
}
