//! Optional social work is single-flight per actor, off Core/native locks.
use super::*;
#[path="next_social_read.rs"]mod read;
#[path="next_social_actions.rs"]mod actions;
#[cfg(test)]#[path="next_social_native_test.rs"]mod native_test;
#[cfg(test)]#[path="next_social_freshness_test.rs"]mod freshness_test;
use nt2_desktop_g0::{g3_social::Owner,g3_social_projection::{Consumer,Pager,Page},g3_social_frame::Command,g3_a2_frame::{self as a2,Command as Display},g3_a2_local::{Store,Operation},g3_social_input::Input,g3_local_consumer::Binding};
#[derive(Clone,Copy,Debug,PartialEq,Eq)]enum Lane{Publication,Information,Markers}
#[derive(Default)]pub(super) struct Work{lane_cursor:usize,key:String,clock:Option<Instant>,hud:Arc<Mutex<a2::HudReplay>>,next:Option<Instant>,publish_next:Option<Instant>,task:Option<tokio::task::JoinHandle<Ready>>,scan:Option<Scan>,producer:Option<Owner>,publish_task:Option<tokio::task::JoinHandle<Published>>,store:Option<Arc<Store>>,out:Vec<Command>,backlog:Option<Vec<Vec<Command>>>,poi_task:Option<tokio::task::JoinHandle<PoiReady>>,poi_owner:Option<Owner>,poi_backlog:Option<Vec<Vec<Command>>>,poi_next:Option<Instant>,latest:Latest,producer_started:Option<Instant>,#[cfg(test)]progress:Arc<Mutex<Value>>,#[cfg(test)]last_error:Option<String>,#[cfg(test)]last_error_context:Option<Value>,hud_progress:Arc<Mutex<Value>>,display_started:Option<Instant>,poi_started:Option<Instant>,display_ready:Option<Instant>,poi_ready:Option<Instant>,poi_first:bool,poi_restart:bool,rescans:u64}
#[cfg(test)]impl Work{pub(super) fn fixture_view(&self)->Value{let error=self.last_error.as_deref().map(|s|{if s.len()>240||s.chars().any(char::is_control)||["token","secret","password","proof","authorization","bearer"].iter().any(|w|s.to_ascii_lowercase().contains(w)){"redacted"}else{s}});json!({"hud_boundary":*self.hud_progress.lock().unwrap(),"progress":*self.progress.lock().unwrap(),"last_error":error,"last_error_context":self.last_error_context,"capture_pending":self.task.is_some(),"poi_capture_pending":self.poi_task.is_some(),"information_age_ms":self.display_started.map(|t|t.elapsed().as_millis()),"poi_age_ms":self.poi_started.map(|t|t.elapsed().as_millis()),"information_wait_ms":self.display_ready.map(|t|t.elapsed().as_millis()),"poi_wait_ms":self.poi_ready.map(|t|t.elapsed().as_millis()),"poi_backlog_batches":self.poi_backlog.as_ref().map_or(0,Vec::len),"rescans":self.rescans,"producer_pending":self.publish_task.is_some(),"output_commands":self.out.len(),"backlog_batches":self.backlog.as_ref().map_or(0,Vec::len),"poi_after":self.scan.as_ref().map(|s|&s.after),"poi_incomplete":self.scan.as_ref().is_some_and(|s|s.pager.is_some())})}}
#[cfg(test)]impl Work{pub(super) fn fixture_delayed_hud(&mut self,scope:&nt2_desktop_g0::protocol_g3_social::wire::Scope,players:&[nt2_desktop_g0::protocol_g3_social::wire::Player],p:&nt2_desktop_g0::protocol_g3_social::wire::Projection)->Result<bool,String>{let value=a2::hud(scope,players,p)?;Ok(self.hud.lock().unwrap().accept(value)?.is_some())}}
#[cfg(test)]impl Work{pub(super) fn fixture_delayed_presentation(&mut self,scope:&nt2_desktop_g0::protocol_g3_social::wire::Scope,players:&[nt2_desktop_g0::protocol_g3_social::wire::Player],p:&nt2_desktop_g0::protocol_g3_social::wire::Projection)->Result<Option<bool>,String>{if self.task.is_some(){return Ok(None)}let clock=self.clock.ok_or("Presentation clock absent")?;Ok(Some(self.latest.presentation.accept(scope,players,p,Duration::ZERO,clock.elapsed().as_secs_f64())?.is_some()))}}
#[cfg(test)]impl Work{pub(super) fn fixture_store(&self)->Option<Arc<Store>>{self.store.clone()}}
impl Drop for Work{fn drop(&mut self){if let Some(t)=&self.task{t.abort()}if let Some(t)=&self.publish_task{t.abort()}if let Some(t)=&self.poi_task{t.abort()}}}
impl Work{
 pub(super) fn item_players(&self)->Value{self.hud_progress.lock().unwrap().get("item_roster").cloned().unwrap_or_else(||json!([]))}
 fn busy(&self)->bool{self.publish_task.is_some()||self.task.is_some()||self.poi_task.is_some()}
 fn lane(&self,now:Instant,publication:bool)->Option<Lane>{
  if self.busy(){return None}
  let lanes=[Lane::Publication,Lane::Information,Lane::Markers];
  (0..3).map(|offset|lanes[(self.lane_cursor+offset)%3]).find(|lane|match lane{
   Lane::Publication=>publication&&self.can_start_publication(now),Lane::Information=>self.can_start_information(now),Lane::Markers=>self.can_start_markers(now)})
 }
 fn started(&mut self,lane:Lane){self.lane_cursor=match lane{Lane::Publication=>1,Lane::Information=>2,Lane::Markers=>0};}
 fn completed(&mut self,lane:Lane,now:Instant){
  // A slow completion must not trigger catch-up work. These are optional-job
  // guards, independent of authority, native/movement leases or presentation age.
  let next=Some(now+Duration::from_millis(500));match lane{Lane::Publication=>self.publish_next=next,Lane::Information=>self.next=next,Lane::Markers=>self.poi_next=next}
 }
 fn can_start_information(&self,now:Instant)->bool{!self.busy()&&self.backlog.is_none()&&self.next.is_none_or(|at|now>=at)}
 fn can_start_publication(&self,now:Instant)->bool{!self.busy()&&self.publish_next.is_none_or(|at|now>=at)}
 fn can_start_markers(&self,now:Instant)->bool{!self.busy()&&self.poi_backlog.is_none()&&self.poi_next.is_none_or(|at|now>=at)}
 fn rescan_markers(&mut self){
  // Losing a native prefix does not invalidate a scoped HTTP query. Repeated
  // short ACK gaps must not cancel every slow marker read. A first page can
  // restart the renderer with Begin; only a continuation needs a fresh prefix.
  if self.poi_task.is_some(){self.poi_restart|=!self.poi_first;}
  else{self.poi_started=None;}
  self.scan=None;self.poi_backlog=None;self.poi_ready=None;self.rescans=self.rescans.saturating_add(1);
 }
 fn finish_markers(&mut self,value:PoiReady,current:&str)->Result<(),String>{
  self.poi_started=None;let restart=std::mem::take(&mut self.poi_restart);
  if value.key!=current{return Ok(())}self.poi_owner=Some(value.owner);
  if restart{self.scan=None;return Ok(())}
  match value.result{Ok(batch)=>{self.scan=Some(value.scan);self.poi_backlog=Some(batch);self.poi_ready=Some(Instant::now());Ok(())},Err(e)=>{
   self.scan=None;
   #[cfg(test)]{self.last_error=Some(e.clone());let p=self.progress.lock().unwrap();self.last_error_context=Some(json!({"phase":p["phase"],"poi_after":p["poi_after"],"at_ms":self.clock.unwrap().elapsed().as_millis(),"request_started_ms":p["at_ms"]}));}
   Err(e)
  }}
 }
 fn publication_input(&self,input:Option<(u64,Input)>)->Option<(Option<(u64,Input)>,bool)>{
  if self.out.is_empty(){Some((input,false))}
  else if input.as_ref().is_some_and(|(_,value)|matches!(value,Input::Hud{..})){Some((input,true))}
  else{None}
 }
 fn ready_batches(&mut self)->Vec<Vec<Command>>{let mut batches=Vec::new();if !self.out.is_empty(){batches.push(std::mem::take(&mut self.out));}if let Some(info)=self.backlog.take(){batches.extend(info);}if let Some(poi)=self.poi_backlog.take(){batches.extend(poi);}batches}}
