//! CR2 source composition. Engine loading, report and protection facts are injected;
//! HTTPS actions, scheduling, canonical decisions and protected memory store are real.
use super::*;
use std::{io::{BufRead,BufReader,Write},process::{Child,Command,Stdio},sync::mpsc};
struct Fixture{child:Child,events:mpsc::Receiver<Value>,origin:String,ca:String}
#[path="next_startup_evidence_test.rs"]mod startup_evidence;
impl Fixture{
 fn start()->Self{
  Self::start_script("spikes/noita-entry/.build/g3-connection-cr2-client-20261015-01/connection-fixture.mjs")
 }
 fn start_script(script:&str)->Self{
  let repo=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..").canonicalize().unwrap();
  let mut cmd=Command::new("C:/Program Files/nodejs/node.exe");
  #[cfg(windows)]{use std::os::windows::process::CommandExt;cmd.creation_flags(0x08000000);}
  let mut child=cmd.current_dir(repo).args(["--import","tsx",script]).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::inherit()).spawn().unwrap();
  let (tx,events)=mpsc::channel();let output=child.stdout.take().unwrap();std::thread::spawn(move||{for line in BufReader::new(output).lines(){if let Ok(line)=line{if let Ok(v)=serde_json::from_str::<Value>(&line){for key in ["allocatedFixtureSchema","verifiedAbsentSchema"]{if let Some(s)=v[key].as_str().filter(|s|s.starts_with("g3_rejoin_")&&s.len()==42&&s[10..].bytes().all(|b|b.is_ascii_hexdigit())){eprintln!("CR2 {key}={s}");}}if v["databaseTarget"]=="127.0.0.1:54328/nt2_g1"{eprintln!("CR2 databaseTarget=127.0.0.1:54328/nt2_g1");}let _=tx.send(v);}}}});
  // Own the child before waiting: failure/timeout now runs Drop and EOF cleanup.
  let mut fixture=Self{child,events,origin:String::new(),ca:String::new()};let until=Instant::now()+Duration::from_secs(55);
  loop{let ready=fixture.events.recv_timeout(until.saturating_duration_since(Instant::now())).expect("CR2 disposable fixture explicit ready deadline");if ready["event"]!="ready"{continue}fixture.origin=ready["origin"].as_str().expect("Fixture ready origin").into();fixture.ca=ready["caPath"].as_str().expect("Fixture ready CA").into();return fixture}
 }
 fn command(&mut self,line:&str){eprintln!("CR2 fixture control {line}");writeln!(self.child.stdin.as_mut().unwrap(),"{line}").unwrap();self.child.stdin.as_mut().unwrap().flush().unwrap();loop{let v=self.events.recv_timeout(Duration::from_secs(55)).unwrap_or_else(|e|panic!("fixture control {line}: {e}"));if v["event"]==line{break;}}}
}
impl Drop for Fixture{fn drop(&mut self){if let Some(mut input)=self.child.stdin.take(){let _=writeln!(input,"quit");let _=input.flush();drop(input);}let _=self.child.wait();}}
fn facts<S:CredentialStore>(c:&mut Core<S>,sequence:u64,protected:bool){
 let mut slot=c.pending.lock().unwrap();let p=slot.as_mut().expect("normal setup armed native grant");let owner=p.owner.unwrap_or((nt2_local_ws::owner::Id::OsProcess(std::process::id()),*c.device.as_bytes()));p.owner=Some(owner);p.loaded=true;p.accepted=true;p.actual_seed=Some(u64::from(p.seed));p.last_seen=Some(Instant::now());
 let observation=native_report::Observation{actual_seed:Some(p.seed.to_string()),mods:vec!["noita_together2".into()],complete:true,error:None,beta:Some(false),loaded:true,protected,mod_version:nt2_desktop_g0::g3_social_installation::MOD_VERSION.into(),bridge_version:nt2_desktop_g0::g3_social_installation::BRIDGE_VERSION.into(),..Default::default()};let digest=observation.content_digest().unwrap();
 p.observation=Some(native_report::Latest{context:owner.1,sequence,observation,digest,observed_at:Instant::now()});
}
#[test]#[ignore="one explicit disposable TLS/DB CR2 source composition"]
fn cr2_normal_actions_typed_health_start_capture_renew_loss_and_peer(){tauri::async_runtime::block_on(async{
 let mut fixture=Fixture::start();let mut c=core();
 let ca=reqwest::Certificate::from_pem(&std::fs::read(&fixture.ca).unwrap()).unwrap();c.http=reqwest::Client::builder().use_native_tls().tls_certs_only(vec![ca]).no_proxy().redirect(reqwest::redirect::Policy::none()).timeout(Duration::from_secs(3)).build().unwrap();
 std::fs::create_dir_all(&c.claims_root).unwrap();c.enable_g3(&c.claims_root.join("journal.jsonl"),Arc::new(Mutex::new(nt2_desktop_g0::g3_runtime::Exchange::default()))).unwrap();
 c.enable_live_transport().unwrap(); // Same selector used by the real C2/rejoin app.
 c.select(&fixture.origin,"",false).await.unwrap();registered(&mut c).await;
 c.request_social(Arc::new(Mutex::new(nt2_desktop_g0::g3_social_stream::Exchange::default()))).unwrap();
 // Reviewed existing runtime selector is fixture-only; no package/installation repin.
 c.social_profile=Some(nt2_desktop_g0::g3_social_installation::development_profile().unwrap()["profile"].clone());
 let game=std::env::var("NT2_FLOW_GAME_FIXTURE").unwrap();c.configure_game(&game).unwrap();c.enable_automatic_setup();
 c.create_fixture(false).await.unwrap();assert!(c.pending.lock().unwrap().is_some());facts(&mut c,1,true);
 *c.expected_game.lock().unwrap()=Some(crate::pipe::verified_process_path(&std::env::current_exe().unwrap()).unwrap());
 let mut health=c.health_runner();c.game_tick().await.unwrap();let before=health.pump().await;c.sync_health();assert!(!c.pending.lock().unwrap().as_ref().unwrap().release,"before Start must remain held: {before:?}");
 c.collect_inspection().await.unwrap();c.final_start(vec![c.desktop.as_ref().unwrap().id.clone()],vec![]).await.unwrap();
 tokio::time::sleep(Duration::from_millis(2050)).await;facts(&mut c,2,true);health.pump().await.unwrap();c.sync_health();assert!(c.pending.lock().unwrap().as_ref().unwrap().release);assert!(!c.game.as_ref().unwrap().entered);
 tokio::time::sleep(Duration::from_millis(2050)).await;facts(&mut c,3,false);health.pump().await.unwrap();c.sync_health();assert!(c.game.as_ref().unwrap().entered);assert_eq!(c.view()["connection"]["state"],"Playing");
 for n in 4..=5{tokio::time::sleep(Duration::from_millis(2050)).await;facts(&mut c,n,false);health.pump().await.unwrap();c.sync_health();}
 let a=c.resume_intent.association().expect("typed selected capture adopted and proof remembered").clone();let token=c.game.as_ref().unwrap().session.access.clone();
 c.g3_sync().unwrap();let mut movement=c.movement_runner().unwrap();movement.pump().await.unwrap();let pending=c.pending.clone();
 let ui=Arc::new(tokio::sync::Mutex::new(c));let locked=ui.clone();let(ready,started)=tokio::sync::oneshot::channel();let ui_task=tokio::spawn(async move{let _guard=locked.lock().await;let _=ready.send(());tokio::time::sleep(Duration::from_secs(13)).await;});started.await.unwrap();let mut locked_cycles=0;
 for cycle in 0..7{tokio::time::sleep(Duration::from_millis(2050)).await;if ui.try_lock().is_err(){locked_cycles+=1;}
  {let mut slot=pending.lock().unwrap();let p=slot.as_mut().unwrap();p.last_seen=Some(Instant::now());let report=p.observation.as_mut().unwrap();report.sequence+=1;report.observed_at=Instant::now();}
  if cycle==2{health.renew_for_controlled_clock();}health.pump().await.unwrap();movement.pump().await.unwrap();let p=pending.lock().unwrap();assert!(p.as_ref().unwrap().release&&p.as_ref().unwrap().authority_until>Instant::now());
 }
 ui_task.await.unwrap();assert!(locked_cycles>=5);let mut c=Arc::try_unwrap(ui).ok().unwrap().into_inner();c.sync_health();assert_ne!(token.as_str(),c.game.as_ref().unwrap().session.access.as_str());let movement_metrics=c.g3_view()["movement"].clone();assert!(movement_metrics["live_connections"].as_u64().unwrap_or(0)>=1,"{movement_metrics}");assert!(movement_metrics["live_reauthentications"].as_u64().unwrap_or(0)>=1,"{movement_metrics}");
 fixture.command("advance:120000");facts(&mut c,13,true);tokio::time::sleep(Duration::from_millis(2050)).await;let _=health.pump().await;c.sync_health();
 // Expire only local monotonic authority to represent the omitted HTTP health
 // interval; service time advanced separately while an unrelated peer pulsed.
 c.lifecycle.lock().unwrap().authority.authority.shorten(Instant::now());c.pending.lock().unwrap().as_mut().unwrap().authority_until=Instant::now();
 c.rejoin_tick().await.unwrap();assert!(c.game.is_none());assert_eq!(c.view()["connection"]["protection_confirmed"],true);
 tokio::time::sleep(Duration::from_millis(2100)).await;c.rejoin_tick().await.unwrap();assert!(c.game.is_some());assert!(c.rejoin_transport.generation>1);assert_eq!(c.resume_intent.association(),Some(&a));
 facts(&mut c,1,true);c.publish_health();tokio::time::sleep(Duration::from_millis(2050)).await;health.pump().await.unwrap();c.sync_health();assert!(c.pending.lock().unwrap().as_ref().unwrap().release);
 fixture.command("peer-check");
 if let Some(path)=std::env::var_os("NT2_CR2_EVIDENCE"){std::fs::write(path,serde_json::to_vec_pretty(&json!({"normalActions":true,"typedHealth":true,"initialProtectedStart":true,"sameContentPostStart":true,"actualEntryInjected":true,"captureAdopted":true,"gameTokenRotated":true,"uiCoreLockMs":13000,"healthCyclesWhileLocked":locked_cycles,"realWssConnections":movement_metrics["live_connections"],"realWssReauthentications":movement_metrics["live_reauthentications"],"controlledServiceLossMs":120000,"resumedGeneration":c.rejoin_transport.generation,"unrelatedPeerHealthy":true,"engineEvidence":false})).unwrap()).unwrap();}
 drop(movement);drop(health);let root=c.claims_root.clone();drop(c);std::fs::remove_dir_all(root).unwrap();
});}

#[path="next_connection_cr2_correction_test.rs"] mod correction;
#[path="next_waiting_composition_test.rs"]mod waiting_consumer;
#[path="next_startup_repair_composition_test.rs"]mod startup_repair;
#[path="next_entry_composition_test.rs"]mod entry_composition;
