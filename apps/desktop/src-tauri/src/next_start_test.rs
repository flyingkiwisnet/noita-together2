use super::*;
use super::super::setup::tests::{network,Reply};
use super::super::tests::{MemoryStore,release_case};
pub(crate) fn fixture()->(Core<MemoryStore>,Vec<wire::InspectedParticipant>){
 let(mut c,_,_)=release_case();c.clear_game();c.enable_automatic_setup();c.info=Some(wire::Info{connection_flow_revision:Some("g3-flow-1".into()),..Default::default()});
 let room=c.room.as_mut().unwrap();room.state=Some("staging".into());room.run_id=Some(uuid::Uuid::new_v4().to_string());room.run_epoch=Some(uuid::Uuid::new_v4().to_string());room.host_account_id=Some(c.desktop.as_ref().unwrap().account.clone());
 let m=&mut room.members.as_mut().unwrap()[0];m.present=Some(true);m.ready=Some(true);m.readiness=Some("ready".into());m.consented_config_revision=room.config_revision;
 let p=wire::InspectedParticipant{desktop_id:m.desktop_id.clone(),attachment_id:Some(uuid::Uuid::new_v4().to_string()),world_token:Some(uuid::Uuid::new_v4().to_string()),observation_revision:Some(1),content_digest:Some("a".repeat(64))};
 c.inspections=vec![wire::Inspection{desktop_id:p.desktop_id.clone(),attachment_id:p.attachment_id.clone(),world_token:p.world_token.clone(),game_instance_id:Some(uuid::Uuid::new_v4().to_string()),observation_revision:p.observation_revision,content_digest:p.content_digest.clone(),actual_seed:Some("424242".into()),active_mods:Some(vec!["noita_together2".into()]),loaded:Some(true),protected:Some(true),inventory_complete:Some(true),..Default::default()}];(c,vec![p])
}
fn refresh(room:&wire::Room,report:&wire::Inspection)->Vec<Reply>{vec![Reply::Data(json!({"room":room})),Reply::Data(json!({"inspections":[report]})),Reply::Data(json!({"room":room}))]}
#[test]fn ux1_start_harmless_observation_churn_and_paused_live_world_use_latest_revision(){tauri::async_runtime::block_on(async{
 let(mut c,p)=fixture();let room=c.room.clone().unwrap();let mut latest=c.inspections[0].clone();latest.observation_revision=Some(7);latest.captured_at=Some("2026-10-31T00:00:00.000Z".into());
 let mut replies=refresh(&room,&latest);replies.push(Reply::Data(json!({})));let(origin,worker)=network(replies);c.base=origin;
 // The displayed report and current native report may differ before the queued action runs.
 let capture=c.capture_start().unwrap();c.inspections[0]=latest;
 c.start_captured(capture,p,vec![],room.config_revision.unwrap()).await.unwrap();let calls=worker.join().unwrap();assert_eq!(calls.len(),4);assert_eq!(calls[3]["finalStart"]["participants"][0]["observationRevision"],7);assert!(c.game.is_none(),"Start must not require a new native game tick");
});}
#[test]fn ux1_start_verified_races_retry_once_with_new_request_and_fresh_reports(){tauri::async_runtime::block_on(async{
 for code in ["STALE_REVISION","REPORT_CHANGED"]{for succeeds in [false,true]{let(mut c,p)=fixture();let room=c.room.clone().unwrap();let mut latest=c.inspections[0].clone();latest.observation_revision=Some(2);let mut replies=refresh(&room,&latest);replies.push(Reply::Error(code));latest.observation_revision=Some(3);replies.extend(refresh(&room,&latest));replies.push(if succeeds{Reply::Data(json!({}))}else{Reply::Error(code)});let(origin,worker)=network(replies);c.base=origin;let result=c.start_displayed(p,vec![],room.config_revision.unwrap()).await;assert_eq!(result.is_ok(),succeeds);let calls=worker.join().unwrap();assert_eq!(calls.len(),8);assert_ne!(calls[3]["requestId"],calls[7]["requestId"]);assert_eq!(calls[7]["finalStart"]["participants"][0]["observationRevision"],3);}}
});}
#[test]fn ux1_start_unknown_outcome_retains_exact_original_and_blocks_new_submit(){tauri::async_runtime::block_on(async{
 let(mut c,p)=fixture();let room=c.room.clone().unwrap();let mut replies=refresh(&room,&c.inspections[0]);replies.extend([Reply::Lost,Reply::Data(json!({}))]);let(origin,worker)=network(replies);c.base=origin;
 assert!(c.start_displayed(p.clone(),vec![],room.config_revision.unwrap()).await.is_err());assert_eq!(c.ux1_view()["outcome"]["kind"],"Start");assert!(c.start_displayed(p,vec![],room.config_revision.unwrap()).await.unwrap_err().contains("Previous lobby action"));c.ux1_retry().await.unwrap();let calls=worker.join().unwrap();assert_eq!(calls.len(),5);assert_eq!(calls[3],calls[4]);
});}
#[test]fn ux1_start_meaningful_inspection_changes_block_without_submission(){tauri::async_runtime::block_on(async{
 for field in ["worldToken","gameInstanceId","attachmentId","contentDigest","actualSeed","activeMods","loaded","protected","inventoryComplete","collectorFailure","observationRevision"]{
  let(mut c,p)=fixture();let room=c.room.clone().unwrap();let mut latest=json!(c.inspections[0]);latest[field]=match field{"loaded"|"protected"|"inventoryComplete"=>json!(false),"activeMods"=>json!(["different_mod"]),"observationRevision"=>json!(0),"contentDigest"=>json!("b".repeat(64)),"actualSeed"=>json!("123"),"collectorFailure"=>json!("disconnected"),_=>json!(uuid::Uuid::new_v4().to_string())};let(origin,worker)=network(vec![Reply::Data(json!({"room":room})),Reply::Data(json!({"inspections":[latest]}))]);c.base=origin;
  assert!(c.start_displayed(p,vec![],room.config_revision.unwrap()).await.is_err(),"{field}");assert_eq!(worker.join().unwrap().len(),2);
 }
});}
#[test]fn ux1_start_room_turnover_and_lost_readiness_block_at_final_recheck(){tauri::async_runtime::block_on(async{
 for field in ["roomId","runId","runEpoch","hostAccountId","configRevision","config","members","present","ready"]{
  let(mut c,p)=fixture();let room=c.room.clone().unwrap();let mut changed=json!(room);match field{"configRevision"=>changed[field]=json!(99),"config"=>changed[field]["seed"]=json!(17),"members"=>changed[field]=json!([]),"present"|"ready"=>changed["members"][0][field]=json!(false),_=>changed[field]=json!(uuid::Uuid::new_v4().to_string())};let(origin,worker)=network(vec![Reply::Data(json!({"room":room})),Reply::Data(json!({"inspections":c.inspections})),Reply::Data(json!({"room":changed}))]);c.base=origin;assert!(c.start_displayed(p,vec![],room.config_revision.unwrap()).await.is_err(),"{field}");assert_eq!(worker.join().unwrap().len(),3);
 }
});}
#[test]fn ux1_start_pages_complete_reports_and_rejects_stale_displayed_scope(){tauri::async_runtime::block_on(async{
 let(mut c,p)=fixture();let room=c.room.clone().unwrap();let first=c.inspections[0].clone();let mut other=first.clone();other.desktop_id=Some(uuid::Uuid::new_v4().to_string());let(origin,worker)=network(vec![Reply::Data(json!({"room":room})),Reply::Data(json!({"inspections":[first],"inspectionNextDesktopId":first.desktop_id})),Reply::Data(json!({"inspections":[other]})),Reply::Data(json!({"room":room})),Reply::Data(json!({}))]);c.base=origin;let mut old=c.capture_start().unwrap();old.session_epoch="previous sign-in".into();assert!(c.start_captured(old,p.clone(),vec![],room.config_revision.unwrap()).await.is_err());c.start_displayed(p,vec![],room.config_revision.unwrap()).await.unwrap();let calls=worker.join().unwrap();assert_eq!(calls.len(),5);assert_eq!(calls[2]["inspectionList"]["afterDesktopId"],first.desktop_id.unwrap());
});}
