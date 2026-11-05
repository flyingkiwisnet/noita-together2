// Actual app/service/TLS/SQL/native pipe and ordinary scheduling. Noita engine
// facts and physical pickup callbacks are injected by lifecycle-lc1-c1.lua.
include!("next_c1_death_end_composition_test.rs");
fn c1_view(n:&Native)->Value{fixture_signal::read(n.root.join("c1-view.json")).unwrap().and_then(|s|serde_json::from_str(&s).ok()).unwrap_or(Value::Null)}
fn c1_hp(n:&Native,key:&str)->f64{c1_view(n)[key].as_str().and_then(|s|s.parse().ok()).unwrap_or(-1.0)}
fn c1_action(n:&Native,kind:&str){fixture_signal::write(n.root.join("c1-action"),serde_json::to_vec(&json!({"kind":kind})).unwrap()).unwrap();}
fn c1_data(n:&Native,value:Value){fixture_signal::write(n.root.join("c1-action"),serde_json::to_vec(&value).unwrap()).unwrap();}
async fn c1_delayed_progress(service:&mut Service,a:&Client,na:&Native,b:&Client,nb:&Native){
 let before=[current_snapshot(a,Some(na)).await,current_snapshot(b,Some(nb)).await];
 let after=wait_pair(service,a,na,b,nb,"C1 delayed transport retains advancing poses/rendering",20,|views|views.iter().enumerate().all(|(i,v)|current_state_is(v,"Playing")&&v["fixture_current"]["identity"]==before[i]["fixture_current"]["identity"]&&v["fixture_current"]["native_controls"]==true&&v["fixture_current"]["native"]["ghosts"]==1&&v["fixture_current"]["native"]["ghost_motion_updates"].as_u64().unwrap_or(0)>before[i]["fixture_current"]["native"]["ghost_motion_updates"].as_u64().unwrap_or(0)&&v["g3"]["movement"]["publications"].as_u64().unwrap_or(0)>before[i]["g3"]["movement"]["publications"].as_u64().unwrap_or(0)+1&&v["g3"]["movement"]["receives"].as_u64().unwrap_or(0)>before[i]["g3"]["movement"]["receives"].as_u64().unwrap_or(0)+1)).await;
 std::fs::write(a.root.parent().unwrap().join("c1-delayed-progress.json"),serde_json::to_vec_pretty(&json!({"before":before,"after":after,"network":service.command(json!({"op":"networkStatus"}))})).unwrap()).unwrap();
}
async fn c1_retired_audit(service:&mut Service,target:&str,revision:&Value)->Value{
 let until=Instant::now()+Duration::from_secs(30);loop{service.wall_clock();let audit=service.command(json!({"op":"c1Status"}));if audit["c1"]["events"].as_array().is_some_and(|events|events.iter().any(|e|e["source"]==target&&e["kind"]=="death"&&e["rulesRevision"]==*revision&&e["sourceDisposition"]=="applied")){return audit}assert!(Instant::now()<until,"Retired C1 source metadata completion deadline: {audit}");tokio::time::sleep(Duration::from_secs(1)).await;}
}
async fn c1_rules(service:&mut Service,na:&Native,nb:&Native,patch:Value){
 let old=service.command(json!({"op":"c1Rules"}));let next=service.command(json!({"op":"c1Rules","expectedRevision":old["snapshot"]["revision"],"rules":patch}));let revision=next["snapshot"]["revision"].clone();
 until(service,"atomic live policy reaches both current worlds",||c1_view(na)["revision"]==revision&&c1_view(nb)["revision"]==revision).await;
}
#[test]#[ignore="Root serialized C1 player rules actual-component composition"]
fn c1_player_rules_ordinary_scheduler_composition(){
 assert_eq!(std::env::var("NT2_C3_COMPOSITION").as_deref(),Ok("1"));assert_eq!(std::env::var("NT2_C1_PLAYER_FIXTURE").as_deref(),Ok("1"));
 std::thread::Builder::new().name("c1-player-rules".into()).stack_size(16*1024*1024).spawn(||tauri::async_runtime::block_on(async{
  verify_current_fixture_inputs();let own=repo().join("spikes/noita-entry/.build/c1-client-20261026-01").join(format!("composition-{}",uuid::Uuid::new_v4()));std::fs::create_dir_all(&own).unwrap();eprintln!("C1_PLAYER_COMPOSITION {}",own.display());
  let mut service=Service::start(&own);network_profile::enable_elapsed_clock(&mut service,&own);let router=PipeHarness::new();
  let mut a=Client::start(&service.private(),&own.join("host"),None,false,true,router.routes.clone(),service.watches.clone(),service.stores.clone()).await;bundle(&mut service,&a).await;let mut na=Native::start(&a,None,false);
  wait(&mut service,&a,"C1 host Ready",30,|v|current_state_is(v,"Ready")&&na.root.join("associated").is_file()).await;let wa=a.handle.view()["rejoin"]["current_world"].clone();
  service.command(json!({"op":"peer"}));let mut b=Client::start(&service.private(),&own.join("peer"),None,true,true,router.routes.clone(),service.watches.clone(),service.stores.clone()).await;bundle(&mut service,&b).await;let mut nb=Native::start(&b,None,false);
  wait(&mut service,&b,"C1 peer Ready",30,|v|current_state_is(v,"Ready")&&nb.root.join("associated").is_file()).await;let wb=b.handle.view()["rejoin"]["current_world"].clone();
  two_peer_ghost_with_start(&mut service,&a,&na,&wa,&b,&nb,&wb,"startReviewed").await;
  until(&mut service,"C1 current rules and retained membership before pickup",||c1_view(&na)["ready"]==true&&c1_view(&nb)["ready"]==true&&c1_view(&na)["count"]==2&&c1_view(&nb)["count"]==2).await;
  for n in [&na,&nb]{let bytes=fixture_signal::read(n.root.join("c1-protected-bootstrap.json")).unwrap().unwrap();let value:Value=serde_json::from_str(&bytes).unwrap();assert_eq!(value["controls"],false);assert_eq!(value["mutations"],0);assert_eq!(value["queued"],0);}
  assert_eq!(c1_hp(&na,"hp"),2.0);assert_eq!(c1_hp(&nb,"hp"),2.0);
  c1_action(&na,"heart");until(&mut service,"normal local pickup reaches other production handler",||c1_hp(&na,"max")==4.5&&c1_hp(&nb,"max")==4.5&&c1_hp(&nb,"hp")==2.5).await;
  assert_eq!(c1_hp(&na,"hp"),2.0,"Collector quirk preserved; no sender echo healing");
  let first=c1_view(&nb)["event"].clone();let mutations=c1_view(&nb)["mutations"].as_u64().unwrap();
  c1_action(&nb,"repeat_delivery");elapsed(&mut service,2).await;assert_eq!(c1_hp(&nb,"hp"),2.5);assert_eq!(c1_view(&nb)["mutations"].as_u64().unwrap(),mutations);
  c1_action(&na,"large_heart");until(&mut service,"large local heart retains ordinary remote event",||c1_hp(&na,"max")==5.5&&c1_hp(&nb,"max")==5.0&&c1_hp(&nb,"hp")==3.0).await;
  assert_eq!(c1_hp(&na,"hp"),2.0);
  ready_owner(&mut service,&a,&na,&wa,true).await;ready_owner(&mut service,&b,&nb,&wb,true).await;fresh_bidirectional_ghosts(&mut service,&a,&na,&b,&nb,"C1 effects preserve movement and HUD").await;
  let gold=bank_submit(&mut service,&na,"deposit",5,false).await;accepted(&mut service,&na,&gold).await;until(&mut service,"bank remains responsive beside C1",||bank_view(&na)["wallet"]==995&&bank_view(&nb)["view"]["balance"]==5).await;
  c1_data(&na,json!({"kind":"perk","id":"EXTRA_MONEY"}));until(&mut service,"team perk uses actual source and receiving hooks",||c1_view(&nb)["perks"]["EXTRA_MONEY"]==1).await;
  c1_data(&na,json!({"kind":"perk","id":"EXTRA_MONEY"}));until(&mut service,"later legitimate perk stack is distinct",||c1_view(&nb)["perks"]["EXTRA_MONEY"]==2).await;
  c1_rules(&mut service,&na,&nb,json!({"NT_sync_perks":true})).await;
  c1_data(&na,json!({"kind":"perk","id":"EDIT_WANDS_EVERYWHERE"}));until(&mut service,"all-perk policy shares outside team subset",||c1_view(&nb)["perks"]["EDIT_WANDS_EVERYWHERE"]==1).await;
  c1_data(&na,json!({"kind":"orb","id":0}));until(&mut service,"forced orb local component and delayed engine pickup",||c1_view(&nb)["orbZero"]==true).await;
  c1_action(&na,"fungal");until(&mut service,"chosen named multi-material mapping converts without recipient reroll",||c1_view(&nb)["conversions"]==2).await;
  c1_action(&na,"fungal");elapsed(&mut service,3).await;assert_eq!(c1_view(&nb)["conversions"],2,"Cooldown is completed skip, not delayed repeat");
  c1_rules(&mut service,&na,&nb,json!({"NT_death_penalty":"weak_respawn"})).await;c1_action(&na,"checkpoint");until(&mut service,"checkpoint callback consumed",||!na.root.join("c1-action").exists()).await;
  c1_action(&na,"death");until(&mut service,"weak death heals self and applies one remote penalty",||(c1_hp(&na,"max")-4.4).abs()<1e-8&&(c1_hp(&nb,"max")-4.0).abs()<1e-8&&c1_hp(&na,"hp")==c1_hp(&na,"max")).await;
  assert_eq!(c1_view(&na)["checkpointX"],"50");assert_eq!(c1_view(&na)["checkpointY"],"-20");
  elapsed(&mut service,3).await;assert!((c1_hp(&nb,"max")-4.0).abs()<1e-8);
  c1_rules(&mut service,&na,&nb,json!({"NT_death_penalty":"full_respawn"})).await;c1_action(&na,"death");until(&mut service,"later full respawn has new identity and no penalty",||c1_view(&na)["deaths"]==2&&c1_hp(&na,"hp")==c1_hp(&na,"max")).await;elapsed(&mut service,2).await;assert!((c1_hp(&nb,"max")-4.0).abs()<1e-8);
  for target in ["consumer","peer"]{service.command(json!({"op":"c1Network","target":target,"durationMs":120000}));}
  service.command(json!({"op":"c1DropPublishReply","target":"consumer"}));let before=c1_hp(&nb,"max");c1_action(&na,"heart");until(&mut service,"committed lost reply retries one original without duplicate HP",||(c1_hp(&nb,"max")-(before+0.5)).abs()<1e-8).await;elapsed(&mut service,3).await;assert!((c1_hp(&nb,"max")-(before+0.5)).abs()<1e-8);
  c1_delayed_progress(&mut service,&a,&na,&b,&nb).await;
  let mut outage=expect_loss(&mut service,&nb,"C1 retained delivery during peer interruption").await;outage.before_fault(&nb);
  for kind in ["http","wss"]{service.command(json!({"op":"transport","kind":kind,"enabled":true,"target":"peer"}));}
  until(&mut service,"C1 interrupted peer held",||native_activity(&nb)["native_owned_hold"]==true).await;outage.protected(&nb);let before=c1_hp(&nb,"max");c1_action(&na,"heart");elapsed(&mut service,2).await;assert_eq!(c1_hp(&nb,"max"),before);assert_eq!(c1_view(&na)["count"],2);
  for kind in ["http","wss"]{service.command(json!({"op":"transport","kind":kind,"enabled":false,"target":"peer"}));}
  ready_owner(&mut service,&b,&nb,&wb,true).await;outage.rearm(&mut service,&nb).await;until(&mut service,"eligible retained delivery progresses after interruption",||(c1_hp(&nb,"max")-(before+0.5)).abs()<1e-8).await;
  for target in ["consumer","peer"]{service.command(json!({"op":"networkReset","target":target}));}
  let mut own_death=expect_loss(&mut service,&na,"C1 genuine own death during expired connection protection").await;own_death.before_fault(&na);
  for kind in ["http","wss"]{service.command(json!({"op":"transport","kind":kind,"enabled":true,"target":"consumer"}));}
  until(&mut service,"C1 own death precondition is genuine connection protection",||native_activity(&na)["native_owned_hold"]==true).await;own_death.protected(&na);let maximum=c1_hp(&na,"max");c1_action(&na,"death");elapsed(&mut service,2).await;assert_eq!(c1_hp(&na,"hp"),-1.0);assert_eq!(c1_hp(&na,"max"),maximum);assert_eq!(native_activity(&na)["controls_enabled"],false);
  for kind in ["http","wss"]{service.command(json!({"op":"transport","kind":kind,"enabled":false,"target":"consumer"}));}
  ready_owner(&mut service,&a,&na,&wa,true).await;own_death.rearm(&mut service,&na).await;assert_eq!(c1_hp(&na,"hp"),maximum);
  let protected:Value=serde_json::from_str(&fixture_signal::read(na.root.join("c1-protected-completion.json")).unwrap().unwrap()).unwrap();assert_eq!(protected["controls"],false);assert_eq!(protected["eventAbsent"],true);assert_eq!(protected["permitAbsent"],true);
  until(&mut service,"prior C1 notices and effects fully drained before known-unissued cold case",||c1_view(&nb)["event"].is_null()&&b.handle.view()["g3"]["c1"]["event"].is_null()).await;
  fixture_signal::write(nb.root.join("c1-missing-actor"),"injected actor absence").unwrap();until(&mut service,"missing actor parks C1 without inventing death",||c1_view(&nb)["playing"]==false).await;c1_action(&na,"heart");elapsed(&mut service,3).await;
  let saved=nb.root.clone();let old_store=b.store.clone();let old_root=b.root.clone();nb.close();b.close().await;let cold_root=own.join("peer-cold");std::fs::create_dir_all(&cold_root).unwrap();
  for name in ["transfers.jsonl","c1-effects.jsonl"]{let path=old_root.join(name);if path.exists(){std::fs::copy(path,cold_root.join(name)).unwrap();}}
  b=Client::start(&service.private(),&cold_root,Some(old_store),true,false,router.routes.clone(),service.watches.clone(),service.stores.clone()).await;nb=Native::start(&b,Some(&saved),true);
  wait(&mut service,&b,"C1 cold app explicit saved-world choice",15,|v|v["connection"]["state"]=="Choice").await;fixture_signal::write(nb.root.join("choose-rejoin"),"explicit rejoin").unwrap();ready_owner(&mut service,&b,&nb,&wb,true).await;
  until(&mut service,"same-world known-unissued effect progresses after cold app",||(c1_hp(&nb,"max")-4.5).abs()<1e-8&&(c1_hp(&nb,"hp")-2.5).abs()<1e-8).await;
  let audit=service.command(json!({"op":"c1Status"}));std::fs::write(own.join("c1-audit.json"),serde_json::to_vec_pretty(&audit).unwrap()).unwrap();
  fixture_signal::write(nb.root.join("unaffected-check"),"monitor the replacement peer fixture after cold restore").unwrap();
  until(&mut service,"replacement peer current health snapshot",||nb.root.join("unaffected-snapshot").is_file()).await;
  fresh_bidirectional_ghosts(&mut service,&a,&na,&b,&nb,"C1 recovered movement and HUD").await;
  until(&mut service,"known-unissued cold effect receipt drained before explicit partial failure",||c1_view(&nb)["event"].is_null()&&b.handle.view()["g3"]["c1"]["event"].is_null()).await;
  fixture_signal::write(nb.root.join("c1-partial-fail"),"injected failure after real production HP mutation").unwrap();c1_action(&na,"heart");until(&mut service,"partial effect becomes uncertain without retry",||b.handle.view()["g3"]["c1"]["status"].as_str().is_some_and(|s|s.contains("uncertain"))).await;assert_eq!(c1_hp(&nb,"hp"),3.0);assert_eq!(c1_hp(&nb,"max"),4.5);let partial_mutations=c1_view(&nb)["mutations"].clone();elapsed(&mut service,2).await;assert_eq!(c1_view(&nb)["mutations"],partial_mutations);
  let saved=nb.root.clone();let old_store=b.store.clone();let old_root=b.root.clone();nb.close();b.close().await;let cold_root=own.join("peer-uncertain-cold");std::fs::create_dir_all(&cold_root).unwrap();for name in ["transfers.jsonl","c1-effects.jsonl"]{let path=old_root.join(name);if path.exists(){std::fs::copy(path,cold_root.join(name)).unwrap();}}
  b=Client::start(&service.private(),&cold_root,Some(old_store),true,false,router.routes.clone(),service.watches.clone(),service.stores.clone()).await;nb=Native::start(&b,Some(&saved),true);wait(&mut service,&b,"uncertain cold explicit saved-world choice",15,|v|v["connection"]["state"]=="Choice").await;fixture_signal::write(nb.root.join("choose-rejoin"),"explicit rejoin").unwrap();ready_owner(&mut service,&b,&nb,&wb,true).await;until(&mut service,"cold ambiguous effect stays blocked with no engine replay",||c1_view(&nb)["status"].as_str().is_some_and(|s|s.contains("uncertain"))).await;assert_eq!(c1_hp(&nb,"hp"),2.0);assert_eq!(c1_hp(&nb,"max"),4.0);assert_eq!(c1_view(&nb)["mutations"],0);
  c1_rules(&mut service,&na,&nb,json!({"NT_ondeath_kick":true})).await;
  let winner=expect_loss(&mut service,&nb,"C1 genuine winning terminal").await;winner.before_fault(&nb);service.command(json!({"op":"c1DropPublishReply","target":"peer"}));c1_action(&nb,"win_death");
  let won=c1_retired_audit(&mut service,"peer",&c1_view(&na)["revision"]).await;std::fs::write(own.join("c1-winner-retired.json"),serde_json::to_vec_pretty(&won).unwrap()).unwrap();assert_eq!(c1_hp(&nb,"hp"),-1.0);assert_eq!(c1_view(&nb)["terminal"],true);
  let progress=bank_submit(&mut service,&na,"deposit",1,false).await;accepted(&mut service,&na,&progress).await;
  let current=service.command(json!({"op":"c1Rules"}));let next=service.command(json!({"op":"c1Rules","expectedRevision":current["snapshot"]["revision"],"rules":{"NT_death_penalty":"end"}}));until(&mut service,"terminal world cannot consume later policy; live source receives it",||c1_view(&na)["revision"]==next["snapshot"]["revision"]).await;
  let terminal=expect_loss(&mut service,&na,"C1 terminal source kick with lost committed reply").await;terminal.before_fault(&na);service.command(json!({"op":"c1DropPublishReply","target":"consumer"}));let revision=c1_view(&na)["revision"].clone();c1_action(&na,"death");
  let retired=c1_retired_audit(&mut service,"consumer",&revision).await;std::fs::write(own.join("c1-terminal-retired.json"),serde_json::to_vec_pretty(&retired).unwrap()).unwrap();assert_eq!(c1_hp(&na,"hp"),-1.0);
  service.command(json!({"op":"end"}));elapsed(&mut service,1).await;
  let final_a=c1_view(&na);let final_b=c1_view(&nb);na.close();nb.close();a.close().await;b.close().await;service.close();
  std::fs::write(own.join("checks.json"),serde_json::to_vec_pretty(&json!({"passed":true,"actualComponents":true,"ordinaryScheduler":true,"engineFactsInjected":true,"actualNoita":false,"savePersistenceProven":false,"normalHeartCollectorUnhealed":true,"largeHeartOrdinaryRemote":true,"duplicatesDidNotMutate":true,"membershipCount":2,"firstDelivery":first,"finalHost":final_a,"finalPeer":final_b,"bankAndGhostsProgressed":true})).unwrap()).unwrap();
 })).unwrap().join().unwrap();
}

