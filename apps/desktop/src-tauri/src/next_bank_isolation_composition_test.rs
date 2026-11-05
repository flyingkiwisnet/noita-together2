#[cfg(feature="g3-bank-latency-fixture")]
#[test]#[ignore="Root serialized two-player bank burst/refresh/latency/outage composition"]
fn bank_isolation_source_wait_ordinary_scheduler_composition(){
 assert_eq!(std::env::var("NT2_C3_COMPOSITION").as_deref(),Ok("1"));assert_eq!(std::env::var("NT2_ITEM_FIXTURE").as_deref(),Ok("1"));
 std::thread::Builder::new().name("bank-latency".into()).stack_size(16*1024*1024).spawn(||tauri::async_runtime::block_on(async{
  verify_current_fixture_inputs();let own=repo().join("spikes/noita-entry/.build/bank-refresh-burst-20261025-01").join(format!("isolation-{}",uuid::Uuid::new_v4()));std::fs::create_dir_all(&own).unwrap();eprintln!("BANK_ISOLATION_COMPOSITION {}",own.display());
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
  let history=a.handle.command(|c|Box::pin(async move{
   let mut consumer=c.g3.as_ref().unwrap().consumer.clone();let mut foreign=c.g3.as_ref().unwrap().bank_context().unwrap().binding;foreign.context=*uuid::Uuid::new_v4().as_bytes();foreign.scope.room_id=Some(uuid::Uuid::new_v4().to_string());foreign.scope.run_id=Some(uuid::Uuid::new_v4().to_string());foreign.scope.world_token=Some(uuid::Uuid::new_v4().to_string());consumer.bind(foreign)?;
   let gold=uuid::Uuid::new_v4().to_string();consumer.begin_gold_with_id("deposit",7,consumer.generation(),gold.clone())?;
   let item=uuid::Uuid::new_v4().to_string();consumer.begin_item_with_id(nt2_desktop_g0::g3_items::Original{kind:"claim".into(),payload_json:json!({"itemId":uuid::Uuid::new_v4().to_string(),"expectedRevision":"1"}).to_string()},consumer.generation(),item.clone())?;Ok(json!([gold,item]))
  })).await.unwrap();
  let sample=json!({"catalogueRevision":"nt2-items-base-2","spell":{"actionId":"LIGHT_BULLET","usesRemaining":"-1"}});let mut results=Vec::new();
  nt2_desktop_g0::g3_http::bank_timing::arm(origin,principals,250);
  for(index,stage)in ["captured","prepared"].into_iter().enumerate(){
   fixture_signal::remove(na.root.join("items-blocked.json")).unwrap();
   fixture_signal::write(na.root.join("items-source-block"),serde_json::to_vec(&json!({"ordinal":index*3+2,"stage":stage})).unwrap()).unwrap();
   fixture_signal::write(na.root.join("items-spawn-burst"),serde_json::to_vec(&vec![sample.clone(),sample.clone(),sample.clone()]).unwrap()).unwrap();
   until(&mut service,"exact source blocked before local effect",||na.root.join("items-blocked.json").is_file()).await;
   let blocked:Value=serde_json::from_str(&fixture_signal::read(na.root.join("items-blocked.json")).unwrap().unwrap()).unwrap();let id=blocked["id"].as_str().unwrap().to_owned();
   until(&mut service,"later siblings finish around unavailable middle source",||items(&na)["kills"]==(index*3+2)as u64&&available(&nb).len()==index*3+2).await;
   assert_eq!(items(&na)["effectOutcomes"].as_array().unwrap().iter().filter(|o|o["id"]==id&&o["state"]=="removing").count(),0);
   assert!(items(&na)["effectOutcomes"].as_array().unwrap().iter().any(|o|o["id"]==id&&o["state"]=="waiting"&&o["reason"]=="source_unavailable"));
   let gold=bank_submit(&mut service,&na,"deposit",10,false).await;accepted(&mut service,&na,&gold).await;until(&mut service,"current gold succeeds during blocked item",||bank_view(&na)["wallet"]==(990-index*10)as u64).await;
   fresh_bidirectional_ghosts(&mut service,&a,&na,&b,&nb,"source waiting concurrent movement").await;
   fixture_signal::write(na.root.join("items-source-release"),"release unchanged owned source").unwrap();
   until(&mut service,"same original resumes exactly once",||items(&na)["kills"]==((index+1)*3)as u64&&available(&nb).len()==(index+1)*3&&items(&na)["aliveOriginals"].as_array().is_some_and(Vec::is_empty)).await;
   assert_eq!(items(&na)["effectOutcomes"].as_array().unwrap().iter().filter(|o|o["id"]==id&&o["state"]=="removing").count(),1);results.push(json!({"stage":stage,"blocked":blocked,"sameOriginalResumed":true,"siblingsCompletedFirst":true}));
  }
  elapsed(&mut service,2).await;let timing=nt2_desktop_g0::g3_http::bank_timing::snapshot();let raw=timing.to_string();for id in history.as_array().unwrap(){assert!(!raw.contains(id.as_str().unwrap()),"Foreign lobby original reached automatic HTTP work");}
  let current=a.handle.view();assert!(!current["g3"]["bank"]["status"].as_str().unwrap_or("").contains("uncertain"));assert!(!current["g3"]["bank"]["items"]["status"].as_str().unwrap_or("").contains("unresolved"));
  let retained=current["g3"]["operations"].as_array().unwrap();for id in history.as_array().unwrap(){assert!(retained.iter().any(|o|o["operation_id"]==*id&&o["current_scope"]==false&&o["server_status"].is_null()));}
  nt2_desktop_g0::g3_http::bank_timing::disarm();
  let host=expect_loss(&mut service,&na,"isolation End host").await;let peer=expect_loss(&mut service,&nb,"isolation End peer").await;item_end_pair(&mut service,[(&a,&na,&wa,&host),(&b,&nb,&wb,&peer)]).await;
  na.close();nb.close();a.close().await;b.close().await;service.close();
  std::fs::write(own.join("checks.json"),serde_json::to_vec_pretty(&json!({"passed":true,"actualComponents":true,"foreignLobbyOriginalsPreservedAndUnscheduled":true,"sourceCases":results,"timing":timing,"actualNoita":false,"engineFactsInjected":true})).unwrap()).unwrap();
 })).unwrap().join().unwrap();
}
