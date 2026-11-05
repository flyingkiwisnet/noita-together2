use super::*;
#[test]
#[ignore="explicit C3 matched metadata/private native/actual Core TLS WSS; injected engine facts"]
fn rejoin_rj2_actual_core_live_and_saved_continue(){
 assert_eq!(std::env::var("NT2_C3_COMPOSITION").as_deref(),Ok("1"));
 assert!(std::env::var_os("NT2_C3_ENTRY_PATH").is_none(),"composition must use exact verified mapping, no entry override");
 assert!(std::env::var_os("NT2_C3_DIAGNOSTIC_PIN").is_none(),"composition must rely on entry-owned images");
 tauri::async_runtime::block_on(async {
  let repo=repo();let dir=repo.join("spikes/noita-entry/.build/g3-rejoin-composition").join(uuid::Uuid::new_v4().to_string());std::fs::create_dir_all(&dir).unwrap();
  let mut fixture=Fixture::start_with(true);let mut native=Native{children:vec![],workers:vec![]};
  let ca=reqwest::Certificate::from_pem(&std::fs::read(&fixture.ca).unwrap()).unwrap();
  let runtime=nt2_desktop_g0::g3_social_installation::development_profile().unwrap();
  assert_eq!(runtime["profile"]["modVersion"],"nt2-g3-c2-dev-1","C3 requires Server's matched C2 resource");
  let entry_sha=runtime["profile"]["modSha256"].as_str().unwrap();
  let runner=repo.join("spikes/noita-entry/.build/Lua51Runner.exe");let expected=crate::pipe::verified_process_path(&runner).unwrap();
  let mut host=core();let mut peer=core();
  for(index,c)in [&mut host,&mut peer].into_iter().enumerate(){
   let own=dir.join(index.to_string());let game=own.join("verified-files");std::fs::create_dir_all(&game).unwrap();
   c.http=reqwest::Client::builder().use_native_tls().tls_certs_only(vec![ca.clone()]).no_proxy().redirect(reqwest::redirect::Policy::none()).timeout(Duration::from_secs(3)).build().unwrap();
   let exchange=Arc::new(Mutex::new(nt2_desktop_g0::g3_runtime::Exchange::default()));let social=Arc::new(Mutex::new(nt2_desktop_g0::g3_social_stream::Exchange::default()));
   c.enable_g3(&own.join("journal.jsonl"),exchange).unwrap();c.enable_live_transport().unwrap();c.request_social(social).unwrap();
   c.enable_automatic_setup();if let Err(error)=c.select(&fixture.origin,"",false).await{
    let info=c.http.get(format!("{}/v1/info",fixture.origin)).send().await.unwrap().bytes().await.unwrap();
    std::fs::write(own.join("info-diagnostic.json"),&info).unwrap();panic!("select {error}; protocol detail {:?}; public info {}",protocol::info(&info).err(),String::from_utf8_lossy(&info));
   }registered_as(c,if index==0{"alex"}else{"sam"}).await;
   for name in ["noita.exe","lua51.dll"]{std::fs::copy(Path::new("C:/Noita").join(name),game.join(name)).unwrap();}
   for mapping in runtime["runtimeMappings"].as_array().unwrap(){let target=game.join(mapping["path"].as_str().unwrap());std::fs::create_dir_all(target.parent().unwrap()).unwrap();std::fs::copy(repo.join(mapping["sourcePath"].as_str().unwrap()),target).unwrap();}
   c.configure_game(game.to_str().unwrap()).unwrap();assert!(c.social_enabled);
   use sha2::Digest;
   assert_eq!(format!("{:x}",sha2::Sha256::digest(std::fs::read(game.join("mods/noita_together2/init.lua")).unwrap())),entry_sha);
  }
  if std::env::var("NT2_CF02").as_deref()==Ok("1"){
   use nt2_desktop_g0::{rejoin_local as l,secrets as ps};
   struct StoreBridge<'a>(&'a MemoryStore);
   impl ps::CredentialStore for StoreBridge<'_>{
    fn save(&self,p:&ps::ServiceProfile,b:&ps::SecretBlob)->Result<(),ps::StoreError>{let p=ServiceProfile::new(p.origin(),p.service_id(),p.profile_id()).unwrap();self.0.save(&p,&SecretBlob::new(b.bytes().to_vec()).unwrap()).map_err(|_|ps::StoreError::Unavailable)}
    fn load(&self,p:&ps::ServiceProfile)->Result<Option<ps::SecretBlob>,ps::StoreError>{let p=ServiceProfile::new(p.origin(),p.service_id(),p.profile_id()).unwrap();self.0.load(&p).map_err(|_|ps::StoreError::Unavailable)?.map(|b|ps::SecretBlob::new(b.bytes().to_vec())).transpose()}
    fn delete(&self,p:&ps::ServiceProfile)->Result<(),ps::StoreError>{let p=ServiceProfile::new(p.origin(),p.service_id(),p.profile_id()).unwrap();self.0.delete(&p).map_err(|_|ps::StoreError::Unavailable)}
   }
   let prior=l::Association{version:1,local_world:uuid::Uuid::new_v4(),origin:host.base.clone(),service:host.service.clone(),run:uuid::Uuid::new_v4().to_string(),participant:uuid::Uuid::new_v4().to_string(),room:uuid::Uuid::new_v4().to_string(),world_token:"1".repeat(32),account:host.desktop.as_ref().unwrap().account.clone(),original_start:uuid::Uuid::new_v4().to_string(),config_revision:1,actual_seed:"424242".into(),seed_policy:"shared-random".into(),loot_policy:"shared:1:424242".into(),frozen_config_digest:"a".repeat(64)};
   l::remember(&StoreBridge(&host.store),&prior,&ps::SecretBlob::new(vec![b'x';43]).unwrap()).unwrap();
   use sha2::Digest;let key=sha2::Sha256::digest(serde_json::to_vec(&("nt2-rejoin-index-v1",host.device,&prior.account)).unwrap());
   host.store.save(&ServiceProfile::new(&host.base,&host.service,uuid::Uuid::from_bytes(key[..16].try_into().unwrap())).unwrap(),&SecretBlob::new(prior.encode().unwrap()).unwrap()).unwrap();
   if std::env::var("NT2_CF02_OFFER").as_deref()==Ok("1"){host.offer_saved_world(&prior.encode().unwrap()).unwrap();}
   std::fs::write(dir.join("prior-store-fixture.json"),serde_json::to_vec_pretty(&json!({"priorRoom":prior.room,"priorRun":prior.run,"realCredentialStoreAbstraction":true,"proofBytesInjectedNotRealCredentials":true})).unwrap()).unwrap();
  }
  host.create_fixture(false).await.unwrap();peer.preview_lobby(host.room.as_ref().unwrap().room_id.as_deref().unwrap()).await.unwrap();peer.join(host.room.as_ref().unwrap().room_id.as_deref().unwrap(),None).await.unwrap();
  for c in [&mut host,&mut peer]{room(c).await;assert!(c.pending.lock().unwrap().is_some());}
  if std::env::var("NT2_CF02").as_deref()==Ok("1"){
   for c in [&mut host,&mut peer]{c.next_work=Instant::now();Box::pin(c.tick()).await.unwrap();}
   std::fs::write(dir.join("cf02-before-start.json"),serde_json::to_vec_pretty(&json!({"rememberedIntent":host.resume_intent.view(),"associationRoom":host.resume_intent.association().map(|a|&a.room),"currentRoom":host.room.as_ref().unwrap().room_id,"cleanIntent":peer.resume_intent.view()})).unwrap()).unwrap();
  }
  for(index,c)in [&mut host,&mut peer].into_iter().enumerate(){
   room(c).await;let own=dir.join(index.to_string());
   let prefix=format!(r"\\.\pipe\nt2-g3-c3-private-{}-{index}-",uuid::Uuid::new_v4());let dlls=own.join("private-dlls");std::fs::write(own.join("pipe-prefix.txt"),&prefix).unwrap();
   let output=background_command("powershell.exe").args(["-NoProfile","-ExecutionPolicy","Bypass","-File"]).arg(repo.join("native/g3-ipc/build.ps1")).arg("-OutputDirectory").arg(&dlls).arg("-Private").arg("-TestAppPath").arg(crate::pipe::verified_process_path(&std::env::current_exe().unwrap()).unwrap()).arg("-TestPipePrefix").arg(&prefix).output().unwrap();
   std::fs::write(own.join("private-build.txt"),&output.stdout).unwrap();assert!(output.status.success(),"private build: {}",String::from_utf8_lossy(&output.stderr));
   // Compile before issuing preparation; compilation must not spend the
   // original native lease or make this fixture depend on machine build speed.
   room(c).await;c.setup_tick().await.unwrap();assert!(c.pending.lock().unwrap().is_some());
   // OS image identity is the injected engine process, not a forged observation.
   *c.expected_game.lock().unwrap()=Some(expected.clone());
   c.game_tick().await.unwrap(); // Renew unused local contact through actual Core after private compile.
   let stop=Arc::new(std::sync::atomic::AtomicBool::new(false));let worker=crate::pipe::start_g3_social_named(format!("{prefix}{}",crate::pipe::user_sid().unwrap()),c.pending.clone(),stop.clone(),c.expected_game.clone(),c.native_diagnostics.clone(),c.g3.as_ref().unwrap().exchange.clone(),c.social_stream.as_ref().unwrap().clone()).unwrap();native.workers.push((stop,worker));
   tokio::time::sleep(Duration::from_millis(100)).await; // Allow the newly spawned private listener to enter its accept loop before injected engine launch.
   if index==0{
    // Real OS pipe client image check, before consuming the original grant.
    *c.expected_game.lock().unwrap()=Some(crate::pipe::verified_process_path(&std::env::current_exe().unwrap()).unwrap());
    let refusal=background_command(&runner).arg(own.join("verified-files/lua51.dll")).arg(repo.join("apps/desktop/tools/g3-rejoin-composition-native.lua")).env("NT2_C3_REPO",&repo).env("NT2_C3_DIR",&own).env("NT2_C3_DLL_DIR",&dlls).env("NT2_C3_REFUSE","1").output().unwrap();
    std::fs::write(own.join("wrong-process.txt"),&refusal.stdout).unwrap();assert!(refusal.status.success());assert!(c.pending.lock().unwrap().as_ref().unwrap().owner.is_none());
    *c.expected_game.lock().unwrap()=Some(expected.clone());
   }
   let log=std::fs::File::create(own.join("native-output.txt")).unwrap();
   native.children.push(background_command(&runner).arg(own.join("verified-files/lua51.dll")).arg(repo.join("apps/desktop/tools/g3-rejoin-composition-native.lua")).env("NT2_C3_REPO",&repo).env("NT2_C3_DIR",&own).env("NT2_C3_DLL_DIR",&dlls).env("NT2_U2_PIPE_NAME",format!("{prefix}{}",crate::pipe::user_sid().unwrap())).env("NT2_C3_CORRECTION","1").env("NT2_REJOIN_MALFORMED_PROBE",if index==0{"1"}else{"0"}).env("NT2_CF04_DELAY_ENTRY",if index==0&&std::env::var("NT2_CF04").as_deref()==Ok("1"){"1"}else{"0"}).env("NT2_U2_POLICY",c.room.as_ref().unwrap().config.as_ref().unwrap().seed_policy.as_deref().unwrap()).env("NT2_U2_SEED",c.room.as_ref().unwrap().config.as_ref().unwrap().seed.unwrap().to_string()).env("NT2_U2_ACTUAL",if index==0{"777"}else{"18446744073709551615"}).env("NT2_C3_X",if index==0{"1"}else{"-7.25"}).stdout(log.try_clone().unwrap()).stderr(log).spawn().unwrap());
   let until=Instant::now()+Duration::from_secs(8);
   loop{c.game_tick().await.unwrap();if c.game.as_ref().is_some_and(|g|g.reported>0)&&c.pending.lock().unwrap().as_ref().is_some_and(|p|p.loaded&&p.observation.is_some()){break;}if Instant::now()>=until{let diagnostics={let d=c.native_diagnostics.lock().unwrap();json!({"view":d.view(),"trace":d.initial_contact_trace})};let state=c.view();panic!("native preparation/report timeout: {}; diagnostics {}; state {}",own.display(),diagnostics,state);}tokio::time::sleep(Duration::from_millis(20)).await;}
   assert!(own.join("generation-policy-bound").exists());assert!(own.join("loot-early-separate-context-bound").exists());assert!(own.join("loot-world-separate-context-bound").exists());assert!(!own.join("playing").exists());assert!(c.pending.lock().unwrap().as_ref().unwrap().observation.as_ref().unwrap().observation.protected);
  }
  for c in [&mut host,&mut peer]{room(c).await;c.collect_inspection().await.unwrap();c.collect_inspection().await.unwrap();}
  let participants=host.inspections.iter().map(|r|wire::InspectedParticipant{desktop_id:r.desktop_id.clone(),attachment_id:r.attachment_id.clone(),world_token:r.world_token.clone(),observation_revision:r.observation_revision,content_digest:r.content_digest.clone()}).collect();
  if std::env::var("NT2_CF04").as_deref()==Ok("1"){
   mark(&dir.join("0"),"delay-first-entry");let until=Instant::now()+Duration::from_secs(3);
   while !dir.join("0/first-entry-paused").exists(){assert!(Instant::now()<until);tokio::time::sleep(Duration::from_millis(10)).await;}
  }
  host.start_displayed(participants,vec![],host.room.as_ref().unwrap().config_revision.unwrap()).await.unwrap();
  for index in 0..2{mark(&dir.join(index.to_string()),"start");}
  for c in [&mut host,&mut peer]{room(c).await;let error=c.game_tick().await.err();if let Some(error)=error{std::fs::write(dir.join("cf02-first-causal-error.json"),serde_json::to_vec_pretty(&json!({"error":error,"entered":c.game.as_ref().map(|g|g.entered),"intent":c.resume_intent.view(),"associationRoom":c.resume_intent.association().map(|a|&a.room),"currentRoom":c.room.as_ref().unwrap().room_id})).unwrap()).unwrap();panic!("initial Start error {error}");}}

  if std::env::var("NT2_CF04").as_deref()==Ok("1"){
   let until=Instant::now()+Duration::from_secs(4);
   while !dir.join("1/playing").exists()||peer.pending.lock().unwrap().as_ref().unwrap().observation.as_ref().unwrap().observation.protected{
    assert!(Instant::now()<until,"prompt peer did not consume first release");tokio::time::sleep(Duration::from_millis(10)).await;
   }
   peer.game_tick().await.unwrap();assert!(peer.game.as_ref().unwrap().entered);
   let anchor=host.game.as_ref().unwrap().permit.clone().unwrap();
   let review=host.preparation.as_ref().unwrap().review_expires_at.clone();
   let mut errors=vec![];let mut leases=vec![];
   let mut peer_health=peer.health_runner();
   for cycle in 0..if std::env::var("NT2_CF04_BASELINE").as_deref()==Ok("1"){6}else{12}{
    peer_health.pump().await.unwrap();
    peer.next_work=Instant::now();Box::pin(peer.tick()).await.unwrap();
    host.next_work=Instant::now();let error=Box::pin(host.tick()).await.err();if let Some(e)=error{errors.push(e);}
    let mut request=host.request(true);request.room_get=Some(wire::Empty{});
    let current=host.rpc(&request,true,false).await.unwrap().release.unwrap();
    let original=serde_json::to_value(&anchor).unwrap();let fresh=serde_json::to_value(&current).unwrap();
    let changed:Vec<_>=original.as_object().unwrap().iter().filter(|(k,v)|fresh.get(*k)!=Some(*v)).map(|(k,_)|k.clone()).collect();
    leases.push(json!({"cycle":cycle,"changedFields":changed,"lease":current.release_expires_at,"anchorLease":anchor.release_expires_at,"reviewDeadline":review,"hostEntered":host.game.as_ref().unwrap().entered,"hostProtected":host.pending.lock().unwrap().as_ref().unwrap().observation.as_ref().unwrap().observation.protected,"peerEntered":peer.game.as_ref().unwrap().entered}));
    assert!(!host.game.as_ref().unwrap().entered);assert!(host.pending.lock().unwrap().as_ref().unwrap().observation.as_ref().unwrap().observation.protected);
    tokio::time::sleep(Duration::from_millis(if std::env::var("NT2_CF04_BASELINE").as_deref()==Ok("1"){400}else{2800})).await;
   }
   std::fs::write(dir.join("cf04-delayed-polls.json"),serde_json::to_vec_pretty(&json!({"errors":errors,"leases":leases,"normalCoreTick":true,"actualOriginalNativeDelayed":true,"peerPlaying":peer.game.as_ref().unwrap().entered})).unwrap()).unwrap();
   assert!(peer.game.as_ref().unwrap().entered&&dir.join("1/playing").exists());
   assert!(leases.iter().any(|v|v["changedFields"].as_array().unwrap().iter().any(|k|k=="releaseExpiresAt")));
   if std::env::var("NT2_CF04_BASELINE").as_deref()==Ok("1"){
    assert!(errors.iter().any(|e|e=="Frozen authorization changed"),"baseline must expose frozen lease equality refusal");
    println!("CF04_BASELINE {}",dir.display());return;
   }
   assert!(errors.is_empty(),"delayed original first-entry refused: {errors:?}");
   assert!(now()>timestamp(anchor.release_expires_at.as_deref().unwrap()).unwrap(),"delay must exceed first short lease");
   mark(&dir.join("0"),"allow-first-entry");
   host.cancel_health();peer.cancel_health();
  }
  if std::env::var("NT2_CF05").as_deref()==Ok("1"){
   let mut proofs=vec![];
   for c in [&mut host,&mut peer]{
    assert!(!c.game.as_ref().unwrap().entered);let mut lane=c.health_runner();let until=Instant::now()+Duration::from_secs(8);
    loop{lane.pump().await.unwrap();c.sync_health();if c.game.as_ref().is_some_and(|g|g.entered){break;}assert!(Instant::now()<until);tokio::time::sleep(Duration::from_millis(25)).await;}
    let game=c.game.take();let retained_preparation=c.preparation.is_some();let reacquired=c.fresh_setup_owns_world();c.game=game;
    assert!(retained_preparation&&!reacquired,"health entry did not complete root setup monotonically");
    proofs.push(json!({"healthOwnedFirstConsumption":true,"rootSyncAdoptedEntry":true,"originalPreparationRetained":retained_preparation,"gameAbsentBoundaryModeled":true,"freshSetupReacquired":reacquired}));
    c.cancel_health();
   }
   std::fs::write(dir.join("cf05-health-first-entry.json"),serde_json::to_vec_pretty(&proofs).unwrap()).unwrap();
  }
  if std::env::var("NT2_CF02").as_deref()==Ok("1"){
   let mut outcomes=vec![];
   for(index,c)in [&mut host,&mut peer].into_iter().enumerate(){
    let until=Instant::now()+Duration::from_secs(8);
    let mut first_error=None;loop{if let Err(e)=c.game_tick().await{first_error=Some(e);break;}if c.game.as_ref().is_some_and(|g|g.entered){break;}assert!(Instant::now()<until);tokio::time::sleep(Duration::from_millis(25)).await;}
    c.next_work=Instant::now();let error=Box::pin(c.tick()).await.err().or(first_error);
    outcomes.push(json!({"index":index,"error":error,"intent":c.resume_intent.view(),"currentRoom":c.room.as_ref().unwrap().room_id,"associationRoom":c.resume_intent.association().map(|a|&a.room),"gameEntered":c.game.as_ref().map(|g|g.entered),"nativeRevoked":c.native_revoked}));
   }
   std::fs::write(dir.join("cf02-initial-tick.json"),serde_json::to_vec_pretty(&outcomes).unwrap()).unwrap();
   assert!(outcomes.iter().all(|v|v["error"].is_null()&&v["intent"]["state"]=="playing"&&v["associationRoom"]==v["currentRoom"]),"fresh Start retained stale association: {outcomes:?}");
   let mut lanes=vec![host.health_runner(),peer.health_runner()];
   for cycle in 0..6{
    for (index,c) in [&mut host,&mut peer].into_iter().enumerate(){
     lanes[index].pump().await.unwrap();c.next_work=Instant::now();Box::pin(c.tick()).await.unwrap();c.sync_health();
     assert_eq!(c.resume_intent.phase(),nt2_desktop_g0::rejoin_local::Phase::Playing);
     assert!(c.game.as_ref().unwrap().entered&&!c.native_revoked);
     assert!(!c.native_diagnostics.lock().unwrap().rejoin.loaded_saved,"false initial saved-load event");
     let p=c.pending.lock().unwrap().clone().unwrap();assert!(p.release&&p.authority_until>Instant::now());
    }
    tokio::time::sleep(Duration::from_millis(600)).await;
    std::fs::write(dir.join("cf02-poll-health-cycles.json"),serde_json::to_vec_pretty(&json!({"passed":true,"cycles":cycle+1,"cleanAndRemembered":true,"normalCoreTick":true,"realHealthRunner":true,"noFalseChoice":true,"noFalseSavedLoad":true,"sameCurrentRun":true})).unwrap()).unwrap();
   }
   for c in [&mut host,&mut peer]{c.sync_health();c.cancel_health();}
   if std::env::var("NT2_CF04").as_deref()==Ok("1"){
    std::fs::write(dir.join("cf04-delayed-entry-result.json"),serde_json::to_vec_pretty(&json!({"passed":true,"ordinaryPollsBeforeConsumption":12,"delayedBeyondOriginalShortLease":true,"originalReviewBoundRetained":true,"hostEntered":host.game.as_ref().unwrap().entered,"hostReport":host.game.as_ref().unwrap().reported,"peerEntered":peer.game.as_ref().unwrap().entered,"hostAssociationCurrent":host.resume_intent.association().map(|a|&a.room)==host.room.as_ref().and_then(|r|r.room_id.as_ref()),"engineFactsInjected":true})).unwrap()).unwrap();
   }
  }
  let until=Instant::now()+Duration::from_secs(10);
  loop{
   for c in [&mut host,&mut peer]{c.game_tick().await.unwrap();c.g3_sync().unwrap();c.rejoin_tick().await.unwrap();}
   if dir.join("0/associated").exists()&&dir.join("1/associated").exists(){break}
   assert!(Instant::now()<until,"selected metadata capture timeout");tokio::time::sleep(Duration::from_millis(25)).await;
  }
  let mut peer_health=peer.health_runner();let health_stop=Arc::new(std::sync::atomic::AtomicBool::new(false));let health_flag=health_stop.clone();
  let peer_health_task=tokio::spawn(async move{while !health_flag.load(Ordering::Acquire){peer_health.pump().await.unwrap();tokio::time::sleep(Duration::from_millis(500)).await;}});
  if std::env::var("NT2_RJ2_POLLING").as_deref()==Ok("1"){Box::pin(assert_normal_app_polling(&mut host,&mut peer,&mut fixture,"selected",&dir)).await;}
  let mut host_movement=host.movement_runner().unwrap();let mut peer_movement=peer.movement_runner().unwrap();
  assert!(dir.join("0/malformed-metadata-refused").exists());
  let original=host.resume_intent.association().unwrap().clone();let original_generation=host.rejoin_transport.generation;
  let xml_restore=background_command("python").arg(repo.join("apps/desktop/tools/g3-save-cf03.py")).arg("--restore").arg(dir.join("0/serialized-world.xml")).arg(dir.join("0")).output().unwrap();
  std::fs::write(dir.join("xml-restore-proof.txt"),&xml_restore.stdout).unwrap();assert!(xml_restore.status.success(),"serialized XML restore failed");
  fixture.command(&format!("participant-loss:{}",original.participant));
  let until=Instant::now()+Duration::from_secs(35);
  loop{
   peer.sync_health();peer.g3_sync().unwrap();
   let _=host.rejoin_tick().await;
   if host.rejoin_transport.contact.is_none(){let _=host.game_tick().await;}
   if dir.join("0/resumed-playing").exists()&&host.rejoin_transport.generation>original_generation{break}
   assert!(Instant::now()<until,"live recovery timeout: {}",host.view());tokio::time::sleep(Duration::from_millis(25)).await;
  }
  let ghost_until=Instant::now()+Duration::from_secs(8);
  while !dir.join("0/ghost").exists()||!dir.join("1/ghost").exists(){
   host.rejoin_tick().await.unwrap();peer.sync_health();peer.g3_sync().unwrap();let _=host_movement.pump().await;let _=peer_movement.pump().await;
   assert!(Instant::now()<ghost_until,"actual WSS ghost timeout");tokio::time::sleep(Duration::from_millis(20)).await;
  }
  assert!(host.g3_view()["movement"]["live_connections"].as_u64().is_some_and(|n|n>0));
  assert!(host.game.as_ref().unwrap().entered);assert_eq!(host.resume_intent.association(),Some(&original));
  if std::env::var("NT2_RJ2_POLLING").as_deref()==Ok("1"){Box::pin(assert_normal_app_polling(&mut host,&mut peer,&mut fixture,"recovered",&dir)).await;}
  // Continue while the app/lobby and native process stay alive: old playing
  // intent must not silently count as a new saved-world choice.
  mark(&dir.join("0"),"continue-in-process");
  let until=Instant::now()+Duration::from_secs(8);
  loop{let _=host.rejoin_tick().await;if host.resume_intent.phase()==nt2_desktop_g0::rejoin_local::Phase::Choice{break;}assert!(Instant::now()<until,"same-app Continue did not offer a choice");tokio::time::sleep(Duration::from_millis(25)).await;}
  assert!(host.game.is_none());assert!(host.rejoin_transport.contact.is_none());
  let mut cf05_choice_errors=vec![];
  for _ in 0..5{
   if std::env::var("NT2_CF05").as_deref()==Ok("1"){
    assert!(!host.fresh_setup_owns_world(),"completed fresh setup recaptured Continue");
    host.next_work=Instant::now();if let Err(e)=Box::pin(host.tick()).await{cf05_choice_errors.push(e);}
   }else{host.rejoin_tick().await.unwrap();}
   tokio::time::sleep(Duration::from_millis(25)).await;assert_eq!(host.resume_intent.phase(),nt2_desktop_g0::rejoin_local::Phase::Choice);
  }
  if std::env::var("NT2_CF05").as_deref()==Ok("1"){
   std::fs::write(dir.join("cf05-established-continue-ticks.json"),serde_json::to_vec_pretty(&json!({"ordinaryCoreTicks":5,"gameCleared":host.game.is_none(),"freshSetupCannotReacquire":!host.fresh_setup_owns_world(),"choiceRetained":true,"errors":cf05_choice_errors,"engineFactsInjected":true})).unwrap()).unwrap();
   assert!(cf05_choice_errors.is_empty(),"ordinary Continue tick failed: {cf05_choice_errors:?}");
   assert!(host.setup_view()["error"].is_null(),"completed setup emitted a stale fresh-world instruction: {}",host.setup_view());
  }
  mark(&dir.join("0"),"choose-rejoin");
  let until=Instant::now()+Duration::from_secs(70);
  loop{let error=host.rejoin_tick().await.err();if let Some(e)=&error{std::fs::write(dir.join("same-app-last-error.txt"),e).unwrap();}std::fs::write(dir.join("same-app-chosen-progress.json"),serde_json::to_vec_pretty(&json!({"error":error,"intent":host.resume_intent.view(),"nativeContact":host.native_diagnostics.lock().unwrap().rejoin.contact.as_ref().map(|c|hex(&c.context)),"transportContact":host.rejoin_transport.contact.as_ref().map(|c|hex(&c.context)),"gameEntered":host.game.as_ref().map(|g|g.entered)})).unwrap()).unwrap();if host.game.as_ref().is_some_and(|g|g.entered){break;}assert!(Instant::now()<until,"same-app Continue chosen recovery timeout");tokio::time::sleep(Duration::from_millis(25)).await;}
  std::fs::remove_file(dir.join("0/choose-rejoin")).unwrap();
  std::fs::write(dir.join("same-app-continue-result.json"),serde_json::to_vec_pretty(&json!({"passed":true,"actualCore":true,"sameNativeProcess":true,"actualEntryCallbacks":true,"noPreinit":true,"choiceHeldBeforeConsent":true,"chosenRejoinInspected":true,"engineSaveFactsInjected":true})).unwrap()).unwrap();
  let renewed_session=host.game.as_ref().unwrap().session.id.clone();let renewed_context=host.game.as_ref().unwrap().owner;
  host.game.as_mut().unwrap().session.expires=now();host.rejoin_due_for_controlled_clock();host.rejoin_tick().await.unwrap();
  assert_eq!(host.game.as_ref().unwrap().session.id,renewed_session);assert_eq!(host.game.as_ref().unwrap().owner,renewed_context);assert!(host.game.as_ref().unwrap().session.expires>now()+60000);
  host.social_scheduler_tick().await.unwrap();peer.social_scheduler_tick().await.unwrap();
  assert!(peer.pending.lock().unwrap().as_ref().unwrap().release,"unaffected peer continues");
  let store=host.store.as_ref().clone();let device=host.device;
  mark(&dir.join("0"),"stop");assert!(native.children[0].wait().unwrap().success());
  fixture.command(&format!("participant-loss:{}",original.participant));
  drop(host);
  let mut restored=Core::new(store,device,Arc::new(Mutex::new(None)),dir.join("reload-claims"));restored.open_browser=false;
  restored.http=reqwest::Client::builder().use_native_tls().tls_certs_only(vec![ca.clone()]).no_proxy().redirect(reqwest::redirect::Policy::none()).timeout(Duration::from_secs(3)).build().unwrap();
  restored.enable_g3(&dir.join("reload-journal.jsonl"),Arc::new(Mutex::new(nt2_desktop_g0::g3_runtime::Exchange::default()))).unwrap();restored.enable_live_transport().unwrap();restored.request_social(Arc::new(Mutex::new(nt2_desktop_g0::g3_social_stream::Exchange::default()))).unwrap();
  restored.select(&fixture.origin,"",false).await.unwrap();registered_as(&mut restored,"alex").await;
  restored.configure_game(dir.join("0/verified-files").to_str().unwrap()).unwrap();
  restored.rejoin_tick().await.unwrap();assert_eq!(restored.resume_intent.phase(),nt2_desktop_g0::rejoin_local::Phase::Choice);
  restored.rejoin_previous().unwrap(); // App-first: Continue must not ask again.
  let own=dir.join("0");let prefix=format!(r"\\.\pipe\nt2-g3-rejoin-saved-{}-",uuid::Uuid::new_v4());let dlls=own.join("private-saved-dlls");
  let output=background_command("powershell.exe").args(["-NoProfile","-ExecutionPolicy","Bypass","-File"]).arg(repo.join("native/g3-ipc/build.ps1")).arg("-OutputDirectory").arg(&dlls).arg("-Private").arg("-SavedWorld").arg("-TestAppPath").arg(crate::pipe::verified_process_path(&std::env::current_exe().unwrap()).unwrap()).arg("-TestPipePrefix").arg(&prefix).output().unwrap();assert!(output.status.success());
  *restored.expected_game.lock().unwrap()=Some(expected.clone());let stop=Arc::new(std::sync::atomic::AtomicBool::new(false));
  let worker=crate::pipe::start_g3_social_named(format!("{prefix}{}",crate::pipe::user_sid().unwrap()),restored.pending.clone(),stop.clone(),restored.expected_game.clone(),restored.native_diagnostics.clone(),restored.g3.as_ref().unwrap().exchange.clone(),restored.social_stream.as_ref().unwrap().clone()).unwrap();native.workers.push((stop,worker));
  std::fs::remove_file(own.join("stop")).unwrap();std::fs::remove_file(own.join("resumed-playing")).unwrap();
  let log=std::fs::File::create(own.join("saved-native-output.txt")).unwrap();
  native.children.push(background_command(&runner).arg(own.join("verified-files/lua51.dll")).arg(repo.join("apps/desktop/tools/g3-rejoin-composition-native.lua")).env("NT2_C3_REPO",&repo).env("NT2_C3_DIR",&own).env("NT2_C3_DLL_DIR",&dlls).env("NT2_REJOIN_SAVED","1").env("NT2_C3_X","1").env("NT2_C3_CORRECTION","1").env("NT2_U2_POLICY",original.seed_policy.clone()).env("NT2_U2_SEED",original.actual_seed.clone()).env("NT2_U2_ACTUAL",original.actual_seed.clone()).stdout(log.try_clone().unwrap()).stderr(log).spawn().unwrap());
  let until=Instant::now()+Duration::from_secs(35);
  loop{
   peer.sync_health();peer.g3_sync().unwrap();let _=restored.rejoin_tick().await;
   if restored.game.as_ref().is_some_and(|g|g.entered){break}
   assert!(Instant::now()<until,"saved reload timeout: {}",restored.view());tokio::time::sleep(Duration::from_millis(25)).await;
  }
  assert_eq!(restored.resume_intent.association(),Some(&original));assert!(peer.pending.lock().unwrap().as_ref().unwrap().release);
  mark(&dir.join("1"),"unaffected-check");let snapshot_until=Instant::now()+Duration::from_secs(3);
  while !dir.join("1/unaffected-snapshot").exists(){assert!(Instant::now()<snapshot_until);tokio::time::sleep(Duration::from_millis(20)).await;}
  let unaffected=std::fs::read_to_string(dir.join("1/unaffected-snapshot")).unwrap();assert!(unaffected.contains("playing=true;holds=0"),"unaffected native peer: {unaffected}");
  health_stop.store(true,Ordering::Release);peer_health_task.await.unwrap();peer.sync_health();peer.cancel_health();
  let previous_epoch=restored.epoch.clone();let saved_generation=restored.rejoin_transport.generation;
  fixture.command("restart");
  let until=Instant::now()+Duration::from_secs(40);
  loop{
   let _=restored.rejoin_tick().await;let _=peer.rejoin_tick().await;
   if restored.epoch!=previous_epoch&&restored.rejoin_transport.generation>saved_generation&&restored.game.as_ref().is_some_and(|g|g.entered)&&peer.game.as_ref().is_some_and(|g|g.entered)&&peer.epoch==restored.epoch{break}
   assert!(Instant::now()<until,"same-issuer restart recovery timeout: {} / {}",restored.view(),peer.view());tokio::time::sleep(Duration::from_millis(100)).await;
  }
  assert_eq!(restored.resume_intent.association(),Some(&original));
  restored.social_scheduler_tick().await.unwrap();peer.social_scheduler_tick().await.unwrap();
  let mut saved_movement=restored.movement_runner().unwrap();let until=Instant::now()+Duration::from_secs(5);
  while Instant::now()<until{restored.rejoin_tick().await.unwrap();peer.rejoin_tick().await.unwrap();let _=saved_movement.pump().await;let _=peer_movement.pump().await;tokio::time::sleep(Duration::from_millis(30)).await;}
  assert!(restored.g3_view()["movement"]["live_connections"].as_u64().is_some_and(|n|n>0));
  // Controlled service clock moves beyond five minutes while only this owner
  // loses authenticated transport. Peer's real consumer continues health.
  fixture.command(&format!("session-loss:{}",restored.desktop.as_ref().unwrap().id));
  let outage_generation=restored.rejoin_transport.generation;
  for _ in 0..31{
   fixture.command("advance:10000");peer.rejoin_due_for_controlled_clock();
   peer.rejoin_tick().await.unwrap();let _=restored.rejoin_tick().await;
  }
  assert!(peer.pending.lock().unwrap().as_ref().unwrap().release);
  fixture.command(&format!("session-restore:{}",restored.desktop.as_ref().unwrap().id));
  let until=Instant::now()+Duration::from_secs(35);
  loop{
   let _=restored.rejoin_tick().await;let _=peer.rejoin_tick().await;
   if restored.rejoin_transport.generation>outage_generation&&restored.game.as_ref().is_some_and(|g|g.entered){break}
   assert!(Instant::now()<until,"long outage recovery timeout: {}",restored.view());tokio::time::sleep(Duration::from_millis(100)).await;
  }
  // Explicit offline in an ongoing run must traverse the canonical RPC,
  // retire this participation, and leave the other playing world unaffected.
  peer.continue_offline().unwrap();peer.rejoin_tick().await.unwrap();assert!(peer.rejoin_retirement_verified());
  let offline_until=Instant::now()+Duration::from_secs(5);
  while !dir.join("1/offline-restored").exists(){peer.rejoin_tick().await.unwrap();assert!(Instant::now()<offline_until,"explicit app offline did not restore native world");tokio::time::sleep(Duration::from_millis(30)).await;}
  restored.rejoin_due_for_controlled_clock();restored.rejoin_tick().await.unwrap();assert!(restored.pending.lock().unwrap().as_ref().unwrap().release);
  restored.end().await.unwrap();mark(&dir.join("0"),"assert-terminal-end");let until=Instant::now()+Duration::from_secs(12);
  while !dir.join("0/terminal-protected").exists(){let _=restored.rejoin_tick().await;assert!(Instant::now()<until,"terminal End did not keep the actual native world protected");tokio::time::sleep(Duration::from_millis(100)).await;}
  assert_eq!(restored.resume_intent.phase(),nt2_desktop_g0::rejoin_local::Phase::Terminal);
  restored.continue_offline().unwrap();restored.rejoin_tick().await.unwrap();assert!(restored.rejoin_retirement_verified());
  let until=Instant::now()+Duration::from_secs(5);
  while !dir.join("0/offline-restored").exists(){restored.rejoin_tick().await.unwrap();assert!(Instant::now()<until,"terminal world did not honour the single explicit offline choice");tokio::time::sleep(Duration::from_millis(30)).await;}
  for index in 0..2{mark(&dir.join(index.to_string()),"stop");}
  for child in &mut native.children{assert!(child.wait().unwrap().success());}
  std::fs::write(dir.join("result.json"),serde_json::to_vec_pretty(&json!({"gate":"G3A/reconnect-rejoin","checkpoint":"RJ2-C","actualCore":true,"actualNativeBothWrappers":true,"actualLua":true,"authenticatedTlsWss":true,"liveRecovery":true,"savedContinueAppReload":true,"sameIssuerRestart":true,"longOutageServiceClockMs":310000,"terminalEnd":true,"actualMalformedMetadataRefusal":true,"sameSessionRenewalStableWorld":true,"recoveredSocialOwner":true,"canonicalOfflineAccepted":true,"terminalStatusThenLocalOffline":true,"explicitOfflineRetirement":true,"unaffectedPeer":true,"unaffectedNative":unaffected,"engineSaveFactsInjected":true,"actualNoitaPersistence":false})).unwrap()).unwrap();println!("RJ2_COMPOSITION {}",dir.display());
 });
}

