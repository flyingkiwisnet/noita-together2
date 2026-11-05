//! Draft3 current presentation, independent of finite mailbox delivery leases.
use crate::{protocol_g3_social::wire,g3_social_projection::{EMOTE_IDS,ROBE_IDS,duration}};
use std::{collections::BTreeMap,time::Duration};
#[derive(Clone,Debug)]pub struct Emote{pub actor:String,pub ghost:String,pub revision:u32,pub event:String,pub sequence:u32,pub id:u8,pub active:bool,pub duration_frames:u32,pub age_ms:u32,pub observed:f64}
#[derive(Clone,Debug,PartialEq)]pub struct Cosmetic{pub actor:String,pub ghost:String,pub revision:u32,pub event:String,pub sequence:u32,pub robe:u8,pub hat:bool,pub amulet:bool,pub gem:bool}
pub enum Command{Emote(Emote),Cosmetic(Cosmetic),Outcome{id:String,status:u8,reason:String,event:String}}
fn identity(scope:&wire::Scope,players:&[wire::Player],p:&wire::Projection)->Result<(String,String,u32,String,u32),String>{
 let sender=p.scope.as_ref().ok_or("Missing presentation scope")?;
 if sender.room_id!=scope.room_id||sender.run_id!=scope.run_id||sender.run_epoch!=scope.run_epoch||sender.config_revision!=scope.config_revision||p.provenance.as_deref()!=Some("validated-client-report")||p.hud.is_some()||p.hud_state_revision.is_some()||p.expires_at.is_some(){return Err("Foreign presentation state".into())}
 let player=players.iter().find(|a|(a.state.as_deref()==Some("playing")||p.cosmetic.is_some()&&a.state.as_deref()==Some("waiting"))&&a.principal_id==p.principal_id&&a.game_session_id==p.game_session_id&&a.world_token==sender.world_token).ok_or("Presentation outside eligible roster")?;
 let event=p.event_id.clone().ok_or("Missing presentation original")?;if uuid::Uuid::parse_str(&event).is_err(){return Err("Presentation original UUID".into())}
 let revision=p.presentation_revision.filter(|v|*v>0).ok_or("Missing presentation revision")?;let sequence=p.sequence.filter(|v|*v>0).ok_or("Missing presentation sequence")?;
 let actor=player.player_id.clone().filter(|a|Some(a)==player.world_token.as_ref()).ok_or("Presentation world actor")?;
 let ghost=crate::g3_a2_frame::roster(&wire::Response{roster_complete:Some(true),players:Some(vec![player.clone()]),..Default::default()})?.remove(0).ghost;
 Ok((actor,ghost,revision,event,sequence))
}
fn text(out:&mut Vec<u8>,s:&str,max:usize)->Result<(),String>{if s.is_empty()||s.len()>max||s.chars().any(char::is_control){return Err("A3 literal bound".into())}out.extend((s.len()as u16).to_le_bytes());out.extend(s.as_bytes());Ok(())}
fn encoded_identity(ghost:&str,event:&str)->bool{ghost.len()==32&&ghost.bytes().all(|b|b.is_ascii_digit()||(b'a'..=b'f').contains(&b))&&event.len()==36&&uuid::Uuid::parse_str(event).is_ok()}
pub fn encode(out:&mut Vec<u8>,c:&Command,now:f64)->Result<(),String>{match c{
 Command::Emote(v)=>{if !encoded_identity(&v.ghost,&v.event)||v.id>=20||v.revision==0||v.sequence==0||v.duration_frames!=duration(v.id as usize)||!now.is_finite()||!v.observed.is_finite()||v.observed<0.||now<v.observed{return Err("A3 emote frame".into())}out.push(8);text(out,&v.actor,128)?;text(out,&v.ghost,32)?;out.extend(v.revision.to_le_bytes());text(out,&v.event,36)?;out.extend(v.sequence.to_le_bytes());out.push(v.id);out.push(u8::from(v.active));out.extend((v.duration_frames as u16).to_le_bytes());out.extend(v.age_ms.saturating_add(((now-v.observed)*1000.).min(u32::MAX as f64)as u32).to_le_bytes());},
 Command::Cosmetic(v)=>{if !encoded_identity(&v.ghost,&v.event)||v.robe>=6||v.revision==0||v.sequence==0{return Err("A3 cosmetic frame".into())}out.push(9);text(out,&v.actor,128)?;text(out,&v.ghost,32)?;out.extend(v.revision.to_le_bytes());text(out,&v.event,36)?;out.extend(v.sequence.to_le_bytes());out.push(v.robe);out.push(u8::from(v.hat)|(u8::from(v.amulet)<<1)|(u8::from(v.gem)<<2));},
 Command::Outcome{id,status,reason,event}=>{if *status>2||id.len()!=32||uuid::Uuid::parse_str(id).is_err(){return Err("A3 outcome".into())}out.push(15);text(out,id,32)?;out.push(*status);out.extend((reason.len()as u16).to_le_bytes());if reason.len()>396||reason.chars().any(char::is_control){return Err("A3 outcome reason".into())}out.extend(reason.as_bytes());out.extend((event.len()as u16).to_le_bytes());if !event.is_empty()&&uuid::Uuid::parse_str(event).is_err(){return Err("A3 outcome event".into())}out.extend(event.as_bytes());}
 }Ok(())}
