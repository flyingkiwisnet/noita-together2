use super::*;
use super::continue_cf05::{prior,remember,StoreBridge};
use nt2_desktop_g0::{rejoin_local as local};
#[test]
fn cr1_offline_original_never_crosses_origin_after_logout(){tauri::async_runtime::block_on(async{
 let(mut c,_,_)=client();let a0=prior(&c);
 let association=json!({"serviceId":a0.service,"roomId":a0.room,"runId":a0.run,"participantId":a0.participant,"worldToken":a0.world_token,"accountId":a0.account,"originalStartId":a0.original_start,"configRevision":a0.config_revision,"actualSeed":a0.actual_seed,"seedPolicy":a0.seed_policy,"randomizeLoot":false});
 let(origin,old)=network(vec![Reply::Data(json!({"rooms":[]})),Reply::Data(json!({"participant":{"association":association,"generation":1,"state":"active","returnEligible":true}})),Reply::Lost,Reply::Data(json!({}))]);
 set_origin(&mut c,&origin);let mut a=a0;a.origin=origin;remember(&c,&a);c.next_work=Instant::now();c.tick().await.unwrap();c.continue_offline().unwrap();c.next_work=Instant::now();assert!(c.tick().await.is_err());
 let original=c.rejoin_transport.offline.as_ref().unwrap().to_vec();let desktop=c.desktop.as_ref().unwrap();let credentials=json!({"kind":"desktop","deviceId":c.device.to_string(),"principal":desktop.principal,"accountId":desktop.account,"sessionId":desktop.id,"sessionEpoch":desktop.epoch,"accessToken":"new-origin-only","refreshSecret":"B".repeat(43),"expiresAt":later(900000)});
 c.logout().await.unwrap();assert_eq!(old.join().unwrap().len(),4);
 let baseline=std::env::var("NT2_CR1_BASELINE").as_deref()==Ok("1");let mut replies=vec![Reply::Data(json!({"credentials":credentials}))];
 if baseline{replies.push(Reply::Data(json!({"participant":{"association":association,"generation":1,"state":"offline","returnEligible":false}})));}replies.push(Reply::Data(json!({"rooms":[]})));
 let(new_origin,new)=selectable_network(replies,c.service.clone(),c.epoch.clone());c.social_requested=false;c.select(&new_origin,"",false).await.unwrap();c.guest("Alex").await.unwrap();inject_credential_timing(&mut c);c.next_work=Instant::now();c.tick().await.unwrap();let calls=new.join().unwrap();
 let leaked=calls.iter().any(|v|v["offline"]["resumeProof"].is_string());
 if baseline{assert!(leaked);}else{assert!(!leaked);assert!(c.resume_intent.association().is_none());assert_eq!(c.parked_recovery.len(),1);assert_eq!(c.parked_recovery[0].transport.offline.as_ref().unwrap().as_slice(),original.as_slice());assert!(local::retired_proof(&StoreBridge(&c.store),&a).unwrap().is_some());assert!(c.fresh_join_recovery_blocker().unwrap().is_none());}
 if let Some(path)=std::env::var_os("NT2_CR1_EVIDENCE"){std::fs::write(path,serde_json::to_vec_pretty(&json!({"baseline":baseline,"oldOriginCalls":4,"newOriginCalls":calls.len(),"actualSelectAction":true,"proofSentToNewOrigin":leaked,"engineFactsInjected":true})).unwrap()).unwrap();}clean(c);
});}

#[test]fn cr1_consumed_grant_is_never_presented_as_rearmed(){tauri::async_runtime::block_on(async{
 let(mut c,room,prep)=client();let(origin,worker)=network(vec![Reply::Data(data(&room,&prep))]);set_origin(&mut c,&origin);join(&mut c).await.unwrap();
 assert_eq!(handoff::transition(&mut c.pending.lock().unwrap(),nt2_local_ws::owner::Id::OsProcess(7),&frame(1,0))[8],1);let claim=c.pending.lock().unwrap().as_ref().unwrap().claim_path.clone().unwrap();assert!(claim.exists());
 handoff::transition(&mut c.pending.lock().unwrap(),nt2_local_ws::owner::Id::OsProcess(7),&frame(6,0));assert!(c.pending.lock().unwrap().is_none());assert!(c.arm().unwrap_err().contains("already locally consumed"));assert!(c.pending.lock().unwrap().is_none());assert!(claim.exists());worker.join().unwrap();clean(c);
});}

