//! Explicit test-only engine/profile arming. Core owns real proof-bound credentials,
//! attachment/report/inspection/Start requests and derives gameplay authority itself.
use super::*;
#[path="next_inspection_test.rs"] mod inspection;
use std::{io::{BufRead,BufReader,Write},process::{Child,Command,Stdio},sync::mpsc};
struct ServiceFixture{child:Child,events:mpsc::Receiver<String>,origin:String,ca:std::path::PathBuf,namespace:String}
impl ServiceFixture{
 fn start(repo:&std::path::Path,social:bool)->Self{
  let mut command=Command::new("C:/Program Files/nodejs/node.exe");command.current_dir(repo)
   .args(["--import","tsx","apps/desktop/tools/g3-joined-fixture.mjs"]).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::inherit());
  if !social&&std::env::var("NT2_G3_A1_SOURCE_TEST").as_deref()==Ok("1"){command.arg("--a1-primary");}
  if social{command.arg(if std::env::var("NT2_G3_SOCIAL_SOURCE_TEST").as_deref()==Ok("1"){"--native-social"}else{"--social"});}
  if std::env::var("NT2_G3_MOVEMENT_GRANT_TEST").as_deref()==Ok("1"){command.arg("--movement-trace");}
  if social&&std::env::var("NT2_G3_SOCIAL_SOURCE_TEST").as_deref()==Ok("1")||!social&&std::env::var("NT2_G3_A1_SOURCE_TEST").as_deref()==Ok("1"){
   if cfg!(feature="g3-a1-runtime-diagnostic"){command.arg("--a1-runtime-diagnostic");}else if cfg!(feature="g3-a1-runtime-fix"){command.arg("--a1-runtime-fix");}
  }
  use std::os::windows::process::CommandExt;command.creation_flags(0x08000000);
  let mut child=command.spawn().unwrap();let output=child.stdout.take().unwrap();let (send,receive)=mpsc::channel();
  std::thread::spawn(move||{for line in BufReader::new(output).lines(){if let Ok(line)=line{let _=send.send(line);}}});
  let ready:Value=serde_json::from_str(&receive.recv_timeout(Duration::from_secs(40)).unwrap()).unwrap();
  assert_eq!(ready["event"],"ready");assert_eq!(ready["evidenceStatus"],"simulated-engine-profile-only");
  Self{child,events:receive,origin:ready["origin"].as_str().unwrap().into(),ca:ready["caPath"].as_str().unwrap().into(),namespace:ready["namespace"].as_str().unwrap().into()}
 }
 fn event(&self)->Value{serde_json::from_str(&self.events.recv_timeout(Duration::from_secs(5)).unwrap()).unwrap()}
 fn command(&mut self,name:&str){writeln!(self.child.stdin.as_mut().unwrap(),"{name}").unwrap();assert_eq!(self.event()["event"],name);}
}
impl Drop for ServiceFixture{fn drop(&mut self){if let Some(input)=self.child.stdin.as_mut(){let _=writeln!(input,"quit");}self.child.stdin.take();
 let end=Instant::now()+Duration::from_secs(15);while Instant::now()<end{if self.child.try_wait().ok().flatten().is_some(){return}std::thread::sleep(Duration::from_millis(25));}
 let _=self.child.kill();let _=self.child.wait();}}
#[derive(Default)]
struct NativeHarness{children:Vec<Child>,workers:Vec<(Arc<std::sync::atomic::AtomicBool>,std::thread::JoinHandle<()>)>}
impl Drop for NativeHarness{fn drop(&mut self){for child in &mut self.children{if child.try_wait().ok().flatten().is_none(){let _=child.kill();}let _=child.wait();}
 for (stop,worker) in self.workers.drain(..){stop.store(true,Ordering::Release);let _=worker.join();}}}
fn arm_fixture_only(c:&mut Core<MemoryStore>,dir:&std::path::Path,runner:&std::path::Path){
 let room=c.room.as_ref().unwrap();let mut p=handoff::new(424242,room.run_id.clone().unwrap(),u64::from(room.config_revision.unwrap()));
 p.epoch=room.run_epoch.clone().unwrap();p.native_v2=true;p.authority_until=Instant::now()+Duration::from_secs(12);
 p.generation=Some((c.generation.clone(),c.generation.load(Ordering::Acquire)));p.claim_path=Some(dir.join("claims/world.bin"));
 p.preparation_id=c.preparation.as_ref().unwrap().preparation_id.clone();
 *c.pending.lock().unwrap()=Some(p);*c.expected_game.lock().unwrap()=Some(runner.to_path_buf());
}
fn fixture_metadata_only(c:&mut Core<MemoryStore>){
 // Simulated profile only. Production Load game remains refused; no native approval.
 let mut request=c.request(false);request.game_attach=Some(wire::GameAttach{game_instance_id:Some(uuid::Uuid::new_v4().to_string()),
  game_version:Some(if c.social_requested||std::env::var("NT2_G3_A1_SOURCE_TEST").as_deref()==Ok("1"){"Noita-Jan25-2025"}else{"g3-simulated"}.into()),mod_version:Some(if c.social_requested{"nt2-g3-social-dev-1"}else if std::env::var("NT2_G3_A1_SOURCE_TEST").as_deref()==Ok("1"){"nt2-g3-dev-1"}else{"g3-simulated"}.into()),bridge_version:Some(if c.social_requested{"g3-social-transport-1"}else if std::env::var("NT2_G3_A1_SOURCE_TEST").as_deref()==Ok("1"){"g3-transport-1"}else{"g3-simulated"}.into()),
  capabilities:Some({let mut caps=vec!["core.seed".into(),"core.ghost".into()];if c.social_fixture||c.social_requested{caps.extend(["core.hud".into(),"core.poi".into()]);}caps}),preparation_id:c.preparation.as_ref().unwrap().preparation_id.clone()});
 c.attach_retry=Some(Retry{request,until:Instant::now()+Duration::from_secs(30)});
}
fn mark(dir:&std::path::Path,name:&str){std::fs::write(dir.join(name),b"fixture command").unwrap();}
fn wait_file(dir:&std::path::Path,name:&str){let end=Instant::now()+Duration::from_secs(10);while !dir.join(name).exists(){
 assert!(Instant::now()<end,"Native fixture timed out {name}: {}",dir.display());std::thread::sleep(Duration::from_millis(10));}}
