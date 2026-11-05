//! Authenticated desktop membership contact, independent of game observations
//! and optional projections. This lane never changes world or play authority.
use super::*;

#[derive(Clone)]
struct Binding {
    key:String, revision:u64, generation:u64, fence:Arc<AtomicU64>,
    http:reqwest::Client, origin:String, service:String, epoch:String,
    session:Session, room:String, run:Option<String>, run_epoch:Option<String>, config:u32,
}
impl Binding { fn live(&self)->bool { self.fence.load(Ordering::Acquire)==self.generation } }
#[derive(Default)]
pub(super) struct Feed {
    binding:Option<Binding>, revision:u64, until:Option<Instant>,
    success_at:Option<Instant>, error:Option<String>, renewals:u64,
}
impl Feed {
    fn current(&self,b:&Binding)->bool { b.live()&&self.binding.as_ref().is_some_and(|c|c.live()&&c.key==b.key&&c.revision==b.revision) }
    pub(super) fn view(&self)->Value { let at=Instant::now();json!({"online":self.binding.as_ref().is_some_and(|b|b.live())&&self.until.is_some_and(|until|at<until),"renewals":self.renewals,"last_success_age_ms":self.success_at.map(|t|at.duration_since(t).as_millis().min(3600000)),"remaining_ms":self.until.map(|until|until.saturating_duration_since(at).as_millis().min(30000)),"error":self.error}) }
    fn retire(&mut self) { self.revision=self.revision.saturating_add(1);self.binding=None;self.until=None;self.success_at=None;self.error=None; }
}
pub struct Runner { feed:Arc<Mutex<Feed>>, auth:Arc<Mutex<auth_owner::Feed>>, next:Instant }
impl<S:CredentialStore> Core<S> {
    pub(super) fn publish_presence_owner(&mut self) {
        let mut feed=self.presence_owner.lock().unwrap();
        let Some((session,room,config))=self.desktop.as_ref().zip(self.room.as_ref()).and_then(|(s,r)|Some((s,r,r.config_revision?))) else { if feed.binding.is_some(){feed.retire();}return };
        let Some(room_id)=room.room_id.as_ref() else {if feed.binding.is_some(){feed.retire();}return};
        let generation=self.generation.load(Ordering::Acquire);
        let key=json!([self.base,self.service,self.epoch,self.device,session.account,session.id,session.epoch,room_id,room.run_id,room.run_epoch,config,generation]).to_string();
        let same=feed.binding.as_ref().is_some_and(|b|b.key==key&&b.live());
        if same&&feed.binding.as_ref().is_some_and(|b|b.session.access.as_str()==session.access.as_str()){return}
        if !same{feed.retire();}else{feed.revision=feed.revision.saturating_add(1);}
        let revision=feed.revision;
        feed.binding=Some(Binding{key,revision,generation,fence:self.generation.clone(),http:self.http.clone(),origin:self.base.clone(),service:self.service.clone(),epoch:self.epoch.clone(),session:session.clone(),room:room_id.clone(),run:room.run_id.clone(),run_epoch:room.run_epoch.clone(),config});
    }
    pub(super) fn publish_lobby_display(&self){
        use nt2_desktop_g0::world_lifecycle::Stage;
        let diagnostic=self.compatibility_issue().filter(|i|i.confirmed&&i.origin==self.base&&self.world.coordinator.stage()!=Stage::Ended).map(|i|crate::pipe::LobbyDisplay{code:5,name:i.native_message().into(),until:Instant::now()+Duration::from_secs(2),generation:self.generation.load(Ordering::Acquire),fence:self.generation.clone()});
        let display=diagnostic.or_else(||{
            if !self.lc1||self.desktop.is_none()||matches!(self.world.coordinator.stage(),Stage::Ended|Stage::Offline){return None}
            let room=self.room.as_ref()?;if room.state.as_deref()==Some("ended"){return None}
            let feed=self.presence_owner.lock().ok()?;let binding=feed.binding.as_ref().filter(|b|b.live())?;
            if room.room_id.as_deref()!=Some(binding.room.as_str())||room.run_id!=binding.run||room.run_epoch!=binding.run_epoch||room.config_revision!=Some(binding.config){return None}
            // A remembered room/binding alone has never established membership.
            if matches!(feed.error.as_deref(),Some("FORBIDDEN"|"UNAUTHENTICATED"|"EXPIRED")){return None}
            let success=feed.success_at?;if success.elapsed()>=Duration::from_secs(30){return None}
            let name:String=room.name.as_ref()?.chars().filter(|c|!c.is_control()).take(64).collect();
            if name.is_empty()||name.len()>192{return None}
            let connected=feed.error.is_none()&&feed.until.is_some_and(|t|Instant::now()<t)&&success.elapsed()<Duration::from_secs(6);
            let same_world=self.world.record.as_ref().is_none_or(|r|r.identity.origin==self.base&&r.identity.service==self.service&&r.identity.account==binding.session.account&&r.identity.room==binding.room&&Some(r.identity.run.as_str())==binding.run.as_deref()&&Some(r.identity.run_epoch.as_str())==binding.run_epoch.as_deref()&&r.identity.config==u64::from(binding.config));
            let code=if !connected{3}else if same_world&&self.world.host_started&&room.state.as_deref()==Some("running"){4}else if same_world&&self.world.coordinator.stage()==Stage::Ready&&self.world.waiting_for_start&&!self.world.host_started&&room.state.as_deref()==Some("staging"){2}else{1};
            Some(crate::pipe::LobbyDisplay{code,name,until:Instant::now()+Duration::from_secs(2),generation:binding.generation,fence:binding.fence.clone()})
        });
        self.native_diagnostics.lock().unwrap().lobby_display=display;
    }
    pub fn presence_runner(&mut self)->Runner { self.publish_presence_owner();Runner{feed:self.presence_owner.clone(),auth:self.auth_owner.clone(),next:Instant::now()} }
}
impl Runner {
    pub async fn pump(&mut self)->Result<(),String> {
        if Instant::now()<self.next{return Ok(())}self.next=Instant::now()+Duration::from_secs(2);
        let Some(binding)=self.feed.lock().unwrap().binding.clone().filter(Binding::live) else{return Ok(())};
        let started=Instant::now();let result=Self::send(&binding).await;
        let mut feed=self.feed.lock().unwrap();if !feed.current(&binding){return Ok(())}
        match result {
            Ok(())=>{feed.until=started.checked_add(Duration::from_secs(30));feed.success_at=Some(started);feed.error=None;feed.renewals=feed.renewals.saturating_add(1);Ok(())},
            Err(error)=>{if matches!(error.as_str(),"UNAUTHENTICATED"|"EXPIRED"){self.auth.lock().unwrap().force();}if error=="FORBIDDEN"{feed.until=None;}feed.error=Some(error.clone());Err(error)},
        }
    }
    async fn send(b:&Binding)->Result<(),String> {
        let request=wire::Request{major:Some(1),revision:Some(protocol::REVISION.into()),service_id:Some(b.service.clone()),service_epoch:Some(b.epoch.clone()),request_id:Some(uuid::Uuid::new_v4().to_string()),session_epoch:Some(b.session.epoch.clone()),room_id:Some(b.room.clone()),run_id:b.run.clone(),run_epoch:b.run_epoch.clone(),config_revision:Some(b.config),presence:Some(wire::Presence{present:Some(true)}),..Default::default()};
        let url=format!("{}/v1/rpc",b.origin);let started=Instant::now();
        let response=tokio::time::timeout(Duration::from_secs(3),async {
            let bytes=zeroize::Zeroizing::new(serde_json::to_vec(&request).map_err(|_|"Desktop presence encoding")?);
            let mut reply=b.http.post(&url).bearer_auth(b.session.access.as_str()).header("content-type","application/json").body(bytes.to_vec()).send().await.map_err(|_|"Desktop presence unavailable")?;
            if reply.url().as_str()!=url{return Err("Desktop presence redirect refused".into())}let status=reply.status();let mut bytes=zeroize::Zeroizing::new(Vec::new());while let Some(part)=reply.chunk().await.map_err(|_|"Desktop presence interrupted")?{if bytes.len()+part.len()>65536{return Err("Desktop presence response bound".into())}bytes.extend_from_slice(&part)}
            let response=protocol::response(&bytes).map_err(|_|"Desktop presence schema")?;
            if response.request_id!=request.request_id||response.service_id!=request.service_id||response.service_epoch!=request.service_epoch{return Err("Desktop presence response retired".into())}
            if let Some(error)=response.error{return Err(error.code.unwrap_or("Desktop presence refused".into()))}if !status.is_success(){return Err("Desktop presence refused".into())}Ok(response)
        }).await.map_err(|_|"Desktop presence deadline")??;
        let sample=serde_json::from_value(serde_json::to_value(response.credential_validity.as_ref().ok_or("Desktop presence credential sample absent")?).map_err(|_|"Desktop presence sample encoding")?).map_err(|_|"Desktop presence sample shape")?;
        // Correlate the actual authenticated reply; its membership acknowledgement
        // is UI/contact evidence only and never renews a native/game report.
        admission::credential_sample(&mut Default::default(),&b.origin,&b.service,&b.session,&sample,request.request_id.as_deref().unwrap(),nt2_desktop_g0::connection_time::Stamp{sequence:1,started})?;
        if !b.live(){return Err("Desktop presence owner retired".into())}Ok(())
    }
}

