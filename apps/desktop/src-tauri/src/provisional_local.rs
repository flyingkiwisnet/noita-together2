//! Client-only provisional B. This is NOT PendingIntent or canonical authority.
//! A reviewed entry-preview decoder will construct Preview; no wire is invented here.
//! The independent worker never receives A's pending/lifecycle/contact/generation.
use crate::{secrets::{CredentialStore,SecretString},startup_local::Scope};
use serde::{Serialize,Deserialize};use serde_json::Value;use sha2::{Digest,Sha256};
use crate::protocol_waiting::{self as protocol,wire as w};
use std::sync::{Arc,Mutex,mpsc,atomic::{AtomicBool,Ordering}};use std::time::Instant;
const PURPOSE:&str="nt2-local-provisional-entry-v1";const HISTORY:usize=16;
#[derive(Clone)]pub struct Preview{pub scope:Scope,pub configuration:Value,pub base:Value,pub seed:u32,pub policy:u32,pub until:Instant,pub sample:crate::connection_time::Stamp}
impl Preview{
 fn validate(&self)->Result<(),String>{
  // This field is preallocated locally, never borrowed from A's preparation.
  if !self.scope.preparation.is_empty(){return Err("Provisional preview must not contain a canonical preparation".into())}
  let mut scope=self.scope.clone();scope.preparation="local-provisional".into();scope.validate()?;
  let config=serde_json::to_vec(&self.configuration).map_err(|_|"Provisional config encoding")?;
  let entry:w::EntryScope=serde_json::from_value(self.base.clone()).map_err(|_|"Exact entry preview scope absent")?;protocol::validate("EntryScope",&entry)?;
  if entry.service_id.as_deref()!=Some(&scope.service)||entry.service_epoch.as_deref()!=Some(&scope.service_epoch)||entry.account_id.as_deref()!=Some(&scope.account)||entry.device_id.as_deref()!=Some(scope.device.to_string().as_str())||entry.room_id.as_deref()!=Some(&scope.room)||entry.run_id.as_deref()!=Some(&scope.run)||entry.run_epoch.as_deref()!=Some(&scope.run_epoch)||entry.config_revision.map(u64::from)!=Some(scope.config)||entry.config_digest.as_deref()!=Some(&scope.config_digest)
   ||entry.base_preparation_id.as_deref()==Some(&scope.preparation)||entry.base_fingerprint.as_ref().is_none_or(|v|v.len()!=64)
   ||(self.policy==2 && (entry.seed_policy.as_deref()!=Some("individual-random")||entry.actual_seed.is_some()||entry.randomize_loot!=Some(false)))
   ||(self.policy!=2 && (!matches!(entry.seed_policy.as_deref(),Some("shared-fixed"|"shared-random"))||entry.actual_seed.as_deref()!=Some(self.seed.to_string().as_str())||entry.randomize_loot!=Some(self.policy==5))){return Err("Entry preview scope/settings differ".into())}
  if config.len()>8192||!self.configuration.is_object()||crate::protocol_connection::canonical_fingerprint(&config)?!=scope.config_digest
    ||serde_json::to_vec(&self.base).map_err(|_|"Provisional base encoding")?.len()>4096
    ||!matches!(self.policy,1|2|5)||self.policy==2&&self.seed!=0||self.sample.sequence==0||self.sample.started>Instant::now()||self.until<=Instant::now()||self.until>self.sample.started+std::time::Duration::from_secs(30){return Err("Invalid provisional preview/settings/validity".into())}Ok(())
 }
 fn identity(&self)->Result<String,String>{fingerprint(&(&self.scope.origin,&self.configuration,&self.base,self.seed,self.policy))}
}
fn fingerprint<T:Serialize>(value:&T)->Result<String,String>{crate::protocol_connection::canonical_fingerprint(&serde_json::to_vec(value).map_err(|_|"Provisional identity encoding")?)}
#[derive(Clone,Copy,Debug,PartialEq,Eq,Serialize,Deserialize)]
pub enum Phase{Staged,Claimed,ActivationSubmitted,ActivationReconciled,Detached,CancelSubmitted,Cancelled}
#[derive(Clone,Serialize,Deserialize)]#[serde(deny_unknown_fields)]
struct Original{fingerprint:String,meta:w::EntryOriginal,bytes:Vec<u8>}
impl Drop for Original{fn drop(&mut self){use zeroize::Zeroize;self.bytes.zeroize()}}
struct SealedRequest(w::Request);
impl std::ops::Deref for SealedRequest{type Target=w::Request;fn deref(&self)->&w::Request{&self.0}}
impl Drop for SealedRequest{fn drop(&mut self){use zeroize::Zeroize;if let Some(p)=self.0.entry_activate.as_mut().and_then(|p|p.recovery_proof.as_mut()){p.zeroize()}if let Some(p)=self.0.entry_cancel.as_mut().and_then(|p|p.recovery_proof.as_mut()){p.zeroize()}}}
struct SealedResponse(w::Response);
impl std::ops::Deref for SealedResponse{type Target=w::Response;fn deref(&self)->&w::Response{&self.0}}
impl Drop for SealedResponse{fn drop(&mut self){use zeroize::Zeroize;for proof in [&mut self.0.pending_proof,&mut self.0.waiting_proof,&mut self.0.resume_proof].into_iter().flatten(){proof.zeroize()}if let Some(c)=&mut self.0.credentials{if let Some(s)=&mut c.access_token{s.zeroize()}if let Some(s)=&mut c.refresh_secret{s.zeroize()}}}}
#[derive(Clone,Serialize,Deserialize)]#[serde(deny_unknown_fields)]
struct Owner{pid:crate::local_owner::Id,process_start:u64}
#[derive(Clone,Serialize,Deserialize)]#[serde(deny_unknown_fields)]
pub struct Canonical{pub scope:Scope,pub marker:crate::startup_local::WaitingMarker,pub pending:w::PendingIntent,pub pending_generation:u32,pub waiting:Option<w::WaitingIntent>,pub waiting_proof:Option<String>,pub waiting_generation:u32,pub attachment:Option<String>}
impl Drop for Canonical{fn drop(&mut self){use zeroize::Zeroize;if let Some(p)=&mut self.waiting_proof{p.zeroize()}}}
#[derive(Clone,Serialize,Deserialize)]#[serde(deny_unknown_fields)]
pub struct Record{
 version:u8,kind:String,scope:Scope,world:uuid::Uuid,context:uuid::Uuid,pending:uuid::Uuid,instance:uuid::Uuid,attempt:uuid::Uuid,
 preview_identity:String,binding:String,configuration:Value,base:Value,seed:u32,policy:u32,secret:String,commitment:String,
 phase:Phase,owner:Option<Owner>,activation:Option<Original>,cancel:Option<Original>,detached:bool,
 #[serde(default)]canonical:Option<Canonical>,#[serde(default)]retries:Vec<Original>,
}
impl Drop for Record{fn drop(&mut self){use zeroize::Zeroize;self.secret.zeroize();if let Some(o)=&mut self.activation{o.bytes.zeroize()}if let Some(o)=&mut self.cancel{o.bytes.zeroize()}for o in &mut self.retries{o.bytes.zeroize()}}}
impl Record{
 fn new(preview:&Preview)->Result<Self,String>{
  preview.validate()?;let secret=random_secret()?;let commitment=format!("{:x}",Sha256::digest(secret.as_bytes()));
  let mut scope=preview.scope.clone();scope.preparation=uuid::Uuid::new_v4().to_string();
  let mut record=Self{version:1,kind:"nt2-provisional-entry-v1".into(),scope,world:uuid::Uuid::new_v4(),context:uuid::Uuid::new_v4(),pending:uuid::Uuid::new_v4(),instance:uuid::Uuid::new_v4(),attempt:uuid::Uuid::new_v4(),preview_identity:preview.identity()?,binding:String::new(),configuration:preview.configuration.clone(),base:preview.base.clone(),seed:preview.seed,policy:preview.policy,secret,commitment,phase:Phase::Staged,owner:None,activation:None,cancel:None,detached:false,canonical:None,retries:vec![]};record.binding=record.identity()?;Ok(record)
 }
 fn identity(&self)->Result<String,String>{fingerprint(&(&self.scope,self.world,self.context,self.pending,self.instance,self.attempt,&self.preview_identity,&self.commitment))}
 fn validate(&self)->Result<(),String>{
  use base64::Engine;self.scope.validate()?;
  let bytes=zeroize::Zeroizing::new(base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(&self.secret).map_err(|_|"Provisional future proof encoding")?);
  if self.version!=1||self.kind!="nt2-provisional-entry-v1"||[self.world,self.context,self.pending,self.instance,self.attempt].iter().any(uuid::Uuid::is_nil)
   ||self.world==self.context||bytes.len()!=32||self.secret.len()!=43||self.commitment!=format!("{:x}",Sha256::digest(self.secret.as_bytes()))
   ||self.binding!=self.identity()?||!matches!(self.policy,1|2|5)||self.policy==2&&self.seed!=0{return Err("Provisional identity corrupt".into())}
  if self.preview_identity!=fingerprint(&(&self.scope.origin,&self.configuration,&self.base,self.seed,self.policy))?{return Err("Provisional immutable preview differs".into())}
  for (original,cancel) in [(&self.activation,false),(&self.cancel,true)]{if let Some(original)=original{
   let captured=self.capture(&original.bytes,cancel,original.meta.original_session_id.as_deref().ok_or("Original entry actor absent")?)?;
   if original.fingerprint!=captured.fingerprint||serde_json::to_value(&original.meta).ok()!=serde_json::to_value(&captured.meta).ok(){return Err("Provisional original differs".into())}
  }}
  if self.retries.len()>16{return Err("Entry reauthentication original capacity".into())}
  for original in &self.retries{let cancel=original.meta.kind.as_deref()==Some("entryCancel");let captured=self.capture(&original.bytes,cancel,original.meta.original_session_id.as_deref().ok_or("Entry retry actor absent")?)?;if captured.fingerprint!=original.fingerprint||serde_json::to_value(&captured.meta).ok()!=serde_json::to_value(&original.meta).ok(){return Err("Entry reauthentication original differs".into())}}
  if self.owner.as_ref().is_some_and(|o|o.pid==0||o.process_start==0)||self.activation.is_some()&&self.owner.is_none()||self.cancel.is_some()&&!self.detached{return Err("Provisional owner/retirement differs".into())}
  if let Some(c)=&self.canonical{
   c.scope.validate()?;c.marker.encode()?;protocol::validate("PendingIntent",&c.pending)?;
   let mut original=self.scope.clone();original.desktop=c.scope.desktop.clone();original.desktop_epoch=c.scope.desktop_epoch.clone();let entry=self.entry_scope()?;
   if self.activation.is_none()||c.scope!=original||c.marker.local_world!=self.world||c.marker.bundle_digest!=self.binding||c.pending_generation==0
    ||c.pending.pending_id.as_deref()!=Some(self.pending.to_string().as_str())||c.pending.preparation_id.as_deref()!=Some(&self.scope.preparation)||c.pending.world_token.as_deref()!=Some(self.world.simple().to_string().as_str())||c.pending.context_id.as_deref()!=Some(self.context.simple().to_string().as_str())
    ||c.pending.original_desktop_id.as_deref()!=Some(&c.scope.desktop)||c.pending.original_desktop_epoch.as_deref()!=Some(&c.scope.desktop_epoch)
    ||c.pending.run_id.as_deref()!=Some(&c.scope.run)||c.pending.run_epoch.as_deref()!=Some(&c.scope.run_epoch)||c.pending.account_id.as_deref()!=Some(&c.scope.account)||c.pending.device_id.as_deref()!=Some(c.scope.device.to_string().as_str())||c.pending.config_digest.as_deref()!=Some(&c.scope.config_digest)
    ||c.pending.service_id!=entry.service_id||c.pending.service_epoch!=entry.service_epoch||c.pending.room_id!=entry.room_id||c.pending.config_revision!=entry.config_revision||c.pending.seed_policy!=entry.seed_policy||c.pending.randomize_loot!=entry.randomize_loot||c.pending.actual_seed!=entry.actual_seed
    ||c.marker.kind!=if c.waiting.is_some(){"nt2-waiting"}else{"nt2-pending"}||c.marker.loot!=if self.policy==2{"individual:0".into()}else{format!("shared:{}:{}",u8::from(self.policy==5),c.marker.actual_seed)}||self.policy!=2&&c.marker.actual_seed!=self.seed.to_string()
    ||c.waiting_proof.as_ref().is_some_and(|p|p.len()!=43)||c.waiting.is_none()&&(c.waiting_proof.is_some()||c.waiting_generation!=0)||c.waiting.is_some()&&c.waiting_generation==0{return Err("Provisional canonical receipt differs from sealed original".into())}
   if let Some(waiting)=&c.waiting{let mut journal=crate::waiting_local::Journal::new(c.marker.clone());journal.intent=Some(waiting.clone());journal.generation=c.waiting_generation;journal.current_attachment=c.attachment.clone();journal.validate(&c.scope)?;}
  }Ok(())
 }
 pub fn world(&self)->uuid::Uuid{self.world}
 pub fn phase(&self)->Phase{self.phase}
 pub fn scope(&self)->&Scope{&self.scope}
 pub fn context(&self)->uuid::Uuid{self.context}
 pub fn future_pending(&self)->uuid::Uuid{self.pending}
 pub fn game_instance(&self)->uuid::Uuid{self.instance}
 pub fn digest(&self)->&str{&self.binding}
 pub fn canonical(&self)->Option<Canonical>{self.canonical.clone()}
 pub fn commitment(&self)->&str{&self.commitment}
 pub fn base(&self)->&Value{&self.base}
 pub fn entry_scope(&self)->Result<w::EntryScope,String>{let scope=serde_json::from_value(self.base.clone()).map_err(|_|"Stored entry scope corrupt")?;protocol::validate("EntryScope",&scope)?;Ok(scope)}
 pub fn entry_identity(&self)->w::EntryIdentity{w::EntryIdentity{preparation_id:Some(self.scope.preparation.clone()),pending_id:Some(self.pending.to_string()),world_token:Some(self.world.simple().to_string()),context_id:Some(self.context.simple().to_string()),proof_commitment:Some(self.commitment.clone())}}
 pub fn activation_original(&self)->Option<zeroize::Zeroizing<Vec<u8>>>{self.activation.as_ref().map(|o|zeroize::Zeroizing::new(o.bytes.clone()))}
 pub fn cancellation_original(&self)->Option<zeroize::Zeroizing<Vec<u8>>>{self.cancel.as_ref().map(|o|zeroize::Zeroizing::new(o.bytes.clone()))}
 pub fn activation_metadata(&self)->Option<w::EntryOriginal>{self.activation.as_ref().map(|o|o.meta.clone())}
 pub fn cancellation_metadata(&self)->Option<w::EntryOriginal>{self.cancel.as_ref().map(|o|o.meta.clone())}
 pub fn operation_for(&self,cancel:bool,actor:&str,epoch:&str)->Option<zeroize::Zeroizing<Vec<u8>>>{self.retries.iter().chain(if cancel{self.cancel.iter()}else{self.activation.iter()}).find(|o|o.meta.kind.as_deref()==Some(if cancel{"entryCancel"}else{"entryActivate"})&&o.meta.original_session_id.as_deref()==Some(actor)&&o.meta.original_session_epoch.as_deref()==Some(epoch)).map(|o|zeroize::Zeroizing::new(o.bytes.clone()))}
 pub fn operation_for_service(&self,cancel:bool,actor:&str,epoch:&str,service_epoch:&str)->Option<zeroize::Zeroizing<Vec<u8>>>{self.retries.iter().chain(if cancel{self.cancel.iter()}else{self.activation.iter()}).find(|o|o.meta.kind.as_deref()==Some(if cancel{"entryCancel"}else{"entryActivate"})&&o.meta.original_session_id.as_deref()==Some(actor)&&o.meta.original_session_epoch.as_deref()==Some(epoch)&&o.meta.original_service_epoch.as_deref()==Some(service_epoch)).map(|o|zeroize::Zeroizing::new(o.bytes.clone()))}
 pub fn future_secret(&self)->SecretString{SecretString::from(self.secret.clone())} // Rust-only canonical activation/recovery adapter.
 pub fn snapshot(&self)->Result<Vec<u8>,String>{
  // No secret, base body, request envelope, credential or play/release field.
  serde_json::to_vec(&serde_json::json!({"version":1,"kind":self.kind,"scope":self.scope,"world":self.world,"context":self.context,"future_pending":self.pending,"game_instance":self.instance,"attempt":self.attempt,"preview_digest":self.preview_identity,"base_digest":fingerprint(&self.base)?,"seed":self.seed,"policy":self.policy,"config":self.configuration})).map_err(|_|"Provisional nonsecret snapshot encoding".into())
 }
 fn claim(&mut self,pid:crate::local_owner::Id,process_start:u64)->Result<(),String>{
  if self.detached||pid==0||process_start==0{return Err("Provisional claim refused".into())}
  if let Some(owner)=&self.owner{if owner.pid==pid&&owner.process_start==process_start{return Ok(())}return Err("Original provisional owner differs".into())}
  if self.phase!=Phase::Staged{return Err("Provisional attempt already consumed".into())}self.owner=Some(Owner{pid,process_start});self.phase=Phase::Claimed;Ok(())
 }
 fn capture(&self,bytes:&[u8],cancel:bool,actor:&str)->Result<Original,String>{
  if bytes.is_empty()||bytes.len()>32768||uuid::Uuid::parse_str(actor).is_err(){return Err("Entry original actor/bound".into())}
  struct Envelope(w::Request);impl Drop for Envelope{fn drop(&mut self){use zeroize::Zeroize;if let Some(p)=self.0.entry_activate.as_mut().and_then(|a|a.recovery_proof.as_mut()){p.zeroize()}if let Some(p)=self.0.entry_cancel.as_mut().and_then(|a|a.recovery_proof.as_mut()){p.zeroize()}}}
  let request=Envelope(serde_json::from_slice(bytes).map_err(|_|"Entry original encoding")?);let r=&request.0;protocol::validate("Request",r)?;
  if r.revision.as_deref()!=Some(protocol::REVISION)||r.service_id.as_deref()!=Some(&self.scope.service)||r.service_epoch.as_ref().is_none_or(|e|uuid::Uuid::parse_str(e).is_err())||(!cancel&&r.service_epoch.as_deref()!=Some(&self.scope.service_epoch)){return Err("Entry original issuer differs".into())}
  let (scope,identity,proof)=if cancel{let a=r.entry_cancel.as_ref().ok_or("Entry cancellation absent")?;if a.world_offline!=Some(true){return Err("Entry cancellation must be explicit world offline".into())}(&a.scope,&a.identity,&a.recovery_proof)}else{
   let a=r.entry_activate.as_ref().ok_or("Entry activation absent")?;let o=a.observation.as_ref().ok_or("Entry actual observation absent")?;
   if o.attachment_id.is_some()||o.world_token.as_deref()!=Some(self.world.simple().to_string().as_str())||o.game_instance_id.as_deref()!=Some(self.instance.to_string().as_str())||o.loaded!=Some(true)||o.protected!=Some(true)||o.inventory_complete!=Some(true)
      ||o.actual_seed.as_ref().is_none_or(|s|s.parse::<u64>().ok().is_none_or(|n|n.to_string()!=*s||(self.policy!=2&&n!=u64::from(self.seed)))){return Err("Entry activation actual protected world differs".into())}
   (&a.scope,&a.identity,&a.recovery_proof)
  };
  if serde_json::to_value(scope).map_err(|_|"Entry scope encoding")?!=self.base||serde_json::to_value(identity).ok()!=serde_json::to_value(self.entry_identity()).ok()||proof.as_deref()!=Some(&self.secret){return Err("Entry original scope/identity/proof differs".into())}
  let fingerprint=crate::protocol_connection::canonical_fingerprint(bytes)?;
  let meta=w::EntryOriginal{account_id:Some(self.scope.account.clone()),device_id:Some(self.scope.device.to_string()),original_session_id:Some(actor.into()),original_session_epoch:r.session_epoch.clone(),original_service_epoch:r.service_epoch.clone(),request_id:r.request_id.clone(),kind:Some(if cancel{"entryCancel"}else{"entryActivate"}.into()),fingerprint:Some(fingerprint.clone()),waiting_id:None,run_id:Some(self.scope.run.clone()),pending_id:Some(self.pending.to_string())};protocol::validate("EntryOriginal",&meta)?;
  Ok(Original{fingerprint,meta,bytes:bytes.to_vec()})
 }
 fn original(&mut self,bytes:Vec<u8>,cancel:bool,actor:&str)->Result<(),String>{
  let bytes=zeroize::Zeroizing::new(bytes);
  if bytes.is_empty()||bytes.len()>32768{return Err("Provisional original bound".into())}
  if cancel{if !self.detached||self.activation.is_none(){return Err("Cancellation requires the original submitted activation".into())}}
  else if self.detached||self.owner.is_none(){return Err("Detached/unclaimed provisional cannot activate".into())}
  let captured=self.capture(&bytes,cancel,actor)?;let fingerprint=captured.fingerprint.clone();
  let target=if cancel{&mut self.cancel}else{&mut self.activation};
  if let Some(original)=target{if original.bytes.as_slice()==bytes.as_slice()&&original.fingerprint==fingerprint&&original.meta.original_session_id==captured.meta.original_session_id{return Ok(())}
   let old=SealedRequest(serde_json::from_slice(&original.bytes).map_err(|_|"Original operation corrupt")?);let new=SealedRequest(serde_json::from_slice(&bytes).map_err(|_|"Entry retry corrupt")?);
   if old.session_epoch==new.session_epoch&&old.service_epoch==new.service_epoch&&original.meta.original_session_id==captured.meta.original_session_id||serde_json::to_value((&old.entry_activate,&old.entry_cancel)).ok()!=serde_json::to_value((&new.entry_activate,&new.entry_cancel)).ok()||self.operation_for_service(cancel,actor,new.session_epoch.as_deref().unwrap_or(""),new.service_epoch.as_deref().unwrap_or("")).is_some()||self.retries.len()>=16{return Err("Original provisional operation retained".into())}
   self.retries.push(captured);self.phase=if cancel{Phase::CancelSubmitted}else{Phase::ActivationSubmitted};return Ok(())
  }
  *target=Some(captured);self.phase=if cancel{Phase::CancelSubmitted}else{Phase::ActivationSubmitted};Ok(())
 }
 fn detach(&mut self){self.detached=true;self.phase=Phase::Detached;}
 fn confirm_activation(&mut self,response:&w::Response,actual:&str)->Result<(),String>{
  let request=SealedRequest(serde_json::from_slice(&self.activation.as_ref().ok_or("Sealed entry activation absent")?.bytes).map_err(|_|"Original entry request corrupt")?);self.confirm_current(response,actual,&request,false)
 }
 fn confirm_current(&mut self,response:&w::Response,actual:&str,request:&w::Request,cancel:bool)->Result<(),String>{
  protocol::validate("Request",request)?;if request.service_id.as_deref()!=Some(&self.scope.service)||(!cancel&&request.service_epoch.as_deref()!=Some(&self.scope.service_epoch))||request.service_epoch.as_ref().is_none_or(|e|uuid::Uuid::parse_str(e).is_err()){return Err("Current entry query issuer differs".into())}
  if let Some(q)=&request.entry_status{if serde_json::to_value(&q.scope).ok()!=Some(self.base.clone())||serde_json::to_value(&q.identity).ok()!=serde_json::to_value(self.entry_identity()).ok(){return Err("Current entry lookup differs".into())}}
  else{let captured=self.capture(&serde_json::to_vec(request).map_err(|_|"Entry operation encoding")?,cancel,if cancel{self.cancel.as_ref()}else{self.activation.as_ref()}.and_then(|o|o.meta.original_session_id.as_deref()).ok_or("Entry original absent")?)?;if !self.activation.iter().chain(self.cancel.iter()).chain(self.retries.iter()).any(|o|o.fingerprint==captured.fingerprint){return Err("Current entry operation was not sealed".into())}}
  if response.request_id!=request.request_id||response.service_epoch!=request.service_epoch{return Err("Current entry response correlation differs".into())}
  if cancel{return self.confirm_cancel_state(response)}
  protocol::validate("Response",response)?;let original=self.activation.as_ref().ok_or("Sealed entry activation absent")?;
  if response.major!=Some(1)||response.revision.as_deref()!=Some(protocol::REVISION)||response.service_id.as_deref()!=Some(&self.scope.service)||original.meta.kind.as_deref()!=Some("entryActivate"){return Err("Entry activation response correlation differs".into())}
  let entry=response.entry.as_ref().ok_or("Entry disposition absent")?;let state=response.pending.as_ref().ok_or("Current canonical pending absent")?;let intent=state.intent.as_ref().ok_or("Current canonical pending intent absent")?;let scope=self.entry_scope()?;
  if entry.state.as_deref()!=Some("activated")||entry.may_play!=Some(false)||serde_json::to_value(&entry.scope).ok()!=Some(self.base.clone())||serde_json::to_value(&entry.identity).ok()!=serde_json::to_value(self.entry_identity()).ok()
   ||state.may_play!=Some(false)||!matches!(state.state.as_deref(),Some("pending"|"promoted"))||state.generation.is_none_or(|g|g==0)
   ||intent.pending_id.as_deref()!=Some(self.pending.to_string().as_str())||intent.preparation_id.as_deref()!=Some(&self.scope.preparation)||intent.service_id!=scope.service_id||intent.service_epoch!=scope.service_epoch||intent.account_id!=scope.account_id||intent.device_id!=scope.device_id||intent.room_id!=scope.room_id||intent.run_id!=scope.run_id||intent.run_epoch!=scope.run_epoch||intent.config_revision!=scope.config_revision||intent.config_digest!=scope.config_digest||intent.seed_policy!=scope.seed_policy||intent.randomize_loot!=scope.randomize_loot||intent.world_token.as_deref()!=Some(self.world.simple().to_string().as_str())||intent.context_id.as_deref()!=Some(self.context.simple().to_string().as_str())
   ||actual.parse::<u64>().ok().is_none_or(|n|n.to_string()!=actual||(self.policy!=2&&n!=u64::from(self.seed)))||state.bound_actual_seed.as_deref().is_some_and(|s|s!=actual)||response.pending_proof.as_ref().is_some_and(|p|p!=&self.secret){return Err("Historical/changed entry does not establish current B ownership".into())}
  let canonical_scope=Scope{origin:self.scope.origin.clone(),service:self.scope.service.clone(),service_epoch:self.scope.service_epoch.clone(),account:self.scope.account.clone(),device:self.scope.device,desktop:intent.original_desktop_id.clone().ok_or("Canonical original desktop absent")?,desktop_epoch:intent.original_desktop_epoch.clone().ok_or("Canonical original session absent")?,room:self.scope.room.clone(),run:self.scope.run.clone(),run_epoch:self.scope.run_epoch.clone(),config:self.scope.config,config_digest:self.scope.config_digest.clone(),preparation:self.scope.preparation.clone()};canonical_scope.validate()?;
  let waiting=response.waiting.as_ref();if let Some(waiting)=waiting{let i=waiting.intent.as_ref().ok_or("Canonical waiting intent absent")?;
   // A current receipt can describe expired ownership. It preserves identity
   // for common proof recovery; it never establishes a healthy owner or play.
   if waiting.may_play!=Some(false)||!matches!(waiting.state.as_deref(),Some("waiting"|"recoverable"|"started")){return Err("Canonical waiting B receipt state is terminal or grants play".into())}
   if state.waiting_id!=i.waiting_id||i.pending_reservation_id.as_ref().is_some_and(|v|Some(v)!=intent.pending_id.as_ref())||i.preparation_id!=intent.preparation_id||i.world_token!=intent.world_token||i.actual_seed.as_deref()!=Some(actual)||i.config_digest!=intent.config_digest||i.run_id!=intent.run_id||i.run_epoch!=intent.run_epoch||i.account_id!=intent.account_id||i.device_id!=intent.device_id||i.original_desktop_id!=intent.original_desktop_id||i.original_desktop_epoch!=intent.original_desktop_epoch||i.context_id!=intent.context_id||i.seed_policy!=intent.seed_policy||i.randomize_loot!=intent.randomize_loot{return Err("Canonical waiting B link differs".into())}
  }
  if response.waiting_proof.as_ref().is_some_and(|p|p.len()!=43)||response.waiting_proof.is_some()&&waiting.is_none(){return Err("Canonical waiting proof differs".into())}
  let marker=crate::startup_local::WaitingMarker{kind:if waiting.is_some(){"nt2-waiting"}else{"nt2-pending"}.into(),version:1,local_world:self.world,bundle_digest:self.binding.clone(),actual_seed:actual.into(),loot:if self.policy==2{"individual:0".into()}else{format!("shared:{}:{actual}",u8::from(self.policy==5))}};
  let canonical=Canonical{scope:canonical_scope,marker,pending:intent.clone(),pending_generation:state.generation.unwrap(),waiting:waiting.and_then(|v|v.intent.clone()),waiting_proof:response.waiting_proof.clone(),waiting_generation:waiting.and_then(|v|v.generation).unwrap_or(0),attachment:waiting.and_then(|v|v.owner.as_ref()).and_then(|v|v.attachment_id.clone())};
  if let Some(old)=&self.canonical{if old.scope!=canonical.scope||serde_json::to_value(&old.pending).ok()!=serde_json::to_value(&canonical.pending).ok()||canonical.pending_generation<old.pending_generation||old.waiting.is_some()&&serde_json::to_value(&old.waiting).ok()!=serde_json::to_value(&canonical.waiting).ok(){return Err("Canonical B immutable original/generation differs".into())}}
  self.canonical=Some(canonical);if !self.detached{self.phase=Phase::ActivationReconciled;}Ok(())
 }
 fn confirm_cancel(&mut self,response:&w::Response)->Result<(),String>{
  let original=self.cancel.as_ref().ok_or("Cancellation original absent")?;if response.request_id!=original.meta.request_id||response.service_epoch!=original.meta.original_service_epoch{return Err("Cancellation response correlation differs".into())}self.confirm_cancel_state(response)
 }
 fn confirm_cancel_state(&mut self,response:&w::Response)->Result<(),String>{
  protocol::validate("Response",response)?;let original=self.cancel.as_ref().ok_or("Cancellation original absent")?;
  let state=response.entry.as_ref().ok_or("Cancellation disposition absent")?;
  if response.major!=Some(1)||response.revision.as_deref()!=Some(protocol::REVISION)||response.service_id.as_deref()!=Some(&self.scope.service)||original.meta.kind.as_deref()!=Some("entryCancel")
   ||state.state.as_deref()!=Some("cancelled")||state.reason.as_deref()!=Some("explicit_offline")||state.may_play!=Some(false)||serde_json::to_value(&state.scope).ok()!=Some(self.base.clone())||serde_json::to_value(&state.identity).ok()!=serde_json::to_value(self.entry_identity()).ok()
   ||response.credentials.is_some()||response.pending_proof.is_some()||response.waiting_proof.is_some()||response.resume_proof.is_some()||response.canonical_admission.is_some(){return Err("Cancellation acknowledgment differs; retirement pending".into())}
  self.phase=Phase::Cancelled;self.detached=true;Ok(())
 }
}
#[derive(Serialize,Deserialize)]#[serde(deny_unknown_fields)]struct Head{history:Vec<uuid::Uuid>,record:Record}
fn same_issuer(a:&Scope,b:&Scope)->bool{a.origin==b.origin&&a.service==b.service&&a.account==b.account&&a.device==b.device}
fn read_head<S:CredentialStore>(store:&S,scope:&Scope)->Result<Option<Head>,String>{
 let Some(bytes)=crate::protected_chunks::load(store,scope,PURPOSE,uuid::Uuid::nil())?else{return Ok(None)};
 let head:Head=serde_json::from_slice(&bytes).map_err(|_|"Provisional head corrupt")?;head.record.validate()?;
 if head.history.len()>HISTORY||!same_issuer(&head.record.scope,scope){return Err("Provisional issuer/history differs".into())}Ok(Some(head))
}
fn save_head<S:CredentialStore>(store:&S,head:&Head)->Result<(),String>{head.record.validate()?;let bytes=zeroize::Zeroizing::new(serde_json::to_vec(head).map_err(|_|"Provisional head encoding")?);crate::protected_chunks::save(store,&head.record.scope,PURPOSE,uuid::Uuid::nil(),&bytes)}
pub fn load<S:CredentialStore>(store:&S,scope:&Scope,world:uuid::Uuid)->Result<Option<Record>,String>{
 if let Some(head)=read_head(store,scope)?{if head.record.world==world{return Ok(Some(head.record))}}
 let Some(bytes)=crate::protected_chunks::load(store,scope,PURPOSE,world)?else{return Ok(None)};
 let record:Record=serde_json::from_slice(&bytes).map_err(|_|"Provisional archive corrupt")?;record.validate()?;
 if record.world!=world||!same_issuer(&record.scope,scope){return Err("Provisional archive identity differs".into())}Ok(Some(record))
}
pub fn current<S:CredentialStore>(store:&S,scope:&Scope)->Result<Option<Record>,String>{Ok(read_head(store,scope)?.map(|h|h.record))}
#[cfg(test)]fn stage<S:CredentialStore>(store:&S,preview:&Preview)->Result<Record,String>{stage_with(store,preview,true)}
fn stage_with<S:CredentialStore>(store:&S,preview:&Preview,replace_unused:bool)->Result<Record,String>{
 preview.validate()?;let mut history=vec![];
 if let Some(head)=read_head(store,&preview.scope)?{
  if (!head.record.detached&&head.record.canonical.is_none()&&(head.record.owner.is_some()||!replace_unused||head.record.preview_identity==preview.identity()?))||(head.record.activation.is_some()&&head.record.phase!=Phase::Cancelled&&head.record.canonical.is_none()){
   if head.record.preview_identity==preview.identity()?{return Ok(head.record)}
   return Err("Original provisional B remains unresolved; no replacement/rebase".into())
  }
  // Resolved/never-submitted old B stays addressable by its exact world key.
  // It is not an unresolved queue entry and must not exhaust a lifetime slot.
  history=head.history;
  let archive=zeroize::Zeroizing::new(serde_json::to_vec(&head.record).map_err(|_|"Provisional archive encoding")?);
  crate::protected_chunks::save(store,&head.record.scope,PURPOSE,head.record.world,&archive)?;
 }
 let record=Record::new(preview)?;save_head(store,&Head{history,record:record.clone()})?;Ok(record)
}
#[derive(Clone,Debug)]pub struct View{pub world:uuid::Uuid,pub context:uuid::Uuid,pub seed:u32,pub policy:u32,pub scope:Scope,pub digest:String,pub owner:Option<(crate::local_owner::Id,u64)>,pub until:Option<Instant>,pub phase:Phase,pub snapshot:Vec<u8>,pub retirement_pending:bool}
impl View{fn of(record:&Record)->Result<Self,String>{Ok(Self{world:record.world,context:record.context,seed:record.seed,policy:record.policy,scope:record.scope.clone(),digest:record.binding.clone(),owner:record.owner.as_ref().map(|o|(o.pid,o.process_start)),until:None,phase:record.phase,snapshot:record.snapshot()?,retirement_pending:record.detached&&record.activation.is_some()&&record.phase!=Phase::Cancelled})}}
#[derive(Default)]struct Cache{busy:bool,view:Option<View>,error:Option<String>}
// Local ordering only. No lease or play authority. Offer and transition use
// this same short lock; no credential, HTTP, lifecycle or pending I/O under it.
struct LineageState{key:String,epoch:u64,ready:bool,offered:Option<uuid::Uuid>}
pub struct Lineage(Mutex<LineageState>);
impl Default for Lineage{fn default()->Self{Self(Mutex::new(LineageState{key:String::new(),epoch:1,ready:true,offered:None}))}}
impl Lineage{
 pub fn update(&self,key:String,ready:bool)->bool{let mut s=self.0.lock().unwrap();if s.key==key&&s.ready==ready{return true}if s.offered.is_some(){return false}if s.ready||ready{s.epoch=s.epoch.checked_add(1).expect("Local lineage epoch exhausted")}s.key=key;s.ready=ready;true}
 pub fn block(&self,key:&str)->bool{self.update(key.into(),false)}
 pub fn snapshot(&self)->Option<u64>{let s=self.0.lock().unwrap();s.ready.then_some(s.epoch)}
 pub fn current(&self,epoch:u64)->bool{self.snapshot()==Some(epoch)}
 pub fn resolve(&self,world:uuid::Uuid){let mut s=self.0.lock().unwrap();if s.offered==Some(world){s.offered=None;}}
}
enum Job{Stage(Preview,Option<u64>),Recover{scope:Scope,world:uuid::Uuid},Claim{world:uuid::Uuid,pid:crate::local_owner::Id,start:u64},Activation{world:uuid::Uuid,bytes:zeroize::Zeroizing<Vec<u8>>,actor:String},Detach,Cancel{world:uuid::Uuid,bytes:zeroize::Zeroizing<Vec<u8>>,actor:String},CancelAck(SealedResponse),ActivationAck(SealedResponse,String),Current(SealedResponse,String,SealedRequest,bool)}
pub struct Worker{tx:Option<mpsc::SyncSender<Job>>,thread:Option<std::thread::JoinHandle<()>>,cache:Arc<Mutex<Cache>>,detached:Arc<AtomicBool>,fresh_started:Arc<AtomicBool>,lineage:Option<Arc<Lineage>>,slot_epoch:Arc<std::sync::atomic::AtomicU64>}
impl Worker{
 pub fn new<S:CredentialStore+'static>(store:Arc<S>,writer:Arc<Mutex<()>>)->Self{
  Self::construct(store,writer,None)
 }
 pub fn coherent<S:CredentialStore+'static>(store:Arc<S>,writer:Arc<Mutex<()>>,lineage:Arc<Lineage>)->Self{Self::construct(store,writer,Some(lineage))}
 fn construct<S:CredentialStore+'static>(store:Arc<S>,writer:Arc<Mutex<()>>,lineage:Option<Arc<Lineage>>)->Self{
  let(tx,rx)=mpsc::sync_channel(1);let cache=Arc::new(Mutex::new(Cache::default()));let detached=Arc::new(AtomicBool::new(false));let fresh_started=Arc::new(AtomicBool::new(false));
  let slot_epoch=Arc::new(std::sync::atomic::AtomicU64::new(0));let published=slot_epoch.clone();
  let(c,d,f)=(cache.clone(),detached.clone(),fresh_started.clone());let thread=std::thread::spawn(move||{
   let mut scope:Option<Scope>=None;let mut deadline:Option<Instant>=None;let mut current_world=None;let mut last_sample:Option<crate::connection_time::Stamp>=None;
   while let Ok(job)=rx.recv(){let staged_epoch=if let Job::Stage(_,epoch)=&job{*epoch}else{None};let result=(||->Result<View,String>{
    // Only this independent worker acquires the credential writer. No Core,
    // pending, native frame or lifecycle lock is passed into it.
    let _guard=writer.lock().unwrap();
    if let Job::Stage(preview,_)=job{
     if last_sample.is_some_and(|old|preview.sample.sequence<old.sequence||preview.sample.started<old.started||preview.sample.sequence==old.sequence&&preview.sample.started!=old.started){return Err("Stale/reordered provisional preview".into())}
     if scope.as_ref().is_some_and(|s|!same_issuer(s,&preview.scope)){return Err("Existing provisional worker issuer retained".into())}
     let before=read_head(&*store,&preview.scope)?;
     let mut record=if f.load(Ordering::Acquire){
      // A queued read can reach the writer after the native offer freezes B.
      // Ignore that replacement snapshot and retain the offered original.
      preview.validate()?;
      before.as_ref().filter(|h|Some(h.record.world)==current_world).ok_or("Offered entry original absent")?.record.clone()
     }else{stage_with(&*store,&preview,true)?};
     // An offer may start while an off-lock refresh is reading protected data.
     // Keep that offered immutable original if the new read would replace it.
     if f.load(Ordering::Acquire)&&current_world.is_some_and(|world|world!=record.world){let old=before.filter(|h|Some(h.record.world)==current_world).ok_or("Offered entry original was replaced")?;save_head(&*store,&old)?;record=old.record;}
     scope=Some(preview.scope.clone());
     let newer=last_sample.is_none_or(|old|preview.sample.sequence>old.sequence);
     // Only a new authenticated read may refresh an unused local snapshot.
     // An offered/claimed attempt keeps its original deadline and cold budget.
     deadline=Some(if current_world==Some(record.world)&&(!newer||f.load(Ordering::Acquire)||record.owner.is_some()){deadline.map_or(preview.until,|old|old.min(preview.until))}else{preview.until});last_sample=Some(preview.sample);current_world=Some(record.world);
     if d.load(Ordering::Acquire){record.detach();let mut head=read_head(&*store,&record.scope)?.ok_or("Provisional head absent")?;head.record=record.clone();save_head(&*store,&head)?;}
     return View::of(&record)
    }
    if let Job::Recover{scope:lookup,world}=job{let record=load(&*store,&lookup,world)?.ok_or("Saved provisional original absent")?;scope=Some(lookup);deadline=None;current_world=Some(world);return View::of(&record)}
    let scope=scope.as_ref().ok_or("Provisional preview must be reconciled before claim")?;
    let mut head=read_head(&*store,scope)?;let world=current_world.ok_or("Provisional selected world absent")?;
    let mut record=if let Some(current)=head.as_ref().filter(|h|h.record.world==world){current.record.clone()}else{load(&*store,scope,world)?.ok_or("Provisional original absent")?};
    match job{
     Job::Claim{world,pid,start}=>{if record.world!=world||d.load(Ordering::Acquire)||deadline.is_none_or(|at|Instant::now()>=at){return Err("Provisional identity expired/detached".into())}record.claim(pid,start)?;},
     Job::Activation{world,bytes,actor}=>{if world!=record.world||d.load(Ordering::Acquire){return Err("Provisional activation retired".into())}record.original(bytes.to_vec(),false,&actor)?;},
     Job::Detach=>record.detach(),
     Job::Cancel{world,bytes,actor}=>{if world!=record.world{return Err("Provisional cancellation identity differs".into())}record.original(bytes.to_vec(),true,&actor)?;},
     Job::CancelAck(response)=>record.confirm_cancel(&response)?,
     Job::ActivationAck(response,actual)=>record.confirm_activation(&response,&actual)?,
     Job::Current(response,actual,request,cancel)=>record.confirm_current(&response,&actual,&request,cancel)?,
     Job::Stage(..)|Job::Recover{..}=>unreachable!(),
    }
    if d.load(Ordering::Acquire)&&record.phase!=Phase::Cancelled{record.detach();}
    if let Some(head)=head.as_mut().filter(|h|h.record.world==world){head.record=record.clone();save_head(&*store,head)?}
    else{let bytes=zeroize::Zeroizing::new(serde_json::to_vec(&record).map_err(|_|"Provisional archive update encoding")?);crate::protected_chunks::save(&*store,&record.scope,PURPOSE,world,&bytes)?;}
    View::of(&record)
   })();let mut state=c.lock().unwrap();state.busy=false;match result{Ok(mut view)=>{view.until=deadline;if d.load(Ordering::Acquire)&&view.phase!=Phase::Cancelled{view.phase=Phase::Detached;}if let Some(epoch)=staged_epoch{published.store(epoch,Ordering::Release)}state.view=Some(view);state.error=None;},Err(error)=>state.error=Some(error)}
   }
  });Self{tx:Some(tx),thread:Some(thread),cache,detached,fresh_started,lineage,slot_epoch}
 }
 fn send(&self,job:Job)->Result<(),String>{let mut state=self.cache.lock().unwrap();if state.busy{return Err("Provisional worker busy; original retained".into())}state.busy=true;if self.tx.as_ref().unwrap().try_send(job).is_err(){state.busy=false;return Err("Provisional worker capacity".into())}Ok(())}
 pub fn stage(&self,preview:Preview)->Result<(),String>{let epoch=self.lineage.as_ref().map(|g|g.snapshot().ok_or("Local base transition is not settled")).transpose()?;self.stage_at(preview,epoch)}
 pub fn stage_at(&self,preview:Preview,epoch:Option<u64>)->Result<(),String>{if self.detached.load(Ordering::Acquire){return Err("Detached provisional worker cannot silently rearm".into())}if self.lineage.as_ref().is_some_and(|g|epoch.is_none_or(|e|!g.current(e))){return Err("Replacement preview local lineage retired".into())}self.send(Job::Stage(preview,epoch))}
 pub fn claim(&self,world:uuid::Uuid,pid:crate::local_owner::Id,process_start:u64)->Result<(),String>{self.send(Job::Claim{world,pid,start:process_start})}
 pub fn begin_fresh(&self,world:uuid::Uuid)->Result<(),String>{let mut lineage=self.lineage.as_ref().map(|g|g.0.lock().unwrap());if lineage.as_ref().is_some_and(|s|!s.ready||s.epoch!=self.slot_epoch.load(Ordering::Acquire)||s.offered.is_some_and(|w|w!=world)){return Err("Fresh entry base transition is not settled".into())}let cache=self.cache.lock().unwrap();if cache.error.is_some()||cache.view.as_ref().is_none_or(|v|v.world!=world||v.phase!=Phase::Staged||v.until.is_none_or(|at|at<=Instant::now())){return Err("Fresh provisional preview unavailable".into())}self.fresh_started.store(true,Ordering::Release);if let Some(s)=lineage.as_mut(){s.offered=Some(world)}Ok(())}
 pub fn fresh_started(&self)->bool{self.fresh_started.load(Ordering::Acquire)}
 pub fn recover_selector(&self,scope:Scope,world:uuid::Uuid)->Result<(),String>{self.send(Job::Recover{scope,world})}
 /// Caller must validate the REVIEWED canonical envelope before this byte seal.
 /// This method does not send it or fabricate a PendingIntent.
 pub fn seal_activation(&self,world:uuid::Uuid,bytes:Vec<u8>,actor:String)->Result<(),String>{self.send(Job::Activation{world,bytes:zeroize::Zeroizing::new(bytes),actor})}
 pub fn detach(&self)->Result<(),String>{self.detached.store(true,Ordering::Release);let mut state=self.cache.lock().unwrap();if let Some(view)=&mut state.view{view.retirement_pending=matches!(view.phase,Phase::ActivationSubmitted|Phase::ActivationReconciled|Phase::CancelSubmitted);view.phase=Phase::Detached;}state.busy=true;drop(state);match self.tx.as_ref().unwrap().try_send(Job::Detach){Ok(())=>Ok(()),Err(mpsc::TrySendError::Full(_))=>Ok(()),Err(_)=>Err("Provisional detach worker unavailable; local world remains detached".into())}}
 pub fn seal_cancel(&self,world:uuid::Uuid,bytes:Vec<u8>,actor:String)->Result<(),String>{self.send(Job::Cancel{world,bytes:zeroize::Zeroizing::new(bytes),actor})}
 pub fn confirm_cancel(&self,response:w::Response)->Result<(),String>{self.send(Job::CancelAck(SealedResponse(response)))}
 pub fn confirm_activation(&self,response:w::Response,actual:String)->Result<(),String>{self.send(Job::ActivationAck(SealedResponse(response),actual))}
 pub fn confirm_current(&self,response:w::Response,actual:String,request:w::Request,cancel:bool)->Result<(),String>{self.send(Job::Current(SealedResponse(response),actual,SealedRequest(request),cancel))}
 pub fn view(&self)->(bool,Option<View>,Option<String>){let (busy,mut view,error,epoch)={let state=self.cache.lock().unwrap();(state.busy,state.view.clone(),state.error.clone(),self.slot_epoch.load(Ordering::Acquire))};if !self.fresh_started()&&view.as_ref().is_some_and(|v|v.phase==Phase::Staged)&&self.lineage.as_ref().is_some_and(|g|!g.current(epoch)){view=None}(busy,view,error)}
}
impl Drop for Worker{fn drop(&mut self){self.tx.take();if let Some(thread)=self.thread.take(){let _=thread.join();}}}
fn random_secret()->Result<String,String>{
 let mut bytes=zeroize::Zeroizing::new([0u8;32]);
 #[cfg(windows)]{#[link(name="bcrypt")]unsafe extern "system"{fn BCryptGenRandom(a:*mut std::ffi::c_void,b:*mut u8,n:u32,f:u32)->i32;}if unsafe{BCryptGenRandom(std::ptr::null_mut(),bytes.as_mut_ptr(),32,2)}<0{return Err("Provisional CSPRNG unavailable".into())}}
 #[cfg(not(windows))]return Err("Provisional source candidate requires Windows CSPRNG".into());
 use base64::Engine;Ok(base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&*bytes))
}
#[cfg(test)]#[path="provisional_local_test.rs"]mod tests;
