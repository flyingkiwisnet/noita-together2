// Payload-free fixture attribution; counts are diagnostic events, not users or
// unique failures. Client permission/progress/transport guards stay independent.
fn capacity_shape(value:&Value)->Result<(),String>{
 if value["revision"]!=1{return Err("Capacity diagnostic revision differs".into())}
 let events=value["events"].as_array().ok_or("Missing capacity events")?;
 if events.len()>64{return Err("Capacity event bound exceeded".into())}
 let counts=value["counts"].as_object().ok_or("Missing capacity counters")?;
 if counts.len()>32||counts.iter().any(|(k,v)|k.is_empty()||k.len()>64||!k.bytes().all(|b|b.is_ascii_lowercase()||b==b'-')||v.as_u64().is_none()){return Err("Invalid/unbounded fixed-category capacity counters".into())}
 let event=|e:&Value|->Result<(),String>{
  let object=e.as_object().ok_or("Invalid capacity event")?;
  const FIELDS:[&str;15]=["elapsedMs","guard","limit","count","windowRemainingMs","windowWallElapsedMs","windowRateElapsedMs","credit","refillRemainingMs","retire","busy","controlFrames","bufferedBytes","stalledMs","admission"];
  if object.keys().any(|k|!FIELDS.contains(&k.as_str()))||e["elapsedMs"].as_u64().is_none()||!e["guard"].as_str().is_some_and(|s|!s.is_empty()&&s.len()<=64&&s.bytes().all(|b|b.is_ascii_lowercase()||b==b'-')){return Err("Capacity event has unknown/private fields or missing guard/time".into())}
  for (k,v) in object{if ["elapsedMs","guard","retire","busy","admission"].contains(&k.as_str()){continue}if !v.as_f64().is_some_and(f64::is_finite){return Err("Invalid capacity numeric diagnostic".into())}}
  if e.get("retire").is_some()&&e["retire"]!="CAPACITY"{return Err("Unexpected capacity retirement kind".into())}
  if e.get("busy").is_some()&&!e["busy"].is_boolean(){return Err("Invalid capacity busy flag".into())}
  if let Some(a)=e.get("admission"){let a=a.as_object().ok_or("Invalid admission counters")?;if a.len()>8||a.values().any(|v|v.as_u64().is_none()){return Err("Invalid admission counter bounds".into())}}
  Ok(())
 };
 for e in events{event(e)?;}
 match value.get("firstRetire"){Some(Value::Null)=>{},Some(v)=>{event(v)?;if v["retire"]!="CAPACITY"{return Err("First capacity retirement lost its kind".into())}},None=>return Err("Missing first capacity retirement slot".into())}
 Ok(())
}
fn capacity_phase(before:&Value,after:&Value)->Result<Value,String>{
 capacity_shape(before)?;capacity_shape(after)?;
 if !before["firstRetire"].is_null()||!after["firstRetire"].is_null(){return Err("Unexpected service CAPACITY retirement in eligible flow".into())}
 if before["counts"]!=after["counts"]{return Err("Capacity guard counters changed during eligible steady play".into())}
 Ok(json!({"before":before,"after":after,"counts_are":"fixed-category diagnostic events, not unique failures","new_guard_events":0,"capacity_retirement_observed":false}))
}

#[test]fn lc1_network_capacity_snapshot_preserves_first_cause_and_rejects_missing_unbounded_evidence(){
 let clean=json!({"revision":1,"events":[],"counts":{},"firstRetire":null});capacity_phase(&clean,&clean).unwrap();
 let event=json!({"elapsedMs":17,"guard":"traffic-pose-burst","retire":"CAPACITY","credit":0,"busy":false,"controlFrames":0,"bufferedBytes":0,"stalledMs":0,"admission":{"active":0,"peak":1,"rejected":0,"limit":16}});
 let refused=json!({"revision":1,"events":[event.clone()],"counts":{"traffic-pose-burst":2},"firstRetire":event});capacity_shape(&refused).unwrap();assert!(capacity_phase(&clean,&refused).is_err());
 let mut changed=clean.clone();changed["counts"]=json!({"traffic-class-health":1});assert!(capacity_phase(&clean,&changed).is_err());
 let mut missing=clean.clone();missing.as_object_mut().unwrap().remove("firstRetire");assert!(capacity_shape(&missing).is_err());
 let mut unbounded=refused.clone();unbounded["events"]=Value::Array(vec![refused["firstRetire"].clone();65]);assert!(capacity_shape(&unbounded).is_err());
 let mut private=refused;private["events"][0]["sessionId"]=json!("must-not-be-accepted");assert!(capacity_shape(&private).is_err());
}
