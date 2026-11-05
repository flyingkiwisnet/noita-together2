//! Scoped CR2 review corrections; real HTTP/WSS and production worker/lane,
//! synthetic native facts and an explicitly blocked in-memory credential store.
use super::*;
use std::sync::Condvar;
#[derive(Default)]struct Gate{armed:bool,blocked:bool,released:bool,journals:u32,finals:u32}
#[derive(Clone,Default)]struct SlowStore{inner:MemoryStore,gate:Arc<(Mutex<Gate>,Condvar)>}
impl CredentialStore for SlowStore{
 fn save(&self,p:&ServiceProfile,b:&SecretBlob)->Result<(),crate::secrets::StoreError>{
  let record:Value=serde_json::from_slice(b.bytes()).unwrap_or(Value::Null);
  if record["refresh"].is_string()&&record["request_id"].is_string(){
   let(mut gate,wake)=(self.gate.0.lock().unwrap(),&self.gate.1);
   if gate.armed{if record["request_id"]==""{gate.blocked=true;wake.notify_all();let deadline=Instant::now()+Duration::from_secs(30);while !gate.released{let remaining=deadline.saturating_duration_since(Instant::now());if remaining.is_zero(){return Err(crate::secrets::StoreError::Unavailable);}gate=wake.wait_timeout(gate,remaining).unwrap().0;}gate.finals+=1;}else{gate.journals+=1;}}
  }
  self.inner.save(p,b)
 }
 fn load(&self,p:&ServiceProfile)->Result<Option<SecretBlob>,crate::secrets::StoreError>{self.inner.load(p)}
 fn delete(&self,p:&ServiceProfile)->Result<(),crate::secrets::StoreError>{self.inner.delete(p)}
}
struct Release(Arc<(Mutex<Gate>,Condvar)>);
impl Release{fn now(&self){self.0.0.lock().unwrap().released=true;self.0.1.notify_all();}}
impl Drop for Release{fn drop(&mut self){self.now();}}
#[test]#[ignore="focused disposable CR2-C-R1/R2 real-worker regression"]
fn cr2_correction_blocked_refresh_and_denied_wss_recovers(){tauri::async_runtime::block_on(async{
 let mut fixture=Fixture::start_script("spikes/noita-entry/.build/g3-connection-cr2-client-correction-20261015-01/connection-fixture.mjs");
 let store=SlowStore::default();let mut c=Core::new(store.clone(),uuid::Uuid::new_v4(),Arc::new(Mutex::new(None)),std::env::temp_dir().join(format!("nt2-cr2-correction-{}",uuid::Uuid::new_v4())));c.open_browser=false;
 let ca=reqwest::Certificate::from_pem(&std::fs::read(&fixture.ca).unwrap()).unwrap();c.http=reqwest::Client::builder().use_native_tls().tls_certs_only(vec![ca]).no_proxy().redirect(reqwest::redirect::Policy::none()).timeout(Duration::from_secs(3)).build().unwrap();
 std::fs::create_dir_all(&c.claims_root).unwrap();c.enable_g3(&c.claims_root.join("journal.jsonl"),Arc::new(Mutex::new(nt2_desktop_g0::g3_runtime::Exchange::default()))).unwrap();c.enable_live_transport().unwrap();c.select(&fixture.origin,"",false).await.unwrap();registered_as(&mut c,"alex").await;
 c.request_social(Arc::new(Mutex::new(nt2_desktop_g0::g3_social_stream::Exchange::default()))).unwrap();c.social_profile=Some(nt2_desktop_g0::g3_social_installation::development_profile().unwrap()["profile"].clone());c.configure_game(&std::env::var("NT2_FLOW_GAME_FIXTURE").unwrap()).unwrap();c.enable_automatic_setup();c.create_fixture(false).await.unwrap();facts(&mut c,1,true);
 *c.expected_game.lock().unwrap()=Some(crate::pipe::verified_process_path(&std::env::current_exe().unwrap()).unwrap());let mut health=c.health_runner();c.game_tick().await.unwrap();let _=health.pump().await;c.sync_health();c.collect_inspection().await.unwrap();c.final_start(vec![c.desktop.as_ref().unwrap().id.clone()],vec![]).await.unwrap();
 tokio::time::sleep(Duration::from_millis(2050)).await;facts(&mut c,2,true);health.pump().await.unwrap();c.sync_health();tokio::time::sleep(Duration::from_millis(2050)).await;facts(&mut c,3,false);health.pump().await.unwrap();c.sync_health();assert!(c.game.as_ref().unwrap().entered);c.g3_sync().unwrap();
 let original_desktop=c.desktop.as_ref().unwrap().access.clone();let original_game=c.game.as_ref().unwrap().session.access.clone();let owner=c.game.as_ref().unwrap().owner;let generation=c.generation.load(Ordering::Acquire);
 c.remember=true;c.publish_health();store.gate.0.lock().unwrap().armed=true;let release=Release(store.gate.clone());health.refresh_for_controlled_store();fixture.command("deny-wss-once");let mut movement=c.movement_runner().unwrap();movement.pump().await.unwrap();
 let mut sequence=3;let mut denied=0;let until=Instant::now()+Duration::from_secs(7);
 while !store.gate.0.lock().unwrap().blocked{assert!(Instant::now()<until,"normal refresh did not reach final persistence");sequence+=1;facts(&mut c,sequence,false);health.pump().await.unwrap();c.sync_health();if let Err(error)=movement.pump().await{assert_eq!(error,"FORBIDDEN");denied+=1;}tokio::time::sleep(Duration::from_millis(100)).await;}
 assert_eq!(store.gate.0.lock().unwrap().journals,1);let profile=c.profile.as_ref().unwrap().clone();let journal=store.load(&profile).unwrap().unwrap();let journal:Value=serde_json::from_slice(journal.bytes()).unwrap();assert!(journal["request_id"].as_str().is_some_and(|s|!s.is_empty()),"original refresh journal must remain while final save is blocked");
 // A second writer must queue behind the same serialized final commit, while
 // neither it nor the slow CredentialStore can delay health network attempts.
 let writer=c.credential_writer.clone();let contender=tokio::task::spawn_blocking(move||{let _guard=writer.lock().unwrap();});
 let until=Instant::now()+Duration::from_secs(13);let mut cycles=0;let mut seen=c.health_seen;let mut max_pump_ms=0;
 while Instant::now()<until{
  sequence+=1;facts(&mut c,sequence,false);let at=Instant::now();health.pump().await.unwrap();max_pump_ms=max_pump_ms.max(at.elapsed().as_millis());c.sync_health();if c.health_seen!=seen{cycles+=1;seen=c.health_seen;}
  if let Err(error)=movement.pump().await{assert_eq!(error,"FORBIDDEN");denied+=1;}
  let p=c.pending.lock().unwrap();assert!(p.as_ref().unwrap().release&&p.as_ref().unwrap().authority_until>Instant::now());drop(p);assert!(!contender.is_finished());tokio::time::sleep(Duration::from_millis(100)).await;
 }
 assert!(cycles>=5);assert!(max_pump_ms<3000);assert_eq!(denied,1);assert_ne!(c.game.as_ref().unwrap().session.access.as_str(),original_game.as_str(),"WSS denial must cause verified renewal without manual token replacement");assert_eq!(c.game.as_ref().unwrap().owner,owner);assert_eq!(c.generation.load(Ordering::Acquire),generation);
 let metrics=c.g3_view()["movement"].clone();assert_eq!(metrics["live_admission_renewal_retries"],1);assert!(metrics["live_connections"].as_u64().unwrap_or(0)>=1,"{metrics}");assert_eq!(metrics["peer_attempts"],0,"no HTTP movement fallback");fixture.command("denial-check");fixture.command("peer-check");
 assert_ne!(c.desktop.as_ref().unwrap().access.as_str(),original_desktop.as_str(),"verified replacement must serve health before final persistence");release.now();contender.await.unwrap();let until=Instant::now()+Duration::from_secs(5);
 while health.refresh_pending_for_controlled_store(){assert!(Instant::now()<until,"final refresh result not adopted");sequence+=1;facts(&mut c,sequence,false);health.pump().await.unwrap();c.sync_health();movement.pump().await.unwrap();tokio::time::sleep(Duration::from_millis(100)).await;}
 let record=store.load(&profile).unwrap().unwrap();let record:Value=serde_json::from_slice(record.bytes()).unwrap();assert_eq!(record["request_id"],"");assert_eq!(store.gate.0.lock().unwrap().finals,1);
 // Exercise the existing logout fence against another real final commit that
 // has already entered CredentialStore::save. Logout must delete after it.
 *store.gate.0.lock().unwrap()=Gate{armed:true,..Default::default()};health.refresh_for_controlled_store();let until=Instant::now()+Duration::from_secs(5);
 while !store.gate.0.lock().unwrap().blocked{assert!(Instant::now()<until);sequence+=1;facts(&mut c,sequence,false);health.pump().await.unwrap();c.sync_health();tokio::time::sleep(Duration::from_millis(100)).await;}
 // Let the normal health projection deliver this verified RAM credential to
 // the UI before issuing logout, while its storage commit remains blocked.
 tokio::time::sleep(Duration::from_millis(2050)).await;sequence+=1;facts(&mut c,sequence,false);health.pump().await.unwrap();c.sync_health();
 let fence=c.generation.clone();let prior=fence.load(Ordering::Acquire);let gate=store.gate.clone();
 // Logout's existing synchronous writer wait may occupy the runtime thread.
 // Release independently of Tokio only after observing its retirement fence.
 let controller=std::thread::spawn(move||{let until=Instant::now()+Duration::from_secs(5);while fence.load(Ordering::Acquire)==prior&&Instant::now()<until{std::thread::sleep(Duration::from_millis(10));}let fenced=fence.load(Ordering::Acquire)!=prior;gate.0.lock().unwrap().released=true;gate.1.notify_all();fenced});
 c.logout().await.unwrap();assert!(controller.join().unwrap(),"logout must retire before final store release");assert!(c.desktop.is_none());assert!(store.load(&profile).unwrap().is_none());
 if let Some(path)=std::env::var_os("NT2_CR2_CORRECTION_EVIDENCE"){std::fs::write(path,serde_json::to_vec_pretty(&json!({"blockedFinalStoreMs":13000,"healthCyclesWhileBlocked":cycles,"maxHealthPumpMs":max_pump_ms,"serializedWriterContended":true,"originalRefreshJournalRetained":true,"finalRefreshPersistedAndAdopted":true,"logoutFencedBlockedFinalWrite":true,"wssDenials":denied,"automaticVerifiedGameRenewal":true,"realWssConnections":metrics["live_connections"],"renewalRequests":metrics["live_admission_renewal_retries"],"ownerGenerationUnchangedBeforeLogout":true,"httpMovementFallback":false,"peerHealthy":true,"engineFactsInjected":true})).unwrap()).unwrap();}
 drop(movement);drop(health);let root=c.claims_root.canonicalize().unwrap();assert!(root.starts_with(std::env::temp_dir().canonicalize().unwrap())&&root.file_name().unwrap().to_string_lossy().starts_with("nt2-cr2-correction-"));drop(c);std::fs::remove_dir_all(root).unwrap();
});}

