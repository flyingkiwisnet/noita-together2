//! Candidate local transaction v9. Reserve under the pending mutex, persist on
//! one bounded worker, recheck before publication. No server or frame I/O.
use super::*;use std::sync::{mpsc,atomic::AtomicU32};
use nt2_desktop_g0::startup_local::Prepared;
pub const BYTES:usize=16384;const HEADER:usize=192;
pub const READY:u32=1;pub const QUEUED:u32=2;
pub const UNAVAILABLE:u32=1;pub const EXPIRED:u32=2;pub const OWNER:u32=3;pub const CONFIG:u32=4;pub const DURABILITY:u32=5;pub const CAPACITY:u32=6;pub const RETIRED:u32=7;pub const MALFORMED:u32=8;
#[derive(Debug,Clone)]struct Outcome{state:u32,reason:u32,claim_ms:u32}
#[derive(Debug)]pub struct Attempt{bundle:Arc<Prepared>,pid:crate::local_owner::Id,instance:u64,context:[u8;16],started:Instant,deadline:Instant,canceled:AtomicBool,result:Mutex<Outcome>,queue_ms:AtomicU32}
struct Job{attempt:Arc<Attempt>,path:std::path::PathBuf,delay:Duration}
struct Offer{pid:crate::local_owner::Id,instance:u64,bootstrap:[u8;16],world:[u8;16]}
pub struct Lane{pending:Arc<Mutex<Option<crate::handoff::Pending>>>,tx:Option<mpsc::SyncSender<Job>>,worker:Option<thread::JoinHandle<()>>,history:Mutex<std::collections::VecDeque<Arc<Attempt>>>,offers:Mutex<Vec<Offer>>}
impl Lane{
 pub(crate) fn delivery(&self,pid:crate::local_owner::Id,instance:u64,request:&[u8])->Vec<u8>{
  let pending=self.pending.lock().unwrap();
  let Some(p)=pending.as_ref()else{return failure(request,RETIRED)};
  let Some(a)=p.startup_transaction.as_ref().filter(|a|a.pid==pid&&a.instance==instance&&request.get(16..32)==Some(a.context.as_slice()))else{return failure(request,OWNER)};
  reply(a,Some(p),0)
 }
 pub fn new(pending:Arc<Mutex<Option<crate::handoff::Pending>>>)->Self{let(tx,rx)=mpsc::sync_channel::<Job>(1);let p=pending.clone();let worker=thread::spawn(move||while let Ok(job)=rx.recv(){work(&p,job)});Self{pending,tx:Some(tx),worker:Some(worker),history:Mutex::new(Default::default()),offers:Mutex::new(Vec::new())}}
 pub(crate) fn retire(&self){let mut pending=self.pending.lock().unwrap();if let Some(a)=pending.as_ref().and_then(|p|p.startup_transaction.as_ref()){a.canceled.store(true,Ordering::Release);}*pending=None;drop(pending);for a in self.history.lock().unwrap().iter(){a.canceled.store(true,Ordering::Release);}}
 pub(crate) fn bound(&self,_pid:crate::local_owner::Id,_instance:u64,request:&[u8])->bool{
  if request.len()<32{return false}let input:[u8;16]=request[16..32].try_into().unwrap();let offer=request[8..12]==19u32.to_le_bytes();
  // An original from another process still names this source; its handler must
  // refuse ownership, never reinterpret it as a new B offer.
  if self.offers.lock().unwrap().iter().any(|o|if offer{o.bootstrap==input}else{o.world==input}){return true}
  !offer&&self.history.lock().unwrap().iter().any(|a|a.context==input)
 }
 #[cfg(test)]pub fn handle(&self,pid:crate::local_owner::Id,request:&[u8],delay:Duration)->Vec<u8>{self.handle_instance(pid,1,request,delay)}
 pub fn handle_instance(&self,pid:crate::local_owner::Id,instance:u64,request:&[u8],delay:Duration)->Vec<u8>{
  if instance==0{return failure(request,OWNER)}
  if request.len()<32||request[..4]!=crate::handoff::MAGIC.to_le_bytes()||request[4..8]!=9u32.to_le_bytes(){return failure(request,MALFORMED)}
  let op=u32::from_le_bytes(request[8..12].try_into().unwrap());let context:[u8;16]=request[16..32].try_into().unwrap();if context==[0;16]||request[12..16]!=[0;4]||!matches!(op,16|17|18|19)||request.len()!=if op==17{80}else{32}{return failure(request,MALFORMED)}
  let mut pending=self.pending.lock().unwrap();
  if op==17{
   if let Some(p)=pending.as_ref(){if let Some(a)=p.startup_transaction.as_ref().filter(|a|a.pid==pid&&a.instance==instance&&a.context==context&&a.bundle.world().as_bytes()==&request[32..48]&&hex(a.bundle.digest())==request[48..80]){let retire=matching(p,a);a.canceled.store(true,Ordering::Release);if retire{*pending=None;}}}
   return failure(request,RETIRED)
  }
  let Some(p)=pending.as_mut()else{drop(pending);let old=self.history.lock().unwrap().iter().find(|a|a.pid==pid&&a.instance==instance&&a.context==context).cloned();return old.map(|a|reply(&a,None,RETIRED)).unwrap_or_else(||failure(request,UNAVAILABLE))};
  if !p.startup_ready{return failure(request,UNAVAILABLE)}
  let Some(bundle)=p.startup_bundle.clone()else{return failure(request,UNAVAILABLE)};
  if !bundle.matches(&p.run,&p.epoch,p.config,p.preparation_id.as_deref(),p.seed,p.world_policy){return p.startup_transaction.as_ref().map(|a|reply(a,None,CONFIG)).unwrap_or_else(||failure(request,CONFIG))}
  if op==19{let mut offers=self.offers.lock().unwrap();if let Some(old)=offers.iter().find(|o|o.bootstrap==context){if old.pid!=pid||old.instance!=instance{return failure(request,OWNER)}if old.world!=*bundle.world().as_bytes(){return failure(request,RETIRED)}}else{if offers.len()>=16{return failure(request,CAPACITY)}offers.push(Offer{pid,instance,bootstrap:context,world:*bundle.world().as_bytes()});}let mut b=failure(request,0);b[12..16].copy_from_slice(&QUEUED.to_le_bytes());b[48..64].copy_from_slice(bundle.world().as_bytes());b[64..80].copy_from_slice(bundle.world().as_bytes());b[80..112].copy_from_slice(&hex(bundle.digest()));b[112..144].copy_from_slice(&hex(&bundle.scope().config_digest));b[160..176].copy_from_slice(&context);return b}
  if context!=*bundle.world().as_bytes(){return failure(request,OWNER)}
  if p.generation.as_ref().is_none_or(|(g,n)|g.load(Ordering::Acquire)!=*n)||!bundle.live(p.generation.as_ref().map(|(_,n)|*n).unwrap_or(0))||Instant::now()>=p.expires{return failure(request,EXPIRED)}
  if let Some(a)=p.startup_transaction.clone(){if a.pid!=pid||a.instance!=instance||a.context!=context{return failure(request,OWNER)}return reply(&a,Some(p),0)}
  if op==18{return failure(request,UNAVAILABLE)}
  if p.owner.is_some()||Instant::now()>=p.authority_until{return failure(request,OWNER)}
  if bundle.snapshot().is_err(){return failure(request,CONFIG)}
  let Some(path)=p.claim_path.clone()else{return failure(request,DURABILITY)};
  let attempt=Arc::new(Attempt{bundle,pid,instance,context,started:Instant::now(),deadline:Instant::now()+Duration::from_secs(6),canceled:AtomicBool::new(false),result:Mutex::new(Outcome{state:QUEUED,reason:0,claim_ms:0}),queue_ms:AtomicU32::new(0)});
  p.owner=Some((pid,context));p.last_seen=Some(Instant::now());p.startup_transaction=Some(attempt.clone());
  drop(pending);
  {let mut history=self.history.lock().unwrap();if history.len()==16{history.pop_front();}history.push_back(attempt.clone());}
  if self.tx.as_ref().unwrap().try_send(Job{attempt:attempt.clone(),path,delay}).is_err(){*attempt.result.lock().unwrap()=Outcome{state:0,reason:CAPACITY,claim_ms:0};}
  let pending=self.pending.lock().unwrap();reply(&attempt,pending.as_ref(),0)
 }
}
impl Drop for Lane{fn drop(&mut self){self.tx.take();if let Some(worker)=self.worker.take(){let _=worker.join();}}}
fn hex(s:&str)->Vec<u8>{s.as_bytes().chunks_exact(2).map(|b|u8::from_str_radix(std::str::from_utf8(b).unwrap_or(""),16).unwrap_or(0)).collect()}
fn matching(p:&crate::handoff::Pending,a:&Arc<Attempt>)->bool{p.owner==Some((a.pid,a.context))&&p.startup_transaction.as_ref().is_some_and(|current|Arc::ptr_eq(current,a))&&p.startup_bundle.as_ref().is_some_and(|b|b.digest()==a.bundle.digest()&&b.matches(&p.run,&p.epoch,p.config,p.preparation_id.as_deref(),p.seed,p.world_policy))&&p.generation.as_ref().is_some_and(|(g,n)|g.load(Ordering::Acquire)==*n)&&!a.canceled.load(Ordering::Acquire)}
fn work(pending:&Arc<Mutex<Option<crate::handoff::Pending>>>,job:Job){let a=job.attempt;a.queue_ms.store(a.started.elapsed().as_millis().min(u128::from(u32::MAX)) as u32,Ordering::Release);
 if !pending.lock().unwrap().as_ref().is_some_and(|p|matching(p,&a)){*a.result.lock().unwrap()=Outcome{state:0,reason:RETIRED,claim_ms:0};return}
 // This delay models disk latency only in the isolated fixture; no shared
 // mutex is held during delay, directory inspection, writes, or sync_all.
 let began=Instant::now();if !job.delay.is_zero(){thread::sleep(job.delay)}
 let result=(||->Result<(),String>{
  if a.canceled.load(Ordering::Acquire){return Err("retired".into())}
  let parent=job.path.parent().ok_or("claim parent")?;std::fs::create_dir_all(parent).map_err(|_|"claim directory")?;
  if std::fs::read_dir(parent).map_err(|_|"claim inventory")?.take(385).count()>=384{return Err("capacity".into())}
  let owner=serde_json::to_vec(&serde_json::json!({"pid":a.pid,"process_start":a.instance,"context":a.context,"bundle_digest":a.bundle.digest()})).map_err(|_|"owner encoding")?;
  use std::io::Write;
  let owner_path=job.path.with_extension("startup-owner.json");match std::fs::OpenOptions::new().write(true).create_new(true).open(&owner_path){Ok(mut f)=>{f.write_all(&owner).map_err(|_|"owner write")?;f.sync_all().map_err(|_|"owner flush")?;},Err(e) if e.kind()==std::io::ErrorKind::AlreadyExists=>{if std::fs::read(owner_path).map_err(|_|"owner read")?!=owner{return Err("different original process".into())}},Err(_)=>return Err("owner create".into())}
  let witness=a.pid.witness(a.context);
  match std::fs::OpenOptions::new().write(true).create_new(true).open(&job.path){Ok(mut f)=>{f.write_all(&witness).map_err(|_|"claim write")?;f.sync_all().map_err(|_|"claim flush")?;},Err(e) if e.kind()==std::io::ErrorKind::AlreadyExists=>{if std::fs::read(&job.path).map_err(|_|"claim read")?!=witness{return Err("uncertain original".into())}},Err(_)=>return Err("claim create".into())}
  let snapshot=a.bundle.snapshot()?;let value=serde_json::json!({"version":1,"pid":a.pid,"process_start":a.instance,"context":a.context,"bundle":serde_json::from_slice::<serde_json::Value>(&snapshot).map_err(|_|"snapshot shape")?});let bytes=serde_json::to_vec(&value).map_err(|_|"outcome encoding")?;if bytes.len()>BYTES{return Err("outcome bound".into())}let path=job.path.with_extension("startup.json");
  match std::fs::OpenOptions::new().write(true).create_new(true).open(&path){Ok(mut f)=>{f.write_all(&bytes).map_err(|_|"outcome write")?;f.sync_all().map_err(|_|"outcome flush")?;},Err(e) if e.kind()==std::io::ErrorKind::AlreadyExists=>{if std::fs::read(path).map_err(|_|"outcome read")?!=bytes{return Err("outcome differs".into())}},Err(_)=>return Err("outcome create".into())}Ok(())
 })();
 let ms=began.elapsed().as_millis().min(u128::from(u32::MAX)) as u32;
 let current=pending.lock().unwrap().as_ref().is_some_and(|p|matching(p,&a));let outcome=match result{Ok(())=>Outcome{state:READY,reason:if current{0}else{RETIRED},claim_ms:ms},Err(e)=>Outcome{state:0,reason:if e=="capacity"{CAPACITY}else if e=="retired"{RETIRED}else{DURABILITY},claim_ms:ms}};
 *a.result.lock().unwrap()=outcome;
}
pub(crate) fn failure(request:&[u8],reason:u32)->Vec<u8>{let mut b=vec![0;BYTES];b[..4].copy_from_slice(&crate::handoff::MAGIC.to_le_bytes());b[4..8].copy_from_slice(&9u32.to_le_bytes());b[8..12].copy_from_slice(&16u32.to_le_bytes());b[16..20].copy_from_slice(&reason.to_le_bytes());if request.len()>=32{b[48..64].copy_from_slice(&request[16..32]);}b}
fn reply(a:&Arc<Attempt>,current:Option<&crate::handoff::Pending>,forced:u32)->Vec<u8>{let outcome=a.result.lock().unwrap().clone();let mut request=[0;32];request[16..32].copy_from_slice(&a.context);let mut b=failure(&request,0);b[64..80].copy_from_slice(a.bundle.world().as_bytes());b[80..112].copy_from_slice(&hex(a.bundle.digest()));b[112..144].copy_from_slice(&hex(&a.bundle.scope().config_digest));b[144..148].copy_from_slice(&outcome.state.to_le_bytes());b[40..44].copy_from_slice(&a.queue_ms.load(Ordering::Acquire).to_le_bytes());b[44..48].copy_from_slice(&outcome.claim_ms.to_le_bytes());
 let reason=if forced!=0{forced}else if a.canceled.load(Ordering::Acquire)||current.is_none_or(|p|!matching(p,a)){RETIRED}else if Instant::now()>=a.deadline||current.is_some_and(|p|Instant::now()>=p.expires){EXPIRED}else{outcome.reason};b[16..20].copy_from_slice(&reason.to_le_bytes());
 if reason!=0{return b}b[12..16].copy_from_slice(&outcome.state.to_le_bytes());if outcome.state==READY{let Some(p)=current else{return b};let lease=p.authority_until.saturating_duration_since(Instant::now()).as_millis().min(12000) as u32;let snapshot=a.bundle.snapshot().unwrap_or_default();if lease==0||snapshot.is_empty()||snapshot.len()>BYTES-HEADER{b[12..16].fill(0);b[16..20].copy_from_slice(&EXPIRED.to_le_bytes());return b}b[20..24].copy_from_slice(&a.bundle.seed().to_le_bytes());b[24..28].copy_from_slice(&a.bundle.policy().to_le_bytes());b[28..32].copy_from_slice(&(a.bundle.generation() as u32).to_le_bytes());b[32..36].copy_from_slice(&lease.to_le_bytes());b[36..40].copy_from_slice(&(snapshot.len() as u32).to_le_bytes());b[HEADER..HEADER+snapshot.len()].copy_from_slice(&snapshot);}b}
#[cfg(test)]#[path="startup_transaction_test.rs"]mod tests;
