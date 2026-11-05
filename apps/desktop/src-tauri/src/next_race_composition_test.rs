// Ordinary scheduler, actual Client/TLS Server/SQL/native/local WebSocket.
// Only Noita entity facts and save bytes are injected.
fn race(n:&Native)->Value{fixture_signal::read(n.root.join("race-view.json")).unwrap().and_then(|s|serde_json::from_str(&s).ok()).unwrap_or(Value::Null)}
async fn observe(service:&mut Service,n:&Native,mut request:Value){
 let id=uuid::Uuid::new_v4().to_string();request["id"]=json!(id);fixture_signal::write(n.root.join("race-input"),serde_json::to_vec(&request).unwrap()).unwrap();
 until(service,"Race production callback consumed",||fixture_signal::read(n.root.join("race-last-input.json")).unwrap().and_then(|s|serde_json::from_str::<Value>(&s).ok()).is_some_and(|v|v["id"]==id)).await;
}
fn own(c:&Client)->Value{let v=c.handle.view();let snapshot=&v["g3"]["race"]["snapshot"];snapshot["entrants"].as_array().and_then(|rows|rows.iter().find(|r|r["rowKey"]==snapshot["selfRow"])).cloned().unwrap_or(Value::Null)}
#[test]#[ignore="Client sole isolated Race composition slot"]
fn race_all_goals_composition(){std::thread::Builder::new().name("race-all-goals".into()).stack_size(16*1024*1024).spawn(||tauri::async_runtime::block_on(async{
 verify_current_fixture_inputs();let root=repo().join("spikes/noita-entry/.build/race-first-release-client-20261105").join(format!("composition-{}",uuid::Uuid::new_v4()));std::fs::create_dir_all(&root).unwrap();eprintln!("RACE_COMPOSITION {}",root.display());
 let config=json!({"registryRevision":"race-goals-1","durationMs":0,"goals":[
  {"instanceId":"classic","id":"classic_ending"},{"instanceId":"orbs","id":"collect_orbs","target":2},
  {"instanceId":"bosses","id":"defeat_bosses","target":2},{"instanceId":"named","id":"defeat_named_bosses","bossIds":["high_alchemist","wand_connoisseur"]},
  {"instanceId":"biome","id":"visit_biome","biomeId":"coal_pits"},{"instanceId":"parallel","id":"visit_parallel_world","direction":"east","index":1},
  {"instanceId":"hp","id":"reach_max_hp","target":500},{"instanceId":"gold","id":"accumulate_gold","target":10000}]});
 std::fs::write(root.join("race-config.json"),serde_json::to_vec(&config).unwrap()).unwrap();
 let mut service=Service::start(&root);service.command(json!({"op":"hostApp"}));network_profile::enable_elapsed_clock(&mut service,&root);let router=PipeHarness::new();
 let mut a=Client::start(&service.private(),&root.join("host"),None,false,true,router.routes.clone(),service.watches.clone(),service.stores.clone()).await;bundle(&mut service,&a).await;let mut na=Native::start(&a,None,false);
 wait(&mut service,&a,"all-goal Ready",35,|v|current_state_is(v,"Ready")&&na.root.join("associated").is_file()).await;let wa=a.handle.view()["rejoin"]["current_world"].clone();
 // Staging Continue must retain this competitor before inspected Start.
 let saved=na.root.clone();na.close();na=Native::start(&a,Some(&saved),false);
 ready_owner(&mut service,&a,&na,&wa,false).await;
 service.command(json!({"op":"peer"}));let mut b=Client::start(&service.private(),&root.join("peer"),None,true,true,router.routes.clone(),service.watches.clone(),service.stores.clone()).await;bundle(&mut service,&b).await;let mut nb=Native::start(&b,None,false);
 wait(&mut service,&b,"all-goal peer Ready",35,|v|current_state_is(v,"Ready")&&nb.root.join("associated").is_file()).await;let wb=b.handle.view()["rejoin"]["current_world"].clone();
 two_peer_ghost_with_start(&mut service,&a,&na,&wa,&b,&nb,&wb,"startReviewed").await;
 wait(&mut service,&a,"all-goal racing",30,|v|v["g3"]["race"]["status"]=="racing").await;
 fixture_signal::write(nb.root.join("race-expected-parallel-teleport"),"35840").unwrap();
 for fact in [json!({"kind":"orb","orbId":0}),json!({"kind":"orb","orbId":0}),json!({"kind":"orb","orbId":128}),
  json!({"kind":"boss","bossId":"high_alchemist"}),json!({"kind":"boss","bossId":"high_alchemist"}),json!({"kind":"boss","bossId":"wand_connoisseur"}),
  json!({"kind":"biome","biome":"$biome_excavationsite"}),json!({"kind":"parallel","direction":"east","index":1}),
  json!({"kind":"max_hp","value":500}),json!({"kind":"gold","value":10000})]{observe(&mut service,&na,fact).await;}
 wait(&mut service,&a,"seven goals before Classic",30,|_|own(&a)["completedGoals"]==7).await;
 observe(&mut service,&na,json!({"kind":"max_hp","value":100})).await;observe(&mut service,&na,json!({"kind":"gold","value":0})).await;
 // Cold desktop and same saved world retain accepted progress and restore ghosts.
 let saved=na.root.clone();na.close();let store=a.store.clone();let private=service.private();a.close().await;
 a=Client::start(&private,&root.join("cold"),Some(store),false,false,router.routes.clone(),service.watches.clone(),service.stores.clone()).await;na=Native::start(&a,Some(&saved),true);wait(&mut service,&a,"Race cold explicit saved-world choice",15,|v|v["connection"]["state"]=="Choice").await;fixture_signal::write(na.root.join("choose-rejoin"),"explicit saved-world rejoin").unwrap();ready_owner(&mut service,&a,&na,&wa,true).await;
 fresh_bidirectional_ghosts(&mut service,&a,&na,&b,&nb,"all-goal cold Continue fresh ghosts").await;
 wait(&mut service,&a,"all-goal cold highwaters",30,|_|own(&a)["completedGoals"]==7).await;
 let mut finish_guard=expect_loss(&mut service,&na,"Race individual finish").await;finish_guard.before_fault(&na);
 service.command(json!({"op":"raceDropReply","target":"consumer"}));observe(&mut service,&na,json!({"kind":"ending","classic":true})).await;
 wait(&mut service,&a,"Classic terminal original recovers",30,|v|v["g3"]["race"]["status"]=="finished"&&v["g3"]["race"]["queued"]==0).await;
 until(&mut service,"finished entrant is protected",||native_activity(&na)["native_owned_hold"]==true).await;finish_guard.protected(&na);finish_guard.terminal(&na);
 let finished=own(&a);assert_eq!(finished["completedGoals"],8);assert_eq!(finished["finishOrder"],1);assert_eq!(own(&b)["completedGoals"],0);
 // The finished native remains a read-only spectator while the peer progresses.
 elapsed(&mut service,14).await;
 observe(&mut service,&nb,json!({"kind":"gold","value":10000})).await;
 wait(&mut service,&b,"peer independent gold",25,|_|own(&b)["completedGoals"]==1).await;
 wait(&mut service,&a,"finished viewer receives peer update",25,|v|v["g3"]["race"]["snapshot"]["entrants"].as_array().is_some_and(|rows|rows.iter().any(|r|r["rowKey"]!=finished["rowKey"]&&r["completedGoals"]==1))).await;
 until(&mut service,"terminal native keeps read-only peer count beyond gameplay leases",||race(&na)["view"]["players"].as_array().is_some_and(|rows|rows.iter().any(|r|r["count"]==1))).await;
 assert_eq!(native_activity(&na)["native_owned_hold"],true);
 let mut dnf_guard=expect_loss(&mut service,&nb,"Race individual Other ending").await;dnf_guard.before_fault(&nb);observe(&mut service,&nb,json!({"kind":"ending","classic":false})).await;
 wait(&mut service,&b,"Other ending DNF",30,|v|v["g3"]["race"]["status"]=="dnf"&&v["g3"]["race"]["queued"]==0&&own(&b)["status"]=="dnf").await;
 until(&mut service,"DNF entrant is protected",||native_activity(&nb)["native_owned_hold"]==true).await;dnf_guard.protected(&nb);dnf_guard.terminal(&nb);
 wait(&mut service,&a,"final canonical Race standings",25,|v|v["g3"]["race"]["snapshot"]["status"]=="ended"&&v["g3"]["race"]["snapshot"]["entrants"].as_array().is_some_and(|rows|rows.iter().any(|r|r["status"]=="dnf"))).await;
 let standings=a.handle.view()["g3"]["race"]["snapshot"].clone();let dnf=own(&b);na.close();nb.close();
 let race_room=service.private()["room"]["roomId"].clone();let app_sessions=[a.handle.view()["desktop_id"].clone(),b.handle.view()["desktop_id"].clone()];
 let mut core=service.private()["room"]["config"].clone();core["mode"]=json!("core");core.as_object_mut().unwrap().remove("race");if let Some(caps)=core["requiredCapabilities"].as_array_mut(){caps.retain(|c|c!="race.goals.v1");}
 app_action(&a,json!({"action":"leave_lobby"})).await;app_action(&b,json!({"action":"leave_lobby"})).await;
 wait(&mut service,&a,"Race local scope retired on Leave",15,|v|v["room"].is_null()&&v["g3"]["optional_stream"]["race_copy_owner_live"]==false&&v["g3"]["optional_stream"]["race_copy_ack"]==0).await;
 wait(&mut service,&b,"peer Race local scope retired on Leave",15,|v|v["room"].is_null()&&v["g3"]["optional_stream"]["race_copy_owner_live"]==false&&v["g3"]["optional_stream"]["race_copy_ack"]==0).await;
 service.command(json!({"op":"coreLobbyCreate","config":core}));let room=service.private()["room"]["roomId"].as_str().unwrap().to_owned();assert_ne!(json!(room),race_room);join_current(&a,room.clone()).await;join_current(&b,room.clone()).await;
 assert_eq!([a.handle.view()["desktop_id"].clone(),b.handle.view()["desktop_id"].clone()],app_sessions,"Cross-lobby cleanup must use the same app sessions");
 let ca=bundle(&mut service,&a).await.0;let cb=bundle(&mut service,&b).await.0;na=Native::start(&a,None,false);nb=Native::start(&b,None,false);ready_owner(&mut service,&a,&na,&ca,false).await;ready_owner(&mut service,&b,&nb,&cb,false).await;
 two_peer_ghost_with_start(&mut service,&a,&na,&ca,&b,&nb,&cb,"startReviewed").await;
 c1_rules(&mut service,&na,&nb,json!({"NT_sync_hearts":true,"NT_death_penalty":"full_respawn"})).await;let before=c1_hp(&nb,"max");c1_action(&na,"heart");
 until(&mut service,"Core sharing restored after Race",||c1_hp(&nb,"max")>before).await;
 let mutations=c1_view(&na)["mutations"].as_u64().unwrap_or(0);c1_action(&na,"death");
 until(&mut service,"Core death resumes cooperatively after Race",||c1_hp(&na,"hp")>0.0&&c1_view(&na)["mutations"].as_u64().unwrap_or(0)>mutations).await;
 let results=json!({"passed":true,"allEightGoals":true,"stagingContinue":true,"coldContinueFreshGhosts":true,"coreSharingAndRespawnAfterRace":true,"separateCoreLobby":room,"retainedRaceLobby":race_room,"sameDesktopSessions":true,"finished":finished,"dnf":dnf,"finalStandings":standings,"ordinaryScheduler":true,"engineFactsInjected":true,"actualNoita":false});
 na.close();nb.close();a.close().await;b.close().await;service.close();std::fs::write(root.join("results.json"),serde_json::to_vec_pretty(&results).unwrap()).unwrap();
})).unwrap().join().unwrap();}
#[test]#[ignore="Client sole isolated Race composition slot"]
fn race_ordinary_scheduler_composition(){std::thread::Builder::new().name("race-composition".into()).stack_size(16*1024*1024).spawn(||tauri::async_runtime::block_on(async{
 verify_current_fixture_inputs();let root=repo().join("spikes/noita-entry/.build/race-first-release-client-20261105").join(format!("composition-{}",uuid::Uuid::new_v4()));std::fs::create_dir_all(&root).unwrap();eprintln!("RACE_COMPOSITION {}",root.display());
 let mut service=Service::start(&root);service.command(json!({"op":"hostApp"}));network_profile::enable_elapsed_clock(&mut service,&root);let router=PipeHarness::new();
 let mut host_private=service.private();let credentials=host_private["hostApp"].clone();host_private["desktop"]=credentials.clone();host_private["deviceId"]=credentials["deviceId"].clone();let mut host=Client::start(&host_private,&root.join("host-management"),None,false,false,router.routes.clone(),service.watches.clone(),service.stores.clone()).await;
 let mut a=Client::start(&service.private(),&root.join("host"),None,false,true,router.routes.clone(),service.watches.clone(),service.stores.clone()).await;bundle(&mut service,&a).await;let mut na=Native::start(&a,None,false);
 wait(&mut service,&a,"Race host Ready",35,|v|current_state_is(v,"Ready")&&na.root.join("associated").is_file()).await;let wa=a.handle.view()["rejoin"]["current_world"].clone();
 service.command(json!({"op":"peer"}));let mut b=Client::start(&service.private(),&root.join("peer"),None,true,true,router.routes.clone(),service.watches.clone(),service.stores.clone()).await;bundle(&mut service,&b).await;let mut nb=Native::start(&b,None,false);
 wait(&mut service,&b,"Race peer Ready",35,|v|current_state_is(v,"Ready")&&nb.root.join("associated").is_file()).await;let wb=b.handle.view()["rejoin"]["current_world"].clone();
 two_peer_ghost_with_start(&mut service,&a,&na,&wa,&b,&nb,&wb,"startReviewed").await;
 wait(&mut service,&a,"Race canonical own entry",30,|v|v["g3"]["race"]["status"]=="racing").await;
 wait(&mut service,&b,"Race canonical peer entry",30,|v|v["g3"]["race"]["status"]=="racing").await;
 observe(&mut service,&na,json!({"kind":"max_hp","value":200})).await;
 wait(&mut service,&a,"Race HP highwater",20,|_|own(&a)["progress"][0]["value"]=="200").await;
 assert_eq!(own(&b)["progress"][0]["value"],"100","Race HP must not propagate cooperatively");
 let saved=na.root.clone();na.close();na=Native::start(&a,Some(&saved),true);ready_owner(&mut service,&a,&na,&wa,true).await;
 fresh_bidirectional_ghosts(&mut service,&a,&na,&b,&nb,"Race Continue restores fresh ghosts").await;
 assert_eq!(own(&a)["progress"][0]["value"],"200","Continue erased accepted highwater");
 service.command(json!({"op":"raceDropReply","target":"consumer"}));observe(&mut service,&na,json!({"kind":"finish_death","value":500})).await;
 fixture_signal::write(na.root.join("c1-missing-actor"),"injected actor loss after terminal capture").unwrap();
 wait(&mut service,&a,"Race terminal original is durable before cold app",25,|v|v["g3"]["race"]["queued"].as_u64().is_some_and(|n|n>0)).await;
 na.close();let store=a.store.clone();a.close().await;
 {let journal=nt2_desktop_g0::c1_journal::Journal::open(&root.join("host/race-observations.jsonl")).unwrap();assert!(!journal.all_rows().is_empty(),"Cold terminal test must restore an actual pending original");}
 a=Client::start(&service.private(),&root.join("cold-terminal"),Some(store),false,false,router.routes.clone(),service.watches.clone(),service.stores.clone()).await;
 wait(&mut service,&a,"Race original receipt after terminal actor loss",30,|v|v["g3"]["race"]["status"]=="finished"&&v["g3"]["race"]["queued"]==0).await;
 let finished=own(&a);assert_eq!(finished["finishOrder"],1);assert_eq!(finished["completedGoals"],1);assert_eq!(race(&na)["gameOvers"],1);assert_eq!(race(&na)["wait"],false);
 assert_eq!(own(&b)["status"],"racing");assert_eq!(own(&b)["completedGoals"],0);
 service.command(json!({"op":"raceDropReply","target":"peer"}));observe(&mut service,&nb,json!({"kind":"death"})).await;
 wait(&mut service,&b,"Race incomplete death seals entry",30,|v|v["g3"]["race"]["status"]=="dead"&&v["g3"]["race"]["queued"]==0).await;
 let faults=service.command(json!({"op":"raceStatus"}));assert_eq!(faults["dropped"],2);assert_eq!(own(&a)["finishOrder"],1);
 let results=json!({"passed":true,"finished":finished,"dead":own(&b),"faults":faults,"ordinaryScheduler":true,"actualClientServerNative":true,"engineFactsInjected":true,"actualNoita":false});
 na.close();nb.close();a.close().await;b.close().await;host.close().await;service.close();std::fs::write(root.join("results.json"),serde_json::to_vec_pretty(&results).unwrap()).unwrap();
})).unwrap().join().unwrap();}
