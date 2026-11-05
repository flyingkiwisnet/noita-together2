use super::{protocol::*,server::*,client::Client,private::Bootstrap};
use std::{sync::{Arc,atomic::{AtomicU64,Ordering}},time::{Duration,Instant}};
use futures_util::{SinkExt,StreamExt};
use tokio_tungstenite::tungstenite::{Message,client::IntoClientRequest};
type Socket=tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>;
async fn open(d:&Descriptor)->Socket{let(address,_,_)=d.validate(true).unwrap();let stream=tokio::net::TcpStream::connect(address).await.unwrap();tokio_tungstenite::client_async_with_config(&d.endpoint,stream,Some(ws_config())).await.unwrap().0}
async fn authenticate(ws:&mut Socket,d:&Descriptor,native:[u8;16],nonce:[u8;32])->u64{
 let(_,app,secret)=d.validate(true).unwrap();let mut body=app.to_vec();body.extend_from_slice(&native);body.extend_from_slice(&nonce);ws.send(Message::Binary(auth(1,&body).into())).await.unwrap();
 let Message::Binary(b)=ws.next().await.unwrap().unwrap()else{panic!("challenge")};let b=auth_body(&b,2,64).unwrap();let server=b[..32].try_into().unwrap();assert!(verify(&secret,true,&app,&native,&nonce,&server,&b[32..]));let finish=proof(&secret,false,&app,&native,&nonce,&server);ws.send(Message::Binary(auth(3,&finish).into())).await.unwrap();let Message::Binary(b)=ws.next().await.unwrap().unwrap()else{panic!("accepted")};u64::from_le_bytes(auth_body(&b,4,8).unwrap().try_into().unwrap())
}
async fn refused(ws:&mut Socket){assert!(!matches!(tokio::time::timeout(Duration::from_secs(3),ws.next()).await,Ok(Some(Ok(Message::Binary(_))))));}
#[test]fn native_ffi_setup_failure_can_recover_and_reply_names_its_authenticated_app(){
 let root=root();let missing=root.join("missing-module/.nt2-local/bridge.json");let input=[1u8;32];let mut output=[0u8;32];let mut app=[0u8;16];let mut wrote=9;
 let invoke=|path:&std::path::Path,output:&mut [u8;32],app:&mut [u8;16],wrote:&mut u32|{let path=path.to_string_lossy().encode_utf16().collect::<Vec<_>>();unsafe{super::ffi::nt2_local_ws_exchange(path.as_ptr(),path.len(),1,input.as_ptr(),input.len(),output.as_mut_ptr(),output.len(),80,wrote,app.as_mut_ptr())}};
 assert_eq!(invoke(&missing,&mut output,&mut app,&mut wrote),0);assert_eq!(wrote,0);
 let handler=Arc::new(Echo{calls:AtomicU64::new(0)});let server=Server::bind(0,true,handler).unwrap();let descriptor=server.descriptor();let bootstrap=Bootstrap::publish(&root,&descriptor).unwrap();let until=Instant::now()+Duration::from_secs(3);loop{if invoke(bootstrap.path(),&mut output,&mut app,&mut wrote)==1{break}assert!(Instant::now()<until,"Transient native setup failure prevented later authentication");std::thread::sleep(Duration::from_millis(20));}assert_eq!(output,input);assert_eq!(wrote,1);assert_eq!(app,unhex::<16>(&descriptor.app).unwrap());drop(server);drop(bootstrap);std::fs::remove_dir(root).unwrap();
}
#[test]fn hostile_headers_proofs_replay_and_malformed_input_never_dispatch(){
 let handler=Arc::new(Echo{calls:AtomicU64::new(0)});let server=Server::bind(0,true,handler.clone()).unwrap();let d=server.descriptor();let runtime=tokio::runtime::Runtime::new().unwrap();runtime.block_on(async{
  for (header,value) in [("origin","https://example.com"),("host","localhost"),("sec-websocket-version","12"),("sec-websocket-protocol","unapproved"),("sec-websocket-extensions","permessage-deflate")]{let mut req=d.endpoint.clone().into_client_request().unwrap();req.headers_mut().insert(tokio_tungstenite::tungstenite::http::HeaderName::from_bytes(header.as_bytes()).unwrap(),value.parse().unwrap());let stream=tokio::net::TcpStream::connect(server.address()).await.unwrap();assert!(tokio_tungstenite::client_async(req,stream).await.is_err(),"header {header}");}
  for suffix in ["?extra=1","/wrong"]{let stream=tokio::net::TcpStream::connect(server.address()).await.unwrap();assert!(tokio_tungstenite::client_async(format!("{}{suffix}",d.endpoint),stream).await.is_err());}
  let(_,app,_)=d.validate(true).unwrap();let native=random::<16>().unwrap();let nonce=random::<32>().unwrap();let mut hello=app.to_vec();hello.extend_from_slice(&native);hello.extend_from_slice(&nonce);
  {let mut ws=open(&d).await;ws.send(Message::Binary(auth(1,&hello).into())).await.unwrap();assert!(matches!(ws.next().await,Some(Ok(Message::Binary(_)))));ws.send(Message::Binary(auth(3,&[0;32]).into())).await.unwrap();refused(&mut ws).await;}
  {let mut ws=open(&d).await;ws.send(Message::Binary(auth(1,&hello).into())).await.unwrap();refused(&mut ws).await;}
  for message in [Message::Text("not binary".into()),Message::Binary(vec![0;MAX_MESSAGE+1].into())]{let mut ws=open(&d).await;ws.send(message).await.unwrap();refused(&mut ws).await;}
  {let mut ws=open(&d).await;let generation=authenticate(&mut ws,&d,native,random().unwrap()).await;let mut b=Envelope{reply:false,generation,request:1,payload:&[1;32]}.encode().unwrap();b[24..28].copy_from_slice(&999u32.to_le_bytes());ws.send(Message::Binary(b.into())).await.unwrap();refused(&mut ws).await;}
 });assert_eq!(handler.calls.load(Ordering::SeqCst),0);
}
struct Echo{calls:AtomicU64}
impl Handler for Echo{
 fn connected(&self,_:Peer)->bool{true}
 fn request(&self,_:Peer,r:&[u8])->Option<Vec<u8>>{self.calls.fetch_add(1,Ordering::SeqCst);if r[0]==99{std::thread::sleep(Duration::from_millis(150))}Some(r.to_vec())}
 fn delivery(&self,_:Peer,_:&[u8],r:Vec<u8>)->Option<Vec<u8>>{Some(r)}
 fn disconnected(&self,_:Peer){}
}
fn root()->std::path::PathBuf{let root=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spikes/noita-entry/.build").join(format!("local-ws-test-{}",hex(&random::<16>().unwrap())));std::fs::create_dir_all(&root).unwrap();root.canonicalize().unwrap()}
fn exchange(client:&Client,b:&[u8])->Vec<u8>{let until=Instant::now()+Duration::from_secs(5);loop{let(value,_)=client.exchange(b,Duration::from_millis(100));if let Ok(b)=value{return b}assert!(Instant::now()<until,"Authenticated local exchange did not become ready: {}",serde_json::to_string(&client.stats()).unwrap());std::thread::sleep(Duration::from_millis(20));}}
#[test]fn real_ws_private_capability_second_game_and_correlated_late_reply(){
 let root=root();let handler=Arc::new(Echo{calls:AtomicU64::new(0)});let server=Server::bind(0,true,handler.clone()).unwrap();let bootstrap=Bootstrap::publish(&root,&server.descriptor()).unwrap();assert!(super::private::read(bootstrap.path()).is_ok());let client=Client::new(bootstrap.path().to_owned(),true).unwrap();let b=[1;32];assert_eq!(exchange(&client,&b),b);assert_eq!(server.stats().authenticated,1);
 let second=Client::new(bootstrap.path().to_owned(),true).unwrap();let before=handler.calls.load(Ordering::SeqCst);let(result,wrote)=second.exchange(&b,Duration::from_millis(100));assert!(result.is_err()&&!wrote);assert_eq!(handler.calls.load(Ordering::SeqCst),before);
 let(result,wrote)=client.exchange(&[99;32],Duration::from_millis(30));assert!(result.is_err()&&wrote);assert_eq!(exchange(&client,&[2;32]),[2;32]);std::thread::sleep(Duration::from_millis(170));assert!(client.stats().late>=1);
 drop(second);drop(client);drop(server);drop(bootstrap);assert!(!root.join(".nt2-local/bridge.json").exists());std::fs::remove_dir(root).unwrap();
}
#[test]fn listener_conflict_cannot_replace_descriptor_and_restart_rotates_capability(){
 let root=root();let handler=Arc::new(Echo{calls:AtomicU64::new(0)});let first=Server::bind(0,true,handler.clone()).unwrap();let port=first.address().port();let old=Bootstrap::publish(&root,&first.descriptor()).unwrap();let bytes=std::fs::read(old.path()).unwrap();assert!(Server::bind(port,true,handler.clone()).is_err());assert_eq!(std::fs::read(old.path()).unwrap(),bytes);
 let client=Client::new(old.path().to_owned(),true).unwrap();assert_eq!(exchange(&client,&[3;32]),[3;32]);drop(first);let next=Server::bind(port,true,handler).unwrap();let fresh=Bootstrap::publish(&root,&next.descriptor()).unwrap();drop(old);assert!(fresh.path().is_file());assert_ne!(std::fs::read(fresh.path()).unwrap(),bytes);assert_eq!(exchange(&client,&[4;32]),[4;32]);drop(client);drop(next);drop(fresh);std::fs::remove_dir(root).unwrap();
}
#[test]fn fragmented_binary_and_global_queue_bound_survive_disconnect(){
 use tokio_tungstenite::tungstenite::protocol::frame::{Frame,coding::{OpCode,Data}};
 let handler=Arc::new(Echo{calls:AtomicU64::new(0)});let server=Server::bind(0,true,handler.clone()).unwrap();let d=server.descriptor();let runtime=tokio::runtime::Runtime::new().unwrap();runtime.block_on(async{
  let mut ws=open(&d).await;let generation=authenticate(&mut ws,&d,random().unwrap(),random().unwrap()).await;
  let payload=[7;MAX_PAYLOAD];let frame=Envelope{reply:false,generation,request:1,payload:&payload}.encode().unwrap();
  ws.send(Message::Frame(Frame::message(frame[..100].to_vec(),OpCode::Data(Data::Binary),false))).await.unwrap();ws.send(Message::Frame(Frame::message(frame[100..].to_vec(),OpCode::Data(Data::Continue),true))).await.unwrap();
  let Message::Binary(reply)=ws.next().await.unwrap().unwrap()else{panic!("fragmented reply")};assert_eq!(Envelope::decode(&reply).unwrap().payload,payload);
  for id in 2..=10{let frame=Envelope{reply:false,generation,request:id,payload:&[99;32]}.encode().unwrap();let _=ws.send(Message::Binary(frame.into())).await;}
  while matches!(tokio::time::timeout(Duration::from_secs(1),ws.next()).await,Ok(Some(Ok(Message::Binary(_))))){ }
 });std::thread::sleep(Duration::from_millis(200));let stats=server.stats();assert!(stats.max_queued<=MAX_INFLIGHT as u64);assert_eq!(stats.queued,0,"Disconnected jobs leaked the global work bound");assert!(handler.calls.load(Ordering::SeqCst)<=9);
}
#[test]fn wrong_secret_and_spoofed_server_do_not_receive_gameplay_or_client_proof(){
 let root=root();let handler=Arc::new(Echo{calls:AtomicU64::new(0)});let server=Server::bind(0,true,handler.clone()).unwrap();let mut wrong=server.descriptor();wrong.secret=hex(&random::<32>().unwrap());let bootstrap=Bootstrap::publish(&root,&wrong).unwrap();let client=Client::new(bootstrap.path().to_owned(),true).unwrap();std::thread::sleep(Duration::from_millis(50));let(result,wrote)=client.exchange(&[1;32],Duration::from_millis(100));assert!(result.is_err()&&!wrote);assert_eq!(handler.calls.load(Ordering::SeqCst),0);drop(client);drop(server);drop(bootstrap);
 let listener=std::net::TcpListener::bind("127.0.0.1:0").unwrap();let descriptor=Descriptor{version:1,endpoint:format!("ws://127.0.0.1:{}{PATH}",listener.local_addr().unwrap().port()),app:hex(&random::<16>().unwrap()),secret:hex(&random::<32>().unwrap())};let bootstrap=Bootstrap::publish(&root,&descriptor).unwrap();let secret=descriptor.secret.clone();let thread=std::thread::spawn(move||{let(stream,_)=listener.accept().unwrap();stream.set_read_timeout(Some(Duration::from_secs(3))).unwrap();let mut ws=tokio_tungstenite::tungstenite::accept(stream).unwrap();let hello=ws.read().unwrap();assert!(!hello.into_data().windows(secret.len()).any(|w|w==secret.as_bytes()));ws.send(Message::Binary(auth(2,&[0;64]).into())).unwrap();assert!(ws.read().is_err(),"Native sent a proof to a listener without a valid server proof");});
 let client=Client::new(bootstrap.path().to_owned(),true).unwrap();thread.join().unwrap();let(result,wrote)=client.exchange(&[1;32],Duration::from_millis(50));assert!(result.is_err()&&!wrote);drop(client);drop(bootstrap);std::fs::remove_dir(root).unwrap();
}
#[cfg(windows)]#[test]fn module_mount_junction_resolves_but_private_directory_redirection_is_refused(){
 use std::os::windows::process::CommandExt;
 let root=root();let physical=root.join("physical-mod");std::fs::create_dir(&physical).unwrap();let handler=Arc::new(Echo{calls:AtomicU64::new(0)});let server=Server::bind(0,true,handler).unwrap();let bootstrap=Bootstrap::publish(&physical,&server.descriptor()).unwrap();let alias=root.join("mod-mount");
 let status=std::process::Command::new("powershell.exe").creation_flags(0x08000000).args(["-NoProfile","-Command","New-Item -ItemType Junction -Path $env:NT2_WS_LINK -Target $env:NT2_WS_TARGET | Out-Null"]).env("NT2_WS_LINK",alias.to_string_lossy().trim_start_matches(r"\\?\")).env("NT2_WS_TARGET",physical.to_string_lossy().trim_start_matches(r"\\?\")).status().unwrap();assert!(status.success());assert!(super::private::read(&alias.join(".nt2-local/bridge.json")).is_err());let path=super::private::module_descriptor(&alias.join(".nt2-local/bridge.json")).unwrap();assert!(super::private::read(&path).is_ok());let traversal=std::path::PathBuf::from(format!(r"{}\..\.nt2-local\bridge.json",alias.to_string_lossy().trim_start_matches(r"\\?\")));assert!(super::private::module_descriptor(&traversal).is_err());
 drop(bootstrap);let external=root.join("external-mod");std::fs::create_dir(&external).unwrap();let external_bootstrap=Bootstrap::publish(&external,&server.descriptor()).unwrap();let private_link=physical.join(".nt2-local");let status=std::process::Command::new("powershell.exe").creation_flags(0x08000000).args(["-NoProfile","-Command","New-Item -ItemType Junction -Path $env:NT2_WS_LINK -Target $env:NT2_WS_TARGET | Out-Null"]).env("NT2_WS_LINK",private_link.to_string_lossy().trim_start_matches(r"\\?\")).env("NT2_WS_TARGET",external.join(".nt2-local").to_string_lossy().trim_start_matches(r"\\?\")).status().unwrap();assert!(status.success());let redirected=super::private::module_descriptor(&alias.join(".nt2-local/bridge.json")).unwrap();assert!(super::private::read(&redirected).is_err());std::fs::remove_dir(private_link).unwrap();drop(external_bootstrap);std::fs::remove_dir(external).unwrap();std::fs::remove_dir(&alias).unwrap();drop(server);std::fs::remove_dir(&physical).unwrap();std::fs::remove_dir(root).unwrap();
}
