use super::*;
fn client_room()->(Core<MemoryStore>,wire::Room){
 let(mut c,_,_)=release_case();let mut room=c.room.take().unwrap();c.clear_game();c.room=None;c.preparation=None;c.enable_lc1();c.enable_automatic_setup();c.setup.preview=None;let desktop=c.desktop.as_mut().unwrap();desktop.id=uuid::Uuid::new_v4().to_string();desktop.epoch=uuid::Uuid::new_v4().to_string();
 room.room_id=Some(uuid::Uuid::new_v4().to_string());room.run_id=Some(uuid::Uuid::new_v4().to_string());room.run_epoch=Some(uuid::Uuid::new_v4().to_string());room.revision=Some(3);room.state=Some("staging".into());
 (c,room)
}
fn preview(room:&wire::Room,revision:u32,eligible:bool)->Value{let mut room=room.clone();room.revision=Some(revision);json!({"admission":{"roomId":room.room_id,"revision":revision,"configRevision":room.config_revision,"joinAllowed":eligible},"room":room})}
fn joined(room:&wire::Room,revision:u32)->Value{let mut room=room.clone();room.revision=Some(revision);json!({"room":room})}
async fn reviewed(c:&mut Core<MemoryStore>,room:&wire::Room){c.preview_lobby(room.room_id.as_deref().unwrap()).await.unwrap();}
async fn join(c:&mut Core<MemoryStore>)->Result<(),String>{let p=c.setup.preview.as_ref().unwrap().clone();c.join_displayed(&p.room,Some("fixture-password".into()),p.revision,p.config_revision).await}
fn direct(c:&mut Core<MemoryStore>){c.info=Some(wire::Info{connection_flow_revision:Some("g3-flow-1".into()),..Default::default()});}
#[test]fn ux1_direct_join_action_has_no_prior_review_or_renderer_revision_requirement(){tauri::async_runtime::block_on(async{
 let(mut c,room)=client_room();direct(&mut c);let(origin,server)=network(vec![Reply::Data(preview(&room,3,true)),Reply::Data(joined(&room,4))]);set_origin(&mut c,&origin);assert!(c.setup.preview.is_none());let action=serde_json::from_value(json!({"action":"join","room":room.room_id,"password":"fixture-password"})).unwrap();crate::next_app::apply_action(&mut c,action).await.unwrap();assert!(c.world.wants_prepare);assert!(c.pending.lock().unwrap().is_none());assert!(c.world.bridge.as_ref().unwrap().pending.lock().unwrap().is_none());let calls=server.join().unwrap();assert_eq!(calls.len(),2);assert!(calls[0]["roomPreview"].is_object());assert_eq!(calls[1]["expectedRevision"],3);assert_eq!(calls[1]["roomJoin"]["password"],"fixture-password");
});}
#[test]fn ux1_direct_join_follows_latest_config_and_run_after_definite_stale_once(){tauri::async_runtime::block_on(async{
 let(mut c,room)=client_room();direct(&mut c);let mut fresh=room.clone();fresh.config_revision=Some(room.config_revision.unwrap()+1);fresh.config.as_mut().unwrap().seed=Some(9988);fresh.run_id=Some(uuid::Uuid::new_v4().to_string());fresh.run_epoch=Some(uuid::Uuid::new_v4().to_string());let(origin,server)=network(vec![Reply::Data(preview(&room,3,true)),Reply::Error("STALE_REVISION"),Reply::Data(preview(&fresh,4,true)),Reply::Data(joined(&fresh,5))]);set_origin(&mut c,&origin);c.join_selected(room.room_id.as_deref().unwrap(),Some("fixture-password".into())).await.unwrap();assert_eq!(c.room.as_ref().unwrap().config.as_ref().unwrap().seed,Some(9988));assert!(c.world.wants_prepare);let calls=server.join().unwrap();assert_eq!(calls.len(),4);assert_eq!(calls[3]["expectedRevision"],4);assert_eq!(calls[3]["configRevision"],fresh.config_revision.unwrap());assert_ne!(calls[1]["requestId"],calls[3]["requestId"]);
});}
#[test]fn ux1_direct_join_is_bounded_and_does_not_replace_an_unknown_join(){tauri::async_runtime::block_on(async{
 for lost in [true,false]{let(mut c,room)=client_room();direct(&mut c);let replies=if lost{vec![Reply::Data(preview(&room,3,true)),Reply::Lost]}else{vec![Reply::Data(preview(&room,3,true)),Reply::Error("STALE_REVISION"),Reply::Data(preview(&room,4,true)),Reply::Error("STALE_REVISION"),Reply::Data(preview(&room,5,true))]};let(origin,server)=network(replies);set_origin(&mut c,&origin);let error=c.join_selected(room.room_id.as_deref().unwrap(),None).await.unwrap_err();if lost{assert!(error.contains("could not be confirmed"));assert!(c.join_selected(room.room_id.as_deref().unwrap(),None).await.unwrap_err().contains("unconfirmed"));}else{assert!(error.contains("Try Join again"));}assert!(!c.world.wants_prepare);let calls=server.join().unwrap();assert_eq!(calls.iter().filter(|q|q["roomJoin"].is_object()).count(),if lost{1}else{2});}
});}
#[test]fn ux1_direct_join_late_selection_or_session_cannot_enroll(){tauri::async_runtime::block_on(async{
 for selection in [true,false]{let(mut c,room)=client_room();direct(&mut c);let fence=if selection{c.lobby_review_fence()}else{c.generation.clone()};let(origin,server)=network(vec![Reply::Fence(preview(&room,3,true),fence)]);set_origin(&mut c,&origin);assert!(c.join_selected(room.room_id.as_deref().unwrap(),None).await.is_err());assert!(!c.world.wants_prepare);assert_eq!(server.join().unwrap().len(),1);}
});}
#[test]fn ux1_direct_join_preserves_saved_world_and_continue_choice(){tauri::async_runtime::block_on(async{
 let(mut c,room)=client_room();direct(&mut c);let(origin,server)=network(vec![Reply::Data(preview(&room,3,true)),Reply::Data(joined(&room,4))]);set_origin(&mut c,&origin);let identity=nt2_desktop_g0::world_local::Identity{origin:c.base.clone(),service:c.service.clone(),account:c.desktop.as_ref().unwrap().account.clone(),device:c.device,room:room.room_id.clone().unwrap(),run:room.run_id.clone().unwrap(),run_epoch:room.run_epoch.clone().unwrap(),config:u64::from(room.config_revision.unwrap()),config_digest:"a".repeat(64)};let mut record=nt2_desktop_g0::world_local::Record::prepared(identity,uuid::Uuid::new_v4(),"b".repeat(64),Some("424242".into()),1,serde_json::to_value(room.config.as_ref().unwrap()).unwrap()).unwrap();record.consumed=true;let before=serde_json::to_value(&record).unwrap();c.world.record=Some(record);c.world.coordinator.intent(nt2_desktop_g0::world_lifecycle::Stage::Choice);c.join_selected(room.room_id.as_deref().unwrap(),None).await.unwrap();assert_eq!(serde_json::to_value(&c.world.record).unwrap(),before);assert_eq!(c.world.coordinator.stage(),nt2_desktop_g0::world_lifecycle::Stage::Choice);assert!(c.pending.lock().unwrap().is_none());assert_eq!(server.join().unwrap().len(),2);
});}
#[test]fn lc1_join_revision_two_desktops_preview_then_join_in_sequence(){tauri::async_runtime::block_on(async{
 let(mut a,room)=client_room();let(mut b,_)=client_room();b.epoch=a.epoch.clone();assert_ne!(a.device,b.device);assert_ne!(a.desktop.as_ref().unwrap().id,b.desktop.as_ref().unwrap().id);
 let revision=Arc::new(AtomicU64::new(3));let mut replies=vec![Reply::Data(preview(&room,3,true)),Reply::Data(preview(&room,3,true))];
 for _ in 0..4{let revision=revision.clone();let room=room.clone();replies.push(Reply::Build(Box::new(move|q|{
  if q["roomPreview"].is_object(){return preview(&room,revision.load(Ordering::Acquire)as u32,true)}
  assert_eq!(q["roomJoin"]["password"],"fixture-password");let current=revision.load(Ordering::Acquire);if q["expectedRevision"].as_u64()!=Some(current){return json!({"error":{"code":"STALE_REVISION"}})}revision.fetch_add(1,Ordering::AcqRel);joined(&room,current as u32+1)
 })));}
 let(origin,server)=network(replies);set_origin(&mut a,&origin);set_origin(&mut b,&origin);reviewed(&mut a,&room).await;reviewed(&mut b,&room).await;join(&mut a).await.unwrap();join(&mut b).await.unwrap();
 assert_eq!(revision.load(Ordering::Acquire),5);assert!(a.world.wants_prepare&&b.world.wants_prepare);let calls=server.join().unwrap();assert_eq!(calls.len(),6);assert_eq!(calls[2]["expectedRevision"],3);assert_eq!(calls[3]["expectedRevision"],3);assert_eq!(calls[5]["expectedRevision"],4);assert_ne!(calls[3]["requestId"],calls[5]["requestId"]);
});}
#[test]fn lc1_join_revision_changed_review_is_displayed_and_requires_new_click(){tauri::async_runtime::block_on(async{
 for field in ["config-revision","actual-config","run","run-epoch","state","eligibility"]{
  let(mut c,room)=client_room();let mut fresh=room.clone();match field{
   "config-revision"=>fresh.config_revision=Some(room.config_revision.unwrap()+1),"actual-config"=>fresh.config.as_mut().unwrap().seed=Some(9988),"run"=>fresh.run_id=Some(uuid::Uuid::new_v4().to_string()),"run-epoch"=>fresh.run_epoch=Some(uuid::Uuid::new_v4().to_string()),"state"=>fresh.state=Some("waiting".into()),_=>{}
  }
  let mut replies=vec![Reply::Data(preview(&room,3,true)),Reply::Error("STALE_REVISION"),Reply::Data(preview(&fresh,4,field!="eligibility"))];if field!="eligibility"{replies.push(Reply::Data(joined(&fresh,5)));}
  let(origin,server)=network(replies);set_origin(&mut c,&origin);reviewed(&mut c,&room).await;let error=join(&mut c).await.unwrap_err();assert!(error.contains("Review the refreshed"),"{field}: {error}");assert!(!c.world.wants_prepare);let review=c.setup.preview.as_ref().unwrap();assert_eq!(review.revision,4);assert_eq!(serde_json::to_value(&review.config).unwrap(),serde_json::to_value(fresh.config.as_ref().unwrap()).unwrap());
  if field!="eligibility"{join(&mut c).await.unwrap();assert!(c.world.wants_prepare);}
  let calls=server.join().unwrap();assert_eq!(calls.iter().filter(|q|q["roomJoin"].is_object()).count(),if field=="eligibility"{1}else{2});
 }
});}
#[test]fn lc1_join_revision_repeated_stale_is_bounded_and_next_click_uses_fresh_preview(){tauri::async_runtime::block_on(async{
 let(mut c,room)=client_room();let(origin,server)=network(vec![Reply::Data(preview(&room,3,true)),Reply::Error("STALE_REVISION"),Reply::Data(preview(&room,4,true)),Reply::Error("STALE_REVISION"),Reply::Data(preview(&room,5,true)),Reply::Data(joined(&room,6))]);set_origin(&mut c,&origin);reviewed(&mut c,&room).await;assert!(join(&mut c).await.unwrap_err().contains("still changing"));assert_eq!(c.setup.preview.as_ref().unwrap().revision,5);assert!(!c.world.wants_prepare);join(&mut c).await.unwrap();let calls=server.join().unwrap();assert_eq!(calls.len(),6);assert_eq!(calls[5]["expectedRevision"],5);
});}
#[test]fn lc1_join_revision_unconfirmed_reply_never_retries_or_mints_another_join(){tauri::async_runtime::block_on(async{
 let(mut c,room)=client_room();let(origin,server)=network(vec![Reply::Data(preview(&room,3,true)),Reply::Lost]);set_origin(&mut c,&origin);reviewed(&mut c,&room).await;assert!(join(&mut c).await.unwrap_err().contains("could not be confirmed"));assert!(join(&mut c).await.unwrap_err().contains("unconfirmed"));assert!(!c.world.wants_prepare);let calls=server.join().unwrap();assert_eq!(calls.len(),2);
});}
#[test]fn lc1_join_revision_selection_or_generation_turnover_rejects_late_refresh(){tauri::async_runtime::block_on(async{
 for during_refresh in [false,true]{for selection in [false,true]{
  let(mut c,room)=client_room();let fence=if selection{c.lobby_review_fence()}else{c.generation.clone()};let changed=Reply::Fence(if during_refresh{preview(&room,4,true)}else{json!({"error":{"code":"STALE_REVISION"}})},fence);
  let replies=if during_refresh{vec![Reply::Data(preview(&room,3,true)),Reply::Error("STALE_REVISION"),changed]}else{vec![Reply::Data(preview(&room,3,true)),changed]};let(origin,server)=network(replies);set_origin(&mut c,&origin);reviewed(&mut c,&room).await;assert!(join(&mut c).await.is_err());assert!(!c.world.wants_prepare);let calls=server.join().unwrap();assert_eq!(calls.iter().filter(|q|q["roomJoin"].is_object()).count(),1);
 }}
});}
#[test]fn lc1_join_revision_context_fields_cannot_reuse_an_old_review(){tauri::async_runtime::block_on(async{
 for field in 0..7{let(mut c,room)=client_room();let(origin,server)=network(vec![Reply::Data(preview(&room,3,true))]);set_origin(&mut c,&origin);reviewed(&mut c,&room).await;match field{0=>c.base.push_str("/other"),1=>c.service.push_str("-other"),2=>c.epoch.push_str("-other"),3=>c.device=uuid::Uuid::new_v4(),4=>c.desktop.as_mut().unwrap().account.push_str("-other"),5=>c.desktop.as_mut().unwrap().id.push_str("-other"),_=>c.desktop.as_mut().unwrap().epoch.push_str("-other")};assert!(join(&mut c).await.unwrap_err().contains("review"));assert_eq!(server.join().unwrap().len(),1);}
});}