struct Scan{consumer:Consumer,pager:Option<Pager>,after:String}
#[derive(Default)]struct Latest{after:String,pages:usize,presentation:nt2_desktop_g0::g3_a3_frame::Replay}
fn poi_page(scan:&mut Scan,generation:u64,response:&nt2_desktop_g0::protocol_g3_social::wire::Response)->Result<(Option<u64>,Page),String>{
 let first=scan.pager.is_none();if first{scan.pager=Some(scan.consumer.pager())}
 let page=scan.consumer.page(generation,response,scan.pager.as_mut().unwrap())?;let begin=first.then_some(page.high);
 scan.after=page.next.to_string();if page.complete{scan.pager=None;scan.after="0".into();}
 // Capture completion is not a renderer acknowledgement. Continue refreshing
 // the same retained prefix after a missing/local-discarded display delivery.
 Ok((begin,page))
}
#[cfg(test)]mod tests{use super::*;
 #[test]fn a3_optional_single_flight_and_slow_completion_cooldown(){tauri::async_runtime::block_on(async{
  let mut work=Work::default();let start=Instant::now();assert_eq!(work.lane(start,true),Some(Lane::Publication));work.started(Lane::Publication);
  work.publish_task=Some(tokio::spawn(std::future::pending()));
  assert_eq!(work.lane(start+Duration::from_secs(10),true),None,"A slow optional job cannot overlap any other optional lane");
  assert!(!work.can_start_information(start)&&!work.can_start_markers(start));
  work.publish_task.take().unwrap().abort();let completed=start+Duration::from_millis(1184);work.completed(Lane::Publication,completed);
  assert!(!work.can_start_publication(completed+Duration::from_millis(499)),"A slow completion cannot immediately catch up its old start-based guard");
  assert!(work.can_start_publication(completed+Duration::from_millis(500)));assert_eq!(work.lane(completed,true),Some(Lane::Information));
  work.task=Some(tokio::spawn(std::future::pending()));assert_eq!(work.lane(completed,true),None);work.task.take().unwrap().abort();
  work.poi_task=Some(tokio::spawn(std::future::pending()));assert_eq!(work.lane(completed,true),None);work.rescan_markers();
  assert!(!work.can_start_markers(Instant::now()),"An aborted rescan cannot immediately hammer the marker lane");
 });}
 #[test]fn a3_optional_rotation_keeps_publication_information_and_markers_fair(){
  let mut work=Work::default();let mut now=Instant::now();let lanes=[Lane::Publication,Lane::Information,Lane::Markers];let delays=[379,1184,600];let mut counts=[0;3];
  for step in 0..18{let index=step%3;let lane=work.lane(now,true).unwrap();assert_eq!(lane,lanes[index]);counts[index]+=1;work.started(lane);now+=Duration::from_millis(delays[index]);work.completed(lane,now);}
  assert_eq!(counts,[6,6,6],"Continuous publications must not starve retained information or POI, including slow service completions");
  now+=Duration::from_secs(60);let lane=work.lane(now,true).unwrap();assert_eq!(lane,Lane::Publication);work.started(lane);work.completed(lane,now);assert_eq!(work.lane(now,true),Some(Lane::Information),"Long gaps coalesce into one eligible job per lane, without catch-up queues");
  let mut blocked=Work::default();blocked.backlog=Some(vec![]);assert_eq!(blocked.lane(now,false),Some(Lane::Markers));blocked.poi_backlog=Some(vec![]);assert_eq!(blocked.lane(now,false),None);assert_eq!(blocked.lane(now,true),Some(Lane::Publication),"Undrained reads cannot block eligible bounded HUD publication");
 }
 #[test]fn a2_normal_hud_diagnostics_count_success_refusal_and_redact_errors(){
  let work=Work::default();let d=&work.hud_progress;record_hud_publish(d,&Ok::<(),String>(()));record_hud_publish(d,&Err::<(),String>("CONFLICT".into()));record_hud_publish(d,&Err::<(),String>("private-account-secret".into()));record_information_failure(d,"HUD HP bounds");record_information_failure(d,"private-account-secret");
  let view=work.diagnostics();assert_eq!(view["hud_boundary"]["producer"]["attempts"],3);assert_eq!(view["hud_boundary"]["producer"]["successes"],1);assert_eq!(view["hud_boundary"]["producer"]["refusals"],2);assert_eq!(view["hud_boundary"]["producer"]["last_code"],"other");assert_eq!(view["hud_boundary"]["information_failures"]["count"],2);assert_eq!(view["hud_boundary"]["information_failures"]["last_reason"],"other");assert!(!view.to_string().contains("private-account-secret"));assert!(view["producer_age_ms"].is_null());
 }
 #[test]fn a2_co_drains_maximum_information_marker_and_outcome_batches_within_existing_caps(){
  let mut work=Work::default();let actors:Vec<_>=(0..32).map(|_|uuid::Uuid::new_v4().to_string()).collect();let players=actors.iter().map(|actor|a2::Player{actor:actor.clone(),name:"🧙".repeat(99),state:"playing".into(),ghost:"a".repeat(32)}).collect();let mut display=vec![optional_command(Display::Players{run:"r".repeat(512),local:actors[0].clone(),players})];for actor in actors.iter().take(16){display.push(optional_command(Display::Hud(a2::Hud{actor:actor.clone(),revision:1,hp:Some(1000),max_hp:Some(2000),sampo:Some(true),biome:"b".repeat(128)})));}
  let mut information=vec![display];for n in 0..4{information.push(vec![optional_command(Display::Chat{id:format!("chat-{n}"),name:"🧙".repeat(99),text:"x".repeat(4096)})]);}work.backlog=Some(information);work.out=vec![optional_command(Display::Dismissed((1..=256).map(|n|n.to_string()).collect())),optional_command(Display::Outcome{id:"a".repeat(32),status:0,reason:"🧙".repeat(99),draft:"🧙".repeat(99)})];
  let markers=(1..=16).map(|n|nt2_desktop_g0::g3_social_projection::Marker{sequence:n,actor:actors[0].clone(),name:"🧙".repeat(99),text:"🧙".repeat(99),x:100.,y:100.}).collect();work.poi_backlog=Some(vec![vec![Command::Begin(256),Command::Page(Page{high:256,next:16,markers,complete:false})]]);
  let batches=work.ready_batches();assert_eq!(batches.len(),7);let mut q=nt2_desktop_g0::g3_social_frame::Queue::native([1;16],1);q.publish(0.,batches).unwrap();let mut frames=0;while q.queued(){let frame=q.take([1;16],1,true).unwrap();assert!(frame.len()<=32768);frames+=1;if frames==7{assert_eq!(u16::from_le_bytes(frame[36..38].try_into().unwrap()),2,"Historical Begin/Page did not drain with current information");}}assert_eq!(frames,7);assert!(work.out.is_empty()&&work.backlog.is_none()&&work.poi_backlog.is_none());
 }
 #[test]fn a2_same_high_canonical_snapshot_retries_unapplied_own_marker_and_attributes_peer(){
  use nt2_desktop_g0::protocol_g3_social::wire;let raw:Value=serde_json::from_slice(include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../packages/contracts/g3-social/replay-policy.json"))).unwrap();let scope:wire::Scope=serde_json::from_value(raw["vectors"][0]["scope"].clone()).unwrap();
  let marker=wire::Marker{poi_sequence:Some("1".into()),event_id:Some(uuid::Uuid::new_v4().to_string()),principal_id:Some(uuid::Uuid::new_v4().to_string()),game_session_id:Some(uuid::Uuid::new_v4().to_string()),display_name:Some("Alex".into()),scope:Some(scope.clone()),provenance:Some("validated-client-report".into()),poi:Some(wire::Poi{text:Some("🧙".repeat(99)),x_milli:Some("100000".into()),y_milli:Some("1190000".into())}),..Default::default()};
  for own in [true,false]{let mut recipient=scope.clone();if !own{recipient.world_token=Some(uuid::Uuid::new_v4().to_string());recipient.attachment_id=Some(uuid::Uuid::new_v4().to_string());}let response=wire::Response{scope:Some(recipient.clone()),high_sequence:Some("1".into()),next_sequence:Some("1".into()),markers:Some(vec![marker.clone()]),..Default::default()};let mut scan=Scan{consumer:Consumer::new(recipient,1,vec![]).unwrap(),pager:None,after:"0".into()};
   let(begin,captured)=poi_page(&mut scan,1,&response).unwrap();assert_eq!(begin,Some(1));assert_eq!(captured.markers.len(),1);drop(captured); // Captured but not applied locally.
   let(begin,retry)=poi_page(&mut scan,1,&response).unwrap();assert_eq!(begin,Some(1),"Unchanged canonical high must not suppress display recovery");assert_eq!(retry.markers.len(),1);assert_eq!(retry.markers[0].actor,scope.world_token.as_deref().unwrap());assert_eq!(retry.markers[0].text,"🧙".repeat(99));assert_eq!(scan.after,"0");
   scan.consumer.remember_dismissal(1).unwrap();let(_,dismissed)=poi_page(&mut scan,1,&response).unwrap();assert!(dismissed.markers.is_empty());assert!(dismissed.complete);
  }
 }
}
struct Ready{key:String,owner:Owner,latest:Latest,result:Result<Vec<Vec<Command>>,String>}
struct PoiReady{key:String,owner:Owner,scan:Scan,result:Result<Vec<Vec<Command>>,String>}
struct Published{key:String,owner:Owner,result:Result<Vec<Command>,String>}
fn hud_publish_reason(error:&str)->&'static str{match error{"CAPACITY"=>"capacity","CONFLICT"=>"conflict","FORBIDDEN"=>"forbidden","STALE_EPOCH"=>"stale_epoch","UNAUTHENTICATED"=>"unauthenticated","INVALID_MESSAGE"=>"invalid_message","Social deadline"=>"deadline","Social transport unavailable"=>"transport",_=>"other"}}
fn record_hud_publish<T>(diagnostic:&Arc<Mutex<Value>>,sent:&Result<T,String>){
 let mut d=diagnostic.lock().unwrap();let old=&d["producer"];
 d["producer"]=json!({"attempts":old["attempts"].as_u64().unwrap_or(0).saturating_add(1),"successes":old["successes"].as_u64().unwrap_or(0).saturating_add(u64::from(sent.is_ok())),"refusals":old["refusals"].as_u64().unwrap_or(0).saturating_add(u64::from(sent.is_err())),"last_code":sent.as_ref().err().map(|e|hud_publish_reason(e))});
}
fn record_information_failure(diagnostic:&Arc<Mutex<Value>>,error:&str){let mut d=diagnostic.lock().unwrap();let count=d["information_failures"]["count"].as_u64().unwrap_or(0).saturating_add(1);let reason=match error{"HUD outside authenticated roster"=>"roster","HUD HP bounds"=>"hp_bounds","HUD freshness"=>"freshness","HUD original identity"=>"identity","Foreign HUD scope/provenance"=>"scope",_=>hud_publish_reason(error)};d["information_failures"]=json!({"count":count,"last_reason":reason});}
impl Work{pub(super) fn diagnostics(&self)->Value{json!({"hud_boundary":*self.hud_progress.lock().unwrap(),"information_pending":self.task.is_some(),"producer_pending":self.publish_task.is_some(),"producer_age_ms":self.producer_started.map(|t|t.elapsed().as_millis()),"information_age_ms":self.display_started.map(|t|t.elapsed().as_millis()),"information_queue_ms":self.display_ready.map(|t|t.elapsed().as_millis()),"poi_pending":self.poi_task.is_some(),"poi_age_ms":self.poi_started.map(|t|t.elapsed().as_millis()),"poi_queue_ms":self.poi_ready.map(|t|t.elapsed().as_millis()),"output_commands":self.out.len(),"information_batches":self.backlog.as_ref().map_or(0,Vec::len),"poi_batches":self.poi_backlog.as_ref().map_or(0,Vec::len),"rescans":self.rescans})}}
fn key(binding:&Binding,session:&str,generation:u64)->String{json!([binding.context,binding.service_origin,binding.service_id,binding.service_epoch,binding.session_epoch,binding.principal_id,binding.scope,session,generation]).to_string()}
async fn stored(store:Arc<Store>)->Result<nt2_desktop_g0::g3_a2_local::State,String>{tokio::task::spawn_blocking(move||store.read()).await.map_err(|_|"Social journal read canceled")?}
async fn record<T:Send+'static>(store:Arc<Store>,apply:impl FnOnce(&mut nt2_desktop_g0::g3_a2_local::State)->Result<T,String>+Send+'static)->Result<T,String>{tokio::task::spawn_blocking(move||store.update(apply)).await.map_err(|_|"Social journal write canceled")?}
async fn commit_chat_receipt(store:Arc<Store>,stream:&Arc<Mutex<nt2_desktop_g0::g3_social_stream::Exchange>>,pid:crate::local_owner::Id,context:[u8;16],generation:u32,observed:u32)->Result<(),String>{
 // An old asynchronous publisher must not copy the next owner's receipt into
 // its old journal. Snapshot only its captured binding, then persist unlocked.
 let received=stream.lock().unwrap().acknowledged_chat_high(pid,context,generation);
 if let Some(received)=received.filter(|n|*n>observed){record(store,move|s|{s.chat_high=s.chat_high.max(received);Ok(())}).await?;}Ok(())
}
fn optional_command(value:Display)->Command{Command::A2(value)}
// Test builds only: observe a durable original before its first network publish.
// No supplied response, alternate transport or authority bypass is introduced.
#[cfg(test)]type PoiGate=(String,Arc<Mutex<Option<nt2_desktop_g0::protocol_g3_social::wire::Publish>>>,Arc<std::sync::atomic::AtomicBool>);
#[cfg(test)]static POI_GATE:Mutex<Option<PoiGate>>=Mutex::new(None);
#[cfg(test)]pub(super) struct FixturePoiPause{pub original:Arc<Mutex<Option<nt2_desktop_g0::protocol_g3_social::wire::Publish>>>,released:Arc<std::sync::atomic::AtomicBool>}
#[cfg(test)]impl FixturePoiPause{pub fn release(&self){self.released.store(true,Ordering::Release)}}
#[cfg(test)]impl Drop for FixturePoiPause{fn drop(&mut self){self.release();let mut gate=POI_GATE.lock().unwrap();if gate.as_ref().is_some_and(|g|Arc::ptr_eq(&g.2,&self.released)){*gate=None}}}
#[cfg(test)]pub(super) fn fixture_pause_poi(world:String)->FixturePoiPause{let guard=FixturePoiPause{original:Arc::new(Mutex::new(None)),released:Arc::new(std::sync::atomic::AtomicBool::new(false))};let mut gate=POI_GATE.lock().unwrap();assert!(gate.is_none(),"One bounded A2 fixture pause");*gate=Some((world,guard.original.clone(),guard.released.clone()));guard}
#[cfg(test)]async fn fixture_before_publish(world:&str,original:&nt2_desktop_g0::protocol_g3_social::wire::Publish)->Result<(),String>{let gate={let mut slot=POI_GATE.lock().unwrap();if slot.as_ref().is_some_and(|v|v.0==world){slot.take()}else{None}};if let Some((_,observed,released))=gate{*observed.lock().unwrap()=Some(original.clone());tokio::time::timeout(Duration::from_secs(10),async{while !released.load(Ordering::Acquire){tokio::time::sleep(Duration::from_millis(10)).await}}).await.map_err(|_|"A2 fixture pause expired")?;}Ok(())}
fn chat_commands(rows:Vec<Value>,after:u32,room:&str)->(u32,Vec<Command>){
 let mut rows=rows;rows.sort_by_key(|r|r["sequence"].as_u64().unwrap_or(0));let mut high=after;let mut result=Vec::new();
 for row in rows{let Some(sequence)=row["sequence"].as_u64().filter(|n|*n>high as u64&&*n<=u32::MAX as u64)else{continue};
  let account=row["accountId"].as_str();let name=row["principal"]["displayName"].as_str();let message=row["text"].as_str();
  if account.is_none()||row["principal"]["principalId"].as_str()!=account||name.is_none()||message.is_none(){continue}let(name,message)=(name.unwrap(),message.unwrap());
  if name.len()>396||message.len()>4096||name.chars().any(char::is_control){continue}
  let message=message.chars().map(|c|if c.is_control(){' '}else{c}).collect();
  result.push(optional_command(Display::Chat{id:format!("{room}:{sequence}"),name:name.into(),text:message}));high=sequence as u32;if result.len()>=4{break}
 }(high,result)
}
async fn publish(owner:&mut Owner,input:Option<(u64,Input)>,store:Arc<Store>,stream:Arc<Mutex<nt2_desktop_g0::g3_social_stream::Exchange>>,pid:crate::local_owner::Id,context:[u8;16],generation:u32,transient_only:bool,hud_progress:Arc<Mutex<Value>>)->Result<Vec<Command>,String>{
 owner.discover().await?;let mut output=Vec::new();
 if let Some((sequence,input))=input{match input{
  Input::Hud{hud}=>{stream.lock().unwrap().acknowledge_input(context,generation,sequence);let id=owner.begin(nt2_desktop_g0::protocol_g3_social::wire::Publish{hud:Some(hud),..Default::default()})?;let sent=owner.send_original(&id).await;record_hud_publish(&hud_progress,&sent);let _=sent;if transient_only{return Ok(vec![])}},
  Input::Dismiss{sequence:poi}=>{let number=poi.parse::<u64>().map_err(|_|"Dismissal identity")?;record(store.clone(),move|s|{s.dismissed.insert(number);Ok(())}).await?;stream.lock().unwrap().acknowledge_input(context,generation,sequence);let _=owner.dismiss(&poi).await;},
  Input::Emote{id,emote}=>{actions::accept(owner,store.clone(),id,nt2_desktop_g0::protocol_g3_social::wire::Publish{emote:Some(emote),..Default::default()}).await?;stream.lock().unwrap().acknowledge_input(context,generation,sequence);},
  Input::Cosmetic{id,cosmetic}=>{actions::accept(owner,store.clone(),id,nt2_desktop_g0::protocol_g3_social::wire::Publish{cosmetic:Some(cosmetic),..Default::default()}).await?;stream.lock().unwrap().acknowledge_input(context,generation,sequence);},
  Input::Poi{id,text,x,y}=>{
   let old=stored(store.clone()).await?;
   if old.pending.as_ref().is_some_and(|pending|pending!=&id&&old.operations.get(pending).is_some_and(|op|op.status==0)){
    stream.lock().unwrap().acknowledge_input(context,generation,sequence);output.push(optional_command(Display::Outcome{id,status:2,reason:"A previous message is awaiting its original result".into(),draft:text}));
   }else{
    if old.operations.get(&id).is_some_and(|op|!op.matches(&text,x,y)){return Err("Substituted local POI original".into())}
    if !old.operations.contains_key(&id){
     let floor=old.operations.values().filter_map(|op|op.publish.sequence).max().unwrap_or(0);owner.sequence_floor("poi",floor);
     let event=owner.begin(nt2_desktop_g0::protocol_g3_social::wire::Publish{poi:Some(nt2_desktop_g0::protocol_g3_social::wire::Poi{text:Some(text.clone()),x_milli:Some(x.to_string()),y_milli:Some(y.to_string())}),..Default::default()})?;
     let request:nt2_desktop_g0::protocol_g3_social::wire::Request=serde_json::from_slice(owner.original(&event)?).map_err(|_|"Original POI encoding")?;let operation=Operation{publish:request.publish.unwrap(),text,status:0,reason:String::new(),uncertain:false};let identity=id.clone();
     record(store.clone(),move|s|{s.operations.insert(identity.clone(),operation);s.pending=Some(identity);Ok(())}).await?;
    }
    stream.lock().unwrap().acknowledge_input(context,generation,sequence);
   }
  }
 }}
 output.extend(actions::pump(owner,store.clone()).await?);
 let state=stored(store.clone()).await?;
 commit_chat_receipt(store.clone(),&stream,pid,context,generation,state.chat_high).await?;
 if let Some(id)=state.pending{if let Some(operation)=state.operations.get(&id){
  let mut status=operation.status;let mut reason=operation.reason.clone();let event=operation.publish.event_id.as_deref().ok_or("Missing original POI identity")?;
  if status==0{
   let outcome=owner.poi_outcome(event).await;
   let accepted=match outcome{Ok(response)=>owner.original_outcome(&operation.publish,&response)?,Err(_)=>false};
   if accepted{status=1;}else{
    #[cfg(test)]fixture_before_publish(owner.world(),&operation.publish).await?;
    let event=owner.restore_poi(operation.publish.clone())?;
    let identity=id.clone();record(store.clone(),move|s|{s.operations.get_mut(&identity).ok_or("Missing original")?.uncertain=true;Ok(())}).await?;
    match owner.send_original(&event).await{Ok(_)=>status=1,Err(e)if !operation.uncertain&&matches!(e.as_str(),"TOO_SOON"|"TOO_CLOSE"|"CAPACITY"|"INVALID_MESSAGE"|"CONFLICT")=>{status=2;reason=match e.as_str(){"TOO_SOON"=>"Wait for the 30-second message cooldown","TOO_CLOSE"=>"Messages must be at least 50 world units apart","CAPACITY"=>"Message capacity reached; keep the draft and try later",_=>"The original message was refused"}.into();owner.confirmed_poi_refusal(&event);},Err(_)=>{let identity=id.clone();record(store.clone(),move|s|{s.operations.get_mut(&identity).ok_or("Missing original")?.uncertain=true;Ok(())}).await?;}}
   }
   if status!=0{let identity=id.clone();let text=reason.clone();record(store.clone(),move|s|{let op=s.operations.get_mut(&identity).ok_or("Missing pending POI")?;op.status=status;op.reason=text;Ok(())}).await?;}
  }
  output.push(optional_command(Display::Outcome{id,status,reason,draft:operation.text.clone()}));
 }}
 output.push(optional_command(Display::Dismissed(state.dismissed.iter().map(ToString::to_string).collect())));Ok(output)
}
impl<S:CredentialStore+'static> Core<S>{
 pub(super) async fn lc1_social_tick(&mut self)->Result<(),String>{
  if !self.social_enabled||self.social_stream.is_none(){return Ok(())}let Some(binding)=self.g3_binding()?else{self.social_pause();return Ok(())};
  let root=self.game_path.as_ref().ok_or("No verified social installation")?;let profile=self.social_profile.as_ref().ok_or("Service social profile missing")?;
  if self.social_verified.as_ref().is_none_or(|v|!v.matches(root,profile)){self.social_pause();return Err("Social profile/installation changed".into())}
  let game=self.game.as_ref().unwrap();let generation=self.generation.load(Ordering::Acquire);let current=key(&binding,&game.session.id,generation);let stream=self.social_stream.as_ref().unwrap().clone();
  stream.lock().unwrap().arm(game.owner.0,binding.context,generation.try_into().map_err(|_|"Social generation exhausted")?,self.pending.lock().unwrap().as_ref().unwrap().authority_until)?;
  if self.social_work.key!=current{let mut work=Work::default();work.key=current.clone();work.clock=Some(Instant::now());work.store=Some(Arc::new(Store::new(&self.claims_root,&binding)));self.social_work=work;self.social_owner=None;self.social_consumer=None;}
  if stream.lock().unwrap().take_rescan(){self.social_work.rescan_markers();}
  if self.social_work.publish_task.as_ref().is_some_and(|t|t.is_finished()){
   self.social_work.completed(Lane::Publication,Instant::now());self.social_work.producer_started=None;let value=self.social_work.publish_task.take().unwrap().await.map_err(|_|"Social producer canceled")?;
   if value.key==current{self.social_work.producer=Some(value.owner);match value.result{Ok(mut commands)=>self.social_work.out.append(&mut commands),Err(e)=>self.record_optional_failure("social-publish",FailureStage::Chat,&e)}}
  }
  if self.social_work.task.as_ref().is_some_and(|t|t.is_finished()){
   self.social_work.completed(Lane::Information,Instant::now());let value=self.social_work.task.take().unwrap().await.map_err(|_|"Social information canceled")?;
   {self.social_work.display_started=None;}
   if value.key==current{self.social_owner=Some(value.owner);match value.result{Ok(batch)=>{self.social_work.latest=value.latest;self.social_work.backlog=Some(batch);{self.social_work.display_ready=Some(Instant::now());}},Err(e)=>{
    self.social_work.latest=value.latest;self.social_work.latest.after.clear();self.social_work.latest.pages=0;record_information_failure(&self.social_work.hud_progress,&e);
    #[cfg(test)]{self.social_work.last_error=Some(e.clone());self.social_work.last_error_context=Some(json!({"phase":"information","at_ms":self.social_work.clock.unwrap().elapsed().as_millis()}));}
    self.record_optional_failure("social-information",FailureStage::Chat,&e)
   }}}
  }
  if self.social_work.poi_task.as_ref().is_some_and(|t|t.is_finished()){
   self.social_work.completed(Lane::Markers,Instant::now());let value=self.social_work.poi_task.take().unwrap().await.map_err(|_|"Social marker read canceled")?;
   if let Err(e)=self.social_work.finish_markers(value,&current){self.record_optional_failure("social-markers",FailureStage::Chat,&e);}
  }
  // At most one pending batch per read plus one producer result. Co-drain them
  // into the existing12-frame queue, so neither periodic read starves the other.
  if stream.lock().unwrap().ready(){
   let batches=self.social_work.ready_batches();
   if !batches.is_empty(){stream.lock().unwrap().publish(self.social_work.clock.unwrap().elapsed().as_secs_f64(),batches)?;{self.social_work.display_ready=None;self.social_work.poi_ready=None;}}
  }
  let now=Instant::now();let information_due=self.social_work.next.is_none_or(|at|now>=at);let publish_due=self.social_work.publish_next.is_none_or(|at|now>=at);let poi_due=self.social_work.poi_next.is_none_or(|at|now>=at);if !information_due&&!publish_due&&!poi_due{return Ok(())}
  let game=self.game.as_ref().unwrap();self.social_credentials.publish(binding.clone(),game.session.id.clone(),zeroize::Zeroizing::new(game.session.access.as_str().to_string()))?;
  let session=game.session.id.clone();let access=zeroize::Zeroizing::new(game.session.access.as_str().to_string());let http=self.http.clone();let fence=self.generation.clone();let feed=self.social_credentials.clone();
  let new_owner=||->Result<Owner,String>{let mut owner=Owner::new(binding.clone(),session.clone(),access.clone(),http.clone(),fence.clone())?;owner.credential_feed(feed.clone())?;Ok(owner)};
  let store=self.social_work.store.as_ref().unwrap().clone();let clock=self.social_work.clock.unwrap();
  let publication=self.social_work.publication_input(stream.lock().unwrap().input());
  let lane=self.social_work.lane(now,publication.is_some());
  if lane==Some(Lane::Publication){
   let(input,transient_only)=publication.unwrap();
   let mut owner=if let Some(o)=self.social_work.producer.take(){o}else{new_owner()?};owner.resume()?;let stream=stream.clone();let store=store.clone();let key=current.clone();let context=binding.context;let pid=game.owner.0;
   self.social_work.started(Lane::Publication);
   let hud_progress=self.social_work.hud_progress.clone();
   self.social_work.producer_started=Some(Instant::now());self.social_work.publish_task=Some(tokio::spawn(async move{let result=publish(&mut owner,input,store,stream,pid,context,generation as u32,transient_only,hud_progress).await;Published{key,owner,result}}));
  }
  let scope:nt2_desktop_g0::protocol_g3_social::wire::Scope=serde_json::from_value(serde_json::to_value(&binding.scope).unwrap()).unwrap();
  if lane==Some(Lane::Information){
   self.social_work.started(Lane::Information);
   let mut owner=if let Some(o)=self.social_owner.take(){o}else{new_owner()?};owner.resume()?;let mut latest=std::mem::take(&mut self.social_work.latest);
   let scope=scope.clone();let store=store.clone();let actor=binding.scope.world_token.clone().ok_or("Local actor world")?;let chat=self.chat.iter().map(|m|serde_json::to_value(m).unwrap()).collect();let key=current.clone();let hud=self.social_work.hud.clone();
   let diagnostic=self.social_work.hud_progress.clone();{self.social_work.display_started=Some(Instant::now());}
   self.social_work.task=Some(tokio::spawn(async move{let result=read::information(&mut owner,&mut latest,&scope,actor,store,chat,clock,hud,diagnostic).await;Ready{key,owner,latest,result}}));
  }
  if lane==Some(Lane::Markers){
   self.social_work.started(Lane::Markers);
   let mut owner=if let Some(o)=self.social_work.poi_owner.take(){o}else{new_owner()?};owner.resume()?;
   let mut scan=if let Some(s)=self.social_work.scan.take(){s}else{Scan{consumer:Consumer::new(scope,generation,vec![])?,pager:None,after:"0".into()}};let key=current;
   self.social_work.poi_first=scan.pager.is_none();self.social_work.poi_restart=false;
   #[cfg(test)]let progress=self.social_work.progress.clone();{self.social_work.poi_started=Some(Instant::now());}
   self.social_work.poi_task=Some(tokio::spawn(async move{let result=read::markers(&mut owner,&mut scan,generation,store,clock,#[cfg(test)]progress).await;PoiReady{key,owner,scan,result}}));
  }Ok(())
 }
}