#[derive(Default)]pub struct Replay{emotes:BTreeMap<String,Emote>,cosmetics:BTreeMap<String,Cosmetic>}
impl Replay{
 pub fn roster(&mut self,players:&[wire::Player]){let current=crate::g3_a2_frame::roster(&wire::Response{roster_complete:Some(true),players:Some(players.to_vec()),..Default::default()}).unwrap_or_default();self.emotes.retain(|id,v|{if let Some(p)=players.iter().find(|p|p.player_id.as_ref()==Some(id)){if p.state.as_deref()!=Some("playing")||current.iter().any(|p|p.actor==*id&&p.ghost!=v.ghost){v.active=false;}true}else{false}});self.cosmetics.retain(|id,_|players.iter().any(|p|p.player_id.as_ref()==Some(id)));}
 pub fn accept(&mut self,scope:&wire::Scope,players:&[wire::Player],p:&wire::Projection,transit:Duration,now:f64)->Result<Option<Command>,String>{
  if !now.is_finite()||now<0.{return Err("A3 local monotonic time".into())}let(actor,ghost,revision,event,sequence)=identity(scope,players,p)?;
  if p.emote.is_some()==p.cosmetic.is_some(){return Err("A3 ambiguous state".into())}
  if let Some(e)=&p.emote{
   let id=EMOTE_IDS.iter().position(|id|Some(*id)==e.emote_id.as_deref()).ok_or("Unknown emote")?;let frames=duration(id);if e.duration_frames!=Some(frames){return Err("A3 duration substitution".into())}
   let mut active=e.active.ok_or("A3 active")?;let mut age=e.age_ms.ok_or("A3 age")?.saturating_add(transit.as_millis().min(u32::MAX as u128)as u32);
   if let Some(old)=self.emotes.get(&actor){if revision<old.revision{return Ok(None)}if revision==old.revision{
    if old.event!=event||old.sequence!=sequence||old.id!=id as u8||old.duration_frames!=frames{return Err("Changed emote revision".into())}
    active&=old.active;age=age.max(old.age_ms.saturating_add(((now-old.observed).max(0.)*1000.).min(u32::MAX as f64)as u32));
   }}
   if let Some(old)=self.emotes.get(&actor){if event==old.event{if old.id!=id as u8||old.sequence!=sequence||old.duration_frames!=frames{return Err("Substituted accepted emote original".into())}active&=old.active;age=age.max(old.age_ms.saturating_add(((now-old.observed).max(0.)*1000.).min(u32::MAX as f64)as u32));}}
   if frames>0&&(u64::from(age)*60/1000)>=u64::from(frames)+2{active=false}
   let value=Emote{actor:actor.clone(),ghost,revision,event,sequence,id:id as u8,active,duration_frames:frames,age_ms:age,observed:now};self.emotes.insert(actor,value.clone());Ok(Some(Command::Emote(value)))
  }else{
   let c=p.cosmetic.as_ref().unwrap();let robe=ROBE_IDS.iter().position(|id|Some(*id)==c.robe_id.as_deref()).ok_or("Unknown robe")?;
   let value=Cosmetic{actor:actor.clone(),ghost,revision,event,sequence,robe:robe as u8,hat:c.hat.ok_or("A3 hat")?,amulet:c.amulet.ok_or("A3 amulet")?,gem:c.gem.ok_or("A3 gem")?};
   if let Some(old)=self.cosmetics.get(&actor){if revision<old.revision{return Ok(None)}if value.event==old.event&&(value.sequence!=old.sequence||value.robe!=old.robe||value.hat!=old.hat||value.amulet!=old.amulet||value.gem!=old.gem){return Err("Substituted accepted cosmetic original".into())}if revision==old.revision{let mut comparable=value.clone();comparable.ghost=old.ghost.clone();if comparable!=*old{return Err("Changed cosmetic revision".into())}}}
   self.cosmetics.insert(actor,value.clone());Ok(Some(Command::Cosmetic(value)))
  }
 }
}

