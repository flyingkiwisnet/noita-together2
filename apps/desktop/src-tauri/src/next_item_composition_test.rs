include!("next_item_observation_test.rs");
// Same ordinary scheduler/native watchers and scoped TLS/SQL fixture as B1.
// Only Noita entities/components/material writes and physical UI are injected.
fn items(n:&Native)->Value{fixture_signal::read(n.root.join("items-view.json")).unwrap().and_then(|s|serde_json::from_str(&s).ok()).unwrap_or(Value::Null)}
fn available(n:&Native)->Vec<Value>{items(n)["bank"]["items"].as_array().cloned().unwrap_or_default().into_iter().filter(|r|r["state"]=="available").collect()}
fn received(n:&Native)->usize{items(n)["received"].as_array().map_or(0,Vec::len)}
fn item_deposit_input(n:&Native)->Value{fixture_signal::read(n.root.join("items-last-deposit.json")).unwrap().and_then(|s|serde_json::from_str(&s).ok()).unwrap_or(Value::Null)}
async fn item_spawn(service:&mut Service,n:&Native,item:&Value,block:bool)->String{
 if block{fixture_signal::write(n.root.join("items-copy-block"),serde_json::to_vec(&json!({"kind":"item_chunk","channel":"deposit","id":"next"})).unwrap()).unwrap();}
 let old=item_deposit_input(n)["id"].clone();fixture_signal::write(n.root.join("items-spawn"),serde_json::to_vec(item).unwrap()).unwrap();
 until(service,"immutable item capture copied by production bank",||item_deposit_input(n)["kind"]=="item_chunk"&&item_deposit_input(n)["channel"]=="deposit"&&item_deposit_input(n)["id"].is_string()&&item_deposit_input(n)["id"]!=old).await;let id=item_deposit_input(n)["id"].as_str().unwrap().to_owned();if block{fixture_signal::write(n.root.join("items-copy-block"),serde_json::to_vec(&json!({"kind":"item_chunk","channel":"deposit","id":id})).unwrap()).unwrap();}id
}
fn item_cell_click(n:&Native)->bool{
 let snapshot=items(n);let Some(rows)=snapshot["bank"]["items"].as_array()else{return false};let Some(index)=rows.iter().position(|r|r["state"]=="available"&&r["claimAllowed"]==true)else{return false};assert!(index<40);let row=&rows[index];let label="";
 // Pixel gesture against the actual full-slot GuiImageButton; the fixture never calls claim.
 fixture_signal::write(n.root.join("items-cell-click"),serde_json::to_vec(&json!({"x":210+28*(index%8),"y":160+25*(index/8),"label":label,"itemId":row["itemId"]})).unwrap()).unwrap();true
}
async fn item_click(service:&mut Service,n:&Native)->String{
 until(service,"confirmed available item bank",||!available(n).is_empty()&&items(n)["view"]["fresh"]==true).await;
 if bank_view(n)["open"]!=true{click(service,n,"Gold bank").await;}
 until(service,"item panel open",||bank_view(n)["open"]==true).await;
 until(service,"item query complete before cell click",||items(n)["loading"]==false&&!available(n).is_empty()).await;
 let claim=||fixture_signal::read(n.root.join("items-last-claim.json")).unwrap().and_then(|v|serde_json::from_str::<Value>(&v).ok()).unwrap_or(Value::Null);let before=claim()["id"].clone();
 let deadline=Instant::now()+Duration::from_secs(25);let mut last_gesture=None::<Instant>;
 loop{service.wall_clock();let observed=claim();if observed["id"].is_string()&&observed["id"]!=before{let id=observed["id"].as_str().unwrap().to_owned();click(service,n,"Close").await;return id}
  assert!(Instant::now()<deadline,"New immutable claim copied by production mailbox25s deadline");let snapshot=items(n);
  // Retry only a physical click that invoked NO new claim ID. Once any ID is
  // observed above, all subsequent recovery follows that immutable operation.
  if !n.root.join("items-cell-click").exists()&&snapshot["view"]["enabled"]==true&&snapshot["view"]["fresh"]==true&&snapshot["loading"]==false&&last_gesture.is_none_or(|at|at.elapsed()>=Duration::from_millis(200)){
   if item_cell_click(n){last_gesture=Some(Instant::now());let mut log=std::fs::OpenOptions::new().create(true).append(true).open(n.root.join("item-claim-gestures.jsonl")).unwrap();writeln!(log,"{}",json!({"fresh":snapshot["view"]["fresh"],"loading":snapshot["loading"],"remainingMs":deadline.saturating_duration_since(Instant::now()).as_millis(),"previousClaim":before,"noNewClaimIdObserved":true})).unwrap();}
  }
  tokio::time::sleep(Duration::from_millis(50)).await;
 }
}
fn item_delay_active(status:&Value,id:&str)->bool{status["inFlightDelayedOp"]["operationId"]==id&&status["inFlightDelayedOp"]["delayMs"]==2500&&status["inFlightDelayedOp"]["kind"]=="itemDeposit"&&status["inFlightDelayedOp"]["target"]=="consumer"}
async fn item_delay_started(service:&mut Service,id:&str)->Value{
 let until=Instant::now()+Duration::from_secs(30);loop{service.wall_clock();let status=service.command(json!({"op":"goldStatus"}));if item_delay_active(&status,id){return status}assert!(Instant::now()<until,"Exact delayed item request did not begin within existing transfer deadline");tokio::time::sleep(Duration::from_millis(50)).await;}
}
fn item_movement_observer(a:&Client,na:&Native,b:&Client,nb:&Native,started:Instant)->([Value;2],[Value;2],u64,u64){
 tauri::async_runtime::block_on(async{
  let label="item work keeps movement and auth renewal live";let until=started+Duration::from_millis(2500);
  let before=item_observation_pair(a,na,b,nb,started,until,"delayed item baseline").await;
  for(i,client)in[a,b].into_iter().enumerate(){assert!(current_state_is(&before[i],"Playing")&&before[i]["fixture_current"]["native_controls"]==true,"Concurrent movement baseline lacks current Playing/Controls: {}",before[i]["fixture_current"]);write_current(client,"delayed-item observer baseline",&before[i]);}
  let baseline_ms=started.elapsed().as_millis() as u64;let marked=std::time::SystemTime::now();for native in[na,nb]{fixture_input_write(native.root.join("move-source"),b"continued synthetic engine movement").unwrap();fixture_input_write(native.root.join("foreground-monitor"),b"scoped native evidence").unwrap();}
  loop{
   check_natives(&a.watches,label);let after=item_observation_pair(a,na,b,nb,started,until,"delayed item fresh movement").await;
   if fresh_pair_ready(&before,&after,[na.root.as_path(),nb.root.as_path()],marked){for(i,client)in[a,b].into_iter().enumerate(){write_current(client,label,&after[i]);}return(before,after,baseline_ms,started.elapsed().as_millis() as u64)}
   assert!(Instant::now()<until,"Concurrent movement did not satisfy the unchanged fresh-owner predicates within the2500ms observation bound");tokio::time::sleep(Duration::from_millis(250)).await;
  }
 })
}
fn item_overlap_probe(service:&mut Service,a:&Client,na:&Native,b:&Client,nb:&Native,id:&str,delay_before:Value)->Value{
 let started=Instant::now();
 // A scoped OS thread drives the read-only observer independently of the
 // blocking fixture RPC. Scope joins it on both success and panic unwinding.
 std::thread::scope(|scope|{
  let movement=scope.spawn(||item_movement_observer(a,na,b,nb,started));
  let chat=service.command(json!({"op":"chat","target":"host","text":"Slow item work; social still live"}));let chat_ms=started.elapsed().as_millis() as u64;let notice_until=Instant::now()+Duration::from_secs(25);
  let(before,after,baseline_ms,accepted_ms)=movement.join().unwrap();assert_peer_healthy(b,nb);check_natives(&a.watches,"delayed item overlap bookend");
  let mut notice_ms=text(nb,"a2-notices.txt").contains("Slow item work; social still live").then(||started.elapsed().as_millis() as u64);
  let delay_after=service.command(json!({"op":"goldStatus"}));let overlap_ms=started.elapsed().as_millis() as u64;let overlap=item_delay_active(&delay_before,id)&&item_delay_active(&delay_after,id);
  // Display follows the production2s room/chat poll and optional native lane.
  // Its original25s bound starts when chat returns, never after the join.
  let display_status=if notice_ms.is_some(){delay_after.clone()}else{
   loop{service.wall_clock();if text(nb,"a2-notices.txt").contains("Slow item work; social still live"){notice_ms=Some(started.elapsed().as_millis() as u64);break}assert!(Instant::now()<notice_until,"Social notice exceeded existing25s deadline");std::thread::sleep(Duration::from_millis(50));}
   service.command(json!({"op":"goldStatus"}))
  };
  let notices=text(nb,"a2-notices.txt").matches("Slow item work; social still live").count();assert_eq!(notices,1,"Optional chat must be delivered once");check_natives(&a.watches,"delayed item native display");write_fresh_ghost_evidence(a.root.parent().unwrap(),"item work keeps movement and auth renewal live",&after);
  let samples=|views:&[Value;2]|json!(views.iter().map(|v|json!({"owner":v["fixture_current"],"movement":v["g3"]["movement"],"liveTransport":v["g3"]["live_transport"],"peers":v["g3"]["fixture_peers"]})).collect::<Vec<_>>());
  json!({"operationId":id,"delayMs":2500,"before":delay_before["inFlightDelayedOp"],"after":delay_after["inFlightDelayedOp"],"displayActiveCheck":display_status["inFlightDelayedOp"],"chat":chat["chat"],"chatDiagnostic":chat["chatDiagnostic"],"nativeNotices":notices,"nativeNoticeBoundSeconds":25,"observationTimingFiles":[a.root.join("item-observation-timing.jsonl"),b.root.join("item-observation-timing.jsonl")],"movement":{"before":samples(&before),"after":samples(&after)},"timingMs":{"baseline":baseline_ms,"movementAccepted":accepted_ms,"chatReturned":chat_ms,"nativeNoticeObserved":notice_ms,"overlapActiveCheck":overlap_ms},"acceptedDuringDelay":overlap,"movementDuringDelay":overlap,"displayedDuringDelay":item_delay_active(&display_status,id),"engineFactsInjected":true})
 })
}
#[test]fn item_overlap_shared_predicate_keeps_every_owner_freshness_and_render_fence(){
 // Fabricated snapshots test only the extracted predicate, not game evidence.
 let own=repo().join("spikes/noita-entry/.build/item-client-20261025-01").join(format!("overlap-predicate-{}",uuid::Uuid::new_v4()));std::fs::create_dir(&own).unwrap();let roots=[own.join("a"),own.join("b")];for root in &roots{std::fs::create_dir(root).unwrap();std::fs::write(root.join("ghost-motion"),b"synthetic predicate sample").unwrap();}
 let identities=[json!({"session":"a","world":"world-a","context":"context-a"}),json!({"session":"b","world":"world-b","context":"context-b"})];let sample=|i:usize,n:u64|json!({"connection":{"state":"Playing"},"fixture_current":{"playing":true,"native_controls":true,"identity":identities[i]},"g3":{"movement":{"publications":n},"live_transport":{"nonempty_snapshots":n},"fixture_peers":[{"session":identities[1-i]["session"],"world":identities[1-i]["world"],"sequence":n,"age_ms":20}]}});
 let before=[sample(0,5),sample(1,5)];let after=[sample(0,6),sample(1,6)];let paths=[roots[0].as_path(),roots[1].as_path()];let marked=std::time::SystemTime::now()-Duration::from_secs(1);assert!(fresh_pair_ready(&before,&after,paths,marked));
 for side in 0..2{for fault in 0..11{let mut bad=after.clone();match fault{0=>bad[side]["connection"]["state"]=json!("Reconnecting"),1=>bad[side]["fixture_current"]["playing"]=json!(false),2=>bad[side]["fixture_current"]["native_controls"]=json!(false),3=>bad[side]["fixture_current"]["identity"]["context"]=json!("retired"),4=>bad[side]["g3"]["movement"]["publications"]=json!(5),5=>bad[side]["g3"]["live_transport"]["nonempty_snapshots"]=json!(5),6=>bad[side]["g3"]["fixture_peers"][0]["sequence"]=json!(5),7=>bad[side]["g3"]["fixture_peers"][0]["world"]=json!("other"),8=>bad[side]["g3"]["fixture_peers"][0]["session"]=json!("other"),9=>bad[side]["g3"]["fixture_peers"][0]["age_ms"]=json!(500),_=>bad[side]["g3"]["fixture_peers"]=json!([])}assert!(!fresh_pair_ready(&before,&bad,paths,marked),"Fresh movement fence lost: side{side} fault{fault}");}}
 assert!(!fresh_pair_ready(&before,&after,paths,std::time::SystemTime::now()+Duration::from_secs(1)));std::fs::remove_file(roots[0].join("ghost-motion")).unwrap();assert!(!fresh_pair_ready(&before,&after,paths,marked));std::fs::remove_file(roots[1].join("ghost-motion")).unwrap();for root in roots{std::fs::remove_dir(root).unwrap();}std::fs::remove_dir(own).unwrap();
}
// End is one operation and one25s observation budget, including its RPC.
// App terminal intent and native file publication complete asynchronously.
fn item_end_app_terminal(v:&Value)->bool{
 v["connection"]["state"]=="Ended"&&v["fixture_current"]["stage"]=="Ended"&&v["fixture_current"]["pending_release"]==false&&v["fixture_current"]["pending_authority_live"]==false&&v["fixture_current"]["pending_collection_live"]==false&&v["g3"]["live_transport"]["grant_active"]==false
}
fn item_end_native_terminal(v:&Value,before:&Value,world:&Value)->bool{
 let n=&v["fixture_current"]["native"];let context=&before["native_context"];
 // Reuse the established exact scoped lookup3/intent/freshness predicate.
 // Cached gameplay status is diagnostic only after production enters terminal.
 final_tail_terminal(v,world,context)&&n["controls_enabled"]==false&&n["offline_marker"]==false&&n["loss_monitor_armed"]==false&&n["holds"].as_u64().zip(before["holds"].as_u64()).is_some_and(|(now,old)|now==old+1)&&n["frame"].as_u64().zip(before["frame"].as_u64()).is_some_and(|(now,old)|now>old)
}
async fn item_end_pair(service:&mut Service,sides:[(&Client,&Native,&Value,&ExpectedLoss);2]){
 let started=Instant::now();let deadline=started+Duration::from_secs(25);let mut app_at=[None::<u64>;2];let mut native_at=[None::<u64>;2];let mut snapshots=[Value::Null,Value::Null];
 let evidence=|app_at:[Option<u64>;2],native_at:[Option<u64>;2],snapshots:&[Value;2],accepted:bool|json!({"accepted":accepted,"deadlineSeconds":25,"singleDeadlineFromEndInvocation":true,"elapsedMs":started.elapsed().as_millis(),"engineFactsInjected":true,"sides":sides.iter().enumerate().map(|(i,(_,n,w,loss))|json!({"pid":n.child.id(),"nativeRoot":n.root,"world":w,"context":loss.before["native_context"],"before":loss.before,"firstAppTerminalMs":app_at[i],"firstNativeTerminalProtectedMs":native_at[i],"current":snapshots[i]})).collect::<Vec<_>>()});
 let path=service.root.join("item-end-observation.json");
 std::fs::write(&path,serde_json::to_vec_pretty(&evidence(app_at,native_at,&snapshots,false)).unwrap()).unwrap();
 service.command(json!({"op":"end"}));
 loop{
  service.wall_clock();let mut accepted=true;
  for(i,(client,native,world,loss))in sides.iter().enumerate(){
   let remaining=deadline.saturating_duration_since(Instant::now());
   let result=tokio::time::timeout(remaining.min(Duration::from_secs(5)),final_tail_snapshot(client,native)).await;
   if let Ok(v)=result{snapshots[i]=v;}else{std::fs::write(&path,serde_json::to_vec_pretty(&evidence(app_at,native_at,&snapshots,false)).unwrap()).unwrap();panic!("End current observation exceeded original deadline; {}",path.display());}
   if item_end_app_terminal(&snapshots[i])&&app_at[i].is_none(){app_at[i]=Some(started.elapsed().as_millis() as u64);}
   let protected=item_end_native_terminal(&snapshots[i],&loss.before,world);
   if protected&&native_at[i].is_none(){native_at[i]=Some(started.elapsed().as_millis() as u64);}
   accepted&=item_end_app_terminal(&snapshots[i])&&protected;
  }
  accepted&=Instant::now()<=deadline;
  std::fs::write(&path,serde_json::to_vec_pretty(&evidence(app_at,native_at,&snapshots,accepted)).unwrap()).unwrap();
  if accepted{return}
  assert!(Instant::now()<deadline,"End app/native terminal evidence exceeded original25s deadline; {}",path.display());
  tokio::time::sleep(Duration::from_millis(50)).await;
 }
}
#[test]fn item_end_observation_requires_fresh_terminal_source_and_one_owned_hold(){
 let before=json!({"native_context":"same","holds":2,"frame":99});let world=json!("world");
 let good=json!({"connection":{"state":"Ended"},"fixture_current":{"stage":"Ended","pending_release":false,"pending_authority_live":false,"pending_collection_live":false,"native":{"frame":100,"native_lookup":3,"native_lookup_frame":100,"native_lookup_world":"world","native_lookup_context":"same","native_context":"same","protected":true,"native_owned_hold":true,"native_hold_marker":true,"native_hold_effect":true,"controls_enabled":false,"offline_marker":false,"loss_monitor_armed":false,"holds":3,"native_status":2,"native_status_frame":99}},"lc1_lifecycle":{"record_world":"world","record_intent":"Terminal"},"g3":{"live_transport":{"grant_active":false}},"fixture_terminal_native_fresh":true});
 assert!(item_end_app_terminal(&good)&&item_end_native_terminal(&good,&before,&world));
 for fault in 0..13{let mut bad=good.clone();match fault{
  0=>bad["fixture_current"]["native"]["holds"]=json!(2),1=>bad["fixture_current"]["native"]["holds"]=json!(4),2=>bad["fixture_current"]["native"]["frame"]=json!(99),3=>bad["fixture_current"]["native"]["offline_marker"]=json!(true),4=>bad["fixture_current"]["native"]["controls_enabled"]=json!(true),5=>bad["fixture_current"]["native"]["native_lookup"]=json!(4),6=>bad["fixture_terminal_native_fresh"]=json!(false),7=>bad["fixture_current"]["pending_authority_live"]=json!(true),8=>bad["fixture_current"]["pending_collection_live"]=json!(true),9=>bad["fixture_current"]["native"]["native_lookup_world"]=json!("other"),10=>bad["fixture_current"]["native"]["native_lookup_context"]=json!("other"),11=>bad["fixture_current"]["native"]["native_owned_hold"]=json!(false),_=>bad["fixture_current"]["native"]["native_lookup_frame"]=json!(98)};
  assert!(!(item_end_app_terminal(&bad)&&item_end_native_terminal(&bad,&before,&world)),"Accepted incomplete End evidence {fault}");
 }
}
#[test]#[ignore="Root serialized exact delayed item/current actor observation; actual components, synthetic engine facts"]
fn item_delayed_observation_composition(){
 assert_eq!(std::env::var("NT2_C3_COMPOSITION").as_deref(),Ok("1"));
 std::thread::Builder::new().name("item-overlap".into()).stack_size(16*1024*1024).spawn(||tauri::async_runtime::block_on(async{
  verify_current_fixture_inputs();let own=repo().join("spikes/noita-entry/.build/item-client-20261025-01").join(format!("overlap-{}",uuid::Uuid::new_v4()));std::fs::create_dir_all(&own).unwrap();eprintln!("ITEM_OVERLAP_COMPOSITION {}",own.display());
  let mut service=Service::start(&own);network_profile::enable_elapsed_clock(&mut service,&own);let router=PipeHarness::new();
  let mut a=Client::start(&service.private(),&own.join("host"),None,false,true,router.routes.clone(),service.watches.clone(),service.stores.clone()).await;bundle(&mut service,&a).await;let mut na=Native::start(&a,None,false);
  wait(&mut service,&a,"End host Ready",30,|v|current_state_is(v,"Ready")&&na.root.join("associated").is_file()).await;let wa=a.handle.view()["rejoin"]["current_world"].clone();
  service.command(json!({"op":"peer"}));let mut b=Client::start(&service.private(),&own.join("peer"),None,true,true,router.routes.clone(),service.watches.clone(),service.stores.clone()).await;bundle(&mut service,&b).await;let mut nb=Native::start(&b,None,false);
  wait(&mut service,&b,"End peer Ready",30,|v|current_state_is(v,"Ready")&&nb.root.join("associated").is_file()).await;let wb=b.handle.view()["rejoin"]["current_world"].clone();two_peer_ghost_with_start(&mut service,&a,&na,&wa,&b,&nb,&wb,"startReviewed").await;
  let canonical:Value=serde_json::from_slice(include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../packages/contracts/g3-items/operation-digest.json"))).unwrap();let wand=canonical["vectors"][0]["original"]["payload"].clone();let id=item_spawn(&mut service,&na,&wand,true).await;
  service.command(json!({"op":"goldArm","plan":{"target":"consumer","operationId":id,"kind":"itemDeposit","effect":"delay","delayMs":2500}}));fixture_signal::remove(na.root.join("items-copy-block")).unwrap();
  let before=item_delay_started(&mut service,&id).await;let evidence=item_overlap_probe(&mut service,&a,&na,&b,&nb,&id,before);std::fs::write(own.join("slow-item-overlap.json"),serde_json::to_vec_pretty(&evidence).unwrap()).unwrap();assert!(evidence["acceptedDuringDelay"]==true&&evidence["movementDuringDelay"]==true);
  let capacity=service.command(json!({"op":"liveCapacityStatus"}));std::fs::write(own.join("service-final-diagnostics.json"),serde_json::to_vec_pretty(&json!({"capacity":capacity,"boundary":"focused delayed observation only"})).unwrap()).unwrap();
  na.close();nb.close();a.close().await;b.close().await;service.close();std::fs::write(own.join("checks.json"),serde_json::to_vec_pretty(&json!({"passed":true,"actualComponents":true,"engineFactsInjected":true,"delayMs":2500,"loadedSaveProof":false})).unwrap()).unwrap();
 })).unwrap().join().unwrap();
}
#[test]#[ignore="Root serialized focused End app/native observation; actual components, synthetic engine facts"]
fn item_end_terminal_native_composition(){
 assert_eq!(std::env::var("NT2_C3_COMPOSITION").as_deref(),Ok("1"));
 std::thread::Builder::new().name("item-end".into()).stack_size(16*1024*1024).spawn(||tauri::async_runtime::block_on(async{
  verify_current_fixture_inputs();let own=repo().join("spikes/noita-entry/.build/item-client-20261025-01").join(format!("terminal-{}",uuid::Uuid::new_v4()));std::fs::create_dir_all(&own).unwrap();eprintln!("ITEM_END_COMPOSITION {}",own.display());
  let mut service=Service::start(&own);network_profile::enable_elapsed_clock(&mut service,&own);let router=PipeHarness::new();
  let mut a=Client::start(&service.private(),&own.join("host"),None,false,true,router.routes.clone(),service.watches.clone(),service.stores.clone()).await;bundle(&mut service,&a).await;let mut na=Native::start(&a,None,false);
  wait(&mut service,&a,"End host Ready",30,|v|current_state_is(v,"Ready")&&na.root.join("associated").is_file()).await;let wa=a.handle.view()["rejoin"]["current_world"].clone();
  service.command(json!({"op":"peer"}));let mut b=Client::start(&service.private(),&own.join("peer"),None,true,true,router.routes.clone(),service.watches.clone(),service.stores.clone()).await;bundle(&mut service,&b).await;let mut nb=Native::start(&b,None,false);
  wait(&mut service,&b,"End peer Ready",30,|v|current_state_is(v,"Ready")&&nb.root.join("associated").is_file()).await;let wb=b.handle.view()["rejoin"]["current_world"].clone();two_peer_ghost_with_start(&mut service,&a,&na,&wa,&b,&nb,&wb,"startReviewed").await;
  let mut host=expect_loss(&mut service,&na,"focused End host").await;let mut peer=expect_loss(&mut service,&nb,"focused End peer").await;host.before_fault(&na);peer.before_fault(&nb);
  item_end_pair(&mut service,[(&a,&na,&wa,&host),(&b,&nb,&wb,&peer)]).await;host.protected(&na);peer.protected(&nb);host.terminal(&na);peer.terminal(&nb);
  na.close();nb.close();a.close().await;b.close().await;service.close();std::fs::write(own.join("checks.json"),serde_json::to_vec_pretty(&json!({"passed":true,"actualComponents":true,"engineFactsInjected":true,"singleEndDeadlineSeconds":25,"loadedSaveProof":false})).unwrap()).unwrap();
 })).unwrap().join().unwrap();
}
#[test]#[ignore="Root serialized item composition: actual scheduler/TLS/SQL/native/mod with injected Noita engine facts"]
fn item_transfers_ordinary_scheduler_composition(){
 assert_eq!(std::env::var("NT2_C3_COMPOSITION").as_deref(),Ok("1"));assert_eq!(std::env::var("NT2_ITEM_FIXTURE").as_deref(),Ok("1"));
 std::thread::Builder::new().name("item-bank".into()).stack_size(16*1024*1024).spawn(||tauri::async_runtime::block_on(async{
  verify_current_fixture_inputs();let own=repo().join("spikes/noita-entry/.build/item-client-20261025-01").join(format!("composition-{}",uuid::Uuid::new_v4()));std::fs::create_dir_all(&own).unwrap();eprintln!("ITEM_COMPOSITION {}",own.display());
  let mut service=Service::start(&own);network_profile::enable_elapsed_clock(&mut service,&own);let router=PipeHarness::new();let room_a=service.private()["room"]["roomId"].as_str().unwrap().to_owned();
  let mut a=Client::start(&service.private(),&own.join("host"),None,false,true,router.routes.clone(),service.watches.clone(),service.stores.clone()).await;bundle(&mut service,&a).await;let mut na=Native::start(&a,None,false);
  wait(&mut service,&a,"item host Ready",30,|v|current_state_is(v,"Ready")&&na.root.join("associated").is_file()).await;let wa=a.handle.view()["rejoin"]["current_world"].clone();
  service.command(json!({"op":"peer"}));let mut b=Client::start(&service.private(),&own.join("peer"),None,true,true,router.routes.clone(),service.watches.clone(),service.stores.clone()).await;bundle(&mut service,&b).await;let mut nb=Native::start(&b,None,false);
  wait(&mut service,&b,"item peer Ready",30,|v|current_state_is(v,"Ready")&&nb.root.join("associated").is_file()).await;let wb=b.handle.view()["rejoin"]["current_world"].clone();two_peer_ghost_with_start(&mut service,&a,&na,&wa,&b,&nb,&wb,"startReviewed").await;
  until(&mut service,"item and gold views enabled together",||items(&na)["view"]["enabled"]==true&&items(&nb)["view"]["fresh"]==true&&bank_view(&na)["view"]["fresh"]==true).await;
  let canonical:Value=serde_json::from_slice(include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../packages/contracts/g3-items/operation-digest.json"))).unwrap();let wand=canonical["vectors"][0]["original"]["payload"].clone();
  let samples=vec![json!({"catalogueRevision":"nt2-items-base-2","spell":{"actionId":"LIGHT_BULLET","usesRemaining":"-1"}}),wand,json!({"catalogueRevision":"nt2-items-base-2","container":{"containerType":"potion","color":"4294967295","contents":[{"materialId":"oil","amount":"200"},{"materialId":"water","amount":"450.25"}]}}),json!({"catalogueRevision":"nt2-items-base-2","container":{"containerType":"powder","color":"4294967295","contents":[]}}),json!({"catalogueRevision":"nt2-items-base-2","object":{"objectId":"pickup_runestones_runestone_disc"}})];
  let mut originals=Vec::new();let mut expected=0;
  for (index,item) in samples.iter().enumerate(){
   let id=item_spawn(&mut service,&na,item,true).await;originals.push(id.clone());
   if index==0{service.command(json!({"op":"goldArm","plan":{"target":"consumer","operationId":id,"kind":"itemDeposit"}}));}
   if index==1{service.command(json!({"op":"goldArm","plan":{"target":"consumer","operationId":id,"kind":"itemDeposit","effect":"delay","delayMs":2500}}));}
   fixture_signal::remove(na.root.join("items-copy-block")).unwrap();
   if index==1{
    let before=item_delay_started(&mut service,&id).await;let evidence=item_overlap_probe(&mut service,&a,&na,&b,&nb,&id,before);
    std::fs::write(own.join("slow-item-overlap.json"),serde_json::to_vec_pretty(&evidence).unwrap()).unwrap();assert!(evidence["acceptedDuringDelay"]==true&&evidence["movementDuringDelay"]==true,"Server chat acceptance and concurrent fresh movement were not observed during the exact2500ms item reply delay");
   }
   until(&mut service,"escrow receipt publishes item after original removal",||items(&na)["aliveOriginals"].as_array().is_some_and(Vec::is_empty)&&available(&nb).len()==1).await;
   let before=received(&nb);item_click(&mut service,&nb).await;
   until(&mut service,"exclusive item claim queued away from outlet",||items(&nb)["view"]["queued"]==1).await;assert_eq!(received(&nb),before);elapsed(&mut service,1).await;assert_eq!(received(&nb),before);
   fixture_signal::write(nb.root.join("b1-outlet"),"injected normal player at outlet").unwrap();expected+=1;until(&mut service,"matching guarded outlet reconstruction",||received(&nb)==expected).await;
   let actual=items(&nb)["received"].as_array().unwrap().iter().find(|v|*v==item).cloned();assert!(actual.is_some(),"Delivered descriptor differs for sample{index}");elapsed(&mut service,1).await;assert_eq!(received(&nb),expected,"No duplicate from lost/native copies");fixture_signal::remove(nb.root.join("b1-outlet")).unwrap();checkpoint(&own,&format!("item-kind-{index}-source-flushed-removed-before-available-queued-outlet-fidelity"));
  }
  // Actual gold still composes with item pages and independent optional work.
  let gold=bank_submit(&mut service,&na,"deposit",10,false).await;accepted(&mut service,&na,&gold).await;until(&mut service,"gold preserved beside item bank",||bank_view(&na)["wallet"]==990&&bank_view(&nb)["view"]["balance"]==10).await;
  // Two physical clicks on the same immutable row/revision have one winner.
  item_spawn(&mut service,&na,&samples[4],false).await;until(&mut service,"one shared item for concurrent claim",||available(&na).len()==1&&available(&nb).len()==1).await;
  for n in [&na,&nb]{click(&mut service,n,"Gold bank").await;until(&mut service,"concurrent panels ready",||bank_view(n)["open"]==true&&items(n)["loading"]==false).await;}
  item_cell_click(&na);item_cell_click(&nb);until(&mut service,"both concurrent item pixel gestures consumed",||!na.root.join("items-cell-click").exists()&&!nb.root.join("items-cell-click").exists()).await;
  until(&mut service,"exactly one concurrent item claim",||items(&na)["view"]["queued"].as_u64().unwrap_or(0)+items(&nb)["view"]["queued"].as_u64().unwrap_or(0)==1).await;
  for n in [&na,&nb]{click(&mut service,n,"Close").await;fixture_signal::write(n.root.join("b1-outlet"),"injected concurrent outlet").unwrap();}
  until(&mut service,"one constructor for concurrent claim",||received(&na)+received(&nb)==expected+1).await;elapsed(&mut service,2).await;assert_eq!(received(&na)+received(&nb),expected+1);expected=received(&nb);for n in [&na,&nb]{fixture_signal::remove(n.root.join("b1-outlet")).unwrap();}checkpoint(&own,"simultaneous-claim-one-winner-and-once-only-construction");
  // A constructor failure retains one uncertain original and never retries.
  let failed=item_spawn(&mut service,&na,&samples[4],false).await;until(&mut service,"failure sample available",||available(&nb).len()==1).await;let failed_claim=item_click(&mut service,&nb).await;fixture_signal::write(nb.root.join("items-spawn-fail"),"injected constructor refusal").unwrap();fixture_signal::write(nb.root.join("b1-outlet"),"injected outlet").unwrap();
  until(&mut service,"partial item failure acknowledged uncertain without repeated spawn",||{let v=items(&nb);v["view"]["status"].as_str().is_some_and(|s|s.contains("uncertain"))&&v["view"]["diagnostics"]["receipt_success"]["operation_id"]==failed_claim&&v["view"]["diagnostics"]["receipt_success"]["state"]=="uncertain"}).await;let created=items(&nb)["created"].clone();elapsed(&mut service,2).await;assert_eq!(items(&nb)["created"],created);assert_eq!(received(&nb),expected);fixture_signal::remove(nb.root.join("items-spawn-fail")).unwrap();fixture_signal::remove(nb.root.join("b1-outlet")).unwrap();
  // A known pre-effect claim survives genuine outage and app/journal reopen.
  item_spawn(&mut service,&na,&samples[4],false).await;until(&mut service,"cold pre-effect sample available",||available(&na).len()==1).await;item_click(&mut service,&na).await;until(&mut service,"host pre-effect queue",||items(&na)["view"]["queued"]==1).await;
  let mut outage=expect_loss(&mut service,&na,"item retained HTTP/WSS outage").await;outage.before_fault(&na);for kind in ["http","wss"]{service.command(json!({"op":"transport","kind":kind,"enabled":true,"target":"consumer"}));}until(&mut service,"item outage protects owner",||native_activity(&na)["native_owned_hold"]==true).await;outage.protected(&na);assert_peer_healthy(&b,&nb);for kind in ["http","wss"]{service.command(json!({"op":"transport","kind":kind,"enabled":false,"target":"consumer"}));}ready_owner(&mut service,&a,&na,&wa,true).await;outage.rearm(&mut service,&na).await;
  let saved=na.root.clone();let old_store=a.store.clone();let journal=a.root.join("transfers.jsonl");na.close();a.close().await;let cold_root=own.join("host-cold");std::fs::create_dir_all(&cold_root).unwrap();std::fs::copy(journal,cold_root.join("transfers.jsonl")).unwrap();a=Client::start(&service.private(),&cold_root,Some(old_store),false,false,router.routes.clone(),service.watches.clone(),service.stores.clone()).await;na=Native::start(&a,Some(&saved),true);wait(&mut service,&a,"cold item explicit choice",15,|v|v["connection"]["state"]=="Choice").await;fixture_signal::write(na.root.join("choose-rejoin"),"explicit rejoin").unwrap();ready_owner(&mut service,&a,&na,&wa,true).await;fixture_signal::write(na.root.join("b1-outlet"),"injected cold outlet").unwrap();until(&mut service,"same pre-effect item collected after cold app",||received(&na)==1&&items(&na)["view"]["queued"]==0).await;fixture_signal::remove(na.root.join("b1-outlet")).unwrap();checkpoint(&own,"retained-outage-and-known-pre-effect-cold-item-collectible");
  // Lose the local result after construction, then reopen an older synthetic
  // engine world. App EffectIntent is uncertain and must never replay.
  item_spawn(&mut service,&na,&samples[4],false).await;until(&mut service,"ambiguous sample available",||available(&na).len()==1).await;let target=item_click(&mut service,&na).await;until(&mut service,"exact claim original before result fault",||items(&na)["view"]["submission"]["id"]==target&&items(&na)["view"]["submission"]["status"]=="accepted").await;fixture_signal::write(na.root.join("items-copy-block"),serde_json::to_vec(&json!({"kind":"item_result","id":target})).unwrap()).unwrap();fixture_signal::write(na.root.join("b1-outlet"),"injected ambiguous outlet").unwrap();until(&mut service,"item effect before result delivery",||bank_input(&na)["kind"]=="item_result").await;let late_result=bank_input(&na);let ambiguous=late_result["id"].as_str().unwrap().to_owned();let saved=na.root.clone();let old_store=a.store.clone();let journal=a.root.join("transfers.jsonl");na.close();a.close().await;let again_root=own.join("host-ambiguous");std::fs::create_dir_all(&again_root).unwrap();std::fs::copy(journal,again_root.join("transfers.jsonl")).unwrap();a=Client::start(&service.private(),&again_root,Some(old_store),false,false,router.routes.clone(),service.watches.clone(),service.stores.clone()).await;na=Native::start(&a,Some(&saved),true);wait(&mut service,&a,"ambiguous item explicit choice",15,|v|v["connection"]["state"]=="Choice").await;fixture_signal::write(na.root.join("choose-rejoin"),"explicit rejoin").unwrap();ready_owner(&mut service,&a,&na,&wa,true).await;fixture_signal::write(na.root.join("b1-outlet"),"injected ambiguous restored outlet").unwrap();elapsed(&mut service,4).await;assert_eq!(received(&na),0,"Restored ambiguous item cannot replay even in an older world");assert!(current_originals(&a).iter().any(|v|v["original_id"]==ambiguous&&v["phase"]=="EffectIntent"));checkpoint(&own,"post-effect-cold-original-fenced-no-replay");
  assert_peer_healthy(&b,&nb);fresh_bidirectional_ghosts(&mut service,&a,&na,&b,&nb,"final item bank movement").await;
  let captured=Arc::new(Mutex::new(None));let slot=captured.clone();a.handle.command(move|c|Box::pin(async move{*slot.lock().unwrap()=c.g3.as_ref().and_then(|g|g.bank_context());Ok(json!({"captured":true}))})).await.unwrap();let mut context=captured.lock().unwrap().take().unwrap();let original=context.consumer.journal.pending().find(|p|p.record.original.operation_id==ambiguous).unwrap().record.original;
  for field in ["roomId","worldToken"]{let mut scope:Value=serde_json::from_str(&original.scope_json).unwrap();scope[field]=uuid::Uuid::new_v4().to_string().into();let request=nt2_desktop_g0::protocol_g3::wire::Request{major:Some(1),revision:Some(nt2_desktop_g0::protocol_g3::REVISION.into()),service_id:Some(context.binding.service_id.clone()),service_epoch:Some(context.binding.service_epoch.clone()),session_epoch:Some(context.binding.session_epoch.clone()),request_id:Some(uuid::Uuid::new_v4().to_string()),scope:Some(serde_json::from_value(scope).unwrap()),current_scope:Some(context.binding.scope.clone()),item_status:Some(nt2_desktop_g0::protocol_g3::wire::ItemStatus{operation_id:Some(ambiguous.clone())}),..Default::default()};assert!(matches!(context.owner.call(serde_json::to_vec(&request).unwrap()).await,Err(e)if e=="Original recovery crossed world"));}
  let mut host_end=expect_loss(&mut service,&na,"items global End host").await;let mut peer_end=expect_loss(&mut service,&nb,"items global End peer").await;host_end.before_fault(&na);peer_end.before_fault(&nb);item_end_pair(&mut service,[(&a,&na,&wa,&host_end),(&b,&nb,&wb,&peer_end)]).await;host_end.protected(&na);peer_end.protected(&nb);host_end.terminal(&na);peer_end.terminal(&nb);na.close();nb.close();a.close().await;b.close().await;
  // Reuse the accepted cleanup driver's run/open/close and exact-ID helpers.
  use super::lobby_delete_composition::{begin_run,end_run,journal_rows,retained,audit};
  let mut disabled=service.private()["room"]["config"].clone();let mut settings=disabled["settings"].as_array().cloned().unwrap_or_default();settings.retain(|v|!matches!(v["key"].as_str(),Some("NT_send_wands"|"NT_send_flasks"|"NT_send_items")));for key in ["NT_send_wands","NT_send_flasks","NT_send_items"]{settings.push(json!({"key":key,"value":"false"}));}disabled["settings"]=json!(settings);service.command(json!({"op":"nextRun","config":disabled}));service.command(json!({"op":"signIn"}));let(mut policy,mut np)=begin_run(&mut service,&own.join("disabled-run"),&router,Some(&a)).await;
  for item in [&samples[1],&samples[2],&samples[4]]{let id=item_spawn(&mut service,&np,item,false).await;until(&mut service,"disabled category refusal retains original",||items(&np)["view"]["submission"]["id"]==id&&items(&np)["view"]["submission"]["status"]=="refused").await;assert_eq!(items(&np)["kills"],0);assert!(available(&np).is_empty());}assert_eq!(items(&np)["aliveOriginals"].as_array().unwrap().len(),3);let deleted_originals=journal_rows(&policy).await;end_run(&mut service,&mut policy,&mut np).await;checkpoint(&own,"three-frozen-item-policy-denials-retain-all-originals");
  service.command(json!({"op":"lobbyCreate"}));let room_b=service.private()["room"]["roomId"].as_str().unwrap().to_owned();assert_ne!(room_a,room_b);let(mut other,mut no)=begin_run(&mut service,&own.join("other-lobby"),&router,Some(&policy)).await;
  item_spawn(&mut service,&no,&samples[0],false).await;until(&mut service,"other lobby spell available with no standalone spell toggle",||available(&no).len()==1).await;item_click(&mut service,&no).await;until(&mut service,"other lobby queue retained",||items(&no)["view"]["queued"]==1).await;
  let keep:std::collections::BTreeSet<_>=journal_rows(&other).await.into_iter().filter(|o|serde_json::from_str::<Value>(&o.scope_json).unwrap()["roomId"]==room_b).map(|o|o.operation_id).collect();assert_eq!(keep.len(),2);let before=audit(&mut service,&room_a,&room_b);assert!(before["a"]["itemOperations"].as_u64().unwrap()>0);let denied=service.command(json!({"op":"lobbyDeleteForbidden","roomId":room_a}));assert_eq!(denied["accepted"],false);assert_eq!(audit(&mut service,&room_a,&room_b),before);service.command(json!({"op":"lobbyDelete","roomId":room_a}));service.command(json!({"op":"lobbyDelete","roomId":room_a}));
  let deleted=audit(&mut service,&room_a,&room_b);for field in ["bankItems","itemOperations","itemReceipts","itemInventories","itemOutbox"]{assert_eq!(deleted["a"][field],0);}assert_eq!(deleted["a"]["statistics"],before["a"]["statistics"]);assert_eq!(deleted["a"]["statisticsTotal"],before["a"]["statisticsTotal"]);assert_eq!(deleted["b"],before["b"]);
  let deadline=Instant::now()+Duration::from_secs(35);loop{service.wall_clock();let ids=retained(&other).await;let view=other.handle.view();let shown=view["g3"]["operations"].as_array().and_then(|r|r.iter().map(|v|v["operation_id"].as_str().map(str::to_owned)).collect::<Option<std::collections::BTreeSet<_>>>());if ids==keep&&shown.as_ref()==Some(&keep){break}assert!(Instant::now()<deadline,"Item lobby retirement deadline");tokio::time::sleep(Duration::from_millis(200)).await;}
  let stale=deleted_originals.into_iter().find(|o|o.operation_id==ambiguous).unwrap();other.handle.command(move|c|Box::pin(async move{let j=c.g3.as_ref().unwrap().consumer.journal.clone();assert!(j.prepare_effect(&stale.operation_id).is_err());assert!(tokio::task::spawn_blocking(move||j.begin(stale)).await.unwrap().is_err());Ok(json!({"lateItemOriginalFenced":true}))})).await.unwrap();fixture_signal::write(no.root.join("items-late-result"),serde_json::to_vec(&late_result).unwrap()).unwrap();until(&mut service,"late deleted item result reaches normal mailbox",||bank_input(&no)["kind"]=="item_result"&&bank_input(&no)["id"]==ambiguous).await;elapsed(&mut service,1).await;assert_eq!(retained(&other).await,keep);assert_eq!(audit(&mut service,&room_a,&room_b),deleted);assert_eq!(received(&no),0);fixture_signal::write(no.root.join("b1-outlet"),"injected other-lobby outlet").unwrap();until(&mut service,"unrelated lobby queue still collectible",||received(&no)==1&&items(&no)["view"]["queued"]==0).await;end_run(&mut service,&mut other,&mut no).await;
  let reopen_root=own.join("deleted-reopen");std::fs::create_dir_all(&reopen_root).unwrap();std::fs::copy(other.root.join("transfers.jsonl"),reopen_root.join("transfers.jsonl")).unwrap();let mut reopened=Client::start(&service.private(),&reopen_root,Some(other.store.clone()),false,false,router.routes.clone(),service.watches.clone(),service.stores.clone()).await;elapsed(&mut service,2).await;assert_eq!(retained(&reopened).await,keep);assert!(reopened.pending.lock().unwrap().is_none());reopened.close().await;checkpoint(&own,"item-delete-other-lobby-late-result-cold-reopen-and-statistics-preserved");
  let capacity=service.command(json!({"op":"liveCapacityStatus"}));let authentication=service.command(json!({"op":"liveAuthStatus"}));let items=service.command(json!({"op":"goldStatus"}));
  std::fs::write(own.join("service-final-diagnostics.json"),serde_json::to_vec_pretty(&json!({"capacity":capacity,"authentication":authentication,"items":items,"boundary":"after all accepted item checks and cooperative app/native closure"})).unwrap()).unwrap();service.close();
  std::fs::write(own.join("checks.json"),serde_json::to_vec_pretty(&json!({"passed":true,"actualComponents":true,"engineItemFactsInjected":true,"loadedSaveProof":false,"noitaLaunched":false,"packageChanged":false,"representativeKinds":5,"originals":originals,"uncertainSource":failed,"uncertainClaim":failed_claim,"sourceRemovalBeforePublication":true,"lostIntentRecovered":true,"slowItemsKeepSocialMovementGoldAlive":true,"scopedSchemaCleanupByFixture":true})).unwrap()).unwrap();
 })).unwrap().join().unwrap();
}

#[test]#[ignore="Root serialized deferred-removal composition: actual scheduler/TLS/SQL/native/mod, injected engine timing"]
fn item_deferred_removal_ordinary_scheduler_composition(){
 assert_eq!(std::env::var("NT2_C3_COMPOSITION").as_deref(),Ok("1"));assert_eq!(std::env::var("NT2_ITEM_FIXTURE").as_deref(),Ok("1"));
 std::thread::Builder::new().name("item-removal".into()).stack_size(16*1024*1024).spawn(||tauri::async_runtime::block_on(async{
  verify_current_fixture_inputs();let own=repo().join("spikes/noita-entry/.build/item-deferred-removal-20261025-01").join(format!("composition-{}",uuid::Uuid::new_v4()));std::fs::create_dir_all(&own).unwrap();eprintln!("ITEM_REMOVAL_COMPOSITION {}",own.display());
  let mut service=Service::start(&own);network_profile::enable_elapsed_clock(&mut service,&own);let router=PipeHarness::new();
  let mut a=Client::start(&service.private(),&own.join("host"),None,false,true,router.routes.clone(),service.watches.clone(),service.stores.clone()).await;bundle(&mut service,&a).await;let mut na=Native::start(&a,None,false);
  wait(&mut service,&a,"removal host Ready",30,|v|current_state_is(v,"Ready")&&na.root.join("associated").is_file()).await;let wa=a.handle.view()["rejoin"]["current_world"].clone();
  service.command(json!({"op":"peer"}));let mut b=Client::start(&service.private(),&own.join("peer"),None,true,true,router.routes.clone(),service.watches.clone(),service.stores.clone()).await;bundle(&mut service,&b).await;let mut nb=Native::start(&b,None,false);
  wait(&mut service,&b,"removal peer Ready",30,|v|current_state_is(v,"Ready")&&nb.root.join("associated").is_file()).await;let wb=b.handle.view()["rejoin"]["current_world"].clone();two_peer_ghost_with_start(&mut service,&a,&na,&wa,&b,&nb,&wb,"startReviewed").await;
  until(&mut service,"removal bank views ready",||items(&na)["view"]["fresh"]==true&&items(&nb)["view"]["fresh"]==true).await;
  let sample=json!({"catalogueRevision":"nt2-items-base-2","spell":{"actionId":"LIGHT_BULLET","usesRemaining":"-1"}});
  fixture_signal::write(na.root.join("items-removal-stuck"),"injected stuck engine deletion").unwrap();let stuck=item_spawn(&mut service,&na,&sample,false).await;
  until(&mut service,"bounded stuck removal acknowledged uncertain",||items(&na)["view"]["diagnostics"]["receipt_success"]["operation_id"]==stuck&&items(&na)["view"]["diagnostics"]["receipt_success"]["state"]=="uncertain").await;
  assert!(available(&nb).is_empty());assert_eq!(items(&na)["kills"],1);assert_eq!(items(&na)["aliveOriginals"].as_array().unwrap().len(),1);
  fixture_signal::remove(na.root.join("items-removal-stuck")).unwrap();let mut completed=Vec::new();
  for index in 0..3{
   let id=item_spawn(&mut service,&na,&sample,false).await;
   until(&mut service,"same-live deferred removal receipt completed and peer bank available",||items(&na)["view"]["diagnostics"]["receipt_success"]["operation_id"]==id&&items(&na)["view"]["diagnostics"]["receipt_success"]["state"]=="completed"&&available(&nb).len()==index+1).await;
   let snapshot=items(&na);let history=snapshot["removalHistory"].as_array().unwrap();let removal=history.last().unwrap();assert!(removal["observedFrame"].as_u64().unwrap()>removal["requestedFrame"].as_u64().unwrap(),"Engine deletion must have been deferred across multiple actual mod frames");
   assert!(snapshot["effectOutcomes"].as_array().unwrap().iter().any(|e|e["id"]==id&&e["state"]=="applied_session"&&e["reason"]=="removal_observed"));
   assert_eq!(snapshot["kills"],index+2,"One kill request per original, including stuck original");assert_eq!(snapshot["aliveOriginals"].as_array().unwrap().len(),1,"The unrelated ambiguous original is retained");completed.push(id);
  }
  elapsed(&mut service,1).await;assert_eq!(items(&na)["kills"],4);assert_eq!(available(&nb).len(),3);assert_eq!(items(&na)["aliveOriginals"].as_array().unwrap().len(),1);
  std::fs::write(own.join("removal-outcomes.json"),serde_json::to_vec_pretty(&json!({"host":items(&na),"peer":items(&nb),"stuckOriginal":stuck,"completedOriginals":completed})).unwrap()).unwrap();
  let host=expect_loss(&mut service,&na,"deferred removal End host").await;let peer=expect_loss(&mut service,&nb,"deferred removal End peer").await;
  item_end_pair(&mut service,[(&a,&na,&wa,&host),(&b,&nb,&wb,&peer)]).await;
  na.close();nb.close();a.close().await;b.close().await;service.close();
  std::fs::write(own.join("checks.json"),serde_json::to_vec_pretty(&json!({"passed":true,"actualComponents":true,"engineRemovalTimingInjected":true,"actualNoitaTrace":false,"loadedSaveProof":false,"stuckOriginalUncertainRetained":true,"threeFollowingOriginalsCompleted":true,"oneKillPerOriginal":true,"bankAvailableOnlyAfterObservedRemovalAndReceipt":true})).unwrap()).unwrap();
 })).unwrap().join().unwrap();
}

include!("next_bank_latency_composition_test.rs");
include!("next_bank_throughput_comparison_test.rs");

include!("next_bank_events_composition_test.rs");

include!("next_bank_events_fault_test.rs");

include!("next_bank_isolation_composition_test.rs");

include!("next_item_target_composition_test.rs");
include!("next_c1_player_composition_test.rs");
include!("next_c2_world_composition_test.rs");

mod d1_composition{use super::*;include!("next_d1_composition_test.rs");}
mod race_composition{use super::*;include!("next_race_composition_test.rs");}
