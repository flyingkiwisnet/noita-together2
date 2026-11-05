// Fixture-only read observer. A recovered socket must not erase a steady-play
// failure, including when that socket was outside network injection queues.
const TRANSPORT_COUNTERS:[&str;7]=[
 "/g3/live_transport/attempts","/g3/live_transport/failures","/g3/movement/live_connections",
 "/g3/movement/live_admission_renewal_retries","/g3/movement/live_renewal_requests",
 "/g3/movement/live_renewals_consumed","/g3/movement_failure/count",
];
fn transport_change(previous:&[Value;2],current:&[Value;2],allowed:[bool;2])->Option<Value>{
 for i in 0..2 {
  let old=&previous[i]["g3"]["live_transport"];let now=&current[i]["g3"]["live_transport"];
  let diagnostic_changed=now["last_receive_failure"]!=old["last_receive_failure"];
  let failure_changed=now["failures"]!=old["failures"];
  let unexpected_code=|s:&Value|s.as_str().is_some_and(|s|["CAPACITY","FORBIDDEN","INVALID_MESSAGE","STALE_EPOCH","UNAUTHENTICATED","REAUTH_REQUIRED"].contains(&s));
  if diagnostic_changed&&unexpected_code(&now["last_receive_failure"]["retire_code"])||failure_changed&&unexpected_code(&now["last_error"]){return Some(json!({"side":i,"reason":"Unexplained service refusal; explicit transport transition cannot excuse it","metric":"service_refusal","before":old,"after":now}));}
  for path in TRANSPORT_COUNTERS {
   let (before,after)=(previous[i].pointer(path).and_then(Value::as_u64),current[i].pointer(path).and_then(Value::as_u64));
   if before.is_none()||after.is_none(){return Some(json!({"side":i,"reason":"Missing transport continuity diagnostic","metric":path,"before":before,"after":after}));}
   if !allowed[i]&&before!=after{return Some(json!({"side":i,"reason":"Unexplained transport counter change during eligible play","metric":path,"before":before,"after":after}));}
  }
  if !allowed[i]&&(diagnostic_changed||now["last_error"]!=old["last_error"]){return Some(json!({"side":i,"reason":"Transport failure diagnostic changed during eligible play","metric":"last_receive_failure_or_error","before":old,"after":now}));}
 }
 None
}

