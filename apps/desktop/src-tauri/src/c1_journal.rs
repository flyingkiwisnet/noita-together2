//! World-scoped ordinary-event receipts; no bank reservation or escrow phases.
use serde::{Deserialize,Serialize};
use serde_json::Value;
use std::{collections::BTreeMap,fs::File,io::Write,path::{Path,PathBuf}};
#[derive(Clone,Debug,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Record{pub ordinal:u64,pub source:Option<Value>,pub lobby:crate::g3_journal::LobbyIdentity,pub world:String,pub id:String,pub sequence:String,pub state:String,pub reason:String,pub original:Value}
pub struct Journal{path:PathBuf,file:Option<File>,_owner:File,rows:BTreeMap<(String,String,String),Record>,restored_pickups:std::collections::BTreeSet<(String,String,String)>,pub restored:bool,receipt_limit:usize,receipts:BTreeMap<(String,String,String),Record>}
impl Journal{
 pub fn open(path:&Path)->Result<Self,String>{Self::open_with_receipts(path,0)}
 /// Race retains a bounded exact-original receipt window; ordinary C1 keeps its existing compaction.
 pub fn open_with_receipts(path:&Path,receipt_limit:usize)->Result<Self,String>{
  // Reuse the storage ownership/reparse checks used by existing local journals.
  if path.exists(){crate::g3_journal::plain_path(path)?;}else{crate::g3_journal::plain_path(path.parent().ok_or("C1 storage parent")?)?;}
  let owner=crate::g3_journal::journal_owner(path)?;
  let file=crate::g3_journal::owned_file(path,false)?;
  if file.metadata().map_err(|_|"C1 storage metadata")?.len()>16*1024*1024{return Err("C1 storage oversized".into())}
  let bytes=std::fs::read(path).map_err(|_|"C1 storage read failed")?;
  if bytes.len()>16*1024*1024||!bytes.is_empty()&&bytes.last()!=Some(&b'\n'){return Err("C1 storage truncated or oversized".into())}
  let mut rows=BTreeMap::new();let mut receipts=BTreeMap::new();
  for line in bytes.split(|b|*b==b'\n').filter(|b|!b.is_empty()){
   let mut row:Record=serde_json::from_slice(line).map_err(|_|"Invalid C1 receipt")?;
   if row.sequence=="0"&&row.state!="acknowledged"{row.reason="uncertain".into();}
   else if matches!(row.state.as_str(),"applying"|"applied"){row.state="uncertain".into();row.reason="ambiguous_restore".into();}
   let key=(row.world.clone(),row.id.clone(),row.sequence.clone());
   if row.state=="acknowledged"{rows.remove(&key);if receipt_limit>0{receipts.insert(key,row);}}else{rows.insert(key,row);}
  }
  let restored_pickups=rows.iter().filter(|(_,r)|r.sequence=="0"&&r.original.get("payload").and_then(|p|p.get("death")).is_none()).map(|(key,_)|key.clone()).collect();
  let mut journal=Self{path:path.into(),file:Some(file),_owner:owner,rows,restored_pickups,restored:!bytes.is_empty(),receipt_limit,receipts};journal.trim_receipts();Ok(journal)
 }
 fn trim_receipts(&mut self){while self.receipts.len()>self.receipt_limit{let key=self.receipts.iter().min_by_key(|(_,r)|r.ordinal).unwrap().0.clone();self.receipts.remove(&key);}}
 pub fn receipts(&self,world:&str)->Vec<Record>{let mut rows:Vec<_>=self.receipts.values().filter(|r|r.world==world).cloned().collect();rows.sort_by_key(|r|r.ordinal);rows}
 pub fn receipt(&self,world:&str,id:&str,sequence:&str)->Option<Record>{self.receipts.get(&(world.into(),id.into(),sequence.into())).cloned()}
 pub fn all_rows(&self)->Vec<Record>{let mut rows:Vec<_>=self.rows.values().cloned().collect();rows.sort_by_key(|r|r.ordinal);rows}
 pub fn rows(&self,world:&str)->Vec<Record>{let mut rows:Vec<_>=self.rows.values().filter(|r|r.world==world).cloned().collect();rows.sort_by_key(|r|r.ordinal);rows}
 pub fn get(&self,world:&str,id:&str,sequence:&str)->Option<Record>{self.rows.get(&(world.into(),id.into(),sequence.into())).cloned()}
 pub fn put(&mut self,mut row:Record)->Result<(),String>{
  let key=(row.world.clone(),row.id.clone(),row.sequence.clone());
  if let Some(old)=self.rows.get(&key).or_else(||self.receipts.get(&key)){if old.original!=row.original||old.lobby!=row.lobby||old.source!=row.source{return Err("C1 immutable original changed".into())}row.ordinal=old.ordinal;if old.state=="acknowledged"&&row.state!="acknowledged"{return Err("C1 original already acknowledged".into())}}
  else if self.rows.values().filter(|r|r.world==row.world).count()>=128{return Err("C1 current work capacity".into())}
  // A cold retained pickup cannot become confirmed from a stale native copy.
  // New session originals and a deliberately resumed own death remain distinct.
  if row.state=="pending"&&self.restored_pickups.contains(&key){row.reason="uncertain".into();}
  if row.ordinal==0{row.ordinal=self.rows.values().chain(self.receipts.values()).map(|r|r.ordinal).max().unwrap_or(0).checked_add(1).ok_or("C1 current ordering capacity")?;}
  let mut bytes=serde_json::to_vec(&row).map_err(|_|"C1 receipt encoding")?;bytes.push(b'\n');
  let write=self.file.as_mut().ok_or("C1 storage poisoned")?.write_all(&bytes).and_then(|_|self.file.as_mut().unwrap().sync_all());
  if write.is_err(){self.file.take();return Err("C1 receipt flush failed".into())}
  if row.state=="acknowledged"{self.rows.remove(&key);self.restored_pickups.remove(&key);if self.receipt_limit>0{self.receipts.insert(key,row);self.trim_receipts();}}else{self.rows.insert(key,row);}
  // Compact completed history, so ordinary hearts have no lifetime quota.
  if self.file.as_ref().ok_or("C1 storage poisoned")?.metadata().map_err(|_|"C1 storage metadata")?.len()>1024*1024{self.compact()?}
  Ok(())
 }
 pub fn retire_deleted(&mut self,deleted:&[crate::g3_journal::LobbyIdentity])->Result<(),String>{
  if self.rows.values().chain(self.receipts.values()).any(|r|deleted.contains(&r.lobby)){
   let before=self.rows.clone();let receipts_before=self.receipts.clone();self.receipts.retain(|_,r|!deleted.contains(&r.lobby));self.rows.retain(|_,r|!deleted.contains(&r.lobby));
   if let Err(error)=self.compact(){self.rows=before;self.receipts=receipts_before;return Err(error)}
  }Ok(())
 }
 fn compact(&mut self)->Result<(),String>{
  let temp=self.path.with_file_name(format!("c1-compact-{}.tmp",uuid::Uuid::new_v4()));
  let mut file=crate::g3_journal::owned_file(&temp,true)?;
  for row in self.rows.values().chain(self.receipts.values()){let mut b=serde_json::to_vec(row).map_err(|_|"C1 compact encoding")?;b.push(b'\n');file.write_all(&b).map_err(|_|"C1 compact failed")?;}
  file.sync_all().map_err(|_|"C1 compact flush failed")?;drop(file);
  drop(self.file.take());
  crate::g3_journal::replace_journal(&temp,&self.path)?;
  self.file=Some(crate::g3_journal::owned_file(&self.path,false)?);Ok(())
 }
}
#[cfg(test)]mod tests{
 use super::*;
 fn record(state:&str)->Record{Record{ordinal:0,source:None,lobby:crate::g3_journal::LobbyIdentity{service_origin:"https://fixture.invalid".into(),service_id:"service".into(),principal_id:"principal".into(),room_id:"room".into()},world:"a".repeat(64),id:"11111111-1111-4111-8111-111111111111".into(),sequence:"1".into(),state:state.into(),reason:String::new(),original:serde_json::json!({"payload":{"heart":{}}})}}
 #[test]fn c1_restored_pickup_cannot_be_promoted_by_stale_input_but_new_effects_and_resumed_death_can_complete(){
  let dir=std::env::temp_dir().join(format!("nt2-c1-journal-{}",uuid::Uuid::new_v4()));std::fs::create_dir(&dir).unwrap();let path=dir.join("c1.jsonl");
  let mut pickup=record("pending");pickup.sequence="0".into();pickup.reason="applied".into();
  let mut death=pickup.clone();death.id=uuid::Uuid::new_v4().to_string();death.original=serde_json::json!({"payload":{"death":{"state":"respawning","cause":"normal"}}});
  {let mut j=Journal::open(&path).unwrap();j.put(pickup.clone()).unwrap();j.put(death.clone()).unwrap();}
  {let mut j=Journal::open(&path).unwrap();assert_eq!(j.get(&pickup.world,&pickup.id,"0").unwrap().reason,"uncertain");j.put(pickup.clone()).unwrap();let retained=j.get(&pickup.world,&pickup.id,"0").unwrap();assert_eq!(retained.reason,"uncertain");assert_eq!(retained.original,pickup.original);
   j.put(death.clone()).unwrap();assert_eq!(j.get(&death.world,&death.id,"0").unwrap().reason,"applied","A newly completed known-unissued own death is not a restored pickup");
   let mut fresh=pickup.clone();fresh.id=uuid::Uuid::new_v4().to_string();fresh.reason="uncertain".into();j.put(fresh.clone()).unwrap();fresh.reason="applied".into();j.put(fresh.clone()).unwrap();assert_eq!(j.get(&fresh.world,&fresh.id,"0").unwrap().reason,"applied","A current-session completion keeps its own classification");j.compact().unwrap();
  }
  {let j=Journal::open(&path).unwrap();assert_eq!(j.get(&pickup.world,&pickup.id,"0").unwrap().reason,"uncertain");}
  std::fs::remove_dir_all(dir).unwrap();
 }
 #[test]fn cold_applying_and_applied_are_uncertain_but_pending_progresses(){
  for state in ["pending","applying","applied"]{
   let dir=std::env::temp_dir().join(format!("nt2-c1-journal-{}",uuid::Uuid::new_v4()));std::fs::create_dir(&dir).unwrap();let path=dir.join("c1.jsonl");
   {let mut j=Journal::open(&path).unwrap();j.put(record(state)).unwrap();assert!(Journal::open(&path).is_err());}
   {let j=Journal::open(&path).unwrap();assert_eq!(j.rows(&"a".repeat(64))[0].state,if state=="pending"{"pending"}else{"uncertain"});}
   std::fs::remove_file(path).unwrap();std::fs::remove_dir_all(dir).unwrap();
  }
 }
 #[test]fn immutable_payload_and_compaction_retain_only_unacknowledged_work(){
  let dir=std::env::temp_dir().join(format!("nt2-c1-journal-{}",uuid::Uuid::new_v4()));std::fs::create_dir(&dir).unwrap();let path=dir.join("c1.jsonl");
  {let mut j=Journal::open(&path).unwrap();j.put(record("pending")).unwrap();let mut changed=record("pending");changed.original=serde_json::json!({"payload":{"perk":{"perkId":"EXTRA_MONEY"}}});assert!(j.put(changed).is_err());j.compact().unwrap();assert_eq!(j.rows(&"a".repeat(64)).len(),1);j.put(record("acknowledged")).unwrap();j.compact().unwrap();}
  {let j=Journal::open(&path).unwrap();assert!(j.rows(&"a".repeat(64)).is_empty());}
  std::fs::remove_file(path).unwrap();std::fs::remove_dir_all(dir).unwrap();
 }
 #[test]fn capture_order_survives_cold_restore_and_deletion_is_explicit(){
  let dir=std::env::temp_dir().join(format!("nt2-c1-journal-{}",uuid::Uuid::new_v4()));std::fs::create_dir(&dir).unwrap();let path=dir.join("c1.jsonl");
  let mut first=record("pending");first.id="ffffffff-ffff-4fff-8fff-ffffffffffff".into();
  let mut second=record("pending");second.id="00000000-0000-4000-8000-000000000001".into();
  {let mut j=Journal::open(&path).unwrap();j.put(first.clone()).unwrap();j.put(second.clone()).unwrap();assert_eq!(j.rows(&first.world)[0].id,first.id);j.compact().unwrap();}
  {let mut j=Journal::open(&path).unwrap();assert_eq!(j.rows(&first.world)[0].id,first.id);j.retire_deleted(&[]).unwrap();assert_eq!(j.rows(&first.world).len(),2);j.retire_deleted(&[first.lobby.clone()]).unwrap();assert!(j.rows(&first.world).is_empty());}
  {let j=Journal::open(&path).unwrap();assert!(j.rows(&first.world).is_empty());}
  std::fs::remove_file(path).unwrap();std::fs::remove_dir_all(dir).unwrap();
 }
 #[test]fn bounded_race_receipts_survive_compaction_and_retire_with_the_lobby(){
  let dir=std::env::temp_dir().join(format!("nt2-race-receipt-window-{}",uuid::Uuid::new_v4()));std::fs::create_dir(&dir).unwrap();let path=dir.join("race.jsonl");let row=record("pending");
  {let mut j=Journal::open_with_receipts(&path,2).unwrap();for i in 1..=4{let mut r=row.clone();r.id=format!("event-{i}");r.sequence=i.to_string();j.put(r.clone()).unwrap();r.state="acknowledged".into();j.put(r).unwrap();}assert_eq!(j.receipts(&row.world).len(),2);assert_eq!(j.receipts(&row.world)[0].id,"event-3");assert!(j.rows(&row.world).is_empty());j.compact().unwrap();}
  {let mut j=Journal::open_with_receipts(&path,2).unwrap();assert_eq!(j.receipts(&row.world).len(),2);assert_eq!(j.receipts(&row.world)[1].id,"event-4");let mut reopen=j.receipts(&row.world)[0].clone();reopen.state="pending".into();assert!(j.put(reopen).is_err());j.retire_deleted(&[row.lobby.clone()]).unwrap();assert!(j.receipts(&row.world).is_empty());}
  {let j=Journal::open_with_receipts(&path,2).unwrap();assert!(j.receipts(&row.world).is_empty());}
  std::fs::remove_dir_all(dir).unwrap();
 }
 #[test]fn partial_receipt_is_refused_without_replay(){
  let dir=std::env::temp_dir().join(format!("nt2-c1-journal-{}",uuid::Uuid::new_v4()));std::fs::create_dir(&dir).unwrap();let path=dir.join("c1.jsonl");std::fs::write(&path,b"{\"world\":").unwrap();assert!(Journal::open(&path).is_err());assert_eq!(std::fs::read(&path).unwrap(),b"{\"world\":");std::fs::remove_file(path).unwrap();std::fs::remove_dir_all(dir).unwrap();
 }
}
