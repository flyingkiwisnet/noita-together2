//! Local, bounded, optional diagnostics. Producers never perform filesystem I/O.
use serde::{Deserialize,Serialize};
use serde_json::Value;
use std::{collections::{BTreeMap,VecDeque},path::{Path,PathBuf},sync::{Arc,atomic::{AtomicBool,AtomicU64,Ordering},mpsc},time::{Duration,Instant,SystemTime,UNIX_EPOCH}};
#[path="debug_bank.rs"] mod bank;
pub const REVISION:&str="nt2-client-debug-1";
pub const MAX_FILE:usize=262144;
pub const MAX_EVENTS:usize=64;
pub const MAX_SESSIONS:usize=4;
pub const NUMBER_LIMIT:u64=9007199254740991;
pub fn utc_ms()->u64{SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis().min(NUMBER_LIMIT as u128)as u64}
pub fn tag(value:&str)->String{use sha2::{Digest,Sha256};Sha256::digest(value.as_bytes())[..8].iter().map(|b|format!("{b:02x}")).collect()}
pub fn game_tag(path:&Path)->String{tag(&path.to_string_lossy().trim_start_matches(r"\\?\").replace('/',"\\").trim_end_matches('\\').to_lowercase())}
pub fn runtime_id(value:&str)->bool{!value.is_empty()&&value.len()<=128&&value.bytes().all(|b|b.is_ascii_alphanumeric()||b"-_.".contains(&b))}
pub fn hex(value:&str,n:usize)->bool{value.len()==n&&value.bytes().all(|b|b.is_ascii_digit()||(b'a'..=b'f').contains(&b))}
#[derive(Clone,Default,PartialEq,Eq,Serialize,Deserialize)]
#[serde(rename_all="camelCase",deny_unknown_fields)]
pub struct Scope{pub service_tag:Option<String>,pub game_tag:Option<String>,pub owner_tag:Option<String>,pub profile_tag:Option<String>}
impl Scope{fn valid(&self)->bool{[&self.service_tag,&self.game_tag,&self.owner_tag,&self.profile_tag].into_iter().all(|v|v.as_ref().is_none_or(|s|hex(s,16)))}}
#[derive(Clone,Copy,Default,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModEvent{pub frame:u64,pub time_ms:u64,pub kind:u8,pub playing:bool,pub copied:i32,pub roster:u32,pub poses:u32,pub canonical_roster:Option<u32>}
#[derive(Clone,Default,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModReport{
 pub revision:u8,pub enabled:bool,pub logger:bool,pub frame:u64,pub time_ms:u64,
 pub peer_copies:u64,pub peer_unavailable:u64,pub peer_denied:u64,pub roster:u32,pub poses:u32,
 pub social_frames:u64,pub hud_samples:u64,pub hud_copied:u64,pub hud_received:u64,pub hud_refused:u64,
 pub poi_copied:u64,pub poi_refused:u64,pub social_errors:u64,pub movement_errors:u64,
 pub first_loss:Option<ModEvent>,pub recent:Vec<ModEvent>,pub truncated:bool,
 pub canonical_roster:Option<u32>,
 #[serde(default)]pub before_loss:Vec<ModEvent>,#[serde(default)]pub after_loss:Vec<ModEvent>,pub recovery:Option<ModEvent>,
}
impl ModReport{
 pub fn decode(bytes:&[u8])->Option<Self>{
  if bytes.is_empty()||bytes.len()>4096{return None}let v:Self=serde_json::from_slice(bytes).ok()?;
  let event=|e:&ModEvent|e.frame<=NUMBER_LIMIT&&e.time_ms<=NUMBER_LIMIT&&e.kind<=7&&(-1..=864).contains(&e.copied)&&e.roster<=32&&e.poses<=4&&e.canonical_roster.is_none_or(|n|n<=32);
  if v.revision!=1||v.roster>32||v.poses>4||v.canonical_roster.is_some_and(|n|n>32)||v.recent.len()>8||v.first_loss.as_ref().is_some_and(|e|!event(e))||v.recent.iter().any(|e|!event(e))||[v.frame,v.time_ms,v.peer_copies,v.peer_unavailable,v.peer_denied,v.social_frames,v.hud_samples,v.hud_copied,v.hud_received,v.hud_refused,v.poi_copied,v.poi_refused,v.social_errors,v.movement_errors].into_iter().any(|n|n>NUMBER_LIMIT){return None}
  if v.before_loss.len()>4||v.after_loss.len()>4||v.before_loss.iter().chain(v.after_loss.iter()).any(|e|!event(e))||v.recovery.as_ref().is_some_and(|e|!event(e)){return None}
  if (!v.enabled||!v.logger)&&(v.first_loss.is_some()||!v.recent.is_empty()||!v.before_loss.is_empty()||!v.after_loss.is_empty()||v.recovery.is_some()){return None}Some(v)
 }
}
pub const FIELDS:&[&str]=&["control_failures","movement_failures","live_failures","peer_attempts","missed","remote_age_p95_ms","receive_gap_p95_ms","canonical_roster","state","cached_peers","wire_peers","accepted_peers","native_roster","snapshot_age_ms","publication_attempts","publications","receives","max_publication_gap_ms","max_receive_gap_ms","hud_attempts","hud_successes","hud_refusals","hud_rows","information_age_ms","information_queue_ms","poi_age_ms","rescans","inputs_received","inputs_acknowledged","inputs_rejected","pending_age_ms","frame","time_ms","kind","copied","roster","poses","peer_copies","peer_unavailable","peer_denied","social_frames","hud_samples","hud_copied","hud_received","hud_refused","poi_copied","poi_refused","social_errors","movement_errors","local_authenticated","local_refused","local_requests","local_delivered","local_stale","local_queued","local_max_queued","local_max_latency_us","local_generation","local_closed","local_close_reason","local_last_request","local_last_kind"];
pub const FLAGS:&[&str]=&["shared_rules_bootstrap","shared_rules_metadata_ready","shared_rules_epoch_matches","playing","authority_valid","input_pending","stream_aborted","stream_ready","stream_rescan","information_pending","producer_pending","poi_pending","canonical_empty","expected_departure"];
#[derive(Clone,Serialize,Deserialize)]
#[serde(rename_all="camelCase",deny_unknown_fields)]
pub struct Record{pub sequence:u64,pub utc_ms:u64,pub monotonic_ms:u64,pub kind:String,pub fields:BTreeMap<String,u64>,pub flags:BTreeMap<String,bool>,pub scope:Scope}
impl Record{
 fn valid(&self)->bool{self.scope.valid()&&self.fields.len()<=FIELDS.len()&&self.flags.len()<=FLAGS.len()&&self.fields.iter().all(|(k,v)|FIELDS.contains(&k.as_str())&&*v<=NUMBER_LIMIT)&&self.flags.keys().all(|k|FLAGS.contains(&k.as_str()))&&["sample","first_loss","recovery","expected_departure","scope_change","mod_sample","mod_first_loss","mod_recovery","mod_error"].contains(&self.kind.as_str())}
}
#[derive(Clone,Serialize,Deserialize)]
#[serde(rename_all="camelCase",deny_unknown_fields)]
pub struct Envelope{
 pub revision:String,pub component:String,pub runtime_id:String,pub runtime_sha256:String,pub session_id:String,
 pub scope:Scope,pub started_at_utc_ms:u64,pub updated_at_utc_ms:u64,pub closed:bool,pub enabled:bool,pub reason:String,
 pub truncated:bool,pub dropped:u64,pub first_loss:Option<Record>,pub recovery:Option<Record>,pub before_loss:Vec<Record>,pub after_loss:Vec<Record>,pub recent:VecDeque<Record>,
}
#[derive(Clone,Serialize,Deserialize)]
#[serde(rename_all="camelCase",deny_unknown_fields)]
pub struct Manifest{pub revision:String,pub runtime_id:String,pub runtime_sha256:String,pub session_id:String,pub scope:Scope,pub updated_at_utc_ms:u64,pub closed:bool,pub enabled:bool,pub reason:String,pub app_file:String,pub mod_file:String,pub retained_sessions:usize,pub file_limit_bytes:usize,pub event_limit:usize,pub dropped:u64}
#[derive(Clone,Serialize,Deserialize)]
#[serde(rename_all="camelCase",deny_unknown_fields)]
pub struct Selection{pub revision:String,pub runtime_id:String,pub runtime_sha256:String,pub session_id:String,pub scope:Scope,pub updated_at_utc_ms:u64,pub enabled:bool,pub reason:String}
#[derive(Deserialize,Serialize)]#[serde(rename_all="camelCase",deny_unknown_fields)]struct Settings{debug_diagnostics:Option<bool>}
pub fn environment(value:Option<&str>)->Result<Option<bool>,()>{match value.map(str::to_ascii_lowercase).as_deref(){None=>Ok(None),Some("1"|"on"|"true")=>Ok(Some(true)),Some("0"|"off"|"false")=>Ok(Some(false)),_=>Err(())}}
pub fn choice(file:Option<bool>,env:Result<Option<bool>,()>,development:bool)->(bool,&'static str){
 if env.is_err(){return(false,"invalid-setting")}if file==Some(false)||env==Ok(Some(false)){return(false,"disabled")}
 (file==Some(true)||env==Ok(Some(true))||development,"enabled")
}
enum Message{Sample(Record,Option<ModReport>,Option<bank::BankData>),Setting(bool),Finish}
pub struct Handle{sender:mpsc::SyncSender<Message>,pub enabled:Arc<AtomicBool>,pub failed:Arc<AtomicBool>,dropped:Arc<AtomicU64>,clock:Instant,sequence:u64,last_sample:Instant,last_peers:u64,seen:bool,first:bool,scope:Scope,pending_first:Option<Message>,worker:Option<std::thread::JoinHandle<()>>}
impl Handle{
 pub fn start(root:PathBuf,config:PathBuf,runtime:String,runtime_sha:String,development:bool)->Option<Self>{
  // No producer-side filesystem work. Stay off until the writer has resolved
  // explicit opt-outs, including before the first queued sample.
  if !runtime_id(&runtime)||!hex(&runtime_sha,64){return None}
  let(tx,rx)=mpsc::sync_channel(8);let enabled=Arc::new(AtomicBool::new(false));let failed=Arc::new(AtomicBool::new(false));let dropped=Arc::new(AtomicU64::new(0));let(e,f,d)=(enabled.clone(),failed.clone(),dropped.clone());
  let worker=std::thread::Builder::new().name("nt2-debug-files".into()).spawn(move||{writer(root,config,runtime,runtime_sha,development,rx,e,f,d)}).ok()?;
  Some(Self{sender:tx,enabled,failed,dropped,clock:Instant::now(),sequence:0,last_sample:Instant::now()-Duration::from_secs(1),last_peers:0,seen:false,first:false,scope:Scope::default(),pending_first:None,worker:Some(worker)})
 }
 pub fn setting(&self,value:bool)->Result<(),&'static str>{self.sender.try_send(Message::Setting(value)).map_err(|_|"Diagnostic setting queue unavailable")?;if !value{self.enabled.store(false,Ordering::Release)}Ok(())}
 pub fn status(&self)->Value{serde_json::json!({"enabled":self.enabled.load(Ordering::Acquire),"write_failed":self.failed.load(Ordering::Acquire),"dropped":self.dropped.load(Ordering::Relaxed)})}
 pub fn observe(&mut self,view:&Value,scope:Scope,mod_report:Option<ModReport>){
  if !scope.valid(){return}let on=self.enabled.load(Ordering::Acquire);let now=Instant::now();let mut fields=BTreeMap::new();let mut flags=BTreeMap::new();
  let numeric=[("control_failures","/g3/control/failure/count"),("movement_failures","/g3/movement_failure/count"),("live_failures","/g3/live_transport/failures"),("peer_attempts","/g3/movement/peer_attempts"),("missed","/g3/movement/missed"),("remote_age_p95_ms","/g3/movement/remote_age_p95_ms"),("receive_gap_p95_ms","/g3/movement/receive_gap_p95_ms"),("cached_peers","/g3/movement_roster/cached_peers"),("snapshot_age_ms","/g3/movement_roster/snapshot_age_ms"),("publication_attempts","/g3/movement/publication_attempts"),("publications","/g3/movement/publications"),("receives","/g3/movement/receives"),("max_publication_gap_ms","/g3/movement/max_publication_gap_ms"),("max_receive_gap_ms","/g3/movement/max_receive_gap_ms"),("hud_attempts","/g3/player_information/hud_boundary/producer/attempts"),("hud_successes","/g3/player_information/hud_boundary/producer/successes"),("hud_refusals","/g3/player_information/hud_boundary/producer/refusals"),("hud_rows","/g3/player_information/hud_boundary/consumer/hud_rows"),("information_age_ms","/g3/player_information/information_age_ms"),("information_queue_ms","/g3/player_information/information_queue_ms"),("poi_age_ms","/g3/player_information/poi_age_ms"),("rescans","/g3/player_information/rescans"),("inputs_received","/g3/optional_stream/inputs_received"),("inputs_acknowledged","/g3/optional_stream/inputs_acknowledged"),("inputs_rejected","/g3/optional_stream/inputs_rejected"),("pending_age_ms","/g3/optional_stream/pending_age_ms")];
  for(k,p)in numeric{if let Some(n)=view.pointer(p).and_then(Value::as_u64){fields.insert(k.into(),n.min(NUMBER_LIMIT));}}
  // Transport counters are numeric only; descriptors, proofs and capabilities never enter captures.
  for key in ["authenticated","refused","requests","delivered","stale","queued","max_queued","max_latency_us","generation","closed","close_reason","last_request","last_kind"]{if let Some(n)=view.pointer(&format!("/native_connection/local_bridge/listener/{key}")).and_then(Value::as_u64){fields.insert(format!("local_{key}"),n.min(NUMBER_LIMIT));}}
  let state=view.pointer("/connection/state").and_then(Value::as_str).unwrap_or("");let playing=state=="Playing";flags.insert("playing".into(),playing);flags.insert("authority_valid".into(),view.pointer("/g3/control/current/current_validated")==Some(&Value::Bool(true)));
  for(k,p)in[("shared_rules_bootstrap","/rejoin/shared_rules/bootstrap"),("shared_rules_metadata_ready","/rejoin/shared_rules/metadata_ready"),("shared_rules_epoch_matches","/rejoin/shared_rules/native_epoch_matches"),("input_pending","/g3/optional_stream/input_pending"),("stream_aborted","/g3/optional_stream/aborted"),("stream_ready","/g3/optional_stream/ready"),("stream_rescan","/g3/optional_stream/rescan"),("information_pending","/g3/player_information/information_pending"),("producer_pending","/g3/player_information/producer_pending"),("poi_pending","/g3/player_information/poi_pending")]{if let Some(b)=view.pointer(p).and_then(Value::as_bool){flags.insert(k.into(),b);}}
  fields.insert("state".into(),["Playing","Ready","Reconnecting","Protected","Choice","Ended","Offline","Disconnected"].iter().position(|s|*s==state).map_or(99,|n|n as u64));
  if let Some(pending)=self.pending_first.take(){if let Err(mpsc::TrySendError::Full(message))=self.sender.try_send(pending){self.pending_first=Some(message);}}
  let canonical=view.pointer("/g3/debug_canonical_peers").and_then(Value::as_u64);if let Some(n)=canonical{fields.insert("canonical_roster".into(),n.min(32));}
  let peers=*fields.get("cached_peers").unwrap_or(&0);let changed=self.scope!=scope;
  let mut kind="sample";if changed{self.seen=false;self.first=false;self.last_peers=0;kind="scope_change";self.scope=scope.clone();}
  if on&&playing&&self.seen&&self.last_peers>0&&peers==0&&canonical!=Some(0){kind=if self.first{"sample"}else{self.first=true;"first_loss"};}
  else if on&&peers>0&&self.seen&&self.last_peers==0{kind="recovery";}
  else if on&&(matches!(state,"Ended"|"Offline")||canonical==Some(0))&&self.last_peers>0{kind="expected_departure";flags.insert("expected_departure".into(),true);}
  self.seen|=peers>0;self.last_peers=peers;
  if on&&kind=="sample"&&now.duration_since(self.last_sample)<Duration::from_secs(1)&&mod_report.is_none(){return}self.last_sample=now;self.sequence=self.sequence.saturating_add(1);
  let r=Record{sequence:self.sequence,utc_ms:utc_ms(),monotonic_ms:self.clock.elapsed().as_millis().min(NUMBER_LIMIT as u128)as u64,kind:kind.into(),fields:if on{fields}else{BTreeMap::new()},flags:if on{flags}else{BTreeMap::new()},scope};
  let important=r.kind=="first_loss"||mod_report.as_ref().is_some_and(|m|m.first_loss.is_some());
  if let Err(mpsc::TrySendError::Full(message))=self.sender.try_send(Message::Sample(r,mod_report,on.then(||bank::snapshot(view)))){if important&&self.pending_first.is_none(){self.pending_first=Some(message);}self.dropped.fetch_add(1,Ordering::Relaxed);}
 }
 pub fn finish(&mut self){if let Some(worker)=self.worker.take(){if let Some(message)=self.pending_first.take(){let _=self.sender.send(message);}let _=self.sender.send(Message::Finish);let _=worker.join();}}
}
impl Drop for Handle{fn drop(&mut self){self.finish()}}
fn plain(path:&Path)->bool{let mut p=Some(path);while let Some(v)=p{if let Ok(m)=std::fs::symlink_metadata(v){if m.file_type().is_symlink(){return false}#[cfg(windows)]{use std::os::windows::fs::MetadataExt;if m.file_attributes()&0x400!=0{return false}}}p=v.parent();}true}
fn atomic(path:&Path,value:&impl Serialize)->std::io::Result<()>{
 if !plain(path){return Err(std::io::Error::other("linked diagnostics refused"))}let bytes=serde_json::to_vec(value).map_err(std::io::Error::other)?;if bytes.len()>MAX_FILE{return Err(std::io::Error::other("diagnostic file bound"))}
 let parent=path.parent().ok_or_else(||std::io::Error::other("diagnostic parent"))?;std::fs::create_dir_all(parent)?;let tmp=parent.join(format!(".{}.tmp",uuid::Uuid::new_v4()));
 let result=(||{use std::io::Write;let mut f=std::fs::OpenOptions::new().write(true).create_new(true).open(&tmp)?;f.write_all(&bytes)?;drop(f);std::fs::rename(&tmp,path)})();if result.is_err(){let _=std::fs::remove_file(tmp);}result
}
fn settings(path:&Path)->Result<Option<bool>,()>{if !plain(path){return Err(())}match std::fs::metadata(path){Err(e)if e.kind()==std::io::ErrorKind::NotFound=>Ok(None),Err(_)=>Err(()),Ok(m)if m.len()>16384=>Err(()),Ok(_)=>serde_json::from_slice::<Settings>(&std::fs::read(path).map_err(|_|())?).map(|v|v.debug_diagnostics).map_err(|_|())}}
fn envelope(component:&str,runtime:&str,sha:&str,session:&str)->Envelope{let now=utc_ms();Envelope{revision:REVISION.into(),component:component.into(),runtime_id:runtime.into(),runtime_sha256:sha.into(),session_id:session.into(),scope:Scope::default(),started_at_utc_ms:now,updated_at_utc_ms:now,closed:false,enabled:true,reason:"enabled".into(),truncated:false,dropped:0,first_loss:None,recovery:None,before_loss:Vec::new(),after_loss:Vec::new(),recent:VecDeque::new()}}
fn push(e:&mut Envelope,r:Record){if !r.valid(){return}e.scope=r.scope.clone();e.updated_at_utc_ms=r.utc_ms;
 if (r.kind=="first_loss"||r.kind=="mod_first_loss")&&e.first_loss.is_none(){e.before_loss=e.recent.iter().rev().take(16).cloned().collect::<Vec<_>>().into_iter().rev().collect();e.first_loss=Some(r.clone());}
 else if e.first_loss.is_some()&&e.after_loss.len()<16{e.after_loss.push(r.clone());}
 if r.kind=="recovery"||r.kind=="mod_recovery"{e.recovery=Some(r.clone());}if e.recent.len()>=MAX_EVENTS{e.recent.pop_front();e.truncated=true;}e.recent.push_back(r);}
fn mod_record(report:ModReport,base:&Record)->Record{
 let mut r=base.clone();r.kind=if report.social_errors+report.movement_errors>0{"mod_error"}else{"mod_sample"}.into();r.fields.clear();r.flags.clear();
 for(k,n)in[("frame",report.frame),("time_ms",report.time_ms),("peer_copies",report.peer_copies),("peer_unavailable",report.peer_unavailable),("peer_denied",report.peer_denied),("roster",u64::from(report.roster)),("poses",u64::from(report.poses)),("social_frames",report.social_frames),("hud_samples",report.hud_samples),("hud_copied",report.hud_copied),("hud_received",report.hud_received),("hud_refused",report.hud_refused),("poi_copied",report.poi_copied),("poi_refused",report.poi_refused),("social_errors",report.social_errors),("movement_errors",report.movement_errors)]{r.fields.insert(k.into(),n);}
 if let Some(n)=report.canonical_roster{r.fields.insert("canonical_roster".into(),u64::from(n));}r
}
fn mod_event(r:&mut Record,event:ModEvent){for(k,n)in[("frame",event.frame),("time_ms",event.time_ms),("kind",u64::from(event.kind)),("copied",(event.copied+1)as u64),("roster",u64::from(event.roster)),("poses",u64::from(event.poses))]{r.fields.insert(k.into(),n);}r.flags.insert("playing".into(),event.playing);if let Some(n)=event.canonical_roster{r.fields.insert("canonical_roster".into(),u64::from(n));}}
fn active(path:&Path)->std::io::Result<std::fs::File>{
 if !plain(path){return Err(std::io::Error::other("linked diagnostics refused"))}
 let mut options=std::fs::OpenOptions::new();options.read(true).write(true).create(true).truncate(false);
 #[cfg(windows)]{use std::os::windows::fs::OpenOptionsExt;options.share_mode(0);}
 options.open(path)
}
// Delete only fixed files of our validated sessions, after taking the same
// exclusive handle used by a live writer. Unknown directories are never ours.
fn rotate(root:&Path,keep:&Path)->bool{
 if !plain(root){return false}let Ok(entries)=std::fs::read_dir(root)else{return true};
 let mut dirs=entries.filter_map(Result::ok).filter(|e|e.path()!=keep&&e.file_name().to_str().is_some_and(|s|uuid::Uuid::parse_str(s).is_ok())&&e.file_type().is_ok_and(|t|t.is_dir())&&plain(&e.path())).collect::<Vec<_>>();
 dirs.sort_by_key(|e|e.metadata().and_then(|m|m.modified()).ok());let mut count=dirs.len();
 for entry in dirs{if count<MAX_SESSIONS{break}let dir=entry.path();
  let Ok(data)=std::fs::read(dir.join("manifest.json"))else{continue};if data.len()>MAX_FILE{continue}
  let Ok(manifest)=serde_json::from_slice::<Manifest>(&data)else{continue};
  if manifest.revision!=REVISION||Some(manifest.session_id.as_str())!=entry.file_name().to_str()||Some(manifest.runtime_id.as_str())!=root.file_name().and_then(|s|s.to_str()){continue}
  let Ok(files)=std::fs::read_dir(&dir)else{continue};if files.filter_map(Result::ok).any(|f|!plain(&f.path())||!f.file_type().is_ok_and(|t|t.is_file())||!f.file_name().to_str().is_some_and(|n|[".active","manifest.json","app.json","mod.json","bank.json"].contains(&n))){continue}
  let Ok(lock)=active(&dir.join(".active"))else{continue};drop(lock);
  for name in ["app.json","mod.json","bank.json","manifest.json",".active"]{let _=std::fs::remove_file(dir.join(name));}
  if std::fs::remove_dir(&dir).is_ok(){count-=1;}
 }count<MAX_SESSIONS
}
fn persist_session(dir:&Path,runtime_root:&Path,app:&Envelope,module:&mut Envelope,lock:&mut Option<std::fs::File>)->std::io::Result<()>{
 if app.recent.is_empty(){return Ok(())}
 if lock.is_none(){if !rotate(runtime_root,dir){return Err(std::io::Error::other("active session capacity"))}if !plain(dir){return Err(std::io::Error::other("linked diagnostics refused"))}std::fs::create_dir_all(dir)?;*lock=Some(active(&dir.join(".active"))?);}
 module.scope=app.scope.clone();module.closed=app.closed;module.updated_at_utc_ms=app.updated_at_utc_ms;
 let manifest=Manifest{revision:REVISION.into(),runtime_id:app.runtime_id.clone(),runtime_sha256:app.runtime_sha256.clone(),session_id:app.session_id.clone(),scope:app.scope.clone(),updated_at_utc_ms:app.updated_at_utc_ms,closed:app.closed,enabled:app.enabled,reason:app.reason.clone(),app_file:"app.json".into(),mod_file:"mod.json".into(),retained_sessions:MAX_SESSIONS,file_limit_bytes:MAX_FILE,event_limit:MAX_EVENTS,dropped:app.dropped};
 atomic(&dir.join("app.json"),app)?;atomic(&dir.join("mod.json"),module)?;atomic(&dir.join("manifest.json"),&manifest)
}
fn persist_bank_session(dir:&Path,root:&Path,app:&Envelope,module:&mut Envelope,lock:&mut Option<std::fs::File>,data:Option<&bank::BankData>)->std::io::Result<()>{
 persist_session(dir,root,app,module,lock)?;
 if lock.is_some(){if let Some(data)=data{atomic(&dir.join("bank.json"),&bank::BankFile{revision:"nt2-bank-debug-1".into(),runtime_id:app.runtime_id.clone(),runtime_sha256:app.runtime_sha256.clone(),session_id:app.session_id.clone(),scope:app.scope.clone(),data:data.clone()})?;}}Ok(())
}
fn writer(root:PathBuf,config:PathBuf,runtime:String,sha:String,development:bool,rx:mpsc::Receiver<Message>,enabled:Arc<AtomicBool>,failed:Arc<AtomicBool>,dropped:Arc<AtomicU64>){
 let mut session=uuid::Uuid::new_v4().to_string();let runtime_root=root.join("app").join(&runtime);let mut dir=runtime_root.join(&session);let mut app=envelope("app",&runtime,&sha,&session);let mut module=envelope("mod",&runtime,&sha,&session);module.reason="missing-mod".into();module.enabled=false;let env=environment(std::env::var("NT2_DEBUG_DIAGNOSTICS").ok().as_deref());let mut last_config=Instant::now()-Duration::from_secs(2);let mut last_flush=last_config;let mut finish=false;let mut lock=None;let mut selection_scope=Scope::default();let mut dirty=false;let mut bank_data:Option<bank::BankData>=None;
 // Captures allow 60 seconds of age; idle selection metadata needs only a 10-second heartbeat.
 let mut selection_key:Option<(Scope,String,bool,String)>=None;let mut selection_written=Instant::now();
 loop{
  if last_config.elapsed()>=Duration::from_secs(1){let selected=settings(&config);let(on,reason)=match selected{Ok(v)=>choice(v,env,development),Err(_)=>(false,"invalid-setting")};enabled.store(on,Ordering::Release);dirty|=app.enabled!=on||app.reason!=reason;app.enabled=on;app.reason=reason.into();if !on{module.enabled=false;module.reason=reason.into();}last_config=Instant::now();}
  match rx.recv_timeout(Duration::from_millis(100)){
   Ok(Message::Setting(value))=>{if atomic(&config,&Settings{debug_diagnostics:Some(value)}).is_err(){failed.store(true,Ordering::Release)}last_config=Instant::now()-Duration::from_secs(2);},
   Ok(Message::Sample(r,m,b))=>{
    selection_scope=r.scope.clone();
    if app.scope!=r.scope&&!app.recent.is_empty(){
     app.closed=true;app.updated_at_utc_ms=utc_ms();
     if persist_bank_session(&dir,&runtime_root,&app,&mut module,&mut lock,bank_data.as_ref()).is_err(){failed.store(true,Ordering::Release)}
     let effective=(app.enabled,app.reason.clone());lock=None;session=uuid::Uuid::new_v4().to_string();dir=runtime_root.join(&session);app=envelope("app",&runtime,&sha,&session);app.enabled=effective.0;app.reason=effective.1;module=envelope("mod",&runtime,&sha,&session);module.reason="missing-mod".into();module.enabled=false;dirty=false;bank_data=None;
    }
    app.scope=r.scope.clone();if !r.fields.is_empty(){bank_data=b;}
    // A queued first loss collected before opt-out must still reach disk. Off
    // producers send empty fields, so this never creates fresh disabled samples.
    if !r.fields.is_empty(){dirty=true;push(&mut app,r.clone());if let Some(m)=m{
     let recorded=m.enabled&&m.logger;module.enabled=app.enabled&&recorded;module.reason=if !app.enabled{app.reason.as_str()}else if !m.logger{"logger-opt-out"}else if !m.enabled{"disabled"}else{"enabled"}.into();module.truncated|=m.truncated;
     if recorded{
      if module.first_loss.is_none(){if let Some(first)=m.first_loss{
       for event in &m.before_loss{let mut record=r.clone();record.kind="mod_sample".into();record.fields.clear();record.flags.clear();mod_event(&mut record,*event);push(&mut module,record);}
       let mut record=r.clone();record.kind="mod_first_loss".into();record.fields.clear();record.flags.clear();mod_event(&mut record,first);push(&mut module,record);
      }}
      // First report carries the small surrounding ring. Later reports append
      // only events newer than the last persisted game frame.
      let mut high=module.recent.iter().filter_map(|v|v.fields.get("frame")).copied().max();
      for event in m.after_loss.iter().chain(m.recent.iter()){if high.is_none_or(|h|event.frame>h){let mut record=r.clone();record.kind=match event.kind{2=>"mod_recovery",6|7=>"mod_error",_=>"mod_sample"}.into();record.fields.clear();record.flags.clear();mod_event(&mut record,*event);push(&mut module,record);high=Some(event.frame);}}
      if module.recovery.is_none(){if let Some(event)=m.recovery{let mut record=r.clone();record.kind="mod_recovery".into();record.fields.clear();record.flags.clear();mod_event(&mut record,event);push(&mut module,record);}}
      push(&mut module,mod_record(m,&r));
     }
    }}},
   Ok(Message::Finish)|Err(mpsc::RecvTimeoutError::Disconnected)=>finish=true,
   Err(mpsc::RecvTimeoutError::Timeout)=>{},
  }
  if last_flush.elapsed()>=Duration::from_millis(500)||finish{
   let now=utc_ms();app.updated_at_utc_ms=now;module.updated_at_utc_ms=now;app.closed=finish;module.closed=finish;app.dropped=dropped.load(Ordering::Relaxed);module.scope=app.scope.clone();
   let result=(||{
    // Off creates only essential current-selection metadata, never a new
    // gameplay session or new samples. Previously recorded sessions survive.
    if dirty||finish{persist_bank_session(&dir,&runtime_root,&app,&mut module,&mut lock,bank_data.as_ref())?;}
    let key=(selection_scope.clone(),session.clone(),app.enabled,app.reason.clone());
     if finish||selection_key.as_ref()!=Some(&key)||selection_written.elapsed()>=Duration::from_secs(10){
      atomic(&runtime_root.join("selection.json"),&Selection{revision:REVISION.into(),runtime_id:runtime.clone(),runtime_sha256:sha.clone(),session_id:session.clone(),scope:selection_scope.clone(),updated_at_utc_ms:now,enabled:app.enabled,reason:app.reason.clone()})?;
      selection_key=Some(key);selection_written=Instant::now();
     }
     Ok::<(),std::io::Error>(())
   })();
   if result.is_err(){failed.store(true,Ordering::Release)}else{dirty=false}last_flush=Instant::now();
  }if finish{break}
 }
}

#[cfg(test)]mod tests{
 use super::*;
 #[test]fn websocket_samples_persist_after_four_unclosed_old_sessions_and_match_export_schema(){
  let root=root();let config=root.join("settings.json");
  for _ in 0..MAX_SESSIONS{let mut h=recording(root.clone(),config.clone(),"test".into(),"b".repeat(64),true).unwrap();h.observe(&view("Playing",1),scope(),None);h.finish();let file=session(&root).join("manifest.json");let mut m:Manifest=serde_json::from_slice(&std::fs::read(&file).unwrap()).unwrap();m.closed=false;atomic(&file,&m).unwrap();}
  let mut current=view("Playing",1);let keys=["authenticated","refused","requests","delivered","stale","queued","max_queued","max_latency_us","generation","closed","close_reason","last_request","last_kind"];
  let mut listener=serde_json::Map::new();for key in keys{listener.insert(key.into(),Value::from(7));}listener.insert("secret".into(),Value::from("must-not-persist"));current["native_connection"]=serde_json::json!({"local_bridge":{"listener":listener}});
  let mut h=recording(root.clone(),config,"test".into(),"a".repeat(64),true).unwrap();h.observe(&current,scope(),None);h.finish();assert!(!h.failed.load(Ordering::Acquire));
  let selection:Selection=serde_json::from_slice(&std::fs::read(root.join("app/test/selection.json")).unwrap()).unwrap();let bytes=std::fs::read(root.join("app/test").join(selection.session_id).join("app.json")).unwrap();let app:Envelope=serde_json::from_slice(&bytes).unwrap();assert!(app.closed&&app.recent.len()==1);assert_eq!(app.runtime_sha256,"a".repeat(64));assert!(!String::from_utf8(bytes).unwrap().contains("must-not-persist"));for key in keys{assert_eq!(app.recent[0].fields.get(&format!("local_{key}")),Some(&7));}
  let schema:Value=serde_json::from_str(include_str!("../../tools/client-debug.schema.json")).unwrap();let props=&schema["$defs"]["envelope"]["properties"];
  for record in [&props["firstLoss"]["anyOf"][0],&props["recovery"]["anyOf"][0],&props["recent"]["items"],&props["beforeLoss"]["items"],&props["afterLoss"]["items"]]{
   for(name,expected)in[("fields",FIELDS),("flags",FLAGS)]{let shape=&record["properties"][name];assert_eq!(shape["additionalProperties"],false);let actual=shape["properties"].as_object().unwrap();assert_eq!(actual.len(),expected.len());assert!(expected.iter().all(|key|actual.contains_key(*key)));}
  }
 }
 fn recording(root:PathBuf,config:PathBuf,runtime:String,sha:String,development:bool)->Option<Handle>{
  let h=Handle::start(root,config,runtime,sha,development)?;let until=Instant::now()+Duration::from_secs(3);while !h.enabled.load(Ordering::Acquire){assert!(Instant::now()<until,"writer setting readiness");std::thread::sleep(Duration::from_millis(1));}Some(h)
 }
 #[test]fn shared_rules_bootstrap_diagnostics_remain_bounded_and_scoped(){
  let root=root();let mut h=recording(root.clone(),root.join("settings.json"),"test".into(),"a".repeat(64),true).unwrap();
  let mut v=view("Ready",0);v["rejoin"]=serde_json::json!({"shared_rules":{"bootstrap":true,"metadata_ready":true,"native_epoch_matches":false}});h.observe(&v,scope(),None);h.finish();
  let app:Envelope=serde_json::from_slice(&std::fs::read(session(&root).join("app.json")).unwrap()).unwrap();let row=app.recent.back().unwrap();assert_eq!(row.scope.owner_tag,scope().owner_tag);
  for(k,value)in[("shared_rules_bootstrap",true),("shared_rules_metadata_ready",true),("shared_rules_epoch_matches",false)]{assert_eq!(row.flags.get(k),Some(&value));}assert!(row.valid());
 }
 fn root()->PathBuf{let p=PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../spikes/noita-entry/.build/automatic-debug-20261023-01").join(uuid::Uuid::new_v4().to_string());std::fs::create_dir_all(&p).unwrap();p}
 fn scope()->Scope{Scope{service_tag:Some("a".repeat(16)),game_tag:Some("b".repeat(16)),profile_tag:Some("c".repeat(16)),owner_tag:Some("d".repeat(16))}}
 fn view(state:&str,peers:u64)->Value{serde_json::json!({"connection":{"state":state},"g3":{"movement_roster":{"cached_peers":peers}}})}
 fn session(path:&Path)->PathBuf{let s:Selection=serde_json::from_slice(&std::fs::read(path.join("app/test/selection.json")).unwrap()).unwrap();path.join("app/test").join(s.session_id)}
 #[test]fn deliberate_false_and_invalid_settings_win(){
  for file in [None,Some(false),Some(true)]{for env in [Ok(None),Ok(Some(false)),Ok(Some(true))]{assert_eq!(choice(file,env,true).0,file!=Some(false)&&env!=Ok(Some(false)));}}
  assert!(!choice(None,Err(()),true).0);assert!(!choice(None,Ok(None),false).0);assert!(choice(Some(true),Ok(None),false).0);
  for value in ["0","off","false"]{assert_eq!(environment(Some(value)),Ok(Some(false)));}assert!(environment(Some("raw-secret")).is_err());
 }
 #[test]fn persisted_first_loss_recovery_and_privacy_survive_restart(){
  let root=root();let config=root.join("debug-settings.json");atomic(&config,&Settings{debug_diagnostics:Some(true)}).unwrap();
  let mut h=recording(root.clone(),config.clone(),"test".into(),"a".repeat(64),true).unwrap();
  h.observe(&view("Playing",2),scope(),None);h.observe(&view("Playing",0),scope(),None);h.observe(&view("Playing",2),scope(),None);h.observe(&view("Ended",0),scope(),None);h.finish();
  let old=session(&root);let data=std::fs::read(old.join("app.json")).unwrap();let e:Envelope=serde_json::from_slice(&data).unwrap();assert_eq!(e.first_loss.unwrap().kind,"first_loss");assert_eq!(e.recovery.unwrap().kind,"recovery");assert!(e.closed);assert!(e.recent.iter().any(|r|r.kind=="expected_departure"));
  let mut h=recording(root.clone(),config,"test".into(),"a".repeat(64),true).unwrap();let mut v=view("Playing",1);v["credentials"]=serde_json::json!({"token":"DO_NOT_EXPORT","chat":"DO_NOT_EXPORT"});h.observe(&v,scope(),None);h.finish();assert!(old.join("app.json").is_file());assert!(!String::from_utf8(std::fs::read(session(&root).join("app.json")).unwrap()).unwrap().contains("DO_NOT_EXPORT"));
 }
 #[test]fn normal_departure_is_not_unexpected_loss_and_scope_isolated(){
  let root=root();let mut h=recording(root.clone(),root.join("settings.json"),"test".into(),"a".repeat(64),true).unwrap();h.observe(&view("Playing",2),scope(),None);h.observe(&view("Offline",0),scope(),None);h.finish();let e:Envelope=serde_json::from_slice(&std::fs::read(session(&root).join("app.json")).unwrap()).unwrap();assert!(e.first_loss.is_none());assert!(e.recent.iter().any(|r|r.kind=="expected_departure"));
 }
 #[test]fn idle_selection_heartbeat_and_scope_change_are_bounded(){
  let root=root();let config=root.join("settings.json");atomic(&config,&Settings{debug_diagnostics:Some(false)}).unwrap();
  let mut h=Handle::start(root.clone(),config,"test".into(),"a".repeat(64),true).unwrap();let path=root.join("app/test/selection.json");
  let read=||std::fs::read(&path).ok().and_then(|b|serde_json::from_slice::<Selection>(&b).ok());
  let wait=|predicate:&dyn Fn(&Selection)->bool|{let until=Instant::now()+Duration::from_secs(12);loop{if let Some(s)=read(){if predicate(&s){break s}}assert!(Instant::now()<until,"selection metadata not published");std::thread::sleep(Duration::from_millis(20));}};
  h.observe(&view("Playing",1),scope(),None);let first=wait(&|s|s.scope==scope());
  std::thread::sleep(Duration::from_millis(1200));assert_eq!(read().unwrap().updated_at_utc_ms,first.updated_at_utc_ms,"idle selection rewritten before heartbeat");
  let heartbeat=wait(&|s|s.updated_at_utc_ms>first.updated_at_utc_ms);assert!(heartbeat.updated_at_utc_ms-first.updated_at_utc_ms>=10000);assert!(heartbeat.updated_at_utc_ms-first.updated_at_utc_ms<60000);
  let mut other=scope();other.profile_tag=Some("e".repeat(16));h.observe(&view("Playing",1),other.clone(),None);let began=Instant::now();let changed=wait(&|s|s.scope==other);assert!(began.elapsed()<Duration::from_secs(2));assert!(!changed.enabled);
  h.finish();assert!(!session(&root).exists());assert!(read().unwrap().scope==other);
 }
 #[test]fn off_writes_only_selection_and_unwritable_does_not_block_producer(){
  let root=root();let config=root.join("settings.json");atomic(&config,&Settings{debug_diagnostics:Some(false)}).unwrap();let mut h=Handle::start(root.clone(),config,"test".into(),"a".repeat(64),true).unwrap();h.observe(&view("Playing",1),scope(),None);h.finish();assert!(!session(&root).exists());let s:Selection=serde_json::from_slice(&std::fs::read(root.join("app/test/selection.json")).unwrap()).unwrap();assert!(!s.enabled);assert_eq!(s.scope.profile_tag,scope().profile_tag);
  let blocked=root.join("blocked");std::fs::write(&blocked,b"file").unwrap();let mut h=recording(blocked,root.join("none"),"test".into(),"a".repeat(64),true).unwrap();for _ in 0..128{h.observe(&view("Playing",1),scope(),None);}h.finish();assert!(h.failed.load(Ordering::Acquire));
 }
 #[test]fn rotation_bounded_and_active_writer_preserved(){
  let root=root();let config=root.join("settings.json");let mut live=recording(root.clone(),config.clone(),"test".into(),"a".repeat(64),true).unwrap();live.observe(&view("Playing",1),scope(),None);std::thread::sleep(Duration::from_millis(700));let live_path=session(&root);
  for _ in 0..6{let mut h=recording(root.clone(),config.clone(),"test".into(),"a".repeat(64),true).unwrap();h.observe(&view("Playing",1),scope(),None);h.finish();}
  assert!(live_path.join("app.json").exists());assert!(std::fs::read_dir(root.join("app/test")).unwrap().filter_map(Result::ok).filter(|e|e.file_type().unwrap().is_dir()).count()<=MAX_SESSIONS);live.finish();
 }
 #[test]fn optional_mod_report_rejects_raw_fields_and_out_of_bounds(){
  let report=ModReport{revision:1,enabled:true,logger:true,..Default::default()};let mut value=serde_json::to_value(report).unwrap();assert!(ModReport::decode(&serde_json::to_vec(&value).unwrap()).is_some());value["token"]=Value::String("secret".into());assert!(ModReport::decode(&serde_json::to_vec(&value).unwrap()).is_none());value.as_object_mut().unwrap().remove("token");value["poses"]=Value::from(5);assert!(ModReport::decode(&serde_json::to_vec(&value).unwrap()).is_none());assert!(ModReport::decode(&vec![b' ';4097]).is_none());
 }
 #[test]fn saturated_queue_keeps_first_loss_until_a_slot_is_available(){
  let(tx,rx)=mpsc::sync_channel(1);let mut h=Handle{sender:tx,enabled:Arc::new(AtomicBool::new(true)),failed:Arc::new(AtomicBool::new(false)),dropped:Arc::new(AtomicU64::new(0)),clock:Instant::now(),sequence:0,last_sample:Instant::now()-Duration::from_secs(1),last_peers:0,seen:false,first:false,scope:Scope::default(),pending_first:None,worker:None};
  h.observe(&view("Playing",1),scope(),None);h.observe(&view("Playing",0),scope(),None);assert!(h.pending_first.is_some());rx.try_recv().unwrap();h.observe(&view("Playing",1),scope(),None);
  let Message::Sample(record,_,_)=rx.try_recv().unwrap()else{panic!("sample required")};assert_eq!(record.kind,"first_loss");assert!(h.pending_first.is_none());assert!(h.dropped.load(Ordering::Relaxed)>0);
 }
 #[test]fn first_context_and_scope_are_retained_without_cross_profile_mixing(){
  let mut e=envelope("app","test",&"a".repeat(64),&uuid::Uuid::new_v4().to_string());
  for i in 0..100{push(&mut e,Record{sequence:i,utc_ms:i,monotonic_ms:i,kind:if i==8{"first_loss"}else if i==9{"recovery"}else{"sample"}.into(),fields:BTreeMap::new(),flags:BTreeMap::new(),scope:scope()});}
  assert_eq!(e.first_loss.unwrap().sequence,8);assert_eq!(e.before_loss.len(),8);assert_eq!(e.before_loss[0].sequence,0);assert_eq!(e.after_loss.len(),16);assert_eq!(e.recovery.unwrap().sequence,9);assert_eq!(e.recent.len(),64);assert!(e.truncated);
  let root=root();let mut h=recording(root.clone(),root.join("settings.json"),"test".into(),"a".repeat(64),true).unwrap();h.observe(&view("Playing",1),scope(),None);h.observe(&view("Playing",0),scope(),None);
  let mut other=scope();other.profile_tag=Some("e".repeat(16));h.observe(&view("Playing",1),other.clone(),None);h.finish();
  let current:Envelope=serde_json::from_slice(&std::fs::read(session(&root).join("app.json")).unwrap()).unwrap();assert_eq!(current.scope.profile_tag,other.profile_tag);assert!(current.first_loss.is_none());assert!(current.recent.iter().all(|r|r.scope==other));
 }
 #[test]fn shared_file_setting_can_disable_and_reenable_without_restart(){
  let root=root();let mut h=recording(root.clone(),root.join("settings.json"),"test".into(),"a".repeat(64),true).unwrap();h.observe(&view("Playing",1),scope(),None);h.setting(false).unwrap();assert!(!h.enabled.load(Ordering::Acquire));
  h.setting(true).unwrap();let until=Instant::now()+Duration::from_secs(3);while !h.enabled.load(Ordering::Acquire){assert!(Instant::now()<until);std::thread::sleep(Duration::from_millis(10));}
  h.observe(&view("Playing",0),scope(),None);h.finish();assert!(std::fs::read(root.join("settings.json")).is_ok());let e:Envelope=serde_json::from_slice(&std::fs::read(session(&root).join("app.json")).unwrap()).unwrap();assert!(e.enabled);
 }
 #[test]fn immediate_disable_flushes_preexisting_loss_and_closed_metadata(){
  let root=root();let mut h=recording(root.clone(),root.join("settings.json"),"test".into(),"a".repeat(64),true).unwrap();h.observe(&view("Playing",1),scope(),None);h.observe(&view("Playing",0),scope(),None);h.setting(false).unwrap();h.finish();
  let dir=session(&root);let app:Envelope=serde_json::from_slice(&std::fs::read(dir.join("app.json")).unwrap()).unwrap();let manifest:Manifest=serde_json::from_slice(&std::fs::read(dir.join("manifest.json")).unwrap()).unwrap();assert!(app.first_loss.is_some());assert_eq!(app.before_loss.len(),1);assert!(app.closed&&manifest.closed);assert!(!app.enabled&&!manifest.enabled);
 }
 #[test]fn scope_switch_while_off_keeps_setting_and_flushes_prior_scope(){
  let root=root();let mut h=recording(root.clone(),root.join("settings.json"),"test".into(),"a".repeat(64),true).unwrap();h.observe(&view("Playing",1),scope(),None);h.observe(&view("Playing",0),scope(),None);h.setting(false).unwrap();let mut other=scope();other.profile_tag=Some("e".repeat(16));h.observe(&view("Playing",1),other.clone(),None);h.finish();
  let selection:Selection=serde_json::from_slice(&std::fs::read(root.join("app/test/selection.json")).unwrap()).unwrap();assert!(!selection.enabled);assert_eq!(selection.scope.profile_tag,other.profile_tag);assert!(!session(&root).exists());
  let prior=std::fs::read_dir(root.join("app/test")).unwrap().filter_map(Result::ok).find(|e|e.file_type().unwrap().is_dir()).unwrap().path();let app:Envelope=serde_json::from_slice(&std::fs::read(prior.join("app.json")).unwrap()).unwrap();assert!(app.closed&&app.first_loss.is_some());assert_eq!(app.scope.profile_tag,scope().profile_tag);
 }
}

/// Safe report projection; arbitrary strings, session IDs and raw files never export.
pub fn safe_report(bytes:&[u8])->Option<Value>{if bytes.len()>MAX_FILE{return None}let e:Envelope=serde_json::from_slice(bytes).ok()?;if e.revision!=REVISION||!runtime_id(&e.runtime_id)||!hex(&e.runtime_sha256,64){return None}let records:Vec<_>=e.before_loss.iter().chain(e.after_loss.iter()).chain(e.recent.iter()).take(MAX_EVENTS*3).filter(|r|r.valid()).map(|r|serde_json::json!({"kind":r.kind,"sequence":r.sequence,"utcMs":r.utc_ms,"fields":r.fields,"flags":r.flags,"scope":r.scope})).collect();Some(serde_json::json!({"runtimeId":e.runtime_id,"runtimeSha256":e.runtime_sha256,"closed":e.closed,"enabled":e.enabled,"truncated":e.truncated,"dropped":e.dropped,"records":records}))}
