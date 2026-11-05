// Client-owned network-profile observation. Server hooks are wired only after
// Root routes their settled interface. These helpers grant no authority.
mod telemetry {use super::*;include!("next_lc1_network_telemetry.rs");}
include!("next_lc1_network_transport_watch.rs");
include!("next_lc1_network_clock.rs");
include!("next_lc1_network_capacity.rs");
include!("next_lc1_network_owner_trace.rs");
include!("next_lc1_network_auth.rs");
include!("next_lc1_scoped_outage_test.rs");
#[derive(Debug,PartialEq)]
struct ProfileFailure {side:usize,reason:&'static str}
struct ProfileProbe {
 expected:[Value;2],before:[Value;2],native_before:[Value;2],native_start:[Value;2],
 floor:[u64;2],sampled:[u64;2],ages:[Vec<u64>;2],window_ms:u64,last_ms:u64,samples:u64,
}
fn counter(v:&Value,path:&str)->u64{v.pointer(path).and_then(Value::as_u64).unwrap_or(0)}
fn peer_sample(view:&Value,expected:&Value)->Option<(u64,u64)>{view["g3"]["fixture_peers"].as_array()?.iter().find(|p|p["session"]==expected["session"]&&p["world"]==expected["world"]).and_then(|p|Some((p["sequence"].as_u64()?,p["age_ms"].as_u64()?)))}
fn distribution(samples:&[u64])->Value{if samples.is_empty(){return json!({"count":0})}let mut sorted=samples.to_vec();sorted.sort_unstable();let q=|p:usize|sorted[((sorted.len()*p+99)/100).saturating_sub(1).min(sorted.len()-1)];json!({"count":sorted.len(),"min_ms":sorted[0],"p50_ms":q(50),"p95_ms":q(95),"p99_ms":q(99),"max_ms":sorted[sorted.len()-1]})}

fn write_row(root: &std::path::Path, name: &str, row: &Value) {
 let mut output=std::fs::OpenOptions::new().create(true).append(true).open(root.join(name)).unwrap();
 writeln!(output,"{row}").unwrap();
}

fn status(service:&mut Service,root:&std::path::Path)->Value {
 let mut result=service.command(json!({"op":"networkStatus"}));
 let clock=service.command(json!({"op":"clockStatus"}));clock_shape(&clock["clock"],"elapsed").unwrap();result["clock"]=clock["clock"].clone();
 let capacity=service.command(json!({"op":"liveCapacityStatus"}));capacity_shape(&capacity["liveCapacity"]).unwrap();result["liveCapacity"]=capacity["liveCapacity"].clone();
 let auth=service.command(json!({"op":"liveAuthStatus"}));auth_shape(&auth["liveAuth"]).unwrap();result["liveAuth"]=auth["liveAuth"].clone();
 if let Err(reason)=telemetry::validate(&result){
  let path=root.join("network-telemetry-first-loss.json");
  if !path.exists(){std::fs::write(&path,serde_json::to_vec_pretty(&json!({"reason":reason,"network":result,"phase_context":std::fs::read(root.join("network-profile-context.json")).ok().and_then(|b|serde_json::from_slice::<Value>(&b).ok())})).unwrap()).unwrap();}
  panic!("Network telemetry failed reviewed schema/bounds: {reason}; {}",path.display());
 }
 result
}

// This narrow guard applies while only movement transport is interrupted.
// Canonical expiry uses the existing separate protective-outage probe.
fn permission_failure(views:&[Value;2],native:&[Value;2],before:&[Value;2])->Option<ProfileFailure>{
 for i in 0..2 {
  if native[i]["protected"]!=false||native[i]["native_status"]!=2||native[i]["holds"]!=before[i]["holds"]||native[i]["native_context"]!=before[i]["native_context"] {
   return Some(ProfileFailure{side:i,reason:"Transport transition revoked actual native permission or replaced its context"});
  }
  if counter(&native[i],"/frame").saturating_sub(counter(&native[i],"/native_status_frame"))>1 {
   return Some(ProfileFailure{side:i,reason:"Transport transition has no current wrapper status"});
  }
  if views[i]["g3"]["live_transport"]["grant_active"]!=true {
   return Some(ProfileFailure{side:i,reason:"Transport transition revoked current movement authority"});
  }
 }
 None
}

struct PermissionProbe {initial:[Value;2],previous:[Value;2],window_ms:u64}
impl PermissionProbe {
 fn new(native:[Value;2])->Self{Self{initial:native.clone(),previous:native,window_ms:0}}
 fn observe(&mut self,elapsed_ms:u64,views:&[Value;2],native:&[Value;2])->Option<ProfileFailure>{
  if let Some(failure)=permission_failure(views,native,&self.initial){return Some(failure)}
  if elapsed_ms.saturating_sub(self.window_ms)>=2000{
   for i in 0..2{if counter(&native[i],"/frame")<=counter(&self.previous[i],"/frame")||counter(&native[i],"/hud_button_frames")<=counter(&self.previous[i],"/hud_button_frames"){return Some(ProfileFailure{side:i,reason:"Transport interruption stalled actual callbacks/button rendering in2s window"})}}
   self.previous=native.clone();self.window_ms=elapsed_ms;
  }
  None
 }
}

async fn first_failure(service:&mut Service,a:&Client,na:&Native,b:&Client,nb:&Native,label:&str,failure:ProfileFailure,previous:&Value)->! {
 let root=a.root.parent().unwrap();
 let path=root.join("network-first-loss.json");
 let evidence=json!({"phase":label,"side":failure.side,"reason":failure.reason,"previous":previous,
  "current_app":[a.handle.view(),b.handle.view()],"current_native":[native_activity(na),native_activity(nb)],
  "current_actor":[outage_state(a).await,outage_state(b).await],
  "network":service.command(json!({"op":"networkStatus"})),"authority_deadlines_changed":false});
 if !path.exists(){std::fs::write(&path,serde_json::to_vec_pretty(&evidence).unwrap()).unwrap();}
 for client in [a,b]{boundary_snapshot(client,"FIRST_NETWORK_PROFILE_LOSS");}
 panic!("{}: {}; {}",label,failure.reason,path.display());
}

// New WAN helper: requires distinct current-session poses and actual render
// progress inside the existing protocol TTL. The local500ms helper is intact.
async fn fresh_profile_pair(service:&mut Service,a:&Client,na:&Native,b:&Client,nb:&Native,label:&str,preserve_permission:bool){
 let expected=[ghost_identity(b).await,ghost_identity(a).await];
 for native in [na,nb]{
  std::fs::write(native.root.join("foreground-monitor"),b"actual callbacks, Controls and native ghost motion").unwrap();
  std::fs::write(native.root.join("move-source"),b"continued injected engine movement under network profile").unwrap();
 }
 wait(service,a,"profile native observation available",5,|_|[na,nb].iter().all(|n|n.root.join("activity.json").is_file())).await;
 let before=[a.handle.view(),b.handle.view()];let native_before=[native_activity(na),native_activity(nb)];
 let floors:[u64;2]=std::array::from_fn(|i|peer_sample(&before[i],&expected[i]).map(|p|p.0).unwrap_or(0));
 let began=Instant::now();let mut next=Instant::now();let mut continuity=PermissionProbe::new(native_before.clone());
 loop {
  service.wall_clock();let views=[a.handle.view(),b.handle.view()];let activity=[native_activity(na),native_activity(nb)];
  if preserve_permission {if let Some(failure)=continuity.observe(began.elapsed().as_millis() as u64,&views,&activity){first_failure(service,a,na,b,nb,label,failure,&json!({"app":before,"native":native_before})).await;}}
  let fresh=(0..2).all(|i|views[i]["connection"]["state"]=="Playing"&&views[i]["g3"]["control"]["current"]["current_validated"]==true&&views[i]["g3"]["live_transport"]["grant_active"]==true&&activity[i]["protected"]==false&&activity[i]["native_status"]==2&&counter(&activity[i],"/frame").saturating_sub(counter(&activity[i],"/native_status_frame"))<=1&&counter(&views[i],"/g3/movement/publications")>counter(&before[i],"/g3/movement/publications")&&counter(&views[i],"/g3/live_transport/nonempty_snapshots")>counter(&before[i],"/g3/live_transport/nonempty_snapshots")&&counter(&activity[i],"/ghost_motion_updates")>counter(&native_before[i],"/ghost_motion_updates")&&counter(&activity[i],"/ghosts")>0&&peer_sample(&views[i],&expected[i]).is_some_and(|(seq,age)|seq>floors[i]&&age<2000));
  if fresh {write_row(a.root.parent().unwrap(),"network-settling.jsonl",&json!({"phase":label,"elapsed_ms":began.elapsed().as_millis(),"expected":expected,"app":views,"native":activity,"pose_age_bound_ms":2000,"baseline500ms_unchanged":true}));return}
  if began.elapsed()>Duration::from_secs(20){first_failure(service,a,na,b,nb,label,ProfileFailure{side:0,reason:"Fresh bidirectional current-session profile movement did not settle"},&json!({"app":before,"native":native_before,"expected":expected})).await;}
  if Instant::now()>=next{for client in [a,b]{boundary_snapshot(client,label);recovery_owner_trace(client,label).await;}next=Instant::now()+Duration::from_secs(2);}
  tokio::time::sleep(Duration::from_millis(100)).await;
 }
}

async fn measured_phase(service:&mut Service,a:&Client,na:&Native,b:&Client,nb:&Native,label:&str,frame_ms:u32,stall_optional:bool,target:&str,profile:&str)->Value {
 assert!([17,33].contains(&frame_ms));
 for native in [na,nb]{std::fs::write(native.root.join("frame-ms"),frame_ms.to_string()).unwrap();}
 fresh_profile_pair(service,a,na,b,nb,label,true).await;
 let expected=[ghost_identity(b).await,ghost_identity(a).await];
 let views=[a.handle.view(),b.handle.view()];let native=[native_activity(na),native_activity(nb)];
 let mut probe=ProfileProbe::new(expected,views.clone(),native.clone());
 let before_status=status(service,a.root.parent().unwrap());
 if stall_optional{service.command(json!({"op":"optionalStall","ms":5000}));}
 let began=Instant::now();let mut next=Instant::now();let mut previous=json!({"app":views,"native":native});
 while began.elapsed()<Duration::from_secs(36){
  service.wall_clock();let views=[a.handle.view(),b.handle.view()];let activity=[native_activity(na),native_activity(nb)];
  if let Err(failure)=probe.observe(began.elapsed().as_millis() as u64,&views,&activity){first_failure(service,a,na,b,nb,label,failure,&previous).await;}
  previous=json!({"app":views,"native":activity});
  if Instant::now()>=next{
   let current_status=status(service,a.root.parent().unwrap());
   let context=json!({"phase":label,"elapsed_ms":began.elapsed().as_millis(),"app":views,"native":activity,"network":current_status});
   std::fs::write(a.root.parent().unwrap().join("network-profile-context.json"),serde_json::to_vec_pretty(&context).unwrap()).unwrap();
   if let Err(reason)=telemetry::steady(&before_status,&current_status,target,profile){write_row(a.root.parent().unwrap(),"network-telemetry-errors.jsonl",&json!({"phase":label,"reason":reason,"network":current_status}));first_failure(service,a,na,b,nb,label,ProfileFailure{side:0,reason:"Steady profile telemetry changed/cancelled/overflowed"},&previous).await;}
   write_row(a.root.parent().unwrap(),"network-progress.jsonl",&context);next=Instant::now()+Duration::from_secs(2);
  }
  tokio::time::sleep(Duration::from_millis(100)).await;
 }
 // Check the final partial window as well as all completed2s windows.
 let end=[native_activity(na),native_activity(nb)];let views=[a.handle.view(),b.handle.view()];
 if let Err(failure)=probe.observe(began.elapsed().as_millis() as u64,&views,&end){first_failure(service,a,na,b,nb,label,failure,&previous).await;}
 if stall_optional{service.command(json!({"op":"optionalStall","ms":0}));}
 let after_status=status(service,a.root.parent().unwrap());
 let measured_clock=clock_phase(&before_status["clock"],&after_status["clock"]).unwrap();
 let measured_capacity=capacity_phase(&before_status["liveCapacity"],&after_status["liveCapacity"]).unwrap();
 let measured_auth=auth_phase(&before_status["liveAuth"],&after_status["liveAuth"]).unwrap();
 let achieved=match telemetry::phase(&before_status,&after_status,target,profile){Ok(v)=>v,Err(reason)=>{write_row(a.root.parent().unwrap(),"network-telemetry-errors.jsonl",&json!({"phase":label,"reason":reason,"network":after_status}));first_failure(service,a,na,b,nb,label,ProfileFailure{side:0,reason:"Required measured network delivery or isolation evidence missing"},&previous).await;}};
 let summary=json!({"phase":label,"target_fps":if frame_ms==17{60}else{30},"optional_stall_ms":if stall_optional{5000}else{0},
  "measurements":probe.summary(&end),"achieved_network":achieved,"clock":measured_clock,"capacity":measured_capacity,"authentication":measured_auth,"network_before":before_status,"network_after":after_status,
  "app_metrics_scope":"App counters/latency metrics are cumulative unless explicitly differenced; no phase-specific latency claim"});
 write_row(a.root.parent().unwrap(),"network-phases.jsonl",&summary);checkpoint(a.root.parent().unwrap(),label);summary
}

async fn scoped_wss_cut(service:&mut Service,a:&Client,na:&Native,b:&Client,nb:&Native){
 let identities=[ghost_identity(a).await,ghost_identity(b).await];
 let native=[native_activity(na),native_activity(nb)];
 let journal=[std::fs::read(a.root.join("transfers.jsonl")).unwrap(),std::fs::read(b.root.join("transfers.jsonl")).unwrap()];
 let before_status=status(service,a.root.parent().unwrap());
 service.command(json!({"op":"transport","kind":"wss","enabled":true,"target":"consumer"}));
 let began=Instant::now();let mut previous=json!({"native":native});let mut continuity=PermissionProbe::new(native.clone());
 while began.elapsed()<Duration::from_secs(35){
  service.wall_clock();let views=[a.handle.view(),b.handle.view()];let activity=[native_activity(na),native_activity(nb)];
  if let Some(failure)=continuity.observe(began.elapsed().as_millis() as u64,&views,&activity){first_failure(service,a,na,b,nb,"400ms profile WSS-only cut",failure,&previous).await;}
  for (i,client) in [a,b].into_iter().enumerate(){assert_eq!(ghost_identity(client).await,identities[i]);assert_eq!(std::fs::read(client.root.join("transfers.jsonl")).unwrap(),journal[i]);}
  previous=json!({"app":views,"native":activity});tokio::time::sleep(Duration::from_millis(100)).await;
 }
 let before_restore=status(service,a.root.parent().unwrap());
 service.command(json!({"op":"transport","kind":"wss","enabled":false,"target":"consumer"}));
 fresh_profile_pair(service,a,na,b,nb,"400ms profile WSS-only reconnect",true).await;
 write_row(a.root.parent().unwrap(),"network-disconnects.jsonl",&json!({"kind":"WSS-only","elapsed_ms":began.elapsed().as_millis(),"identity_unchanged":true,"journal_unchanged":true,"network_before":before_status,"network_before_restore":before_restore,"network_after":status(service,a.root.parent().unwrap())}));
 checkpoint(a.root.parent().unwrap(),"400ms-profile-35s-WSS-only-loss-keeps-current-Controls-and-fresh-reconnect");
}

// Reviewed application transport simulation, with explicit nonseamless switch.
async fn change_profile(service:&mut Service,a:&Client,na:&Native,b:&Client,nb:&Native,target:&str,profile:&str,seed:u16){
 assert!(["consumer","peer"].contains(&target));assert!(["rtt80","rtt200","rtt400"].contains(&profile));
 let before=status(service,a.root.parent().unwrap());
 let identities=[ghost_identity(a).await,ghost_identity(b).await];
 let command=json!({"op":"networkProfile","target":target,"profile":profile,"seed":seed,"durationMs":600000});
 let replacing=before["network"]["profiles"].as_array().unwrap().iter().any(|p|p["target"]==target);
 let label=if replacing{format!("intentional {target} replacement with {profile}; target cancellation/close then fresh settlement")}else{format!("enable {target} {profile} for future arrivals; settle fresh traffic across generation boundary")};
 let context=json!({"phase":label,"command":command,"status_before":before,"identity":identities,"app_before":[a.handle.view(),b.handle.view()],"native_before":[native_activity(na),native_activity(nb)]});
 std::fs::write(a.root.parent().unwrap().join("network-profile-context.json"),serde_json::to_vec_pretty(&context).unwrap()).unwrap();
 let armed=service.command(command);
 telemetry::transition(&before,&armed,target,Some(profile),Some(seed)).unwrap();
 fresh_profile_pair(service,a,na,b,nb,&label,true).await;
 for (i,client) in [a,b].into_iter().enumerate(){assert_eq!(ghost_identity(client).await,identities[i],"A profile transition changed canonical/native ownership");}
 let settled=status(service,a.root.parent().unwrap());telemetry::transition(&before,&settled,target,Some(profile),Some(seed)).unwrap();
 write_row(a.root.parent().unwrap(),"network-transitions.jsonl",&json!({"phase":label,"status_before":before,"armed":armed,"status_settled":settled,"identity_unchanged":true,"replacement_cancel_and_close_semantics":replacing,"actual_socket_close_claimed":false}));
}

#[test]
#[ignore="Root serialized network-profile composition; requires reviewed settled Server hooks and pinned readiness"]
fn lc1_network_normal_scheduler_two_peer_recovery_probe(){run_network_probe(false);}
#[test]
#[ignore="Root serialized affected400ms actual native reopen; requires current reviewed readiness"]
fn lc1_network_reopen_400_current_owner_probe(){run_network_probe(true);}
fn run_network_probe(focused_reopen:bool){
 std::thread::Builder::new().name("lc1-network-profile".into()).stack_size(16*1024*1024).spawn(move||tauri::async_runtime::block_on(async move {
  let own=repo().join("spikes/noita-entry/.build/lc1-network-client-20261020-11").join(format!("network-profile-{}",uuid::Uuid::new_v4()));
  std::fs::create_dir_all(&own).unwrap();eprintln!("LC1_NETWORK_PROFILE {}",own.display());
  std::fs::write(own.join("probe-scope.json"),serde_json::to_vec_pretty(&json!({"focused_reopen":focused_reopen,"accepted_prior_phases_not_repeated":focused_reopen,"no_shared_execution_by_preparation":true})).unwrap()).unwrap();
  std::fs::write(own.join("capture-initial-foreground"),b"first actual Controls loss after initial entry").unwrap();
  let mut service=Service::start(&own);enable_elapsed_clock(&mut service,&own);let private=service.private();let router=PipeHarness::new();
  let mut a=Client::start(&private,&own.join("consumer"),None,false,true,router.routes.clone(),service.watches.clone(),service.stores.clone()).await;
  bundle(&mut service,&a).await;let mut first=Native::start(&a,None,false);
  wait(&mut service,&a,"network consumer Ready",30,|v|v["connection"]["state"]=="Ready"&&first.root.join("associated").is_file()).await;
  let world=a.handle.view()["rejoin"]["current_world"].clone();
  service.command(json!({"op":"peer"}));let private=service.private();
  let mut b=Client::start(&private,&own.join("peer"),None,true,true,router.routes.clone(),service.watches.clone(),service.stores.clone()).await;
  bundle(&mut service,&b).await;let mut nb=Native::start(&b,None,false);
  wait(&mut service,&b,"network peer Ready",30,|v|v["connection"]["state"]=="Ready"&&nb.root.join("associated").is_file()).await;
  let peer_world=b.handle.view()["rejoin"]["current_world"].clone();
  two_peer_ghost(&mut service,&a,&first,&world,&b,&nb,&peer_world).await;
  std::fs::write(nb.root.join("unaffected-check"),b"continuously eligible healthy peer").unwrap();
  for native in [&first,&nb]{std::fs::write(native.root.join("foreground-monitor"),b"actual profile cadence/control/ghost counters").unwrap();std::fs::write(native.root.join("frame-ms"),b"17").unwrap();}
  fresh_bidirectional_ghosts(&mut service,&a,&first,&b,&nb,"unimpaired baseline before any network profile").await;
  let watch=TransportWatch::start(&a,&first,&b,&nb);
  let healthy_identity=ghost_identity(&b).await;
  let profiles:Vec<&str>=if focused_reopen{vec!["rtt400"]}else{vec!["rtt80","rtt200","rtt400"]};
  for profile in profiles{
   let index=match profile{"rtt80"=>0,"rtt200"=>1,_=>2};
   watch.mode(&format!("explicit consumer profile transition {profile}"),[true,false]);
   change_profile(&mut service,&a,&first,&b,&nb,"consumer",profile,17920+index as u16).await;
   watch.mode(&format!("eligible {profile} after real settlement"),[false,false]);
   if focused_reopen{for native in [&first,&nb]{std::fs::write(native.root.join("frame-ms"),b"33").unwrap();}fresh_profile_pair(&mut service,&a,&first,&b,&nb,"400ms moving pair before focused reopen; prior steady phases not repeated",true).await;}
   let cadences:Vec<u32>=if focused_reopen{vec![]}else{vec![17,33]};
   for frame_ms in cadences{let label=format!("{profile} consumer foreground nominal{}fps",if frame_ms==17{60}else{30});watch.mode(&label,[false,false]);measured_phase(&mut service,&a,&first,&b,&nb,&label,frame_ms,frame_ms==33,"consumer",profile).await;}
   assert_eq!(ghost_identity(&b).await,healthy_identity);
  }
  watch.mode("explicit consumer native close/reopen",[true,false]);
  recovery_owner_trace(&a,"BEFORE actual consumer native close").await;recovery_owner_trace(&b,"healthy peer BEFORE consumer close").await;
  let old=ghost_identity(&a).await;first.close();let saved=first.root.clone();
  let mut resumed=Native::start(&a,Some(&saved),true);
  watch.native(0,&resumed);
  recovery_owner_trace(&a,"AFTER new native launch BEFORE current-owner reconciliation").await;
  std::fs::write(resumed.root.join("foreground-monitor"),b"native reopen under retained400ms profile").unwrap();std::fs::write(resumed.root.join("frame-ms"),b"17").unwrap();
  ready_owner(&mut service,&a,&resumed,&world,true).await;
  recovery_owner_trace(&a,"AFTER current reopened owner Playing BEFORE retired-context check").await;recovery_owner_trace(&b,"healthy peer at new current-owner Playing").await;
  let reopened=ghost_identity(&a).await;assert_ne!(reopened["context"],old["context"]);assert_ne!(reopened["pid"],old["pid"]);assert_eq!(reopened["world"],old["world"]);
  std::fs::write(resumed.root.join("retired-context.hex"),old["context"].as_str().unwrap()).unwrap();
  wait(&mut service,&a,"network retired native context refused",5,|_|resumed.root.join("retired-context-refused").is_file()).await;
  fresh_profile_pair(&mut service,&a,&resumed,&b,&nb,"400ms native close/reopen retained app fresh both-direction poses/rendering",false).await;
  watch.mode("eligible400ms after actual native reopen",[false,false]);
  assert_eq!(ghost_identity(&b).await,healthy_identity);
  measured_phase(&mut service,&a,&resumed,&b,&nb,"400ms foreground after actual native reopen",17,false,"consumer","rtt400").await;
  if focused_reopen{
   recovery_owner_trace(&a,"PASSED current reopened owner fresh movement/rendering and36s foreground").await;
   watch.mode("explicit focused consumer profile reset/reconnect",[true,false]);let before=status(&mut service,&own);let reset=service.command(json!({"op":"networkReset","target":"consumer"}));telemetry::transition(&before,&reset,"consumer",None,None).unwrap();
   fresh_profile_pair(&mut service,&a,&resumed,&b,&nb,"focused400ms recovery complete profile reset",true).await;watch.mode("eligible focused recovery complete",[false,false]);checkpoint(&own,"focused400ms-current-owner-reopen-bidirectional-recovery-and-foreground-complete");watch.close().await;
   resumed.close();nb.close();a.close().await;b.close().await;service.close();return
  }
  watch.mode("explicit consumer WSS-only cut/reconnect",[true,false]);scoped_wss_cut(&mut service,&a,&resumed,&b,&nb).await;
  watch.mode("eligible400ms after WSS-only reconnect",[false,false]);
  measured_phase(&mut service,&a,&resumed,&b,&nb,"400ms foreground after WSS-only reconnect",33,true,"consumer","rtt400").await;
  let prior=ghost_identity(&a).await;
  write_row(&own,"network-disconnects.jsonl",&json!({"kind":"HTTP+WSS-before35s","network":status(&mut service,&own)}));
  watch.mode("explicit consumer full outage and same-context canonical recovery",[true,false]);
  same_context_cut(&mut service,&a,&resumed,&b,&nb,&prior).await;
  ready_owner(&mut service,&a,&resumed,&world,true).await;
  std::fs::write(resumed.root.join("capture-first-loss"),b"rearm after intentional genuine canonical expiry").unwrap();
  let replaced=ghost_identity(&a).await;
  for field in ["context","pid","world"]{assert_eq!(replaced[field],prior[field]);}
  assert!(replaced["canonical_generation"].as_u64().unwrap()>prior["canonical_generation"].as_u64().unwrap());
  for field in ["session","session_epoch","attachment"]{assert_ne!(replaced[field],prior[field]);}
  fresh_profile_pair(&mut service,&a,&resumed,&b,&nb,"400ms same-context higher canonical owner fresh both-direction poses/rendering",false).await;
  watch.mode("eligible400ms after same-context canonical replacement",[false,false]);
  assert_eq!(ghost_identity(&b).await,healthy_identity);
  measured_phase(&mut service,&a,&resumed,&b,&nb,"400ms foreground after same-context canonical replacement",17,false,"consumer","rtt400").await;
  // The separate peer target must work without changing either identity.
  watch.mode("explicit consumer profile reset/reconnect",[true,false]);
  let before_reset=status(&mut service,&own);let reset=service.command(json!({"op":"networkReset","target":"consumer"}));telemetry::transition(&before_reset,&reset,"consumer",None,None).unwrap();
  fresh_profile_pair(&mut service,&a,&resumed,&b,&nb,"consumer profile reset intentional reconnect",true).await;
  write_row(&own,"network-transitions.jsonl",&json!({"phase":"consumer reset","status_before":before_reset,"status_settled":status(&mut service,&own)}));
  watch.mode("eligible after consumer profile reset",[false,false]);
  watch.mode("explicit peer profile enable/settlement",[false,true]);
  change_profile(&mut service,&a,&resumed,&b,&nb,"peer","rtt200",17924).await;
  watch.mode("eligible peer200ms phase",[false,false]);
  measured_phase(&mut service,&a,&resumed,&b,&nb,"200ms peer-target foreground nominal30fps",33,false,"peer","rtt200").await;
  watch.mode("explicit all-target profile reset/reconnect",[true,true]);
  let before_reset=status(&mut service,&own);let reset=service.command(json!({"op":"networkReset"}));telemetry::transition(&before_reset,&reset,"peer",None,None).unwrap();assert!(reset["network"]["profiles"].as_array().unwrap().is_empty());
  fresh_profile_pair(&mut service,&a,&resumed,&b,&nb,"all profiles reset intentional reconnect",true).await;
  write_row(&own,"network-transitions.jsonl",&json!({"phase":"all profiles reset","status_before":before_reset,"status_settled":status(&mut service,&own)}));
  watch.mode("eligible after all profiles reset",[false,false]);
  checkpoint(&own,"all-network-phases-protection-recovery-and-profile-resets-complete");
  watch.close().await;
  resumed.close();nb.close();a.close().await;b.close().await;service.close();
 })).unwrap().join().unwrap();
}
impl ProfileProbe {
 fn new(expected:[Value;2],views:[Value;2],native:[Value;2])->Self{let floor=std::array::from_fn(|i|peer_sample(&views[i],&expected[i]).map(|p|p.0).unwrap_or(0));Self{expected,before:views,native_before:native.clone(),native_start:native,floor,sampled:floor,ages:[Vec::new(),Vec::new()],window_ms:0,last_ms:0,samples:0}}
 fn observe(&mut self,elapsed_ms:u64,views:&[Value;2],native:&[Value;2])->Result<(),ProfileFailure>{
  if let Some(event)=transport_change(&self.before,views,[false,false]){return Err(ProfileFailure{side:event["side"].as_u64().unwrap_or(0)as usize,reason:"Unexplained live transport event during eligible steady play"})}
  if elapsed_ms<self.last_ms{return Err(ProfileFailure{side:0,reason:"Observer elapsed time regressed"})}self.last_ms=elapsed_ms;self.samples+=1;if self.samples>2048{return Err(ProfileFailure{side:0,reason:"Profile observation sample bound exceeded"})}
  for i in 0..2{let fail=|reason|ProfileFailure{side:i,reason};if views[i]["connection"]["state"]!="Playing"||views[i]["g3"]["control"]["current"]["current_validated"]!=true||views[i]["g3"]["live_transport"]["grant_active"]!=true{return Err(fail("Eligible profile lost current canonical or movement authority"))}if native[i]["protected"]!=false||native[i]["native_status"]!=2||native[i]["holds"]!=self.native_start[i]["holds"]{return Err(fail("Eligible profile lost actual native Controls"))}if native[i]["native_context"]!=self.native_start[i]["native_context"]{return Err(fail("Native context changed inside eligible profile"))}if counter(&native[i],"/frame").saturating_sub(counter(&native[i],"/native_status_frame"))>1{return Err(fail("No current wrapper status sample"))}
   if let Some((sequence,age))=peer_sample(&views[i],&self.expected[i]){if age>=2000{return Err(fail("Peer pose exceeds unchanged protocol2s validity"))}if sequence>self.sampled[i]{if self.ages[i].len()>=1024{return Err(fail("Profile pose-age sample bound exceeded"))}self.sampled[i]=sequence;self.ages[i].push(age);}}
   if elapsed_ms.saturating_sub(self.window_ms)>=2000{if counter(&native[i],"/frame")<=counter(&self.native_before[i],"/frame")||counter(&native[i],"/hud_button_frames")<=counter(&self.native_before[i],"/hud_button_frames")||counter(&native[i],"/ghosts")==0||counter(&native[i],"/ghost_motion_updates")<=counter(&self.native_before[i],"/ghost_motion_updates"){return Err(fail("No fresh callbacks/button rendering/native ghost"))}if counter(&views[i],"/g3/movement/publications")<=counter(&self.before[i],"/g3/movement/publications"){return Err(fail("No fresh profile publication within unchanged2s window"))}if counter(&views[i],"/g3/live_transport/nonempty_snapshots")<=counter(&self.before[i],"/g3/live_transport/nonempty_snapshots"){return Err(fail("No fresh profile peer snapshot within unchanged2s window"))}let Some((sequence,age))=peer_sample(&views[i],&self.expected[i])else{return Err(fail("Current peer session/world absent"))};if sequence<=self.floor[i]||age>=2000{return Err(fail("No advancing current-session peer pose in2s window"))}}
  }
  if elapsed_ms.saturating_sub(self.window_ms)>=2000{self.before=views.clone();self.native_before=native.clone();self.window_ms=elapsed_ms;self.floor=std::array::from_fn(|i|peer_sample(&views[i],&self.expected[i]).unwrap().0);}Ok(())
 }
 fn summary(&self,native:&[Value;2])->Value{let sides:Vec<Value>=(0..2).map(|i|{let elapsed=counter(&native[i],"/tick_ms").saturating_sub(counter(&self.native_start[i],"/tick_ms"));let frames=counter(&native[i],"/frame").saturating_sub(counter(&self.native_start[i],"/frame"));json!({"side":i,"callback_frames":frames,"callback_elapsed_ms":elapsed,"achieved_fps":frames as f64*1000.0/elapsed.max(1)as f64,"accepted_relative_peer_pose_age":distribution(&self.ages[i]),"age_samples":"distinct accepted sequences observed by Client; conservative correlated age, not raw transit RTT","pose_age_bound_ms":2000,"bound_source":"existing protocol pose validity; baseline500ms checks unchanged","ghost_motion_updates":counter(&native[i],"/ghost_motion_updates").saturating_sub(counter(&self.native_start[i],"/ghost_motion_updates")),"hud_button_frames":counter(&native[i],"/hud_button_frames").saturating_sub(counter(&self.native_start[i],"/hud_button_frames")),"hud_content_frames":counter(&native[i],"/hud_content_frames").saturating_sub(counter(&self.native_start[i],"/hud_content_frames")),"authenticated_hud_payload_accepted":false})}).collect();json!({"observer_samples":self.samples,"elapsed_ms":self.last_ms,"progress_window_ms":2000,"authority_deadlines_changed":false,"sides":sides})}
}
#[test]fn lc1_network_observer_requires_current_authority_and_distinct_scoped_pose_progress(){
 let expected=[json!({"session":"peer-b","world":"world-b"}),json!({"session":"peer-a","world":"world-a"})];let views:[Value;2]=std::array::from_fn(|i|json!({"connection":{"state":"Playing"},"g3":{"control":{"current":{"current_validated":true}},"movement":{"publications":10,"live_connections":1,"live_admission_renewal_retries":0,"live_renewal_requests":0,"live_renewals_consumed":0},"movement_failure":{"count":0},"live_transport":{"grant_active":true,"nonempty_snapshots":10,"attempts":1,"failures":0,"last_error":null,"last_receive_failure":null},"fixture_peers":[{"session":expected[i]["session"],"world":expected[i]["world"],"sequence":10,"age_ms":800}]}}));let native:[Value;2]=std::array::from_fn(|_|json!({"frame":100,"native_status_frame":100,"tick_ms":1000,"protected":false,"native_status":2,"holds":0,"ghosts":1,"native_context":"current-native","ghost_motion_updates":10,"hud_button_frames":100,"hud_content_frames":0}));
 let mut next=views.clone();let mut callbacks=native.clone();for i in 0..2{next[i]["g3"]["movement"]["publications"]=json!(11);next[i]["g3"]["live_transport"]["nonempty_snapshots"]=json!(11);next[i]["g3"]["fixture_peers"][0]["sequence"]=json!(11);callbacks[i]["ghost_motion_updates"]=json!(11);callbacks[i]["frame"]=json!(160);callbacks[i]["native_status_frame"]=json!(160);callbacks[i]["tick_ms"]=json!(3000);callbacks[i]["hud_button_frames"]=json!(160);}
 let probe=||ProfileProbe::new(expected.clone(),views.clone(),native.clone());let mut valid=probe();valid.observe(2000,&next,&callbacks).unwrap();assert_eq!(valid.summary(&callbacks)["sides"][0]["accepted_relative_peer_pose_age"]["p95_ms"],800);
 for fault in 0..9{let mut v=next.clone();let mut n=callbacks.clone();match fault{0=>n[0]["protected"]=json!(true),1=>v[0]["g3"]["control"]["current"]["current_validated"]=json!(false),2=>v[0]["g3"]["fixture_peers"][0]["session"]=json!("old-session"),3=>v[0]["g3"]["fixture_peers"][0]["sequence"]=json!(10),4=>v[0]["g3"]["fixture_peers"][0]["age_ms"]=json!(2000),5=>v[0]["g3"]["movement"]["publications"]=json!(10),6=>n[0]["ghost_motion_updates"]=json!(10),7=>n[0]["native_context"]=json!("replaced-context"),_=>n[0]["native_status_frame"]=json!(158)}assert!(probe().observe(2000,&v,&n).is_err(),"Profile fault{fault} was accepted");}
 assert_eq!(distribution(&[10,50,400,800])["p50_ms"],50);assert_eq!(distribution(&[])["count"],0);
}

#[test]fn lc1_network_wss_continuity_requires_real_callbacks_without_requiring_remote_poses(){
 let views:[Value;2]=std::array::from_fn(|_|json!({"g3":{"live_transport":{"grant_active":true}}}));
 let native:[Value;2]=std::array::from_fn(|_|json!({"frame":100,"native_status_frame":100,"protected":false,"native_status":2,"holds":0,"native_context":"same-context","hud_button_frames":100}));
 let mut current=native.clone();for n in &mut current{n["frame"]=json!(160);n["native_status_frame"]=json!(160);n["hud_button_frames"]=json!(160);}
 assert!(PermissionProbe::new(native.clone()).observe(2000,&views,&current).is_none(),"WSS loss must preserve otherwise valid permission even with no remote poses");
 assert_eq!(PermissionProbe::new(native.clone()).observe(2000,&views,&native).unwrap().reason,"Transport interruption stalled actual callbacks/button rendering in2s window");
 let mut expired=current.clone();expired[0]["native_status"]=json!(1);expired[0]["protected"]=json!(true);
 assert!(PermissionProbe::new(native).observe(100,&views,&expired).is_some(),"Real protection must fail an eligible WSS-only continuity phase");
}
