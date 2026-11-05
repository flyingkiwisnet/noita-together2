// Root serializes this actual-component composition. Only engine wallet,
// physical input, outlet observations and labelled transport faults are injected.
mod lobby_delete_composition{use super::*;include!("next_lobby_delete_composition_test.rs");}
mod item_composition{use super::*;include!("next_item_composition_test.rs");}
fn bank_view(n:&Native)->Value{fixture_signal::read(n.root.join("b1-view.json")).unwrap().and_then(|s|serde_json::from_str(&s).ok()).unwrap_or(Value::Null)}
fn bank_input(n:&Native)->Value{fixture_signal::read(n.root.join("b1-last-input.json")).unwrap().and_then(|s|serde_json::from_str(&s).ok()).unwrap_or(Value::Null)}
fn bank_submit_input(n:&Native)->Value{fixture_signal::read(n.root.join("b1-last-submit.json")).unwrap().and_then(|s|serde_json::from_str(&s).ok()).unwrap_or(Value::Null)}
async fn bank_submit(service:&mut Service,n:&Native,direction:&str,amount:u32,blocked:bool)->String{
 until(service,"bank ready for deliberate input",||bank_view(n)["view"]["fresh"]==true&&bank_view(n)["pending"].is_null()).await;
 if bank_view(n)["open"]!=true{click(service,n,"Gold bank").await;}
 until(service,"bank panel open",||bank_view(n)["open"]==true).await;
 fixture_signal::write(n.root.join("b1-amount"),amount.to_string()).unwrap();
 until(service,"bank amount edited",||bank_view(n)["amount"]==amount.to_string()).await;
 if blocked{fixture_signal::write(n.root.join("b1-copy-block-kind"),"submit").unwrap();}
 let before=bank_submit_input(n)["id"].clone();click(service,n,if direction=="deposit"{"DEPOSIT"}else{"TAKE"}).await;
 until(service,"bank original copied or deliberately blocked",||{let i=bank_submit_input(n);i["kind"]=="submit"&&i["id"].is_string()&&i["id"]!=before}).await;
 let id=bank_submit_input(n)["id"].as_str().unwrap().to_owned();
 // Editing deliberately owns Controls. Close the actual panel before walking
 // or checking unrelated gameplay; the bank request/result stays in flight.
 click(service,n,"Close").await;until(service,"bank panel closed after input",||bank_view(n)["open"]==false).await;
 id
}
async fn accepted(service:&mut Service,n:&Native,id:&str){until(service,"same bank original accepted",||{let v=bank_view(n);v["view"]["submission"]["id"]==id&&v["view"]["submission"]["status"]=="accepted"&&v["amount"]=="1"}).await;}
fn current_originals(c:&Client)->Vec<Value>{std::fs::read_to_string(c.root.join("transfers.jsonl")).unwrap_or_default().lines().filter_map(|l|serde_json::from_str(l).ok()).collect()}

