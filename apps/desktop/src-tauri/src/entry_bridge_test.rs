use super::*;
use nt2_desktop_g0::{secrets::{CredentialStore,ServiceProfile,SecretBlob,StoreError},startup_local::Scope,provisional_local as local};
#[derive(Default)]pub(crate) struct Store(Mutex<std::collections::BTreeMap<String,Vec<u8>>>);
impl CredentialStore for Store{
 fn save(&self,p:&ServiceProfile,b:&SecretBlob)->Result<(),StoreError>{self.0.lock().unwrap().insert(format!("{p:?}"),b.bytes().to_vec());Ok(())}
 fn load(&self,p:&ServiceProfile)->Result<Option<SecretBlob>,StoreError>{self.0.lock().unwrap().get(&format!("{p:?}")).map(|b|SecretBlob::new(b.clone())).transpose()}
 fn delete(&self,p:&ServiceProfile)->Result<(),StoreError>{self.0.lock().unwrap().remove(&format!("{p:?}"));Ok(())}
}
impl crate::secrets::CredentialStore for Store{
 fn save(&self,p:&crate::secrets::ServiceProfile,b:&crate::secrets::SecretBlob)->Result<(),crate::secrets::StoreError>{self.0.lock().unwrap().insert(format!("{p:?}"),b.bytes().to_vec());Ok(())}
 fn load(&self,p:&crate::secrets::ServiceProfile)->Result<Option<crate::secrets::SecretBlob>,crate::secrets::StoreError>{self.0.lock().unwrap().get(&format!("{p:?}")).map(|b|crate::secrets::SecretBlob::new(b.clone())).transpose()}
 fn delete(&self,p:&crate::secrets::ServiceProfile)->Result<(),crate::secrets::StoreError>{self.0.lock().unwrap().remove(&format!("{p:?}"));Ok(())}
}
pub(crate) fn preview()->local::Preview{
 let uuid=||uuid::Uuid::new_v4().to_string();let config=serde_json::json!({"mode":"core","seed":424242});let digest=nt2_desktop_g0::protocol_connection::canonical_fingerprint(&serde_json::to_vec(&config).unwrap()).unwrap();
 let scope=Scope{origin:"https://entry.invalid".into(),service:"service".into(),service_epoch:uuid(),account:uuid(),device:uuid::Uuid::new_v4(),desktop:uuid(),desktop_epoch:uuid(),room:uuid(),run:uuid(),run_epoch:uuid(),config:1,config_digest:digest.clone(),preparation:String::new()};
 let base=serde_json::json!({"serviceId":scope.service,"serviceEpoch":scope.service_epoch,"accountId":scope.account,"deviceId":scope.device.to_string(),"roomId":scope.room,"runId":scope.run,"runEpoch":scope.run_epoch,"configRevision":1,"configDigest":digest,"basePreparationId":uuid(),"baseFingerprint":"a".repeat(64),"seedPolicy":"shared-fixed","randomizeLoot":false,"actualSeed":"424242"});let started=Instant::now();
 local::Preview{scope,configuration:config,base,seed:424242,policy:1,until:started+Duration::from_secs(30),sample:nt2_desktop_g0::connection_time::Stamp{sequence:1,started}}
}
pub(crate) fn ready(worker:&Worker)->View{let until=Instant::now()+Duration::from_secs(3);loop{let(b,v,e)=worker.view();if !b{assert!(e.is_none(),"{e:?}");return v.unwrap()}assert!(Instant::now()<until);std::thread::yield_now()}}
pub(crate) fn staged(p:local::Preview)->(Arc<Worker>,View,Arc<Store>){let store=Arc::new(Store::default());let worker=Arc::new(Worker::new(store.clone(),Arc::new(Mutex::new(()))));worker.stage(p).unwrap();let v=ready(&worker);(worker,v,store)}
#[test]fn se1_r1_concurrent_production_rejoin_dispatch_and_promotion_collection_are_bounded(){
 let(worker,v,_)=staged(preview());let b=Arc::new(Bridge::new(worker));b.install_held(&v,nt2_local_ws::owner::Id::OsProcess(7),*v.context.as_bytes());b.pending.lock().unwrap().as_mut().unwrap().actual_seed=Some(424242);let contact=super::super::rejoin::Contact{pid:nt2_local_ws::owner::Id::OsProcess(7),context:*v.context.as_bytes(),association:*v.world.as_bytes(),actual:424242};let d=Arc::new(Mutex::new(super::super::NativeDiagnostics::default()));d.lock().unwrap().provisional=Some(b.clone());d.lock().unwrap().rejoin.admitted=Some(contact.clone());
 let mut q=request(12,contact.context);q[4..8].copy_from_slice(&7u32.to_le_bytes());q.extend(contact.association);q.extend(contact.actual.to_le_bytes());let (done,rx)=std::sync::mpsc::channel();let start=Arc::new(std::sync::Barrier::new(3));
 let (diag,bridge,ready,done1)=(d.clone(),b.clone(),start.clone(),done.clone());let traffic=std::thread::spawn(move||{ready.wait();for _ in 0..2000{assert_eq!(super::super::dispatch_rejoin(&diag,&bridge.pending,nt2_local_ws::owner::Id::OsProcess(7),&q)[8],1);}done1.send(()).unwrap();});
 let (diag,ready)=(d.clone(),start.clone());let promotion=std::thread::spawn(move||{ready.wait();for _ in 0..2000{let p=super::super::entry_collection(&diag,&contact).unwrap();assert_eq!(p.native_epoch,Some(1));}done.send(()).unwrap();});start.wait();
 // Timeout is bounded and leaves no wait/join on a deadlocked worker on failure.
 rx.recv_timeout(Duration::from_secs(3)).expect("production dispatcher stalled");rx.recv_timeout(Duration::from_secs(3)).expect("promotion collection stalled");traffic.join().unwrap();promotion.join().unwrap();
}
fn request(op:u32,context:[u8;16])->Vec<u8>{let mut q=vec![0;32];q[..4].copy_from_slice(&crate::handoff::MAGIC.to_le_bytes());q[4..8].copy_from_slice(&9u32.to_le_bytes());q[8..12].copy_from_slice(&op.to_le_bytes());q[16..32].copy_from_slice(&context);q}
fn field(reply:&[u8],at:usize)->u32{u32::from_le_bytes(reply[at..at+4].try_into().unwrap())}
fn claim(bridge:&Bridge,v:&View){assert_eq!(field(&bridge.handle(nt2_local_ws::owner::Id::OsProcess(7),100,&request(19,[7;16])),12),2);assert_eq!(field(&bridge.handle(nt2_local_ws::owner::Id::OsProcess(7),100,&request(16,*v.context.as_bytes())),12),2);ready(&bridge.worker);let reply=bridge.handle(nt2_local_ws::owner::Id::OsProcess(7),100,&request(18,*v.context.as_bytes()));assert_eq!(field(&reply,12),1);assert_eq!(field(&reply,176),1);assert_ne!(&reply[48..64],&reply[64..80]);}
#[test]fn se1_native_separate_slot_exact_claim_and_old_close_never_revoke_other_world(){
 let(worker,v,_)=staged(preview());let bridge=Bridge::new(worker);let a=Arc::new(Mutex::new(Some(crate::handoff::new(99,"saved-A".into(),1))));a.lock().unwrap().as_mut().unwrap().owner=Some((nt2_local_ws::owner::Id::OsProcess(7),[9;16]));claim(&bridge,&v);
 let held=bridge.pending.lock().unwrap().clone().unwrap();assert!(!held.release);assert_eq!(held.owner,Some((crate::local_owner::Id::os_process(7),*v.context.as_bytes())));assert_eq!(a.lock().unwrap().as_ref().unwrap().owner,Some((crate::local_owner::Id::os_process(7),[9;16])));
 let(other,w,_)=staged(preview());let replacement=Bridge::new(other);claim(&replacement,&w);
 let mut close=request(17,*v.context.as_bytes());close.extend(v.world.as_bytes());close.extend(unhex(&v.digest));bridge.handle(nt2_local_ws::owner::Id::OsProcess(7),100,&close);assert!(bridge.pending.lock().unwrap().is_none());assert!(replacement.pending.lock().unwrap().is_some());assert!(a.lock().unwrap().is_some());
 assert!(!replacement.explicit_offline(nt2_local_ws::owner::Id::OsProcess(7),101,*w.context.as_bytes()));assert!(replacement.explicit_offline(nt2_local_ws::owner::Id::OsProcess(7),100,*w.context.as_bytes()));assert!(replacement.pending.lock().unwrap().is_none());assert_eq!(ready(&replacement.worker).phase,Phase::Detached);
}
#[test]fn se1_native_original_offer_budget_and_nonce_cannot_rearm_after_timeout(){
 let(worker,v,_)=staged(preview());let bridge=Bridge::new(worker);assert_eq!(field(&bridge.handle(nt2_local_ws::owner::Id::OsProcess(7),100,&request(19,[7;16])),12),2);bridge.attempt.lock().unwrap().as_mut().unwrap().began-=Duration::from_secs(7);
 assert_eq!(field(&bridge.handle(nt2_local_ws::owner::Id::OsProcess(7),100,&request(16,*v.context.as_bytes())),12),0);assert_eq!(field(&bridge.handle(nt2_local_ws::owner::Id::OsProcess(7),100,&request(19,[8;16])),12),0);assert_eq!(ready(&bridge.worker).phase,Phase::Staged);assert!(bridge.pending.lock().unwrap().is_none());
}
#[test]fn se1_native_offer_during_refresh_keeps_original_and_queues_claim_without_timer_race(){
 let store=Arc::new(Store::default());let writer=Arc::new(Mutex::new(()));let worker=Arc::new(Worker::new(store,writer.clone()));let p=preview();worker.stage(p.clone()).unwrap();let v=ready(&worker);let bridge=Bridge::new(worker.clone());let held=writer.lock().unwrap();let mut fresh=p;fresh.sample.sequence=2;fresh.sample.started=Instant::now();fresh.until=fresh.sample.started+Duration::from_secs(30);fresh.base["baseFingerprint"]=serde_json::json!("b".repeat(64));worker.stage(fresh).unwrap();
 assert_eq!(field(&bridge.handle(nt2_local_ws::owner::Id::OsProcess(7),100,&request(19,[7;16])),12),2);assert_eq!(field(&bridge.handle(nt2_local_ws::owner::Id::OsProcess(7),100,&request(16,*v.context.as_bytes())),12),2);assert_eq!(field(&bridge.handle(nt2_local_ws::owner::Id::OsProcess(7),100,&request(18,*v.context.as_bytes())),12),2);drop(held);assert_eq!(ready(&worker).world,v.world);
 assert_eq!(field(&bridge.handle(nt2_local_ws::owner::Id::OsProcess(7),100,&request(18,*v.context.as_bytes())),12),2);ready(&worker);let reply=bridge.handle(nt2_local_ws::owner::Id::OsProcess(7),100,&request(18,*v.context.as_bytes()));assert_eq!(field(&reply,12),1);assert_eq!(&reply[64..80],v.world.as_bytes());
}
#[test]fn se1_native_epoch_survives_common_promotion_but_never_bypasses_lifecycle_fence(){
 let(worker,v,_)=staged(preview());let bridge=Bridge::new(worker);claim(&bridge,&v);let mut p=bridge.pending.lock().unwrap().clone().unwrap();let generation=Arc::new(std::sync::atomic::AtomicU64::new(77));p.generation=Some((generation.clone(),77));p.loaded=true;p.accepted=true;p.actual_seed=Some(424242);let mut pending=Some(p);let mut heartbeat=request(5,*v.context.as_bytes());heartbeat[4..8].copy_from_slice(&4u32.to_le_bytes());let reply=super::super::c2_transition(&mut pending,nt2_local_ws::owner::Id::OsProcess(7),&heartbeat);assert_eq!(field(&reply,36),1);generation.store(78,Ordering::Release);let refused=super::super::c2_transition(&mut pending,nt2_local_ws::owner::Id::OsProcess(7),&heartbeat);assert_eq!(field(&refused,36),0);assert_eq!(field(&refused,8),0);
}
