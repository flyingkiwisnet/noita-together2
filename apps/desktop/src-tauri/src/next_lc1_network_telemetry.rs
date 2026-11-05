// Checks the reviewed fixture telemetry; it never creates service responses.
const SEMANTICS:&str="minimum-application-arrival-to-release-includes-classification";
const LANES:[&str;4]=["httpIn","httpOut","wssIn","wssOut"];
const TARGETS:[&str;2]=["consumer","peer"];
const SAMPLE_FIELDS:[&str;8]=["plannedMs","actualMs","clampedMs","classificationMs","rawQueueMs","preScheduleMs","remainingPropagationWaitMs","timerLatenessMs"];
fn need(ok:bool,message:&str)->Result<(),String>{if ok{Ok(())}else{Err(message.into())}}
fn integer(v:&Value,key:&str)->Result<u64,String>{v[key].as_u64().ok_or_else(||format!("Missing/noninteger network telemetry: {key}"))}
fn number(v:&Value,key:&str)->Result<f64,String>{v[key].as_f64().filter(|x|x.is_finite()&&*x>=0.0).ok_or_else(||format!("Missing/invalid network timing: {key}"))}
fn array<'a>(v:&'a Value,key:&str)->Result<&'a Vec<Value>,String>{v[key].as_array().ok_or_else(||format!("Missing network array: {key}"))}
fn bounded(v:&Value,key:&str,limit:u64)->Result<u64,String>{let n=integer(v,key)?;need(n<=limit,&format!("Network {key} bound exceeded"))?;Ok(n)}
fn definition(name:&str)->Result<(f64,f64),String>{match name{"rtt80"=>Ok((40.0,8.0)),"rtt200"=>Ok((100.0,20.0)),"rtt400"=>Ok((200.0,40.0)),_=>Err("Unknown network profile".into())}}
fn target(v:&Value)->Result<&str,String>{let t=v["target"].as_str().ok_or("Missing network target")?;need(TARGETS.contains(&t),"Unknown network target")?;Ok(t)}
fn lane(v:&Value)->Result<&str,String>{let l=v["lane"].as_str().ok_or("Missing network lane")?;need(LANES.contains(&l),"Unknown network lane")?;Ok(l)}
fn network(response:&Value)->Result<&Value,String>{response.get("network").filter(|v|v.is_object()).ok_or_else(||"Missing network control result".into())}
fn lane_row<'a>(n:&'a Value,t:&str,l:&str)->Result<&'a Value,String>{array(n,"telemetry")?.iter().find(|r|r["target"]==t&&r["lane"]==l).ok_or_else(||format!("Missing telemetry for {t}/{l}"))}
fn timings(values:&[f64])->Value {
 if values.is_empty(){return json!({"count":0})}let mut sorted=values.to_vec();sorted.sort_by(f64::total_cmp);
 let q=|p:usize|sorted[((sorted.len()*p+99)/100).saturating_sub(1).min(sorted.len()-1)];
 json!({"count":sorted.len(),"min_ms":sorted[0],"p50_ms":q(50),"p95_ms":q(95),"p99_ms":q(99),"max_ms":sorted[sorted.len()-1]})
}