struct TransportWatchState {previous:[Value;2],native_roots:[std::path::PathBuf;2],label:String,allowed:[bool;2],samples:u64}
struct TransportWatch {state:Arc<Mutex<TransportWatchState>>,stop:Arc<std::sync::atomic::AtomicBool>,task:Option<tokio::task::JoinHandle<()>>,handles:[scheduler::Handle<Protected>;2],root:std::path::PathBuf}
fn watch_native(root:&std::path::Path)->Value{for _ in 0..5{if let Some(v)=std::fs::read(root.join("activity.json")).ok().and_then(|b|serde_json::from_slice::<Value>(&b).ok()){return v}std::thread::sleep(Duration::from_millis(1));}Value::Null}
fn save_transport_failure(root:&std::path::Path,state:&TransportWatchState,current:&[Value;2],event:Value){
 let path=root.join("network-first-transport-loss.json");if path.exists(){return}
 let context=std::fs::read(root.join("network-profile-context.json")).ok().and_then(|b|serde_json::from_slice::<Value>(&b).ok());
 let row=json!({"phase":state.label,"event":event,"allowed_transport_transition_targets":state.allowed,"previous_app":state.previous,"current_app":current,"native":[watch_native(&state.native_roots[0]),watch_native(&state.native_roots[1])],"current_sequences":[current[0]["g3"]["fixture_peers"],current[1]["g3"]["fixture_peers"]],"last_driver_network_context":context,"network_context_is_last_driver_sample_not_failure_time":true,"sample":state.samples,"observer_interval_ms":100,"authority_deadlines_changed":false});
 let temporary=root.join("network-first-transport-loss.tmp");std::fs::write(&temporary,serde_json::to_vec_pretty(&row).unwrap()).unwrap();std::fs::rename(temporary,path).unwrap();
}
impl TransportWatch {
 fn start(a:&Client,na:&Native,b:&Client,nb:&Native)->Self{
  let handles=[a.handle.clone(),b.handle.clone()];let previous=[handles[0].view(),handles[1].view()];let root=a.root.parent().unwrap().to_path_buf();
  let state=Arc::new(Mutex::new(TransportWatchState{previous:previous.clone(),native_roots:[na.root.clone(),nb.root.clone()],label:"initial eligible moving pair before any network action".into(),allowed:[false,false],samples:0}));
  for i in 0..2{if previous[i]["g3"]["live_transport"]["failures"]!=0||previous[i]["g3"]["live_transport"]["attempts"]!=1||previous[i]["g3"]["movement"]["live_connections"]!=1{
   save_transport_failure(&root,&state.lock().unwrap(),&previous,json!({"side":i,"reason":"Initial eligible pair already has an unexplained transport failure/replacement","metric":"initial_transport_baseline"}));panic!("Initial live transport is not clean; {}",root.join("network-first-transport-loss.json").display());
  }}
  let stop=Arc::new(std::sync::atomic::AtomicBool::new(false));let state_task=state.clone();let stopped=stop.clone();let views=handles.clone();let output=root.clone();
  let task=tokio::spawn(async move{
   while !stopped.load(Ordering::Acquire){
    let current=[views[0].view(),views[1].view()];{
     let mut state=state_task.lock().unwrap();state.samples+=1;
     let failure=if state.samples>16384{Some(json!({"reason":"Transport observer sample bound exceeded","metric":"observer_samples"}))}else{transport_change(&state.previous,&current,state.allowed)};
     if let Some(event)=failure{save_transport_failure(&output,&state,&current,event);return}
     state.previous=current;
    }
    tokio::time::sleep(Duration::from_millis(100)).await;
   }
  });Self{state,stop,task:Some(task),handles,root}
 }
 fn mode(&self,label:&str,allowed:[bool;2]){
  let current=[self.handles[0].view(),self.handles[1].view()];let mut state=self.state.lock().unwrap();
  // Check under the OLD scope before changing exceptions or counter floors.
  if let Some(event)=transport_change(&state.previous,&current,state.allowed){save_transport_failure(&self.root,&state,&current,event);}
  assert!(!self.root.join("network-first-transport-loss.json").exists(),"Unexplained transport change before phase boundary");
  write_row(&self.root,"network-transport-boundaries.jsonl",&json!({"from":state.label,"to":label,"before_allowed":state.allowed,"after_allowed":allowed,"current_app":current}));
  state.previous=current;state.label=label.into();state.allowed=allowed;
 }
 fn native(&self,side:usize,native:&Native){self.state.lock().unwrap().native_roots[side]=native.root.clone();}
 async fn close(mut self){self.mode("eligible completion before cooperative fixture shutdown",[false,false]);self.stop.store(true,Ordering::Release);if let Some(task)=self.task.take(){task.await.unwrap();}}
}
impl Drop for TransportWatch{fn drop(&mut self){self.stop.store(true,Ordering::Release);if let Some(task)=self.task.take(){task.abort();}}}

#[test]fn lc1_network_transport_watch_catches_brief_recovery_and_keeps_transition_exceptions_scoped(){
 let view=||json!({"g3":{"live_transport":{"attempts":1,"failures":0,"last_error":null,"last_receive_failure":null},"movement":{"live_connections":1,"live_admission_renewal_retries":0,"live_renewal_requests":0,"live_renewals_consumed":0},"movement_failure":{"count":0}}});
 let before=[view(),view()];let mut after=before.clone();
 after[0]["g3"]["live_transport"]["attempts"]=json!(2);after[0]["g3"]["movement"]["live_connections"]=json!(2);
 assert!(transport_change(&before,&after,[false,false]).is_some(),"A reconnected socket with empty queues must fail steady acceptance");
 assert!(transport_change(&before,&after,[true,false]).is_none(),"Explicit consumer transition can replace its socket");
 assert!(transport_change(&before,&after,[false,true]).is_some(),"Peer exception cannot excuse consumer reconnect");
 let mut peer=before.clone();peer[1]["g3"]["live_transport"]["failures"]=json!(1);peer[1]["g3"]["live_transport"]["last_error"]=json!("Live peer closed");
 assert_eq!(transport_change(&before,&peer,[true,false]).unwrap()["side"],1,"Healthy unprofiled peer stays guarded during consumer transition");
 after[0]["g3"]["live_transport"]["last_receive_failure"]=json!({"source":"service_retire","retire_code":"CAPACITY"});
 assert!(transport_change(&before,&after,[true,false]).is_some(),"Fixture switch does not synthesize/excuse service CAPACITY");
 let mut renewal=before.clone();renewal[0]["g3"]["movement"]["live_reauthentications"]=json!(4);assert!(transport_change(&before,&renewal,[false,false]).is_none(),"Ordinary successful same-socket reauth is eligible progress");
}