// Change only the capture control, never a first-loss receipt or the native
// observer's history/counters. NativeWatch remains fatal on retained receipts,
// including unrelated transport failures, throughout an intentional window.
fn set_loss_capture(root:&std::path::Path,armed:bool)->Result<(),String>{
 let run=root.parent().and_then(|p|p.parent()).ok_or("Native run root absent")?;
 for receipt in [root.join("first-unexpected-hold.json"),run.join("network-first-transport-loss.json")]{
  if fixture_signal::present(&receipt).map_err(|e|e.to_string())?{return Err("Unexpected loss receipt already retained".into())}
 }
 let flag=root.join("capture-first-loss");
 if fixture_signal::present(&flag).map_err(|e|e.to_string())?==armed{return Err("Loss capture control already in requested state".into())}
 if armed{fixture_signal::write(flag,"B1 resume unexpected loss capture")}else{fixture_signal::remove(flag)}.map_err(|e|e.to_string())
}
fn loss_sample(value:&Value,holds:u64,context:&Value,playing:bool)->Result<(),&'static str>{
 if value["holds"].as_u64()!=Some(holds){return Err("Unexpected native permission-loss count")}
 if !context.as_str().is_some_and(|s|!s.is_empty())||value["native_context"]!=*context{return Err("Native context changed during expected loss")}
 if playing{
  if value["protected"]!=false||value["native_owned_hold"]!=false||value["controls_enabled"]!=true||value["input_locked"]!=false||value["native_status"]!=2{return Err("Expected loss boundary requires actual recovered Controls and permission")}
  let frame=value["frame"].as_u64().ok_or("Native frame absent")?;
  let status=value["native_status_frame"].as_u64().ok_or("Native status frame absent")?;
  if frame==0||status>frame||frame-status>1{return Err("Native permission sample is stale")}
 }else if value["protected"]!=true||value["native_owned_hold"]!=true||value["controls_enabled"]!=false{return Err("Expected loss did not install actual native protection")}
 Ok(())
}
pub(in crate::next_client::lc1_composition) struct ExpectedLoss{label:&'static str,before:Value,observed:bool}
impl ExpectedLoss{
 pub(in crate::next_client::lc1_composition) fn holds(&self)->u64{self.before["holds"].as_u64().unwrap()}
 pub(in crate::next_client::lc1_composition) fn record(&self,n:&Native,phase:&str){
  let value=json!({"window":self.label,"phase":phase,"before":self.before,"current":native_activity(n),"engineFactsInjected":true,"historyRetained":true});
  let mut out=std::fs::OpenOptions::new().create(true).append(true).open(n.root.join("expected-loss-windows.jsonl")).unwrap();writeln!(out,"{value}").unwrap();
 }
 pub(in crate::next_client::lc1_composition) fn before_fault(&self,n:&Native){
  let sample=native_activity(n);loss_sample(&sample,self.holds(),&self.before["native_context"],true).unwrap();assert_eq!(sample["loss_monitor_armed"],false);
 }
 pub(in crate::next_client::lc1_composition) fn protected(&mut self,n:&Native){
  assert!(!self.observed);let sample=native_activity(n);loss_sample(&sample,self.holds()+1,&self.before["native_context"],false).unwrap();assert_eq!(sample["loss_monitor_armed"],false);self.observed=true;self.record(n,"expected protection");
 }
 pub(in crate::next_client::lc1_composition) async fn rearm(self,service:&mut Service,n:&Native){
  assert!(self.observed);check_natives(&service.watches,"before expected-loss recovery rearm");
  let recovered=native_activity(n);loss_sample(&recovered,self.holds()+1,&self.before["native_context"],true).unwrap();assert_eq!(recovered["loss_monitor_armed"],false);
  set_loss_capture(&n.root,true).unwrap();
  until(service,"B1 unexpected-loss capture rearmed after recovery",||{
   let sample=native_activity(n);loss_sample(&sample,self.holds()+1,&self.before["native_context"],true).unwrap();
   sample["frame"].as_u64()>recovered["frame"].as_u64()&&sample["loss_monitor_armed"]==true
  }).await;
  self.record(n,"recovered and rearmed");
 }
 pub(in crate::next_client::lc1_composition) fn terminal(self,n:&Native){assert!(self.observed);loss_sample(&native_activity(n),self.holds()+1,&self.before["native_context"],false).unwrap();self.record(n,"terminal End; cooperative close follows");}
}
pub(in crate::next_client::lc1_composition) async fn expect_loss(service:&mut Service,n:&Native,label:&'static str)->ExpectedLoss{
 check_natives(&service.watches,"before deliberate B1 loss window");let before=native_activity(n);
 let holds=before["holds"].as_u64().expect("Current native loss counter");loss_sample(&before,holds,&before["native_context"],true).unwrap();assert_eq!(before["loss_monitor_armed"],true);
 set_loss_capture(&n.root,false).unwrap();
 until(service,"B1 loss capture disarmed before deliberate fault",||{
  let sample=native_activity(n);loss_sample(&sample,holds,&before["native_context"],true).unwrap();
  sample["frame"].as_u64()>before["frame"].as_u64()&&sample["loss_monitor_armed"]==false
 }).await;
 let window=ExpectedLoss{label,before,observed:false};window.record(n,"disarmed before deliberate fault");window
}
fn assert_loss_capture(n:&Native){assert!(fixture_signal::present(n.root.join("capture-first-loss")).unwrap());assert_eq!(native_activity(n)["loss_monitor_armed"],true);}

#[test]fn b1_expected_loss_controls_preserve_and_refuse_unexpected_receipts(){
 let run=repo().join("spikes/noita-entry/.build/b1-client-20261024-01").join(format!("loss-controls-{}",uuid::Uuid::new_v4()));let root=run.join("host/native");std::fs::create_dir_all(&root).unwrap();
 let flag=root.join("capture-first-loss");fixture_signal::write(&flag,"armed from startup").unwrap();let history=root.join("permission-edges.jsonl");let retained=b"{\"holds\":3,\"input_locks\":7}\n";std::fs::write(&history,retained).unwrap();
 let receipt=root.join("first-unexpected-hold.json");std::fs::write(&receipt,b"{\"holds\":4}").unwrap();assert!(set_loss_capture(&root,false).is_err());assert!(flag.is_file());assert_eq!(std::fs::read(&receipt).unwrap(),b"{\"holds\":4}");
 // This synthetic receipt is removed only by the test that created it. The
 // implementation cannot remove/rename/clear any receipt to open a window.
 std::fs::remove_file(&receipt).unwrap();let transport=run.join("network-first-transport-loss.json");std::fs::write(&transport,b"{\"retire_code\":\"CAPACITY\"}").unwrap();assert!(set_loss_capture(&root,false).is_err());assert!(flag.is_file());
 std::fs::remove_file(&transport).unwrap();set_loss_capture(&root,false).unwrap();assert!(!flag.exists());assert!(set_loss_capture(&root,false).is_err());
 std::fs::write(&transport,b"{\"retire_code\":\"CAPACITY\"}").unwrap();assert!(set_loss_capture(&root,true).is_err());assert!(!flag.exists());assert_eq!(std::fs::read(&transport).unwrap(),b"{\"retire_code\":\"CAPACITY\"}");
 std::fs::remove_file(&transport).unwrap();set_loss_capture(&root,true).unwrap();assert!(flag.is_file());assert_eq!(std::fs::read(&history).unwrap(),retained);
}
#[test]fn b1_expected_loss_requires_one_actual_hold_and_coherent_recovery(){
 let playing=json!({"holds":3,"native_context":"same-native","frame":100,"native_status_frame":100,"protected":false,"native_owned_hold":false,"controls_enabled":true,"input_locked":false,"native_status":2});let context=playing["native_context"].clone();assert!(loss_sample(&playing,3,&context,true).is_ok());
 let mut held=playing.clone();held["holds"]=json!(4);held["protected"]=json!(true);held["native_owned_hold"]=json!(true);held["controls_enabled"]=json!(false);held["native_status"]=json!(0);assert!(loss_sample(&held,4,&context,false).is_ok());assert!(loss_sample(&held,4,&context,true).is_err());
 let mut recovered=playing.clone();recovered["holds"]=json!(4);assert!(loss_sample(&recovered,4,&context,true).is_ok());recovered["holds"]=json!(5);assert!(loss_sample(&recovered,4,&context,true).is_err(),"A second loss while rearming must remain visible");assert!(loss_sample(&recovered,3,&context,true).is_err(),"A loss during disarm preparation must fail");
 recovered=playing.clone();recovered["native_status_frame"]=json!(98);assert!(loss_sample(&recovered,3,&context,true).is_err());recovered=playing.clone();recovered["native_context"]=json!("replacement");assert!(loss_sample(&recovered,3,&context,true).is_err());
}
#[test]#[ignore="Root serialized B1 ordinary scheduler + TLS/SQL/local WebSocket/native/Lua; synthetic engine wallet, no loaded-save proof"]
fn b1_gold_ordinary_scheduler_composition(){
 assert_eq!(std::env::var("NT2_C3_COMPOSITION").as_deref(),Ok("1"));assert_eq!(std::env::var("NT2_B1_FIXTURE").as_deref(),Ok("1"));
 std::thread::Builder::new().name("b1-bank".into()).stack_size(16*1024*1024).spawn(||tauri::async_runtime::block_on(async{
  verify_current_fixture_inputs();let own=repo().join("spikes/noita-entry/.build/b1-client-20261024-01").join(format!("composition-{}",uuid::Uuid::new_v4()));std::fs::create_dir_all(&own).unwrap();eprintln!("B1_COMPOSITION {}",own.display());
  let mut service=Service::start(&own);network_profile::enable_elapsed_clock(&mut service,&own);let router=PipeHarness::new();
  let mut a=Client::start(&service.private(),&own.join("host"),None,false,true,router.routes.clone(),service.watches.clone(),service.stores.clone()).await;bundle(&mut service,&a).await;let mut na=Native::start(&a,None,false);
  wait(&mut service,&a,"B1 host Ready",30,|v|current_state_is(v,"Ready")&&na.root.join("associated").is_file()).await;let wa=a.handle.view()["rejoin"]["current_world"].clone();
  service.command(json!({"op":"peer"}));let mut b=Client::start(&service.private(),&own.join("peer"),None,true,true,router.routes.clone(),service.watches.clone(),service.stores.clone()).await;bundle(&mut service,&b).await;let mut nb=Native::start(&b,None,false);
  wait(&mut service,&b,"B1 peer Ready",30,|v|current_state_is(v,"Ready")&&nb.root.join("associated").is_file()).await;let wb=b.handle.view()["rejoin"]["current_world"].clone();
  two_peer_ghost_with_start(&mut service,&a,&na,&wa,&b,&nb,&wb,"startReviewed").await;
  until(&mut service,"both confirmed zero bank snapshots",||bank_view(&na)["view"]["balance"]==0&&bank_view(&nb)["view"]["balance"]==0&&bank_view(&na)["view"]["fresh"]==true).await;
  let local_drop=std::env::var("NT2_LOCAL_WS_DROP_BANK").as_deref()==Ok("1");let local_before=native_activity(&na);if local_drop{assert!(!pipe_compare());local_ws_disarm(&mut service,&a,&na).await;a.handle.command(|c|Box::pin(async move{c.native_diagnostics.lock().unwrap().local_drop_bank_reply_once=true;Ok(Value::Null)})).await.unwrap();}
  let deposit=bank_submit(&mut service,&na,"deposit",100,false).await;accepted(&mut service,&na,&deposit).await;
  if local_drop{ready_owner(&mut service,&a,&na,&wa,true).await;assert_eq!(a.diagnostics.lock().unwrap().local_dropped_bank_replies,1);assert!(a.local_bridge().view()["listener"]["authenticated"].as_u64().unwrap()>=2);assert_eq!(a.local_bridge().view()["listener"]["closed"],1,"Only the deliberately dropped reply may close the socket");fresh_bidirectional_ghosts(&mut service,&a,&na,&b,&nb,"lost local bank reply restores current C1 and fresh ghosts").await;let settled_holds=native_activity(&na)["holds"].as_u64().unwrap();assert!(settled_holds>=local_before["holds"].as_u64().unwrap());elapsed(&mut service,2).await;assert_eq!(native_activity(&na)["holds"].as_u64(),Some(settled_holds),"Recovered C1/native permission must stay healthy");assert_peer_healthy(&b,&nb);fixture_input_write(na.root.join("capture-first-loss"),b"rearmed after expected lost local mutation reply").unwrap();until(&mut service,"local mutation reply recovery monitor rearmed",||native_activity(&na)["loss_monitor_armed"]==true).await;checkpoint(&own,"lost-local-WS-mutation-reply-same-original-one-effect-and-reconnected");}

  until(&mut service,"cross-player balance and once-only debit",||bank_view(&na)["wallet"]==900&&bank_view(&na)["writes"]==1&&bank_view(&nb)["view"]["balance"]==100).await;
  until(&mut service,"named deposit notice",||text(&nb,"a2-notices.txt").contains("deposited 100 gold to the bank")).await;
  let take=bank_submit(&mut service,&nb,"withdraw",40,false).await;accepted(&mut service,&nb,&take).await;
  until(&mut service,"reserved queue without early wallet credit",||bank_view(&nb)["view"]["queued"]==40&&bank_view(&nb)["wallet"]==1000&&bank_view(&na)["view"]["balance"]==60).await;
  fixture_signal::write(nb.root.join("b1-no-wallet"),"injected unavailable wallet").unwrap();fixture_signal::write(nb.root.join("b1-outlet"),"injected outlet proximity").unwrap();elapsed(&mut service,2).await;assert_eq!(bank_view(&nb)["wallet"],1000);assert_eq!(bank_view(&nb)["writes"],0);
  fixture_signal::remove(nb.root.join("b1-no-wallet")).unwrap();until(&mut service,"outlet credits once with pink feedback",||bank_view(&nb)["wallet"]==1040&&bank_view(&nb)["pink"]==1&&bank_view(&nb)["writes"]==1).await;elapsed(&mut service,2).await;assert_eq!(bank_view(&nb)["writes"],1);fixture_signal::remove(nb.root.join("b1-outlet")).unwrap();
  checkpoint(&own,"deposit-shared-versioned-balance-live-notice-queued-take-missing-wallet-and-once-only-outlet");

  // A committed goldIntent reply is lost on the real service transport. The
  // same persisted operation is queried; neither wallet nor bank duplicates.
  let lost=bank_submit(&mut service,&na,"deposit",20,true).await;service.command(json!({"op":"goldArm","plan":{"target":"consumer","operationId":lost,"kind":"goldIntent"}}));fixture_signal::remove(na.root.join("b1-copy-block-kind")).unwrap();
  until(&mut service,"lost intent recovers same original and one debit",||bank_view(&na)["wallet"]==880&&bank_view(&na)["writes"]==2&&bank_view(&nb)["view"]["balance"]==80).await;
  assert_eq!(current_originals(&a).iter().filter(|v|v["original"]["operation_id"]==lost&&v["phase"]=="Requested").count(),1);
  // Keep a real committed bank reply slow while ordinary social and movement
  // remain eligible. This fault is scoped to one original, not the whole room.
  let slow=bank_submit(&mut service,&na,"deposit",10,true).await;service.command(json!({"op":"goldArm","plan":{"target":"consumer","operationId":slow,"kind":"goldIntent","effect":"delay","delayMs":2500}}));fixture_signal::remove(na.root.join("b1-copy-block-kind")).unwrap();
  service.command(json!({"op":"chat","target":"host","text":"B1 bank slow; social still live"}));until(&mut service,"chat progresses during slow bank",||text(&nb,"a2-notices.txt").contains("B1 bank slow; social still live")).await;
  fresh_bidirectional_ghosts(&mut service,&a,&na,&b,&nb,"B1 slow bank keeps movement and renewal live").await;
  until(&mut service,"slow bank completes once",||bank_view(&na)["wallet"]==870&&bank_view(&nb)["view"]["balance"]==90).await;
  // Lost committed receipt must retry the same receipt/original without a
  // second wallet mutation. The injected copy block is before the result copy.
  fixture_signal::write(na.root.join("b1-copy-block-kind"),"result").unwrap();let receipt=bank_submit(&mut service,&na,"deposit",10,false).await;
  until(&mut service,"receipt held after one actual guarded wallet mutation",||bank_view(&na)["wallet"]==860&&bank_input(&na)["kind"]=="result").await;
  service.command(json!({"op":"goldArm","plan":{"target":"consumer","operationId":receipt,"kind":"goldReceipt"}}));fixture_signal::remove(na.root.join("b1-copy-block-kind")).unwrap();
  until(&mut service,"lost committed receipt reconciles without duplicate credit",||bank_view(&nb)["view"]["balance"]==100).await;elapsed(&mut service,4).await;assert_eq!(bank_view(&na)["wallet"],860);assert_eq!(bank_view(&na)["writes"],4);
  checkpoint(&own,"lost-intent-lost-receipt-same-original-and-slow-bank-social-movement-renewal");

  // Both players race for the entire remaining balance. Exactly one reserves.
  let first=bank_submit(&mut service,&na,"withdraw",100,true).await;let second=bank_submit(&mut service,&nb,"withdraw",100,true).await;
  fixture_signal::remove(na.root.join("b1-copy-block-kind")).unwrap();fixture_signal::remove(nb.root.join("b1-copy-block-kind")).unwrap();
  until(&mut service,"one concurrent take accepted and one refused",||{let va=bank_view(&na);let vb=bank_view(&nb);let sa=&va["view"]["submission"]["status"];let sb=&vb["view"]["submission"]["status"];(sa=="accepted"&&sb=="refused")||(sa=="refused"&&sb=="accepted")}).await;
  assert_eq!(bank_view(&na)["wallet"],860);assert_eq!(bank_view(&nb)["wallet"],1040);assert_ne!(first,second);
  until(&mut service,"available bank zero after single reservation",||bank_view(&na)["view"]["balance"]==0&&bank_view(&nb)["view"]["balance"]==0).await;
  // Replenish and retain/create a definitely unissued host queue for retained/cold
  // recovery, irrespective of which concurrent withdrawal won. The UI correctly
  // refuses a second take while the first is still waiting at the outlet.
  let replenished=bank_submit(&mut service,&na,"deposit",50,false).await;accepted(&mut service,&na,&replenished).await;until(&mut service,"replenished bank",||bank_view(&nb)["view"]["balance"]==50).await;
  let queued=if bank_view(&na)["view"]["queued"].as_u64().unwrap_or(0)>0{assert!(current_originals(&a).iter().any(|r|r["original"]["operation_id"]==first&&r["phase"]=="ServerAccepted"));first.clone()}else{let id=bank_submit(&mut service,&na,"withdraw",25,false).await;accepted(&mut service,&na,&id).await;id};let queued_wallet=bank_view(&na)["wallet"].as_u64().unwrap();let queued_writes=bank_view(&na)["writes"].clone();
  let mut outage=expect_loss(&mut service,&na,"retained host HTTP/WSS outage").await;assert_loss_capture(&nb);assert_peer_healthy(&b,&nb);outage.before_fault(&na);
  for kind in ["http","wss"]{service.command(json!({"op":"transport","kind":kind,"enabled":true,"target":"consumer"}));}
  until(&mut service,"outage protects host",||native_activity(&na)["native_owned_hold"]==true).await;outage.protected(&na);assert_peer_healthy(&b,&nb);assert_eq!(bank_view(&na)["wallet"],queued_wallet);
  for kind in ["http","wss"]{service.command(json!({"op":"transport","kind":kind,"enabled":false,"target":"consumer"}));}
  ready_owner(&mut service,&a,&na,&wa,true).await;outage.rearm(&mut service,&na).await;fresh_bidirectional_ghosts(&mut service,&a,&na,&b,&nb,"B1 retained outage reconnect").await;assert_eq!(bank_view(&na)["writes"],queued_writes);
  // Cold app reads the SAME durable journal. Wallet restoration is explicitly
  // injected and is not treated as Noita loaded-save evidence.
  // Native::close detaches only this child's watch before its cooperative
  // stop. The peer stays guarded; every replacement is armed from startup,
  // including Choice/rejoin, without a blanket recovery-tail exemption.
  check_natives(&service.watches,"before intentional cold fixture close");assert_loss_capture(&na);assert_loss_capture(&nb);assert_peer_healthy(&b,&nb);
  na.close();a.close().await;let cold_root=own.join("host-cold");std::fs::create_dir_all(&cold_root).unwrap();std::fs::copy(a.root.join("transfers.jsonl"),cold_root.join("transfers.jsonl")).unwrap();
  let mut cold=Client::start(&service.private(),&cold_root,Some(a.store.clone()),false,false,router.routes.clone(),service.watches.clone(),service.stores.clone()).await;let mut nc=Native::start(&cold,Some(&na.root),true);
  fixture_signal::write(nc.root.join("b1-set-wallet"),queued_wallet.to_string()).unwrap();wait(&mut service,&cold,"cold queued original explicit choice",15,|v|v["connection"]["state"]=="Choice").await;assert_loss_capture(&nc);assert_peer_healthy(&b,&nb);fixture_signal::write(nc.root.join("choose-rejoin"),"explicit rejoin").unwrap();ready_owner(&mut service,&cold,&nc,&wa,true).await;assert_loss_capture(&nc);assert_peer_healthy(&b,&nb);
  fixture_signal::write(nc.root.join("b1-outlet"),"injected eligible outlet").unwrap();until(&mut service,"known-unissued cold queue remains collectible",||bank_view(&nc)["writes"].as_u64().is_some_and(|v|v>=1)&&bank_view(&nc)["view"]["queued"]==0).await;fixture_signal::remove(nc.root.join("b1-outlet")).unwrap();
  checkpoint(&own,"concurrent-insufficient-take-retained-outage-and-durable-pre-effect-cold-recovery");

  // Stop the disposable fixture cooperatively after an actual guarded wallet
  // write but BEFORE its result reaches the app. Restored EffectIntent cannot
  // replay, even when a current service operation still authorizes collection.
  fixture_signal::write(nc.root.join("b1-copy-block-kind"),"result").unwrap();let ambiguous=bank_submit(&mut service,&nc,"deposit",5,false).await;
  until(&mut service,"ambiguous post-effect boundary",||bank_input(&nc)["kind"]=="result"&&bank_input(&nc)["id"]==ambiguous).await;let before=bank_view(&nc)["wallet"].as_u64().unwrap();check_natives(&service.watches,"before intentional ambiguous fixture close");assert_loss_capture(&nc);assert_loss_capture(&nb);assert_peer_healthy(&b,&nb);nc.close();cold.close().await;
  let again_root=own.join("host-ambiguous");std::fs::create_dir_all(&again_root).unwrap();std::fs::copy(cold.root.join("transfers.jsonl"),again_root.join("transfers.jsonl")).unwrap();
  let mut again=Client::start(&service.private(),&again_root,Some(cold.store.clone()),false,false,router.routes.clone(),service.watches.clone(),service.stores.clone()).await;let mut ng=Native::start(&again,Some(&nc.root),true);
  fixture_signal::write(ng.root.join("b1-set-wallet"),before.to_string()).unwrap();wait(&mut service,&again,"ambiguous restore explicit choice",15,|v|v["connection"]["state"]=="Choice").await;assert_loss_capture(&ng);assert_peer_healthy(&b,&nb);fixture_signal::write(ng.root.join("choose-rejoin"),"explicit rejoin").unwrap();ready_owner(&mut service,&again,&ng,&wa,true).await;assert_loss_capture(&ng);assert_peer_healthy(&b,&nb);elapsed(&mut service,4).await;
  assert_eq!(bank_view(&ng)["wallet"],before);assert_eq!(bank_view(&ng)["writes"],0);assert!(current_originals(&again).iter().any(|v|v["original"]["operation_id"]==ambiguous&&v["phase"]=="EffectIntent"));
  let captured=Arc::new(Mutex::new(None));let slot=captured.clone();again.handle.command(move|c|Box::pin(async move{*slot.lock().unwrap()=c.g3.as_ref().and_then(|g|g.bank_context());Ok(json!({"captured":true}))})).await.unwrap();
  let mut context=captured.lock().unwrap().take().expect("current B1 authority for wrong-world probe");let original=context.consumer.journal.pending().find(|p|p.record.original.operation_id==ambiguous).unwrap();
  let mut wrong:nt2_desktop_g0::protocol_g3::wire::Scope=serde_json::from_str(&original.record.original.scope_json).unwrap();wrong.world_token=Some(uuid::Uuid::new_v4().to_string());
  let request=nt2_desktop_g0::protocol_g3::wire::Request{major:Some(1),revision:Some(nt2_desktop_g0::protocol_g3::REVISION.into()),service_id:Some(context.binding.service_id.clone()),service_epoch:Some(context.binding.service_epoch.clone()),session_epoch:Some(context.binding.session_epoch.clone()),request_id:Some(uuid::Uuid::new_v4().to_string()),scope:Some(wrong),current_scope:Some(context.binding.scope.clone()),gold_status:Some(nt2_desktop_g0::protocol_g3::wire::GoldStatus{operation_id:Some(ambiguous.clone())}),..Default::default()};
  assert!(matches!(context.owner.call(serde_json::to_vec(&request).unwrap()).await,Err(e)if e=="Original recovery crossed world"),"wrong world must be refused by actual authenticated owner before transport");assert_eq!(bank_view(&ng)["writes"],0);
  assert_peer_healthy(&b,&nb);let mut end_host=expect_loss(&mut service,&ng,"global End host").await;let mut end_peer=expect_loss(&mut service,&nb,"global End peer").await;end_host.before_fault(&ng);end_peer.before_fault(&nb);
  service.command(json!({"op":"end"}));until(&mut service,"End blocks bank effects for both participants",||again.handle.view()["connection"]["state"]=="Ended"&&b.handle.view()["connection"]["state"]=="Ended"&&native_activity(&ng)["native_owned_hold"]==true&&native_activity(&nb)["native_owned_hold"]==true).await;end_host.protected(&ng);end_peer.protected(&nb);elapsed(&mut service,2).await;assert_eq!(bank_view(&ng)["writes"],0);end_host.terminal(&ng);end_peer.terminal(&nb);
  ng.close();nb.close();again.close().await;b.close().await;service.close();
  std::fs::write(own.join("checks.json"),serde_json::to_vec_pretty(&json!({"passed":true,"actualComponents":true,"engineWalletAndOutletInjected":true,"loadedSaveProof":false,"noitaLaunched":false,"packageChanged":false,"ambiguousOriginal":ambiguous,"knownUnissuedOriginal":queued,"sameOriginalLostIntent":lost,"sameOriginalLostReceipt":receipt,"scopedSchemaCleanupByFixture":true})).unwrap()).unwrap();
 })).unwrap().join().unwrap();
}
