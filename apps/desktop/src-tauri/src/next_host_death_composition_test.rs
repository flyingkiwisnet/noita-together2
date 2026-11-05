//! Host management is independent from eliminated game participation.
use super::*;
#[test]#[ignore="Root-serialized disposable DB composition only"]
fn ux1_host_death_core_retains_lobby_controls_without_reentry_or_peer_disruption(){
 assert_eq!(std::env::var("NT2_AUTH1_COMPOSITION").as_deref(),Ok("1"));
 let root=repo().join("spikes/noita-entry/.build").join(format!("ux1-host-death-composition-{}",uuid::Uuid::new_v4()));std::fs::create_dir_all(&root).unwrap();
 tauri::async_runtime::block_on(async{
  let mut f=Fixture::start_script(&root,"apps/desktop/tools/ux1-host-death-fixture.ts");let mut stores=Stores(vec![]);let mut c=client(&f,&root,&mut stores);
  let credentials:wire::Credentials=serde_json::from_value(f.private["host"].clone()).unwrap();c.device=uuid::Uuid::parse_str(credentials.device_id.as_deref().unwrap()).unwrap();let origin=f.private["origin"].as_str().unwrap().to_owned();c.select(&origin,"",false).await.unwrap();c.desktop=Some(Session::read(credentials,&c.device.to_string(),"desktop").unwrap());c.room=Some(serde_json::from_value(f.private["room"].clone()).unwrap());c.enable_automatic_setup();c.enable_lc1();
  let room=c.room.as_ref().unwrap().room_id.clone().unwrap();let host=c.desktop.as_ref().unwrap().account.clone();let desktop=c.desktop.as_ref().unwrap().id.clone();
  f.command(json!({"op":"death"}));c.ux1_refresh().await.unwrap();
  assert_eq!(c.room.as_ref().unwrap().host_account_id.as_deref(),Some(host.as_str()));assert!(!c.room.as_ref().unwrap().members.as_ref().unwrap().iter().any(|m|m.desktop_id.as_deref()==Some(desktop.as_str())));assert!(c.desktop.is_some());assert_eq!(c.view()["room"]["roomId"],room);
  // Actual host-only C1 control remains available after canonical removal.
  c.ux1_rules(true,None,None).await.unwrap();let snapshot=c.ux1_view()["rules"]["player"].clone();let mut changed=snapshot["rules"].clone();changed["NT_ban_perks"]=json!(true);c.ux1_rules(true,Some(serde_json::from_value(json!({"expectedRevision":snapshot["revision"],"rules":changed})).unwrap()),None).await.unwrap();
  let peer=f.command(json!({"op":"peer"}));assert_eq!(peer["authorized"],true);assert_eq!(peer["state"],"running");
  assert!(c.join_selected(&room,None).await.is_err(),"Eliminated host re-entered same run");
  let state=f.command(json!({"op":"state"}));assert_eq!(state["hostMember"],0);assert_eq!(state["owner"],host);assert!(c.game.is_none());assert!(c.pending.lock().unwrap().is_none());
  c.ux1_refresh().await.unwrap();let revision=c.room.as_ref().unwrap().revision.unwrap();c.ux1_room_action("end",revision,None,None,None,None).await.unwrap();assert_eq!(c.room.as_ref().unwrap().state.as_deref(),Some("ended"));assert_eq!(c.room.as_ref().unwrap().host_account_id.as_deref(),Some(host.as_str()));
  drop(c);drop(stores);f.close();std::fs::write(root.join("result.json"),serde_json::to_vec_pretty(&json!({"passed":true,"actualCoreServicePostgres":true,"hostManagementAfterParticipationRemoval":true,"hostRulesMutation":true,"hostEndRun":true,"sameRunJoinRefused":true,"noLocalGameAuthority":true,"peerContinues":true,"schemaCleanupVerified":true,"engineFacts":"synthetic C1 death and game observations","actualNoita":false})).unwrap()).unwrap();eprintln!("Host death Core composition passed: {}",root.display());
 });
}
