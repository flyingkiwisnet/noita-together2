//! Local diagnostics only: no admission, release, retry or authority decision.
use super::*;
#[derive(Clone,Serialize)]pub(super) struct Issue{pub context:String,pub origin:String,pub code:String,pub component:String,pub confirmed:bool,pub message:String,pub action:String,pub details:Value}
impl Issue{pub fn native_message(&self)->&'static str{match self.component.as_str(){"app_service"=>"App and service versions differ. Use matching NT2 app/service; see app details.","installed_runtime"=>"NT2 app and installed mod/runtime differ. Install matching app/mod and restart; see app details.",_=>"Running NT2 runtime differs from service requirements. Use matching app/mod; see app details."}}}
#[derive(Default)]pub(super) struct Notice{issue:Option<Issue>}
impl Notice{
 pub fn current(&self,context:&str)->Option<&Issue>{self.issue.as_ref().filter(|i|i.context==context)}
 pub fn set(&mut self,issue:Issue){if self.current(&issue.context).is_some_and(|i|i.confirmed&&!issue.confirmed){return}self.issue=Some(issue);}
 pub fn clear(&mut self,component:&str){if self.issue.as_ref().is_some_and(|i|i.component==component){self.issue=None;}}
}
pub(super) fn service_issue(context:String,origin:String,code:&str,bytes:&[u8])->Issue{
 let value=protocol::strict_json(bytes).unwrap_or(Value::Null);let major=value["major"].as_u64();let revision=value["revision"].as_str().filter(|s|s.len()<=64&&!s.chars().any(char::is_control));
 let mut normalized=value.clone();if let Some(object)=normalized.as_object_mut(){object.insert("major".into(),json!(1));object.insert("revision".into(),json!(protocol::REVISION));}
 let confirmed=major.is_some()&&revision.is_some()&&(major!=Some(1)||revision!=Some(protocol::REVISION))&&protocol::info(&serde_json::to_vec(&normalized).unwrap()).is_ok();
 Issue{context,origin,code:code.into(),component:if confirmed{"app_service"}else{"service_response"}.into(),confirmed,message:if confirmed{"App and service protocols do not match."}else{"The service response could not be validated."}.into(),action:if confirmed{"Use the matching NT2 app and service release, then reconnect. See Details."}else{"Check the server address and connection, then retry. This is not a confirmed version mismatch; see Details."}.into(),details:json!({"requiredMajor":1,"requiredProtocol":protocol::REVISION,"advertisedMajor":major,"advertisedProtocol":revision,"code":code})}
}
impl<S:CredentialStore> Core<S>{
 pub(super) fn compatibility_context(&self)->String{json!([self.base,self.service,self.epoch,self.room.as_ref().map(|r|(&r.room_id,&r.run_id,&r.run_epoch,&r.config_revision)),self.world.record.as_ref().map(|r|r.world),self.generation.load(Ordering::Acquire)]).to_string()}
 pub(super) fn compatibility_issue(&self)->Option<&Issue>{self.compatibility_notice.current(&self.compatibility_context())}
 pub(super) fn compatibility_service_failure(&mut self,origin:&str,code:&str,bytes:&[u8]){self.compatibility_notice.set(service_issue(self.compatibility_context(),origin.into(),code,bytes));}
 pub(super) fn compatibility_profile_failure(&mut self,code:&str){self.compatibility_notice.set(Issue{context:self.compatibility_context(),origin:self.base.clone(),code:code.into(),component:"service_profile".into(),confirmed:false,message:"This app could not verify the service's NT2 runtime profile.".into(),action:"Check the selected service and use the matching NT2 app/mod release, then reconnect. See Details.".into(),details:json!({"code":code,"requiredRuntimeId":nt2_desktop_g0::g3_social_installation::DEVELOPMENT_ID,"requiredProtocol":nt2_desktop_g0::protocol_g3_social::REVISION})});}
 pub(super) fn compatibility_verification_failure(&mut self,error:&str){
  match error {"Changed social runtime dependency"|"C2 primary/entry identity differs from complete closure"=>self.compatibility_runtime_failure(error,false),"Unreviewed/wrong social runtime metadata"=>self.compatibility_profile_failure(error),_=>{}}
 }
 pub(super) fn compatibility_runtime_failure(&mut self,code:&str,observed:bool){
  let observed_ids=if observed{let pending=self.pending.lock().unwrap();pending.as_ref().filter(|p|p.owner.is_some()&&(self.world.coordinator.ticket.as_ref().is_some_and(|t|Some(t.native)==p.owner)||self.game.as_ref().is_some_and(|g|Some(g.owner)==p.owner))).and_then(|p|p.observation.as_ref().filter(|r|Some(r.context)==p.owner.map(|o|o.1))).map(|r|json!({"modVersion":r.observation.mod_version,"bridgeVersion":r.observation.bridge_version}))}else{None};
  let component=if observed{"observed_runtime"}else{"installed_runtime"};let message=if observed{"Game/runtime metadata does not match the service requirements."}else{"The installed NT2 runtime does not match this app."};
  self.compatibility_notice.set(Issue{context:self.compatibility_context(),origin:self.base.clone(),code:code.into(),component:component.into(),confirmed:true,message:message.into(),action:"Install the matching NT2 app and mod release, then restart the app and Noita. See Details.".into(),details:json!({"code":code,"required":self.info.as_ref().and_then(|i|i.compatibility.as_ref()),"observedRelease":"unknown","observedRuntime":observed_ids,"requiredRuntimeId":nt2_desktop_g0::g3_social_installation::DEVELOPMENT_ID,"appProtocol":protocol::REVISION})});
 }
}
#[cfg(test)]mod tests{
 use super::*;
 struct TestStore;
 impl CredentialStore for TestStore{
  fn load(&self,_:&ServiceProfile)->Result<Option<SecretBlob>,crate::secrets::StoreError>{Ok(None)}
  fn save(&self,_:&ServiceProfile,_:&SecretBlob)->Result<(),crate::secrets::StoreError>{Ok(())}
  fn delete(&self,_:&ServiceProfile)->Result<(),crate::secrets::StoreError>{Ok(())}
 }

