use super::*;
use super::super::setup::tests::{network,Reply};
use base64::Engine;

fn room(ctx:&admission::Context,running:bool)->Value{
 json!({"room":{"roomId":ctx.room,"runId":ctx.run,"runEpoch":ctx.run_epoch,"configRevision":ctx.config,"state":if running{"running"}else{"staging"},"members":[{"desktopId":ctx.desktop.id,"accountId":ctx.game.account,"included":true,"present":true,"consentedConfigRevision":ctx.config,"ready":true}]},"gameStatus":{"phase":if running{"authorized"}else{"hold"},"reason":if running{""}else{"awaiting_final_start"}}})
}
fn sampled_room(q:&Value,ctx:&admission::Context,running:bool)->Value{let mut r=room(ctx,running);r["credentialValidity"]=json!({"principalId":ctx.game.account,"sessionId":ctx.game.id,"sessionEpoch":ctx.game.epoch,"credentialId":"owner-health-game","originRequestId":q["requestId"],"sampleKey":"f".repeat(64),"accessRemainingMs":900000});r}
fn room_reply(ctx:&admission::Context,running:bool)->Reply{let ctx=ctx.clone();Reply::Build(Box::new(move|q|sampled_room(q,&ctx,running)))}
fn worker(origin:&str,mut ctx:admission::Context,mut p:handoff::Pending)->(Core<super::super::tests::MemoryStore>,HealthRunner<super::super::tests::MemoryStore>,Active,handoff::Pending){
 let(mut core,_,_)=super::super::tests::release_case();ctx.issuer.origin=origin.into();ctx.game.access=SecretString::from(format!("fixture.{}.unsigned",base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(json!({"jti":"owner-health-game"}).to_string())));
 let runner=core.health_runner();let mut a=runner.coordinator.lock().unwrap().active.clone().unwrap();a.ctx=ctx.clone();a.reported=0;a.capture_done=true;a.report=None;a.renew=None;a.force_renew=false;
 let world=lifecycle::StableWorld{issuer:ctx.issuer.clone(),room:ctx.room.clone(),run:ctx.run.clone(),participant:ctx.participant.clone().unwrap_or_default(),world:ctx.world.clone()};
 {let mut co=runner.coordinator.lock().unwrap();co.saved(world.clone());a.ticket=co.bind(lifecycle::Owner{world,native:p.owner.unwrap(),session:ctx.desktop.id.clone(),generation:a.generation}).unwrap();co.active=Some(a.clone());co.request_sequence=1;
 let v=c::ConnectionCredential{principal_id:Some(ctx.game.account.clone()),session_id:Some(ctx.game.id.clone()),session_epoch:Some(ctx.game.epoch.clone()),credential_id:Some("owner-health-game".into()),origin_request_id:Some("initial-health".into()),sample_key:Some("d".repeat(64)),access_remaining_ms:Some(900000)};
 admission::credential_sample(&mut co.credentials,&ctx.issuer.origin,&ctx.issuer.service,&ctx.game,&v,"initial-health",Stamp{sequence:1,started:Instant::now()}).unwrap();}
 p.release=false;p.generation=Some((core.generation.clone(),a.generation));*runner.pending.lock().unwrap()=Some(p.clone());(core,runner,a,p)
}

