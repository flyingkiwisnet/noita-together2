//! The local identity/fence owner. Short synchronous transitions only: no I/O,
//! network or storage. Scoped worker credentials and canonical deadlines live
//! here; protected originals and actual native reports retain their own guards.
use super::*;
#[derive(Clone,Debug,PartialEq,Eq,Serialize,Deserialize)]
pub(super) struct Issuer {pub origin:String,pub service:String,pub account:String}
#[derive(Clone,Debug,PartialEq,Eq,Serialize,Deserialize)]
pub(super) struct StableWorld {pub issuer:Issuer,pub room:String,pub run:String,pub participant:String,pub world:String}
#[derive(Clone,Debug,PartialEq,Eq)]
pub(super) struct Owner {pub world:StableWorld,pub native:(crate::local_owner::Id,[u8;16]),pub session:String,pub generation:u64}
#[derive(Clone,Debug)]
pub(super) struct Ticket {revision:u64,owner:Owner}
enum Intent {Fresh(Issuer,String),Saved(StableWorld),Offline}
#[derive(Default)]
pub(super) struct Coordinator {revision:u64,owner:Option<Owner>,intent:Option<Intent>,pub(super) authority:admission::State,pub(super) credentials:std::collections::BTreeMap<String,admission::CredentialLease>,pub(super) request_sequence:u64,pub(super) service_time:Option<admission::ServiceTime>,pub(super) preparations:std::collections::BTreeMap<String,Instant>,pub(super) active:Option<health::Active>,pub(super) waiting_metadata:Option<nt2_desktop_g0::startup_local::WaitingMarker>,request_stamps:std::collections::BTreeMap<String,nt2_desktop_g0::connection_time::Stamp>}
impl Coordinator {
 pub(super) fn fresh_intent(&self)->bool{matches!(self.intent,Some(Intent::Fresh(..)))}
 pub(super) fn identity_changed(&mut self){self.retire_owner();self.intent=None;self.preparations.clear();}
 pub fn fresh(&mut self,issuer:Issuer,room:String){self.retire_owner();self.intent=Some(Intent::Fresh(issuer,room));}
 pub fn saved(&mut self,world:StableWorld){if matches!(&self.intent,Some(Intent::Saved(current)) if current==&world){return;}self.retire_owner();self.intent=Some(Intent::Saved(world));}
 pub fn offline(&mut self){self.retire_owner();self.intent=Some(Intent::Offline);}
 pub fn retire_owner(&mut self){self.revision=self.revision.checked_add(1).expect("local lifecycle revision exhausted");self.owner=None;self.authority=Default::default();self.active=None;self.waiting_metadata=None;}
 pub fn bind(&mut self,owner:Owner)->Option<Ticket>{
  let permitted=match &self.intent{Some(Intent::Fresh(issuer,room))=>owner.world.issuer==*issuer&&owner.world.room==*room,Some(Intent::Saved(world))=>owner.world.issuer==world.issuer&&owner.world.room==world.room&&owner.world.run==world.run&&owner.world.world==world.world&&(world.participant.is_empty()||owner.world.participant==world.participant),Some(Intent::Offline)=>false,None=>true};
  if !permitted{return None;}
  if let Some(current)=self.owner.as_mut().filter(|current|same_owner(current,&owner)){
   if !current.world.participant.is_empty()&&!owner.world.participant.is_empty()&&current.world.participant!=owner.world.participant{return None;}
   if current.world.participant.is_empty(){current.world.participant=owner.world.participant.clone();}
   return Some(Ticket{revision:self.revision,owner:current.clone()});
  }
  self.retire_owner();self.owner=Some(owner.clone());Some(Ticket{revision:self.revision,owner})
 }
 pub fn accepts(&self,t:&Ticket)->bool{self.revision==t.revision&&self.owner.as_ref().is_some_and(|owner|same_owner(owner,&t.owner))}
}
#[derive(Clone,Serialize,Deserialize)]
pub(super) struct OriginalScope {issuer:Issuer,world:StableWorld,service_epoch:String,session_id:String,session_epoch:String,request:String,digest:[u8;32]}
impl OriginalScope {
 pub fn capture(a:&nt2_desktop_g0::rejoin_local::Association,service_epoch:&str,session_id:&str,session_epoch:&str,request:&str,bytes:&[u8])->Self{
  let issuer=Issuer{origin:a.origin.clone(),service:a.service.clone(),account:a.account.clone()};
  Self{world:StableWorld{issuer:issuer.clone(),room:a.room.clone(),run:a.run.clone(),participant:a.participant.clone(),world:a.world_token.clone()},issuer,service_epoch:service_epoch.into(),session_id:session_id.into(),session_epoch:session_epoch.into(),request:request.into(),digest:Sha256::digest(bytes).into()}
 }
 pub fn same_auth(&self,service_epoch:&str,session_id:&str,session_epoch:&str)->bool{self.service_epoch==service_epoch&&self.session_id==session_id&&self.session_epoch==session_epoch}
 pub fn permits(&self,current:&Issuer,service_epoch:&str,session_id:&str,session_epoch:&str,request:&str,bytes:&[u8],a:&nt2_desktop_g0::rejoin_local::Association)->bool{
  self.issuer==*current&&self.session_id==session_id&&self.service_epoch==service_epoch&&self.session_epoch==session_epoch&&self.request==request&&self.digest==<[u8;32]>::from(Sha256::digest(bytes))&&self.world.run==a.run&&self.world.participant==a.participant&&self.world.world==a.world_token
 }
}
impl StableWorld {
 pub fn saved(a:&nt2_desktop_g0::rejoin_local::Association)->Self{Self{issuer:Issuer{origin:a.origin.clone(),service:a.service.clone(),account:a.account.clone()},room:a.room.clone(),run:a.run.clone(),participant:a.participant.clone(),world:a.world_token.clone()}}
}
impl<S:CredentialStore> Core<S>{
 pub(super) fn lifecycle_issuer(&self)->Option<Issuer>{Some(Issuer{origin:self.base.clone(),service:self.service.clone(),account:self.desktop.as_ref()?.account.clone()})}
 pub(super) fn lifecycle_owner_ticket(&self)->Option<Ticket>{
  let game=self.game.as_ref()?;let p=self.pending.lock().unwrap().clone()?;
  if p.owner!=Some(game.owner)||!self.native_authority_valid(&p){return None;}
  let issuer=self.lifecycle_issuer()?;
  let stable=self.resume_intent.association().filter(|a|a.origin==issuer.origin&&a.service==issuer.service&&a.account==issuer.account&&a.run==p.run&&(a.world_token==hex(&game.owner.1)||self.rejoin_transport.contact.as_ref().is_some_and(|c|c.pid==game.owner.0&&c.context==game.owner.1&&c.association==*a.local_world.as_bytes())));
  let owner=Owner{world:StableWorld{issuer,room:self.room.as_ref()?.room_id.clone()?,run:p.run,participant:stable.map(|a|a.participant.clone()).unwrap_or_default(),world:self.waiting_world().map(String::from).or_else(||stable.map(|a|a.world_token.clone())).unwrap_or_else(||hex(&game.owner.1))},native:game.owner,session:game.session.id.clone(),generation:self.generation.load(Ordering::Acquire)};
  self.lifecycle.lock().unwrap().bind(owner)
 }
}

