mod fixture{include!("c1_startup_fixture.rs");}
#[test]fn startup_retained_mixed_history_preserves_bytes_identity_digest_and_fences(){
 let f=Fixture::new();let bytes=fixture::mixed_history();std::fs::write(&f.path,&bytes).unwrap();
 let mut j=Journal::open(&f.path).unwrap();assert_eq!(j.pending().count(),13);
 let entries:Vec<_>=j.pending().cloned().collect();
 for p in entries{
  let o=&p.record.original;assert!(p.restored);assert!(j.uncertain(&o.operation_id));assert!(j.prepare_effect(&o.operation_id).is_err());
  if let Some(d)=&p.record.delivery{assert_eq!(d.payload_digest,retained_operation_digest(o).unwrap());}
  if o.item.as_ref().unwrap().payload_json.contains("nt2-items-base-1"){
   assert!(operation_digest(o).is_err());assert!(j.begin(o.clone()).is_err());
   let mut op=super::items::item_operation(&super::items::item_original());
   op.operation_id=Some(o.operation_id.clone());op.principal_id=Some(o.principal_id.clone());op.original_scope=Some(serde_json::from_str(&o.scope_json).unwrap());op.item=Some(serde_json::from_str(&o.item.as_ref().unwrap().payload_json).unwrap());op.payload_digest=Some(retained_operation_digest(o).unwrap());
   if let Some(d)=&p.record.delivery{op.delivery_id=Some(d.delivery_id.clone());}
   assert!(j.recover_item_unissued(&op).is_err());assert!(j.uncertain(&o.operation_id));
  }
 }
 assert_eq!(bytes,std::fs::read(&f.path).unwrap());
 // New current-catalogue work still proceeds through the complete durable effect gate.
 let o=super::items::item_original();let op=super::items::item_operation(&o);j.begin(o.clone()).unwrap();j.accept_item_operation(&op).unwrap();let permit=j.prepare_effect(&o.operation_id).unwrap();j.finish_session_effect(permit,true).unwrap();drop(j);
 let j=Journal::open(&f.path).unwrap();assert_eq!(j.pending().count(),14);
 for p in j.pending(){assert!(p.restored);}
}
#[test]fn startup_retained_corruption_and_torn_prefix_fail_closed(){
 let bytes=fixture::mixed_history();let lines:Vec<serde_json::Value>=std::str::from_utf8(&bytes).unwrap().lines().map(|l|serde_json::from_str(l).unwrap()).collect();
 for mutation in 0..6{
  let f=Fixture::new();let mut lines=lines.clone();
  match mutation{
   0=>lines[2]["delivery"]["payload_digest"]=serde_json::json!("0".repeat(64)),
   1=>lines[1]["original"]["item"]["payload_json"]=serde_json::json!("{}"),
   2=>lines[2]["phase"]=serde_json::json!("AppliedSession"),
   3=>lines[1]["original"]["operation_id"]=serde_json::json!("bad"),
   4=>lines[1]["sequence"]=serde_json::json!(90),
   _=>{}
  }
  let mut bad=lines.into_iter().map(|l|l.to_string()+"\n").collect::<String>().into_bytes();if mutation==5{bad.pop();}
  std::fs::write(&f.path,&bad).unwrap();assert!(Journal::open(&f.path).is_err(),"mutation {mutation}");assert_eq!(bad,std::fs::read(&f.path).unwrap());
 }
}
#[test]fn startup_retained_snapshots_keep_historical_delivery_and_identity(){
 let f=Fixture::new();let bytes=fixture::mixed_history();std::fs::write(&f.path,&bytes).unwrap();let j=Journal::open(&f.path).unwrap();
 let entries:Vec<_>=j.pending().cloned().collect();drop(j);let mut snapshots=Vec::new();
 for (i,p) in entries.iter().enumerate(){let c=Checkpoint{version:2,sequence:(i+1)as u32,kind:"snapshot".into(),deleted:None,original:Some(p.record.original.clone()),original_id:None,delivery:p.record.delivery.clone(),phase:Some(p.record.phase)};serde_json::to_writer(&mut snapshots,&c).unwrap();snapshots.push(b'\n');}
 std::fs::write(&f.path,&snapshots).unwrap();let j=Journal::open(&f.path).unwrap();assert_eq!(j.pending().count(),13);
 for p in j.pending(){let prior=entries.iter().find(|old|old.record.original.operation_id==p.record.original.operation_id).unwrap();assert_eq!(prior.record.original,p.record.original);assert_eq!(prior.record.delivery,p.record.delivery);assert!(p.restored);}
 assert_eq!(snapshots,std::fs::read(&f.path).unwrap());
}