#[test]fn se1_owner_staging_health_waits_without_admission_then_reads_actual_start(){tauri::async_runtime::block_on(async{
 let(_,ctx,p,mut canonical,_)=super::super::admission::cr2_tests::fixture();canonical.sample.as_mut().unwrap().credential_id=Some("owner-health-game".into());
 let validity=c::ConnectionCredential{principal_id:Some(ctx.game.account.clone()),session_id:Some(ctx.game.id.clone()),session_epoch:Some(ctx.game.epoch.clone()),credential_id:Some("owner-health-game".into()),origin_request_id:Some("retagged".into()),sample_key:Some("e".repeat(64)),access_remaining_ms:Some(900000)};
 let(origin,server)=network(vec![Reply::Data(json!({})),Reply::Data(json!({})),room_reply(&ctx,false),Reply::Data(json!({})),room_reply(&ctx,false),Reply::Data(json!({})),room_reply(&ctx,true),Reply::Data(json!({"admission":canonical,"credentialValidity":validity}))]);
 let(_core,runner,mut a,p)=worker(&origin,ctx,p);
 for _ in 0..2{runner.pending.lock().unwrap().as_mut().unwrap().authority_until=Instant::now();runner.work(&mut a,&p,Instant::now()+Duration::from_secs(3)).await.unwrap();let slot=runner.pending.lock().unwrap();let current=slot.as_ref().unwrap();assert!(!current.release);assert!(current.authority_until>Instant::now()+Duration::from_secs(5)&&current.authority_until<=Instant::now()+Duration::from_secs(6));drop(slot);assert!(!runner.coordinator.lock().unwrap().authority.offered);}
 runner.work(&mut a,&p,Instant::now()+Duration::from_secs(3)).await.unwrap();assert!(runner.pending.lock().unwrap().as_ref().unwrap().release);assert!(runner.coordinator.lock().unwrap().authority.offered);assert!(!runner.coordinator.lock().unwrap().authority.entered);
 let calls=server.join().unwrap();assert_eq!(calls.len(),8);assert!(calls[0]["gameObservation"].is_object());assert!(calls[1]["gamePulse"].is_object()&&calls[2]["roomGet"].is_object());assert!(calls[3]["gamePulse"].is_object()&&calls[4]["roomGet"].is_object());assert!(calls[5]["gamePulse"].is_object()&&calls[6]["roomGet"].is_object()&&calls[7]["admission"].is_object());
});}

#[test]fn se1_owner_staging_health_refusal_scope_expiry_and_exclusion_remain_errors(){tauri::async_runtime::block_on(async{
 for fault in ["forbidden","scope","expired","excluded","member","malformed","credential_expired","credential_missing","native_lost","context_retired","generation_retired"]{
  let(_,ctx,p,_,_)=super::super::admission::cr2_tests::fixture();let mut status=room(&ctx,false);
  match fault{"scope"=>status["room"]["runEpoch"]=json!(uuid::Uuid::new_v4().to_string()),"expired"=>{status["gameStatus"]["phase"]=json!("revoked");status["gameStatus"]["reason"]=json!("waiting_authority_expired")},"excluded"=>status["gameStatus"]["reason"]=json!("excluded"),"member"=>status["room"]["members"][0]["consentedConfigRevision"]=json!(ctx.config+1),"malformed"=>{status.as_object_mut().unwrap().remove("gameStatus");},_=>{}}
  type Slot=Arc<Mutex<Option<Arc<Mutex<Option<handoff::Pending>>>>>>;let slot:Slot=Default::default();let mutate=slot.clone();let sampled=ctx.clone();
  let last=if fault=="forbidden"{Reply::Error("FORBIDDEN")}else if matches!(fault,"credential_expired"|"credential_missing"|"native_lost"|"context_retired"|"generation_retired"){Reply::Build(Box::new(move|q|{let mut r=sampled_room(q,&sampled,false);if fault=="credential_expired"{r["credentialValidity"]["accessRemainingMs"]=json!(0)}if fault=="credential_missing"{r.as_object_mut().unwrap().remove("credentialValidity");}let held=mutate.lock().unwrap();let mut pending=held.as_ref().unwrap().lock().unwrap();let p=pending.as_mut().unwrap();if fault=="native_lost"{p.last_seen=Some(Instant::now()-Duration::from_secs(20));}if fault=="context_retired"{p.owner=Some((nt2_local_ws::owner::Id::OsProcess(999),[8;16]));}if fault=="generation_retired"{p.generation.as_ref().unwrap().0.fetch_add(1,Ordering::AcqRel);}r}))}else{Reply::Data(status)};
  let(origin,server)=network(vec![Reply::Data(json!({})),Reply::Data(json!({})),last]);let(_core,runner,mut a,p)=worker(&origin,ctx,p);*slot.lock().unwrap()=Some(runner.pending.clone());let original_until=runner.pending.lock().unwrap().as_ref().unwrap().authority_until;
  let e=runner.work(&mut a,&p,Instant::now()+Duration::from_secs(3)).await.unwrap_err();if fault=="forbidden"{assert_eq!(e,"FORBIDDEN")}let current=runner.pending.lock().unwrap();assert!(!current.as_ref().unwrap().release);assert_eq!(current.as_ref().unwrap().authority_until,original_until,"refused/stale response extended collection lease");drop(current);assert!(!runner.coordinator.lock().unwrap().authority.offered);assert_eq!(server.join().unwrap().len(),3);
 }
});}
