//! Client-local preparation and waiting intent. No wire ABI or play authority.
//! A waiting marker is only a selector for protected original local evidence.
use crate::secrets::{CredentialStore,SecretBlob,ServiceProfile};
use serde::{Deserialize,Serialize};
use sha2::{Digest,Sha256};
use std::time::Instant;
#[derive(Clone,Debug,PartialEq,Eq,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scope {
 pub origin:String,pub service:String,pub service_epoch:String,pub account:String,
 pub device:uuid::Uuid,pub desktop:String,pub desktop_epoch:String,pub room:String,
 pub run:String,pub run_epoch:String,pub config:u64,pub config_digest:String,pub preparation:String,
}
impl Scope {
 pub(crate) fn validate(&self)->Result<(),String>{
  ServiceProfile::new(&self.origin,&self.service,self.device).map_err(|_|"Invalid startup issuer")?;
  if self.device.is_nil()||self.config==0||[&self.service_epoch,&self.account,&self.desktop,&self.desktop_epoch,&self.room,&self.run,&self.run_epoch,&self.preparation].iter().any(|v|v.is_empty()||v.len()>128||!v.bytes().all(|b|b.is_ascii_alphanumeric()||matches!(b,b'-'|b'_')))||self.config_digest.len()!=64||!self.config_digest.bytes().all(|b|b.is_ascii_hexdigit()&&!b.is_ascii_uppercase()){return Err("Invalid original startup scope".into())}Ok(())
 }
}
#[derive(Clone,Debug)]
pub struct Prepared {
 scope:Scope,world:uuid::Uuid,seed:u32,policy:u32,generation:u64,digest:String,until:Instant,frozen_config:Option<serde_json::Value>,
}
fn digest(scope:&Scope,world:uuid::Uuid,seed:u32,policy:u32,generation:u64)->Result<String,String>{
 Ok(format!("{:x}",Sha256::digest(serde_json::to_vec(&(1u8,scope,world,seed,policy,generation)).map_err(|_|"Startup encoding")?)))
}
impl Prepared {
 pub fn lifecycle(scope:Scope,world:uuid::Uuid,seed:u32,policy:u32,generation:u64,until:Instant,config:serde_json::Value)->Result<Self,String>{let mut b=Self::stage(scope,seed,policy,generation,until)?;if world.is_nil()||crate::protocol_connection::canonical_fingerprint(&serde_json::to_vec(&config).map_err(|_|"LC1 config encoding")?)?!=b.scope.config_digest{return Err("LC1 frozen configuration differs".into())}b.world=world;b.digest=digest(&b.scope,world,seed,policy,generation)?;b.frozen_config=Some(config);Ok(b)}
 pub fn stage(scope:Scope,seed:u32,policy:u32,generation:u64,until:Instant)->Result<Self,String>{
  scope.validate()?;if generation==0||generation>u64::from(u32::MAX)||!matches!(policy,1|2|5)||policy==2&&seed!=0||until<=Instant::now(){return Err("Invalid staged seed/loot preparation".into())}
  let world=uuid::Uuid::new_v4();let digest=digest(&scope,world,seed,policy,generation)?;
  Ok(Self{scope,world,seed,policy,generation,digest,until,frozen_config:None})
 }
 pub fn stage_durable<S:CredentialStore>(store:&S,scope:Scope,seed:u32,policy:u32,generation:u64,until:Instant,config:serde_json::Value)->Result<Self,String>{
  let encoded=zeroize::Zeroizing::new(serde_json::to_vec(&config).map_err(|_|"Startup configuration encoding")?);if encoded.len()>8192||crate::protocol_connection::canonical_fingerprint(&encoded)?!=scope.config_digest{return Err("Startup frozen configuration differs".into())}
  let mut prepared=Self::stage(scope,seed,policy,generation,until)?;
  #[derive(Serialize,Deserialize)]#[serde(deny_unknown_fields)]struct Identity{scope:Scope,world:uuid::Uuid,seed:u32,policy:u32}
  let hash=Sha256::digest(serde_json::to_vec(&("nt2-startup-pending-v1",&prepared.scope.account,prepared.scope.device,&prepared.scope.preparation)).map_err(|_|"Startup pending key")?);
  let target=ServiceProfile::new(&prepared.scope.origin,&prepared.scope.service,uuid::Uuid::from_bytes(hash[..16].try_into().unwrap())).map_err(|_|"Startup pending issuer")?;
  if let Some(old)=store.load(&target).map_err(|_|"Startup pending read unavailable")?{let old:Identity=serde_json::from_slice(old.bytes()).map_err(|_|"Startup pending original corrupt")?;if old.scope!=prepared.scope||old.seed!=seed||old.policy!=policy||old.world.is_nil(){return Err("Startup pending original differs; retain for recovery".into())}prepared.world=old.world;prepared.digest=digest(&prepared.scope,old.world,seed,policy,generation)?;}
  let identity=Identity{scope:prepared.scope.clone(),world:prepared.world,seed,policy};store.save(&target,&SecretBlob::new(serde_json::to_vec(&identity).map_err(|_|"Startup pending identity encoding")?).map_err(|_|"Startup pending identity bound")?).map_err(|_|"Startup pending identity persistence unavailable")?;
  prepared.frozen_config=Some(config);Ok(prepared)
 }
 pub fn snapshot(&self)->Result<Vec<u8>,String>{let config=self.frozen_config.as_ref().ok_or("Complete frozen startup configuration unavailable")?;serde_json::to_vec(&serde_json::json!({"version":1,"scope":self.scope,"local_world":self.world,"seed":self.seed,"policy":self.policy,"generation":self.generation,"bundle_digest":self.digest,"config":config})).map_err(|_|"Startup snapshot encoding".into())}
 pub fn until(&self)->Instant{self.until}
 pub fn world(&self)->uuid::Uuid{self.world}
 pub fn with_deadline(mut self,until:Instant)->Self{self.until=self.until.min(until);self}
 pub fn scope(&self)->&Scope{&self.scope}
 pub fn seed(&self)->u32{self.seed}
 pub fn policy(&self)->u32{self.policy}
 pub fn generation(&self)->u64{self.generation}
 pub fn digest(&self)->&str{&self.digest}
 pub fn live(&self,generation:u64)->bool{self.generation==generation&&Instant::now()<self.until}
 pub fn matches(&self,run:&str,epoch:&str,config:u64,preparation:Option<&str>,seed:u32,policy:u32)->bool{
  self.scope.run==run&&self.scope.run_epoch==epoch&&self.scope.config==config&&preparation==Some(self.scope.preparation.as_str())&&self.seed==seed&&self.policy==policy
 }
 pub fn marker(&self,actual:u64)->Result<WaitingMarker,String>{
  if self.policy!=2&&actual!=u64::from(self.seed){return Err("Wrong seed: shared play requires a fresh New Game".into())}
  Ok(WaitingMarker{kind:"nt2-waiting".into(),version:1,local_world:self.world,bundle_digest:self.digest.clone(),actual_seed:actual.to_string(),loot:if self.policy==2{"individual:0".into()}else{format!("shared:{}:{actual}",u8::from(self.policy==5))}})
 }
 pub fn pending_marker(&self,actual:u64)->Result<WaitingMarker,String>{let mut m=self.marker(actual)?;m.kind="nt2-pending".into();Ok(m)}
}
#[derive(Clone,Debug,PartialEq,Eq,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WaitingMarker {pub kind:String,pub version:u8,pub local_world:uuid::Uuid,pub bundle_digest:String,pub actual_seed:String,pub loot:String}
impl WaitingMarker {
 fn validate(&self)->Result<(),String>{let actual=self.actual_seed.parse::<u64>().map_err(|_|"Invalid waiting seed")?;if !matches!(self.kind.as_str(),"nt2-waiting"|"nt2-pending"|"nt2-provisional")||self.version!=1||self.local_world.is_nil()||actual.to_string()!=self.actual_seed||self.bundle_digest.len()!=64||!self.bundle_digest.bytes().all(|b|b.is_ascii_hexdigit()&&!b.is_ascii_uppercase())||!(self.loot=="individual:0"||self.loot==format!("shared:0:{actual}")||self.loot==format!("shared:1:{actual}")){return Err("Invalid waiting marker".into())}Ok(())}
 pub fn encode(&self)->Result<Vec<u8>,String>{self.validate()?;let bytes=serde_json::to_vec(self).map_err(|_|"Waiting marker encoding")?;if bytes.len()>1024{return Err("Waiting marker bound".into())}Ok(bytes)}
 pub fn decode(bytes:&[u8])->Result<Self,String>{if bytes.len()>1024{return Err("Waiting marker bound".into())}let marker:Self=serde_json::from_slice(bytes).map_err(|_|"Malformed waiting marker")?;marker.validate()?;Ok(marker)}
}
#[derive(Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
struct Record {version:u8,scope:Scope,seed:u32,policy:u32,generation:u64,marker:WaitingMarker,owner_pid:crate::local_owner::Id,owner_context:[u8;16],claim_digest:String,attach:Option<AttachEvidence>}
#[derive(Clone,Debug,PartialEq,Eq,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AttachEvidence {pub request_id:String,pub fingerprint:String,pub session_id:String,pub session_epoch:String,pub service_epoch:String,pub preparation:String,pub attachment:String,pub instance:String}
impl AttachEvidence {
 fn matches(&self,scope:&Scope)->bool{self.session_id==scope.desktop&&self.session_epoch==scope.desktop_epoch&&self.service_epoch==scope.service_epoch&&self.preparation==scope.preparation&&!self.attachment.is_empty()&&!self.instance.is_empty()&&!self.request_id.is_empty()&&self.fingerprint.len()==64&&self.fingerprint.bytes().all(|b|b.is_ascii_hexdigit()&&!b.is_ascii_uppercase())}
}
#[derive(Clone,Debug,PartialEq,Eq)]
pub enum Disposition {AwaitCanonicalWaitingRecovery}
pub struct Parked {pub original:Scope,pub marker:WaitingMarker,pub original_attach:Option<AttachEvidence>,pub claim_digest:String,pub disposition:Disposition}
#[derive(Serialize,Deserialize)]#[serde(deny_unknown_fields)]struct ProvisionalLink{version:u8,scope:Scope,marker:WaitingMarker,source_binding:String}
/// Materialize a selector only AFTER the explicit entry journal validated a
/// real current canonical PendingIntent. No fabricated reserve/gameAttach.
pub fn promote_provisional<S:CredentialStore>(store:&S,record:&crate::provisional_local::Record)->Result<WaitingMarker,String>{
 let canonical=record.canonical().ok_or("Provisional B has no confirmed canonical pending")?;
 if canonical.marker.local_world!=record.world()||canonical.marker.bundle_digest!=record.digest(){return Err("Provisional canonical selector differs".into())}
 let link=ProvisionalLink{version:2,scope:canonical.scope.clone(),marker:canonical.marker.clone(),source_binding:record.digest().into()};
 let bytes=serde_json::to_vec(&link).map_err(|_|"Provisional canonical link encoding")?;let target=profile(&canonical.scope,&canonical.marker)?;
 if let Some(old)=store.load(&target).map_err(|_|"Provisional canonical link read")?{if old.bytes()!=bytes{return Err("Provisional canonical original conflict".into())}}
 store.save(&target,&SecretBlob::new(bytes).map_err(|_|"Provisional canonical link bound")?).map_err(|_|"Provisional canonical link write")?;
 index_loaded(store,&canonical.scope,&canonical.marker)?;Ok(canonical.marker.clone())
}
fn profile(scope:&Scope,marker:&WaitingMarker)->Result<ServiceProfile,String>{
 keyed_profile(scope,marker.local_world,if marker.kind=="nt2-pending"{"nt2-pending-world-v1"}else{"nt2-waiting-intent-v1"})
}
fn retired_profile(scope:&Scope,world:uuid::Uuid)->Result<ServiceProfile,String>{keyed_profile(scope,world,"nt2-waiting-retired-v1")}
fn keyed_profile(scope:&Scope,world:uuid::Uuid,purpose:&str)->Result<ServiceProfile,String>{
 let key=Sha256::digest(serde_json::to_vec(&(purpose,&scope.account,scope.device,world)).map_err(|_|"Waiting key")?);
 ServiceProfile::new(&scope.origin,&scope.service,uuid::Uuid::from_bytes(key[..16].try_into().unwrap())).map_err(|_|"Waiting issuer".into())
}
#[derive(Clone,Serialize,Deserialize)]#[serde(deny_unknown_fields)]
struct RecoverySelector{scope:Scope,marker:WaitingMarker}
pub struct SelectorSnapshot{origin:String,service:String,account:String,device:uuid::Uuid,room:String,entries:Vec<RecoverySelector>}
pub fn selector_snapshot<S:CredentialStore>(store:&S,origin:&str,service:&str,account:&str,device:uuid::Uuid,room:&str)->Result<SelectorSnapshot,String>{
 let scope=Scope{origin:origin.into(),service:service.into(),account:account.into(),device,service_epoch:String::new(),desktop:String::new(),desktop_epoch:String::new(),room:String::new(),run:String::new(),run_epoch:String::new(),config:0,config_digest:String::new(),preparation:String::new()};
 Ok(SelectorSnapshot{origin:origin.into(),service:service.into(),account:account.into(),device,room:room.into(),entries:recovery_index(store,&scope)?.into_iter().filter(|e|e.scope.room==room).collect()})
}
/// Caller has verified the authenticated coherent roomPreview and serializes
/// writers. Only this pre-read snapshot is archived; originals/proofs remain.
pub fn reconcile_completed_runs<S:CredentialStore>(store:&S,snapshot:&SelectorSnapshot,run:&str,run_epoch:&str,request:&str,fence:&std::sync::atomic::AtomicU64,expected:u64)->Result<usize,String>{
 use std::sync::atomic::Ordering;
 if uuid::Uuid::parse_str(run).is_err()||uuid::Uuid::parse_str(run_epoch).is_err()||request.is_empty()||request.len()>128||fence.load(Ordering::Acquire)!=expected{return Err("Completed-run preview is stale/incomplete".into())}
 let scope=Scope{origin:snapshot.origin.clone(),service:snapshot.service.clone(),account:snapshot.account.clone(),device:snapshot.device,service_epoch:String::new(),desktop:String::new(),desktop_epoch:String::new(),room:String::new(),run:String::new(),run_epoch:String::new(),config:0,config_digest:String::new(),preparation:String::new()};
 let before=recovery_index(store,&scope)?;let mut retained=vec![];let mut archived=0;
 for entry in &before{
  let exact=snapshot.entries.iter().any(|old|old.scope==entry.scope&&old.marker==entry.marker);
  if exact&&entry.scope.room==snapshot.room&&entry.scope.run!=run&&entry.scope.run_epoch!=run_epoch{
   let bytes=zeroize::Zeroizing::new(serde_json::to_vec(&serde_json::json!({"version":1,"selector":entry,"current_run":run,"current_run_epoch":run_epoch,"read_request":request})).map_err(|_|"Completed-run index receipt")?);
   crate::protected_chunks::save(store,&entry.scope,"nt2-startup-completed-run-selector-v1",entry.marker.local_world,&bytes)?;archived+=1;
  }else{retained.push(entry.clone())}
 }
 if fence.load(Ordering::Acquire)!=expected{return Err("Completed-run preview retired before index commit".into())}
 if archived>0{
  let bytes=zeroize::Zeroizing::new(serde_json::to_vec(&retained).map_err(|_|"Completed-run index encoding")?);crate::protected_chunks::save(store,&scope,"nt2-startup-recovery-index-v1",uuid::Uuid::nil(),&bytes)?;
  if fence.load(Ordering::Acquire)!=expected{
   // Same serialized writer: restore the pre-write active index on a late fence.
   let bytes=zeroize::Zeroizing::new(serde_json::to_vec(&before).map_err(|_|"Completed-run rollback encoding")?);crate::protected_chunks::save(store,&scope,"nt2-startup-recovery-index-v1",uuid::Uuid::nil(),&bytes)?;return Err("Completed-run index publication retired".into())
  }
 }Ok(archived)
}
fn recovery_index<S:CredentialStore>(store:&S,scope:&Scope)->Result<Vec<RecoverySelector>,String>{
 let Some(blob)=crate::protected_chunks::load(store,scope,"nt2-startup-recovery-index-v1",uuid::Uuid::nil())?else{return Ok(vec![])};
 let entries:Vec<RecoverySelector>=serde_json::from_slice(&blob).map_err(|_|"Recovery selector index corrupt")?;
 if entries.len()>16{return Err("Recovery selector index capacity".into())}
 for entry in &entries{entry.scope.validate()?;entry.marker.validate()?;if entry.scope.origin!=scope.origin||entry.scope.service!=scope.service||entry.scope.account!=scope.account||entry.scope.device!=scope.device{return Err("Recovery selector issuer differs".into())}}
 Ok(entries)
}
fn index_loaded<S:CredentialStore>(store:&S,scope:&Scope,marker:&WaitingMarker)->Result<(),String>{
 let mut entries=Vec::new();
 for entry in recovery_index(store,scope)?{
  let retired=store.load(&retired_profile(&entry.scope,entry.marker.local_world)?).map_err(|_|"Recovery retirement read unavailable")?;
  if let Some(retired)=retired{if retired.bytes()!=[1]{return Err("Recovery retirement corrupt".into())}
   if crate::waiting_local::load_retired(store,&entry.scope,&entry.marker)?.is_some_and(|j|j.retirement_done){continue}
  }
  entries.push(entry);
 }
 if let Some(entry)=entries.iter_mut().find(|e|e.marker.local_world==marker.local_world){
  if entry.scope!=*scope||entry.marker.bundle_digest!=marker.bundle_digest||entry.marker.actual_seed!=marker.actual_seed||entry.marker.loot!=marker.loot{return Err("Recovery selector original differs".into())}
  // Pending -> waiting promotion keeps the exact identity; never demote it.
  if entry.marker.kind=="nt2-pending"{entry.marker=marker.clone();}
 }else{if entries.len()>=16{return Err("Resolve existing saved worlds before another loaded selector".into())}entries.push(RecoverySelector{scope:scope.clone(),marker:marker.clone()});}
 let bytes=zeroize::Zeroizing::new(serde_json::to_vec(&entries).map_err(|_|"Recovery index encoding")?);
 crate::protected_chunks::save(store,scope,"nt2-startup-recovery-index-v1",uuid::Uuid::nil(),&bytes)
}
/// Read-only, current account/device selector. Not authority/readiness/seed input.
/// A cold app must not issue roomJoin before a known loaded original is resolved.
pub fn has_recoverable_room<S:CredentialStore>(store:&S,origin:&str,service:&str,account:&str,device:uuid::Uuid,room:&str)->Result<bool,String>{
 let scope=Scope{origin:origin.into(),service:service.into(),account:account.into(),device,service_epoch:String::new(),desktop:String::new(),desktop_epoch:String::new(),room:String::new(),run:String::new(),run_epoch:String::new(),config:0,config_digest:String::new(),preparation:String::new()};
 for entry in recovery_index(store,&scope)?.into_iter().filter(|e|e.scope.room==room){
  if recall(store,origin,service,account,device,&entry.marker)?.is_some(){return Ok(true)}
  // Local opt-out is not confirmation that the original canonical effect retired.
  if recall_retired(store,origin,service,account,device,&entry.marker)?.is_some()
    && !crate::waiting_local::load_retired(store,&entry.scope,&entry.marker)?.is_some_and(|j|j.retirement_done){return Ok(true)}
 }
 Ok(false)
}
/// Caller must fence owner before/after this blocking protected write, using the
/// existing serialized credential writer. No refresh/resume proof in the marker.
pub fn persist<S:CredentialStore>(store:&S,bundle:&Prepared,actual:u64,pid:crate::local_owner::Id,context:[u8;16],claim:&[u8])->Result<WaitingMarker,String>{
 persist_with_attach(store,bundle,actual,pid,context,claim,None)
}
pub fn persist_with_attach<S:CredentialStore>(store:&S,bundle:&Prepared,actual:u64,pid:crate::local_owner::Id,context:[u8;16],claim:&[u8],attach:Option<&AttachEvidence>)->Result<WaitingMarker,String>{
 persist_kind(store,bundle,actual,pid,context,claim,attach,false)
}
pub fn persist_pending<S:CredentialStore>(store:&S,bundle:&Prepared,actual:u64,pid:crate::local_owner::Id,context:[u8;16],claim:&[u8])->Result<WaitingMarker,String>{
 let reservation=crate::startup_pending::load(store,bundle.scope(),bundle.world())?.ok_or("Pending reservation absent")?;if !reservation.ready(){return Err("Pending reservation proof not sealed".into())}
 persist_kind(store,bundle,actual,pid,context,claim,None,true)
}
fn persist_kind<S:CredentialStore>(store:&S,bundle:&Prepared,actual:u64,pid:crate::local_owner::Id,context:[u8;16],claim:&[u8],attach:Option<&AttachEvidence>,pending:bool)->Result<WaitingMarker,String>{
 if store.load(&retired_profile(&bundle.scope,bundle.world)?).map_err(|e|e.to_string())?.is_some(){return Err("Waiting world explicitly offline".into())}
 if pid==0||context==[0;16]||claim!=pid.witness(context){return Err("Original durable consumption witness differs".into())}
 if attach.is_some_and(|a|!a.matches(&bundle.scope)){return Err("Original attach identity differs from staged scope".into())}
 let marker=if pending{bundle.pending_marker(actual)?}else{bundle.marker(actual)?};
 let record=Record{version:1,scope:bundle.scope.clone(),seed:bundle.seed,policy:bundle.policy,generation:bundle.generation,marker:marker.clone(),owner_pid:pid,owner_context:context,claim_digest:format!("{:x}",Sha256::digest(claim)),attach:attach.cloned()};
 let bytes=zeroize::Zeroizing::new(serde_json::to_vec(&record).map_err(|_|"Waiting record encoding")?);
 let blob=SecretBlob::new(bytes.to_vec()).map_err(|_|"Waiting protected record bound")?;
 let target=profile(&bundle.scope,&marker)?;
 if let Some(old)=store.load(&target).map_err(|e|e.to_string())?{if old.bytes()!=blob.bytes(){return Err("Waiting record conflict; original retained".into())}}
 store.save(&target,&blob).map_err(|e|e.to_string())?;
 index_loaded(store,&bundle.scope,&marker)?;
 if store.load(&retired_profile(&bundle.scope,bundle.world)?).map_err(|e|e.to_string())?.is_some(){return Err("Waiting world retired during persistence".into())}Ok(marker)
}
/// A current authenticated issuer/account/device may park the exact original.
/// Session replacement is allowed; generation/authority is never restored here.
pub fn recall<S:CredentialStore>(store:&S,origin:&str,service:&str,account:&str,device:uuid::Uuid,marker:&WaitingMarker)->Result<Option<Parked>,String>{recall_inner(store,origin,service,account,device,marker,false)}
/// Exact original lookup for terminal queue processing only; never online recovery.
pub fn recall_retired<S:CredentialStore>(store:&S,origin:&str,service:&str,account:&str,device:uuid::Uuid,marker:&WaitingMarker)->Result<Option<Parked>,String>{recall_inner(store,origin,service,account,device,marker,true)}
fn recall_inner<S:CredentialStore>(store:&S,origin:&str,service:&str,account:&str,device:uuid::Uuid,marker:&WaitingMarker,retirement:bool)->Result<Option<Parked>,String>{
 marker.validate()?;
 if marker.kind=="nt2-provisional"{return Err("Provisional selector requires its explicit entry journal consumer".into())}
 let lookup=Scope{origin:origin.into(),service:service.into(),account:account.into(),device,service_epoch:String::new(),desktop:String::new(),desktop_epoch:String::new(),room:String::new(),run:String::new(),run_epoch:String::new(),config:0,config_digest:String::new(),preparation:String::new()};
 if let Some(tombstone)=store.load(&retired_profile(&lookup,marker.local_world)?).map_err(|e|e.to_string())?{if tombstone.bytes()!=[1]{return Err("Waiting retirement record corrupt".into())}if !retirement{return Ok(None)}}else if retirement{return Err("Waiting world is not locally retired".into())}
 let Some(blob)=store.load(&profile(&lookup,marker)?).map_err(|e|e.to_string())? else{return Ok(None)};
 if serde_json::from_slice::<serde_json::Value>(blob.bytes()).map_err(|_|"Waiting original encoding")?["version"]==2{
  let link:ProvisionalLink=serde_json::from_slice(blob.bytes()).map_err(|_|"Provisional canonical link corrupt")?;link.scope.validate()?;
  let record=crate::provisional_local::load(store,&lookup,marker.local_world)?.ok_or("Provisional source original absent")?;let canonical=record.canonical().ok_or("Provisional canonical receipt absent")?;
  if link.version!=2||link.marker!=*marker||canonical.marker!=*marker||canonical.scope!=link.scope||link.source_binding!=record.digest()||link.scope.origin!=origin||link.scope.service!=service||link.scope.account!=account||link.scope.device!=device{return Err("Provisional canonical selector original differs".into())}
  return Ok(Some(Parked{original:link.scope,marker:link.marker,original_attach:None,claim_digest:link.source_binding,disposition:Disposition::AwaitCanonicalWaitingRecovery}))
 }
 let r:Record=serde_json::from_slice(blob.bytes()).map_err(|_|"Waiting protected record corrupt")?;r.scope.validate()?;
 let claim=r.owner_pid.witness(r.owner_context);
 let actual=marker.actual_seed.parse::<u64>().map_err(|_|"Invalid waiting seed")?;let loot=if r.policy==2{"individual:0".into()}else{format!("shared:{}:{actual}",u8::from(r.policy==5))};
 if r.version!=1||r.marker!=*marker||r.scope.origin!=origin||r.scope.service!=service||r.scope.account!=account||r.scope.device!=device||r.owner_pid==0||r.owner_context==[0;16]||r.generation==0||!matches!(r.policy,1|2|5)||r.policy==2&&r.seed!=0||r.policy!=2&&actual!=u64::from(r.seed)||loot!=marker.loot||r.claim_digest!=format!("{:x}",Sha256::digest(&claim))||r.marker.bundle_digest!=digest(&r.scope,r.marker.local_world,r.seed,r.policy,r.generation)?{return Err("Waiting original scope/witness differs".into())}
 if r.attach.as_ref().is_some_and(|a|!a.matches(&r.scope)){return Err("Stored original attach differs".into())}
 Ok(Some(Parked{original:r.scope,marker:r.marker,original_attach:r.attach,claim_digest:r.claim_digest,disposition:Disposition::AwaitCanonicalWaitingRecovery}))
}
/// The existing lifecycle owner must retire current native authority BEFORE
/// this serialized write. A separate key prevents a late active save undoing it.
pub fn retire<S:CredentialStore>(store:&S,original:&Scope,marker:&WaitingMarker)->Result<(),String>{
 marker.validate()?;original.validate()?;
 store.save(&retired_profile(original,marker.local_world)?,&SecretBlob::new(vec![1]).map_err(|_|"Waiting retirement encoding")?).map_err(|e|e.to_string())
}
#[cfg(test)]mod tests{
 use super::*;
 #[derive(Default)]struct Memory(std::sync::Mutex<std::collections::BTreeMap<(String,String,uuid::Uuid),Vec<u8>>>);
 impl CredentialStore for Memory{
  fn save(&self,p:&ServiceProfile,b:&SecretBlob)->Result<(),crate::secrets::StoreError>{self.0.lock().unwrap().insert((p.origin().into(),p.service_id().into(),p.profile_id()),b.bytes().to_vec());Ok(())}
  fn load(&self,p:&ServiceProfile)->Result<Option<SecretBlob>,crate::secrets::StoreError>{self.0.lock().unwrap().get(&(p.origin().into(),p.service_id().into(),p.profile_id())).map(|b|SecretBlob::new(b.clone())).transpose()}
  fn delete(&self,p:&ServiceProfile)->Result<(),crate::secrets::StoreError>{self.0.lock().unwrap().remove(&(p.origin().into(),p.service_id().into(),p.profile_id()));Ok(())}
 }
 fn bundle(policy:u32)->Prepared{Prepared::stage(Scope{origin:"https://waiting.invalid".into(),service:"service".into(),service_epoch:"epoch".into(),account:"account".into(),device:uuid::Uuid::new_v4(),desktop:"desktop".into(),desktop_epoch:"session".into(),room:"room".into(),run:"run".into(),run_epoch:"run-epoch".into(),config:1,config_digest:"a".repeat(64),preparation:"preparation".into()},if policy==2{0}else{424242},policy,7,Instant::now()+std::time::Duration::from_secs(60)).unwrap()}
 fn claim(pid:crate::local_owner::Id,c:[u8;16])->Vec<u8>{pid.witness(c)}
 #[test]fn se1_completed_run_selectors_do_not_exhaust_index_or_delete_originals(){
  use std::sync::atomic::AtomicU64;
  let base=bundle(5);let store=Memory::default();let mut scope=base.scope.clone();let mut originals=vec![];let fence=AtomicU64::new(7);
  for _ in 0..24{
   scope.run=uuid::Uuid::new_v4().to_string();scope.run_epoch=uuid::Uuid::new_v4().to_string();scope.preparation=uuid::Uuid::new_v4().to_string();
   let snapshot=selector_snapshot(&store,&scope.origin,&scope.service,&scope.account,scope.device,&scope.room).unwrap();
   reconcile_completed_runs(&store,&snapshot,&scope.run,&scope.run_epoch,"current-preview",&fence,7).unwrap();
   let prepared=Prepared::stage(scope.clone(),424242,5,7,Instant::now()+std::time::Duration::from_secs(60)).unwrap();
   originals.push(persist(&store,&prepared,424242,nt2_local_ws::owner::Id::OsProcess(9),[1;16],&claim(nt2_local_ws::owner::Id::OsProcess(9),[1;16])).unwrap());
   assert_eq!(recovery_index(&store,&scope).unwrap().len(),1);
  }
  for marker in originals{assert!(recall(&store,&scope.origin,&scope.service,&scope.account,scope.device,&marker).unwrap().is_some());}
 }
 #[test]fn se1_selector_reconciliation_fences_old_read_and_preserves_concurrent_and_same_run(){
  use std::sync::atomic::{AtomicU64,Ordering};
  let mut scope=bundle(5).scope;scope.run=uuid::Uuid::new_v4().to_string();scope.run_epoch=uuid::Uuid::new_v4().to_string();let base=Prepared::stage(scope,424242,5,7,Instant::now()+std::time::Duration::from_secs(60)).unwrap();let store=Memory::default();let old=persist(&store,&base,424242,nt2_local_ws::owner::Id::OsProcess(9),[1;16],&claim(nt2_local_ws::owner::Id::OsProcess(9),[1;16])).unwrap();
  let scope=&base.scope;let snapshot=selector_snapshot(&store,&scope.origin,&scope.service,&scope.account,scope.device,&scope.room).unwrap();let fence=AtomicU64::new(7);
  let run=uuid::Uuid::new_v4().to_string();let epoch=uuid::Uuid::new_v4().to_string();
  // Changed service/run epoch alone cannot retire a same-run selector.
  assert_eq!(reconcile_completed_runs(&store,&snapshot,&scope.run,&epoch,"epoch-only",&fence,7).unwrap(),0);
  fence.store(8,Ordering::Release);assert!(reconcile_completed_runs(&store,&snapshot,&run,&epoch,"old-preview",&fence,7).is_err());assert_eq!(recovery_index(&store,scope).unwrap().len(),1);fence.store(7,Ordering::Release);
  let mut newer=scope.clone();newer.run=run.clone();newer.run_epoch=epoch.clone();newer.preparation=uuid::Uuid::new_v4().to_string();
  let fresh=Prepared::stage(newer,424242,5,7,Instant::now()+std::time::Duration::from_secs(60)).unwrap();let marker=persist(&store,&fresh,424242,nt2_local_ws::owner::Id::OsProcess(9),[1;16],&claim(nt2_local_ws::owner::Id::OsProcess(9),[1;16])).unwrap();
  assert_eq!(reconcile_completed_runs(&store,&snapshot,&run,&epoch,"current-preview",&fence,7).unwrap(),1);
  let current=recovery_index(&store,scope).unwrap();assert_eq!(current.len(),1);assert_eq!(current[0].marker,marker);assert!(recall(&store,&scope.origin,&scope.service,&scope.account,scope.device,&old).unwrap().is_some());
  assert_eq!(reconcile_completed_runs(&store,&snapshot,&run,&epoch,"duplicate-preview",&fence,7).unwrap(),0);
 }
 #[test]fn se1_selector_index_uses_atomic_chunks_and_bounds_live_originals(){
  let base=bundle(5);let store=Memory::default();
  for i in 0..16{let mut scope=base.scope.clone();scope.room=format!("room-{i}");scope.preparation=format!("prep-{i}");let b=Prepared::stage(scope,424242,5,7,Instant::now()+std::time::Duration::from_secs(60)).unwrap();persist(&store,&b,424242,nt2_local_ws::owner::Id::OsProcess(9),[1;16],&claim(nt2_local_ws::owner::Id::OsProcess(9),[1;16])).unwrap();}
  let entries=recovery_index(&store,&base.scope).unwrap();assert_eq!(entries.len(),16);assert!(serde_json::to_vec(&entries).unwrap().len()>crate::secrets::MAX_SECRET_BYTES);
  assert!(has_recoverable_room(&store,&base.scope.origin,&base.scope.service,&base.scope.account,base.scope.device,"room-15").unwrap());
  let extra=Prepared::stage(base.scope.clone(),424242,5,7,Instant::now()+std::time::Duration::from_secs(60)).unwrap();assert!(persist(&store,&extra,424242,nt2_local_ws::owner::Id::OsProcess(9),[1;16],&claim(nt2_local_ws::owner::Id::OsProcess(9),[1;16])).is_err());
  assert_eq!(recovery_index(&store,&base.scope).unwrap().len(),16);
 }
 #[test]fn se1_loaded_selector_survives_session_without_preparing_and_retirement_requires_ack(){
  let b=bundle(5);let store=Memory::default();let scope=b.scope();
  assert!(!has_recoverable_room(&store,&scope.origin,&scope.service,&scope.account,scope.device,&scope.room).unwrap());
  let marker=persist(&store,&b,424242,nt2_local_ws::owner::Id::OsProcess(9),[1;16],&claim(nt2_local_ws::owner::Id::OsProcess(9),[1;16])).unwrap();
  assert!(has_recoverable_room(&store,&scope.origin,&scope.service,&scope.account,scope.device,&scope.room).unwrap());
  assert!(!has_recoverable_room(&store,&scope.origin,&scope.service,"other-account",scope.device,&scope.room).unwrap());
  assert!(!has_recoverable_room(&store,&scope.origin,&scope.service,&scope.account,scope.device,"other-room").unwrap());
  // A replaced desktop session does not key away the saved account/device owner.
  let mut renewed=scope.clone();renewed.desktop_epoch="new-session".into();
  assert!(has_recoverable_room(&store,&renewed.origin,&renewed.service,&renewed.account,renewed.device,&renewed.room).unwrap());
  retire(&store,scope,&marker).unwrap();
  assert!(has_recoverable_room(&store,&scope.origin,&scope.service,&scope.account,scope.device,&scope.room).unwrap());
  let mut journal=crate::waiting_local::Journal::new(marker.clone());journal.terminal=true;journal.retirement_done=true;
  crate::waiting_local::save_retired(&store,scope,&journal).unwrap();
  assert!(!has_recoverable_room(&store,&scope.origin,&scope.service,&scope.account,scope.device,&scope.room).unwrap());
 }
 #[test]fn st1_waiting_original_survives_restart_and_parks_without_authority(){
  let b=bundle(5);let store=Memory::default();let m=persist(&store,&b,424242,nt2_local_ws::owner::Id::OsProcess(9),[1;16],&claim(nt2_local_ws::owner::Id::OsProcess(9),[1;16])).unwrap();
  assert_eq!(WaitingMarker::decode(&m.encode().unwrap()).unwrap(),m);
  let restored=recall(&store,&b.scope.origin,&b.scope.service,&b.scope.account,b.scope.device,&m).unwrap().unwrap();assert_eq!(restored.original,b.scope);assert_eq!(restored.marker,m);assert_eq!(restored.disposition,Disposition::AwaitCanonicalWaitingRecovery);
  let json=String::from_utf8(m.encode().unwrap()).unwrap();for forbidden in ["proof","refresh","access","originalStart","release","desktop"]{assert!(!json.contains(forbidden));}
 }
 #[test]fn st1_waiting_account_device_and_origin_switch_cannot_load_original(){
  let b=bundle(1);let s=Memory::default();let m=persist(&s,&b,424242,nt2_local_ws::owner::Id::OsProcess(9),[1;16],&claim(nt2_local_ws::owner::Id::OsProcess(9),[1;16])).unwrap();
  for (origin,service,account,device) in [("https://other.invalid","service","account",b.scope.device),(b.scope.origin.as_str(),"other-service","account",b.scope.device),(b.scope.origin.as_str(),"service","other-account",b.scope.device),(b.scope.origin.as_str(),"service","account",uuid::Uuid::new_v4())]{assert!(recall(&s,origin,service,account,device,&m).unwrap().is_none());}
  assert!(recall(&s,&b.scope.origin,&b.scope.service,&b.scope.account,b.scope.device,&m).unwrap().is_some());
 }
 #[test]fn st1_waiting_wrong_seed_claim_or_marker_never_mints_authority(){
  let b=bundle(5);let s=Memory::default();assert!(persist(&s,&b,999,nt2_local_ws::owner::Id::OsProcess(9),[1;16],&claim(nt2_local_ws::owner::Id::OsProcess(9),[1;16])).is_err());assert!(persist(&s,&b,424242,nt2_local_ws::owner::Id::OsProcess(9),[2;16],&claim(nt2_local_ws::owner::Id::OsProcess(9),[1;16])).is_err());assert!(persist(&s,&b,424242,nt2_local_ws::owner::Id::OsProcess(9),[1;16],b"partial").is_err());
  let mut m=persist(&s,&b,424242,nt2_local_ws::owner::Id::OsProcess(9),[1;16],&claim(nt2_local_ws::owner::Id::OsProcess(9),[1;16])).unwrap();m.actual_seed="999".into();assert!(recall(&s,&b.scope.origin,&b.scope.service,&b.scope.account,b.scope.device,&m).is_err());
 }
 #[test]fn st1_waiting_individual_full_seed_and_exact_original_conflict(){
  let b=bundle(2);let s=Memory::default();let m=persist(&s,&b,u64::MAX,nt2_local_ws::owner::Id::OsProcess(9),[1;16],&claim(nt2_local_ws::owner::Id::OsProcess(9),[1;16])).unwrap();assert_eq!(m.actual_seed,"18446744073709551615");assert_eq!(m.loot,"individual:0");
  assert_eq!(persist(&s,&b,u64::MAX,nt2_local_ws::owner::Id::OsProcess(9),[1;16],&claim(nt2_local_ws::owner::Id::OsProcess(9),[1;16])).unwrap(),m);assert!(persist(&s,&b,42,nt2_local_ws::owner::Id::OsProcess(9),[1;16],&claim(nt2_local_ws::owner::Id::OsProcess(9),[1;16])).is_err());
 }
 #[test]fn st1_preparation_is_one_immutable_seed_policy_scope_snapshot(){
  let mut b=bundle(5);assert!(b.matches("run","run-epoch",1,Some("preparation"),424242,5));assert!(!b.matches("run","run-epoch",1,Some("preparation"),424242,1));assert!(!b.live(8));b.until=Instant::now();assert!(!b.live(7));
 }
 #[test]fn st1_waiting_offline_tombstone_blocks_late_save_and_recall(){
  let b=bundle(5);let s=Memory::default();let m=persist(&s,&b,424242,nt2_local_ws::owner::Id::OsProcess(9),[1;16],&claim(nt2_local_ws::owner::Id::OsProcess(9),[1;16])).unwrap();retire(&s,&b.scope,&m).unwrap();assert!(recall(&s,&b.scope.origin,&b.scope.service,&b.scope.account,b.scope.device,&m).unwrap().is_none());assert!(persist(&s,&b,424242,nt2_local_ws::owner::Id::OsProcess(9),[1;16],&claim(nt2_local_ws::owner::Id::OsProcess(9),[1;16])).is_err());
 }
}

