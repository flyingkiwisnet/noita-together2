//! Focused independent scheduling with real private native components. HTTP
//! publication/query responses and Noita engine facts are typed test inputs.
use super::*;
use super::native_test::{hidden,number,request};
use nt2_desktop_g0::protocol_g3_social::wire;
use std::{path::{Path,PathBuf},process::Stdio,os::windows::io::AsRawHandle};
use tokio::{net::windows::named_pipe::ServerOptions,io::AsyncWriteExt};
#[link(name="kernel32")]unsafe extern "system"{fn GetNamedPipeClientProcessId(pipe:*mut std::ffi::c_void,pid:*mut u32)->i32;}
struct Facts{sequence:u32,published:Instant,hud:wire::Hud}
struct SlowSource{scope:wire::Scope,player:wire::Player,facts:Arc<Mutex<Facts>>,query:u64}
impl read::Source for SlowSource{
 async fn discover(&mut self)->Result<(),String>{Ok(())}
 async fn roster(&mut self)->Result<wire::Response,String>{tokio::time::sleep(Duration::from_millis(227+self.query%3*7)).await;Ok(wire::Response{roster_complete:Some(true),players:Some(vec![self.player.clone()]),..Default::default()})}
 async fn latest(&mut self,after:&str)->Result<nt2_desktop_g0::g3_social::TimedSnapshot,String>{
  let started=Instant::now();self.query+=1;tokio::time::sleep(Duration::from_millis(176)).await;
  let(sequence,hud)={let facts=self.facts.lock().unwrap();(facts.sequence,facts.hud.clone())};
  let p=wire::Projection{scope:Some(self.scope.clone()),principal_id:self.player.principal_id.clone(),game_session_id:self.player.game_session_id.clone(),hud_state_revision:Some(sequence),provenance:Some("validated-client-report".into()),captured_at:Some("1970-01-01T00:00:01.000Z".into()),hud:Some(hud),..Default::default()};
  let next=format!("{}:hud",self.player.game_session_id.as_deref().unwrap());let projections=if sequence>0&&next.as_str()>after{vec![p]}else{vec![]};let cursor=if projections.is_empty(){after.to_string()}else{next};
  tokio::time::sleep(Duration::from_millis(400+self.query%3*11)).await;
  Ok(nt2_desktop_g0::g3_social::TimedSnapshot{request_started:started,response:wire::Response{projections:Some(projections),next_key:Some(cursor),..Default::default()}})
 }
 async fn poi(&mut self,_:&str)->Result<wire::Response,String>{unreachable!("no POI HTTP in focused fixture")}
}
#[test]#[ignore="explicit real broker/OS-pipe + production Lua/read scheduling; injected engine/service facts"]
fn a2_retained_delayed_read_and_game_jitter_actual_broker(){tauri::async_runtime::block_on(async{
 let repo=Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..").canonicalize().unwrap();let repo=PathBuf::from(repo.to_str().unwrap().trim_start_matches(r"\\?\"));
 let own=repo.join("spikes/noita-entry/.build/a2-retained-20261022-01").join(format!("native-{}",uuid::Uuid::new_v4()));std::fs::create_dir_all(&own).unwrap();
 let prefix=format!(r"\\.\pipe\nt2-a2-retained-{}-",uuid::Uuid::new_v4());let dlls=own.join("private-dlls");
 let build=hidden("powershell.exe").args(["-NoProfile","-ExecutionPolicy","Bypass","-File"]).arg(repo.join("native/g3-ipc/build.ps1")).arg("-OutputDirectory").arg(&dlls).arg("-Private").arg("-TestAppPath").arg(crate::pipe::verified_process_path(&std::env::current_exe().unwrap()).unwrap()).arg("-TestPipePrefix").arg(&prefix).output().unwrap();std::fs::write(own.join("private-build.txt"),&build.stdout).unwrap();assert!(build.status.success());
 let name=format!("{prefix}{}",crate::pipe::user_sid().unwrap());let listen=||ServerOptions::new().reject_remote_clients(true).create(&name).unwrap();let mut pipe=listen();
 let raw:Value=serde_json::from_slice(include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../packages/contracts/g3-social/replay-policy.json"))).unwrap();let mut scope:wire::Scope=serde_json::from_value(raw["vectors"][0]["scope"].clone()).unwrap();scope.world_token=Some(uuid::Uuid::new_v4().to_string());let session=uuid::Uuid::new_v4().to_string();let principal=uuid::Uuid::new_v4().to_string();
 let binding=Binding{context:[1;16],service_origin:"https://fixture.invalid".into(),service_id:"fixture".into(),service_epoch:"epoch".into(),session_epoch:"session".into(),principal_id:principal.clone(),scope:serde_json::from_value(serde_json::to_value(&scope).unwrap()).unwrap()};let store=Arc::new(Store::new(&own,&binding));
 let player=wire::Player{player_id:scope.world_token.clone(),world_token:scope.world_token.clone(),game_session_id:Some(session.clone()),principal_id:Some(principal),display_name:Some("Fixture".into()),state:Some("playing".into()),inventory_revision:Some(0),held_wand_sprite_id:Some(String::new()),..Default::default()};
 let facts=Arc::new(Mutex::new(Facts{sequence:0,published:Instant::now(),hud:Default::default()}));let mut source=Some(SlowSource{scope:scope.clone(),player,facts:facts.clone(),query:0});let mut latest=Latest::default();let hud=Arc::new(Mutex::new(a2::HudReplay::default()));let diagnostic=Arc::new(Mutex::new(Value::Null));
 let log=std::fs::File::create(own.join("lua.txt")).unwrap();let mut child=hidden(repo.join("spikes/noita-entry/.build/Lua51Runner.exe")).arg("C:/Noita/lua51.dll").arg(repo.join("apps/desktop/tools/a2-freshness-native-test.lua")).env("NT2_A2_REPO",&repo).env("NT2_A2_NATIVE_DIR",&dlls).env("NT2_A2_FIXTURE_DIR",&own).stdout(Stdio::from(log.try_clone().unwrap())).stderr(Stdio::from(log)).spawn().unwrap();
 let mut exchange=nt2_desktop_g0::g3_social_stream::Exchange::default();let clock=Instant::now();let deadline=clock+Duration::from_secs(12);let mut context=None;let mut work=Work::default();let mut read_task=None;let mut publish_task=None;let mut actor_at=clock;let mut injected=false;let mut reads=0;let mut publications=0;let mut pauses=0;let mut trace=Vec::new();
 while Instant::now()<deadline{
  if child.try_wait().unwrap().is_some(){break}
  if let Some(ctx)=context{
   let now=Instant::now();if now>=actor_at{actor_at=now+Duration::from_millis(200);
    if read_task.as_ref().is_some_and(|t:&tokio::task::JoinHandle<(SlowSource,Latest,Result<Vec<Vec<Command>>,String>)>|t.is_finished()){let(s,l,result)=read_task.take().unwrap().await.unwrap();source=Some(s);latest=l;work.backlog=Some(result.unwrap());reads+=1;}
    if publish_task.as_ref().is_some_and(|t:&tokio::task::JoinHandle<()>|t.is_finished()){publish_task.take().unwrap().await.unwrap();publications+=1;}
    if publish_task.is_none()&&work.can_start_publication(now){if let Some((sequence,Input::Hud{hud:sample}))=exchange.input(){let (_,transient)=work.publication_input(Some((sequence,Input::Hud{hud:sample.clone()}))).unwrap();assert!(!transient);exchange.acknowledge_input(ctx,1,sequence);work.publish_next=Some(now+Duration::from_millis(500));let facts=facts.clone();publish_task=Some(tokio::spawn(async move{tokio::time::sleep(Duration::from_millis(227)).await;let mut f=facts.lock().unwrap();f.sequence+=1;f.published=Instant::now();f.hud=sample;}));}}
    if exchange.ready(){let batches=work.ready_batches();if !batches.is_empty(){exchange.publish(clock.elapsed().as_secs_f64(),batches).unwrap();}}
    if source.is_some()&&work.can_start_information(now){work.next=Some(now+Duration::from_millis(500));let mut s=source.take().unwrap();let mut l=std::mem::take(&mut latest);let scope=scope.clone();let store=store.clone();let actor=scope.world_token.clone().unwrap();let hud=hud.clone();let diagnostic=diagnostic.clone();read_task=Some(tokio::spawn(async move{let result=read::information(&mut s,&mut l,&scope,actor,store,vec![],clock,hud,diagnostic).await;(s,l,result)}));}
   }
   if exchange.take_rescan(){work.rescan_markers();}
   if !injected&&own.join("block-ready").exists()&&exchange.ready(){work.out.push(optional_command(Display::Dismissed(vec![])));injected=true;}
   if exchange.ready(){let batches=work.ready_batches();if !batches.is_empty(){exchange.publish(clock.elapsed().as_secs_f64(),batches).unwrap();}}
  }
  if tokio::time::timeout(Duration::from_millis(10),pipe.connect()).await.is_err(){continue}let next=listen();let mut pid=0;assert_ne!(unsafe{GetNamedPipeClientProcessId(pipe.as_raw_handle(),&mut pid)},0);assert_eq!(pid,child.id());let bytes=tokio::time::timeout(Duration::from_millis(100),request(&mut pipe)).await.unwrap();
  let reply=if &bytes[..4]==b"NSP1"{let ctx:[u8;16]=bytes[8..24].try_into().unwrap();if context.is_none(){context=Some(ctx);exchange.arm(nt2_local_ws::owner::Id::OsProcess(pid),ctx,1,deadline).unwrap();}assert_eq!(context,Some(ctx));let reply=exchange.transition(nt2_local_ws::owner::Id::OsProcess(pid),&bytes,true);if number(&reply,32)==41&&reply[104]==5{pauses+=1;}trace.push(json!({"at_ms":clock.elapsed().as_millis(),"ack":number(&bytes,24),"frame_bytes":number(&reply,32),"pause":number(&reply,32)==41&&reply[104]==5}));reply}
  else if &bytes[..4]==b"NTG3"{let mut r=vec![0;864];r[..6].copy_from_slice(b"NG3R\x02\x01");r[8..24].copy_from_slice(&bytes[8..24]);r[24..28].copy_from_slice(&200u32.to_le_bytes());r}
  else{let op=number(&bytes,8);let mut r=vec![0;if op==13{1072}else{48}];r[..4].copy_from_slice(&0x3247544eu32.to_le_bytes());r[4..8].copy_from_slice(&bytes[4..8]);r[8..12].copy_from_slice(&1u32.to_le_bytes());r[12..16].copy_from_slice(&(if op==5||op==7{2u32}else{1u32}).to_le_bytes());let start=if op==7{24}else{16};r[16..32].copy_from_slice(&bytes[start..start+16]);if op==13{r[12..16].fill(0)}else{r[32..36].copy_from_slice(&12000u32.to_le_bytes());r[36..40].copy_from_slice(&1u32.to_le_bytes());if op==7{r[40..48].copy_from_slice(&bytes[16..24]);}}r};
  pipe.write_all(&reply).await.unwrap();pipe.flush().await.unwrap();pipe=next;
 }
 let status=child.wait().unwrap();if let Some(t)=publish_task{t.await.unwrap();}if let Some(t)=read_task{let _=t.await.unwrap();}
 let report=json!({"reads":reads,"publications":publications,"pauseReplies":pauses,"rescans":work.rescans,"information":*diagnostic.lock().unwrap(),"trace":trace,"engineFactsInjected":true,"servicePublicationAndQueryInjected":true,"actualBrokerWrappersOsPipe":true,"actualCaptureDecoderRenderer":true,"coreActorPeriodMs":200,"publicationGuardMs":500,"rosterDelayMs":227,"latestDelayMs":576,"readJitterMs":[0,7,14,11,22],"gameCallbackStallMs":300});std::fs::write(own.join("scheduling.json"),serde_json::to_vec_pretty(&report).unwrap()).unwrap();
 assert!(status.success(),"Lua scheduling evidence {}",own.join("lua.txt").display());assert!(injected&&pauses>0&&reads>=2&&publications>=2&&own.join("pause-preserved").exists()&&own.join("fresh-returned").exists());println!("PASS real broker retained scheduling; evidence {}",own.display());
});}
