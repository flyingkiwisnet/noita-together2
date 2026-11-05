//! LC1's single signed-in credential owner. Independent of native/game state.
//! Only short snapshots/CAS use its mutex; all I/O happens in the worker.
use super::*;
use nt2_desktop_g0::connection_time::Stamp;
#[derive(Clone)]struct Binding{key:String,revision:u64,http:reqwest::Client,origin:String,service:String,epoch:String,device:uuid::Uuid,session:Option<Session>,restore:Option<wire::Request>,profile:Option<ServiceProfile>,remember:bool,until:Option<Instant>}
#[derive(Default)]pub(super) struct Feed{binding:Option<Binding>,revision:u64,credential_generation:u64,force:bool,output:Option<Session>,pub(super) error:Option<String>,pub(super) renewals:u64,diagnostic:Value}
impl Feed{pub(super) fn view(&self,core:Option<&Session>)->Value{let mut v=self.diagnostic.clone();if !v.is_object(){v=json!({});}v["renewals"]=json!(self.renewals);v["binding"]=json!(self.binding.is_some());v["output_pending"]=json!(self.output.is_some());v["remaining_ms"]=json!(self.binding.as_ref().and_then(|b|b.until.map(|at|at.saturating_duration_since(Instant::now()).as_millis().min(900000))));v["core_identity_matches"]=json!(core.zip(self.binding.as_ref().and_then(|b|b.session.as_ref())).is_some_and(|(a,b)|a.id==b.id&&a.epoch==b.epoch&&a.account==b.account));v["core_access_matches"]=json!(core.zip(self.binding.as_ref().and_then(|b|b.session.as_ref())).is_some_and(|(a,b)|a.access.as_str()==b.access.as_str()));v}}
impl Feed{pub(super) fn retire(&mut self){self.revision+=1;self.credential_generation=0;self.binding=None;self.output=None;self.force=false;self.error=None;self.diagnostic=Value::Null}pub(super) fn force(&mut self){self.force=true;}}
/// A handle to the existing auth owner, never a second credential owner. Reads
/// happen after local durability; identity retirement cannot adopt a new login.
#[derive(Clone)]pub(super) struct RequestOwner{feed:Arc<Mutex<Feed>>,key:String,revision:u64}
impl RequestOwner{
 pub(super) fn current(&self)->bool{self.feed.lock().unwrap().binding.as_ref().is_some_and(|b|b.key==self.key&&b.revision==self.revision)}
 pub(super) fn generation(&self)->Option<u64>{let f=self.feed.lock().unwrap();f.binding.as_ref().filter(|b|b.key==self.key&&b.revision==self.revision).map(|_|f.credential_generation)}
 pub(super) fn read(&self,origin:&str,service:&str,epoch:&str,session_epoch:&str)->Result<(Session,u64),String>{
  let f=self.feed.lock().unwrap();let b=f.binding.as_ref().filter(|b|b.key==self.key&&b.revision==self.revision).ok_or("Lifecycle credential owner retired")?;
  let s=b.session.as_ref().ok_or("Lifecycle desktop credential absent")?;
  if b.origin!=origin||b.service!=service||b.epoch!=epoch||s.epoch!=session_epoch{return Err("Lifecycle credential context changed".into())}
  if b.until.is_some_and(|at|at<=Instant::now()){return Err("EXPIRED".into())}
  Ok((s.clone(),f.credential_generation))
 }
 pub(super) fn newer_verified(&self,used:u64)->bool{let f=self.feed.lock().unwrap();f.binding.as_ref().is_some_and(|b|b.key==self.key&&b.revision==self.revision&&b.until.is_some_and(|at|at>Instant::now()))&&f.credential_generation>used&&f.error.is_none()}
}
#[cfg(test)]impl Feed{pub(super) fn fixture_refresh_requested(&self)->bool{self.force}}
pub struct Runner<S:CredentialStore>{feed:Arc<Mutex<Feed>>,store:Arc<S>,writer:Arc<Mutex<()>>,original:Option<(String,wire::Request)>,commit:Option<(Binding,Session)>,next:Instant,sequence:u64}
impl<S:CredentialStore> Core<S>{
 pub(super) fn publish_auth_owner(&mut self){
  let mut f=self.auth_owner.lock().unwrap();let Some(session)=self.desktop.clone()else{
   if let Some(retry)=&self.refresh_retry{let key=json!([self.base,self.service,self.epoch,self.device,format!("{:?}",self.profile),retry.request.request_id]).to_string();if f.binding.as_ref().is_none_or(|b|b.key!=key){f.retire();let revision=f.revision;f.binding=Some(Binding{key,revision,http:self.http.clone(),origin:self.base.clone(),service:self.service.clone(),epoch:self.epoch.clone(),device:self.device,session:None,restore:Some(retry.request.clone()),profile:self.profile.clone(),remember:self.remember,until:None});}return}f.retire();return};
  let key=json!([self.base,self.service,self.epoch,self.device,session.account,session.id,session.epoch,format!("{:?}",self.profile),self.remember]).to_string();
  if f.binding.as_ref().is_none_or(|b|b.key!=key&&!(b.origin==self.base&&b.service==self.service&&b.epoch==self.epoch&&b.device==self.device&&b.profile==self.profile&&b.remember==self.remember&&b.session.as_ref().is_some_and(|s|s.id==session.id&&s.epoch==session.epoch&&s.account==session.account))){f.retire();let revision=f.revision;let until=admission::credential_key(&self.base,&self.service,&session).ok().and_then(|key|self.lifecycle.lock().unwrap().credentials.get(&key).and_then(|l|l.deadline.until()));f.binding=Some(Binding{key,revision,http:self.http.clone(),origin:self.base.clone(),service:self.service.clone(),epoch:self.epoch.clone(),device:self.device,session:Some(session),restore:None,profile:self.profile.clone(),remember:self.remember,until});}
 }
 pub(super) fn sync_auth_owner(&mut self){let mut f=self.auth_owner.lock().unwrap();if let Some(s)=f.output.take(){if self.desktop.as_ref().is_none_or(|d|d.account==s.account&&d.id==s.id&&d.epoch==s.epoch)&&f.binding.as_ref().is_some_and(|b|b.origin==self.base&&b.service==self.service&&b.epoch==self.epoch&&b.device==self.device&&b.profile==self.profile){self.desktop=Some(s);self.refresh_retry=None;self.untimed_refresh=None;}}}
 pub fn auth_runner(&mut self)->Runner<S>{self.publish_auth_owner();Runner{feed:self.auth_owner.clone(),store:self.store.clone(),writer:self.auth_writer.clone(),original:None,commit:None,next:Instant::now(),sequence:0}}
 pub(super) fn lifecycle_request_owner(&mut self)->Result<RequestOwner,String>{self.publish_auth_owner();let f=self.auth_owner.lock().unwrap();let b=f.binding.as_ref().filter(|b|b.session.is_some()).ok_or("Lifecycle desktop owner absent")?;Ok(RequestOwner{feed:self.auth_owner.clone(),key:b.key.clone(),revision:b.revision})}
}

