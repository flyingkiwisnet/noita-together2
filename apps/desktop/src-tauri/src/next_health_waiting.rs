//! Independent waiting enrollment lane; same lifecycle fences, no Core clone.
use super::*;use nt2_desktop_g0::{protocol_waiting::{self as protocol,wire as w},waiting_local as journal,startup_local as startup};
fn current(c:&Arc<Mutex<lifecycle::Coordinator>>,nonce:&AtomicU64,generation:&AtomicU64,a:&Active)->bool{a.nonce==nonce.load(Ordering::Acquire)&&a.generation==generation.load(Ordering::Acquire)&&c.lock().unwrap().accepts(&a.ticket)}
pub(super) async fn enroll<S:CredentialStore+'static>(store:Arc<S>,writer:Arc<Mutex<()>>,c:Arc<Mutex<lifecycle::Coordinator>>,nonce:Arc<AtomicU64>,generation:Arc<AtomicU64>,http:reqwest::Client,a:Active,bundle:Arc<startup::Prepared>,owner:(crate::local_owner::Id,[u8;16]),actual:u64,path:std::path::PathBuf,attach:startup::AttachEvidence,ack:native_report::Latest)->Result<startup::WaitingMarker,String>{
 let request=w::Request{major:Some(1),revision:Some(protocol::REVISION.into()),service_id:Some(a.ctx.issuer.service.clone()),service_epoch:Some(a.ctx.epoch.clone()),session_epoch:Some(a.ctx.desktop.epoch.clone()),request_id:Some(uuid::Uuid::new_v4().to_string()),enroll:Some(w::Enroll{preparation_id:Some(bundle.scope().preparation.clone()),attachment_id:Some(a.ctx.game.attachment.clone()),original_attach_request_id:Some(attach.request_id.clone()),original_attach_fingerprint:Some(attach.fingerprint.clone()),world_token:Some(a.ctx.world.clone()),game_instance_id:Some(a.ctx.instance.clone()),observation_revision:Some(ack.sequence.try_into().map_err(|_|"Waiting observation bound")?),content_digest:Some(hex(&ack.digest)),context_id:Some(hex(&owner.1))}),..Default::default()};
 let captured=journal::Mutation::capture(bundle.scope(),request,&a.ctx.desktop.id)?;
 let (j,marker)={let (store,writer,c,nonce,generation,a,bundle)=(store.clone(),writer.clone(),c.clone(),nonce.clone(),generation.clone(),a.clone(),bundle.clone());tokio::task::spawn_blocking(move||{
  let _guard=writer.lock().unwrap();if !current(&c,&nonce,&generation,&a){return Err("Waiting owner retired".into())}
  use std::io::Read;let mut file=std::fs::File::open(path).map_err(|_|"Waiting claim unavailable")?;if file.metadata().map_err(|_|"Waiting claim metadata")?.len()!=20{return Err("Waiting claim bound".into())}let mut claim=[0;20];file.read_exact(&mut claim).map_err(|_|"Waiting claim read")?;
  let adapter=super::super::rejoin::ResumeStore(store);let marker=startup::persist_with_attach(&adapter,&bundle,actual,owner.0,owner.1,&claim,Some(&attach))?;
  let mut j=journal::load(&adapter,bundle.scope(),&marker)?.unwrap_or_else(||journal::Journal::new(marker.clone()));
  if j.terminal{return Err("Waiting original terminal".into())}if !j.ready()&&j.operation.is_none(){j.operation=Some(captured);journal::save(&adapter,bundle.scope(),&j)?;}
  if !current(&c,&nonce,&generation,&a){return Err("Waiting owner retired during journal".into())}Ok::<_,String>((j,marker))
 }).await.map_err(|_|"Waiting storage worker cancelled")??};
 if j.ready(){return Ok(marker)}
 let op=j.operation.as_ref().ok_or("Waiting original unavailable")?;
 if !op.permits(bundle.scope(),&a.ctx.desktop.id,&a.ctx.desktop.epoch){return Err("Waiting original auth changed; outcome remains uncertain".into())}
 let mut job=original(&op.request,op.request.request_id.as_deref().ok_or("Missing waiting request")?,&a.ctx.desktop,protocol::ENDPOINT,&c)?;
 job.stamp=c.lock().unwrap().request_stamp(&json!([a.ctx.issuer.origin,a.ctx.issuer.service,a.ctx.desktop.id,a.ctx.desktop.epoch,"waiting"]).to_string(),&job.id)?;
 if !current(&c,&nonce,&generation,&a){return Err("Waiting owner retired before send".into())}
 let bytes=call(&http,&a.ctx,&job,&a.ctx.desktop,Instant::now()+Duration::from_secs(3)).await?;let value:Value=serde_json::from_slice(&bytes).map_err(|_|"Invalid waiting response")?;verify(&value,&a.ctx,&job.id)?;let mut response=protocol::response(&bytes)?;
 let state=response.waiting.as_ref().ok_or("Missing canonical waiting state")?;let intent=state.intent.clone().ok_or("Missing canonical waiting intent")?;
 let sample=state.sample.as_ref().ok_or("Missing waiting sample")?;let report=state.current_report.as_ref().ok_or("Missing waiting report")?;let original_owner=state.owner.as_ref().ok_or("Missing waiting owner")?;
 let enroll=op.request.enroll.as_ref().ok_or("Stored original is not enrollment")?;
 let mut failed=enrollment_reports(enroll,&intent,report,&ack);
 for (ok,field) in [
  (state.may_play==Some(false),"waiting.mayPlay"),(state.state.as_deref()==Some("waiting"),"waiting.state"),
  (response.canonical_admission.is_none(),"canonicalAdmission.absent"),(response.credentials.is_none(),"credentials.absent"),
  (sample.channel.as_deref()==Some("waiting-http"),"sample.channel"),(sample.origin_request_id.as_deref()==Some(&job.id),"sample.originRequestId"),
  (sample.credential_id.as_deref()==Some(&admission::token_id(a.ctx.desktop.access.as_str())?),"sample.credentialId"),
  (sample.sample_key.as_ref().is_some_and(|v|v.len()==64&&v.bytes().all(|b|b.is_ascii_hexdigit()&&!b.is_ascii_uppercase())),"sample.sampleKey"),
  (state.current_context_id.as_deref()==Some(&hex(&owner.1)),"waiting.currentContextId"),
  (original_owner.desktop_id.as_deref()==Some(&a.ctx.desktop.id),"owner.desktopId"),(original_owner.desktop_session_epoch.as_deref()==Some(&a.ctx.desktop.epoch),"owner.desktopEpoch"),
  (original_owner.game_session_id.as_deref()==Some(&a.ctx.game.id),"owner.gameSessionId"),(original_owner.game_session_epoch.as_deref()==Some(&a.ctx.game.epoch),"owner.gameEpoch"),
  (original_owner.attachment_id.as_deref()==Some(&a.ctx.game.attachment),"owner.attachmentId"),(original_owner.game_instance_id.as_deref()==Some(&a.ctx.instance),"owner.gameInstanceId"),
  (intent.original_attachment_id.as_deref()==Some(&a.ctx.game.attachment),"intent.currentAttachment"),(intent.original_game_instance_id.as_deref()==Some(&a.ctx.instance),"intent.currentInstance"),
  (intent.world_token.as_deref()==Some(&a.ctx.world),"intent.currentWorld"),(intent.context_id.as_deref()==Some(&hex(&owner.1)),"intent.currentContext")
 ]{if !ok{failed.push(field)}}
 #[cfg(test)]if !failed.is_empty()||enroll.observation_revision.map(u64::from)!=Some(ack.sequence){eprintln!("WAITING_ENROLL_CORRELATION stored_revision={:?} current_ack={} current_reply={:?} immutable_reply={:?} failed={:?}",enroll.observation_revision,ack.sequence,report.revision,intent.original_observation_revision,failed);}
 if !failed.is_empty(){return Err(format!("Waiting enrollment response scope differs: {}",failed.join(",")))}

 let mut validity=nt2_desktop_g0::connection_time::Deadline::default();let until=validity.install(job.stamp,state.review_remaining_ms.ok_or("Missing waiting review duration")?,3600000,3600000,Instant::now())?;if until<=Instant::now(){return Err("Waiting review expired in flight".into())}if state.owner_remaining_ms.is_none_or(|ms|ms>30000){return Err("Waiting owner duration bound".into())}
 let mut committed=j;committed.current_attachment=original_owner.attachment_id.clone();committed.intent=Some(intent);committed.generation=state.generation.filter(|g|*g>0).ok_or("Missing waiting generation")?;committed.proof=response.waiting_proof.take();if committed.proof.is_none(){return Err("Waiting proof missing; original retained".into())}committed.operation=None;committed.validate(bundle.scope())?;
 let marker=marker.clone();tokio::task::spawn_blocking(move||{let _guard=writer.lock().unwrap();if !current(&c,&nonce,&generation,&a){return Err("Waiting owner retired before proof commit".into())}journal::save(&super::super::rejoin::ResumeStore(store),bundle.scope(),&committed)?;if !current(&c,&nonce,&generation,&a){return Err("Waiting owner retired during proof commit".into())}Ok(marker)}).await.map_err(|_|"Waiting commit worker cancelled")?
}
fn enrollment_reports(original:&w::Enroll,intent:&w::WaitingIntent,current:&w::ConnectionReport,ack:&native_report::Latest)->Vec<&'static str>{
 let digest=hex(&ack.digest);let mut failed=vec![];
 for (ok,field) in [
  (intent.preparation_id==original.preparation_id,"intent.preparationId"),
  (intent.original_attachment_id==original.attachment_id,"intent.originalAttachmentId"),
  (intent.original_attach_request_id==original.original_attach_request_id,"intent.originalAttachRequestId"),
  (intent.original_attach_fingerprint==original.original_attach_fingerprint,"intent.originalAttachFingerprint"),
  (intent.original_game_instance_id==original.game_instance_id,"intent.originalGameInstanceId"),
  (intent.world_token==original.world_token,"intent.worldToken"),(intent.context_id==original.context_id,"intent.contextId"),
  // Immutable intent belongs to the persisted original, not a later worker ack.
  (intent.original_observation_revision==original.observation_revision,"intent.originalObservationRevision"),
  (intent.content_digest==original.content_digest,"intent.contentDigest"),
  // Current report still must match an actually acknowledged current report.
  (original.content_digest.as_deref()==Some(&digest),"original.currentContentDigest"),
  (current.revision.map(u64::from)==Some(ack.sequence),"currentReport.revision"),(current.content_digest.as_deref()==Some(&digest),"currentReport.contentDigest"),
  (current.complete==Some(true),"currentReport.complete"),(current.protected==Some(true),"currentReport.protected"),(current.loaded==Some(true),"currentReport.loaded"),(current.content_valid==Some(true),"currentReport.contentValid")
 ]{if !ok{failed.push(field)}}failed
}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn waiting_enroll_retry_keeps_original_revision_and_requires_new_ack(){
  let gold:Value=serde_json::from_str(include_str!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../packages/contracts/g3-waiting/golden.json"))).unwrap();let state:w::WaitingState=serde_json::from_value(gold["fixtures"][0]["json"]["waiting"].clone()).unwrap();let intent=state.intent.unwrap();let mut report=state.current_report.unwrap();
  let original=w::Enroll{preparation_id:intent.preparation_id.clone(),attachment_id:intent.original_attachment_id.clone(),original_attach_request_id:intent.original_attach_request_id.clone(),original_attach_fingerprint:intent.original_attach_fingerprint.clone(),world_token:intent.world_token.clone(),game_instance_id:intent.original_game_instance_id.clone(),observation_revision:intent.original_observation_revision,content_digest:intent.content_digest.clone(),context_id:intent.context_id.clone()};
  let digest=intent.content_digest.as_ref().unwrap().as_bytes().chunks_exact(2).map(|b|u8::from_str_radix(std::str::from_utf8(b).unwrap(),16).unwrap()).collect::<Vec<_>>().try_into().unwrap();
  let mut ack=native_report::Latest{context:[1;16],sequence:u64::from(original.observation_revision.unwrap()),digest,observed_at:Instant::now(),observation:native_report::Observation{actual_seed:intent.actual_seed.clone(),mods:vec![],complete:true,error:None,beta:Some(false),loaded:true,protected:true,mod_version:"fixture".into(),bridge_version:"fixture".into(),..Default::default()}};
  assert!(enrollment_reports(&original,&intent,&report,&ack).is_empty());ack.sequence+=9;report.revision=Some(ack.sequence as u32);
  assert!(enrollment_reports(&original,&intent,&report,&ack).is_empty(),"A storage/lost-reply retry must not replace the original enrollment revision");
  let mut wrong=intent.clone();wrong.original_observation_revision=report.revision;assert!(enrollment_reports(&original,&wrong,&report,&ack).contains(&"intent.originalObservationRevision"));
  report.revision=original.observation_revision;assert!(enrollment_reports(&original,&intent,&report,&ack).contains(&"currentReport.revision"));report.revision=Some(ack.sequence as u32);
  let mut wrong=intent.clone();wrong.original_attach_fingerprint=Some("f".repeat(64));assert!(enrollment_reports(&original,&wrong,&report,&ack).contains(&"intent.originalAttachFingerprint"));
  report.protected=Some(false);assert!(enrollment_reports(&original,&intent,&report,&ack).contains(&"currentReport.protected"));report.protected=Some(true);report.complete=Some(false);assert!(enrollment_reports(&original,&intent,&report,&ack).contains(&"currentReport.complete"));report.complete=Some(true);ack.digest=[0;32];assert!(enrollment_reports(&original,&intent,&report,&ack).contains(&"original.currentContentDigest"));
 }
}