#[test]fn lc1_join_revision_queued_preview_fences_before_actor_execution(){tauri::async_runtime::block_on(async{
 let(c,_)=client_room();let fence=c.lobby_review_fence();let before=fence.load(Ordering::Acquire);let(handle,actor)=crate::next_app::scheduler::new(c,Arc::new(std::sync::atomic::AtomicBool::new(false)));
 let action:crate::next_app::Action=serde_json::from_value(json!({"action":"preview_lobby","room":"another-room"})).unwrap();let queued=tokio::spawn(async move{handle.action(action).await});
 let until=Instant::now()+Duration::from_secs(1);while fence.load(Ordering::Acquire)==before{assert!(Instant::now()<until);tokio::task::yield_now().await;}
 assert!(fence.load(Ordering::Acquire)>before,"The queued selection must retire the old Join without polling the actor");queued.abort();assert!(tokio::time::timeout(Duration::from_secs(1),queued).await.unwrap().unwrap_err().is_cancelled());drop(actor);
});}

#[test]fn lc1_join_revision_failed_command_publishes_fresh_review_before_reply(){tauri::async_runtime::block_on(async{
 let(mut c,room)=client_room();let mut fresh=room.clone();fresh.config_revision=Some(room.config_revision.unwrap()+1);
 let(origin,server)=network(vec![Reply::Data(preview(&room,3,true)),Reply::Error("STALE_REVISION"),Reply::Data(preview(&fresh,4,true))]);set_origin(&mut c,&origin);reviewed(&mut c,&room).await;c.info=None;
 let stop=Arc::new(std::sync::atomic::AtomicBool::new(false));let(handle,actor)=crate::next_app::scheduler::new(c,stop.clone());let actor=tokio::spawn(actor);
 assert!(handle.command(|c|Box::pin(async move{join(c).await?;Ok(Value::Null)})).await.is_err());assert_eq!(handle.view()["setup"]["preview"]["revision"],4);
 stop.store(true,Ordering::Release);tokio::time::timeout(Duration::from_secs(5),actor).await.unwrap().unwrap();assert_eq!(server.join().unwrap().len(),3);
});}
