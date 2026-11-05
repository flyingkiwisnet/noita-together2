use super::*;
use nt2_desktop_g0::{rejoin_local as local,secrets as protected};

pub(super) struct StoreBridge<'a>(pub(super) &'a MemoryStore);
impl protected::CredentialStore for StoreBridge<'_>{
 fn save(&self,p:&protected::ServiceProfile,b:&protected::SecretBlob)->Result<(),protected::StoreError>{let p=ServiceProfile::new(p.origin(),p.service_id(),p.profile_id()).unwrap();self.0.save(&p,&SecretBlob::new(b.bytes().to_vec()).unwrap()).map_err(|_|protected::StoreError::Unavailable)}
 fn load(&self,p:&protected::ServiceProfile)->Result<Option<protected::SecretBlob>,protected::StoreError>{let p=ServiceProfile::new(p.origin(),p.service_id(),p.profile_id()).unwrap();self.0.load(&p).map_err(|_|protected::StoreError::Unavailable)?.map(|b|protected::SecretBlob::new(b.bytes().to_vec())).transpose()}
 fn delete(&self,p:&protected::ServiceProfile)->Result<(),protected::StoreError>{let p=ServiceProfile::new(p.origin(),p.service_id(),p.profile_id()).unwrap();self.0.delete(&p).map_err(|_|protected::StoreError::Unavailable)}
}
fn index(c:&Core<MemoryStore>)->ServiceProfile{use sha2::Digest;let key=sha2::Sha256::digest(serde_json::to_vec(&("nt2-rejoin-index-v1",c.device,&c.desktop.as_ref().unwrap().account)).unwrap());ServiceProfile::new(&c.base,&c.service,uuid::Uuid::from_bytes(key[..16].try_into().unwrap())).unwrap()}
pub(super) fn prior(c:&Core<MemoryStore>)->local::Association {
 local::Association{version:1,local_world:uuid::Uuid::new_v4(),origin:c.base.clone(),service:c.service.clone(),run:uuid::Uuid::new_v4().to_string(),participant:uuid::Uuid::new_v4().to_string(),room:uuid::Uuid::new_v4().to_string(),world_token:"1".repeat(32),account:c.desktop.as_ref().unwrap().account.clone(),original_start:uuid::Uuid::new_v4().to_string(),config_revision:1,actual_seed:"424242".into(),seed_policy:"shared-fixed".into(),loot_policy:"shared:0:424242".into(),frozen_config_digest:"a".repeat(64)}
}
pub(super) fn remember(c:&Core<MemoryStore>,a:&local::Association){
 local::remember(&StoreBridge(&c.store),a,&protected::SecretBlob::new(vec![b'A';43]).unwrap()).unwrap();
 c.store.save(&index(c),&SecretBlob::new(a.encode().unwrap()).unwrap()).unwrap();
}
fn contact(c:&mut Core<MemoryStore>,a:&local::Association){
 let mut lookup=vec![0;32];lookup[..4].copy_from_slice(&handoff::MAGIC.to_le_bytes());lookup[4..8].copy_from_slice(&8u32.to_le_bytes());lookup[8..12].copy_from_slice(&14u32.to_le_bytes());lookup[12..16].copy_from_slice(&(a.encode().unwrap().len() as u32).to_le_bytes());lookup[16..32].copy_from_slice(&[7;16]);lookup.extend(a.encode().unwrap());
 let mut request=vec![0;56];request[..4].copy_from_slice(&handoff::MAGIC.to_le_bytes());request[4..8].copy_from_slice(&7u32.to_le_bytes());request[8..12].copy_from_slice(&12u32.to_le_bytes());request[16..32].copy_from_slice(&[7;16]);request[32..48].copy_from_slice(a.local_world.as_bytes());request[48..56].copy_from_slice(&424242u64.to_le_bytes());
 let pending=c.pending.lock().unwrap().clone();let mut d=c.native_diagnostics.lock().unwrap();assert_eq!(d.rejoin.transition(&pending,nt2_local_ws::owner::Id::OsProcess(123),&lookup)[8],1);assert_eq!(d.rejoin.transition(&pending,nt2_local_ws::owner::Id::OsProcess(123),&request)[12],3);
}
#[test]
fn cf05_restored_choice_and_stale_contact_cannot_preempt_explicit_join_retry(){tauri::async_runtime::block_on(async{
 let(mut c,room,prep)=client();let desktop=c.desktop.as_ref().unwrap();
 let credentials=json!({"kind":"desktop","deviceId":c.device.to_string(),"principal":desktop.principal,"accountId":desktop.account,"sessionId":desktop.id,"sessionEpoch":desktop.epoch,"accessToken":"fixture-access","refreshSecret":"A".repeat(43),"expiresAt":later(900000)});
 let mut preview=data(&room,&prep);preview["admission"]=json!({"roomId":room.room_id,"revision":room.revision,"configRevision":room.config_revision});
 let baseline=std::env::var("NT2_CF05_BASELINE").as_deref()==Ok("1");
 let mut replies=vec![Reply::Data(json!({"rooms":[]})),Reply::Data(json!({"credentials":credentials})),Reply::Data(preview),Reply::Lost];
 if !baseline{for _ in 0..9{replies.push(Reply::Data(data(&room,&prep)));}}
 let(origin,worker)=network(replies);set_origin(&mut c,&origin);let a=prior(&c);remember(&c,&a);
 c.next_work=Instant::now();c.tick().await.unwrap(); // Ordinary sign-in state imports index offer.
 assert_eq!(c.resume_intent.phase(),local::Phase::Choice);
 c.rejoin_previous().unwrap(); // Actual app-first action; no native world yet.
 contact(&mut c,&a); // Inject only an old game's bounded mailbox frames.
 c.guest("Alex").await.unwrap();inject_credential_timing(&mut c); // Production sign-in installation clears native state.
 assert!(c.pending.lock().unwrap().is_none()&&c.game.is_none());
 let before=c.view();
 c.preview_lobby(room.room_id.as_deref().unwrap()).await.unwrap();
 assert!(join(&mut c).await.is_err()); // Ambiguous original Join remains retained.
 let original=serde_json::to_vec(&c.setup.intent.as_ref().unwrap().operation.as_ref().unwrap().request).unwrap();
 if !baseline{contact(&mut c,&a);} // Old frames arrive after the explicit Join/lost reply.
 wake(&mut c);c.next_work=Instant::now();let error=c.tick().await.err();
 if !baseline{
  let grant=c.pending.lock().unwrap().as_ref().unwrap().preparation_id.clone();
  contact(&mut c,&a); // Repeat after fresh arm, while its owner is still None.
  wake(&mut c);c.next_work=Instant::now();c.tick().await.unwrap();
  assert_eq!(c.pending.lock().unwrap().as_ref().unwrap().preparation_id,grant);
  assert!(c.rejoin_transport.contact.is_none()&&c.native_diagnostics.lock().unwrap().rejoin.contact.is_none());
  assert!(c.rejoin_previous().unwrap_err().contains("Fresh lobby setup owns this session"));
 }
 let calls=worker.join().unwrap();
 let evidence=json!({"baseline":baseline,"beforeRejoin":before["rejoin"],"beforeNoCurrentNative":before["inspection_status"]["current"]==false,"tickError":error,"recoveryPreemptedJoin":c.rejoin_transport.contact.is_some()&&calls.len()==4,"originalJoinRetried":calls.get(4).is_some_and(|v|v["roomJoin"].is_object()),"pendingFresh":c.pending.lock().unwrap().as_ref().is_some_and(|p|p.owner.is_none()),"lateFramesAfterJoinAndArm":!baseline,"engineFactsInjected":true});
 if let Some(path)=std::env::var_os("NT2_CF05_EVIDENCE"){std::fs::write(path,serde_json::to_vec_pretty(&evidence).unwrap()).unwrap();}
 if baseline{
  assert_eq!(before["rejoin"]["held"],true);assert_eq!(before["rejoin"]["intent_retained"],true);
  assert_eq!(calls.len(),4);assert!(c.rejoin_transport.contact.is_some());assert!(c.pending.lock().unwrap().is_none());assert!(c.setup.intent.as_ref().unwrap().operation.is_some());
 }else{
  assert_eq!(before["rejoin"]["held"],false);assert!(error.is_none(),"{evidence}");
  assert_eq!(calls[4],serde_json::from_slice::<Value>(&original).unwrap(),"retry original Join payload/request identity");assert_eq!(calls[3],calls[4]);
  assert!(calls.iter().all(|v|v["resume"].is_null()));assert!(c.pending.lock().unwrap().as_ref().is_some_and(|p|p.owner.is_none()));
  assert!(!local::is_retired(&StoreBridge(&c.store),&a).unwrap());assert!(local::recall(&StoreBridge(&c.store),&a).unwrap().is_some());
 }
 clean(c);
});}

