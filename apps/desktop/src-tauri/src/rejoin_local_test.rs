use super::*;
use std::{sync::Mutex,collections::BTreeMap};
#[derive(Default)]struct Memory(Mutex<BTreeMap<String,Vec<u8>>>);
fn key(p:&ServiceProfile)->String{format!("{}|{}|{}",p.origin(),p.service_id(),p.profile_id())}
impl CredentialStore for Memory{
 fn save(&self,p:&ServiceProfile,b:&SecretBlob)->Result<(),StoreError>{self.0.lock().unwrap().insert(key(p),b.bytes().to_vec());Ok(())}
 fn load(&self,p:&ServiceProfile)->Result<Option<SecretBlob>,StoreError>{self.0.lock().unwrap().get(&key(p)).map(|b|SecretBlob::new(b.clone())).transpose()}
 fn delete(&self,p:&ServiceProfile)->Result<(),StoreError>{self.0.lock().unwrap().remove(&key(p));Ok(())}
}
fn association()->Association{Association{version:1,local_world:uuid::Uuid::new_v4(),origin:"https://192.168.0.104:4358".into(),service:"nt2-g3-c2-potion-correction".into(),run:uuid::Uuid::new_v4().to_string(),participant:uuid::Uuid::new_v4().to_string(),room:uuid::Uuid::new_v4().to_string(),world_token:"1234567890abcdef1234567890abcdef".into(),account:uuid::Uuid::new_v4().to_string(),original_start:uuid::Uuid::new_v4().to_string(),config_revision:1,actual_seed:"424242".into(),seed_policy:"shared-random".into(),loot_policy:"shared:1:424242".into(),frozen_config_digest:"a".repeat(64)}}
#[test]fn rejoin_metadata_bound_roundtrip_and_foreign_refusal(){
 let a=association();assert_eq!(Association::decode(&a.encode().unwrap()).unwrap(),a);
 for mutation in 0..5{let mut bad=a.clone();match mutation{0=>bad.actual_seed="0424242".into(),1=>bad.loot_policy="shared:1:42".into(),2=>bad.world_token="0".repeat(32),3=>bad.version=9,_=>bad.seed_policy="individual-random".into()};assert!(bad.encode().is_err())}
 assert!(Association::decode(b"{}").is_err());assert!(Association::decode(&vec![0;1025]).is_err());
 let mut individual=a;individual.seed_policy="individual-random".into();individual.actual_seed=u64::MAX.to_string();individual.loot_policy="individual:0".into();individual.validate().unwrap();
}
#[test]fn rejoin_protected_store_secret_redaction_and_foreign_scope(){
 let store=Memory::default();let a=association();let proof=SecretBlob::new(vec![b'a';43]).unwrap();remember(&store,&a,&proof).unwrap();
 assert_eq!(recall(&store,&a).unwrap().unwrap().bytes(),proof.bytes());assert!(!format!("{proof:?}").contains(&"a".repeat(43)));
 let mut foreign=a.clone();foreign.local_world=uuid::Uuid::new_v4();assert!(recall(&store,&foreign).is_err());
 retire_local(&store,&a).unwrap();assert!(recall(&store,&a).unwrap().is_none());assert_eq!(retired_proof(&store,&a).unwrap().unwrap().bytes(),proof.bytes());
 forget(&store,&a).unwrap();assert!(recall(&store,&a).unwrap().is_none());
}
#[test]fn rejoin_terminal_holds_and_keeps_explicit_offline_action(){
 let a=association();let mut intent=ResumeIntent::default();intent.track_selected_playing(a).unwrap();intent.transient_loss(Instant::now());
 let generation=intent.generation();intent.terminal();let view=intent.view();
 assert_eq!(view["state"],"terminal");assert_eq!(view["held"],true);assert_eq!(view["can_offline"],true);assert_eq!(view["can_rejoin"],false);assert!(view["terminal_reason"].as_str().is_some_and(|s|!s.is_empty()));
 assert!(!intent.retry_due(Instant::now()+Duration::from_secs(10000)));assert!(intent.admitted(generation).is_err());
 intent.offline();assert_eq!(intent.view()["can_offline"],false);assert_eq!(intent.phase(),Phase::Offline);
}
#[test]fn rejoin_late_selected_capture_cannot_mutate_terminal_store(){
 let store=Memory::default();let a=association();let proof=SecretBlob::new(vec![b'a';43]).unwrap();let mut intent=ResumeIntent::default();
 remember_selected(&store,&mut intent,a.clone(),&proof).unwrap();intent.terminal();let before=store.0.lock().unwrap().clone();
 assert!(remember_selected(&store,&mut intent,a.clone(),&proof).is_err());assert_eq!(*store.0.lock().unwrap(),before);
 let mut late=a.clone();late.local_world=uuid::Uuid::new_v4();assert!(remember_selected(&store,&mut intent,late,&proof).is_err());assert_eq!(*store.0.lock().unwrap(),before);
}
#[test]fn rejoin_retirement_survives_reload_uuid_change_and_acknowledged_proof_deletion(){
 let store=Memory::default();let a=association();let proof=SecretBlob::new(vec![b'a';43]).unwrap();remember(&store,&a,&proof).unwrap();retire_local(&store,&a).unwrap();
 let before=store.0.lock().unwrap().clone();let mut late=a.clone();late.local_world=uuid::Uuid::new_v4();
 for record in [a.clone(),late.clone()]{let mut restarted=ResumeIntent::default();assert!(remember_selected(&store,&mut restarted,record.clone(),&proof).is_err());assert!(remember(&store,&record,&proof).is_err());assert!(recall(&store,&record).unwrap().is_none());assert_eq!(*store.0.lock().unwrap(),before)}
 assert_eq!(retired_proof(&store,&a).unwrap().unwrap().bytes(),proof.bytes());forget(&store,&a).unwrap();let acknowledged=store.0.lock().unwrap().clone();
 assert!(retired_proof(&store,&a).unwrap().is_none());assert!(remember(&store,&late,&proof).is_err());assert_eq!(*store.0.lock().unwrap(),acknowledged);
 let mut fresh=a;fresh.world_token="b".repeat(32);fresh.local_world=uuid::Uuid::new_v4();let mut restarted=ResumeIntent::default();remember_selected(&store,&mut restarted,fresh.clone(),&proof).unwrap();assert_eq!(restarted.phase(),Phase::Playing);assert!(recall(&store,&fresh).unwrap().is_some());
}
#[test]fn rejoin_racing_late_capture_cannot_clear_guard_or_replace_offline_proof(){
 use std::sync::atomic::{AtomicBool,Ordering};
 struct Race{inner:Memory,association:Association,retire_on_active_save:AtomicBool}
 impl CredentialStore for Race{
  fn save(&self,p:&ServiceProfile,b:&SecretBlob)->Result<(),StoreError>{
   if p==&self.association.store_profile()?&&self.retire_on_active_save.swap(false,Ordering::SeqCst){retire_local(self,&self.association)?;}
   self.inner.save(p,b)
  }
  fn load(&self,p:&ServiceProfile)->Result<Option<SecretBlob>,StoreError>{self.inner.load(p)}
  fn delete(&self,p:&ServiceProfile)->Result<(),StoreError>{self.inner.delete(p)}
 }
 let a=association();let store=Race{inner:Memory::default(),association:a.clone(),retire_on_active_save:AtomicBool::new(false)};
 let original=SecretBlob::new(vec![b'a';43]).unwrap();remember(&store,&a,&original).unwrap();store.retire_on_active_save.store(true,Ordering::SeqCst);
 let late=SecretBlob::new(vec![b'b';43]).unwrap();assert!(remember(&store,&a,&late).is_err());assert!(recall(&store,&a).unwrap().is_none());
 assert_eq!(retired_proof(&store,&a).unwrap().unwrap().bytes(),original.bytes());assert_eq!(retirement_state(&store,&a).unwrap(),Some(1));
 forget(&store,&a).unwrap();assert_eq!(retirement_state(&store,&a).unwrap(),Some(2));assert!(retired_proof(&store,&a).unwrap().is_none());
}
#[test]fn rejoin_intent_survives_long_outage_original_operation_and_terminal_fences(){
 let a=association();let mut s=ResumeIntent::default();s.offer(a.clone(),true).unwrap();let g=s.choose_rejoin().unwrap();s.world_loaded(&a).unwrap();s.bind_operation(g,b"immutable canonical resume".to_vec()).unwrap();
 let start=Instant::now();for minutes in [1,5,20]{s.transient_loss(start+Duration::from_secs(minutes*60));assert!(s.retry_due(start+Duration::from_secs(minutes*60+17)));assert_eq!(s.original_operation(),Some(&b"immutable canonical resume"[..]));}
 assert!(s.bind_operation(g,b"replacement".to_vec()).is_err());s.admitted(g).unwrap();assert!(s.release(g,false).is_err());s.release(g,true).unwrap();
 s.transient_loss(Instant::now());assert_eq!(s.phase(),Phase::Reconnecting);s.offline();assert!(!s.retry_due(start+Duration::from_secs(9999)));assert!(s.admitted(g).is_err());assert!(s.offer(a,true).is_err());
}
#[test]fn rejoin_app_first_continue_first_and_one_choice(){
 let a=association();let mut s=ResumeIntent::default();assert!(s.offer(a.clone(),false).is_err());s.offer(a.clone(),true).unwrap();assert!(s.world_loaded(&a).is_err());
 let g=s.choose_rejoin().unwrap();s.offer(a.clone(),true).unwrap();assert_eq!(s.generation(),g);s.world_loaded(&a).unwrap();let mut foreign=a;foreign.participant=uuid::Uuid::new_v4().to_string();assert!(s.world_loaded(&foreign).is_err());
}
#[test]fn rejoin_canonical_request_and_fresh_admission_fences(){
 use crate::protocol_rejoin::wire as w;let a=association();let proof=SecretBlob::new(vec![b'a';43]).unwrap();let bytes=resume_request(&a,&proof,"service-new","desktop-new","operation-1",uuid::Uuid::new_v4()).unwrap();let req:w::Request=serde_json::from_slice(&bytes).unwrap();assert_eq!(req.run_id,Some(a.run.to_string()));assert_eq!(req.world_token,Some(a.world_token.clone()));assert!(req.resume.is_some());assert!(req.status.is_none());
 let canonical=w::Association{service_id:Some(a.service.clone()),room_id:Some(a.room.to_string()),run_id:Some(a.run.to_string()),participant_id:Some(a.participant.to_string()),world_token:Some(a.world_token.clone()),account_id:Some(a.account.to_string()),original_start_id:Some(a.original_start.to_string()),config_revision:Some(1),actual_seed:Some(a.actual_seed.clone()),seed_policy:Some(a.seed_policy.clone()),randomize_loot:Some(true)};
 let mut r=w::Response{major:Some(1),revision:Some("g3-rejoin-1".into()),service_id:Some(a.service.clone()),service_epoch:Some("service-new".into()),request_id:Some("operation-1".into()),participant:Some(w::Participant{association:Some(canonical),generation:Some(2),state:Some("inspecting".into()),return_eligible:Some(true),service_epoch:Some("service-new".into()),run_epoch:Some("run-new".into()),desktop_id:Some("desktop-new".into()),attachment_id:Some("attachment-new".into()),game_session_id:Some("game-new".into()),..Default::default()}),credentials:Some(w::Credentials{account_id:Some(a.account.to_string()),attachment_id:Some("attachment-new".into()),device_id:Some("device-original".into()),session_epoch:Some("game-epoch-new".into()),session_id:Some("game-new".into()),access_token:Some("never-in-ui".into()),..Default::default()}),..Default::default()};
 assert_eq!(validate_admission(&a,&r,"service-new","operation-1","desktop-new","device-original",1).unwrap(),2);assert!(validate_admission(&a,&r,"old-epoch","operation-1","desktop-new","device-original",1).is_err());assert!(validate_admission(&a,&r,"service-new","other-op","desktop-new","device-original",1).is_err());assert!(validate_admission(&a,&r,"service-new","operation-1","desktop-new","device-original",2).is_err());r.credentials.as_mut().unwrap().attachment_id=Some("old-copy".into());assert!(validate_admission(&a,&r,"service-new","operation-1","desktop-new","device-original",1).is_err());
}
