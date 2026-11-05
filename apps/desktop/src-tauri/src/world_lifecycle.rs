//! Pure LC1 coordinator. No HTTP, filesystem or credential-store calls here.
use serde::Serialize;
use std::time::Instant;
#[derive(Clone,Copy,Debug,PartialEq,Eq,Serialize)]pub enum Stage{WaitingForNoita,Loading,Checking,Ready,Playing,Reconnecting,Choice,Ended,Offline}
#[derive(Clone,Debug,PartialEq,Eq)]pub struct Ticket{pub world:uuid::Uuid,pub revision:u64,pub native:(crate::local_owner::Id,[u8;16]),pub attempt:u64}
#[derive(Default)]pub struct Coordinator{revision:u64,pub ticket:Option<Ticket>,stage:Option<Stage>,error:Option<(Stage,String)>,pub native_seen:Option<Instant>,confirmed_until:Option<Instant>,observation_at:Option<Instant>,last_confirmed:Option<Stage>}
impl Coordinator{
 pub fn bind(&mut self,world:uuid::Uuid,native:(crate::local_owner::Id,[u8;16]),attempt:u64)->Ticket{self.revision=self.revision.checked_add(1).expect("LC1 revision exhausted");let t=Ticket{world,revision:self.revision,native,attempt};self.ticket=Some(t.clone());self.stage=Some(Stage::Reconnecting);self.error=None;self.confirmed_until=None;self.observation_at=None;self.last_confirmed=None;t}
 pub fn current(&self,t:&Ticket)->bool{self.ticket.as_ref()==Some(t)}
 pub fn advance(&mut self,t:&Ticket,stage:Stage)->bool{if !self.current(t){return false}if self.stage!=Some(stage){self.error=None;self.stage=Some(stage);self.confirmed_until=None;}true}
 pub fn fail(&mut self,t:&Ticket,reason:&str)->bool{if !self.current(t){return false}let stage=self.stage.unwrap_or(Stage::Reconnecting);self.error=Some((stage,reason.chars().take(240).collect()));true}
 pub fn intent(&mut self,stage:Stage){self.revision=self.revision.checked_add(1).expect("LC1 revision exhausted");self.ticket=None;self.stage=Some(stage);self.error=None;self.native_seen=None;self.confirmed_until=None;self.observation_at=None;self.last_confirmed=None;}
 pub fn contact(&mut self,t:&Ticket,at:Instant)->bool{if !self.current(t){return false}self.native_seen=Some(at);true}
 pub fn waiting_for_game(&self)->bool{(matches!(self.stage,Some(Stage::Ready|Stage::Playing))||self.stage==Some(Stage::Checking)&&self.last_confirmed.is_some())&&self.observation_at.is_none_or(|at|at.elapsed()>=std::time::Duration::from_secs(6))}
 pub fn protected_wait(&mut self,t:&Ticket,observation_at:Instant)->bool{if !self.current(t){return false}self.stage=Some(Stage::Checking);self.error=None;self.confirmed_until=None;self.observation_at=Some(observation_at);true}
 pub fn stage(&self)->Stage{let stage=self.stage.unwrap_or(Stage::WaitingForNoita);if matches!(stage,Stage::Ready|Stage::Playing)&&(self.confirmed_until.is_none_or(|until|Instant::now()>=until)||self.waiting_for_game()){Stage::Checking}else{stage}}
 pub fn observe(&mut self,t:&Ticket,at:Instant)->bool{if !self.current(t)||at>Instant::now()||self.observation_at.is_some_and(|old|old>at){return false}self.observation_at=Some(at);true}
 pub fn invalidate_observation(&mut self,t:&Ticket)->bool{if !self.current(t){return false}self.observation_at=None;if let Some(until)=self.confirmed_until.as_mut(){*until=(*until).min(Instant::now());}true}
 pub fn reason(&self)->Option<&str>{self.error.as_ref().map(|(_,s)|s.as_str())}
 pub fn block(&mut self,reason:&str){self.error=Some((self.stage(),reason.chars().take(240).collect()));}
 pub fn complete(&mut self,t:&Ticket,stage:Stage)->bool{if !self.current(t){return false}self.stage=Some(stage);self.error=None;self.confirmed_until=None;self.observation_at=None;true}
 pub fn complete_until(&mut self,t:&Ticket,stage:Stage,until:Instant,observation_at:Instant)->bool{if !self.complete(t,stage){return false}self.confirmed_until=Some(until);self.observation_at=Some(observation_at);self.last_confirmed=Some(stage);true}
 pub fn view(&self)->serde_json::Value{serde_json::json!({"state":self.stage(),"reason":self.reason(),"play_guard":if !matches!(self.stage,Some(Stage::Ready|Stage::Playing)){"canonical_revalidation_required"}else if self.observation_at.is_none(){"observation_unvalidated"}else if self.confirmed_until.is_none_or(|until|Instant::now()>=until){"canonical_confirmation_expired"}else if self.waiting_for_game(){"game_observation_stale"}else{"current"},"waiting_for_game":self.waiting_for_game(),"last_confirmed_state":self.last_confirmed,"confirmation_remaining_ms":self.confirmed_until.map(|until|until.saturating_duration_since(Instant::now()).as_millis().min(30000)),"observation_age_ms":self.observation_at.map(|at|at.elapsed().as_millis().min(3600000)),"native_contact_age_ms":self.native_seen.map(|at|at.elapsed().as_millis() as u64)})}
}
#[cfg(test)]mod tests{use super::*;
#[test]fn lc1_optional_success_cannot_clear_recovery_error_or_accept_old_ticket(){let mut c=Coordinator::default();let world=uuid::Uuid::new_v4();let old=c.bind(world,(nt2_local_ws::owner::Id::OsProcess(1),[1;16]),1);c.fail(&old,"OWNER_ACTIVE");c.contact(&old,Instant::now());c.advance(&old,Stage::Reconnecting);assert_eq!(c.reason(),Some("OWNER_ACTIVE"));let current=c.bind(world,(nt2_local_ws::owner::Id::OsProcess(2),[2;16]),2);assert!(!c.advance(&old,Stage::Playing));assert!(!c.fail(&old,"late reply"));c.complete_until(&current,Stage::Ready,Instant::now()+std::time::Duration::from_secs(6),Instant::now());assert_eq!(c.stage(),Stage::Ready);}
#[test]fn lc1_presence_contact_and_fresh_sample_cannot_resurrect_expired_confirmation(){
 let mut c=Coordinator::default();let t=c.bind(uuid::Uuid::new_v4(),(nt2_local_ws::owner::Id::OsProcess(1),[1;16]),1);let now=Instant::now();
 c.complete_until(&t,Stage::Ready,now+std::time::Duration::from_secs(30),now-std::time::Duration::from_secs(31));c.contact(&t,Instant::now());assert_eq!(c.stage(),Stage::Checking);assert!(c.waiting_for_game());assert_eq!(c.reason(),None);assert_eq!(c.view()["last_confirmed_state"],"Ready");
 c.complete_until(&t,Stage::Playing,now-std::time::Duration::from_secs(1),now);assert_eq!(c.stage(),Stage::Checking);assert!(!c.waiting_for_game());c.observe(&t,Instant::now());assert_eq!(c.stage(),Stage::Checking,"An engine observation is not renewed canonical authority");
 c.complete_until(&t,Stage::Ready,Instant::now()+std::time::Duration::from_secs(6),Instant::now());assert_eq!(c.stage(),Stage::Ready);c.invalidate_observation(&t);c.observe(&t,Instant::now());assert_eq!(c.stage(),Stage::Checking,"Changed content must revalidate, not revive the old grant");
 c.fail(&t,"UNAUTHENTICATED");assert!(c.protected_wait(&t,now-std::time::Duration::from_secs(31)));assert_eq!(c.stage(),Stage::Checking);assert!(c.waiting_for_game());assert_eq!(c.reason(),None);c.observe(&t,Instant::now());assert_eq!(c.stage(),Stage::Checking,"Held acknowledgement plus fresh observation still needs canonical validation");
 let replacement=c.bind(uuid::Uuid::new_v4(),(nt2_local_ws::owner::Id::OsProcess(1),[2;16]),2);assert!(!c.complete_until(&t,Stage::Playing,Instant::now()+std::time::Duration::from_secs(30),Instant::now()));assert!(!c.observe(&t,Instant::now()));assert!(c.current(&replacement));assert_eq!(c.stage(),Stage::Reconnecting);
}
}