// Network facts and optional faults use the same real disposable service. No
// direct collector calls: the actual app scheduler must produce observations.
async fn assert_normal_app_polling(c:&mut Core<MemoryStore>,peer:&mut Core<MemoryStore>,fixture:&mut Fixture,phase:&str,dir:&Path){
 assert!(c.rejoin_transport.contact.is_some());assert!(c.game.as_ref().unwrap().entered);
 let owner=c.game.as_ref().unwrap().owner;let session=c.game.as_ref().unwrap().session.id.clone();let generation=c.rejoin_transport.generation;
 let mut runner=c.health_runner();let stop=Arc::new(std::sync::atomic::AtomicBool::new(false));let flag=stop.clone();
 let task=tokio::spawn(async move{while !flag.load(Ordering::Acquire){runner.pump().await.unwrap();tokio::time::sleep(Duration::from_millis(500)).await;}});
 let incoming=format!("normal tick peer {phase}");peer.chat_send(incoming.clone()).await.unwrap();fixture.command("poll-trace-reset");
 for _ in 0..3{c.next_work=Instant::now();Box::pin(c.tick()).await.unwrap();tokio::time::sleep(Duration::from_millis(100)).await;}
 let trace=fixture.trace();
 std::fs::write(dir.join(format!("poll-{phase}-initial.json")),serde_json::to_vec_pretty(&json!({"trace":trace,"peerChatVisible":c.chat.iter().any(|m|m.text.as_deref()==Some(incoming.as_str()))})).unwrap()).unwrap();
 assert!(c.chat.iter().any(|m|m.text.as_deref()==Some(incoming.as_str())),"normal Core::tick stopped collecting peer chat in {phase}");
 assert!(trace["chatList"].as_u64().unwrap()>0&&trace["inspectionList"].as_u64().unwrap()>0&&trace["roomGet"].as_u64().unwrap()>0,"normal app collectors/room not scheduled: {trace}");
 assert!(c.inspections.iter().any(|r|r.desktop_id.as_deref()==Some(peer.desktop.as_ref().unwrap().id.as_str())&&r.observation_revision.is_some_and(|n|n>=2)),"current actual peer inspection not refreshed");
 assert!(c.room.as_ref().unwrap().members.as_ref().unwrap().iter().any(|m|m.desktop_id.as_deref()==Some(c.desktop.as_ref().unwrap().id.as_str())&&m.loaded==Some(true)),"current loaded room projection not refreshed");
 fixture.command("drop-chat-reply");let outgoing=format!("immutable normal tick {phase}");assert!(c.chat_send(outgoing.clone()).await.is_err());
 let original=serde_json::to_vec(&c.chat_retry.as_ref().unwrap().request).unwrap();let original_id=c.chat_retry.as_ref().unwrap().request.request_id.clone().unwrap();
 fixture.command("drop-chat-reply");c.next_work=Instant::now();assert!(Box::pin(c.tick()).await.is_err());
 assert_eq!(serde_json::to_vec(&c.chat_retry.as_ref().unwrap().request).unwrap(),original,"ambiguous chat identity/body changed");
 c.next_work=Instant::now();Box::pin(c.tick()).await.unwrap();assert!(c.chat_retry.is_none());assert_eq!(c.chat.iter().filter(|m|m.text.as_deref()==Some(outgoing.as_str())).count(),1,"original chat replay duplicated history");
 let retry_trace=fixture.trace();assert_eq!(retry_trace["chatSendIds"].as_array().unwrap().iter().filter(|v|v.as_str()==Some(original_id.as_str())).count(),3);
 let mut failures=vec![];
 for fault in ["500","malformed","timeout"]{
  fixture.command(&format!("optional:{fault}"));c.next_work=Instant::now();assert!(Box::pin(c.tick()).await.is_err());c.sync_health();
  let p=c.pending.lock().unwrap().clone().unwrap();assert!(p.release&&p.authority_until>Instant::now(),"optional {fault} revoked healthy gameplay");
  assert_eq!(c.game.as_ref().unwrap().owner,owner);assert_eq!(c.game.as_ref().unwrap().session.id,session);assert_eq!(c.rejoin_transport.generation,generation);
  failures.push(json!({"fault":fault,"nativeLeaseValid":true,"sameOwnerSessionGeneration":true}));fixture.command("optional:none");
 }
 c.next_work=Instant::now();Box::pin(c.tick()).await.unwrap();
 stop.store(true,Ordering::Release);task.await.unwrap();c.sync_health();c.cancel_health();
 std::fs::write(dir.join(format!("poll-{phase}-result.json")),serde_json::to_vec_pretty(&json!({"passed":true,"normalCoreTick":true,"phase":phase,"peerChatVisible":true,"actualInspectionAndRoomVisible":true,"immutableOriginalChatRetries":3,"singleStoredMessage":true,"trace":trace,"optionalFailures":failures,"independentHealthPreserved":true})).unwrap()).unwrap();
}

