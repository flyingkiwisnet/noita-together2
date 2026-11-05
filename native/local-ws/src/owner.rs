//! Local capability identity is not an OS PID or executable attestation.
use std::{collections::BTreeMap,sync::{Mutex,OnceLock},path::{Path,PathBuf}};
#[derive(Clone,Copy,Debug,PartialEq,Eq,PartialOrd,Ord,Hash)]
pub enum Id{Unbound,OsProcess(u32),Socket{app:[u8;16],native:[u8;16]}}
impl Default for Id{fn default()->Self{Self::Unbound}}
impl Id{pub const NONE:Self=Self::Unbound;pub fn os_process(pid:u32)->Self{if pid==0{Self::Unbound}else{Self::OsProcess(pid)}}}
impl Id{
 pub fn claim_prefix(self)->Vec<u8>{match self{Self::Unbound=>Vec::new(),Self::OsProcess(pid)=>pid.to_le_bytes().to_vec(),Self::Socket{app,native}=>{let mut b=b"NLO1".to_vec();b.extend_from_slice(&app);b.extend_from_slice(&native);b}}}
 pub fn witness(self,context:[u8;16])->Vec<u8>{let mut b=self.claim_prefix();b.extend_from_slice(&context);b}
}
impl From<u32> for Id{fn from(pid:u32)->Self{Self::os_process(pid)}}
impl From<crate::server::Peer> for Id{fn from(p:crate::server::Peer)->Self{Self::Socket{app:p.app,native:p.native}}}
impl PartialEq<u32> for Id{fn eq(&self,p:&u32)->bool{match self{Self::Unbound=>*p==0,Self::OsProcess(pid)=>pid==p,Self::Socket{..}=>false}}}
impl PartialEq<Id> for u32{fn eq(&self,p:&Id)->bool{p==self}}
impl std::fmt::Display for Id{fn fmt(&self,f:&mut std::fmt::Formatter<'_>)->std::fmt::Result{match self{Self::Unbound=>f.write_str("unbound"),Self::OsProcess(pid)=>write!(f,"process:{pid}"),Self::Socket{app,native}=>write!(f,"local:{}:{}",crate::protocol::hex(app),crate::protocol::hex(native))}}}
impl serde::Serialize for Id{fn serialize<S:serde::Serializer>(&self,s:S)->Result<S::Ok,S::Error>{match self{Self::OsProcess(pid)=>s.serialize_u32(*pid),_=>s.serialize_str(&self.to_string())}}}
impl<'de> serde::Deserialize<'de> for Id{fn deserialize<D:serde::Deserializer<'de>>(d:D)->Result<Self,D::Error>{
 let value=serde_json::Value::deserialize(d)?;
 if let Some(pid)=value.as_u64(){return u32::try_from(pid).map(Self::os_process).map_err(serde::de::Error::custom)}
 let s=value.as_str().ok_or_else(||serde::de::Error::custom("Invalid local owner"))?;if s=="unbound"{return Ok(Self::Unbound)}
 let mut parts=s.split(':');if parts.next()!=Some("local"){return Err(serde::de::Error::custom("Invalid local owner"))}let app=parts.next().ok_or_else(||serde::de::Error::custom("Invalid local owner"))?;let native=parts.next().ok_or_else(||serde::de::Error::custom("Invalid local owner"))?;if parts.next().is_some(){return Err(serde::de::Error::custom("Invalid local owner"))}Ok(Self::Socket{app:crate::protocol::unhex(app).map_err(serde::de::Error::custom)?,native:crate::protocol::unhex(native).map_err(serde::de::Error::custom)?})
}}
struct Current{generation:u64,incarnation:u64,alive:bool,game:PathBuf}
#[derive(Default)]struct Registry{entries:BTreeMap<Id,Current>,serial:u64}
static REGISTRY:OnceLock<Mutex<Registry>>=OnceLock::new();
fn registry()->&'static Mutex<Registry>{REGISTRY.get_or_init(Default::default)}
pub fn connected(peer:crate::server::Peer,verified_game:&Path)->Result<Id,String>{
 let game=verified_game.canonicalize().map_err(|_|"Verified local game unavailable")?;let id=Id::from(peer);let mut r=registry().lock().unwrap();
 if r.entries.get(&id).is_some_and(|v|v.alive){return Err("Local game already attached".into())}
 if r.entries.len()>=64&&!r.entries.contains_key(&id){let retired=r.entries.iter().filter(|(_,v)|!v.alive).min_by_key(|(_,v)|v.incarnation).map(|(id,_)|*id).ok_or("Local owner registry bound")?;r.entries.remove(&retired);}
 let incarnation=if let Some(current)=r.entries.get(&id){if current.game!=game{return Err("Local installation changed".into())}current.incarnation}else{r.serial=r.serial.checked_add(1).ok_or("Local owner generation exhausted")?;r.serial};
 r.entries.insert(id,Current{generation:peer.generation,incarnation,alive:true,game});Ok(id)
}
pub fn disconnected(peer:crate::server::Peer){if let Some(current)=registry().lock().unwrap().entries.get_mut(&Id::from(peer)){if current.generation==peer.generation{current.alive=false;}}}
pub fn retire_app(app:[u8;16]){registry().lock().unwrap().entries.retain(|id,_|!matches!(id,Id::Socket{app:a,..}if *a==app));}
pub fn current(peer:crate::server::Peer)->bool{registry().lock().unwrap().entries.get(&Id::from(peer)).is_some_and(|v|v.alive&&v.generation==peer.generation)}
pub fn instance(id:Id)->Option<u64>{registry().lock().unwrap().entries.get(&id).filter(|v|v.alive).map(|v|v.incarnation)}
pub fn matches_installation(id:Id,game:&Path)->bool{let Ok(game)=game.canonicalize()else{return false};registry().lock().unwrap().entries.get(&id).is_some_and(|v|v.alive&&v.game==game)}