#[test]
fn cf05_uncertain_resume_blocks_join_without_replacing_original_or_proof(){tauri::async_runtime::block_on(async{
 let(mut c,_,_)=client();let(origin,worker)=network(vec![Reply::Data(json!({"rooms":[]})),Reply::Lost]);set_origin(&mut c,&origin);let mut a=prior(&c);a.room=c.setup.preview.as_ref().unwrap().room.clone();remember(&c,&a);
 c.next_work=Instant::now();c.tick().await.unwrap();c.rejoin_previous().unwrap();contact(&mut c,&a);
 c.next_work=Instant::now();c.tick().await.unwrap(); // New native contact binds recovery, then backoff.
 tokio::time::sleep(Duration::from_millis(2100)).await;c.next_work=Instant::now();assert!(c.tick().await.is_err());
 let original=c.resume_intent.original_operation().unwrap().to_vec();let proof=local::recall(&StoreBridge(&c.store),&a).unwrap().unwrap();
 let error=join(&mut c).await.unwrap_err();assert!(error.contains("outcome is unresolved"));
 assert_eq!(c.resume_intent.original_operation(),Some(original.as_slice()));assert!(c.setup.error.as_ref().unwrap().contains("outcome is unresolved"));
 assert_eq!(local::recall(&StoreBridge(&c.store),&a).unwrap().unwrap().bytes(),proof.bytes());
 assert!(!local::is_retired(&StoreBridge(&c.store),&a).unwrap());assert_eq!(worker.join().unwrap().len(),2);clean(c);
});}

