//! Isolated draft-4 product Core. Server owns roles/admission; Rust owns credentials and native context.
use crate::{
    g1_client::{now, timestamp},
    handoff,
    protocol_g2_next::{self as protocol, wire},
    secrets::{CredentialStore, SecretBlob, SecretString, ServiceProfile},
    signin::SignIn,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};
use zeroize::Zeroize;
use crate::native_report;
use nt2_desktop_g0::g3_runtime::{FailureSnapshot,FailureStage};
fn required(value: Option<String>) -> Result<String, String> {
    value
        .filter(|v| !v.is_empty())
        .ok_or("Incomplete candidate response".into())
}
#[derive(Clone)]
struct Session {
    access: SecretString,
    refresh: SecretString,
    id: String,
    epoch: String,
    account: String,
    attachment: String,
    principal: wire::Principal,
    expires: u64,
}
impl Session {
    fn read(credentials: wire::Credentials, device: &str, kind: &str) -> Result<Self, String> {
        let mut credentials = CredentialsGuard(credentials);
        if credentials.kind.as_deref() != Some(kind)
            || credentials.device_id.as_deref() != Some(device)
        {
            return Err("Wrong credential context".into());
        }
        let principal = credentials
            .principal
            .take()
            .ok_or("Missing trusted principal")?;
        let account = required(credentials.account_id.take())?;
        if principal.principal_id.as_deref() != Some(&account) {
            return Err("Conflicting principal identity".into());
        }
        let refresh = credentials.refresh_secret.take().unwrap_or_default();
        if kind != "game" && refresh.len() != 43 {
            return Err("Invalid remembered credential".into());
        }
        Ok(Self {
            access: SecretString::from(required(credentials.access_token.take())?),
            refresh: SecretString::from(refresh),
            id: required(credentials.session_id.take())?,
            epoch: required(credentials.session_epoch.take())?,
            account,
            attachment: credentials.attachment_id.take().unwrap_or_default(),
            principal,
            expires: timestamp(&required(credentials.expires_at.take())?)?,
        })
    }
}
struct CredentialsGuard(wire::Credentials);
impl std::ops::Deref for CredentialsGuard {
    type Target = wire::Credentials;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl std::ops::DerefMut for CredentialsGuard {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
impl Drop for CredentialsGuard {
    fn drop(&mut self) {
        for secret in [&mut self.0.access_token, &mut self.0.refresh_secret] {
            if let Some(secret) = secret {
                secret.zeroize();
            }
        }
    }
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Remembered {
    revision: String,
    device: String,
    refresh: String,
    request_id: String,
    started: u64,
}
impl Drop for Remembered {
    fn drop(&mut self) {
        self.refresh.zeroize();
    }
}
#[derive(Clone)]
struct Retry {
    request: wire::Request,
    until: Instant,
}
impl Drop for Retry {
    fn drop(&mut self) {
        if let Some(create)=&mut self.request.room_create{if let Some(password)=&mut create.password{password.zeroize();}}
        if let Some(policy)=&mut self.request.room_policy{if let Some(password)=&mut policy.password{password.zeroize();}}
        if let Some(join)=&mut self.request.room_join { if let Some(password)=&mut join.password {password.zeroize();} }
        if let Some(r) = &mut self.request.auth_cancel {
            if let Some(secret) = &mut r.initiator_proof {
                secret.zeroize();
            }
        }
        if let Some(r) = &mut self.request.refresh {
            if let Some(secret) = &mut r.secret {
                secret.zeroize();
            }
        }
        if let Some(r) = &mut self.request.auth_exchange {
            if let Some(secret) = &mut r.initiator_proof {
                secret.zeroize();
            }
            if let Some(secret) = &mut r.completion_grant {
                secret.zeroize();
            }
        }
        if let Some(r) = &mut self.request.guest_issue {
            if let Some(secret) = &mut r.issuance_proof {
                secret.zeroize();
            }
        }
    }
}
#[derive(Clone)]
struct Game {
    session: Session,
    instance: String,
    owner: (crate::local_owner::Id, [u8; 16]),
    reported: u64,
    report_retry: Option<Retry>,
    renew_retry: Option<Retry>,
    permit: Option<wire::GameRelease>,
    entered: bool,
    offered: bool,
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IdentityRecovery {
    pub origin: String,
    pub expected: String,
    pub discovered: String,
    pub ticket: String,
}
struct IdentityMismatch {
    recovery: IdentityRecovery,
    generation: u64,
    base: String,
    service: String,
}
pub struct Core<S: CredentialStore> {
    lc1:bool,
    auth_owner:Arc<Mutex<auth_owner::Feed>>,
    presence_owner:Arc<Mutex<presence::Feed>>,
    world:world::Transport,
    world_files:Arc<nt2_desktop_g0::world_store::Files>,
    pub generation: Arc<AtomicU64>,
    pub pending: Arc<Mutex<Option<handoff::Pending>>>,
    pub expected_game: Arc<Mutex<Option<std::path::PathBuf>>>,
    pub native_diagnostics: Arc<Mutex<crate::pipe::NativeDiagnostics>>,
    provisional:Option<Arc<nt2_desktop_g0::provisional_local::Worker>>,
    replacement_lineage:Arc<nt2_desktop_g0::provisional_local::Lineage>,
    entry:entry_transport::Transport,
    store: Arc<S>,
    http: reqwest::Client,
    device: uuid::Uuid,
    profile: Option<ServiceProfile>,
    base: String,
    service: String,
    epoch: String,
    info: Option<wire::Info>,
    desktop: Option<Session>,
    auth: Option<SignIn>,
    auth1: auth1::State,
    ux1: ux1::State,
    auth_begin: Option<Retry>,
    guest_retry: Option<Retry>,
    guest_generation: u64,
    refresh_retry: Option<Retry>,
    pub pending_launch: Option<String>,
    pub open_browser: bool,
    remember: bool,
    room: Option<wire::Room>,
    rooms: Vec<wire::RoomSummary>,
    next_room: Option<String>,
    room_cursor: String,
    preparation: Option<wire::Preparation>,
    game: Option<Game>,
    native_revoked: bool,
    attach_retry: Option<Retry>,
    attach_cycle: Option<(String,Instant)>,
    inspections: Vec<wire::Inspection>,
    inspection_cursor: String,
    chat: Vec<wire::ChatMessage>,
    chat_retry: Option<Retry>,
    chat_feedback:Value,
    status_error: Option<String>,
    login_error: Option<String>,
    identity_mismatch: Option<IdentityMismatch>,
    compatibility_notice: compatibility::Notice,
    optional_failures:std::collections::BTreeMap<&'static str,FailureSnapshot>,
    control_failure: FailureSnapshot,
    control_failure_context: Value,
    control_stage: FailureStage,
    native_failure: Option<String>,
    native_stage: &'static str,
    last_native_observation: Option<u64>,
    last_service_observation: u64,
    auth_state: String,
    next_work: Instant,
    game_path: Option<std::path::PathBuf>,
    installation_state: String,
    installation_diagnostics: String,
    claims_root: std::path::PathBuf,
    preferences: Option<crate::preferences::Preferences>,
    installation_preferences: Option<crate::preferences::InstallationPreferences>,
    preferred_game_path: Option<std::path::PathBuf>,
    setup: setup::Setup,
    resume_intent:nt2_desktop_g0::rejoin_local::ResumeIntent,
    rejoin_transport:rejoin::Transport,
    waiting:waiting::Transport,
    startup_stage:startup_stage::Runner,
    waiting_retirement:waiting::RetirementRunner,
    parked_recovery:Vec<rejoin::Parked>,
    parking_loaded:Option<lifecycle::Issuer>,
    last_original_outcome:Option<Value>,untimed_refresh:Option<String>,
    lifecycle:Arc<Mutex<lifecycle::Coordinator>>,
    recent_servers: Vec<crate::preferences::RecentServer>,
    g3: Option<nt2_desktop_g0::g3_runtime::Runtime>,
    g3_release: Option<wire::GameRelease>,
    #[cfg(test)] social_fixture: bool,
    social_owner: Option<nt2_desktop_g0::g3_social::Owner>,
    social_work:social_worker::Work,
    bank_work:nt2_desktop_g0::g3_bank::Work,
    c1_work:nt2_desktop_g0::c1_work::Work,
    d1_work:nt2_desktop_g0::d1_work::Work,
    race_work:nt2_desktop_g0::race_work::Work,
    race_retained:Option<(nt2_desktop_g0::g3_local_consumer::Binding,u32)>,
    c2_work:nt2_desktop_g0::c2_work::Work,
    transfer_cleanup:nt2_desktop_g0::g3_bank::Cleanup,
    social_stream: Option<Arc<Mutex<nt2_desktop_g0::g3_social_stream::Exchange>>>,
    social_requested: bool,
    social_profile:Option<Value>,
    social_consumer: Option<nt2_desktop_g0::g3_social_projection::Consumer>,
    social_next: Instant,
    social_enabled: bool,
    social_verified: Option<nt2_desktop_g0::g3_social_installation::Verified>,
    social_credentials:nt2_desktop_g0::g3_social::CredentialFeed,
    credential_writer:Arc<Mutex<()>>,
    auth_writer:Arc<Mutex<()>>,
    health_enabled:bool,
    health_ticket:Option<u64>,
    health_nonce:Arc<AtomicU64>,
    health_updates:Arc<Mutex<Option<health::Checkpoint>>>,
    health_seen:u64,
    health_publisher:Option<nt2_desktop_g0::g3_runtime::HealthPublisher>,
    debug:Option<nt2_desktop_g0::debug_diagnostics::Handle>,
}
impl<S: CredentialStore> Core<S> {
    pub fn enable_debug(&mut self,root:std::path::PathBuf,settings:std::path::PathBuf){
        use nt2_desktop_g0::{debug_diagnostics::Handle,g3_social_installation::{DEVELOPMENT_ID,DEVELOPMENT_SHA}};
        self.debug=Handle::start(root,settings,DEVELOPMENT_ID.into(),DEVELOPMENT_SHA.into(),true);
        if let(Some(debug),Some(stream))=(&self.debug,&self.social_stream){stream.lock().unwrap().debug_control(debug.enabled.clone());}
    }
    pub fn set_debug(&self,on:bool)->Result<(),String>{let debug=self.debug.as_ref().ok_or("Diagnostics unavailable")?;debug.setting(on).map_err(str::to_owned)}
    pub fn observe_debug(&mut self,view:&Value){
        use nt2_desktop_g0::debug_diagnostics::{Scope,tag,game_tag};
        let scope=Scope{service_tag:(!self.service.is_empty()&&!self.epoch.is_empty()).then(||tag(&format!("{}\n{}",self.service,self.epoch))),game_tag:self.game_path.as_deref().map(game_tag),owner_tag:self.game.as_ref().map(|g|tag(&g.session.id)),profile_tag:self.desktop.as_ref().map(|d|tag(&format!("{}\n{}",self.base,d.account)))};
        let report=self.social_stream.as_ref().and_then(|s|s.lock().unwrap().take_debug_report());
        if let Some(debug)=&mut self.debug{debug.observe(view,scope,report);}
    }
    pub fn finish_debug(&mut self){if let Some(debug)=&mut self.debug{debug.finish();}}
    pub fn enable_lc1(&mut self){self.lc1=true;self.provisional=None;self.entry=Default::default();self.waiting=Default::default();self.rejoin_transport=Default::default();let bridge=Arc::new(crate::pipe::world::Bridge::new());self.world.bridge=Some(bridge.clone());let mut d=self.native_diagnostics.lock().unwrap();d.provisional=None;d.world=Some(bridge);d.world_fence=Some(self.lc1_preparation_fence());}
    pub fn sync_background(&mut self){if self.lc1{self.sync_auth_owner();self.publish_auth_owner();self.publish_presence_owner();}else{self.sync_health();}self.publish_lobby_display();self.poll_auth1();}
    pub fn lc1_enabled(&self)->bool{self.lc1}
    pub fn legacy_health_runner(&mut self)->Option<health::HealthRunner<S>>{if !self.lc1&&self.g3.is_some(){Some(self.health_runner())}else{None}}
    pub fn enable_live_transport(&mut self)->Result<(),String>{self.g3.as_mut().ok_or("G3 runtime required")?.enable_live();Ok(())}
    /// Only the separate G3 entry enables this owned journal/pipe; accepted G2 remains opt-out.
    pub fn enable_g3(&mut self,path:&std::path::Path,exchange:Arc<Mutex<nt2_desktop_g0::g3_runtime::Exchange>>)->Result<(),String>{
        if self.g3.is_some()||self.pending.lock().unwrap().is_some(){return Err("Enable G3 before native preparation".into());}
        let mut runtime=nt2_desktop_g0::g3_runtime::Runtime::new(nt2_desktop_g0::g3_journal::Journal::open(path)?,exchange);
        runtime.cancellation(self.generation.clone());self.g3=Some(runtime);Ok(())
    }
    fn g3_retire(&mut self){self.social_credentials.retire();if let Some(p)=&self.health_publisher{p.pause();}if let Some(g3)=&mut self.g3{g3.retire();} self.social_owner=None;self.social_work=Default::default();self.social_consumer=None;if let Some(x)=&self.social_stream{x.lock().unwrap().end();}}
    // Terminal control must survive ordinary protection/lease pauses. Keep the
    // existing bounded stream/owner long enough to deliver its control-only End.
    fn social_end(&mut self){self.social_credentials.retire();self.social_owner=None;self.social_work=Default::default();self.social_consumer=None;if let Some(x)=&self.social_stream{x.lock().unwrap().end();}}
    fn social_pause(&mut self){if let Some(owner)=&mut self.social_owner{owner.pause();}if let Some(x)=&self.social_stream{x.lock().unwrap().pause();}}
    fn g3_pause(&mut self){if let Some(p)=&self.health_publisher{p.pause();}if let Some(g3)=&mut self.g3{g3.pause();}self.social_pause();}
    pub fn enable_social(&mut self,root:&std::path::Path,metadata:&Value,stream:Arc<Mutex<nt2_desktop_g0::g3_social_stream::Exchange>>)->Result<(),String>{
        if self.g3.is_none(){return Err("Enable reviewed social runtime before preparation".into());}
        if self.pending.lock().unwrap().is_some()&&(!self.social_enabled
            ||self.social_verified.as_ref().is_none_or(|verified|!verified.matches(root,metadata))
            ||self.social_stream.as_ref().is_none_or(|current|!Arc::ptr_eq(current,&stream))){
            return Err("Enable reviewed social runtime before preparation".into());
        }
        let verified=if self.social_requested{nt2_desktop_g0::g3_social_installation::verify_development(root,metadata)?}else{nt2_desktop_g0::g3_social_installation::verify(root,metadata)?};self.social_verified=Some(verified);self.social_stream=Some(stream);self.social_enabled=true;Ok(())
    }
    /// Separate launcher requests this mode; it cannot approve runtime assets or credentials.
    pub fn request_social(&mut self,stream:Arc<Mutex<nt2_desktop_g0::g3_social_stream::Exchange>>)->Result<(),String>{
        if self.g3.is_none()||self.preparation.is_some()||self.pending.lock().unwrap().is_some(){return Err("Choose social launcher before preparation".into());}
        self.social_requested=true;self.social_stream=Some(stream);self.social_enabled=false;self.social_verified=None;Ok(())
    }
    pub fn social_view(&self)->Value{json!({"requested":self.social_requested,"verified":self.social_verified.is_some(),"enabled":self.social_enabled,"development_only":true,"state":if !self.social_requested{"Social disabled"}else if !self.social_enabled{"HUD/POI test mode; choose matching service and verify the test mod"}else if self.social_owner.is_some(){"HUD/POI connected"}else{"Test mod verified; waiting for inspected Start"}})}
    async fn refresh_social_profile(&mut self)->Result<(),String>{
        let generation=self.generation.load(Ordering::Acquire);let url=format!("{}/v3/social/info",self.base);
        let response=tokio::time::timeout(Duration::from_secs(3),async{
            let mut response=self.http.get(&url).send().await.map_err(|_|"HUD/POI service unavailable")?.error_for_status().map_err(|_|"HUD/POI service refused")?;
            if response.url().as_str()!=url||response.content_length().is_some_and(|n|n>32768){return Err("HUD/POI discovery redirect/capacity".to_string());}
            let mut bytes=Vec::new();while let Some(chunk)=response.chunk().await.map_err(|_|"HUD/POI discovery interrupted")?{if bytes.len()+chunk.len()>32768{return Err("HUD/POI discovery capacity".into());}bytes.extend_from_slice(&chunk);}serde_json::from_slice::<Value>(&bytes).map_err(|_|"HUD/POI discovery invalid".into())
        }).await.map_err(|_|"HUD/POI discovery deadline")??;
        let pinned=nt2_desktop_g0::g3_social_installation::development_profile()?;
        if generation!=self.generation.load(Ordering::Acquire)||response["serviceId"]!=self.service||response["serviceId"]!=pinned["serviceId"]||response["serviceEpoch"]!=self.epoch||response["major"]!=1||response["revision"]!=nt2_desktop_g0::protocol_g3_social::REVISION||response["schemaSha256"]!=nt2_desktop_g0::protocol_g3_social::SCHEMA_SHA||response["replayPolicy"]!=nt2_desktop_g0::protocol_g3_social::POLICY||response["sourceApproved"]!=true||response["developmentOnly"]!=true||response["installApproved"]!=false||response["runtimeApproval"]["sha256"]!=nt2_desktop_g0::g3_social_installation::DEVELOPMENT_SHA||response["runtimeProfile"]!=pinned["profile"]{return Err("Choose the exact reviewed HUD/POI test service; runtime metadata differs".into());}
        self.social_profile=Some(response["runtimeProfile"].clone());self.compatibility_notice.clear("service_profile");Ok(())
    }
    async fn refresh_social_profile_checked(&mut self)->Result<(),String>{
        if let Err(error)=self.refresh_social_profile().await{
            self.social_enabled=false;self.social_verified=None;self.social_profile=None;
            self.revoke_native();
            if error=="Choose the exact reviewed HUD/POI test service; runtime metadata differs"{self.compatibility_profile_failure(&error);}
            self.native_failure=Some("Social service profile discovery failed or changed. Current preparation is revoked; keep the world held and verify the matching service before preparing a fresh world.".into());
            return Err(error);
        }
        Ok(())
    }
    fn g3_binding(&mut self)->Result<Option<nt2_desktop_g0::g3_local_consumer::Binding>,String>{
        self.sync_health();
        if self.g3.is_none()&&self.health_publisher.is_none(){return Ok(None)}
        let local=self.pending.lock().unwrap().clone();
        let Some(local)=local else{self.g3_retire();return Ok(None)};
        let Some(game)=self.game.as_ref() else{self.g3_pause();return Ok(None)};
        let Some(report)=local.observation.as_ref() else{self.g3_pause();return Ok(None)};
        if !self.native_authority_valid(&local)||!local.release||!local.accepted||!local.loaded
            ||local.authority_until<=Instant::now()||local.last_seen.is_none_or(|at|at.elapsed()>Duration::from_secs(12))
            ||!game.entered||local.owner!=Some(game.owner)
            ||report.observation.protected||!report.observation.complete||!report.observation.loaded {
            self.g3_pause();return Ok(None)
        }
        // The original service-acknowledged release must still match the exact
        // current content and phase. A newer unchanged local report awaiting the
        // next control poll is not a new grant and does not renew its deadline.
        #[cfg(feature="g3-rejoin")]
        let validated=self.validate_current_admission(&local);
        #[cfg(not(feature="g3-rejoin"))]
        let validated=match self.g3_release.as_ref(){Some(release)=>self.validate_release(release,&local),None=>Err("No current validated G3 release".into())};
        if let Err(error)=validated{self.g3_pause();return Err(error);}
        let expected=self.expected_game.lock().unwrap().clone().ok_or("No verified G3 process")?;
        if !crate::pipe::matches_local_owner(game.owner.0,&expected){self.g3_retire();return Err("G3 process identity changed".into());}
        let room=self.room.as_ref().ok_or("No G3 room")?;
        let binding=nt2_desktop_g0::g3_local_consumer::Binding{context:game.owner.1,service_origin:self.base.clone(),service_id:self.service.clone(),
            service_epoch:self.epoch.clone(),session_epoch:game.session.epoch.clone(),principal_id:game.session.account.clone(),
            scope:nt2_desktop_g0::protocol_g3::wire::Scope{room_id:room.room_id.clone(),run_id:Some(local.run),run_epoch:Some(local.epoch),
                config_revision:Some(u32::try_from(local.config).map_err(|_|"Invalid G3 config")?),attachment_id:Some(game.session.attachment.clone()),world_token:Some(self.waiting_world().map(String::from).or_else(||self.rejoin_intended_world()).unwrap_or_else(||hex(&game.owner.1)))}};
        Ok(Some(binding))
    }
    fn g3_sync(&mut self)->Result<(),String>{
        #[cfg(feature="g3-rejoin")]
        if self.lc1{
            self.sync_health();
            let local=self.pending.lock().unwrap().clone();
            let ready=local.as_ref().is_some_and(|p|self.lc1_live_movement_ready(p));
            if !ready{
                // The protected ACK can release Controls before the Server
                // acknowledges entry. Only movement waits here; LC1 HTTP and
                // presence reconciliation, and social work, remain independent.
                if let Some(runtime)=&mut self.g3{runtime.pause();}
                else if let Some(publisher)=&self.health_publisher{publisher.pause();}
                return Ok(())
            }
        }
        let Some(binding)=self.g3_binding()? else{return Ok(())};
        let game=self.game.as_ref().unwrap();let local=self.pending.lock().unwrap().clone().ok_or("No current native authority")?;
        let until=local.authority_until;let generation=local.generation.as_ref().ok_or("No authority generation")?.1;
        let token=zeroize::Zeroizing::new(game.session.access.as_str().to_string());
        let scope=nt2_desktop_g0::protocol_g3_live::wire::Scope{principal_id:Some(game.session.account.clone()),session_id:Some(game.session.id.clone()),session_epoch:Some(game.session.epoch.clone()),room_id:binding.scope.room_id.clone(),run_id:binding.scope.run_id.clone(),run_epoch:binding.scope.run_epoch.clone(),config_revision:binding.scope.config_revision,attachment_id:binding.scope.attachment_id.clone(),world_token:binding.scope.world_token.clone()};
        if let Some(publisher)=&self.health_publisher{publisher.bind_live_scope(scope);}else{self.g3.as_ref().unwrap().bind_live_scope(scope);}
        if self.lc1{
            let runtime=self.g3.as_mut().ok_or("No LC1 gameplay runtime")?;
            runtime.authorize_lifecycle(binding.clone(),token,game.owner.0,until,self.http.clone(),generation,self.world.record.as_ref().ok_or("No LC1 world")?.canonical_generation.try_into().map_err(|_|"Canonical generation bound")?)?;
            if runtime.take_live_renewal(&binding,game.session.access.as_str(),game.owner.0,generation){self.lc1_request_live_renewal();}Ok(())
        }else if self.rejoin_transport.contact.is_some()&&self.health_ticket.is_none(){
            self.g3.as_mut().ok_or("No recovered gameplay runtime")?.authorize_recovered(binding,token,game.owner.0,until,self.http.clone(),generation,self.rejoin_transport.generation)
        }else if let Some(publisher)=&self.health_publisher{publisher.authorize(binding,token,game.owner.0,until,self.http.clone(),generation)}
        else{self.g3.as_mut().unwrap().authorize_at_generation(binding,token,game.owner.0,until,self.http.clone(),generation)}
    }
    #[cfg(test)]
    async fn social_fixture_sync(&mut self)->Result<(),String>{
        if !self.social_fixture{return Err("Social simulated fixture is not enabled".into());}
        self.social_enabled=true;self.social_sync().await
    }
    async fn social_sync(&mut self)->Result<(),String>{
        if !self.social_enabled{return Err("Social runtime opt-in disabled".into());}
        let simulated={#[cfg(test)] {self.social_fixture} #[cfg(not(test))] {false}};
        if !simulated{let root=self.game_path.as_ref().ok_or("No verified social installation")?;let profile=self.social_profile.as_ref().ok_or("Service social profile missing")?;
            if self.social_verified.as_ref().is_none_or(|v|!v.matches(root,profile)){return Err("Social profile/installation changed".into());}}
        let binding=self.g3_binding()?.ok_or("No current Core-owned playing authority")?;
        if self.social_owner.as_ref().is_some_and(|owner|!owner.authority_matches(&binding,&self.game.as_ref().unwrap().session.id)){
            self.social_owner.as_mut().unwrap().pause();return Err("Social Core authority replaced; retire original owner".into());
        }
        if self.social_owner.is_none(){let game=self.game.as_ref().unwrap();
            self.social_credentials.publish(binding.clone(),game.session.id.clone(),zeroize::Zeroizing::new(game.session.access.as_str().to_string()))?;
            let mut owner=nt2_desktop_g0::g3_social::Owner::new(binding,game.session.id.clone(),zeroize::Zeroizing::new(game.session.access.as_str().to_string()),self.http.clone(),self.generation.clone())?;
            owner.credential_feed(self.social_credentials.clone())?;owner.discover().await?;self.social_owner=Some(owner);
        }
        self.social_owner.as_mut().unwrap().resume()
    }
    /// Compiled opt-in source pump; published runtime/installation approval is required.
    async fn social_native_tick(&mut self)->Result<(),String>{
        let result=tokio::time::timeout(Duration::from_secs(3),self.social_native_pump()).await.unwrap_or_else(|_|Err("Social whole capture deadline".into()));
        if result.is_err(){
            self.social_pause();
            // A failed social capture is not movement revocation. Genuine
            // shared authority loss still pauses both lanes through binding.
            if self.g3_binding().is_err(){self.g3_pause();}
        }
        result
    }
    async fn social_native_pump(&mut self)->Result<(),String>{
        use nt2_desktop_g0::{g3_social_frame::Command,g3_social_projection::Consumer};
        self.social_sync().await?;
        let binding=self.g3_binding()?.ok_or("No social playing owner")?;
        let generation=self.generation.load(Ordering::Acquire);let native_generation=u32::try_from(generation).map_err(|_|"Social generation exhausted")?;
        let stream=self.social_stream.clone().ok_or("Unapproved stream disabled")?;
        {let p=self.pending.lock().unwrap();let p=p.as_ref().ok_or("No native owner")?;stream.lock().unwrap().arm(self.game.as_ref().unwrap().owner.0,binding.context,native_generation,p.authority_until)?;}
        if Instant::now()<self.social_next||!stream.lock().unwrap().ready(){return Ok(())}
        self.social_next=Instant::now()+Duration::from_millis(500);
        let started=Instant::now();tokio::time::sleep(Duration::from_millis(110)).await;let snapshot=self.social_owner.as_mut().unwrap().latest().await?;
        let scope=snapshot.scope.as_ref().ok_or("No latest scope")?;
        let players=self.social_owner.as_mut().unwrap().roster().await?.players.unwrap_or_default();let mut presentation=nt2_desktop_g0::g3_a3_frame::Replay::default();
        let selected=self.room.as_ref().and_then(|r|r.members.as_ref()).ok_or("No selected roster")?.iter().filter(|m|m.selected==Some(true)).filter_map(|m|m.account_id.clone()).collect::<Vec<_>>();
        if self.social_consumer.is_none(){self.social_consumer=Some(Consumer::from_authenticated_projections(scope.clone(),generation,&selected,&snapshot)?);}
        let consumer=self.social_consumer.as_mut().unwrap();let removed=consumer.reconcile_latest(generation,&selected,&snapshot)?;
        let mut pager=consumer.pager();let mut pages=vec![];let mut after="0".to_string();
        loop{if started.elapsed()>Duration::from_secs(3)||pages.len()>=8{return Err("Social whole-scan budget".into());}
            tokio::time::sleep(Duration::from_millis(100)).await;let response=self.social_owner.as_mut().unwrap().poi_snapshot(&after).await?;
            let page=consumer.page(generation,&response,&mut pager)?;let done=page.complete;after=page.next.to_string();pages.push(page);if done{break}
        }
        let local_now=started.elapsed().as_secs_f64();let mut commands=vec![Command::Roster(consumer.actors())];commands.extend(removed.into_iter().map(Command::Remove));
        for p in snapshot.projections.as_ref().ok_or("No latest rows")?{
            if p.hud.is_some(){if let Some(hud)=consumer.hud(generation,scope,p)?{commands.push(Command::Hud(hud));}}
            else if let Some(value)=presentation.accept(scope,&players,p,started.elapsed(),local_now)?{commands.push(Command::A3(value));}
        }
        commands.push(Command::Begin(pages.first().unwrap().high));let mut batches=vec![];let mut batch=vec![];
        for command in commands{batch.push(command);if batch.len()==32{batches.push(std::mem::take(&mut batch));}}if !batch.is_empty(){batches.push(batch);}
        batches.extend(pages.into_iter().map(|page|vec![Command::Page(page)]));
        // Native playing and Core selection are rechecked after all awaits, before publication.
        self.g3_binding()?.ok_or("Social owner retired during scan")?;
        if self.generation.load(Ordering::Acquire)!=generation{return Err("Late social scan generation".into())}
        let result=stream.lock().unwrap().publish(local_now,batches);result
    }
    pub async fn g3_tick(&mut self)->Result<(),String>{
        let result=match self.g3_sync(){Ok(())=>{if let Some(g3)=&mut self.g3{g3.pump().await}else{Ok(())}},Err(error)=>Err(error)};
        if result.is_err(){self.g3_pause();}result
    }
    pub async fn social_scheduler_tick(&mut self)->Result<(),String>{
        if !self.social_enabled||self.social_stream.is_none(){return Ok(())}
        if self.g3_binding()?.is_none(){return Ok(())}
        let result=self.social_native_tick().await;
        if let Err(error)=&result{self.record_optional_failure("social",FailureStage::Chat,error);}
        result
    }
    fn original_status_desktop(&mut self)->Result<Session,String>{
        let desktop=self.desktop.as_ref().ok_or("Sign in to the owning service before checking an original")?.clone();
        // LC1's independent auth owner validates and renews desktop credentials
        // even without a native world. Its sample is not a game admission lease
        // in lifecycle.credentials; requiring that cache blocks cold cleanup.
        if self.lc1{Ok(self.lifecycle_request_owner()?.read(&self.base,&self.service,&self.epoch,&desktop.epoch)?.0)}
            else{if self.session_expiring(&desktop,0){return Err("Desktop session expired".into())}Ok(desktop)}
    }
    fn original_recovery_owner(&mut self)->Result<nt2_desktop_g0::g3_http::RecoveryOwner,String>{
        let desktop=self.original_status_desktop()?;
        Ok(nt2_desktop_g0::g3_http::RecoveryOwner::new(self.base.clone(),self.service.clone(),self.epoch.clone(),desktop.epoch.clone(),desktop.account.clone(),zeroize::Zeroizing::new(desktop.access.as_str().to_owned()),self.generation.clone()).with_verified_client(self.http.clone()))
    }
    fn race_context(&mut self)->Option<nt2_desktop_g0::race_work::Context>{
        let race=self.room.as_ref().and_then(|r|r.config.as_ref()).and_then(|c|c.mode.as_deref())==Some("race");
        if !race{self.race_retained=None;if let Some(s)=&self.social_stream{s.lock().ok()?.race_disarm();}}
        let room=self.room.as_ref()?;
        let read_results=race&&matches!(room.state.as_deref(),Some("running"|"ended"));let locator=room.stats_locator.clone().unwrap_or_default();let room_id=room.room_id.clone()?;
        let live=if race{self.g3.as_ref().and_then(|g|g.bank_context()).map(|c|c.binding).or_else(||self.c1_bootstrap_context().map(|c|c.transport.binding))}else{None};
        let generation=self.lifecycle.lock().ok()?.authority.admission.as_ref().and_then(|a|a.generation).filter(|n|*n>0);
        if let Some(binding)=live{if let Some(generation)=generation{self.race_retained=Some((binding,generation));}}
        let desktop=self.desktop.as_ref()?;
        let display=if race{race_lobby::delivery(self)}else{None};let display_only=display.is_some();
        let(binding,generation,recover_only)=if let Some((binding,epoch,until))=display{if let Some(local)=self.pending.lock().ok()?.as_ref(){if let Some((pid,context))=local.owner{self.social_stream.as_ref()?.lock().ok()?.race_arm(pid,context,epoch,until).ok()?;}}(binding,0,false)}else if let Some((binding,generation))=&self.race_retained{(binding.clone(),*generation,false)}else{
            (nt2_desktop_g0::g3_local_consumer::Binding{context:[0;16],service_origin:self.base.clone(),service_id:self.service.clone(),service_epoch:self.epoch.clone(),session_epoch:desktop.epoch.clone(),principal_id:desktop.account.clone(),scope:nt2_desktop_g0::protocol_g3::wire::Scope{room_id:Some(room_id.clone()),..Default::default()}},0,true)
        };
        if binding.service_origin!=self.base||binding.service_id!=self.service||binding.principal_id!=desktop.account||binding.scope.room_id.as_deref()!=Some(&room_id){return None}
        if !recover_only{if let Some(local)=self.pending.lock().ok()?.as_ref(){
            if let Some((pid,context))=local.owner.filter(|(_,c)|*c==binding.context){
                let readonly=self.race_readonly_metadata(local);
                if local.last_seen.is_some_and(|t|t.elapsed()<Duration::from_secs(12))&&(self.native_authority_valid(local)||readonly){
                    let until=if readonly{local.collection_until?}else{local.authority_until};
                    if let Some(epoch)=local.native_epoch{let _=self.social_stream.as_ref()?.lock().ok()?.race_arm(pid,context,epoch,until);}
                }
            }
        }}
        let local_world=self.world.record.as_ref().and_then(|r|nt2_desktop_g0::race_work::local_world_for(&r.identity,r.world,&binding));
        Some(nt2_desktop_g0::race_work::Context{local_world,owner:self.original_recovery_owner().ok()?,journal:self.g3.as_ref()?.consumer.journal.clone(),binding,generation,locator,lobby:self.room.as_ref().map(|room|race_lobby::project(room,self.world.lobby_observed.as_ref())).unwrap_or(Value::Null),players:self.social_work.item_players(),display_only,recover_only,read_results})
    }
    fn c1_bootstrap_context(&mut self)->Option<nt2_desktop_g0::c1_work::Context>{
        #[cfg(not(feature="g3-rejoin"))]{return None}
        #[cfg(feature="g3-rejoin")]{
        if !self.lc1{return None}let local=self.pending.lock().ok()?.clone()?;
        if !self.lc1_c1_metadata_ready(&local)||!self.native_authority_valid(&local)||!local.accepted||!local.loaded||local.last_seen.is_none_or(|at|at.elapsed()>Duration::from_secs(12)){return None}
        let game=self.game.as_ref()?.clone();if self.session_expiring(&game.session,0){return None}
        // Desktop validity belongs to the independent LC1 auth owner, not the
        // game-admission credential cache. Reuse its current validated lease.
        let desktop=self.desktop.as_ref()?.clone();let (current,_)=self.lifecycle_request_owner().ok()?.read(&self.base,&self.service,&self.epoch,&desktop.epoch).ok()?;
        if current.id!=desktop.id||current.account!=desktop.account||current.epoch!=desktop.epoch{return None}
        let record=self.world.record.as_ref()?;let runtime=self.g3.as_ref()?;
        let expected=self.expected_game.lock().ok()?.clone()?;if !crate::pipe::matches_local_owner(game.owner.0,&expected){return None}
        let binding=nt2_desktop_g0::g3_local_consumer::Binding{context:game.owner.1,service_origin:self.base.clone(),service_id:self.service.clone(),service_epoch:self.epoch.clone(),session_epoch:game.session.epoch.clone(),principal_id:game.session.account.clone(),scope:nt2_desktop_g0::protocol_g3::wire::Scope{room_id:Some(record.identity.room.clone()),run_id:Some(local.run.clone()),run_epoch:Some(local.epoch.clone()),config_revision:Some(u32::try_from(local.config).ok()?),attachment_id:Some(game.session.attachment.clone()),world_token:Some(record.world.to_string())}};
        let generation=local.generation.as_ref()?.1;let ownership=self.lifecycle.lock().ok()?.authority.admission.as_ref()?.generation.filter(|g|*g>0)?;let native_generation=local.native_epoch.filter(|epoch|*epoch>0)?;
        let mut owner=nt2_desktop_g0::g3_http::HttpOwner::from_verified_client(binding.clone(),zeroize::Zeroizing::new(game.session.access.as_str().to_owned()),self.generation.clone(),self.http.clone()).ok()?;
        owner.c1_guard(self.generation.clone(),generation,local.authority_until);
        let completion=self.lc1_c1_completion_ready(&local);
        {let stream=self.social_stream.as_ref()?;let mut stream=stream.lock().ok()?;if completion{stream.c1_completion_arm(game.owner.0,binding.context,native_generation,ownership,local.authority_until).ok()?;}else{stream.c1_bootstrap_arm(game.owner.0,binding.context,native_generation,ownership,local.authority_until).ok()?;}}
        Some(nt2_desktop_g0::c1_work::Context{transport:nt2_desktop_g0::g3_bank::Context{binding,consumer:runtime.consumer.clone(),owner,events:None,items:false,item_players:json!([])},ownership_generation:ownership,bootstrap:true,completion})
        }
    }
    pub async fn bank_scheduler_tick(&mut self){
        let cleanup=self.g3_binding().ok().flatten().and_then(|b|self.original_recovery_owner().ok().zip(self.g3.as_ref().map(|g|g.consumer.journal.clone())).map(|(o,j)|(o,j,b)));
        self.transfer_cleanup.tick(cleanup).await;
        if let Some(g3)=&mut self.g3{g3.prune_transfer_statuses();}
        if let Some(stream)=self.social_stream.clone(){
        let mut context=if self.g3_binding().ok().flatten().is_some(){self.g3.as_ref().and_then(|g|g.bank_context())}else{None};
        if let Some(context)=&mut context{context.items=self.local_game_capabilities().iter().any(|c|c=="core.items");context.item_players=self.social_work.item_players();}
        let c1_context=if self.room.as_ref().and_then(|r|r.config.as_ref()).and_then(|c|c.mode.as_deref())==Some("core"){
         self.g3.as_ref().and_then(|g|g.bank_context()).map(|transport|{let ownership_generation=self.lifecycle.lock().unwrap().authority.admission.as_ref().and_then(|a|a.generation).unwrap_or(0);nt2_desktop_g0::c1_work::Context{transport,ownership_generation,bootstrap:false,completion:false}}).or_else(||self.c1_bootstrap_context())
        }else{None};
        if c1_context.is_none()&&self.room.as_ref().and_then(|r|r.config.as_ref()).and_then(|c|c.mode.as_deref())==Some("core"){stream.lock().unwrap().clear_c1_bootstrap();}
        if c1_context.is_none()&&self.room.as_ref().and_then(|r|r.config.as_ref()).and_then(|c|c.mode.as_deref()).is_some_and(|m|m!="core"){
         if let Some(context)=&context{stream.lock().unwrap().c1_view_for(context.binding.context,json!({"enabled":false,"initialized":true}));}
         else if let Some(bootstrap)=self.c1_bootstrap_context(){stream.lock().unwrap().c1_bootstrap_view(bootstrap.binding.context,bootstrap.ownership_generation,json!({"enabled":false,"initialized":true}));}
        }
        let c2_caps=self.local_game_capabilities();
        let c2_context=if self.room.as_ref().and_then(|r|r.config.as_ref()).and_then(|c|c.mode.as_deref())==Some("core")&&["core.world-rules","core.hourglass"].iter().all(|cap|c2_caps.iter().any(|c|c==cap)){
         self.g3.as_ref().and_then(|g|g.bank_context()).map(|transport|{let ownership_generation=self.lifecycle.lock().unwrap().authority.admission.as_ref().and_then(|a|a.generation).unwrap_or(0);nt2_desktop_g0::c1_work::Context{transport,ownership_generation,bootstrap:false,completion:false}}).or_else(||self.c1_bootstrap_context())
        }else{None};
        self.c2_work.collect().await;let c2_recovery=if self.c2_work.source_recovery_needed(){self.original_recovery_owner().ok()}else{None};self.c2_work.tick(c2_context,stream.clone(),c2_recovery).await;
        let retired=c1_context.is_none();self.c1_work.tick(c1_context,stream.clone()).await;
        if retired{let recovery=self.original_recovery_owner().ok().zip(self.g3.as_ref().map(|g|g.consumer.journal.clone()));self.c1_work.retired_tick(recovery).await;if ["core.world-rules","core.hourglass"].iter().all(|cap|c2_caps.iter().any(|c|c==cap)){let journal=self.g3.as_ref().map(|g|g.consumer.journal.clone());if journal.as_ref().is_some_and(|j|self.c2_work.retired_due(j)){let recovery=self.original_recovery_owner().ok().zip(journal);self.c2_work.retired_tick(recovery).await;}}}
        let stats_context=self.g3.as_ref().and_then(|g|g.bank_context()).map(|transport|{let ownership_generation=self.lifecycle.lock().unwrap().authority.admission.as_ref().and_then(|a|a.generation).unwrap_or(0);nt2_desktop_g0::c1_work::Context{transport,ownership_generation,bootstrap:false,completion:false}});
        self.d1_work.tick(stats_context,stream.clone()).await;
        let race_context=self.race_context();self.race_work.tick(race_context,stream.clone()).await;
        // Race keeps vanilla local pickups. Do not poll the cooperative bank
        // and item service for a mode that cannot consume those results.
        if self.room.as_ref().and_then(|r|r.config.as_ref()).and_then(|c|c.mode.as_deref())==Some("race"){context=None;}
        self.bank_work.tick(context,stream).await;
    }}
    pub async fn bank_shutdown(&mut self){self.transfer_cleanup.shutdown().await;self.bank_work.shutdown().await;self.c1_work.shutdown().await;self.c2_work.shutdown().await;self.d1_work.shutdown().await;self.race_work.shutdown().await;}
    pub fn g3_view(&self)->Value{
        let Some(g3)=self.g3.as_ref() else{return Value::Null};let mut view=g3.view();
        #[cfg(all(test,feature="g3-rejoin"))]{let p=self.pending.lock().unwrap().clone();view["fixture_c1_bootstrap"]=p.as_ref().map(|p|json!({"metadataReady":self.lc1_c1_metadata_ready(p),"nativeValid":self.native_authority_valid(p),"accepted":p.accepted,"loaded":p.loaded,"seen":p.last_seen.is_some_and(|at|at.elapsed()<Duration::from_secs(12)),"hasGame":self.game.is_some(),"gameExpires":self.game.as_ref().is_none_or(|g|self.session_expiring(&g.session,0)),"desktopExpires":self.desktop.as_ref().is_none_or(|d|self.session_expiring(d,0)),"hasRuntime":self.g3.is_some(),"mode":self.room.as_ref().and_then(|r|r.config.as_ref()).and_then(|c|c.mode.as_deref())})).unwrap_or(Value::Null);}

        let current=self.control_context();view["control"]=json!({"last_validated_phase":current["phase"],"entered_locally":self.game.as_ref().is_some_and(|g|g.entered),"current":current,"failure":self.control_failure,"failure_context":self.control_failure_context});
        #[cfg(test)]{view["fixture_peers"]=g3.fixture_peers();}
        #[cfg(test)]{view["fixture_social"]=self.social_work.fixture_view();view["fixture_social_stream"]=self.social_stream.as_ref().map(|s|s.lock().unwrap().progress()).unwrap_or(Value::Null);}
        view["optional_failures"]=json!(self.optional_failures);view["race"]=self.race_work.view();if view["race"].is_null(){view["race"]=json!({});}view["race"]["detectorFailure"]=json!(self.world.record.as_ref().and_then(|r|self.pending.lock().ok().and_then(|p|p.as_ref().and_then(|p|p.observation.as_ref()).and_then(|o|o.observation.race_failure(&r.config)))));view["c1"]=self.c1_work.view();view["c2"]=self.c2_work.view();
        view["player_information"]=self.social_work.diagnostics();
        let selected=g3.bank_context().map(|c|c.binding).filter(|b|{
            let local=self.pending.lock().unwrap();let game=self.game.as_ref();
            b.service_origin==self.base&&b.service_id==self.service&&self.room.as_ref().is_some_and(|r|r.room_id==b.scope.room_id)
            &&game.is_some_and(|g|g.owner.1==b.context&&g.session.account==b.principal_id)
            &&local.as_ref().is_some_and(|l|b.scope.run_id.as_ref()==Some(&l.run)&&b.scope.run_epoch.as_ref()==Some(&l.epoch)&&b.scope.config_revision==u32::try_from(l.config).ok())
        });view["bank"]=self.bank_work.view_for(selected.as_ref());
        let statuses:std::collections::BTreeMap<_,_>=view["bank"]["operation_statuses"].as_array().into_iter().flatten().chain(view["bank"]["items"]["operation_statuses"].as_array().into_iter().flatten()).filter_map(|v|Some((v["operation_id"].as_str()?.to_owned(),v["server_status"].clone()))).collect();
        if let Some(operations)=view["operations"].as_array_mut(){for op in operations{if selected.is_none(){op["current_scope"]=json!(false);}if op["current_scope"]==true{if let Some(status)=op["operation_id"].as_str().and_then(|id|statuses.get(id)){op["server_status"]=status.clone();}}}}

        view["transfer_cleanup"]=self.transfer_cleanup.view();
        view["debug_canonical_peers"]=self.social_consumer.as_ref().map(|c|c.actors().iter().filter(|id|self.game.as_ref().is_none_or(|g|**id!=g.session.id)).count()).map_or(Value::Null,|n|json!(n));
        view["optional_stream"]=self.social_stream.as_ref().map(|s|s.lock().unwrap().progress()).unwrap_or(Value::Null);
        view
    }
    pub fn movement_runner(&self)->Option<nt2_desktop_g0::g3_runtime::MovementRunner>{self.g3.as_ref().map(|runtime|runtime.runner())}
    pub async fn g3_original_status(&mut self,id:&str)->Result<Value,String>{
        let mut owner=self.original_recovery_owner()?;
        let g3=self.g3.as_mut().ok_or("G3 candidate not enabled")?;
        if !g3.consumer.journal.pending().any(|p|p.record.original.operation_id==id){return Err("Unknown retained original".into());}
        if Instant::now()<g3.recovery_after{return Err("Original status polling capacity".into());}
        g3.recovery_after=Instant::now()+Duration::from_millis(100);
        match owner.status_or_deleted(&g3.consumer.journal,id).await?{
            nt2_desktop_g0::g3_http::OriginalStatus::Operation(operation)=>g3.record_status(&operation)?,
            nt2_desktop_g0::g3_http::OriginalStatus::ItemOperation(operation)=>g3.record_item_status(&operation)?,
            nt2_desktop_g0::g3_http::OriginalStatus::LobbyDeleted(proof)=>{let journal=g3.consumer.journal.clone();tokio::task::spawn_blocking(move||journal.retire_lobby(proof)).await.map_err(|_|"Lobby cleanup worker failed")??;g3.prune_transfer_statuses();},
        }Ok(g3.view())
    }
    pub fn public_preferences(&mut self, path: std::path::PathBuf) -> Result<(), String> {
        let installation=crate::preferences::InstallationPreferences::at(path.with_file_name("installation.json"));
        match installation.load(){
            Ok(preferred)=>self.preferred_game_path=preferred,
            Err(error)=>self.installation_state=format!("{error}; choose your Noita folder"),
        }
        self.installation_preferences=Some(installation);
        let preferences = crate::preferences::Preferences::at(path);
        self.recent_servers = preferences.load(self.device)?;
        self.preferences = Some(preferences);
        Ok(())
    }
    fn remember_server(&mut self, profile: &ServiceProfile, service: &str) -> Result<(), String> {
        let mut servers = self.recent_servers.clone();
        let remembered_name=servers.iter().find(|s|s.origin==profile.origin()&&s.service_id==service).and_then(|s|s.remembered_name.clone());
        let has_saved_account=servers.iter().find(|s|s.origin==profile.origin()&&s.service_id==service).and_then(|s|s.has_saved_account);
        servers.retain(|s| s.origin != profile.origin());
        servers.insert(
            0,
            crate::preferences::RecentServer {
                origin: profile.origin().into(),
                service_id: service.into(),
                remembered_name,
                has_saved_account,
            },
        );
        servers.truncate(16);
        if let Some(preferences) = &self.preferences {
            preferences.save(&servers, self.device)?;
        }
        self.recent_servers = servers;
        Ok(())
    }
    fn reject_current_identity(&mut self, origin: &str) {
        // Preserve another server on a failed switch. At the active origin,
        // prevent subsequent ticks from using credentials after an Info mismatch.
        if self.base != origin {
            return;
        }
        self.stop_setup("Service identity changed; fresh sign-in and explicit lobby selection required");
        if self.lc1{self.auth_owner.lock().unwrap().retire();}
        self.revoke_native();
        self.desktop = None;
        self.profile = None;
        self.info = None;
        self.room = None;
        self.rooms.clear();
        self.auth = None;
        self.auth_begin = None;
        self.guest_retry = None;
        self.refresh_retry = None;
        self.chat_retry = None;
        self.pending_launch = None;
        self.inspections.clear();
        self.chat.clear();
        self.generation.fetch_add(1, Ordering::AcqRel);
        self.auth_state = "Server identity changed; credentials held locally".into();
    }
    pub fn configure_game(&mut self, path: &str) -> Result<std::path::PathBuf, String> {
        let root = crate::installation::root(path)?;
        let executable = crate::pipe::verified_process_path(&root.join("noita.exe"))?;
        if self.pending.lock().unwrap().is_some()
            && self.expected_game.lock().unwrap().as_ref() != Some(&executable)
        {
            return Err("Cancel preparation before changing installation".into());
        }
        self.installation_diagnostics = crate::installation::candidate_diagnostics(&root);
        if cfg!(feature="beta-portable")&&self.info.is_none(){crate::installation::beta::verify_game(&root)?;}else{self.verify_game(&root)?;}
        if self.social_requested&&self.info.is_some(){let metadata=self.social_profile.clone().ok_or("No social profile")?;
            self.enable_social(&root,&metadata,self.social_stream.clone().ok_or("No social listener")?)?;}
        self.game_path = Some(root.clone());
        *self.expected_game.lock().unwrap() = Some(executable.clone());
        if let Some(bridge)=self.native_diagnostics.lock().unwrap().local_ws.clone(){bridge.select_game(root.clone())?;}
        let exe_hash = crate::installation::hash(&root.join("noita.exe"))?;
        let lua_hash = crate::installation::hash(&root.join("lua51.dll"))?;
        self.installation_state = format!(
            "Installation verified: {}",
            crate::installation::approved_candidate_pair(&exe_hash, &lua_hash)
                .unwrap_or("approved build")
        );
        if let Some(preferences)=&self.installation_preferences { preferences.save(&root)?; }
        self.preferred_game_path=Some(root);
        Ok(executable)
    }
    pub(super) fn restore_local_installation(&mut self){self.discover_game();}
    fn discover_game(&mut self) {
        if let Some(root)=self.game_path.clone(){
            if cfg!(feature="beta-portable")&&self.social_requested&&!self.social_enabled&&self.info.is_some(){if let Err(error)=self.configure_game(root.to_str().unwrap_or("")){self.installation_state=format!("Matching service setup needs attention: {error}");}}
            return;
        }
        let preferred=self.preferred_game_path.clone();
        if let Some(root)=preferred {
            match self.configure_game(root.to_str().unwrap_or("")) {
                Ok(_)=>return,
                Err(error)=>self.installation_state=format!("Remembered Noita folder needs attention: {error}"),
            }
        }
        self.discover_candidates(crate::installation::discover_candidates());
    }
    fn discover_candidates(&mut self,candidates:Vec<std::path::PathBuf>) {
        for root in candidates {
        match self.configure_game(root.to_str().unwrap_or("")) {
            Ok(_) => {
                self.installation_state = format!(
                    "Detected Noita installation and verified candidate artifacts: {}",
                    self.installation_state
                )
                ;return;
            }
            Err(error) => {
                self.installation_state = format!("Detected Noita needs verification: {error}")
            }
        }
        }
        self.installation_state=format!("{} · Choose your supported Noita folder",self.installation_state);
    }
    pub async fn load_game(&mut self) -> Result<(), String> {
        if self.lc1{return Err("The selected lobby lifecycle prepares normal New Game automatically".into())}
        if self.automatic_setup_enabled(){return self.setup_tick().await;}
        if self.social_requested{self.refresh_social_profile_checked().await?;}
        let root = self
            .game_path
            .clone()
            .ok_or("Verify your Noita installation before Load game; no preparation was issued")?;
        self.verify_game(&root)?;
        // A previously selected installation is not proof that the currently
        // selected social service has an enabled, matching verified closure.
        if self.social_requested{
            let metadata=self.social_profile.clone().ok_or("No social profile")?;
            self.enable_social(&root,&metadata,self.social_stream.clone().ok_or("No social listener")?)?;
        }
        if self.expected_game.lock().unwrap().is_none() {
            return Err("Verified game connection path unavailable".into());
        }
        self.prepare().await?;
        self.arm()
    }
    fn game_verifier(&self, root: &std::path::Path) -> Result<impl FnOnce() -> Result<(), String> + Send + 'static, String> {
        let profile = self.info.as_ref().and_then(|i| i.profile.as_ref())
            .ok_or("Service has no candidate native profile")?.clone();
        let root=root.to_path_buf();let g3=self.g3.is_some();let social=self.social_requested;
        let social_profile=self.social_profile.clone();let service=self.service.clone();
        #[cfg(test)]let verification_hook=self.world.verification_hook.clone();
        Ok(move || {
            #[cfg(test)]if let Some(hook)=verification_hook{hook();}
            if g3 {
                let metadata=serde_json::to_value(&profile).map_err(|_|"Invalid G3 service profile")?;
                if social {nt2_desktop_g0::g3_social_installation::verify_development(&root,social_profile.as_ref().ok_or("Compatible development social service required")?)?;return Ok(())}
                if service!=nt2_desktop_g0::g3_installation::profile()?["serviceId"].as_str().ok_or("Missing reviewed development service")?{return Err("Choose the exact reviewed A1 seed/ghost service".into())}
                nt2_desktop_g0::g3_installation::verify(&root,&metadata)?;return Ok(())
            }
            crate::installation::verify_candidate(&root,&profile).map(|_| ())
        })
    }
    fn verify_game(&mut self, root: &std::path::Path) -> Result<(), String> {
        let result=(self.game_verifier(root)?)();
        if let Err(error)=&result{self.compatibility_verification_failure(error);}else{self.compatibility_notice.clear("installed_runtime");}
        result
    }
    pub fn arm(&mut self) -> Result<(), String> {
        let root = self
            .game_path
            .clone()
            .ok_or("Choose and verify a supported Noita folder before New Game")?;
        self.verify_game(&root)?;
        if self.pending.lock().unwrap().is_some() {
            return Err("A native preparation is already armed".into());
        }
        let preparation = self
            .preparation
            .as_ref()
            .ok_or("Obtain this desktop's preparation first")?;
        let room = self.room.as_ref().ok_or("No room")?;
        if preparation.room_id != room.room_id
            || preparation.run_id != room.run_id
            || preparation.run_epoch != room.run_epoch
            || preparation.config_revision != room.config_revision
            || preparation.state.as_deref() != Some("issued")
        {
            return Err("Preparation is not current".into());
        }
        let remaining=self.preparation_remaining(preparation,false)?.min(self.preparation_remaining(preparation,true)?);
        if remaining == 0 {
            return Err("Preparation expired".into());
        }
        let policy=self.supported_policy(room.config.as_ref().ok_or("Missing configuration")?)?;
        let mut pending = handoff::new(
            policy.seed,
            required(room.run_id.clone())?,
            u64::from(room.config_revision.ok_or("Missing config revision")?),
        );
        pending.world_policy=policy.bits;
        pending.epoch = required(room.run_epoch.clone())?;
        pending.native_v2 = true;
        pending.preparation_id = preparation.preparation_id.clone();
        pending.expires = Instant::now() + Duration::from_millis(remaining);
        pending.authority_until = Instant::now() + Duration::from_secs(12);
        pending.generation = Some((
            self.generation.clone(),
            self.generation.load(Ordering::Acquire),
        ));
        let context = serde_json::to_vec(&(
            self.service.as_str(),
            self.epoch.as_str(),
            preparation.preparation_id.as_deref(),
            preparation.desktop_id.as_deref(),
            preparation.desktop_session_epoch.as_deref(),
        ))
        .map_err(|_| "Preparation context encoding failed")?;
        pending.claim_path = Some(self.claims_root.join(hex(&Sha256::digest(context))));
        if self.rejoin_transport.offline.is_some(){return Err("Original offline retirement is still being confirmed".into());}
        // Stage one immutable local preparation before Waiting for Noita. This
        // contains no service credential and cannot authorize entering play.
        let desktop=self.desktop.as_ref().ok_or("Missing startup desktop owner")?;
        let scope=nt2_desktop_g0::startup_local::Scope{
            origin:self.base.clone(),service:self.service.clone(),service_epoch:self.epoch.clone(),account:desktop.account.clone(),device:self.device,
            desktop:desktop.id.clone(),desktop_epoch:desktop.epoch.clone(),room:required(room.room_id.clone())?,run:pending.run.clone(),run_epoch:pending.epoch.clone(),config:pending.config,
            config_digest:nt2_desktop_g0::protocol_connection::canonical_fingerprint(&serde_json::to_vec(room.config.as_ref().ok_or("Missing startup configuration")?).map_err(|_|"Startup configuration encoding")?)?,
            preparation:required(preparation.preparation_id.clone())?,
        };
        self.startup_stage=Default::default();self.startup_stage.config=Some(serde_json::to_value(room.config.as_ref().ok_or("Missing startup frozen configuration")?).map_err(|_|"Startup frozen configuration encoding")?);
        pending.startup_bundle=Some(Arc::new(nt2_desktop_g0::startup_local::Prepared::stage(scope,pending.seed,pending.world_policy,self.generation.load(Ordering::Acquire),pending.expires)?));
        self.rejoin_transport=Default::default();self.native_diagnostics.lock().unwrap().rejoin=Default::default();
        *self.pending.lock().unwrap() = Some(pending);
        self.native_revoked = false;
        self.native_failure = None;
        self.native_stage = "reserving-startup";
        self.last_native_observation = None;
        self.last_service_observation = 0;
        let current = self.pending.lock().unwrap().clone().unwrap();
        if !self.native_authority_valid(&current) {
            self.revoke_native();
            return Err("Preparation authority no longer current".into());
        }
        Ok(())
    }
    pub fn new(
        store: S,
        device: uuid::Uuid,
        pending: Arc<Mutex<Option<handoff::Pending>>>,
        claims_root: std::path::PathBuf,
    ) -> Self {
        Self {
            lc1:false,
            auth_owner:Arc::new(Mutex::new(Default::default())),
            presence_owner:Arc::new(Mutex::new(Default::default())),
            world:Default::default(),
            world_files:Arc::new(nt2_desktop_g0::world_store::Files::user()),
            generation: Arc::new(AtomicU64::new(1)),
            pending,
            expected_game: Arc::new(Mutex::new(None)),
            native_diagnostics: Arc::new(Mutex::new(crate::pipe::NativeDiagnostics::default())),
            provisional:None,
            replacement_lineage:Arc::new(Default::default()),
            entry:Default::default(),
            store:Arc::new(store),
            http: reqwest::Client::builder()
                .use_native_tls()
                .no_proxy()
                .redirect(reqwest::redirect::Policy::none())
                .timeout(Duration::from_secs(5))
                .build()
                .expect("candidate transport"),
            device,
            profile: None,
            base: String::new(),
            service: String::new(),
            epoch: String::new(),
            info: None,
            desktop: None,
            auth: None,
            auth1: Default::default(),
            ux1: Default::default(),
            auth_begin: None,
            guest_retry: None,
            guest_generation: 0,
            refresh_retry: None,
            pending_launch: None,
            open_browser: true,
            remember: false,
            room: None,
            rooms: vec![],
            next_room: None,
            room_cursor: String::new(),
            preparation: None,
            game: None,
            native_revoked: false,
            attach_retry: None,
            attach_cycle: None,
            inspections: vec![],
            inspection_cursor: String::new(),
            chat: vec![],
            chat_retry: None,
            chat_feedback:Value::Null,
            status_error: None,
            login_error: None,
            identity_mismatch: None,
            compatibility_notice: Default::default(),
            optional_failures:Default::default(),
            control_failure: FailureSnapshot::default(),
            control_failure_context: Value::Null,
            control_stage: FailureStage::Authentication,
            native_failure: None,
            native_stage: "not-prepared",
            last_native_observation: None,
            last_service_observation: 0,
            auth_state: String::new(),
            next_work: Instant::now(),
            game_path: None,
            installation_state: "Connect to verify the candidate installation automatically".into(),
            installation_diagnostics: String::new(),
            claims_root,
            preferences: None,
            installation_preferences: None,
            preferred_game_path: None,
            setup: setup::Setup::default(),
            resume_intent:Default::default(),
            rejoin_transport:Default::default(),
            waiting:Default::default(),
            startup_stage:Default::default(),
            waiting_retirement:Default::default(),
            parked_recovery:Vec::new(),parking_loaded:None,last_original_outcome:None,untimed_refresh:None,
            lifecycle:Arc::new(Mutex::new(lifecycle::Coordinator::default())),
            recent_servers: vec![],
            g3: None,
            g3_release: None,
            #[cfg(test)] social_fixture:false,
            social_owner:None,social_work:Default::default(),bank_work:Default::default(),c1_work:Default::default(),d1_work:Default::default(),race_work:Default::default(),race_retained:None,c2_work:Default::default(),transfer_cleanup:Default::default(),
            social_stream:None,
            social_requested:false,
            social_profile:None,
            social_consumer:None,
            social_next:Instant::now(),
            social_enabled:false,
            social_verified:None,
            social_credentials:Default::default(),
            credential_writer:Arc::new(Mutex::new(())),
            auth_writer:Arc::new(Mutex::new(())),
            health_enabled:false,health_ticket:None,health_nonce:Arc::new(AtomicU64::new(1)),health_updates:Arc::new(Mutex::new(None)),health_seen:0,health_publisher:None,debug:None,
        }
    }
    fn request(&self, game: bool) -> wire::Request {
        let session = if game {
            self.game.as_ref().map(|g| &g.session)
        } else {
            self.desktop.as_ref()
        };
        wire::Request {
            major: Some(1),
            revision: Some(protocol::REVISION.into()),
            service_id: Some(self.service.clone()),
            service_epoch: Some(self.epoch.clone()),
            request_id: Some(uuid::Uuid::new_v4().to_string()),
            session_epoch: session.map(|s| s.epoch.clone()),
            room_id: self.room.as_ref().and_then(|r| r.room_id.clone()),
            expected_revision: self.room.as_ref().and_then(|r| r.revision),
            config_revision: self.room.as_ref().and_then(|r| r.config_revision),
            run_id: self.room.as_ref().and_then(|r| r.run_id.clone()),
            run_epoch: self.room.as_ref().and_then(|r| r.run_epoch.clone()),
            ..Default::default()
        }
    }
    async fn rpc(
        &mut self,
        request: &wire::Request,
        game: bool,
        anonymous: bool,
    ) -> Result<wire::Response, String> {
        self.rpc_as(request, game, anonymous, None).await
    }
    async fn rpc_as(
        &mut self,
        request: &wire::Request,
        game: bool,
        anonymous: bool,
        override_token: Option<&SecretString>,
    ) -> Result<wire::Response, String> {
        let generation = self.generation.load(Ordering::Acquire);
        #[cfg(feature="g3-rejoin")]
        let sample_stamp=self.lifecycle.lock().unwrap().request_stamp(&json!([self.base,self.service,self.epoch,request.session_epoch,game,anonymous]).to_string(),request.request_id.as_deref().ok_or("Missing request timing identity")?)?;
        let body = zeroize::Zeroizing::new(
            serde_json::to_vec(request).map_err(|_| "Candidate request encoding failed")?,
        );
        protocol::request(&body).map_err(|_| "Candidate request exceeds canonical bounds")?;
        let mut call = self
            .http
            .post(format!("{}/v1/rpc", self.base))
            .header("content-type", "application/json")
            .body(body.to_vec());
        if !anonymous {
            if let Some(token) = override_token {
                call = call.bearer_auth(token.as_str());
            } else {
                let session = if game {
                    self.game.as_ref().map(|g| &g.session)
                } else {
                    self.desktop.as_ref()
                }
                .ok_or("Sign in first")?;
                call = call.bearer_auth(session.access.as_str());
            }
        }
        let response = call.send().await.map_err(|_| "Candidate connection unavailable")?;
        if response.status()==reqwest::StatusCode::UNAUTHORIZED{return Err("UNAUTHORIZED".into());}
        if response.status()==reqwest::StatusCode::FORBIDDEN{return Err("FORBIDDEN".into());}
        let mut response=response.error_for_status()
            .map_err(|_| "Candidate HTTP refused")?;
        let mut bytes = zeroize::Zeroizing::new(Vec::new());
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| "Candidate response unavailable")?
        {
            if bytes.len() + chunk.len() > 65536 {
                return Err("Candidate response too large".into());
            }
            bytes.extend_from_slice(&chunk);
        }
        if self.generation.load(Ordering::Acquire) != generation || self.health_ticket.is_some_and(|ticket|self.health_nonce.load(Ordering::Acquire)!=ticket) {
            return Err("Client action cancelled".into());
        }
        let value = protocol::response(&bytes)
            .map_err(|_| "Candidate response violates canonical schema")?;
        if value.request_id != request.request_id
            || value.service_id.as_deref() != Some(&self.service)
        {
            return Err("Candidate response context changed".into());
        }
        if value.service_epoch.as_deref() != Some(&self.epoch) {
            #[cfg(feature="g3-rejoin")]
            if self.resume_intent.association().is_some(){
                self.rejoin_transport.rediscover=true;self.g3_pause();
                if let Some(p)=self.pending.lock().unwrap().as_mut(){p.release=false;p.authority_until=Instant::now();}
                return Err("Recovery service epoch changed".into());
            }

            self.clear_game();
            self.room = None;
            self.desktop = None;
            self.auth = None;
            self.auth_begin = None;
            self.guest_retry = None;
            self.refresh_retry = None;
            self.chat_retry = None;
            self.pending_launch = None;
            self.chat.clear();
            self.info = None;
            self.generation.fetch_add(1, Ordering::AcqRel);
            self.inspections.clear();
            return Err("Service epoch changed; reconnect".into());
        }
        #[cfg(feature="g3-rejoin")]
        if value.error.is_none(){self.record_credential_reply(&value,request.request_id.as_deref().unwrap_or(""),sample_stamp,game,override_token)?;}
        if let Some(error) = value.error {
            return Err(error
                .code
                .unwrap_or_else(|| "Candidate command refused".into()));
        }
        Ok(value)
    }
    fn absorb(&mut self, response: &wire::Response) {
        self.sync_health();
        if response.room.as_ref().zip(self.room.as_ref()).is_some_and(|(new,old)|new.room_id==old.room_id&&new.revision<old.revision){return;}
        if let Some(room) = &response.room {
            if self.room.as_ref().is_some_and(|old| {
                old.room_id != room.room_id
                    || old.run_id != room.run_id
                    || old.run_epoch != room.run_epoch
            }) {
                self.clear_game();
                self.inspections.clear();
                self.inspection_cursor.clear();
            }
            if self.room.as_ref().and_then(|r| r.room_id.as_ref()) != room.room_id.as_ref() {
                self.inspections.clear();
                self.chat.clear();
            }
            self.inspections.retain(|report| {
                room.members.as_ref().is_some_and(|members| {
                    members
                        .iter()
                        .any(|member| member.desktop_id == report.desktop_id)
                })
            });
            self.room = Some(room.clone());
        }
        self.ux1_projection(response);
        if let Some(preparation) = &response.preparation {
            self.preparation = Some(preparation.clone());
        }
        let invalid = self
            .pending
            .lock()
            .unwrap()
            .as_ref()
            .is_some_and(|p| !self.native_authority_valid(p));
        if invalid {
            self.revoke_native();
        }
    }
    fn clear_game(&mut self) {
        if self.lc1{self.auth_owner.lock().unwrap().retire();self.world=Default::default();let bridge=Arc::new(crate::pipe::world::Bridge::new());self.world.bridge=Some(bridge.clone());let mut d=self.native_diagnostics.lock().unwrap();d.world=Some(bridge);d.world_fence=Some(self.lc1_preparation_fence());d.rejoin=Default::default();}
        if self.health_ticket.is_none(){self.lifecycle.lock().unwrap().retire_owner();}
        self.cancel_health();
        self.native_stage="preparation-cleared";
        if self.pending.lock().unwrap().is_some()||self.preparation.is_some()||self.game.is_some(){
            self.native_failure=Some("Preparation cleared after a room/session change or End. Keep this world held; cancel and prepare a fresh world before another shared run.".into());
        }
        self.g3_retire();
        self.g3_release=None;
        *self.pending.lock().unwrap() = None;
        self.preparation = None;
        self.game = None;
        self.attach_retry = None;
        self.attach_cycle = None;
        self.native_revoked = false;
    }
    fn revoke_native(&mut self) {
        if self.lc1{self.lc1_retire_preparation();}
        self.resume_intent.transient_loss(Instant::now());
        self.cancel_health();
        self.native_stage="authority-revoked";
        if self.pending.lock().unwrap().is_some()||self.preparation.is_some()||self.game.is_some(){self.native_failure=Some("Preparation authority revoked. This world remains held; use Leave NT2 for offline play or cancel and prepare a fresh world.".into());}
        self.g3_retire();
        self.g3_release=None;
        let mut pending = self.pending.lock().unwrap();
        if pending.as_ref().is_some_and(|p| p.owner.is_none()) {
            *pending = None;
        } else if let Some(p) = pending.as_mut() {
            p.release = false;
            p.authority_until = Instant::now();
        }
        self.native_revoked = true;
        self.preparation = None;
        self.game = None;
        self.attach_retry = None;
        self.attach_cycle = None;
    }
    fn native_authority_valid(&self, p: &handoff::Pending) -> bool {
        if self.lc1{return p.owner==self.world.coordinator.ticket.as_ref().map(|t|t.native)&&self.world.record.as_ref().is_some_and(|r|r.intent==nt2_desktop_g0::world_local::Intent::Rejoin&&r.identity.origin==self.base&&r.identity.service==self.service&&r.identity.device==self.device&&self.desktop.as_ref().is_some_and(|d|d.account==r.identity.account))&&p.generation.as_ref().is_some_and(|(g,n)|Arc::ptr_eq(g,&self.generation)&&g.load(Ordering::Acquire)==*n)}
        if self.waiting.contact.is_some(){return self.waiting_authority_valid(p);}
        if self.rejoin_transport.contact.is_some(){return self.rejoin_authority_valid(p);}

        if self.native_revoked
            || !p.native_v2
            || p.generation.as_ref().is_none_or(|(g, n)| {
                !Arc::ptr_eq(g, &self.generation) || g.load(Ordering::Acquire) != *n
            })
        {
            return false;
        }
        let (Some(room), Some(desktop), Some(grant)) =
            (&self.room, &self.desktop, &self.preparation)
        else {
            return false;
        };
        if !["staging", "running"].contains(&room.state.as_deref().unwrap_or(""))
            || room.run_id.as_deref() != Some(&p.run)
            || room.run_epoch.as_deref() != Some(&p.epoch)
            || room.config_revision.map(u64::from) != Some(p.config)
            || room.config.as_ref().and_then(|c|seed_policy::resolved(c).ok()).is_none_or(|policy|policy.seed!=p.seed||policy.bits!=p.world_policy)
        {
            return false;
        }
        #[cfg(feature="g3-rejoin")]
        if p.loaded&&p.owner.is_some()&&self.game.as_ref().is_some_and(|g|Some(g.owner)==p.owner&&g.session.account==desktop.account){return true;}
        let member = room.members.as_ref().and_then(|members| {
            members.iter().find(|m| {
                m.desktop_id.as_deref() == Some(&desktop.id)
                    && m.account_id.as_deref() == Some(&desktop.account)
            })
        });
        if member.is_none_or(|m| {
            m.present != Some(true)
                || (room.state.as_deref() == Some("running") && m.selected != Some(true))
        }) {
            return false;
        }
        if p.preparation_id != grant.preparation_id {
            return false;
        }
        if grant.preparation_id.as_ref().is_none_or(String::is_empty)
            || grant.service_id.as_deref() != Some(&self.service)
            || grant.service_epoch.as_deref() != Some(&self.epoch)
            || grant.room_id != room.room_id
            || grant.run_id != room.run_id
            || grant.run_epoch != room.run_epoch
            || grant.config_revision != room.config_revision
            || grant.desktop_id.as_deref() != Some(&desktop.id)
            || grant.desktop_session_epoch.as_deref() != Some(&desktop.epoch)
        {
            return false;
        }
        let state = grant.state.as_deref().unwrap_or("");
        if p.owner.is_none() {
            return room.state.as_deref() == Some("staging")
                && state == "issued"
                && Instant::now() < p.expires
                && self.preparation_remaining(grant,false).is_ok_and(|v|v>0)
                && self.preparation_remaining(grant,true).is_ok_and(|v|v>0);
        }
        if !["issued", "consumed", "loaded"].contains(&state) {
            return false;
        }
        let entered = self
            .game
            .as_ref()
            .is_some_and(|g| g.owner == p.owner.unwrap() && g.entered && g.permit.is_some());
        if room.state.as_deref() == Some("running") && entered {
            return true;
        }
        let load_confirmed = p.loaded
            && self
                .game
                .as_ref()
                .is_some_and(|g| g.owner == p.owner.unwrap() && g.reported > 0);
        self.preparation_remaining(grant,true).is_ok_and(|v|v>0)
            && (state == "loaded"
                || load_confirmed
                || (Instant::now() < p.expires
                    && self.preparation_remaining(grant,false).is_ok_and(|v|v>0)))
    }
    fn credential_profile(&self, origin: &str, service: &str) -> Result<ServiceProfile, String> {
        let digest = Sha256::digest(
            [
                b"nt2-g2next-credential-v1".as_slice(),
                self.device.as_bytes(),
            ]
            .concat(),
        );
        ServiceProfile::new(
            origin,
            service,
            uuid::Uuid::from_bytes(digest[..16].try_into().unwrap()),
        )
        .map_err(|e| e.to_string())
    }
    fn save(&self, refresh: &str, request_id: &str, started: u64) -> Result<(), String> {
        let _writer=self.credential_writer.lock().unwrap();
        let profile = self.profile.as_ref().ok_or("No selected service")?;
        if !self.remember {
            return self.store.delete(profile).map_err(|e| e.to_string());
        }
        let record = Remembered {
            revision: protocol::REVISION.into(),
            device: self.device.to_string(),
            refresh: refresh.into(),
            request_id: request_id.into(),
            started,
        };
        let blob =
            SecretBlob::new(serde_json::to_vec(&record).map_err(|_| "Credential encoding failed")?)
                .map_err(|e| e.to_string())?;
        self.store.save(profile, &blob).map_err(|e| e.to_string())
    }
    fn install_session(&mut self, credentials: wire::Credentials) -> Result<(), String> {
        let session = Session::read(credentials, &self.device.to_string(), "desktop")?;
        self.cancel_health();
        self.save(session.refresh.as_str(), "", 0)?;
        self.clear_game();
        self.room = None;
        self.inspections.clear();
        self.chat.clear();
        self.desktop = Some(session);
        self.remember_account_name();
        self.lifecycle.lock().unwrap().identity_changed();
        self.park_foreign_recovery()?;
        self.auth = None;
        self.auth_begin = None;
        self.guest_retry = None;
        self.pending_launch = None;
        self.auth_state = "Signed in".into();
        self.login_error = None;
        Ok(())
    }
    pub async fn select(
        &mut self,
        origin: &str,
        service: &str,
        remember: bool,
    ) -> Result<(), String> {
        self.identity_mismatch = None;
        self.select_identity(origin, service, remember, None).await
    }
    pub fn login_result(&mut self, result: &Result<(), String>) {
        self.login_error = result.as_ref().err().cloned();
    }
    pub async fn login(&mut self, origin: &str, remember: bool, provider: &str,
        guest: Option<&str>, recovery: Option<IdentityRecovery>) -> Result<(), String> {
        let result = async {
            if let Some(recovery) = recovery {
                self.recover_identity(origin, remember, recovery).await?;
            } else {
                self.select(origin, "", remember).await?;
            }
            if self.desktop.is_none() {
                if let Some(name) = guest { self.guest(name).await?; }
                else { self.sign_in(provider).await?; }
            }
            if self.desktop.is_some()&&self.info.as_ref().and_then(|i|i.mock_provider)!=Some(true){self.auth1.active=true;self.poll_auth1();}
            self.remember_account_name();
            Ok(())
        }.await;
        self.login_result(&result);
        result
    }
    pub async fn recover_identity(
        &mut self, origin: &str, remember: bool, recovery: IdentityRecovery,
    ) -> Result<(), String> {
        let address = self.credential_profile(origin, "discovery")?;
        let challenge = self.identity_mismatch.as_ref().ok_or("Identity recovery expired; click Sign in again")?;
        if recovery != challenge.recovery || address.origin() != recovery.origin
            || self.generation.load(Ordering::Acquire) != challenge.generation
            || self.base != challenge.base || self.service != challenge.service {
            return Err("Identity recovery changed; click Sign in again".into());
        }
        self.select_identity(origin, "", remember, Some(recovery)).await
    }
    async fn select_identity(
        &mut self, origin: &str, service: &str, remember: bool, recovery: Option<IdentityRecovery>,
    ) -> Result<(), String> {
        // Validate the exact origin before contact. Discovery never carries credentials.
        let address = self.credential_profile(origin, "discovery")?;
        let pin = self
            .recent_servers
            .iter()
            .find(|s| s.origin == address.origin())
            .map(|s| s.service_id.clone())
            .or_else(|| {
                (self.base == address.origin() && !self.service.is_empty())
                    .then(|| self.service.clone())
            });
        if recovery.as_ref().is_some_and(|r| pin.as_deref() != Some(r.expected.as_str())) {
            return Err("Saved identity changed; click Sign in again".into());
        }
        if !service.is_empty() && pin.as_ref().is_some_and(|p| p != service) {
            return Err("Saved server identity differs; choose a different server address".into());
        }
        let generation = self.generation.load(Ordering::Acquire);
        let mut response = self
            .http
            .get(format!("{}/v1/info", address.origin()))
            .send()
            .await
            .map_err(|_| "Service TLS/connection unavailable")?
            .error_for_status()
            .map_err(|_| "Service Info refused")?;
        if !response.status().is_success()
            || response.url().as_str() != format!("{}/v1/info", address.origin())
        {
            self.reject_current_identity(address.origin());
            return Err("Service Info redirect refused".into());
        }
        if response.content_length().is_some_and(|n| n > 65536) {
            self.reject_current_identity(address.origin());
            return Err("Service Info exceeds bounds".into());
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| "Service Info unavailable")?
        {
            if bytes.len() + chunk.len() > 65536 {
                self.reject_current_identity(address.origin());
                return Err("Service Info exceeds bounds".into());
            }
            bytes.extend_from_slice(&chunk);
        }
        if generation != self.generation.load(Ordering::Acquire) {
            return Err("Client action cancelled".into());
        }
        let info = match protocol::info(&bytes) {
            Ok(info) => info,
            Err(_) => {
                self.reject_current_identity(address.origin());
                self.compatibility_service_failure(address.origin(),"Unsupported candidate contract",&bytes);
                return Err("Unsupported candidate contract".into());
            }
        };
        if info.major != Some(1) || info.revision.as_deref() != Some(protocol::REVISION) {
            self.reject_current_identity(address.origin());
            self.compatibility_service_failure(address.origin(),"Service/contract mismatch",&bytes);
            return Err("Service/contract mismatch".into());
        }
        let discovered = required(info.service_id.clone()).map_err(|error| {
            self.reject_current_identity(address.origin());
            error
        })?;
        // Validate the discovered public identity and epoch before offering trust.
        let profile = self.credential_profile(address.origin(), &discovered).map_err(|error| {
            self.reject_current_identity(address.origin()); error
        })?;
        let epoch = required(info.service_epoch.clone()).map_err(|error| {
            self.reject_current_identity(address.origin()); error
        })?;
        if recovery.as_ref().is_some_and(|r| r.discovered != discovered) {
            self.identity_mismatch = None;
            self.reject_current_identity(address.origin());
            return Err("Service identity changed again; click Sign in to review it".into());
        }
        if recovery.is_none() && ((!service.is_empty() && service != discovered)
            || pin.as_ref().is_some_and(|p| p != &discovered)
        ) {
            self.reject_current_identity(address.origin());
            self.identity_mismatch = Some(IdentityMismatch {
                recovery: IdentityRecovery { origin: address.origin().into(),
                    expected: pin.unwrap_or_else(|| service.into()), discovered,
                    ticket: uuid::Uuid::new_v4().to_string() },
                generation: self.generation.load(Ordering::Acquire),
                base: self.base.clone(), service: self.service.clone(),
            });
            return Err(
                "Saved server identity mismatch; remembered credentials were not sent".into(),
            );
        }
        let service = discovered.as_str();
        let fresh = recovery.is_some();
        let context_changed=fresh || self.base != profile.origin()
            || self.service != service
            || (!self.epoch.is_empty() && info.service_epoch.as_deref() != Some(&self.epoch));
        if context_changed {
            self.auth1=Default::default();
            self.stop_setup("Service/session changed; select a lobby");
            self.clear_game();
            self.desktop = None;
            self.room = None;
            self.auth = None;
            self.auth_begin = None;
            self.guest_retry = None;
            self.refresh_retry = None;
            self.chat_retry = None;
            self.inspections.clear();
            self.chat.clear();
            self.rooms.clear();
            self.next_room = None;
            self.room_cursor.clear();
            self.inspection_cursor.clear();
            self.pending_launch = None;
            self.generation.fetch_add(1, Ordering::AcqRel);
        }
        self.base = profile.origin().into();
        self.service = service.into();
        self.epoch = epoch;
        self.profile = Some(profile);
        self.info = Some(info);
        self.compatibility_notice.clear("app_service");self.compatibility_notice.clear("service_response");
        if self.social_requested{
            if context_changed{self.social_enabled=false;self.social_verified=None;self.social_profile=None;}
            self.refresh_social_profile_checked().await?;
            if self.social_enabled{
                let matched=self.game_path.as_ref().zip(self.social_profile.as_ref()).is_some_and(|(root,profile)|
                    self.social_verified.as_ref().is_some_and(|verified|verified.matches(root,profile)));
                if !matched{
                    self.social_enabled=false;self.social_verified=None;self.revoke_native();
                    self.compatibility_profile_failure("Social runtime/profile changed; world remains held");
                    return Err("Social runtime/profile changed; world remains held".into());
                }
            }
        }
        let selected_profile = self.profile.as_ref().unwrap().clone();
        self.remember_server(&selected_profile, service)?;
        self.identity_mismatch = None;
        self.discover_game();
        self.remember = remember;
        if self.desktop.is_none() && remember && !fresh {
            if let Some(blob) = self
                .store
                .load(self.profile.as_ref().unwrap())
                .map_err(|e| e.to_string())?
            {
                let record: Remembered = serde_json::from_slice(blob.bytes())
                    .map_err(|_| "Remembered candidate session invalid")?;
                if record.revision != protocol::REVISION
                    || record.device != self.device.to_string()
                    || record.refresh.len() != 43
                {
                    return Err("Remembered candidate session needs sign-in".into());
                }
                let mut request = self.request(false);
                request.session_epoch = None;
                request.refresh = Some(wire::Refresh {
                    secret: Some(record.refresh.clone()),
                    device_id: Some(record.device.clone()),
                });
                if !record.request_id.is_empty() {
                    self.untimed_refresh=Some(record.request_id.clone());
                    request.request_id = Some(record.request_id.clone());
                }
                self.save(
                    &record.refresh,
                    request.request_id.as_deref().unwrap(),
                    if record.started == 0 {
                        now()
                    } else {
                        record.started
                    },
                )?;
                self.refresh_retry = Some(Retry {
                    request,
                    until: Instant::now() + Duration::from_secs(30),
                });
                self.rotate().await?;
            }
        }
        Ok(())
    }
    pub async fn sign_in(&mut self, provider: &str) -> Result<(), String> {
        if !["website", "twitch", "google", "local"].contains(&provider) {
            return Err("Select a labelled account provider".into());
        }
        if self.auth.is_some() || self.guest_retry.is_some() {
            return Err("Cancel the current sign-in first".into());
        }
        let mock=self.info.as_ref().and_then(|i|i.mock_provider)==Some(true)&&matches!(provider,"website"|"twitch");
        if !mock{
            self.ensure_auth1_discovery().await?;
            if self.auth1_view()["providers"].as_array().is_none_or(|p|!p.iter().any(|p|p["id"]==provider&&p["available"]==true)){let label=match provider{"google"=>"Google","twitch"=>"Twitch","local"=>"Local account",_=>"Selected provider"};return Err(format!("{label} sign-in is not supported by this service. Choose another sign-in method."))}
        }
        let flow = SignIn::new(
            &self.base,
            self.device,
            self.generation.load(Ordering::Acquire),
        )?;
        let mut request = self.request(false);
        request.request_id = Some(flow.begin.request_id.to_string());
        request.auth_begin = Some(wire::AuthBegin {
            device_id: Some(flow.begin.device_id.to_string()),
            app_instance_id: Some(flow.begin.app_instance_id.to_string()),
            provider: Some(provider.into()),
            proof_challenge: Some(flow.begin.proof_challenge.clone()),
            callback_state: Some(flow.begin.callback_state.clone()),
            callback_port: Some(flow.begin.callback_port.into()),
        });
        self.auth = Some(flow);
        self.auth_begin = Some(Retry {
            request,
            until: Instant::now() + Duration::from_secs(30),
        });
        self.auth_state = if mock{"Opening development mock sign-in"}else{"Opening sign-in in your browser"}.into();
        self.auth_tick().await
    }
    pub async fn guest(&mut self, name: &str) -> Result<(), String> {
        if self.auth.is_some() || self.guest_retry.is_some() {
            return Err("Cancel the current sign-in first".into());
        }
        let mut request = self.request(false);
        self.guest_generation = self.generation.load(Ordering::Acquire);
        request.guest_issue = Some(wire::GuestIssue {
            device_id: Some(self.device.to_string()),
            display_name: Some(name.trim().into()),
            issuance_proof: Some(crate::auth_return::random_state()?),
        });
        self.guest_retry = Some(Retry {
            request,
            until: Instant::now() + Duration::from_secs(30),
        });
        self.auth_state = "Creating guest session".into();
        self.auth_tick().await
    }
    async fn auth_tick(&mut self) -> Result<(), String> {
        if self.guest_retry.is_some()
            && self.guest_generation != self.generation.load(Ordering::Acquire)
        {
            self.guest_retry = None;
            self.auth_state = "Guest sign-in cancelled".into();
            return Ok(());
        }
        if self
            .auth
            .as_ref()
            .is_some_and(|flow| flow.proof(self.generation.load(Ordering::Acquire)).is_err())
        {
            self.auth = None;
            self.auth_begin = None;
            self.pending_launch = None;
            self.auth_state = "Sign-in cancelled or expired".into();
            return Ok(());
        }
        if let Some(retry) = self.guest_retry.take() {
            if Instant::now() >= retry.until {
                return Err("Guest issue retry expired".into());
            }
            match self.rpc(&retry.request, false, true).await {
                Ok(mut response) => self
                    .install_session(response.credentials.take().ok_or("Missing guest session")?)?,
                Err(error) => {
                    self.guest_retry = Some(retry);
                    return Err(error);
                }
            }
        }
        if let Some(retry) = self.auth_begin.take() {
            if Instant::now() >= retry.until {
                self.auth = None;
                return Err("Sign-in request retry expired".into());
            }
            match self.rpc(&retry.request, false, true).await {
                Ok(response) => {
                    let attempt = response.auth_attempt.ok_or("Missing sign-in attempt")?;
                    let id = uuid::Uuid::parse_str(&required(attempt.attempt_id)?)
                        .map_err(|_| "Invalid sign-in attempt")?;
                    let launch = required(attempt.launch_url)?;
                    self.auth
                        .as_mut()
                        .ok_or("Sign-in cancelled")?
                        .accept_attempt(self.generation.load(Ordering::Acquire), id, &launch)?;
                    self.pending_launch = Some(launch);
                    self.auth_state = if self.info.as_ref().and_then(|i|i.mock_provider)==Some(true){"Choose your development mock account in the browser"}else{"Complete sign-in in your browser, then return here"}.into();
                    self.login_error = None;
                }
                Err(error) => {
                    self.auth_begin = Some(retry);
                    return Err(error);
                }
            }
        }
        if self.open_browser {
            if let (Some(url), Some(flow)) = (&self.pending_launch, &self.auth) {
                let id = uuid::Uuid::parse_str(url.rsplit('/').next().unwrap_or(""))
                    .map_err(|_| "Invalid sign-in launch")?;
                crate::browser::open_signin(&self.base, url, id)?;
                let _ = flow;
                self.pending_launch = None;
            }
        }
        let completion = if let Some(flow) = &mut self.auth {
            flow.completion(self.generation.load(Ordering::Acquire))?
                .map(|(proof, exchange)| wire::AuthExchange {
                    attempt_id: Some(exchange.attempt_id.to_string()),
                    device_id: None,
                    app_instance_id: None,
                    initiator_proof: Some(proof.as_str().into()),
                    completion_grant: Some(exchange.completion_grant.as_str().into()),
                })
        } else {
            None
        };
        if let Some(mut exchange) = completion {
            let flow = self.auth.as_mut().unwrap();
            exchange.device_id = Some(flow.begin.device_id.to_string());
            exchange.app_instance_id = Some(flow.begin.app_instance_id.to_string());
            let id = flow
                .exchange(self.generation.load(Ordering::Acquire))?
                .unwrap()
                .request_id;
            let mut request = self.request(false);
            request.request_id = Some(id.to_string());
            request.auth_exchange = Some(exchange);
            let retry = Retry {
                request,
                until: Instant::now() + Duration::from_secs(30),
            };
            let mut response = self.rpc(&retry.request, false, true).await?;
            self.install_session(
                response
                    .credentials
                    .take()
                    .ok_or("Missing account session")?,
            )?;
        }
        Ok(())
    }
    pub async fn rotate(&mut self) -> Result<(), String> {
        self.auth1.next_account=None;
        if self.lc1{self.sync_auth_owner();self.publish_auth_owner();self.auth_owner.lock().unwrap().force();return Ok(())}
        self.sync_health();if self.health_active(){return Ok(())}
        let retry = if let Some(retry) = self.refresh_retry.take() {
            retry
        } else {
            let session = self.desktop.as_ref().ok_or("Sign in first")?;
            let mut request = self.request(false);
            request.session_epoch = None;
            request.refresh = Some(wire::Refresh {
                device_id: Some(self.device.to_string()),
                secret: Some(session.refresh.as_str().into()),
            });
            self.save(
                session.refresh.as_str(),
                request.request_id.as_deref().unwrap(),
                now(),
            )?;
            Retry {
                request,
                until: Instant::now() + Duration::from_secs(30),
            }
        };
        if Instant::now() >= retry.until {
            self.clear_game();
            self.desktop = None;
            if let Some(profile) = &self.profile {
                self.store.delete(profile).map_err(|e| e.to_string())?;
            }
            self.auth_state = "Refresh recovery expired; sign in again".into();
            return Err("Session refresh retry expired".into());
        }
        match self.rpc(&retry.request, false, true).await {
            Ok(mut response) => {
                let session = Session::read(
                    response
                        .credentials
                        .take()
                        .ok_or("Missing refreshed session")?,
                    &self.device.to_string(),
                    "desktop",
                )?;
                if self.desktop.as_ref().is_some_and(|old| {
                    old.id != session.id
                        || old.epoch != session.epoch
                        || old.account != session.account
                }) {
                    self.revoke_native();
                    return Err("Refreshed desktop context changed".into());
                }
                self.save(session.refresh.as_str(), "", 0)?;
                self.desktop = Some(session);
                if self.untimed_refresh.take().is_some(){
                    // A restarted original has no recoverable monotonic send time.
                    // Its cached credentials grant no lease; read fresh current auth.
                    let mut probe=self.request(false);probe.presence=Some(wire::Presence{present:Some(true)});
                    self.rpc(&probe,false,false).await?;
                }
                Ok(())
            }
            Err(error) => {
                self.refresh_retry = Some(retry);
                Err(error)
            }
        }
    }
    pub async fn room_page(&mut self, next: bool) -> Result<(), String> {
        let cursor = if next {
            self.next_room.clone().ok_or("No next page")?
        } else {
            String::new()
        };
        let mut request = self.request(false);
        request.room_list = Some(wire::ListRooms {
            limit: Some(32),
            after_room_id: if cursor.is_empty() {
                None
            } else {
                Some(cursor.clone())
            },
        });
        let response = match self.rpc(&request,false,false).await{Ok(response)=>response,Err(error)=>{self.record_optional_failure("room",FailureStage::Room,&error);return Err(error);}};
        self.rooms = response.rooms.unwrap_or_default();
        self.next_room = response.next_room_id.filter(|s| !s.is_empty());
        self.room_cursor = cursor;
        Ok(())
    }
    pub async fn join(&mut self, room: &str, password: Option<String>) -> Result<(), String> where S:'static {
        if self.automatic_setup_enabled() {
            let review=self.setup.preview.as_ref().ok_or("Review this lobby configuration before Join")?;
            return self.join_displayed(room,password,review.revision,review.config_revision).await;
        }
        let mut preview = self.request(false);
        preview.room_id = Some(room.into());
        preview.expected_revision = None;
        preview.config_revision = None;
        preview.run_id = None;
        preview.run_epoch = None;
        preview.room_preview = Some(wire::Empty {});
        let response = self.rpc(&preview, false, false).await?;
        let admission = response.admission.ok_or("Room unavailable")?;
        let mut request = preview;
        request.request_id = Some(uuid::Uuid::new_v4().to_string());
        request.room_preview = None;
        request.room_join = Some(wire::RoomJoin { password });
        request.expected_revision = admission.revision;
        request.config_revision = admission.config_revision;
        let response = self.rpc(&request, false, false).await?;
        if self.room.as_ref().and_then(|r| r.room_id.as_deref()) != Some(room) {
            self.clear_game();
        }
        self.absorb(&response);
        Ok(())
    }
    pub async fn ready(&mut self, ready: bool) -> Result<(), String> {
        if self.automatic_setup_enabled(){return Err("Game readiness comes from accepted protected inspection, not a checkbox".into());}
        let mut request = self.request(false);
        request.ready = Some(wire::Ready { ready: Some(ready) });
        let response = self.rpc(&request, false, false).await?;
        self.absorb(&response);
        Ok(())
    }
    pub async fn prepare(&mut self) -> Result<(), String> {
        if self.lc1{return Err("Fresh generation uses the selected lobby lifecycle intent".into())}
        let mut request = self.request(false);
        request.prepare = Some(wire::Prepare {
            review_minutes: None,
        });
        let response = self.rpc(&request, false, false).await?;
        self.absorb(&response);
        let preparation = response
            .preparation
            .ok_or("No desktop preparation returned")?;
        let desktop = self.desktop.as_ref().ok_or("Sign in first")?;
        if preparation.desktop_id.as_deref() != Some(&desktop.id)
            || preparation.desktop_session_epoch.as_deref() != Some(&desktop.epoch)
            || preparation.service_id.as_deref() != Some(&self.service)
            || preparation.service_epoch.as_deref() != Some(&self.epoch)
        {
            return Err("Preparation context changed".into());
        }
        self.preparation = Some(preparation);
        self.reset_capture_diagnostics();
        Ok(())
    }
    fn reset_capture_diagnostics(&mut self){
        self.control_failure=FailureSnapshot::default();
        self.control_failure_context=Value::Null;
        if let Some(g3)=self.g3.as_mut(){g3.reset_diagnostics();}
    }
    fn control_context(&self)->Value{
        let pending=self.pending.lock().unwrap();
        let legacy=self.g3_release.as_ref().and_then(|r|r.phase.as_deref()).filter(|p|matches!(*p,"playing"|"restoring"|"authorized"));
        let mut current=json!({"source":"legacy","phase":legacy,
            "entered_locally":self.game.as_ref().is_some_and(|g|g.entered),
            "local_revision":pending.as_ref().and_then(|p|p.observation.as_ref().map(|r|r.sequence)),
            "service_revision":self.game.as_ref().map(|g|g.reported)});
        if self.lc1{
            let valid=pending.as_ref().is_some_and(|p|self.game.as_ref().is_some_and(|g|p.owner==Some(g.owner))&&self.native_authority_valid(p)&&self.validate_current_admission(p).is_ok());let co=self.lifecycle.lock().unwrap();let a=&co.authority;
            let scope_matches=a.admission.as_ref().is_some_and(|admitted|self.game.as_ref().is_some_and(|game|admitted.scope.as_ref().is_some_and(|s|s.session_id.as_deref()==Some(&game.session.id)&&s.session_epoch.as_deref()==Some(&game.session.epoch)&&s.attachment_id.as_deref()==Some(&game.session.attachment)&&self.world.record.as_ref().is_some_and(|r|s.world_token.as_deref()==Some(r.world.to_string().as_str())))&&admitted.owner.as_ref().is_some_and(|owner|owner.game_instance_id.as_deref()==Some(&uuid::Uuid::from_bytes(game.owner.1).to_string()))));
            let historical=a.admission.as_ref().filter(|admitted|admitted.decision.as_ref().is_some_and(|d|d.authorized==Some(true))).map(|_|if a.entered{"playing"}else{"authorized"});
            let locally_playing=self.game.as_ref().is_some_and(|g|g.entered)&&a.entered&&pending.as_ref().and_then(|p|p.observation.as_ref()).is_some_and(|r|!r.observation.protected);
            current["source"]=json!("lc1");current["historical_phase"]=json!(historical);current["phase"]=if valid&&scope_matches{json!(if locally_playing{"playing"}else{"authorized"})}else{Value::Null};current["canonical_live"]=json!(a.authority.live(Instant::now()));current["canonical_offered"]=json!(a.offered);current["scope_matches"]=json!(scope_matches);current["current_validated"]=json!(valid&&scope_matches);current["canonical_generation"]=json!(a.admission.as_ref().and_then(|a|a.generation));current["lifecycle_state"]=json!(self.world.coordinator.stage());
        }current
    }
    fn record_control_failure(&mut self,stage:FailureStage,error:&str){
        self.control_failure.record(stage,error);self.control_failure_context=self.control_context();
        self.control_failure_context["binding_reason"]=json!(match error{
            "Native owner context cannot change frozen authority"=>"frozen_owner_changed",
            "LC1 replacement differs from current participation or canonical generation"|"Stale recovered consumer generation"=>"owner_generation_or_scope",
            "Current canonical authority unavailable"=>"canonical_unavailable",
            "Current content differs from admission"=>"content_changed",
            "G3 process identity changed"=>"process_changed",
            _=>"other_refusal",
        });
    }
    fn record_optional_failure(&mut self,kind:&'static str,stage:FailureStage,error:&str){
        self.optional_failures.entry(kind).or_default().record(stage,error);
    }
    pub async fn chat_send(&mut self, text: String) -> Result<(), String> {
        if self.chat_retry.is_some() {
            return Err("Previous chat send is still being retried".into());
        }
        let mut request = self.request(false);
        request.chat_send = Some(wire::ChatSend { text: Some(text) });
        self.chat_feedback=json!({"room":request.room_id,"session":request.session_epoch,"request":request.request_id,"status":"sending"});
        self.chat_retry = Some(Retry {
            request,
            until: Instant::now() + Duration::from_secs(15*60),
        });
        self.retry_chat().await
    }
    async fn retry_chat(&mut self) -> Result<(), String> {
        let Some(retry) = self.chat_retry.take() else {
            return Ok(());
        };
        if retry.request.room_id != self.room.as_ref().and_then(|r| r.room_id.clone())
            || retry.request.session_epoch != self.desktop.as_ref().map(|d| d.epoch.clone())
        {
            return Err("Chat retry expired or context changed".into());
        }
        if Instant::now()>=retry.until||self.chat_feedback["status"]=="expired"{self.chat_feedback["status"]=json!("expired");self.chat_retry=Some(retry);return Err("Earlier chat delivery unknown. Review retained history before acknowledging and separately sending again".into())}
        let response=self.rpc(&retry.request,false,false).await;let response=self.ux1_observe_reply(response);match response {
            Ok(response) => {
                self.chat_feedback=json!({"room":retry.request.room_id,"session":retry.request.session_epoch,"request":retry.request.request_id,"status":"accepted","sequence":response.chat_messages.as_ref().and_then(|m|m.first()).and_then(|m|m.sequence)});
                self.absorb(&response);
                Ok(())
            }
            Err(error) => {
                let definite=ux1::definite_outcome(&error);
                self.chat_feedback=json!({"room":retry.request.room_id,"session":retry.request.session_epoch,"request":retry.request.request_id,"status":if error=="EXPIRED"{"expired"}else if definite{"failed"}else{"uncertain"},"error":error});
                if !definite{self.chat_retry = Some(retry);}
                Err(error)
            }
        }
    }
    async fn collect_chat(&mut self) -> Result<(), String> {
        let mut request = self.request(false);
        request.chat_list = Some(wire::ChatList {
            limit: Some(32),
            after_sequence: self.chat.last().and_then(|m| m.sequence),
        });
        let response = self.rpc(&request, false, false).await?;
        if response.chat_reset == Some(true) {
            self.chat.clear();
        }
        for message in response.chat_messages.unwrap_or_default() {
            if !self.chat.iter().any(|m| m.sequence == message.sequence) {
                self.chat.push(message);
            }
        }
        if self.chat.len() > 250 {
            self.chat.drain(..self.chat.len() - 250);
        }
        Ok(())
    }
    async fn collect_inspection(&mut self) -> Result<(), String> {
        if self.room.as_ref().is_some_and(|r|r.state.as_deref()==Some("ended")){self.inspections.clear();self.inspection_cursor.clear();return Ok(())}
        if self
            .room
            .as_ref()
            .and_then(|r| r.run_id.as_deref())
            .is_none()
        {
            self.inspections.clear();
            return Ok(());
        }
        // One bounded full report per request. Cycle the room roster without monopolizing commands.
        let cursor = if self.inspection_cursor.is_empty() {
            None
        } else {
            Some(self.inspection_cursor.clone())
        };
        let mut request = self.request(false);
        request.inspection_list = Some(wire::InspectionList {
            limit: Some(1),
            after_desktop_id: cursor,
        });
        let response = self.rpc(&request, false, false).await?;
        if let Some(reports) = response.inspections {
            for report in reports {
                self.inspections
                    .retain(|r| r.desktop_id != report.desktop_id);
                self.inspections.push(report);
            }
        }
        self.inspection_cursor = response.inspection_next_desktop_id.unwrap_or_default();
        if self.inspection_cursor.is_empty() {
            // Restart a bounded refresh pass on the next tick.
            self.inspections
                .sort_by(|a, b| a.desktop_id.cmp(&b.desktop_id));
        }
        Ok(())
    }
    pub async fn create_fixture(&mut self, allow_guests: bool) -> Result<(), String> where S:'static {
        if self.info.as_ref().and_then(|info| info.mock_provider) != Some(true) {
            return Err("Development fixture unavailable".into());
        }
        let desktop = self.desktop.as_ref().ok_or("Sign in first")?;
        if desktop.principal.kind.as_deref() != Some("registered") {
            return Err("Guest cannot host".into());
        }
        let subject = match desktop.principal.source.as_deref() {
            Some("mock:website") => "website-local-alex",
            Some("mock:twitch") => "twitch-local-alex",
            _ => return Err("Disposable fixture requires a labelled mock account".into()),
        };
        let device = uuid::Uuid::new_v4().to_string();
        let body = wire::BrowserLogin {
            subject: Some(subject.into()),
            device_id: Some(device.clone()),
            request_id: Some(uuid::Uuid::new_v4().to_string()),
        };
        let response = self
            .http
            .post(format!("{}/v1/mock/browser-login", self.base))
            .json(&body)
            .send()
            .await
            .map_err(|_| "Mock fixture unavailable")?
            .error_for_status()
            .map_err(|_| "Mock fixture refused")?;
        if response.content_length().is_none_or(|size| size > 65536) {
            return Err("Mock fixture response exceeds bounds".into());
        }
        let bytes = zeroize::Zeroizing::new(
            response
                .bytes()
                .await
                .map_err(|_| "Mock fixture response unavailable")?
                .to_vec(),
        );
        let mut response: wire::BrowserLoginResponse =
            protocol::typed("BrowserLoginResponse", &bytes)
                .map_err(|_| "Invalid mock fixture session")?;
        let browser = Session::read(
            response
                .credentials
                .take()
                .ok_or("Missing mock fixture credentials")?,
            &device,
            "browser",
        )?;
        if self
            .desktop
            .as_ref()
            .is_none_or(|desktop| desktop.account != browser.account)
        {
            return Err("Fixture browser does not match signed-in account".into());
        }
        let mut request = self.request(false);
        request.session_epoch = Some(browser.epoch.clone());
        request.room_create = Some(wire::RoomCreate {
            name: Some("Disposable NT2 candidate room".into()),
            capacity:None,password:None,
            visibility: Some("public".into()),
            allow_guests: Some(allow_guests),
            config: Some(wire::Config {
                seed: Some(424242),
                mode: Some("core".into()),
                required_capabilities: Some({let mut caps=if self.g3.is_some(){vec!["core.seed".into(),"core.ghost".into()]}else{vec!["core.seed".into()]};
                    if self.social_requested{caps.extend(["core.hud".into(),"core.poi".into()]);}
                    #[cfg(test)] if self.social_fixture&&!self.social_requested{caps.extend(["core.hud".into(),"core.poi".into()]);} caps}),
                settings: Some(vec![]),
                ..Default::default()
            }),
        });
        #[cfg(feature="g3-seed-loot")]{
            let config=request.room_create.as_mut().unwrap().config.as_mut().unwrap();
            let policy="shared-random".to_string();
            #[cfg(test)] let policy=std::env::var("NT2_U2_POLICY").unwrap_or(policy);
            config.seed=Some(if policy=="shared-fixed"{424242}else{0});config.seed_policy=Some(policy.clone());
            config.randomize_loot=Some(policy!="individual-random");
            #[cfg(test)] if std::env::var("NT2_U2_LOOT_OFF").as_deref()==Ok("1"){config.randomize_loot=Some(false);}
            let caps=config.required_capabilities.get_or_insert_default();caps.push("core.seed-policy.v1".into());if config.randomize_loot==Some(true){caps.push("core.randomize-loot.v1".into());}
        }
        let response = self
            .rpc_as(&request, false, false, Some(&browser.access))
            .await?;
        self.clear_game();
        self.absorb(&response);
        let id = self
            .room
            .as_ref()
            .and_then(|room| room.room_id.clone())
            .ok_or("Missing disposable room")?;
        if self.automatic_setup_enabled(){self.review_created_lobby()?;}
        self.join(&id, None).await?;
        Ok(())
    }
    /// Retire only the human sign-in attempt. Choosing a lobby, a protected
    /// saved world and its authority generation belong to their existing owners.
    pub async fn cancel_sign_in(&mut self) -> Result<(), String> {
        let cancel=self.auth.as_ref().and_then(|flow|flow.cancellation().map(|(id,proof)|wire::AuthProof{
            attempt_id:Some(id.to_string()),device_id:Some(flow.begin.device_id.to_string()),
            app_instance_id:Some(flow.begin.app_instance_id.to_string()),initiator_proof:Some(proof.as_str().into()),
        }));
        self.auth=None;self.auth_begin=None;self.guest_retry=None;self.pending_launch=None;
        self.auth_state="Sign-in cancelled; choose a sign-in option to retry".into();self.login_error=None;
        if let Some(cancel)=cancel{
            let mut request=self.request(false);request.auth_cancel=Some(cancel);
            let retry=Retry{request,until:Instant::now()+Duration::from_secs(30)};
            if self.rpc(&retry.request,false,true).await.is_err(){self.login_error=Some("Sign-in cancelled locally; the service could not confirm cancellation".into());}
        }
        Ok(())
    }
    pub async fn cancel(&mut self) -> Result<(), String> {
        if !self.lc1{self.resume_intent.terminal();}
        self.stop_setup("Cancelled; select a lobby again when ready for a fresh New Game");
        let cancel = self.auth.as_ref().and_then(|flow| {
            flow.cancellation().map(|(id, proof)| wire::AuthProof {
                attempt_id: Some(id.to_string()),
                device_id: Some(flow.begin.device_id.to_string()),
                app_instance_id: Some(flow.begin.app_instance_id.to_string()),
                initiator_proof: Some(proof.as_str().into()),
            })
        });
        self.generation.fetch_add(1, Ordering::AcqRel);
        self.auth = None;
        self.auth_begin = None;
        self.guest_retry = None;
        self.pending_launch = None;
        self.auth_state = "Sign-in/preparation cancelled".into();
        let preparation = self.preparation.clone();
        self.clear_game();
        if let Some(cancel) = cancel {
            let mut request = self.request(false);
            request.auth_cancel = Some(cancel);
            let retry = Retry {
                request,
                until: Instant::now() + Duration::from_secs(30),
            };
            self.rpc(&retry.request, false, true).await?;
        }
        if let Some(preparation) = preparation {
            let mut request = self.request(false);
            request.preparation_cancel = Some(wire::PreparationCancel {
                preparation_id: preparation.preparation_id,
            });
            let response = self.rpc(&request, false, false).await?;
            self.absorb(&response);
        }
        Ok(())
    }
    pub async fn logout(&mut self) -> Result<(), String> {
        self.stop_setup("Signed out");
        self.identity_mismatch = None;
        self.login_error = None;
        self.generation.fetch_add(1, Ordering::AcqRel);
        self.clear_game();
        self.auth = None;
        self.auth_begin = None;
        self.guest_retry = None;
        self.refresh_retry = None;
        self.chat_retry = None;
        self.pending_launch = None;
        let mut request = self.request(false);
        request.logout = Some(wire::Empty {});
        let remote = if self.desktop.is_some() {
            self.rpc(&request, false, false).await.map(|_| ())
        } else {
            Ok(())
        };
        self.desktop = None;
        self.remember_account_name();
        self.room = None;
        self.rooms.clear();
        self.inspections.clear();
        self.chat.clear();
        self.auth1=Default::default();
        self.auth_state = "Signed out locally".into();
        let _writer=self.credential_writer.lock().unwrap();
        if let Some(profile) = &self.profile {
            self.store.delete(profile).map_err(|e| e.to_string())?;
        }
        remote
    }
    pub async fn tick(&mut self) -> Result<(), String> where S:'static {
        if self.lc1{
            #[cfg(test)]crate::next_app::scheduler::fixture_core_phase("tick.sync_auth_owner");
            self.sync_auth_owner();
            #[cfg(test)]crate::next_app::scheduler::fixture_core_phase("tick.publish_auth_owner");
            self.publish_auth_owner();
            #[cfg(test)]crate::next_app::scheduler::fixture_core_phase("tick.lc1_tick");
            let result=self.lc1_tick().await;
            #[cfg(test)]crate::next_app::scheduler::fixture_core_phase("tick.complete");
            return result
        }
        self.sync_health();
        // Skipped 200ms scheduler wakes are not successful network polls and must
        // not erase the diagnostic from the most recent real 2s poll.
        if self.info.is_none() || Instant::now() < self.next_work {
            return Ok(());
        }
        let result = self.tick_inner().await;
        if let Err(error) = &result {
            if matches!(self.control_stage, FailureStage::Authentication) {
                self.login_error = Some(error.clone());
            }
            self.record_control_failure(self.control_stage,error);
            match self.control_stage{FailureStage::Chat=>self.record_optional_failure("chat",self.control_stage,error),FailureStage::Inspection=>self.record_optional_failure("inspection",self.control_stage,error),FailureStage::Room=>self.record_optional_failure("room",self.control_stage,error),_=>{}}
            if !matches!(self.control_stage,FailureStage::Chat|FailureStage::Inspection|FailureStage::Room)
                || error=="Candidate response context changed" {self.protect_health_error(error);}
            self.status_error = Some(error.clone());
        } else {
            self.status_error = None;
        }
        if result.is_ok()&&!self.health_active()&&!self.entry.holding{if let Err(error)=self.g3_sync(){self.record_control_failure(FailureStage::Binding,&error);return Err(error);}}
        self.publish_health();
        result
    }
    async fn tick_inner(&mut self) -> Result<(), String> where S:'static {
        self.poll_startup_stage().await;
        if self.info.is_none() || Instant::now() < self.next_work {
            return Ok(());
        }
        self.next_work = Instant::now() + Duration::from_secs(2);
        // The same desktop renewal owner must keep running while B is held.
        // Rotate before entry_tick so its context fence retires old HTTP replies.
        let entry_auth=self.provisional.is_some();
        if entry_auth{
            self.control_stage=FailureStage::Authentication;self.auth_tick().await?;
            if self.desktop.as_ref().is_some_and(|d|self.session_expiring(d,60000))||self.refresh_retry.is_some(){
                self.entry.renewal_fence();
                self.control_stage=FailureStage::Session;self.rotate().await?;
            }
        }
        if self.entry_tick().await?{return self.poll_active_views().await;}
        if self.waiting_tick().await?{self.publish_health();return self.poll_active_views().await;}
        self.rejoin_tick().await?;
        if self.rejoin_transport.contact.is_some()||self.health_active(){
            // Recovery/health owns game authority; the app still owns its
            // optional room, chat and inspection projections and chat original.
            self.g3_sync()?;self.publish_health();
            return self.poll_active_views().await;
        }
        self.control_stage=FailureStage::Authentication;
        if !entry_auth{self.auth_tick().await?;}
        if self.desktop.is_none() {
            return Ok(());
        }
        if !entry_auth&&(self.refresh_retry.is_some() || self.session_expiring(self.desktop.as_ref().unwrap(),60000)) {
            self.control_stage=FailureStage::Session;
            self.rotate().await?;
        }
        if self.automatic_setup_enabled() {
            self.control_stage=FailureStage::Room;
            // Setup has its own visible retry state. Keep membership presence and
            // independent health work alive while local installation needs repair.
            let _=self.setup_tick().await;
        }
        if self.room.is_some() {
            self.control_stage=FailureStage::Presence;
            let mut request = self.request(false);
            request.presence = Some(wire::Presence {
                present: Some(true),
            });
            let response = self.rpc(&request, false, false).await?;
            self.absorb(&response);
            let mut request = self.request(false);
            request.room_get = Some(wire::Empty {});
            self.control_stage=FailureStage::Room;
            let response = self.rpc(&request, false, false).await?;
            let previous_run = self.room.as_ref().and_then(|r| r.run_id.clone());
            self.absorb(&response);
            if previous_run != self.room.as_ref().and_then(|r| r.run_id.clone()) {
                self.clear_game();
                self.inspections.clear();
            }
            self.control_stage=FailureStage::Game;
            self.game_tick().await?;
            self.g3_sync()?;self.publish_health();
            self.control_stage=FailureStage::Chat;
            self.retry_chat().await?;
            self.collect_chat().await?;
            self.control_stage=FailureStage::Inspection;
            self.collect_inspection().await?;
        } else {
            self.control_stage=FailureStage::Room;
            self.room_page(false).await?;
        }
        Ok(())
    }
    async fn poll_active_views(&mut self)->Result<(),String>{
        if self.desktop.is_none()||self.room.is_none(){return Ok(());}
        let mut first=None;
        let mut request=self.request(false);request.room_get=Some(wire::Empty{});
        let room_result=match self.rpc(&request,false,false).await{Ok(response)=>{self.absorb(&response);Ok(())},Err(error)=>Err(error)};
        self.app_view_outcome(&mut first,FailureStage::Room,"room",room_result);
        let retry=self.retry_chat().await;
        self.app_view_outcome(&mut first,FailureStage::Chat,"chat",retry);
        let chat=self.collect_chat().await;
        self.app_view_outcome(&mut first,FailureStage::Chat,"chat",chat);
        let inspection=self.collect_inspection().await;
        self.app_view_outcome(&mut first,FailureStage::Inspection,"inspection",inspection);
        self.sync_health();
        if let Some((stage,error))=first{self.control_stage=stage;Err(error)}else{Ok(())}
    }
    fn app_view_outcome(&mut self,first:&mut Option<(FailureStage,String)>,stage:FailureStage,key:&'static str,result:Result<(),String>){
        if let Err(error)=result{
            // A failed optional lane must not starve the other projections.
            // tick records the first error once, after all lanes are attempted.
            if first.is_none(){*first=Some((stage,error));}else{self.record_optional_failure(key,stage,&error);}
        }
    }
    pub fn view(&self) -> Value {
        let installation_state=self.native_diagnostics.lock().unwrap().local_ws.as_ref().and_then(|b|b.error()).unwrap_or_else(||self.installation_state.clone());
        let desktop = self.desktop.as_ref();
        let setup = self.setup_view();
        // A coherent read snapshot; status assembly must not hold the native
        // slot while entering independent diagnostic/lifecycle locks.
        let pending = self.pending.lock().unwrap().clone();
        let current=self.game.as_ref().zip(pending.as_ref()).is_some_and(|(game,p)|p.owner==Some(game.owner)&&self.native_authority_valid(p)&&p.last_seen.is_some_and(|seen|seen.elapsed()<Duration::from_secs(12)));
        let associated_native=self.resume_intent.association().is_some_and(|a|a.origin==self.base&&a.service==self.service&&desktop.is_some_and(|d|d.account==a.account)&&pending.as_ref().is_some_and(|p|p.run==a.run&&p.config==u64::from(a.config_revision)&&p.actual_seed.map(|n|n.to_string()).as_deref()==Some(&a.actual_seed)&&p.last_seen.is_some_and(|seen|seen.elapsed()<Duration::from_secs(12))&&p.owner.is_some_and(|o|self.rejoin_transport.contact.as_ref().is_some_and(|c|c.pid==o.0&&c.context==o.1&&c.association==*a.local_world.as_bytes()))));
        let mut view=json!({"protocol_revision":protocol::REVISION,"setup":setup,"profile_selected":self.profile.is_some(),"server_origin":self.base,"service_id":self.service,"service_epoch":self.epoch,"session_epoch":desktop.map(|d|&d.epoch),"device_id":self.device.to_string(),"desktop_id":desktop.map(|d|&d.id),"account_id":desktop.map(|d|&d.account),"principal":desktop.map(|d|&d.principal),"signed_in":desktop.is_some(),"auth_state":self.auth_state,"login_error":self.login_error,"identity_mismatch":self.identity_mismatch.as_ref().map(|m|&m.recovery),"auth_pending":self.auth.is_some()||self.guest_retry.is_some(),"activation_code":null,"activation_attempt":null,"website":self.base,"mock":self.info.as_ref().and_then(|i|i.mock_provider).unwrap_or(false),"rooms":self.rooms,"room":self.room,"inspections":self.inspections,"chat_messages":self.chat,"room_cursor":self.room_cursor,"next_room_id":self.next_room,"preparation":self.preparation,"native_armed":pending.as_ref().is_some_and(|p|p.startup_ready),"seed":pending.as_ref().map(|p|if p.world_policy==2{p.actual_seed.map(|seed|Value::String(seed.to_string())).unwrap_or(Value::Null)}else{json!(p.seed)}),"inspection_status":json!({"stage":self.native_stage,"failure":self.native_failure,"local_observation_revision":pending.as_ref().and_then(|p|p.observation.as_ref().map(|r|r.sequence)).or(self.last_native_observation),"current":current,"service_observation_revision":if current{self.game.as_ref().map(|game|game.reported).unwrap_or(0)}else{0},"historical_service_observation_revision":self.last_service_observation,"child_attached":self.game.is_some(),"native_contact_age_ms":pending.as_ref().and_then(|p|p.last_seen.map(|seen|seen.elapsed().as_millis() as u64))}),"handoff":if self.native_failure.is_some()&&pending.is_none(){self.native_failure.as_deref().unwrap()}else if self.native_revoked&&pending.is_some(){"Run authority revoked; world held. Use the in-game Leave NT2 option to continue offline"}else if pending.as_ref().is_some_and(|p|p.release){"Matching host authorization; Lua revalidates before play"}else if pending.as_ref().is_some_and(|p|!p.startup_ready){"Preparing and sealing this world identity; not ready for startup"}else if pending.as_ref().is_some_and(|p|p.owner.is_none()){if pending.as_ref().is_some_and(|p|p.world_policy==2){"Individual world armed: choose New Game; Continue keeps the saved seed"}else{"Shared seed armed: choose normal New Game; Continue keeps the saved seed"}}else if pending.as_ref().is_some_and(|p|!p.accepted){"Game handshake consumed this grant; waiting for seed confirmation"}else if pending.as_ref().is_some_and(|p|!p.loaded){"Seed confirmed; waiting for protected player loading"}else if pending.is_some(){"Protected candidate world; waiting for inspected host Start"}else if self.preparation.is_some(){"Desktop grant prepared; matched candidate mod required before New Game"}else if self.automatic_setup_enabled(){if self.game_path.is_none(){"Choose a supported Noita folder; lobby setup continues automatically"}else{"Select and join a lobby, then run Noita and choose New Game"}}else if self.game_path.is_none(){"Verify the Noita installation before Load game"}else{"No native preparation; join and ready, then Load game before New Game"},"game_path":self.game_path.as_ref().map(|p|p.to_string_lossy()),"installation_state":installation_state,"installation_diagnostics":self.installation_diagnostics,"native_connection":self.native_diagnostics.lock().unwrap().view(),"release_enabled":true,"status_error":self.status_error,"limits":"Candidate · bounded captured reports · explicit inspected Start","suggested_server":self.recent_servers.first().map(|s|s.origin.as_str()).unwrap_or(""),"history":self.recent_servers.iter().map(|s|&s.origin).collect::<Vec<_>>(),"recent_servers":self.recent_servers});
        view["compatibility_issue"]=json!(self.compatibility_issue());
        if cfg!(feature="beta-portable"){view["limits"]=json!("Windows beta · exact app/mod/service compatibility · manual updates");}
        view["beta"]=json!({"enabled":cfg!(feature="beta-portable"),"version":crate::installation::beta::VERSION,"name":crate::installation::beta::NAME});
        #[cfg(test)]if self.lc1{view["lc1_lifecycle"]=self.lc1_fixture_view_with(pending.as_ref());}
        view["startup_staging"]=json!(pending.as_ref().is_some_and(|p|!p.startup_ready));
        view["rejoin"]=self.resume_intent.view();
        view["original_outcome"]=self.last_original_outcome.clone().unwrap_or(Value::Null);
        #[cfg(feature="g3-rejoin")]{
            let c=self.lifecycle.lock().unwrap();let a=&c.authority;
            let recent=pending.as_ref().filter(|p|(current||associated_native)&&p.last_seen.is_some_and(|at|at.elapsed()<Duration::from_secs(12)));
            let protection=recent.and_then(|p|p.observation.as_ref()).filter(|r|r.context==recent.unwrap().owner.map(|o|o.1).unwrap_or_default()).map(|r|r.observation.protected);
            let live=a.authority.live(Instant::now())&&recent.is_some_and(|p|p.release&&p.authority_until>Instant::now());
            let state=if a.terminal{"Ended"}else if a.entered&&live&&protection==Some(false){"Playing"}else if (a.entered||associated_native)&&!live{"Reconnecting"}else if a.offered&&live{"Ready"}else if recent.is_some(){"Checking seed and mods"}else{"Waiting for Noita"};
            view["connection"]=json!({"state":state,"reason":self.status_error.as_ref().or(a.reason.as_ref()),"protection_confirmed":protection,"authority_remaining_ms":if live{a.authority.remaining(Instant::now()).as_millis() as u64}else{0}});
        }
        // Retained intent is not evidence about the currently loaded world.
        let recovery_current=(current||associated_native) && self.resume_intent.association().is_some_and(|a|
            a.origin==self.base && a.service==self.service
            && desktop.is_some_and(|d|d.account==a.account)
            && pending.as_ref().is_some_and(|p|p.run==a.run && p.config==u64::from(a.config_revision)
                && p.owner.is_some_and(|(pid,token)|hex(&token)==a.world_token
                    ||self.rejoin_transport.contact.as_ref().is_some_and(|c|c.pid==pid&&c.context==token&&c.association==*a.local_world.as_bytes()))
                && p.actual_seed.map(|v|v.to_string()).as_deref()==Some(&a.actual_seed)
                && p.loaded && p.accepted && p.observation.as_ref().is_some_and(|r|
                    r.context==p.owner.unwrap().1 && r.observation.complete && r.observation.loaded
                    && p.matches_seed(r.observation.actual_seed.as_deref()))));
        view["rejoin"]["current_world"]=json!(recovery_current);
        view["rejoin"]["held"]=json!(recovery_current && pending.as_ref().and_then(|p|p.observation.as_ref()).is_some_and(|r|r.observation.protected));
        view["rejoin"]["fresh_setup_active"]=json!(self.fresh_setup_owns_world());
        view["authentication"]=self.auth1_view();
        view["ux1"]=self.ux1_view();
        if self.lc1{
            #[cfg(test)]{view["fixture_lifecycle_credentials"]=self.lc1_credential_view();}
            let state=self.world.coordinator.view();view["connection"]=state.clone();view["status_error"]=json!(self.world.coordinator.reason());view["handoff"]=json!(self.world.coordinator.reason().unwrap_or_else(||match self.world.coordinator.stage(){nt2_desktop_g0::world_lifecycle::Stage::Ready=>"Ready; protected until inspected host Start",nt2_desktop_g0::world_lifecycle::Stage::Playing=>"Playing",nt2_desktop_g0::world_lifecycle::Stage::Reconnecting=>"Reconnecting this saved world",nt2_desktop_g0::world_lifecycle::Stage::Choice=>"Rejoin this saved world or play offline",_=>"Waiting for Noita"}));view["rejoin"]=json!({"phase":self.world.coordinator.stage(),"current_world":self.world.record.as_ref().map(|r|r.world),"held":pending.as_ref().is_some_and(|p|!p.release),"intent_retained":self.world.record.is_some(),"can_rejoin":self.world.coordinator.stage()==nt2_desktop_g0::world_lifecycle::Stage::Choice&&self.world.record.as_ref().is_some_and(|r|r.intent==nt2_desktop_g0::world_local::Intent::Rejoin),"can_offline":self.world.record.as_ref().is_some_and(|r|r.intent==nt2_desktop_g0::world_local::Intent::Rejoin),"terminal_reason":self.world.coordinator.reason(),"choice_required":self.world.coordinator.stage()==nt2_desktop_g0::world_lifecycle::Stage::Choice});view["setup"]["stage"]=json!(match self.world.coordinator.stage(){nt2_desktop_g0::world_lifecycle::Stage::WaitingForNoita=>if self.room.is_some(){"waiting_noita"}else{"choose_lobby"},nt2_desktop_g0::world_lifecycle::Stage::Loading=>"loading_world",nt2_desktop_g0::world_lifecycle::Stage::Checking=>"checking_report",nt2_desktop_g0::world_lifecycle::Stage::Ready=>"ready",nt2_desktop_g0::world_lifecycle::Stage::Playing=>"playing",nt2_desktop_g0::world_lifecycle::Stage::Reconnecting=>"Reconnecting this saved world",nt2_desktop_g0::world_lifecycle::Stage::Choice=>"Rejoin this saved world or play offline",nt2_desktop_g0::world_lifecycle::Stage::Ended=>"Ended",nt2_desktop_g0::world_lifecycle::Stage::Offline=>"Offline"});view["setup"]["error"]=state["reason"].clone();view["setup"]["intent_retained"]=json!(self.world.record.is_some()||self.world.wants_prepare);view["auth_error"]=json!(self.auth_owner.lock().unwrap().error);view["auth_renewals"]=json!(self.auth_owner.lock().unwrap().renewals);view["auth_diagnostic"]=self.auth_owner.lock().unwrap().view(self.desktop.as_ref());view["native_armed"]=json!(self.world.bridge.as_ref().is_some_and(|b|b.pending.lock().unwrap().as_ref().is_some_and(|p|p.startup_ready)));
            let phase=self.world.coordinator.stage();let presence=self.presence_owner.lock().unwrap().view();let connected=presence["online"]==true||matches!(phase,nt2_desktop_g0::world_lifecycle::Stage::Ready|nt2_desktop_g0::world_lifecycle::Stage::Playing);
            view["connection"]["desktop_presence"]=presence;view["connection"]["app_connected"]=json!(connected);view["rejoin"]["app_connected"]=json!(connected);view["rejoin"]["waiting_for_game"]=state["waiting_for_game"].clone();view["rejoin"]["last_confirmed_state"]=state["last_confirmed_state"].clone();view["rejoin"]["awaiting_host_start"]=json!(phase==nt2_desktop_g0::world_lifecycle::Stage::Ready&&self.world.waiting_for_start);view["rejoin"]["host_started"]=json!(self.world.host_started);view["rejoin"]["reason"]=state["reason"].clone();view["rejoin"]["terminal_reason"]=if phase==nt2_desktop_g0::world_lifecycle::Stage::Ended{state["reason"].clone()}else{Value::Null};
            view["rejoin"]["shared_rules"]=pending.as_ref().and_then(|p|{
                let(pid,context)=p.owner?;let epoch=p.native_epoch?;
                if !self.world.host_started||p.observation.as_ref().is_none_or(|r|!r.observation.protected){return None}
                self.social_stream.as_ref()?.lock().ok()?.c1_metadata_diagnostics(pid,context,epoch)
            }).unwrap_or(Value::Null);
            if self.world.coordinator.reason().is_none(){if state["waiting_for_game"]==true{view["handoff"]=json!(if connected{"Connected; waiting for game updates"}else{"Checking desktop connection; shared play held"});view["setup"]["stage"]=json!("waiting_game");}else if phase==nt2_desktop_g0::world_lifecycle::Stage::Checking{view["handoff"]=json!("Checking current world and host permission");view["setup"]["stage"]=json!("checking_permission");}else if phase==nt2_desktop_g0::world_lifecycle::Stage::Ready&&!self.world.waiting_for_start{view["handoff"]=json!("Host Start confirmed; waiting for game entry");view["setup"]["stage"]=json!("entering_play");}}
            if self.world.coordinator.reason().is_none()&&view["rejoin"]["shared_rules"]["bootstrap"]==true{
                view["handoff"]=json!("Host Start confirmed; initializing shared player rules and waiting for game confirmation");view["setup"]["stage"]=json!("initializing_shared_rules");
            }
        }
        view["debug_diagnostics"]=self.debug.as_ref().map(|d|d.status()).unwrap_or(json!({"available":false}));
        view
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
#[cfg(test)]
#[path = "next_test.rs"]
mod tests;
#[path="next_health.rs"] mod health;
#[path="next_auth_owner.rs"] mod auth_owner;
#[path="next_presence.rs"] mod presence;
#[path="next_compatibility.rs"] mod compatibility;
#[path="next_world.rs"] mod world;
#[path="next_social_worker.rs"] mod social_worker;
#[cfg(test)]#[path="next_lc1_composition_test.rs"]mod lc1_composition;
#[path="next_waiting.rs"] mod waiting;
#[path="next_startup_stage.rs"] mod startup_stage;
#[path="next_setup.rs"] mod setup;
#[path="next_start.rs"] pub(crate) mod start;
#[path="next_entry.rs"] mod entry;
#[path="next_entry_transport.rs"] mod entry_transport;
#[path="next_rejoin.rs"] mod rejoin;
#[path="next_lifecycle.rs"] mod lifecycle;
#[path="next_admission.rs"] mod admission;
impl<S: CredentialStore> Core<S> {
    pub async fn final_start(
        &mut self,
        selected: Vec<String>,
        excluded: Vec<String>,
    ) -> Result<(), String> {
        if selected.is_empty() || selected.len() > 32 || excluded.len() > 32 {
            return Err("Select a bounded complete roster".into());
        }
        let mut snapshot = self.request(false);
        snapshot.room_get = Some(wire::Empty {});
        let response = self.rpc(&snapshot, false, false).await?;
        self.absorb(&response);
        let participants = selected
            .into_iter()
            .map(|id| {
                let r = self
                    .inspections
                    .iter()
                    .find(|r| r.desktop_id.as_deref() == Some(&id))
                    .ok_or("Inspect each selected loaded player before Start")?;
                Ok(wire::InspectedParticipant {
                    desktop_id: Some(id),
                    attachment_id: r.attachment_id.clone(),
                    world_token: r.world_token.clone(),
                    observation_revision: r.observation_revision,
                    content_digest: r.content_digest.clone(),
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        let mut request = self.request(false);
        request.final_start = Some(wire::FinalStart {
            participants: Some(participants),
            excluded_desktop_ids: Some(excluded),
        });
        let response = self.rpc(&request, false, false).await?;
        self.absorb(&response);
        Ok(())
    }
    pub async fn end(&mut self) -> Result<(), String> {
        if self.lc1{self.lc1_retire_preparation();if let Some(p)=self.pending.lock().unwrap().as_mut(){p.release=false;p.authority_until=Instant::now();}self.g3_pause();let mut request=self.request(false);request.end_run=Some(wire::Empty{});let response=self.rpc(&request,false,false).await?;self.room=response.room;self.lc1_terminal("Host ended the run");return Ok(())}
        self.resume_intent.terminal();
        self.stop_setup("Host ended the run; review and Join for a fresh New Game");
        self.clear_game();
        let mut request = self.request(false);
        request.end_run = Some(wire::Empty {});
        let response = self.rpc(&request, false, false).await?;
        self.absorb(&response);
        Ok(())
    }
    async fn game_tick(&mut self) -> Result<(), String> where S:'static {
        let result=self.game_tick_inner().await;
        if let Err(error)=&result{
            // Fixed explanations only: never copy arbitrary response text or
            // credentials/context tokens into durable native diagnostics.
            let reason=match error.as_str(){
                "INCOMPATIBLE_GAME"=>"Game attachment refused: mod/bridge metadata does not match the selected service. Keep the world held and verify the matching A1 app, test mod and server.",
                "STALE_REVISION"=>"Game attachment rejected because the room revision changed. The world stays held while a fresh room snapshot is checked.",
                "Native context lost; prepare a new game"=>"Native game contact expired. Cancel and prepare a fresh world; this world cannot be reattached automatically.",
                "Game attachment retry expired; cancel and prepare again"=>"Game attachment retry expired before service acceptance. Cancel and prepare a fresh world.",
                "Observation retry expired"=>"Service inspection submission expired. A local inspection receipt is not service acceptance. Cancel and prepare a fresh world.",
                "Game renewal retry expired"=>"Game session renewal expired. Keep this world held; cancel and prepare a fresh world.",
                "Social runtime is not verified for this preparation; world remains held"=>"Social test runtime is not verified for this preparation. Keep the world held; cancel preparation, verify the matching test mod and prepare a fresh world.",
                "Native preparation expired"=>"Native preparation expired before protected loading. Cancel and prepare a fresh world.",
                "Native preparation authority revoked; world remains held"=>"Preparation authority revoked. Keep this world held; cancel and prepare a fresh world.",
                _=>"Game preparation/inspection request failed. Keep the world held; record the Connection error and the native/service inspection stages below.",
            };
            if error=="INCOMPATIBLE_GAME"{self.compatibility_runtime_failure(error,true);}
            self.native_failure=Some(reason.into());
        }else if self.game.as_ref().is_some_and(|game|game.reported>0){
            self.native_failure=None;self.compatibility_notice.clear("observed_runtime");
        }
        result
    }
    async fn game_tick_inner(&mut self) -> Result<(), String> where S:'static {
        if self.rejoin_transport.contact.is_some(){return self.rejoin_playing_tick().await;} 
        if self.native_revoked {
            return Ok(());
        }
        let Some(local) = self.pending.lock().unwrap().clone() else {
            self.game = None;
            return Ok(());
        };
        self.last_native_observation=local.observation.as_ref().map(|report|report.sequence);
        self.native_stage="awaiting-native";
        if !self.native_authority_valid(&local) {
            self.revoke_native();
            return Err("Native preparation authority revoked; world remains held".into());
        }
        if !local.loaded && Instant::now() >= local.expires {
            self.clear_game();
            return Err("Native preparation expired".into());
        }
        let room = self.room.as_ref().ok_or("Room disappeared")?;
        if room.run_id.as_deref() != Some(&local.run)
            || room.run_epoch.as_deref() != Some(&local.epoch)
            || room.config_revision.map(u64::from) != Some(local.config)
        {
            self.clear_game();
            return Err("Native run context changed".into());
        }
        if local.owner.is_none() {
            let shared = self.pending.clone();
            let mut pending = shared.lock().unwrap();
            if let Some(p) = pending.as_mut() {
                if self.native_authority_valid(p) {
                    p.authority_until = Instant::now() + Duration::from_secs(12);
                } else {
                    p.release = false;
                    p.authority_until = Instant::now();
                }
            }
            return Ok(());
        }
        if local
            .last_seen
            .is_none_or(|seen| seen.elapsed() > Duration::from_secs(12))
        {
            self.clear_game();
            return Err("Native context lost; prepare a new game".into());
        }
        let owner = local.owner.unwrap();
        if self.game.as_ref().is_some_and(|g| g.owner != owner) {
            self.clear_game();
            return Err("Replacement native world refused".into());
        }
        if self.game.is_none() {
            self.native_stage="attaching-game";
            if self.attach_cycle.as_ref().is_some_and(|(_,until)|Instant::now()>=*until){
                if self.automatic_setup_enabled(){self.setup_restart_required("Original attachment reply recovery expired; world held. Leave it and choose a fresh New Game");return Err("Original attachment reply recovery expired; explicit restart required".into());}
                self.clear_game();
                return Err("Game attachment retry expired; cancel and prepare again".into());
            }
            if self.social_requested&&!self.social_enabled{
                return Err("Social runtime is not verified for this preparation; world remains held".into());
            }
            if self.attach_retry.is_none() {
                let mut request = self.request(false);
                request.room_get = Some(wire::Empty {});
                let response = self.rpc(&request, false, false).await?;
                self.absorb(&response);
                if self.preparation.is_none() || self.native_revoked {
                    return Err("Native preparation no longer current".into());
                }
            }
            let retry = if let Some(retry) = self.attach_retry.take() {
                if self.attach_cycle.is_none(){
                    self.attach_cycle=Some((required(retry.request.game_attach.as_ref().and_then(|attach|attach.game_instance_id.clone()))?,retry.until));
                }
                retry
            } else {
                let (instance,until)=self.attach_cycle.get_or_insert_with(||(uuid::Uuid::new_v4().to_string(),Instant::now()+Duration::from_secs(30))).clone();
                let mut request = self.request(false);
                request.game_attach = Some(wire::GameAttach {
                    game_instance_id: Some(instance),
                    game_version: Some("Noita-Jan25-2025".into()),
                    mod_version: Some(if self.social_requested&&self.social_enabled{nt2_desktop_g0::g3_social_installation::MOD_VERSION}else if self.g3.is_some(){"nt2-g3-dev-1"}else{"nt2-g2-inspect-1"}.into()),
                    bridge_version: Some(if self.social_requested&&self.social_enabled{nt2_desktop_g0::g3_social_installation::BRIDGE_VERSION}else if self.g3.is_some(){"g3-transport-1"}else{"g2-inspect-1"}.into()),
                    capabilities: Some(self.local_game_capabilities()),
                    preparation_id: self
                        .preparation
                        .as_ref()
                        .and_then(|p| p.preparation_id.clone()),
                });
                Retry {
                    request,
                    until,
                }
            };
            if Instant::now() >= retry.until {
                if self.automatic_setup_enabled(){self.attach_retry=Some(retry);self.setup_restart_required("Original attachment reply recovery expired; explicit restart required");return Err("Original attachment reply recovery expired; explicit restart required".into());}
                self.clear_game();
                return Err("Game attachment retry expired; cancel and prepare again".into());
            }
            if local.startup_bundle.is_some()&&(!local.loaded||!local.accepted||local.observation.as_ref().is_none_or(|r|!r.observation.loaded||!r.observation.complete||!r.observation.protected)){self.attach_retry=Some(retry);return Ok(())}
            match self.seal_startup_attach(&local,&retry).await{Ok(true)=>{},Ok(false)=>{self.attach_retry=Some(retry);return Ok(())},Err(e)=>{self.attach_retry=Some(retry);return Err(e)}}
            if !self.replacement_lineage.block("canonical game attachment"){self.attach_retry=Some(retry);return Ok(())}
            match self.rpc(&retry.request, false, false).await {
                Ok(mut response) => {
                    let session = Session::read(
                        response
                            .credentials
                            .take()
                            .ok_or("Missing child game session")?,
                        &self.device.to_string(),
                        "game",
                    )?;
                    if self
                        .desktop
                        .as_ref()
                        .is_none_or(|d| session.account != d.account)
                        || session.attachment.is_empty()
                    {
                        return Err("Game principal/attachment mismatch".into());
                    }
                    // Preserve the exact successful attach identity before its
                    // Retry is dropped. Waiting enrollment will bind this real
                    // operation, not reconstruct one from a saved Lua marker.
                    let attach=retry.request.game_attach.as_ref().ok_or("Missing original game attach")?;
                    let evidence=nt2_desktop_g0::startup_local::AttachEvidence{
                        request_id:required(retry.request.request_id.clone())?,
                        fingerprint:nt2_desktop_g0::protocol_connection::canonical_fingerprint(&serde_json::to_vec(&retry.request).map_err(|_|"Original attach encoding")?)?,
                        session_id:required(self.desktop.as_ref().map(|d|d.id.clone()))?,session_epoch:required(retry.request.session_epoch.clone())?,service_epoch:required(retry.request.service_epoch.clone())?,
                        preparation:required(attach.preparation_id.clone())?,attachment:session.attachment.clone(),instance:required(attach.game_instance_id.clone())?,
                    };
                    if let Some(p)=self.pending.lock().unwrap().as_mut().filter(|p|p.owner==Some(owner)&&p.startup_bundle.is_some()){p.startup_attach=Some(evidence);}
                    self.game = Some(Game {
                        session,
                        instance: required(
                            retry
                                .request
                                .game_attach
                                .as_ref()
                                .unwrap()
                                .game_instance_id
                                .clone(),
                        )?,
                        owner,
                        reported: 0,
                        report_retry: None,
                        renew_retry: None,
                        permit: None,
                        entered: false,
                        offered: false,
                    });
                    self.attach_cycle=None;
                }
                Err(error) => {
                    // The authenticated canonical stale-revision reply proves
                    // this attach was rejected. A fresh snapshot/new request is
                    // safe; ambiguous transport failures still retry exact bytes.
                    if error!="STALE_REVISION"{self.attach_retry = Some(retry);}
                    return Err(error);
                }
            }
        }
        #[cfg(feature="g3-rejoin")]
        if self.health_enabled{self.publish_health();return Ok(());}
        #[cfg(all(feature="g3-rejoin",not(test)))]{return Err("Independent connection health worker is not running".into());}
        #[cfg(any(not(feature="g3-rejoin"),test))]{
        self.native_stage="renewing-game";
        self.renew_game().await?;
        if let Some(report) = &local.observation {
            // Consumption is local evidence, never inferred from a server timestamp on a fresh Core.
            if let Some(game) = self.game.as_mut() {
                if game.offered
                    && !game.entered
                    && !report.observation.protected
                    && report.observation.complete
                    && report.observation.loaded
                    && local.matches_seed(report.observation.actual_seed.as_deref())
                    && game.permit.as_ref().is_some_and(|r| {
                        if r.content_digest.as_deref() != Some(&hex(&report.digest))
                            || r.observation_revision
                                .is_none_or(|rev| u64::from(rev) < local.invalidated_observation)
                        {
                            return false;
                        }
                        timestamp(r.release_expires_at.as_deref().unwrap_or("")).is_ok_and(
                            |deadline| {
                                now()
                                    .saturating_sub(report.observed_at.elapsed().as_millis() as u64)
                                    < deadline
                            },
                        )
                    })
                {
                    game.entered = true;
                }
            }
            self.note_native_entry(&local);
            self.native_stage="submitting-inspection";
            self.submit_report(report).await?;
        }
        #[cfg(feature="g3-rejoin")]
        if self.game.as_ref().is_some_and(|g|g.entered){
            self.capture_selected_association().await?;
            if self.rejoin_transport.contact.is_some(){return self.rejoin_playing_tick().await;}
        }
        let game = self.game.as_ref().unwrap();
        // The asynchronous shared broker may bind the world before its first
        // inspection arrives. The service requires that observation to bind a
        // pulse's world token; an early pulse cannot prove native readiness.
        if game.reported==0 {
            self.native_stage="attached-awaiting-inspection";
            return Ok(());
        }
        let mut request = self.request(true);
        request.game_pulse = Some(wire::GamePulse {
            attachment_id: Some(game.session.attachment.clone()),
            game_instance_id: Some(game.instance.clone()),
            world_token: Some(hex(&owner.1)),
        });
        self.rpc(&request, true, false).await?;
        let mut request = self.request(true);
        request.room_get = Some(wire::Empty {});
        let response = self.rpc(&request, true, false).await?;
        self.apply_release(response.release, &local)?;
        self.native_stage=if self.last_service_observation>0{"service-inspection-accepted"}else{"attached-awaiting-inspection"};
        Ok(())
        }
    }
    #[cfg(any(not(feature="g3-rejoin"),test))]
    async fn renew_game(&mut self) -> Result<(), String> {
        let game = self.game.as_ref().ok_or("No native game")?;
        if !self.session_expiring(&game.session,60000) && game.renew_retry.is_none() {
            return Ok(());
        }
        let retry = if let Some(retry) = self.game.as_mut().unwrap().renew_retry.take() {
            retry
        } else {
            let game = self.game.as_ref().unwrap();
            let mut request = self.request(false);
            request.game_renew = Some(wire::GameRenew {
                attachment_id: Some(game.session.attachment.clone()),
                game_instance_id: Some(game.instance.clone()),
                world_token: Some(self.waiting_world().map(String::from).or_else(||self.rejoin_intended_world()).unwrap_or_else(||hex(&game.owner.1))),
                game_session_id: Some(game.session.id.clone()),
                game_session_epoch: Some(game.session.epoch.clone()),
            });
            Retry {
                request,
                until: Instant::now() + Duration::from_secs(30),
            }
        };
        if Instant::now() >= retry.until {
            self.clear_game();
            return Err("Game renewal retry expired".into());
        }
        match self.rpc(&retry.request, false, false).await {
            Ok(mut response) => {
                let session = Session::read(
                    response
                        .credentials
                        .take()
                        .ok_or("Missing renewed game session")?,
                    &self.device.to_string(),
                    "game",
                )?;
                let game = self.game.as_mut().unwrap();
                if session.id != game.session.id
                    || session.epoch != game.session.epoch
                    || session.attachment != game.session.attachment
                    || session.account != game.session.account
                {
                    self.revoke_native();
                    return Err("Renewed child context changed".into());
                }
                self.social_credentials.renew(&session.id,&session.epoch,&session.account,&session.attachment,game.owner.1,zeroize::Zeroizing::new(session.access.as_str().to_string()))?;
                game.session = session;
                Ok(())
            }
            Err(error) => {
                if let Some(game) = self.game.as_mut() {
                    game.renew_retry = Some(retry);
                }
                Err(error)
            }
        }
    }
    #[cfg(any(not(feature="g3-rejoin"),test))]
    async fn submit_report(&mut self, latest: &native_report::Latest) -> Result<(), String> {
        let game = self.game.as_ref().unwrap();
        if latest.context != game.owner.1 {
            return Err("Wrong observed world".into());
        }
        if latest.sequence <= game.reported && game.report_retry.is_none() {
            return Ok(());
        }
        let retry = if let Some(retry) = self.game.as_mut().unwrap().report_retry.take() {
            retry
        } else {
            let game = self.game.as_ref().unwrap();
            let observation = &latest.observation;
            let mut request = self.request(true);
            let mut report = wire::GameObservation {
                attachment_id: Some(game.session.attachment.clone()),
                game_instance_id: Some(game.instance.clone()),
                world_token: Some(hex(&latest.context)),
                observation_revision: Some(
                    latest
                        .sequence
                        .try_into()
                        .map_err(|_| "Observation revision exhausted")?,
                ),
                actual_seed: observation.actual_seed.clone(),
                active_mods: Some(observation.mods.clone()),
                inventory_complete: Some(observation.complete),
                beta: Some(
                    match observation.beta {
                        Some(true) => "true",
                        Some(false) => "false",
                        None => "unknown",
                    }
                    .into(),
                ),
                protected: Some(observation.protected),
                loaded: Some(observation.loaded),
                seed_status: Some(
                    if observation.actual_seed.is_some() {
                        "known"
                    } else {
                        "unknown"
                    }
                    .into(),
                ),
                collector_failure: observation.error.clone(),
                mod_version: Some(observation.mod_version.clone()),
                bridge_version: Some(observation.bridge_version.clone()),
                compatibility_provenance: Some("game-thread-client-observation".into()),
            };
            if serde_json::to_vec(&report)
                .map_err(|_| "Report encoding failed")?
                .len()
                > native_report::REPORT_BYTES
            {
                report.active_mods = Some(vec![]);
                report.inventory_complete = Some(false);
                report.collector_failure = Some("inventory_limit".into());
            }
            request.game_observation = Some(report);
            Retry {
                request,
                until: Instant::now() + Duration::from_secs(30),
            }
        };
        if Instant::now() >= retry.until {
            if self.automatic_setup_enabled(){self.game.as_mut().unwrap().report_retry=Some(retry);self.setup_restart_required("Original inspection reply recovery expired; world held, explicit restart required");return Err("Original inspection reply recovery expired; explicit restart required".into());}
            self.clear_game();return Err("Observation retry expired".into());
        }
        match self.rpc(&retry.request, true, false).await {
            Ok(_) => {
                self.game.as_mut().unwrap().reported = u64::from(
                    retry
                        .request
                        .game_observation
                        .as_ref()
                        .unwrap()
                        .observation_revision
                        .unwrap(),
                );
                self.last_service_observation=self.game.as_ref().unwrap().reported;
                Ok(())
            }
            Err(error) => {
                if let Some(game) = self.game.as_mut() {
                    game.report_retry = Some(retry);
                }
                Err(error)
            }
        }
    }
    #[cfg(any(not(feature="g3-rejoin"),test))]
    fn apply_release(
        &mut self,
        release: Option<wire::GameRelease>,
        local: &handoff::Pending,
    ) -> Result<(), String> {
        // The native worker can change Pending during every network await.
        // Lock across CURRENT report validation and lease publication.
        let shared = self.pending.clone();
        let mut pending = shared.lock().unwrap();
        let p = pending
            .as_mut()
            .ok_or("Native preparation cancelled during release")?;
        if p.owner != local.owner
            || p.run != local.run
            || p.epoch != local.epoch
            || p.config != local.config
            || !self.native_authority_valid(p)
        {
            p.release = false;
            p.authority_until = Instant::now();
            return Err("Current native authority changed during release".into());
        }
        if let Some(release) = release {
            let Some(current)=p.observation.as_ref() else{
                p.release=false;return Err("No current actual observation".into());
            };
            // Submit a newer phase report first; preserve the frozen authorization.
            if current.sequence > self.game.as_ref().ok_or("No child game")?.reported {
                // Preserve only an existing, still-valid grant for this exact
                // eligible content/phase. No new grant or lease extension while
                // its latest report is awaiting service acknowledgment.
                let first_playing_phase=!self.game.as_ref().unwrap().entered
                    && !current.observation.protected && current.observation.complete
                    && current.observation.loaded
                    && p.matches_seed(current.observation.actual_seed.as_deref())
                    && local.observation.as_ref().is_some_and(|old|old.observation.protected&&old.digest==current.digest)
                    && release.observation_revision.is_some_and(|rev|u64::from(rev)>=p.invalidated_observation)
                    && self.validate_release(&release,local).is_ok();
                if !p.release || p.authority_until<=Instant::now()
                    || p.last_seen.is_none_or(|at|at.elapsed()>Duration::from_secs(12))
                    || self.game.as_ref().unwrap().permit.is_none()
                    || (!first_playing_phase&&self.validate_release(&release,p).is_err()){p.release=false;}
                return Ok(());
            }
            p.release = false;
            self.validate_release(&release, p)?;
            self.g3_release=Some(release.clone());
            let game = self.game.as_mut().unwrap();
            if game.permit.is_none() {
                game.permit = Some(release);
            } else {
                // Rejoin generation1 status renews transport presence while
                // the original inspected Start identity remains frozen.
                #[cfg(feature="g3-rejoin")]
                { game.permit.as_mut().unwrap().release_expires_at=release.release_expires_at; }
            }
            game.offered = true;
            p.release = true;
        }else{
            p.release=false;
        }
        p.authority_until = Instant::now() + Duration::from_secs(12);
        Ok(())
    }
    #[cfg(any(not(feature="g3-rejoin"),test))]
    fn validate_release(
        &self,
        r: &wire::GameRelease,
        local: &handoff::Pending,
    ) -> Result<(), String> {
        if self.rejoin_transport.contact.is_some(){return self.validate_rejoin_release(r,local);}
        let game = self.game.as_ref().ok_or("No child game")?;
        // The renewable transport lease is not the human Start deadline.
        // Retain the exact scoped preparation review bound until genuine local
        // consumption; a later lease cannot rearm an expired/changed grant.
        #[cfg(feature="g3-rejoin")]
        if !game.entered && !self.native_authority_valid(local) {
            return Err("Original inspected Start authority expired or changed".into());
        }
        let desktop = self.desktop.as_ref().ok_or("No desktop")?;
        let room = self.room.as_ref().ok_or("No room")?;
        let report = local
            .observation
            .as_ref()
            .ok_or("No actual game observation")?;
        let config=room.config.as_ref().ok_or("Missing release configuration")?;
        let policy=seed_policy::resolved(config)?;
        if policy.explicit&&(r.actual_seed.as_deref()!=report.observation.actual_seed.as_deref()||r.seed_policy.as_deref()!=Some(policy.name.as_str())||r.randomize_loot.unwrap_or(false)!=policy.loot){return Err("Host release seed/loot policy differs from this actual world".into());}
        if r.contract_revision.as_deref() != Some(protocol::REVISION)
            || r.service_id.as_deref() != Some(&self.service)
            || r.service_epoch.as_deref() != Some(&self.epoch)
            || r.room_id != room.room_id
            || r.run_id.as_deref() != Some(&local.run)
            || r.run_epoch.as_deref() != Some(&local.epoch)
            || r.config_revision.map(u64::from) != Some(local.config)
            || r.desktop_id.as_deref() != Some(&desktop.id)
            || r.desktop_session_epoch.as_deref() != Some(&desktop.epoch)
            || r.game_session_id.as_deref() != Some(&game.session.id)
            || r.game_session_epoch.as_deref() != Some(&game.session.epoch)
            || r.attachment_id.as_deref() != Some(&game.session.attachment)
            || r.game_instance_id.as_deref() != Some(&game.instance)
            || r.world_token.as_deref() != Some(&hex(&game.owner.1))
            || r.content_digest.as_deref() != Some(&hex(&report.digest))
            || r.observation_revision
                .is_none_or(|rev| rev == 0 || u64::from(rev) > game.reported)
            || r.observation_revision
                .is_none_or(|rev| u64::from(rev) < local.invalidated_observation)
            || !report.observation.complete
            || !report.observation.loaded
            || !local.matches_seed(report.observation.actual_seed.as_deref())
            || !local.accepted
            || !local.loaded
        {
            return Err("Host release does not match the actual protected world".into());
        }
        let deadline = timestamp(
            r.release_expires_at
                .as_deref()
                .ok_or("Missing first-release deadline")?,
        )?;
        let authorized = timestamp(
            r.authorized_at
                .as_deref()
                .ok_or("Missing authorization time")?,
        )?;
        if timestamp(
            r.captured_at
                .as_deref()
                .ok_or("Missing frozen observation time")?,
        )? > authorized
            || authorized >= deadline
        {
            return Err("Invalid authorization lifetime".into());
        }
        if r.authorization_id
            .as_ref()
            .is_none_or(|s| uuid::Uuid::parse_str(s).is_err())
            || r.start_request_id
                .as_ref()
                .is_none_or(|s| uuid::Uuid::parse_str(s).is_err())
            || r.authorization_id == r.start_request_id
        {
            return Err("Invalid host authorization identity".into());
        }
        if let Some(anchor) = &game.permit {
            let mut current = r.clone();
            current.phase = anchor.phase.clone();
            current.entered_playing_at = anchor.entered_playing_at.clone();
            // Only the canonical finite transport expiry may refresh. Every
            // other identity/report/content field still matches the anchor.
            #[cfg(feature="g3-rejoin")]
            { current.release_expires_at=anchor.release_expires_at.clone(); }
            if serde_json::to_value(current).ok() != serde_json::to_value(anchor).ok() {
                return Err("Frozen authorization changed".into());
            }
        }
        match r.phase.as_deref() {
            Some("authorized")
                if !game.entered && now() < deadline && report.observation.protected =>
            {
                Ok(())
            }
            Some("playing" | "restoring")
                if game.entered
                    && (r.phase.as_deref() == Some("restoring"))
                        == report.observation.protected
                    && r.entered_playing_at.as_deref().is_some_and(|value| {
                        timestamp(value).is_ok_and(|time| time >= authorized && time < deadline)
                    }) =>
            {
                Ok(())
            }
            _ => Err("First-entry deadline or local consumption evidence refused".into()),
        }
    }
}

#[path="next_seed_policy.rs"] mod seed_policy;

#[path="next_auth1.rs"] mod auth1;
#[path="next_ux1.rs"] pub(crate) mod ux1;

#[path="next_race_lobby.rs"]mod race_lobby;