pub(super) fn validate(response:&Value)->Result<(),String>{
 let n=network(response)?;
 need(n["revision"]==1&&n["semantics"]==SEMANTICS,"Network revision/timing semantics differ from reviewed source")?;
 need(n["limits"]==json!({"profiles":2,"configurations":64,"durationMinMs":1000,"durationMaxMs":600000,"defaultDurationMs":120000,"flows":128,"frames":256,"bytes":4194304,"wssFrames":64,"wssBytes":65536,"httpFrames":1,"httpBytes":262144,"maxResidenceMs":4000,"samples":64,"completedProfiles":8,"transitions":16}),"Network scheduler limits differ")?;
 need(n["definitions"]==json!({"rtt80":{"rttMs":80,"oneWayMs":40,"jitterMs":8},"rtt200":{"rttMs":200,"oneWayMs":100,"jitterMs":20},"rtt400":{"rttMs":400,"oneWayMs":200,"jitterMs":40}}),"Network profile definitions differ")?;
 bounded(n,"configurations",64)?;let revision=integer(n,"stateRevision")?;
 let profiles=array(n,"profiles")?;need(profiles.len()<=2,"Too many network profiles")?;
 let mut seen=std::collections::BTreeSet::new();
 for p in profiles {need(seen.insert(target(p)?),"Duplicate active network target")?;definition(p["profile"].as_str().ok_or("Missing profile name")?)?;bounded(p,"seed",65535)?;need(integer(p,"generation")?>0,"Missing profile generation")?;bounded(p,"remainingMs",600000)?;}
 let pending=&n["pending"];let high=&n["highWater"];
 for (key,limit) in [("flows",128),("frames",256),("bytes",4194304),("timers",130)]{let current=bounded(pending,key,limit)?;need(bounded(high,key,limit)?>=current,"Scheduler high-water below current pending")?;}
 need(integer(pending,"timers")?<=integer(pending,"flows")?+profiles.len() as u64,"More than one scheduler head timer per flow plus profile expiries")?;
 let raw=&n["rawIngress"];
 need(raw["limits"]==json!({"frames":64,"bytes":65536,"aggregateFrames":4096,"aggregateBytes":4194304}),"Raw ingress limits differ")?;
 for (key,limit) in [("frames",4096),("bytes",4194304)]{let current=bounded(&raw["pending"],key,limit)?;need(bounded(&raw["highWater"],key,limit)?>=current,"Raw ingress high-water below pending")?;let mut sum=0;for t in ["consumer","peer","unclassified"]{sum+=bounded(&raw["byTarget"][t],key,limit)?;}need(sum==current,"Raw ingress per-target total differs from pending")?;}
 bounded(&raw["highWater"],"maxSocketFrames",64)?;bounded(&raw["highWater"],"maxSocketBytes",65536)?;
 integer(raw,"overflow")?;integer(raw,"clearedFrames")?;
 let upper=&n["injectionBufferUpperBound"];
 need(integer(upper,"currentPayloadBytes")?==integer(&raw["pending"],"bytes")?+integer(pending,"bytes")?,"Injection payload upper bound must include both queues")?;
 need(upper["limitPayloadBytes"]==8388608&&upper["sharedIngressHeadsDoubleCounted"]==true&&upper["excludes"]=="application/TLS/kernel buffers and non-payload heap","Injection upper-bound accounting/exclusions differ")?;
 let telemetry=array(n,"telemetry")?;need(telemetry.len()<=8,"Too many telemetry lanes")?;
 let mut unique=std::collections::BTreeSet::new();
 for row in telemetry {
  let t=target(row)?;let l=lane(row)?;need(unique.insert((t,l)),"Duplicate network telemetry lane")?;
  let scheduled=integer(row,"scheduled")?;let delivered=integer(row,"delivered")?;let cancelled=integer(row,"cancelled")?;
  need(delivered.checked_add(cancelled).is_some_and(|v|v<=scheduled),"Delivered/cancelled exceeds scheduled work")?;
  integer(row,"overflow")?;integer(row,"late")?;
  bounded(row,"maxQueuedFrames",if l.starts_with("wss"){64}else{1})?;bounded(row,"maxQueuedBytes",if l.starts_with("wss"){65536}else{262144})?;
  for key in ["actualDelaySumMs","actualDelayMaxMs","timerLatenessMaxMs","classificationSumMs","rawQueueSumMs","preScheduleSumMs","propagationWaitSumMs","meanActualMs","meanClassificationMs","meanRawQueueMs","meanRemainingPropagationWaitMs"]{number(row,key)?;}
  need(number(row,"actualDelayMaxMs")?<=4000.0,"Delivered sample exceeded residence bound")?;
  let samples=array(row,"samples")?;need(samples.len()==delivered.min(64) as usize,"Delivered telemetry sample ring is incomplete/unbounded")?;
  let profile=profiles.iter().find(|p|p["target"]==t).or_else(||n["completed"].as_array()?.iter().rev().find(|p|p["target"]==t));
  for s in samples {
   for key in SAMPLE_FIELDS{number(s,key)?;}
   let name=profile.and_then(|p|p["profile"].as_str()).ok_or("Samples have no active/completed profile identity")?;
   let (one_way,jitter)=definition(name)?;let planned=number(s,"plannedMs")?;
   need([-1.0,1.0,0.0,-0.5,0.5].iter().any(|j|(planned-(one_way+j*jitter)).abs()<0.0001),"Sample planned delay is outside deterministic profile jitter")?;
   let actual=number(s,"actualMs")?;let clamped=number(s,"clampedMs")?;let pre=number(s,"preScheduleMs")?;let wait=number(s,"remainingPropagationWaitMs")?;let late=number(s,"timerLatenessMs")?;
   need(actual<=4000.0&&actual+0.001>=clamped&&clamped+0.001>=planned,"Delivered timing violates arrival deadline/residence")?;
   need((wait-(clamped-pre).max(0.0)).abs()<0.01&&(actual-pre-wait-late).abs()<0.01,"Timing double-counts classification or propagation wait")?;
   need(number(s,"classificationMs")?+number(s,"rawQueueMs")?<=pre+0.01,"Classification/raw queue time exceeds pre-schedule residence")?;
  }
 }
 let completed=array(n,"completed")?;need(completed.len()<=8,"Completed profile history exceeded bound")?;
 for p in completed {target(p)?;definition(p["profile"].as_str().ok_or("Missing completed profile")?)?;bounded(p,"seed",65535)?;integer(p,"generation")?;need(integer(p,"stateRevision")?<=revision,"Completed profile revision is in future")?;need(array(p,"telemetry")?.len()==4,"Completed profile has incomplete lane summaries")?;}
 let transitions=array(n,"transitions")?;need(transitions.len()<=16,"Transition history exceeded bound")?;
 let mut last=0;for t in transitions {target(t)?;let r=integer(t,"stateRevision")?;need(r>last&&r<=revision,"Transition revisions are unordered/in future")?;last=r;need(["profile-enabled","profile-replaced","profile-reset","profile-expired","fixture-close","http-cut","wss-cut"].contains(&t["reason"].as_str().unwrap_or("")),"Unknown network transition reason")?;}
 Ok(())
}

