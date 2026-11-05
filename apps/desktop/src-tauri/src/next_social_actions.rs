//! Durable bounded A3 originals. Retired owners never replay transient actions.
use super::*;
use nt2_desktop_g0::{g3_a2_local::SocialOperation,g3_a3_frame,protocol_g3_social::wire};
fn kind(p:&wire::Publish)->&'static str{if p.emote.is_some(){"emote"}else{"cosmetic"}}
async fn ensure(owner:&Owner,store:Arc<Store>)->Result<(),String>{let key=owner.original_owner();if stored(store.clone()).await?.social_owner!=key{record(store,move|s|{s.social_owner=key;s.social.clear();s.social_pending.clear();Ok(())}).await?;}Ok(())}
pub(super) async fn accept(owner:&mut Owner,store:Arc<Store>,id:String,payload:wire::Publish)->Result<(),String>{
 ensure(owner,store.clone()).await?;let previous=stored(store.clone()).await?;let k=kind(&payload);
 if let Some(op)=previous.social.get(&id){if serde_json::to_vec(&op.publish.emote).unwrap()!=serde_json::to_vec(&payload.emote).unwrap()||serde_json::to_vec(&op.publish.cosmetic).unwrap()!=serde_json::to_vec(&payload.cosmetic).unwrap(){return Err("Substituted A3 local original".into())}return Ok(())}
 let floor=previous.social.values().filter(|op|kind(&op.publish)==k).filter_map(|op|op.publish.sequence).max().unwrap_or(0);owner.sequence_floor(k,floor);let event=owner.begin(payload)?;let request:wire::Request=serde_json::from_slice(owner.original(&event)?).map_err(|_|"A3 original encoding")?;let publish=request.publish.ok_or("A3 original payload")?;
 record(store,move|s|{
  if let Some(old)=s.social_pending.remove(k){if let Some(op)=s.social.get_mut(&old){op.status=2;op.reason="Superseded by a newer social action".into();}}
  while s.social.values().filter(|op|kind(&op.publish)==k).count()>=64{let key=s.social.iter().filter(|(_,op)|kind(&op.publish)==k&&op.status!=0).min_by_key(|(_,op)|op.publish.sequence).map(|(id,_)|id.clone()).ok_or("A3 original capacity")?;s.social.remove(&key);}
  s.social.insert(id.clone(),SocialOperation{publish,status:0,reason:String::new(),uncertain:false});s.social_pending.insert(k.into(),id);Ok(())
 }).await?;Ok(())
}
pub(super) async fn reconcile(store:Arc<Store>,p:&wire::Projection)->Result<(),String>{
 let event=p.event_id.clone().ok_or("A3 snapshot original")?;let is_emote=p.emote.is_some();
 if !stored(store.clone()).await?.social.values().any(|op|op.status==0&&op.publish.event_id.as_ref()==Some(&event)&&op.publish.emote.is_some()==is_emote){return Ok(())}
 record(store,move|s|{
  let found=s.social.iter().find(|(_,op)|op.status==0&&op.publish.event_id.as_ref()==Some(&event)&&op.publish.emote.is_some()==is_emote).map(|(id,op)|(id.clone(),kind(&op.publish).to_owned()));
  if let Some((id,k))=found{let op=s.social.get_mut(&id).unwrap();op.status=1;op.reason.clear();if s.social_pending.get(&k)==Some(&id){s.social_pending.remove(&k);}}Ok(())
 }).await?;Ok(())
}
pub(super) async fn pump(owner:&mut Owner,store:Arc<Store>)->Result<Vec<Command>,String>{
 ensure(owner,store.clone()).await?;
 for k in ["emote","cosmetic"]{
  let current=stored(store.clone()).await?;let Some(id)=current.social_pending.get(k).cloned()else{continue};let op=current.social.get(&id).ok_or("Missing A3 pending original")?.clone();let event=owner.restore_social(op.publish.clone())?;
  let identity=id.clone();record(store.clone(),move|s|{if let Some(op)=s.social.get_mut(&identity){op.uncertain=true;}Ok(())}).await?;
  let sent=owner.send_original(&event).await;let cleanup=op.publish.emote.as_ref().is_some_and(|e|e.active==Some(false));
  let status=match &sent{Ok(_)=>1,Err(e)if !cleanup&&!op.uncertain&&matches!(e.as_str(),"CAPACITY"|"FORBIDDEN"|"STALE_EPOCH"|"UNAUTHENTICATED"|"CONFLICT"|"INVALID_MESSAGE")=>2,_=>0};
  // CAPACITY is also used for response/work/retained-state bounds. The wire
  // code alone cannot establish that the player's action rate caused it.
  let reason=if status==2{if sent.as_ref().err().map(String::as_str)==Some("CAPACITY"){"Social selection capacity reached; try again"}else{"Social selection refused"}}else{""}.to_owned();
  record(store.clone(),move|s|{let op=s.social.get_mut(&id).ok_or("Lost A3 original")?;if op.status==0{op.status=status;op.reason=reason;}if op.status!=0&&s.social_pending.get(k)==Some(&id){s.social_pending.remove(k);}Ok(())}).await?;
 }
 let state=stored(store).await?;let mut commands=Vec::new();for k in ["emote","cosmetic"]{if let Some((id,op))=state.social.iter().filter(|(_,op)|kind(&op.publish)==k).max_by_key(|(_,op)|op.publish.sequence){commands.push(Command::A3(g3_a3_frame::Command::Outcome{id:id.clone(),status:op.status,reason:op.reason.clone(),event:op.publish.event_id.clone().unwrap()}));}}Ok(commands)
}