fn selectable_network(replies:Vec<Reply>,service:String,epoch:String)->(String,std::thread::JoinHandle<Vec<Value>>){
 let listener=std::net::TcpListener::bind("127.0.0.1:0").unwrap();let origin=format!("http://{}",listener.local_addr().unwrap());
 let worker=std::thread::spawn(move||{let mut calls=vec![];let mut replies=replies.into_iter();let mut discovery=true;listener.set_nonblocking(true).unwrap();
  loop{let next=if discovery{None}else{match replies.next(){Some(v)=>Some(v),None=>break}};let until=Instant::now()+Duration::from_secs(10);let mut socket=loop{match listener.accept(){Ok((s,_))=>break s,Err(e) if e.kind()==std::io::ErrorKind::WouldBlock=>{assert!(Instant::now()<until);std::thread::sleep(Duration::from_millis(5));},Err(e)=>panic!("{e}")}};socket.set_nonblocking(false).unwrap();socket.set_read_timeout(Some(Duration::from_secs(3))).unwrap();let mut head=vec![];let mut byte=[0];while !head.ends_with(b"\r\n\r\n"){socket.read_exact(&mut byte).unwrap();head.push(byte[0]);assert!(head.len()<8192);}let header=String::from_utf8(head).unwrap();
   let mut response=if discovery{assert!(header.starts_with("GET /v1/info "));discovery=false;json!({"major":1,"revision":protocol::REVISION,"serviceId":service,"serviceEpoch":epoch,"connectionFlowRevision":"g3-flow-1"})}else{
    let size=header.lines().find_map(|l|l.to_lowercase().strip_prefix("content-length:").and_then(|v|v.trim().parse::<usize>().ok())).unwrap();let mut body=vec![0;size];socket.read_exact(&mut body).unwrap();let request:Value=serde_json::from_slice(&body).unwrap();calls.push(request.clone());let mut response=match next.unwrap(){Reply::Data(v)=>v,Reply::Lost=>continue,_=>panic!()};for key in ["major","revision","serviceId","serviceEpoch","requestId"]{response[key]=request[key].clone();}response
   };let body=serde_json::to_vec(&mut response).unwrap();write!(socket,"HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",body.len()).unwrap();socket.write_all(&body).unwrap();
  }calls
 });(origin,worker)
}
#[test]fn cr1_other_account_parks_original_and_returning_account_reconciles(){tauri::async_runtime::block_on(async{
 let(mut c,_,_)=client();let a0=prior(&c);let association=json!({"serviceId":a0.service,"roomId":a0.room,"runId":a0.run,"participantId":a0.participant,"worldToken":a0.world_token,"accountId":a0.account,"originalStartId":a0.original_start,"configRevision":a0.config_revision,"actualSeed":a0.actual_seed,"seedPolicy":a0.seed_policy,"randomizeLoot":false});
 let(origin,worker)=network(vec![Reply::Data(json!({"rooms":[]})),Reply::Data(json!({"participant":{"association":association,"generation":1,"state":"active","returnEligible":true}})),Reply::Lost,Reply::OutcomeTerminal,Reply::Data(json!({"rooms":[]}))]);set_origin(&mut c,&origin);let mut a=a0;a.origin=origin;remember(&c,&a);c.next_work=Instant::now();c.tick().await.unwrap();c.continue_offline().unwrap();c.next_work=Instant::now();assert!(c.tick().await.is_err());let original=c.rejoin_transport.offline.as_ref().unwrap().to_vec();
 let old=c.desktop.clone().unwrap();let mut other=old.clone();other.account=uuid::Uuid::new_v4().to_string();other.id=uuid::Uuid::new_v4().to_string();other.epoch=uuid::Uuid::new_v4().to_string();c.desktop=Some(other); // Inject verified session replacement at install boundary.
 c.park_foreign_recovery().unwrap();assert!(c.fresh_join_recovery_blocker().unwrap().is_none());assert_eq!(c.parked_recovery[0].transport.offline.as_ref().unwrap().as_slice(),original.as_slice());assert!(local::retired_proof(&StoreBridge(&c.store),&a).unwrap().is_some());
 c.desktop=Some(old);c.desktop.as_mut().unwrap().epoch=uuid::Uuid::new_v4().to_string();inject_credential_timing(&mut c);c.park_foreign_recovery().unwrap();c.next_work=Instant::now();c.tick().await.unwrap();let calls=worker.join().unwrap();assert_eq!(calls.len(),5);assert!(calls[3]["outcome"].is_object()&&calls[3]["offline"].is_null(),"new auth must query exact original outcome, never resend old envelope");assert!(c.rejoin_transport.offline.is_none());assert!(local::retired_proof(&StoreBridge(&c.store),&a).unwrap().is_none());clean(c);
});}

