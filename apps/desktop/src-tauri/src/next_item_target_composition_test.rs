#[cfg(feature="g3-bank-latency-fixture")]
#[test]#[ignore="Root serialized two-player bank burst/refresh/latency/outage composition"]
fn bank_slot_hover_claim_ordinary_scheduler_composition(){
 assert_eq!(std::env::var("NT2_C3_COMPOSITION").as_deref(),Ok("1"));assert_eq!(std::env::var("NT2_ITEM_FIXTURE").as_deref(),Ok("1"));
 std::thread::Builder::new().name("bank-latency".into()).stack_size(16*1024*1024).spawn(||tauri::async_runtime::block_on(async{
  verify_current_fixture_inputs();let own=repo().join("spikes/noita-entry/.build/bank-refresh-burst-20261025-01").join(format!("target-{}",uuid::Uuid::new_v4()));std::fs::create_dir_all(&own).unwrap();eprintln!("BANK_TARGET_COMPOSITION {}",own.display());
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
  let sample=json!({"catalogueRevision":"nt2-items-base-2","spell":{"actionId":"LIGHT_BULLET","usesRemaining":"5"}});
  fixture_signal::write(na.root.join("items-spawn-burst"),serde_json::to_vec(&vec![sample.clone(),sample.clone(),sample.clone()]).unwrap()).unwrap();
  until(&mut service,"three items deposited through unchanged production capture",||available(&nb).len()==3&&items(&na)["kills"]==3).await;
  click(&mut service,&nb,"Gold bank").await;until(&mut service,"matching cached rows stay usable during refresh",||items(&nb)["bank"]["items"].as_array().is_some_and(|v|v.len()==3)).await;
  let rows=items(&nb)["bank"]["items"].as_array().unwrap().clone();let target=rows[2]["itemId"].as_str().unwrap().to_owned();
  nt2_desktop_g0::g3_http::bank_timing::arm(origin,principals,250);
  fixture_signal::write(nb.root.join("items-cell-hover"),serde_json::to_vec(&json!({"x":210,"y":160,"label":""})).unwrap()).unwrap();
  until(&mut service,"real hover detail request admitted",||nt2_desktop_g0::g3_http::bank_timing::snapshot()["events"].as_array().unwrap().iter().any(|v|v["kind"]=="itemInspect")).await;
  let copied=||fixture_signal::read(nb.root.join("items-last-claim.json")).unwrap().and_then(|v|serde_json::from_str::<Value>(&v).ok()).unwrap_or(Value::Null);
  let started=Instant::now();let gesture=json!({"x":266,"y":160,"label":"","itemId":target});fixture_signal::write(nb.root.join("items-cell-click"),serde_json::to_vec(&gesture).unwrap()).unwrap();
  until(&mut service,"full-slot physical click copies claim behind existing detail",||copied()["kind"]=="item_claim"&&copied()["item_id"]==target).await;
  let copied_ms=started.elapsed().as_millis();let original=copied()["id"].as_str().unwrap().to_owned();
  // Repeat the same physical gesture while the original is pending. The row
  // must not mint a second claim, even if a newer bank revision arrives.
  fixture_signal::write(nb.root.join("items-cell-click"),serde_json::to_vec(&gesture).unwrap()).unwrap();
  elapsed(&mut service,1).await;fixture_signal::remove(nb.root.join("items-cell-click")).unwrap();fixture_signal::remove(nb.root.join("items-cell-hover")).unwrap();click(&mut service,&nb,"Close").await;
  fixture_signal::write(nb.root.join("b1-outlet"),"injected owned outlet").unwrap();
  until(&mut service,"one exact claimed item reconstructed",||received(&nb)==1&&items(&nb)["view"]["queued"]==0).await;
  let completed_ms=started.elapsed().as_millis();elapsed(&mut service,1).await;fixture_signal::remove(nb.root.join("items-cell-click")).unwrap();fixture_signal::remove(nb.root.join("items-cell-hover")).unwrap();
  let originals=b.handle.command(|c|Box::pin(async move{Ok(json!(c.g3.as_ref().unwrap().consumer.journal.pending().filter(|p|p.record.original.direction=="claim").map(|p|p.record.original.operation_id).collect::<Vec<_>>()))})).await.unwrap();let claims=originals.as_array().unwrap();assert_eq!(claims.len(),1,"Hover/copy/refresh/repeated gesture cannot create another claim original");assert_eq!(claims[0],original);assert_eq!(available(&nb).len(),2);assert_eq!(received(&nb),1);
  fresh_bidirectional_ghosts(&mut service,&a,&na,&b,&nb,"hover and claim preserve movement").await;
  let timing=nt2_desktop_g0::g3_http::bank_timing::snapshot();let reads=timing["events"].as_array().unwrap().iter().filter(|e|e["kind"]=="itemInspect").count();assert!(reads<=2,"A stable hover cannot flood detail RPCs");nt2_desktop_g0::g3_http::bank_timing::disarm();
  let host=expect_loss(&mut service,&na,"target End host").await;let peer=expect_loss(&mut service,&nb,"target End peer").await;item_end_pair(&mut service,[(&a,&na,&wa,&host),(&b,&nb,&wb,&peer)]).await;
  na.close();nb.close();a.close().await;b.close().await;service.close();
  std::fs::write(own.join("checks.json"),serde_json::to_vec_pretty(&json!({"passed":true,"actualComponents":true,"actualNoita":false,"engineFactsInjected":true,"physicalFullSlotClaim":true,"differentHoveredItemCannotChangeSelectedItem":true,"oneClaimOriginal":true,"oneReconstruction":true,"cachedRowsDuringRefresh":true,"detailReads":reads,"clickCopiedMs":copied_ms,"claimCompletedMs":completed_ms,"timing":timing})).unwrap()).unwrap();
 })).unwrap().join().unwrap();
}
