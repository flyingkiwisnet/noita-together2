//! Disposable canonical TLS fixture; native engine facts are explicitly injected.
use super::*;
#[test]
#[ignore="explicit disposable two-client A1 TLS attachment revision regression; injected native facts"]
fn stale_attachment_rejection_refreshes_without_replaying_rejected_request(){
 assert_eq!(std::env::var("NT2_G3_SOCIAL_SOURCE_TEST").as_deref(),Ok("1"));
 tauri::async_runtime::block_on(async{
  let repo=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..").canonicalize().unwrap();
  let repo=std::path::PathBuf::from(repo.to_str().unwrap().trim_start_matches(r"\\?\"));
  let mut service=ServiceFixture::start(&repo,true);
  let ca=reqwest::Certificate::from_pem(&std::fs::read(&service.ca).unwrap()).unwrap();
  let dir=repo.join("spikes/noita-entry/.build/g3-inspection-stale").join(uuid::Uuid::new_v4().to_string());
  std::fs::create_dir_all(&dir).unwrap();
  let mut host=core();let mut peer=core();
  for (i,c) in [&mut host,&mut peer].into_iter().enumerate(){
   c.http=reqwest::Client::builder().tls_backend_native().tls_certs_only(vec![ca.clone()]).no_proxy().redirect(reqwest::redirect::Policy::none()).timeout(Duration::from_secs(3)).build().unwrap();
   c.enable_g3(&dir.join(format!("{i}.journal")),Arc::new(Mutex::new(nt2_desktop_g0::g3_runtime::Exchange::default()))).unwrap();
   c.request_social(Arc::new(Mutex::new(nt2_desktop_g0::g3_social_stream::Exchange::default()))).unwrap();
   c.select(&service.origin,"",false).await.unwrap();registered_as(c,if i==0{"alex"}else{"sam"}).await;
   // Read-only reuse of an owned verified file fixture, never the real install.
   c.configure_game(repo.join("spikes/noita-entry/.build/g3-joined/4fb992c0-0d49-4030-a417-57ea4ad756bc/0/verified-files").to_str().unwrap()).unwrap();
  }
  host.create_fixture(false).await.unwrap();peer.join(host.room.as_ref().unwrap().room_id.as_deref().unwrap(),None).await.unwrap();
  room(&mut host).await;host.ready(true).await.unwrap();room(&mut peer).await;peer.ready(true).await.unwrap();room(&mut host).await;
  host.load_game().await.unwrap();fixture_metadata_only(&mut host);
  let original_until=host.attach_retry.as_ref().unwrap().until;let original_request=host.attach_retry.as_ref().unwrap().request.clone();
  let instance=original_request.game_attach.as_ref().unwrap().game_instance_id.clone().unwrap();
  // Independent peer Load happens after host's snapshot, before its attach RPC.
  room(&mut peer).await;peer.load_game().await.unwrap();
  inject_social_observation(&mut host);
  assert_eq!(host.game_tick().await.unwrap_err(),"STALE_REVISION");
  println!("A1_STALE_REJECTION exact_retry_retained={}",host.attach_retry.is_some());
  assert!(host.attach_retry.is_none(),"A deterministic stale rejection must not be replayed until expiry");
  assert_eq!(host.attach_cycle.as_ref().unwrap(),&(instance.clone(),original_until));
  // A second real race: delay the refreshed room response while the independent
  // peer changes ready status. Deadline/instance must survive both rejections.
  service.command("delay-room-next");
  let fault=std::thread::spawn(move||{
   assert_eq!(service.event()["kind"],"room");
   tauri::async_runtime::block_on(async{room(&mut peer).await;peer.ready(false).await.unwrap();});
   service.command("release");(peer,service)
  });
  assert_eq!(host.game_tick().await.unwrap_err(),"STALE_REVISION");
  (peer,service)=fault.join().unwrap();
  assert!(host.attach_retry.is_none());assert_eq!(host.attach_cycle.as_ref().unwrap(),&(instance.clone(),original_until));
  host.next_work=Instant::now();host.tick().await.unwrap();
  assert_eq!(host.game.as_ref().unwrap().reported,1);
  assert_eq!(host.game.as_ref().unwrap().instance,instance);
  inject_social_observation(&mut peer);room(&mut peer).await;peer.ready(true).await.unwrap();
  service.command("drop-attach-next");
  let dropped=peer.game_tick().await;println!("A1_DROP_RESULT {:?}",dropped);assert!(dropped.is_err());assert_eq!(service.event()["event"],"dropped-attach");
  let ambiguous_until=peer.attach_retry.as_ref().unwrap().until;let ambiguous_request=peer.attach_retry.as_ref().unwrap().request.clone();
  assert_eq!(peer.attach_cycle.as_ref().unwrap().1,ambiguous_until);
  // No changed request ID/payload/deadline after committed-but-lost response.
  assert_eq!(serde_json::to_vec(&peer.attach_retry.as_ref().unwrap().request).unwrap(),serde_json::to_vec(&ambiguous_request).unwrap());
  peer.next_work=Instant::now();peer.tick().await.unwrap();
  let replay=service.event();assert_eq!(replay["event"],"replayed-attach");assert_eq!(replay["exactRequest"],true);
  assert_eq!(peer.game.as_ref().unwrap().instance,ambiguous_request.game_attach.as_ref().unwrap().game_instance_id.as_deref().unwrap());
  for _ in 0..3{host.collect_inspection().await.unwrap();}
  assert_eq!(host.inspections.len(),2);
  assert!(host.inspections.iter().all(|r|r.actual_seed.as_deref()==Some("424242")));
  std::fs::write(dir.join("result.json"),serde_json::to_vec_pretty(&json!({"passed":true,"actualNoita":false,"injectedNativeFacts":true,"verifiedTls":true,"independentPeerLoadChangesRevision":true,"staleRejectedRequestRetired":true,"repeatedStalePreservesFirstDeadlineAndInstance":true,"ambiguousCommittedReplyRecoversExactRequest":true,"bothServiceInspections":2,"guardsRetained":true})).unwrap()).unwrap();
  println!("A1_INSPECTION_STALE_EVIDENCE {}",dir.display());
 });
}
fn inject_social_observation(c:&mut Core<MemoryStore>){
 native(c,1,true,false);
 let mut pending=c.pending.lock().unwrap();let report=pending.as_mut().unwrap().observation.as_mut().unwrap();
 report.observation.mod_version="nt2-g3-social-dev-1".into();report.observation.bridge_version="g3-social-transport-1".into();report.digest=report.observation.content_digest().unwrap();
}
#[test]
#[ignore="explicit public same-origin forged social profile refusal in disposable TLS fixture"]
fn social_discovery_failure_revokes_preparation_and_keeps_durable_reason(){
 assert_eq!(std::env::var("NT2_G3_SOCIAL_SOURCE_TEST").as_deref(),Ok("1"));
 tauri::async_runtime::block_on(async{
  let repo=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..").canonicalize().unwrap();
  let repo=std::path::PathBuf::from(repo.to_str().unwrap().trim_start_matches(r"\\?\"));
  let mut fixture=ServiceFixture::start(&repo,true);let mut c=core();
  let ca=reqwest::Certificate::from_pem(&std::fs::read(&fixture.ca).unwrap()).unwrap();
  c.http=reqwest::Client::builder().tls_backend_native().tls_certs_only(vec![ca]).no_proxy().redirect(reqwest::redirect::Policy::none()).timeout(Duration::from_secs(3)).build().unwrap();
  let journal=repo.join("spikes/noita-entry/.build").join(format!("g3-inspection-profile-{}.journal",uuid::Uuid::new_v4()));
  c.enable_g3(&journal,Arc::new(Mutex::new(nt2_desktop_g0::g3_runtime::Exchange::default()))).unwrap();
  c.request_social(Arc::new(Mutex::new(nt2_desktop_g0::g3_social_stream::Exchange::default()))).unwrap();
  c.select(&fixture.origin,"",false).await.unwrap();registered_as(&mut c,"alex").await;
  c.configure_game(repo.join("spikes/noita-entry/.build/g3-joined/4fb992c0-0d49-4030-a417-57ea4ad756bc/0/verified-files").to_str().unwrap()).unwrap();
  c.create_fixture(false).await.unwrap();c.ready(true).await.unwrap();c.load_game().await.unwrap();
  assert!(c.social_enabled&&c.pending.lock().unwrap().is_some());
  fixture.command("bad-social-info-next");
  assert!(c.select(&fixture.origin,"",false).await.unwrap_err().contains("runtime metadata differs"));
  assert!(!c.social_enabled&&c.social_verified.is_none()&&c.pending.lock().unwrap().is_none()&&c.preparation.is_none());
  assert_eq!(c.view()["inspection_status"]["current"],false);
  let reason=c.view()["inspection_status"]["failure"].clone();
  c.next_work=Instant::now();c.tick().await.unwrap();
  assert_eq!(c.view()["inspection_status"]["failure"],reason);
  println!("A1_FORGED_DISCOVERY refused=true revoked=true durable_after_successful_poll=true actualNoita=false");
 });
}