#[derive(Default)]struct FailureState{failures:u32,failed:u32,final_attempts:u32,journals:Vec<Vec<u8>>}
#[derive(Clone,Default)]struct FailStore{inner:MemoryStore,state:Arc<Mutex<FailureState>>}
impl CredentialStore for FailStore{
 fn save(&self,p:&ServiceProfile,b:&SecretBlob)->Result<(),crate::secrets::StoreError>{
  let v:Value=serde_json::from_slice(b.bytes()).unwrap_or(Value::Null);
  if v["refresh"].is_string()&&v["request_id"].is_string(){let mut state=self.state.lock().unwrap();if v["request_id"]==""{state.final_attempts+=1;if state.failures>0{state.failures-=1;state.failed+=1;return Err(crate::secrets::StoreError::Unavailable);}}else{state.journals.push(b.bytes().to_vec());}}
  self.inner.save(p,b)
 }
 fn load(&self,p:&ServiceProfile)->Result<Option<SecretBlob>,crate::secrets::StoreError>{self.inner.load(p)}
 fn delete(&self,p:&ServiceProfile)->Result<(),crate::secrets::StoreError>{self.inner.delete(p)}
}
#[test]#[ignore="focused disposable CR2-C-R3 final storage failure regression"]
fn cr2_r3_verified_refresh_retries_only_storage_and_retires_safely(){tauri::async_runtime::block_on(async{
 let mut fixture=Fixture::start_script("spikes/noita-entry/.build/g3-connection-cr2-client-r3-20261015-01/connection-fixture.mjs");
 let store=FailStore::default();let mut c=Core::new(store.clone(),uuid::Uuid::new_v4(),Arc::new(Mutex::new(None)),std::env::temp_dir().join(format!("nt2-cr2-correction-{}",uuid::Uuid::new_v4())));c.open_browser=false;
 let ca=reqwest::Certificate::from_pem(&std::fs::read(&fixture.ca).unwrap()).unwrap();c.http=reqwest::Client::builder().use_native_tls().tls_certs_only(vec![ca]).no_proxy().redirect(reqwest::redirect::Policy::none()).timeout(Duration::from_secs(3)).build().unwrap();
 std::fs::create_dir_all(&c.claims_root).unwrap();c.enable_g3(&c.claims_root.join("journal.jsonl"),Arc::new(Mutex::new(nt2_desktop_g0::g3_runtime::Exchange::default()))).unwrap();c.enable_live_transport().unwrap();c.select(&fixture.origin,"",false).await.unwrap();registered_as(&mut c,"alex").await;
 c.request_social(Arc::new(Mutex::new(nt2_desktop_g0::g3_social_stream::Exchange::default()))).unwrap();c.social_profile=Some(nt2_desktop_g0::g3_social_installation::development_profile().unwrap()["profile"].clone());c.configure_game(&std::env::var("NT2_FLOW_GAME_FIXTURE").unwrap()).unwrap();c.enable_automatic_setup();c.create_fixture(false).await.unwrap();facts(&mut c,1,true);
 *c.expected_game.lock().unwrap()=Some(crate::pipe::verified_process_path(&std::env::current_exe().unwrap()).unwrap());let mut health=c.health_runner();c.game_tick().await.unwrap();let _=health.pump().await;c.sync_health();c.collect_inspection().await.unwrap();c.final_start(vec![c.desktop.as_ref().unwrap().id.clone()],vec![]).await.unwrap();
 tokio::time::sleep(Duration::from_millis(2050)).await;facts(&mut c,2,true);health.pump().await.unwrap();c.sync_health();tokio::time::sleep(Duration::from_millis(2050)).await;facts(&mut c,3,false);health.pump().await.unwrap();c.sync_health();assert!(c.game.as_ref().unwrap().entered);c.g3_sync().unwrap();

 let profile=c.profile.as_ref().unwrap().clone();let mut sequence=3;let owner=c.game.as_ref().unwrap().owner;let generation=c.generation.load(Ordering::Acquire);
 c.remember=true;c.publish_health();let old_secret=c.desktop.as_ref().unwrap().refresh.clone();store.state.lock().unwrap().failures=1;health.refresh_for_controlled_store();
 let until=Instant::now()+Duration::from_secs(12);let mut notice=false;let mut cycles=0;let mut seen=c.health_seen;let mut max_ms=0;
 loop{
  assert!(Instant::now()<until,"storage retry did not reconcile");sequence+=1;facts(&mut c,sequence,false);let at=Instant::now();health.pump().await.unwrap();max_ms=max_ms.max(at.elapsed().as_millis());c.sync_health();if c.health_seen!=seen{cycles+=1;seen=c.health_seen;}
  notice|=c.status_error.as_deref().is_some_and(|e|e.contains("storage retry pending"));
  {let state=store.state.lock().unwrap();if state.failed==1&&state.final_attempts==1{let saved=store.load(&profile).unwrap().unwrap();let v:Value=serde_json::from_slice(saved.bytes()).unwrap();assert_eq!(v["refresh"],old_secret.as_str());assert!(!v["request_id"].as_str().unwrap().is_empty());assert_eq!(saved.bytes(),state.journals[0]);}}
  let p=c.pending.lock().unwrap();assert!(p.as_ref().unwrap().release&&p.as_ref().unwrap().authority_until>Instant::now());drop(p);
  if store.state.lock().unwrap().final_attempts==2&&!health.refresh_pending_for_controlled_store()&&!health.refresh_commit_pending_for_test(){break;}
  tokio::time::sleep(Duration::from_millis(100)).await;
 }
 assert!(notice&&cycles>=2&&max_ms<3000);fixture.command("refresh-count-1");assert_eq!(store.state.lock().unwrap().journals.len(),1);let saved=store.load(&profile).unwrap().unwrap();let v:Value=serde_json::from_slice(saved.bytes()).unwrap();assert_eq!(v["request_id"],"");assert_eq!(v["refresh"],c.desktop.as_ref().unwrap().refresh.as_str());assert_ne!(v["refresh"],old_secret.as_str());
 // A legitimate later rotation uses the verified new secret and a NEW request.
 let next_secret=c.desktop.as_ref().unwrap().refresh.clone();store.state.lock().unwrap().failures=1;health.refresh_for_controlled_store();let until=Instant::now()+Duration::from_secs(7);
 while store.state.lock().unwrap().failed<2{assert!(Instant::now()<until);sequence+=1;facts(&mut c,sequence,false);health.pump().await.unwrap();c.sync_health();tokio::time::sleep(Duration::from_millis(100)).await;}
 let original=store.load(&profile).unwrap().unwrap();let v:Value=serde_json::from_slice(original.bytes()).unwrap();assert_eq!(v["refresh"],next_secret.as_str());{let state=store.state.lock().unwrap();let first:Value=serde_json::from_slice(&state.journals[0]).unwrap();assert_ne!(first["request_id"],v["request_id"]);}
 // Policy retirement cancels the pending commit without replaying HTTP or
 // overwriting its exact journal under a changed remember choice.
 c.remember=false;c.publish_health();tokio::time::sleep(Duration::from_millis(2050)).await;sequence+=1;facts(&mut c,sequence,false);health.pump().await.unwrap();c.sync_health();assert!(!health.refresh_commit_pending_for_test());assert_eq!(store.load(&profile).unwrap().unwrap().bytes(),original.bytes());assert_eq!(store.state.lock().unwrap().final_attempts,3);fixture.command("refresh-count-2");
 // Owner retirement with a subsequent failed final write must never retry after
 // logout or resurrect stored credentials. This is another new rotation.
 c.remember=true;c.publish_health();store.state.lock().unwrap().failures=1;health.refresh_for_controlled_store();let until=Instant::now()+Duration::from_secs(7);
 while store.state.lock().unwrap().failed<3{assert!(Instant::now()<until);sequence+=1;facts(&mut c,sequence,false);health.pump().await.unwrap();c.sync_health();tokio::time::sleep(Duration::from_millis(100)).await;}
 tokio::time::sleep(Duration::from_millis(2050)).await;sequence+=1;facts(&mut c,sequence,false);health.pump().await.unwrap();c.sync_health();assert!(health.refresh_commit_pending_for_test());assert_eq!(c.game.as_ref().unwrap().owner,owner);assert_eq!(c.generation.load(Ordering::Acquire),generation);c.logout().await.unwrap();health.pump().await.unwrap();assert!(store.load(&profile).unwrap().is_none());assert_eq!(store.state.lock().unwrap().final_attempts,4);fixture.command("refresh-count-3");fixture.command("peer-check");
 if let Some(path)=std::env::var_os("NT2_CR2_CORRECTION_EVIDENCE"){std::fs::write(path,serde_json::to_vec_pretty(&json!({"normalRefreshFinalFailureThenSuccess":true,"firstRotationHttpRequests":1,"firstRotationFinalStoreAttempts":2,"exactOriginalSecretRequestRetained":true,"storageRetryNotice":notice,"healthCyclesDuringReconciliation":cycles,"maxHealthPumpMs":max_ms,"policyRetiredPendingCommit":true,"logoutRetiredPendingCommit":true,"newFutureRotationsDistinct":true,"totalHttpRefreshRequests":3,"totalFinalStoreAttempts":4,"peerHealthy":true,"engineFactsInjected":true})).unwrap()).unwrap();}
 drop(health);let root=c.claims_root.canonicalize().unwrap();assert!(root.starts_with(std::env::temp_dir().canonicalize().unwrap())&&root.file_name().unwrap().to_string_lossy().starts_with("nt2-cr2-correction-"));drop(c);std::fs::remove_dir_all(root).unwrap();
});}