#[cfg(test)]mod tests{
 use super::*;
 #[test]fn cr1_stable_world_survives_owner_replacement_but_old_results_do_not(){
  let issuer=Issuer{origin:"https://old.invalid".into(),service:"service".into(),account:"account".into()};let world=StableWorld{issuer:issuer.clone(),room:"room".into(),run:"run".into(),participant:"participant".into(),world:"stable-world".into()};
  let mut c=Coordinator::default();c.saved(world.clone());let original=Owner{world:world.clone(),native:(nt2_local_ws::owner::Id::OsProcess(1),[1;16]),session:"old-session".into(),generation:1};let old=c.bind(original.clone()).unwrap();assert!(c.accepts(&old));c.saved(world.clone());assert!(c.accepts(&old),"repeated same-world choice must be idempotent");
  let mut replacement=original.clone();replacement.native=(nt2_local_ws::owner::Id::OsProcess(2),[2;16]);replacement.session="new-session".into();replacement.generation=2;let new=c.bind(replacement).unwrap();assert!(!c.accepts(&old));assert!(c.accepts(&new));
  let mut foreign=original.clone();foreign.world.issuer.account="other-account".into();assert!(c.bind(foreign).is_none());assert!(c.accepts(&new));c.offline();assert!(c.bind(original.clone()).is_none());assert!(!c.accepts(&new));c.fresh(issuer,"another-room".into());assert!(c.bind(original).is_none());
 }
}

