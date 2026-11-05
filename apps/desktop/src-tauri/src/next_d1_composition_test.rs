// Normal Core scheduler, real TLS/SQL service and authenticated native WebSocket.
// Only engine entities, save bytes and physical gestures are synthetic.
async fn action(c:&Client,value:Value)->Value{let kind=value["action"].clone();for attempt in 0..6{match c.handle.action(serde_json::from_value(value.clone()).unwrap()).await{Ok(view)=>return view,Err(reason)if kind=="refresh_lobby"&&reason=="CAPACITY"&&attempt<5=>tokio::time::sleep(Duration::from_secs(5)).await,Err(reason)=>panic!("D1 app action {kind}: {reason}")}}unreachable!()}
async fn public_stats(c:&Client){
 let room=c.handle.view()["room"]["roomId"].clone();let before=action(c,json!({"action":"refresh_lobby","room":room})).await;
 let after=action(c,json!({"action":"set_lobby_policy","room":room,"revision":before["room"]["revision"],"policy":{"publicStats":true}})).await;
 assert_eq!(after["room"]["publicStats"],true);assert_eq!(after["room"]["configRevision"],before["room"]["configRevision"]);assert_eq!(after["rejoin"]["current_world"],before["rejoin"]["current_world"]);
}
async fn read_stats(service:&mut Service,c:&Client)->Value{
 let private=service.private();let ca=reqwest::Certificate::from_pem(&std::fs::read(private["caPath"].as_str().unwrap()).unwrap()).unwrap();
 let http=reqwest::Client::builder().add_root_certificate(ca).redirect(reqwest::redirect::Policy::none()).timeout(Duration::from_secs(4)).build().unwrap();
 let request=json!({"major":1,"revision":"g3-stats-1","serviceId":private["serviceId"],"serviceEpoch":private["serviceEpoch"],"requestId":uuid::Uuid::new_v4().to_string(),"read":{"locator":c.handle.view()["room"]["statsLocator"],"limit":32}});
 nt2_desktop_g0::protocol_stats::request(&serde_json::to_vec(&request).unwrap()).unwrap();
 for attempt in 0..6{service.wall_clock();let response=http.post(format!("{}/v3/stats/rpc",private["origin"].as_str().unwrap())).json(&request).send().await.unwrap();assert!(response.status().is_success());let bytes=response.bytes().await.unwrap();assert!(bytes.len()<=32768);
 let decoded=nt2_desktop_g0::protocol_stats::typed::<nt2_desktop_g0::protocol_stats::wire::Response>("Response",&bytes).unwrap();let value=serde_json::to_value(decoded).unwrap();assert_eq!(value["requestId"],request["requestId"]);assert_eq!(value["serviceId"],request["serviceId"]);assert_eq!(value["serviceEpoch"],request["serviceEpoch"]);if value["error"]["code"]=="CAPACITY"&&attempt<5{tokio::time::sleep(Duration::from_secs(5)).await;continue}assert!(value["error"].is_null(),"Anonymous public summary refused: {}",value["error"]);return value["summary"].clone()}unreachable!()
}
fn sum(summary:&Value,key:&str)->u64{summary["rows"].as_array().unwrap().iter().map(|r|r[key].as_str().unwrap().parse::<u64>().unwrap()).sum()}
async fn counts(service:&mut Service,c:&Client,expected:&[(&str,u64)])->Value{
 let deadline=Instant::now()+Duration::from_secs(30);loop{service.wall_clock();let summary=read_stats(service,c).await;
  if expected.iter().all(|(key,count)|sum(&summary,key)==*count){return summary}
  assert!(Instant::now()<deadline,"D1 expected {expected:?}, observed {summary}");tokio::time::sleep(Duration::from_secs(5)).await;
 }
}
async fn pickup(service:&mut Service,n:&Native,value:Value){let before=c1_view(n)["actions"].as_u64().unwrap();c1_data(n,value);until(service,"production local hook completed",||c1_view(n)["actions"].as_u64()==Some(before+1)).await;}
#[test]#[ignore="Root serialized D1 actual Client/service/native composition"]
fn d1_statistics_ordinary_scheduler_composition(){std::thread::Builder::new().name("d1-statistics".into()).stack_size(16*1024*1024).spawn(||tauri::async_runtime::block_on(async{
 verify_current_fixture_inputs();let own=repo().join("spikes/noita-entry/.build/d1-client-20261104").join(format!("composition-{}",uuid::Uuid::new_v4()));std::fs::create_dir_all(&own).unwrap();eprintln!("D1_COMPOSITION {}",own.display());
 let mut service=Service::start(&own);service.command(json!({"op":"hostApp"}));network_profile::enable_elapsed_clock(&mut service,&own);let router=PipeHarness::new();
 let mut host_private=service.private();let credentials=host_private["hostApp"].clone();assert_eq!(credentials["accountId"],host_private["room"]["hostAccountId"]);host_private["desktop"]=credentials.clone();host_private["deviceId"]=credentials["deviceId"].clone();let mut host=Client::start(&host_private,&own.join("host-management"),None,false,false,router.routes.clone(),service.watches.clone(),service.stores.clone()).await;
 let mut a=Client::start(&service.private(),&own.join("host"),None,false,true,router.routes.clone(),service.watches.clone(),service.stores.clone()).await;bundle(&mut service,&a).await;let mut na=Native::start(&a,None,false);
 wait(&mut service,&a,"D1 host Ready",30,|v|current_state_is(v,"Ready")&&na.root.join("associated").is_file()).await;let wa=a.handle.view()["rejoin"]["current_world"].clone();
 service.command(json!({"op":"peer"}));let mut b=Client::start(&service.private(),&own.join("peer"),None,true,true,router.routes.clone(),service.watches.clone(),service.stores.clone()).await;bundle(&mut service,&b).await;let mut nb=Native::start(&b,None,false);
 wait(&mut service,&b,"D1 peer Ready",30,|v|current_state_is(v,"Ready")&&nb.root.join("associated").is_file()).await;let wb=b.handle.view()["rejoin"]["current_world"].clone();
 two_peer_ghost_with_start(&mut service,&a,&na,&wa,&b,&nb,&wb,"startReviewed").await;
 until(&mut service,"D1 current local rules",||c1_view(&na)["ready"]==true&&c1_view(&nb)["ready"]==true).await;

 public_stats(&host).await;fresh_bidirectional_ghosts(&mut service,&a,&na,&b,&nb,"Public stats retains gameplay identity").await;
 let zero=counts(&mut service,&a,&[("hearts",0),("orbs",0),("steveKills",0),("deaths",0),("wins",0),("goldDeposited",0),("itemsDeposited",0)]).await;assert_eq!(zero["rows"].as_array().unwrap().len(),2);
 c1_rules(&mut service,&na,&nb,json!({"NT_sync_hearts":false,"NT_sync_orbs":false,"NT_death_penalty":"full_respawn","NT_ondeath_kick":false})).await;
 let remote_max=c1_hp(&nb,"max");let remote_hp=c1_hp(&nb,"hp");
 service.command(json!({"op":"d1DropReply","target":"consumer"}));pickup(&mut service,&na,json!({"kind":"heart"})).await;
 counts(&mut service,&a,&[("hearts",1)]).await;
 until(&mut service,"stats retry original journal drained",||std::fs::read_to_string(a.root.join("d1-observations.jsonl")).unwrap_or_default().contains("acknowledged")).await;
 pickup(&mut service,&na,json!({"kind":"large_heart"})).await;pickup(&mut service,&na,json!({"kind":"orb","id":0})).await;pickup(&mut service,&na,json!({"kind":"orb","id":128})).await;
 for _ in 0..2{pickup(&mut service,&na,json!({"kind":"steve"})).await;}
 let before_continue=counts(&mut service,&a,&[("hearts",2),("orbs",2),("steveKills",2)]).await;assert_eq!(c1_hp(&nb,"max"),remote_max);assert_eq!(c1_hp(&nb,"hp"),remote_hp);assert_eq!(c1_view(&nb)["orbCount"],0);
 service.command(json!({"op":"d1Delay","target":"consumer","durationMs":10000}));pickup(&mut service,&na,json!({"kind":"heart"})).await;
 let deadline=Instant::now()+Duration::from_secs(10);loop{service.wall_clock();if service.command(json!({"op":"d1Status"}))["pendingDelays"].as_u64().unwrap_or(0)>0{break}assert!(Instant::now()<deadline);tokio::time::sleep(Duration::from_millis(100)).await;}
 let deposit=bank_submit(&mut service,&na,"deposit",5,false).await;accepted(&mut service,&na,&deposit).await;let bank_overlap=service.command(json!({"op":"d1Status"}));until(&mut service,"confirmed completed deposit balance before deliberate TAKE",||bank_view(&na)["view"]["balance"]==5&&bank_view(&na)["wallet"]==995).await;fixture_signal::write(na.root.join("b1-outlet"),"synthetic local player at bank outlet").unwrap();let withdraw=bank_submit(&mut service,&na,"withdraw",2,false).await;accepted(&mut service,&na,&withdraw).await;until(&mut service,"completed take applied at guarded outlet",||bank_view(&na)["wallet"]==997).await;fixture_signal::remove(na.root.join("b1-outlet")).unwrap();
 fresh_bidirectional_ghosts(&mut service,&a,&na,&b,&nb,"Slow optional stats keeps bank and ghosts responsive").await;
 let spell=json!({"catalogueRevision":"nt2-items-base-2","spell":{"actionId":"LIGHT_BULLET","usesRemaining":"-1"}});item_spawn(&mut service,&na,&spell,false).await;until(&mut service,"D1 completed item deposit",||available(&nb).len()==1).await;fixture_signal::write(nb.root.join("b1-outlet"),"synthetic outlet").unwrap();item_click(&mut service,&nb).await;until(&mut service,"D1 completed item claim",||received(&nb)==1).await;
 let banked=counts(&mut service,&a,&[("hearts",3),("orbs",2),("steveKills",2),("goldDeposited",5),("goldWithdrawn",2),("itemsDeposited",1),("itemsClaimed",1)]).await;
 fixture_input_write(nb.root.join("unaffected-check"),b"healthy peer through D1 Continue").unwrap();let saved=na.root.clone();na.close();assert_peer_healthy(&b,&nb);na=Native::start(&a,Some(&saved),true);ready_owner(&mut service,&a,&na,&wa,true).await;fresh_bidirectional_ghosts(&mut service,&a,&na,&b,&nb,"D1 Continue restores fresh ghosts").await;assert_peer_healthy(&b,&nb);
 counts(&mut service,&a,&[("hearts",3),("orbs",2),("steveKills",2)]).await;pickup(&mut service,&na,json!({"kind":"heart"})).await;counts(&mut service,&a,&[("hearts",4)]).await;
 c1_rules(&mut service,&na,&nb,json!({"NT_sync_hearts":true})).await;let before_remote=c1_hp(&nb,"max");pickup(&mut service,&na,json!({"kind":"heart"})).await;until(&mut service,"shared production heart reaches peer",||c1_hp(&nb,"max")>before_remote).await;counts(&mut service,&a,&[("hearts",5)]).await;
 pickup(&mut service,&na,json!({"kind":"death"})).await;until(&mut service,"real local respawn",||c1_hp(&na,"hp")>0.0).await;counts(&mut service,&a,&[("deaths",1),("wins",0)]).await;
 fixture_input_remove(nb.root.join("unaffected-check")).unwrap();let mut end_a=expect_loss(&mut service,&na,"D1 canonical End").await;let mut end_b=expect_loss(&mut service,&nb,"D1 peer canonical End").await;end_a.before_fault(&na);end_b.before_fault(&nb);service.command(json!({"op":"end"}));
 wait(&mut service,&a,"D1 final End",20,|v|v["connection"]["state"]=="Ended"&&native_activity(&na)["native_owned_hold"]==true).await;wait(&mut service,&b,"D1 peer End",20,|v|v["connection"]["state"]=="Ended"&&native_activity(&nb)["native_owned_hold"]==true).await;end_a.protected(&na);end_b.protected(&nb);end_a.terminal(&na);end_b.terminal(&nb);
 let frozen=counts(&mut service,&a,&[("hearts",5),("orbs",2),("steveKills",2),("deaths",1),("wins",0)]).await;elapsed(&mut service,2).await;assert_eq!(read_stats(&mut service,&a).await,frozen,"End summary mutated");na.close();nb.close();
 service.command(json!({"op":"nextRun"}));let room=service.private()["room"]["roomId"].as_str().unwrap().to_owned();join_current(&a,room.clone()).await;join_current(&b,room).await;let next_a=bundle(&mut service,&a).await.0;let next_b=bundle(&mut service,&b).await.0;na=Native::start(&a,None,false);nb=Native::start(&b,None,false);ready_owner(&mut service,&a,&na,&next_a,false).await;ready_owner(&mut service,&b,&nb,&next_b,false).await;two_peer_ghost_with_start(&mut service,&a,&na,&next_a,&b,&nb,&next_b,"startReviewed").await;
 let next=counts(&mut service,&a,&[("hearts",0),("orbs",0),("steveKills",0),("deaths",0),("wins",0),("goldDeposited",0),("goldWithdrawn",0),("itemsDeposited",0),("itemsClaimed",0)]).await;assert_eq!(next["rows"].as_array().unwrap().len(),2);assert_ne!(next["runKey"],frozen["runKey"]);
 let faults=service.command(json!({"op":"d1Status"}));assert_eq!(faults["dropped"],1);assert_eq!(faults["delayed"],1);
 na.close();nb.close();a.close().await;b.close().await;host.close().await;service.close();std::fs::write(own.join("results.json"),serde_json::to_vec_pretty(&json!({"passed":true,"zero":zero,"beforeContinue":before_continue,"banked":banked,"frozen":frozen,"next":next,"faults":faults,"statsDelayAtCompletedDeposit":bank_overlap,"engineFactsInjected":true,"actualNoita":false,"ordinaryScheduler":true,"transport":"authenticated-loopback-websocket"})).unwrap()).unwrap();
})).unwrap().join().unwrap();}
