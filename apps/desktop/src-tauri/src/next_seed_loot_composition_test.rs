//! Actual automatic Core/canonical TLS/private broker/Lua composition; engine facts injected.
use super::*;
#[test]
#[ignore="explicit disposable TLS canonical cancellation"]
fn seedloot_u2_matched_cancel_does_not_rearm(){tauri::async_runtime::block_on(async{
 let fixture=Fixture::start_with(true);let repo=repo();let own=repo.join("spikes/noita-entry/.build").join(format!("g3-seed-loot-cancel-{}",uuid::Uuid::new_v4()));let game=own.join("game");std::fs::create_dir_all(&game).unwrap();
 let approval=nt2_desktop_g0::g3_social_installation::development_profile().unwrap();
 for name in ["noita.exe","lua51.dll"]{std::fs::copy(Path::new("C:/Noita").join(name),game.join(name)).unwrap();}
 for m in approval["runtimeMappings"].as_array().unwrap(){let target=game.join(m["path"].as_str().unwrap());std::fs::create_dir_all(target.parent().unwrap()).unwrap();std::fs::copy(repo.join(m["sourcePath"].as_str().unwrap()),target).unwrap();}
 let mut c=core();let ca=reqwest::Certificate::from_pem(&std::fs::read(&fixture.ca).unwrap()).unwrap();c.http=reqwest::Client::builder().use_native_tls().tls_certs_only(vec![ca]).no_proxy().redirect(reqwest::redirect::Policy::none()).build().unwrap();
 c.enable_g3(&own.join("journal.jsonl"),Arc::new(Mutex::new(nt2_desktop_g0::g3_runtime::Exchange::default()))).unwrap();c.enable_live_transport().unwrap();c.request_social(Arc::new(Mutex::new(nt2_desktop_g0::g3_social_stream::Exchange::default()))).unwrap();c.enable_automatic_setup();c.select(&fixture.origin,"",false).await.unwrap();registered_as(&mut c,"alex").await;c.configure_game(game.to_str().unwrap()).unwrap();c.create_fixture(false).await.unwrap();assert!(c.pending.lock().unwrap().is_some());
 c.cancel().await.unwrap();assert!(c.pending.lock().unwrap().is_none());c.setup_tick().await.unwrap();c.setup_tick().await.unwrap();assert!(c.pending.lock().unwrap().is_none());
 std::fs::write(own.join("result.json"),serde_json::to_vec_pretty(&json!({"passed":true,"actualCore":true,"actualCanonicalTlsCancellation":true,"pollingCannotRearm":true,"engineExecuted":false})).unwrap()).unwrap();println!("U2_CANCEL_EVIDENCE {}",own.display());
});}
#[test]
#[ignore="explicit C3 matched metadata/private native/actual Core TLS WSS; injected engine facts"]
fn seedloot_u2_matched_automatic_core_broker_wrappers_live_and_retirement(){
 assert_eq!(std::env::var("NT2_C3_COMPOSITION").as_deref(),Ok("1"));
 assert!(std::env::var_os("NT2_C3_ENTRY_PATH").is_none(),"composition must use exact verified mapping, no entry override");
 assert!(std::env::var_os("NT2_C3_DIAGNOSTIC_PIN").is_none(),"composition must rely on entry-owned images");
 tauri::async_runtime::block_on(async {
  let repo=repo();let dir=repo.join("spikes/noita-entry/.build/g3-seed-loot-composition").join(uuid::Uuid::new_v4().to_string());std::fs::create_dir_all(&dir).unwrap();
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
  host.create_fixture(false).await.unwrap();peer.preview_lobby(host.room.as_ref().unwrap().room_id.as_deref().unwrap()).await.unwrap();peer.join(host.room.as_ref().unwrap().room_id.as_deref().unwrap(),None).await.unwrap();
  for c in [&mut host,&mut peer]{room(c).await;assert!(c.pending.lock().unwrap().is_some());}
  for(index,c)in [&mut host,&mut peer].into_iter().enumerate(){
   room(c).await;let own=dir.join(index.to_string());
   let prefix=format!(r"\\.\pipe\nt2-g3-c3-private-{}-{index}-",uuid::Uuid::new_v4());let dlls=own.join("private-dlls");std::fs::write(own.join("pipe-prefix.txt"),&prefix).unwrap();
   let output=Command::new("powershell.exe").args(["-NoProfile","-ExecutionPolicy","Bypass","-File"]).arg(repo.join("native/g3-ipc/build.ps1")).arg("-OutputDirectory").arg(&dlls).arg("-Private").arg("-TestAppPath").arg(crate::pipe::verified_process_path(&std::env::current_exe().unwrap()).unwrap()).arg("-TestPipePrefix").arg(&prefix).output().unwrap();
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
    let refusal=Command::new(&runner).arg(own.join("verified-files/lua51.dll")).arg(repo.join("apps/desktop/tools/g3-seed-loot-composition-native.lua")).env("NT2_C3_REPO",&repo).env("NT2_C3_DIR",&own).env("NT2_C3_DLL_DIR",&dlls).env("NT2_C3_REFUSE","1").output().unwrap();
    std::fs::write(own.join("wrong-process.txt"),&refusal.stdout).unwrap();assert!(refusal.status.success());assert!(c.pending.lock().unwrap().as_ref().unwrap().owner.is_none());
    *c.expected_game.lock().unwrap()=Some(expected.clone());
   }
   let log=std::fs::File::create(own.join("native-output.txt")).unwrap();
   native.children.push(Command::new(&runner).arg(own.join("verified-files/lua51.dll")).arg(repo.join("apps/desktop/tools/g3-seed-loot-composition-native.lua")).env("NT2_C3_REPO",&repo).env("NT2_C3_DIR",&own).env("NT2_C3_DLL_DIR",&dlls).env("NT2_U2_PIPE_NAME",format!("{prefix}{}",crate::pipe::user_sid().unwrap())).env("NT2_C3_CORRECTION","1").env("NT2_U2_POLICY",c.room.as_ref().unwrap().config.as_ref().unwrap().seed_policy.as_deref().unwrap()).env("NT2_U2_SEED",c.room.as_ref().unwrap().config.as_ref().unwrap().seed.unwrap().to_string()).env("NT2_U2_ACTUAL",if index==0{"777"}else{"18446744073709551615"}).env("NT2_C3_X",if index==0{"1"}else{"-7.25"}).stdout(log.try_clone().unwrap()).stderr(log).spawn().unwrap());
   let until=Instant::now()+Duration::from_secs(8);
   loop{c.game_tick().await.unwrap();if c.game.as_ref().is_some_and(|g|g.reported>0)&&c.pending.lock().unwrap().as_ref().is_some_and(|p|p.loaded&&p.observation.is_some()){break;}if Instant::now()>=until{let diagnostics={let d=c.native_diagnostics.lock().unwrap();json!({"view":d.view(),"trace":d.initial_contact_trace})};let state=c.view();panic!("native preparation/report timeout: {}; diagnostics {}; state {}",own.display(),diagnostics,state);}tokio::time::sleep(Duration::from_millis(20)).await;}
   assert!(own.join("generation-policy-bound").exists());assert!(own.join("loot-early-separate-context-bound").exists());assert!(own.join("loot-world-separate-context-bound").exists());assert!(!own.join("playing").exists());assert!(c.pending.lock().unwrap().as_ref().unwrap().observation.as_ref().unwrap().observation.protected);
  }
  for c in [&mut host,&mut peer]{room(c).await;c.collect_inspection().await.unwrap();c.collect_inspection().await.unwrap();}
  let participants=host.inspections.iter().map(|r|wire::InspectedParticipant{desktop_id:r.desktop_id.clone(),attachment_id:r.attachment_id.clone(),world_token:r.world_token.clone(),observation_revision:r.observation_revision,content_digest:r.content_digest.clone()}).collect();
  host.start_displayed(participants,vec![],host.room.as_ref().unwrap().config_revision.unwrap()).await.unwrap();
  for index in 0..2{mark(&dir.join(index.to_string()),"start");}
  for c in [&mut host,&mut peer]{room(c).await;c.game_tick().await.unwrap();}
  let original_until=host.pending.lock().unwrap().as_ref().unwrap().authority_until;
  let mut health=host.health_runner();let mut peer_health=peer.health_runner();let mut movement=host.movement_runner().unwrap();let mut peer_movement=peer.movement_runner().unwrap();
  let stop=Arc::new(std::sync::atomic::AtomicBool::new(false));let health_stop=stop.clone();
  let health_task=tokio::spawn(async move{let began=Instant::now();let mut renewed=false;while !health_stop.load(Ordering::Acquire){if !renewed&&began.elapsed()>Duration::from_secs(7){health.renew_for_controlled_clock();peer_health.renew_for_controlled_clock();renewed=true;}health.pump().await.unwrap();peer_health.pump().await.unwrap();tokio::time::sleep(Duration::from_millis(750)).await;}(health,peer_health)});
  let movement_stop=stop.clone();let movement_task=tokio::spawn(async move{while !movement_stop.load(Ordering::Acquire){let _=movement.pump().await;let _=peer_movement.pump().await;tokio::time::sleep(Duration::from_millis(20)).await;}(movement,peer_movement)});
  let began=Instant::now();let mut disconnected=false;let mut faults=false;
  while began.elapsed()<Duration::from_secs(38){
   host.sync_health();peer.sync_health();
   for c in [&mut host,&mut peer]{c.game_tick().await.unwrap();c.g3_sync().unwrap();let _=c.social_scheduler_tick().await;}
   if !faults&&began.elapsed()>Duration::from_secs(5){
    // Core is exclusively borrowed across slow UI/social calls; health and
    // movement owners must continue from the actual broker's contact facts.
    for mode in ["500","capacity","malformed","slow","timeout"]{fixture.command(&format!("optional:{mode}"));let _=host.room_page(false).await;let _=host.social_scheduler_tick().await;}
    fixture.command("optional:none");faults=true;
   }
   if !disconnected&&began.elapsed()>Duration::from_secs(17){fixture.command("disconnect-live");disconnected=true;}
   if began.elapsed()>Duration::from_secs(24)&&host.g3_view()["movement"]["live_connections"].as_u64().is_some_and(|n|n>=2){break;}
   tokio::time::sleep(Duration::from_millis(20)).await;
  }
  stop.store(true,Ordering::Release);let (_health,mut peer_health)=health_task.await.unwrap();let (mut movement,mut peer_movement)=movement_task.await.unwrap();host.sync_health();peer.sync_health();
  assert!(Instant::now()>original_until);for index in 0..2{assert!(dir.join(index.to_string()).join("ghost").exists());assert!(dir.join(index.to_string()).join("playing").exists());assert!(dir.join(index.to_string()).join("wrong-world-refused").exists());}
  let metrics=host.g3_view()["movement"].clone();println!("C3 bounded live metrics: {metrics}");assert!(metrics["receives"].as_u64().unwrap()>100);assert!(metrics["live_connections"].as_u64().unwrap()>=2);assert!(metrics["live_reauthentications"].as_u64().unwrap()>=1);
  // Stop verified health, retain native contact, and let the ORIGINAL last
  // permission deadline genuinely expire. No clock shift or lease mutation.
  let expiry=[&host,&peer].into_iter().map(|c|c.pending.lock().unwrap().as_ref().unwrap().authority_until).max().unwrap();
  for index in 0..2{mark(&dir.join(index.to_string()),"expire");}
  tokio::time::sleep(expiry.saturating_duration_since(Instant::now())+Duration::from_millis(2200)).await;
  for index in 0..2{assert!(dir.join(index.to_string()).join("permission-expired-protected").exists(),"original permission exhaustion must protect despite native contact");}
  host.end().await.unwrap();assert!(peer_health.pump().await.is_err());peer.sync_health();let _=movement.pump().await;let _=peer_movement.pump().await;
  for index in 0..2{mark(&dir.join(index.to_string()),"end");}
  for child in &mut native.children{assert!(child.wait().unwrap().success());}
  {let own=dir.join("0");let prefix=std::fs::read_to_string(own.join("pipe-prefix.txt")).unwrap();
    let saved_dlls=own.join("private-continue-dlls");
    let saved_build=Command::new("powershell.exe").args(["-NoProfile","-ExecutionPolicy","Bypass","-File"]).arg(repo.join("native/g3-ipc/build.ps1")).arg("-OutputDirectory").arg(&saved_dlls).arg("-Private").arg("-SavedWorld").arg("-TestAppPath").arg(crate::pipe::verified_process_path(&std::env::current_exe().unwrap()).unwrap()).arg("-TestPipePrefix").arg(&prefix).output().unwrap();assert!(saved_build.status.success());
    let continued=Command::new(&runner).arg(own.join("verified-files/lua51.dll")).arg(repo.join("apps/desktop/tools/g3-seed-loot-composition-native.lua")).env("NT2_C3_REPO",&repo).env("NT2_C3_DIR",&own).env("NT2_C3_DLL_DIR",&saved_dlls).env("NT2_C3_REFUSE","1").output().unwrap();std::fs::write(own.join("continue.txt"),&continued.stdout).unwrap();assert!(continued.status.success());assert!(host.pending.lock().unwrap().is_none());

  }
  for index in 0..2{assert!(dir.join(index.to_string()).join("end-protected").exists());}
  if true{
   for index in 0..2{let log=std::fs::read_to_string(dir.join(index.to_string()).join("native-output.txt")).unwrap();let lines:Vec<_>=log.lines().filter(|s|s.starts_with("NT2_POTION|v=1|")).collect();assert!(lines.len()<=8);
    for phase in ["spawn_before","spawn_after_protect","first_post_update","waiting_after_60_frames","start_restored","reprotected"]{assert!(lines.iter().any(|s|s.contains(&format!("|phase={phase}|"))),"missing real entry diagnostic phase {phase}");}
    assert!(lines.iter().all(|s|s.len()<=4096));assert!(lines.iter().any(|s|s.contains("|child_count=3|")),"injected existing loadout recorded");
   }
  }
  std::fs::write(dir.join("result.json"),serde_json::to_vec_pretty(&json!({"activeGate":"G3A/A1","checkpoint":if cfg!(feature="g3-loot01"){"LOOT01-C-PACK"}else{"U2-C"},"realisticEarlyGlobalsRefusal":true,"separateHookContextProof":true,"runtimeSha256":nt2_desktop_g0::g3_social_installation::DEVELOPMENT_SHA,"diagnosticPinnedImages":std::env::var("NT2_C3_DIAGNOSTIC_PIN").as_deref()==Ok("1"),"passed":true,"actualCore":true,"actualPrivateBrokerAndBothWrappers":true,"actualTlsWss":true,"actualLuaEntryAndGhostAdapter":true,"automaticPreviewJoinArmingInspectionDefaultStart":true,"seedPolicy":std::env::var("NT2_U2_POLICY").unwrap_or("shared-random".into()),"lootOff":std::env::var("NT2_U2_LOOT_OFF").as_deref()==Ok("1"),"engineFactsInjected":true,"productionDllsExecuted":false,"actualNoita":false,"installed":false,"metrics":metrics})).unwrap()).unwrap();
  println!("C3_COMPOSITION_EVIDENCE {}",dir.display());
 });
}
