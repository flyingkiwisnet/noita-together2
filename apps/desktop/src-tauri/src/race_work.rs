//! Race originals are durable before copy ACK. This lane never grants gameplay.
use crate::{protocol_race::{self as p,wire},c1_journal::{Journal,Record},g3_social_stream::Exchange};
use serde::Deserialize;
use serde_json::{json,Value};
use std::{collections::VecDeque,sync::{Arc,Mutex},time::{Duration,Instant}};
#[derive(Clone,Deserialize)]#[serde(deny_unknown_fields)]
pub struct Input{kind:String,pub world:String,pub context:String,pub commit:wire::Commit}
fn decimal(v:&str)->bool{
 let mut parts=v.split('.');let integer=parts.next().unwrap_or("");let fraction=parts.next();
 !integer.is_empty()&&(integer=="0"||!integer.starts_with('0'))&&integer.bytes().all(|b|b.is_ascii_digit())&&parts.next().is_none()&&fraction.is_none_or(|f|!f.is_empty()&&!f.ends_with('0')&&f.bytes().all(|b|b.is_ascii_digit()))&&v.parse::<f64>().is_ok_and(|n|n.is_finite()&&n>=0.0&&n<=2147483647.0)
}
impl Input{pub fn decode(bytes:&[u8])->Option<Self>{
 if bytes.len()>4096{return None}let v:Self=serde_json::from_slice(bytes).ok()?;
 if v.kind!="race_completion"||!crate::debug_diagnostics::hex(&v.world,64)||!crate::debug_diagnostics::hex(&v.context,32){return None}
 p::typed::<wire::Commit>("Commit",&serde_json::to_vec(&v.commit).ok()?).ok()?;
 uuid::Uuid::parse_str(v.commit.event_id.as_deref()?).ok()?;
 if v.commit.sequence?==0||!matches!(v.commit.terminal.as_deref(),Some("none"|"dead"|"ending")){return None}
 let observations=v.commit.observations.as_ref()?;
 for o in observations{let valid=match o.kind.as_deref(){
  Some("orb")=>o.orb_id.is_some()&&o.boss_id.is_none()&&o.biome_id.is_none()&&o.direction.is_none()&&o.index.is_none()&&o.value.is_none()&&o.ending.is_none(),
  Some("boss")=>o.boss_id.is_some()&&o.orb_id.is_none()&&o.biome_id.is_none()&&o.direction.is_none()&&o.index.is_none()&&o.value.is_none()&&o.ending.is_none(),
  Some("biome")=>o.biome_id.is_some()&&o.orb_id.is_none()&&o.boss_id.is_none()&&o.direction.is_none()&&o.index.is_none()&&o.value.is_none()&&o.ending.is_none(),
  Some("parallel")=>matches!(o.direction.as_deref(),Some("east"|"west"))&&o.index.is_some_and(|i|i>0&&i<=10)&&o.orb_id.is_none()&&o.boss_id.is_none()&&o.biome_id.is_none()&&o.value.is_none()&&o.ending.is_none(),
  Some("max_hp"|"gold")=>o.value.as_ref().is_some_and(|v|decimal(v))&&o.orb_id.is_none()&&o.boss_id.is_none()&&o.biome_id.is_none()&&o.direction.is_none()&&o.index.is_none()&&o.ending.is_none(),
  Some("ending")=>matches!(o.ending.as_deref(),Some("classic"|"other"))&&v.commit.terminal.as_deref()==Some("ending")&&o.orb_id.is_none()&&o.boss_id.is_none()&&o.biome_id.is_none()&&o.direction.is_none()&&o.index.is_none()&&o.value.is_none(),
  _=>false};if !valid{return None}
 }Some(v)
}}
pub struct Context{pub local_world:Option<String>,pub owner:crate::g3_http::RecoveryOwner,pub journal:crate::g3_journal::SharedJournal,pub binding:crate::g3_local_consumer::Binding,pub generation:u32,pub locator:String,pub players:Value,pub lobby:Value,pub display_only:bool,pub recover_only:bool,pub read_results:bool}
pub fn world(b:&crate::g3_local_consumer::Binding)->String{use sha2::{Digest,Sha256};let s=&b.scope;Sha256::digest(serde_json::to_vec(&json!([b.service_origin,b.service_id,b.principal_id,s.room_id,s.run_id,s.run_epoch,s.world_token])).unwrap()).iter().map(|b|format!("{b:02x}")).collect()}
/// Presentation binding only: the protected local record and live Race scope
/// must identify the same origin/account/lobby/run before exporting its UUID.
pub fn local_world_for(id:&crate::world_local::Identity,world:uuid::Uuid,b:&crate::g3_local_consumer::Binding)->Option<String>{
 (id.origin==b.service_origin&&id.service==b.service_id&&id.account==b.principal_id&&b.scope.room_id.as_deref()==Some(&id.room)&&b.scope.run_id.as_deref()==Some(&id.run)&&b.scope.run_epoch.as_deref()==Some(&id.run_epoch)&&b.scope.config_revision.map(u64::from)==Some(id.config)).then(||world.to_string())
}
#[derive(Default)]struct State{lobby:Value,local_world:Option<String>,world:String,journal:Option<Journal>,journal_path:Option<std::path::PathBuf>,accepted:VecDeque<String>,sequence:u32,snapshot:Value,snapshot_next:Option<Instant>,receipt:Value,status:String,players:Value}
impl State{fn view(&self)->Value{
 let key=self.snapshot["selfRow"].as_str();let own=self.snapshot["entrants"].as_array().and_then(|rows|rows.iter().find(|r|r["rowKey"].as_str()==key));
 let newer=self.receipt["revision"].as_str().and_then(|v|v.parse::<u64>().ok()).unwrap_or(0)>self.snapshot["revision"].as_str().and_then(|v|v.parse::<u64>().ok()).unwrap_or(0);
 json!({"localWorld":self.local_world,"world":self.world,"initialized":true,"accepted":self.accepted,"acceptedSequence":self.sequence,"reservedSequence":self.journal.as_ref().and_then(|j|j.rows(&self.world).iter().filter_map(|r|r.original["sequence"].as_u64()).max()).unwrap_or(u64::from(self.sequence)),"queued":self.journal.as_ref().map(|j|j.rows(&self.world).len()).unwrap_or(0),"status":if newer{Some(&self.receipt["status"])}else{own.map(|v|&v["status"])},"snapshot":self.snapshot,"error":self.status})
}
fn native_page_count(&self)->usize{
 let base=[self.snapshot["config"]["goals"].as_array().map_or(0,Vec::len),self.snapshot["entrants"].as_array().map_or(0,Vec::len),self.players.as_array().map_or(0,Vec::len)].into_iter().map(|n|n.div_ceil(4)).max().unwrap_or(1).clamp(1,8);
 if self.lobby.is_object(){base.max(self.lobby["members"].as_array().map_or(0,Vec::len).div_ceil(4)).max(1)*2}else{base}
}
fn native_view(&self,page:usize)->Value{
 if self.lobby.is_object()&&page%2==1{let rows=self.lobby["members"].as_array().cloned().unwrap_or_default();let offset=((page/2)%rows.len().div_ceil(4).max(1))*4;return json!({"world":self.world,"lobbyOnly":true,"displayOnly":true,"lobbyScope":self.lobby["scope"],"lobbyRevision":self.lobby["revision"],"lobbyTotal":rows.len(),"lobbyOffset":offset,"lobbyMembers":rows.iter().skip(offset).take(4).collect::<Vec<_>>()})}
 let page=if self.lobby.is_object(){page/2}else{page};
 let mut v=self.view();v["displayOnly"]=json!(self.lobby.is_object());v["lobbyAvailable"]=json!(self.lobby.is_object());v["accepted"]=json!(self.accepted.iter().rev().take(8).collect::<Vec<_>>());
 let players=self.players.as_array().cloned().unwrap_or_default();v["players"]=json!(players.iter().skip((page%players.len().div_ceil(4).max(1))*4).take(4).map(|p|{let row=self.snapshot["entrants"].as_array().and_then(|rows|rows.iter().find(|r|r["rowKey"]==p["row"]));json!({"actor":p["actor"],"count":row.map(|r|&r["completedGoals"]).unwrap_or(&p["count"]),"status":row.map(|r|&r["status"]).unwrap_or(&p["status"])})}).collect::<Vec<_>>());
 if !self.snapshot.is_object(){return v}
 let goals=self.snapshot["config"]["goals"].as_array().cloned().unwrap_or_default();let pages=goals.len().div_ceil(4).max(1);let offset=(page%pages)*4;
 v["goalTotal"]=json!(goals.len());v["goalOffset"]=json!(offset);
 v["needsClassic"]=json!(goals.iter().any(|g|g["id"]=="classic_ending"));v["needsMap"]=json!(goals.iter().any(|g|g["id"]=="visit_parallel_world"));
 let selected:Vec<_>=goals.into_iter().skip(offset).take(4).collect();v["snapshot"]["config"]["goals"]=json!(selected);
 let entrants=self.snapshot["entrants"].as_array().cloned().unwrap_or_default();
 if self.local_world.is_some()&&self.snapshot["status"]=="ended"&&self.snapshot["endReason"]=="race_deadline"{
  if let Some(own)=entrants.iter().find(|r|r["rowKey"]==self.snapshot["selfRow"]){v["deadlineResult"]=json!({"completedGoals":own["completedGoals"],"goalTotal":v["goalTotal"],"rank":own["rank"],"status":own["status"]});}
 }

 v["standingsTotal"]=json!(entrants.len());v["standingsOffset"]=json!((page%entrants.len().div_ceil(4).max(1))*4);
 v["standings"]=json!(entrants.iter().skip((page%entrants.len().div_ceil(4).max(1))*4).take(4).map(|r|json!({"rowKey":r["rowKey"],"displayName":r["displayName"],"rank":r["rank"],"completedGoals":r["completedGoals"],"status":r["status"],"finalCountAtMs":r["finalCountAtMs"],"finishOrder":r["finishOrder"]})).collect::<Vec<_>>());
 let mut rows=Vec::new();if let Some(own)=self.snapshot["entrants"].as_array().and_then(|rows|rows.iter().find(|r|r["rowKey"]==self.snapshot["selfRow"])){
  let mut progress=Vec::new();for mut p in own["progress"].as_array().cloned().unwrap_or_default(){
   if !selected.iter().any(|g|g["instanceId"]==p["instanceId"]){continue}
   p["credited"]=json!(p["credits"].as_array().map(|rows|rows.iter().filter_map(|r|r["id"].as_str()).collect::<Vec<_>>()).unwrap_or_default());p.as_object_mut().unwrap().remove("credits");progress.push(p);
  }rows.push(json!({"rowKey":own["rowKey"],"status":own["status"],"progress":progress}));
 }v["snapshot"]["entrants"]=json!(rows);v
}}
#[derive(Default)]pub struct Work{state:State,display:Value,native_page:usize,task:Option<tokio::task::JoinHandle<State>>,next:Option<Instant>}
impl Work{
 pub fn view(&self)->Value{self.display.clone()}
 pub async fn shutdown(&mut self){if let Some(t)=self.task.take(){let _=t.await;}}
 pub async fn tick(&mut self,context:Option<Context>,stream:Arc<Mutex<Exchange>>){
  if self.task.as_ref().is_some_and(|t|t.is_finished()){if let Ok(s)=self.task.take().unwrap().await{self.state=s;self.display=self.state.view();}self.next=Some(Instant::now()+Duration::from_millis(if self.state.status.is_empty(){500}else{1500}));}
  let Some(c)=context else{return};if self.task.is_some(){return}
  let key=world(&c.binding);if !c.recover_only&&self.state.world!=key{self.state.world=key;self.state.local_world=None;self.state.accepted.clear();self.state.sequence=0;self.state.snapshot=Value::Null;self.state.players=Value::Null;self.state.receipt=Value::Null;self.state.snapshot_next=None;self.next=None;self.display=Value::Null;}
  self.state.lobby=c.lobby.clone();
  self.native_page=(self.native_page+1)%self.state.native_page_count();if !c.recover_only{stream.lock().unwrap().race_view_for(c.binding.context,self.state.native_view(self.native_page));}
  if self.next.is_some_and(|t|Instant::now()<t){return}
  let input=if c.recover_only||c.display_only{None}else{stream.lock().unwrap().race_input()};let mut state=std::mem::take(&mut self.state);let runtime=tokio::runtime::Handle::current();
  let page=self.native_page;self.task=Some(tokio::task::spawn_blocking(move||runtime.block_on(async move{let mut c=c;let result=step(&mut c,&mut state,input,&stream).await;state.status=result.err().unwrap_or_default();if !c.recover_only{stream.lock().unwrap().race_view_for(c.binding.context,state.native_view(page));}state})));
 }
}
// A Lua original may already be in the native copy lane when its service receipt
// reaches the app. Match durable exact receipts before considering another delivery.
fn retain_input(j:&mut Journal,world:&str,lobby:&crate::g3_journal::LobbyIdentity,source:Value,id:&str,sequence:&str,original:Value)->Result<(),String>{
 if j.rows(world).iter().chain(j.receipts(world).iter()).any(|r|r.sequence==sequence&&(r.id!=id||r.original!=original)){return Err("Race sequence original changed".into())}
 if let Some(old)=j.get(world,id,sequence).or_else(||j.receipt(world,id,sequence)){if old.original!=original||old.lobby!=*lobby{return Err("Race immutable original changed".into())}return Ok(())}
 j.put(Record{ordinal:0,source:Some(source),lobby:lobby.clone(),world:world.into(),id:id.into(),sequence:sequence.into(),state:"pending".into(),reason:"observed".into(),original})
}
fn remember_receipts(accepted:&mut VecDeque<String>,sequence:&mut u32,j:&Journal,world:&str){
 for row in j.receipts(world){if !accepted.contains(&row.id){accepted.push_back(row.id);}}
 while accepted.len()>64{accepted.pop_front();}
 for row in j.receipts(world){*sequence=(*sequence).max(row.original["sequence"].as_u64().unwrap_or(0)as u32);}
}
async fn step(c:&mut Context,s:&mut State,input:Option<(u64,u64,Input)>,stream:&Arc<Mutex<Exchange>>)->Result<(),String>{
 if c.display_only{return Ok(())} // Room membership display never reads/commits gameplay or drains originals.

 if !c.recover_only{s.local_world=c.local_world.clone();}
 let players:Vec<_>=c.players.as_array().map(|rows|rows.iter().filter(|r|r["raceRowKey"].is_string()).map(|r|json!({"actor":r["actor"],"row":r["raceRowKey"],"count":r["completedGoals"],"status":r["raceStatus"]})).collect()).unwrap_or_default();
 // Terminal actor retirement may clear live social work. Retain the previously
 // authenticated row mapping for this world so read-only results keep updating.
 if !players.is_empty(){s.players=json!(players);}
 let path=c.journal.c1_path().with_file_name("race-observations.jsonl");if s.journal_path.as_ref()!=Some(&path){s.journal=None;s.journal_path=Some(path.clone());}
 if s.journal.is_none(){s.journal=Some(Journal::open_with_receipts(&path,64)?)}
 let j=s.journal.as_mut().unwrap();remember_receipts(&mut s.accepted,&mut s.sequence,j,&s.world);j.retire_deleted(&c.journal.deleted_lobbies())?;
 let lobby=crate::g3_journal::LobbyIdentity{service_origin:c.binding.service_origin.clone(),service_id:c.binding.service_id.clone(),principal_id:c.binding.principal_id.clone(),room_id:c.binding.scope.room_id.clone().ok_or("Race room missing")?};
 if c.recover_only{
  if let Some(row)=j.all_rows().into_iter().find(|r|r.lobby==lobby){
   let source=row.source.as_ref().ok_or("Race retained source absent")?;
   c.binding.scope=serde_json::from_value(source["scope"].clone()).map_err(|_|"Race retained scope")?;
   c.generation=source["generation"].as_u64().and_then(|n|u32::try_from(n).ok()).filter(|n|*n>0).ok_or("Race retained generation")?;
   if world(&c.binding)!=row.world{return Err("Race retained world mismatch".into())}s.world=row.world;
  }else if !c.read_results{return Ok(())}
 }
 if let Some((serial,sequence,input))=input{
  if input.world!=s.world{return Err("Race world changed".into())}
  let id=input.commit.event_id.clone().unwrap();let sequence_key=format!("race:{}",input.commit.sequence.unwrap());let original=serde_json::to_value(input.commit).unwrap();
  retain_input(j,&s.world,&lobby,json!({"scope":c.binding.scope,"generation":c.generation}),&id,&sequence_key,original)?;
  remember_receipts(&mut s.accepted,&mut s.sequence,j,&s.world);
  if j.receipt(&s.world,&id,&sequence_key).is_some(){s.accepted.retain(|old|old!=&id);s.accepted.push_back(id);}
  // sync_all above succeeded. Delivery may finish after the actor disappears.
  stream.lock().unwrap().acknowledge_race(serial,sequence);
 }
 let delivery:Result<(),String>=async {if let Some(mut row)=j.rows(&s.world).into_iter().min_by_key(|r|r.original["sequence"].as_u64()){
  let scope=serde_json::from_value(serde_json::to_value(&c.binding.scope).unwrap()).map_err(|_|"Race retained scope")?;
  let original:wire::Commit=serde_json::from_value(row.original.clone()).map_err(|_|"Race retained original")?;
  let response=c.owner.race(&lobby,Some(scope),c.generation,Some(original.clone()),None).await?;
  let receipt=response.receipt.ok_or("Race receipt missing")?;
  if receipt.event_id!=original.event_id||receipt.sequence!=original.sequence{return Err("Race original receipt mismatch".into())}
  s.sequence=s.sequence.max(receipt.sequence.unwrap_or(0));s.receipt=serde_json::to_value(&receipt).unwrap();
  row.state="acknowledged".into();j.put(row)?;remember_receipts(&mut s.accepted,&mut s.sequence,j,&s.world);
 }Ok(())}.await;
 // A refused/uncertain original remains durable. It must not prevent the
 // read-only results from converging after End, expiry or an owner change.
 if !c.read_results||s.snapshot_next.is_some_and(|at|Instant::now()<at){return delivery}
 s.snapshot_next=Some(Instant::now()+Duration::from_secs(2));
 // Revision-fenced pages; restart the next scheduled read on concurrent changes.
 let known_run=if !c.recover_only{s.snapshot["runKey"].as_str().map(str::to_owned)}else{None};
 let mut snapshot:Option<wire::Snapshot>=None;let mut after=None;
 for _ in 0..4{
  let read=wire::Read{locator:Some(c.locator.clone()),limit:Some(8),self_row_requested:None,after_row:after,run_key:snapshot.as_ref().and_then(|s|s.run_key.clone()).or_else(||known_run.clone()),snapshot_revision:snapshot.as_ref().and_then(|s|s.revision.clone())};
  let page=c.owner.race(&lobby,None,c.generation,None,Some(read)).await?.snapshot.ok_or("Race snapshot absent")?;
  if known_run.as_ref().is_some_and(|run|page.run_key.as_ref()!=Some(run)){return Err("Race results run changed".into())}
  after=page.next_row.clone().filter(|s|!s.is_empty());
  if let Some(base)=&mut snapshot{if base.run_key!=page.run_key||base.revision!=page.revision{return Err("Race snapshot changed".into())}base.entrants.get_or_insert_default().extend(page.entrants.unwrap_or_default());if base.self_row.is_none(){base.self_row=page.self_row;}}
  else{snapshot=Some(page)}if after.is_none(){break}
 }
 let snapshot=serde_json::to_value(snapshot.ok_or("Race snapshot missing")?).unwrap();let key=snapshot["selfRow"].as_str();
 if let Some(own)=snapshot["entrants"].as_array().and_then(|rows|rows.iter().find(|r|r["rowKey"].as_str()==key)){s.sequence=s.sequence.max(own["acceptedSequence"].as_u64().unwrap_or(0)as u32);}
 s.snapshot=snapshot;delivery
}
#[cfg(test)]mod tests{
 use super::*;
 fn original()->Value{json!({"kind":"race_completion","world":"a".repeat(64),"context":"b".repeat(32),"commit":{"eventId":"11111111-1111-4111-8111-111111111111","sequence":1,"observations":[{"kind":"max_hp","value":"500"}],"terminal":"dead"}})}
 #[test]fn race_input_preserves_order_and_rejects_ambiguous_facts(){
  let v=original();let input=Input::decode(&serde_json::to_vec(&v).unwrap()).unwrap();assert_eq!(serde_json::to_value(input.commit).unwrap(),v["commit"]);
  for extra in ["accountId","worldToken","rank","finishOrder"]{let mut bad=v.clone();bad["commit"][extra]=json!("winner");assert!(Input::decode(&serde_json::to_vec(&bad).unwrap()).is_none());}
  for value in ["NaN","1e3","01","-1","1.0","2147483648"]{let mut bad=v.clone();bad["commit"]["observations"][0]["value"]=json!(value);assert!(Input::decode(&serde_json::to_vec(&bad).unwrap()).is_none());}
  let mut bad=v.clone();bad["commit"]["observations"][0]["orbId"]=json!(0);assert!(Input::decode(&serde_json::to_vec(&bad).unwrap()).is_none());
  let mut bad=v.clone();bad["commit"]["observations"]=json!([{"kind":"ending","ending":"classic"}]);assert!(Input::decode(&serde_json::to_vec(&bad).unwrap()).is_none());
  let mut empty=v;empty["commit"]["observations"]=json!([]);assert!(Input::decode(&serde_json::to_vec(&empty).unwrap()).is_some());
 }
 #[test]fn race_pending_original_survives_cold_restore_without_effect_replay(){
  let root=std::env::temp_dir().join(format!("nt2-race-journal-{}",uuid::Uuid::new_v4()));std::fs::create_dir(&root).unwrap();let path=root.join("race.jsonl");let original=original();
  let row=Record{ordinal:0,source:Some(json!({"roomId":"room","runId":"run","worldToken":"world"})),lobby:crate::g3_journal::LobbyIdentity{service_origin:"https://fixture.invalid".into(),service_id:"service".into(),principal_id:"account".into(),room_id:"room".into()},world:"a".repeat(64),id:original["commit"]["eventId"].as_str().unwrap().into(),sequence:"race:1".into(),state:"pending".into(),reason:"observed".into(),original:original["commit"].clone()};
  {let mut j=Journal::open(&path).unwrap();j.put(row.clone()).unwrap();let mut changed=row.clone();changed.original["terminal"]=json!("ending");assert!(j.put(changed).is_err());}
  {let mut j=Journal::open(&path).unwrap();let rows=j.rows(&row.world);assert_eq!(rows.len(),1);assert_eq!(rows[0].original,row.original);assert_eq!(rows[0].state,"pending");let mut ack=row.clone();ack.state="acknowledged".into();j.put(ack).unwrap();assert!(j.rows(&row.world).is_empty());}
  std::fs::remove_dir_all(root).unwrap();
 }
 #[test]fn race_delayed_original_is_not_requeued_after_receipt_or_cold_restore(){
  let root=std::env::temp_dir().join(format!("nt2-race-receipts-{}",uuid::Uuid::new_v4()));std::fs::create_dir(&root).unwrap();let path=root.join("race.jsonl");
  let lobby=crate::g3_journal::LobbyIdentity{service_origin:"https://fixture.invalid".into(),service_id:"service".into(),principal_id:"account".into(),room_id:"room".into()};let world="a".repeat(64);let mut v=original()["commit"].clone();v["sequence"]=json!(3);let id=v["eventId"].as_str().unwrap();let source=json!({"generation":1});
  {
   let mut j=Journal::open_with_receipts(&path,64).unwrap();retain_input(&mut j,&world,&lobby,source.clone(),id,"race:3",v.clone()).unwrap();let mut row=j.rows(&world).remove(0);row.state="acknowledged".into();j.put(row).unwrap();let size=std::fs::metadata(&path).unwrap().len();
   for _ in 0..20{retain_input(&mut j,&world,&lobby,source.clone(),id,"race:3",v.clone()).unwrap();assert!(j.rows(&world).is_empty(),"Delayed copy must not create another service delivery");}
   assert_eq!(std::fs::metadata(&path).unwrap().len(),size,"Duplicate copy does not append journal work");
   let mut changed=v.clone();changed["terminal"]=json!("ending");assert!(retain_input(&mut j,&world,&lobby,source.clone(),id,"race:3",changed).is_err());
   let mut changed=v.clone();changed["eventId"]=json!(uuid::Uuid::new_v4().to_string());assert!(retain_input(&mut j,&world,&lobby,source.clone(),changed["eventId"].as_str().unwrap(),"race:3",changed.clone()).is_err());
  }
  {
   let mut j=Journal::open_with_receipts(&path,64).unwrap();retain_input(&mut j,&world,&lobby,source.clone(),id,"race:3",v.clone()).unwrap();assert!(j.rows(&world).is_empty());let mut accepted=VecDeque::new();let mut sequence=0;remember_receipts(&mut accepted,&mut sequence,&j,&world);assert_eq!(accepted,VecDeque::from([id.to_owned()]));assert_eq!(sequence,3);
   // An unknown original below a known high-water must still reach the service.
   let mut unknown=v.clone();unknown["eventId"]=json!(uuid::Uuid::new_v4().to_string());unknown["sequence"]=json!(1);retain_input(&mut j,&world,&lobby,source,unknown["eventId"].as_str().unwrap(),"race:1",unknown.clone()).unwrap();assert_eq!(j.rows(&world).len(),1);
  }
  std::fs::remove_dir_all(root).unwrap();
 }
 #[test]fn race_native_pages_reach_two_account_lua_standings_and_compact_panel(){
  let repo=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");let root=repo.join("spikes/noita-entry/.build/race-owner-feedback-20261105").join(format!("standings-{}",uuid::Uuid::new_v4()));std::fs::create_dir_all(&root).unwrap();
  let goal=json!({"instanceId":"hp","id":"reach_max_hp","target":500});
  let row=|key:&str,rank:u32,count:u32,value:&str|json!({"rowKey":key,"displayName":"Same Name","rank":rank,"completedGoals":count,"status":"racing","finalCountAtMs":if count>0{"1234"}else{""},"finishOrder":0,"progress":[{"instanceId":"hp","value":value,"target":"500","complete":count>0,"completedAtMs":if count>0{"1234"}else{""},"credits":[]}]});
  let mut frames=String::new();
  for account in ["row-a","row-b"]{
   for (label,revision,rows) in [("initial","7",json!([row("row-a",1,0,"100"),row("row-b",1,0,"100")])),("update","8",json!([row("row-a",1,1,"500"),row("row-b",2,0,"100")]))]{
    let state=State{world:"a".repeat(64),snapshot:json!({"selfRow":account,"runKey":"canonical-run","revision":revision,"config":{"goals":[goal],"durationMs":0},"entrants":rows,"status":"open","elapsedMs":"1234"}),..Default::default()};
    for page in (0..8).rev(){let body=serde_json::to_string(&json!({"race":state.native_view(page)})).unwrap();assert!(body.len()<=8192);frames.push_str(&format!("{account}-{label}|{body}\n"));}
   }
  }
  // Two-page canonical roster in query order; the leader arrives on page1.
  for (label,revision,updated) in [("initial","7",false),("update","8",true)]{
   let rows:Vec<_>=(0..6).map(|n|json!({"rowKey":format!("row-{n}"),"displayName":format!("Player{n}"),"rank":if updated&&n!=5{2}else{1},"completedGoals":if updated&&n==5{1}else{0},"status":"racing","finalCountAtMs":"","finishOrder":0,"progress":[]})).collect();
   let state=State{world:"b".repeat(64),snapshot:json!({"selfRow":"row-0","runKey":"paced-run","revision":revision,"config":{"goals":[goal],"durationMs":0},"entrants":rows,"status":"open","elapsedMs":"1234"}),..Default::default()};
   assert_eq!(state.native_page_count(),2);
   for page in 0..2{frames.push_str(&format!("paced-{label}-{page}|{}\n",serde_json::to_string(&json!({"race":state.native_view(page)})).unwrap()));}
  }
  let input=root.join("frames.jsonl");std::fs::write(&input,frames).unwrap();
  let mut command=std::process::Command::new(repo.join("spikes/noita-entry/.build/Lua51Runner.exe"));command.arg("C:/Noita/lua51.dll").arg(repo.join("apps/desktop/tools/race-standings-test.lua")).env("NT2_RACE_REPO",&repo).env("NT2_RACE_VIEW_INPUT",&input);
  #[cfg(windows)]{use std::os::windows::process::CommandExt;command.creation_flags(0x08000000);}
  let output=command.output().unwrap();std::fs::write(root.join("lua-stdout.txt"),&output.stdout).unwrap();std::fs::write(root.join("lua-stderr.txt"),&output.stderr).unwrap();assert!(output.status.success(),"{}",String::from_utf8_lossy(&output.stderr));eprintln!("{}; evidence {}",String::from_utf8_lossy(&output.stdout),root.display());
 }
 #[test]fn race_native_cycles_only_needed_pages_without_losing_goal_or_actor_coverage(){
  for (roster,goals,actors,expected) in [(2,1,2,1),(6,1,0,2),(2,16,2,4),(2,1,32,8),(32,16,32,8)]{
   let state=State{snapshot:json!({"config":{"goals":vec![json!({});goals]},"entrants":vec![json!({});roster]}),players:json!(vec![json!({});actors]),..Default::default()};
   assert_eq!(state.native_page_count(),expected);
   let mut offsets=std::collections::BTreeSet::new();let mut goal_offsets=std::collections::BTreeSet::new();
   for page in 0..expected{let view=state.native_view(page);offsets.insert(view["standingsOffset"].as_u64().unwrap());goal_offsets.insert(view["goalOffset"].as_u64().unwrap());assert!(!view["standings"].as_array().unwrap().is_empty());}
   assert_eq!(offsets.len(),roster.div_ceil(4));assert_eq!(goal_offsets.len(),goals.div_ceil(4));
  }
 }
 #[test]fn race_deadline_serialization_and_main_protected_status_remain_scoped(){
  let repo=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");let root=repo.join("spikes/noita-entry/.build/race-deadline-status-20261105").join(format!("status-{}",uuid::Uuid::new_v4()));std::fs::create_dir_all(&root).unwrap();
  let local_world=uuid::Uuid::parse_str("11111111-1111-4111-8111-111111111111").unwrap();
  let id=crate::world_local::Identity{origin:"https://fixture.invalid".into(),service:"service".into(),account:"account".into(),device:uuid::Uuid::new_v4(),room:"room".into(),run:"run".into(),run_epoch:"epoch".into(),config:1,config_digest:"a".repeat(64)};
  let binding=crate::g3_local_consumer::Binding{context:[1;16],service_origin:id.origin.clone(),service_id:id.service.clone(),service_epoch:"service-epoch".into(),session_epoch:"session".into(),principal_id:id.account.clone(),scope:crate::protocol_g3::wire::Scope{room_id:Some(id.room.clone()),run_id:Some(id.run.clone()),run_epoch:Some(id.run_epoch.clone()),config_revision:u32::try_from(id.config).ok(),..Default::default()}};
  assert_eq!(local_world_for(&id,local_world,&binding),Some(local_world.to_string()));
  for field in ["room","run","epoch","account"]{let mut other=id.clone();match field{"room"=>other.room="other".into(),"run"=>other.run="other".into(),"epoch"=>other.run_epoch="other".into(),_=>other.account="other".into()};assert!(local_world_for(&other,local_world,&binding).is_none());}
  let mut frames=String::new();
  for (case,status,reason,world) in [("deadline","ended","race_deadline",Some(local_world.to_string())),("outage","running","",Some(local_world.to_string())),("hostEnd","ended","host_end",Some(local_world.to_string())),("otherworld","ended","race_deadline",Some(uuid::Uuid::new_v4().to_string())),("unbound","ended","race_deadline",None)]{
   let state=State{local_world:world,world:"b".repeat(64),snapshot:json!({"selfRow":"self","runKey":"canonical-run","revision":"8","status":status,"endReason":reason,"collectionState":"available","elapsedMs":"1000","deadlineAt":"2026-11-05T00:00:01Z","config":{"durationMs":1000,"goals":(0..8).map(|n|json!({"instanceId":n.to_string(),"id":"reach_max_hp","target":500})).collect::<Vec<_>>()},"entrants":[{"rowKey":"self","rank":2,"completedGoals":3,"status":"dnf","progress":[]}]}),..Default::default()};
   let body=serde_json::to_string(&json!({"race":state.native_view(0)})).unwrap();assert!(body.len()<=8192);if status!="ended"||reason!="race_deadline"||state.local_world.is_none(){assert!(state.native_view(0).get("deadlineResult").is_none());}frames.push_str(&format!("{case}|{body}\n"));
  }
  let input=root.join("frames.jsonl");std::fs::write(&input,frames).unwrap();let mut command=std::process::Command::new(repo.join("spikes/noita-entry/.build/Lua51Runner.exe"));command.arg("C:/Noita/lua51.dll").arg(repo.join("apps/desktop/tools/race-deadline-status-test.lua")).env("NT2_REJOIN_REPO",&repo).env("NT2_RACE_VIEW_INPUT",&input);
  #[cfg(windows)]{use std::os::windows::process::CommandExt;command.creation_flags(0x08000000);}
  let output=command.output().unwrap();std::fs::write(root.join("lua-stdout.txt"),&output.stdout).unwrap();std::fs::write(root.join("lua-stderr.txt"),&output.stderr).unwrap();assert!(output.status.success(),"{}",String::from_utf8_lossy(&output.stderr));eprintln!("{}; evidence {}",String::from_utf8_lossy(&output.stdout),root.display());
 }
 #[test]fn race_native_pages_fit_existing_mailbox_with_sixteen_goals_and_full_roster(){
  let catalogue:Value=serde_json::from_str(include_str!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../packages/contracts/g3-race-goals/catalogue.json"))).unwrap();let ids:Vec<_>=catalogue["bosses"].as_array().unwrap().iter().filter(|b|b["available"]==true).map(|b|b["id"].clone()).collect();
  let goals:Vec<_>=(0..16).map(|i|json!({"instanceId":format!("{i:064}"),"id":"defeat_named_bosses","bossIds":ids})).collect();
  let progress:Vec<_>=goals.iter().map(|g|json!({"instanceId":g["instanceId"],"value":"2147483647","currentValue":"2147483647","target":"2147483647","complete":true,"completedAtMs":"9007199254740991","credits":ids.iter().map(|id|json!({"id":id,"firstAtMs":"9007199254740991"})).collect::<Vec<_>>()})).collect();
  let players:Vec<_>=(0..32).map(|i|json!({"actor":format!("{i:0128}"),"row":i.to_string(),"count":16,"status":"finished"})).collect();
  let rows:Vec<_>=(0..32).map(|i|json!({"rowKey":format!("{i:036}"),"displayName":"n".repeat(396),"completedGoals":16,"acceptedSequence":65536,"rank":32,"finishOrder":32,"status":"finished","progress":progress})).collect();
  let mut state=State{local_world:Some("11111111-1111-4111-8111-111111111111".into()),world:"a".repeat(64),players:json!(players),snapshot:json!({"selfRow":format!("{:036}",0),"runKey":"a".repeat(64),"revision":"65536","config":{"registryRevision":"race-goals-1","durationMs":604800000,"goals":goals},"entrants":rows,"status":"ended","elapsedMs":"604800000","collectionState":"available","startedAt":"2026-11-05T00:00:00.000Z","deadlineAt":"2026-11-12T00:00:00.000Z","endedAt":"2026-11-12T00:00:00.000Z","endReason":"race_deadline"}),..Default::default()};
  for _ in 0..64{state.accepted.push_back(uuid::Uuid::new_v4().to_string());}
  let mut maximum=0;let mut seen=std::collections::BTreeSet::new();let mut standings=std::collections::BTreeSet::new();let mut actors=std::collections::BTreeSet::new();for page in 0..8{let v=state.native_view(page);let bytes=serde_json::to_vec(&json!({"race":v})).unwrap();maximum=maximum.max(bytes.len());assert!(bytes.len()<=8192,"Native Race page too large: {}",bytes.len());assert_eq!(v["snapshot"]["entrants"].as_array().unwrap().len(),1);assert_eq!(v["players"].as_array().unwrap().len(),4);for row in v["standings"].as_array().unwrap(){assert!(row.get("progress").is_none());standings.insert(row["rowKey"].as_str().unwrap().to_string());}for row in v["players"].as_array().unwrap(){actors.insert(row["actor"].as_str().unwrap().to_string());}for g in v["snapshot"]["config"]["goals"].as_array().unwrap(){seen.insert(g["instanceId"].as_str().unwrap().to_string());}}
  assert_eq!(seen.len(),16);assert_eq!(standings.len(),32);assert_eq!(actors.len(),32);eprintln!("Race maximum native page: {maximum}/8192 bytes, all32 summaries/actors and16 goals");
 }
}

#[cfg(test)]mod lobby_tests{use super::*;
 #[test]fn race_lobby_private_pages_stay_bounded_and_do_not_create_game_actors(){
  let members:Vec<_>=(0..32).map(|i|json!({"desktop":uuid::Uuid::from_u128(i+1).to_string(),"name":"🧙".repeat(99),"status":"waiting"})).collect();let state=State{world:"a".repeat(64),lobby:json!({"scope":"room/run/epoch/1","revision":"10","members":members}),..Default::default()};let mut seen=std::collections::BTreeSet::new();assert_eq!(state.native_page_count(),16);
  for page in (1..16).step_by(2){let v=state.native_view(page);assert_eq!(v["lobbyOnly"],true);assert!(serde_json::to_vec(&json!({"race":v})).unwrap().len()<=8192);for row in v["lobbyMembers"].as_array().unwrap(){seen.insert(row["desktop"].as_str().unwrap().to_owned());assert!(row.get("actor").is_none()&&row.get("hp").is_none()&&row.get("worldToken").is_none());}}
  assert_eq!(seen.len(),32);let v=state.native_view(0);assert!(v.get("lobbyOnly").is_none());assert_eq!(v["lobbyAvailable"],true);
 }
}