#[test]#[ignore="Root serialized C1 respawn-kick actual-component acceptance"]
fn c1_player_kick_respawn_ordinary_scheduler_composition(){
 assert_eq!(std::env::var("NT2_C3_COMPOSITION").as_deref(),Ok("1"));assert_eq!(std::env::var("NT2_C1_PLAYER_FIXTURE").as_deref(),Ok("1"));
 std::thread::Builder::new().name("c1-kick-respawn".into()).stack_size(16*1024*1024).spawn(||tauri::async_runtime::block_on(async{
  verify_current_fixture_inputs();let own=repo().join("spikes/noita-entry/.build/c1-client-20261026-01").join(format!("kick-{}",uuid::Uuid::new_v4()));std::fs::create_dir_all(&own).unwrap();eprintln!("C1_KICK_COMPOSITION {}",own.display());
  let mut service=Service::start(&own);network_profile::enable_elapsed_clock(&mut service,&own);let router=PipeHarness::new();
  let mut a=Client::start(&service.private(),&own.join("host"),None,false,true,router.routes.clone(),service.watches.clone(),service.stores.clone()).await;bundle(&mut service,&a).await;let mut na=Native::start(&a,None,false);
  wait(&mut service,&a,"C1 kick consumer Ready",30,|v|current_state_is(v,"Ready")&&na.root.join("associated").is_file()).await;let wa=a.handle.view()["rejoin"]["current_world"].clone();
  service.command(json!({"op":"peer"}));let mut b=Client::start(&service.private(),&own.join("peer"),None,true,true,router.routes.clone(),service.watches.clone(),service.stores.clone()).await;bundle(&mut service,&b).await;let mut nb=Native::start(&b,None,false);
  wait(&mut service,&b,"C1 kick peer Ready",30,|v|current_state_is(v,"Ready")&&nb.root.join("associated").is_file()).await;let wb=b.handle.view()["rejoin"]["current_world"].clone();two_peer_ghost_with_start(&mut service,&a,&na,&wa,&b,&nb,&wb,"startReviewed").await;
  c1_rules(&mut service,&na,&nb,json!({"NT_death_penalty":"weak_respawn","NT_ondeath_kick":true})).await;
  let mut consumer=expect_loss(&mut service,&na,"C1 weak respawn kick").await;consumer.before_fault(&na);service.command(json!({"op":"c1DropPublishReply","target":"consumer"}));let revision=c1_view(&na)["revision"].clone();c1_action(&na,"death");let weak=c1_retired_audit(&mut service,"consumer",&revision).await;
  until(&mut service,"weak kick fences completed own respawn",||native_activity(&na)["native_owned_hold"]==true).await;consumer.protected(&na);assert!((c1_hp(&na,"hp")-3.2).abs()<1e-8);until(&mut service,"healthy peer applies exactly one weak penalty",||(c1_hp(&nb,"max")-3.2).abs()<1e-8).await;
  let gold=bank_submit(&mut service,&nb,"deposit",1,false).await;accepted(&mut service,&nb,&gold).await;
  let current=service.command(json!({"op":"c1Rules"}));let next=service.command(json!({"op":"c1Rules","expectedRevision":current["snapshot"]["revision"],"rules":{"NT_death_penalty":"full_respawn"}}));until(&mut service,"remaining peer receives full respawn mode",||c1_view(&nb)["revision"]==next["snapshot"]["revision"]).await;
  let mut peer=expect_loss(&mut service,&nb,"C1 full respawn kick").await;peer.before_fault(&nb);service.command(json!({"op":"c1DropPublishReply","target":"peer"}));let revision=c1_view(&nb)["revision"].clone();c1_action(&nb,"death");let full=c1_retired_audit(&mut service,"peer",&revision).await;
  until(&mut service,"full kick fences completed own respawn",||native_activity(&nb)["native_owned_hold"]==true).await;peer.protected(&nb);assert!((c1_hp(&nb,"hp")-3.2).abs()<1e-8);elapsed(&mut service,2).await;
  for client in [&a,&b]{assert_ne!(client.handle.view()["g3"]["live_transport"]["grant_active"],true,"Late life completion revived removed gameplay");}
  service.command(json!({"op":"end"}));na.close();nb.close();a.close().await;b.close().await;service.close();std::fs::write(own.join("checks.json"),serde_json::to_vec_pretty(&json!({"passed":true,"actualComponents":true,"engineFactsInjected":true,"actualNoita":false,"weak":weak,"full":full,"lostCommittedRepliesRecovered":true,"removedOwnerNotRevived":true,"healthyPeerBankContinued":true})).unwrap()).unwrap();
 })).unwrap().join().unwrap();
}