#[test]fn cr1_recovered_native_owner_keeps_stable_world_projection(){
 let(mut c,_,_)=release_case();c.base="https://fixture.invalid".into();let mut a=prior(&c);{let mut lock=c.pending.lock().unwrap();let p=lock.as_mut().unwrap();p.actual_seed=Some(424242);a.run=p.run.clone();a.config_revision=p.config as u32;a.world_token=hex(&p.owner.unwrap().1);}
 a.room=c.room.as_ref().unwrap().room_id.clone().unwrap();remember(&c,&a);c.offer_saved_world(&a.encode().unwrap()).unwrap();c.rejoin_previous().unwrap();let old=c.game.as_ref().unwrap().owner;let new=(old.0,[9;16]);c.game.as_mut().unwrap().owner=new;
 {let mut lock=c.pending.lock().unwrap();let p=lock.as_mut().unwrap();p.owner=Some(new);p.observation.as_mut().unwrap().context=new.1;}
 c.rejoin_transport.generation=2;c.rejoin_transport.contact=Some(crate::pipe::rejoin::Contact{pid:new.0,context:new.1,association:*a.local_world.as_bytes(),actual:424242});assert_eq!(c.view()["rejoin"]["current_world"],true);assert_eq!(c.view()["rejoin"]["held"],true);assert!(c.lifecycle_owner_ticket().is_some());
 c.rejoin_transport.contact.as_mut().unwrap().association=[8;16];assert_eq!(c.view()["rejoin"]["current_world"],false);assert!(c.lifecycle_owner_ticket().is_none());
}

#[test]fn cr1_initial_selected_capture_remains_proof_free_and_current_scoped(){tauri::async_runtime::block_on(async{
 let(mut c,_,_)=release_case();c.game.as_mut().unwrap().entered=true;{let mut slot=c.pending.lock().unwrap();let p=slot.as_mut().unwrap();p.actual_seed=Some(u64::from(p.seed));}let p=c.pending.lock().unwrap().clone().unwrap();let a=prior(&c);
 let association=json!({"serviceId":c.service,"roomId":c.room.as_ref().unwrap().room_id,"runId":p.run,"participantId":a.participant,"worldToken":hex(&p.owner.unwrap().1),"accountId":c.desktop.as_ref().unwrap().account,"originalStartId":a.original_start,"configRevision":p.config,"actualSeed":p.actual_seed.unwrap().to_string(),"seedPolicy":"shared-fixed","randomizeLoot":false});
 let(origin,worker)=network(vec![Reply::Data(json!({"participant":{"association":association,"generation":1,"state":"active","returnEligible":true},"resumeProof":"A".repeat(43)}))]);set_origin(&mut c,&origin);c.capture_selected_association().await.unwrap();let calls=worker.join().unwrap();assert_eq!(calls.len(),1);assert!(calls[0]["status"].is_object());assert!(calls[0]["resume"].is_null()&&calls[0]["offline"].is_null()&&calls[0]["worldToken"].is_null());assert!(c.resume_intent.association().is_some());
});}
