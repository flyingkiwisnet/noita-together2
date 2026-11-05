//! Receiver-monotonic samples. No server wall clock is used for permission.
use std::{collections::VecDeque,time::{Duration,Instant}};
#[derive(Clone,Copy,Debug)]
pub struct Stamp {pub sequence:u64,pub started:Instant}
#[derive(Clone,Default)]
pub struct Deadline {last:Option<(Stamp,Instant)>}
impl Deadline {
 pub fn install(&mut self,stamp:Stamp,remaining_ms:u32,maximum_ms:u32,ceiling_ms:u32,received:Instant)->Result<Instant,String>{
  if stamp.sequence==0||stamp.started>received||remaining_ms>maximum_ms{return Err("Invalid correlated validity sample".into());}
  let proposed=stamp.started.checked_add(Duration::from_millis(u64::from(remaining_ms.min(ceiling_ms)))).ok_or("Validity deadline overflow")?;
  let until=match self.last{
   Some((old,_until)) if stamp.sequence<old.sequence=>return Err("Stale validity sample".into()),
   Some((old,until)) if stamp.sequence==old.sequence=>{if stamp.started!=old.started{return Err("Retagged duplicate validity sample".into());}until.min(proposed)},
   Some((old,_)) if stamp.started<old.started=>return Err("Validity request start moved backward".into()),
   _=>proposed,
  };
  self.last=Some((stamp,until));Ok(until)
 }
 pub fn shorten(&mut self,until:Instant){if let Some((_,old))=self.last.as_mut(){*old=(*old).min(until);}}
 pub fn until(&self)->Option<Instant>{self.last.map(|v|v.1)}
 pub fn live(&self,at:Instant)->bool{self.until().is_some_and(|until|at<until)}
 pub fn remaining(&self,at:Instant)->Duration{self.until().map(|v|v.saturating_duration_since(at)).unwrap_or_default()}
}
#[derive(Default)]pub struct Acknowledgements {sent:VecDeque<Stamp>,last:u64}
impl Acknowledgements {
 pub fn sent(&mut self,sequence:u64,at:Instant)->Result<(),String>{
  if sequence==0||sequence<=self.last{return Err("Nonincreasing outbound frame".into());}self.last=sequence;self.sent.push_back(Stamp{sequence,started:at});while self.sent.len()>32{self.sent.pop_front();}Ok(())
 }
 pub fn acknowledged(&self,sequence:u64,received:Instant)->Result<Stamp,String>{
  self.sent.iter().find(|s|s.sequence==sequence&&s.started<=received).copied().ok_or("Unknown acknowledged frame".into())
 }
}
pub fn relative_age(age_ms:u32,stamp:Stamp,received:Instant)->Result<Duration,String>{
 if age_ms>2000||stamp.started>received{return Err("Invalid relative movement age".into());}
 let age=Duration::from_millis(u64::from(age_ms)).saturating_add(received.duration_since(stamp.started));
 if age>=Duration::from_secs(2){return Err("Expired relative movement sample".into());}Ok(age)
}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn cr2_deadline_whole_attempt_duplicate_delay_and_clock_independence(){
  let start=Instant::now();let receive=start+Duration::from_secs(3);let stamp=Stamp{sequence:1,started:start};let mut d=Deadline::default();
  let initial=d.install(stamp,12000,30000,12000,receive).unwrap();assert_eq!(initial,start+Duration::from_secs(12));assert_eq!(d.remaining(receive),Duration::from_secs(9));
  assert_eq!(d.install(stamp,30000,30000,12000,start+Duration::from_secs(11)).unwrap(),initial);assert!(!d.live(start+Duration::from_secs(12)));
  assert!(d.install(Stamp{started:receive,..stamp},12000,30000,12000,receive).is_err());assert!(d.install(Stamp{sequence:2,started:receive},30001,30000,12000,receive).is_err());
  let next=Stamp{sequence:2,started:receive};assert_eq!(d.install(next,2000,30000,12000,receive).unwrap(),receive+Duration::from_secs(2));assert!(d.install(stamp,12000,30000,12000,receive).is_err());
  for _wall_clock_offset in [-86400000i64,0,86400000]{let mut d=Deadline::default();assert_eq!(d.install(stamp,900000,900000,900000,receive).unwrap(),start+Duration::from_secs(900));}
 }
 #[test]fn cr2_acknowledged_frames_bounded_and_delayed_ages_expire(){
  let start=Instant::now();let mut a=Acknowledgements::default();for n in 1..=64{a.sent(n,start+Duration::from_millis(n*10)).unwrap();}assert_eq!(a.sent.len(),32);assert!(a.acknowledged(1,start+Duration::from_secs(1)).is_err());
  let at=start+Duration::from_secs(1);let stamp=a.acknowledged(64,at).unwrap();assert_eq!(relative_age(500,stamp,at).unwrap(),Duration::from_millis(860));assert!(relative_age(1700,stamp,at).is_err());assert!(relative_age(2001,stamp,at).is_err());assert!(a.sent(64,at).is_err());
 }
}
