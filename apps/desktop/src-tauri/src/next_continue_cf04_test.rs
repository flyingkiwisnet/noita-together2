use super::*;

#[test]
fn cf04_renewable_lease_refresh_keeps_original_identity_and_start_window() {
 let(mut c,p,mut first)=release_case();
 first.release_expires_at=Some("2098-01-01T00:00:00Z".into());
 c.apply_release(Some(first.clone()),&p).unwrap();
 // Simulate time passing beyond the first short transport lease while the
 // separately scoped fixed Start/review window remains valid.
 c.game.as_mut().unwrap().permit.as_mut().unwrap().release_expires_at=Some("2021-01-01T00:00:00Z".into());
 let mut current=first.clone();current.release_expires_at=Some("2097-01-01T00:00:00Z".into());
 c.apply_release(Some(current.clone()),&p).unwrap();
 assert!(c.pending.lock().unwrap().as_ref().unwrap().release);
 let mut stored=c.game.as_ref().unwrap().permit.clone().unwrap();
 assert_eq!(stored.release_expires_at,current.release_expires_at);
 stored.release_expires_at=first.release_expires_at.clone();
 assert_eq!(serde_json::to_value(stored).unwrap(),serde_json::to_value(first).unwrap());
 assert!(!c.game.as_ref().unwrap().entered,"service lease must not fabricate native consumption");
 c.preparation.as_mut().unwrap().review_expires_at=Some("2021-01-01T00:00:00Z".into());
 assert!(c.apply_release(Some(current.clone()),&p).is_err(),"healthy renewable lease cannot extend fixed Start window");
 assert!(!c.pending.lock().unwrap().as_ref().unwrap().release);
 assert_eq!(c.game.as_ref().unwrap().permit.as_ref().unwrap().release_expires_at,current.release_expires_at);
}

#[test]
fn cf04_lease_normalization_never_accepts_changed_frozen_fields() {
 for key in ["authorizationId","startRequestId","contractRevision","serviceId","serviceEpoch","roomId","runId","runEpoch","configRevision","desktopId","desktopSessionEpoch","gameSessionId","gameSessionEpoch","attachmentId","gameInstanceId","worldToken","observationRevision","contentDigest","actualSeed","seedPolicy","randomizeLoot","capturedAt","authorizedAt"] {
  let(mut c,p,first)=release_case();c.apply_release(Some(first.clone()),&p).unwrap();
  let mut value=serde_json::to_value(&first).unwrap();
  value["releaseExpiresAt"]=json!("2098-01-01T00:00:00Z");
  value[key]=match key {
   "configRevision"|"observationRevision"=>json!(2),
   "randomizeLoot"=>json!(!first.randomize_loot.unwrap_or(false)),
   "capturedAt"=>json!("2000-01-01T00:00:00.500Z"),
   "authorizedAt"=>json!("2000-01-01T00:00:02Z"),
   _=>json!(uuid::Uuid::new_v4().to_string()),
  };
  let changed=serde_json::from_value(value).unwrap();
  assert!(c.apply_release(Some(changed),&p).is_err(),"changed {key}");
  assert!(!c.pending.lock().unwrap().as_ref().unwrap().release,"changed {key}");
  assert_eq!(serde_json::to_value(c.game.as_ref().unwrap().permit.as_ref().unwrap()).unwrap(),serde_json::to_value(first).unwrap(),"refused field must not replace permit: {key}");
 }
}

#[test]
fn cf04_first_entry_still_requires_live_lease_scope_generation_phase_and_report() {
 for fault in 0..12 {
  let(mut c,mut p,first)=release_case();c.apply_release(Some(first.clone()),&p).unwrap();
  let mut current=first.clone();current.release_expires_at=Some("2098-01-01T00:00:00Z".into());
  match fault {
   0=>current.release_expires_at=Some("2021-01-01T00:00:00Z".into()),
   1=>current.release_expires_at=Some("bad".into()),
   2=>current.phase=Some("playing".into()),
   3=>c.room.as_mut().unwrap().state=Some("ended".into()),
   4=>c.native_revoked=true,
   5=>{c.generation.fetch_add(1,Ordering::AcqRel);},
   6=>c.preparation.as_mut().unwrap().room_id=Some(uuid::Uuid::new_v4().to_string()),
   7=>c.preparation=None,
   8=>c.preparation.as_mut().unwrap().review_expires_at=Some("bad".into()),
   9=>{p.observation.as_mut().unwrap().observation.complete=false;*c.pending.lock().unwrap()=Some(p.clone());},
   10=>{p.invalidated_observation=2;*c.pending.lock().unwrap()=Some(p.clone());},
   _=>c.room.as_mut().unwrap().config.as_mut().unwrap().seed=Some(7),
  }
  assert!(c.apply_release(Some(current),&p).is_err(),"fault {fault}");
  assert!(!c.pending.lock().unwrap().as_ref().unwrap().release,"fault {fault}");
  assert!(!c.game.as_ref().unwrap().entered);
 }
 let(mut c,p,first)=release_case();c.apply_release(Some(first),&p).unwrap();c.apply_release(None,&p).unwrap();
 assert!(!c.pending.lock().unwrap().as_ref().unwrap().release,"missing/terminal service release holds world");
}
