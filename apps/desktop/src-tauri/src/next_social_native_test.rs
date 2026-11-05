//! Focused real broker/OS pipe interoperability. Only base admission is injected;
//! no canonical service, Noita process, saves, installation or authority claim.
use super::*;
use std::{os::windows::{io::AsRawHandle,process::CommandExt},process::{Command as ProcessCommand,Stdio},path::{Path,PathBuf}};
use tokio::{io::{AsyncReadExt,AsyncWriteExt},net::windows::named_pipe::{NamedPipeServer,ServerOptions}};
#[link(name="kernel32")]unsafe extern "system"{fn GetNamedPipeClientProcessId(pipe:*mut std::ffi::c_void,pid:*mut u32)->i32;}
pub(super) fn hidden(path:impl AsRef<std::ffi::OsStr>)->ProcessCommand{let mut c=ProcessCommand::new(path);c.creation_flags(0x08000000);c}
pub(super) fn number(bytes:&[u8],at:usize)->u32{u32::from_le_bytes(bytes[at..at+4].try_into().unwrap())}
pub(super) async fn request(pipe:&mut NamedPipeServer)->Vec<u8>{
 let mut b=vec![0;8];pipe.read_exact(&mut b).await.unwrap();
 let header=if &b[..4]==b"NSP1"||&b[..4]==b"NTG3"{64}else{16};b.resize(header,0);pipe.read_exact(&mut b[8..]).await.unwrap();
 let total=if &b[..4]==b"NSP1"{64+number(&b,56) as usize+number(&b,28) as usize}else if &b[..4]==b"NTG3"{64}else{match number(&b,8){12=>56,7=>48+number(&b,12) as usize,_=>32}};
 assert!(total<=32816);let old=b.len();b.resize(total,0);pipe.read_exact(&mut b[old..]).await.unwrap();b
}
fn chat(high:u32)->Vec<Vec<Command>>{vec![vec![optional_command(Display::Chat{id:format!("room:{high}"),name:"A".into(),text:"fixture".into()})]]}
#[test]#[ignore="explicit real broker/wrappers/OS-pipe input contention; injected admission/producer ACK"]
fn a3_explicit_input_survives_hud_occupancy_and_lost_ack_actual_broker(){tauri::async_runtime::block_on(async{
 let repo=Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..").canonicalize().unwrap();let repo=PathBuf::from(repo.to_str().unwrap().trim_start_matches(r"\\?\"));
 let own=repo.join("spikes/noita-entry/.build/a3-owner-busy-cape-20261023-01").join(format!("native-{}",uuid::Uuid::new_v4()));std::fs::create_dir_all(&own).unwrap();
 let prefix=format!(r"\\.\pipe\nt2-a3-input-{}-",uuid::Uuid::new_v4());let dlls=own.join("private-dlls");
 let build=hidden("powershell.exe").args(["-NoProfile","-ExecutionPolicy","Bypass","-File"]).arg(repo.join("native/g3-ipc/build.ps1")).arg("-OutputDirectory").arg(&dlls).arg("-Private").arg("-TestAppPath").arg(crate::pipe::verified_process_path(&std::env::current_exe().unwrap()).unwrap()).arg("-TestPipePrefix").arg(&prefix).env("PATH",format!("{0}/System32;{0};{0}/System32/Wbem;{0}/System32/WindowsPowerShell/v1.0",std::env::var("WINDIR").unwrap())).env("PSModulePath",format!("{}/System32/WindowsPowerShell/v1.0/Modules",std::env::var("WINDIR").unwrap())).output().unwrap();std::fs::write(own.join("private-build.txt"),&build.stdout).unwrap();std::fs::write(own.join("private-build.err"),&build.stderr).unwrap();assert!(build.status.success(),"{}",String::from_utf8_lossy(&build.stderr));
 let name=format!("{prefix}{}",crate::pipe::user_sid().unwrap());let listen=||ServerOptions::new().reject_remote_clients(true).create(&name).unwrap();let mut pipe=listen();
 let log=std::fs::File::create(own.join("lua.txt")).unwrap();let mut child=hidden(repo.join("spikes/noita-entry/.build/Lua51Runner.exe")).arg("C:/Noita/lua51.dll").arg(repo.join("apps/desktop/tools/a3-input-native-test.lua")).env("NT2_A3_REPO",&repo).env("NT2_A3_NATIVE_DIR",&dlls).stdout(Stdio::from(log.try_clone().unwrap())).stderr(Stdio::from(log)).spawn().unwrap();
 let debug=Arc::new(std::sync::atomic::AtomicBool::new(true));let mut debug_reports=0;let mut exchange=nt2_desktop_g0::g3_social_stream::Exchange::default();exchange.debug_control(debug.clone());let mut context=None;let clock=Instant::now();let deadline=clock+Duration::from_secs(10);let mut actor_at=clock;let mut input_at=None;let mut seen=std::collections::BTreeMap::new();let mut hud=0;let mut poi=0;let mut emote=0;let mut lost_ack=false;let mut hide_ack=false;let mut rescans=0;let mut work=Work::default();let mut trace=Vec::new();
 while Instant::now()<deadline{
  debug.store(!(4000..6000).contains(&clock.elapsed().as_millis()),std::sync::atomic::Ordering::Release);if exchange.take_debug_report().is_some(){debug_reports+=1;}
  if child.try_wait().unwrap().is_some(){break}
  if let Some(ctx)=context{let now=Instant::now();if now>=actor_at{actor_at=now+Duration::from_millis(200);
   if exchange.take_rescan(){work.rescan_markers();rescans+=1;}
   let input=exchange.input();if let Some((seq,_))=&input{if input_at.is_none(){input_at=Some((*seq,now));}}
   let publication=work.publication_input(input);
   if work.lane(now,publication.is_some())==Some(Lane::Publication){work.started(Lane::Publication);work.completed(Lane::Publication,now);
    if let Some((seq,value))=publication.unwrap().0{if input_at.is_some_and(|(old,at)|old==seq&&now.duration_since(at)>=Duration::from_millis(900)){
     match value{Input::Hud{..}=>hud+=1,Input::Poi{id,..}=>{poi+=1;assert!(seen.insert(id,"poi").is_none());hide_ack=true;},Input::Emote{id,..}=>{emote+=1;assert!(seen.insert(id,"emote").is_none());},_=>panic!("Unexpected input")}
     exchange.acknowledge_input(ctx,1,seq);input_at=None;
    }}
   }else if let Some(lane)=work.lane(now,false){work.started(lane);work.completed(lane,now);}
   if exchange.ready(){exchange.publish(clock.elapsed().as_secs_f64(),vec![vec![optional_command(Display::Dismissed(vec![]))]]).unwrap();}
  }}
  if tokio::time::timeout(Duration::from_millis(10),pipe.connect()).await.is_err(){continue}let next=listen();let mut pid=0;assert_ne!(unsafe{GetNamedPipeClientProcessId(pipe.as_raw_handle(),&mut pid)},0);assert_eq!(pid,child.id());let bytes=tokio::time::timeout(Duration::from_millis(100),request(&mut pipe)).await.unwrap();
  let reply=if &bytes[..4]==b"NSP1"{let ctx:[u8;16]=bytes[8..24].try_into().unwrap();if context.is_none(){context=Some(ctx);exchange.arm(nt2_local_ws::owner::Id::OsProcess(pid),ctx,1,deadline).unwrap();}let mut reply=exchange.transition(nt2_local_ws::owner::Id::OsProcess(pid),&bytes,true);if hide_ack&&!lost_ack{reply[48..56].fill(0);lost_ack=true;hide_ack=false;}trace.push(json!({"at_ms":clock.elapsed().as_millis(),"input_sequence":u64::from_le_bytes(bytes[48..56].try_into().unwrap()),"progress":exchange.progress()}));reply}
  else if &bytes[..4]==b"NTG3"{let mut r=vec![0;864];r[..6].copy_from_slice(b"NG3R\x02\x01");r[8..24].copy_from_slice(&bytes[8..24]);r[24..28].copy_from_slice(&200u32.to_le_bytes());r}
  else{let op=number(&bytes,8);let mut r=vec![0;if op==13{1072}else{48}];r[..4].copy_from_slice(&0x3247544eu32.to_le_bytes());r[4..8].copy_from_slice(&bytes[4..8]);r[8..12].copy_from_slice(&1u32.to_le_bytes());r[12..16].copy_from_slice(&(if op==5||op==7{2u32}else{1u32}).to_le_bytes());let start=if op==7{24}else{16};r[16..32].copy_from_slice(&bytes[start..start+16]);if op==13{r[12..16].fill(0)}else{r[32..36].copy_from_slice(&12000u32.to_le_bytes());r[36..40].copy_from_slice(&1u32.to_le_bytes());if op==7{r[40..48].copy_from_slice(&bytes[16..24]);}}r};
  pipe.write_all(&reply).await.unwrap();pipe.flush().await.unwrap();pipe=next;
 }
 let status=child.wait().unwrap();let report=json!({"hud_acknowledged":hud,"poi_originals":poi,"emote_originals":emote,"lost_ack_injected":lost_ack,"rescans":rescans,"progress":exchange.progress(),"trace":trace,"actualNativePipeAndLua":true,"engineAndAdmissionAndProducerAckInjected":true,"actualNoita":false});std::fs::write(own.join("contention.json"),serde_json::to_vec_pretty(&report).unwrap()).unwrap();assert!(status.success(),"See {}",own.join("lua.txt").display());assert!(hud>=2&&poi==1&&emote==1&&lost_ack&&rescans>0);assert!(debug_reports>10);assert_eq!(exchange.progress()["inputs_rejected"],0);println!("PASS actual native occupied-input/lost-ACK fixture; evidence {}",own.display());
});}
#[test]#[ignore="explicit real private native broker / OS-pipe reset check; injected admission only"]
fn a2_native_reset_handshake_actual_broker(){tauri::async_runtime::block_on(async{
 let repo=Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..").canonicalize().unwrap();let repo=PathBuf::from(repo.to_str().unwrap().trim_start_matches(r"\\?\"));
 let own=repo.join("spikes/noita-entry/.build/a2-capture-correction-20261022-01").join(format!("native-reset-{}",uuid::Uuid::new_v4()));std::fs::create_dir_all(&own).unwrap();
 let prefix=format!(r"\\.\pipe\nt2-a2-reset-{}-",uuid::Uuid::new_v4());let dlls=own.join("private-dlls");
 let build=hidden("powershell.exe").args(["-NoProfile","-ExecutionPolicy","Bypass","-File"]).arg(repo.join("native/g3-ipc/build.ps1")).arg("-OutputDirectory").arg(&dlls).arg("-Private").arg("-TestAppPath").arg(crate::pipe::verified_process_path(&std::env::current_exe().unwrap()).unwrap()).arg("-TestPipePrefix").arg(&prefix).output().unwrap();std::fs::write(own.join("private-build.txt"),&build.stdout).unwrap();assert!(build.status.success(),"{}",String::from_utf8_lossy(&build.stderr));
 let name=format!("{prefix}{}",crate::pipe::user_sid().unwrap());let listen=||ServerOptions::new().reject_remote_clients(true).create(&name).unwrap();let mut pipe=listen();
 let log=std::fs::File::create(own.join("lua.txt")).unwrap();let mut child=hidden(repo.join("spikes/noita-entry/.build/Lua51Runner.exe")).arg("C:/Noita/lua51.dll").arg(repo.join("apps/desktop/tools/a2-native-reset-test.lua")).env("NT2_A2_NATIVE_DIR",&dlls).stdout(Stdio::from(log.try_clone().unwrap())).stderr(Stdio::from(log)).spawn().unwrap();
 let mut exchange=nt2_desktop_g0::g3_social_stream::Exchange::default();let mut context=None;let mut generation=1;let mut resets=0;let mut zero_after_reset=false;let deadline=Instant::now()+Duration::from_secs(13);let mut trace=Vec::new();
 while Instant::now()<deadline{
  if child.try_wait().unwrap().is_some(){break}
  if tokio::time::timeout(Duration::from_millis(50),pipe.connect()).await.is_err(){continue}
  let next=listen();let mut pid=0;assert_ne!(unsafe{GetNamedPipeClientProcessId(pipe.as_raw_handle(),&mut pid)},0);assert_eq!(pid,child.id());
  let bytes=tokio::time::timeout(Duration::from_millis(100),request(&mut pipe)).await.unwrap();
  let reply=if &bytes[..4]==b"NSP1"{
   let ctx:[u8;16]=bytes[8..24].try_into().unwrap();if context.is_none(){context=Some(ctx);exchange.arm(nt2_local_ws::owner::Id::OsProcess(pid),ctx,1,deadline).unwrap();exchange.publish(0.,chat(1)).unwrap();}assert_eq!(context,Some(ctx));
   let ack=number(&bytes,24);if resets>0&&ack==0{zero_after_reset=true;}
   let reply=exchange.transition(nt2_local_ws::owner::Id::OsProcess(pid),&bytes,true);trace.push(json!({"generation":generation,"ack":ack,"state":reply[5],"receipt":exchange.acknowledged_chat_high(nt2_local_ws::owner::Id::OsProcess(pid),ctx,generation)}));
   if reply[5]==3{resets+=1;}
   if generation==1&&exchange.acknowledged_chat_high(nt2_local_ws::owner::Id::OsProcess(pid),ctx,1)==Some(1){generation=2;exchange.arm(nt2_local_ws::owner::Id::OsProcess(pid),ctx,2,deadline).unwrap();exchange.publish(0.,chat(2)).unwrap();}
   reply
  }else if &bytes[..4]==b"NTG3"{let mut reply=vec![0;864];reply[..6].copy_from_slice(b"NG3R\x02\x01");reply[8..24].copy_from_slice(&bytes[8..24]);reply[24..28].copy_from_slice(&200u32.to_le_bytes());reply}
  else{
   let op=number(&bytes,8);let revision=number(&bytes,4);let mut reply=vec![0;if op==13{1072}else{48}];reply[..4].copy_from_slice(&0x3247544eu32.to_le_bytes());reply[4..8].copy_from_slice(&revision.to_le_bytes());reply[8..12].copy_from_slice(&1u32.to_le_bytes());reply[12..16].copy_from_slice(&(if op==5||op==7{2u32}else{1u32}).to_le_bytes());let start=if op==7{24}else{16};reply[16..32].copy_from_slice(&bytes[start..start+16]);if op==13{reply[12..16].fill(0);}else{reply[32..36].copy_from_slice(&12000u32.to_le_bytes());reply[36..40].copy_from_slice(&1u32.to_le_bytes());if op==7{reply[40..48].copy_from_slice(&bytes[16..24]);}}reply
  };
  pipe.write_all(&reply).await.unwrap();pipe.flush().await.unwrap();pipe=next;
 }
 // The fixture exits itself within10 seconds; never terminate a test process.
 let status=child.wait().unwrap();std::fs::write(own.join("pipe-trace.json"),serde_json::to_vec_pretty(&trace).unwrap()).unwrap();
 assert!(status.success(),"See {}",own.join("lua.txt").display());assert!(resets>=1&&zero_after_reset,"Broker did not negotiate zero ACK");assert_eq!(generation,2);assert_eq!(exchange.acknowledged_chat_high(nt2_local_ws::owner::Id::OsProcess(child.id()),context.unwrap(),2),Some(2));println!("PASS real broker/OS-pipe generation reset; evidence {}",own.display());
});}
