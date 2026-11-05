//! Bounded non-secret game mailbox. Pipe thread never performs network I/O.
use super::*;
use nt2_desktop_g0::rejoin_local::Association;
#[derive(Clone,PartialEq,Eq)]
pub(crate) struct Contact{pub pid:crate::local_owner::Id,pub context:[u8;16],pub association:[u8;16],pub actual:u64}
#[derive(Default)]
pub(crate) struct Mailbox{
 pub world_offered:Option<(crate::local_owner::Id,nt2_desktop_g0::world_local::Marker)>,pub world_context:Option<[u8;16]>,
 pub metadata:Option<(crate::local_owner::Id,[u8;16],Vec<u8>)>,pub offered:Option<(crate::local_owner::Id,Association)>,pub offered_context:Option<[u8;16]>,pub lookup:u32,
 pub waiting_offered:Option<(crate::local_owner::Id,nt2_desktop_g0::startup_local::WaitingMarker)>,pub waiting_context:Option<[u8;16]>,
 pub loaded_saved:bool,pub lookup_context:Option<[u8;16]>,
 pub contact:Option<Contact>,pub admitted:Option<Contact>,pub terminal:bool,pub offline:bool,
}
impl Mailbox{
 pub fn transition(&mut self,pending:&Option<super::super::handoff::Pending>,pid:crate::local_owner::Id,bytes:&[u8])->Vec<u8>{
  if bytes.len()<32{return vec![0;48]}
  let field=|i|u32::from_le_bytes(bytes[i..i+4].try_into().unwrap());let op=field(8);let version=field(4);
  let context:[u8;16]=bytes[16..32].try_into().unwrap();let mut reply=vec![0;if op==13{1072}else{48}];
  reply[..4].copy_from_slice(&super::super::handoff::MAGIC.to_le_bytes());reply[4..8].copy_from_slice(&version.to_le_bytes());reply[16..32].copy_from_slice(&context);
  if version==7&&op==12&&bytes.len()==56&&field(12)==0{
   if context==[0;16]{return reply}
   let contact=Contact{pid,context,association:bytes[32..48].try_into().unwrap(),actual:u64::from_le_bytes(bytes[48..56].try_into().unwrap())};
   let known=self.world_offered.as_ref().is_some_and(|(owner,a)|*owner==pid&&a.local_world.as_bytes()==&contact.association&&a.actual_seed==contact.actual.to_string())||self.offered.as_ref().is_some_and(|(owner,a)|*owner==pid&&a.local_world.as_bytes()==&contact.association&&a.actual_seed==contact.actual.to_string())||self.waiting_offered.as_ref().is_some_and(|(owner,a)|*owner==pid&&a.local_world.as_bytes()==&contact.association&&a.actual_seed==contact.actual.to_string());
   if self.terminal||self.offline{if known{self.contact=Some(contact);}reply[12..16].copy_from_slice(&(if self.offline{6u32}else{4u32}).to_le_bytes());return reply}
   if self.admitted.as_ref()==Some(&contact){
    if let Some(p)=pending.as_ref().filter(|p|p.native_v2&&p.owner==Some((pid,context))&&p.actual_seed==Some(contact.actual)&&p.generation.as_ref().is_some_and(|(g,n)|g.load(Ordering::Acquire)==*n)){
     let lease=p.collection_until.unwrap_or(p.authority_until).saturating_duration_since(Instant::now()).as_millis().min(12000) as u32;
     if lease>0{reply[8..12].copy_from_slice(&1u32.to_le_bytes());reply[12..16].copy_from_slice(&p.world_policy.to_le_bytes());reply[32..36].copy_from_slice(&lease.to_le_bytes());
      let generation=p.native_epoch.unwrap_or_else(||p.generation.as_ref().map(|(_,n)|u32::try_from(*n).unwrap_or(0)).unwrap_or(0));reply[36..40].copy_from_slice(&generation.to_le_bytes());return reply}
    }
   }
   if known{
    let owner_active=pending.as_ref().is_some_and(|p|p.collection_until.unwrap_or(p.authority_until)>Instant::now()&&p.owner!=Some((pid,context)));
    if self.contact.as_ref().is_none_or(|old|old==&contact)||!owner_active{self.contact=Some(contact);reply[12..16].copy_from_slice(&3u32.to_le_bytes());}
   }
  }else if version==8&&op==13&&bytes.len()==32&&field(12)==0{
   if let Some((owner,token,metadata))=&self.metadata{if *owner==pid&&*token==context&&metadata.len()<=1024{
    reply[8..12].copy_from_slice(&1u32.to_le_bytes());reply[12..16].copy_from_slice(&(metadata.len() as u32).to_le_bytes());reply[48..48+metadata.len()].copy_from_slice(metadata);
   }}
  }else if version==8&&op==14&&field(12)>0&&field(12)<=1024&&bytes.len()==32+field(12) as usize{
   let shape:serde_json::Value=match serde_json::from_slice(&bytes[32..]){Ok(v)=>v,Err(_)=>return reply};
   if shape.get("kind").and_then(|v|v.as_str())==Some("nt2-world"){
    let Ok(marker)=nt2_desktop_g0::world_local::Marker::decode(&bytes[32..])else{return reply};let active=pending.as_ref().is_some_and(|p|p.collection_until.unwrap_or(p.authority_until)>Instant::now()&&p.owner.is_some_and(|owner|owner!=(pid,context)));
    if active{return reply}
    // The first selector lookup may precede creation of the native context.
    // Reading terminal/offline intent from the matching admitted conversation
    // must not reset its lookup/contact or turn it into another world restore.
    let current=self.admitted.as_ref().or(self.contact.as_ref()).is_some_and(|c|c.pid==pid&&c.context==context&&c.association==*marker.local_world.as_bytes()&&c.actual.to_string()==marker.actual_seed);
    let current_metadata=current&&self.metadata.as_ref().is_some_and(|(owner,ctx,bytes)|*owner==pid&&*ctx==context&&nt2_desktop_g0::world_local::Marker::decode(bytes).ok().as_ref()==Some(&marker));
    if current_metadata{self.world_offered=Some((pid,marker));self.world_context=Some(context);}
    else if self.world_offered.as_ref()!=Some(&(pid,marker.clone()))||self.world_context!=Some(context)&&!current{self.world_offered=Some((pid,marker));self.world_context=Some(context);self.contact=None;self.admitted=None;self.lookup=4;self.offline=false;self.terminal=false;}
    reply[8..12].copy_from_slice(&1u32.to_le_bytes());reply[12..16].copy_from_slice(&self.lookup.to_le_bytes());return reply
   }
   if shape.get("kind").is_some(){
    // Any explicit kind is disjoint: malformed waiting never becomes admitted.
    let Ok(marker)=nt2_desktop_g0::startup_local::WaitingMarker::decode(&bytes[32..])else{return reply};
    let active=pending.as_ref().is_some_and(|p|p.collection_until.unwrap_or(p.authority_until)>Instant::now()&&p.owner.is_some_and(|owner|owner!=(pid,context)));
    // Cached metadata identifies a saved world, not its current live owner.
    // An old metadata tuple must not override a different current native lease.
    if active{return reply}
    if self.waiting_offered.as_ref().is_none_or(|(owner,a)|*owner==pid&&a==&marker)||!active{
     if self.waiting_offered.as_ref().is_none_or(|(owner,a)|*owner!=pid||a!=&marker)||self.waiting_context!=Some(context){self.waiting_offered=Some((pid,marker));self.waiting_context=Some(context);self.contact=None;self.admitted=None;self.lookup=4;self.offline=false;self.terminal=false;}
     self.offered=None;self.offered_context=None;reply[8..12].copy_from_slice(&1u32.to_le_bytes());reply[12..16].copy_from_slice(&self.lookup.to_le_bytes());
    }
    return reply
   }
   if let Ok(a)=Association::decode(&bytes[32..]){
    if self.waiting_offered.is_some(){return reply}
    let owner_active=pending.as_ref().is_some_and(|p|p.collection_until.unwrap_or(p.authority_until)>Instant::now()&&p.owner.is_some_and(|(owner,_)|owner!=pid));
    if !owner_active&&self.offered.as_ref().is_some_and(|(owner,current)|*owner!=pid&&current==&a){self.offered=None;self.offered_context=None;self.contact=None;self.admitted=None;}
    if self.offered.as_ref().is_none_or(|(owner,current)|*owner==pid&&current==&a){
     if self.offered.is_none(){self.offered=Some((pid,a));self.offered_context=Some(context);self.lookup=4;}
     // A verified loaded-save lookup is distinct from the playing world's
     // original Start intent. Fence the first answer until Core reviews it.
     if self.metadata.as_ref().is_some_and(|(owner,_,_)|*owner==pid)&&self.lookup_context!=Some(context){
      self.loaded_saved=true;self.lookup_context=Some(context);self.lookup=4;
     }
     reply[8..12].copy_from_slice(&1u32.to_le_bytes());reply[12..16].copy_from_slice(&self.lookup.to_le_bytes());
    }
   }
  }else if version==8&&op==15&&bytes.len()==32{
   let scoped=self.world_offered.as_ref().is_some_and(|(owner,a)|*owner==pid&&(self.world_context==Some(context)||self.contact.as_ref().is_some_and(|c|c.pid==pid&&c.context==context&&a.local_world.as_bytes()==&c.association)))||self.offered.as_ref().is_some_and(|(owner,a)|*owner==pid&&(self.offered_context==Some(context)||self.contact.as_ref().is_some_and(|c|c.pid==pid&&c.context==context&&a.local_world.as_bytes()==&c.association)));
   let scoped=scoped||self.waiting_offered.as_ref().is_some_and(|(owner,a)|*owner==pid&&(self.waiting_context==Some(context)||self.contact.as_ref().is_some_and(|c|c.pid==pid&&c.context==context&&a.local_world.as_bytes()==&c.association)));
   if scoped{self.offline=true;self.lookup=5;reply[8..12].copy_from_slice(&1u32.to_le_bytes());}
  }
  reply
 }
}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn lc1_current_metadata_preserves_contact_only_for_exact_marker(){
  let marker=nt2_desktop_g0::world_local::Marker{kind:"nt2-world".into(),version:1,local_world:uuid::Uuid::new_v4(),bundle_digest:"a".repeat(64),actual_seed:"424242".into(),loot:"shared:0:424242".into()};
  for changed in [false,true]{let mut m=Mailbox::default();let contact=Contact{pid:nt2_local_ws::owner::Id::OsProcess(77),context:[2;16],association:*marker.local_world.as_bytes(),actual:424242};m.contact=Some(contact.clone());m.admitted=Some(contact);m.metadata=Some((nt2_local_ws::owner::Id::OsProcess(77),[2;16],marker.encode().unwrap()));m.lookup=2;
   let mut offered=marker.clone();if changed{offered.bundle_digest="b".repeat(64);}let body=offered.encode().unwrap();let mut request=vec![0;32];request[..4].copy_from_slice(&crate::handoff::MAGIC.to_le_bytes());request[4..8].copy_from_slice(&8u32.to_le_bytes());request[8..12].copy_from_slice(&14u32.to_le_bytes());request[12..16].copy_from_slice(&(body.len() as u32).to_le_bytes());request[16..32].fill(2);request.extend(body);
   assert_eq!(m.transition(&None,nt2_local_ws::owner::Id::OsProcess(77),&request)[8],1);assert_eq!(m.contact.is_some(),!changed);assert_eq!(m.admitted.is_some(),!changed);assert_eq!(m.lookup,if changed{4}else{2});assert_eq!(m.world_context,Some([2;16]));
  }
 }
 #[test]fn waiting_mailbox_kind_scope_malformed_and_shared_contact(){
  let marker=nt2_desktop_g0::startup_local::WaitingMarker{kind:"nt2-waiting".into(),version:1,local_world:uuid::Uuid::new_v4(),bundle_digest:"a".repeat(64),actual_seed:"424242".into(),loot:"shared:0:424242".into()};let data=marker.encode().unwrap();let mut m=Mailbox::default();
  let mut lookup=vec![0u8;32];lookup[..4].copy_from_slice(&crate::handoff::MAGIC.to_le_bytes());lookup[4..8].copy_from_slice(&8u32.to_le_bytes());lookup[8..12].copy_from_slice(&14u32.to_le_bytes());lookup[12..16].copy_from_slice(&(data.len() as u32).to_le_bytes());lookup.extend_from_slice(&data);
  assert_eq!(m.transition(&None,nt2_local_ws::owner::Id::OsProcess(77),&lookup)[12],4);assert!(m.offered.is_none());assert_eq!(m.waiting_offered.as_ref().unwrap().1,marker);
  let mut contact=vec![0u8;56];contact[..4].copy_from_slice(&crate::handoff::MAGIC.to_le_bytes());contact[4..8].copy_from_slice(&7u32.to_le_bytes());contact[8..12].copy_from_slice(&12u32.to_le_bytes());contact[16..32].fill(5);contact[32..48].copy_from_slice(marker.local_world.as_bytes());contact[48..56].copy_from_slice(&424242u64.to_le_bytes());
  assert_eq!(m.transition(&None,nt2_local_ws::owner::Id::OsProcess(78),&contact)[12],0);assert!(m.contact.is_none());assert_eq!(m.transition(&None,nt2_local_ws::owner::Id::OsProcess(77),&contact)[12],3);assert!(m.contact.is_some());assert!(m.admitted.is_none());
  let original=m.contact.clone();let mut p=crate::handoff::new(424242,"run".into(),1);p.owner=Some((nt2_local_ws::owner::Id::OsProcess(77),[5;16]));p.authority_until=Instant::now()+Duration::from_secs(6);let mut foreign=lookup.clone();foreign[16..32].fill(9);assert_eq!(m.transition(&Some(p.clone()),nt2_local_ws::owner::Id::OsProcess(78),&foreign)[8],0);assert!(m.contact==original);
  let mut bad:serde_json::Value=serde_json::from_slice(&data).unwrap();bad["original_start"]=serde_json::json!("fake");let b=serde_json::to_vec(&bad).unwrap();let mut malformed=lookup[..32].to_vec();malformed[12..16].copy_from_slice(&(b.len() as u32).to_le_bytes());malformed.extend_from_slice(&b);assert_eq!(m.transition(&None,nt2_local_ws::owner::Id::OsProcess(77),&malformed)[8],0);assert!(m.offered.is_none());assert!(m.contact==original);
  let mut offline=lookup[..32].to_vec();offline[8..12].copy_from_slice(&15u32.to_le_bytes());offline[12..16].fill(0);offline[16..32].fill(5);assert_eq!(m.transition(&None,nt2_local_ws::owner::Id::OsProcess(78),&offline)[8],0);assert_eq!(m.transition(&None,nt2_local_ws::owner::Id::OsProcess(77),&offline)[8],1);assert!(m.offline);
 }
 fn a()->Association{Association{version:1,local_world:uuid::Uuid::new_v4(),origin:"https://fixture.invalid".into(),service:"test-service".into(),run:"run".into(),participant:"participant".into(),room:"room".into(),world_token:"1".repeat(32),account:"account".into(),original_start:"start".into(),config_revision:1,actual_seed:"424242".into(),seed_policy:"shared-fixed".into(),loot_policy:"shared:0:424242".into(),frozen_config_digest:"a".repeat(64)}}
 fn request(op:u32,ctx:[u8;16],a:&Association)->Vec<u8>{let mut b=vec![0;56];b[..4].copy_from_slice(&crate::handoff::MAGIC.to_le_bytes());b[4..8].copy_from_slice(&7u32.to_le_bytes());b[8..12].copy_from_slice(&op.to_le_bytes());b[16..32].copy_from_slice(&ctx);b[32..48].copy_from_slice(a.local_world.as_bytes());b[48..56].copy_from_slice(&424242u64.to_le_bytes());b}
 #[test]fn rejoin_mailbox_known_metadata_never_grants_and_competing_owner_is_fenced(){
  let a=a();let mut m=Mailbox::default();let frame=request(12,[2;16],&a);
  assert_eq!(m.transition(&None,nt2_local_ws::owner::Id::OsProcess(1),&frame)[8],0);assert!(m.contact.is_none());
  m.offered=Some((nt2_local_ws::owner::Id::OsProcess(1),a.clone()));let reply=m.transition(&None,nt2_local_ws::owner::Id::OsProcess(1),&frame);assert_eq!(reply[8],0);assert_eq!(reply[12],3);
  let old=m.contact.clone();let mut p=crate::handoff::new(424242,"run".into(),1);p.owner=Some((nt2_local_ws::owner::Id::OsProcess(1),[1;16]));p.authority_until=Instant::now()+Duration::from_secs(6);
  let competing=request(12,[3;16],&a);assert_eq!(m.transition(&Some(p),nt2_local_ws::owner::Id::OsProcess(1),&competing)[8],0);assert!(m.contact==old);
  let mut foreign=frame.clone();foreign[48..56].copy_from_slice(&42u64.to_le_bytes());assert_eq!(m.transition(&None,nt2_local_ws::owner::Id::OsProcess(1),&foreign)[8],0);assert!(m.contact==old);
  m.terminal=true;assert_eq!(m.transition(&None,nt2_local_ws::owner::Id::OsProcess(1),&frame)[12],4);m.offline=true;assert_eq!(m.transition(&None,nt2_local_ws::owner::Id::OsProcess(1),&frame)[12],6);
 }
 #[test]fn rejoin_mailbox_admission_requires_current_cancel_fence_and_finite_lease(){
  let a=a();let mut m=Mailbox::default();m.offered=Some((nt2_local_ws::owner::Id::OsProcess(1),a.clone()));let c=Contact{pid:nt2_local_ws::owner::Id::OsProcess(1),context:[2;16],association:*a.local_world.as_bytes(),actual:424242};m.admitted=Some(c);
  let fence=Arc::new(std::sync::atomic::AtomicU64::new(7));let mut p=crate::handoff::new(424242,"run".into(),1);p.owner=Some((nt2_local_ws::owner::Id::OsProcess(1),[2;16]));p.native_v2=true;p.actual_seed=Some(424242);p.generation=Some((fence.clone(),7));p.authority_until=Instant::now()+Duration::from_secs(30);
  let request=request(12,[2;16],&a);let reply=m.transition(&Some(p.clone()),nt2_local_ws::owner::Id::OsProcess(1),&request);assert_eq!(reply[8],1);assert_eq!(u32::from_le_bytes(reply[32..36].try_into().unwrap()),12000);
  fence.store(8,Ordering::Release);assert_eq!(m.transition(&Some(p.clone()),nt2_local_ws::owner::Id::OsProcess(1),&request)[8],0);
  fence.store(7,Ordering::Release);p.authority_until=Instant::now();assert_eq!(m.transition(&Some(p),nt2_local_ws::owner::Id::OsProcess(1),&request)[8],0);
 }
 #[test]fn rejoin_offline_choice_cannot_retire_a_replacement_context(){
  let a=a();let mut m=Mailbox::default();m.offered=Some((nt2_local_ws::owner::Id::OsProcess(1),a.clone()));m.offered_context=Some([2;16]);
  let mut request=vec![0u8;32];request[..4].copy_from_slice(&crate::handoff::MAGIC.to_le_bytes());request[4..8].copy_from_slice(&8u32.to_le_bytes());request[8..12].copy_from_slice(&15u32.to_le_bytes());request[16..32].fill(1);
  assert_eq!(m.transition(&None,nt2_local_ws::owner::Id::OsProcess(1),&request)[8],0);assert!(!m.offline);
  request[16..32].fill(2);assert_eq!(m.transition(&None,nt2_local_ws::owner::Id::OsProcess(2),&request)[8],0);assert!(!m.offline);
  assert_eq!(m.transition(&None,nt2_local_ws::owner::Id::OsProcess(1),&request)[8],1);assert!(m.offline);
 }

}
