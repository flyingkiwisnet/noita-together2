#[test]#[ignore="Root serialized shared death end actual Core/service/Lua/native composition"]
fn c1_shared_death_end_ordinary_scheduler_regression(){
 assert_eq!(std::env::var("NT2_C1_PLAYER_FIXTURE").as_deref(),Ok("1"));
 std::thread::Builder::new().name("shared-death-end".into()).stack_size(32*1024*1024).spawn(||tauri::async_runtime::block_on(async{
  verify_current_fixture_inputs();let own=repo().join("spikes/noita-entry/.build/end-death-client-20261031-01").join(format!("composition-{}",uuid::Uuid::new_v4()));std::fs::create_dir_all(&own).unwrap();eprintln!("SHARED_DEATH_END {}",own.display());
  let mut service=Service::start(&own);network_profile::enable_elapsed_clock(&mut service,&own);let router=PipeHarness::new();
  let mut a=Client::start(&service.private(),&own.join("a"),None,false,true,router.routes.clone(),service.watches.clone(),service.stores.clone()).await;bundle(&mut service,&a).await;let mut na=Native::start(&a,None,false);
  wait(&mut service,&a,"host Ready",40,|v|current_state_is(v,"Ready")&&na.root.join("associated").is_file()).await;
  service.command(json!({"op":"peer"}));let mut b=Client::start(&service.private(),&own.join("b"),None,true,true,router.routes.clone(),service.watches.clone(),service.stores.clone()).await;bundle(&mut service,&b).await;let mut nb=Native::start(&b,None,false);
  wait(&mut service,&b,"peer Ready",40,|v|current_state_is(v,"Ready")&&nb.root.join("associated").is_file()).await;
  let mut proofs=vec![];
  for scenario in ["normal-lost-reply","paused-outage-recipient"]{
   let wa=a.handle.view()["rejoin"]["current_world"].clone();let wb=b.handle.view()["rejoin"]["current_world"].clone();two_peer_ghost_with_start(&mut service,&a,&na,&wa,&b,&nb,&wb,"startReviewed").await;
   until(&mut service,"both production Lua owners received End death with Kick off",||[c1_view(&na),c1_view(&nb)].iter().all(|v|v["ready"]==true&&v["rules"]["NT_death_penalty"]=="end"&&v["rules"]["NT_ondeath_kick"]==false)).await;
   let peer_max_before=c1_hp(&nb,"max");let source_loss=expect_loss(&mut service,&na,"End death source").await;source_loss.before_fault(&na);let peer_loss=expect_loss(&mut service,&nb,"End death peer").await;peer_loss.before_fault(&nb);
   if scenario=="paused-outage-recipient"{fixture_signal::write(nb.root.join("pause-game-thread"),"paused recipient").unwrap();until(&mut service,"recipient game thread paused",||nb.root.join("game-thread-paused").is_file()).await;service.command(json!({"op":"transport","kind":"http","enabled":true,"target":"peer"}));}
   service.command(json!({"op":"c1DropPublishReply","target":"consumer"}));c1_action(&na,"death");
   until(&mut service,"local death executes legacy GameOver",||c1_view(&na)["gameOvers"]==1).await;
   if scenario=="paused-outage-recipient"{elapsed(&mut service,8).await;service.command(json!({"op":"transport","kind":"http","enabled":false,"target":"peer"}));fixture_signal::remove(nb.root.join("pause-game-thread")).unwrap();}
   let last_progress=std::cell::Cell::new(Instant::now());until(&mut service,"canonical shared end reaches source and recipient Lua",||{if last_progress.get().elapsed()>Duration::from_secs(5){std::fs::write(own.join("current-end-progress.json"),serde_json::to_vec_pretty(&json!({"a":a.handle.view(),"b":b.handle.view(),"sourceLua":c1_view(&na),"peerLua":c1_view(&nb)})).unwrap()).unwrap();last_progress.set(Instant::now());}c1_view(&na)["gameOvers"]==1&&c1_view(&nb)["gameOvers"]==1&&a.handle.view()["connection"]["state"]=="Ended"&&b.handle.view()["connection"]["state"]=="Ended"}).await;
   for (client,world) in [(&a,&wa),(&b,&wb)]{let v=client.handle.view();assert_ne!(v["rejoin"]["host_started"],true);assert_ne!(v["g3"]["live_transport"]["grant_active"],true);assert_eq!(server_world(&mut service,world)["terminal_reason"],"player_death_end");}
   elapsed(&mut service,3).await;assert_eq!(c1_view(&nb)["gameOvers"],1);assert_eq!(c1_hp(&nb,"max"),peer_max_before,"End must not apply weak penalty");
   proofs.push(json!({"scenario":scenario,"source":c1_view(&na),"peer":c1_view(&nb),"audit":service.command(json!({"op":"c1Status"}))}));
   na.close();nb.close();
   // Existing explicit host next-run + Join path produces new grants/worlds;
   // neither New Game nor a saved terminal world may reuse the earlier Start.
   service.command(json!({"op":"nextRun"}));let room=service.private()["room"]["roomId"].as_str().unwrap().to_owned();join_current(&a,room.clone()).await;join_current(&b,room).await;let nexta=bundle(&mut service,&a).await;let nextb=bundle(&mut service,&b).await;
   na=Native::start(&a,None,false);nb=Native::start(&b,None,false);ready_owner(&mut service,&a,&na,&nexta.0,false).await;ready_owner(&mut service,&b,&nb,&nextb.0,false).await;assert_ne!(nexta.0,wa);assert_ne!(nextb.0,wb);assert!(!na.root.join("playing").exists());assert!(!nb.root.join("playing").exists());
  }
  let wa=a.handle.view()["rejoin"]["current_world"].clone();let wb=b.handle.view()["rejoin"]["current_world"].clone();
  // The fixture's two playing desktops are consumers. Sign in a separate
  // canonical host management desktop; it never joins or prepares a game.
  service.command(json!({"op":"hostApp"}));let mut host_private=service.private();let credentials=host_private["hostApp"].clone();assert_eq!(credentials["accountId"],host_private["room"]["hostAccountId"]);assert_ne!(credentials["accountId"],a.handle.view()["account_id"]);host_private["desktop"]=credentials.clone();host_private["deviceId"]=credentials["deviceId"].clone();
  let mut host=Client::start(&host_private,&own.join("host-management"),None,false,false,router.routes.clone(),service.watches.clone(),service.stores.clone()).await;
  until(&mut service,"app displayed both fresh-world inspections",||host.handle.view()["inspections"].as_array().is_some_and(|rows|rows.len()==2&&rows.iter().all(|r|r["worldToken"]==wa||r["worldToken"]==wb))).await;
  let room=host.handle.view()["room"]["roomId"].clone();let reviewed=app_action(&host,json!({"action":"refresh_lobby","room":room})).await;
  let mut capture=json!({});for key in ["server_origin","service_id","service_epoch","session_epoch","desktop_id","account_id","room","inspections"]{capture[key]=reviewed[key].clone();}
  let participants:Vec<Value>=reviewed["inspections"].as_array().unwrap().iter().map(|r|json!({"desktopId":r["desktopId"],"attachmentId":r["attachmentId"],"worldToken":r["worldToken"],"observationRevision":r["observationRevision"],"contentDigest":r["contentDigest"]})).collect();assert_eq!(participants.len(),2);
  // Explicit synthetic task-switch input. The production app Start refreshes
  // the displayed review; normal engine callbacks resume after its response.
  for native in [&na,&nb]{fixture_signal::write(native.root.join("pause-game-thread"),"synthetic staged task switch for app Start").unwrap();fixture_signal::write(native.root.join("start"),"host Start requested").unwrap();}
  until(&mut service,"staged game threads paused for app Start",||[&na,&nb].iter().all(|n|n.root.join("game-thread-paused").is_file())).await;
  app_action(&host,json!({"action":"start","participants":participants,"excluded":[],"config_revision":reviewed["room"]["configRevision"],"start_capture":capture})).await;
  for native in [&na,&nb]{fixture_signal::remove(native.root.join("pause-game-thread")).unwrap();}
  two_peer_ghost_after_start(&mut service,&a,&na,&wa,&b,&nb,&wb).await;
  host.handle.command(|c|Box::pin(async move{assert!(c.game.is_none()&&c.world.record.is_none()&&c.pending.lock().unwrap().is_none());assert_eq!(c.room.as_ref().unwrap().host_account_id.as_deref(),c.desktop.as_ref().map(|d|d.account.as_str()));Ok(Value::Null)})).await.unwrap();
  na.close();nb.close();host.close().await;a.close().await;b.close().await;service.close();std::fs::write(own.join("result.json"),serde_json::to_vec_pretty(&json!({"passed":true,"actualOrdinarySchedulerServiceLuaNative":true,"cases":proofs,"freshRunAndStart":true,"finalStart":"canonical host management-only Core app start_captured with explicit synthetic staged task switch; resumed callbacks and current bidirectional ghosts","actualNoita":false,"engineFacts":"injected engine APIs and death callback; production Lua handlers, native IPC, Core and service"})).unwrap()).unwrap();
 })).unwrap().join().unwrap();
}

