use crate::handoff;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};
const REV: &str = "g0-pairing-1";
const SERVICE: &str = "nt2-g0-local";
struct Session {
    token: String,
    epoch: String,
}
struct Attempt {
    id: String,
    device: String,
    secret: String,
    code: String,
    persona: String,
}
struct Attachment {
    session: Session,
    binding: Value,
    instance: String,
}
pub(super) struct Core {
    http: reqwest::Client,
    base: String,
    service_epoch: Option<String>,
    session: Option<Session>,
    attempt: Option<Attempt>,
    attachment: Option<Attachment>,
    room: Option<Value>,
    history: Vec<String>,
    sent_seed: bool,
    sent_loaded: bool,
}
pub(super) struct App {
    pub core: tokio::sync::Mutex<Core>,
    pub pending: Arc<Mutex<Option<handoff::Pending>>>,
}
pub(super) fn new_core() -> Core {
    Core {
        http: reqwest::Client::builder()
            .no_proxy()
            .timeout(std::time::Duration::from_secs(3))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .unwrap(),
        base: String::new(),
        service_epoch: None,
        session: None,
        attempt: None,
        attachment: None,
        room: None,
        history: vec![],
        sent_seed: false,
        sent_loaded: false,
    }
}
fn trusted(w: &tauri::WebviewWindow) -> Result<(), String> {
    let u = w.url().map_err(|_| "Unavailable origin")?;
    if w.label() != "main"
        || !(u.scheme() == "tauri"
            || (u.scheme() == "http" || u.scheme() == "https")
                && u.host_str() == Some("tauri.localhost"))
    {
        return Err("Command origin denied".into());
    }
    Ok(())
}
fn endpoint(s: &str) -> Result<String, String> {
    let u = reqwest::Url::parse(s).map_err(|_| "Invalid server")?;
    if u.scheme() != "http"
        || u.host_str() != Some("127.0.0.1")
        || !u.username().is_empty()
        || u.password().is_some()
        || u.query().is_some()
        || u.fragment().is_some()
        || u.path() != "/"
        || u.port().is_none()
    {
        return Err("G0 accepts only an explicit http://127.0.0.1:port mock endpoint".into());
    }
    Ok(s.trim_end_matches('/').to_string())
}
fn id(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 80
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}
fn identifier(v: &Value, k: &str) -> Result<String, String> {
    v[k].as_str()
        .filter(|s| id(s))
        .map(String::from)
        .ok_or(format!("Invalid {k}"))
}
fn session(v: &Value, kind: &str) -> Result<Session, String> {
    if v["session"]["kind"] != kind {
        return Err("Wrong session kind".into());
    }
    let token = v["token"]
        .as_str()
        .filter(|s| !s.is_empty() && s.len() <= 256 && s.is_ascii())
        .ok_or("Invalid session")?
        .to_string();
    Ok(Session {
        token,
        epoch: identifier(&v["session"], "epoch")?,
    })
}
async fn response(r: reqwest::RequestBuilder, legacy: bool) -> Result<Value, String> {
    let mut r = r.send().await.map_err(|_| "Mock service unavailable")?;
    let status = r.status();
    let mut b = Vec::new();
    while let Some(c) = r.chunk().await.map_err(|_| "Response failed")? {
        if b.len() + c.len() > 65536 {
            return Err("Oversized service response".into());
        }
        b.extend_from_slice(&c);
    }
    let v: Value = serde_json::from_slice(&b).map_err(|_| "Invalid service response")?;
    let revision = if legacy { "g0-json-1" } else { REV };
    let key = if legacy { "serverId" } else { "serviceId" };
    if v["mock"] != true || v[key] != SERVICE || v["contractRevision"] != revision {
        return Err("Wrong service identity or contract".into());
    }
    if !status.is_success() {
        return Err(v["error"]["code"]
            .as_str()
            .unwrap_or("Service rejected request")
            .to_string());
    }
    Ok(v)
}
fn common() -> Value {
    json!({"contractRevision":REV,"serviceId":SERVICE})
}
fn proof(c: &Core) -> Result<Value, String> {
    let a = c.attempt.as_ref().ok_or("No pairing attempt")?;
    let mut p = common();
    p["attemptId"] = json!(a.id);
    p["deviceId"] = json!(a.device);
    p["pollingSecret"] = json!(a.secret);
    Ok(p)
}
fn same_service(c: &Core, v: &Value) -> Result<(), String> {
    if c.service_epoch.as_deref() != v["serviceEpoch"].as_str() {
        return Err("Service restarted".into());
    }
    Ok(())
}
async fn begin(c: &mut Core, base: String, persona: String) -> Result<(), String> {
    if c.session.is_some() || c.attempt.is_some() {
        return Err("Already paired or pairing".into());
    }
    if !["host", "guest", "observer"].contains(&persona.as_str()) {
        return Err("Invalid fixture persona".into());
    }
    let device = uuid::Uuid::new_v4().to_string();
    let mut p = common();
    p["deviceId"] = json!(device);
    p["requestId"] = json!(uuid::Uuid::new_v4().to_string());
    let v = response(
        c.http.post(format!("{base}/g0/pairings/initiate")).json(&p),
        false,
    )
    .await?;
    c.service_epoch = Some(identifier(&v, "serviceEpoch")?);
    if v["deviceId"] != device || v["state"] != "pending" {
        return Err("Invalid pairing response".into());
    }
    c.attempt = Some(Attempt {
        id: identifier(&v, "attemptId")?,
        device,
        secret: identifier(&v, "pollingSecret")?,
        code: identifier(&v, "activationCode")?,
        persona,
    });
    c.base = base.clone();
    if !c.history.contains(&base) {
        c.history.push(base);
        if c.history.len() > 16 {
            c.history.remove(0);
        }
    }
    Ok(())
}
async fn approve(c: &Core) -> Result<(), String> {
    let a = c.attempt.as_ref().ok_or("Start pairing first")?;
    let browser = response(
        c.http
            .post(format!("{}/g0/sessions", c.base))
            .json(&json!({"persona":a.persona,"kind":"browser"})),
        true,
    )
    .await?;
    let browser = session(&browser, "browser")?;
    let mut p = common();
    p["activationCode"] = json!(a.code);
    let v = response(
        c.http
            .post(format!("{}/g0/pairings/approve", c.base))
            .bearer_auth(&browser.token)
            .json(&p),
        false,
    )
    .await?;
    same_service(c, &v)
}
async fn poll_pairing(c: &mut Core) -> Result<(), String> {
    let p = proof(c)?;
    let v = response(
        c.http
            .post(format!("{}/g0/pairings/status", c.base))
            .json(&p),
        false,
    )
    .await?;
    same_service(c, &v)?;
    match v["state"].as_str() {
        Some("pending") => Ok(()),
        Some("approved") => {
            let v = response(
                c.http
                    .post(format!("{}/g0/pairings/exchange", c.base))
                    .json(&p),
                false,
            )
            .await?;
            same_service(c, &v)?;
            c.session = Some(session(&v, "desktop")?);
            c.attempt = None;
            Ok(())
        }
        _ => Err("Pairing no longer usable; start again".into()),
    }
}
async fn cancel_pairing(c: &mut Core) -> Result<(), String> {
    let p = proof(c)?;
    c.attempt = None;
    response(
        c.http
            .post(format!("{}/g0/pairings/cancel", c.base))
            .json(&p),
        false,
    )
    .await?;
    Ok(())
}
async fn refresh(c: &mut Core) -> Result<(), String> {
    let s = c.session.as_ref().ok_or("Pair first")?;
    let r = c.room.as_ref().ok_or("Join first")?;
    let v = response(
        c.http
            .get(format!(
                "{}/g0/desktop/rooms/{}",
                c.base,
                r["id"].as_str().ok_or("Invalid room")?
            ))
            .bearer_auth(&s.token),
        false,
    )
    .await?;
    same_service(c, &v)?;
    c.room = Some(v["room"].clone());
    Ok(())
}
async fn command(c: &mut Core, kind: &str, body: Value, game: bool) -> Result<(), String> {
    let s = if game {
        &c.attachment.as_ref().ok_or("No attached game")?.session
    } else {
        c.session.as_ref().ok_or("Pair first")?
    };
    let r = c.room.as_ref().ok_or("Join first")?;
    let mut p = common();
    for k in ["roomId", "runId", "runEpoch", "configRevision"] {
        p[k] = r[if k == "roomId" { "id" } else { k }].clone();
    }
    p["serviceEpoch"] = json!(c.service_epoch);
    p["requestId"] = json!(uuid::Uuid::new_v4().to_string());
    p["connectionEpoch"] = json!(s.epoch);
    p["expectedRevision"] = r["revision"].clone();
    p["kind"] = json!(kind);
    p["body"] = body;
    let v = response(
        c.http
            .post(format!("{}/g0/desktop/commands", c.base))
            .bearer_auth(&s.token)
            .json(&p),
        false,
    )
    .await?;
    same_service(c, &v)?;
    c.room = Some(v["room"].clone());
    Ok(())
}
async fn join(c: &mut Core, room: String) -> Result<(), String> {
    if !["g0-desktop-room-forest", "g0-desktop-room-caverns"].contains(&room.as_str()) {
        return Err("Unknown fixture room".into());
    }
    if c.room.is_some() {
        return Err("Already joined".into());
    }
    let s = c.session.as_ref().ok_or("Pair first")?;
    let v = response(
        c.http
            .get(format!("{}/g0/desktop/rooms/{room}", c.base))
            .bearer_auth(&s.token),
        false,
    )
    .await?;
    same_service(c, &v)?;
    c.room = Some(v["room"].clone());
    command(c, "desktop.join", json!({}), false).await?;
    command(c, "desktop.ready", json!({"ready":true}), false).await
}
fn preparation(c: &Core) -> Result<handoff::Pending, String> {
    let r = c.room.as_ref().ok_or("No room")?;
    let seed = r["seed"]
        .as_u64()
        .filter(|x| *x <= u32::MAX as u64)
        .ok_or("Invalid seed")? as u32;
    let config = r["configRevision"]
        .as_u64()
        .filter(|n| *n > 0)
        .ok_or("Invalid config")?;
    let deadline = r["preparationExpiresAt"]
        .as_u64()
        .ok_or("No preparation deadline")?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| "Clock unavailable")?
        .as_millis() as u64;
    let duration = deadline
        .checked_sub(now)
        .filter(|d| *d > 0 && *d <= 120000)
        .ok_or("Invalid preparation deadline")?;
    let mut p = handoff::new(seed, identifier(r, "runId")?, config);
    p.epoch = identifier(r, "runEpoch")?;
    p.expires = std::time::Instant::now() + std::time::Duration::from_millis(duration);
    Ok(p)
}
async fn attach(c: &mut Core, p: &handoff::Pending) -> Result<(), String> {
    let (pid, nonce) = p.owner.ok_or("No native owner")?;
    let instance = format!(
        "game-{pid}-{}",
        nonce.iter().map(|b| format!("{b:02x}")).collect::<String>()
    );
    let r = c.room.as_ref().ok_or("No room")?;
    let s = c.session.as_ref().ok_or("No desktop")?;
    let mut body = common();
    for k in ["roomId", "runId", "runEpoch", "configRevision"] {
        body[k] = r[if k == "roomId" { "id" } else { k }].clone();
    }
    body["serviceEpoch"] = json!(c.service_epoch);
    body["connectionEpoch"] = json!(s.epoch);
    body["expectedRevision"] = r["revision"].clone();
    body["gameInstanceId"] = json!(instance);
    let v = response(
        c.http
            .post(format!("{}/g0/desktop/attach", c.base))
            .bearer_auth(&s.token)
            .json(&body),
        false,
    )
    .await?;
    same_service(c, &v)?;
    let b = &v["binding"];
    for k in [
        "serviceEpoch",
        "roomId",
        "runId",
        "runEpoch",
        "configRevision",
        "gameInstanceId",
    ] {
        if b[k] != body[k] {
            return Err("Wrong attachment binding".into());
        }
    }
    identifier(b, "attachmentId")?;
    c.attachment = Some(Attachment {
        session: session(&v, "game")?,
        binding: b.clone(),
        instance,
    });
    refresh(c).await
}
fn invalidate(c: &mut Core, pending: &Mutex<Option<handoff::Pending>>) {
    *pending.lock().unwrap() = None;
    c.attachment = None;
    c.sent_seed = false;
    c.sent_loaded = false;
}
async fn sync_core(c: &mut Core, pending: &Mutex<Option<handoff::Pending>>) -> Result<(), String> {
    if c.attempt.is_some() {
        if let Err(e) = poll_pairing(c).await {
            c.attempt = None;
            invalidate(c, pending);
            return Err(e);
        }
    }
    if c.room.is_none() {
        return Ok(());
    }
    if let Err(e) = refresh(c).await {
        invalidate(c, pending);
        c.session = None;
        c.room = None;
        return Err(format!("{e}; preparation invalidated"));
    }
    let p = pending.lock().unwrap().clone();
    if let Some(p) = p {
        let r = c.room.as_ref().unwrap();
        if r["runId"] != p.run
            || r["runEpoch"] != p.epoch
            || r["configRevision"] != p.config
            || !["preparing", "loading", "running"].contains(&r["state"].as_str().unwrap_or(""))
            || std::time::Instant::now() >= p.expires
        {
            invalidate(c, pending);
            return Ok(());
        }
        let result=async{if p.owner.is_some()&&c.attachment.is_none(){attach(c,&p).await?}if p.accepted&&!c.sent_seed{let a=c.attachment.as_ref().ok_or("No attachment")?;command(c,"game.seedAccepted",json!({"attachmentId":a.binding["attachmentId"],"gameInstanceId":a.instance,"seed":p.seed}),true).await?;c.sent_seed=true}if p.loaded&&!c.sent_loaded&&c.room.as_ref().unwrap()["state"]=="loading"{let a=c.attachment.as_ref().ok_or("No attachment")?;command(c,"game.loaded",json!({"attachmentId":a.binding["attachmentId"],"gameInstanceId":a.instance}),true).await?;c.sent_loaded=true}Ok::<(),String>(())}.await;
        if let Err(e) = result {
            invalidate(c, pending);
            return Err(e);
        }
    } else {
        c.attachment = None
    }
    Ok(())
}
#[derive(Serialize)]
pub(super) struct View {
    mock: bool,
    signed_in: bool,
    activation_code: Option<String>,
    history: Vec<String>,
    room: Option<Value>,
    handoff: String,
    seed: Option<u32>,
    limits: &'static str,
}
fn view(c: &Core, p: &Option<handoff::Pending>) -> View {
    View {
        mock: true,
        signed_in: c.session.is_some(),
        activation_code: c.attempt.as_ref().map(|a| a.code.clone()),
        history: c.history.clone(),
        room: c.room.clone(),
        handoff: match p {
            None => "No preparation",
            Some(p) if p.loaded => "World loaded",
            Some(p) if p.accepted => "Seed accepted",
            Some(p) if p.owner.is_some() => "Game attached",
            Some(_) => "Choose normal New Game in Noita",
        }
        .into(),
        seed: p.as_ref().map(|p| p.seed),
        limits: "Local mock browser approval; g0-pairing-1 provisional; no provider/refresh persistence",
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Login {
    server: String,
    persona: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Join {
    room: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Chat {
    text: String,
}
#[tauri::command]
pub(super) async fn g0_login(
    w: tauri::WebviewWindow,
    state: tauri::State<'_, App>,
    input: Login,
) -> Result<View, String> {
    trusted(&w)?;
    let mut c = state.core.lock().await;
    begin(&mut c, endpoint(&input.server)?, input.persona).await?;
    Ok(view(&c, &state.pending.lock().unwrap()))
}
#[tauri::command]
pub(super) async fn g0_approve(
    w: tauri::WebviewWindow,
    state: tauri::State<'_, App>,
) -> Result<View, String> {
    trusted(&w)?;
    let mut c = state.core.lock().await;
    approve(&c).await?;
    sync_core(&mut c, &state.pending).await?;
    Ok(view(&c, &state.pending.lock().unwrap()))
}
#[tauri::command]
pub(super) async fn g0_join(
    w: tauri::WebviewWindow,
    state: tauri::State<'_, App>,
    input: Join,
) -> Result<View, String> {
    trusted(&w)?;
    let mut c = state.core.lock().await;
    join(&mut c, input.room).await?;
    Ok(view(&c, &state.pending.lock().unwrap()))
}
#[tauri::command]
pub(super) async fn g0_start(
    w: tauri::WebviewWindow,
    state: tauri::State<'_, App>,
) -> Result<View, String> {
    trusted(&w)?;
    let mut c = state.core.lock().await;
    if state.pending.lock().unwrap().is_some() {
        return Err("Preparation already exists".into());
    }
    command(&mut c, "run.start", json!({}), false).await?;
    *state.pending.lock().unwrap() = Some(preparation(&c)?);
    c.attachment = None;
    c.sent_seed = false;
    c.sent_loaded = false;
    Ok(view(&c, &state.pending.lock().unwrap()))
}
#[tauri::command]
pub(super) async fn g0_cancel(
    w: tauri::WebviewWindow,
    state: tauri::State<'_, App>,
) -> Result<View, String> {
    trusted(&w)?;
    *state.pending.lock().unwrap() = None;
    let mut c = state.core.lock().await;
    invalidate(&mut c, &state.pending);
    if c.attempt.is_some() {
        cancel_pairing(&mut c).await?
    } else if c
        .room
        .as_ref()
        .is_some_and(|r| r["state"] == "preparing" || r["state"] == "loading")
    {
        command(&mut c, "run.cancel", json!({}), false).await?
    }
    Ok(view(&c, &None))
}
#[tauri::command]
pub(super) async fn g0_chat(
    w: tauri::WebviewWindow,
    state: tauri::State<'_, App>,
    input: Chat,
) -> Result<View, String> {
    trusted(&w)?;
    if input.text.trim().is_empty() || input.text.chars().count() > 500 {
        return Err("Chat must contain 1–500 characters".into());
    }
    let mut c = state.core.lock().await;
    command(&mut c, "chat.send", json!({"text":input.text}), false).await?;
    Ok(view(&c, &state.pending.lock().unwrap()))
}
#[tauri::command]
pub(super) async fn g0_status(
    w: tauri::WebviewWindow,
    state: tauri::State<'_, App>,
) -> Result<View, String> {
    trusted(&w)?;
    let mut c = state.core.lock().await;
    sync_core(&mut c, &state.pending).await?;
    Ok(view(&c, &state.pending.lock().unwrap()))
}
#[cfg(test)]
#[path = "service_test.rs"]
mod tests;
