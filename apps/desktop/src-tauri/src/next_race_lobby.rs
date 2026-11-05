//! Display-only authenticated room membership; never a game actor or grant.
use super::*;
pub(super) fn scope(room:&wire::Room)->String{json!([room.room_id,room.run_id,room.run_epoch,room.config_revision]).to_string()}
pub(super) fn project(room:&wire::Room,observed:Option<&(String,Instant)>)->Value{
 if room.config.as_ref().and_then(|c|c.mode.as_deref())!=Some("race")||room.state.as_deref()!=Some("staging")||observed.is_none_or(|(key,at)|*key!=scope(room)||at.elapsed()>Duration::from_secs(6)){return Value::Null}
 let Some(members)=room.members.as_ref().filter(|m|m.len()<=32)else{return Value::Null};let mut rows=Vec::new();let mut seen=std::collections::BTreeSet::new();
 for member in members{let(Some(desktop),Some(principal))=(&member.desktop_id,&member.principal)else{return Value::Null};let Some(name)=principal.display_name.as_ref().filter(|n|!n.is_empty()&&n.len()<=396&&!n.chars().any(char::is_control))else{return Value::Null};if uuid::Uuid::parse_str(desktop).is_err()||!seen.insert(desktop){return Value::Null}
  let status=if member.present!=Some(true){"disconnected"}else if member.included==Some(false){"excluded"}else if member.ready==Some(true){"ready"}else{"waiting"};rows.push(json!({"desktop":desktop,"name":name,"status":status}));
 }
 json!({"scope":scope(room),"revision":room.revision.unwrap_or(0).to_string(),"members":rows})
}

#[cfg(test)]mod tests{use super::*;
 #[test]fn race_lobby_members_are_readonly_scoped_and_do_not_require_selected(){
  let mut room:wire::Room=serde_json::from_value(json!({"roomId":"r","runId":"run","runEpoch":"epoch","configRevision":1,"revision":2,"state":"staging","config":{"mode":"race"},"members":[{"accountId":"account","desktopId":"11111111-1111-4111-8111-111111111111","present":true,"included":true,"selected":false,"ready":false,"principal":{"principalId":"account","displayName":"Local"}},{"desktopId":"22222222-2222-4222-8222-222222222222","present":false,"included":true,"principal":{"displayName":"Remote"}}]})).unwrap();
  let at=(scope(&room),Instant::now());let v=project(&room,Some(&at));assert_eq!(v["members"].as_array().unwrap().len(),2);assert_eq!(v["members"][0]["status"],"waiting");assert_eq!(v["members"][1]["status"],"disconnected");for row in v["members"].as_array().unwrap(){assert!(row.get("actor").is_none()&&row.get("hp").is_none()&&row.get("worldToken").is_none());}
  assert!(project(&room,None).is_null());assert!(project(&room,Some(&(scope(&room),Instant::now()-Duration::from_secs(7)))).is_null());room.run_id=Some("other".into());assert!(project(&room,Some(&at)).is_null());room.run_id=Some("run".into());room.members.as_mut().unwrap().remove(1);assert_eq!(project(&room,Some(&at))["members"].as_array().unwrap().len(),1);room.state=Some("running".into());assert!(project(&room,Some(&at)).is_null());room.state=Some("staging".into());room.config.as_mut().unwrap().mode=Some("core".into());assert!(project(&room,Some(&at)).is_null());
 }
}

pub(super) fn delivery<S:CredentialStore>(c:&Core<S>)->Option<(nt2_desktop_g0::g3_local_consumer::Binding,u32,Instant)>{
 if !c.lc1{return None}let room=c.room.as_ref()?;if project(room,c.world.lobby_observed.as_ref()).is_null(){return None}
 let record=c.world.record.as_ref()?;let game=c.game.as_ref()?;let desktop=c.desktop.as_ref()?;let p=c.pending.lock().ok()?.clone()?;let id=&record.identity;
 if id.origin!=c.base||id.service!=c.service||id.device!=c.device||id.account!=desktop.account||game.session.account!=desktop.account||room.room_id.as_ref()!=Some(&id.room)||room.run_id.as_ref()!=Some(&id.run)||room.run_epoch.as_ref()!=Some(&id.run_epoch)||room.config_revision.map(u64::from)!=Some(id.config)||p.run!=id.run||p.epoch!=id.run_epoch||p.config!=id.config||!p.matches_seed(record.seed.as_deref()){return None}
 if !p.native_v2||!p.accepted||!p.loaded||p.owner!=Some(game.owner)||c.world.native!=p.owner||p.last_seen.is_none_or(|at|at.elapsed()>Duration::from_secs(12))||p.generation.as_ref().is_none_or(|(generation,n)|!Arc::ptr_eq(generation,&c.generation)||generation.load(Ordering::Acquire)!=*n)||crate::pipe::owner_instance(game.owner.0).is_none(){return None}
 let expected=c.expected_game.lock().ok()?.clone()?;if !crate::pipe::matches_local_owner(game.owner.0,&expected){return None}
 let epoch=p.native_epoch.filter(|n|*n>0)?;let until=p.collection_until.filter(|t|*t>Instant::now()).or_else(||(p.authority_until>Instant::now()).then_some(p.authority_until))?;
 let binding=nt2_desktop_g0::g3_local_consumer::Binding{context:game.owner.1,service_origin:c.base.clone(),service_id:c.service.clone(),service_epoch:c.epoch.clone(),session_epoch:game.session.epoch.clone(),principal_id:desktop.account.clone(),scope:nt2_desktop_g0::protocol_g3::wire::Scope{room_id:Some(id.room.clone()),run_id:Some(id.run.clone()),run_epoch:Some(id.run_epoch.clone()),config_revision:Some(id.config.try_into().ok()?),attachment_id:Some(game.session.attachment.clone()),world_token:Some(record.world.to_string())}};
 Some((binding,epoch,until))
}
