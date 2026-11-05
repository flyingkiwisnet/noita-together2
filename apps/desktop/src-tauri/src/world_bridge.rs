//! LC1 early preparation slot. Same lifecycle record as active/recovered worlds;
//! this only retains the irreversible local fresh-generation consumption facts.
use super::*;
pub struct Bridge{pub pending:Arc<Mutex<Option<crate::handoff::Pending>>>,pub lane:startup::Lane}
impl Bridge{
 pub fn new()->Self{let pending=Arc::new(Mutex::new(None));let lane=startup::Lane::new(pending.clone());Self{pending,lane}}
 pub fn owns(&self,pid:crate::local_owner::Id,context:&[u8])->bool{self.pending.lock().unwrap().as_ref().is_some_and(|p|p.owner.is_some_and(|o|o.0==pid&&o.1.as_slice()==context))}
 pub fn frame(&self,pid:crate::local_owner::Id,bytes:&[u8])->Vec<u8>{c2_transition(&mut self.pending.lock().unwrap(),pid,bytes)}
 pub fn marker(&self,pid:crate::local_owner::Id,context:[u8;16])->Option<Vec<u8>>{let slot=self.pending.lock().unwrap();let p=slot.as_ref()?;if p.owner!=Some((pid,context))||!p.accepted{return None}let b=p.startup_bundle.as_ref()?;let actual=p.actual_seed?;nt2_desktop_g0::world_local::Marker{kind:"nt2-world".into(),version:1,local_world:b.world(),bundle_digest:b.digest().into(),actual_seed:actual.to_string(),loot:if p.world_policy==2{"individual:0".into()}else{format!("shared:{}:{actual}",u8::from(p.world_policy==5))}}.encode().ok()}
}
