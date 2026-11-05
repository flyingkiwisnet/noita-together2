#[cfg(feature="g3-bank-latency-fixture")]
#[test]#[ignore="Root serialized two-player bank burst/refresh/latency/outage composition"]
fn bank_events_fault_recovery_ordinary_scheduler_composition(){
 assert_eq!(std::env::var("NT2_C3_COMPOSITION").as_deref(),Ok("1"));assert_eq!(std::env::var("NT2_ITEM_FIXTURE").as_deref(),Ok("1"));
 std::thread::Builder::new().name("bank-latency".into()).stack_size(16*1024*1024).spawn(||tauri::async_runtime::block_on(async{
  verify_current_fixture_inputs();let own=repo().join("spikes/noita-entry/.build/bank-refresh-burst-20261025-01").join(format!("events-fault-{}",uuid::Uuid::new_v4()));std::fs::create_dir_all(&own).unwrap();eprintln!("BANK_LATENCY_COMPOSITION {}",own.display());
  struct TimingEvidence(std::path::PathBuf);impl Drop for TimingEvidence{fn drop(&mut self){let _=std::fs::write(&self.0,serde_json::to_vec_pretty(&nt2_desktop_g0::g3_http::bank_timing::snapshot()).unwrap());}}
  let _timing_evidence=TimingEvidence(own.join("last-latency-timing.json"));
  let mut service=Service::start(&own);network_profile::enable_elapsed_clock(&mut service,&own);let router=PipeHarness::new();
  let mut a=Client::start(&service.private(),&own.join("host"),None,false,true,router.routes.clone(),service.watches.clone(),service.stores.clone()).await;bundle(&mut service,&a).await;let mut na=Native::start(&a,None,false);
  wait(&mut service,&a,"latency host Ready",30,|v|current_state_is(v,"Ready")&&na.root.join("associated").is_file()).await;let wa=a.handle.view()["rejoin"]["current_world"].clone();
  service.command(json!({"op":"peer"}));let mut b=Client::start(&service.private(),&own.join("peer"),None,true,true,router.routes.clone(),service.watches.clone(),service.stores.clone()).await;bundle(&mut service,&b).await;let mut nb=Native::start(&b,None,false);
  wait(&mut service,&b,"latency peer Ready",30,|v|current_state_is(v,"Ready")&&nb.root.join("associated").is_file()).await;let wb=b.handle.view()["rejoin"]["current_world"].clone();two_peer_ghost_with_start(&mut service,&a,&na,&wa,&b,&nb,&wb,"startReviewed").await;
  until(&mut service,"bank initially confirmed without any original",||bank_view(&na)["view"]["fresh"]==true&&items(&na)["view"]["fresh"]==true&&items(&nb)["view"]["fresh"]==true).await;
  // Confirm the actual UI query before the delayed reopen check. Initial
  // worker query{} is different and must never be displayed as these filters.
  for native in [&na,&nb]{click(&mut service,native,"Gold bank").await;until(&mut service,"confirmed cached UI query",||items(native)["loading"]==false&&items(native)["bank"]["query"]["page"]==0).await;click(&mut service,native,"Close").await;}
  assert!(current_originals(&a).is_empty());assert!(!bank_view(&na)["view"]["status"].as_str().unwrap_or("").contains("Recovering"));
  let mut identities=Vec::new();for client in [&a,&b]{identities.push(client.handle.command(|c|Box::pin(async move{let context=c.g3.as_ref().unwrap().bank_context().unwrap();Ok(json!({"origin":context.binding.service_origin,"principal":context.binding.principal_id}))})).await.unwrap());}
  let origin=identities[0]["origin"].as_str().unwrap().to_owned();let principals=identities.iter().map(|v|v["principal"].as_str().unwrap().to_owned()).collect::<Vec<_>>();
  let sample=json!({"catalogueRevision":"nt2-items-base-2","spell":{"actionId":"LIGHT_BULLET","usesRemaining":"-1"}});let mut metrics=Vec::new();
  for (index,rtt) in [250u64,1000].into_iter().enumerate(){
   nt2_desktop_g0::g3_http::bank_timing::arm(origin.clone(),principals.clone(),rtt);let started=Instant::now();let deadline=started+Duration::from_secs(120);let before=native_activity(&nb);let previous=index*2;
   let first=item_spawn(&mut service,&na,&sample,true).await;
   if index==0{service.command(json!({"op":"goldArm","plan":{"target":"consumer","operationId":first,"kind":"itemDeposit","effect":"drop"}}));}
   fixture_signal::write(na.root.join("items-spawn-burst"),serde_json::to_vec(&vec![sample.clone(),sample.clone()]).unwrap()).unwrap();fixture_signal::remove(na.root.join("items-copy-block")).unwrap();
   let gold=bank_submit(&mut service,&na,"deposit",10,false).await;
   click(&mut service,&nb,"Gold bank").await;elapsed(&mut service,2).await;assert!(items(&nb)["bank"].is_object(),"Opening during delayed refresh retains the confirmed page");assert!(available(&nb).len()>=previous,"Delayed refresh cannot erase existing rows");click(&mut service,&nb,"Close").await;
   fresh_bidirectional_ghosts(&mut service,&a,&na,&b,&nb,"bank burst RTT movement").await;
   let mut refreshing_seen=false;let mut samples=Vec::new();
   bank_burst_until(&mut service,"three burst deposits become available alongside gold",deadline,||{
    let h=items(&na);let p=items(&nb);let rows=available(&nb).len();assert!(rows>=previous,"Confirmed rows disappear while waiting for refresh");
    refreshing_seen|=p["view"]["refresh_status"].as_str().is_some_and(|s|s.contains("Refreshing"));
    if samples.len()<64{samples.push(json!({"elapsedMs":started.elapsed().as_millis(),"peerRows":rows,"fresh":p["view"]["fresh"],"refresh":p["view"]["refresh_status"],"hostQueuedCopies":h["uploads"]}));}
    rows==previous+3&&bank_view(&na)["view"]["balance"]==((index+1)*5+5)as u64&&h["aliveOriginals"].as_array().is_some_and(Vec::is_empty)
   }).await;
   let burst_ms=started.elapsed().as_millis();accepted(&mut service,&na,&gold).await;
   let withdrawal=bank_submit(&mut service,&nb,"withdraw",5,false).await;accepted(&mut service,&nb,&withdrawal).await;fixture_signal::write(nb.root.join("b1-outlet"),"injected authorized outlet").unwrap();
   bank_burst_until(&mut service,"peer gold completes during delayed item work",deadline,||bank_view(&nb)["wallet"]==(1000+(index+1)*5)as u64).await;
   item_click(&mut service,&nb).await;bank_burst_until(&mut service,"one claim reconstructed exactly once",deadline,||received(&nb)==index+1&&items(&nb)["view"]["queued"]==0).await;
   fixture_signal::remove(nb.root.join("b1-outlet")).unwrap();elapsed(&mut service,1).await;assert_eq!(received(&nb),index+1);assert_eq!(items(&na)["kills"],(index+1)*3);assert!(native_activity(&nb)["ghost_motion_updates"].as_u64().unwrap()>before["ghost_motion_updates"].as_u64().unwrap());
   fresh_bidirectional_ghosts(&mut service,&a,&na,&b,&nb,"bank completed RTT movement").await;
   let timing=nt2_desktop_g0::g3_http::bank_timing::snapshot();let count=timing["events"].as_array().unwrap().len();assert!(count>0&&count<160,"Bounded measured optional request count");
   metrics.push(json!({"injectedRttMs":rtt,"jitterMs":[-40,20,0,40,-20],"occasionalStallMs":2400,"burstItems":3,"burstCompletionMs":burst_ms,"wholePhaseMs":started.elapsed().as_millis(),"optionalResponses":count,"refreshingSeen":refreshing_seen,"samples":samples,"timing":timing}));
   std::fs::write(own.join("latency-progress.json"),serde_json::to_vec_pretty(&metrics).unwrap()).unwrap();
   nt2_desktop_g0::g3_http::bank_timing::disarm();
  }
  let fault=service.command(json!({"op":"goldStatus"}));assert!(fault["events"].as_array().unwrap().iter().any(|e|e["kind"]=="itemDeposit"&&e["effect"]=="drop"&&e["committed"]==true),"Real committed deposit reply must be lost");
  let originals=a.handle.command(|c|Box::pin(async move{let rows:Vec<_>=c.g3.as_ref().unwrap().consumer.journal.pending().filter(|p|p.record.original.item.is_some()&&p.record.original.direction=="deposit").map(|p|p.record.original.operation_id).collect();Ok(json!(rows))})).await.unwrap();assert_eq!(originals.as_array().unwrap().len(),6,"Lost or late replies never create another original");
  // One uncertain engine removal must not discard or block successful siblings.
  fixture_signal::write(na.root.join("items-removal-stuck-once"),"injected only first deletion in group stuck").unwrap();
  fixture_signal::write(na.root.join("items-spawn-burst"),serde_json::to_vec(&vec![sample.clone(),sample.clone(),sample.clone()]).unwrap()).unwrap();
  until(&mut service,"partial group siblings complete independently",||items(&na)["kills"]==9&&available(&nb).len()==6).await;
  assert_eq!(items(&na)["aliveOriginals"].as_array().unwrap().len(),1,"Uncertain original retained; siblings removed once");
  assert!(items(&na)["effectOutcomes"].as_array().unwrap().iter().any(|e|e["state"]=="uncertain"&&e["reason"]=="removal_timeout"));
  let kills=items(&na)["kills"].clone();let before_received=received(&nb);let mut outage=expect_loss(&mut service,&na,"bank latency genuine outage").await;outage.before_fault(&na);
  for kind in ["http","wss"]{service.command(json!({"op":"transport","kind":kind,"enabled":true,"target":"consumer"}));}
  until(&mut service,"real outage protects bank owner",||native_activity(&na)["native_owned_hold"]==true).await;outage.protected(&na);assert_eq!(items(&na)["kills"],kills);assert_eq!(received(&nb),before_received);assert_peer_healthy(&b,&nb);
  for kind in ["http","wss"]{service.command(json!({"op":"transport","kind":kind,"enabled":false,"target":"consumer"}));}ready_owner(&mut service,&a,&na,&wa,true).await;outage.rearm(&mut service,&na).await;
  until(&mut service,"same bank rows recover without duplicate effects",||items(&na)["view"]["fresh"]==true&&available(&nb).len()==6).await;assert_eq!(items(&na)["kills"],kills);assert_eq!(received(&nb),2);
  std::fs::write(own.join("latency-metrics.json"),serde_json::to_vec_pretty(&json!({"profiles":metrics,"lostCommittedReply":fault,"sixDepositOriginals":originals,"engineFactsInjected":true,"extraRttOncePerActualTlsResponse":true,"wanOrCapacityCertification":false})).unwrap()).unwrap();
  let host=expect_loss(&mut service,&na,"latency End host").await;let peer=expect_loss(&mut service,&nb,"latency End peer").await;item_end_pair(&mut service,[(&a,&na,&wa,&host),(&b,&nb,&wb,&peer)]).await;
  na.close();nb.close();a.close().await;b.close().await;service.close();
  std::fs::write(own.join("checks.json"),serde_json::to_vec_pretty(&json!({"passed":true,"actualComponents":true,"injectedRttMs":[250,1000],"jitterAndStalls":true,"burstDeposits":6,"partialAdditionalOriginals":3,"uncertainOriginalRetained":true,"itemClaims":2,"simultaneousGold":true,"lostReplySameOriginal":true,"realOutageRecovery":true,"freshMovement":true,"noDuplicateEffects":true,"engineFactsInjected":true,"actualNoita":false,"wanOrCapacityCertification":false})).unwrap()).unwrap();
 })).unwrap().join().unwrap();
}
