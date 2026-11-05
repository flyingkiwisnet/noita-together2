fn in_lobby(room:&str,run:&str,phase:Phase,journal:&mut Journal)->Original{
 let mut o=original();let mut scope:serde_json::Value=serde_json::from_str(&o.scope_json).unwrap();scope["roomId"]=room.into();scope["runId"]=run.into();o.scope_json=scope.to_string();journal.begin(o.clone()).unwrap();
 let mut op=operation(&o);op.run_id=Some(run.into());
 if phase!=Phase::Requested{journal.accept_operation(&op).unwrap();}
 if matches!(phase,Phase::EffectIntent|Phase::AppliedSession|Phase::Uncertain){let permit=journal.prepare_effect(&o.operation_id).unwrap();if phase!=Phase::EffectIntent{journal.finish_session_effect(permit,phase==Phase::AppliedSession).unwrap();}}
 o
}
#[test]fn lobby_delete_compacts_all_runs_and_reopen_preserves_other_uncertain_originals(){
 let fixture=Fixture::new();let mut j=Journal::open(&fixture.path).unwrap();let mut deleted=vec![];
 for(i,phase)in[Phase::Requested,Phase::ServerAccepted,Phase::EffectIntent,Phase::AppliedSession,Phase::Uncertain].into_iter().enumerate(){deleted.push(in_lobby("room-delete",&format!("run-{i}"),phase,&mut j));}
 let keep=in_lobby("room-keep","run-live",Phase::Uncertain,&mut j);let kept_phase=j.entries[&keep.operation_id].record.phase;
 let key=LobbyIdentity::from_original(&deleted[0]).unwrap();assert_eq!(j.retire_lobby(DeletedLobby::authenticated(key.clone()).unwrap()).unwrap(),5);
 assert_eq!(j.pending().count(),1);assert_eq!(j.entries[&keep.operation_id].record.phase,kept_phase);
 let bytes=std::fs::read_to_string(&fixture.path).unwrap();for o in &deleted{assert!(!bytes.contains(&o.operation_id),"Deleted transfer details retained in file");assert!(j.begin(o.clone()).is_err());}
 assert!(bytes.contains(&keep.operation_id));assert!(bytes.contains("lobby_deleted"));assert_eq!(j.retire_lobby(DeletedLobby::authenticated(key.clone()).unwrap()).unwrap(),0);
 assert!(Journal::open(&fixture.path).is_err(),"Atomic compaction lost exclusive path ownership");drop(j);
 let mut reopened=Journal::open(&fixture.path).unwrap();assert_eq!(reopened.pending().count(),1);assert!(reopened.lobby_deleted(&key));assert!(reopened.uncertain(&keep.operation_id));
 let mut fresh=deleted[0].clone();fresh.operation_id=uuid::Uuid::new_v4().to_string();assert!(reopened.begin(fresh).is_err(),"New original ID escaped deleted lobby fence");assert!(reopened.prepare_effect(&keep.operation_id).is_err());
 let other=in_lobby("room-third","run-third",Phase::Requested,&mut reopened);assert!(reopened.entries.contains_key(&other.operation_id));drop(reopened);
 let reopened=Journal::open(&fixture.path).unwrap();assert_eq!(reopened.pending().count(),2);assert!(reopened.lobby_deleted(&key));
}
#[test]fn lobby_delete_scopes_origin_service_account_room_and_revokes_old_permits(){
 let fixture=Fixture::new();let j=SharedJournal::new(Journal::open(&fixture.path).unwrap());let o=original();j.begin(o.clone()).unwrap();let op=operation(&o);j.accept_operation(&op).unwrap();let permit=j.prepare_effect(&o.operation_id).unwrap();
 let key=LobbyIdentity::from_original(&o).unwrap();
 for field in 0..4{let mut other=o.clone();other.operation_id=uuid::Uuid::new_v4().to_string();match field{0=>other.service_origin="https://other.invalid".into(),1=>other.service_id="other-service".into(),2=>other.principal_id="other-account".into(),_=>{let mut scope:serde_json::Value=serde_json::from_str(&other.scope_json).unwrap();scope["roomId"]="other-room".into();other.scope_json=scope.to_string();}}j.begin(other).unwrap();}
 assert_eq!(j.retire_lobby(DeletedLobby::authenticated(key.clone()).unwrap()).unwrap(),1);assert_eq!(j.pending().count(),4);assert!(j.lobby_deleted(&key));assert!(j.finish_session_effect(permit,true).is_err());assert!(j.accept_operation(&op).is_err());assert!(j.validate_status(&op).is_err());assert!(j.recover_unissued(&op).is_err());assert!(j.begin(o).is_err());
}
#[test]fn lobby_delete_replacement_failure_keeps_other_file_bytes_and_blocks_effects(){
 let fixture=Fixture::new();let mut j=Journal::open(&fixture.path).unwrap();let o=in_lobby("room-delete","run-delete",Phase::ServerAccepted,&mut j);let keep=in_lobby("room-keep","run-keep",Phase::Requested,&mut j);
 let before=std::fs::read(&fixture.path).unwrap();let key=LobbyIdentity::from_original(&o).unwrap();
 // Actual owned scratch destination deliberately cannot be replaced. The
 // original data file stays complete and no stale/new effect can be granted.
 j.path=fixture.dir.join("directory-blocker");std::fs::create_dir(&j.path).unwrap();
 assert!(j.retire_lobby(DeletedLobby::authenticated(key.clone()).unwrap()).is_err());assert!(j.storage_blocked());assert!(j.lobby_deleted(&key));assert_eq!(std::fs::read(&fixture.path).unwrap(),before);assert!(j.prepare_effect(&o.operation_id).is_err());let mut fresh=keep;fresh.operation_id=uuid::Uuid::new_v4().to_string();assert!(j.begin(fresh).is_err());std::fs::remove_dir(&j.path).unwrap();
 drop(j);let reopened=Journal::open(&fixture.path).unwrap();assert_eq!(reopened.pending().count(),2,"Uncommitted deletion must retain originals for authenticated retry");
}
