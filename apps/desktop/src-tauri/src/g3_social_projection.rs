//! Trusted service DTO normalization, before a bounded game-thread projection.
//! No grants or network IO here. Core supplies frozen authority and selected roster.
use crate::protocol_g3_social::wire;
use std::collections::{BTreeMap,BTreeSet};
#[derive(Clone)]
pub struct Sender{pub principal:String,pub session:String,pub scope:wire::Scope}
pub struct Consumer{
 scope:wire::Scope,generation:u64,roster:BTreeMap<String,Sender>,sequences:BTreeMap<String,u32>,
 dismissed:BTreeSet<u64>,pending_dismissals:BTreeSet<u64>,
}
#[derive(Clone,Debug)]
pub struct Hud{pub actor:String,pub name:String,pub sequence:u32,pub hp:u32,pub max_hp:u32,pub sampo:bool}
pub const EMOTE_IDS:[&str;20]=["wave","heart","thumbs_up","disapprove","come_on","t_pose","mario","luigi","clap","macarena","the_wave","aggressive_point","default_dance","praise_the_sun","dab","worm","meditate","repose","egyptian","salute"];
pub const ROBE_IDS:[&str;6]=["red","orange","yellow","green","blue","purple"];
pub fn duration(id:usize)->u32{match id{1=>101,2=>240,3=>140,13=>375,15=>216,_=>0}}
#[derive(Clone,Debug)]
pub struct Marker{pub sequence:u64,pub actor:String,pub name:String,pub text:String,pub x:f64,pub y:f64}
#[derive(Debug)]
pub struct Page{pub high:u64,pub next:u64,pub markers:Vec<Marker>,pub complete:bool}
pub struct Pager{high:Option<u64>,live_high:u64,after:u64,pages:usize,count:usize,complete:bool}
fn number(s:Option<&str>)->Result<u64,String>{let s=s.ok_or("Missing POI cursor")?;let n=s.parse::<u64>().map_err(|_|"Invalid POI cursor")?;if n.to_string()!=s{return Err("Noncanonical POI cursor".into());}Ok(n)}
fn same(a:&wire::Scope,b:&wire::Scope)->bool{serde_json::to_vec(a).unwrap()==serde_json::to_vec(b).unwrap()}
fn name(value:Option<&str>)->Result<String,String>{let text=value.ok_or("Missing authenticated display name")?;if text.len()>396||text.chars().count()>99||text.chars().any(char::is_control){return Err("Invalid literal display name".into());}Ok(text.into())}
#[derive(Default)]
pub struct LatestScan{after:String,pages:usize,rows:Vec<wire::Projection>,envelope:Option<wire::Response>,complete:bool}
impl LatestScan{
 pub fn after(&self)->&str{&self.after} pub fn pages(&self)->usize{self.pages}
 pub fn accept(&mut self,response:&wire::Response)->Result<bool,String>{
  if self.complete||self.pages>=8||response.error.is_some(){return Err("Latest scan capacity/refusal".into());}
  if let Some(first)=&self.envelope{if first.service_id!=response.service_id||first.service_epoch!=response.service_epoch||first.session_epoch!=response.session_epoch||serde_json::to_vec(&first.scope).unwrap()!=serde_json::to_vec(&response.scope).unwrap(){return Err("Substituted latest page authority".into());}}
  let values=response.projections.as_deref().unwrap_or(&[]);if values.len()>32||self.rows.len()+values.len()>96{return Err("Latest projection capacity".into());}
  let mut last=self.after.clone();for p in values{let kinds=[("hud",p.hud.is_some()),("emote",p.emote.is_some()),("cosmetic",p.cosmetic.is_some())];let kind=kinds.into_iter().filter(|(_,present)|*present).collect::<Vec<_>>();if kind.len()!=1{return Err("Ambiguous latest kind".into());}
   let key=format!("{}:{}",p.game_session_id.as_deref().ok_or("Missing latest sender")?,kind[0].0);if key<=last{return Err("Unordered/duplicate latest key".into());}last=key;}
  if response.next_key.as_deref()!=Some(last.as_str()){return Err("Substituted latest cursor".into());}
  if self.envelope.is_none(){self.envelope=Some(response.clone());}self.after=last;self.pages+=1;self.rows.extend(values.iter().cloned());self.complete=values.is_empty();Ok(self.complete)
 }
 pub fn finish(self)->Result<wire::Response,String>{if !self.complete{return Err("Incomplete latest scan".into());}let mut response=self.envelope.ok_or("Missing latest envelope")?;response.projections=Some(self.rows);response.next_key=Some(self.after);Ok(response)}
}
impl Consumer{
 /// Only a Core-correlated, schema-checked Owner response may reach this seam.
 /// Selected principals come from Core's authenticated current room snapshot.
 pub fn from_authenticated_projections(scope:wire::Scope,generation:u64,selected:&[String],response:&wire::Response)->Result<Self,String>{
  if selected.len()>32||response.scope.as_ref().is_none_or(|s|!same(s,&scope))||response.error.is_some(){return Err("Foreign social discovery response".into());}
  let projections=response.projections.as_ref().ok_or("No authenticated projections")?;if projections.len()>96{return Err("Social discovery capture capacity".into());}
  let mut roster=BTreeMap::new();for p in projections{
   let principal=p.principal_id.as_ref().ok_or("Missing derived principal")?;let session=p.game_session_id.as_ref().ok_or("Missing derived session")?;
   let sender_scope=p.scope.as_ref().ok_or("Missing derived sender scope")?;
   if !selected.contains(principal)||p.provenance.as_deref()!=Some("validated-client-report"){return Err("Unselected projection sender".into());}
   crate::protocol_g3_social::typed::<wire::Scope>("Scope",&serde_json::to_vec(sender_scope).unwrap()).map_err(|_|"Invalid peer scope")?;
   let sender=Sender{principal:principal.clone(),session:session.clone(),scope:sender_scope.clone()};
   if let Some(old)=roster.get(session){let old:&Sender=old;if old.principal!=sender.principal||!same(&old.scope,&sender.scope){return Err("Substituted peer discovery".into());}}
   else{roster.insert(session.clone(),sender);}
  }
  Self::new(scope,generation,roster.into_values().collect())
 }
 pub fn new(scope:wire::Scope,generation:u64,roster:Vec<Sender>)->Result<Self,String>{
  if roster.len()>32||generation==0||generation>u32::MAX as u64{return Err("Invalid selected social roster".into());}
  let mut map=BTreeMap::new();for sender in roster{if sender.session.is_empty()||sender.principal.is_empty()
   ||sender.scope.room_id!=scope.room_id||sender.scope.run_id!=scope.run_id||sender.scope.run_epoch!=scope.run_epoch||sender.scope.config_revision!=scope.config_revision
   ||map.insert(sender.session.clone(),sender).is_some(){return Err("Foreign/duplicate social roster".into());}}
  Ok(Self{scope,generation,roster:map,sequences:Default::default(),dismissed:Default::default(),pending_dismissals:Default::default()})
 }
 pub fn reconcile_latest(&mut self,generation:u64,selected:&[String],response:&wire::Response)->Result<Vec<String>,String>{
  self.active(generation,response.scope.as_ref().ok_or("Missing latest scope")?)?;
  let fresh=Self::from_authenticated_projections(self.scope.clone(),generation,selected,response)?;
  let removed=self.roster.keys().filter(|key|!fresh.roster.contains_key(*key)).cloned().collect();self.roster=fresh.roster;Ok(removed)
 }
 pub fn actors(&self)->Vec<String>{self.roster.keys().cloned().collect()}
 /// Only current live projections enter this lookup; private historical markers never enter roster.
 pub fn current_sender(&self,actor:&str)->Option<(&str,&str,&str)>{let s=self.roster.get(actor)?;Some((&s.principal,&s.session,s.scope.world_token.as_deref()?))}
 fn active(&self,generation:u64,scope:&wire::Scope)->Result<(),String>{if generation!=self.generation||!same(scope,&self.scope){return Err("Retired/foreign social projection".into());}Ok(())}
 fn sender(&self,session:Option<&str>,principal:Option<&str>,scope:Option<&wire::Scope>,provenance:Option<&str>)->Result<&Sender,String>{
  let sender=self.roster.get(session.ok_or("Missing social actor")?).ok_or("Unselected social actor")?;
  if Some(sender.principal.as_str())!=principal||scope.is_none_or(|s|!same(s,&sender.scope))||provenance!=Some("validated-client-report"){return Err("Substituted social sender/scope/provenance".into());}Ok(sender)
 }
 /// Retained HUD state uses the canonical revision and actor identity, not age.
 pub fn hud(&mut self,generation:u64,response_scope:&wire::Scope,p:&wire::Projection)->Result<Option<Hud>,String>{
  self.active(generation,response_scope)?;let sender=self.sender(p.game_session_id.as_deref(),p.principal_id.as_deref(),p.scope.as_ref(),p.provenance.as_deref())?;
  let actor=sender.session.clone();let world=sender.scope.world_token.clone().ok_or("Missing HUD world")?;let sequence=p.hud_state_revision.filter(|n|*n>0).ok_or("Invalid HUD state revision")?;
  if p.event_id.is_some()||p.sequence.is_some()||p.expires_at.is_some(){return Err("Transient fields on retained HUD".into())}
  if self.sequences.get(&world).is_some_and(|old|sequence<*old){return Ok(None)}
  let hud=p.hud.as_ref().ok_or("Expected HUD projection")?;let known=hud.hp_known==Some(true);
  let hp=hud.hp_milli.unwrap_or(0);let max_hp=hud.max_hp_milli.unwrap_or(0);if known&&(hp>i32::MAX as u32||max_hp==0||max_hp>i32::MAX as u32){return Err("Invalid HUD bounds".into())}
  self.sequences.insert(world,sequence);if !known{return Ok(None)}
  Ok(Some(Hud{actor,name:name(p.display_name.as_deref())?,sequence,hp,max_hp,sampo:hud.sampo.unwrap_or(false)}))
 }
 pub fn remember_dismissal(&mut self,sequence:u64)->Result<(),String>{
  if sequence==0||(!self.dismissed.contains(&sequence)&&self.dismissed.len()>=256){return Err("Dismissal capacity".into());}
  self.dismissed.insert(sequence);self.pending_dismissals.insert(sequence);Ok(())
 }
 pub fn acknowledged_dismissal(&mut self,sequence:u64){self.pending_dismissals.remove(&sequence);}
 pub fn pending_dismissals(&self)->impl Iterator<Item=u64>+'_ {self.pending_dismissals.iter().copied()}
 pub fn pager(&self)->Pager{Pager{high:None,live_high:0,after:0,pages:0,count:0,complete:false}}
 pub fn page(&self,generation:u64,response:&wire::Response,pager:&mut Pager)->Result<Page,String>{
  self.active(generation,response.scope.as_ref().ok_or("Missing correlated scope")?)?;
  if pager.complete||pager.pages>=32||response.error.is_some(){return Err("POI scan stopped/capacity".into());}
  let live_high=number(response.high_sequence.as_deref())?;let next=number(response.next_sequence.as_deref())?;
  if live_high<pager.live_high||next<pager.after||next>live_high{return Err("Regressing POI cursor/high".into());}
  let high=pager.high.unwrap_or(live_high);let values=response.markers.as_deref().unwrap_or(&[]);
  if values.len()>32{return Err("POI page capacity".into());}let mut previous=pager.after;let mut markers=vec![];let mut count=pager.count;
  for marker in values{
   let sequence=number(marker.poi_sequence.as_deref())?;if sequence<=previous||sequence>live_high{return Err("Unordered POI marker".into());}previous=sequence;
   // A private canonical Marker remains valid after sender departure/no fresh HUD.
   // This attributes one marker only; it never adds a live publisher/ghost grant.
   let actor=marker.game_session_id.as_ref().ok_or("Missing POI sender")?;
   if self.roster.contains_key(actor){self.sender(Some(actor),marker.principal_id.as_deref(),marker.scope.as_ref(),marker.provenance.as_deref())?;}
   else{let scope=marker.scope.as_ref().ok_or("Missing historical marker scope")?;
    if marker.provenance.as_deref()!=Some("validated-client-report")||marker.principal_id.as_ref().is_none_or(|s|uuid::Uuid::parse_str(s).is_err())||uuid::Uuid::parse_str(actor).is_err()
     ||scope.room_id!=self.scope.room_id||scope.run_id!=self.scope.run_id||scope.run_epoch!=self.scope.run_epoch||scope.config_revision!=self.scope.config_revision{return Err("Foreign historical marker".into());}
    crate::protocol_g3_social::typed::<wire::Scope>("Scope",&serde_json::to_vec(scope).unwrap()).map_err(|_|"Invalid historical marker scope")?;
   }
   let name=name(marker.display_name.as_deref())?;
   let poi=marker.poi.as_ref().ok_or("Missing POI")?;let text=poi.text.as_ref().ok_or("Missing literal POI text")?;
   if text.len()>396||text.chars().count()>99||text.chars().any(char::is_control){return Err("Invalid literal POI text".into());}
   let coordinate=|s:Option<&str>|->Result<f64,String>{let s=s.ok_or("Missing coordinate")?;let n=s.parse::<i32>().map_err(|_|"Invalid coordinate")?;if n.to_string()!=s{return Err("Noncanonical coordinate".into());}Ok(n as f64/1000.)};
   let (x,y)=(coordinate(poi.x_milli.as_deref())?,coordinate(poi.y_milli.as_deref())?);
   if sequence<=high{count+=1;if count>256{return Err("POI captured-prefix capacity".into());}
    if !self.dismissed.contains(&sequence){markers.push(Marker{sequence,actor:marker.scope.as_ref().and_then(|s|s.world_token.clone()).ok_or("Missing marker world identity")?,name,text:text.clone(),x,y});}}
  }
  if next<previous||(next==pager.after&&next<high){return Err("Nonprogressing POI page".into());}
  pager.high=Some(high);pager.live_high=live_high;pager.after=next;pager.pages+=1;pager.count=count;pager.complete=next>=high;
  Ok(Page{high,next:next.min(high),markers,complete:pager.complete})
 }
}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn a2_maximum_cold_scan_reproduces_atomic_queue_limit_and_streams_sixteen_pages(){
  use crate::g3_social_frame::{Queue,Command};let(c,scope,_)=setup();let mut scan=c.pager();let mut all=vec![vec![Command::Begin(256)]];
  for start in(1..=256).step_by(16){let mut response=page(&scope,256,start+15,start..=start+15);for marker in response.markers.as_mut().unwrap(){marker.poi.as_mut().unwrap().text=Some("🧙".repeat(99));marker.display_name=Some("雪".repeat(99));}all.push(vec![Command::Page(c.page(1,&response,&mut scan).unwrap())]);}
  let mut q=Queue::native([1;16],1);assert!(q.publish(0.,all).is_err(),"Old full capture is17frames, greater than the retained12frame bound");
  let mut scan=c.pager();let mut count=0;for start in(1..=256).step_by(16){let mut response=page(&scope,256,start+15,start..=start+15);for marker in response.markers.as_mut().unwrap(){marker.poi.as_mut().unwrap().text=Some("🧙".repeat(99));marker.display_name=Some("雪".repeat(99));}let normalized=c.page(1,&response,&mut scan).unwrap();count+=normalized.markers.len();let mut batch=Vec::new();if start==1{batch.push(vec![Command::Begin(256)]);}batch.push(vec![Command::Page(normalized)]);q.publish(0.,batch).unwrap();while q.queued(){let frame=q.take([1;16],1,true).unwrap();assert!(frame.len()<=32768);} }
  assert_eq!(count,256);assert!(scan.complete);let mut scan=c.pager();let first=page(&scope,256,16,1..=16);c.page(1,&first,&mut scan).unwrap();assert!(c.page(1,&first,&mut scan).is_err());let mut reversed=page(&scope,256,32,17..=32);reversed.markers.as_mut().unwrap().reverse();assert!(c.page(1,&reversed,&mut scan).is_err());
 }
 fn setup()->(Consumer,wire::Scope,Sender){let raw:serde_json::Value=serde_json::from_slice(include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../packages/contracts/g3-social/replay-policy.json"))).unwrap();
  let scope:wire::Scope=serde_json::from_value(raw["vectors"][0]["scope"].clone()).unwrap();let sender=Sender{principal:"principal".into(),session:"session".into(),scope:scope.clone()};(Consumer::new(scope.clone(),1,vec![sender.clone()]).unwrap(),scope,sender)}
 fn page(scope:&wire::Scope,high:u64,next:u64,rows:std::ops::RangeInclusive<u64>)->wire::Response{
  wire::Response{scope:Some(scope.clone()),high_sequence:Some(high.to_string()),next_sequence:Some(next.to_string()),markers:Some(rows.map(|n|wire::Marker{
   poi_sequence:Some(n.to_string()),game_session_id:Some("session".into()),principal_id:Some("principal".into()),scope:Some(scope.clone()),provenance:Some("validated-client-report".into()),display_name:Some("literal name".into()),
   poi:Some(wire::Poi{text:Some(format!("marker {n}")),x_milli:Some("1250".into()),y_milli:Some("-2500".into()),..Default::default()}),..Default::default()
  }).collect()),..Default::default()}
 }

 #[test]fn growing_high_captured_prefix_refresh_empty_tail_dismissals_and_fences(){
  let (mut c,scope,_)=setup();let mut scan=c.pager();assert!(!c.page(1,&page(&scope,33,32,1..=32),&mut scan).unwrap().complete);
  c.remember_dismissal(33).unwrap();let final_page=c.page(1,&page(&scope,34,34,33..=34),&mut scan).unwrap();assert!(final_page.complete);assert_eq!(final_page.high,33);assert_eq!(final_page.next,33);assert!(final_page.markers.is_empty());assert!(c.page(1,&page(&scope,34,34,33..=34),&mut scan).is_err());
  let refresh=c.page(1,&page(&scope,34,34,33..=34),&mut c.pager()).unwrap();assert_eq!(refresh.markers.len(),1);assert_eq!(refresh.markers[0].sequence,34);assert_eq!(c.pending_dismissals().collect::<Vec<_>>(),vec![33]);c.acknowledged_dismissal(33);
  assert!(c.page(1,&page(&scope,34,34,35..=34),&mut c.pager()).unwrap().complete);
  assert!(c.page(2,&page(&scope,34,34,33..=34),&mut c.pager()).is_err());let mut foreign=page(&scope,34,34,33..=34);foreign.markers.as_mut().unwrap()[0].principal_id=Some("foreign".into());assert!(c.page(1,&foreign,&mut c.pager()).is_err());
  let mut wrong_scope=page(&scope,34,34,33..=34);wrong_scope.scope.as_mut().unwrap().config_revision=Some(99);assert!(c.page(1,&wrong_scope,&mut c.pager()).is_err());
  let mut regressed=c.pager();c.page(1,&page(&scope,33,32,1..=32),&mut regressed).unwrap();assert!(c.page(1,&page(&scope,32,32,33..=32),&mut regressed).is_err());
  assert!(c.page(1,&page(&scope,34,0,1..=0),&mut c.pager()).is_err());let mut scan=c.pager();for n in 1..=32{c.page(1,&page(&scope,100,n,n..=n),&mut scan).unwrap();}assert!(c.page(1,&page(&scope,100,33,33..=33),&mut scan).is_err());
 }
 #[test]fn overmax_zero_retained_hud_revision_generation_and_scope(){
  let(mut c,scope,_)=setup();let mut p=wire::Projection{display_name:Some("literal name".into()),game_session_id:Some("session".into()),principal_id:Some("principal".into()),scope:Some(scope.clone()),provenance:Some("validated-client-report".into()),hud_state_revision:Some(10),hud:Some(wire::Hud{hp_milli:Some(3000),max_hp_milli:Some(2000),sampo:Some(true),hp_known:Some(true),sampo_known:Some(true),..Default::default()}),..Default::default()};
  assert_eq!(c.hud(1,&scope,&p).unwrap().unwrap().hp,3000);p.hud_state_revision=Some(9);assert!(c.hud(1,&scope,&p).unwrap().is_none());p.hud_state_revision=Some(11);p.hud.as_mut().unwrap().hp_milli=Some(0);assert_eq!(c.hud(1,&scope,&p).unwrap().unwrap().hp,0);assert!(c.hud(2,&scope,&p).is_err());p.hud.as_mut().unwrap().hp_milli=Some(u32::MAX);assert!(c.hud(1,&scope,&p).is_err());
 }
 #[test]fn latest_all96_sorted_rows_four_pages_empty_tail_and_substitution(){
  let (_,scope,_)=setup();let mut rows=vec![];for n in 1..=32{for kind in ["cosmetic","emote","hud"]{let mut p=wire::Projection{game_session_id:Some(uuid::Uuid::from_u128(n).to_string()),..Default::default()};match kind{"hud"=>p.hud=Some(Default::default()),"emote"=>p.emote=Some(Default::default()),_=>p.cosmetic=Some(Default::default())};rows.push(p);}}
  let mut scan=LatestScan::default();for chunk in rows.chunks(32){let last=chunk.last().unwrap();let kind=if last.hud.is_some(){"hud"}else if last.emote.is_some(){"emote"}else{"cosmetic"};let response=wire::Response{scope:Some(scope.clone()),service_id:Some("service".into()),service_epoch:Some("epoch".into()),session_epoch:Some("session".into()),projections:Some(chunk.to_vec()),next_key:Some(format!("{}:{kind}",last.game_session_id.as_ref().unwrap())),..Default::default()};assert!(!scan.accept(&response).unwrap());}
  let tail=wire::Response{scope:Some(scope),service_id:Some("service".into()),service_epoch:Some("epoch".into()),session_epoch:Some("session".into()),projections:Some(vec![]),next_key:Some(scan.after().into()),..Default::default()};let mut bad=tail.clone();bad.service_epoch=Some("changed".into());assert!(scan.accept(&bad).is_err());assert!(scan.accept(&tail).unwrap());assert_eq!(scan.finish().unwrap().projections.unwrap().len(),96);
 }
 #[test]fn historical_poi_without_live_hud_is_not_an_active_sender(){
  let (mut c,scope,_)=setup();let mut response=page(&scope,1,1,1..=1);let marker=&mut response.markers.as_mut().unwrap()[0];marker.principal_id=Some(uuid::Uuid::new_v4().to_string());marker.game_session_id=Some(uuid::Uuid::new_v4().to_string());let historical=marker.game_session_id.clone().unwrap();
  assert_eq!(c.page(1,&response,&mut c.pager()).unwrap().markers[0].actor,scope.world_token.as_deref().unwrap());assert!(!c.roster.contains_key(&historical));c.remember_dismissal(1).unwrap();assert!(c.page(1,&response,&mut c.pager()).unwrap().markers.is_empty());
 }
}
