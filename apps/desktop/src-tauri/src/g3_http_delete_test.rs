#[test]fn lobby_delete_requires_authenticated_correlated_current_desktop_status_and_exact_original_scope(){tauri::async_runtime::block_on(async{
 use crate::g3_journal::{Journal,Original};
 let dir=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).ancestors().nth(3).unwrap().join("spikes/noita-entry/.build").join(format!("http-delete-{}",uuid::Uuid::new_v4()));std::fs::create_dir_all(&dir).unwrap();let path=dir.join("journal.jsonl");let mut journal=Journal::open(&path).unwrap();let initial=owner();
 let original=Original { item:None,service_origin:initial.binding.service_origin.clone(),service_id:initial.binding.service_id.clone(),service_epoch:"old-service-epoch".into(),session_epoch:"old-game-epoch".into(),principal_id:initial.binding.principal_id.clone(),scope_json:serde_json::to_string(&initial.binding.scope).unwrap(),operation_id:uuid::Uuid::new_v4().to_string(),direction:"deposit".into(),amount:10};journal.begin(original.clone()).unwrap();
 for case in 0..10{
  let fence=Arc::new(AtomicU64::new(1));let mut recovery=RecoveryOwner::new(original.service_origin.clone(),original.service_id.clone(),"current-service-epoch".into(),"current-desktop-epoch".into(),original.principal_id.clone(),Zeroizing::new("desktop-token".into()),fence.clone());
  let expected=original.clone();let result=recovery.status_or_deleted_via(&journal,&original.operation_id,move|http,bytes|{
   let request=protocol_g3::request(&bytes).unwrap();assert_eq!(request.service_epoch.as_deref(),Some("current-service-epoch"));assert_eq!(request.session_epoch.as_deref(),Some("current-desktop-epoch"));assert!(request.current_scope.is_none());assert_eq!(serde_json::to_value(&request.scope).unwrap(),serde_json::from_str::<serde_json::Value>(&expected.scope_json).unwrap());assert_eq!(request.gold_status.unwrap().operation_id.as_deref(),Some(expected.operation_id.as_str()));assert_eq!(http.token.as_str(),"desktop-token");
   let mut response=serde_json::json!({"major":1,"revision":protocol_g3::REVISION,"requestId":request.request_id,"serviceId":request.service_id,"serviceEpoch":request.service_epoch,"error":{"code":"ROOM_DELETED"}});
   match case{0=>response["requestId"]=uuid::Uuid::new_v4().to_string().into(),1=>response["serviceId"]="foreign".into(),2=>response["serviceEpoch"]="old-service-epoch".into(),3=>{fence.fetch_add(1,Ordering::AcqRel);},4=>response["error"]["code"]="NOT_FOUND".into(),5=>response["error"]["code"]="FORBIDDEN".into(),6=>response["operation"]=serde_json::json!({}),7=>return Box::pin(async{Err("network unavailable".into())}),8=>response["revision"]="unknown".into(),_=>{}}
   Box::pin(async move{Ok(serde_json::to_vec(&response).unwrap())})
  }).await;
  if case==9{match result.unwrap(){OriginalStatus::LobbyDeleted(proof)=>{assert_eq!(proof.identity(),&crate::g3_journal::LobbyIdentity::from_original(&original).unwrap());assert_eq!(journal.retire_lobby(proof).unwrap(),1);},_=>panic!("Terminal deletion proof absent")}}
  else{assert!(!matches!(result,Ok(OriginalStatus::LobbyDeleted(_))),"Invalid deletion proof case {case}");assert_eq!(journal.pending().count(),1);assert!(!journal.lobby_deleted(&crate::g3_journal::LobbyIdentity::from_original(&original).unwrap()));}
 }
 drop(journal);let reopened=Journal::open(&path).unwrap();assert_eq!(reopened.pending().count(),0);drop(reopened);std::fs::remove_file(path).unwrap();std::fs::remove_dir(dir).unwrap();
});}
