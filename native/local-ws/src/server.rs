use crate::protocol::*;
use futures_util::{SinkExt,StreamExt};
use std::{sync::{Arc,Mutex,atomic::{AtomicBool,AtomicU64,Ordering}},net::{SocketAddr,SocketAddrV4,Ipv4Addr},time::{Duration,Instant}};
use tokio::{net::TcpStream,sync::{mpsc,Semaphore},time::timeout};
use tokio_tungstenite::{tungstenite::{Message,handshake::server::{Request,Response,ErrorResponse}},accept_hdr_async_with_config};
use zeroize::Zeroizing;
#[derive(Clone,Copy,Debug,PartialEq,Eq,Hash)]
pub struct Peer{pub app:[u8;16],pub native:[u8;16],pub generation:u64}
pub trait Handler:Send+Sync+'static{
 fn connected(&self,peer:Peer)->bool;
 fn request(&self,peer:Peer,request:&[u8])->Option<Vec<u8>>;
 /// Recheck local generation AND current world fences immediately before delivery.
 fn delivery(&self,peer:Peer,request:&[u8],reply:Vec<u8>)->Option<Vec<u8>>;
 fn disconnected(&self,peer:Peer);
}
#[derive(Default,Clone,serde::Serialize)]
pub struct Stats{pub authenticated:u64,pub refused:u64,pub malformed:u64,pub requests:u64,pub delivered:u64,pub stale:u64,pub queued:u64,pub max_queued:u64,pub max_latency_us:u64,pub generation:u64,pub closed:u64,pub close_reason:u64,pub last_request:u64,pub last_kind:u64}
struct Shared{app:[u8;16],secret:Zeroizing<[u8;32]>,active:Mutex<Option<Peer>>,generation:AtomicU64,seen:Mutex<std::collections::VecDeque<[u8;32]>>,stats:Mutex<Stats>,stop:AtomicBool,blocked:AtomicBool,handler:Arc<dyn Handler>,work:Arc<Semaphore>}
pub struct Server{address:SocketAddrV4,private:bool,shared:Arc<Shared>,thread:Option<std::thread::JoinHandle<()>>}
impl Server{
 pub fn bind(port:u16,private:bool,handler:Arc<dyn Handler>)->Result<Self,String>{
  if !private&&port!=PORT{return Err("Fixed local bridge port required".into())}
  let listener=std::net::TcpListener::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST,port)).map_err(|_|"Local bridge already in use or unavailable")?;
  listener.set_nonblocking(true).map_err(|_|"Local listener unavailable")?;
  let SocketAddr::V4(address)=listener.local_addr().map_err(|_|"Local listener unavailable")? else{return Err("IPv4 loopback required".into())};
  let shared=Arc::new(Shared{app:random()?,secret:Zeroizing::new(random()?),active:Mutex::new(None),generation:AtomicU64::new(0),seen:Mutex::new(Default::default()),stats:Mutex::new(Default::default()),stop:AtomicBool::new(false),blocked:AtomicBool::new(false),handler,work:Arc::new(Semaphore::new(MAX_INFLIGHT))});
  let worker=shared.clone();let listener_port=address.port();let thread=std::thread::Builder::new().name("nt2-local-listener".into()).spawn(move||{
   let Ok(runtime)=tokio::runtime::Builder::new_multi_thread().worker_threads(2).enable_all().build()else{return};
   runtime.block_on(async move{let Ok(listener)=tokio::net::TcpListener::from_std(listener)else{return};let slots=Arc::new(Semaphore::new(4));
    while !worker.stop.load(Ordering::Acquire){tokio::select!{
     _=tokio::time::sleep(Duration::from_millis(50))=>{},
     accepted=listener.accept()=>{if let Ok((stream,address))=accepted{if !address.ip().is_loopback(){continue}let Ok(permit)=slots.clone().try_acquire_owned()else{worker.stats.lock().unwrap().refused+=1;continue};let state=worker.clone();tokio::spawn(async move{let _permit=permit;let _=connection(stream,address.port(),state,listener_port).await;});}}
    }}
   });
  }).map_err(|_|"Local listener worker unavailable")?;
  Ok(Self{address,private,shared,thread:Some(thread)})
 }
 pub fn descriptor(&self)->Descriptor{Descriptor{version:1,endpoint:format!("ws://127.0.0.1:{}{PATH}",self.address.port()),app:hex(&self.shared.app),secret:hex(self.shared.secret.as_ref())}}
 pub fn address(&self)->SocketAddrV4{self.address}
 pub fn stats(&self)->Stats{self.shared.stats.lock().unwrap().clone()}
 pub fn current(&self)->Option<Peer>{*self.shared.active.lock().unwrap()}
 /// Fault injection is unavailable on the production listener and has no UI.
 pub fn fixture_outage(&self,blocked:bool)->Result<(),String>{if !self.private{return Err("Private fixture required".into())}self.shared.blocked.store(blocked,Ordering::Release);Ok(())}
}
impl Drop for Server{fn drop(&mut self){self.shared.stop.store(true,Ordering::Release);if let Some(t)=self.thread.take(){let _=t.join();}}}
struct Lease{state:Arc<Shared>,peer:Peer,reason:u64}
struct Work{state:Arc<Shared>,_permit:tokio::sync::OwnedSemaphorePermit}
impl Drop for Work{fn drop(&mut self){self.state.stats.lock().unwrap().queued-=1;}}
impl Drop for Lease{fn drop(&mut self){let mut active=self.state.active.lock().unwrap();if *active==Some(self.peer){*active=None;self.state.handler.disconnected(self.peer);}let mut s=self.state.stats.lock().unwrap();s.closed+=1;s.close_reason=self.reason;}}
async fn header_bound(stream:&TcpStream)->Result<(),()>{
 timeout(Duration::from_millis(HANDSHAKE_MS),async{let mut b=[0;4097];loop{let n=stream.peek(&mut b).await.map_err(|_|())?;if n==0||n>4096{return Err(())}if b[..n].windows(4).any(|w|w==b"\r\n\r\n"){return Ok(())}tokio::time::sleep(Duration::from_millis(2)).await}}).await.map_err(|_|())?
}
async fn connection(stream:TcpStream,_remote:u16,state:Arc<Shared>,port:u16)->Result<(),()>{
 stream.set_nodelay(true).map_err(|_|())?;
 if state.blocked.load(Ordering::Acquire){return Err(())}
 header_bound(&stream).await.map_err(|_|{state.stats.lock().unwrap().refused+=1;})?;
 let callback=move|request:&Request,response:Response|->Result<Response,ErrorResponse>{
  if request.uri().path()!=PATH||request.uri().query().is_some()||request.headers().contains_key("origin")||request.headers().contains_key("sec-websocket-protocol")||request.headers().contains_key("sec-websocket-extensions")||request.headers().get_all("host").iter().count()!=1||request.headers().get("host").and_then(|v|v.to_str().ok())!=Some(format!("127.0.0.1:{port}").as_str()){
   return Err(tokio_tungstenite::tungstenite::http::Response::builder().status(403).body(Some("Local bridge request refused".into())).unwrap())
  }Ok(response)
 };
 let mut ws=timeout(Duration::from_millis(HANDSHAKE_MS),accept_hdr_async_with_config(stream,callback,Some(ws_config()))).await.map_err(|_|{state.stats.lock().unwrap().refused+=1;})?.map_err(|_|{state.stats.lock().unwrap().refused+=1;})?;
 let accepted=timeout(Duration::from_millis(HANDSHAKE_MS),async{
  let Some(Ok(Message::Binary(hello)))=ws.next().await else{return Err(())};let body=auth_body(&hello,1,64).map_err(|_|())?;
  if body[..16]!=state.app{return Err(())}let native:[u8;16]=body[16..32].try_into().unwrap();let client:[u8;32]=body[32..64].try_into().unwrap();if native==[0;16]||client==[0;32]{return Err(())}
  {let mut seen=state.seen.lock().unwrap();if seen.contains(&client){return Err(())}if seen.len()==256{seen.pop_front();}seen.push_back(client);}
  let nonce=random::<32>().map_err(|_|())?;let mut body=nonce.to_vec();body.extend_from_slice(&proof(&state.secret,true,&state.app,&native,&client,&nonce));ws.send(Message::Binary(auth(2,&body).into())).await.map_err(|_|())?;
  let Some(Ok(Message::Binary(finish)))=ws.next().await else{return Err(())};let body=auth_body(&finish,3,32).map_err(|_|())?;if !verify(&state.secret,false,&state.app,&native,&client,&nonce,body){return Err(())}
  let generation=state.generation.fetch_add(1,Ordering::AcqRel)+1;if generation==0{return Err(())}let peer=Peer{app:state.app,native,generation};
  {let mut active=state.active.lock().unwrap();if active.is_some()||!state.handler.connected(peer){return Err(())}*active=Some(peer);}
  let lease=Lease{state:state.clone(),peer,reason:1};ws.send(Message::Binary(auth(4,&generation.to_le_bytes()).into())).await.map_err(|_|())?;
  Ok(lease)
 }).await;
 let mut lease=match accepted{Ok(Ok(lease))=>lease,_=>{state.stats.lock().unwrap().refused+=1;return Err(())}};
 let peer=lease.peer;{let mut s=state.stats.lock().unwrap();s.authenticated+=1;s.generation=peer.generation;}
 let(mut writer,mut reader)=ws.split();let(tx,mut rx)=mpsc::channel::<(u64,Vec<u8>,Vec<u8>,Instant,Work)>(MAX_INFLIGHT);let mut last=0;
 let mut close_reason=1;let result=async{
  loop{if state.stop.load(Ordering::Acquire)||state.blocked.load(Ordering::Acquire){close_reason=if state.stop.load(Ordering::Acquire){6}else{7};return Ok(())}tokio::select!{
   _=tokio::time::sleep(Duration::from_millis(50))=>{},
   received=reader.next()=>{let bytes=match received{Some(Ok(Message::Binary(bytes)))=>bytes,Some(Ok(Message::Ping(bytes)))=>{writer.send(Message::Pong(bytes)).await.map_err(|_|())?;continue},Some(Ok(Message::Pong(_)))=>continue,Some(Ok(Message::Close(_)))=>{close_reason=2;return Ok(())},_=>return Err(())};
    let envelope=Envelope::decode(&bytes).map_err(|_|{close_reason=3;state.stats.lock().unwrap().malformed+=1;})?;if envelope.reply||envelope.generation!=peer.generation||envelope.request<=last{close_reason=4;state.stats.lock().unwrap().malformed+=1;return Err(())}last=envelope.request;
    let permit=state.work.clone().try_acquire_owned().map_err(|_|{close_reason=5;})?;let request=envelope.payload.to_vec();let id=envelope.request;let state=state.clone();let tx=tx.clone();let start=Instant::now();
    {let mut s=state.stats.lock().unwrap();s.requests+=1;s.last_request=id;s.last_kind=u64::from(u32::from_le_bytes(request[..4].try_into().unwrap()));s.queued+=1;s.max_queued=s.max_queued.max(s.queued);}
    let work=Work{state:state.clone(),_permit:permit};tokio::task::spawn_blocking(move||{let reply=state.handler.request(peer,&request);if let Some(reply)=reply{let _=tx.blocking_send((id,request,reply,start,work));}});
   },
   ready=rx.recv()=>{let Some((id,request,reply,start,_work))=ready else{return Err(())};let latency=start.elapsed().as_micros().min(u64::MAX as u128)as u64;{let mut s=state.stats.lock().unwrap();s.max_latency_us=s.max_latency_us.max(latency);}
    if *state.active.lock().unwrap()!=Some(peer){state.stats.lock().unwrap().stale+=1;return Err(())}
    let Some(reply)=state.handler.delivery(peer,&request,reply)else{state.stats.lock().unwrap().stale+=1;continue};let b=Envelope{reply:true,generation:peer.generation,request:id,payload:&reply}.encode().map_err(|_|())?;writer.send(Message::Binary(b.into())).await.map_err(|_|())?;state.stats.lock().unwrap().delivered+=1;
   }
  }}
 }.await;
 lease.reason=close_reason;drop(lease);result
}
