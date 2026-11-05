// Additive clean throughput comparison. Only the historical global300ms
// cooldown is reinstated in comparison arms; other current fixes stay enabled.
// This is not an exact archived Fix3 binary benchmark or WAN certification.
#[cfg(feature="g3-bank-latency-fixture")]
#[test]#[ignore="Root serialized clean bank throughput comparison"]
fn bank_clean_throughput_comparison(){
 assert_eq!(std::env::var("NT2_C3_COMPOSITION").as_deref(),Ok("1"));assert_eq!(std::env::var("NT2_ITEM_FIXTURE").as_deref(),Ok("1"));
 std::thread::Builder::new().name("bank-throughput".into()).stack_size(16*1024*1024).spawn(||tauri::async_runtime::block_on(async{
  verify_current_fixture_inputs();let own=repo().join("spikes/noita-entry/.build/bank-refresh-burst-20261025-01").join(format!("throughput-{}",uuid::Uuid::new_v4()));std::fs::create_dir_all(&own).unwrap();eprintln!("BANK_THROUGHPUT {}",own.display());
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
  let final_only=std::env::var("NT2_BANK_COMPARISON_FINAL").as_deref()==Ok("1");
  let arms=if final_only{vec![(false,0u64),(false,250)]}else{vec![(false,0u64),(true,0),(true,250),(false,250)]};
  for (index,(prior,rtt)) in arms.iter().copied().enumerate(){
   until(&mut service,"same empty visible bank before each comparison",||available(&nb).is_empty()&&items(&nb)["view"]["fresh"]==true).await;
   nt2_desktop_g0::g3_http::bank_timing::arm_clean(origin.clone(),principals.clone(),rtt,prior);
   let started=Instant::now();let deadline=started+Duration::from_secs(90);let host_root=na.root.clone();let peer_root=nb.root.clone();let expected=(index+1)*3;
   // Independent observer records first ready time even while the main driver
   // is operating the concurrent gold UI. No gold/probe time is added to it.
   let observer=tokio::spawn(async move{
    let mut first_visible=serde_json::Map::new();let mut outcomes=serde_json::Map::new();
    loop{
     let load=|root:&std::path::Path|fixture_signal::read(root.join("items-view.json")).unwrap().and_then(|v|serde_json::from_str::<Value>(&v).ok()).unwrap_or(Value::Null);
     let h=load(&host_root);let p=load(&peer_root);let rows=p["bank"]["items"].as_array().cloned().unwrap_or_default();let visible:Vec<_>=rows.into_iter().filter(|r|r["state"]=="available").collect();
     for row in &visible{if let Some(id)=row["itemId"].as_str(){first_visible.entry(id.to_owned()).or_insert(json!(started.elapsed().as_millis()));}}
     for e in h["effectOutcomes"].as_array().into_iter().flatten(){if let Some(id)=e["id"].as_str(){let key=format!("{}:{}",id,e["state"]);outcomes.entry(key).or_insert(json!({"observedMs":started.elapsed().as_millis(),"event":e}));}}
     if visible.len()==3&&h["kills"]==expected&&h["aliveOriginals"].as_array().is_some_and(Vec::is_empty){return json!({"burstReadyMs":started.elapsed().as_millis(),"itemFirstVisibleMs":first_visible,"sourceOutcomesFirstObserved":outcomes,"hostRemovalHistory":h["removalHistory"],"peerBankBytes":serde_json::to_vec(&p["bank"]).unwrap().len(),"timingAtReady":nt2_desktop_g0::g3_http::bank_timing::snapshot()})}
     assert!(Instant::now()<deadline,"Clean three-item ready deadline");tokio::time::sleep(Duration::from_millis(20)).await;
    }
   });
   fixture_signal::write(na.root.join("items-spawn-burst"),serde_json::to_vec(&vec![sample.clone(),sample.clone(),sample.clone()]).unwrap()).unwrap();
   fixture_signal::write(nb.root.join("b1-outlet"),"injected mixed gold outlet").unwrap();
   let deposit=bank_submit(&mut service,&na,"deposit",5,false).await;
   let withdrawal=bank_submit(&mut service,&nb,"withdraw",5,false).await;
   bank_burst_until(&mut service,"independent ready observer",deadline,||observer.is_finished()).await;let measured=observer.await.unwrap();
   accepted(&mut service,&na,&deposit).await;accepted(&mut service,&nb,&withdrawal).await;
   bank_burst_until(&mut service,"mixed gold completes separately from burst clock",deadline,||bank_view(&na)["wallet"]==(990-(index+1)*5)as u64&&bank_view(&nb)["wallet"]==(1000+(index+1)*5)as u64&&bank_view(&nb)["view"]["balance"]==10).await;
   let timing=nt2_desktop_g0::g3_http::bank_timing::snapshot();assert!(timing["events"].as_array().unwrap().iter().all(|e|e["stallMs"]==0&&e["jitterMs"]==0));
   let result=json!({"priorCooldownOnly":prior,"extraRttMs":rtt,"burst":measured,"goldCompletionMs":started.elapsed().as_millis(),"wholeWindowTiming":timing});results.push(result);
   std::fs::write(own.join("comparison-progress.json"),serde_json::to_vec_pretty(&results).unwrap()).unwrap();nt2_desktop_g0::g3_http::bank_timing::disarm();
   // Drain with real claims outside the measured window, keeping visible-page
   // size constant and all original/receipt histories intact for later arms.
   for item in 0..3{item_click(&mut service,&nb).await;until(&mut service,"untimed claimed item delivered once",||received(&nb)==index*3+item+1&&items(&nb)["view"]["queued"]==0).await;}
   fixture_signal::remove(nb.root.join("b1-outlet")).unwrap();until(&mut service,"comparison bank drained",||available(&nb).is_empty()).await;
   assert_eq!(items(&na)["kills"],expected);fresh_bidirectional_ghosts(&mut service,&a,&na,&b,&nb,"clean comparison movement").await;
  }
  let host=expect_loss(&mut service,&na,"throughput End host").await;let peer=expect_loss(&mut service,&nb,"throughput End peer").await;item_end_pair(&mut service,[(&a,&na,&wa,&host),(&b,&nb,&wb,&peer)]).await;
  na.close();nb.close();a.close().await;b.close().await;service.close();
  std::fs::write(own.join("checks.json"),serde_json::to_vec_pretty(&json!({"passed":true,"actualComponents":true,"cleanExtraRttMs":[0,250],"jitter":false,"stalls":false,"lostReply":false,"burstItemsPerArm":3,"arms":arms.len(),"mixedGold":true,"sourceRemovals":arms.len()*3,"received":arms.len()*3,"comparisonBoundary":"Historical Fix3 global300ms cooldown only; other current fixes including due capture cadence remain enabled. Not exact archived binary.","results":results,"engineFactsInjected":true,"wanOrCapacityCertification":false})).unwrap()).unwrap();
 })).unwrap().join().unwrap();
}