 #[test]fn compatibility_notice_is_sticky_scoped_and_cleared_only_by_matching_validation(){
  let mut n=Notice::default();let issue=service_issue("a".into(),"https://fixture.invalid".into(),"Unsupported candidate contract",b"not JSON");assert!(!issue.confirmed);n.set(issue.clone());assert!(n.current("a").is_some());assert!(n.current("b").is_none());n.clear("installed_runtime");assert!(n.current("a").is_some());n.clear("service_response");assert!(n.current("a").is_none());
  let mut positive=issue.clone();positive.confirmed=true;positive.component="app_service".into();n.set(positive);n.set(issue);assert!(n.current("a").unwrap().confirmed);n.clear("app_service");assert!(n.current("a").is_none());
 }
 #[test]fn compatibility_service_refusal_and_matching_recovery_use_actual_discovery(){
  use std::io::{Read,Write};
  tauri::async_runtime::block_on(async{
   let listener=std::net::TcpListener::bind("127.0.0.1:0").unwrap();let origin=format!("http://{}",listener.local_addr().unwrap());
   let worker=std::thread::spawn(move||{for revision in ["different-protocol",protocol::REVISION]{listener.set_nonblocking(true).unwrap();let until=Instant::now()+Duration::from_secs(8);let(mut socket,_)=loop{match listener.accept(){Ok(pair)=>break pair,Err(e)if e.kind()==std::io::ErrorKind::WouldBlock=>{assert!(Instant::now()<until,"discovery request timeout");std::thread::sleep(Duration::from_millis(5));},Err(e)=>panic!("{e}")}};socket.set_nonblocking(false).unwrap();socket.set_read_timeout(Some(Duration::from_secs(3))).unwrap();let mut request=vec![];let mut b=[0];while !request.ends_with(b"\r\n\r\n"){socket.read_exact(&mut b).unwrap();request.push(b[0]);}assert!(!String::from_utf8_lossy(&request).to_lowercase().contains("authorization:"));let body=json!({"major":1,"revision":revision,"serviceId":"fixture-service","serviceEpoch":"epoch","capabilities":[],"mockProvider":true}).to_string();write!(socket,"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",body.len(),body).unwrap();}});
   let mut c=Core::new(TestStore,uuid::Uuid::new_v4(),Arc::new(Mutex::new(None)),std::env::temp_dir().join(format!("nt2-compat-{}",uuid::Uuid::new_v4())));c.open_browser=false;
   let error=c.select(&origin,"",false).await.unwrap_err();assert_eq!(error,"Service/contract mismatch");assert!(c.compatibility_issue().unwrap().confirmed);assert!(c.compatibility_issue().unwrap().message.contains("App and service"));
   c.status_error=Some("ordinary refresh unavailable".into());assert!(c.view()["compatibility_issue"]["confirmed"]==true);c.select(&origin,"",false).await.unwrap();assert!(c.compatibility_issue().is_none());worker.join().unwrap();
  });
 }

}
