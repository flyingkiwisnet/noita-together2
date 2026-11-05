use nt2_desktop_g0::g3_journal::{Original,LobbyIdentity};
pub(super) async fn journal_rows(c:&Client)->Vec<Original>{let rows=c.handle.command(|c|Box::pin(async move{Ok(json!(c.g3.as_ref().unwrap().consumer.journal.pending().map(|p|p.record.original).collect::<Vec<_>>()))})).await.unwrap();serde_json::from_value(rows).unwrap()}
pub(super) async fn retained(c:&Client)->std::collections::BTreeSet<String>{journal_rows(c).await.into_iter().map(|o|o.operation_id).collect()}
pub(super) fn audit(service:&mut Service,a:&str,b:&str)->Value{let a=service.command(json!({"op":"lobbyTransferSummary","roomId":a}))["summary"].clone();let b=service.command(json!({"op":"lobbyTransferSummary","roomId":b}))["summary"].clone();assert!(a.is_object()&&b.is_object());json!({"a":a,"b":b})}
pub(super) async fn begin_run(service:&mut Service,root:&std::path::Path,router:&PipeHarness,previous:Option<&Client>)->(Client,Native){
 std::fs::create_dir_all(root).unwrap();if let Some(previous)=previous{std::fs::copy(previous.root.join("transfers.jsonl"),root.join("transfers.jsonl")).unwrap();}
 let client=Client::start(&service.private(),root,None,false,true,router.routes.clone(),service.watches.clone(),service.stores.clone()).await;
 // A fresh desktop sign-in does not inherit the prior desktop's membership or
 // consent to a new run. Use the ordinary reviewed Preview/Join actions.
 if previous.is_some(){join_current(&client,service.private()["room"]["roomId"].as_str().unwrap().into()).await;}
 bundle(service,&client).await;let native=Native::start(&client,None,false);
 wait(service,&client,"cleanup transfer run Ready",30,|v|current_state_is(v,"Ready")&&native.root.join("associated").is_file()).await;let world=client.handle.view()["rejoin"]["current_world"].clone();
 fixture_signal::write(native.root.join("start"),"actual host Start requested").unwrap();service.command(json!({"op":"startReviewed"}));ready_owner(service,&client,&native,&world,true).await;
 until(service,"cleanup run bank ready",||bank_view(&native)["view"]["fresh"]==true).await;(client,native)
}
pub(super) async fn end_run(service:&mut Service,client:&mut Client,native:&mut Native){
 let before=retained(client).await;let wallet=bank_view(native)["wallet"].clone();let writes=bank_view(native)["writes"].clone();let mut end=expect_loss(service,native,"cleanup ordinary End").await;end.before_fault(native);service.command(json!({"op":"end"}));
 until(service,"cleanup ordinary End held",||client.handle.view()["connection"]["state"]=="Ended"&&native_activity(native)["native_owned_hold"]==true).await;end.protected(native);elapsed(service,1).await;assert_eq!(retained(client).await,before,"End must preserve transfer originals");assert_eq!(bank_view(native)["wallet"],wallet);assert_eq!(bank_view(native)["writes"],writes);end.terminal(native);native.close();client.close().await;
}
async fn transfers(service:&mut Service,client:&Client,native:&Native,ambiguous:bool)->Vec<String>{
 let deposit=bank_submit(service,native,"deposit",50,false).await;accepted(service,native,&deposit).await;
 until(service,"cleanup completed deposit",||bank_view(native)["writes"]==1&&bank_view(native)["view"]["balance"]==50).await;
 // The operation reaches completed through the real flushed result/receipt.
 let id=deposit.clone();wait(service,client,"cleanup completed journal phase",10,move|v|v["g3"]["operations"].as_array().is_some_and(|ops|ops.iter().any(|o|o["operation_id"]==id))).await;
 let queued=bank_submit(service,native,"withdraw",10,false).await;accepted(service,native,&queued).await;until(service,"cleanup queued withdrawal",||bank_view(native)["view"]["queued"]==10).await;
 let mut ids=vec![deposit,queued];
 if ambiguous{fixture_signal::write(native.root.join("b1-copy-block-kind"),"result").unwrap();let id=bank_submit(service,native,"deposit",5,false).await;until(service,"cleanup ambiguous effect result held before app",||bank_input(native)["kind"]=="result"&&bank_input(native)["id"]==id).await;let rows=current_originals(client);assert!(rows.iter().any(|v|v["original"]["operation_id"]==id&&v["phase"]=="EffectIntent"));ids.push(id);}
 ids
}
#[test]#[ignore="Root serializes actual Client scheduler/TLS/SQL/native/OS pipe; wallet and outlet engine inputs injected"]
fn lobby_delete_ordinary_scheduler_composition(){
 assert_eq!(std::env::var("NT2_C3_COMPOSITION").as_deref(),Ok("1"));assert_eq!(std::env::var("NT2_B1_FIXTURE").as_deref(),Ok("1"));assert_eq!(std::env::var("NT2_A3_FIXTURE").as_deref(),Ok("1"));
 assert_eq!(std::env::var("NT2_POLYMORPH_FIXTURE").as_deref(),Ok("1"),"Current player identity/GameStats engine fixture required; permission and Start assertions remain production-owned");
 std::thread::Builder::new().name("lobby-delete".into()).stack_size(16*1024*1024).spawn(||tauri::async_runtime::block_on(async{
  verify_current_fixture_inputs();let own=repo().join("spikes/noita-entry/.build/lobby-delete-client-20261025-01").join(format!("composition-{}",uuid::Uuid::new_v4()));std::fs::create_dir_all(&own).unwrap();eprintln!("LOBBY_DELETE_COMPOSITION {}",own.display());
  let mut service=Service::start(&own);network_profile::enable_elapsed_clock(&mut service,&own);let router=PipeHarness::new();
  let room_a=service.private()["room"]["roomId"].as_str().unwrap().to_owned();let(mut a,mut na)=begin_run(&mut service,&own.join("a-run1"),&router,None).await;let mut deleted_ids=transfers(&mut service,&a,&na,true).await;let original_a=journal_rows(&a).await;end_run(&mut service,&mut a,&mut na).await;
  service.command(json!({"op":"nextRun"}));service.command(json!({"op":"signIn"}));let(mut a2,mut na2)=begin_run(&mut service,&own.join("a-run2"),&router,Some(&a)).await;deleted_ids.extend(transfers(&mut service,&a2,&na2,false).await);assert_eq!(journal_rows(&a2).await.iter().map(|o|serde_json::from_str::<Value>(&o.scope_json).unwrap()["runId"].as_str().unwrap().to_owned()).collect::<std::collections::BTreeSet<_>>().len(),2);end_run(&mut service,&mut a2,&mut na2).await;
  service.command(json!({"op":"lobbyCreate"}));let room_b=service.private()["room"]["roomId"].as_str().unwrap().to_owned();assert_ne!(room_a,room_b);let(mut b,mut nb)=begin_run(&mut service,&own.join("b"),&router,Some(&a2)).await;let mut kept_ids=transfers(&mut service,&b,&nb,false).await;
  // Labelled synthetic missing original, queried through the actual authenticated
  // original-status seam: ordinary NOT_FOUND cannot erase any local original.
  let mut missing=journal_rows(&b).await.into_iter().find(|o|LobbyIdentity::from_original(o).unwrap().room_id==room_b).unwrap();missing.operation_id=uuid::Uuid::new_v4().to_string();let mut scope:Value=serde_json::from_str(&missing.scope_json).unwrap();scope["worldToken"]=uuid::Uuid::new_v4().to_string().into();missing.scope_json=scope.to_string();let missing_id=missing.operation_id.clone();let copy=missing.clone();
  b.handle.command(move|c|Box::pin(async move{let j=c.g3.as_ref().unwrap().consumer.journal.clone();tokio::task::spawn_blocking(move||j.begin(copy)).await.unwrap()?;Ok(json!({"injectedMissingOriginal":true}))})).await.unwrap();
  let id=missing_id.clone();let result=b.handle.command(move|c|Box::pin(async move{c.g3_original_status(&id).await})).await;assert!(matches!(result,Err(ref e)if e=="NOT_FOUND"));kept_ids.push(missing_id);let before=retained(&b).await;assert_eq!(before.len(),deleted_ids.len()+kept_ids.len());end_run(&mut service,&mut b,&mut nb).await;
  let audit_before=audit(&mut service,&room_a,&room_b);assert_eq!(audit_before["a"]["runs"],2);assert!(audit_before["a"]["operations"].as_u64().unwrap()>=5);assert!(audit_before["a"]["receipts"].as_u64().unwrap()>=2);let denied=service.command(json!({"op":"lobbyDeleteForbidden","roomId":room_a}));assert_eq!(denied["accepted"],false);assert_eq!(denied["rejection"],"FORBIDDEN");assert_eq!(audit(&mut service,&room_a,&room_b),audit_before);
  service.command(json!({"op":"lobbyDelete","roomId":room_a}));service.command(json!({"op":"lobbyDelete","roomId":room_a}));let audit_deleted=audit(&mut service,&room_a,&room_b);assert_eq!(audit_deleted["a"]["deleted"],true);assert_eq!(audit_deleted["a"]["runs"],2);for field in ["operations","receipts","ledgerEntries","goldOutbox"]{assert_eq!(audit_deleted["a"][field],0,"Deleted lobby retains {field}");}for field in ["ledgerDelta","balanceTotal"]{assert_eq!(audit_deleted["a"][field],"0");}for field in ["statistics","statisticsTotal"]{assert_eq!(audit_deleted["a"][field],audit_before["a"][field]);}assert_eq!(audit_deleted["b"],audit_before["b"]);
  // Both actual apps/natives are closed at deletion. Cold refresh/sign-in and
  // cleanup run without a game/current attachment. A temporary network failure
  // preserves originals until an authenticated ROOM_DELETED response arrives.
  let cold_root=own.join("cold");std::fs::create_dir_all(&cold_root).unwrap();std::fs::copy(b.root.join("transfers.jsonl"),cold_root.join("transfers.jsonl")).unwrap();service.command(json!({"op":"transport","kind":"http","target":"consumer","enabled":true}));
  let mut cold=Client::start(&service.private(),&cold_root,Some(b.store.clone()),false,false,router.routes.clone(),service.watches.clone(),service.stores.clone()).await;elapsed(&mut service,2).await;assert_eq!(retained(&cold).await,before);assert!(cold.pending.lock().unwrap().is_none());service.command(json!({"op":"transport","kind":"http","target":"consumer","enabled":false}));
  let keep=kept_ids.iter().cloned().collect::<std::collections::BTreeSet<_>>();let until_at=Instant::now()+Duration::from_secs(35);let mut sample_at=Instant::now();loop{
   service.wall_clock();if Instant::now()>=sample_at{boundary_snapshot(&cold,"cold desktop lobby cleanup");sample_at=Instant::now()+Duration::from_secs(2);}
   let durable=retained(&cold).await;let view=cold.handle.view();
   let published=view["g3"]["operations"].as_array().filter(|rows|rows.len()==keep.len()).and_then(|rows|rows.iter().map(|row|row["operation_id"].as_str().map(str::to_owned)).collect::<Option<std::collections::BTreeSet<_>>>());
   // Journal retirement can finish between the ordinary actor's command reply
   // and its next published snapshot. Require both boundaries, without pumping.
   if durable==keep&&published.as_ref()==Some(&keep){break}
   assert!(Instant::now()<until_at,"Cold desktop cleanup deadline; inspect {}",cold.root.display());tokio::time::sleep(Duration::from_millis(200)).await;
  }
  assert!(cold.pending.lock().unwrap().is_none());assert_eq!(cold.handle.view()["g3"]["operations"].as_array().unwrap().len(),kept_ids.len());let bytes=std::fs::read_to_string(cold.root.join("transfers.jsonl")).unwrap();for id in &deleted_ids{assert!(!bytes.contains(id));}for id in &kept_ids{assert!(bytes.contains(id));}
  // Late originals and late cached status/effect paths cannot re-create a
  // deleted room. This is a local stale-work probe, not an invented server ACK.
  let old=original_a[0].clone();cold.handle.command(move|c|Box::pin(async move{let j=c.g3.as_ref().unwrap().consumer.journal.clone();let key=LobbyIdentity::from_original(&old)?;assert!(j.lobby_deleted(&key));assert!(j.prepare_effect(&old.operation_id).is_err());let result=tokio::task::spawn_blocking(move||j.begin(old)).await.unwrap();assert!(result.is_err());Ok(json!({"lateOriginalFenced":true}))})).await.unwrap();
  let audit_after=audit(&mut service,&room_a,&room_b);assert_eq!(audit_after,audit_deleted,"Cold cleanup must not change any ledger/balance/statistics");
  cold.close().await;let reopen_root=own.join("reopen");std::fs::create_dir_all(&reopen_root).unwrap();std::fs::copy(cold.root.join("transfers.jsonl"),reopen_root.join("transfers.jsonl")).unwrap();let mut reopened=Client::start(&service.private(),&reopen_root,Some(cold.store.clone()),false,false,router.routes.clone(),service.watches.clone(),service.stores.clone()).await;elapsed(&mut service,2).await;assert_eq!(retained(&reopened).await,keep);assert!(reopened.pending.lock().unwrap().is_none());reopened.close().await;
  let audit_final=audit(&mut service,&room_a,&room_b);assert_eq!(audit_final,audit_deleted);service.close();
  std::fs::write(own.join("checks.json"),serde_json::to_vec_pretty(&json!({"passed":true,"actualComponents":true,"engineWalletAndOutletInjected":true,"missingOriginalInjected":true,"loadedSaveProof":false,"noitaLaunched":false,"packageChanged":false,"deletedOriginals":deleted_ids.len(),"keptOriginals":kept_ids.len(),"twoDeletedLobbyRuns":true,"offlineAtDelete":true,"coldDesktopWithoutGame":true,"notFoundAndNetworkRetained":true,"lateOriginalFenced":true,"auditBefore":audit_before,"auditDeleted":audit_deleted,"auditAfter":audit_after,"schemaCleanupOwnedByFixture":true})).unwrap()).unwrap();
 })).unwrap().join().unwrap();
}
