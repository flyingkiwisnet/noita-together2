//! Local same-world recovery intent. No service eligibility or release is inferred here.
use crate::secrets::{CredentialStore,SecretBlob,ServiceProfile,StoreError};
use serde::{Deserialize,Serialize};
use sha2::{Digest,Sha256};
use std::time::{Duration,Instant};
#[derive(Clone,Debug,PartialEq,Eq,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Association {
 pub version:u8,pub local_world:uuid::Uuid,pub origin:String,pub service:String,
 pub run:String,pub participant:String,pub room:String,pub world_token:String,
 pub account:String,pub original_start:String,pub config_revision:u32,pub actual_seed:String,pub seed_policy:String,pub loot_policy:String,
 pub frozen_config_digest:String,
}
impl Association {
 pub fn validate(&self)->Result<(),String>{
  if self.version!=1||self.local_world.is_nil()||[&self.run,&self.participant,&self.room,&self.account,&self.original_start].iter().any(|id|id.is_empty()||id.len()>128||!id.bytes().all(|b|b.is_ascii_alphanumeric()||b==b'-'||b==b'_'))||self.config_revision==0||self.world_token.len()!=32||!self.world_token.bytes().all(|b|b.is_ascii_hexdigit()&&!b.is_ascii_uppercase())||self.world_token.bytes().all(|b|b==b'0'){return Err("Invalid saved-world association".into())}
  ServiceProfile::new(&self.origin,&self.service,self.local_world).map_err(|_|"Invalid saved issuer")?;
  let seed=self.actual_seed.parse::<u64>().map_err(|_|"Invalid actual saved seed")?;
  if seed.to_string()!=self.actual_seed||self.frozen_config_digest.len()!=64||!self.frozen_config_digest.bytes().all(|b|b.is_ascii_hexdigit()&&!b.is_ascii_uppercase()){return Err("Invalid saved world identity".into())}
  let loot=if self.loot_policy=="individual:0"{true}else{
   self.loot_policy.strip_prefix("shared:").and_then(|v|v.split_once(':')).is_some_and(|(bit,s)|{
    matches!(bit,"0"|"1")&&s.parse::<u32>().is_ok_and(|n|n.to_string()==s&&(bit=="0"||n>0))&&s==self.actual_seed
   })};
  if !loot||!(if self.loot_policy=="individual:0"{self.seed_policy=="individual-random"}else{matches!(self.seed_policy.as_str(),"shared-fixed"|"shared-random")}){return Err("Saved loot policy differs from actual world".into())}Ok(())
 }
 pub fn encode(&self)->Result<Vec<u8>,String>{self.validate()?;let b=serde_json::to_vec(self).map_err(|_|"Invalid association")?;if b.len()>1024{return Err("Saved association exceeds bound".into())}Ok(b)}
 pub fn decode(b:&[u8])->Result<Self,String>{if b.len()>1024{return Err("Saved association exceeds bound".into())}let a:Self=serde_json::from_slice(b).map_err(|_|"Malformed saved association")?;a.validate()?;Ok(a)}
 fn store_profile(&self)->Result<ServiceProfile,StoreError>{
  self.participation_profile(b"nt2-local-resume-v1\0")
 }
 fn retirement_profile(&self)->Result<ServiceProfile,StoreError>{self.participation_profile(b"nt2-local-retired-v1\0")}
 fn offline_proof_profile(&self)->Result<ServiceProfile,StoreError>{self.participation_profile(b"nt2-local-offline-proof-v1\0")}
 fn participation_profile(&self,purpose:&[u8])->Result<ServiceProfile,StoreError>{
  let mut digest=Sha256::new();digest.update(purpose);
  for field in [&self.run,&self.participant,&self.world_token]{digest.update((field.len() as u32).to_le_bytes());digest.update(field.as_bytes());}
  let hash=digest.finalize();let id=uuid::Uuid::from_bytes(hash[..16].try_into().unwrap());ServiceProfile::new(&self.origin,&self.service,id)
 }
 pub fn same_participation(&self,other:&Self)->bool{self.origin==other.origin&&self.service==other.service&&self.run==other.run&&self.participant==other.participant&&self.world_token==other.world_token}
 pub fn matches_canonical(&self,a:&crate::protocol_rejoin::wire::Association)->bool{
  a.service_id.as_deref()==Some(self.service.as_str())&&a.room_id.as_deref()==Some(self.room.to_string().as_str())
   &&a.run_id.as_deref()==Some(self.run.to_string().as_str())&&a.participant_id.as_deref()==Some(self.participant.to_string().as_str())
   &&a.world_token.as_deref()==Some(self.world_token.as_str())&&a.account_id.as_deref()==Some(self.account.to_string().as_str())
   &&a.original_start_id.as_deref()==Some(self.original_start.to_string().as_str())&&a.config_revision==Some(self.config_revision)
   &&a.actual_seed.as_deref()==Some(self.actual_seed.as_str())&&a.randomize_loot==Some(self.loot_policy.starts_with("shared:1:"))
   &&a.seed_policy.as_deref()==Some(self.seed_policy.as_str())
 }
}
/// Opaque canonical resume proof is protected in Rust, never serialized to Lua/UI.
pub fn remember<S:CredentialStore>(store:&S,a:&Association,proof:&SecretBlob)->Result<(),StoreError>{
 a.validate().map_err(|_|StoreError::InvalidSecret)?;
 if retirement_state(store,a)?.is_some(){return Err(StoreError::InvalidSecret)}
 if let Some(previous)=store.load(&a.store_profile()?)?{
  let (stored,retired,_)=stored_record(previous.bytes())?;
  if retired||stored!=*a{return Err(StoreError::InvalidSecret)}
 }
 if proof.bytes().len()!=43||!proof.bytes().iter().all(|b|b.is_ascii_alphanumeric()||*b==b'-'||*b==b'_'){return Err(StoreError::InvalidSecret)}
 let association=a.encode().map_err(|_|StoreError::InvalidSecret)?;
 let mut payload=Vec::with_capacity(5+association.len()+proof.bytes().len());
 payload.extend_from_slice(&(association.len() as u32).to_le_bytes());payload.push(0);payload.extend_from_slice(&association);payload.extend_from_slice(proof.bytes());
 let sealed=SecretBlob::new(payload)?;
 store.save(&a.store_profile()?,&sealed)?;
 // A concurrent retirement uses another immutable key; active writes cannot
 // clear it. Remove this stale proof if retirement won during the save.
 if retirement_state(store,a)?.is_some(){store.delete(&a.store_profile()?)?;return Err(StoreError::InvalidSecret)}Ok(())
}
fn retirement_state<S:CredentialStore>(store:&S,a:&Association)->Result<Option<u8>,StoreError>{
 let Some(marker)=store.load(&a.retirement_profile()?)? else{return Ok(None)};
 match marker.bytes(){[state @ (1|2)]=>Ok(Some(*state)),_=>Err(StoreError::Corrupt)}
}
fn stored_record(bytes:&[u8])->Result<(Association,bool,&[u8]),StoreError>{
 if bytes.len()<6{return Err(StoreError::Corrupt)}
 let n=u32::from_le_bytes(bytes[..4].try_into().unwrap()) as usize;
 if n>1024||5+n>=bytes.len()||bytes[4]>1{return Err(StoreError::Corrupt)}
 let stored=Association::decode(&bytes[5..5+n]).map_err(|_|StoreError::Corrupt)?;
 Ok((stored,bytes[4]==1,&bytes[5+n..]))
}
pub fn recall<S:CredentialStore>(store:&S,a:&Association)->Result<Option<SecretBlob>,StoreError>{
 recall_proof(store,a,false)
}
pub fn is_retired<S:CredentialStore>(store:&S,a:&Association)->Result<bool,StoreError>{retirement_state(store,a).map(|v|v.is_some())}
/// Retired proof may finish only the original offline RPC; never resume.
pub fn retired_proof<S:CredentialStore>(store:&S,a:&Association)->Result<Option<SecretBlob>,StoreError>{recall_proof(store,a,true)}
fn recall_proof<S:CredentialStore>(store:&S,a:&Association,retired:bool)->Result<Option<SecretBlob>,StoreError>{
 a.validate().map_err(|_|StoreError::InvalidSecret)?;
 let terminal=retirement_state(store,a)?;
 if (retired&&terminal!=Some(1))||(!retired&&terminal.is_some()){return Ok(None)}
 let profile=if retired{a.offline_proof_profile()?}else{a.store_profile()?};
 let Some(blob)=store.load(&profile)? else{return Ok(None)};
 let (stored,record_retired,proof)=stored_record(blob.bytes())?;
 if stored!=*a{return Err(StoreError::Corrupt)}
 if record_retired&&!retired{return Ok(None)}
 Ok(Some(SecretBlob::new(proof.to_vec())?))
}
pub fn retire_local<S:CredentialStore>(store:&S,a:&Association)->Result<(),StoreError>{
 a.validate().map_err(|_|StoreError::InvalidSecret)?;
 if retirement_state(store,a)?.is_some(){return Ok(())}
 let blob=store.load(&a.store_profile()?)?;
 if let Some(blob)=&blob{let(stored,_,_)=stored_record(blob.bytes())?;if stored!=*a{return Err(StoreError::Corrupt)}}
 // Preserve the original proof separately before publishing retirement.
 // A racing active capture cannot overwrite the offline RPC's proof.
 if let Some(blob)=&blob{store.save(&a.offline_proof_profile()?,blob)?;}
 // Separate canonical guard survives process reload, local UUID replacement
 // and proof deletion after the offline RPC is acknowledged.
 store.save(&a.retirement_profile()?,&SecretBlob::new(vec![1])?)?;
 if let Some(blob)=blob{let mut bytes=blob.bytes().to_vec();bytes[4]=1;store.save(&a.store_profile()?,&SecretBlob::new(bytes)?)?}Ok(())
}
/// Finish proof deletion after canonical retirement; never delete the guard.
pub fn forget<S:CredentialStore>(store:&S,a:&Association)->Result<(),StoreError>{
 a.validate().map_err(|_|StoreError::InvalidSecret)?;
 store.save(&a.retirement_profile()?,&SecretBlob::new(vec![2])?)?;
 store.delete(&a.store_profile()?)?;store.delete(&a.offline_proof_profile()?)
}
pub fn resume_request(a:&Association,proof:&SecretBlob,service_epoch:&str,session_epoch:&str,request_id:&str,instance:uuid::Uuid)->Result<Vec<u8>,String>{
 a.validate()?;let proof=std::str::from_utf8(proof.bytes()).map_err(|_|"Invalid protected proof")?;
 if proof.len()!=43||!proof.bytes().all(|b|b.is_ascii_alphanumeric()||b==b'-'||b==b'_')||instance.is_nil()||[service_epoch,session_epoch,request_id].iter().any(|s|s.is_empty()||s.len()>128){return Err("Invalid resume identity".into())}
 let request=crate::protocol_rejoin::wire::Request{major:Some(1),revision:Some(crate::protocol_rejoin::REVISION.into()),service_id:Some(a.service.clone()),service_epoch:Some(service_epoch.into()),session_epoch:Some(session_epoch.into()),request_id:Some(request_id.into()),room_id:Some(a.room.to_string()),run_id:Some(a.run.to_string()),participant_id:Some(a.participant.to_string()),world_token:Some(a.world_token.to_string()),config_revision:Some(a.config_revision),resume:Some(crate::protocol_rejoin::wire::Resume{resume_proof:Some(proof.into()),game_instance_id:Some(instance.to_string()),game_version:Some("Noita-Jan25-2025".into()),mod_version:Some("nt2-g3-c2-dev-1".into()),bridge_version:Some("g3-c2-ipc-1".into()),capabilities:Some(vec!["core.seed".into(),"core.ghost".into(),"core.hud".into(),"core.poi".into(),"core.seed-policy.v1".into(),"core.randomize-loot.v1".into()])}),..Default::default()};
 let mut request=request;let encoded=serde_json::to_vec(&request).map_err(|_|"Resume encoding failed".into());
 use zeroize::Zeroize;if let Some(secret)=request.resume.as_mut().and_then(|r|r.resume_proof.as_mut()){secret.zeroize()};encoded
}
pub fn validate_admission(a:&Association,response:&crate::protocol_rejoin::wire::Response,epoch:&str,request_id:&str,desktop_id:&str,device_id:&str,previous_generation:u32)->Result<u32,String>{
 if response.major!=Some(1)||response.revision.as_deref()!=Some(crate::protocol_rejoin::REVISION)||response.service_id.as_deref()!=Some(&a.service)||response.service_epoch.as_deref()!=Some(epoch)||response.request_id.as_deref()!=Some(request_id)||response.error.is_some(){return Err("Stale or refused resume admission".into())}
 let p=response.participant.as_ref().ok_or("Missing resumed participant")?;
 let generation=p.generation.filter(|g|*g>previous_generation).ok_or("Resume ownership generation is stale")?;
 if !p.association.as_ref().is_some_and(|v|a.matches_canonical(v))||p.desktop_id.as_deref()!=Some(desktop_id)||p.return_eligible!=Some(true)||p.state.as_deref()!=Some("inspecting")||p.service_epoch.as_deref()!=Some(epoch)||p.run_epoch.as_deref().is_none_or(str::is_empty)||p.attachment_id.as_deref().is_none_or(str::is_empty)||p.game_session_id.as_deref().is_none_or(str::is_empty){return Err("Resume association/owner differs from this saved world".into())}
 let c=response.credentials.as_ref().ok_or("Missing fresh resumed credentials")?;
 if c.account_id.as_deref()!=Some(a.account.to_string().as_str())||c.device_id.as_deref()!=Some(device_id)||c.session_epoch.as_deref().is_none_or(str::is_empty)||c.attachment_id!=p.attachment_id||c.session_id!=p.game_session_id||c.access_token.as_deref().is_none_or(str::is_empty){return Err("Fresh resumed credentials are not bound to owner".into())}Ok(generation)
}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Phase { Offline, Choice, WaitingForWorld, Reconnecting, Inspecting, Playing, Terminal }
#[derive(Clone,Default)]
pub struct ResumeIntent {association:Option<Association>,phase:Option<Phase>,generation:u64,failures:u32,next:Option<Instant>,operation:Option<zeroize::Zeroizing<Vec<u8>>>,terminal_reason:Option<String>}
impl ResumeIntent {
 pub fn phase(&self)->Phase{self.phase.unwrap_or(Phase::Offline)}
 pub fn association(&self)->Option<&Association>{self.association.as_ref()}
 pub fn generation(&self)->u64{self.generation}
 pub fn validate_selected(&self,a:&Association)->Result<(),String>{
  a.validate()?;self.generation.checked_add(1).ok_or("Resume generation exhausted")?;
  if matches!(self.phase(),Phase::Terminal|Phase::Offline)&&self.association.as_ref().is_some_and(|current|current.same_participation(a)){return Err("Terminal participation cannot be revived by a late status".into())}Ok(())
 }
 pub fn track_selected_playing(&mut self,a:Association)->Result<(),String>{self.validate_selected(&a)?;self.generation+=1;self.association=Some(a);self.phase=Some(Phase::Playing);self.operation=None;self.terminal_reason=None;Ok(())}
 pub fn view(&self)->serde_json::Value{serde_json::json!({"state":format!("{:?}",self.phase()).to_lowercase(),"terminal_reason":self.terminal_reason,"can_rejoin":self.phase()==Phase::Choice,"can_offline":self.association.is_some()&&self.phase()!=Phase::Offline,"intent_retained":matches!(self.phase(),Phase::WaitingForWorld|Phase::Reconnecting|Phase::Inspecting),"held":matches!(self.phase(),Phase::Choice|Phase::WaitingForWorld|Phase::Reconnecting|Phase::Inspecting|Phase::Terminal)})}
 pub fn offer(&mut self,a:Association,protected_proof_present:bool)->Result<(),String>{
  a.validate()?;if self.phase()==Phase::Terminal||(self.phase()==Phase::Offline&&self.association.as_ref().is_some_and(|current|current.same_participation(&a))){return Err("Participation ended; use a fresh authorized world".into())}
  if !protected_proof_present{return Err("Saved world has no protected local resume proof".into())}
  if self.association.as_ref()==Some(&a)&&matches!(self.phase(),Phase::WaitingForWorld|Phase::Reconnecting|Phase::Inspecting){return Ok(())}
  if self.association.as_ref().is_some_and(|current|current!=&a)&&matches!(self.phase(),Phase::WaitingForWorld|Phase::Reconnecting|Phase::Inspecting|Phase::Playing){return Err("Another world owns the retained resume intent".into())}
  self.generation=self.generation.checked_add(1).ok_or("Resume generation exhausted")?;self.association=Some(a);self.phase=Some(Phase::Choice);self.operation=None;Ok(())
 }
 pub fn choose_rejoin(&mut self)->Result<u64,String>{
  if !matches!(self.phase(),Phase::Choice|Phase::WaitingForWorld|Phase::Reconnecting){return Err("No eligible saved world choice".into())}
  if self.phase()!=Phase::Choice{return Ok(self.generation)}
  self.phase=Some(Phase::WaitingForWorld);self.next=None;Ok(self.generation)
 }
 pub fn world_loaded(&mut self,a:&Association)->Result<(),String>{
  if self.association.as_ref()!=Some(a)||!matches!(self.phase(),Phase::WaitingForWorld|Phase::Reconnecting){return Err("Unrelated save or missing rejoin choice".into())}
  self.phase=Some(Phase::Reconnecting);Ok(())
 }
 pub fn bind_operation(&mut self,g:u64,bytes:Vec<u8>)->Result<(),String>{
  if g!=self.generation||self.phase()!=Phase::Reconnecting||bytes.is_empty()||bytes.len()>8192{return Err("Invalid resume operation scope".into())}
  if self.operation.as_ref().is_some_and(|original|original.as_slice()!=bytes.as_slice()){return Err("Ambiguous resume must retain original bytes".into())}
  self.operation=Some(zeroize::Zeroizing::new(bytes));Ok(())
 }
 pub fn original_operation(&self)->Option<&[u8]>{self.operation.as_ref().map(|b|b.as_slice())}
 /// Only an authenticated canonical status may prove an old successful
 /// generation is no longer owned, permitting a fresh resume request.
 pub fn reconcile_lost_generation(&mut self)->Result<(),String>{
  if !matches!(self.phase(),Phase::Reconnecting|Phase::Inspecting|Phase::Playing){return Err("Terminal recovery cannot be reconciled".into())}
  self.operation=None;self.phase=Some(Phase::Reconnecting);self.next=None;Ok(())
 }
 pub fn transient_loss(&mut self,now:Instant){if matches!(self.phase(),Phase::Playing|Phase::Inspecting|Phase::Reconnecting|Phase::WaitingForWorld){self.phase=Some(Phase::Reconnecting);self.failures=self.failures.saturating_add(1);let cap=(500u64.saturating_mul(1u64<<self.failures.min(5))).min(10000);let random=u16::from_le_bytes(uuid::Uuid::new_v4().as_bytes()[..2].try_into().unwrap()) as u64;self.next=Some(now+Duration::from_millis(cap/2+random%(cap/2+1)));}}
 pub fn retry_due(&self,now:Instant)->bool{self.phase()==Phase::Reconnecting&&self.next.is_none_or(|n|now>=n)}
 pub fn admitted(&mut self,g:u64)->Result<(),String>{if g!=self.generation||self.phase()!=Phase::Reconnecting{return Err("Stale resume admission".into())}self.phase=Some(Phase::Inspecting);Ok(())}
 pub fn release(&mut self,g:u64,current_authenticated_inspection:bool)->Result<(),String>{if g!=self.generation||self.phase()!=Phase::Inspecting||!current_authenticated_inspection{return Err("Current protected inspection required".into())}self.phase=Some(Phase::Playing);self.operation=None;self.failures=0;Ok(())}
 pub fn terminal(&mut self){
  let offline=self.phase()==Phase::Offline&&self.association.is_some();
  self.generation=self.generation.saturating_add(1);self.phase=Some(if offline{Phase::Offline}else{Phase::Terminal});self.next=None;self.operation=None;
  if !offline{self.terminal_reason=Some("Shared participation ended; rejoin is unavailable".into())}
 }
 pub fn offline(&mut self){self.terminal();self.phase=Some(Phase::Offline);self.terminal_reason=None;}
}
/// Preflight all fallible intent checks before any protected-store mutation.
pub fn remember_selected<S:CredentialStore>(store:&S,intent:&mut ResumeIntent,a:Association,proof:&SecretBlob)->Result<Vec<u8>,String>{
 intent.validate_selected(&a)?;let bytes=a.encode()?;
 remember(store,&a,proof).map_err(|e|e.to_string())?;intent.track_selected_playing(a)?;Ok(bytes)
}
#[cfg(test)]#[path="rejoin_local_test.rs"]mod tests;