fn active_identity(response:&Value)->Result<Value,String>{let n=network(response)?;Ok(Value::Array(array(n,"profiles")?.iter().map(|p|json!({"target":p["target"],"profile":p["profile"],"seed":p["seed"],"generation":p["generation"]})).collect()))}

pub(super) fn steady(before:&Value,after:&Value,expected_target:&str,expected_profile:&str)->Result<(),String>{
 validate(before)?;validate(after)?;let start=network(before)?;let end=network(after)?;
 need(active_identity(before)?==active_identity(after)?,"Profile identity/generation changed inside steady measurement")?;
 need(start["stateRevision"]==end["stateRevision"]&&start["configurations"]==end["configurations"],"Transport/profile transition occurred inside steady measurement")?;
 let profiles=array(end,"profiles")?;need(profiles.len()==1&&profiles[0]["target"]==expected_target&&profiles[0]["profile"]==expected_profile&&integer(&profiles[0],"remainingMs")?>0,"Requested target/profile is not the active measured profile")?;
 need(end["rawIngress"]["overflow"]==0&&start["rawIngress"]["overflow"]==0,"Raw ingress overflow occurred")?;
 need(start["rawIngress"]["clearedFrames"]==end["rawIngress"]["clearedFrames"],"Raw ingress was cleared during eligible steady play")?;
 for row in array(end,"telemetry")? {
  let t=target(row)?;let l=lane(row)?;let old=lane_row(start,t,l)?;
  for key in ["scheduled","delivered","cancelled","overflow","late"]{need(integer(row,key)?>=integer(old,key)?,"Telemetry counter regressed inside profile")?;}
  if t!=expected_target {for key in ["scheduled","delivered","cancelled","overflow","late"]{need(row[key]==old[key],"Unselected target was affected by profile scheduling")?;}continue}
  need(row["overflow"]==0&&row["late"]==0,"Profile overflow/residence cancellation occurred")?;
  need(row["cancelled"]==old["cancelled"],"Unexpected scheduled transport cancellation during steady profile")?;
 }
 Ok(())
}