#[cfg(test)]mod tests{
 use super::*;
 #[test]fn a3_same_cosmetic_original_cannot_change_payload_at_a_new_revision(){
  let(s,a,mut p)=setup();let mut r=Replay::default();p.cosmetic=Some(wire::Cosmetic{robe_id:Some("red".into()),hat:Some(false),amulet:Some(true),gem:Some(false)});r.accept(&s,&[a.clone()],&p,Duration::ZERO,0.).unwrap();let original=p.clone();
  for field in 0..5{p=original.clone();p.presentation_revision=Some(2);let c=p.cosmetic.as_mut().unwrap();match field{0=>c.robe_id=Some("blue".into()),1=>c.hat=Some(true),2=>c.amulet=Some(false),3=>c.gem=Some(true),_=>p.sequence=Some(2)}assert!(r.accept(&s,&[a.clone()],&p,Duration::ZERO,1.).is_err());}
  p=original;p.presentation_revision=Some(2);assert!(r.accept(&s,&[a],&p,Duration::ZERO,2.).unwrap().is_some(),"Unchanged world cosmetic can retain its original through a canonical rebind revision");
 }
 #[test]fn a3_native_frame_vectors_and_strict_encode_bounds(){
  use crate::g3_social_frame::{self,Command as Frame};let(s,a,mut p)=setup();let mut replay=Replay::default();let mut lines=String::new();let mut sequence=0;
  let mut add=|kind:&str,index:u32,c|{sequence+=1;let bytes=g3_social_frame::encode([1;16],1,sequence,101.,&[Frame::A3(c)]).unwrap();let hex:String=bytes.iter().map(|b|format!("{b:02x}")).collect();lines.push_str(&format!("{kind} {index} {hex}\n"));};
  for(i,id)in EMOTE_IDS.iter().enumerate(){p.presentation_revision=Some(i as u32+1);p.sequence=p.presentation_revision;p.event_id=Some(uuid::Uuid::new_v4().to_string());p.emote=Some(wire::EmoteState{emote_id:Some((*id).into()),active:Some(true),duration_frames:Some(duration(i)),age_ms:Some(300)});let c=replay.accept(&s,&[a.clone()],&p,Duration::ZERO,100.).unwrap().unwrap();add("emote",i as u32,c);}
  p.emote=None;for index in 0..48{p.presentation_revision=Some(index+1);p.sequence=p.presentation_revision;p.event_id=Some(uuid::Uuid::new_v4().to_string());p.cosmetic=Some(wire::Cosmetic{robe_id:Some(ROBE_IDS[index as usize/8].into()),hat:Some(index&1!=0),amulet:Some(index&2!=0),gem:Some(index&4!=0)});add("cosmetic",index,replay.accept(&s,&[a.clone()],&p,Duration::ZERO,100.).unwrap().unwrap());}
  for status in 0..3{add("outcome",status,Command::Outcome{id:uuid::Uuid::new_v4().simple().to_string(),status:status as u8,reason:if status==2{"Selection refused".into()}else{String::new()},event:uuid::Uuid::new_v4().to_string()});}
  let dir=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../spikes/noita-entry/.build/a3-client-20261022-01");std::fs::create_dir_all(&dir).unwrap();std::fs::write(dir.join("wire.frames"),lines).unwrap();
  let good=Cosmetic{actor:"world".into(),ghost:"a".repeat(32),revision:1,event:uuid::Uuid::new_v4().to_string(),sequence:1,robe:0,hat:true,amulet:false,gem:true};for ghost in ["a".into(),"X".repeat(32),"a".repeat(33)]{let mut bad=good.clone();bad.ghost=ghost;assert!(encode(&mut vec![],&Command::Cosmetic(bad),1.).is_err());}let mut bad=good;bad.event="invalid".into();assert!(encode(&mut vec![],&Command::Cosmetic(bad),1.).is_err());
 }
 fn setup()->(wire::Scope,wire::Player,wire::Projection){let raw:serde_json::Value=serde_json::from_slice(include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../packages/contracts/g3-social/replay-policy.json"))).unwrap();let scope:wire::Scope=serde_json::from_value(raw["vectors"][0]["scope"].clone()).unwrap();let player=wire::Player{player_id:scope.world_token.clone(),world_token:scope.world_token.clone(),principal_id:Some(uuid::Uuid::new_v4().to_string()),game_session_id:Some(uuid::Uuid::new_v4().to_string()),display_name:Some("Alex".into()),state:Some("playing".into()),inventory_revision:Some(0),held_wand_sprite_id:Some(String::new()),..Default::default()};let p=wire::Projection{scope:Some(scope.clone()),principal_id:player.principal_id.clone(),game_session_id:player.game_session_id.clone(),presentation_revision:Some(1),sequence:Some(1),event_id:Some(uuid::Uuid::new_v4().to_string()),provenance:Some("validated-client-report".into()),..Default::default()};(scope,player,p)}
 #[test]fn a3_all_twenty_six_colours_and_flags_are_retained_without_lifetime_ttl(){
  let(s,a,mut p)=setup();let mut r=Replay::default();for(i,id)in EMOTE_IDS.iter().enumerate(){p.presentation_revision=Some(i as u32+1);p.sequence=p.presentation_revision;p.event_id=Some(uuid::Uuid::new_v4().to_string());p.emote=Some(wire::EmoteState{emote_id:Some((*id).into()),active:Some(true),duration_frames:Some(duration(i)),age_ms:Some(if duration(i)==0{31000}else{0})});let Command::Emote(e)=r.accept(&s,&[a.clone()],&p,Duration::ZERO,1.).unwrap().unwrap()else{panic!()};assert_eq!(e.id,i as u8);assert!(e.active);let mut body=vec![];encode(&mut body,&Command::Emote(e),2.).unwrap();assert_eq!(body[0],8);}
  p.emote=None;for(i,robe)in ROBE_IDS.iter().enumerate(){for bits in 0..8{p.presentation_revision=Some(i as u32*8+bits+1);p.sequence=p.presentation_revision;p.event_id=Some(uuid::Uuid::new_v4().to_string());p.cosmetic=Some(wire::Cosmetic{robe_id:Some((*robe).into()),hat:Some(bits&1!=0),amulet:Some(bits&2!=0),gem:Some(bits&4!=0)});let Command::Cosmetic(c)=r.accept(&s,&[a.clone()],&p,Duration::from_secs(90),100.).unwrap().unwrap()else{panic!()};assert_eq!(c.robe,i as u8);let mut early=vec![];let mut late=vec![];encode(&mut early,&Command::Cosmetic(c.clone()),100.).unwrap();encode(&mut late,&Command::Cosmetic(c),10000.).unwrap();assert_eq!(early,late);}}
 }
 #[test]fn a3_timed_boundary_monotonic_transit_duplicate_old_start_and_turnover_fences(){
  let(s,mut a,mut p)=setup();let mut r=Replay::default();p.emote=Some(wire::EmoteState{emote_id:Some("heart".into()),active:Some(true),duration_frames:Some(101),age_ms:Some(1616)});
  let Command::Emote(e)=r.accept(&s,&[a.clone()],&p,Duration::from_millis(100),1.).unwrap().unwrap()else{panic!()};assert!(e.active);let Command::Emote(e)=r.accept(&s,&[a.clone()],&p,Duration::from_millis(101),1.001).unwrap().unwrap()else{panic!()};assert!(!e.active,"Exact103frame boundary");
  p.emote.as_mut().unwrap().age_ms=Some(0);let Command::Emote(e)=r.accept(&s,&[a.clone()],&p,Duration::ZERO,1.002).unwrap().unwrap()else{panic!()};assert!(!e.active,"Delayed duplicate cannot restart or renew completed original");
  p.presentation_revision=Some(2);p.sequence=Some(2);p.event_id=Some(uuid::Uuid::new_v4().to_string());p.emote=Some(wire::EmoteState{emote_id:Some("wave".into()),active:Some(false),duration_frames:Some(0),age_ms:Some(0)});r.accept(&s,&[a.clone()],&p,Duration::ZERO,2.).unwrap();p.presentation_revision=Some(1);p.emote.as_mut().unwrap().active=Some(true);assert!(r.accept(&s,&[a.clone()],&p,Duration::ZERO,3.).unwrap().is_none());
  p.presentation_revision=Some(3);p.sequence=Some(3);p.event_id=Some(uuid::Uuid::new_v4().to_string());p.emote.as_mut().unwrap().active=Some(true);r.accept(&s,&[a.clone()],&p,Duration::ZERO,4.).unwrap();a.game_session_id=Some(uuid::Uuid::new_v4().to_string());r.roster(&[a.clone()]);p.game_session_id=a.game_session_id.clone();let Command::Emote(e)=r.accept(&s,&[a.clone()],&p,Duration::ZERO,5.).unwrap().unwrap()else{panic!()};assert!(!e.active,"Owner replacement cannot resume a prior emote");
  p.cosmetic=Some(wire::Cosmetic{robe_id:Some("red".into()),hat:Some(true),amulet:Some(false),gem:Some(true)});p.emote=None;let original=p.event_id.clone();assert!(r.accept(&s,&[a.clone()],&p,Duration::ZERO,6.).unwrap().is_some());a.game_session_id=Some(uuid::Uuid::new_v4().to_string());p.game_session_id=a.game_session_id.clone();r.roster(&[a.clone()]);assert!(r.accept(&s,&[a.clone()],&p,Duration::ZERO,7.).unwrap().is_some());assert_eq!(p.event_id,original,"Cosmetic restoration keeps old accepted original identity");
  a.state=Some("disconnected".into());assert!(r.accept(&s,&[a],&p,Duration::ZERO,8.).is_err());p.expires_at=Some("2099-01-01T00:00:00Z".into());assert!(r.accept(&s,&[],&p,Duration::ZERO,8.).is_err());
 }
}