#[cfg(test)]mod tests{
 use super::*;use base64::Engine;
 use super::super::setup::tests::{client,network,Reply};
 #[test]fn lc1_normal_scheduler_renews_signed_in_auth_without_game_while_world_writer_stalls(){tauri::async_runtime::block_on(async{
  let(mut c,_,_)=client();c.enable_lc1();c.game=None;c.room=None;c.world.coordinator.intent(nt2_desktop_g0::world_lifecycle::Stage::Reconnecting);c.world.coordinator.block("saved world reconciliation pending");c.world.optional_next=Some(Instant::now()+Duration::from_secs(60));let d=c.desktop.clone().unwrap();let device=c.device.to_string();let renewed=d.clone();
  let(origin,server)=network(vec![Reply::Build(Box::new(move|q|{assert!(q["refresh"].is_object(),"Normal auth worker must own renewal");let access=format!("fixture.{}.unsigned",base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(json!({"jti":"lc1-auth-renewal"}).to_string()));json!({"credentials":{"accessToken":access,"refreshSecret":"r".repeat(43),"accountId":renewed.account,"sessionId":renewed.id,"sessionEpoch":renewed.epoch,"kind":"desktop","deviceId":device,"expiresAt":"2099-01-01T00:00:00Z","principal":renewed.principal},"credentialValidity":{"principalId":renewed.account,"sessionId":renewed.id,"sessionEpoch":renewed.epoch,"credentialId":"lc1-auth-renewal","originRequestId":q["requestId"],"sampleKey":"c".repeat(64),"accessRemainingMs":900000}})}))]);c.base=origin.clone();c.profile=Some(c.credential_profile(&origin,&c.service).unwrap());c.remember=true;
  let writer=c.credential_writer.clone();let(tx,rx)=std::sync::mpsc::channel();let(release,wait)=std::sync::mpsc::channel();let held=std::thread::spawn(move||{let _guard=writer.lock().unwrap();tx.send(()).unwrap();wait.recv().unwrap();});rx.recv().unwrap();
  let stop=Arc::new(std::sync::atomic::AtomicBool::new(false));let(handle,actor)=crate::next_app::scheduler::new(c,stop.clone());let actor=tokio::spawn(actor);let until=Instant::now()+Duration::from_secs(8);loop{let v=handle.view();if v["auth_renewals"]==1{assert_eq!(v["connection"]["reason"],"saved world reconciliation pending");assert_eq!(v["signed_in"],true);break}assert!(Instant::now()<until,"Ordinary no-game auth owner did not renew: {}",v["auth_error"]);tokio::time::sleep(Duration::from_millis(50)).await;}
  tokio::time::timeout(Duration::from_millis(750),handle.command(|c|Box::pin(async move{assert!(c.game.is_none());assert_eq!(c.desktop.as_ref().unwrap().refresh.as_str(),"r".repeat(43));
   // Cold original-status/cleanup follows the existing verified desktop auth
   // owner, even though no game has populated the separate admission cache.
   assert!(c.session_expiring(c.desktop.as_ref().unwrap(),0));let current=c.original_status_desktop()?;assert_eq!(current.access.as_str(),c.desktop.as_ref().unwrap().access.as_str());
   let until={let mut feed=c.auth_owner.lock().unwrap();let b=feed.binding.as_mut().unwrap();let until=b.until;b.until=Some(Instant::now());until};assert!(c.original_status_desktop().is_err(),"Expired desktop auth must not enable original-status cleanup");c.auth_owner.lock().unwrap().binding.as_mut().unwrap().until=until;
   Ok(Value::Null)}))).await.unwrap().unwrap();release.send(()).unwrap();held.join().unwrap();stop.store(true,Ordering::Release);actor.await.unwrap();assert_eq!(server.join().unwrap().len(),1);
 });}
}
impl<S:CredentialStore+'static> Runner<S>{
 fn diagnostic(&self,b:&Binding,stage:&str,details:Value){let mut f=self.feed.lock().unwrap();if f.binding.as_ref().is_some_and(|current|current.revision==b.revision&&current.key==b.key){f.diagnostic=json!({"stage":stage,"request_sequence":self.sequence,"details":details});}}
 pub async fn pump(&mut self)->Result<(),String>{
  if Instant::now()<self.next{return Ok(())}self.next=Instant::now()+Duration::from_secs(2);
  if let Some((b,s))=self.commit.clone(){if self.feed.lock().unwrap().binding.as_ref().is_none_or(|current|current.key!=b.key||current.revision!=b.revision){self.commit=None;}else{match self.persist(&b,s.refresh.as_str(),"",0).await{Ok(())=>{self.commit=None;self.feed.lock().unwrap().error=None;},Err(e)=>{self.feed.lock().unwrap().error=Some(e.clone());return Err(e)}}}}
  let (binding,force)={let f=self.feed.lock().unwrap();(f.binding.clone(),f.force)};let Some(b)=binding else{self.original=None;return Ok(())};
  if self.original.as_ref().is_some_and(|(key,_)|key!=&b.key){self.original=None;}
  if self.original.is_none()&&!force&&b.until.is_some_and(|at|at>Instant::now()+Duration::from_secs(60)){return Ok(())}
  if self.original.is_none(){let r=if let Some(r)=&b.restore{r.clone()}else{wire::Request{major:Some(1),revision:Some(protocol::REVISION.into()),service_id:Some(b.service.clone()),service_epoch:Some(b.epoch.clone()),request_id:Some(uuid::Uuid::new_v4().to_string()),refresh:Some(wire::Refresh{device_id:Some(b.device.to_string()),secret:Some(b.session.as_ref().ok_or("No signed-in refresh intent")?.refresh.as_str().into())}),..Default::default()}};self.original=Some((b.key.clone(),r));}
  let r=self.original.as_ref().unwrap().1.clone();let result=self.refresh(&b,&r).await;
  match result{Ok((session,until))=>{
   // Publish RAM auth before retrying potentially slow protected storage.
   {let mut f=self.feed.lock().unwrap();if f.binding.as_ref().is_none_or(|current|current.revision!=b.revision||current.key!=b.key){return Ok(())}let generation=f.credential_generation.checked_add(1).ok_or("Credential generation exhausted")?;f.binding.as_mut().unwrap().session=Some(session.clone());f.binding.as_mut().unwrap().restore=None;f.binding.as_mut().unwrap().until=Some(until);f.credential_generation=generation;f.output=Some(session.clone());f.force=false;f.error=None;f.renewals+=1;}
   self.original=None;self.commit=Some((b.clone(),session.clone()));let stored=self.persist(&b,session.refresh.as_str(),"",0).await;if let Err(e)=stored{self.feed.lock().unwrap().error=Some(e.clone());return Err(e)}self.commit=None;Ok(())
  },Err(e)=>{let mut f=self.feed.lock().unwrap();if f.binding.as_ref().is_some_and(|current|current.revision==b.revision&&current.key==b.key){f.error=Some(e.clone());}Err(e)}}
 }
 async fn persist(&self,b:&Binding,refresh:&str,request_id:&str,started:u64)->Result<(),String>{
  let(store,writer,feed)=(self.store.clone(),self.writer.clone(),self.feed.clone());let b=b.clone();let refresh=SecretString::from(refresh);let request_id=request_id.to_string();
  tokio::task::spawn_blocking(move||{let _writer=writer.lock().unwrap();if feed.lock().unwrap().binding.as_ref().is_none_or(|c|c.revision!=b.revision||c.key!=b.key){return Err("Auth persistence owner retired".into())}if let Some(profile)=&b.profile{if b.remember{let record=Remembered{revision:protocol::REVISION.into(),device:b.device.to_string(),refresh:refresh.as_str().into(),request_id,started};let bytes=zeroize::Zeroizing::new(serde_json::to_vec(&record).map_err(|_|"Auth journal encoding")?);store.save(profile,&SecretBlob::new(bytes.to_vec()).map_err(|_|"Auth journal bound")?).map_err(|e|format!("Auth protected storage unavailable: {e}"))?;}else{store.delete(profile).map_err(|_|"Auth local forget failed")?;}}Ok(())}).await.map_err(|_|"Auth storage worker canceled")?
 }
 async fn refresh(&mut self,b:&Binding,r:&wire::Request)->Result<(Session,Instant),String>{
  self.diagnostic(b,"persist-original",json!({}));
  self.persist(b,r.refresh.as_ref().and_then(|r|r.secret.as_deref()).ok_or("Auth original secret absent")?,r.request_id.as_deref().unwrap(),now()).await?;
  self.sequence+=1;let stamp=Stamp{sequence:self.sequence,started:Instant::now()};let url=format!("{}/v1/rpc",b.origin);let bytes=zeroize::Zeroizing::new(serde_json::to_vec(r).map_err(|_|"Auth request encoding")?);
  self.diagnostic(b,"http",json!({}));let response=tokio::time::timeout(Duration::from_secs(3),async{let mut reply=b.http.post(&url).header("content-type","application/json").body(bytes.to_vec()).send().await.map_err(|_|"Auth transport unavailable")?;self.diagnostic(b,"http-response",json!({"status":reply.status().as_u16(),"url_matches":reply.url().as_str()==url}));if reply.url().as_str()!=url||!reply.status().is_success(){return Err("Auth transport refused".into())}let mut bytes=zeroize::Zeroizing::new(Vec::new());while let Some(part)=reply.chunk().await.map_err(|_|"Auth reply interrupted")?{if bytes.len()+part.len()>65536{return Err("Auth reply bound".into())}bytes.extend_from_slice(&part)}Ok::<_,String>(bytes)}).await.map_err(|_|"Auth whole request deadline")??;
  let mut reply=protocol::response(&response).map_err(|_|"Auth response violates schema")?;if reply.request_id!=r.request_id||reply.service_id.as_deref()!=Some(&b.service)||reply.service_epoch.as_deref()!=Some(&b.epoch){return Err("Auth response scope changed".into())}if let Some(e)=reply.error{self.diagnostic(b,"server-error",json!({"code":e.code,"credentials_present":reply.credentials.is_some(),"validity_present":reply.credential_validity.is_some()}));return Err(e.code.unwrap_or("Authentication refused".into()))}
  self.diagnostic(b,"validated-response",json!({"credentials_present":reply.credentials.is_some(),"validity_present":reply.credential_validity.is_some()}));let session=Session::read(reply.credentials.take().ok_or("Refreshed credentials absent")?,&b.device.to_string(),"desktop")?;let identity=b.session.as_ref().is_none_or(|old|session.id==old.id&&session.epoch==old.epoch&&session.account==old.account);self.diagnostic(b,"credential-identity",json!({"identity_matches":identity}));if !identity{return Err("Renewed authentication identity changed".into())}
  let v=serde_json::from_value(serde_json::to_value(reply.credential_validity.as_ref().ok_or("Auth credential sample absent")?).map_err(|_|"Auth validity encoding")?).map_err(|_|"Auth validity shape")?;
  let until=admission::credential_sample(&mut Default::default(),&b.origin,&b.service,&session,&v,r.request_id.as_deref().unwrap(),stamp)?;self.diagnostic(b,"sample-validated",json!({"identity_matches":identity,"validity_present":true,"credential_sample_valid":true,"http_status":200,"url_matches":true,"request_scope_matches":true}));Ok((session,until))
 }
}
