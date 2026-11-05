// Read-only fixture observations retain actor queue and shared-window timing.
// Native evidence is acquired at actor entry, never before waiting in its queue.
async fn item_observation_snapshot<S:CredentialStore+'static>(handle:&scheduler::Handle<S>,pid:crate::local_owner::Id,native_root:std::path::PathBuf,client_root:&std::path::Path,started:Instant,deadline:Instant,phase:&str)->Result<Value,String>{
 let enqueued=Instant::now();let budget=deadline.saturating_duration_since(enqueued);let trace=Arc::new(Mutex::new(json!({"phase":phase,"observerElapsedAtEnqueueUs":started.elapsed().as_micros() as u64,"requestedRemainingBudgetUs":budget.as_micros() as u64,"actorEnteredUs":Value::Null,"actorCompletedUs":Value::Null,"nativeAtBoundary":Value::Null,"actorBeforeEnqueue":handle.fixture_timing(),"pid":pid,"nativeRoot":native_root})));
 let current=trace.clone();
 let result=if budget.is_zero(){Err("Shared item observation window already consumed".into())}else{
  match tokio::time::timeout(budget,handle.command(move|c|Box::pin(async move{
   let entered=Instant::now();{let mut t=current.lock().unwrap();t["actorEnteredUs"]=json!(started.elapsed().as_micros() as u64);t["queueDelayUs"]=json!(enqueued.elapsed().as_micros() as u64);}
   let native=native_evidence_at(pid,&native_root);let native_value=native.value.clone();let native_age=native.modified.and_then(|at|at.elapsed().ok()).map(|age|age.as_micros() as u64);
   let value=core_current_snapshot(c,Some(native));{let mut t=current.lock().unwrap();t["actorCompletedUs"]=json!(started.elapsed().as_micros() as u64);t["actorWorkUs"]=json!(entered.elapsed().as_micros() as u64);t["nativeModifiedAgeUs"]=json!(native_age);t["nativeAtBoundary"]=native_value;}
   Ok(value)
  }))).await{Ok(result)=>result,Err(_)=>Err("Fixture current-owner observation timed out inside shared item window".into())}
 };
 let mut timing=trace.lock().unwrap().clone();timing["replyObservedUs"]=json!(started.elapsed().as_micros() as u64);timing["actorAfterReply"]=handle.fixture_timing();timing["completed"]=json!(result.is_ok());
 let mut out=std::fs::OpenOptions::new().create(true).append(true).open(client_root.join("item-observation-timing.jsonl")).unwrap();writeln!(out,"{timing}").unwrap();
 if let Err(reason)=&result{let path=snapshot_failure(client_root,phase,reason,handle.view(),timing["nativeAtBoundary"].clone());let mut v:Value=serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();v["observationTiming"]=timing;std::fs::write(&path,serde_json::to_vec_pretty(&v).unwrap()).unwrap();}
 result
}
async fn item_observation_pair(a:&Client,na:&Native,b:&Client,nb:&Native,started:Instant,deadline:Instant,phase:&str)->[Value;2]{
 let (left,right)=futures_util::future::join(item_observation_snapshot(&a.handle,na.owner,na.root.clone(),&a.root,started,deadline,phase),item_observation_snapshot(&b.handle,nb.owner,nb.root.clone(),&b.root,started,deadline,phase)).await;
 [left.unwrap_or_else(|e|panic!("Host {phase}: {e}; {}",a.root.join("current-observation-failure.json").display())),right.unwrap_or_else(|e|panic!("Peer {phase}: {e}; {}",b.root.join("current-observation-failure.json").display()))]
}
#[test]fn item_observation_distinguishes_consumed_budget_from_actor_queue_and_samples_native_at_entry(){tauri::async_runtime::block_on(async{
 let own=repo().join("spikes/noita-entry/.build/item-client-20261025-01").join(format!("observation-unit-{}",uuid::Uuid::new_v4()));std::fs::create_dir(&own).unwrap();fixture_signal::write(own.join("activity.json"),br#"{"frame":77,"native_context":"at-entry"}"#).unwrap();
 let core=Core::new(super::super::super::super::tests::MemoryStore::default(),uuid::Uuid::new_v4(),Arc::new(Mutex::new(None)),own.join("claims"));let stop=Arc::new(std::sync::atomic::AtomicBool::new(false));let(handle,actor)=scheduler::new(core,stop.clone());let actor=tokio::spawn(actor);
 let start=Instant::now();let fresh=item_observation_snapshot(&handle,nt2_local_ws::owner::Id::OsProcess(7),own.clone(),&own,start,start+Duration::from_secs(2),"unit fresh").await.unwrap();assert_eq!(fresh["fixture_current"]["native"]["frame"],77);
 let(entered_tx,entered_rx)=tokio::sync::oneshot::channel();let(release_tx,release_rx)=tokio::sync::oneshot::channel();let blocking=handle.clone();let blocked=tokio::spawn(async move{blocking.command(move|_|Box::pin(async move{let _=entered_tx.send(());let _=release_rx.await;Ok(Value::Null)})).await});entered_rx.await.unwrap();
 let start=Instant::now();let queued=item_observation_snapshot(&handle,nt2_local_ws::owner::Id::OsProcess(7),own.clone(),&own,start,start+Duration::from_millis(30),"unit queued").await;let past=Instant::now()-Duration::from_secs(3);let consumed=item_observation_snapshot(&handle,nt2_local_ws::owner::Id::OsProcess(7),own.clone(),&own,past,past+Duration::from_millis(2500),"unit consumed").await;
 let lines=std::fs::read_to_string(own.join("item-observation-timing.jsonl")).unwrap();let rows:Vec<Value>=lines.lines().map(|l|serde_json::from_str(l).unwrap()).collect();
 let _=release_tx.send(());let _=blocked.await;stop.store(true,Ordering::Release);actor.await.unwrap();
 assert!(queued.unwrap_err().contains("timed out"));assert!(consumed.unwrap_err().contains("already consumed"));assert!(rows[0]["actorEnteredUs"].as_u64().is_some());assert!(rows[0]["actorCompletedUs"].as_u64().unwrap()>=rows[0]["actorEnteredUs"].as_u64().unwrap());assert!(rows[1]["actorEnteredUs"].is_null());assert_eq!(rows[1]["actorBeforeEnqueue"]["phase"],"command");assert!(rows[1]["requestedRemainingBudgetUs"].as_u64().unwrap()>0);assert_eq!(rows[2]["requestedRemainingBudgetUs"],0);assert!(rows[2]["actorEnteredUs"].is_null());
});}
