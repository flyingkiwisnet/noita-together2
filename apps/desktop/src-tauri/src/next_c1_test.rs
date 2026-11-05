//! Real one-issuer TLS/production Core; only native engine facts/clock injected.
use super::*;
use std::{io::{BufRead,BufReader,Write},process::{Child,Command,Stdio},sync::mpsc};
#[cfg(feature="g3-c2-live")]
#[path="next_c3_test.rs"] mod c3;
// Console test workers are background processes, never interactive tools.
fn background_command(program:impl AsRef<std::ffi::OsStr>)->Command{
 let mut command=Command::new(program);
 #[cfg(windows)]{use std::os::windows::process::CommandExt;command.creation_flags(0x08000000);}
 command
}
#[test]
fn background_console_child_has_no_window_and_retains_output(){
 let output=background_command("powershell.exe").args(["-NoProfile","-Command", "Add-Type -TypeDefinition 'using System;using System.Runtime.InteropServices;public class ConsoleProbe {[DllImport(\"kernel32.dll\")]public static extern IntPtr GetConsoleWindow();}'; if ([ConsoleProbe]::GetConsoleWindow() -ne [IntPtr]::Zero){exit 7}; [Console]::Out.WriteLine('NO_WINDOW_STDOUT');[Console]::Error.WriteLine('NO_WINDOW_STDERR');exit 0"]).output().unwrap();
 assert!(output.status.success(),"hidden console child failed");assert!(String::from_utf8_lossy(&output.stdout).contains("NO_WINDOW_STDOUT"));assert!(String::from_utf8_lossy(&output.stderr).contains("NO_WINDOW_STDERR"));
}
struct Fixture{child:Child,events:mpsc::Receiver<Value>,origin:String,ca:String}
impl Fixture{
 fn start()->Self{
  Self::start_with(false)
 }
 fn start_with(live:bool)->Self{
  let repo=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..").canonicalize().unwrap();
  let mut command=background_command("C:/Program Files/nodejs/node.exe");command.current_dir(repo).args(["--import","tsx","apps/desktop/tools/g3-connection-fixture.mjs"]);if live{command.arg("--live");}
  if std::env::var("NT2_C3_COMPOSITION").as_deref()==Ok("1"){command.arg(if cfg!(feature="g3-rejoin"){"--rejoin"}else if cfg!(feature="g3-loot01"){"--loot01-runtime"}else if cfg!(feature="g3-seed-loot"){"--seed-loot-runtime"}else if cfg!(feature="g3-c2-potion-correction"){"--potion-correction"}else if cfg!(feature="g3-c2-potion-diagnostic"){"--potion-diagnostic"}else{"--c3"});}
  let mut child=command.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::inherit()).spawn().unwrap();
  let (tx,events)=mpsc::channel::<Value>();let output=child.stdout.take().unwrap();
  std::thread::spawn(move||{for line in BufReader::new(output).lines(){let line=line.unwrap();if let Ok(v)=serde_json::from_str(&line){let _=tx.send(v);}}});
  let ready=events.recv_timeout(Duration::from_secs(45)).unwrap();assert_eq!(ready["event"],"ready");
  Self{child,events,origin:ready["origin"].as_str().unwrap().into(),ca:ready["caPath"].as_str().unwrap().into()}
 }
 fn command(&mut self,command:&str){writeln!(self.child.stdin.as_mut().unwrap(),"{command}").unwrap();self.child.stdin.as_mut().unwrap().flush().unwrap();self.event(command);}
 fn trace(&mut self)->Value{writeln!(self.child.stdin.as_mut().unwrap(),"poll-trace").unwrap();self.child.stdin.as_mut().unwrap().flush().unwrap();loop{let item=self.events.recv_timeout(Duration::from_secs(12)).unwrap();if item["event"]=="poll-trace"{return item["trace"].clone();}}}
 fn event(&self,event:&str){loop{let item=self.events.recv_timeout(Duration::from_secs(12)).unwrap();if item["event"]==event{return;}}}
}
impl Drop for Fixture{fn drop(&mut self){let _=writeln!(self.child.stdin.as_mut().unwrap(),"quit");let _=self.child.wait();}}
fn native_g3(c:&mut Core<MemoryStore>,sequence:u64,protected:bool){
 native(c,sequence,protected,false);let mut pending=c.pending.lock().unwrap();let p=pending.as_mut().unwrap();
 if c.game.is_none(){p.owner.as_mut().unwrap().0=nt2_local_ws::owner::Id::OsProcess(std::process::id());}
 let report=p.observation.as_mut().unwrap();report.observation.mod_version="g3-simulated".into();report.observation.bridge_version="g3-simulated".into();report.digest=report.observation.content_digest().unwrap();
}
fn attach_metadata(c:&mut Core<MemoryStore>){
 let mut request=c.request(false);request.game_attach=Some(wire::GameAttach{game_instance_id:Some(uuid::Uuid::new_v4().to_string()),game_version:Some("g3-simulated".into()),mod_version:Some("g3-simulated".into()),bridge_version:Some("g3-simulated".into()),capabilities:Some(vec!["core.seed".into(),"core.ghost".into(),"core.hud".into(),"core.poi".into()]),preparation_id:c.preparation.as_ref().unwrap().preparation_id.clone()});
 c.attach_retry=Some(Retry{request,until:Instant::now()+Duration::from_secs(30)});
}
fn pose(c:&Core<MemoryStore>,sequence:u32){
 let game=c.game.as_ref().unwrap();let mut bytes=[0u8;64];bytes[..8].copy_from_slice(&[78,84,71,51,1,0,1,0]);bytes[8..24].copy_from_slice(&game.owner.1);bytes[24..28].copy_from_slice(&sequence.to_le_bytes());
 let reply=c.g3.as_ref().unwrap().exchange.lock().unwrap().transition(game.owner.0,&bytes,true);assert_eq!(reply[5],1);
}
#[test]
#[ignore="explicit disposable TLS/production Core health/token integration"]
fn c1_health_optional_lock_renewal_and_retirement(){
 tauri::async_runtime::block_on(async{
  let mut fixture=Fixture::start();let mut host=core();let mut peer=core();
  let ca=reqwest::Certificate::from_pem(&std::fs::read(&fixture.ca).unwrap()).unwrap();
  let dir=std::path::PathBuf::from("C:/NT2-Source-Example/spikes/noita-entry/.build/g3-connection-c1-20261009-01/fixture");std::fs::create_dir_all(&dir).unwrap();
  for (index,c) in [&mut host,&mut peer].into_iter().enumerate(){
   c.http=reqwest::Client::builder().use_native_tls().tls_certs_only(vec![ca.clone()]).no_proxy().redirect(reqwest::redirect::Policy::none()).timeout(Duration::from_secs(3)).build().unwrap();
   c.enable_g3(&dir.join(format!("journal-{index}-{}.jsonl",uuid::Uuid::new_v4())),Arc::new(Mutex::new(nt2_desktop_g0::g3_runtime::Exchange::default()))).unwrap();c.social_fixture=true;
   *c.expected_game.lock().unwrap()=Some(crate::pipe::verified_process_path(&std::env::current_exe().unwrap()).unwrap());
   c.select(&fixture.origin,"",false).await.unwrap();registered_as(c,if index==0{"alex"}else{"sam"}).await;
  }
  host.create_fixture(false).await.unwrap();peer.join(host.room.as_ref().unwrap().room_id.as_deref().unwrap(),None).await.unwrap();
  for c in [&mut host,&mut peer]{room(c).await;c.ready(true).await.unwrap();}
  for c in [&mut host,&mut peer]{room(c).await;c.prepare().await.unwrap();native_g3(c,1,true);attach_metadata(c);c.game_tick().await.unwrap();}
  for c in [&mut host,&mut peer]{room(c).await;c.collect_inspection().await.unwrap();c.collect_inspection().await.unwrap();}
  host.final_start(vec![host.desktop.as_ref().unwrap().id.clone(),peer.desktop.as_ref().unwrap().id.clone()],vec![]).await.unwrap();
  for c in [&mut host,&mut peer]{room(c).await;c.game_tick().await.unwrap();native_g3(c,2,false);c.game_tick().await.unwrap();c.g3_sync().unwrap();c.social_fixture_sync().await.unwrap();}
  let original_token=zeroize::Zeroizing::new(host.game.as_ref().unwrap().session.access.as_str().to_string());
  let game_id=host.game.as_ref().unwrap().session.id.clone();let generation=host.generation.load(Ordering::Acquire);
  let id=host.social_owner.as_mut().unwrap().begin(nt2_desktop_g0::protocol_g3_social::wire::Publish{hud:Some(nt2_desktop_g0::protocol_g3_social::wire::Hud{hp_milli:Some(1),max_hp_milli:Some(2),sampo:Some(false),hp_known:Some(true),sampo_known:Some(true),..Default::default()}),..Default::default()}).unwrap();
  let original=host.social_owner.as_ref().unwrap().original(&id).unwrap().to_vec();
  host.social_owner.as_mut().unwrap().send_original(&id).await.unwrap();
  let binding=host.g3_binding().unwrap().unwrap();
  let mut old_social=nt2_desktop_g0::g3_social::Owner::new(binding,game_id.clone(),original_token.clone(),host.http.clone(),host.generation.clone()).unwrap();old_social.discover().await.unwrap();
  let mut movement=host.movement_runner().unwrap();pose(&host,1);movement.pump().await.unwrap();let sent_before=host.g3_view()["movement"]["publications"].as_u64().unwrap();assert!(sent_before>0);
  let mut health=host.health_runner();let mut peer_health=peer.health_runner();
  // A real UI Core mutex is held longer than the original12s native lease.
  let pending=host.pending.clone();let ui=Arc::new(tokio::sync::Mutex::new(host));
  let ui_task=ui.clone();let slow_ui=tokio::spawn(async move{let mut guard=ui_task.lock().await;
   for mode in ["500","capacity","malformed","slow","timeout"]{
    fixture.command(&format!("optional:{mode}"));guard.next_work=Instant::now();assert!(guard.tick().await.is_err()||mode=="slow");
    assert!(guard.room_page(false).await.is_err()||mode=="slow");
    let mut request=guard.request(false);request.room_get=Some(wire::Empty{});
    assert!(guard.rpc(&request,false,false).await.is_err()||mode=="slow");
    if mode=="slow"{guard.social_sync().await.unwrap();assert!(guard.social_owner.as_mut().unwrap().snapshot("").await.is_ok());}
   }
   tokio::time::sleep(Duration::from_secs(13)).await;fixture.command("optional:none");
   drop(guard);fixture
  });
  let health_work=async{
   let until=Instant::now()+Duration::from_secs(23);let mut cycles=0;
   while Instant::now()<until||!slow_ui.is_finished(){
    pending.lock().unwrap().as_mut().unwrap().last_seen=Some(Instant::now());peer.pending.lock().unwrap().as_mut().unwrap().last_seen=Some(Instant::now());
    health.pump().await.unwrap();peer_health.pump().await.unwrap();cycles+=1;
    assert!(pending.lock().unwrap().as_ref().unwrap().release);tokio::time::sleep(Duration::from_secs(2)).await;
   }assert!(cycles>=10);println!("C1 independent health cycles={cycles} across optional faults/slow UI");
  };
  health_work.await;let mut fixture=slow_ui.await.unwrap();
  let mut host=Arc::try_unwrap(ui).ok().unwrap().into_inner();host.sync_health();peer.sync_health();
  // Actual issuer clock crosses original15min expiry, with fresh presence/pulses.
  for _ in 0..46{
   fixture.command("advance:20000");health.renew_for_controlled_clock();peer_health.renew_for_controlled_clock();
   for c in [&mut host,&mut peer]{c.pending.lock().unwrap().as_mut().unwrap().last_seen=Some(Instant::now());}
   health.pump().await.unwrap();peer_health.pump().await.unwrap();host.sync_health();peer.sync_health();
  }
  assert_eq!(host.game.as_ref().unwrap().session.id,game_id);assert_eq!(host.generation.load(Ordering::Acquire),generation);
  assert_eq!(host.social_owner.as_ref().unwrap().original(&id).unwrap(),original);assert_ne!(host.game.as_ref().unwrap().session.access.as_str(),original_token.as_str());
  // The production social pump revalidates current Core authority before each
  // capture. Exercise that same path, including pause recovery, not raw resume.
  host.social_sync().await.unwrap();
  assert_eq!(host.social_owner.as_ref().unwrap().original(&id).unwrap(),original);
  tokio::time::sleep(Duration::from_millis(110)).await;host.social_owner.as_mut().unwrap().send_original(&id).await.unwrap();
  pose(&host,2);movement.pump().await.unwrap();assert!(host.g3_view()["movement"]["publications"].as_u64().unwrap()>sent_before);
  assert!(matches!(old_social.snapshot("").await,Err(error) if error=="UNAUTHENTICATED"));
  let mut request=host.request(true);request.game_pulse=Some(wire::GamePulse{attachment_id:Some(host.game.as_ref().unwrap().session.attachment.clone()),game_instance_id:Some(host.game.as_ref().unwrap().instance.clone()),world_token:Some(hex(&host.game.as_ref().unwrap().owner.1))});
  assert!(matches!(host.rpc_as(&request,true,false,Some(&SecretString::from(original_token.as_str()))).await,Err(error) if matches!(error.as_str(),"UNAUTHORIZED"|"UNAUTHENTICATED"|"EXPIRED")));
  // Genuine End fences an already-issued positive release response.
  fixture.command("hold-game-next");let pending=host.pending.clone();
  let retire=tokio::spawn(async move{tokio::time::sleep(Duration::from_millis(200)).await;host.end().await.unwrap();host});
  let late=health.pump().await;let host=retire.await.unwrap();assert!(late.is_err());assert!(pending.lock().unwrap().is_none());
  assert!(host.social_owner.is_none());assert!(host.pending.lock().unwrap().is_none());
  assert!(peer_health.pump().await.is_err());peer.sync_health();
  assert!(peer.pending.lock().unwrap().as_ref().is_none_or(|p|!p.release));
  println!("C1 original token refused after920s; renewed social original retained; late retirement fenced");
 });
}
#[test]fn c1_original_lease_and_contact_failures_still_protect(){
 for error in ["FORBIDDEN","UNAUTHORIZED","Renewed child context changed","Candidate response violates canonical schema"]{
  let (mut c,p,r)=release_case();c.apply_release(Some(r),&p).unwrap();c.protect_health_error(error);assert!(!c.pending.lock().unwrap().as_ref().unwrap().release);
 }
 for error in ["CAPACITY","Candidate HTTP refused","Candidate connection unavailable"]{
  let (mut c,p,r)=release_case();c.apply_release(Some(r),&p).unwrap();let until=c.pending.lock().unwrap().as_ref().unwrap().authority_until;
  c.protect_health_error(error);assert_eq!(c.pending.lock().unwrap().as_ref().unwrap().authority_until,until);assert!(c.pending.lock().unwrap().as_ref().unwrap().release);
 }
}
#[test]
#[ignore="explicit disposable TLS/production Core WSS integration; engine facts injected"]
fn c2_core_tls_live_remote_receipt_renewed_health_reauth_and_end(){
 tauri::async_runtime::block_on(async{
  let mut fixture=Fixture::start_with(true);let mut host=core();let mut peer=core();
  let ca=reqwest::Certificate::from_pem(&std::fs::read(&fixture.ca).unwrap()).unwrap();
  let dir=std::path::PathBuf::from("C:/NT2-Source-Example/spikes/noita-entry/.build/g3-connection-c2-20261010-01/fixture");std::fs::create_dir_all(&dir).unwrap();
  for (index,c) in [&mut host,&mut peer].into_iter().enumerate(){
   c.http=reqwest::Client::builder().use_native_tls().tls_certs_only(vec![ca.clone()]).no_proxy().redirect(reqwest::redirect::Policy::none()).timeout(Duration::from_secs(3)).build().unwrap();
   c.enable_g3(&dir.join(format!("live-journal-{index}-{}.jsonl",uuid::Uuid::new_v4())),Arc::new(Mutex::new(nt2_desktop_g0::g3_runtime::Exchange::default()))).unwrap();c.enable_live_transport().unwrap();c.social_fixture=true;
   *c.expected_game.lock().unwrap()=Some(crate::pipe::verified_process_path(&std::env::current_exe().unwrap()).unwrap());
   c.select(&fixture.origin,"",false).await.unwrap();registered_as(c,if index==0{"alex"}else{"sam"}).await;
  }
  host.create_fixture(false).await.unwrap();peer.join(host.room.as_ref().unwrap().room_id.as_deref().unwrap(),None).await.unwrap();
  for c in [&mut host,&mut peer]{room(c).await;c.ready(true).await.unwrap();}
  for c in [&mut host,&mut peer]{room(c).await;c.prepare().await.unwrap();native_g3(c,1,true);attach_metadata(c);c.game_tick().await.unwrap();}
  for c in [&mut host,&mut peer]{room(c).await;c.collect_inspection().await.unwrap();c.collect_inspection().await.unwrap();}
  host.final_start(vec![host.desktop.as_ref().unwrap().id.clone(),peer.desktop.as_ref().unwrap().id.clone()],vec![]).await.unwrap();
  for c in [&mut host,&mut peer]{room(c).await;c.game_tick().await.unwrap();native_g3(c,2,false);c.game_tick().await.unwrap();c.g3_sync().unwrap();}
  let original_until=host.pending.lock().unwrap().as_ref().unwrap().authority_until;
  let mut health=host.health_runner();let mut peer_health=peer.health_runner();let mut movement=host.movement_runner().unwrap();let mut peer_movement=peer.movement_runner().unwrap();
  health.pump().await.unwrap();peer_health.pump().await.unwrap();host.sync_health();peer.sync_health();
  // JWT expiry uses whole seconds. Let a genuine later mint increase its
  // deadline without shifting service timestamps used by sample-age checks.
  tokio::time::sleep(Duration::from_millis(1100)).await;
  // Hold actual encrypted admission bytes while the real issuer renews the
  // same child session. The current server keeps older child tokens until
  // their original expiry; admission must still converge to the current token.
  fixture.command("hold-auth-next");movement.pump().await.unwrap();fixture.event("held-auth");
  health.renew_for_controlled_clock();peer_health.renew_for_controlled_clock();for c in [&mut host,&mut peer]{c.pending.lock().unwrap().as_mut().unwrap().last_seen=Some(Instant::now());}
  health.pump().await.unwrap();peer_health.pump().await.unwrap();host.sync_health();peer.sync_health();
  let stop=Arc::new(std::sync::atomic::AtomicBool::new(false));let producer_stop=stop.clone();
  let sources=[&host,&peer].map(|c|(c.g3.as_ref().unwrap().exchange.clone(),c.game.as_ref().unwrap().owner));
  let producer=tokio::spawn(async move{let mut sequence=0u32;while !producer_stop.load(Ordering::Acquire){sequence+=1;for(exchange,owner)in &sources{let mut bytes=[0;64];bytes[..8].copy_from_slice(b"NTG3\x01\x00\x01\x00");bytes[8..24].copy_from_slice(&owner.1);bytes[24..28].copy_from_slice(&sequence.to_le_bytes());assert_eq!(exchange.lock().unwrap().transition(owner.0,&bytes,true)[5],1);}tokio::time::sleep(Duration::from_millis(100)).await;}});
  let began=Instant::now();let mut next_health=began;let mut renewed=false;let mut remote=0;let mut gaps=vec![];let mut last=None;let mut delay_stage=0;let mut reconnected=false;
  while began.elapsed()<Duration::from_secs(23){
   let wanted=if began.elapsed().as_secs()<6{50}else if began.elapsed().as_secs()<11{100}else{200};
   if delay_stage!=wanted{fixture.command(&format!("rtt:{wanted}"));delay_stage=wanted;}
   if !reconnected&&began.elapsed()>Duration::from_secs(17){assert_eq!(host.g3_view()["movement"]["live_connections"],1);fixture.command("disconnect-live");reconnected=true;}
   if Instant::now()>=next_health{
    for c in [&mut host,&mut peer]{c.pending.lock().unwrap().as_mut().unwrap().last_seen=Some(Instant::now());}
    if !renewed&&began.elapsed()>Duration::from_secs(6){health.renew_for_controlled_clock();peer_health.renew_for_controlled_clock();renewed=true;}
    health.pump().await.unwrap();peer_health.pump().await.unwrap();host.sync_health();peer.sync_health();next_health=Instant::now()+Duration::from_secs(2);
   }
   if let Err(error)=movement.pump().await{println!("C2 host live event: {error}");}if let Err(error)=peer_movement.pump().await{println!("C2 peer live event: {error}");}
   let mut exchange=host.g3.as_ref().unwrap().exchange.lock().unwrap();let game=host.game.as_ref().unwrap();let mut poll=[0;64];poll[..8].copy_from_slice(b"NTG3\x01\x00\x02\x00");poll[8..24].copy_from_slice(&game.owner.1);let reply=exchange.transition(game.owner.0,&poll,true);
   if reply[6]>0{let received=u32::from_le_bytes(reply[584..588].try_into().unwrap());if received>remote{let at=Instant::now();if let Some(previous)=last{gaps.push(at.duration_since(previous).as_millis() as u64);}last=Some(at);remote=received;}}
   drop(exchange);tokio::time::sleep(Duration::from_millis(20)).await;
  }
  stop.store(true,Ordering::Release);producer.await.unwrap();let metrics=host.g3_view()["movement"].clone();println!("C2 fixed metrics: {metrics}");assert!(Instant::now()>original_until);assert!(remote>=190,"actual remote peer sequence delivered: {remote}");assert_eq!(metrics["live_connections"],2);assert_eq!(metrics["live_reauthentications"],2);assert!(metrics["receives"].as_u64().unwrap()>190);
  assert_eq!(metrics["live_admission_renewal_retries"],0);assert!(metrics["receive_gap_p95_ms"].as_u64().unwrap()<=250);assert!(metrics["remote_age_p95_ms"].as_u64().unwrap()<=400);
  println!("C2 TLS/Core live RTT=50/100/200ms+jitter remote_sequence={remote} receives={} connections={} reauth={} actual_receive_gap_p50_ms={} actual_receive_gap_p95_ms={} remote_age_p50_ms={} remote_age_p95_ms={} beyond_original_native_lease=true",metrics["receives"],metrics["live_connections"],metrics["live_reauthentications"],metrics["receive_gap_p50_ms"],metrics["receive_gap_p95_ms"],metrics["remote_age_p50_ms"],metrics["remote_age_p95_ms"]);
  println!("C2 same-session credential renewed during actual delayed admission; original child token retained until original expiry; current-token reauthentication verified");
  host.end().await.unwrap();assert!(host.pending.lock().unwrap().is_none());movement.pump().await.unwrap();
  assert!(peer_health.pump().await.is_err());peer.sync_health();peer_movement.pump().await.unwrap();assert_eq!(peer.g3_view()["native_playing"],false);
 });
}
#[test]fn c1_checkpoint_is_cancelled_on_world_or_session_replacement(){
 let (mut c,_,_)=release_case();let _runner=c.health_runner();assert!(c.health_active());let nonce=c.health_nonce.load(Ordering::Acquire);
 c.clear_game();assert!(c.health_nonce.load(Ordering::Acquire)>nonce);assert!(!c.health_active());c.sync_health();assert!(c.game.is_none());assert!(c.pending.lock().unwrap().is_none());
}
#[test]fn c1_expiry_wrong_world_and_generation_protect_without_network(){
 tauri::async_runtime::block_on(async{
  for fault in 0..5{
   let (mut c,dir)=g3_playing_case();
   assert!(matches!(c.g3_binding(),Ok(Some(_))));
   match fault{
    0=>c.pending.lock().unwrap().as_mut().unwrap().authority_until=Instant::now(),
    1=>c.pending.lock().unwrap().as_mut().unwrap().last_seen=Some(Instant::now()-Duration::from_secs(13)),
    2=>c.pending.lock().unwrap().as_mut().unwrap().owner.as_mut().unwrap().1=[9;16],
    3=>{c.generation.fetch_add(1,Ordering::AcqRel);},
    _=>c.g3_release.as_mut().unwrap().content_digest=Some("0".repeat(64)),
   }
   if matches!(fault,1|2|3){assert!(c.game_tick().await.is_err());}
   assert!(!matches!(c.g3_binding(),Ok(Some(_))));
   drop(c);std::fs::remove_dir_all(dir).unwrap();
  }
 });
}
