//! Start refreshes observation versions, while retaining the displayed decision.
use super::*;
use std::collections::HashSet;

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Capture {
    pub server_origin:String, pub service_id:String, pub service_epoch:String,
    pub session_epoch:String, pub desktop_id:String, pub account_id:String,
    pub room:wire::Room, pub inspections:Vec<wire::Inspection>,
}
fn roster(room:&wire::Room)->Vec<(Option<String>,Option<String>)>{
    let mut ids=room.members.as_deref().unwrap_or_default().iter().map(|m|(m.desktop_id.clone(),m.account_id.clone())).collect::<Vec<_>>();ids.sort();ids
}
fn room_matches(old:&wire::Room,new:&wire::Room)->Result<(),String>{
    if old.room_id.is_none()||old.run_id.is_none()||old.run_epoch.is_none()||old.config_revision.is_none()||old.config.is_none()||old.host_account_id.is_none()
        ||old.room_id!=new.room_id||old.run_id!=new.run_id||old.run_epoch!=new.run_epoch||old.config_revision!=new.config_revision
        ||old.host_account_id!=new.host_account_id||json!(old.config)!=json!(new.config)||new.state.as_deref()!=Some("staging"){
        return Err("Lobby host, run or configuration changed; review before Start".into())
    }
    if roster(old)!=roster(new){return Err("Lobby membership changed; review every player before Start".into())}Ok(())
}
// Exclude only observation bookkeeping and service-derived lease/readiness fields.
fn semantics(r:&wire::Inspection)->Value{
    json!([r.desktop_id,r.attachment_id,r.world_token,r.game_instance_id,r.content_digest,r.actual_seed,r.active_mods,r.inventory_complete,r.beta,r.protected,r.loaded,r.collector_failure,r.mod_version,r.bridge_version,r.compatibility_provenance,r.seed_status])
}
fn eligible(r:&wire::Inspection)->bool{
    r.attachment_id.is_some()&&r.world_token.is_some()&&r.game_instance_id.is_some()&&r.content_digest.is_some()&&r.observation_revision.is_some()
        &&r.loaded==Some(true)&&r.protected==Some(true)&&r.inventory_complete==Some(true)&&r.collector_failure.as_deref().is_none_or(str::is_empty)
}
impl<S:CredentialStore+'static> Core<S>{
    pub(crate) fn capture_start(&self)->Result<Capture,String>{
        let d=self.desktop.as_ref().ok_or("Sign in first")?;
        Ok(Capture{server_origin:self.base.clone(),service_id:self.service.clone(),service_epoch:self.epoch.clone(),session_epoch:d.epoch.clone(),desktop_id:d.id.clone(),account_id:d.account.clone(),room:self.room.clone().ok_or("Current lobby unavailable")?,inspections:self.inspections.clone()})
    }
    fn start_context(&self,c:&Capture)->Result<(),String>{
        let d=self.desktop.as_ref().ok_or("Sign in first")?;
        if c.server_origin!=self.base||c.service_id!=self.service||c.service_epoch!=self.epoch||c.session_epoch!=d.epoch||c.desktop_id!=d.id||c.account_id!=d.account{return Err("Start belongs to the previous service or sign-in".into())}
        room_matches(&c.room,self.room.as_ref().ok_or("Current lobby unavailable")?)
    }
    fn start_roster(&self,c:&Capture,participants:&[wire::InspectedParticipant],excluded:&[String],revision:u32)->Result<(),String>{
        self.start_context(c)?;
        if !self.automatic_setup_enabled()||participants.is_empty()||participants.len()>32||excluded.len()>32||c.inspections.len()>32||c.room.config_revision!=Some(revision){return Err("Review the bounded current roster before Start".into())}
        let members=c.room.members.as_deref().ok_or("Current roster unavailable")?;
        let mut ids=HashSet::new();
        for id in participants.iter().filter_map(|p|p.desktop_id.as_ref()).chain(excluded){if !ids.insert(id){return Err("Duplicate roster identity refused".into())}}
        if ids.len()!=participants.len()+excluded.len()||ids.len()!=members.len()||members.iter().any(|m|m.desktop_id.as_ref().is_none_or(|id|!ids.contains(id))){return Err("Lobby membership changed; review every player before Start".into())}
        for p in participants{
            let reports=c.inspections.iter().filter(|r|r.desktop_id==p.desktop_id).collect::<Vec<_>>();
            if reports.len()!=1{return Err("Current inspection unavailable".into())}let r=reports[0];
            if !eligible(r)||r.attachment_id!=p.attachment_id||r.world_token!=p.world_token||r.observation_revision!=p.observation_revision||r.content_digest!=p.content_digest{return Err("Displayed game inspection changed; review before Start".into())}
        }Ok(())
    }
    async fn start_refresh(&mut self,c:&Capture,participants:&[wire::InspectedParticipant])->Result<Vec<wire::InspectedParticipant>,String>{
        self.start_context(c)?;
        let mut q=self.request(false);q.room_get=Some(wire::Empty{});let r=self.rpc(&q,false,false).await?;if r.room.is_none(){return Err("Current lobby read was incomplete".into())}self.absorb(&r);self.start_context(c)?;
        let mut reports=Vec::new();let mut cursor=None;let mut seen=HashSet::new();let mut complete=false;
        for _ in 0..32{
            let mut q=self.request(false);q.inspection_list=Some(wire::InspectionList{limit:Some(1),after_desktop_id:cursor.clone()});
            let response=self.rpc(&q,false,false).await?;
            for r in response.inspections.unwrap_or_default(){let id=r.desktop_id.clone().ok_or("Inspection identity unavailable")?;if !seen.insert(id)||reports.len()>=32{return Err("Inspection page changed; refresh the lobby".into())}reports.push(r);}
            let next=response.inspection_next_desktop_id;
            if next.as_deref().is_none_or(str::is_empty){complete=true;break}
            if next==cursor||!seen.contains(next.as_ref().unwrap()){return Err("Inspection page did not advance".into())}cursor=next;
        }
        if !complete{return Err("Complete bounded game inspections unavailable".into())}
        let mut current=Vec::new();
        for p in participants{
            let old=c.inspections.iter().find(|r|r.desktop_id==p.desktop_id).ok_or("Displayed inspection unavailable")?;
            let new=reports.iter().find(|r|r.desktop_id==p.desktop_id).ok_or("An included player's game inspection is unavailable")?;
            if !eligible(new)||semantics(old)!=semantics(new)||new.observation_revision<old.observation_revision{return Err("An included player's world, game content or connection changed; review their inspection before Start".into())}
            let mut p=p.clone();p.observation_revision=new.observation_revision;current.push(p);
        }
        let mut q=self.request(false);q.room_get=Some(wire::Empty{});let r=self.rpc(&q,false,false).await?;if r.room.is_none(){return Err("Current lobby read was incomplete".into())}self.absorb(&r);self.start_context(c)?;
        let room=self.room.as_ref().unwrap();
        for p in participants{let m=room.members.as_deref().unwrap_or_default().iter().find(|m|m.desktop_id==p.desktop_id).ok_or("Roster changed")?;
            if m.present!=Some(true)||m.ready!=Some(true)||m.readiness.as_deref()!=Some("ready")||m.consented_config_revision!=c.room.config_revision{return Err("An included player is disconnected or not ready; wait or explicitly exclude them".into())}
        }
        self.inspections=reports;Ok(current)
    }
    pub(crate) fn start_captured<'a>(&'a mut self,c:Capture,participants:Vec<wire::InspectedParticipant>,excluded:Vec<String>,revision:u32)->std::pin::Pin<Box<dyn std::future::Future<Output=Result<(),String>>+Send+'a>>{
        Box::pin(async move{
            if self.ux1_view()["pending"]==true||self.ux1_view()["blocked"]==true{return Err("Previous lobby action is unconfirmed. Retry that same action before making another change".into())}
            self.start_roster(&c,&participants,&excluded,revision)?;
            for attempt in 0..2{
                let current=self.start_refresh(&c,&participants).await?;
                let mut q=self.request(false);q.final_start=Some(wire::FinalStart{participants:Some(current),excluded_desktop_ids:Some(excluded.clone())});
                match self.ux1_send(q,true).await{
                    Err(e)if attempt==0&&matches!(e.as_str(),"STALE_REVISION"|"REPORT_CHANGED")=>continue,
                    result=>return result,
                }
            }unreachable!()
        })
    }
}
#[cfg(test)]#[path="next_start_test.rs"]pub(super) mod tests;
