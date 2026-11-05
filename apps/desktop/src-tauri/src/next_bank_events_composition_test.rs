// Event/batch three-item comparison against accepted source measurements.
// Actual components; injected engine and once-after-body extra RTT.
#[cfg(feature="g3-bank-latency-fixture")]
#[test]#[ignore="Root serialized event bank composition"]
fn bank_events_ordinary_scheduler_composition(){
 assert_eq!(std::env::var("NT2_C3_COMPOSITION").as_deref(),Ok("1"));assert_eq!(std::env::var("NT2_ITEM_FIXTURE").as_deref(),Ok("1"));
 std::thread::Builder::new().name("bank-throughput".into()).stack_size(16*1024*1024).spawn(||tauri::async_runtime::block_on(async{
  verify_current_fixture_inputs();let own=repo().join("spikes/noita-entry/.build/bank-refresh-burst-20261025-01").join(format!("events-{}",uuid::Uuid::new_v4()));std::fs::create_dir_all(&own).unwrap();eprintln!("BANK_THROUGHPUT {}",own.display());
  struct Last(std::path::PathBuf);impl Drop for Last{fn drop(&mut self){let _=std::fs::write(&self.0,serde_json::to_vec_pretty(&nt2_desktop_g0::g3_http::bank_timing::snapshot()).unwrap());}}let _last=Last(own.join("last-timing.json"));
  let mut service=Service::start(&own);network_profile::enable_elapsed_clock(&mut service,&own);let router=PipeHarness::new();
  let mut a=Client::start(&service.private(),&own.join("host"),None,false,true,router.routes.clone(),service.watches.clone(),service.stores.clone()).await;bundle(&mut service,&a).await;let mut na=Native::start(&a,None,false);
  wait(&mut service,&a,"throughput host Ready",30,|v|current_state_is(v,"Ready")&&na.root.join("associated").is_file()).await;let wa=a.handle.view()["rejoin"]["current_world"].clone();
  service.command(json!({"op":"peer"}));let mut b=Client::start(&service.private(),&own.join("peer"),None,true,true,router.routes.clone(),service.watches.clone(),service.stores.clone()).await;bundle(&mut service,&b).await;let mut nb=Native::start(&b,None,false);
  wait(&mut service,&b,"throughput peer Ready",30,|v|current_state_is(v,"Ready")&&nb.root.join("associated").is_file()).await;let wb=b.handle.view()["rejoin"]["current_world"].clone();two_peer_ghost_with_start(&mut service,&a,&na,&wa,&b,&nb,&wb,"startReviewed").await;
  until(&mut service,"clean banks ready",||items(&na)["view"]["fresh"]==true&&items(&nb)["view"]["fresh"]==true&&bank_view(&na)["view"]["fresh"]==true).await;
  let seed=bank_submit(&mut service,&na,"deposit",10,false).await;accepted(&mut service,&na,&seed).await;until(&mut service,"seeded gold before timed workload",||bank_view(&na)["view"]["balance"]==10&&bank_view(&na)["wallet"]==990&&bank_view(&nb)["view"]["balance"]==10).await;
  let mut identities=Vec::new();for client in [&a,&b]{identities.push(client.handle.command(|c|Box::pin(async move{let v=c.g3.as_ref().unwrap().bank_context().unwrap();Ok(json!({"origin":v.binding.service_origin,"principal":v.binding.principal_id}))})).await.unwrap());}
  let origin=identities[0]["origin"].as_str().unwrap().to_owned();let principals=identities.iter().map(|v|v["principal"].as_str().unwrap().to_owned()).collect::<Vec<_>>();
  let sample=json!({"catalogueRevision":"nt2-items-base-2","spell":{"actionId":"LIGHT_BULLET","usesRemaining":"-1"}});let mut results=Vec::new();
  let final_only=true;
  let arms=if final_only{vec![(false,0u64),(false,250)]}else{vec![(false,0u64),(true,0),(true,250),(false,250)]};
  for (index,(prior,rtt)) in arms.iter().copied().enumerate(){
   until(&mut service,"same empty visible bank before each comparison",||available(&nb).is_empty()&&items(&nb)["view"]["fresh"]==true).await;
   nt2_desktop_g0::g3_http::bank_timing::arm_clean(origin.clone(),principals.clone(),rtt,prior);
   let started=Instant::now();let deadline=started+Duration::from_secs(90);let host_root=na.root.clone();let peer_root=nb.root.clone();let expected=(index+1)*3;
   // Independent observer records first ready time even while the main driver
   // is operating the concurrent gold UI. No gold/probe time is added to it.
   let observer=tokio::spawn(async move{
    let mut first_visible=serde_json::Map::new();let mut outcomes=serde_json::Map::new();let mut burst_ready=None;let mut ready_timing=Value::Null;let mut first_gold=None;let mut all_gold=None;let mut last_motion=0;let mut motion_at=0u128;let mut max_motion_gap=0u128;let mut max_uploads=0u64;
    loop{
     let load=|root:&std::path::Path|fixture_signal::read(root.join("items-view.json")).unwrap().and_then(|v|serde_json::from_str::<Value>(&v).ok()).unwrap_or(Value::Null);
     let elapsed=started.elapsed().as_millis();
     let load_file=|root:&std::path::Path,file:&str|fixture_signal::read(root.join(file)).unwrap().and_then(|v|serde_json::from_str::<Value>(&v).ok()).unwrap_or(Value::Null);
     let hg=load_file(&host_root,"b1-view.json");let pg=load_file(&peer_root,"b1-view.json");
     let host_done=hg["wallet"]==json!(990-(index+1)*5);let peer_done=pg["wallet"]==json!(1000+(index+1)*5);
     if (host_done||peer_done)&&first_gold.is_none(){first_gold=Some(elapsed)}if host_done&&peer_done&&all_gold.is_none(){all_gold=Some(elapsed)}
     let activity=load_file(&peer_root,"activity.json");if let Some(motion)=activity["ghost_motion_updates"].as_u64(){if motion>last_motion{max_motion_gap=max_motion_gap.max(elapsed-motion_at);motion_at=elapsed;last_motion=motion;}}
     let h=load(&host_root);let p=load(&peer_root);max_uploads=max_uploads.max(h["uploads"].as_u64().unwrap_or(0));let rows=p["bank"]["items"].as_array().cloned().unwrap_or_default();let visible:Vec<_>=rows.into_iter().filter(|r|r["state"]=="available").collect();
     for row in &visible{if let Some(id)=row["itemId"].as_str(){first_visible.entry(id.to_owned()).or_insert(json!(started.elapsed().as_millis()));}}
     for e in h["effectOutcomes"].as_array().into_iter().flatten(){if let Some(id)=e["id"].as_str(){let key=format!("{}:{}",id,e["state"]);outcomes.entry(key).or_insert(json!({"observedMs":started.elapsed().as_millis(),"event":e}));}}
     if burst_ready.is_none()&&visible.len()==3&&h["kills"]==expected&&h["aliveOriginals"].as_array().is_some_and(Vec::is_empty){burst_ready=Some(elapsed);ready_timing=nt2_desktop_g0::g3_http::bank_timing::snapshot();}
     if burst_ready.is_some()&&all_gold.is_some(){return json!({"firstGoldCompletionMs":first_gold,"allGoldCompletionMsAtBurst":all_gold,"maximumObservedMotionCounterGapMs":max_motion_gap,"maxQueuedUploads":max_uploads,"burstReadyMs":burst_ready,"itemFirstVisibleMs":first_visible,"sourceOutcomesFirstObserved":outcomes,"hostRemovalHistory":h["removalHistory"],"peerBankBytes":serde_json::to_vec(&p["bank"]).unwrap().len(),"timingAtReady":ready_timing})}
     assert!(Instant::now()<deadline,"Clean three-item ready deadline");tokio::time::sleep(Duration::from_millis(20)).await;
    }
   });
   fixture_signal::write(na.root.join("items-spawn-burst"),serde_json::to_vec(&vec![sample.clone(),sample.clone(),sample.clone()]).unwrap()).unwrap();
   fixture_signal::write(nb.root.join("b1-outlet"),"injected mixed gold outlet").unwrap();
   service.command(json!({"op":"chat","target":"host","text":format!("bank-events-arm-{index}")}));
   let deposit=bank_submit(&mut service,&na,"deposit",5,false).await;
   let withdrawal=bank_submit(&mut service,&nb,"withdraw",5,false).await;
   bank_burst_until(&mut service,"independent ready observer",deadline,||observer.is_finished()).await;let measured=observer.await.unwrap();
   accepted(&mut service,&na,&deposit).await;accepted(&mut service,&nb,&withdrawal).await;
   bank_burst_until(&mut service,"mixed gold completes separately from burst clock",deadline,||bank_view(&na)["wallet"]==(990-(index+1)*5)as u64&&bank_view(&nb)["wallet"]==(1000+(index+1)*5)as u64&&bank_view(&nb)["view"]["balance"]==10).await;
   until(&mut service,"chat remains responsive under bank load",||text(&nb,"a2-notices.txt").contains(&format!("bank-events-arm-{index}"))).await;
   let timing=nt2_desktop_g0::g3_http::bank_timing::snapshot();assert!(timing["events"].as_array().unwrap().iter().all(|e|e["stallMs"]==0&&e["jitterMs"]==0));
   let mut visibility=Vec::new();let mut bytes_sent=0u64;let mut bytes_received=0u64;let mut reads=0;let mut capacity=0;
   for event in measured["timingAtReady"]["events"].as_array().unwrap(){
    bytes_sent+=event["requestBytes"].as_u64().unwrap_or(0);bytes_received+=event["responseBytes"].as_u64().unwrap_or(0);
    if event["kind"]=="itemBank"||event["kind"]=="goldBank"{reads+=1}if event["responseError"]=="CAPACITY"{capacity+=1}
    capacity+=event["entryErrors"].as_array().into_iter().flatten().filter(|v|**v=="CAPACITY").count();
    if event["kind"]!="itemReceipt"{continue}
    for op in event["itemResults"].as_array().into_iter().flatten().filter(|op|op["state"]=="completed"){
     if let Some(visible)=op["itemId"].as_str().and_then(|id|measured["itemFirstVisibleMs"][id].as_u64()){
      visibility.push(json!({"itemId":op["itemId"],"lowerMs":visible.saturating_sub(event["tlsBodyReadMs"].as_u64().unwrap()),"upperMs":visible.saturating_sub(event["startedMs"].as_u64().unwrap()),"boundary":"Actual commit lies between actual HTTP request start and complete TLS body read; bounds, not exact commit timestamp. Observer resolution20ms."}));
     }
    }
   }
   assert!(measured["maxQueuedUploads"].as_u64().unwrap()<=4);
   let result=json!({"visibilityBounds":visibility,"rpcJsonBytesAtReady":{"sent":bytes_sent,"received":bytes_received},"bankReadsAtReady":reads,"capacityAtReady":capacity,"priorCooldownOnly":prior,"extraRttMs":rtt,"burst":measured,"goldCompletionMs":started.elapsed().as_millis(),"wholeWindowTiming":timing});results.push(result);
   std::fs::write(own.join("comparison-progress.json"),serde_json::to_vec_pretty(&results).unwrap()).unwrap();nt2_desktop_g0::g3_http::bank_timing::disarm();
   // Drain with real claims outside the measured window, keeping visible-page
   // size constant and all original/receipt histories intact for later arms.
   for item in 0..3{item_click(&mut service,&nb).await;until(&mut service,"untimed claimed item delivered once",||received(&nb)==index*3+item+1&&items(&nb)["view"]["queued"]==0).await;}
   fixture_signal::remove(nb.root.join("b1-outlet")).unwrap();until(&mut service,"comparison bank drained",||available(&nb).is_empty()).await;
   assert_eq!(items(&na)["kills"],expected);fresh_bidirectional_ghosts(&mut service,&a,&na,&b,&nb,"clean comparison movement").await;
  }
  // Healthy event-connected idle must not reintroduce the old2s SQL polling.
  elapsed(&mut service,3).await;nt2_desktop_g0::g3_http::bank_timing::arm_clean(origin.clone(),principals.clone(),0,false);
  elapsed(&mut service,6).await;let idle=nt2_desktop_g0::g3_http::bank_timing::snapshot();
  assert!(idle["events"].as_array().unwrap().iter().all(|e|e["kind"]!="goldBank"&&e["kind"]!="itemBank"),"Healthy idle banks must not poll");
  std::fs::write(own.join("idle-timing.json"),serde_json::to_vec_pretty(&idle).unwrap()).unwrap();nt2_desktop_g0::g3_http::bank_timing::disarm();
  let host=expect_loss(&mut service,&na,"throughput End host").await;let peer=expect_loss(&mut service,&nb,"throughput End peer").await;item_end_pair(&mut service,[(&a,&na,&wa,&host),(&b,&nb,&wb,&peer)]).await;
  na.close();nb.close();a.close().await;b.close().await;service.close();
  std::fs::write(own.join("checks.json"),serde_json::to_vec_pretty(&json!({"passed":true,"actualComponents":true,"cleanExtraRttMs":[0,250],"jitter":false,"stalls":false,"lostReply":false,"burstItemsPerArm":3,"arms":arms.len(),"mixedGold":true,"sourceRemovals":arms.len()*3,"received":arms.len()*3,"comparisonBoundary":"Accepted source historical single observations:11286ms local/15991ms250ms; current event/batch path. No exact Fix3 or statistical comparison.","results":results,"engineFactsInjected":true,"wanOrCapacityCertification":false})).unwrap()).unwrap();
 })).unwrap().join().unwrap();
}