#[test]
#[ignore="test-g3-joined.ps1 only; real Core/TLS/x86/pipe, simulated Noita/profile facts"]
fn joined_core_https_native_nonempty_ghosts(){
 assert_eq!(std::env::var("NT2_G3_JOINED_TEST").as_deref(),Ok("1"));
 joined(false);
}
#[test]
#[ignore="explicit Core/social simulated HTTPS/native observation/local Lua composition only"]
fn joined_core_social_https_local_hud_poi(){
 assert_eq!(std::env::var("NT2_G3_JOINED_TEST").as_deref(),Ok("1"));joined(true);
}
fn joined(social:bool){joined_mode(social,false)}
#[test]
#[ignore="explicit A1 source/native/verified TLS fixture; injected engine facts"]
fn joined_movement_lane_survives_slow_core_lock(){
 assert_eq!(std::env::var("NT2_G3_JOINED_TEST").as_deref(),Ok("1"));joined_mode(false,true);
}
#[test]
#[ignore="explicit A1 matched social movement/native/verified TLS fixture; injected engine facts"]
fn joined_social_movement_lane_survives_slow_core_lock(){
 assert_eq!(std::env::var("NT2_G3_JOINED_TEST").as_deref(),Ok("1"));
 assert_eq!(std::env::var("NT2_G3_SOCIAL_SOURCE_TEST").as_deref(),Ok("1"));
 joined_mode(true,true);
}
fn joined_mode(social:bool,detached:bool){
 joined_mode_reselect(social,detached,false);
}
#[test]
#[ignore="explicit A1 same-origin reselection/public Load/native/TLS regression; injected engine facts"]
fn joined_social_reselection_keeps_matched_native_inspection(){
 assert_eq!(std::env::var("NT2_G3_SOCIAL_SOURCE_TEST").as_deref(),Ok("1"));
 assert_eq!(std::env::var("NT2_G3_SOCIAL_NATIVE_TEST").as_deref(),Ok("1"));
 joined_mode_reselect(true,true,true);
}
#[test]
#[ignore="explicit private pipes/disposable TLS; production Core scheduler, injected engine facts"]
fn joined_social_production_scheduler_movement_grant(){
 assert_eq!(std::env::var("NT2_G3_MOVEMENT_GRANT_TEST").as_deref(),Ok("1"));
 assert!(cfg!(feature="g3-a1-runtime-diagnostic"));
 joined_mode_reselect(true,false,true);
}
#[test]
#[ignore="explicit private x86 workers/TLS; heartbeat wait/open race against normal social/gameplay workers"]
fn joined_social_heartbeat_availability_race(){
 assert_eq!(std::env::var("NT2_G3_HEARTBEAT_RACE_TEST").as_deref(),Ok("1"));
 assert_eq!(std::env::var("NT2_G3_MOVEMENT_GRANT_TEST").as_deref(),Ok("1"));
 joined_mode_reselect(true,false,true);
}
fn joined_mode_reselect(social:bool,detached:bool,reselect:bool){
 tauri::async_runtime::block_on(async{
  let runtime_stability=std::env::var("NT2_G3_RUNTIME_STABILITY_TEST").as_deref()==Ok("1");
  let movement_grant=std::env::var("NT2_G3_MOVEMENT_GRANT_TEST").as_deref()==Ok("1");
  let heartbeat_race=std::env::var("NT2_G3_HEARTBEAT_RACE_TEST").as_deref()==Ok("1");
  let social_native=social&&std::env::var("NT2_G3_SOCIAL_NATIVE_TEST").as_deref()==Ok("1");
  let source=(social_native&&std::env::var("NT2_G3_SOCIAL_SOURCE_TEST").as_deref()==Ok("1"))||(!social&&std::env::var("NT2_G3_A1_SOURCE_TEST").as_deref()==Ok("1"));
  let canonical=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..").canonicalize().unwrap();
  let repo=std::path::PathBuf::from(canonical.to_str().unwrap().trim_start_matches(r"\\?\"));
  let dir=repo.join("spikes/noita-entry/.build/g3-joined").join(uuid::Uuid::new_v4().to_string());std::fs::create_dir_all(&dir).unwrap();
  let mut service=ServiceFixture::start(&repo,social);let mut native=NativeHarness::default();
  let root=reqwest::Certificate::from_pem(&std::fs::read(&service.ca).unwrap()).unwrap();
  let mut host=core();let mut peer=core();
  let exchanges=[Arc::new(Mutex::new(nt2_desktop_g0::g3_runtime::Exchange::default())),Arc::new(Mutex::new(nt2_desktop_g0::g3_runtime::Exchange::default()))];
  let social_exchanges=[Arc::new(Mutex::new(nt2_desktop_g0::g3_social_stream::Exchange::default())),Arc::new(Mutex::new(nt2_desktop_g0::g3_social_stream::Exchange::default()))];
  for (index,c) in [&mut host,&mut peer].into_iter().enumerate(){
   c.social_fixture=social&&!source;
   if social_native{c.social_stream=Some(social_exchanges[index].clone());}
   std::fs::create_dir_all(dir.join(index.to_string())).unwrap();
   c.http=reqwest::Client::builder().tls_backend_native().tls_certs_only(vec![root.clone()]).no_proxy().redirect(reqwest::redirect::Policy::none()).timeout(Duration::from_secs(3)).build().unwrap();
   c.enable_g3(&dir.join(index.to_string()).join("journal.jsonl"),exchanges[index].clone()).unwrap();
   if source&&social{c.request_social(social_exchanges[index].clone()).unwrap();}
   c.select(&service.origin,"",false).await.unwrap();registered_as(c,if index==0{"alex"}else{"sam"}).await;
   if source{
    // Genuine original bytes copied read-only into OWNED FILE fixtures; never executed.
    let game=dir.join(index.to_string()).join("verified-files");std::fs::create_dir_all(&game).unwrap();
    let originals=if runtime_stability{repo.join("spikes/noita-entry/.build/g3-inspection-build19-cc26b9c694054c1dac73888617aefc3f/social-Steam")}else{std::path::PathBuf::from("C:/Noita")};
    for name in ["noita.exe","lua51.dll"]{std::fs::copy(originals.join(name),game.join(name)).unwrap();}
    let runtime=if social{nt2_desktop_g0::g3_social_installation::development_profile().unwrap()}else{nt2_desktop_g0::g3_installation::profile().unwrap()};for mapping in runtime["runtimeMappings"].as_array().unwrap(){let destination=game.join(mapping["path"].as_str().unwrap());std::fs::create_dir_all(destination.parent().unwrap()).unwrap();
     let source_path=mapping["sourcePath"].as_str().unwrap();
     std::fs::copy(repo.join(source_path),destination).unwrap();}
    c.configure_game(game.to_str().unwrap()).unwrap();if social{assert_eq!(c.social_view()["enabled"],true);}
    // Owned copied files only. Public Configure/Load revalidate bytes; metadata
    // assembled by this fixture cannot grant trust or waive the compiled closure.
    let mapping=&runtime["runtimeMappings"][0];let target=game.join(mapping["path"].as_str().unwrap());
    assert_ne!(target,game.join("noita.exe"));assert_ne!(target,game.join("lua51.dll"));
    let original=std::fs::read(&target).unwrap();std::fs::write(&target,b"A1 owned fixture drift").unwrap();
    assert!(c.configure_game(game.to_str().unwrap()).is_err());assert!(c.load_game().await.is_err());assert!(c.preparation.is_none());
    std::fs::write(&target,original).unwrap();
    if social{let approved=c.social_profile.clone().unwrap();let mut forged=approved.clone();forged["bridgeSha256"]=json!("0".repeat(64));c.social_profile=Some(forged);
     assert!(c.configure_game(game.to_str().unwrap()).is_err());c.social_profile=Some(approved);
    }else{let approved=c.info.as_ref().unwrap().profile.clone().unwrap();let mut forged=serde_json::to_value(&approved).unwrap();forged["bridgeSha256"]=json!("0".repeat(64));
     c.info.as_mut().unwrap().profile=Some(serde_json::from_value(forged).unwrap());assert!(c.configure_game(game.to_str().unwrap()).is_err());c.info.as_mut().unwrap().profile=Some(approved);
     let service=c.service.clone();c.service="nt2-g3-unreviewed-fixture".into();assert!(c.configure_game(game.to_str().unwrap()).is_err());c.service=service;
    }
    c.configure_game(game.to_str().unwrap()).unwrap();
    if reselect{
     c.select(&service.origin,"",false).await.unwrap();
     println!("A1_RESELECT_AFTER_VERIFIED enabled={}",c.social_enabled);
    }
   }
  }
  host.create_fixture(false).await.unwrap();let room_id=host.room.as_ref().unwrap().room_id.clone().unwrap();peer.join(&room_id,None).await.unwrap();
  room(&mut host).await;host.ready(true).await.unwrap();room(&mut peer).await;peer.ready(true).await.unwrap();
  room(&mut host).await;if source{host.load_game().await.unwrap();}else{host.prepare().await.unwrap();}room(&mut peer).await;if source{peer.load_game().await.unwrap();}else{peer.prepare().await.unwrap();}room(&mut host).await;
  if reselect{for c in [&mut host,&mut peer]{
   assert!(c.social_enabled);let preparation=c.preparation.as_ref().unwrap().preparation_id.clone();let generation=c.generation.load(Ordering::Acquire);
   c.select(&service.origin,"",false).await.unwrap();c.configure_game(c.game_path.clone().unwrap().to_str().unwrap()).unwrap();
   assert!(c.social_enabled);assert_eq!(c.preparation.as_ref().unwrap().preparation_id,preparation);assert_eq!(c.generation.load(Ordering::Acquire),generation);
  }}
  let runner=repo.join("spikes/noita-entry/.build/Lua51Runner.exe");let expected=crate::pipe::verified_process_path(&runner).unwrap();
  for (index,c) in [&mut host,&mut peer].into_iter().enumerate(){
   let own=dir.join(index.to_string());arm_fixture_only(c,&own,&expected);
   let stop=Arc::new(std::sync::atomic::AtomicBool::new(false));let diagnostics=Arc::new(Mutex::new(crate::pipe::NativeDiagnostics::default()));
   if runtime_stability{c.native_diagnostics=diagnostics.clone();}
   let worker=if movement_grant{crate::pipe::start_g3_social_named(format!("{}{}",std::env::var(if index==0{"NT2_G3_GRANT_HOST_PREFIX"}else{"NT2_G3_GRANT_PEER_PREFIX"}).unwrap(),crate::pipe::user_sid().unwrap()),c.pending.clone(),stop.clone(),c.expected_game.clone(),diagnostics,exchanges[index].clone(),social_exchanges[index].clone())}
    else if social_native{crate::pipe::start_g3_social_named(format!(r"\\.\pipe\nt2-g3-social-{}-v1-{}",if index==0&&reselect{"inspection-host"}else if index==0{"dev"}else{"peer"},crate::pipe::user_sid().unwrap()),c.pending.clone(),stop.clone(),c.expected_game.clone(),diagnostics,exchanges[index].clone(),social_exchanges[index].clone())}
    else if index==0{crate::pipe::start_g3(c.pending.clone(),stop.clone(),c.expected_game.clone(),diagnostics,exchanges[index].clone())}
    else{crate::pipe::start_g3_named(format!(r"\\.\pipe\nt2-g3-joined-peer-v1-{}",crate::pipe::user_sid().unwrap()),c.pending.clone(),stop.clone(),c.expected_game.clone(),diagnostics,exchanges[index].clone())}.unwrap();
   native.workers.push((stop,worker));
   let local=own.join("Local");std::fs::create_dir_all(local.join("NoitaTogether2/g3-dev")).unwrap();std::fs::create_dir_all(own.join("Roaming")).unwrap();
   std::fs::write(local.join("NoitaTogether2/g3-dev/bridge-app.txt"),crate::pipe::verified_process_path(&std::env::current_exe().unwrap()).unwrap().to_str().unwrap()).unwrap();
   let log=std::fs::File::create(own.join("native-output.txt")).unwrap();let dll=if movement_grant{std::env::var(if index==0{"NT2_G3_GRANT_HOST_DLL"}else{"NT2_G3_GRANT_PEER_DLL"}).unwrap().into()}else if reselect&&index==0{std::env::var("NT2_G3_INSPECTION_HOST_DLL").unwrap().into()}else if social_native&&index==0{repo.join("spikes/noita-entry/.build/g3-social/nt2_g3_social.dll")}else if index==0{repo.join("spikes/noita-entry/.build/g3-transport/nt2_g3_transport.dll")}else{std::env::var("NT2_G3_JOINED_PEER_DLL").unwrap().into()};
   let lua=if runtime_stability{own.join("verified-files/lua51.dll")}else{std::path::PathBuf::from(r"C:\Noita\lua51.dll")};
   let mut command=Command::new(&runner);command.arg(lua).arg(repo.join("apps/desktop/tools/g3-joined-native.lua"))
    .env("LOCALAPPDATA",local).env("APPDATA",own.join("Roaming")).env("NT2_G3_JOINED_DLL",dll).env("NT2_G3_JOINED_DIR",&own)
    .env("NT2_G3_MODULES",repo.join("mod/noita_together2/files/g3")).env("NT2_G3_JOINED_X",if index==0{"1"}else{"-7.25"}).stdout(log.try_clone().unwrap()).stderr(log);
   if social_native{command.env("NT2_G3_SOCIAL_NATIVE","1");}
   if source{command.env(if social{"NT2_G3_SOCIAL_SOURCE"}else{"NT2_G3_PRIMARY_SOURCE"},"1");}
   if runtime_stability{command.env("NT2_G3_RUNTIME_HARNESS",repo.join("apps/desktop/tools/g3-runtime-entry-harness.lua"));}
   native.children.push(command.spawn().unwrap());wait_file(&own,"protected");room(c).await;
   if reselect{let owner=c.pending.lock().unwrap().as_ref().unwrap().owner;c.select(&service.origin,"",false).await.unwrap();assert_eq!(c.pending.lock().unwrap().as_ref().unwrap().owner,owner);assert!(c.social_enabled);}
   if !reselect{fixture_metadata_only(c);}
   let attached=c.game_tick().await;
   if reselect{println!("A1_RESELECT_NATIVE_ATTACH enabled={} accepted={} error={}",c.social_enabled,attached.is_ok(),attached.as_ref().err().map(String::as_str).unwrap_or("none"));}
   attached.unwrap();
   assert!(!c.game.as_ref().unwrap().entered);c.g3_tick().await.unwrap();assert_eq!(c.g3_view()["native_playing"],false);
   if social{assert!(joined_social_sync(c).await.is_err());}
  }
  room(&mut host).await;room(&mut peer).await;for _ in 0..3{host.collect_inspection().await.unwrap();}
  assert_eq!(host.inspections.len(),2);
  host.final_start(vec![host.desktop.as_ref().unwrap().id.clone(),peer.desktop.as_ref().unwrap().id.clone()],vec![]).await.unwrap();
  room(&mut peer).await;
  if movement_grant{
   let stopped=Arc::new(std::sync::atomic::AtomicBool::new(false));let mut lanes=vec![];
   for mut runner in [host.movement_runner().unwrap(),peer.movement_runner().unwrap()]{let stop=stopped.clone();lanes.push(tauri::async_runtime::spawn(async move{
    let mut clock=tokio::time::interval(Duration::from_millis(20));clock.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    while !stop.load(Ordering::Acquire){clock.tick().await;if let Err(error)=runner.pump().await{println!("G3_GRANT_FIRST_HTTP_FAILURE {error}");}}
   }));}
   for index in 0..2{mark(&dir.join(index.to_string()),"start");mark(&dir.join(index.to_string()),"pump");}
   let mut snapshots=vec![];let began=Instant::now();let until=began+Duration::from_secs(20);let mut held=false;
   while Instant::now()<until{
    if !held&&began.elapsed()>Duration::from_secs(6){
     if heartbeat_race{for index in 0..2{mark(&dir.join(index.to_string()),"heartbeat-race");}}
     else{for c in [&mut host,&mut peer]{c.native_diagnostics.lock().unwrap().deny_next_heartbeat=true;}}
     held=true;
    }
    if heartbeat_race&&began.elapsed()>Duration::from_secs(12){for index in 0..2{mark(&dir.join(index.to_string()),"heartbeat-race-off");}}
    for c in [&mut host,&mut peer]{
     let control=c.tick().await;let social=c.social_scheduler_tick().await;
     snapshots.push(json!({"elapsed_ms":began.elapsed().as_millis(),"control_ok":control.is_ok(),"social_ok":social.is_ok(),"view":c.g3_view()}));
    }
    tokio::time::sleep(Duration::from_millis(200)).await;
   }
   stopped.store(true,Ordering::Release);for lane in lanes{lane.await.unwrap();}
   writeln!(service.child.stdin.as_mut().unwrap(),"movement-trace").unwrap();let request_trace=service.event();assert_eq!(request_trace["event"],"movement-trace");
   std::fs::write(dir.join("movement-grant-scheduler.json"),serde_json::to_vec_pretty(&json!({"actualNoita":false,"productionScheduler":true,"requestTrace":request_trace,"snapshots":snapshots})).unwrap()).unwrap();
   println!("G3_GRANT_EVIDENCE {}",dir.display());
   for c in [&host,&peer]{let view=c.g3_view();assert!(view["movement"]["publications"].as_u64().unwrap()>8,"No sustained acknowledged movement: {view}");assert!(view["movement"]["receives"].as_u64().unwrap()>8,"No sustained peer replies: {view}");}
   for index in 0..2{
    let own=dir.join(index.to_string());assert!(own.join("ghost").exists(),"No game-thread peer delivery");
    let phase=std::fs::read_to_string(own.join("runtime-phase-result")).unwrap();
    if heartbeat_race{
     let log=std::fs::read_to_string(own.join("native-output.txt")).unwrap();
     assert!(log.contains("op=5 stage=test-availability-race-injected"),"Heartbeat race not exercised");
     assert!(phase.starts_with("holds=0;"),"Availability race unnecessarily held the game: {phase}; evidence {}",dir.display());
     assert!(!log.contains("op=5 stage=pipe-busy"),"Heartbeat availability race exhausted its budget unexpectedly");
    }else{assert!(!phase.starts_with("holds=0;"),"No injected native heartbeat hold observed: {phase}");}
   }
   for c in [&host,&peer]{let view=c.g3_view();assert_eq!(view["native_playing"],true);assert_eq!(view["control"]["last_validated_phase"],"playing");}
   let before_end=[host.g3_view()["movement"].clone(),peer.g3_view()["movement"].clone()];
   room(&mut host).await;host.end().await.unwrap();room(&mut peer).await;peer.tick().await.unwrap();peer.g3_sync().unwrap();
   for mut runner in [host.movement_runner().unwrap(),peer.movement_runner().unwrap()]{runner.pump().await.unwrap();}
   for (index,c) in [&host,&peer].into_iter().enumerate(){assert_eq!(c.g3_view()["native_playing"],false);assert_eq!(c.g3_view()["movement"],before_end[index]);}
   return;
  }
  for c in [&mut host,&mut peer]{c.game_tick().await.unwrap();assert!(!c.game.as_ref().unwrap().entered);c.g3_tick().await.unwrap();assert_eq!(c.g3_view()["native_playing"],false);}
  for index in 0..2{mark(&dir.join(index.to_string()),"start");wait_file(&dir.join(index.to_string()),"playing");}
  for c in [&mut host,&mut peer]{c.game_tick().await.unwrap();assert!(c.game.as_ref().unwrap().entered);c.g3_sync().unwrap();}
  for index in 0..2{mark(&dir.join(index.to_string()),"pump");}
  let end=Instant::now()+Duration::from_secs(8);
  while !(dir.join("0/ghost").exists()&&dir.join("1/ghost").exists()){
   assert!(Instant::now()<end,"No joined nonempty ghosts: {}",dir.display());
   for c in [&mut host,&mut peer]{c.g3_tick().await.unwrap();}
   std::thread::sleep(Duration::from_millis(110));
  }
  assert_eq!(std::fs::read_to_string(dir.join("0/ghost")).unwrap(),"-7.25");assert_eq!(std::fs::read_to_string(dir.join("1/ghost")).unwrap(),"1");
  if runtime_stability{
   if std::env::var("NT2_G3_RUNTIME_HEARTBEAT_DELAY_TEST").as_deref()==Ok("1"){
    for c in [&mut host,&mut peer]{c.native_diagnostics.lock().unwrap().delay_next_heartbeat=true;}
   }
   let stopped=Arc::new(std::sync::atomic::AtomicBool::new(false));let mut lanes=vec![];
   for mut runner in [host.movement_runner().unwrap(),peer.movement_runner().unwrap()]{let stop=stopped.clone();lanes.push(tauri::async_runtime::spawn(async move{while !stop.load(Ordering::Acquire){runner.pump().await.unwrap();tokio::time::sleep(Duration::from_millis(20)).await;}}));}
   let until=Instant::now()+Duration::from_secs(6);let mut cycles=0;
   while Instant::now()<until{
    for c in [&mut host,&mut peer]{presence(c).await;room(c).await;c.game_tick().await.unwrap();c.g3_sync().unwrap();c.social_scheduler_tick().await.unwrap();assert_eq!(c.g3_view()["native_playing"],true);assert!(c.pending.lock().unwrap().as_ref().unwrap().release);}
    cycles+=1;tokio::time::sleep(Duration::from_millis(200)).await;
   }
   service.command("delay-social-next");peer.social_next=Instant::now();
   let delayed=std::thread::spawn(move||{assert_eq!(service.event()["kind"],"social");let began=Instant::now();std::thread::sleep(Duration::from_millis(650));service.command("release");(service,began.elapsed().as_millis())});
   peer.social_scheduler_tick().await.unwrap();let measured=delayed.join().unwrap();service=measured.0;let delayed_social_ms=measured.1;assert!(delayed_social_ms>=650&&delayed_social_ms<2000);
   stopped.store(true,Ordering::Release);for lane in lanes{lane.await.unwrap();}
   for i in 0..2{
    let text=std::fs::read_to_string(dir.join(i.to_string()).join("runtime-phase-result")).unwrap();assert!(text.starts_with("holds=0;"),"Unexpected real entry hold: {text}");
    let text=std::fs::read_to_string(dir.join(i.to_string()).join("runtime-ghost-result")).unwrap();assert_eq!(text,"active=1;removed=0","Peer ghost lost during valid authority: {text}");
   }
   std::fs::write(dir.join("runtime-stability.json"),serde_json::to_vec_pretty(&json!({"actualNoita":false,"actualEntryPhaseCallbacks":true,"publicCoreConfigureLoad":true,"installationVerificationUsesReviewedRuntime":true,"originalFingerprintsFromOwnedCopies":true,"runtimeFixConsumer":cfg!(feature="g3-a1-runtime-fix"),"newEntryRequiresSeparateRuntimeReview":!cfg!(feature="g3-a1-runtime-fix"),"stableCycles":cycles,"stableWindowMs":6000,"delayedSocialCaptureMs":delayed_social_ms,"unexpectedHolds":0,"initialPeerGhosts":2,"peerGhostsRetainedWithoutRemoval":true,"socialConsumerSchedulerEnabled":true,"actualHudProducerImplemented":false})).unwrap()).unwrap();
  }
  if detached{
   // Production MovementRunner: actual Core mutex held for two seconds, no
   // control/Core methods called by either network lane during that interval.
   let before=host.g3_view()["movement"]["receives"].as_u64().unwrap();
   let blocked=Arc::new(tokio::sync::Mutex::new(host));
   let mut lanes=Vec::new();let stopped=Arc::new(std::sync::atomic::AtomicBool::new(false));
   for mut runner in [blocked.lock().await.movement_runner().unwrap(),peer.movement_runner().unwrap()]{
    let stop=stopped.clone();lanes.push(tauri::async_runtime::spawn(async move{
     let mut clock=tokio::time::interval(Duration::from_millis(20));clock.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
     while !stop.load(Ordering::Acquire){clock.tick().await;runner.pump().await.unwrap();}
    }));
   }
   {let _slow_control=blocked.lock().await;tokio::time::sleep(Duration::from_secs(2)).await;}
   stopped.store(true,Ordering::Release);for lane in lanes{lane.await.unwrap();}
   host=Arc::try_unwrap(blocked).ok().unwrap().into_inner();
   let metrics=host.g3_view()["movement"].clone();
   assert!(metrics["receives"].as_u64().unwrap()-before>=8,"Independent movement did not progress: {metrics}");
   assert!(metrics["max_receive_gap_ms"].as_u64().unwrap()<500,"Unbounded movement receive gap: {metrics}");
   std::fs::write(dir.join("a1-movement-timing.json"),serde_json::to_vec_pretty(&json!({"actualNoita":false,"coreLockHeldMs":2000,"independentRunner":true,"host":metrics,"peer":peer.g3_view()["movement"]})).unwrap()).unwrap();
  }
  if social&&!detached&&!runtime_stability{
   compose_social(&mut host,&mut peer,&repo,&dir).await;
   service.command("delay-social-next");let generation=peer.generation.clone();
   let fault=std::thread::spawn(move||{assert_eq!(service.event()["kind"],"social");generation.fetch_add(1,Ordering::AcqRel);service.command("release");service});
   assert!(peer.social_owner.as_mut().unwrap().snapshot("").await.is_err());service=fault.join().unwrap();assert!(joined_social_sync(&mut peer).await.is_err());
  }
  // Pause the consumer before End; subsequent old same-context poses cannot recreate.
  // The longer stability fixture also needs the normal desktop presence lane.
  // Refresh through public Core/service BEFORE reading a new room snapshot.
  if runtime_stability{for c in [&mut host,&mut peer]{presence(c).await;}}
  room(&mut host).await;host.game_tick().await.unwrap(); // Renew through actual Core/service, not a fixture lease write.
  let fault_state=|c:&Core<MemoryStore>|{let p=c.pending.lock().unwrap();let p=p.as_ref().unwrap();json!({"nativeAuthorityValid":c.native_authority_valid(p),"release":p.release,"accepted":p.accepted,"loaded":p.loaded,"authorityRemainingMs":p.authority_until.saturating_duration_since(Instant::now()).as_millis(),"lastSeenAgeMs":p.last_seen.map(|at|at.elapsed().as_millis()),"generationChanged":p.generation.as_ref().is_some_and(|(g,n)|g.load(Ordering::Acquire)!=*n),"entered":c.game.as_ref().map(|g|g.entered),"nativeRevoked":c.native_revoked,"memberPresent":c.room.as_ref().and_then(|r|r.members.as_ref()).map(|ms|ms.iter().filter(|m|m.desktop_id==c.desktop.as_ref().map(|d|d.id.clone())).map(|m|m.present).collect::<Vec<_>>()),"protected":p.observation.as_ref().map(|r|r.observation.protected),"reportAcknowledged":p.observation.as_ref().is_some_and(|r|c.game.as_ref().is_some_and(|g|g.reported>=r.sequence))})};
  std::fs::write(dir.join("late-fault-before-hold.json"),serde_json::to_vec_pretty(&fault_state(&host)).unwrap()).unwrap();
  assert!(host.g3_binding().unwrap().is_some(),"Late fault requires a valid playing binding before pausing the fixture consumer");
  for index in 0..2{mark(&dir.join(index.to_string()),"hold");wait_file(&dir.join(index.to_string()),"cleared");}
  std::thread::sleep(Duration::from_millis(220));exchanges[0].lock().unwrap().take_pose();
  assert!(host.g3_binding().unwrap().is_some(),"Late fault must start under valid unchanged authority");
  service.command("delay-next");let generation=host.generation.clone();
  let fault=std::thread::spawn(move||{
   let held=service.event();assert_eq!(held["event"],"held");assert_eq!(held["kind"],"ghosts");generation.fetch_add(1,Ordering::AcqRel);
   std::thread::sleep(Duration::from_millis(150));service.command("release");service
  });
  let fault_until=Instant::now()+Duration::from_secs(3);
  let mut generation_retired=false;
  let late=loop{assert!(Instant::now()<fault_until,"No bounded ghost fault request");let result=host.g3_tick().await;if result.is_err(){break result;}std::thread::sleep(Duration::from_millis(110));
   if host.g3_binding().unwrap().is_none(){
    let changed={let p=host.pending.lock().unwrap();let p=p.as_ref().unwrap();p.generation.as_ref().is_some_and(|(g,n)|g.load(Ordering::Acquire)!=*n)};
    if !changed{std::fs::write(dir.join("late-fault-unexpected-retirement.json"),serde_json::to_vec_pretty(&fault_state(&host)).unwrap()).unwrap();}
    assert!(changed,"Fault may retire only its explicitly changed generation");
    generation_retired=true;break Ok(());
   }
  };service=fault.join().unwrap();
  assert!(host.pending.lock().unwrap().as_ref().unwrap().generation.as_ref().is_some_and(|(g,n)|g.load(Ordering::Acquire)!=*n),"Reached ghost response must change the exact original generation");
  assert!(late.is_err()||generation_retired);assert_eq!(host.g3_view()["native_playing"],false);
  room(&mut host).await;host.end().await.unwrap();room(&mut peer).await;peer.g3_tick().await.unwrap();
  assert_eq!(host.g3_view()["native_playing"],false);assert_eq!(peer.g3_view()["native_playing"],false);
  if social{assert!(joined_social_sync(&mut host).await.is_err());assert!(joined_social_sync(&mut peer).await.is_err());}
  for index in 0..2{mark(&dir.join(index.to_string()),"end");wait_file(&dir.join(index.to_string()),"closed");}
  for child in &mut native.children{assert!(child.wait().unwrap().success());}
  for c in [&host,&peer]{let view=c.g3_view().to_string();assert!(!view.contains(c.desktop.as_ref().unwrap().access.as_str()));}
  std::fs::write(dir.join("result.json"),serde_json::to_vec_pretty(&serde_json::json!({"passed":true,"actualNoita":false,"simulatedEngineProfile":true,"namespace":service.namespace,
   "actualCoreSignInAttachmentInspectionStart":true,"actualHttps":true,"actualOsPipes":2,"nonemptyGameThreadGhosts":2,
   "peerDll":"fixture-only same-source second pipe prefix; production artifact unchanged","socialNativeStream":social_native,"pinnedSourceRuntime":source,"publicConfigureAndLoadGame":source,"verifiedRuntimeDriftAndForgedMetadataRefused":source,"a1MovementOnly":detached,"fullSocialPayloadChecked":social&&!detached,"realOriginalFileFingerprints":source,"enginePidAndApisInjected":true,"lateReplyFaultInjection":true,"lateGhostReplyGenerationFenced":true})).unwrap()).unwrap();
  println!("G3_JOINED_EVIDENCE {}",dir.display());
 });
}
async fn joined_social_sync(c:&mut Core<MemoryStore>)->Result<(),String>{if c.social_requested{c.social_sync().await}else{c.social_fixture_sync().await}}
async fn compose_social(host:&mut Core<MemoryStore>,peer:&mut Core<MemoryStore>,repo:&std::path::Path,dir:&std::path::Path){
 use nt2_desktop_g0::{protocol_g3_social::wire as social,g3_social_projection::Consumer};
 joined_social_sync(host).await.unwrap();joined_social_sync(peer).await.unwrap();
 let peer_binding=peer.g3_binding().unwrap().unwrap();
 let convert=|binding:&nt2_desktop_g0::g3_local_consumer::Binding|->social::Scope{serde_json::from_value(serde_json::to_value(&binding.scope).unwrap()).unwrap()};
 let own_scope=convert(&peer_binding);let generation=peer.generation.load(Ordering::Acquire);
 let sender=host.social_owner.as_mut().unwrap();
 // Engine HP input is injected; identity/scope/token are exclusively Core-derived.
 let hud_id=sender.begin(social::Publish{hud:Some(social::Hud{hp_milli:Some(3000),max_hp_milli:Some(2000),sampo:Some(true),hp_known:Some(true),sampo_known:Some(true),..Default::default()}),..Default::default()}).unwrap();sender.send_original(&hud_id).await.unwrap();
 let snapshot=peer.social_owner.as_mut().unwrap().latest().await.unwrap();
 let selected=peer.room.as_ref().unwrap().members.as_ref().unwrap().iter().filter(|m|m.selected==Some(true)).filter_map(|m|m.account_id.clone()).collect::<Vec<_>>();
 let mut consumer=Consumer::from_authenticated_projections(own_scope.clone(),generation,&selected,&snapshot).unwrap();
 let projection=snapshot.projections.as_ref().unwrap().iter().find(|p|p.hud.is_some()).unwrap();
 let hud=consumer.hud(generation,snapshot.scope.as_ref().unwrap(),projection).unwrap().unwrap();assert_eq!((hud.hp,hud.max_hp),(3000,2000));
 peer.g3_tick().await.unwrap();let depth=peer.g3.as_ref().unwrap().trusted_depth(projection.principal_id.as_deref().unwrap(),projection.game_session_id.as_deref().unwrap(),projection.scope.as_ref().unwrap().world_token.as_deref().unwrap()).unwrap();assert_eq!(depth,2.);
 assert!(consumer.hud(generation,snapshot.scope.as_ref().unwrap(),projection).unwrap().is_some());
 std::thread::sleep(Duration::from_millis(110));
 let poi_id=sender.begin(social::Publish{poi:Some(social::Poi{text:Some("literal 雪 <img onerror=literal>".into()),x_milli:Some("1000".into()),y_milli:Some("2000".into())}),..Default::default()}).unwrap();sender.send_original(&poi_id).await.unwrap();
 std::thread::sleep(Duration::from_millis(110));let response=peer.social_owner.as_mut().unwrap().poi_snapshot("0").await.unwrap();let mut pager=consumer.pager();let page=consumer.page(generation,&response,&mut pager).unwrap();assert!(page.complete);assert_eq!(page.markers.len(),1);
 if peer.social_stream.is_some(){
  peer.social_scheduler_tick().await.unwrap();wait_file(&dir.join("1"),"social-native");
  std::thread::sleep(Duration::from_millis(110));let next=sender.begin(social::Publish{hud:Some(social::Hud{hp_milli:Some(5000),max_hp_milli:Some(2000),sampo:Some(true),hp_known:Some(true),sampo_known:Some(true),..Default::default()}),..Default::default()}).unwrap();sender.send_original(&next).await.unwrap();
  std::thread::sleep(Duration::from_millis(510));peer.social_scheduler_tick().await.unwrap();wait_file(&dir.join("1"),"social-native-refresh");
  // Maximum payload is synthetic transport stress, NOT 256 authenticated service markers.
  let stream=peer.social_stream.as_ref().unwrap().clone();let deadline=Instant::now()+Duration::from_secs(2);while !stream.lock().unwrap().ready(){assert!(Instant::now()<deadline);std::thread::sleep(Duration::from_millis(10));}
  use nt2_desktop_g0::g3_social_frame::Command as C;
  let mut stress=vec![vec![C::Roster(vec![hud.actor.clone()]),C::Begin(256)]];
  for start in (1..=256).step_by(32){let markers=(start..start+32).map(|sequence|nt2_desktop_g0::g3_social_projection::Marker{sequence,actor:hud.actor.clone(),name:"🙂".repeat(99),text:"🙂".repeat(99),x:2147483.647,y:-2147483.648}).collect();stress.push(vec![C::Page(nt2_desktop_g0::g3_social_projection::Page{high:256,next:start+31,markers,complete:start==225})]);}
  stream.lock().unwrap().publish(0.,stress).unwrap();wait_file(&dir.join("1"),"social-native-max");
  room(peer).await;peer.game_tick().await.unwrap();std::thread::sleep(Duration::from_millis(2200));peer.social_scheduler_tick().await.unwrap();wait_file(&dir.join("1"),"social-native-historical");
 }
 let marker=&page.markers[0];consumer.remember_dismissal(marker.sequence).unwrap();
 // Lost dismissal request: service still sends original marker; local retention filters it.
 std::thread::sleep(Duration::from_millis(110));let refresh=peer.social_owner.as_mut().unwrap().poi_snapshot("0").await.unwrap();assert_eq!(refresh.markers.as_ref().unwrap().len(),1);
 assert!(consumer.page(generation,&refresh,&mut consumer.pager()).unwrap().markers.is_empty());
 std::thread::sleep(Duration::from_millis(110));let _lost_ack=peer.social_owner.as_mut().unwrap().dismiss(&marker.sequence.to_string()).await.unwrap();
 assert_eq!(consumer.pending_dismissals().collect::<Vec<_>>(),vec![marker.sequence]);
 std::thread::sleep(Duration::from_millis(110));assert!(peer.social_owner.as_mut().unwrap().poi_snapshot("0").await.unwrap().markers.unwrap().is_empty());
 assert_eq!(sender.poi_snapshot("0").await.unwrap().markers.unwrap().len(),1);
 assert!(consumer.page(generation+1,&response,&mut consumer.pager()).is_err());
 peer.social_owner.as_mut().unwrap().pause();assert!(peer.social_owner.as_mut().unwrap().begin(social::Publish::default()).is_err());joined_social_sync(peer).await.unwrap();
 // Fixture-only bounded data file. Lua parses fixed records; no service text becomes code/path.
 let text_hex=marker.text.as_bytes().iter().map(|b|format!("{b:02x}")).collect::<String>();let scope=hex(&peer_binding.context);
 let name_hex=hud.name.as_bytes().iter().map(|b|format!("{b:02x}")).collect::<String>();
 let data=format!("bind\t{scope}\t{generation}\t{}\t{name_hex}\t{depth}\nhud\t{}\t{}\t{}\t{}\t{}\npoi\t{}\t{}\t{}\t{}\t{}\t{}\t{text_hex}\n",hud.actor,hud.actor,hud.sequence,hud.hp,hud.max_hp,u8::from(hud.sampo),page.high,page.next,marker.sequence,marker.actor,marker.x,marker.y);
 assert!(data.len()<4096);let input=dir.join("social-projection.data");std::fs::write(&input,data).unwrap();
 use nt2_desktop_g0::g3_social_frame::{Queue,Command as LocalCommand};
 let mut queue=Queue::new(peer_binding.context,u32::try_from(generation).unwrap());
 let framed_page=nt2_desktop_g0::g3_social_projection::Page{high:page.high,next:page.next,markers:page.markers.clone(),complete:page.complete};
 queue.publish(0.,vec![vec![LocalCommand::Hud(hud.clone()),LocalCommand::Begin(page.high),LocalCommand::Page(framed_page)]]).unwrap();
 let binary=dir.join("social-projection.bin");std::fs::write(&binary,queue.take(peer_binding.context,u32::try_from(generation).unwrap(),true).unwrap()).unwrap();
 // Synthetic local latest-removal fault; private marker/name still comes from HTTPS.
 let framed_page=nt2_desktop_g0::g3_social_projection::Page{high:page.high,next:page.next,markers:page.markers.clone(),complete:page.complete};
 queue.publish(0.,vec![vec![LocalCommand::Remove(hud.actor.clone()),LocalCommand::Begin(page.high),LocalCommand::Page(framed_page)]]).unwrap();
 let next_binary=dir.join("social-projection-next.bin");std::fs::write(&next_binary,queue.take(peer_binding.context,u32::try_from(generation).unwrap(),true).unwrap()).unwrap();
 let output=Command::new(repo.join("spikes/noita-entry/.build/Lua51Runner.exe")).arg(r"C:\Noita\lua51.dll")
  .arg(repo.join("apps/desktop/tools/g3-social-projection-test.lua")).env("NT2_G3_MODULES",repo.join("mod/noita_together2/files/g3"))
  .env("NT2_G3_SOCIAL_INPUT",input).env("NT2_G3_SOCIAL_FRAME",binary).env("NT2_G3_SOCIAL_NEXT_FRAME",next_binary).output().unwrap();std::fs::write(dir.join("social-lua-output.txt"),&output.stdout).unwrap();assert!(output.status.success(),"{}",String::from_utf8_lossy(&output.stderr));
 std::fs::write(dir.join("social-result.json"),serde_json::to_vec_pretty(&json!({"passed":true,"actualNoita":false,"coreDerivedAuthority":true,"verifiedHttps":true,"nativeObservationAuthority":true,
  "socialNativeFraming":false,"rustLuaSocialFraming":true,"socialFrameTransport":"fixture file read before callbacks; no native social pipe","localGameThreadLua":true,"legalOvermaxHp":true,"servicePrivateDismissal":true,"lostDismissalRequestAndAckRetained":true,"restartDurability":false})).unwrap()).unwrap();
}
