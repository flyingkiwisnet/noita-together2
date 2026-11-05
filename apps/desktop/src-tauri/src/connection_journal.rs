//! Bounded two-bank protected parking journal. Chunk writes precede the sole
//! manifest commit. Incomplete writes never replace the active original bytes.
use crate::secrets::{CredentialStore,SecretBlob,ServiceProfile,StoreError};
use serde::{Deserialize,Serialize};use sha2::{Digest,Sha256};use zeroize::Zeroizing;
const RECORDS:usize=16;const CHUNK:usize=2048;const BYTES:usize=65536;
#[derive(Serialize,Deserialize)]#[serde(deny_unknown_fields)]struct Entry {bytes:usize,digest:String}
#[derive(Serialize,Deserialize)]#[serde(deny_unknown_fields)]struct Manifest {version:u32,bank:u8,entries:Vec<Entry>}
fn manifest<S:CredentialStore>(store:&S,profile:&ServiceProfile)->Result<Option<Manifest>,StoreError>{
 let Some(blob)=store.load(profile)? else{return Ok(None)};let m:Manifest=serde_json::from_slice(blob.bytes()).map_err(|_|StoreError::Corrupt)?;
 if m.version!=1||m.bank>1||m.entries.len()>RECORDS||m.entries.iter().any(|e|e.bytes==0||e.bytes>BYTES||e.digest.len()!=64||!e.digest.bytes().all(|b|b.is_ascii_hexdigit()&&!b.is_ascii_uppercase())){return Err(StoreError::Corrupt);}Ok(Some(m))
}
fn chunk_profile(profile:&ServiceProfile,bank:u8,record:usize,index:usize)->Result<ServiceProfile,StoreError>{
 let digest=Sha256::digest(serde_json::to_vec(&("nt2-parked-chunk-v1",profile.profile_id(),bank,record,index)).map_err(|_|StoreError::Corrupt)?);
 ServiceProfile::new(profile.origin(),profile.service_id(),uuid::Uuid::from_bytes(digest[..16].try_into().unwrap()))
}
pub fn load<S:CredentialStore>(store:&S,profile:&ServiceProfile)->Result<Vec<Zeroizing<Vec<u8>>>,StoreError>{
 let Some(m)=manifest(store,profile)? else{return Ok(vec![])};let mut out=vec![];
 for (record,e) in m.entries.iter().enumerate(){
  let mut bytes=Zeroizing::new(Vec::with_capacity(e.bytes));
  for index in 0..e.bytes.div_ceil(CHUNK){let part=store.load(&chunk_profile(profile,m.bank,record,index)?)?.ok_or(StoreError::Corrupt)?;let expected=(e.bytes-index*CHUNK).min(CHUNK);if part.bytes().len()!=expected{return Err(StoreError::Corrupt);}bytes.extend_from_slice(part.bytes());}
  if format!("{:x}",Sha256::digest(bytes.as_slice()))!=e.digest{return Err(StoreError::Corrupt);}out.push(bytes);
 }Ok(out)
}
pub fn save<S:CredentialStore>(store:&S,profile:&ServiceProfile,records:&[Zeroizing<Vec<u8>>])->Result<(),StoreError>{
 if records.len()>RECORDS||records.iter().any(|v|v.is_empty()||v.len()>BYTES){return Err(StoreError::InvalidSecret);}
 let bank=manifest(store,profile)?.map(|m|1-m.bank).unwrap_or(0);let mut entries=vec![];
 for (record,bytes) in records.iter().enumerate(){
  for (index,part) in bytes.chunks(CHUNK).enumerate(){store.save(&chunk_profile(profile,bank,record,index)?,&SecretBlob::new(part.to_vec())?)?;}
  entries.push(Entry{bytes:bytes.len(),digest:format!("{:x}",Sha256::digest(bytes.as_slice()))});
 }
 // CredWrite/our store abstraction replaces one value atomically. The two fixed
 // banks cap storage at1024 chunks per issuer journal even across crashes.
 let manifest=Zeroizing::new(serde_json::to_vec(&Manifest{version:1,bank,entries}).map_err(|_|StoreError::Corrupt)?);store.save(profile,&SecretBlob::new(manifest.to_vec())?)
}
#[cfg(test)]mod tests{
 use super::*;use std::{collections::BTreeMap,sync::Mutex};
 #[derive(Default)]struct Store {values:Mutex<BTreeMap<(String,String,uuid::Uuid),Vec<u8>>>,fail_after:Mutex<Option<usize>>}
 impl CredentialStore for Store{
  fn save(&self,p:&ServiceProfile,b:&SecretBlob)->Result<(),StoreError>{let mut f=self.fail_after.lock().unwrap();if let Some(n)=f.as_mut(){if *n==0{return Err(StoreError::Unavailable)}*n-=1;}self.values.lock().unwrap().insert((p.origin().into(),p.service_id().into(),p.profile_id()),b.bytes().to_vec());Ok(())}
  fn load(&self,p:&ServiceProfile)->Result<Option<SecretBlob>,StoreError>{self.values.lock().unwrap().get(&(p.origin().into(),p.service_id().into(),p.profile_id())).map(|v|SecretBlob::new(v.clone())).transpose()}
  fn delete(&self,p:&ServiceProfile)->Result<(),StoreError>{self.values.lock().unwrap().remove(&(p.origin().into(),p.service_id().into(),p.profile_id()));Ok(())}
 }
 #[test]fn cr2_protected_original_restart_and_partial_write_preserves_commit(){
  let s=Store::default();let p=ServiceProfile::new("https://original.invalid","service",uuid::Uuid::new_v4()).unwrap();let old=Zeroizing::new(vec![7;9000]);save(&s,&p,&[old.clone()]).unwrap();assert_eq!(load(&s,&p).unwrap()[0].as_slice(),old.as_slice());
  *s.fail_after.lock().unwrap()=Some(2);assert!(save(&s,&p,&[Zeroizing::new(vec![9;12000])]).is_err());assert_eq!(load(&s,&p).unwrap()[0].as_slice(),old.as_slice());
  *s.fail_after.lock().unwrap()=None;for n in 0..10{save(&s,&p,&[Zeroizing::new(vec![n;12000])]).unwrap();assert_eq!(load(&s,&p).unwrap()[0][0],n);}assert!(s.values.lock().unwrap().len()<=13);
  let foreign=ServiceProfile::new("https://foreign.invalid","service",p.profile_id()).unwrap();assert!(load(&s,&foreign).unwrap().is_empty());
 }
 #[test]fn cr2_protected_journal_corrupt_chunk_is_not_an_original(){
  let s=Store::default();let p=ServiceProfile::new("https://original.invalid","service",uuid::Uuid::new_v4()).unwrap();save(&s,&p,&[Zeroizing::new(vec![1;5000])]).unwrap();let cp=chunk_profile(&p,0,0,0).unwrap();let key=(cp.origin().into(),cp.service_id().into(),cp.profile_id());s.values.lock().unwrap().get_mut(&key).unwrap()[0]=2;assert!(load(&s,&p).is_err());
 }
}
