//! Read-only AUTH1 metadata on the existing app runtime. No credential issuer,
//! password storage, provider network calls or gameplay authority lives here.
use super::*;
use std::sync::mpsc::{self,Receiver};
#[path="protocol_auth1.rs"]pub(super) mod protocol;
use protocol::wire as account_wire;
#[cfg(test)]#[path="next_auth1_composition_test.rs"]mod composition;
fn completed<T>(job:&mut Option<Receiver<Result<T,String>>>)->Option<Result<T,String>>{
 let result=match job.as_ref()?.try_recv(){Ok(v)=>v,Err(mpsc::TryRecvError::Empty)=>return None,Err(mpsc::TryRecvError::Disconnected)=>Err("Account worker unavailable".into())};
 *job=None;Some(result)
}
#[cfg(test)]mod tests{
 use super::*;
 fn fixture()->Core<super::super::tests::MemoryStore>{
  let(mut c,_,_)=super::super::tests::release_case();c.base="https://service.test".into();c.info=Some(wire::Info::default());c.auth1.active=true;c.reset_auth1_context();c
 }
 fn projection(c:&Core<impl CredentialStore>,role:&str,revision:u32)->account_wire::Account{
  let values:Vec<Value>=serde_json::from_str(include_str!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../packages/contracts/g3-auth1/fixture-inputs.json"))).unwrap();let mut a=values[1]["json"].clone();
  a["serviceId"]=c.service.clone().into();a["serviceEpoch"]=c.epoch.clone().into();a["account"]["accountId"]=c.desktop.as_ref().unwrap().account.clone().into();a["account"]["role"]=role.into();a["account"]["revision"]=revision.into();
  a["account"]["entitlements"]["customLobbyName"]=(role!="user").into();a["account"]["entitlements"]["capacityLimit"]=(if role=="user"{8}else{16}).into();a["account"]["entitlements"]["manageUsers"]=matches!(role,"admin"|"dev").into();a["account"]["entitlements"]["manageAllRoles"]=(role=="dev").into();a["account"]["entitlements"]["testingMode"]=(role=="dev").into();
  protocol::account(&serde_json::to_vec(&a).unwrap(),&c.service,&c.epoch,&c.desktop.as_ref().unwrap().account).unwrap()
 }
 fn deliver(c:&mut Core<impl CredentialStore>,value:Result<account_wire::Account,String>){let(tx,rx)=mpsc::sync_channel(1);c.auth1.account_job=Some(rx);tx.send(value).unwrap();c.poll_auth1();}
 #[test]fn auth1_http_credentials_routes_redirects_and_unavailable_provider(){tauri::async_runtime::block_on(async{
  use std::io::{Read,Write};
  let listener=std::net::TcpListener::bind("127.0.0.1:0").unwrap();listener.set_nonblocking(true).unwrap();let origin=format!("http://{}",listener.local_addr().unwrap());
  let worker=std::thread::spawn(move||{
   let mut requests=Vec::new();for index in 0..3{
    let until=Instant::now()+Duration::from_secs(10);let mut socket=loop{match listener.accept(){Ok((socket,_))=>break socket,Err(e)if e.kind()==std::io::ErrorKind::WouldBlock=>{assert!(Instant::now()<until);std::thread::sleep(Duration::from_millis(5));},Err(e)=>panic!("{e}")}};
    socket.set_read_timeout(Some(Duration::from_secs(2))).unwrap();let mut bytes=Vec::new();let mut byte=[0];while !bytes.ends_with(b"\r\n\r\n"){socket.read_exact(&mut byte).unwrap();bytes.push(byte[0]);assert!(bytes.len()<8192);}
    requests.push(String::from_utf8(bytes).unwrap().to_lowercase());socket.write_all(if index==2{b"HTTP/1.1 302 Found\r\nLocation: http://127.0.0.1:1/stolen\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"}else{b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}"}).unwrap();
   }requests
  });
  let http=reqwest::Client::builder().no_proxy().redirect(reqwest::redirect::Policy::none()).build().unwrap();
  assert_eq!(get(http.clone(),origin.clone(),"/v1/auth/info",None).await.unwrap(),b"{}");
  assert!(get(http.clone(),origin.clone(),"/v1/auth/account",Some(SecretString::new("fixture-access".into()))).await.is_ok());
  assert!(get(http,origin,"/v1/auth/account",Some(SecretString::new("fixture-access".into()))).await.is_err());
  let requests=worker.join().unwrap();assert!(requests[0].starts_with("get /v1/auth/info "));assert!(!requests[0].contains("authorization:")&&!requests[0].contains("cookie:"));assert!(requests[1].contains("authorization: bearer fixture-access"));assert!(requests[2].contains("authorization: bearer fixture-access"));
  let mut c=fixture();let values:Vec<Value>=serde_json::from_str(include_str!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../packages/contracts/g3-auth1/fixture-inputs.json"))).unwrap();let mut d=values[0]["json"].clone();d["serviceId"]=c.service.clone().into();d["serviceEpoch"]=c.epoch.clone().into();c.auth1.discovery=Some(protocol::discovery(&serde_json::to_vec(&d).unwrap(),&c.service,&c.epoch).unwrap());
  c.open_browser=false;assert!(c.sign_in("google").await.unwrap_err().contains("not supported"));assert!(c.auth.is_none()&&c.auth_begin.is_none());
 });}
 #[test]fn auth1_promotions_demotions_stale_results_and_service_logout_fences_are_ui_only(){
  let mut c=fixture();let original_room=serde_json::to_value(&c.room).unwrap();let original_preparation=serde_json::to_value(&c.preparation).unwrap();let authority=c.pending.lock().unwrap().as_ref().unwrap().authority_until;
  let dev=projection(&c,"dev",2);deliver(&mut c,Ok(dev));assert_eq!(c.auth1_view()["account"]["entitlements"]["testingMode"],true);
  let user=projection(&c,"user",3);deliver(&mut c,Ok(user));assert_eq!(c.auth1_view()["account"]["entitlements"]["testingMode"],false);
  let old=projection(&c,"dev",1);deliver(&mut c,Ok(old));assert_eq!(c.auth1_view()["account"]["role"],"user");
  c.auth1.account.as_mut().unwrap().1=Instant::now()-Duration::from_secs(7);assert!(c.auth1_view()["account"].is_null());
  let dev=projection(&c,"dev",4);deliver(&mut c,Ok(dev));c.generation.fetch_add(1,Ordering::AcqRel);assert!(c.auth1_view()["account"].is_null());c.poll_auth1();
  assert_eq!(serde_json::to_value(&c.room).unwrap(),original_room);assert_eq!(serde_json::to_value(&c.preparation).unwrap(),original_preparation);assert_eq!(c.pending.lock().unwrap().as_ref().unwrap().authority_until,authority);
  let dev=projection(&c,"dev",5);deliver(&mut c,Ok(dev));c.desktop=None;assert!(c.auth1_view()["account"].is_null());assert!(c.open_password_change().is_err());
 }
 #[test]fn auth1_failed_projection_drops_hints_and_uses_existing_auth_owner(){
  let mut c=fixture();let user=projection(&c,"user",1);deliver(&mut c,Ok(user));deliver(&mut c,Err("UNAUTHENTICATED".into()));assert!(c.auth1_view()["account"].is_null());assert!(c.auth_owner.lock().unwrap().fixture_refresh_requested());
  let(tx,rx)=mpsc::sync_channel::<Result<account_wire::Account,String>>(1);c.auth1.account_job=Some(rx);drop(tx);c.poll_auth1();assert!(c.auth1.account_job.is_none());
 }
 #[test]fn auth1_signin_cancel_preserves_selected_world_and_other_authority(){tauri::async_runtime::block_on(async{
  let mut c=fixture();let before_room=serde_json::to_value(&c.room).unwrap();let before_prep=serde_json::to_value(&c.preparation).unwrap();let generation=c.generation.load(Ordering::Acquire);let owner=c.game.as_ref().unwrap().owner;
  c.auth=Some(SignIn::new(&c.base,c.device,generation).unwrap());let callback=c.auth.as_ref().unwrap().begin.callback_port;c.pending_launch=Some("not-opened".into());c.cancel_sign_in().await.unwrap();
  assert!(c.auth.is_none()&&c.pending_launch.is_none());assert_eq!(c.generation.load(Ordering::Acquire),generation);assert_eq!(c.game.as_ref().unwrap().owner,owner);assert_eq!(serde_json::to_value(&c.room).unwrap(),before_room);assert_eq!(serde_json::to_value(&c.preparation).unwrap(),before_prep);assert!(std::net::TcpStream::connect(("127.0.0.1",callback)).is_err());
 });}
}
#[derive(Default)]pub(super) struct State{
 pub(super) active:bool,context:String,discovery:Option<account_wire::Discovery>,
 discovery_job:Option<Receiver<Result<account_wire::Discovery,String>>>,next_discovery:Option<Instant>,
 account_key:String,account:Option<(account_wire::Account,Instant)>,
 account_job:Option<Receiver<Result<account_wire::Account,String>>>,pub(super) next_account:Option<Instant>,
 discovery_error:Option<String>,account_error:Option<String>,
}
async fn get(http:reqwest::Client,origin:String,path:&'static str,token:Option<SecretString>)->Result<Vec<u8>,String>{
 tokio::time::timeout(Duration::from_secs(3),async move{
  let url=format!("{origin}{path}");let mut call=http.get(&url);
  if let Some(token)=token.as_ref(){call=call.bearer_auth(token.as_str());}
  let mut response=call.send().await.map_err(|_|"Account service unavailable")?;
  if response.url().as_str()!=url{return Err("Account redirect refused".into())}
  if response.status()==reqwest::StatusCode::UNAUTHORIZED{return Err("UNAUTHENTICATED".into())}
  if response.status()==reqwest::StatusCode::FORBIDDEN{return Err("FORBIDDEN".into())}
  if !response.status().is_success(){return Err("Account service unavailable".into())}
  let mut bytes=Vec::new();while let Some(part)=response.chunk().await.map_err(|_|"Account response interrupted")?{if bytes.len()+part.len()>65536{return Err("Account response exceeds bounds".into())}bytes.extend_from_slice(&part)}Ok(bytes)
 }).await.map_err(|_|"Account service deadline")?
}
impl<S:CredentialStore> Core<S>{
 fn auth1_context(&self)->String{json!([self.base,self.service,self.epoch,self.generation.load(Ordering::Acquire)]).to_string()}
 fn auth1_account_key(&self)->String{json!([self.auth1_context(),self.desktop.as_ref().map(|s|(&s.account,&s.id,&s.epoch))]).to_string()}
 fn reset_auth1_context(&mut self){
  let context=self.auth1_context();if self.auth1.context!=context{let active=self.auth1.active;self.auth1=State{active,context,..Default::default()};}
  let key=self.auth1_account_key();if self.auth1.account_key!=key{self.auth1.account_key=key;self.auth1.account=None;self.auth1.account_job=None;self.auth1.next_account=None;self.auth1.account_error=None;}
 }
 pub(super) async fn ensure_auth1_discovery(&mut self)->Result<(),String>{
  self.auth1.active=true;self.reset_auth1_context();
  if self.auth1.discovery.is_some(){return Ok(())}
  let context=self.auth1.context.clone();
  let bytes=get(self.http.clone(),self.base.clone(),"/v1/auth/info",None).await?;
  let result=protocol::discovery(&bytes,&self.service,&self.epoch)?;
  if self.auth1_context()!=context{return Err("Sign-in service selection retired".into())}
  self.auth1.discovery=Some(result);self.auth1.discovery_error=None;self.auth1.next_discovery=Some(Instant::now()+Duration::from_secs(30));Ok(())
 }
 pub async fn discover_sign_in(&mut self,origin:&str,remember:bool)->Result<(),String>{
  self.select(origin,"",remember).await?;
  if self.info.as_ref().and_then(|i|i.mock_provider)==Some(true){return Ok(())}
  let result=self.ensure_auth1_discovery().await;self.login_result(&result);result
 }
 pub(super) fn poll_auth1(&mut self){
  if self.info.is_none()||self.service.is_empty()||self.epoch.is_empty(){self.auth1=Default::default();return}
  self.reset_auth1_context();if !self.auth1.active{return}
  if let Some(result)=completed(&mut self.auth1.discovery_job){
   self.auth1.discovery_job=None;match result{Ok(v)=>{self.auth1.discovery=Some(v);self.auth1.discovery_error=None;},Err(e)=>{self.auth1.discovery=None;self.auth1.discovery_error=Some(e);self.auth1.account=None;}}
  }
  if let Some(result)=completed(&mut self.auth1.account_job){
   self.auth1.account_job=None;match result{
    Ok(v)=>{if self.auth1.account.as_ref().is_none_or(|(old,_)|v.revision>=old.revision){self.auth1.account=Some((v,Instant::now()));self.auth1.account_error=None;}},
    Err(e)=>{if e=="UNAUTHENTICATED"{self.auth_owner.lock().unwrap().force();}self.auth1.account=None;self.auth1.account_error=Some(e);}
   }
  }
  let Ok(runtime)=tokio::runtime::Handle::try_current()else{return};
  if self.auth1.discovery_job.is_none()&&self.auth1.next_discovery.is_none_or(|at|Instant::now()>=at){
   let(tx,rx)=mpsc::sync_channel(1);let(http,origin,service,epoch)=(self.http.clone(),self.base.clone(),self.service.clone(),self.epoch.clone());
   runtime.spawn(async move{let result=match get(http,origin,"/v1/auth/info",None).await{Ok(bytes)=>protocol::discovery(&bytes,&service,&epoch),Err(e)=>Err(e)};let _=tx.send(result);});
   self.auth1.discovery_job=Some(rx);self.auth1.next_discovery=Some(Instant::now()+Duration::from_secs(30));
  }
  if self.auth1.discovery.is_some()&&self.auth1.account_job.is_none()&&self.auth1.next_account.is_none_or(|at|Instant::now()>=at){if let Some(session)=self.desktop.as_ref(){
   let(tx,rx)=mpsc::sync_channel(1);let(http,origin,service,epoch,id,token)=(self.http.clone(),self.base.clone(),self.service.clone(),self.epoch.clone(),session.account.clone(),session.access.clone());
   runtime.spawn(async move{let result=match get(http,origin,"/v1/auth/account",Some(token)).await{Ok(bytes)=>protocol::account(&bytes,&service,&epoch,&id),Err(e)=>Err(e)};let _=tx.send(result);});
   self.auth1.account_job=Some(rx);self.auth1.next_account=Some(Instant::now()+Duration::from_secs(2));
  }}
 }
 pub(super) fn auth1_view(&self)->Value{
  let current=self.auth1.context==self.auth1_context();
  let account=(current&&self.auth1.account_key==self.auth1_account_key()).then(||self.auth1.account.as_ref()).flatten().filter(|(_,at)|at.elapsed()<Duration::from_secs(6)).map(|(a,_)|a);
  json!({"revision":"auth1-1","discovered":current&&self.auth1.discovery.is_some(),"providers":if current{self.auth1.discovery.as_ref().and_then(|d|d.providers.as_ref())}else{None},"account":account,"options_error":if current{self.auth1.discovery_error.as_ref()}else{None},"account_error":if current{self.auth1.account_error.as_ref()}else{None},"password_change_available":account.is_some_and(|a|a.provider.as_deref()==Some("local"))})
 }
 pub fn open_password_change(&self)->Result<(),String>{
  if self.auth1_view()["password_change_available"]!=true{return Err("Check the current local account first".into())}
  crate::browser::open_account_password(&self.base)
 }
}
