//! Separately built candidate entry. Never selects the historical Core or installed pipe.
use crate::{handoff, next_client::{Core, IdentityRecovery}, secrets::WindowsCredentialStore};
use serde::Deserialize;
use serde_json::Value;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
};
use tauri::Manager;
use crate::installation::beta;
#[cfg(test)]#[path="next_beta_setup_actual_test.rs"]mod beta_actual_tests;
#[path="next_scheduler.rs"]pub(crate) mod scheduler;
struct App {
    core: scheduler::Handle<WindowsCredentialStore>,
    diagnostics:Arc<Mutex<crate::pipe::NativeDiagnostics>>,
    pending: Arc<Mutex<Option<handoff::Pending>>>,
    generation: Arc<AtomicU64>,
    slots: Arc<AtomicUsize>,
}
struct Permit(Arc<AtomicUsize>);
impl Drop for Permit {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::AcqRel);
    }
}
fn permit(state: &App, priority: bool) -> Result<Permit, String> {
    let limit = if priority { 18 } else { 16 };
    state
        .slots
        .try_update(Ordering::AcqRel, Ordering::Acquire, |v| {
            if v < limit { Some(v + 1) } else { None }
        })
        .map_err(|_| "Client command queue full")?;
    Ok(Permit(state.slots.clone()))
}
fn trusted(w: &tauri::WebviewWindow) -> Result<(), String> {
    if w.label() != "main"
        || !crate::g1_client::navigation(
            w.url().map_err(|_| "Unavailable command origin")?.as_str(),
        )
    {
        return Err("Command origin denied".into());
    }
    Ok(())
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Login {
    server: String,
    #[serde(default, rename = "service_id")]
    _diagnostic_service_id: String,
    remember: bool,
    provider: Option<String>,
    guest_name: Option<String>,
    recovery: Option<IdentityRecovery>,
    #[serde(default)]
    discovery_only: bool,
    #[serde(default)]
    fresh:bool,
    #[serde(default)]
    restore_only:bool,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Action {
    action: ActionKind,
    room: Option<String>,
    password: Option<String>,
    ready: Option<bool>,
    debug_diagnostics: Option<bool>,
    text: Option<String>,
    selected: Option<Vec<String>>,
    excluded: Option<Vec<String>>,
    game_path: Option<String>,
    allow_guests: Option<bool>,
    review_revision: Option<u32>,
    config_revision: Option<u32>,
    participants: Option<Vec<crate::protocol_g2_next::wire::InspectedParticipant>>,
    start_capture: Option<crate::next_client::start::Capture>,
    create:Option<crate::next_client::ux1::Create>,
    revision:Option<u32>,
    config:Option<crate::protocol_g2_next::wire::Config>,
    policy:Option<crate::protocol_g2_next::wire::RoomPolicy>,
    access:Option<crate::protocol_g2_next::wire::RoomAccess>,
    target:Option<crate::protocol_g2_next::wire::PlayerTarget>,
    moderation:Option<crate::protocol_g2_next::wire::RoomModerate>,
    ban_page:Option<crate::protocol_g2_next::wire::RoomBanList>,
    player_rules:Option<nt2_desktop_g0::protocol_c1::wire::RuleSet>,
    world_rules:Option<nt2_desktop_g0::protocol_c2::wire::RulesSet>,
    recovery_player:Option<nt2_desktop_g0::protocol_rejoin::wire::Participant>,
    origin:Option<String>,
    outcome_lane:Option<String>,
    outcome_review:Option<String>,
    outcome_identity:Option<String>,
}
#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum ActionKind {
    ReviewUnknownOutcome,AcknowledgeUnknownOutcome,ForgetSaved,
    RecoveryList,RecoveryOriginal,RecoveryPending,ReviewRecovery,RefreshLobby,
    CreateLobby,ConfigureLobby,SetLobbyPolicy,SetLobbyAccess,ModeratePlayer,PlayerDetails,RoomBans,CancelLobby,EndLobby,DeleteLobby,RetryLobbyAction,GetPlayerRules,SetPlayerRules,GetWorldRules,SetWorldRules,
    SetDebugDiagnostics,
    RejoinPrevious,
    ContinueOffline,
    ConfigureGame,
    CreateFixture,
    Join,
    PreviewLobby,
    LeaveLobby,
    Ready,
    ChatSend,
    NextRooms,
    FirstRooms,
    Start,
    LoadGame,
    End,
    Refresh,
    CancelSignIn,
    Cancel,
    OpenWebsite,
    OpenPasswordChange,
}
#[tauri::command]
async fn g1_login(
    w: tauri::WebviewWindow,
    state: tauri::State<'_, App>,
    input: Login,
) -> Result<Value, String> {
    trusted(&w)?;
    let _permit = permit(&state, false)?;
    let revision=state.generation.clone();let generation=revision.load(Ordering::Acquire);
    state.core.command(move|core|Box::pin(async move{
    if generation != revision.load(Ordering::Acquire) {
        return Err("Client action cancelled".into());
    }
    // Renderer service IDs are diagnostics only; Rust pins saved/discovered identity.
    if input.fresh{core.fresh_login(&input.server,input.remember,input.provider.as_deref().ok_or("Choose a sign-in method")?,input.recovery).await?;}else if input.restore_only{core.restore_session(&input.server,input.remember).await?;}else if input.discovery_only{core.discover_sign_in(&input.server,input.remember).await?;}else{
    core.login(&input.server, input.remember,
        input.provider.as_deref().unwrap_or("website"), input.guest_name.as_deref(), input.recovery).await?;}
    Ok(core.view())})).await
}
#[tauri::command]
async fn g1_status(w: tauri::WebviewWindow, state: tauri::State<'_, App>) -> Result<Value, String> {
    trusted(&w)?;
    let _permit = permit(&state, false)?;
    Ok(state.core.view())
}
#[path="folder_picker.rs"]mod folder_picker;
#[tauri::command]async fn g1_browse_game(w:tauri::WebviewWindow)->Result<Option<String>,String>{trusted(&w)?;tauri::async_runtime::spawn_blocking(folder_picker::choose).await.map_err(|_|"Folder picker unavailable".to_string())?}
#[tauri::command]async fn g1_beta_setup(w:tauri::WebviewWindow,path:String,token:Option<String>,state:tauri::State<'_,App>)->Result<Value,String>{
 trusted(&w)?;if !cfg!(feature="beta-portable"){return Err("Portable beta setup is not enabled in this build".into())}let _permit=permit(&state,true)?;let selected=path.clone();
 let result=tauri::async_runtime::spawn_blocking(move||if let Some(token)=token{beta::install(&path,&token)}else{beta::preview(&path)}).await.map_err(|_|"Mod setup worker failed")??;
 if result["installed"]==true{let configure=state.core.command(move|core|Box::pin(async move{core.configure_game(&selected).map(|_|core.view())})).await;return Ok(serde_json::json!({"result":result,"view":configure.as_ref().ok(),"verificationError":configure.err()}))}Ok(result)
}
#[tauri::command]async fn g1_beta_discover(w:tauri::WebviewWindow)->Result<Value,String>{trusted(&w)?;if !cfg!(feature="beta-portable"){return Ok(Value::Null)}tauri::async_runtime::spawn_blocking(||serde_json::json!({"version":beta::VERSION,"name":beta::NAME,"candidates":crate::installation::discover_candidates()})).await.map_err(|_|"Discovery unavailable".into())}
#[tauri::command]async fn g1_beta_export(w:tauri::WebviewWindow,state:tauri::State<'_,App>)->Result<Option<String>,String>{trusted(&w)?;let view=state.core.view();tauri::async_runtime::spawn_blocking(move||{let data=beta::export_report(&view,std::env::var_os("LOCALAPPDATA").map(std::path::PathBuf::from).as_deref());let Some(folder)=folder_picker::choose_title("Choose a folder for the NT2 diagnostic report")?else{return Ok(None)};let path=std::path::Path::new(&folder).join(format!("NT2-diagnostics-{}.json",uuid::Uuid::new_v4().simple()));std::fs::write(&path,serde_json::to_vec_pretty(&data).map_err(|_|"Diagnostic encoding failed")?).map_err(|_|"Cannot write diagnostic report")?;Ok(Some(path.to_string_lossy().into_owned()))}).await.map_err(|_|"Diagnostic export unavailable".to_string())?}
#[tauri::command]
async fn g3_original_status(w:tauri::WebviewWindow,state:tauri::State<'_,App>,input:OriginalStatus)->Result<Value,String>{
    trusted(&w)?;let _permit=permit(&state,false)?;
    state.core.command(move|core|Box::pin(async move{core.g3_original_status(&input.operation_id).await?;let mut view=core.view();view["g3"]=core.g3_view();Ok(view)})).await
}
#[derive(Deserialize)]
#[serde(rename_all="camelCase",deny_unknown_fields)]
struct OriginalStatus{operation_id:String}
#[tauri::command]
async fn g1_action(
    w: tauri::WebviewWindow,
    state: tauri::State<'_, App>,
    input: Action,
) -> Result<Value, String> {
    trusted(&w)?;
    let priority = matches!(input.action, ActionKind::Cancel|ActionKind::End);
    if priority {
        crate::pipe::retire_lc1_bridge(&state.diagnostics);
        state.generation.fetch_add(1, Ordering::AcqRel);
        *state.pending.lock().unwrap() = None;
    }
    let _permit = permit(&state, priority)?;
    state.core.action(input).await
}
// Shared typed app dispatch, entered only through the ordinary scheduler.
pub(super) async fn apply_action<S:crate::secrets::CredentialStore+'static>(core:&mut Core<S>,input:Action)->Result<Value,String>{
    if matches!(input.action,ActionKind::ConfigureLobby|ActionKind::SetLobbyPolicy|ActionKind::SetLobbyAccess|ActionKind::ModeratePlayer|ActionKind::PlayerDetails|ActionKind::RoomBans|ActionKind::CancelLobby|ActionKind::EndLobby|ActionKind::DeleteLobby|ActionKind::GetPlayerRules|ActionKind::SetPlayerRules|ActionKind::GetWorldRules|ActionKind::SetWorldRules){core.ux1_guard_room(input.room.as_deref())?;}
    match input.action {
        ActionKind::ReviewUnknownOutcome=>core.ux1_review_outcome(input.outcome_lane.as_deref().ok_or("Select the uncertain action")?,input.outcome_identity.as_deref().ok_or("Select the original action")?).await?,
        ActionKind::AcknowledgeUnknownOutcome=>core.ux1_acknowledge_outcome(input.outcome_lane.as_deref().ok_or("Select the uncertain action")?,input.outcome_review.as_deref().ok_or("Review current state first")?)?,
        ActionKind::ForgetSaved=>core.forget_saved(input.origin.as_deref().ok_or("Select the saved service address")?)?,
        ActionKind::RefreshLobby=>{core.ux1_guard_room(input.room.as_deref())?;core.ux1_refresh().await?;},
        ActionKind::RecoveryList=>{core.ux1_guard_room(input.room.as_deref())?;core.ux1_recovery(None,None).await?;},
        ActionKind::RecoveryOriginal|ActionKind::RecoveryPending=>{core.ux1_guard_room(input.room.as_deref())?;let selector=if matches!(input.action,ActionKind::RecoveryOriginal){"original"}else{"pending"};core.ux1_recovery(Some(input.recovery_player.ok_or("Select the current player inspection")?),Some(selector)).await?;},
        ActionKind::ReviewRecovery=>{core.ux1_guard_room(input.room.as_deref())?;core.ux1_recovery(Some(input.recovery_player.ok_or("Review the current player inspection")?),None).await?;},
        ActionKind::CreateLobby=>core.ux1_create(input.create.ok_or("Lobby creation details required")?).await?,
        ActionKind::RetryLobbyAction=>core.ux1_retry().await?,
        ActionKind::PlayerDetails=>core.ux1_player(input.target.ok_or("Select a player")?).await?,
        ActionKind::RoomBans=>core.ux1_bans(input.ban_page.unwrap_or_default()).await?,
        ActionKind::GetPlayerRules=>core.ux1_rules(true,None,None).await?,
        ActionKind::SetPlayerRules=>core.ux1_rules(true,Some(input.player_rules.ok_or("Player rules required")?),None).await?,
        ActionKind::GetWorldRules=>core.ux1_rules(false,None,None).await?,
        ActionKind::SetWorldRules=>core.ux1_rules(false,None,Some(input.world_rules.ok_or("World rules required")?)).await?,
        ActionKind::ConfigureLobby|ActionKind::SetLobbyPolicy|ActionKind::SetLobbyAccess|ActionKind::ModeratePlayer|ActionKind::CancelLobby|ActionKind::EndLobby|ActionKind::DeleteLobby=>{
            let kind=match input.action{ActionKind::ConfigureLobby=>"configure",ActionKind::SetLobbyPolicy=>"policy",ActionKind::SetLobbyAccess=>"access",ActionKind::ModeratePlayer=>"moderate",ActionKind::CancelLobby=>"cancel",ActionKind::EndLobby=>"end",_=>"delete"};
            core.ux1_room_action(kind,input.revision.ok_or("Review the current lobby revision")?,input.config,input.policy,input.access,input.moderation).await?;
        },
        ActionKind::SetDebugDiagnostics=>core.set_debug(input.debug_diagnostics.ok_or("Debug setting required")?)?,
        ActionKind::RejoinPrevious=>core.rejoin_previous()?,
        ActionKind::ContinueOffline=>core.continue_offline()?,
        ActionKind::ConfigureGame => {
            core.configure_game(input.game_path.as_deref().ok_or("Choose installation")?).map_err(|e|if cfg!(feature="beta-portable"){format!("Cannot verify this setup: {e}. Check the selected Noita folder and use Review mod installation / repair for the matching beta; see Compatibility details for service mismatches.")}else{e})?;
        }
        ActionKind::CreateFixture => {
            core.create_fixture(input.allow_guests.unwrap_or(false))
                .await?
        }
        ActionKind::Join => core.join_selected(input.room.as_deref().ok_or("Choose lobby")?,input.password).await?,
        ActionKind::PreviewLobby=>core.preview_lobby(input.room.as_deref().ok_or("Choose lobby")?).await?,
        ActionKind::LeaveLobby=>core.leave_lobby().await?,
        ActionKind::Ready => core.ready(input.ready.ok_or("Readiness required")?).await?,
        ActionKind::ChatSend => {
            core.chat_send(input.text.ok_or("Message required")?)
                .await?
        }
        ActionKind::NextRooms => core.room_page(true).await?,
        ActionKind::FirstRooms => core.room_page(false).await?,
        ActionKind::LoadGame => {
            core.load_game().await?;
        }
        ActionKind::Start => {
            if core.automatic_setup_enabled(){
                core.start_captured(input.start_capture.ok_or("Review current lobby and game inspections")?,input.participants.ok_or("Review current game inspections")?,input.excluded.unwrap_or_default(),input.config_revision.ok_or("Review lobby configuration")?).await?;
            } else {
            core.final_start(
                input.selected.unwrap_or_default(),
                input.excluded.unwrap_or_default(),
            )
            .await?
            }
        }
        ActionKind::End => core.end().await?,
        ActionKind::Refresh => core.rotate().await?,
        ActionKind::OpenPasswordChange => core.open_password_change()?,
        ActionKind::CancelSignIn => core.cancel_sign_in().await?,
        ActionKind::Cancel => core.cancel().await?,
        ActionKind::OpenWebsite => crate::browser::open_service(
            core.view()["server_origin"]
                .as_str()
                .ok_or("Choose service")?,
        )?,
    }
    Ok(core.view())
}
#[tauri::command]
async fn g1_logout(w: tauri::WebviewWindow, state: tauri::State<'_, App>) -> Result<Value, String> {
    trusted(&w)?;
    crate::pipe::retire_lc1_bridge(&state.diagnostics);
    state.generation.fetch_add(1, Ordering::AcqRel);
    *state.pending.lock().unwrap() = None;
    let _permit = permit(&state, true)?;
    state.core.command(move|core|Box::pin(async move{core.logout().await?;Ok(core.view())})).await
}
pub fn run() {
    run_mode(false,false)
}
/// Separate candidate entry only. No native compatibility approval or installation implied.
pub fn run_g3(){run_mode(true,false)}
/// Explicit separate uninstalled launcher; a request never bypasses installation approval.
pub fn run_g3_social(){run_mode(true,true)}
#[derive(Debug)]
struct StartupError(&'static str);
impl std::fmt::Display for StartupError{fn fmt(&self,f:&mut std::fmt::Formatter<'_>)->std::fmt::Result{f.write_str(self.0)}}
impl std::error::Error for StartupError{}
// Shared by the normal setup and isolated initialization tests. No credential reads or network work.
fn initialize_core<S:crate::secrets::CredentialStore>(store:S,device:uuid::Uuid,
    pending:Arc<Mutex<Option<handoff::Pending>>>,config:&std::path::Path,
    expected:Arc<Mutex<Option<std::path::PathBuf>>>,
    diagnostics:Arc<Mutex<crate::pipe::NativeDiagnostics>>,
    exchange:Arc<Mutex<nt2_desktop_g0::g3_runtime::Exchange>>,
    social:Arc<Mutex<nt2_desktop_g0::g3_social_stream::Exchange>>,
    g3_enabled:bool,social_requested:bool)->Result<Box<Core<S>>,StartupError>{
    let mut core=Box::new(Core::new(store,device,pending,config.join("consumed-worlds")));
    core.expected_game=expected;core.native_diagnostics=diagnostics;
    if g3_enabled{core.enable_g3(&config.join("g3-operations.jsonl"),exchange).map_err(|_|StartupError("bank-history"))?;}
    if cfg!(feature="g3-c2-live"){core.enable_live_transport().map_err(|_|StartupError("transport"))?;core.enable_automatic_setup();}
    if social_requested{core.request_social(social).map_err(|_|StartupError("transport"))?;}
    if cfg!(feature="g3-rejoin"){core.enable_lc1();}
    core.public_preferences(config.join("servers.json")).map_err(|_|StartupError("server-preferences"))?;
    if cfg!(feature="beta-portable"){core.restore_local_installation();}
    Ok(core)
}
// Shared production spawn boundary, also exercised on a normal-sized startup thread.
fn start_scheduler<S:crate::secrets::CredentialStore+'static>(core:Box<Core<S>>,stop:Arc<AtomicBool>)
    ->(scheduler::Handle<S>,tauri::async_runtime::JoinHandle<()>){
    let(handle,task)=scheduler::new_boxed(core,stop);
    (handle,tauri::async_runtime::spawn(task))
}
// Fixed codes only: never copy journal payloads, credentials or arbitrary errors into diagnostics.
fn startup_diagnostic(root:Option<&std::path::Path>,stage:&str)->String{
    let code=match stage{"bridge"=>"bridge","runtime"=>"runtime","bank-history"=>"bank-history","server-preferences"=>"server-preferences","transport"=>"transport","webview2"=>"webview2","state"=>"state",_=>"application-setup"};
    if code=="webview2"{return beta::webview_prerequisite_message()}
    if code=="state"{return "Noita Together 2 could not open its selected per-user state folder. Restore the folder selected by LOCALAPPDATA/NoitaTogether2/state-location.json or fix access permissions. NT2 will not silently switch history. Do not delete/reset account, device or world-association files.".into()}
    let saved=root.and_then(|root|{
        std::fs::create_dir_all(root).ok()?;
        let path=root.join("startup-error.json");
        std::fs::write(&path,format!("{{\"version\":1,\"code\":\"client-startup-failed\",\"stage\":\"{code}\"}}\n")).ok()?;
        Some(path)
    });
    let detail=saved.map(|p|format!("Diagnostic: {}",p.display())).unwrap_or_else(||"The startup diagnostic could not be saved.".into());
    let bridge=if code=="bridge"{" The local game bridge at 127.0.0.1:25570 is already in use or unavailable."}else{""};
    format!("Noita Together could not start ({code}).{bridge} Local history has been preserved.\n\nClose any other Client instance and try again. If this continues, send this diagnostic to the coordinator. Do not delete or reset the journal.\n\n{detail}")
}
fn present_startup_failure(root:Option<&std::path::Path>,stage:&str,display:impl FnOnce(&str)){
    display(&startup_diagnostic(root,stage));
}
fn startup_failure(stage:&str){
    let root=std::env::var_os("LOCALAPPDATA").map(std::path::PathBuf::from).map(|p|p.join("NoitaTogether2/diagnostics"));
    present_startup_failure(root.as_deref(),stage,|message|{
    #[cfg(windows)] unsafe{
        #[link(name="user32")] unsafe extern "system"{fn MessageBoxW(window:*mut std::ffi::c_void,text:*const u16,title:*const u16,flags:u32)->i32;}
        let text:Vec<u16>=message.encode_utf16().chain(Some(0)).collect();
        let title:Vec<u16>="Noita Together - startup failed".encode_utf16().chain(Some(0)).collect();
        MessageBoxW(std::ptr::null_mut(),text.as_ptr(),title.as_ptr(),0x10|0x10000);
    }
    #[cfg(not(windows))] eprintln!("{message}");
    });
}
fn run_mode(g3_enabled:bool,social_requested:bool) {
    if cfg!(feature="beta-portable")&&tauri::webview_version().is_err(){startup_failure("webview2");return;}

    let pending = Arc::new(Mutex::new(None));
    let stop = Arc::new(AtomicBool::new(false));
    let expected = Arc::new(Mutex::new(None));
    let diagnostics = Arc::new(Mutex::new(crate::pipe::NativeDiagnostics::default()));
    let exchange=Arc::new(Mutex::new(nt2_desktop_g0::g3_runtime::Exchange::default()));
    let social_exchange=Arc::new(Mutex::new(nt2_desktop_g0::g3_social_stream::Exchange::default()));
    let dispatcher=crate::pipe::Dispatcher{pending:pending.clone(),diagnostics:Some(diagnostics.clone()),g3:Some(exchange.clone()),social:Some(social_exchange.clone()),startup:Some(Arc::new(crate::pipe::startup::Lane::new(pending.clone())))};
    let worker=crate::local_bridge::start(dispatcher,expected.clone(),stop.clone());
    let worker=match worker{Ok(worker)=>worker,Err(_)=>{startup_failure("bridge");return;}};
    let network = Arc::new(Mutex::new(None));
    let setup_network = network.clone();
    let setup_stop = stop.clone();
    let app = tauri::Builder::default()
        .setup(move |app| {
            let mut failure_stage="application-setup";
            let result=(|| -> Result<(),Box<dyn std::error::Error>> {
            let legacy = app.path().app_config_dir()?.join(if cfg!(feature="g3-c2-live"){"g3-c2-dev"}else if social_requested{"g3-social-dev"}else if g3_enabled{"g3-dev"}else{"g2-next"});
            let config=if cfg!(feature="beta-portable"){beta::state_root(&legacy,&std::path::PathBuf::from(std::env::var_os("LOCALAPPDATA").ok_or("Local user data folder unavailable")?)).map_err(|e|{failure_stage="state";std::io::Error::other(e)})?}else{legacy};
            std::fs::create_dir_all(&config)?;
            let device = crate::g1_client::device_at(&config.join("device-id"))
                .map_err(std::io::Error::other)?;
            let mut core = initialize_core(WindowsCredentialStore, device, pending.clone(), &config,
                expected.clone(),diagnostics.clone(),exchange.clone(),social_exchange.clone(),g3_enabled,social_requested)
                .inspect_err(|error|failure_stage=error.0)
                .map_err(std::io::Error::other)?;
            if g3_enabled{let shared=std::path::PathBuf::from(std::env::var_os("LOCALAPPDATA").ok_or("No local app data")?).join("NoitaTogether2");core.enable_debug(shared.join("diagnostics"),shared.join("debug-settings.json"));}
            let generation = core.generation.clone();
            let(core,task)=start_scheduler(core,setup_stop.clone());
            *setup_network.lock().unwrap()=Some(task);
            app.manage(App {
                core,diagnostics:diagnostics.clone(),
                pending: pending.clone(),
                generation,
                slots: Arc::new(AtomicUsize::new(0)),
            });
            tauri::WebviewWindowBuilder::from_config(app, &app.config().app.windows[0])?
                .title(if cfg!(feature="beta-portable"){beta::NAME}else if g3_enabled{"NT2 G3 development candidate — uninstalled"}else{"NT2 G2 candidate — isolated"})
                .initialization_script("window.__NT2_CANDIDATE__=true;")
                .on_navigation(|url| crate::g1_client::navigation(url.as_str()))
                .build()?;
            Ok(())
            })();
            if result.is_err(){startup_failure(failure_stage);setup_stop.store(true,Ordering::Release);app.handle().exit(1);}
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            g1_status, g1_login, g1_action, g1_logout, g3_original_status, g1_browse_game, g1_beta_setup, g1_beta_discover, g1_beta_export
        ])
        .on_page_load(|window, payload| {
            if let Some(state) = window.try_state::<App>() {
                crate::pipe::retire_lc1_bridge(&state.diagnostics);
                state.generation.fetch_add(1, Ordering::AcqRel);
                *state.pending.lock().unwrap() = None;
            }
            if !crate::g1_client::navigation(payload.url().as_str()) {
                let _ = window.close();
            }
        })
        .build(tauri::generate_context!());
    let app=match app{Ok(app)=>app,Err(_)=>{startup_failure("runtime");stop.store(true,Ordering::Release);let _=worker.join();return;}};
    app.run(move |handle, event| {
        if matches!(event, tauri::RunEvent::Exit) {
            if let Some(state)=handle.try_state::<App>(){
                state.generation.fetch_add(1, Ordering::AcqRel);
                *state.pending.lock().unwrap() = None;
            }
            stop.store(true, Ordering::Release);
        }
    });
    if let Some(task) = network.lock().unwrap().take() {
        let _ = tauri::async_runtime::block_on(task);
    }
    let _ = worker.join();
}
/// Invoked only by the separate verification example; owns an ephemeral profile/device.
pub fn run_probe() {
    let output = std::path::PathBuf::from(
        std::env::var_os("NT2_PRODUCT_PROBE_OUT").expect("isolated result path"),
    );
    let profile = output.parent().unwrap().join("webview-profile");
    let setup_output = output.clone();
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            g1_status, g1_login, g1_action, g1_logout, g3_original_status
        ])
        .setup(move |app| {
            let pending = Arc::new(Mutex::new(None));
            let mut core = Core::new(
                WindowsCredentialStore,
                crate::g1_client::device_at(&setup_output.parent().unwrap().join("device-id"))
                    .map_err(std::io::Error::other)?,
                pending.clone(),
                setup_output.parent().unwrap().join("consumed-worlds"),
            );
            core.public_preferences(setup_output.parent().unwrap().join("servers.json"))
                .map_err(std::io::Error::other)?;
            core.open_browser = false;
            let diagnostics=core.native_diagnostics.clone();
            let generation = core.generation.clone();
            let(core,scheduler)=scheduler::new(core,Arc::new(AtomicBool::new(false)));
            tauri::async_runtime::spawn(scheduler);
            app.manage(App {
                core,diagnostics:diagnostics.clone(),
                pending,
                generation,
                slots: Arc::new(AtomicUsize::new(0)),
            });
            let handle = app.handle().clone();
            let result = setup_output.clone();
            tauri::WebviewWindowBuilder::from_config(app, &app.config().app.windows[0])?
                .visible(false)
                .data_directory(profile.clone())
                .initialization_script("window.__NT2_CANDIDATE__=true;")
                .initialization_script(include_str!("../../tools/product-webview-probe.js"))
                .on_navigation(move |url| {
                    if let Some((_, value)) = url.query_pairs().find(|(key, _)| key == "nt2_probe")
                    {
                        if value.len() > 8192 {
                            handle.exit(1);
                            return false;
                        }
                        let parsed: Value = serde_json::from_str(&value).unwrap_or_default();
                        let passed = parsed["passed"].as_bool() == Some(true);
                        let _ = std::fs::write(&result, serde_json::to_vec(&parsed).unwrap());
                        handle.exit(if passed { 0 } else { 1 });
                        return false;
                    }
                    crate::g1_client::navigation(url.as_str())
                })
                .build()?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("Isolated product WebView");
}

#[cfg(test)]
#[path="next_startup_test.rs"]
mod startup_tests;
