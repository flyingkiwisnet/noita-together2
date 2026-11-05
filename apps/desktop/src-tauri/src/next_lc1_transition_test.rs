// Test-only current-owner evidence. The actor owns all Core fields; the pending
// slot is cloned once. Native evidence is separately time/context checked.
use nt2_desktop_g0::world_lifecycle::Stage;
struct NativeEvidence{pid:crate::local_owner::Id,value:Value,modified:Option<std::time::SystemTime>}
fn native_evidence(native:&Native)->NativeEvidence{
 let path=native.root.join("activity.json");let value=if path.is_file(){native_activity(native)}else{Value::Null};
 NativeEvidence{pid:native.owner,value,modified:std::fs::metadata(path).and_then(|m|m.modified()).ok()}
}
fn native_evidence_at(pid:crate::local_owner::Id,root:&std::path::Path)->NativeEvidence{
 let path=root.join("activity.json");let mut value=Value::Null;for _ in 0..20{match std::fs::read(&path){Ok(bytes)=>{if let Ok(v)=serde_json::from_slice(&bytes){value=v;break}},Err(e)if e.kind()==std::io::ErrorKind::NotFound=>break,Err(_)=>{}}std::thread::sleep(Duration::from_millis(1));}
 NativeEvidence{pid,value,modified:std::fs::metadata(path).and_then(|m|m.modified()).ok()}
}
fn core_current_snapshot<S:CredentialStore>(c:&Core<S>,native:Option<NativeEvidence>)->Value{
 let mut view=c.view();view["g3"]=c.g3_view();let pending=c.pending.lock().unwrap().clone();let lifecycle=c.lc1_fixture_view_with(pending.as_ref());let now=Instant::now();let stage=c.world.coordinator.stage();
 let identity=c.game.as_ref().zip(c.world.record.as_ref()).map(|(game,record)|json!({"pid":game.owner.0,"context":hex(&game.owner.1),"session":game.session.id,"session_epoch":game.session.epoch,"attachment":game.session.attachment,"world":record.world,"canonical_generation":record.canonical_generation,"local_generation":c.generation.load(Ordering::Acquire)}));
 let owner_current=pending.as_ref().zip(c.game.as_ref()).zip(c.world.record.as_ref()).is_some_and(|((p,game),record)|{
  c.native_authority_valid(p)&&p.owner==Some(game.owner)&&lifecycle["common_matches_native"]==true&&c.world.coordinator.ticket.as_ref().is_some_and(|t|t.native==game.owner&&t.world==record.world)&&p.accepted&&p.loaded&&p.last_seen.is_some_and(|at|now.saturating_duration_since(at)<Duration::from_secs(12))&&p.observation.as_ref().is_some_and(|r|r.context==game.owner.1&&r.observation.complete&&r.observation.loaded&&r.observation.error.is_none()&&r.observed_at.elapsed()<Duration::from_secs(6)&&p.matches_seed(r.observation.actual_seed.as_deref()))&&crate::pipe::owner_instance(game.owner.0).is_some()&&c.world.ctx.as_ref().is_some_and(|ctx|ctx.world==record.world.to_string()&&ctx.game.id==game.session.id&&ctx.game.epoch==game.session.epoch&&ctx.game.attachment==game.session.attachment&&ctx.instance==uuid::Uuid::from_bytes(game.owner.1).to_string()&&c.desktop.as_ref().is_some_and(|d|ctx.desktop.id==d.id&&ctx.desktop.epoch==d.epoch&&ctx.desktop.account==d.account))
 });
 let playing=owner_current&&stage==Stage::Playing&&pending.as_ref().is_some_and(|p|c.lc1_live_movement_ready(p))&&view["g3"]["live_transport"]["grant_active"]==true;
 let held=owner_current&&stage==Stage::Ready&&pending.as_ref().is_some_and(|p|!p.release&&p.authority_until>now&&p.observation.as_ref().is_some_and(|r|r.observation.protected));
 let native_current=native.as_ref().zip(identity.as_ref()).is_some_and(|(n,id)|n.modified.is_some_and(|at|at.elapsed().is_ok_and(|age|age<Duration::from_millis(500)))&&id["pid"]==json!(n.pid)&&id["context"]==n.value["native_context"]&&n.value["frame"].as_u64().zip(n.value["native_status_frame"].as_u64()).is_some_and(|(frame,status)|frame>=status&&frame-status<=1));
 let native_controls=native_current&&native.as_ref().is_some_and(|n|n.value["native_status"]==2&&n.value["protected"]==false);
 let native_held=native_current&&native.as_ref().is_some_and(|n|n.value["native_status"]==1&&n.value["protected"]==true);
 let lookup_pid=c.native_diagnostics.lock().unwrap().rejoin.world_offered.as_ref().map(|(pid,_)|*pid);
 view["fixture_current"]=json!({"boundary":"one actor command; one pending clone; independently fresh matching native context","stage":stage,"owner_current":owner_current,"playing":playing,"held":held,"identity":identity,"pending_release":pending.as_ref().map(|p|p.release),"pending_authority_live":pending.as_ref().is_some_and(|p|p.authority_until>now),"pending_collection_live":pending.as_ref().is_some_and(|p|p.collection_until.is_some_and(|at|at>now)),"native_current":native_current,"native_controls":native_controls,"native_held":native_held,"native":native.as_ref().map(|n|&n.value),"lookup_pid":lookup_pid});view
}
async fn snapshot_command<S:CredentialStore+'static>(handle:&scheduler::Handle<S>,native:Option<NativeEvidence>,budget:Duration)->Result<Value,String>{
 tokio::time::timeout(budget,handle.command(move|c|Box::pin(async move{Ok(core_current_snapshot(c,native))}))).await.map_err(|_|"Fixture current-owner observation timed out".to_string())?
}
fn snapshot_failure(root:&std::path::Path,label:&str,reason:&str,cached:Value,native:Value)->std::path::PathBuf{
 let path=root.join("current-observation-failure.json");std::fs::write(&path,serde_json::to_vec_pretty(&json!({"label":label,"reason":reason,"boundary":"timed-out fixture observation; cached facts are not current-owner acceptance","cachedApp":cached,"nativeBeforeCommand":native,"engineFactsInjected":true,"serviceMocked":false})).unwrap()).unwrap();path
}
async fn current_snapshot_budget(client:&Client,native:Option<&Native>,label:&str,budget:Duration)->Value{
 let native=native.map(native_evidence);let before=native.as_ref().map(|n|n.value.clone()).unwrap_or(Value::Null);
 match snapshot_command(&client.handle,native,budget).await{Ok(view)=>view,Err(reason)=>{let mut cached=client.handle.view();cached["fixture_scheduler"]=client.handle.fixture_timing();let path=snapshot_failure(&client.root,label,&reason,cached,before);boundary_snapshot(client,"bounded current-owner observation failed");panic!("{reason}: {label}; {}",path.display());}}
}
async fn current_snapshot(client:&Client,native:Option<&Native>)->Value{current_snapshot_budget(client,native,"current native owner snapshot",Duration::from_secs(5)).await}
#[test]fn a3_current_snapshot_reports_blocked_actor_without_waiting_for_command_completion(){tauri::async_runtime::block_on(async{
 let own=repo().join("spikes/noita-entry/.build").join(format!("a3-observation-unit-{}",uuid::Uuid::new_v4()));std::fs::create_dir(&own).unwrap();let core=Core::new(super::tests::MemoryStore::default(),uuid::Uuid::new_v4(),Arc::new(Mutex::new(None)),own.join("claims"));let stop=Arc::new(std::sync::atomic::AtomicBool::new(false));let(handle,task)=scheduler::new(core,stop.clone());
 let(entered_tx,entered_rx)=tokio::sync::oneshot::channel();let(release_tx,release_rx)=tokio::sync::oneshot::channel();let blocking=handle.clone();let blocked=tokio::spawn(async move{blocking.command(move|_|Box::pin(async move{let _=entered_tx.send(());let _=release_rx.await;Ok(Value::Null)})).await});let actor=tokio::spawn(task);let entered=tokio::time::timeout(Duration::from_secs(2),entered_rx).await;
 let result=tokio::time::timeout(Duration::from_secs(1),snapshot_command(&handle,None,Duration::from_millis(30))).await;let cached=handle.view();let reason=result.as_ref().ok().and_then(|v|v.as_ref().err()).cloned();let path=reason.as_ref().map(|reason|snapshot_failure(&own,"blocked actor regression",reason,cached,Value::Null));
 // Release the deliberately blocked async command before any assertion. A
 // failed regression cannot leave a test actor or retained command stranded.
 stop.store(true,Ordering::Release);let _=release_tx.send(());let _=blocked.await;let joined=tokio::time::timeout(Duration::from_secs(2),actor).await;
 assert!(entered.is_ok());assert!(reason.as_deref().is_some_and(|s|s.contains("observation timed out")),"Own observation bound did not fire before outer watchdog");assert!(joined.is_ok());let receipt:Value=serde_json::from_slice(&std::fs::read(path.unwrap()).unwrap()).unwrap();assert_eq!(receipt["label"],"blocked actor regression");assert!(receipt["boundary"].as_str().unwrap().contains("not current-owner acceptance"));
 });}