#[test]#[ignore="Root serialized peer initial Core tick diagnosis; source helper only"]
fn c1_initial_core_tick_subphase_diagnosis(){
 std::thread::Builder::new().name("initial-tick-diagnosis".into()).stack_size(32*1024*1024).spawn(||tauri::async_runtime::block_on(async{
  verify_current_fixture_inputs();let own=repo().join("spikes/noita-entry/.build/end-death-client-20261031-01").join(format!("initial-tick-{}",uuid::Uuid::new_v4()));std::fs::create_dir_all(&own).unwrap();eprintln!("INITIAL_TICK {}",own.display());
  let mut service=Service::start(&own);network_profile::enable_elapsed_clock(&mut service,&own);let router=PipeHarness::new();let mut clients=Vec::new();let mut observations=Vec::new();let mut source_native=None;
  for peer in [false,true]{if peer{service.command(json!({"op":"peer"}));}let side=if peer{"b"}else{"a"};let c=Client::start(&service.private(),&own.join(side),None,peer,true,router.routes.clone(),service.watches.clone(),service.stores.clone()).await;
   let timing=c.handle.fixture_timing();observations.push(json!({"side":side,"before":timing}));
   bundle(&mut service,&c).await;
   // Same first-peer-tick orchestration: source is held Ready, but no peer
   // Native, death, Start, retries or synthetic pause are involved.
   let current=current_snapshot(&c,None).await;let timing=c.handle.fixture_timing();observations.push(json!({"side":side,"after":timing,"current":current["fixture_current"]}));if !peer{let native=Native::start(&c,None,false);wait(&mut service,&c,"diagnostic source Ready",40,|v|current_state_is(v,"Ready")&&native.root.join("associated").is_file()).await;source_native=Some(native);}std::fs::write(own.join("subphases.json"),serde_json::to_vec_pretty(&observations).unwrap()).unwrap();clients.push(c);
  }
  if let Some(mut native)=source_native{native.close();}
  for c in &mut clients{c.close().await;observations.push(json!({"closed":c.root.file_name().unwrap().to_string_lossy(),"timing":c.handle.fixture_timing()}));}service.close();std::fs::write(own.join("result.json"),serde_json::to_vec_pretty(&json!({"passed":true,"initialTicks":observations,"peerNativeNeverStarted":true,"sourceNativeHeldReady":true,"noDeathOrStart":true})).unwrap()).unwrap();
 })).unwrap().join().unwrap();
}