#[test]
fn cf05_uncertain_offline_retirement_blocks_join_without_replacing_original(){tauri::async_runtime::block_on(async{
 let(mut c,_,_)=client();let mut a=prior(&c);a.room=c.setup.preview.as_ref().unwrap().room.clone();
 let association=json!({"serviceId":a.service,"roomId":a.room,"runId":a.run,"participantId":a.participant,"worldToken":a.world_token,"accountId":a.account,"originalStartId":a.original_start,"configRevision":a.config_revision,"actualSeed":a.actual_seed,"seedPolicy":a.seed_policy,"randomizeLoot":false});
 let(origin,worker)=network(vec![Reply::Data(json!({"rooms":[]})),Reply::Data(json!({"participant":{"association":association,"generation":1,"state":"active","returnEligible":true}})),Reply::Lost]);set_origin(&mut c,&origin);a.origin=c.base.clone();remember(&c,&a);
 c.next_work=Instant::now();c.tick().await.unwrap();c.continue_offline().unwrap();
 c.next_work=Instant::now();assert!(c.tick().await.is_err());let original=c.rejoin_transport.offline.as_ref().unwrap().to_vec();
 let error=join(&mut c).await.unwrap_err();assert!(error.contains("offline request is still being confirmed"));
 assert_eq!(c.rejoin_transport.offline.as_ref().unwrap().as_slice(),original.as_slice());
 assert!(local::retired_proof(&StoreBridge(&c.store),&a).unwrap().is_some());assert_eq!(worker.join().unwrap().len(),3);clean(c);
});}

