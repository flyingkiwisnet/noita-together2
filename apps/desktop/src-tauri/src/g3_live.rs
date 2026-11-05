//! Canonical live-room client. Core grants health; this lane grants no game authority.
use crate::{protocol_g3_live::{self as protocol,wire},g3_runtime::{MovementGrant,MovementState,Exchange}};
use std::{sync::{Arc,Mutex},time::{Duration,Instant}};
use futures_util::{SinkExt,StreamExt};
use tokio_tungstenite::{WebSocketStream,tungstenite::{Message,protocol::{Role,WebSocketConfig},handshake::derive_accept_key}};
use zeroize::Zeroizing;
use base64::Engine;
use crate::connection_time::{Deadline,Acknowledgements,Stamp,relative_age};
fn expiry(reason:&str)->String{if cfg!(debug_assertions)&&std::env::var("NT2_C2_TRACE").as_deref()==Ok("1"){eprintln!("C2 live expiry guard={reason}");}"EXPIRED".into()}
fn same<T:serde::Serialize>(a:&T,b:&T)->bool{serde_json::to_vec(a).ok()==serde_json::to_vec(b).ok()}
fn id(value:Option<&str>)->bool{value.is_some_and(|s|uuid::Uuid::parse_str(s).is_ok_and(|u|u.to_string()==s))}
fn coordinate(value:Option<&str>)->Result<i32,String>{let text=value.ok_or("INVALID_MESSAGE")?;let n=text.parse::<i32>().map_err(|_|"INVALID_MESSAGE")?;if n.to_string()!=text{return Err("INVALID_MESSAGE".into());}Ok(n)}
fn check_scope(scope:&wire::Scope)->Result<(),String>{
 if ![&scope.principal_id,&scope.session_id,&scope.session_epoch,&scope.room_id,&scope.run_id,&scope.run_epoch,&scope.attachment_id].into_iter().all(|v|id(v.as_deref()))||scope.config_revision.unwrap_or(0)==0||scope.world_token.as_ref().is_none_or(|v|v.is_empty()){return Err("INVALID_MESSAGE".into());}Ok(())
}
fn check_pose(p:&wire::Pose)->Result<(),String>{if p.sequence.unwrap_or(0)==0{return Err("INVALID_MESSAGE".into());}for v in [&p.x_milli,&p.y_milli,&p.vx_milli,&p.vy_milli]{coordinate(v.as_deref())?;}Ok(())}
struct Session{
 scope:wire::Scope,service:String,epoch:String,connection:String,generation:u32,authority:u32,
 outgoing:u32,incoming:u32,auth:u32,pending_auth:Option<(u32,Instant)>,access:Deadline,until:Deadline,
 acknowledgements:Acknowledgements,credential:String,expected_credential:Option<String>,last_ack:u64,retry_after_ms:u32,
}
impl Session{
 fn failure_diagnostic(&self,f:&wire::ServerFrame)->serde_json::Value{
  // Only fixed predicates, revisions and allowlisted protocol codes. Never
  // serialize the frame, its scope, credentials or access material.
  serde_json::json!({"source":if f.retire.is_some(){"service_retire"}else{"client_validation"},"retire_code":f.retire.as_ref().and_then(|r|r.code.as_deref()).filter(|c|matches!(*c,"INVALID_MESSAGE"|"UNAUTHENTICATED"|"FORBIDDEN"|"STALE_EPOCH"|"CAPACITY"|"EXPIRED"|"REAUTH_REQUIRED"|"RETIRED")),"header_matches":f.major==Some(1)&&f.revision.as_deref()==Some(protocol::REVISION)&&f.service_id.as_deref()==Some(&self.service)&&f.service_epoch.as_deref()==Some(&self.epoch)&&f.connection_id.as_deref()==Some(&self.connection)&&f.client_generation==Some(self.generation)&&same(&f.scope,&Some(self.scope.clone())),"sequence_newer":f.sequence.is_some_and(|n|n>self.incoming),"authority_generation":self.authority,"frame_authority_generation":f.authority_generation,"current_auth_revision":self.auth,"frame_auth_revision":f.welcome.as_ref().and_then(|w|w.auth_revision),"pending_auth_revision":self.pending_auth.map(|p|p.0),"pending_auth_timely":self.pending_auth.is_none_or(|p|Instant::now()<p.1),"credential_matches_current":f.credential_id.as_deref()==Some(&self.credential),"credential_matches_expected":self.expected_credential.as_ref().is_some_and(|c|f.credential_id.as_ref()==Some(c))})
 }
 fn new(scope:wire::Scope,service:String,epoch:String,generation:u32)->Result<Self,String>{
  check_scope(&scope)?;if service.is_empty()||!id(Some(&epoch))||generation==0{return Err("INVALID_MESSAGE".into());}
  Ok(Self{scope,service,epoch,connection:uuid::Uuid::new_v4().to_string(),generation,authority:0,outgoing:0,incoming:0,auth:0,pending_auth:None,access:Default::default(),until:Default::default(),acknowledgements:Default::default(),credential:String::new(),expected_credential:None,last_ack:0,retry_after_ms:0})
 }
 fn frame(&mut self)->Result<wire::ClientFrame,String>{
  self.outgoing=self.outgoing.checked_add(1).ok_or("RETIRED")?;
  Ok(wire::ClientFrame{major:Some(1),revision:Some(protocol::REVISION.into()),service_id:Some(self.service.clone()),service_epoch:Some(self.epoch.clone()),connection_id:Some(self.connection.clone()),client_generation:Some(self.generation),authority_generation:if self.authority==0{None}else{Some(self.authority)},sequence:Some(self.outgoing),scope:Some(self.scope.clone()),..Default::default()})
 }
 fn sent(&mut self,frame:&wire::ClientFrame,at:Instant)->Result<(),String>{
  self.acknowledgements.sent(u64::from(frame.sequence.ok_or("INVALID_MESSAGE")?),at)?;
  if let Some(token)=frame.authenticate.as_ref().and_then(|v|v.access_token.as_deref()).or_else(||frame.reauthenticate.as_ref().and_then(|v|v.access_token.as_deref())){self.expected_credential=Some(crate::protocol_connection::credential_id(token)?);}Ok(())
 }
 fn receive(&mut self,f:&wire::ServerFrame)->Result<Stamp,String>{
  if f.major!=Some(1)||f.revision.as_deref()!=Some(protocol::REVISION)||f.service_id.as_deref()!=Some(&self.service)||f.service_epoch.as_deref()!=Some(&self.epoch)||f.connection_id.as_deref()!=Some(&self.connection)||f.client_generation!=Some(self.generation)||f.sequence.is_none_or(|n|n<=self.incoming)||!same(&f.scope,&Some(self.scope.clone())){return Err("STALE_EPOCH".into());}
  let authority=f.authority_generation.filter(|v|*v>0).ok_or("STALE_EPOCH")?;if self.authority!=0&&self.authority!=authority{return Err("STALE_EPOCH".into());}
  if let Some(retire)=&f.retire{let delay=retire.retry_after_ms.unwrap_or(0);if delay>60000{return Err("INVALID_MESSAGE".into());}self.retry_after_ms=delay;return Err(retire.code.clone().filter(|v|matches!(v.as_str(),"INVALID_MESSAGE"|"UNAUTHENTICATED"|"FORBIDDEN"|"STALE_EPOCH"|"CAPACITY"|"EXPIRED"|"REAUTH_REQUIRED"|"RETIRED")).ok_or("INVALID_MESSAGE")?);}
  let ack=u64::from(f.ack_client_sequence.ok_or("INVALID_MESSAGE")?);if ack<self.last_ack{return Err("Live stale acknowledgement".into());}let received=Instant::now();let stamp=self.acknowledgements.acknowledged(ack,received)?;
  let access=f.access_remaining_ms.filter(|v|*v<=900000).ok_or("INVALID_MESSAGE")?;let until=f.authority_remaining_ms.filter(|v|*v<=30000).ok_or("INVALID_MESSAGE")?;let credential=f.credential_id.as_deref().filter(|v|!v.is_empty()&&v.len()<=128).ok_or("INVALID_MESSAGE")?;
  if let Some(w)=&f.welcome{
   let revision=w.auth_revision.filter(|v|*v>0).ok_or("INVALID_MESSAGE")?;let expected=self.pending_auth.map(|p|p.0).unwrap_or(if self.auth==0{1}else{self.auth});
   if revision!=expected&&revision!=self.auth||self.pending_auth.is_some_and(|p|received>=p.1)||w.pose_hz!=Some(10)||w.snapshot_hz!=Some(10){return Err("STALE_EPOCH".into());}
   let renewing=revision>self.auth;if renewing{if self.expected_credential.as_deref()!=Some(credential){return Err("UNAUTHENTICATED".into());}self.access=Deadline::default();self.credential=credential.into();}else if self.credential!=credential{return Err("UNAUTHENTICATED".into());}
   self.authority=authority;self.auth=revision;if revision==expected{self.pending_auth=None;self.expected_credential=None;}
  }else if self.auth==0||self.credential!=credential{return Err("UNAUTHENTICATED".into());}
  let old_access=self.access.until();self.access.install(stamp,access,900000,900000,received)?;if let Some(old)=old_access{self.access.shorten(old);}
  self.until.install(stamp,until,30000,12000,received)?;self.last_ack=ack;self.incoming=f.sequence.unwrap();if !self.live(){return Err(expiry("monotonic-frame-validity"));}Ok(stamp)
 }
 fn live(&self)->bool{let at=Instant::now();self.auth>0&&self.access.live(at)&&self.until.live(at)&&self.pending_auth.is_none_or(|p|at<p.1)}
}
async fn connect(grant:&MovementGrant)->Result<WebSocketStream<reqwest::Upgraded>,String>{
 let origin=reqwest::Url::parse(&grant.binding.service_origin).map_err(|_|"INVALID_MESSAGE")?;
 if origin.scheme()!="https"||origin.host_str().is_none()||!origin.username().is_empty()||origin.password().is_some()||origin.query().is_some()||origin.fragment().is_some()||origin.path()!="/"{return Err("INVALID_MESSAGE".into());}
 let key=base64::engine::general_purpose::STANDARD.encode(uuid::Uuid::new_v4().as_bytes());
 let call=grant.http.get(format!("{}{}",grant.binding.service_origin,protocol::ENDPOINT)).header("connection","Upgrade").header("upgrade","websocket").header("sec-websocket-version","13").header("sec-websocket-key",&key).header("sec-websocket-protocol",protocol::SUBPROTOCOL);
 let response=tokio::time::timeout(Duration::from_secs(3),call.send()).await.map_err(|_|"Live connect deadline")?.map_err(|_|"Live connection unavailable")?;
 match response.status().as_u16(){401=>return Err("UNAUTHENTICATED".into()),403=>return Err("FORBIDDEN".into()),429|503=>return Err("CAPACITY".into()),_=>{}}
 let exact=|key:&str,value:&str|response.headers().get_all(key).iter().count()==1&&response.headers().get(key).and_then(|v|v.to_str().ok())==Some(value);
 if response.status()!=reqwest::StatusCode::SWITCHING_PROTOCOLS||!exact("sec-websocket-protocol",protocol::SUBPROTOCOL)||!exact("sec-websocket-accept",&derive_accept_key(key.as_bytes()))||!exact("upgrade","websocket")||response.headers().contains_key("sec-websocket-extensions"){return Err("Live upgrade refused".into());}
 let upgraded=tokio::time::timeout(Duration::from_secs(3),response.upgrade()).await.map_err(|_|"Live upgrade deadline")?.map_err(|_|"Live upgrade refused")?;
 let config=WebSocketConfig::default().read_buffer_size(32768).write_buffer_size(0).max_write_buffer_size(65536).max_message_size(Some(32768)).max_frame_size(Some(32768));
 Ok(WebSocketStream::from_raw_socket(upgraded,Role::Client,Some(config)).await)
}
fn current(shared:&Arc<Mutex<MovementState>>,original:&MovementGrant,scope:&wire::Scope)->Option<MovementGrant>{
 let state=shared.lock().unwrap();let grant=state.grant.as_ref()?;
 if !matching(grant,original)||!state.live_scope.as_ref().is_some_and(|s|same(s,scope)){return None;}Some(grant.clone())
}
fn matching(grant:&MovementGrant,original:&MovementGrant)->bool{grant.active()&&grant.pid==original.pid&&grant.serial==original.serial&&grant.generation==original.generation&&grant.binding.context==original.binding.context&&grant.binding.service_origin==original.binding.service_origin&&grant.binding.service_id==original.binding.service_id&&grant.binding.service_epoch==original.binding.service_epoch&&grant.binding.session_epoch==original.binding.session_epoch&&grant.binding.principal_id==original.binding.principal_id&&same(&grant.binding.scope,&original.binding.scope)}
pub(crate) struct Lane{task:Option<tokio::task::JoinHandle<Result<(),String>>>,key:Option<([u8;16],u64,u64)>,failures:u32,next:Instant,generation:u32,admission_token:Option<Zeroizing<String>>,denied:Option<String>,stable:Arc<std::sync::atomic::AtomicBool>,backoff:Arc<std::sync::atomic::AtomicU32>,renewal_requested:bool}
fn reconnect_delay(failures:u32)->Duration{
 // Jitter is per attempt; the number of attempts is not a lifetime budget.
 let ceiling=(500u64.saturating_mul(1u64<<failures.min(5))).min(10000);
 let random=u16::from_le_bytes(uuid::Uuid::new_v4().as_bytes()[..2].try_into().unwrap()) as u64;
 Duration::from_millis(ceiling/2+random%(ceiling/2+1))
}
fn renewal_denial(error:&str)->bool{matches!(error,"FORBIDDEN"|"UNAUTHENTICATED"|"EXPIRED"|"STALE_EPOCH"|"RETIRED")}
fn denial(error:&str)->bool{matches!(error,"INVALID_MESSAGE"|"FORBIDDEN"|"UNAUTHENTICATED"|"STALE_EPOCH"|"EXPIRED"|"RETIRED")}
enum Inbound {Application(Vec<u8>),Flush,Ignore,Closed}
fn inbound(message:Message)->Result<Inbound,String>{match message{
 Message::Binary(bytes)=>Ok(Inbound::Application(bytes.to_vec())),
 Message::Ping(_)=>Ok(Inbound::Flush), // tungstenite queues the matching Pong
 Message::Pong(_)=>Ok(Inbound::Ignore),
 Message::Close(_)=>Ok(Inbound::Closed), // flush the queued Close reply before retiring
 _=>Err("INVALID_MESSAGE".into())
}}
impl Default for Lane{fn default()->Self{Self{task:None,key:None,failures:0,next:Instant::now(),generation:0,admission_token:None,denied:None,stable:Arc::new(std::sync::atomic::AtomicBool::new(false)),backoff:Arc::new(std::sync::atomic::AtomicU32::new(0)),renewal_requested:false}}}
impl Drop for Lane{fn drop(&mut self){if let Some(task)=self.task.take(){task.abort();}}}
impl Lane{
 pub(crate) async fn pump(&mut self,shared:Arc<Mutex<MovementState>>,exchange:Arc<Mutex<Exchange>>)->Result<(),String>{
  let (grant,scope)={let state=shared.lock().unwrap();(state.grant.clone(),state.live_scope.clone())};
  let Some(grant)=grant.filter(|g|g.active())else{if let Some(task)=self.task.take(){task.abort();}self.key=None;self.denied=None;return Ok(());};
  let scope=scope.ok_or("No verified live scope")?;let key=(grant.binding.context,grant.serial,grant.generation);
  if self.key!=Some(key){self.renewal_requested=false;if let Some(task)=self.task.take(){task.abort();}if self.key.is_none_or(|old|old.0!=key.0||old.2!=key.2){self.failures=0;self.denied=None;self.next=Instant::now();self.backoff.store(0,std::sync::atomic::Ordering::Release);}self.key=Some(key);}
  if self.task.as_ref().is_some_and(|t|t.is_finished()){
   let result=self.task.take().unwrap().await.unwrap_or_else(|_|Err("Live owner cancelled".into()));
   if let Err(error)=result{
    let renewed_admission=renewal_denial(&error)&&self.admission_token.as_ref().is_some_and(|old|old.as_str()!=grant.token.as_str());
    if renewed_admission{shared.lock().unwrap().live_admission_renewed();}
    if self.stable.swap(false,std::sync::atomic::Ordering::AcqRel){self.failures=0;}
    self.failures=self.failures.saturating_add(1);
    self.denied=if denial(&error)&&!renewed_admission{Some(error.clone())}else{None};
    let delay=reconnect_delay(self.failures.saturating_sub(1)).max(Duration::from_millis(u64::from(self.backoff.swap(0,std::sync::atomic::Ordering::AcqRel))));self.next=Instant::now()+delay;return Err(error);
   }
  }
  if self.denied.as_deref().is_some_and(|e|renewal_denial(e))&&self.admission_token.as_ref().is_some_and(|old|old.as_str()!=grant.token.as_str()){
   self.denied=None;self.renewal_requested=false;shared.lock().unwrap().live_admission_renewed();
  }
  if !self.renewal_requested&&Instant::now()>=self.next&&self.denied.as_deref().is_some_and(|e|renewal_denial(e)){self.renewal_requested=shared.lock().unwrap().request_live_renewal(&grant);}
  if self.task.is_none()&&self.denied.is_none()&&Instant::now()>=self.next{
   {let mut state=shared.lock().unwrap();state.live_attempts=state.live_attempts.saturating_add(1);}
   self.generation=self.generation.checked_add(1).ok_or("Live generation exhausted")?;let generation=self.generation;
   self.admission_token=Some(grant.token.clone());let stable=self.stable.clone();stable.store(false,std::sync::atomic::Ordering::Release);let backoff=self.backoff.clone();self.task=Some(tokio::spawn(async move{run(shared,exchange,grant,scope,generation,stable,backoff).await}));
  }Ok(())
 }
}
async fn run(shared:Arc<Mutex<MovementState>>,exchange:Arc<Mutex<Exchange>>,original:MovementGrant,scope:wire::Scope,generation:u32,stable:Arc<std::sync::atomic::AtomicBool>,backoff:Arc<std::sync::atomic::AtomicU32>)->Result<(),String>{
 let socket=connect(&original).await?;
 run_socket(socket,shared,exchange,original,scope,generation,stable,backoff).await
}
async fn run_socket<S>(socket:WebSocketStream<S>,shared:Arc<Mutex<MovementState>>,exchange:Arc<Mutex<Exchange>>,original:MovementGrant,scope:wire::Scope,generation:u32,stable:Arc<std::sync::atomic::AtomicBool>,backoff:Arc<std::sync::atomic::AtomicU32>)->Result<(),String>
where S:tokio::io::AsyncRead+tokio::io::AsyncWrite+Unpin+Send+'static{
 let initial=current(&shared,&original,&scope).ok_or("RETIRED")?;
 let bank_generation=exchange.lock().unwrap().begin_bank_stream(original.binding.context).ok_or("RETIRED")?;
 let mut session=Session::new(scope.clone(),original.binding.service_id.clone(),original.binding.service_epoch.clone(),generation)?;
 let mut auth=session.frame()?;auth.authenticate=Some(wire::Authenticate{access_token:Some(initial.token.to_string())});
 session.sent(&auth,Instant::now())?;
 let (mut send,mut receive)=socket.split();
 let bytes=Zeroizing::new(protocol::encode("ClientFrame",&auth).map_err(|_|"INVALID_MESSAGE")?);auth.authenticate.as_mut().unwrap().access_token.take().map(|mut t|{use zeroize::Zeroize;t.zeroize();});
 tokio::time::timeout(Duration::from_secs(2),send.send(Message::Binary(bytes.to_vec().into()))).await.map_err(|_|"Live send deadline")?.map_err(|_|"Live connection unavailable")?;
 let session=Arc::new(Mutex::new(session));let reader_session=session.clone();let reader_shared=shared.clone();let reader_exchange=exchange.clone();let reader_original=original.clone();let reader_scope=scope.clone();
 let (control_send,mut control_receive)=tokio::sync::mpsc::channel::<()>(1);
 let read=tokio::spawn(async move{
  let mut bank_generation=bank_generation;
  loop{
   let message=tokio::time::timeout(Duration::from_secs(3),receive.next()).await.map_err(|_|"Live receive deadline")?.ok_or("Live connection unavailable")?.map_err(|_|"Live connection unavailable")?;
   if current(&reader_shared,&reader_original,&reader_scope).is_none(){return Err("RETIRED".to_string());}
   let bytes=match inbound(message)?{
    Inbound::Application(bytes)=>bytes,
    Inbound::Flush=>{let _=control_send.try_send(());continue;},
    Inbound::Ignore=>continue,
    Inbound::Closed=>return Err("Live peer closed".into()),
   };
   let frame:wire::ServerFrame=protocol::decode("ServerFrame",&bytes).map_err(|_|"INVALID_MESSAGE")?;
   let (authenticated,stamp,gap)={let mut session=reader_session.lock().unwrap();let previous=session.auth;let gap=frame.sequence.is_some_and(|n|n>session.incoming.saturating_add(1));let stamp=match session.receive(&frame){Ok(stamp)=>stamp,Err(error)=>{reader_shared.lock().unwrap().live_receive_failure=Some(session.failure_diagnostic(&frame));backoff.store(session.retry_after_ms,std::sync::atomic::Ordering::Release);return Err(error);}};(if session.auth>previous{Some(previous==0)}else{None},stamp,gap)};
   if let Some(initial)=authenticated{reader_shared.lock().unwrap().live_authenticated(initial);}
   let revision=frame.bank_changed.as_ref().map(|notice|{let text=notice.revision.as_deref().ok_or("INVALID_MESSAGE")?;let value=text.parse::<u64>().map_err(|_|"INVALID_MESSAGE")?;if value>i64::MAX as u64||value.to_string()!=text{return Err("INVALID_MESSAGE")}Ok(value)}).transpose()?;
   let c1_sequence=frame.c1_changed.as_ref().map(|n|n.sequence.as_deref().ok_or("INVALID_MESSAGE")?.parse::<u64>().map_err(|_|"INVALID_MESSAGE")).transpose()?;
   let c2_sequence=frame.c2_changed.as_ref().map(|n|n.sequence.as_deref().ok_or("INVALID_MESSAGE")?.parse::<u64>().map_err(|_|"INVALID_MESSAGE")).transpose()?;
   if current(&reader_shared,&reader_original,&reader_scope).is_none(){return Err("RETIRED".into());}
   if gap{bank_generation=reader_exchange.lock().unwrap().bank_gap(reader_original.binding.context,bank_generation).ok_or("RETIRED")?;}
   reader_exchange.lock().unwrap().bank_frame(reader_original.binding.context,bank_generation,frame.connection_id.as_deref().ok_or("INVALID_MESSAGE")?,revision);
   if let Some(sequence)=c1_sequence{reader_exchange.lock().unwrap().c1_frame(reader_original.binding.context,bank_generation,sequence);}
   if let Some(sequence)=c2_sequence{reader_exchange.lock().unwrap().c2_frame(reader_original.binding.context,bank_generation,sequence);}
   if let Some(snapshot)=frame.snapshot{publish(&reader_shared,&reader_exchange,&reader_original,&reader_scope,snapshot,stamp)?;}
  }
 });
 // Reader has its own task: send stalls never accumulate inbound snapshots.
 struct Reader(Option<tokio::task::JoinHandle<Result<(),String>>>);impl Drop for Reader{fn drop(&mut self){if let Some(task)=self.0.take(){task.abort();}}}
 let mut reader=Reader(Some(read));let mut token=initial.token.clone();let mut pose_at=Instant::now();let mut ping_at=Instant::now()+Duration::from_secs(1);let initial_until=Instant::now()+Duration::from_secs(3);let mut authenticated_at=None;
 loop{
  if reader.0.as_ref().is_some_and(|t|t.is_finished()){
   let result=reader.0.take().unwrap().await.unwrap_or_else(|_|Err("Live receive retired".into()));
   if result.as_ref().err().is_some_and(|e|e=="Live peer closed"){let _=tokio::time::timeout(Duration::from_secs(2),send.flush()).await;}
   return result;
  }
  if control_receive.try_recv().is_ok(){tokio::time::timeout(Duration::from_secs(2),send.flush()).await.map_err(|_|"Live control send deadline")?.map_err(|_|"Live connection unavailable")?;}
  if session.lock().unwrap().live(){let since=authenticated_at.get_or_insert_with(Instant::now);if since.elapsed()>=Duration::from_secs(5){stable.store(true,std::sync::atomic::Ordering::Release);}}
  let grant=current(&shared,&original,&scope).ok_or("RETIRED")?;
  let frame={let mut state=session.lock().unwrap();
   if state.auth==0{if Instant::now()>=initial_until{return Err("Live auth deadline".into());}None}
   else{
    if !state.live(){return Err(expiry("writer-monotonic-validity"));}
    if grant.token.as_str()!=token.as_str()&&state.pending_auth.is_none(){
     let revision=state.auth.checked_add(1).ok_or("RETIRED")?;let mut frame=state.frame()?;frame.reauthenticate=Some(wire::Reauthenticate{access_token:Some(grant.token.to_string()),auth_revision:Some(revision)});state.pending_auth=Some((revision,Instant::now()+Duration::from_secs(3)));token=grant.token.clone();Some(frame)
    }else if Instant::now()>=ping_at{ping_at=Instant::now()+Duration::from_secs(1);let mut frame=state.frame()?;frame.ping=Some(wire::Empty{});Some(frame)}
    else if Instant::now()>=pose_at{
     pose_at=Instant::now()+Duration::from_millis(100);
     let raw={let mut x=exchange.lock().unwrap();if !grant.active(){None}else{x.pose.take()}};
     if let Some(raw)=raw{let p=crate::g3_native_frame::decode(&raw,&grant.binding.context,0)?;let pose=wire::Pose{sequence:p.sequence,x_milli:p.x_milli,y_milli:p.y_milli,vx_milli:p.vx_milli,vy_milli:p.vy_milli,facing_left:p.facing_left};check_pose(&pose)?;let mut frame=state.frame()?;frame.pose=Some(pose);Some(frame)}else{None}
    }else{None}
   }
  };
  if let Some(mut frame)=frame{
   session.lock().unwrap().sent(&frame,Instant::now())?;
   let pose=frame.pose.is_some();let bytes=Zeroizing::new(protocol::encode("ClientFrame",&frame).map_err(|_|"INVALID_MESSAGE")?);
   if let Some(auth)=frame.reauthenticate.as_mut(){if let Some(mut t)=auth.access_token.take(){use zeroize::Zeroize;t.zeroize();}}
   tokio::time::timeout(Duration::from_secs(2),send.send(Message::Binary(bytes.to_vec().into()))).await.map_err(|_|"Live send deadline")?.map_err(|_|"Live connection unavailable")?;
   if current(&shared,&original,&scope).is_none(){return Err("RETIRED".into());}
   if pose{shared.lock().unwrap().live_publication();}
  }
  tokio::time::sleep(Duration::from_millis(10)).await;
 }
}
fn publish(shared:&Arc<Mutex<MovementState>>,exchange:&Arc<Mutex<Exchange>>,original:&MovementGrant,scope:&wire::Scope,snapshot:wire::Snapshot,stamp:Stamp)->Result<(),String>{
 let peers=snapshot.peers.unwrap_or_default();let wire_peers=peers.len();let(mut expired_peer_age,mut expired_pose_age)=(0usize,0usize);if peers.len()>31{return Err("INVALID_MESSAGE".into());}
 let mut seen=std::collections::BTreeSet::new();let mut native=vec![];let mut ghosts=vec![];let mut ages=vec![];let mut peer_times=std::collections::BTreeMap::new();
 for peer in peers{
  if ![&peer.principal_id,&peer.session_id,&peer.attachment_id].into_iter().all(|v|id(v.as_deref()))||peer.session_id==scope.session_id||peer.world_token.as_ref().is_none_or(|v|v.is_empty())||!seen.insert(peer.session_id.clone().unwrap()){return Err("INVALID_MESSAGE".into());}
  let peer_age=match relative_age(peer.peer_age_ms_at_send.ok_or("INVALID_MESSAGE")?,stamp,Instant::now()){Ok(age)=>age,Err(_)=>{expired_peer_age+=1;continue}};
  ages.push(peer_age.as_millis() as u64);
  let poses=peer.poses.as_ref().ok_or("INVALID_MESSAGE")?;if poses.is_empty()||poses.len()>4{return Err("INVALID_MESSAGE".into());}let pose_ages=peer.pose_ages_ms_at_send.as_ref().ok_or("INVALID_MESSAGE")?;if pose_ages.len()!=poses.len(){return Err("INVALID_MESSAGE".into());}
  for age in pose_ages{if *age>2000{return Err("INVALID_MESSAGE".into());}}
  let latest_age=match relative_age(*pose_ages.last().ok_or("INVALID_MESSAGE")?,stamp,Instant::now()){Ok(age)=>age.max(peer_age),Err(_)=>{expired_pose_age+=1;continue}};let mut sequence=0;let mut frames=vec![];
  for p in poses{check_pose(p)?;if p.sequence.unwrap()<=sequence{return Err("STALE_EPOCH".into());}sequence=p.sequence.unwrap();frames.push(crate::protocol_g3::wire::MovementFrame{sequence:p.sequence,x_milli:p.x_milli.clone(),y_milli:p.y_milli.clone(),vx_milli:p.vx_milli.clone(),vy_milli:p.vy_milli.clone(),facing_left:p.facing_left,..Default::default()});}
  let p=poses.last().unwrap();let mut frame=[0;64];frame[..8].copy_from_slice(b"NTG3\x01\x00\x01\x00");frame[8..24].copy_from_slice(&original.binding.context);frame[24..28].copy_from_slice(&sequence.to_le_bytes());
  for (offset,value) in [(28,&p.x_milli),(32,&p.y_milli),(36,&p.vx_milli),(40,&p.vy_milli)]{frame[offset..offset+4].copy_from_slice(&coordinate(value.as_deref())?.to_le_bytes());}frame[44]=u8::from(p.facing_left.unwrap_or(false));
  use sha2::{Digest,Sha256};let actor:[u8;16]=Sha256::digest(serde_json::to_vec(&(&peer.principal_id,&peer.session_id,&peer.world_token)).unwrap())[..16].try_into().unwrap();native.push((actor,frame));peer_times.insert(actor,Instant::now().checked_sub(latest_age).ok_or("INVALID_MESSAGE")?);
  ghosts.push(crate::protocol_g3::wire::Ghost{principal_id:peer.principal_id,game_session_id:peer.session_id,world_token:peer.world_token,frames:Some(frames),..Default::default()});
 }
 let evidence=serde_json::json!({"kind":"authenticated_live_snapshot","wire_peers":wire_peers,"accepted_peers":native.len(),"expired_peer_age":expired_peer_age,"expired_pose_age":expired_pose_age,"ack_elapsed_ms":stamp.started.elapsed().as_millis().min(60000)});
 commit_snapshot_aged(shared,exchange,original,scope,native,ghosts,peer_times,evidence)?;shared.lock().unwrap().live_sample_ages(ages);Ok(())
}
#[cfg(test)]
pub(crate) fn commit_snapshot(shared:&Arc<Mutex<MovementState>>,exchange:&Arc<Mutex<Exchange>>,original:&MovementGrant,scope:&wire::Scope,native:Vec<([u8;16],[u8;64])>,ghosts:Vec<crate::protocol_g3::wire::Ghost>)->Result<(),String>{
 commit_snapshot_aged(shared,exchange,original,scope,native,ghosts,std::collections::BTreeMap::new(),serde_json::json!({"kind":"test_commit"}))
}
fn commit_snapshot_aged(shared:&Arc<Mutex<MovementState>>,exchange:&Arc<Mutex<Exchange>>,original:&MovementGrant,scope:&wire::Scope,native:Vec<([u8;16],[u8;64])>,ghosts:Vec<crate::protocol_g3::wire::Ghost>,peer_times:std::collections::BTreeMap<[u8;16],Instant>,evidence:serde_json::Value)->Result<(),String>{
 // Match the immutable connection identity against CURRENT verified authority.
 // Hold both short state locks through commit; renewal may replace the lease,
 // while End/cancellation and native fence changes still refuse late frames.
 let mut x=exchange.lock().unwrap();let mut state=shared.lock().unwrap();
 let grant=state.grant.as_ref().ok_or("RETIRED")?;
 if !matching(grant,original)||!state.live_scope.as_ref().is_some_and(|s|same(s,scope))||x.owner!=Some((grant.pid,grant.binding.context))||Instant::now()>=x.until{return Err("RETIRED".into());}
 use sha2::{Digest,Sha256};let ghost_times=ghosts.iter().filter_map(|g|{let actor:[u8;16]=Sha256::digest(serde_json::to_vec(&(&g.principal_id,&g.game_session_id,&g.world_token)).ok()?)[..16].try_into().ok()?;Some((g.game_session_id.clone()?,*peer_times.get(&actor)?))}).collect();
 x.peers=native;x.peer_times=peer_times;x.peers_at=Instant::now();x.note_projection(evidence);state.accept_ghosts(ghosts);state.ghost_times=ghost_times;Ok(())
}

