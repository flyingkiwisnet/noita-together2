// Root serializes this actual TLS/SQL/WSS/app/native/mod composition. Only the
// engine API and a labelled source-publication suppression are injected.
fn persisted_debug(c:&Client,component:&str)->Value{
 let root=c.root.join("diagnostics/app").join(nt2_desktop_g0::g3_social_installation::DEVELOPMENT_ID);
 let Some(selection)=std::fs::read(root.join("selection.json")).ok().and_then(|v|serde_json::from_slice::<Value>(&v).ok())else{return Value::Null};
 selection["sessionId"].as_str().and_then(|id|std::fs::read(root.join(id).join(format!("{component}.json"))).ok()).and_then(|v|serde_json::from_slice(&v).ok()).unwrap_or(Value::Null)
}
fn capture_debug(c:&Client,own:&std::path::Path,label:&str){
 let output=own.join(label);let result=hidden("powershell.exe").args(["-NoProfile","-NonInteractive","-ExecutionPolicy","Bypass","-File"]).arg(repo().join("apps/desktop/tools/Export-AutomaticDebugFixture.ps1"))
 .arg("-RuntimePath").arg(repo().join(profile_path())).arg("-PrivateTestApp").arg(std::env::current_exe().unwrap())
 .arg("-GamePath").arg(c.root.join("verified-files")).arg("-DiagnosticsRoot").arg(c.root.join("diagnostics"))
 .arg("-ServerDiagnosticsRoot").arg(std::path::PathBuf::from(std::env::var("NT2_SERVER_DIAGNOSTIC_TEST_ROOT").unwrap()).parent().unwrap()).arg("-OutputRoot").arg(&output).arg("-Label").arg(label)
 .arg("-StimulusUtcMs").arg(std::fs::read_to_string(own.join("stimulus-utc-ms")).unwrap())
 .env("PSModulePath",format!("{}/System32/WindowsPowerShell/v1.0/Modules",std::env::var("WINDIR").unwrap())).output().unwrap();
 std::fs::create_dir_all(&output).unwrap();std::fs::write(output.join("collector.stdout.txt"),&result.stdout).unwrap();std::fs::write(output.join("collector.stderr.txt"),&result.stderr).unwrap();assert!(result.status.success(),"Normal collector failed; {}",output.display());
}
#[test]#[ignore="Root-only shared DB slot: actual diagnostic capture with labelled source loss"]
fn automatic_debug_capture_actual_components(){
 assert_eq!(std::env::var("NT2_C3_COMPOSITION").as_deref(),Ok("1"));assert_eq!(std::env::var("NT2_A3_FIXTURE").as_deref(),Ok("1"));
 assert_eq!(std::env::var("NT2_CLIENT_DEBUG_TEST_ROOT"),std::env::var("NT2_SERVER_DIAGNOSTIC_TEST_ROOT"));
 std::thread::Builder::new().name("automatic-debug".into()).stack_size(16*1024*1024).spawn(||tauri::async_runtime::block_on(async{
  verify_current_fixture_inputs();let own=repo().join("spikes/noita-entry/.build/automatic-debug-20261023-01").join(format!("composition-{}",uuid::Uuid::new_v4()));std::fs::create_dir_all(&own).unwrap();eprintln!("AUTOMATIC_DEBUG_COMPOSITION {}",own.display());
  let mut service=Service::start(&own);network_profile::enable_elapsed_clock(&mut service,&own);let router=PipeHarness::new();
  let mut a=Client::start(&service.private(),&own.join("host"),None,false,true,router.routes.clone(),service.watches.clone(),service.stores.clone()).await;bundle(&mut service,&a).await;let mut na=Native::start(&a,None,false);
  wait(&mut service,&a,"debug host Ready",30,|v|current_state_is(v,"Ready")&&na.root.join("associated").is_file()).await;let wa=a.handle.view()["rejoin"]["current_world"].clone();
  service.command(json!({"op":"peer"}));let mut b=Client::start(&service.private(),&own.join("peer"),None,true,true,router.routes.clone(),service.watches.clone(),service.stores.clone()).await;bundle(&mut service,&b).await;let mut nb=Native::start(&b,None,false);
  wait(&mut service,&b,"debug peer Ready",30,|v|current_state_is(v,"Ready")&&nb.root.join("associated").is_file()).await;let wb=b.handle.view()["rejoin"]["current_world"].clone();
  two_peer_ghost_with_start(&mut service,&a,&na,&wa,&b,&nb,&wb,"startReviewed").await;
  until(&mut service,"persisted app and mod healthy samples",||persisted_debug(&b,"mod")["recent"].as_array().is_some_and(|r|!r.is_empty())).await;
  let baseline=persisted_debug(&b,"app");assert!(baseline["firstLoss"].is_null(),"Unlabelled loss occurred before stimulus");
  std::fs::write(own.join("stimulus-utc-ms"),nt2_desktop_g0::debug_diagnostics::utc_ms().to_string()).unwrap();fixture_signal::write(na.root.join("debug-pause-movement"),"labelled injected source suppression; canonical membership and native heartbeat continue").unwrap();
  until(&mut service,"actual persisted loss at app and mod boundaries",||!persisted_debug(&b,"app")["firstLoss"].is_null()&&!persisted_debug(&b,"mod")["firstLoss"].is_null()).await;
  assert_eq!(b.handle.view()["connection"]["state"],"Playing");let first=persisted_debug(&b,"mod")["firstLoss"].clone();capture_debug(&b,&own,"during-loss");
  fixture_signal::remove(na.root.join("debug-pause-movement")).unwrap();
  until(&mut service,"persisted normal recovery",||!persisted_debug(&b,"app")["recovery"].is_null()&&!persisted_debug(&b,"mod")["recovery"].is_null()).await;
  fresh_bidirectional_ghosts(&mut service,&a,&na,&b,&nb,"debug loss recovered without authority change").await;assert_eq!(persisted_debug(&b,"mod")["firstLoss"],first);
  app_action(&b,json!({"action":"set_debug_diagnostics","debug_diagnostics":false})).await;elapsed(&mut service,2).await;
  assert_eq!(b.handle.view()["debug_diagnostics"]["enabled"],false);assert_eq!(b.handle.view()["connection"]["state"],"Playing");capture_debug(&b,&own,"debug-disabled");
  app_action(&b,json!({"action":"set_debug_diagnostics","debug_diagnostics":true})).await;elapsed(&mut service,2).await;assert_eq!(b.handle.view()["debug_diagnostics"]["enabled"],true);
  fresh_bidirectional_ghosts(&mut service,&a,&na,&b,&nb,"debug reenabled ordinary movement").await;
  na.close();nb.close();a.close().await;b.close().await;service.close();
  assert_eq!(persisted_debug(&b,"app")["closed"],true);capture_debug(&b,&own,"after-exit");
  std::fs::write(own.join("checks.json"),serde_json::to_vec_pretty(&json!({"passed":true,"actualClientServerNativeMod":true,"normalCollector":true,"clipboardRead":false,"engineFactsInjected":true,"labelledSourceLossInjected":true,"actualNoita":false,"installedKitChanged":false,"sharedSchemaCleanupRequiredAndPerformedByFixture":true,"firstLossRetained":true,"offOnMovementHealthy":true})).unwrap()).unwrap();
 })).unwrap().join().unwrap();
}