pub(super) fn phase(before:&Value,after:&Value,expected_target:&str,expected_profile:&str)->Result<Value,String>{
 steady(before,after,expected_target,expected_profile)?;let start=network(before)?;let end=network(after)?;
 let profiles=array(end,"profiles")?;let mut measured=Vec::new();
 for row in array(end,"telemetry")? {
  let t=target(row)?;let l=lane(row)?;if t!=expected_target{continue}let old=lane_row(start,t,l)?;
  let delivered=integer(row,"delivered")?-integer(old,"delivered")?;need(delivered>0,"No measured delivery in required HTTP/WSS direction")?;
  let recent=array(row,"samples")?;let count=(delivered as usize).min(recent.len());let tail=&recent[recent.len()-count..];
  let mut sample_distributions=serde_json::Map::new();for key in SAMPLE_FIELDS{sample_distributions.insert(key.into(),timings(&tail.iter().map(|s|number(s,key)).collect::<Result<Vec<_>,_>>()?));}
  let mut means=serde_json::Map::new();for key in ["actualDelaySumMs","classificationSumMs","rawQueueSumMs","preScheduleSumMs","propagationWaitSumMs"]{let delta=number(row,key)?-number(old,key)?;need(delta>=-0.001,"Timing sum regressed inside profile")?;means.insert(key.into(),json!(delta/delivered as f64));}
  measured.push(json!({"target":t,"lane":l,"delivered_in_phase":delivered,"sample_scope":"last min(delivered_in_phase,64) deliveries from this phase; bounded tail, not full-phase percentiles","sample_distributions":sample_distributions,"phase_mean_ms_from_cumulative_sum_differences":means,"cancelled_in_phase":0}));
 }
 need(measured.len()==4,"All four application directions must be measured")?;
 Ok(json!({"semantics":SEMANTICS,"requested_profile":expected_profile,"target":expected_target,"generation":profiles[0]["generation"],"lanes":measured,
  "timing_interpretation":"Ingress minimum arrival-to-release overlaps identity SQL/raw queue time; remaining propagation wait is separate. Requested RTT is not guaranteed additive end-to-end RTT.",
  "scheduler_high_water_cumulative":end["highWater"],"raw_ingress_high_water_cumulative":end["rawIngress"]["highWater"],"injection_payload_upper_bound":end["injectionBufferUpperBound"],
  "buffer_interpretation":"Separate scheduled/raw high-water values may peak at different times. Combined current payload is a conservative bound with shared heads counted twice; excludes application/TLS/kernel buffers and non-payload heap."}))
}

pub(super) fn transition(before:&Value,after:&Value,t:&str,name:Option<&str>,seed:Option<u16>)->Result<(),String>{
 validate(before)?;validate(after)?;let start=network(before)?;let end=network(after)?;
 need(integer(end,"stateRevision")?>integer(start,"stateRevision")?,"Profile transition revision did not advance")?;
 let profiles=array(end,"profiles")?;
 match name {Some(name)=>{let p=profiles.iter().find(|p|p["target"]==t).ok_or("Configured target profile missing")?;need(p["profile"]==name&&p["seed"]==json!(seed.ok_or("Missing expected seed")?)&&integer(p,"remainingMs")?>0,"Configured profile/seed differs")?;},None=>{need(!profiles.iter().any(|p|p["target"]==t),"Reset target profile remains active")?;}}
 let others=|n:&Value|->Result<Vec<Value>,String>{Ok(array(n,"profiles")?.iter().filter(|p|p["target"]!=t).map(|p|json!({"target":p["target"],"profile":p["profile"],"seed":p["seed"],"generation":p["generation"]})).collect())};
 need(others(start)?==others(end)?,"Targeted profile change altered other target profile identity")?;
 Ok(())
}

// Synthetic telemetry only: no app/service/network acceptance is inferred.
fn test_status(delivered:u64)->Value {
 let sample=json!({"plannedMs":40,"actualMs":50,"clampedMs":40,"classificationMs":45,"rawQueueMs":5,"preScheduleMs":50,"remainingPropagationWaitMs":0,"timerLatenessMs":0});
 let rows:Vec<Value>=LANES.iter().map(|l|json!({"target":"consumer","lane":l,"scheduled":delivered,"delivered":delivered,"cancelled":0,"overflow":0,"late":0,"maxQueuedFrames":1,"maxQueuedBytes":1,"actualDelaySumMs":50*delivered,"actualDelayMaxMs":if delivered>0{50}else{0},"timerLatenessMaxMs":0,"classificationSumMs":45*delivered,"rawQueueSumMs":5*delivered,"preScheduleSumMs":50*delivered,"propagationWaitSumMs":0,"samples":if delivered>0{vec![sample.clone()]}else{vec![]},"meanActualMs":if delivered>0{50}else{0},"meanClassificationMs":if delivered>0{45}else{0},"meanRawQueueMs":if delivered>0{5}else{0},"meanRemainingPropagationWaitMs":0})).collect();
 json!({"network":{"revision":1,"stateRevision":1,"semantics":SEMANTICS,
  "limits":{"profiles":2,"configurations":64,"durationMinMs":1000,"durationMaxMs":600000,"defaultDurationMs":120000,"flows":128,"frames":256,"bytes":4194304,"wssFrames":64,"wssBytes":65536,"httpFrames":1,"httpBytes":262144,"maxResidenceMs":4000,"samples":64,"completedProfiles":8,"transitions":16},
  "definitions":{"rtt80":{"rttMs":80,"oneWayMs":40,"jitterMs":8},"rtt200":{"rttMs":200,"oneWayMs":100,"jitterMs":20},"rtt400":{"rttMs":400,"oneWayMs":200,"jitterMs":40}},"configurations":1,
  "profiles":[{"target":"consumer","profile":"rtt80","seed":0,"generation":1,"remainingMs":120000}],
  "pending":{"flows":0,"frames":0,"bytes":0,"timers":1},"highWater":{"flows":4,"frames":4,"bytes":4,"timers":5},
  "completed":[],"transitions":[{"target":"consumer","reason":"profile-enabled","stateRevision":1}],"telemetry":rows,
  "rawIngress":{"limits":{"frames":64,"bytes":65536,"aggregateFrames":4096,"aggregateBytes":4194304},"pending":{"frames":0,"bytes":0},"highWater":{"frames":0,"bytes":0,"maxSocketFrames":0,"maxSocketBytes":0},"overflow":0,"clearedFrames":0,"byTarget":{"consumer":{"frames":0,"bytes":0},"peer":{"frames":0,"bytes":0},"unclassified":{"frames":0,"bytes":0}}},
  "injectionBufferUpperBound":{"currentPayloadBytes":0,"limitPayloadBytes":8388608,"sharedIngressHeadsDoubleCounted":true,"excludes":"application/TLS/kernel buffers and non-payload heap"}}})
}