#[test]
fn cf05_projection_needs_current_associated_native_observation(){
 for fault in 0..11 {
  let(mut c,_,_)=release_case();c.base="https://fixture.invalid".into();let mut a=prior(&c);
  {let mut lock=c.pending.lock().unwrap();let p=lock.as_mut().unwrap();p.actual_seed=Some(424242);a.run=p.run.clone();a.config_revision=p.config as u32;a.world_token=hex(&p.owner.unwrap().1);}
  a.room=c.room.as_ref().unwrap().room_id.clone().unwrap();remember(&c,&a);c.offer_saved_world(&a.encode().unwrap()).unwrap();c.rejoin_previous().unwrap();
  match fault{
   0=>{},
   1=>c.pending.lock().unwrap().as_mut().unwrap().last_seen=Some(Instant::now()-Duration::from_secs(13)),
   2=>{c.pending.lock().unwrap().take();},
   3=>c.game=None,
   4=>c.pending.lock().unwrap().as_mut().unwrap().owner=Some((nt2_local_ws::owner::Id::OsProcess(99),[9;16])),
   5=>{c.generation.fetch_add(1,Ordering::AcqRel);},
   6=>c.pending.lock().unwrap().as_mut().unwrap().observation.as_mut().unwrap().observation.protected=false,
   7=>c.pending.lock().unwrap().as_mut().unwrap().observation.as_mut().unwrap().context=[8;16],
   8=>c.pending.lock().unwrap().as_mut().unwrap().observation.as_mut().unwrap().observation.complete=false,
   9=>c.pending.lock().unwrap().as_mut().unwrap().accepted=false,
   _=>c.pending.lock().unwrap().as_mut().unwrap().observation.as_mut().unwrap().observation.actual_seed=Some("777".into()),
  }
  let view=c.view();assert_eq!(view["rejoin"]["held"],fault==0,"projection fault {fault}");
 }
}

#[test]
fn cf05_unsent_offline_request_also_preserves_its_pending_proof(){tauri::async_runtime::block_on(async{
 let(mut c,_,_)=client();set_origin(&mut c,"https://fixture.invalid");let mut a=prior(&c);a.room=c.setup.preview.as_ref().unwrap().room.clone();remember(&c,&a);
 c.offer_saved_world(&a.encode().unwrap()).unwrap();c.continue_offline().unwrap();
 assert!(c.rejoin_transport.offline.is_none()); // Original status/command not sent yet.
 assert!(join(&mut c).await.unwrap_err().contains("offline request is still being confirmed"));
 assert!(local::retired_proof(&StoreBridge(&c.store),&a).unwrap().is_some());
 assert!(c.pending.lock().unwrap().is_none());clean(c);
});}

#[test]
fn cf05_unresolved_join_cannot_be_replaced_by_a_different_lobby(){tauri::async_runtime::block_on(async{
 let(mut c,room,_)=client();let mut other=room.clone();other.room_id=Some("different-reviewed-room".into());
 let preview=json!({"room":other,"admission":{"roomId":other.room_id,"revision":other.revision,"configRevision":other.config_revision}});
 let(origin,worker)=network(vec![Reply::Lost,Reply::Data(preview)]);set_origin(&mut c,&origin);
 assert!(join(&mut c).await.is_err());let original=serde_json::to_vec(&c.setup.intent.as_ref().unwrap().operation.as_ref().unwrap().request).unwrap();
 c.preview_lobby(other.room_id.as_deref().unwrap()).await.unwrap();
 assert!(join(&mut c).await.unwrap_err().contains("Original lobby request outcome is unresolved"));
 assert_eq!(serde_json::to_vec(&c.setup.intent.as_ref().unwrap().operation.as_ref().unwrap().request).unwrap(),original);
 assert!(c.pending.lock().unwrap().is_none());assert_eq!(worker.join().unwrap().len(),2);clean(c);
});}