#[cfg(test)]mod cr1_tests{
 #[test]fn a3_roster_provenance_distinguishes_wire_empty_and_age_filtered_snapshots(){
  let scope=welcome().scope.unwrap();let(grant,shared,exchange)=crate::g3_runtime::cr1_live_fixture(scope.clone());
  let peer:wire::Peer=serde_json::from_value(serde_json::json!({"principalId":uuid::Uuid::new_v4().to_string(),"sessionId":uuid::Uuid::new_v4().to_string(),"attachmentId":uuid::Uuid::new_v4().to_string(),"worldToken":uuid::Uuid::new_v4().to_string(),"peerAgeMsAtSend":50,"poseAgesMsAtSend":[50],"poses":[{"sequence":1,"xMilli":"0","yMilli":"0","vxMilli":"0","vyMilli":"0","facingLeft":false}]})).unwrap();
  for case in 0..4{let mut p=peer.clone();let mut elapsed=0;if case==2{p.peer_age_ms_at_send=Some(1900);elapsed=200;}if case==3{p.pose_ages_ms_at_send=Some(vec![1900]);elapsed=200;}
   let snapshot=wire::Snapshot{peers:Some(if case==1{vec![]}else{vec![p]}),..Default::default()};publish(&shared,&exchange,&grant,&scope,snapshot,Stamp{sequence:1,started:Instant::now()-Duration::from_millis(elapsed)}).unwrap();
   let x=exchange.lock().unwrap();let diagnostic=x.roster_diagnostics();let facts=&diagnostic["trace"].as_array().unwrap().last().unwrap()["event"];assert_eq!(facts["wire_peers"],usize::from(case!=1));assert_eq!(facts["accepted_peers"],usize::from(case==0));assert_eq!(facts["expired_peer_age"],usize::from(case==2));assert_eq!(facts["expired_pose_age"],usize::from(case==3));assert_eq!(x.peers.len(),usize::from(case==0));
  }
 }
 #[test]fn lc1_stale_transport_retirement_requires_current_new_credential_and_keeps_authority_bounds(){tauri::async_runtime::block_on(async{
  for code in ["STALE_EPOCH","RETIRED","FORBIDDEN"]{
   let scope=welcome().scope.unwrap();let(grant,shared,exchange)=crate::g3_runtime::cr1_live_fixture(scope);let until=exchange.lock().unwrap().until;let serial=exchange.lock().unwrap().fence.load(std::sync::atomic::Ordering::Acquire);
   let mut lane=Lane::default();lane.key=Some((grant.binding.context,grant.serial,grant.generation));lane.admission_token=Some(grant.token.clone());let error=code.to_string();lane.task=Some(tokio::spawn(async move{Err(error)}));while !lane.task.as_ref().unwrap().is_finished(){tokio::task::yield_now().await;}
   assert_eq!(lane.pump(shared.clone(),exchange.clone()).await.unwrap_err(),code);assert_eq!(lane.denied.as_deref(),Some(code));assert!(lane.next>Instant::now());lane.next=Instant::now();
   lane.pump(shared.clone(),exchange.clone()).await.unwrap();assert!(lane.renewal_requested);assert!(lane.task.is_none(),"Same rejected credential cannot retry itself");assert_eq!(exchange.lock().unwrap().until,until);assert_eq!(exchange.lock().unwrap().fence.load(std::sync::atomic::Ordering::Acquire),serial);
   shared.lock().unwrap().grant.as_mut().unwrap().token=Zeroizing::new("fresh-canonically-verified-token".into());lane.pump(shared.clone(),exchange.clone()).await.unwrap();assert!(lane.denied.is_none());assert!(lane.task.is_some());assert_eq!(exchange.lock().unwrap().until,until);
  }
  assert!(!renewal_denial("INVALID_MESSAGE"),"Malformed transport cannot request automatic renewal");
  let mut f=welcome();let mut s=Session::new(f.scope.clone().unwrap(),f.service_id.clone().unwrap(),f.service_epoch.clone().unwrap(),1).unwrap();f.connection_id=Some(s.connection.clone());f.client_generation=Some(1);f.sequence=Some(1);f.retire=Some(wire::Retire{code:Some("STALE_EPOCH".into()),retry_after_ms:Some(0),..Default::default()});assert_eq!(s.receive(&f).unwrap_err(),"STALE_EPOCH");let diagnostic=s.failure_diagnostic(&f);assert_eq!(diagnostic["source"],"service_retire");assert_eq!(diagnostic["header_matches"],true);f.scope.as_mut().unwrap().world_token=Some("secret-world-selector".into());assert_eq!(s.failure_diagnostic(&f)["header_matches"],false);assert!(!s.failure_diagnostic(&f).to_string().contains("secret-world-selector"));
 });}
 use super::*;
 use std::sync::atomic::Ordering;
 #[test]fn lc1_ghost_live_token_turnover_keeps_one_pending_reauth_and_pose_progress(){tauri::async_runtime::block_on(async{
  let mut sample=welcome();let scope=sample.scope.clone().unwrap();let(mut grant,shared,exchange)=crate::g3_runtime::cr1_live_fixture(scope.clone());let ids=[sample.credential_id.clone().unwrap(),uuid::Uuid::new_v4().to_string(),uuid::Uuid::new_v4().to_string()];grant.token=fixture_token(&ids[0]);grant.binding.service_id=sample.service_id.clone().unwrap();grant.binding.service_epoch=sample.service_epoch.clone().unwrap();shared.lock().unwrap().grant=Some(grant.clone());let native_until=exchange.lock().unwrap().until;let serial=exchange.lock().unwrap().fence.load(Ordering::Acquire);
  let(client,server)=tokio::io::duplex(65536);let client=WebSocketStream::from_raw_socket(client,Role::Client,None).await;let mut server=WebSocketStream::from_raw_socket(server,Role::Server,None).await;let owner=tokio::spawn(run_socket(client,shared.clone(),exchange.clone(),grant.clone(),scope,1,Arc::new(std::sync::atomic::AtomicBool::new(false)),Arc::new(std::sync::atomic::AtomicU32::new(0))));
  let Message::Binary(bytes)=server.next().await.unwrap().unwrap()else{panic!()};let auth:wire::ClientFrame=protocol::decode("ClientFrame",&bytes).unwrap();sample.connection_id=auth.connection_id;sample.client_generation=auth.client_generation;sample.ack_client_sequence=auth.sequence;sample.sequence=Some(1);sample.authority_remaining_ms=Some(12000);sample.access_remaining_ms=Some(900000);server.send(Message::Binary(protocol::encode("ServerFrame",&sample).unwrap().into())).await.unwrap();
  let stop=Arc::new(std::sync::atomic::AtomicBool::new(false));let producer_stop=stop.clone();let poses=exchange.clone();let context=grant.binding.context;let pid=grant.pid;let producer=tokio::spawn(async move{let mut sequence=0u32;while !producer_stop.load(Ordering::Acquire){sequence+=1;let mut frame=[0u8;64];frame[..8].copy_from_slice(b"NTG3\x01\x00\x01\x00");frame[8..24].copy_from_slice(&context);frame[24..28].copy_from_slice(&sequence.to_le_bytes());assert_eq!(poses.lock().unwrap().transition(pid,&frame,true)[5],1);tokio::time::sleep(Duration::from_millis(20)).await;}});
  let mut revision=1;let mut outgoing=1;let mut last_pose=0;let mut last_frame=1;let mut counts=[0u32;2];let mut pending:Option<(u32,Instant,usize)>=None;let mut sent_b=false;let mut sent_c=false;let began=Instant::now();
  while began.elapsed()<Duration::from_millis(1600){if !sent_b&&began.elapsed()>=Duration::from_millis(100){shared.lock().unwrap().grant.as_mut().unwrap().token=fixture_token(&ids[1]);sent_b=true;}if pending.is_some()&&!sent_c{shared.lock().unwrap().grant.as_mut().unwrap().token=fixture_token(&ids[2]);sent_c=true;}
   if let Ok(Some(Ok(Message::Binary(bytes))))=tokio::time::timeout(Duration::from_millis(20),server.next()).await{let frame:wire::ClientFrame=protocol::decode("ClientFrame",&bytes).unwrap();assert!(frame.sequence.unwrap()>last_frame);last_frame=frame.sequence.unwrap();if let Some(auth)=frame.reauthenticate{assert!(pending.is_none(),"Writer sent overlapping reauthentication");assert_eq!(auth.auth_revision,Some(revision+1));let index=if revision==1{1}else{2};assert_eq!(crate::protocol_connection::credential_id(auth.access_token.as_deref().unwrap()).unwrap(),ids[index]);pending=Some((revision+1,Instant::now()+Duration::from_millis(350),index));}if let Some(pose)=frame.pose{assert!(pose.sequence.unwrap()>last_pose);last_pose=pose.sequence.unwrap();counts[usize::from(revision>1)]+=1;}}
   let mut acknowledged=None;if let Some((next,at,index))=pending{if Instant::now()>=at{revision=next;acknowledged=Some(index);pending=None;}}outgoing+=1;sample.sequence=Some(outgoing);sample.ack_client_sequence=Some(last_frame);sample.welcome.as_mut().unwrap().auth_revision=Some(revision);if let Some(index)=acknowledged{sample.credential_id=Some(ids[index].clone());}server.send(Message::Binary(protocol::encode("ServerFrame",&sample).unwrap().into())).await.unwrap();assert!(!owner.is_finished(),"Client retired during same-scope delayed credential turnover");
  }
  assert_eq!(revision,3);assert!(counts[0]>=2&&counts[1]>=3,"Pose progress stopped around renewal: {counts:?}");assert_eq!(exchange.lock().unwrap().until,native_until);assert_eq!(exchange.lock().unwrap().fence.load(Ordering::Acquire),serial);assert!(grant.active());stop.store(true,Ordering::Release);producer.await.unwrap();shared.lock().unwrap().grant=None;assert_eq!(owner.await.unwrap().unwrap_err(),"RETIRED");
 });}
 fn welcome()->wire::ServerFrame{let golden:serde_json::Value=serde_json::from_str(include_str!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../packages/contracts/g3-live/golden.json"))).unwrap();serde_json::from_value(golden["fixtures"].as_array().unwrap().iter().find(|v|v["json"].get("welcome").is_some()).unwrap()["json"].clone()).unwrap()}
 #[test]fn cr2_wss_correlated_ack_duplicate_offsets_unknown_and_retire_backoff(){
  for year in ["1970","2099"]{
   let mut f=welcome();let mut s=Session::new(f.scope.clone().unwrap(),f.service_id.clone().unwrap(),f.service_epoch.clone().unwrap(),1).unwrap();f.connection_id=Some(s.connection.clone());f.client_generation=Some(1);f.sequence=Some(1);
   let mut auth=s.frame().unwrap();auth.authenticate=Some(wire::Authenticate{access_token:Some(fixture_token(f.credential_id.as_deref().unwrap()).to_string())});let sent=Instant::now()-Duration::from_millis(500);s.sent(&auth,sent).unwrap();f.ack_client_sequence=auth.sequence;f.welcome.as_mut().unwrap().access_expires_at=Some(format!("{year}-01-01T00:00:00Z"));f.welcome.as_mut().unwrap().authority_until=Some(format!("{year}-01-01T00:00:00Z"));
   s.receive(&f).unwrap();let until=s.until.until().unwrap();assert!(until<=sent+Duration::from_secs(12));f.sequence=Some(2);f.authority_remaining_ms=Some(30000);s.receive(&f).unwrap();assert_eq!(s.until.until(),Some(until));
   let mut unknown=f.clone();unknown.sequence=Some(3);unknown.ack_client_sequence=Some(999);assert!(s.receive(&unknown).is_err());
   f.sequence=Some(3);f.retire=Some(wire::Retire{code:Some("FORBIDDEN".into()),retry_after_ms:Some(15000),..Default::default()});assert_eq!(s.receive(&f).unwrap_err(),"FORBIDDEN");assert_eq!(s.retry_after_ms,15000);
  }
 }
 #[test]fn cr2_wss_forbidden_verified_rotation_reopens_after_scoped_backoff(){tauri::async_runtime::block_on(async{
  let scope=welcome().scope.unwrap();let(grant,shared,exchange)=crate::g3_runtime::cr1_live_fixture(scope);let mut lane=Lane::default();lane.key=Some((grant.binding.context,grant.serial,grant.generation));lane.admission_token=Some(grant.token.clone());lane.backoff.store(15000,Ordering::Release);
  lane.task=Some(tokio::spawn(async{Err("FORBIDDEN".into())}));while !lane.task.as_ref().unwrap().is_finished(){tokio::task::yield_now().await;}
  assert!(lane.pump(shared.clone(),exchange.clone()).await.is_err());assert_eq!(lane.denied.as_deref(),Some("FORBIDDEN"));let retry=lane.next;assert!(retry>=Instant::now()+Duration::from_secs(14));
  shared.lock().unwrap().grant.as_mut().unwrap().token=Zeroizing::new("new-verified-token".into());lane.pump(shared.clone(),exchange.clone()).await.unwrap();assert!(lane.denied.is_none());assert_eq!(lane.next,retry);assert!(lane.task.is_none());lane.next=Instant::now();lane.pump(shared,exchange).await.unwrap();assert!(lane.task.is_some());
 });}
 #[test]fn cr1_live_ongoing_retries_denials_stable_reset_and_retirement(){tauri::async_runtime::block_on(async{
  let scope=welcome().scope.unwrap();let(grant,shared,exchange)=crate::g3_runtime::cr1_live_fixture(scope);let mut lane=Lane::default();lane.key=Some((grant.binding.context,grant.serial,grant.generation));
  let until=exchange.lock().unwrap().until;let fence=exchange.lock().unwrap().fence.load(Ordering::Acquire);
  let mut simulated_loss=Duration::ZERO;
  for _ in 0..32{
   lane.task=Some(tokio::spawn(async{Err("Live connection unavailable".into())}));while !lane.task.as_ref().unwrap().is_finished(){tokio::task::yield_now().await;}
   assert!(lane.pump(shared.clone(),exchange.clone()).await.is_err());assert!(lane.denied.is_none());assert!(lane.next>Instant::now());assert!(lane.next<=Instant::now()+Duration::from_secs(10));simulated_loss+=lane.next.saturating_duration_since(Instant::now());
   lane.next=Instant::now();lane.pump(shared.clone(),exchange.clone()).await.unwrap();assert!(lane.task.is_some(),"lifetime connection budget exhausted");lane.task.take().unwrap().abort();
  }
  assert_eq!(lane.generation,32);assert!(simulated_loss>=Duration::from_secs(120));assert_eq!(exchange.lock().unwrap().until,until);assert_eq!(exchange.lock().unwrap().fence.load(Ordering::Acquire),fence);assert!(grant.active());
  lane.stable.store(true,Ordering::Release);lane.task=Some(tokio::spawn(async{Err("Live peer closed".into())}));while !lane.task.as_ref().unwrap().is_finished(){tokio::task::yield_now().await;}let _=lane.pump(shared.clone(),exchange.clone()).await;assert_eq!(lane.failures,1);
  lane.admission_token=Some(grant.token.clone());lane.task=Some(tokio::spawn(async{Err("UNAUTHENTICATED".into())}));while !lane.task.as_ref().unwrap().is_finished(){tokio::task::yield_now().await;}let _=lane.pump(shared.clone(),exchange.clone()).await;lane.next=Instant::now();lane.pump(shared.clone(),exchange.clone()).await.unwrap();assert!(lane.task.is_none());
  shared.lock().unwrap().grant.as_mut().unwrap().token=Zeroizing::new("verified-replacement".into());lane.pump(shared.clone(),exchange.clone()).await.unwrap();assert!(lane.task.is_some());
  shared.lock().unwrap().grant.as_mut().unwrap().until=Instant::now();lane.pump(shared.clone(),exchange.clone()).await.unwrap();assert!(lane.task.is_none());assert!(lane.key.is_none());
 });}
 #[test]fn cr1_control_frames_real_socket_and_stable_success(){tauri::async_runtime::block_on(async{
  let mut welcome=welcome();let scope=welcome.scope.clone().unwrap();let(mut grant,shared,exchange)=crate::g3_runtime::cr1_live_fixture(scope.clone());grant.token=fixture_token(welcome.credential_id.as_deref().unwrap());grant.binding.service_id=welcome.service_id.clone().unwrap();grant.binding.service_epoch=welcome.service_epoch.clone().unwrap();shared.lock().unwrap().grant=Some(grant.clone());
  let(client,server)=tokio::io::duplex(65536);let client=WebSocketStream::from_raw_socket(client,Role::Client,None).await;let mut server=WebSocketStream::from_raw_socket(server,Role::Server,None).await;
  let stable=Arc::new(std::sync::atomic::AtomicBool::new(false));let observed=stable.clone();let owner=tokio::spawn(run_socket(client,shared,exchange,grant,scope,1,stable,Arc::new(std::sync::atomic::AtomicU32::new(0))));
  let Message::Binary(bytes)=server.next().await.unwrap().unwrap() else{panic!()};let auth:wire::ClientFrame=protocol::decode("ClientFrame",&bytes).unwrap();welcome.ack_client_sequence=auth.sequence;welcome.connection_id=auth.connection_id;welcome.client_generation=auth.client_generation;welcome.sequence=Some(1);let w=welcome.welcome.as_mut().unwrap();w.access_expires_at=Some("2099-01-01T00:00:00.000Z".into());w.authority_until=Some("2099-01-01T00:00:00.000Z".into());
  server.send(Message::Binary(protocol::encode("ServerFrame",&welcome).unwrap().into())).await.unwrap();
  let mut pongs=0;let until=Instant::now()+Duration::from_millis(5300);
  while Instant::now()<until{
   server.send(Message::Ping(vec![1,2,3].into())).await.unwrap();server.send(Message::Pong(vec![8].into())).await.unwrap();
   let deadline=Instant::now()+Duration::from_millis(500);
   while Instant::now()<deadline{match tokio::time::timeout(Duration::from_millis(100),server.next()).await{Ok(Some(Ok(Message::Pong(bytes))))=>{assert_eq!(bytes.as_ref(),[1,2,3]);pongs+=1;break},Ok(Some(Ok(Message::Binary(_))))=>{},Err(_)=>{},v=>panic!("unexpected control {v:?}")}}
   tokio::time::sleep(Duration::from_millis(250)).await;
  }
  assert!(pongs>5);assert!(observed.load(Ordering::Acquire));server.send(Message::Close(None)).await.unwrap();
  loop{match tokio::time::timeout(Duration::from_secs(3),server.next()).await.unwrap(){Some(Ok(Message::Close(_)))=>break,Some(Ok(Message::Binary(_)))|Some(Ok(Message::Pong(_)))=>{},v=>panic!("missing close acknowledgement {v:?}")}}
  assert_eq!(owner.await.unwrap().unwrap_err(),"Live peer closed");
 });}
}

#[cfg(test)]fn fixture_token(id:&str)->Zeroizing<String>{use base64::Engine;Zeroizing::new(format!("fixture.{}.unsigned",base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(serde_json::to_vec(&serde_json::json!({"jti":id})).unwrap())))}