#[test]fn lc1_network_transport_watch_replays_01_capacity_on_profiled_and_healthy_peer(){
 let run=repo().join("spikes/noita-entry/.build/lc1-network-client-20261020-01/network-profile-462c8e18-5c10-4107-9530-ec0c6173d6ba");
 let first=|name:&str|->Value{let file=std::fs::File::open(run.join(name)).unwrap();serde_json::from_str(&BufReader::new(file).lines().next().unwrap().unwrap()).unwrap()};
 let settled=first("network-settling.jsonl");let sampled=first("network-progress.jsonl");
 let before=[settled["app"][0].clone(),settled["app"][1].clone()];let after=[sampled["app"][0].clone(),sampled["app"][1].clone()];
 assert_eq!(before[0]["g3"]["live_transport"]["failures"],0);assert_eq!(before[1]["g3"]["live_transport"]["failures"],0);
 assert_eq!(after[0]["connection"]["state"],"Playing");assert_eq!(sampled["native"][0]["protected"],false);assert_eq!(sampled["native"][1]["native_status"],2);
 assert!(transport_change(&before,&after,[false,false]).is_some(),"Root01 recovered Playing/native snapshots must fail");
 let peer_only=[before[0].clone(),after[1].clone()];assert_eq!(transport_change(&before,&peer_only,[true,false]).unwrap()["side"],1,"Unprofiled peer failure is not excused by consumer switch");
 let expected=[ghost_identity_for_replay(&before[0]),ghost_identity_for_replay(&before[1])];
 let native=[settled["native"][0].clone(),settled["native"][1].clone()];let mut probe=ProfileProbe::new(expected,before,native);
 assert!(probe.observe(100,&after,&[sampled["native"][0].clone(),sampled["native"][1].clone()]).is_err(),"Do not wait for2s movement/queue gap before failing a transport event");
}
fn ghost_identity_for_replay(view:&Value)->Value {view["g3"]["fixture_peers"].as_array().and_then(|p|p.first()).map(|p|json!({"session":p["session"],"world":p["world"]})).unwrap_or(Value::Null)}

#[test]fn lc1_network_transport_watch_atomic_receipt_stops_native_guard_even_after_recovery(){
 let root=repo().join("spikes/noita-entry/.build/lc1-network-client-20261020-11").join(format!("transport-marker-{}",uuid::Uuid::new_v4()));let native=root.join("client/native");std::fs::create_dir_all(&native).unwrap();
 std::fs::write(native.join("activity.json"),br#"{"protected":false,"native_status":2,"holds":0}"#).unwrap();
 let previous=[json!({"connection":{"state":"Playing"}}),json!({"connection":{"state":"Playing"}})];
 let state=TransportWatchState{previous:previous.clone(),native_roots:[native.clone(),native.clone()],label:"synthetic receipt pipeline check".into(),allowed:[false,false],samples:1};
 save_transport_failure(&root,&state,&previous,json!({"side":1,"metric":"attempts","before":1,"after":2}));
 assert!(!root.join("network-first-transport-loss.tmp").exists());
 let event:Value=serde_json::from_slice(&std::fs::read(root.join("network-first-transport-loss.json")).unwrap()).unwrap();assert_eq!(event["native"][0]["protected"],false);assert_eq!(event["event"]["side"],1);
 let core=Core::new(Protected::new("https://127.0.0.1:49112","lc1-transport-receipt-check"),uuid::Uuid::new_v4(),Arc::new(Mutex::new(None)),root.join("claims"));let(view,actor)=scheduler::new(core,Arc::new(std::sync::atomic::AtomicBool::new(true)));drop(actor);
 let child=NativeChild(Arc::new(Mutex::new(hidden("cmd.exe").args(["/d","/c","exit /b 0"]).spawn().unwrap())));
 let guard=NativeWatch{child:child.clone(),root:native,view};assert!(guard.failure("recovered current permission").unwrap().contains("First unexplained transport event"));
 let until=Instant::now()+Duration::from_secs(5);while child.try_wait().unwrap().is_none(){assert!(Instant::now()<until,"Disposable marker test child did not exit naturally");std::thread::sleep(Duration::from_millis(10));}
}
