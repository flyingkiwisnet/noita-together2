//! Separate provisional native slot. No HTTP/storage/lifecycle work on this lane.
use super::*;use nt2_desktop_g0::provisional_local::{Worker,Phase,View};
struct Attempt{pid:crate::local_owner::Id,start:u64,bootstrap:[u8;16],context:[u8;16],began:Instant,requested:bool,queued:bool,closed:bool}
pub(crate) struct Bridge{pub(crate) worker:Arc<Worker>,pub(crate) pending:Arc<Mutex<Option<crate::handoff::Pending>>>,generation:Arc<std::sync::atomic::AtomicU64>,attempt:Mutex<Option<Attempt>>,restored:Mutex<Option<(crate::local_owner::Id,u64,[u8;16])>>}
impl Bridge{
 pub(crate) fn new(worker:Arc<Worker>)->Self{Self{worker,pending:Arc::new(Mutex::new(None)),generation:Arc::new(std::sync::atomic::AtomicU64::new(1)),attempt:Mutex::new(None),restored:Mutex::new(None)}}
 pub(crate) fn bound(&self,_pid:crate::local_owner::Id,_start:u64,request:&[u8])->bool{if request.len()<32{return false}let input:[u8;16]=request[16..32].try_into().unwrap();self.attempt.lock().unwrap().as_ref().is_some_and(|a|if request[8..12]==19u32.to_le_bytes(){a.bootstrap==input}else{a.context==input})}
 pub(crate) fn owns_context(&self,pid:crate::local_owner::Id,context:&[u8])->bool{self.pending.lock().unwrap().as_ref().is_some_and(|p|p.owner.is_some_and(|o|o.0==pid&&o.1.as_slice()==context))}
 pub(crate) fn collection(&self,c:&super::rejoin::Contact)->Option<crate::handoff::Pending>{let(_,v,_)=self.worker.view();let v=v?;if v.world.as_bytes()!=&c.association||matches!(v.phase,Phase::Detached|Phase::CancelSubmitted|Phase::Cancelled){return None}self.pending.lock().unwrap().as_ref().filter(|p|p.owner==Some((c.pid,c.context))&&p.actual_seed==Some(c.actual)&&p.generation.as_ref().is_some_and(|(g,n)|g.load(Ordering::Acquire)==*n)).cloned()}
 pub(crate) fn explicit_offline(&self,pid:crate::local_owner::Id,start:u64,context:[u8;16])->bool{
  let original=if let Some(a)=self.attempt.lock().unwrap().as_ref(){a.pid==pid&&a.start==start&&(a.context==context||a.bootstrap==context)}else{self.restored.lock().unwrap().as_ref()==Some(&(pid,start,context))&&start!=0};
  if !original{return false}
  self.detach_local().is_ok()
 }
 pub(crate) fn detach_local(&self)->Result<(),String>{self.worker.detach()?;self.generation.fetch_add(1,Ordering::AcqRel);*self.pending.lock().unwrap()=None;Ok(())}
 pub(crate) fn frame(&self,pid:crate::local_owner::Id,request:&[u8])->Option<Vec<u8>>{
  if self.worker.view().1.is_some_and(|v|matches!(v.phase,Phase::Detached|Phase::CancelSubmitted|Phase::Cancelled)){return None}
  if request.len()<32{return None}
  let context=if request[4..8]==5u32.to_le_bytes(){request.get(24..40)?}else{request.get(16..32)?};
  if !self.owns_context(pid,context){return None}
  let mut pending=self.pending.lock().unwrap();if let Some(p)=pending.as_mut(){p.authority_until=Instant::now()+Duration::from_secs(6);}
  Some(super::c2_transition(&mut pending,pid,request))
 }
 pub(crate) fn handle(&self,pid:crate::local_owner::Id,start:u64,request:&[u8])->Vec<u8>{
  let mut reply=vec![0u8;16384];reply[..4].copy_from_slice(&crate::handoff::MAGIC.to_le_bytes());reply[4..8].copy_from_slice(&9u32.to_le_bytes());reply[8..12].copy_from_slice(&16u32.to_le_bytes());reply[176..180].copy_from_slice(&1u32.to_le_bytes());
  let fail=|mut r:Vec<u8>,reason:u32|{r[12..16].fill(0);r[16..20].copy_from_slice(&reason.to_le_bytes());r};
  if start==0||request.len()<32||request[..4]!=crate::handoff::MAGIC.to_le_bytes()||request[4..8]!=9u32.to_le_bytes()||request[12..16]!=[0;4]{return fail(reply,8)}
  let op=u32::from_le_bytes(request[8..12].try_into().unwrap());let input:[u8;16]=request[16..32].try_into().unwrap();reply[48..64].copy_from_slice(&input);
  let(busy,view,error)=self.worker.view();let Some(v)=view else{return fail(reply,1)};
  reply[64..80].copy_from_slice(v.world.as_bytes());reply[80..112].copy_from_slice(&unhex(&v.digest));reply[112..144].copy_from_slice(&unhex(&v.scope.config_digest));
  if error.is_some()||!matches!(v.phase,Phase::Staged|Phase::Claimed|Phase::ActivationSubmitted|Phase::ActivationReconciled){return fail(reply,7)}
  let mut attempt=self.attempt.lock().unwrap();
  if op==19{
   if request.len()!=32||v.until.is_none_or(|at|Instant::now()>=at){return fail(reply,2)}
   if let Some(a)=attempt.as_ref(){if a.closed||a.pid!=pid||a.start!=start||a.bootstrap!=input{return fail(reply,3)}}
   else{if v.owner.is_some()||self.worker.begin_fresh(v.world).is_err(){return fail(reply,3)}*attempt=Some(Attempt{pid,start,bootstrap:input,context:*v.context.as_bytes(),began:Instant::now(),requested:false,queued:false,closed:false});}
   reply[12..16].copy_from_slice(&2u32.to_le_bytes());reply[48..64].copy_from_slice(v.context.as_bytes());reply[64..80].copy_from_slice(v.world.as_bytes());reply[80..112].copy_from_slice(&unhex(&v.digest));reply[112..144].copy_from_slice(&unhex(&v.scope.config_digest));reply[160..176].copy_from_slice(&input);return reply
  }
  let Some(a)=attempt.as_mut()else{return fail(reply,3)};
  if a.pid!=pid||a.start!=start||a.context!=input||a.closed{return fail(reply,3)}
  if op==17{
   if request.len()!=80||request[32..48]!=*v.world.as_bytes()||request[48..80]!=unhex(&v.digest){return fail(reply,3)}
   // Incidental close is NOT world-lifetime offline/cancellation. Only this
   // exact fresh native attempt closes; the saved B journal remains recoverable.
   a.closed=true;self.generation.fetch_add(1,Ordering::AcqRel);*self.pending.lock().unwrap()=None;return fail(reply,7)
  }
  if !matches!(op,16|18)||request.len()!=32{return fail(reply,8)}
  if a.began.elapsed()>=Duration::from_secs(6)||v.until.is_none_or(|at|Instant::now()>=at){a.closed=true;return fail(reply,2)}
  if op==16{a.requested=true;}
  if !a.queued{if !a.requested{return fail(reply,1)}if !busy&&self.worker.claim(v.world,pid,start).is_ok(){a.queued=true;}reply[12..16].copy_from_slice(&2u32.to_le_bytes());return reply}
  if busy||v.phase==Phase::Staged{reply[12..16].copy_from_slice(&2u32.to_le_bytes());return reply}
  if v.owner!=Some((pid,start)){return fail(reply,3)}
  if v.snapshot.len()>12000{return fail(reply,8)}
  self.install_held(&v,pid,input);
  reply[12..16].copy_from_slice(&1u32.to_le_bytes());reply[20..24].copy_from_slice(&v.seed.to_le_bytes());reply[24..28].copy_from_slice(&v.policy.to_le_bytes());reply[28..32].copy_from_slice(&1u32.to_le_bytes());reply[32..36].copy_from_slice(&6000u32.to_le_bytes());reply[36..40].copy_from_slice(&(v.snapshot.len() as u32).to_le_bytes());reply[48..64].copy_from_slice(&input);reply[64..80].copy_from_slice(v.world.as_bytes());reply[80..112].copy_from_slice(&unhex(&v.digest));reply[112..144].copy_from_slice(&unhex(&v.scope.config_digest));reply[192..192+v.snapshot.len()].copy_from_slice(&v.snapshot);reply
 }
 pub(crate) fn install_held(&self,v:&View,pid:crate::local_owner::Id,context:[u8;16]){
  if matches!(v.phase,Phase::Detached|Phase::CancelSubmitted|Phase::Cancelled){return}
  let mut pending=self.pending.lock().unwrap();if pending.as_ref().is_some_and(|p|p.owner==Some((pid,context))){return}
  *self.restored.lock().unwrap()=super::owner_instance(pid).map(|start|(pid,start,context));
  let mut p=crate::handoff::new(v.seed,v.scope.run.clone(),v.scope.config);p.epoch=v.scope.run_epoch.clone();p.preparation_id=Some(v.scope.preparation.clone());p.world_policy=v.policy;p.native_v2=true;p.native_epoch=Some(1);p.owner=Some((pid,context));p.expires=v.until.unwrap_or_else(Instant::now);p.authority_until=Instant::now()+Duration::from_secs(6);p.last_seen=Some(Instant::now());p.generation=Some((self.generation.clone(),self.generation.load(Ordering::Acquire)));p.startup_ready=true;*pending=Some(p);
 }
 pub(crate) fn install_restored(&self,v:&View,c:&super::rejoin::Contact){let existing=self.owns_context(c.pid,&c.context);self.install_held(v,c.pid,c.context);if !existing{if let Some(p)=self.pending.lock().unwrap().as_mut(){p.actual_seed=Some(c.actual);p.accepted=true;}}}
 pub(crate) fn metadata(&self,pid:crate::local_owner::Id,context:[u8;16])->Option<Vec<u8>>{
  let(_,view,_)=self.worker.view();let view=view?;if matches!(view.phase,Phase::Detached|Phase::CancelSubmitted|Phase::Cancelled){return None}let pending=self.pending.lock().unwrap();let p=pending.as_ref()?;
  if p.owner!=Some((pid,context))||!p.accepted||!p.loaded{return None}let actual=p.actual_seed?;
  let marker=nt2_desktop_g0::startup_local::WaitingMarker{kind:"nt2-provisional".into(),version:1,local_world:view.world,bundle_digest:view.digest,actual_seed:actual.to_string(),loot:if view.policy==2{"individual:0".into()}else{format!("shared:{}:{actual}",u8::from(view.policy==5))}};marker.encode().ok()
 }
}
fn unhex(value:&str)->Vec<u8>{value.as_bytes().chunks_exact(2).map(|p|u8::from_str_radix(std::str::from_utf8(p).unwrap_or(""),16).unwrap_or(0)).collect()}
#[cfg(test)]#[path="entry_bridge_test.rs"]pub(crate) mod tests;
