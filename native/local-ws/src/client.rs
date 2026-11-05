use crate::protocol::*;
use futures_util::{SinkExt,StreamExt};
use std::{path::PathBuf,sync::{Arc,Mutex,atomic::{AtomicBool,AtomicU64,Ordering}},time::{Duration,Instant}};
use tokio::{net::TcpStream,sync::mpsc,time::timeout};
use tokio_tungstenite::{client_async_with_config,tungstenite::Message};
use zeroize::Zeroizing;
#[derive(Clone,Default,serde::Serialize)]
pub struct Stats{pub stage:&'static str,pub attempts:u64,pub authenticated:u64,pub disconnects:u64,pub sent:u64,pub received:u64,pub expired:u64,pub late:u64,pub refused:u64,pub max_latency_us:u64,pub latency_p95_us:u64,pub retry_delay_ms:u64,pub generation:u64,#[serde(skip)]latencies:std::collections::VecDeque<u64>}
struct Job{request:Vec<u8>,until:Instant,wrote:Arc<AtomicBool>,response:std::sync::mpsc::SyncSender<Result<(Vec<u8>,[u8;16]),()>>}
pub struct Client{tx:mpsc::Sender<Job>,stats:Arc<Mutex<Stats>>}
impl Client{
 pub fn new(path:PathBuf,private:bool)->Result<Self,String>{
  let native=random::<16>()?;let(tx,rx)=mpsc::channel(MAX_INFLIGHT);let stats=Arc::new(Mutex::new(Stats::default()));let report=stats.clone();
  std::thread::Builder::new().name("nt2-local-websocket".into()).spawn(move||{if let Ok(runtime)=tokio::runtime::Builder::new_multi_thread().worker_threads(2).enable_all().build(){runtime.block_on(pump(path,private,native,rx,report));}}).map_err(|_|"Local socket worker unavailable")?;Ok(Self{tx,stats})
 }
 pub fn stats(&self)->Stats{let mut s=self.stats.lock().unwrap().clone();let mut samples=s.latencies.iter().copied().collect::<Vec<_>>();samples.sort_unstable();s.latency_p95_us=if samples.is_empty(){0}else{samples[(samples.len()*95).div_ceil(100)-1]};s}
 pub fn exchange(&self,request:&[u8],budget:Duration)->(Result<Vec<u8>,()>,bool){
  let(reply,wrote)=self.exchange_bound(request,budget);(reply.map(|(payload,_)|payload),wrote)
 }
 pub fn exchange_bound(&self,request:&[u8],budget:Duration)->(Result<(Vec<u8>,[u8;16]),()>,bool){
  if request.len()<32||request.len()>MAX_PAYLOAD||budget.is_zero()||budget>Duration::from_secs(12){return(Err(()),false)}
  let start=Instant::now();let until=start+budget;let(tx,rx)=std::sync::mpsc::sync_channel(1);let wrote=Arc::new(AtomicBool::new(false));
  let job=Job{request:request.to_vec(),until,wrote:wrote.clone(),response:tx};
  if self.tx.try_send(job).is_err(){self.stats.lock().unwrap().refused+=1;return(Err(()),false)}
  let result=rx.recv_timeout(until.saturating_duration_since(Instant::now())).unwrap_or(Err(()));let elapsed=start.elapsed().as_micros().min(u64::MAX as u128)as u64;{let mut s=self.stats.lock().unwrap();s.max_latency_us=s.max_latency_us.max(elapsed);if s.latencies.len()==256{s.latencies.pop_front();}s.latencies.push_back(elapsed);}
  (result,wrote.load(Ordering::Acquire))
 }
}
async fn descriptor(path:&std::path::Path,private:bool)->Result<(std::net::SocketAddrV4,[u8;16],Zeroizing<[u8;32]>),()>{
 // The private-file implementation additionally validates path redirection/ACLs.
 let path=path.to_path_buf();tokio::task::spawn_blocking(move||{
  let d=crate::private::read(&path).map_err(|_|())?;let(address,app,secret)=d.validate(private).map_err(|_|())?;Ok((address,app,Zeroizing::new(secret)))
 }).await.map_err(|_|())?
}
async fn connect(path:&std::path::Path,private:bool,native:&[u8;16],stats:&Arc<Mutex<Stats>>)->Result<(tokio_tungstenite::WebSocketStream<TcpStream>,u64,[u8;16]),()>{
 stats.lock().unwrap().stage="descriptor";let(address,app,secret)=descriptor(path,private).await?;stats.lock().unwrap().stage="tcp";let stream=TcpStream::connect(address).await.map_err(|_|())?;stream.set_nodelay(true).map_err(|_|())?;
 let url=format!("ws://127.0.0.1:{}{PATH}",address.port());stats.lock().unwrap().stage="upgrade";let(mut ws,_)=client_async_with_config(url,stream,Some(ws_config())).await.map_err(|_|())?;
 stats.lock().unwrap().stage="server-proof";let client=random::<32>().map_err(|_|())?;let mut hello=app.to_vec();hello.extend_from_slice(native);hello.extend_from_slice(&client);ws.send(Message::Binary(auth(1,&hello).into())).await.map_err(|_|())?;
 let Some(Ok(Message::Binary(challenge)))=ws.next().await else{return Err(())};let body=auth_body(&challenge,2,64).map_err(|_|())?;let nonce:[u8;32]=body[..32].try_into().unwrap();if !verify(&secret,true,&app,native,&client,&nonce,&body[32..]){return Err(())}
 stats.lock().unwrap().stage="accepted";let finish=proof(&secret,false,&app,native,&client,&nonce);ws.send(Message::Binary(auth(3,&finish).into())).await.map_err(|_|())?;
 let Some(Ok(Message::Binary(accepted)))=ws.next().await else{return Err(())};let b=auth_body(&accepted,4,8).map_err(|_|())?;let generation=u64::from_le_bytes(b.try_into().unwrap());if generation==0{return Err(())}Ok((ws,generation,app))
}
async fn pump(path:PathBuf,private:bool,native:[u8;16],mut jobs:mpsc::Receiver<Job>,stats:Arc<Mutex<Stats>>){
 let mut delay=250u64;let ids=AtomicU64::new(0);
 loop{
  if jobs.is_closed(){return}stats.lock().unwrap().attempts+=1;
  let connection=timeout(Duration::from_millis(HANDSHAKE_MS),connect(&path,private,&native,&stats)).await;
  if let Ok(Ok((ws,generation,app)))=connection{
   {let mut s=stats.lock().unwrap();s.authenticated+=1;s.generation=generation;s.stage="ready";s.retry_delay_ms=0;}delay=250;
   let(mut writer,mut reader)=ws.split();let mut pending=std::collections::BTreeMap::<u64,Job>::new();
   loop{tokio::select!{
    job=jobs.recv()=>{let Some(job)=job else{return};if Instant::now()>=job.until{stats.lock().unwrap().expired+=1;let _=job.response.send(Err(()));continue}if pending.len()>=MAX_INFLIGHT{stats.lock().unwrap().refused+=1;let _=job.response.send(Err(()));continue}
     let id=ids.fetch_add(1,Ordering::AcqRel)+1;if id==0{let _=job.response.send(Err(()));break}let b=Envelope{reply:false,generation,request:id,payload:&job.request}.encode().unwrap();job.wrote.store(true,Ordering::Release);
     let budget=job.until.saturating_duration_since(Instant::now());if !matches!(timeout(budget,writer.send(Message::Binary(b.into()))).await,Ok(Ok(()))){let _=job.response.send(Err(()));break}stats.lock().unwrap().sent+=1;pending.insert(id,job);
    },
    received=reader.next()=>{let bytes=match received{Some(Ok(Message::Binary(b)))=>b,Some(Ok(Message::Ping(b)))=>{if writer.send(Message::Pong(b)).await.is_err(){break}continue},Some(Ok(Message::Pong(_)))=>continue,_=>break};
     let Ok(reply)=Envelope::decode(&bytes)else{break};if !reply.reply||reply.generation!=generation{break}
     if let Some(job)=pending.remove(&reply.request){if Instant::now()<job.until{stats.lock().unwrap().received+=1;let _=job.response.send(Ok((reply.payload.to_vec(),app)));}else{stats.lock().unwrap().late+=1;let _=job.response.send(Err(()));}}else{stats.lock().unwrap().late+=1;}
    },
    _=tokio::time::sleep(Duration::from_millis(5))=>{let expired=pending.iter().filter_map(|(id,j)|(Instant::now()>=j.until).then_some(*id)).collect::<Vec<_>>();for id in expired{let job=pending.remove(&id).unwrap();stats.lock().unwrap().expired+=1;let _=job.response.send(Err(()));}}
   }}
   stats.lock().unwrap().disconnects+=1;for(_,job)in pending{let _=job.response.send(Err(()));}
  }
  // Connection setup is separate from per-request deadlines. Unsent expired
  // work is refused; no expired request or uncertain mutation is replayed.
  let jitter=random::<1>().map(|b|u64::from(b[0])%63).unwrap_or(0);let retry=(delay+jitter).min(2000);stats.lock().unwrap().retry_delay_ms=retry;let until=tokio::time::Instant::now()+Duration::from_millis(retry);delay=(delay*2).min(2000);
  loop{tokio::select!{_ = tokio::time::sleep_until(until)=>break,job=jobs.recv()=>{let Some(job)=job else{return};let _=job.response.send(Err(()));}}}
 }
}
