//! Display invalidation only. Revisions never authorize a local effect.
use std::time::{Duration,Instant};

#[derive(Default)]
pub(super) struct Cache {
 generation:u64,
 wanted:u64,
 confirmed:Option<u64>,
 invalidated:bool,
 due:Option<Instant>,
}
impl Cache {
 pub(super) fn invalidate(&mut self,generation:u64,revision:u64,now:Instant)->bool{
  if generation<self.generation{return false}
  let changed=generation!=self.generation;
  if generation!=self.generation{self.generation=generation;self.invalidated=true;self.due=Some(now);}
  if revision>self.wanted{
   self.wanted=revision;
   // Leading-edge deadline: a continuing burst cannot postpone refresh forever.
   if self.confirmed.is_none_or(|v|v<revision)&&self.due.is_none(){self.due=Some(now+Duration::from_millis(150));}
  }
  changed
 }
 pub(super) fn request(&mut self){self.invalidated=true;self.due=None;}
 pub(super) fn ready(&self,now:Instant)->bool{
  (self.invalidated||self.confirmed.is_none()||self.confirmed.is_some_and(|v|v<self.wanted))&&self.due.is_none_or(|at|now>=at)
 }
 pub(super) fn accept(&mut self,revision:u64){
  if self.confirmed.is_none_or(|v|revision>=v){self.confirmed=Some(revision);self.invalidated=false;self.due=None;}
 }
 pub(super) fn revision(&self)->Option<u64>{self.confirmed}
}

#[cfg(test)]mod tests{
 use super::*;
 #[test]fn revision_during_read_is_not_lost_and_duplicates_do_not_read(){
  let now=Instant::now();let mut c=Cache::default();assert!(c.ready(now));c.invalidate(1,4,now);c.accept(4);assert!(!c.ready(now));
  c.invalidate(1,5,now);c.invalidate(1,7,now+Duration::from_millis(100));assert!(!c.ready(now+Duration::from_millis(149)));assert!(c.ready(now+Duration::from_millis(150)));
  c.accept(5);assert!(c.ready(now+Duration::from_millis(150)),"Notification7 arrived during snapshot5");c.accept(7);
  c.invalidate(1,7,now);c.invalidate(1,6,now);assert!(!c.ready(now));assert_eq!(c.revision(),Some(7));
 }
 #[test]fn reconnect_query_and_retired_generation(){
  let now=Instant::now();let mut c=Cache::default();c.invalidate(3,9,now);c.accept(9);assert!(!c.ready(now));
  c.invalidate(4,9,now);assert!(c.ready(now),"Reconnect resync even without a newer notice");c.accept(9);c.invalidate(3,99,now);assert!(!c.ready(now),"Retired stream cannot dirty current cache");
  c.request();assert!(c.ready(now),"Query/open explicit invalidation");c.accept(9);c.accept(8);assert_eq!(c.revision(),Some(9));
 }
}
