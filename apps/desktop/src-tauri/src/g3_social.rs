//! Rust-only social producer/HTTP seam. Core must supply verified frozen authority.
//! This module is not enabled by the native seed/ghost candidate.
use crate::{g3_local_consumer::Binding,protocol_g3_social::{self as protocol,wire}};
use sha2::{Digest,Sha256};
use std::{collections::{BTreeMap,VecDeque},sync::{Arc,Mutex,atomic::{AtomicU64,Ordering}},time::{Duration,Instant}};
use zeroize::Zeroizing;
pub fn event_identity(scope:&wire::Scope,session:&str,kind:&str,sequence:u32)->Result<(Vec<u8>,uuid::Uuid),String>{
 if !["hud","emote","cosmetic"].contains(&kind)||sequence==0||session.is_empty(){return Err("Invalid transient identity".into());}
 protocol::typed::<wire::Scope>("Scope",&serde_json::to_vec(scope).unwrap()).map_err(|_|"Invalid scope")?;
 let values=serde_json::json!([protocol::POLICY,scope.room_id.as_ref().ok_or("Missing room")?,scope.run_id.as_ref().ok_or("Missing run")?,
  scope.run_epoch.as_ref().ok_or("Missing run epoch")?,scope.config_revision.ok_or("Missing config")?,scope.attachment_id.as_ref().ok_or("Missing attachment")?,
  scope.world_token.as_ref().ok_or("Missing world")?,session,kind,sequence]);
 let bytes=serde_json::to_vec(&values).unwrap();let digest=Sha256::digest(&bytes);let mut id:[u8;16]=digest[..16].try_into().unwrap();
 id[6]=(id[6]&15)|128;id[8]=(id[8]&63)|128;Ok((bytes,uuid::Uuid::from_bytes(id)))
}
pub struct Owner{
 binding:Binding,session:String,scope:wire::Scope,token:Zeroizing<String>,http:reqwest::Client,fence:Arc<AtomicU64>,generation:u64,
 discovered:bool,live:bool,next:Instant,sequences:BTreeMap<String,u32>,originals:BTreeMap<String,Vec<u8>>,recent:BTreeMap<String,VecDeque<String>>,
 credentials:Option<CredentialFeed>,
}
/// Local timing for one fully correlated snapshot response. Never wire data.
pub struct TimedSnapshot{pub response:wire::Response,pub request_started:Instant}
/// Same-session verified credentials shared by health and transport owners.
/// No renderer input, serialisation or history reset; retirement clears the slot.
#[derive(Clone,Default)]
pub struct CredentialFeed(Arc<Mutex<Option<(Binding,String,Zeroizing<String>)>>>);
impl CredentialFeed{
 pub fn retire(&self){*self.0.lock().unwrap()=None;}
 pub fn publish(&self,binding:Binding,session:String,token:Zeroizing<String>)->Result<(),String>{
  valid_token(&token)?;let mut slot=self.0.lock().unwrap();
  if slot.as_ref().is_some_and(|(old,id,_)|id!=&session||!same_binding(old,&binding)){return Err("Changed social credential authority".into());}
  *slot=Some((binding,session,token));Ok(())
 }
 pub fn renew(&self,session:&str,epoch:&str,principal:&str,attachment:&str,context:[u8;16],token:Zeroizing<String>)->Result<(),String>{
  valid_token(&token)?;let mut slot=self.0.lock().unwrap();let Some((binding,id,current))=slot.as_mut() else{return Ok(());};
  if id!=session||binding.session_epoch!=epoch||binding.principal_id!=principal||binding.context!=context||binding.scope.attachment_id.as_deref()!=Some(attachment){return Err("Changed social credential authority".into());}
  *current=token;Ok(())
 }
 fn read(&self,binding:&Binding,session:&str)->Result<Zeroizing<String>,String>{
  let slot=self.0.lock().unwrap();let (old,id,token)=slot.as_ref().ok_or("Retired social credentials")?;
  if id!=session||!same_binding(old,binding){return Err("Changed social credential authority".into());}Ok(token.clone())
 }
}
fn valid_token(token:&str)->Result<(),String>{if token.is_empty()||token.len()>4096||token.bytes().any(|b|b<=32||b>=127){Err("Invalid social credential renewal".into())}else{Ok(())}}
fn same_binding(a:&Binding,b:&Binding)->bool{a.context==b.context&&a.service_origin==b.service_origin&&a.service_id==b.service_id&&a.service_epoch==b.service_epoch&&a.session_epoch==b.session_epoch&&a.principal_id==b.principal_id&&serde_json::to_vec(&a.scope).unwrap()==serde_json::to_vec(&b.scope).unwrap()}
impl Owner{
 pub fn original_owner(&self)->String{serde_json::to_string(&(&self.session,&self.scope)).unwrap()}
 pub fn restore_social(&mut self,publish:wire::Publish)->Result<String,String>{
  self.active()?;let kind=if publish.emote.is_some()&&publish.cosmetic.is_none(){"emote"}else if publish.cosmetic.is_some()&&publish.emote.is_none(){"cosmetic"}else{return Err("Expected A3 original".into())};
  if publish.hud.is_some()||publish.poi.is_some(){return Err("Mixed A3 original".into())}let sequence=publish.sequence.filter(|n|*n>0).ok_or("A3 original sequence")?;
  let id=event_identity(&self.scope,&self.session,kind,sequence)?.1.to_string();if publish.event_id.as_ref()!=Some(&id){return Err("A3 retired original owner".into())}
  self.sequence_floor(kind,sequence);if let Some(bytes)=self.originals.get(&id){let old:wire::Request=serde_json::from_slice(bytes).map_err(|_|"A3 original encoding")?;if serde_json::to_vec(&old.publish).unwrap()!=serde_json::to_vec(&Some(publish)).unwrap(){return Err("Substituted A3 original".into())}return Ok(id)}let mut request=self.base();request.publish=Some(publish);let bytes=serde_json::to_vec(&request).unwrap();protocol::request(&bytes).map_err(|_|"A3 original shape")?;
  if self.originals.len()>=512{return Err("Social original capacity".into())}self.originals.insert(id.clone(),bytes);Ok(id)
 }
 pub fn world(&self)->&str{self.scope.world_token.as_deref().unwrap_or("")}
 pub fn credential_feed(&mut self,feed:CredentialFeed)->Result<(),String>{self.active()?;self.token=feed.read(&self.binding,&self.session)?;self.credentials=Some(feed);Ok(())}
 pub fn authority_matches(&self,binding:&Binding,session:&str)->bool{
  self.session==session&&self.binding.context==binding.context&&self.binding.service_origin==binding.service_origin
   &&self.binding.service_id==binding.service_id&&self.binding.service_epoch==binding.service_epoch
   &&self.binding.session_epoch==binding.session_epoch&&self.binding.principal_id==binding.principal_id
   &&serde_json::to_vec(&self.binding.scope).unwrap()==serde_json::to_vec(&binding.scope).unwrap()
 }
 /// Rust Core integration seam only; construction is not proof of authority.
 pub fn new(binding:Binding,session:String,token:Zeroizing<String>,http:reqwest::Client,fence:Arc<AtomicU64>)->Result<Self,String>{
  let url=reqwest::Url::parse(&binding.service_origin).map_err(|_|"Invalid social origin")?;
  if url.scheme()!="https"||url.host_str().is_none()||!url.username().is_empty()||url.password().is_some()||url.path()!="/"||url.query().is_some()||url.fragment().is_some()
   ||url.as_str().trim_end_matches('/')!=binding.service_origin||session.is_empty()||token.is_empty()||token.len()>4096||token.bytes().any(|b|b<=32||b>=127){return Err("Invalid social owner".into());}
  let scope=serde_json::from_value(serde_json::to_value(&binding.scope).unwrap()).unwrap();
  Ok(Self{binding,session,scope,token,http,generation:fence.load(Ordering::Acquire),fence,discovered:false,live:true,next:Instant::now(),sequences:Default::default(),originals:Default::default(),recent:Default::default(),credentials:None})
 }
 fn active(&self)->Result<(),String>{if !self.live||self.fence.load(Ordering::Acquire)!=self.generation{return Err("Retired social owner".into());}Ok(())}
 pub fn pause(&mut self){self.live=false;}
 /// Temporary hold only, same identity/generation. Terminal replacement needs a new owner.
 pub fn resume(&mut self)->Result<(),String>{if self.fence.load(Ordering::Acquire)!=self.generation{return Err("Retired social owner".into());}self.live=true;Ok(())}
 fn base(&self)->wire::Request{wire::Request{major:Some(1),revision:Some(protocol::REVISION.into()),service_id:Some(self.binding.service_id.clone()),
  service_epoch:Some(self.binding.service_epoch.clone()),session_epoch:Some(self.binding.session_epoch.clone()),request_id:Some(uuid::Uuid::new_v4().to_string()),scope:Some(self.scope.clone()),..Default::default()}}
 pub fn begin(&mut self,mut publish:wire::Publish)->Result<String,String>{
  self.active()?;if !self.discovered{return Err("Social discovery required".into());}
  if publish.event_id.is_some()||publish.sequence.is_some(){return Err("Caller cannot substitute social identity".into());}
  let kinds=[("hud",publish.hud.is_some()),("emote",publish.emote.is_some()),("cosmetic",publish.cosmetic.is_some()),("poi",publish.poi.is_some())];
  let selected:Vec<_>=kinds.into_iter().filter(|(_,present)|*present).collect();if selected.len()!=1{return Err("One social payload required".into());}let kind=selected[0].0;
  let sequence=self.sequences.get(kind).copied().unwrap_or(0).checked_add(1).ok_or("Social sequence exhausted")?;
  let id=if kind=="poi"{if self.recent.get(kind).is_some_and(|r|r.len()>=256){return Err("POI original capacity".into());}uuid::Uuid::new_v4()}
   else{event_identity(&self.scope,&self.session,kind,sequence)?.1}.to_string();
  publish.sequence=Some(sequence);publish.event_id=Some(id.clone());let mut request=self.base();request.publish=Some(publish);
  let bytes=serde_json::to_vec(&request).unwrap();protocol::request(&bytes).map_err(|_|"Invalid social payload")?;
  self.sequences.insert(kind.into(),sequence);self.originals.insert(id.clone(),bytes);let recent=self.recent.entry(kind.into()).or_default();recent.push_back(id.clone());
  if kind!="poi"&&recent.len()>64{self.originals.remove(&recent.pop_front().unwrap());}Ok(id)
 }
 pub fn original(&self,id:&str)->Result<&[u8],String>{self.originals.get(id).map(Vec::as_slice).ok_or("CONFLICT: original retired; no effect".into())}
 pub fn sequence_floor(&mut self,kind:&str,floor:u32){let value=self.sequences.entry(kind.into()).or_default();*value=(*value).max(floor);}
 pub fn confirmed_poi_refusal(&mut self,id:&str){self.originals.remove(id);if let Some(ids)=self.recent.get_mut("poi"){ids.retain(|v|v!=id);}}
 /// Preserve the operation, not a retired authorization envelope. A canonical
 /// same-world rebind may refresh only transport identity around these bytes.
 pub fn restore_poi(&mut self,publish:wire::Publish)->Result<String,String>{
  self.active()?;if publish.poi.is_none()||publish.hud.is_some()||publish.emote.is_some()||publish.cosmetic.is_some(){return Err("Expected original POI".into())}
  let id=publish.event_id.clone().ok_or("Original POI identity")?;if self.originals.contains_key(&id){return Ok(id)}
  let mut request=self.base();request.publish=Some(publish);let bytes=serde_json::to_vec(&request).unwrap();protocol::request(&bytes).map_err(|_|"Original POI shape")?;
  if self.originals.len()>=512{return Err("Social original capacity".into())}self.originals.insert(id.clone(),bytes);Ok(id)
 }
 pub async fn roster(&mut self)->Result<wire::Response,String>{let mut request=self.base();request.roster=Some(wire::Snapshot{after_key:None,limit:Some(32)});self.send(serde_json::to_vec(&request).unwrap()).await}
 pub async fn poi_outcome(&mut self,id:&str)->Result<wire::Response,String>{let mut request=self.base();request.poi_outcome=Some(wire::PoiLookup{event_id:Some(id.into())});self.send(serde_json::to_vec(&request).unwrap()).await}
 /// A lookup can confirm only this principal/world's unchanged original.
 /// Not-found is an observation, never permission to replace an operation.
 pub fn original_outcome(&self,publish:&wire::Publish,response:&wire::Response)->Result<bool,String>{
  let outcome=response.poi_outcome.as_ref().ok_or("Missing original outcome")?;
  if outcome.event_id!=publish.event_id{return Err("Substituted POI outcome".into())}
  match outcome.status.as_deref(){
   Some("not-found")if outcome.marker.is_none()=>Ok(false),
   Some("accepted")=>{let marker=outcome.marker.as_ref().ok_or("Missing accepted marker")?;let scope=marker.scope.as_ref().ok_or("Missing accepted marker scope")?;
    if marker.event_id!=publish.event_id||marker.principal_id.as_deref()!=Some(&self.binding.principal_id)||scope.room_id!=self.scope.room_id||scope.run_id!=self.scope.run_id||scope.run_epoch!=self.scope.run_epoch||scope.config_revision!=self.scope.config_revision||scope.world_token!=self.scope.world_token||marker.provenance.as_deref()!=Some("validated-client-report")||serde_json::to_vec(&marker.poi).unwrap()!=serde_json::to_vec(&publish.poi).unwrap()||marker.poi_sequence.as_deref().and_then(|n|n.parse::<u64>().ok()).is_none_or(|n|n==0){return Err("Foreign/substituted accepted marker".into())}Ok(true)
   },_=>Err("Invalid POI outcome state/shape".into())
  }
 }
 async fn bounded(response:reqwest::Response)->Result<Vec<u8>,String>{
  if !response.status().is_success()||response.content_length().is_some_and(|size|size>32768){return Err("Social response refused".into());}
  let mut response=response;let mut bytes=Vec::new();while let Some(chunk)=response.chunk().await.map_err(|_|"Social response interrupted")?{
   if bytes.len()+chunk.len()>32768{return Err("Social response capacity".into());}bytes.extend_from_slice(&chunk);
  }Ok(bytes)
 }
 pub async fn discover(&mut self)->Result<(),String>{
  self.active()?;if self.discovered{return Ok(())}
  let response=tokio::time::timeout(Duration::from_secs(3),async{
   let response=self.http.get(format!("{}/v3/social/info",self.binding.service_origin)).send().await.map_err(|_|"Social discovery unavailable")?;Self::bounded(response).await
  }).await.map_err(|_|"Social discovery deadline")??;
  self.accept_discovery(&response)
 }
 fn accept_discovery(&mut self,bytes:&[u8])->Result<(),String>{
  self.active()?;if bytes.len()>32768{return Err("Social discovery capacity".into());}
  let info:serde_json::Value=serde_json::from_slice(bytes).map_err(|_|"Invalid social discovery")?;
  if info["major"]!=1||info["revision"]!=protocol::REVISION||info["schemaSha256"]!=protocol::SCHEMA_SHA||info["replayPolicy"]!=protocol::POLICY
   ||info["transientRetryRecordsPerSenderKind"]!=64||info["serviceId"]!=self.binding.service_id||info["serviceEpoch"]!=self.binding.service_epoch{return Err("Social schema/policy/authority mismatch".into());}
  self.discovered=true;Ok(())
 }
 pub async fn send_original(&mut self,id:&str)->Result<wire::Response,String>{
  self.active()?;let bytes=self.original(id)?.to_vec();self.send(bytes).await
 }
 pub async fn snapshot(&mut self,after:&str)->Result<wire::Response,String>{
  let mut request=self.base();request.snapshot=Some(wire::Snapshot{after_key:Some(after.into()),limit:Some(32)});self.send(serde_json::to_vec(&request).unwrap()).await
 }
 pub async fn timed_snapshot(&mut self,after:&str)->Result<TimedSnapshot,String>{
  let mut request=self.base();request.snapshot=Some(wire::Snapshot{after_key:Some(after.into()),limit:Some(32)});self.send_timed(serde_json::to_vec(&request).unwrap()).await
 }
 /// Latest-state keyset has no server snapshot/high token. Frozen selected scope,
 /// at most96 rows/eight pages and a whole-scan deadline bound concurrent refresh.
 pub async fn latest(&mut self)->Result<wire::Response,String>{
  tokio::time::timeout(Duration::from_secs(3),async{
   let mut scan=crate::g3_social_projection::LatestScan::default();
   loop{if scan.pages()>0{tokio::time::sleep(Duration::from_millis(100)).await;}
    let response=self.snapshot(scan.after()).await?;if scan.accept(&response)?{return scan.finish();}
   }
  }).await.map_err(|_|"Social latest scan deadline")?
 }
 pub async fn poi_snapshot(&mut self,after:&str)->Result<wire::Response,String>{
  let mut request=self.base();request.poi_snapshot=Some(wire::Page{after_sequence:Some(after.into()),limit:Some(32)});self.send(serde_json::to_vec(&request).unwrap()).await
 }
 pub async fn dismiss(&mut self,sequence:&str)->Result<wire::Response,String>{
  let mut request=self.base();request.dismiss_poi=Some(wire::Dismiss{poi_sequence:Some(sequence.into())});self.send(serde_json::to_vec(&request).unwrap()).await
 }
 async fn send(&mut self,bytes:Vec<u8>)->Result<wire::Response,String>{
  Ok(self.send_timed(bytes).await?.response)
 }
 async fn send_timed(&mut self,bytes:Vec<u8>)->Result<TimedSnapshot,String>{
  let request_started=Instant::now();
  if let Some(feed)=&self.credentials{self.token=feed.read(&self.binding,&self.session)?;}
  self.active()?;if !self.discovered{return Err("Social discovery missing".into());}if Instant::now()<self.next{tokio::time::sleep(self.next.saturating_duration_since(Instant::now())).await;self.active()?;}
  let request=protocol::request(&bytes).map_err(|_|"Invalid social request")?;self.next=Instant::now()+Duration::from_millis(100);
  let result=tokio::time::timeout(Duration::from_secs(3),async{
   let response=self.http.post(format!("{}/v3/social/rpc",self.binding.service_origin)).bearer_auth(self.token.as_str()).header("content-type","application/json").body(bytes).send().await.map_err(|_|"Social transport unavailable")?;Self::bounded(response).await
  }).await.map_err(|_|"Social deadline")??;
  self.active()?;let response=protocol::response(&result).map_err(|_|"Invalid social response")?;
  if response.service_id!=request.service_id||response.service_epoch!=request.service_epoch||response.session_epoch!=request.session_epoch||response.request_id!=request.request_id
   ||serde_json::to_vec(&response.scope).unwrap()!=serde_json::to_vec(&request.scope).unwrap(){return Err("Substituted social reply".into());}
  if let Some(error)=response.error{return Err(error.code.unwrap_or("Social refusal".into()));}
  if response.accepted_event_id!=request.publish.and_then(|p|p.event_id){return Err("Wrong social original acknowledgement".into());}Ok(TimedSnapshot{response,request_started})
 }
}
#[cfg(test)] mod tests{
 use super::*;
 #[test]fn a3_exact_original_retry_owner_and_payload_fences(){
  let mut o=owner();o.discovered=true;let id=o.begin(wire::Publish{emote:Some(wire::Emote{emote_id:Some("wave".into()),active:Some(false)}),..Default::default()}).unwrap();let original=o.original(&id).unwrap().to_vec();let p:wire::Request=serde_json::from_slice(&original).unwrap();let p=p.publish.unwrap();assert_eq!(o.restore_social(p.clone()).unwrap(),id);assert_eq!(o.original(&id).unwrap(),original);
  let mut changed=p.clone();changed.emote.as_mut().unwrap().active=Some(true);assert!(o.restore_social(changed).is_err());assert_eq!(o.original(&id).unwrap(),original);
  let mut fresh=owner();fresh.discovered=true;assert_eq!(fresh.restore_social(p.clone()).unwrap(),id);let request:wire::Request=serde_json::from_slice(fresh.original(&id).unwrap()).unwrap();assert_eq!(serde_json::to_vec(&request.publish).unwrap(),serde_json::to_vec(&Some(p.clone())).unwrap());fresh.scope.attachment_id=Some(uuid::Uuid::new_v4().to_string());assert!(fresh.restore_social(p).is_err());
 }
 #[test]fn a2_outcome_validates_original_payload_world_and_not_found_is_not_replacement(){
  let mut o=owner();o.discovered=true;let id=o.begin(wire::Publish{poi:Some(wire::Poi{text:Some("literal 雪".into()),x_milli:Some("1000".into()),y_milli:Some("2000".into())}),..Default::default()}).unwrap();let original:wire::Request=serde_json::from_slice(o.original(&id).unwrap()).unwrap();let publish=original.publish.unwrap();
  let mut response=wire::Response{poi_outcome:Some(wire::PoiOutcome{event_id:Some(id.clone()),status:Some("not-found".into()),marker:None}),..Default::default()};assert!(!o.original_outcome(&publish,&response).unwrap());let bytes=o.original(&id).unwrap().to_vec();assert_eq!(o.restore_poi(publish.clone()).unwrap(),id);assert_eq!(o.original(&id).unwrap(),bytes);
  let marker=wire::Marker{event_id:Some(id.clone()),poi_sequence:Some("1".into()),principal_id:Some(o.binding.principal_id.clone()),scope:Some(o.scope.clone()),poi:publish.poi.clone(),provenance:Some("validated-client-report".into()),..Default::default()};let outcome=response.poi_outcome.as_mut().unwrap();outcome.status=Some("accepted".into());outcome.marker=Some(marker);assert!(o.original_outcome(&publish,&response).unwrap());response.poi_outcome.as_mut().unwrap().marker.as_mut().unwrap().poi.as_mut().unwrap().text=Some("substituted".into());assert!(o.original_outcome(&publish,&response).is_err());response.poi_outcome.as_mut().unwrap().marker.as_mut().unwrap().poi=publish.poi.clone();response.poi_outcome.as_mut().unwrap().marker.as_mut().unwrap().scope.as_mut().unwrap().world_token=Some(uuid::Uuid::new_v4().to_string());assert!(o.original_outcome(&publish,&response).is_err());assert_eq!(o.original(&id).unwrap(),bytes);
 }
 #[test]fn c1_renewed_credential_feed_preserves_originals_and_refuses_replacement(){
  let mut owner=owner();owner.discovered=true;let feed=CredentialFeed::default();
  feed.publish(owner.binding.clone(),owner.session.clone(),Zeroizing::new("old-token".into())).unwrap();owner.credential_feed(feed.clone()).unwrap();
  let id=owner.begin(wire::Publish{hud:Some(wire::Hud{hp_milli:Some(1),max_hp_milli:Some(2),sampo:Some(false),hp_known:Some(true),sampo_known:Some(true),..Default::default()}),..Default::default()}).unwrap();let original=owner.original(&id).unwrap().to_vec();
  let b=&owner.binding;feed.renew(&owner.session,&b.session_epoch,&b.principal_id,b.scope.attachment_id.as_deref().unwrap(),b.context,Zeroizing::new("new-token".into())).unwrap();
  owner.token=feed.read(&owner.binding,&owner.session).unwrap();assert_eq!(owner.token.as_str(),"new-token");assert_eq!(owner.sequences["hud"],1);assert_eq!(owner.original(&id).unwrap(),original);assert_eq!(owner.generation,1);
  let mut changed=owner.binding.clone();changed.context=[2;16];assert!(feed.publish(changed,owner.session.clone(),Zeroizing::new("wrong-token".into())).is_err());
  assert!(feed.renew("replacement",&b.session_epoch,&b.principal_id,b.scope.attachment_id.as_deref().unwrap(),b.context,Zeroizing::new("wrong-token".into())).is_err());
  feed.retire();assert!(feed.read(&owner.binding,&owner.session).is_err());assert_eq!(owner.original(&id).unwrap(),original);
 }
 #[test]fn policy_vectors_utf8_uuid_v8_and_schema_are_exact(){
  let raw=include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../packages/contracts/g3-social/replay-policy.json"));
  let hex=|bytes:&[u8]|bytes.iter().map(|b|format!("{b:02x}")).collect::<String>();assert_eq!(hex(&Sha256::digest(raw)),protocol::POLICY_SHA);
  assert_eq!(hex(&Sha256::digest(include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../packages/contracts/g3-social/schema.json")))),protocol::SCHEMA_SHA);
  let policy:serde_json::Value=serde_json::from_slice(raw).unwrap();for vector in policy["vectors"].as_array().unwrap(){
   let scope=serde_json::from_value(vector["scope"].clone()).unwrap();let (bytes,id)=event_identity(&scope,vector["gameSessionId"].as_str().unwrap(),vector["kind"].as_str().unwrap(),vector["sequence"].as_u64().unwrap() as u32).unwrap();
   assert_eq!(String::from_utf8(bytes.clone()).unwrap(),vector["canonicalJson"]);assert_eq!(hex(&bytes),vector["utf8Hex"]);assert_eq!(hex(&Sha256::digest(bytes)),vector["sha256"]);assert_eq!(id.to_string(),vector["eventId"]);
  }
 }
 fn owner()->Owner{
  let v:serde_json::Value=serde_json::from_slice(include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../packages/contracts/g3-social/replay-policy.json"))).unwrap();
  Owner::new(Binding{context:[1;16],service_origin:"https://fixture.invalid".into(),service_id:"service".into(),service_epoch:"epoch".into(),session_epoch:"session".into(),principal_id:"principal".into(),scope:serde_json::from_value(v["vectors"][0]["scope"].clone()).unwrap()},v["vectors"][0]["gameSessionId"].as_str().unwrap().into(),Zeroizing::new("fixture-token".into()),reqwest::Client::new(),Arc::new(AtomicU64::new(1))).unwrap()
 }
 #[test]fn a2_canonical_discovery_revision_hash_and_request_identity_do_not_drift(){
  let schema_raw=include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../packages/contracts/g3-social/schema.json"));
  let schema:serde_json::Value=serde_json::from_slice(schema_raw).unwrap();
  let policy:serde_json::Value=serde_json::from_slice(include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../packages/contracts/g3-social/replay-policy.json"))).unwrap();
  let mut owner=owner();let canonical=serde_json::json!({"major":schema["major"],"revision":schema["revision"],"schemaSha256":format!("{:x}",Sha256::digest(schema_raw)),"replayPolicy":policy["policy"],"transientRetryRecordsPerSenderKind":policy["recordsPerSenderKind"],"serviceId":"service","serviceEpoch":"epoch"});
  owner.accept_discovery(&serde_json::to_vec(&canonical).unwrap()).unwrap();
  let id=owner.begin(wire::Publish{hud:Some(wire::Hud{hp_known:Some(true),hp_milli:Some(0),max_hp_milli:Some(8000),..Default::default()}),..Default::default()}).unwrap();let request:serde_json::Value=serde_json::from_slice(owner.original(&id).unwrap()).unwrap();assert_eq!(request["revision"],schema["revision"]);protocol::request(owner.original(&id).unwrap()).unwrap();
  for (field,bad) in [("revision",serde_json::json!("g3-social-draft-1")),("schemaSha256",serde_json::json!("829dea7c1363cd588ddbd6aa7515d401f394c71ecb997a6437180b3e1ddf7c6e")),("serviceEpoch",serde_json::json!("retired"))]{let mut changed=canonical.clone();changed[field]=bad;assert!(owner.accept_discovery(&serde_json::to_vec(&changed).unwrap()).is_err());}
 }
 #[test]fn discovery_policy_and_retry_retirement_hold_sequences_are_bounded(){
  let mut owner=owner();let good=serde_json::json!({"major":1,"revision":protocol::REVISION,"schemaSha256":protocol::SCHEMA_SHA,"replayPolicy":protocol::POLICY,"transientRetryRecordsPerSenderKind":64,"serviceId":"service","serviceEpoch":"epoch"});
  for field in ["schemaSha256","replayPolicy","transientRetryRecordsPerSenderKind"]{let mut changed=good.clone();changed[field]=serde_json::json!("old");assert!(owner.accept_discovery(&serde_json::to_vec(&changed).unwrap()).is_err());}
  owner.accept_discovery(&serde_json::to_vec(&good).unwrap()).unwrap();
  let payload=||wire::Publish{hud:Some(wire::Hud{hp_milli:Some(1),max_hp_milli:Some(2),sampo:Some(false),hp_known:Some(true),sampo_known:Some(true),..Default::default()}),..Default::default()};
  let first=owner.begin(payload()).unwrap();let bytes=owner.original(&first).unwrap().to_vec();owner.pause();assert!(owner.begin(payload()).is_err());assert_eq!(owner.original(&first).unwrap(),bytes);
  owner.resume().unwrap();for _ in 0..64{owner.begin(payload()).unwrap();}assert!(owner.original(&first).unwrap_err().contains("CONFLICT"));assert_eq!(owner.originals.len(),64);assert_eq!(owner.sequences["hud"],65);
  let poi=owner.begin(wire::Publish{poi:Some(wire::Poi{text:Some("literal 雪 <img>".into()),x_milli:Some("1".into()),y_milli:Some("2".into())}),..Default::default()}).unwrap();assert_ne!(uuid::Uuid::parse_str(&poi).unwrap().get_version_num(),8);
  owner.fence.fetch_add(1,Ordering::AcqRel);assert!(owner.resume().is_err());assert!(owner.begin(payload()).is_err());
 }
 #[test]
 #[ignore="explicit server-owned social HTTPS fixture; simulated engine facts only"]
 fn actual_social_https_policy_original_retry_snapshot_and_private_dismissal(){
  assert_eq!(std::env::var("NT2_G3_SOCIAL_TEST").as_deref(),Ok("1"));
  tauri::async_runtime::block_on(async{
   let mut fixture=crate::g3_http::https_test::Fixture::start_social();let bundle=fixture.bundle();assert_eq!(bundle.evidence_status,"simulated-game-facts");
   let http=reqwest::Client::builder().tls_backend_native().tls_certs_only(vec![reqwest::Certificate::from_pem(&std::fs::read(&bundle.ca_path).unwrap()).unwrap()]).no_proxy().redirect(reqwest::redirect::Policy::none()).build().unwrap();
   let create=|index:usize|Owner::new(Binding{context:[index as u8+1;16],service_origin:bundle.origin.clone(),service_id:bundle.service_id.clone(),service_epoch:bundle.service_epoch.clone(),
    session_epoch:bundle.games[index].session_epoch.clone(),principal_id:bundle.games[index].account_id.clone(),scope:bundle.scopes[index].clone()},bundle.games[index].session_id.clone(),Zeroizing::new(bundle.games[index].access_token.clone()),http.clone(),Arc::new(AtomicU64::new(1))).unwrap();
   let mut sender=create(0);let mut recipient=create(1);sender.discover().await.unwrap();recipient.discover().await.unwrap();
   let id=sender.begin(wire::Publish{hud:Some(wire::Hud{hp_milli:Some(1000),max_hp_milli:Some(2000),sampo:Some(true),hp_known:Some(true),sampo_known:Some(true),..Default::default()}),..Default::default()}).unwrap();
   assert!(sender.send_original(&id).await.is_err());fixture.command("start");std::thread::sleep(Duration::from_millis(110));assert!(sender.send_original(&id).await.is_err());
   fixture.command("playing");std::thread::sleep(Duration::from_millis(110));let accepted=sender.send_original(&id).await.unwrap();assert_eq!(accepted.accepted_event_id.as_deref(),Some(id.as_str()));
   std::thread::sleep(Duration::from_millis(110));assert_eq!(sender.send_original(&id).await.unwrap().duplicate,Some(true));
   let snapshot=recipient.snapshot("").await.unwrap();assert!(snapshot.projections.as_ref().unwrap().iter().any(|p|p.event_id.as_deref()==Some(&id)&&p.principal_id.as_deref()==Some(&bundle.games[0].account_id)&&p.hud.as_ref().unwrap().sampo==Some(true)));
   let poi=sender.begin(wire::Publish{poi:Some(wire::Poi{text:Some("雪 <img onerror=literal>".into()),x_milli:Some("1000".into()),y_milli:Some("2000".into())}),..Default::default()}).unwrap();
   std::thread::sleep(Duration::from_millis(110));sender.send_original(&poi).await.unwrap();std::thread::sleep(Duration::from_millis(110));
   let page=recipient.poi_snapshot("0").await.unwrap();let marker=page.markers.as_ref().unwrap().iter().find(|m|m.event_id.as_deref()==Some(&poi)).unwrap();let sequence=marker.poi_sequence.clone().unwrap();
   assert_eq!(page.next_sequence,page.high_sequence);assert_eq!(marker.poi.as_ref().unwrap().text.as_deref(),Some("雪 <img onerror=literal>"));
   std::thread::sleep(Duration::from_millis(110));recipient.dismiss(&sequence).await.unwrap();std::thread::sleep(Duration::from_millis(110));
   let dismissed=recipient.poi_snapshot("0").await.unwrap();assert!(dismissed.markers.unwrap().is_empty());assert_eq!(dismissed.next_sequence,dismissed.high_sequence);
   let sender_page=sender.poi_snapshot("0").await.unwrap();assert!(!sender_page.markers.unwrap().is_empty());
   fixture.command("end");std::thread::sleep(Duration::from_millis(110));assert!(sender.send_original(&id).await.is_err());
  });
 }
}
