// Typed transport doubles exercise production read futures, journals, replay
// validation and frame queues. No service/DB/game or admission is fabricated.
struct Responses{scope:wire::Scope,actors:Vec<(wire::Scope,String,String)>,sequence:u32,calls:Arc<Mutex<Vec<String>>>,gate:Option<Arc<tokio::sync::Notify>>,entered:Arc<tokio::sync::Notify>,block_poi:bool,refuse_roster:bool}
impl Responses{
 fn new(scope:&wire::Scope)->Self{let mut peer=scope.clone();peer.world_token=Some(uuid::Uuid::new_v4().to_string());peer.attachment_id=Some(uuid::Uuid::new_v4().to_string());let mut actors=vec![(scope.clone(),uuid::Uuid::new_v4().to_string(),uuid::Uuid::new_v4().to_string()),(peer,uuid::Uuid::new_v4().to_string(),uuid::Uuid::new_v4().to_string())];actors.sort_by(|a,b|a.2.cmp(&b.2));Self{scope:scope.clone(),actors,sequence:0,calls:Default::default(),gate:None,entered:Default::default(),block_poi:false,refuse_roster:false}}
}
impl Source for Responses{
 async fn discover(&mut self)->Result<(),String>{Ok(())}
 async fn roster(&mut self)->Result<wire::Response,String>{
  self.calls.lock().unwrap().push("roster".into());if self.refuse_roster{return Err("CAPACITY".into())}if !self.block_poi{if let Some(gate)=&self.gate{self.entered.notify_one();gate.notified().await;}}
  Ok(wire::Response{roster_complete:Some(true),players:Some(self.actors.iter().enumerate().map(|(n,(scope,principal,session))|wire::Player{player_id:scope.world_token.clone(),world_token:scope.world_token.clone(),principal_id:Some(principal.clone()),game_session_id:Some(session.clone()),display_name:Some(format!("Player{n}")),state:Some("playing".into()),inventory_revision:Some(0),held_wand_sprite_id:Some(String::new()),..Default::default()}).collect()),..Default::default()})
 }
 async fn latest(&mut self,after:&str)->Result<nt2_desktop_g0::g3_social::TimedSnapshot,String>{
  self.calls.lock().unwrap().push(format!("latest:{after}"));self.sequence+=1;let sequence=self.sequence;
  let projections:Vec<_>=self.actors.iter().filter(|(_,_,session)|format!("{session}:hud").as_str()>after).take(16).map(|(scope,principal,session)|wire::Projection{scope:Some(scope.clone()),principal_id:Some(principal.clone()),game_session_id:Some(session.clone()),hud_state_revision:Some(sequence),provenance:Some("validated-client-report".into()),captured_at:Some("1970-01-01T00:00:01.000Z".into()),hud:Some(wire::Hud{hp_known:Some(true),hp_milli:Some(4000),max_hp_milli:Some(8000),..Default::default()}),..Default::default()}).collect();
  let next=projections.last().map(|p|format!("{}:hud",p.game_session_id.as_deref().unwrap())).unwrap_or_else(||after.into());Ok(nt2_desktop_g0::g3_social::TimedSnapshot{response:wire::Response{projections:Some(projections),next_key:Some(next),..Default::default()},request_started:Instant::now()})
 }
 async fn poi(&mut self,after:&str)->Result<wire::Response,String>{
  self.calls.lock().unwrap().push(format!("poi:{after}"));if self.block_poi{if let Some(gate)=&self.gate{self.entered.notify_one();gate.notified().await;}}
  let start=after.parse::<u32>().unwrap()+1;let last=(start+15).min(256);let markers=(start..=last).map(|n|wire::Marker{poi_sequence:Some(n.to_string()),event_id:Some(uuid::Uuid::from_u128(u128::from(n)).to_string()),principal_id:Some(self.actors[0].1.clone()),game_session_id:Some(self.actors[0].2.clone()),display_name:Some("雪".repeat(99)),scope:Some(self.scope.clone()),provenance:Some("validated-client-report".into()),poi:Some(wire::Poi{text:Some("🧙".repeat(99)),x_milli:Some((n*100000).to_string()),y_milli:Some("100000".into())}),..Default::default()}).collect();
  Ok(wire::Response{scope:Some(self.scope.clone()),high_sequence:Some("256".into()),next_sequence:Some(last.to_string()),markers:Some(markers),..Default::default()})
 }
}
struct Local{root:std::path::PathBuf,store:Arc<Store>,scope:wire::Scope}
impl Local{fn new()->Self{let raw:Value=serde_json::from_slice(include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../packages/contracts/g3-social/replay-policy.json"))).unwrap();let scope:wire::Scope=serde_json::from_value(raw["vectors"][0]["scope"].clone()).unwrap();let binding=Binding{context:[1;16],service_origin:"https://fixture.invalid".into(),service_id:"service".into(),service_epoch:"epoch".into(),session_epoch:"session".into(),principal_id:uuid::Uuid::new_v4().to_string(),scope:serde_json::from_value(serde_json::to_value(&scope).unwrap()).unwrap()};let root=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../spikes/noita-entry/.build").join(format!("a2-read-unit-{}",uuid::Uuid::new_v4()));std::fs::create_dir(&root).unwrap();let store=Arc::new(Store::new(&root,&binding));Self{root,store,scope}}}
impl Drop for Local{fn drop(&mut self){let base=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../spikes/noita-entry/.build").canonicalize().unwrap();if self.root.canonicalize().is_ok_and(|p|p.starts_with(base)){let _=std::fs::remove_dir_all(&self.root);}}}
struct DiagnosticSource{inner:Responses,mode:&'static str}
impl Source for DiagnosticSource{
 async fn discover(&mut self)->Result<(),String>{self.inner.discover().await}
 async fn roster(&mut self)->Result<wire::Response,String>{self.inner.roster().await}
 async fn poi(&mut self,after:&str)->Result<wire::Response,String>{self.inner.poi(after).await}
 async fn latest(&mut self,after:&str)->Result<nt2_desktop_g0::g3_social::TimedSnapshot,String>{let mut value=self.inner.latest(after).await?;
  if self.mode=="absent"{value.response.projections=Some(vec![]);value.response.next_key=Some(after.into());}
  for p in value.response.projections.iter_mut().flatten(){if self.mode=="unknown"{let h=p.hud.as_mut().unwrap();h.hp_known=Some(false);h.hp_milli=None;h.max_hp_milli=None;}}
  Ok(value)
 }
}
#[test]fn a2_normal_information_counters_distinguish_absent_unknown_and_retained(){tauri::async_runtime::block_on(async{
 let local=Local::new();for(mode,rows,known,display)in[("known",2,2,2),("unknown",2,0,2),("absent",0,0,0)]{
  let mut source=DiagnosticSource{inner:Responses::new(&local.scope),mode};let mut latest=Latest::default();let d:Arc<Mutex<Value>>=Default::default();
  let batches=information(&mut source,&mut latest,&local.scope,local.scope.world_token.clone().unwrap(),local.store.clone(),vec![],Instant::now(),Default::default(),d.clone()).await.unwrap();
  assert!(matches!(batches[0][0],Command::A2(Display::Players{..})));let view=d.lock().unwrap();let c=&view["consumer"];assert_eq!(c["hud_rows"],rows);assert_eq!(c["known_hp_rows"],known);assert_eq!(c["display_commands"],display);assert_eq!(view["information_read"]["phase"],"complete");assert!(view["information_read"]["journal_ms"].is_number());
 }
});}
struct Job<T>(Option<tokio::task::JoinHandle<T>>);
#[test]fn a2_chat_capture_does_not_commit_before_native_receipt(){tauri::async_runtime::block_on(async{
 let local=Local::new();let mut source=Responses::new(&local.scope);let rows=vec![json!({"sequence":1,"accountId":"fixture-principal","principal":{"principalId":"fixture-principal","displayName":"A"},"text":"hello"})];
 let batches=information(&mut source,&mut Latest::default(),&local.scope,local.scope.world_token.clone().unwrap(),local.store.clone(),rows,Instant::now(),Default::default(),Default::default()).await.unwrap();
 assert!(batches.iter().flatten().any(|c|matches!(c,Command::A2(Display::Chat{..}))));
 assert_eq!(local.store.read().unwrap().chat_high,0,"chat cursor advanced before native receipt");
});}
fn fixture_owner(scope:&wire::Scope)->Owner{let binding=Binding{context:[1;16],service_origin:"https://fixture.invalid".into(),service_id:"service".into(),service_epoch:"epoch".into(),session_epoch:"session".into(),principal_id:uuid::Uuid::new_v4().to_string(),scope:serde_json::from_value(serde_json::to_value(scope).unwrap()).unwrap()};Owner::new(binding,uuid::Uuid::new_v4().to_string(),zeroize::Zeroizing::new("fixture-token".into()),reqwest::Client::new(),Arc::new(std::sync::atomic::AtomicU64::new(1))).unwrap()}
#[test]fn a3_repeated_native_rescans_preserve_slow_first_page_and_restart_only_lost_continuation(){tauri::async_runtime::block_on(async{
 for first in [true,false]{
  let local=Local::new();let mut source=Responses::new(&local.scope);let clock=Instant::now();let mut scan=Scan{consumer:Consumer::new(local.scope.clone(),1,vec![]).unwrap(),pager:None,after:"0".into()};
  if !first{let batch=markers(&mut source,&mut scan,1,local.store.clone(),clock,Default::default()).await.unwrap();assert!(matches!(batch[0][0],Command::Begin(256)));assert_eq!(scan.after,"16");}
  let gate=Arc::new(tokio::sync::Notify::new());source.gate=Some(gate.clone());source.block_poi=true;let entered=source.entered.clone();let owner=fixture_owner(&local.scope);let store=local.store.clone();
  let mut work=Work::default();work.clock=Some(clock);work.poi_first=first;work.poi_started=Some(clock-Duration::from_secs(100));
  work.poi_task=Some(tokio::spawn(async move{let result=markers(&mut source,&mut scan,1,store,clock,Default::default()).await;PoiReady{key:"current".into(),owner,scan,result}}));
  tokio::time::timeout(Duration::from_secs(2),entered.notified()).await.unwrap();
  for _ in 0..54{work.rescan_markers();assert!(work.poi_task.is_some()&&!work.poi_task.as_ref().unwrap().is_finished());assert_eq!(work.poi_restart,!first);assert!(work.poi_backlog.is_none());assert_eq!(work.lane(Instant::now(),true),None,"Single flight remains bounded during delay");}
  gate.notify_one();let value=tokio::time::timeout(Duration::from_secs(2),work.poi_task.take().unwrap()).await.unwrap().unwrap();work.finish_markers(value,"current").unwrap();
  if first{assert!(matches!(work.poi_backlog.as_ref().unwrap()[0][0],Command::Begin(256)),"First page restarts renderer despite repeated aborts");assert_eq!(work.scan.as_ref().unwrap().after,"16");}
  else{assert!(work.poi_backlog.is_none()&&work.scan.is_none(),"Continuation cannot apply without its lost prefix");let mut fresh=Scan{consumer:Consumer::new(local.scope.clone(),1,vec![]).unwrap(),pager:None,after:"0".into()};let mut source=Responses::new(&local.scope);let batch=markers(&mut source,&mut fresh,1,local.store.clone(),clock,Default::default()).await.unwrap();assert!(matches!(batch[0][0],Command::Begin(256)));}
  assert!(work.poi_started.is_none()&&!work.poi_restart);assert_eq!(work.rescans,54);
  work.poi_backlog=None;work.poi_next=Some(clock);for _ in 0..54{work.rescan_markers();}assert!(work.can_start_markers(Instant::now()),"Rescans cannot keep pushing an idle lane's completion guard into the future");
 }
});}
#[test]fn a2_marker_rescan_retains_inflight_information_and_original_deadline(){tauri::async_runtime::block_on(async{
 let local=Local::new();let mut source=Responses::new(&local.scope);let gate=Arc::new(tokio::sync::Notify::new());source.gate=Some(gate.clone());let entered=source.entered.clone();let scope=local.scope.clone();let store=local.store.clone();let clock=Instant::now();let owner=fixture_owner(&scope);
 let mut work=Work::default();work.next=Some(clock);work.latest.after="keep-current-cursor".into();work.task=Some(tokio::spawn(async move{let mut latest=Latest::default();let result=information(&mut source,&mut latest,&scope,scope.world_token.clone().unwrap(),store,vec![],clock,Default::default(),Default::default()).await;Ready{key:"fixture".into(),owner,latest,result}}));work.poi_task=Some(tokio::spawn(std::future::pending()));
 tokio::time::timeout(Duration::from_secs(2),entered.notified()).await.unwrap();work.rescan_markers();assert!(work.task.is_some()&&!work.task.as_ref().unwrap().is_finished());assert!(work.poi_task.is_some()&&work.poi_restart);work.poi_task.take().unwrap().abort();assert_eq!(work.latest.after,"keep-current-cursor");assert_eq!(work.next,Some(clock));assert!(!work.can_start_information(Instant::now()),"Cannot overlap current reads");
 gate.notify_one();let ready=tokio::time::timeout(Duration::from_secs(2),work.task.as_mut().unwrap()).await.unwrap().unwrap();work.task.take();assert!(work.can_start_information(Instant::now()),"Busy tick must not consume another500ms guard");let batches=ready.result.unwrap();assert_eq!(batches.iter().flatten().filter(|c|matches!(c,Command::A2(Display::Hud(_)))).count(),2);
 work.backlog=Some(batches);let before:Vec<_>=work.backlog.as_ref().unwrap().iter().flatten().filter_map(|c|if let Command::A2(Display::Hud(h))=c{Some(h.revision)}else{None}).collect();work.rescan_markers();let after:Vec<_>=work.backlog.as_ref().unwrap().iter().flatten().filter_map(|c|if let Command::A2(Display::Hud(h))=c{Some(h.revision)}else{None}).collect();assert_eq!(before,after,"Rescan changed retained revision");
 let mut q=nt2_desktop_g0::g3_social_frame::Queue::native([1;16],1);q.publish(3600.,work.backlog.take().unwrap()).unwrap();assert!(q.take([1;16],1,true).is_some(),"Old retained information remains encodable without a fake TTL");
});}
#[test]fn a2_hud_publication_is_independent_of_display_backpressure_and_read_guard(){
 let now=Instant::now();let mut work=Work::default();work.next=Some(now+Duration::from_millis(500));work.out=vec![optional_command(Display::Dismissed(vec![]))];assert!(!work.can_start_information(now));assert!(work.can_start_publication(now));
 let input=Input::Hud{hud:wire::Hud{hp_known:Some(true),hp_milli:Some(1000),max_hp_milli:Some(2000),..Default::default()}};
 let (Some((sequence,Input::Hud{..})),transient)=work.publication_input(Some((1,input))).unwrap()else{panic!("HUD publication")};assert_eq!(sequence,1);assert!(transient);assert_eq!(work.out.len(),1,"Cannot grow undrained output");assert!(work.publication_input(None).is_none());assert!(work.publication_input(Some((2,Input::Dismiss{sequence:"1".into()}))).is_none());
 work.publish_next=Some(now+Duration::from_millis(500));assert!(!work.can_start_publication(now));assert!(work.can_start_publication(now+Duration::from_millis(500)));
}
#[test]fn a2_unacknowledged_chat_replays_and_native_receipt_commits_off_stream_lock(){tauri::async_runtime::block_on(async{
 let local=Local::new();let mut source=Responses::new(&local.scope);let clock=Instant::now();let rows=vec![json!({"sequence":1,"accountId":"fixture","principal":{"principalId":"fixture","displayName":"A"},"text":"hello"})];
 let mut latest=Latest::default();let hud:Arc<Mutex<a2::HudReplay>>=Default::default();let batches=information(&mut source,&mut latest,&local.scope,local.scope.world_token.clone().unwrap(),local.store.clone(),rows.clone(),clock,hud.clone(),Default::default()).await.unwrap();let stream=Arc::new(Mutex::new(nt2_desktop_g0::g3_social_stream::Exchange::default()));let ctx=[1;16];
 let poll=|ack:u32|{let mut p=[0;64];p[..8].copy_from_slice(b"NSP1\x01\0\0\0");p[8..24].copy_from_slice(&ctx);p[24..28].copy_from_slice(&ack.to_le_bytes());p};
 {let mut x=stream.lock().unwrap();x.arm(nt2_local_ws::owner::Id::OsProcess(7),ctx,1,Instant::now()+Duration::from_secs(3)).unwrap();x.publish(clock.elapsed().as_secs_f64(),batches).unwrap();x.transition(nt2_local_ws::owner::Id::OsProcess(7),&poll(0),true);x.pause();}
 commit_chat_receipt(local.store.clone(),&stream,nt2_local_ws::owner::Id::OsProcess(7),ctx,1,0).await.unwrap();assert_eq!(local.store.read().unwrap().chat_high,0);
 let replay=information(&mut source,&mut latest,&local.scope,local.scope.world_token.clone().unwrap(),local.store.clone(),rows.clone(),clock,hud.clone(),Default::default()).await.unwrap();assert!(replay.iter().flatten().any(|c|matches!(c,Command::A2(Display::Chat{..}))));
 {let mut x=stream.lock().unwrap();x.arm(nt2_local_ws::owner::Id::OsProcess(7),ctx,1,Instant::now()+Duration::from_secs(3)).unwrap();x.publish(clock.elapsed().as_secs_f64(),replay).unwrap();let mut ack=0;for _ in 0..4{let reply=x.transition(nt2_local_ws::owner::Id::OsProcess(7),&poll(ack),true);let n=u32::from_le_bytes(reply[32..36].try_into().unwrap());if n==0{break}ack=u32::from_le_bytes(reply[92..96].try_into().unwrap());}}
 assert_eq!(stream.lock().unwrap().acknowledged_chat_high(nt2_local_ws::owner::Id::OsProcess(7),ctx,1),Some(1));commit_chat_receipt(local.store.clone(),&stream,nt2_local_ws::owner::Id::OsProcess(7),ctx,1,0).await.unwrap();assert_eq!(local.store.read().unwrap().chat_high,1);
 let next=information(&mut source,&mut latest,&local.scope,local.scope.world_token.clone().unwrap(),local.store.clone(),rows,clock,hud,Default::default()).await.unwrap();assert!(!next.iter().flatten().any(|c|matches!(c,Command::A2(Display::Chat{..}))));
});}
#[test]fn a2_delayed_old_publisher_cannot_commit_new_owner_receipt(){tauri::async_runtime::block_on(async{
 let old=Local::new();let new=Local::new();let stream=Arc::new(Mutex::new(nt2_desktop_g0::g3_social_stream::Exchange::default()));let ctx=[1;16];
 stream.lock().unwrap().arm(nt2_local_ws::owner::Id::OsProcess(7),ctx,1,Instant::now()+Duration::from_secs(3)).unwrap();
 let entered=Arc::new(tokio::sync::Notify::new());let release=Arc::new(tokio::sync::Notify::new());let task={let stream=stream.clone();let store=old.store.clone();let entered=entered.clone();let release=release.clone();tokio::spawn(async move{entered.notify_one();release.notified().await;commit_chat_receipt(store,&stream,nt2_local_ws::owner::Id::OsProcess(7),ctx,1,0).await})};
 entered.notified().await;
 {let mut x=stream.lock().unwrap();x.arm(nt2_local_ws::owner::Id::OsProcess(8),ctx,2,Instant::now()+Duration::from_secs(3)).unwrap();x.publish(0.,vec![vec![optional_command(Display::Chat{id:"room:9".into(),name:"A".into(),text:"new".into()})]]).unwrap();let mut poll=[0;64];poll[..8].copy_from_slice(b"NSP1\x01\0\0\0");poll[8..24].copy_from_slice(&ctx);let reply=x.transition(nt2_local_ws::owner::Id::OsProcess(8),&poll,true);poll[24..28].copy_from_slice(&reply[92..96]);x.transition(nt2_local_ws::owner::Id::OsProcess(8),&poll,true);assert_eq!(x.acknowledged_chat_high(nt2_local_ws::owner::Id::OsProcess(8),ctx,2),Some(9));assert_eq!(x.acknowledged_chat_high(nt2_local_ws::owner::Id::OsProcess(7),ctx,2),None);assert_eq!(x.acknowledged_chat_high(nt2_local_ws::owner::Id::OsProcess(8),ctx,1),None);assert_eq!(x.acknowledged_chat_high(nt2_local_ws::owner::Id::OsProcess(8),[2;16],2),None);}
 release.notify_one();tokio::time::timeout(Duration::from_secs(2),task).await.unwrap().unwrap().unwrap();assert_eq!(old.store.read().unwrap().chat_high,0,"New receipt leaked into delayed old journal");
 commit_chat_receipt(new.store.clone(),&stream,nt2_local_ws::owner::Id::OsProcess(8),ctx,2,0).await.unwrap();assert_eq!(new.store.read().unwrap().chat_high,9);
});}
impl<T>Drop for Job<T>{fn drop(&mut self){if let Some(job)=self.0.take(){job.abort()}}}
#[test]fn a2_sixteen_marker_pages_progress_while_information_is_blocked_or_refused(){tauri::async_runtime::block_on(async{
 let local=Local::new();local.store.update(|s|{s.dismissed.insert(1);Ok(())}).unwrap();let clock=Instant::now();let mut info=Responses::new(&local.scope);let gate=Arc::new(tokio::sync::Notify::new());info.gate=Some(gate.clone());let entered=info.entered.clone();let calls=info.calls.clone();let scope=local.scope.clone();let store=local.store.clone();let hud=Arc::new(Mutex::new(a2::HudReplay::default()));let mut job=Job(Some(tokio::spawn(async move{let mut latest=Latest::default();information(&mut info,&mut latest,&scope,scope.world_token.clone().unwrap(),store,vec![],clock,hud,Default::default()).await})));
 tokio::time::timeout(Duration::from_secs(2),entered.notified()).await.unwrap();let mut source=Responses::new(&local.scope);let mut scan=Scan{consumer:Consumer::new(local.scope.clone(),1,vec![]).unwrap(),pager:None,after:"0".into()};let mut count=0;let mut begins=0;let mut queue=nt2_desktop_g0::g3_social_frame::Queue::native([1;16],1);
 for page in 0..16{let batch=markers(&mut source,&mut scan,1,local.store.clone(),clock,Default::default()).await.unwrap();assert_eq!(batch.len(),1,"One atomic frame per historical page");for c in &batch[0]{match c{Command::Begin(_)=>begins+=1,Command::Page(p)=>{count+=p.markers.len();assert_eq!(p.next,(page+1)*16);},_=>panic!("Historical page contains unrelated current work")}}queue.publish(clock.elapsed().as_secs_f64(),batch).unwrap();let frame=queue.take([1;16],1,true).unwrap();assert!(frame.len()<=32768&&!queue.queued());
  if page==7{let after=scan.after.clone();let mut refused=Responses::new(&local.scope);refused.refuse_roster=true;let failure=information(&mut refused,&mut Latest::default(),&local.scope,local.scope.world_token.clone().unwrap(),local.store.clone(),vec![],clock,Default::default(),Default::default()).await;assert_eq!(failure.err().unwrap(),"CAPACITY");assert_eq!(scan.after,after,"Information refusal discarded a valid historical prefix");}
 }
 assert_eq!((begins,count),(1,255));assert!(scan.pager.is_none()&&scan.after=="0");assert!(source.calls.lock().unwrap().iter().all(|s|s.starts_with("poi:")));assert_eq!(calls.lock().unwrap().as_slice(),["roster"]);assert!(!job.0.as_ref().unwrap().is_finished(),"Marker progress needed the blocked information result");gate.notify_one();let displayed=tokio::time::timeout(Duration::from_secs(2),job.0.take().unwrap()).await.unwrap().unwrap().unwrap();assert!(displayed[0].iter().filter(|c|matches!(c,Command::A2(Display::Hud(_)))).count()==2);
});}
#[test]fn a2_current_hud_refreshes_without_empty_queries_while_markers_are_blocked(){tauri::async_runtime::block_on(async{
 let local=Local::new();let clock=Instant::now();let mut poi=Responses::new(&local.scope);let gate=Arc::new(tokio::sync::Notify::new());poi.gate=Some(gate.clone());poi.block_poi=true;let entered=poi.entered.clone();let scope=local.scope.clone();let store=local.store.clone();let mut job=Job(Some(tokio::spawn(async move{let mut scan=Scan{consumer:Consumer::new(scope,1,vec![]).unwrap(),pager:None,after:"0".into()};markers(&mut poi,&mut scan,1,store,clock,Default::default()).await})));
 tokio::time::timeout(Duration::from_secs(2),entered.notified()).await.unwrap();let mut info=Responses::new(&local.scope);let mut latest=Latest::default();let hud=Arc::new(Mutex::new(a2::HudReplay::default()));
 for sequence in 1..=4{let batch=information(&mut info,&mut latest,&local.scope,local.scope.world_token.clone().unwrap(),local.store.clone(),vec![],clock,hud.clone(),Default::default()).await.unwrap();let samples:Vec<_>=batch[0].iter().filter_map(|c|if let Command::A2(Display::Hud(h))=c{Some(h)}else{None}).collect();assert_eq!(samples.len(),2);assert!(samples.iter().all(|h|h.revision==sequence&&h.hp==Some(4000)));assert!(latest.after.is_empty());}
 assert_eq!(info.calls.lock().unwrap().iter().filter(|s|s.as_str()=="latest:").count(),4);assert!(!job.0.as_ref().unwrap().is_finished());gate.notify_one();assert_eq!(tokio::time::timeout(Duration::from_secs(2),job.0.take().unwrap()).await.unwrap().unwrap().unwrap().len(),1);
});}
#[test]fn a2_full_latest_pages_cover_all_thirty_two_actors_before_refreshing_first_page(){tauri::async_runtime::block_on(async{
 let local=Local::new();let clock=Instant::now();let mut source=Responses::new(&local.scope);while source.actors.len()<32{let mut scope=local.scope.clone();scope.world_token=Some(uuid::Uuid::new_v4().to_string());scope.attachment_id=Some(uuid::Uuid::new_v4().to_string());source.actors.push((scope,uuid::Uuid::new_v4().to_string(),uuid::Uuid::new_v4().to_string()));}source.actors.sort_by(|a,b|a.2.cmp(&b.2));let mut latest=Latest::default();let hud=Arc::new(Mutex::new(a2::HudReplay::default()));let mut seen=std::collections::BTreeSet::new();
 for page in 0..3{let batches=information(&mut source,&mut latest,&local.scope,local.scope.world_token.clone().unwrap(),local.store.clone(),vec![],clock,hud.clone(),Default::default()).await.unwrap();let samples:Vec<_>=batches[0].iter().filter_map(|c|if let Command::A2(Display::Hud(h))=c{Some(h)}else{None}).collect();assert_eq!(samples.len(),if page<2{16}else{0});for sample in samples{assert!(seen.insert(sample.actor.clone()),"A full page restarted before reaching later actors");}}
 assert_eq!(seen.len(),32);assert!(latest.after.is_empty());let next=information(&mut source,&mut latest,&local.scope,local.scope.world_token.clone().unwrap(),local.store.clone(),vec![],clock,hud,Default::default()).await.unwrap();assert_eq!(next[0].iter().filter(|c|matches!(c,Command::A2(Display::Hud(_)))).count(),16);
});}