#[test]#[ignore="Serialized owned C1 entry/bootstrap real-component regression"]
fn c1_entry_epoch_ordinary_scheduler_composition(){
 assert_eq!(std::env::var("NT2_C1_PLAYER_FIXTURE").as_deref(),Ok("1"));
 std::thread::Builder::new().name("c1-entry-epoch".into()).stack_size(16*1024*1024).spawn(||tauri::async_runtime::block_on(async{
  verify_current_fixture_inputs();let own=repo().join("spikes/noita-entry/.build/c1-entry-client-20261028-01").join(format!("composition-{}",uuid::Uuid::new_v4()));std::fs::create_dir_all(&own).unwrap();eprintln!("C1_ENTRY_COMPOSITION {}",own.display());
  let mut service=Service::start(&own);network_profile::enable_elapsed_clock(&mut service,&own);let router=PipeHarness::new();
  let mut client=Client::start(&service.private(),&own.join("host"),None,false,true,router.routes.clone(),service.watches.clone(),service.stores.clone()).await;
  // Independent cancellation turnover before the native world enters. The
  // prepared engine epoch remains owned by the original early bundle.
  client.handle.command(|c|Box::pin(async move{c.generation.fetch_add(7,Ordering::AcqRel);c.sync_background();Ok(Value::Null)})).await.unwrap();
  bundle(&mut service,&client).await;let mut native=Native::start(&client,None,false);
  wait(&mut service,&client,"C1 unequal-epoch native held Ready",30,|v|current_state_is(v,"Ready")&&native.root.join("associated").is_file()).await;
  let epochs=client.handle.command(|c|Box::pin(async move{let p=c.pending.lock().unwrap().clone().unwrap();Ok(json!({"cancellation":p.generation.as_ref().unwrap().1,"native":p.native_epoch}))})).await.unwrap();assert_ne!(epochs["cancellation"],epochs["native"]);std::fs::write(own.join("epoch-pair.json"),serde_json::to_vec_pretty(&epochs).unwrap()).unwrap();
  assert!(!native.root.join("playing").is_file());let world=client.handle.view()["rejoin"]["current_world"].clone();
  service.command(json!({"op":"startReviewed"}));fixture_signal::write(native.root.join("start"),b"actual reviewed Start").unwrap();
  wait(&mut service,&client,"C1 actual entry wrapper initializes then releases",30,|v|current_state_is(v,"Playing")&&c1_view(&native)["ready"]==true&&native.root.join("playing").is_file()).await;
  let bootstrap=fixture_signal::read(native.root.join("c1-protected-bootstrap.json")).unwrap().expect("Actual protected entry must consume C1 metadata before release");let bootstrap:Value=serde_json::from_str(&bootstrap).unwrap();assert_eq!(bootstrap["controls"],false);assert_eq!(bootstrap["mutations"],0);
  let fresh=current_snapshot(&client,Some(&native)).await;let saved=native.root.clone();let old_store=client.store.clone();let old_root=client.root.clone();native.close();client.close().await;
  let cold=own.join("cold");std::fs::create_dir_all(&cold).unwrap();for name in ["transfers.jsonl","c1-effects.jsonl"]{let path=old_root.join(name);if path.exists(){std::fs::copy(path,cold.join(name)).unwrap();}}
  client=Client::start(&service.private(),&cold,Some(old_store),false,false,router.routes.clone(),service.watches.clone(),service.stores.clone()).await;
  client.handle.command(|c|Box::pin(async move{c.generation.fetch_add(5,Ordering::AcqRel);c.sync_background();Ok(Value::Null)})).await.unwrap();
  native=Native::start(&client,Some(&saved),true);wait(&mut service,&client,"C1 retained Continue explicit choice",15,|v|v["connection"]["state"]=="Choice").await;fixture_signal::write(native.root.join("choose-rejoin"),b"explicit owner choice").unwrap();ready_owner(&mut service,&client,&native,&world,true).await;
  let continued=current_snapshot(&client,Some(&native)).await;assert_eq!(continued["rejoin"]["current_world"],world);assert_eq!(c1_view(&native)["ready"],true);
  native.close();client.close().await;service.close();std::fs::write(own.join("checks.json"),serde_json::to_vec_pretty(&json!({"passed":true,"actualComponents":true,"actualProductionEntryWrapper":true,"engineFactsInjected":true,"actualNoita":false,"savePersistenceProven":false,"unequalFreshEpochs":epochs,"protectedBootstrap":bootstrap,"fresh":fresh,"continued":continued})).unwrap()).unwrap();
 })).unwrap().join().unwrap();
}

