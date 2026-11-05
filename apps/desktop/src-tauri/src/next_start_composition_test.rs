//! Actual Core + HTTPS service + owned PostgreSQL schema. Engine facts synthetic.
use super::*;
async fn inspect(c:&mut Core<Protected>){
 let mut q=c.request(false);q.room_get=Some(wire::Empty{});let r=c.rpc(&q,false,false).await.unwrap();c.absorb(&r);
 let mut reports=Vec::new();let mut cursor=None;
 for _ in 0..32{let mut q=c.request(false);q.inspection_list=Some(wire::InspectionList{limit:Some(1),after_desktop_id:cursor});let r=c.rpc(&q,false,false).await.unwrap();reports.extend(r.inspections.unwrap_or_default());cursor=r.inspection_next_desktop_id;if cursor.as_deref().is_none_or(str::is_empty){break}}
 c.inspections=reports;
}
#[test]#[ignore="Root-serialized disposable DB composition only"]
fn ux1_start_actual_core_service_paused_churn_race_lost_reply_and_meaningful_change(){
 assert_eq!(std::env::var("NT2_AUTH1_COMPOSITION").as_deref(),Ok("1"));let root=repo().join("spikes/noita-entry/.build").join(format!("ux1-start-composition-{}",uuid::Uuid::new_v4()));std::fs::create_dir_all(&root).unwrap();
 tauri::async_runtime::block_on(async{
  for scenario in ["paused-churn","race","lost-reply","changed-mods","changed-config","changed-roster","lost-connection"]{
   let output=root.join(scenario);std::fs::create_dir_all(&output).unwrap();let mut f=Fixture::start_script(&output,"apps/desktop/tools/ux1-start-fixture.ts");let mut stores=Stores(vec![]);let mut c=client(&f,&output,&mut stores);
   let credentials:wire::Credentials=serde_json::from_value(f.private["host"].clone()).unwrap();c.device=uuid::Uuid::parse_str(credentials.device_id.as_deref().unwrap()).unwrap();let origin=f.private["origin"].as_str().unwrap().to_owned();c.select(&origin,"",false).await.unwrap();c.desktop=Some(Session::read(credentials,&c.device.to_string(),"desktop").unwrap());c.room=Some(serde_json::from_value(f.private["room"].clone()).unwrap());c.enable_automatic_setup();inspect(&mut c).await;
   assert_eq!(c.inspections.len(),2);assert!(c.room.as_ref().unwrap().members.as_ref().unwrap().iter().all(|m|m.ready==Some(true)));
   let capture=c.capture_start().unwrap();let revision=capture.room.config_revision.unwrap();let participants=capture.inspections.iter().map(|r|wire::InspectedParticipant{desktop_id:r.desktop_id.clone(),attachment_id:r.attachment_id.clone(),world_token:r.world_token.clone(),observation_revision:r.observation_revision,content_digest:r.content_digest.clone()}).collect::<Vec<_>>();
   match scenario{
    "paused-churn"=>{f.command(json!({"op":"churn"}));f.command(json!({"op":"pause"}));},
    "race"=>{f.command(json!({"op":"race"}));},
    "lost-reply"=>c.ux1_fixture_lose_next_reply(),
    other=>{f.command(json!({"op":other}));},
   }
   let result=c.start_captured(capture,participants,vec![],revision).await;
   match scenario{
    "paused-churn"|"race"=>{result.unwrap();assert_eq!(c.room.as_ref().unwrap().state.as_deref(),Some("running"));},
    "lost-reply"=>{assert!(result.is_err());assert_eq!(c.ux1_view()["pending"],true);let original=c.ux1_view()["outcome"]["id"].clone();c.ux1_retry().await.unwrap();assert_eq!(c.room.as_ref().unwrap().state.as_deref(),Some("running"));let d=f.command(json!({"op":"diagnostics"}));let starts=d["starts"].as_array().unwrap();assert_eq!(starts.len(),2);assert_eq!(starts[0],starts[1]);assert_eq!(starts[0]["requestId"],original);},
    _=>{assert!(result.is_err(),"{scenario}");assert_eq!(c.ux1_view()["pending"],false);},
   }
   let d=f.command(json!({"op":"diagnostics"}));let starts=d["starts"].as_array().unwrap();let expected=match scenario{"race"|"lost-reply"=>2,"paused-churn"=>1,_=>0};assert_eq!(starts.len(),expected,"{scenario}");if scenario=="race"{assert_ne!(starts[0]["requestId"],starts[1]["requestId"]);assert_ne!(starts[0]["finalStart"],starts[1]["finalStart"]);}
   std::fs::write(output.join("result.json"),serde_json::to_vec_pretty(&json!({"passed":true,"scenario":scenario,"startSubmissions":expected,"actualCoreServicePostgres":true,"engineFacts":"synthetic protected lifecycle observations","actualNoita":false})).unwrap()).unwrap();drop(c);drop(stores);f.close();
  }
  std::fs::write(root.join("result.json"),serde_json::to_vec_pretty(&json!({"passed":true,"scenarios":7,"schemaCleanupVerified":true,"actualNoita":false})).unwrap()).unwrap();eprintln!("Start composition passed: {}",root.display());
 });
}