impl OriginalScope {
 pub(super) fn operation(&self,a:&nt2_desktop_g0::rejoin_local::Association,device:uuid::Uuid,kind:&str,bytes:&[u8])->Result<nt2_desktop_g0::protocol_connection::wire::OriginalOperation,String>{
  if self.issuer.origin!=a.origin||self.issuer.service!=a.service||self.issuer.account!=a.account||self.world.run!=a.run||self.world.participant!=a.participant||self.world.world!=a.world_token||self.digest!=<[u8;32]>::from(Sha256::digest(bytes)){return Err("Parked original scope/fingerprint changed".into());}
  let r:nt2_desktop_g0::protocol_rejoin::wire::Request=serde_json::from_slice(bytes).map_err(|_|"Invalid parked original")?;
  if r.request_id.as_deref()!=Some(&self.request)||r.session_epoch.as_deref()!=Some(&self.session_epoch)||r.service_epoch.as_deref()!=Some(&self.service_epoch)||r.service_id.as_deref()!=Some(&a.service)||r.room_id.as_deref()!=Some(&a.room)||r.run_id.as_deref()!=Some(&a.run)||r.participant_id.as_deref()!=Some(&a.participant)||r.world_token.as_deref()!=Some(&a.world_token)||r.config_revision!=Some(a.config_revision)||r.generation.is_none_or(|g|g==0)||!matches!((kind,r.resume.is_some(),r.offline.is_some()),("resume",true,false)|("offline",false,true)){return Err("Parked original authentication/world/kind changed".into());}
  Ok(nt2_desktop_g0::protocol_connection::wire::OriginalOperation{request_id:Some(self.request.clone()),kind:Some(kind.into()),fingerprint:Some(nt2_desktop_g0::protocol_connection::canonical_fingerprint(bytes)?),original_session_id:Some(self.session_id.clone()),original_session_epoch:Some(self.session_epoch.clone()),original_owner_generation:r.generation,participant_id:Some(a.participant.clone()),room_id:Some(a.room.clone()),run_id:Some(a.run.clone()),world_token:Some(a.world_token.clone()),config_revision:Some(a.config_revision),original_service_epoch:Some(self.service_epoch.clone()),account_id:Some(a.account.clone()),device_id:Some(device.to_string())})
 }
}

impl Coordinator {
 pub(super) fn request_stamp(&mut self,scope:&str,id:&str)->Result<nt2_desktop_g0::connection_time::Stamp,String>{
  let key=json!([scope,id]).to_string();if let Some(stamp)=self.request_stamps.get(&key){return Ok(*stamp);}
  self.request_stamps.retain(|_,s|s.started.elapsed()<Duration::from_secs(180));if self.request_stamps.len()>=512{return Err("Bounded request timing capacity; retry later".into());}
  self.request_sequence=self.request_sequence.checked_add(1).ok_or("Request sequence exhausted")?;let stamp=nt2_desktop_g0::connection_time::Stamp{sequence:self.request_sequence,started:Instant::now()};self.request_stamps.insert(key,stamp);Ok(stamp)
 }
}

fn same_owner(a:&Owner,b:&Owner)->bool{a.native==b.native&&a.session==b.session&&a.generation==b.generation&&a.world.issuer==b.world.issuer&&a.world.room==b.world.room&&a.world.run==b.world.run&&a.world.world==b.world.world}

impl PartialEq for Ticket{fn eq(&self,other:&Self)->bool{self.revision==other.revision&&same_owner(&self.owner,&other.owner)}}
impl Eq for Ticket{}