#[test]fn lc1_network_telemetry_accepts_overlapping_lookup_without_false_additive_delay(){
 let before=test_status(0);let after=test_status(1);let measured=phase(&before,&after,"consumer","rtt80").unwrap();
 assert_eq!(measured["lanes"][0]["sample_distributions"]["classificationMs"]["p95_ms"],45.0);
 assert_eq!(measured["lanes"][0]["sample_distributions"]["remainingPropagationWaitMs"]["p95_ms"],0.0);
 assert_eq!(measured["lanes"][0]["phase_mean_ms_from_cumulative_sum_differences"]["actualDelaySumMs"],50.0);
 assert_eq!(measured["injection_payload_upper_bound"]["sharedIngressHeadsDoubleCounted"],true);
}

#[test]fn lc1_network_telemetry_refuses_unbounded_missing_misattributed_and_cancelled_evidence(){
 let before=test_status(0);
 for fault in 0..12 {
  let mut after=test_status(1);let n=&mut after["network"];
  match fault {
   0=>{n.as_object_mut().unwrap().remove("rawIngress");},
   1=>n["injectionBufferUpperBound"]["limitPayloadBytes"]=json!(4194304),
   2=>n["rawIngress"]["highWater"]["bytes"]=json!(4194305),
   3=>n["telemetry"][0]["samples"][0]["remainingPropagationWaitMs"]=json!(40),
   4=>{n["telemetry"][0]["scheduled"]=json!(2);n["telemetry"][0]["cancelled"]=json!(1);},
   5=>n["profiles"][0]["generation"]=json!(2),
   6=>n["rawIngress"]["byTarget"]["peer"]["frames"]=json!(1),
   7=>{n["telemetry"].as_array_mut().unwrap().pop();},
   8=>n["telemetry"][0]["delivered"]=json!("unknown"),
   9=>n["rawIngress"]["clearedFrames"]=json!(1),
   10=>{n["telemetry"][0]["samples"][0].as_object_mut().unwrap().remove("timerLatenessMs");},
   _=>n["semantics"]=json!("additional-RTT-on-top-of-SQL"),
  }
  assert!(phase(&before,&after,"consumer","rtt80").is_err(),"Invalid telemetry fault{fault} accepted");
 }
 assert!(phase(&before,&test_status(1),"peer","rtt80").is_err());
}

#[test]fn lc1_network_telemetry_accepts_reviewed_real_service_receipts_without_rerunning_service(){
 let path="docs/g3/server-lc1-network-profile-checks.json";
 verify_fixture_pin(path,"dc370a2b941bc419679481b5661a12319cdc74f1236a736cc16d5adb33ce74d1").unwrap();
 let checks:Value=serde_json::from_slice(&std::fs::read(repo().join(path)).unwrap()).unwrap();
 let phases=checks["actualAudit"]["phases"].as_array().unwrap();assert_eq!(phases.len(),3);
 for p in phases{validate(&json!({"network":p["network"]})).unwrap_or_else(|e|panic!("Reviewed {} telemetry rejected: {e}",p["profile"]));}
 validate(&json!({"network":checks["actualAudit"]["final"]})).unwrap();
 // This checks saved Server evidence only. No native/app/WAN flow was run.
}
