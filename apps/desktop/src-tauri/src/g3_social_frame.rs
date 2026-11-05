//! UNAPPROVED local framing tranche. Core supplies already-normalized data.
//! No credentials, HTTP, file paths, Lua code or authority grants in these frames.
use crate::g3_social_projection::{Hud,Page};
use std::collections::VecDeque;
use std::time::{Instant,Duration};
pub const MAX_BYTES:usize=32768;
pub enum Command{Hud(Hud),Remove(String),Begin(u64),Page(Page),Pause,End,Roster(Vec<String>),A3(crate::g3_a3_frame::Command),A2(crate::g3_a2_frame::Command)}
fn string(out:&mut Vec<u8>,value:&str,max:usize)->Result<(),String>{if value.len()>max||value.chars().any(char::is_control){return Err("Local literal capacity/control".into());}out.extend_from_slice(&(value.len() as u16).to_le_bytes());out.extend_from_slice(value.as_bytes());Ok(())}
pub fn encode(context:[u8;16],generation:u32,sequence:u32,now:f64,commands:&[Command])->Result<Vec<u8>,String>{
 if context==[0;16]||generation==0||sequence==0||commands.len()>32||!now.is_finite()||now<0.{return Err("Invalid local frame owner/budget".into());}
 let mut body=Vec::new();for command in commands{match command{
  Command::A2(c)=>crate::g3_a2_frame::encode(&mut body,c,now)?,
  Command::Hud(h)=>{if h.sequence==0||h.hp>i32::MAX as u32||h.max_hp==0||h.max_hp>i32::MAX as u32{return Err("Invalid retained HUD bounds".into());}
   body.push(1);string(&mut body,&h.actor,128)?;string(&mut body,&h.name,396)?;for n in [h.sequence,h.hp,h.max_hp]{body.extend_from_slice(&n.to_le_bytes());}body.push(u8::from(h.sampo));},
  Command::Remove(actor)=>{body.push(2);string(&mut body,actor,128)?;},
  Command::Begin(high)=>{body.push(3);string(&mut body,&high.to_string(),20)?;},
  Command::Page(page)=>{if page.markers.len()>32||page.next>page.high{return Err("Local POI page capacity".into());}body.push(4);string(&mut body,&page.high.to_string(),20)?;string(&mut body,&page.next.to_string(),20)?;body.push(page.markers.len() as u8);
   for marker in &page.markers{string(&mut body,&marker.sequence.to_string(),20)?;string(&mut body,&marker.actor,128)?;string(&mut body,&marker.name,396)?;string(&mut body,&marker.text,396)?;
    for value in [marker.x,marker.y]{let scaled=value*1000.;if !scaled.is_finite()||scaled<i32::MIN as f64||scaled>i32::MAX as f64{return Err("Invalid local marker coordinate".into());}body.extend_from_slice(&(scaled.round() as i32).to_le_bytes());}
   }},Command::Pause=>body.push(5),Command::End=>body.push(6),
   Command::Roster(actors)=>{if actors.len()>32{return Err("Roster capacity".into());}body.push(7);body.push(actors.len() as u8);let mut seen=std::collections::BTreeSet::new();for actor in actors{if actor.is_empty()||!seen.insert(actor){return Err("Roster identity".into());}string(&mut body,actor,128)?;}},
   Command::A3(c)=>crate::g3_a3_frame::encode(&mut body,c,now)?,

 }}
 if body.len()+40>MAX_BYTES{return Err("Local frame byte capacity".into());}
 let mut frame=vec![0u8;40];frame[..4].copy_from_slice(b"NSF1");frame[4]=1;frame[8..24].copy_from_slice(&context);frame[24..28].copy_from_slice(&generation.to_le_bytes());frame[28..32].copy_from_slice(&sequence.to_le_bytes());frame[32..36].copy_from_slice(&(body.len() as u32).to_le_bytes());frame[36..38].copy_from_slice(&(commands.len() as u16).to_le_bytes());frame.extend(body);Ok(frame)
}
/// Atomic batches, <=12 frames. An undrained control/snapshot batch is never evicted.
struct Pending{sequence:u32,now:f64,at:Instant,commands:Vec<Command>}
pub struct Queue{context:[u8;16],generation:u32,sequence:u32,frames:VecDeque<Pending>,age:Duration}
impl Queue{
 pub fn new(context:[u8;16],generation:u32)->Self{Self{context,generation,sequence:0,frames:Default::default(),age:Duration::from_millis(100)}}
 pub fn native(context:[u8;16],generation:u32)->Self{let mut q=Self::new(context,generation);q.age=Duration::from_secs(1);q}
 pub fn native_after(context:[u8;16],generation:u32,sequence:u32)->Self{let mut q=Self::native(context,generation);q.sequence=sequence;q}
 pub fn queued(&self)->bool{!self.frames.is_empty()}
 pub fn sequence(&self)->u32{self.sequence}
 pub fn abort(&mut self)->Option<Vec<u8>>{self.frames.clear();self.sequence=self.sequence.checked_add(1)?;encode(self.context,self.generation,self.sequence,0.,&[Command::Pause]).ok()}
 pub fn terminal(&mut self)->Option<Vec<u8>>{self.frames.clear();self.sequence=self.sequence.checked_add(1)?;encode(self.context,self.generation,self.sequence,0.,&[Command::End]).ok()}
 pub fn publish(&mut self,now:f64,batches:Vec<Vec<Command>>)->Result<(),String>{
  if !self.frames.is_empty()||batches.len()>12{return Err("Local queue backpressure".into());}
  let mut staged=VecDeque::new();let mut sequence=self.sequence;
  for commands in batches{sequence=sequence.checked_add(1).ok_or("Local sequence exhausted")?;encode(self.context,self.generation,sequence,now,&commands)?;staged.push_back(Pending{sequence,now,at:Instant::now(),commands});}
  self.sequence=sequence;self.frames=staged;Ok(())
 }
 pub fn take(&mut self,context:[u8;16],generation:u32,playing:bool)->Option<Vec<u8>>{
  if context!=self.context||generation!=self.generation||!playing{self.frames.clear();return None;}
  let pending=self.frames.pop_front()?;let elapsed=pending.at.elapsed();
  if elapsed>self.age{self.frames.clear();return None;}
  match encode(self.context,self.generation,pending.sequence,pending.now+elapsed.as_secs_f64(),&pending.commands){Ok(frame)=>Some(frame),Err(_)=>{self.frames.clear();None}}
 }
 pub fn pause(&mut self){self.frames.clear();} // Retain sequence for the same owner.
}
#[cfg(test)]mod tests{use super::*;
 #[test]fn retained_hud_has_no_age_limit_and_batch_failure_is_atomic(){
  let hud=Hud{actor:"peer".into(),name:"雪".into(),sequence:1,hp:3000,max_hp:2000,sampo:true};
  let frame=encode([1;16],1,1,100.,&[Command::Hud(hud.clone())]).unwrap();assert_eq!(frame,encode([1;16],1,1,3600.,&[Command::Hud(hud.clone())]).unwrap());
  assert!(encode([1;16],1,2,102.,&[Command::Hud(hud)]).is_ok());
  let mut q=Queue::new([1;16],1);assert!(q.publish(100.,vec![vec![Command::Pause],vec![Command::Remove("\0".into())]]).is_err());
  assert!(q.take([1;16],1,true).is_none());q.publish(100.,vec![vec![Command::End]]).unwrap();let frame=q.take([1;16],1,true).unwrap();assert_eq!(u32::from_le_bytes(frame[28..32].try_into().unwrap()),1);
 }
 #[test]fn atomic_queue_hold_replay_context_backpressure_and_utf8_bounds(){let mut q=Queue::new([1;16],1);q.publish(10.,vec![vec![Command::Remove("peer".into())]]).unwrap();assert!(q.publish(10.,vec![vec![Command::Pause]]).is_err());assert!(q.take([2;16],1,true).is_none());q.publish(10.,vec![vec![Command::Pause]]).unwrap();let frame=q.take([1;16],1,true).unwrap();assert_eq!(u32::from_le_bytes(frame[28..32].try_into().unwrap()),2);q.pause();assert!(encode([1;16],1,3,10.,&[Command::Remove("\0".into())]).is_err());assert!(encode([1;16],1,3,10.,&[Command::Remove("雪".repeat(129))]).is_err());}
}