#[test]#[ignore="Serialized owned C1 raw XML Save/Continue with both apps retained"]
fn c1_saved_globals_xml_ordinary_scheduler_composition(){
 assert_eq!(std::env::var("NT2_C1_PLAYER_FIXTURE").as_deref(),Ok("1"));
 std::thread::Builder::new().name("c1-save-xml".into()).stack_size(16*1024*1024).spawn(||tauri::async_runtime::block_on(async{
  verify_current_fixture_inputs();let own=repo().join("spikes/noita-entry/.build/c1-save-encoding-client-20261029-01").join(format!("composition-{}",uuid::Uuid::new_v4()));std::fs::create_dir_all(&own).unwrap();eprintln!("C1_SAVE_XML_COMPOSITION {}",own.display());
  let mut service=Service::start(&own);network_profile::enable_elapsed_clock(&mut service,&own);let router=PipeHarness::new();
  let mut a=Client::start(&service.private(),&own.join("host"),None,false,true,router.routes.clone(),service.watches.clone(),service.stores.clone()).await;bundle(&mut service,&a).await;let mut na=Native::start(&a,None,false);
  wait(&mut service,&a,"C1 save host Ready",30,|v|current_state_is(v,"Ready")&&na.root.join("associated").is_file()).await;let wa=a.handle.view()["rejoin"]["current_world"].clone();
  service.command(json!({"op":"peer"}));let mut b=Client::start(&service.private(),&own.join("peer"),None,true,true,router.routes.clone(),service.watches.clone(),service.stores.clone()).await;bundle(&mut service,&b).await;let mut nb=Native::start(&b,None,false);
  wait(&mut service,&b,"C1 save peer Ready",30,|v|current_state_is(v,"Ready")&&nb.root.join("associated").is_file()).await;let wb=b.handle.view()["rejoin"]["current_world"].clone();
  two_peer_ghost_with_start(&mut service,&a,&na,&wa,&b,&nb,&wb,"startReviewed").await;
  until(&mut service,"C1 save initialized rules on both",||c1_view(&na)["ready"]==true&&c1_view(&nb)["ready"]==true).await;
  c1_action(&na,"heart");until(&mut service,"C1 pre-save restricted pickup shares",||c1_hp(&na,"max")==4.5&&c1_hp(&nb,"max")==4.5).await;
  c1_data(&na,json!({"kind":"perk","id":"EXTRA_MONEY"}));until(&mut service,"C1 pre-save team perk shares",||c1_view(&nb)["perks"]["EXTRA_MONEY"]==1).await;
  until(&mut service,"C1 settled Save after immutable originals and effect receipts acknowledged",||c1_view(&na)["queued"]==0&&c1_view(&nb)["queued"]==0&&c1_view(&na)["event"].is_null()&&c1_view(&nb)["event"].is_null()&&a.handle.view()["g3"]["c1"]["event"].is_null()&&b.handle.view()["g3"]["c1"]["event"].is_null()).await;
  let before_a=c1_view(&na);let before_b=c1_view(&nb);let saved_a=na.root.clone();let saved_b=nb.root.clone();na.close();nb.close();
  // The two ordinary app actors, credentials, lobby and world records remain
  // alive. Only the injected engine processes Save/Quit and Continue.
  na=Native::start(&a,Some(&saved_a),true);nb=Native::start(&b,Some(&saved_b),true);
  // Retained Rejoin intent uses the accepted bounded owner-turnover path;
  // require its fresh PID/context and publication, rather than a cached view.
  ready_owner(&mut service,&a,&na,&wa,true).await;ready_owner(&mut service,&b,&nb,&wb,true).await;
  let base_a=c1_hp(&na,"max");let base_b=c1_hp(&nb,"max");let base_hp=c1_hp(&nb,"hp");let base_perks=c1_view(&nb)["perks"]["EXTRA_MONEY"].as_u64().unwrap_or(0);
  c1_action(&na,"heart");until(&mut service,"C1 post-Continue local and remote pickup",||c1_hp(&na,"max")==base_a+0.5&&c1_hp(&nb,"max")==base_b+0.5&&c1_hp(&nb,"hp")==base_hp+0.5).await;
  c1_data(&na,json!({"kind":"perk","id":"EXTRA_MONEY"}));until(&mut service,"C1 post-Continue team perk shares",||c1_view(&nb)["perks"]["EXTRA_MONEY"].as_u64()==Some(base_perks+1)).await;
  fixture_signal::write(nb.root.join("unaffected-check"),b"observe replacement peer after settled Continue").unwrap();until(&mut service,"C1 post-Continue peer observation available",||nb.root.join("unaffected-snapshot").is_file()).await;
  fresh_bidirectional_ghosts(&mut service,&a,&na,&b,&nb,"C1 same-app Continue retains ghosts").await;
  let after_a=c1_view(&na);let after_b=c1_view(&nb);na.close();nb.close();a.close().await;b.close().await;service.close();
  std::fs::write(own.join("checks.json"),serde_json::to_vec_pretty(&json!({"passed":true,"actualComponents":true,"ordinaryScheduler":true,"rawGlobalsXml":true,"bothAppActorsRetained":true,"bothWorldIdentitiesRetained":true,"engineFactsInjected":true,"actualNoita":false,"savePersistenceProven":false,"savedHost":saved_a,"savedPeer":saved_b,"beforeHost":before_a,"beforePeer":before_b,"afterHost":after_a,"afterPeer":after_b})).unwrap()).unwrap();
 })).unwrap().join().unwrap();
}
