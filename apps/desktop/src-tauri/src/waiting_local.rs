//! Provisional canonical waiting journal. Protected chunks commit before marker
//! publication. Neither local persistence nor sanitized outcomes grant play.
use crate::{protocol_waiting::{self as protocol,wire as w},startup_local::{self,Scope,WaitingMarker},secrets::{CredentialStore,ServiceProfile,SecretBlob}};
use serde::{Serialize,Deserialize};use sha2::{Digest,Sha256};use zeroize::{Zeroize,Zeroizing};
const MAX:usize=73728;const CHUNK:usize=2048;
#[derive(Clone,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Mutation {pub original:w::WaitingOriginal,pub request:w::Request}
impl Drop for Mutation{fn drop(&mut self){if let Some(r)=self.request.recover.as_mut(){if let Some(p)=r.waiting_proof.as_mut(){p.zeroize()}}if let Some(r)=self.request.offline.as_mut(){if let Some(p)=r.waiting_proof.as_mut(){p.zeroize()}}if let Some(p)=self.request.pending_recover.as_mut().and_then(|r|r.pending_proof.as_mut()){p.zeroize()}if let Some(p)=self.request.pending_offline.as_mut().and_then(|r|r.pending_proof.as_mut()){p.zeroize()}}}
impl Mutation{
 pub fn capture(scope:&Scope,request:w::Request,session:&str)->Result<Self,String>{
  protocol::validate("Request",&request)?;let kind=if request.enroll.is_some(){"enroll"}else if request.recover.is_some(){"recover"}else if request.offline.is_some(){"offline"}else if request.pending_recover.is_some(){"pendingRecover"}else if request.pending_offline.is_some(){"pendingOffline"}else{return Err("Not a waiting/pending mutation".into())};
  if request.service_id.as_deref()!=Some(&scope.service)||request.service_epoch.as_deref()!=Some(&scope.service_epoch)||request.session_epoch.as_ref().is_none_or(String::is_empty){return Err("Waiting original scope differs".into())}
  let original=w::WaitingOriginal{account_id:Some(scope.account.clone()),device_id:Some(scope.device.to_string()),original_session_id:Some(session.into()),original_session_epoch:request.session_epoch.clone(),original_service_epoch:request.service_epoch.clone(),request_id:request.request_id.clone(),kind:Some(kind.into()),fingerprint:Some(crate::protocol_connection::canonical_fingerprint(&serde_json::to_vec(&request).map_err(|_|"Waiting request encoding")?)?),waiting_id:request.waiting_id.clone()};
  protocol::validate("WaitingOriginal",&original)?;Ok(Self{original,request})
 }
 pub fn permits(&self,scope:&Scope,session:&str,epoch:&str)->bool{self.original.account_id.as_deref()==Some(&scope.account)&&self.original.device_id.as_deref()==Some(&scope.device.to_string())&&self.original.original_service_epoch.as_deref()==Some(&scope.service_epoch)&&self.original.original_session_id.as_deref()==Some(session)&&self.original.original_session_epoch.as_deref()==Some(epoch)}
 pub fn validate(&self,scope:&Scope)->Result<(),String>{let rebuilt=Self::capture(scope,self.request.clone(),self.original.original_session_id.as_deref().ok_or("Missing original session")?)?;if serde_json::to_vec(&rebuilt.original).ok()!=serde_json::to_vec(&self.original).ok(){return Err("Waiting exact original changed".into())}Ok(())}
}
#[derive(Clone,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Journal{pub marker:WaitingMarker,pub intent:Option<w::WaitingIntent>,pub proof:Option<String>,pub generation:u32,pub current_attachment:Option<String>,pub operation:Option<Mutation>,pub terminal:bool,pub retirement:Option<Mutation>,pub retirement_done:bool}
impl Drop for Journal{fn drop(&mut self){if let Some(p)=self.proof.as_mut(){p.zeroize()}}}
impl Journal{
 pub fn new(marker:WaitingMarker)->Self{Self{marker,intent:None,proof:None,generation:0,current_attachment:None,operation:None,terminal:false,retirement:None,retirement_done:false}}
 pub fn ready(&self)->bool{!self.terminal&&self.intent.is_some()&&self.proof.as_ref().is_some_and(|p|p.len()==43)&&self.generation>0&&self.operation.is_none()}
 pub fn validate(&self,scope:&Scope)->Result<(),String>{
  self.marker.encode()?;if let Some(op)=&self.retirement{op.validate(scope)?;if !self.terminal||op.request.offline.is_none()&&op.request.pending_offline.is_none(){return Err("Invalid waiting retirement original".into())}}if let Some(op)=&self.operation{op.validate(scope)?}
  if let Some(p)=&self.proof{if p.len()!=43||!p.bytes().all(|v|v.is_ascii_alphanumeric()||v==b'-'||v==b'_'){return Err("Invalid opaque waiting proof".into())}}
  if let Some(i)=&self.intent{protocol::validate("WaitingIntent",i)?;if self.generation==0||self.current_attachment.as_ref().is_none_or(|s|s.is_empty()||s.len()>128)||i.account_id.as_deref()!=Some(&scope.account)||i.device_id.as_deref()!=Some(&scope.device.to_string())||i.service_id.as_deref()!=Some(&scope.service)||i.preparation_service_epoch.as_deref()!=Some(&scope.service_epoch)||i.original_desktop_id.as_deref()!=Some(&scope.desktop)||i.original_desktop_epoch.as_deref()!=Some(&scope.desktop_epoch)||i.room_id.as_deref()!=Some(&scope.room)||i.run_id.as_deref()!=Some(&scope.run)||i.run_epoch.as_deref()!=Some(&scope.run_epoch)||i.preparation_id.as_deref()!=Some(&scope.preparation)||i.config_revision.map(u64::from)!=Some(scope.config)||i.config_digest.as_deref()!=Some(&scope.config_digest)||i.actual_seed.as_deref()!=Some(&self.marker.actual_seed)||!(if self.marker.loot=="individual:0"{i.seed_policy.as_deref()==Some("individual-random")}else{matches!(i.seed_policy.as_deref(),Some("shared-fixed"|"shared-random"))})||i.randomize_loot!=Some(self.marker.loot.starts_with("shared:1:")){return Err("Canonical waiting intent differs from original local world".into())}}
  else if self.proof.is_some()||self.generation!=0{return Err("Waiting proof without intent".into())}Ok(())
 }
}
#[derive(Serialize,Deserialize)]#[serde(deny_unknown_fields)]struct Head{digest:String,len:usize,slot:u8}
fn key(s:&Scope,world:uuid::Uuid,part:&str)->Result<ServiceProfile,String>{let hash=Sha256::digest(serde_json::to_vec(&("nt2-canonical-waiting-v1",&s.account,s.device,world,part)).map_err(|_|"Waiting key")?);ServiceProfile::new(&s.origin,&s.service,uuid::Uuid::from_bytes(hash[..16].try_into().unwrap())).map_err(|_|"Waiting key scope".into())}
fn live<S:CredentialStore>(store:&S,s:&Scope,m:&WaitingMarker)->Result<(),String>{if startup_local::recall(store,&s.origin,&s.service,&s.account,s.device,m)?.is_none(){return Err("Original local waiting world absent or retired".into())}Ok(())}
/// Caller holds the shared serialized writer and fences owner before/after.
pub fn save<S:CredentialStore>(store:&S,s:&Scope,j:&Journal)->Result<(),String>{save_inner(store,s,j,false)}
pub fn save_retired<S:CredentialStore>(store:&S,s:&Scope,j:&Journal)->Result<(),String>{if !j.terminal{return Err("Retired writer cannot save an active world".into())}save_inner(store,s,j,true)}
fn check<S:CredentialStore>(store:&S,s:&Scope,m:&WaitingMarker,retired:bool)->Result<(),String>{if retired{let original=startup_local::recall_retired(store,&s.origin,&s.service,&s.account,s.device,m)?.ok_or("Original retirement missing")?;if original.original!=*s{return Err("Retirement scope changed".into())}Ok(())}else{live(store,s,m)}}
fn save_inner<S:CredentialStore>(store:&S,s:&Scope,j:&Journal,retired:bool)->Result<(),String>{
 check(store,s,&j.marker,retired)?;j.validate(s)?;let bytes=Zeroizing::new(serde_json::to_vec(j).map_err(|_|"Waiting journal encoding")?);if bytes.len()>MAX{return Err("Waiting journal exceeds split bound".into())}
 let digest=format!("{:x}",Sha256::digest(&*bytes));
 let previous=store.load(&key(s,j.marker.local_world,"head")?).map_err(|_|"Waiting commit read")?;
 let slot=match previous{Some(h)=>{let h:Head=serde_json::from_slice(h.bytes()).map_err(|_|"Invalid waiting commit")?;if h.slot>1{return Err("Waiting slot bound".into())}1-h.slot},None=>0};
 // Fixed two slots, at most 72 chunks + one head. Never overwrite the current
 // committed slot. Partial writes can only damage the uncommitted slot.
 for (n,part) in bytes.chunks(CHUNK).enumerate(){store.save(&key(s,j.marker.local_world,&format!("{slot}:{n}"))?,&SecretBlob::new(part.to_vec()).map_err(|_|"Waiting chunk bound")?).map_err(|_|"Waiting chunk storage")?;}
 check(store,s,&j.marker,retired)?;
 let head=serde_json::to_vec(&Head{digest,len:bytes.len(),slot}).map_err(|_|"Waiting commit encoding")?;
 store.save(&key(s,j.marker.local_world,"head")?,&SecretBlob::new(head).map_err(|_|"Waiting commit bound")?).map_err(|_|"Waiting commit storage")?;check(store,s,&j.marker,retired)
}
pub fn load<S:CredentialStore>(store:&S,s:&Scope,m:&WaitingMarker)->Result<Option<Journal>,String>{load_inner(store,s,m,false,false)}
/// Only a saved pending selector can follow a later local waiting journal.
/// Both protected originals must have identical scope/bundle/claim witness;
/// exact uncertain operations stay in the returned journal for reconciliation.
pub fn load_saved_selector<S:CredentialStore>(store:&S,s:&Scope,m:&WaitingMarker)->Result<Option<Journal>,String>{load_inner(store,s,m,false,true)}
pub fn load_retired<S:CredentialStore>(store:&S,s:&Scope,m:&WaitingMarker)->Result<Option<Journal>,String>{let j=load_inner(store,s,m,true,false)?;if j.as_ref().is_some_and(|j|!j.terminal){return Err("Retired journal has active state".into())}Ok(j)}
fn load_inner<S:CredentialStore>(store:&S,s:&Scope,m:&WaitingMarker,retired:bool,promotion:bool)->Result<Option<Journal>,String>{
 check(store,s,m,retired)?;let Some(head)=store.load(&key(s,m.local_world,"head")?).map_err(|_|"Waiting commit read")?else{return Ok(None)};let h:Head=serde_json::from_slice(head.bytes()).map_err(|_|"Invalid waiting commit")?;
 if h.slot>1||h.len==0||h.len>MAX||h.digest.len()!=64||!h.digest.bytes().all(|v|v.is_ascii_hexdigit()&&!v.is_ascii_uppercase()){return Err("Waiting commit bound".into())}
 let mut bytes=Zeroizing::new(Vec::with_capacity(h.len));for n in 0..h.len.div_ceil(CHUNK){let p=store.load(&key(s,m.local_world,&format!("{}:{n}",h.slot))?).map_err(|_|"Waiting chunk read")?.ok_or("Missing waiting chunk")?;let expected=CHUNK.min(h.len-n*CHUNK);if p.bytes().len()!=expected{return Err("Waiting chunk bound differs".into())}bytes.extend_from_slice(p.bytes());}
 if format!("{:x}",Sha256::digest(&*bytes))!=h.digest{return Err("Waiting commit digest differs".into())}
 let j:Journal=serde_json::from_slice(&bytes).map_err(|_|"Invalid waiting journal")?;if j.marker!=*m{
  let mut promoted=m.clone();promoted.kind="nt2-waiting".into();if !promotion||retired||m.kind!="nt2-pending"||j.marker!=promoted{return Err("Waiting selector differs".into())}
  let pending=startup_local::recall(store,&s.origin,&s.service,&s.account,s.device,m)?.ok_or("Pending original absent during promotion")?;let waiting=startup_local::recall(store,&s.origin,&s.service,&s.account,s.device,&promoted)?.ok_or("Waiting original absent during promotion")?;
  if pending.original!=*s||waiting.original!=*s||pending.claim_digest!=waiting.claim_digest{return Err("Pending/waiting protected original scope or claim differs".into())}
 }j.validate(s)?;Ok(Some(j))
}
fn index_scope(origin:&str,service:&str,account:&str,device:uuid::Uuid)->Scope{Scope{origin:origin.into(),service:service.into(),account:account.into(),device,service_epoch:String::new(),desktop:String::new(),desktop_epoch:String::new(),room:String::new(),run:String::new(),run_epoch:String::new(),config:0,config_digest:String::new(),preparation:String::new()}}
/// Bounded pending-retirement index. Only explicit local offline uses it.
pub fn queue_retirement<S:CredentialStore>(store:&S,s:&Scope,m:&WaitingMarker)->Result<(),String>{
 let journal=load_retired(store,s,m)?.ok_or("Retirement journal absent")?;if journal.retirement_done{return Ok(())}
 let profile=key(s,uuid::Uuid::nil(),"retirement-index")?;let mut ids:Vec<uuid::Uuid>=match store.load(&profile).map_err(|_|"Retirement index read")?{Some(b)=>serde_json::from_slice(b.bytes()).map_err(|_|"Retirement index corrupt")?,None=>vec![]};
 if ids.len()>32{return Err("Retirement index bound".into())}if !ids.contains(&m.local_world){if ids.len()==32{return Err("Pending retirement capacity reached".into())}ids.push(m.local_world)}
 store.save(&key(s,m.local_world,"retirement-marker")?,&SecretBlob::new(m.encode()?).map_err(|_|"Retirement selector bound")?).map_err(|_|"Retirement selector write")?;
 store.save(&profile,&SecretBlob::new(serde_json::to_vec(&ids).map_err(|_|"Retirement index encoding")?).map_err(|_|"Retirement index bound")?).map_err(|_|"Retirement index write".into())
}
pub fn retirement_markers<S:CredentialStore>(store:&S,origin:&str,service:&str,account:&str,device:uuid::Uuid)->Result<Vec<WaitingMarker>,String>{
 let s=index_scope(origin,service,account,device);let Some(head)=store.load(&key(&s,uuid::Uuid::nil(),"retirement-index")?).map_err(|_|"Retirement index read")?else{return Ok(vec![])};let ids:Vec<uuid::Uuid>=serde_json::from_slice(head.bytes()).map_err(|_|"Retirement index corrupt")?;if ids.len()>32{return Err("Retirement index bound".into())}let mut out=vec![];
 for id in ids{let b=store.load(&key(&s,id,"retirement-marker")?).map_err(|_|"Retirement selector read")?.ok_or("Retirement selector absent")?;let marker=WaitingMarker::decode(b.bytes())?;if marker.local_world!=id{return Err("Retirement selector differs".into())}out.push(marker)}Ok(out)
}
pub fn finish_retirement<S:CredentialStore>(store:&S,s:&Scope,m:&WaitingMarker)->Result<(),String>{if !load_retired(store,s,m)?.is_some_and(|j|j.retirement_done){return Err("Retirement effect is not confirmed".into())}let p=key(s,uuid::Uuid::nil(),"retirement-index")?;let Some(b)=store.load(&p).map_err(|_|"Retirement index read")?else{return Ok(())};let mut ids:Vec<uuid::Uuid>=serde_json::from_slice(b.bytes()).map_err(|_|"Retirement index corrupt")?;if ids.len()>32{return Err("Retirement index bound".into())}ids.retain(|id|*id!=m.local_world);store.save(&p,&SecretBlob::new(serde_json::to_vec(&ids).map_err(|_|"Retirement index encoding")?).map_err(|_|"Retirement index bound")?).map_err(|_|"Retirement index write".into())}


#[cfg(test)]#[path="waiting_local_test.rs"]mod tests;
