//! Ordinary app actor: Core has one owner, never an async mutex held over I/O.
//! Status snapshots and independently scheduled auth/native/gameplay workers
//! remain available while a command performs bounded network work.
use crate::next_client::Core;
use crate::secrets::CredentialStore;
use std::{future::Future,pin::Pin,sync::{Arc,Mutex,atomic::{AtomicBool,AtomicU64,Ordering}},time::Duration};
use serde_json::Value;
#[cfg(test)]
struct FixtureTiming{phase:&'static str,since:std::time::Instant,last_core_us:u64,last_bank_us:u64,subphase:&'static str,subsince:std::time::Instant,subphases:Vec<(&'static str,u64)>,first_core_us:u64,first_core_subphases:Vec<(&'static str,u64)>}
#[cfg(test)]
fn fixture_phase(timing:&Arc<Mutex<FixtureTiming>>,phase:&'static str){let mut t=timing.lock().unwrap();let elapsed=t.since.elapsed().as_micros() as u64;if t.phase=="core_tick"{if t.first_core_us==0{t.first_core_us=elapsed;t.first_core_subphases=t.subphases.clone();}t.last_core_us=elapsed}if t.phase=="bank_tick"{t.last_bank_us=elapsed}t.phase=phase;t.since=std::time::Instant::now();}
#[cfg(test)]tokio::task_local!{static CORE_TIMING:Arc<Mutex<FixtureTiming>>;}
#[cfg(test)]pub(crate) fn fixture_core_phase(phase:&'static str){let _=CORE_TIMING.try_with(|timing|{let mut t=timing.lock().unwrap();let previous=t.subphase;let elapsed=t.subsince.elapsed().as_micros()as u64;if !previous.is_empty(){if t.subphases.len()==48{t.subphases.remove(0);}t.subphases.push((previous,elapsed));}t.subphase=phase;t.subsince=std::time::Instant::now();});}
type Work<S>=Box<dyn for<'a> FnOnce(&'a mut Core<S>)->Pin<Box<dyn Future<Output=Result<Value,String>>+Send+'a>>+Send>;
struct Command<S:CredentialStore>{work:Work<S>,reply:tokio::sync::oneshot::Sender<Result<Value,String>>}
pub struct Handle<S:CredentialStore>{#[cfg(test)]timing:Arc<Mutex<FixtureTiming>>,tx:tokio::sync::mpsc::Sender<Command<S>>,view:Arc<Mutex<Value>>,generation:Arc<AtomicU64>,review_fence:Arc<AtomicU64>}
impl<S:CredentialStore> Clone for Handle<S>{fn clone(&self)->Self{Self{#[cfg(test)]timing:self.timing.clone(),tx:self.tx.clone(),view:self.view.clone(),generation:self.generation.clone(),review_fence:self.review_fence.clone()}}}
impl<S:CredentialStore+'static> Handle<S>{
 #[cfg(test)]pub(crate) fn fixture_timing(&self)->Value{let t=self.timing.lock().unwrap();serde_json::json!({"phase":t.phase,"phaseAgeUs":t.since.elapsed().as_micros() as u64,"lastCoreTickUs":t.last_core_us,"lastBankTickUs":t.last_bank_us,"queuedCommands":16-self.tx.capacity(),"coreSubphase":t.subphase,"coreSubphaseAgeUs":t.subsince.elapsed().as_micros()as u64,"coreSubphases":t.subphases,"firstCoreTickUs":t.first_core_us,"firstCoreSubphases":t.first_core_subphases})}
 pub fn view(&self)->Value{self.view.lock().unwrap().clone()}
 pub async fn action(&self,input:super::Action)->Result<Value,String>{if matches!(input.action,super::ActionKind::PreviewLobby|super::ActionKind::Join){self.review_fence.fetch_add(1,Ordering::AcqRel);}let revision=self.generation.clone();let generation=revision.load(Ordering::Acquire);self.command(move|core|Box::pin(async move{core.sync_background();if generation!=revision.load(Ordering::Acquire){return Err("Client action cancelled".into())}super::apply_action(core,input).await})).await}
 pub async fn command(&self,work:impl for<'a> FnOnce(&'a mut Core<S>)->Pin<Box<dyn Future<Output=Result<Value,String>>+Send+'a>>+Send+'static)->Result<Value,String>{
  let(tx,rx)=tokio::sync::oneshot::channel();self.tx.try_send(Command{work:Box::new(work),reply:tx}).map_err(|_|"App command queue full")?;rx.await.map_err(|_|"App scheduler stopped")?
 }
}
pub fn new<S:CredentialStore+'static>(core:Core<S>,stop:Arc<AtomicBool>)->(Handle<S>,impl Future<Output=()>+Send){
 new_boxed(Box::new(core),stop)
}
// Production setup keeps the large Core heap-owned across constructor/future moves.
// This bounds native setup frames on the unchanged 1MiB Windows main stack.
pub fn new_boxed<S:CredentialStore+'static>(mut core:Box<Core<S>>,stop:Arc<AtomicBool>)->(Handle<S>,impl Future<Output=()>+Send){
 let review_fence=core.lobby_review_fence();let(tx,mut rx)=tokio::sync::mpsc::channel::<Command<S>>(16);let view=Arc::new(Mutex::new(core.view()));let snapshot=view.clone();let generation=core.generation.clone();
 let auth=core.lc1_enabled().then(||core.auth_runner());let health=if core.lc1_enabled(){None}else{core.legacy_health_runner()};let auth_stop=stop.clone();let movement=core.movement_runner();let movement_stop=stop.clone();
 let presence=core.lc1_enabled().then(||core.presence_runner());let presence_stop=stop.clone();
 #[cfg(test)]let timing=Arc::new(Mutex::new(FixtureTiming{phase:"startup",since:std::time::Instant::now(),last_core_us:0,last_bank_us:0,subphase:"",subsince:std::time::Instant::now(),subphases:Vec::new(),first_core_us:0,first_core_subphases:Vec::new()}));
 #[cfg(test)]let actor_timing=timing.clone();
 let task=async move{
  let auth=tokio::spawn(async move{let mut auth=auth;let mut health=health;let mut clock=tokio::time::interval(Duration::from_secs(2));clock.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);while !auth_stop.load(Ordering::Acquire){clock.tick().await;if let Some(auth)=&mut auth{let _=auth.pump().await;}if let Some(health)=&mut health{let _=health.pump().await;}}});
  let movement=tokio::spawn(async move{let Some(mut movement)=movement else{return};let mut clock=tokio::time::interval(Duration::from_millis(20));clock.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);while !movement_stop.load(Ordering::Acquire){clock.tick().await;let _=movement.pump().await;}});
  let presence=tokio::spawn(async move{let Some(mut presence)=presence else{return};let mut clock=tokio::time::interval(Duration::from_secs(2));clock.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);while !presence_stop.load(Ordering::Acquire){clock.tick().await;if presence_stop.load(Ordering::Acquire){break}let _=presence.pump().await;}});
  let mut clock=tokio::time::interval(Duration::from_millis(200));clock.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
  while !stop.load(Ordering::Acquire){clock.tick().await;
   if let Ok(command)=rx.try_recv(){#[cfg(test)]fixture_phase(&actor_timing,"command");let result=(command.work)(&mut core).await;
    // Failed Join may have refreshed the review. Publish that state before
    // the error reply lets the renderer request status or click Join again.
    if result.is_err(){let mut current=core.view();current["g3"]=core.g3_view();current["social"]=core.social_view();*snapshot.lock().unwrap()=current;}
    let _=command.reply.send(result);}
   #[cfg(test)]fixture_phase(&actor_timing,"core_tick");
   let _=core.tick().await;
   #[cfg(test)]fixture_phase(&actor_timing,"bank_tick");
   core.bank_scheduler_tick().await;
   #[cfg(test)]fixture_phase(&actor_timing,"publish");
   core.sync_background();let mut current=core.view();current["g3"]=core.g3_view();current["social"]=core.social_view();core.observe_debug(&current);*snapshot.lock().unwrap()=current;#[cfg(test)]fixture_phase(&actor_timing,"idle");}
  stop.store(true,Ordering::Release);let _=auth.await;let _=movement.await;let _=presence.await;core.bank_shutdown().await;core.finish_debug();
 };
 #[cfg(test)]let task=CORE_TIMING.scope(timing.clone(),task);
 (Handle{#[cfg(test)]timing,tx,view,generation,review_fence},task)
}