#[cfg(test)]mod tests{
 use super::*;use base64::Engine;
 use super::super::setup::tests::{client,network,Reply};
 #[test]fn lc1_presence_ordinary_scheduler_renews_without_game_or_optional_projection(){tauri::async_runtime::block_on(async{
  let(mut c,room,_)=client();c.enable_lc1();c.game=None;c.room=Some(room);c.world.optional_next=Some(Instant::now()+Duration::from_secs(60));
  let d=c.desktop.as_mut().unwrap();d.access=SecretString::from(format!("fixture.{}.unsigned",base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(json!({"jti":"presence-current"}).to_string())));let desktop=d.clone();
  let(origin,server)=network(vec![Reply::Build(Box::new(move|q|{assert_eq!(q["presence"]["present"],true);assert!(q["gameObservation"].is_null()&&q["reconcile"].is_null()&&q["refresh"].is_null());json!({"credentialValidity":{"principalId":desktop.account,"sessionId":desktop.id,"sessionEpoch":desktop.epoch,"credentialId":"presence-current","originRequestId":q["requestId"],"sampleKey":"a".repeat(64),"accessRemainingMs":900000}})}))]);c.base=origin;
  let desktop=c.desktop.as_ref().unwrap();let sample=serde_json::from_value(json!({"principalId":desktop.account,"sessionId":desktop.id,"sessionEpoch":desktop.epoch,"credentialId":"presence-current","originRequestId":"initial-sample","sampleKey":"b".repeat(64),"accessRemainingMs":900000})).unwrap();admission::credential_sample(&mut c.lifecycle.lock().unwrap().credentials,&c.base,&c.service,desktop,&sample,"initial-sample",nt2_desktop_g0::connection_time::Stamp{sequence:1,started:Instant::now()}).unwrap();
  let pending=c.pending.clone();let before=pending.lock().unwrap().clone();let stop=Arc::new(std::sync::atomic::AtomicBool::new(false));let(handle,actor)=crate::next_app::scheduler::new(c,stop.clone());let actor=tokio::spawn(actor);let until=Instant::now()+Duration::from_secs(5);
  loop{let view=handle.view();if view["connection"]["desktop_presence"]["renewals"]==1{assert_eq!(view["connection"]["app_connected"],true);break}assert!(Instant::now()<until,"Independent desktop presence failed: {}",view["connection"]["desktop_presence"]);tokio::time::sleep(Duration::from_millis(50)).await;}
  tokio::time::timeout(Duration::from_millis(750),handle.command(|c|Box::pin(async move{assert!(c.game.is_none());assert!(c.world.record.is_none());Ok(Value::Null)}))).await.unwrap().unwrap();let after=pending.lock().unwrap().clone();assert_eq!(before.as_ref().map(|p|p.authority_until),after.as_ref().map(|p|p.authority_until));assert_eq!(before.as_ref().and_then(|p|p.observation.as_ref().map(|r|r.observed_at)),after.as_ref().and_then(|p|p.observation.as_ref().map(|r|r.observed_at)));stop.store(true,Ordering::Release);actor.await.unwrap();assert_eq!(server.join().unwrap().len(),1);
 });}
 #[test]fn lc1_presence_late_completion_cannot_touch_replaced_or_rotated_binding(){
  let(mut c,room,_)=client();c.enable_lc1();c.room=Some(room);c.publish_presence_owner();let old=c.presence_owner.lock().unwrap().binding.clone().unwrap();assert!(c.presence_owner.lock().unwrap().current(&old));c.desktop.as_mut().unwrap().access=SecretString::from("new raw token".to_string());c.publish_presence_owner();assert!(!c.presence_owner.lock().unwrap().current(&old));let current=c.presence_owner.lock().unwrap().binding.clone().unwrap();c.generation.fetch_add(1,Ordering::AcqRel);assert!(!c.presence_owner.lock().unwrap().current(&current));assert_eq!(c.presence_owner.lock().unwrap().view()["online"],false);
 }
 #[test]fn protected_lobby_requires_current_membership_and_never_restarts_a_saved_run(){
  let(mut c,mut room,_)=client();c.enable_lc1();room.name=Some("Verified lobby".into());room.state=Some("staging".into());c.room=Some(room);c.publish_presence_owner();
  let code=|c:&Core<_>|{c.publish_lobby_display();c.native_diagnostics.lock().unwrap().lobby_display.as_ref().map(|d|d.code)};
  assert_eq!(code(&c),None,"cached name/binding alone is not connected");
  {let mut f=c.presence_owner.lock().unwrap();f.success_at=Some(Instant::now());f.until=Some(Instant::now()+Duration::from_secs(30));}
  assert_eq!(code(&c),Some(1));
  let t=c.world.coordinator.bind(uuid::Uuid::new_v4(),(nt2_local_ws::owner::Id::OsProcess(1),[1;16]),1);
  c.world.coordinator.complete_until(&t,nt2_desktop_g0::world_lifecycle::Stage::Ready,Instant::now()+Duration::from_secs(6),Instant::now());c.world.waiting_for_start=true;
  assert_eq!(code(&c),Some(2));
  c.room.as_mut().unwrap().state=Some("running".into());assert_eq!(code(&c),Some(1),"saved running world cannot ask for another Start");
  c.world.host_started=true;assert_eq!(code(&c),Some(4));
  c.presence_owner.lock().unwrap().error=Some("Desktop presence unavailable".into());assert_eq!(code(&c),Some(3));
  c.presence_owner.lock().unwrap().error=Some("FORBIDDEN".into());assert_eq!(code(&c),None);
  {let mut f=c.presence_owner.lock().unwrap();f.error=None;f.success_at=Some(Instant::now()-Duration::from_secs(31));}assert_eq!(code(&c),None);
  c.presence_owner.lock().unwrap().success_at=Some(Instant::now());c.generation.fetch_add(1,Ordering::AcqRel);assert_eq!(code(&c),None,"retired generation");
  c.publish_presence_owner();assert_eq!(code(&c),None,"replacement binding is not verified");
  c.room=None;c.publish_presence_owner();assert_eq!(code(&c),None);
 }

}