pub(super) fn fixture_transition_snapshot<S:CredentialStore>(c:&Core<S>,native:Value)->Value{
 core_current_snapshot(c,Some(NativeEvidence{pid:c.game.as_ref().map(|g|g.owner.0).unwrap_or(nt2_local_ws::owner::Id::OsProcess(0)),value:native,modified:Some(std::time::SystemTime::now())}))
}
fn current_state_is(view:&Value,state:&str)->bool{
 view["connection"]["state"]==state&&match state{"Playing"=>view["fixture_current"]["playing"]==true,"Ready"=>view["fixture_current"]["held"]==true,_=>true}
}
#[derive(Default)]struct PublicationFloor{identity:Value,publications:u64,armed:bool}
impl PublicationFloor{
 fn ready(&mut self,view:&Value,pid:crate::local_owner::Id,world:&Value,playing:bool,retired:Option<&str>)->bool{
  let current=&view["fixture_current"];let id=&current["identity"];
  let good=current_state_is(view,if playing{"Playing"}else{"Ready"})&&id["pid"]==json!(pid)&&id["world"]==*world&&retired.is_none_or(|old|id["context"]!=old)&&current[if playing{"native_controls"}else{"native_held"}]==true;
  if !good{self.armed=false;return false}if !playing{return true}
  let published=view["g3"]["movement"]["publications"].as_u64().unwrap_or(0);
  if !self.armed||self.identity!=*id{self.identity=id.clone();self.publications=published;self.armed=true;return false}published>self.publications
 }
}
async fn transition_owner(service:&mut Service,client:&Client,native:&Native,world:&Value,playing:bool,retired:Option<&str>){
 std::fs::write(native.root.join("foreground-monitor"),b"current context/control evidence").unwrap();let until=Instant::now()+Duration::from_secs(45);let mut next=Instant::now();let mut floor=PublicationFloor::default();
 loop{service.wall_clock();if Instant::now()>=next{boundary_snapshot(client,"current native owner transition");if service.trace_movement{network_profile::recovery_owner_trace(client,"current native owner transition").await;}next=Instant::now()+Duration::from_secs(2);}
  let snapshot=current_snapshot(client,Some(native)).await;let ready=floor.ready(&snapshot,native.owner,world,playing,retired);if ready{write_current(client,"accepted current native owner",&snapshot);return}assert!(Instant::now()<until,"Current owner45s deadline: {snapshot}");tokio::time::sleep(Duration::from_millis(250)).await;
 }
}
fn write_current(client:&Client,label:&str,snapshot:&Value){let mut file=std::fs::OpenOptions::new().create(true).append(true).open(client.root.join("current-owner-snapshots.jsonl")).unwrap();writeln!(file,"{}",json!({"label":label,"current":snapshot["fixture_current"],"transport":snapshot["g3"]["live_transport"],"movement":snapshot["g3"]["movement"]})).unwrap();}
fn one_owned_reconnect(before:&[Value;2],after:&[Value;2],label:&str){
 assert_eq!(after[0]["g3"]["live_transport"]["attempts"].as_u64().unwrap(),before[0]["g3"]["live_transport"]["attempts"].as_u64().unwrap()+1,"Unexpected consumer attempts during {label}");assert_eq!(after[0]["g3"]["movement"]["live_connections"].as_u64().unwrap(),before[0]["g3"]["movement"]["live_connections"].as_u64().unwrap()+1,"Unexpected consumer connections during {label}");assert_eq!(after[1]["fixture_current"]["identity"],before[1]["fixture_current"]["identity"]);for key in ["attempts","failures"]{assert_eq!(after[1]["g3"]["live_transport"][key],before[1]["g3"]["live_transport"][key],"Healthy peer transport changed during {label}");}assert_eq!(after[1]["fixture_current"]["native"]["holds"],before[1]["fixture_current"]["native"]["holds"]);
}
async fn wait_pair(service:&mut Service,a:&Client,na:&Native,b:&Client,nb:&Native,label:&str,seconds:u64,mut predicate:impl FnMut(&[Value;2])->bool)->[Value;2]{
 for native in [na,nb]{std::fs::write(native.root.join("foreground-monitor"),b"scoped native evidence").unwrap();}let until=Instant::now()+Duration::from_secs(seconds);let mut next=Instant::now();loop{service.wall_clock();if Instant::now()>=next{for client in [a,b]{boundary_snapshot(client,label);if service.trace_movement{network_profile::recovery_owner_trace(client,label).await;}}next=Instant::now()+Duration::from_secs(2);}let budget=||Duration::from_secs(5).min(until.saturating_duration_since(Instant::now()));let views=[current_snapshot_budget(a,Some(na),label,budget()).await,current_snapshot_budget(b,Some(nb),label,budget()).await];if predicate(&views){for(i,client)in[a,b].into_iter().enumerate(){write_current(client,label,&views[i]);}return views}assert!(Instant::now()<until,"LC1 {label} unchanged{seconds}s deadline; current={views:?}");tokio::time::sleep(Duration::from_millis(250)).await;}
}
#[test]fn lc1_network_transition_readiness_requires_coherent_owner_then_fresh_scoped_publication(){
 let mut v=json!({"connection":{"state":"Playing"},"fixture_current":{"playing":true,"held":false,"native_controls":true,"identity":{"pid":7,"context":"new","world":"world","session":"session"}},"g3":{"movement":{"publications":100}}});let mut floor=PublicationFloor::default();assert!(!floor.ready(&v,nt2_local_ws::owner::Id::OsProcess(7),&json!("world"),true,Some("old")));assert!(!floor.ready(&v,nt2_local_ws::owner::Id::OsProcess(7),&json!("world"),true,Some("old")));v["g3"]["movement"]["publications"]=json!(101);assert!(floor.ready(&v,nt2_local_ws::owner::Id::OsProcess(7),&json!("world"),true,Some("old")));
 for fault in 0..6{let mut bad=v.clone();match fault{0=>bad["fixture_current"]["playing"]=json!(false),1=>bad["fixture_current"]["identity"]=Value::Null,2=>bad["fixture_current"]["identity"]["context"]=json!("old"),3=>bad["fixture_current"]["native_controls"]=json!(false),4=>bad["fixture_current"]["identity"]["world"]=json!("other"),_=>bad["fixture_current"]["identity"]["pid"]=json!(8)}assert!(!floor.ready(&bad,nt2_local_ws::owner::Id::OsProcess(7),&json!("world"),true,Some("old")));assert!(!floor.ready(&v,nt2_local_ws::owner::Id::OsProcess(7),&json!("world"),true,Some("old")),"Invalid sample failed to reset publication floor");}
 let mut switched=v.clone();switched["fixture_current"]["identity"]["session"]=json!("replacement");switched["g3"]["movement"]["publications"]=json!(1000);assert!(!floor.ready(&switched,nt2_local_ws::owner::Id::OsProcess(7),&json!("world"),true,Some("old")),"Cumulative old publications accepted replacement");
}
