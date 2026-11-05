//! Single live client lifecycle. G0 is retained only as regression evidence.
use crate::{
    handoff,
    lifecycle::SessionResources,
    preferences::{Preferences, RecentServer},
    protocol,
    protocol_g2::{self, wire},
    secrets::{CredentialStore, SecretBlob, SecretString, ServiceProfile, WindowsCredentialStore},
};
use serde::{Deserialize, Serialize};
use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use zeroize::{Zeroize, Zeroizing};
fn installed_profile() -> Result<(), String> {
    use sha2::{Digest, Sha256};
    use std::io::Read;
    let profile: serde_json::Value = serde_json::from_str("{}")
    .map_err(|_| "Invalid built native profile")?;
    let root = std::path::Path::new(r"C:\Noita");
    for (file, key) in [
        (
            "mods/nt2_g0_handoff/files/nt2_g0_bridge.dll",
            "bridgeSha256",
        ),
        ("mods/nt2_g0_handoff/init.lua", "modSha256"),
        ("lua51.dll", "originalLuaSha256"),
        ("noita.exe", "gameExeSha256"),
    ] {
        let path = root.join(file);
        let metadata = std::fs::symlink_metadata(&path)
            .map_err(|_| "Matched native profile is not installed")?;
        if !metadata.is_file()
            || metadata.file_type().is_symlink()
            || metadata.len() > 80 * 1024 * 1024
        {
            return Err("Native profile path/size rejected".into());
        }
        let mut input = std::fs::File::open(&path).map_err(|_| "Native profile unavailable")?;
        let mut hash = Sha256::new();
        let mut buffer = [0u8; 65536];
        let mut count = 0usize;
        loop {
            let n = input
                .read(&mut buffer)
                .map_err(|_| "Native profile read failed")?;
            if n == 0 {
                break;
            }
            count += n;
            if count > 80 * 1024 * 1024 {
                return Err("Native profile oversized".into());
            }
            hash.update(&buffer[..n]);
        }
        let actual = hash
            .finalize()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>();
        if profile[key].as_str() != Some(actual.as_str()) {
            return Err("Installed game/mod/bridge does not match the built local profile; repair with Noita closed".into());
        }
    }
    Ok(())
}
const REV: &str = "g1-draft-2";
const G2_REV: &str = "g2-draft-1";
struct Permit(Arc<std::sync::atomic::AtomicUsize>);
impl Drop for Permit {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::AcqRel);
    }
}
fn permit(slots: &Arc<std::sync::atomic::AtomicUsize>) -> Result<Permit, String> {
    permit_limit(slots, 16)
}
fn permit_limit(
    slots: &Arc<std::sync::atomic::AtomicUsize>,
    limit: usize,
) -> Result<Permit, String> {
    slots
        .try_update(Ordering::AcqRel, Ordering::Acquire, |n| {
            (n < limit).then_some(n + 1)
        })
        .map_err(|_| "Too many pending client commands")?;
    Ok(Permit(slots.clone()))
}
fn uuid() -> String {
    uuid::Uuid::new_v4().to_string()
}
pub(crate) fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
fn text(value: Option<String>, max: usize) -> Result<String, String> {
    value
        .filter(|v| !v.is_empty() && v.len() <= max)
        .ok_or_else(|| "Invalid service response".into())
}
pub(crate) fn timestamp(s: &str) -> Result<u64, String> {
    // Canonical service timestamps are UTC ISO-8601. Avoid locale/timezone interpretation.
    let b = s.as_bytes();
    if b.len() < 20
        || b[4] != b'-'
        || b[7] != b'-'
        || b[10] != b'T'
        || b[13] != b':'
        || b[16] != b':'
        || !s.ends_with('Z')
    {
        return Err("Invalid UTC deadline".into());
    }
    let part = |a, c| {
        s.get(a..c)
            .and_then(|s| s.parse::<i64>().ok())
            .ok_or("Invalid UTC deadline")
    };
    let (y, m, d, h, min, sec) = (
        part(0, 4)?,
        part(5, 7)?,
        part(8, 10)?,
        part(11, 13)?,
        part(14, 16)?,
        part(17, 19)?,
    );
    if !(1..=12).contains(&m)
        || !(1..=31).contains(&d)
        || !(0..24).contains(&h)
        || !(0..60).contains(&min)
        || !(0..60).contains(&sec)
    {
        return Err("Invalid UTC deadline".into());
    }
    let y = y - i64::from(m <= 2);
    let era = y.div_euclid(400);
    let yo = y - era * 400;
    let mp = m + if m > 2 { -3 } else { 9 };
    let days = era * 146097 + yo * 365 + yo / 4 - yo / 100 + (153 * mp + 2) / 5 + d - 1 - 719468;
    let millis = if b.len() == 20 {
        0
    } else {
        let f = s.get(20..s.len() - 1).ok_or("Invalid UTC deadline")?;
        if b[19] != b'.' || f.len() != 3 {
            return Err("Invalid UTC deadline".into());
        }
        f.parse::<i64>().map_err(|_| "Invalid UTC deadline")?
    };
    u64::try_from((days * 86400 + h * 3600 + min * 60 + sec) * 1000 + millis)
        .map_err(|_| "Invalid UTC deadline".into())
}
struct Session {
    access: SecretString,
    refresh: SecretString,
    account: String,
    id: String,
    epoch: String,
    device: String,
    expires: u64,
    attachment: String,
}
fn session(mut c: wire::Credentials, kind: &str, device: &str) -> Result<Session, String> {
    if c.kind.as_deref() != Some(kind) || c.device_id.as_deref() != Some(device) {
        return Err("Wrong credential kind/device".into());
    }
    let access = SecretString::from(text(c.access_token.take(), 4096)?);
    let refresh = SecretString::from(c.refresh_secret.take().unwrap_or_default());
    if kind != "game" && (refresh.as_str().len() != 43 || !refresh.as_str().is_ascii()) {
        return Err("Invalid refresh credential".into());
    }
    Ok(Session {
        access,
        refresh,
        account: text(c.account_id.take(), 128)?,
        id: text(c.session_id.take(), 128)?,
        epoch: text(c.session_epoch.take(), 128)?,
        device: device.into(),
        expires: timestamp(&text(c.expires_at.take(), 40)?)?,
        attachment: c.attachment_id.take().unwrap_or_default(),
    })
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Journal {
    version: u32,
    device: String,
    refresh: String,
    attempt: String,
    proof: String,
    code: String,
    expires: u64,
    operation: String,
    #[serde(default)]
    operation_revision: String,
    request_id: String,
    started: u64,
}
impl Drop for Journal {
    fn drop(&mut self) {
        self.refresh.zeroize();
        self.proof.zeroize();
    }
}
impl Journal {
    fn new(device: String) -> Self {
        Self {
            version: 1,
            device,
            refresh: String::new(),
            attempt: String::new(),
            proof: String::new(),
            code: String::new(),
            expires: 0,
            operation: String::new(),
            operation_revision: String::new(),
            request_id: String::new(),
            started: 0,
        }
    }
    fn validate(&self, device: &str, revision: &str) -> Result<(), String> {
        if self.version != 1
            || self.device != device
            || self.refresh.len() > 256
            || self.proof.len() > 256
            || self.attempt.len() > 128
            || self.code.len() > 16
            || (!self.operation.is_empty() && self.operation_revision != revision)
            || self.request_id.len() > 128
            || !["", "exchange", "refresh"].contains(&self.operation.as_str())
        {
            return Err("Invalid remembered session; sign in again".into());
        }
        Ok(())
    }
}
struct Game {
    session: Session,
    instance: String,
    owner: (crate::local_owner::Id, [u8; 16]),
    last_resume: Instant,
    retry: Option<(wire::Request, u64)>,
}
#[derive(Clone, PartialEq, Eq)]
struct AttachScope {
    generation: u64,
    base: String,
    service: String,
    service_epoch: String,
    desktop_id: String,
    desktop_epoch: String,
    desktop_account: String,
    desktop_device: String,
    room: String,
    run: String,
    run_epoch: String,
    config: u64,
    owner: (crate::local_owner::Id, [u8; 16]),
}
struct AttachRetry {
    request: wire::Request,
    scope: AttachScope,
    deadline: Instant,
}
struct ChatRetry {
    request: wire::Request,
    desktop: String,
    generation: u64,
    deadline: Instant,
}
pub(super) struct Core<S: CredentialStore> {
    revision: &'static str,
    pub(super) generation: Arc<AtomicU64>,
    http: reqwest::Client,
    resources: SessionResources<S>,
    device: uuid::Uuid,
    base: String,
    service: String,
    epoch: String,
    info: Option<wire::Info>,
    desktop: Option<Session>,
    browser: Option<Session>,
    game: Option<Game>,
    attach_retry: Option<AttachRetry>,
    room: Option<wire::Room>,
    rooms: Vec<wire::RoomSummary>,
    journal: Journal,
    remember: bool,
    last_presence: Instant,
    history: Vec<String>,
    recent_servers: Vec<RecentServer>,
    preferences: Option<Preferences>,
    room_cursor: String,
    next_room_id: Option<String>,
    status_error: Option<String>,
    armed_runs: std::collections::VecDeque<String>,
    game_path: Option<std::path::PathBuf>,
    game_config: Option<std::path::PathBuf>,
    claims_root: Option<std::path::PathBuf>,
    expected_game: Arc<Mutex<Option<std::path::PathBuf>>>,
    chat_room: String,
    chat_sequence: u32,
    chat_messages: Vec<wire::ChatMessage>,
    chat_retry: Option<ChatRetry>,
    retry_until: Instant,
}
pub(super) struct App {
    pub core: Arc<tokio::sync::Mutex<Core<WindowsCredentialStore>>>,
    pub pending: Arc<Mutex<Option<handoff::Pending>>>,
    pub generation: Arc<AtomicU64>,
    pub slots: Arc<std::sync::atomic::AtomicUsize>,
}
pub(super) fn new_core<S: CredentialStore>(store: S, device: uuid::Uuid) -> Core<S> {
    Core {
        revision: REV,
        generation: Arc::new(AtomicU64::new(0)),
        http: crate::transport::client(),
        resources: SessionResources::new(store),
        device,
        base: String::new(),
        service: String::new(),
        epoch: String::new(),
        info: None,
        desktop: None,
        browser: None,
        game: None,
        attach_retry: None,
        room: None,
        rooms: vec![],
        journal: Journal::new(device.to_string()),
        remember: false,
        last_presence: Instant::now() - Duration::from_secs(10),
        history: vec![],
        recent_servers: vec![],
        preferences: None,
        room_cursor: String::new(),
        next_room_id: None,
        status_error: None,
        armed_runs: std::collections::VecDeque::new(),
        game_path: None,
        game_config: None,
        claims_root: None,
        expected_game: Arc::new(Mutex::new(None)),
        chat_room: String::new(),
        chat_sequence: 0,
        chat_messages: vec![],
        chat_retry: None,
        retry_until: Instant::now(),
    }
}
pub(super) fn new_g2_core<S: CredentialStore>(store: S, device: uuid::Uuid) -> Core<S> {
    let mut core = new_core(store, device);
    core.revision = G2_REV;
    core
}
pub(super) fn device_at(path: &std::path::Path) -> Result<uuid::Uuid, String> {
    if path.exists() {
        let meta = std::fs::symlink_metadata(path).map_err(|_| "Device profile unavailable")?;
        if !meta.is_file() || meta.file_type().is_symlink() || meta.len() > 64 {
            return Err("Invalid device profile".into());
        }
        return uuid::Uuid::parse_str(
            std::fs::read_to_string(path)
                .map_err(|_| "Device profile unavailable")?
                .trim(),
        )
        .map_err(|_| "Invalid device profile".into());
    }
    let device = uuid::Uuid::new_v4();
    let parent = path.parent().ok_or("Invalid profile directory")?;
    std::fs::create_dir_all(parent).map_err(|_| "Device profile directory unavailable")?;
    use std::io::Write;
    let mut f = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)
        .map_err(|_| "Device profile unavailable")?;
    f.write_all(device.to_string().as_bytes())
        .and_then(|_| f.sync_all())
        .map_err(|_| "Device profile unavailable")?;
    Ok(device)
}
impl<S: CredentialStore> Core<S> {
    pub(super) fn installation_at(
        &mut self,
        config: std::path::PathBuf,
        expected: Arc<Mutex<Option<std::path::PathBuf>>>,
    ) {
        self.claims_root = Some(config.join("consumed-worlds"));
        self.game_config = Some(config.join("game-path.txt"));
        self.expected_game = expected;
        let stored = self.game_config.as_ref().and_then(|p| {
            std::fs::symlink_metadata(p)
                .ok()
                .filter(|m| m.is_file() && !m.file_type().is_symlink() && m.len() <= 512)
                .and_then(|_| std::fs::read_to_string(p).ok())
        });
        self.game_path = stored
            .and_then(|s| crate::installation::root(&s).ok())
            .or_else(crate::installation::discover);
        if let Some(root) = &self.game_path {
            if crate::installation::verify(root, None).is_ok() {
                *self.expected_game.lock().unwrap() = Some(root.join("noita.exe"));
            }
        }
    }
    fn configure_game(&mut self, value: &str) -> Result<(), String> {
        if self.game.is_some() {
            return Err("Leave the current NT2 run before changing installation".into());
        }
        let root = crate::installation::root(value)?;
        crate::installation::verify(&root, None)?;
        let file = self
            .game_config
            .as_ref()
            .ok_or("Installation preferences unavailable")?;
        let parent = file.parent().ok_or("Invalid preferences path")?;
        std::fs::create_dir_all(parent).map_err(|_| "Installation preferences unavailable")?;
        if file.exists() {
            let m = std::fs::symlink_metadata(file)
                .map_err(|_| "Installation preferences unavailable")?;
            if !m.is_file() || m.file_type().is_symlink() {
                return Err("Installation preferences rejected".into());
            }
        }
        use std::io::Write;
        let temporary = parent.join(format!("game-path-{}.tmp", uuid::Uuid::new_v4()));
        let result = (|| -> std::io::Result<()> {
            let mut out = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temporary)?;
            out.write_all(value.as_bytes())?;
            out.sync_all()?;
            drop(out);
            std::fs::rename(&temporary, file)
        })();
        if result.is_err() {
            let _ = std::fs::remove_file(&temporary);
            return Err("Installation preferences write failed".into());
        }
        *self.expected_game.lock().unwrap() = Some(root.join("noita.exe"));
        self.game_path = Some(root);
        Ok(())
    }
    fn typed<T: serde::de::DeserializeOwned>(&self, name: &str, bytes: &[u8]) -> Result<T, ()> {
        if self.revision == REV {
            protocol::typed(name, bytes).map_err(|_| ())
        } else {
            protocol_g2::typed(name, bytes).map_err(|_| ())
        }
    }
    fn validate_request(&self, bytes: &[u8]) -> Result<(), String> {
        let valid = if self.revision == REV {
            protocol::request(bytes).map(|_| ()).map_err(|_| ())
        } else {
            protocol_g2::request(bytes).map(|_| ()).map_err(|_| ())
        };
        valid.map_err(|_| "Request violates contract bounds".into())
    }
    pub(super) fn preferences_at(&mut self, path: std::path::PathBuf) -> Result<(), String> {
        let preferences = Preferences::at(path);
        self.recent_servers = preferences.load(self.device)?;
        self.history = self
            .recent_servers
            .iter()
            .map(|s| s.origin.clone())
            .collect();
        self.preferences = Some(preferences);
        Ok(())
    }
    fn clear_game(&mut self, pending: &Mutex<Option<handoff::Pending>>) {
        if self.revision == G2_REV {
            if let Some(key) = self.selected_run_key() {
                self.remember_armed(key);
            }
        }
        *pending.lock().unwrap() = None;
        self.game = None;
        self.attach_retry = None;
    }
    fn persist(&self) -> Result<(), String> {
        if self.remember {
            let lease = self
                .resources
                .task()
                .map_err(|_| "Client scope unavailable")?;
            let bytes =
                serde_json::to_vec(&self.journal).map_err(|_| "Session storage encoding failed")?;
            let blob = SecretBlob::new(bytes).map_err(|e| e.to_string())?;
            self.resources
                .remember(&lease, &blob)
                .map_err(|_| "Remembered session update failed; retry before closing the app")?;
        }
        Ok(())
    }
    async fn bytes(&self, builder: reqwest::RequestBuilder) -> Result<Zeroizing<Vec<u8>>, String> {
        let mut response = builder
            .send()
            .await
            .map_err(|_| "Service unavailable; retry remains bounded")?;
        let mut bytes = Zeroizing::new(Vec::new());
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| "Response interrupted; retry remains bounded")?
        {
            if bytes.len() + chunk.len() > 65536 {
                return Err("Oversized service response".into());
            }
            bytes.extend_from_slice(&chunk);
        }
        Ok(bytes)
    }
    async fn info(&mut self) -> Result<(), String> {
        let generation = self.generation.load(Ordering::Acquire);
        let lease = self
            .resources
            .task()
            .map_err(|_| "Client scope unavailable")?;
        let bytes = self
            .bytes(self.http.get(format!("{}/v1/info", self.base)))
            .await?;
        let info: wire::Info = self
            .typed("Info", &bytes)
            .map_err(|_| "Invalid service information")?;
        if self.generation.load(Ordering::Acquire) != generation {
            return Err("Client action cancelled".into());
        }
        self.resources
            .require_current(&lease)
            .map_err(|_| "Cancelled client scope")?;
        if info.major != Some(1)
            || info.revision.as_deref() != Some(self.revision)
            || info.service_id.as_deref() != Some(self.service.as_str())
        {
            return Err("Wrong service or protocol revision".into());
        }
        self.epoch = text(info.service_epoch.clone(), 128)?;
        self.info = Some(info);
        Ok(())
    }
    fn request(&self, game: bool) -> wire::Request {
        let s = if game {
            self.game.as_ref().map(|g| &g.session)
        } else {
            self.desktop.as_ref()
        };
        let r = self.room.as_ref();
        wire::Request {
            major: Some(1),
            revision: Some(self.revision.into()),
            service_id: Some(self.service.clone()),
            request_id: Some(uuid()),
            service_epoch: Some(self.epoch.clone()),
            session_epoch: s.map(|s| s.epoch.clone()),
            room_id: r.and_then(|r| r.room_id.clone()),
            expected_revision: r.and_then(|r| r.revision),
            config_revision: r.and_then(|r| r.config_revision),
            run_id: r.map(|r| r.run_id.clone().unwrap_or_default()),
            run_epoch: r.map(|r| r.run_epoch.clone().unwrap_or_default()),
            ..Default::default()
        }
    }
    async fn send(
        &mut self,
        request: &wire::Request,
        token: Option<&str>,
    ) -> Result<wire::Response, String> {
        let generation = self.generation.load(Ordering::Acquire);
        let bytes = Zeroizing::new(serde_json::to_vec(request).map_err(|_| "Invalid request")?);
        self.validate_request(&bytes)?;
        let lease = self
            .resources
            .task()
            .map_err(|_| "Client scope unavailable")?;
        let mut builder = self
            .http
            .post(format!("{}/v1/rpc", self.base))
            .header("content-type", "application/json")
            .body(bytes.to_vec());
        if let Some(token) = token {
            builder = builder.bearer_auth(token);
        }
        let bytes = self.bytes(builder).await?;
        if self.generation.load(Ordering::Acquire) != generation {
            return Err("Client action cancelled".into());
        }
        let result: wire::Response = if self.revision == REV {
            protocol::response_as(&bytes).map_err(|_| "Invalid bounded service response")?
        } else {
            protocol_g2::response(&bytes).map_err(|_| "Invalid bounded service response")?
        };
        self.resources
            .require_current(&lease)
            .map_err(|_| "Cancelled client scope")?;
        if result.service_id != request.service_id || result.request_id != request.request_id {
            return Err("Response identity mismatch".into());
        }
        let epoch = text(result.service_epoch.clone(), 128)?;
        if self.epoch != epoch {
            self.epoch = epoch;
            self.game = None;
            self.attach_retry = None;
            self.room = None;
            if request.refresh.is_none()
                && request.pair_exchange.is_none()
                && request.logout.is_none()
            {
                return Err("Service restarted; game attachment invalidated; restore waiting room explicitly".into());
            }
        }
        if let Some(e) = result.error {
            if let Some(ms) = e.retry_after_ms {
                self.retry_until = Instant::now() + Duration::from_millis(u64::from(ms));
            }
            let code = e.code.unwrap_or_default();
            if ["UNAUTHENTICATED", "REFRESH_REUSED", "EXPIRED"].contains(&code.as_str())
                && (request.refresh.is_some()
                    || self
                        .desktop
                        .as_ref()
                        .is_some_and(|s| request.session_epoch.as_deref() == Some(&s.epoch)))
            {
                self.desktop = None;
                self.game = None;
                self.attach_retry = None;
                self.room = None;
                self.journal = Journal::new(self.device.to_string());
                self.persist()?;
            }
            if ["CANCELLED", "RESTART_REQUIRED", "STALE_EPOCH"].contains(&code.as_str())
                || code == "UNAUTHENTICATED" && request.game_resume.is_some()
            {
                self.game = None;
                self.attach_retry = None;
            }
            return Err(
                if [
                    "UNAUTHENTICATED",
                    "REFRESH_REUSED",
                    "EXPIRED",
                    "CANCELLED",
                    "RESTART_REQUIRED",
                    "STALE_EPOCH",
                    "STALE_REVISION",
                    "FORBIDDEN",
                    "PROVIDER_UNAVAILABLE",
                    "NOT_READY",
                    "ROSTER_MISMATCH",
                    "INVALID_STATE",
                    "INCOMPATIBLE_GAME",
                    "CAPACITY",
                    "SERVICE_UNAVAILABLE",
                    "PASSWORD_REQUIRED",
                    "CONFLICT",
                    "ALREADY_EXCHANGED",
                    "NOT_FOUND",
                ]
                .contains(&code.as_str())
                {
                    code
                } else {
                    "Service rejected request".into()
                },
            );
        }
        Ok(result)
    }
    async fn desktop_rpc(&mut self, r: wire::Request) -> Result<wire::Response, String> {
        let token = SecretString::from(self.desktop.as_ref().ok_or("Pair first")?.access.as_str());
        let response = self.send(&r, Some(token.as_str())).await?;
        if let Some(room) = response.room.clone() {
            self.room = Some(room)
        }
        Ok(response)
    }
    async fn select(&mut self, server: &str, service: &str, remember: bool) -> Result<(), String> {
        if self.desktop.is_some() || !self.journal.attempt.is_empty() {
            return Err("Log out or cancel pairing before changing profile".into());
        }
        let profile =
            ServiceProfile::new(server, service, self.device).map_err(|e| e.to_string())?;
        self.journal = Journal::new(self.device.to_string());
        self.browser = None;
        self.game = None;
        self.attach_retry = None;
        self.room = None;
        self.rooms.clear();
        self.chat_room.clear();
        self.chat_sequence = 0;
        self.chat_messages.clear();
        self.room_cursor.clear();
        self.next_room_id = None;
        self.status_error = None;
        self.info = None;
        self.base = profile.origin().into();
        self.service = service.into();
        self.resources.select(profile);
        self.remember = remember;
        self.info().await?;
        if !self.history.contains(&self.base) {
            self.history.push(self.base.clone());
            if self.history.len() > 16 {
                self.history.remove(0);
            }
        }
        self.recent_servers.retain(|s| s.origin != self.base);
        self.recent_servers.insert(
            0,
            RecentServer {
                remembered_name:None,has_saved_account:None,
                origin: self.base.clone(),
                service_id: self.service.clone(),
            },
        );
        self.recent_servers.truncate(16);
        self.history = self
            .recent_servers
            .iter()
            .map(|s| s.origin.clone())
            .collect();
        if let Some(preferences) = &self.preferences {
            preferences.save(&self.recent_servers, self.device)?;
        }
        if remember {
            if let Some(blob) = self
                .resources
                .remembered()
                .map_err(|_| "Remembered session unavailable")?
            {
                self.journal = serde_json::from_slice(blob.bytes())
                    .map_err(|_| "Invalid remembered session")?;
                self.journal
                    .validate(&self.device.to_string(), self.revision)?;
                if !self.journal.operation.is_empty() {
                    self.recover().await?;
                } else if !self.journal.refresh.is_empty() {
                    self.rotate().await?;
                }
            }
        }
        if self.desktop.is_some() {
            return Ok(());
        }
        if !self.journal.attempt.is_empty() {
            if now() < self.journal.expires {
                return Ok(());
            }
            self.journal = Journal::new(self.device.to_string());
            self.persist()?;
        }
        let mut req = self.request(false);
        req.pair_begin = Some(wire::PairBegin {
            device_id: Some(self.device.to_string()),
        });
        let mut p = self
            .send(&req, None)
            .await?
            .pairing
            .ok_or("No pairing response")?;
        self.journal.attempt = text(p.attempt_id.take(), 128)?;
        self.journal.proof = text(p.proof.take(), 256)?;
        self.journal.code = text(p.code.take(), 16)?;
        self.journal.expires = timestamp(&text(p.expires_at.take(), 40)?)?;
        self.persist()
    }
    fn proof(&self) -> wire::PairProof {
        wire::PairProof {
            attempt_id: Some(self.journal.attempt.clone()),
            proof: Some(self.journal.proof.clone()),
            device_id: Some(self.device.to_string()),
        }
    }
    async fn mock_approve(&mut self, subject: &str) -> Result<(), String> {
        if self.info.as_ref().and_then(|i| i.mock_provider) != Some(true)
            || !["host", "guest", "observer"].contains(&subject)
        {
            return Err("Mock approval unavailable".into());
        }
        let payload = wire::BrowserLogin {
            subject: Some(subject.into()),
            device_id: Some(format!("browser-{}", self.device)),
            request_id: Some(uuid()),
        };
        let bytes = self
            .bytes(
                self.http
                    .post(format!("{}/v1/mock/browser-login", self.base))
                    .json(&payload),
            )
            .await?;
        let result: wire::BrowserLoginResponse = self
            .typed("BrowserLoginResponse", &bytes)
            .map_err(|_| "Invalid mock browser response")?;
        self.browser = Some(session(
            result.credentials.ok_or("Mock provider unavailable")?,
            "browser",
            payload.device_id.as_deref().unwrap(),
        )?);
        let mut r = self.request(false);
        r.session_epoch = Some(self.browser.as_ref().unwrap().epoch.clone());
        r.pair_approve = Some(wire::PairApprove {
            attempt_id: Some(self.journal.attempt.clone()),
            code: Some(self.journal.code.clone()),
        });
        let token = SecretString::from(self.browser.as_ref().unwrap().access.as_str());
        self.send(&r, Some(token.as_str())).await?;
        self.exchange().await
    }
    async fn poll(&mut self) -> Result<(), String> {
        if self.journal.attempt.is_empty() {
            return Ok(());
        }
        if now() >= self.journal.expires && self.journal.operation.is_empty() {
            self.journal = Journal::new(self.device.to_string());
            self.persist()?;
            return Err("Pairing expired".into());
        }
        if !self.journal.operation.is_empty() {
            return self.recover().await;
        }
        let mut r = self.request(false);
        r.pair_poll = Some(self.proof());
        let p = self
            .send(&r, None)
            .await?
            .pairing
            .ok_or("Invalid pairing state")?;
        match p.state.as_deref() {
            Some("approved") => self.exchange().await,
            Some("pending") => Ok(()),
            _ => Err("Pairing unusable; cancel and pair again".into()),
        }
    }
    async fn exchange(&mut self) -> Result<(), String> {
        self.journal.operation = "exchange".into();
        self.journal.operation_revision = self.revision.into();
        self.journal.request_id = uuid();
        self.journal.started = now();
        self.persist()?;
        self.recover().await
    }
    async fn rotate(&mut self) -> Result<(), String> {
        if self.journal.refresh.is_empty() {
            return Err("Pair again".into());
        }
        if self.journal.operation.is_empty() {
            self.journal.operation = "refresh".into();
            self.journal.operation_revision = self.revision.into();
            self.journal.request_id = uuid();
            self.journal.started = now();
            self.persist()?;
        }
        self.recover().await
    }
    async fn recover(&mut self) -> Result<(), String> {
        if self.journal.operation_revision != self.revision {
            self.desktop = None;
            return Err("Recovery protocol changed; log out and pair again".into());
        }
        if now().saturating_sub(self.journal.started) >= 30000 {
            self.desktop = None;
            return Err("Credential recovery window expired; log out and pair again".into());
        }
        let mut req = wire::Request {
            major: Some(1),
            revision: Some(self.revision.into()),
            service_id: Some(self.service.clone()),
            request_id: Some(self.journal.request_id.clone()),
            ..Default::default()
        };
        match self.journal.operation.as_str() {
            "exchange" => req.pair_exchange = Some(self.proof()),
            "refresh" => {
                req.refresh = Some(wire::Refresh {
                    secret: Some(self.journal.refresh.clone()),
                    device_id: Some(self.device.to_string()),
                })
            }
            _ => return Err("Invalid recovery operation".into()),
        }
        let credentials = self
            .send(&req, None)
            .await?
            .credentials
            .ok_or("No desktop credentials")?;
        let next = session(credentials, "desktop", &self.device.to_string())?;
        // Commit rotated recovery material to OS storage before adopting access.
        let old = std::mem::replace(&mut self.journal, Journal::new(self.device.to_string()));
        self.journal.refresh = next.refresh.as_str().into();
        if let Err(e) = self.persist() {
            self.journal = old;
            return Err(e);
        }
        self.desktop = Some(next);
        Ok(())
    }
    async fn list(&mut self) -> Result<(), String> {
        let mut r = self.request(false);
        r.room_list = Some(wire::ListRooms {
            limit: Some(32),
            after_room_id: (!self.room_cursor.is_empty()).then(|| self.room_cursor.clone()),
        });
        let response = self.desktop_rpc(r).await?;
        self.rooms = response.rooms.unwrap_or_default();
        self.next_room_id = response.next_room_id.filter(|id| !id.is_empty());
        Ok(())
    }
    async fn room_page(&mut self, next: bool) -> Result<(), String> {
        let cursor = if next {
            self.next_room_id.clone().ok_or("No next room page")?
        } else {
            String::new()
        };
        if !cursor.is_empty() && uuid::Uuid::parse_str(&cursor).is_err() {
            return Err("Invalid room cursor".into());
        }
        let old = std::mem::replace(&mut self.room_cursor, cursor);
        if let Err(error) = self.list().await {
            self.room_cursor = old;
            return Err(error);
        }
        Ok(())
    }
    async fn fixture_room(&mut self) -> Result<(), String> {
        if self.info.as_ref().and_then(|i| i.mock_provider) != Some(true) {
            return Err("Fixture action unavailable".into());
        }
        let browser = self
            .browser
            .as_ref()
            .ok_or("Approve with mock browser first")?;
        let token = SecretString::from(browser.access.as_str());
        let mut r = self.request(false);
        r.session_epoch = Some(browser.epoch.clone());
        r.room_create = Some(wire::RoomCreate {
            name: Some(format!(
                "G1 client fixture {}",
                &self.device.to_string()[..8]
            )),
            visibility: Some("public".into()),
            config: Some(wire::Config {
                seed: Some(424242),
                mode: Some("core".into()),
                settings: None,
                required_capabilities: Some(vec!["core.seed".into()]),
            }),
        });
        let room = self
            .send(&r, Some(token.as_str()))
            .await?
            .room
            .ok_or("No fixture room")?;
        self.room = Some(room);
        Ok(())
    }
    async fn preview(&mut self, room: &str) -> Result<wire::RoomSummary, String> {
        if uuid::Uuid::parse_str(room).is_err() {
            return Err("Invalid room ID".into());
        }
        let mut r = self.request(false);
        r.room_id = Some(room.into());
        r.expected_revision = None;
        r.config_revision = None;
        r.run_id = None;
        r.run_epoch = None;
        r.room_preview = Some(wire::Empty {});
        let admission = self
            .desktop_rpc(r)
            .await?
            .admission
            .ok_or("No room admission preview")?;
        if admission.room_id.as_deref() != Some(room) {
            return Err("Wrong room admission preview".into());
        }
        self.rooms.retain(|r| r.room_id.as_deref() != Some(room));
        if self.rooms.len() >= 32 {
            self.rooms.pop();
        }
        self.rooms.push(admission.clone());
        Ok(admission)
    }
    async fn join_admission(
        &mut self,
        admission: wire::RoomSummary,
        password: Option<String>,
    ) -> Result<(), String> {
        let room = admission.room_id.clone().ok_or("No admission room")?;
        if admission.join_allowed != Some(true) {
            return Err("Room is not accepting this desktop".into());
        }
        let mut r = self.request(false);
        r.room_id = Some(room.clone());
        r.expected_revision = admission.revision;
        r.config_revision = admission.config_revision;
        r.run_id = None;
        r.run_epoch = None;
        r.room_join = Some(wire::RoomJoin { password });
        if let Err(e) = self.desktop_rpc(r).await {
            if e == "STALE_REVISION" {
                self.preview(&room).await?;
                return Err("Room changed; review the updated listing and join again".into());
            }
            return Err(e);
        }
        let mut r = self.request(false);
        r.presence = Some(wire::Presence {
            present: Some(true),
        });
        self.desktop_rpc(r).await?;
        if self.revision == REV {
            self.set_ready(true).await?;
        }
        self.last_presence = Instant::now();
        Ok(())
    }
    #[cfg(test)]
    async fn join(&mut self, room: &str) -> Result<(), String> {
        let admission = self.preview(room).await?;
        self.join_admission(admission, None).await
    }
    async fn join_with_password(
        &mut self,
        room: &str,
        password: Option<String>,
    ) -> Result<(), String> {
        if password.as_ref().is_some_and(|p| p.len() > 256) {
            return Err("Password is too long".into());
        }
        let admission = self.preview(room).await?;
        self.join_admission(admission, password).await
    }
    async fn set_ready(&mut self, ready: bool) -> Result<(), String> {
        let mut r = self.request(false);
        r.ready = Some(wire::Ready { ready: Some(ready) });
        self.desktop_rpc(r).await?;
        Ok(())
    }
    async fn chat_list(&mut self) -> Result<(), String> {
        if self.revision != G2_REV || self.room.is_none() {
            return Ok(());
        }
        let room = self
            .room
            .as_ref()
            .unwrap()
            .room_id
            .clone()
            .ok_or("No room ID")?;
        if self.chat_room != room {
            self.chat_room = room;
            self.chat_sequence = 0;
            self.chat_messages.clear();
        }
        let mut r = self.request(false);
        r.chat_list = Some(wire::ChatList {
            after_sequence: Some(self.chat_sequence),
            limit: Some(32),
        });
        let result = self.desktop_rpc(r).await?;
        if result.chat_reset == Some(true) {
            self.chat_sequence = 0;
            self.chat_messages.clear();
        }
        for message in result.chat_messages.unwrap_or_default() {
            let sequence = message.sequence.ok_or("Invalid chat sequence")?;
            if sequence > self.chat_sequence {
                self.chat_sequence = sequence;
                self.chat_messages.push(message);
            }
        }
        if let Some(next) = result.chat_next_sequence {
            if next < self.chat_sequence {
                return Err("Invalid chat cursor".into());
            }
            self.chat_sequence = next;
        }
        if self.chat_messages.len() > 128 {
            self.chat_messages.drain(..self.chat_messages.len() - 128);
        }
        Ok(())
    }
    async fn chat_send(&mut self, text: String) -> Result<(), String> {
        if self.revision != G2_REV {
            return Err("Chat is unavailable for this protocol".into());
        }
        if text.trim().is_empty() || text.len() > 1024 {
            return Err("Enter a message up to1024 UTF-8 bytes".into());
        }
        if self.chat_retry.is_some() {
            return Err("Previous chat delivery is still being recovered".into());
        }
        let mut r = self.request(false);
        r.chat_send = Some(wire::ChatSend { text: Some(text) });
        self.chat_retry = Some(ChatRetry {
            request: r,
            desktop: self.desktop.as_ref().ok_or("Sign in first")?.id.clone(),
            generation: self.generation.load(Ordering::Acquire),
            deadline: Instant::now() + Duration::from_secs(30),
        });
        self.retry_chat().await?;
        self.chat_list().await
    }
    async fn retry_chat(&mut self) -> Result<(), String> {
        let Some(retry) = self.chat_retry.as_ref() else {
            return Ok(());
        };
        if Instant::now() >= retry.deadline
            || self.generation.load(Ordering::Acquire) != retry.generation
            || self.epoch != retry.request.service_epoch.as_deref().unwrap_or("")
            || self.desktop.as_ref().is_none_or(|d| {
                d.id != retry.desktop
                    || Some(d.epoch.as_str()) != retry.request.session_epoch.as_deref()
            })
            || self.room.as_ref().is_none_or(|r| {
                r.room_id != retry.request.room_id
                    || r.config_revision != retry.request.config_revision
            })
        {
            self.chat_retry = None;
            return Err("Chat recovery scope expired; review the room before sending again".into());
        }
        let request = retry.request.clone();
        let result = self.desktop_rpc(request).await;
        if result.is_ok()
            || result.as_ref().is_err_and(|e| {
                !e.starts_with("Service unavailable") && !e.starts_with("Response interrupted")
            })
        {
            self.chat_retry = None;
        }
        result.map(|_| ())
    }
    async fn start(&mut self, selected: Vec<String>, excluded: Vec<String>) -> Result<(), String> {
        let mut r = self.request(false);
        r.start = Some(wire::Start {
            participant_desktop_ids: Some(selected),
            excluded_desktop_ids: Some(excluded),
        });
        self.desktop_rpc(r).await?;
        Ok(())
    }
    fn selected_run_key(&self) -> Option<String> {
        let desktop = self.desktop.as_ref()?;
        let room = self.room.as_ref()?;
        if !["preparing", "loading"].contains(&room.state.as_deref()?)
            || !room.members.as_ref()?.iter().any(|m| {
                m.desktop_id.as_deref() == Some(&desktop.id)
                    && m.selected == Some(true)
                    && m.loaded != Some(true)
            })
        {
            return None;
        }
        Some(
            serde_json::to_string(&(
                &self.base,
                &self.service,
                &self.epoch,
                &desktop.id,
                &desktop.epoch,
                room.room_id.as_ref()?,
                room.run_id.as_ref()?,
                room.run_epoch.as_ref()?,
                room.config_revision?,
            ))
            .ok()?,
        )
    }
    fn remember_armed(&mut self, key: String) {
        if !self.armed_runs.contains(&key) {
            self.armed_runs.push_back(key);
        }
        while self.armed_runs.len() > 32 {
            self.armed_runs.pop_front();
        }
    }
    fn auto_arm(&mut self, pending: &Mutex<Option<handoff::Pending>>) -> Result<(), String> {
        if self.revision != G2_REV || pending.lock().unwrap().is_some() {
            return Ok(());
        }
        let Some(key) = self.selected_run_key() else {
            return Ok(());
        };
        if self.armed_runs.contains(&key) {
            return Ok(());
        }
        let generation = self.generation.load(Ordering::Acquire);
        let handoff = self.preparation()?;
        if self.generation.load(Ordering::Acquire) != generation {
            return Err("Preparation cancelled".into());
        }
        let mut local = pending.lock().unwrap();
        if self.generation.load(Ordering::Acquire) != generation {
            return Err("Preparation cancelled".into());
        }
        if local.is_none() {
            self.remember_armed(key);
            *local = Some(handoff);
        }
        Ok(())
    }
    fn preparation(&self) -> Result<handoff::Pending, String> {
        let r = self.room.as_ref().ok_or("No room")?;
        if !["preparing", "loading"].contains(&r.state.as_deref().unwrap_or("")) {
            return Err("No active preparation".into());
        }
        let desktop = self.desktop.as_ref().ok_or("No desktop session")?;
        if !r.members.as_ref().is_some_and(|members| {
            members.iter().any(|m| {
                m.desktop_id.as_deref() == Some(&desktop.id)
                    && m.selected == Some(true)
                    && m.loaded != Some(true)
            })
        }) {
            return Err("This desktop is not selected for fresh loading".into());
        }
        let config = r.config.as_ref().ok_or("No locked configuration")?;
        let compatibility = self
            .info
            .as_ref()
            .and_then(|i| i.compatibility.as_ref())
            .ok_or("No compatibility information")?;
        // Never advertise simulated compatibility as a real game's supported version.
        if compatibility.game_version.as_deref() != Some("Noita-Jan25-2025")
            || compatibility.mod_version.as_deref()
                != Some(if self.revision == G2_REV {
                    "nt2-g2-1"
                } else {
                    "nt2-g1-1"
                })
            || compatibility.bridge_version.as_deref()
                != Some(if self.revision == G2_REV {
                    "g2-pipe32-1"
                } else {
                    "g1-pipe32-2"
                })
        {
            return Err("Service compatibility is simulated; native provisioning is unavailable on this profile".into());
        }
        if config.mode.as_deref() != Some("core")
            || config.settings.as_ref().is_some_and(|s| !s.is_empty())
            || config
                .required_capabilities
                .as_ref()
                .is_some_and(|c| c.iter().any(|s| s != "core.seed"))
        {
            return Err("This bridge supports seed provisioning only; requested gameplay capabilities are not installed".into());
        }
        if self.revision == G2_REV {
            let root = self
                .game_path
                .as_ref()
                .ok_or("Configure the matched Noita installation first")?;
            crate::installation::verify(
                root,
                Some(
                    self.info
                        .as_ref()
                        .and_then(|i| i.profile.as_ref())
                        .ok_or("Service native profile missing")?,
                ),
            )?;
        } else {
            installed_profile()?;
        }
        let duration = timestamp(r.preparation_expires_at.as_deref().ok_or("No deadline")?)?
            .checked_sub(now())
            .filter(|n| *n > 0 && *n <= 120000)
            .ok_or("Preparation expired")?;
        let mut p = handoff::new(
            config.seed.ok_or("No seed")?,
            text(r.run_id.clone(), 128)?,
            r.config_revision.ok_or("No config revision")? as u64,
        );
        p.epoch = text(r.run_epoch.clone(), 128)?;
        p.expires = Instant::now() + Duration::from_millis(duration);
        if self.revision == G2_REV {
            use sha2::{Digest, Sha256};
            let key = self.selected_run_key().ok_or("No selected run")?;
            let name: String = Sha256::digest(key.as_bytes())
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect();
            p.claim_path = Some(
                self.claims_root
                    .as_ref()
                    .ok_or("Durable world guard unavailable")?
                    .join(name),
            );
            if p.claim_path.as_ref().unwrap().exists() {
                return Err("This world was already consumed; request a new run".into());
            }
            p.native_v2 = true;
            p.generation = Some((
                self.generation.clone(),
                self.generation.load(Ordering::Acquire),
            ));
            p.authority_until = Instant::now() + Duration::from_secs(12);
        }
        Ok(p)
    }
    fn attach_scope(&self, p: &handoff::Pending) -> Result<AttachScope, String> {
        let desktop = self.desktop.as_ref().ok_or("RESTART_REQUIRED")?;
        let room = self.room.as_ref().ok_or("RESTART_REQUIRED")?;
        let owner = p.owner.ok_or("RESTART_REQUIRED")?;
        if Instant::now() >= p.expires
            || p.last_seen
                .is_none_or(|seen| seen.elapsed() > Duration::from_secs(15))
            || room.run_id.as_deref() != Some(&p.run)
            || room.run_epoch.as_deref() != Some(&p.epoch)
            || room.config_revision.map(u64::from) != Some(p.config)
            || !["preparing", "loading"].contains(&room.state.as_deref().unwrap_or(""))
            || !room.members.as_ref().is_some_and(|members| {
                members.iter().any(|m| {
                    m.desktop_id.as_deref() == Some(&desktop.id) && m.selected == Some(true)
                })
            })
        {
            return Err("RESTART_REQUIRED".into());
        }
        Ok(AttachScope {
            generation: self.generation.load(Ordering::Acquire),
            base: self.base.clone(),
            service: self.service.clone(),
            service_epoch: self.epoch.clone(),
            desktop_id: desktop.id.clone(),
            desktop_epoch: desktop.epoch.clone(),
            desktop_account: desktop.account.clone(),
            desktop_device: desktop.device.clone(),
            room: room.room_id.clone().ok_or("RESTART_REQUIRED")?,
            run: p.run.clone(),
            run_epoch: p.epoch.clone(),
            config: p.config,
            owner,
        })
    }
    async fn attach(
        &mut self,
        p: &handoff::Pending,
        pending: &Mutex<Option<handoff::Pending>>,
        compatibility: wire::Compatibility,
    ) -> Result<(), String> {
        let scope = self.attach_scope(p)?;
        if pending
            .lock()
            .unwrap()
            .as_ref()
            .and_then(|p| self.attach_scope(p).ok())
            .as_ref()
            != Some(&scope)
        {
            self.clear_game(pending);
            return Err("RESTART_REQUIRED".into());
        }
        let instance = format!(
            "game-{}-{}",
            scope.owner.0,
            scope
                .owner
                .1
                .iter()
                .map(|n| format!("{n:02x}"))
                .collect::<String>()
        );
        if let Some(retry) = &self.attach_retry {
            if retry.scope != scope || Instant::now() >= retry.deadline {
                self.clear_game(pending);
                return Err("RESTART_REQUIRED".into());
            }
        } else {
            let mut r = self.request(false);
            r.game_attach = Some(wire::GameAttach {
                game_instance_id: Some(instance.clone()),
                game_version: compatibility.game_version,
                mod_version: compatibility.mod_version,
                bridge_version: compatibility.bridge_version,
                capabilities: Some(vec!["core.seed".into()]),
            });
            self.attach_retry = Some(AttachRetry {
                request: r,
                scope: scope.clone(),
                deadline: (Instant::now() + Duration::from_secs(30)).min(p.expires),
            });
        }
        let retry = self.attach_retry.as_ref().unwrap();
        let r = retry.request.clone();
        // The body/ID/deadline remain original. Refresh can revoke an access token without
        // changing this fenced desktop identity/epoch, so authenticate with its current token.
        let token = SecretString::from(self.desktop.as_ref().unwrap().access.as_str());
        // The core mutex serializes refresh and recovery; a different scope was rejected above.
        let result = self.send(&r, Some(token.as_str())).await?;
        let current = pending.lock().unwrap().clone();
        if current
            .as_ref()
            .and_then(|p| self.attach_scope(p).ok())
            .as_ref()
            != Some(&scope)
            || self
                .attach_retry
                .as_ref()
                .is_none_or(|retry| Instant::now() >= retry.deadline)
        {
            self.clear_game(pending);
            return Err("RESTART_REQUIRED".into());
        }
        let room = result.room.ok_or("No attachment room")?;
        if room.room_id.as_deref() != Some(&scope.room)
            || room.run_id.as_deref() != Some(&scope.run)
            || room.run_epoch.as_deref() != Some(&scope.run_epoch)
            || room.config_revision.map(u64::from) != Some(scope.config)
            || !["preparing", "loading"].contains(&room.state.as_deref().unwrap_or(""))
        {
            self.clear_game(pending);
            return Err("RESTART_REQUIRED".into());
        }
        let attached = session(
            result.credentials.ok_or("No game credential")?,
            "game",
            &self.device.to_string(),
        )?;
        let desktop = self.desktop.as_ref().ok_or("Desktop scope lost")?;
        if attached.account != desktop.account || attached.device != desktop.device {
            self.clear_game(pending);
            return Err("Wrong game credential account/device".into());
        }
        self.game = Some(Game {
            session: attached,
            instance,
            owner: scope.owner,
            last_resume: Instant::now(),
            retry: None,
        });
        self.room = Some(room);
        self.attach_retry = None;
        Ok(())
    }
    async fn fact(&mut self, loaded: bool) -> Result<(), String> {
        let g = self.game.as_ref().ok_or("No attached game")?;
        let token = SecretString::from(g.session.access.as_str());
        let fact = wire::GameFact {
            attachment_id: Some(g.session.attachment.clone()),
            game_instance_id: Some(g.instance.clone()),
        };
        let mut r = self.request(true);
        if loaded {
            r.game_loaded = Some(fact)
        } else {
            r.game_seed_applied = Some(fact)
        }
        let result = self.send(&r, Some(token.as_str())).await?;
        self.room = result.room;
        Ok(())
    }
    async fn resume(&mut self) -> Result<(), String> {
        let g = self.game.as_ref().ok_or("No attached game")?;
        let token = SecretString::from(g.session.access.as_str());
        let retry = g.retry.clone();
        let (r, started) = if let Some(retry) = retry {
            retry
        } else {
            let mut r = self.request(true);
            r.game_resume = Some(wire::GameFact {
                attachment_id: Some(g.session.attachment.clone()),
                game_instance_id: Some(g.instance.clone()),
            });
            (r, now())
        };
        if now().saturating_sub(started) >= 30000 {
            self.game = None;
            self.attach_retry = None;
            return Err("Game recovery window expired; restart required".into());
        }
        self.game.as_mut().unwrap().retry = Some((r.clone(), started));
        let result = self.send(&r, Some(token.as_str())).await?;
        let next = session(
            result.credentials.ok_or("No resumed game credential")?,
            "game",
            &self.device.to_string(),
        )?;
        let g = self.game.as_mut().ok_or("Game scope cancelled")?;
        if next.id != g.session.id || next.attachment != g.session.attachment {
            return Err("Resumed wrong game context".into());
        }
        g.session = next;
        g.retry = None;
        g.last_resume = Instant::now();
        self.room = result.room;
        Ok(())
    }
    async fn sync(&mut self, pending: &Mutex<Option<handoff::Pending>>) -> Result<(), String> {
        let result = self.sync_inner(pending).await;
        if self.revision == G2_REV {
            if let Some(p) = pending.lock().unwrap().as_mut() {
                p.release = result.is_ok()
                    && p.accepted
                    && p.loaded
                    && self.game.is_some()
                    && self.room.as_ref().is_some_and(|r| {
                        r.state.as_deref() == Some("running")
                            && r.run_id.as_deref() == Some(&p.run)
                            && r.run_epoch.as_deref() == Some(&p.epoch)
                            && r.config_revision.map(u64::from) == Some(p.config)
                    });
                if result.is_ok() {
                    p.authority_until = Instant::now() + Duration::from_secs(12);
                }
            }
        }
        if self.desktop.is_none()
            || result.as_ref().is_err_and(|e| {
                e.contains("Service restarted")
                    || [
                        "CANCELLED",
                        "RESTART_REQUIRED",
                        "STALE_EPOCH",
                        "REFRESH_REUSED",
                        "UNAUTHENTICATED",
                    ]
                    .contains(&e.as_str())
            })
        {
            self.clear_game(pending);
        }
        result
    }
    async fn sync_inner(
        &mut self,
        pending: &Mutex<Option<handoff::Pending>>,
    ) -> Result<(), String> {
        let generation = self.generation.load(Ordering::Acquire);
        self.poll().await?;
        if self.desktop.is_none() {
            return Ok(());
        }
        if !self.journal.operation.is_empty()
            || self.desktop.as_ref().unwrap().expires < now() + 60000
        {
            self.rotate().await?;
        }
        if self.room.is_none() {
            return self.list().await;
        }
        let mut r = self.request(false);
        r.room_get = Some(wire::Empty {});
        if let Err(e) = self.desktop_rpc(r).await {
            if e.contains("Service restarted") {
                self.clear_game(pending);
            }
            if e == "UNAUTHENTICATED" || e == "REFRESH_REUSED" {
                self.clear_game(pending);
                self.desktop = None;
            }
            return Err(e);
        }
        let joined = self.desktop.as_ref().is_some_and(|desktop| {
            self.room
                .as_ref()
                .and_then(|r| r.members.as_ref())
                .is_some_and(|members| {
                    members
                        .iter()
                        .any(|m| m.desktop_id.as_deref() == Some(&desktop.id))
                })
        });
        if joined && self.last_presence.elapsed() >= Duration::from_secs(10) {
            let mut r = self.request(false);
            r.presence = Some(wire::Presence {
                present: Some(true),
            });
            self.desktop_rpc(r).await?;
            self.last_presence = Instant::now();
        }
        if self.generation.load(Ordering::Acquire) != generation {
            return Err("Client action cancelled".into());
        }
        if let Err(error) = self.auto_arm(pending) {
            if self.revision == G2_REV && self.generation.load(Ordering::Acquire) == generation {
                self.clear_game(pending);
                let mut r = self.request(false);
                r.presence = Some(wire::Presence {
                    present: Some(false),
                });
                let _ = self.desktop_rpc(r).await;
            }
            return Err(error);
        }
        let local = pending.lock().unwrap().clone();
        if let Some(p) = local {
            let room = self.room.as_ref().ok_or("No room")?;
            if room.run_id.as_deref() != Some(&p.run)
                || room.run_epoch.as_deref() != Some(&p.epoch)
                || room.config_revision.map(u64::from) != Some(p.config)
                || !["preparing", "loading", "running", "suspended"]
                    .contains(&room.state.as_deref().unwrap_or(""))
                || (!p.loaded && Instant::now() >= p.expires)
            {
                self.clear_game(pending);
                return Ok(());
            }
            if self.attach_retry.as_ref().is_some_and(|retry| {
                self.attach_scope(&p).as_ref().ok() != Some(&retry.scope)
                    || Instant::now() >= retry.deadline
            }) {
                self.clear_game(pending);
                return Err("RESTART_REQUIRED".into());
            }
            if let Some(g) = &self.game {
                if p.owner != Some(g.owner)
                    || p.last_seen
                        .is_none_or(|t| t.elapsed() > Duration::from_secs(15))
                {
                    self.clear_game(pending);
                    return Err("Native context lost; game liveness stopped".into());
                }
            }
            if p.owner.is_some() && self.game.is_none() {
                let compatibility = self
                    .info
                    .as_ref()
                    .and_then(|i| i.compatibility.clone())
                    .ok_or("No compatibility")?;
                self.attach(&p, pending, compatibility).await?;
            }
            let me = self.desktop.as_ref().unwrap().id.clone();
            let member = self
                .room
                .as_ref()
                .and_then(|r| r.members.as_ref())
                .and_then(|ms| ms.iter().find(|m| m.desktop_id.as_deref() == Some(&me)));
            let seed_sent = member.and_then(|m| m.seed_applied) == Some(true);
            let loaded_sent = member.and_then(|m| m.loaded) == Some(true);
            if p.accepted && !seed_sent {
                self.fact(false).await?
            }
            if p.loaded
                && !loaded_sent
                && self.room.as_ref().and_then(|r| r.state.as_deref()) == Some("loading")
            {
                self.fact(true).await?
            }
            if self.game.as_ref().is_some_and(|g| {
                g.retry.is_some() || g.last_resume.elapsed() >= Duration::from_secs(10)
            }) {
                self.resume().await?
            }
        } else {
            if self.revision == G2_REV
                && (self.game.is_some()
                    || self
                        .selected_run_key()
                        .is_some_and(|k| self.armed_runs.contains(&k)))
            {
                let mut r = self.request(false);
                r.presence = Some(wire::Presence {
                    present: Some(false),
                });
                self.desktop_rpc(r).await?;
            }
            self.game = None;
            self.attach_retry = None;
        }
        Ok(())
    }
    async fn cancel(&mut self, pending: &Mutex<Option<handoff::Pending>>) -> Result<(), String> {
        self.clear_game(pending);
        if !self.journal.attempt.is_empty() {
            let mut r = self.request(false);
            r.pair_cancel = Some(self.proof());
            self.journal = Journal::new(self.device.to_string());
            self.persist()?;
            self.send(&r, None).await?;
        } else if self
            .room
            .as_ref()
            .and_then(|r| r.state.as_deref())
            .is_some_and(|s| s == "preparing" || s == "loading")
        {
            let old = self.room.as_ref().unwrap().clone();
            let mut read = self.request(false);
            read.room_get = Some(wire::Empty {});
            self.desktop_rpc(read).await?;
            let room = self.room.as_ref().ok_or("RESTART_REQUIRED")?;
            if room.room_id != old.room_id
                || room.run_id != old.run_id
                || room.run_epoch != old.run_epoch
                || room.config_revision != old.config_revision
            {
                return Err("RESTART_REQUIRED".into());
            }
            let mut r = self.request(false);
            if self.revision == G2_REV
                && room.host_account_id.as_deref()
                    != self.desktop.as_ref().map(|d| d.account.as_str())
            {
                r.presence = Some(wire::Presence {
                    present: Some(false),
                });
            } else {
                r.cancel_run = Some(wire::Empty {});
            }
            self.desktop_rpc(r).await?;
        }
        Ok(())
    }
    async fn logout(&mut self, pending: &Mutex<Option<handoff::Pending>>) -> Result<(), String> {
        self.clear_game(pending);
        let desktop = self.desktop.take();
        self.browser = None;
        let mut r = self.request(false);
        r.logout = Some(wire::Empty {});
        if let Some(s) = &desktop {
            r.session_epoch = Some(s.epoch.clone());
        }
        self.journal = Journal::new(self.device.to_string());
        self.room = None;
        self.rooms.clear();
        self.chat_room.clear();
        self.chat_sequence = 0;
        self.chat_messages.clear();
        self.chat_retry = None;
        // Clear remembered data before remote work; no offline credential can revive the scope.
        let cleanup_profile = ServiceProfile::new(&self.base, &self.service, self.device)
            .map_err(|e| e.to_string())?;
        self.resources.select(cleanup_profile);
        let deleted = self
            .resources
            .clear_local()
            .map_err(|_| "Local credential deletion failed");
        let profile = ServiceProfile::new(&self.base, &self.service, self.device)
            .map_err(|e| e.to_string())?;
        self.resources.select(profile);
        let remote = if let Some(s) = desktop {
            if self.info().await.is_ok() {
                r.service_epoch = Some(self.epoch.clone());
            }
            let mut outcome = self.send(&r, Some(s.access.as_str())).await;
            for _ in 0..2 {
                if outcome.as_ref().is_err_and(|e| {
                    e.starts_with("Service unavailable") || e.starts_with("Response interrupted")
                }) {
                    outcome = self.send(&r, Some(s.access.as_str())).await
                } else {
                    break;
                }
            }
            outcome.map(|_| ())
        } else {
            Ok(())
        };
        self.resources.dispose();
        self.remember = false;
        self.info = None;
        deleted.map_err(String::from)?;
        if remote.is_ok() {
            self.base.clear();
            self.service.clear();
            self.epoch.clear();
        }
        remote.map_err(|e| format!("Signed out locally; remote revocation was not confirmed ({e})"))
    }
}

#[derive(Serialize)]
pub(super) struct View {
    server_origin: String,
    service_id: String,
    service_epoch: Option<String>,
    session_epoch: Option<String>,
    game_path: String,
    profile_selected: bool,
    signed_in: bool,
    activation_code: Option<String>,
    activation_attempt: Option<String>,
    website: Option<String>,
    account_id: Option<String>,
    protocol_revision: String,
    device_id: String,
    desktop_id: Option<String>,
    room: Option<wire::Room>,
    rooms: Vec<wire::RoomSummary>,
    history: Vec<String>,
    recent_servers: Vec<RecentServer>,
    room_cursor: String,
    next_room_id: Option<String>,
    status_error: Option<String>,
    chat_messages: Vec<wire::ChatMessage>,
    handoff: String,
    seed: Option<u32>,
    mock: bool,
    limits: &'static str,
}
fn view<S: CredentialStore>(c: &Core<S>, p: &Option<handoff::Pending>) -> View {
    View {
        server_origin: c.base.clone(),
        service_id: c.service.clone(),
        service_epoch: c.info.as_ref().and_then(|i| i.service_epoch.clone()),
        session_epoch: c.desktop.as_ref().map(|d| d.epoch.clone()),
        game_path: c
            .game_path
            .as_ref()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default(),
        profile_selected: !c.base.is_empty(),
        signed_in: c.desktop.is_some(),
        activation_code: (!c.journal.code.is_empty()).then(|| c.journal.code.clone()),
        device_id: c.device.to_string(),
        desktop_id: c.desktop.as_ref().map(|d| d.id.clone()),
        activation_attempt: (!c.journal.attempt.is_empty()).then(|| c.journal.attempt.clone()),
        website: c.info.as_ref().map(|_| c.base.clone()),
        account_id: c.desktop.as_ref().map(|d| d.account.clone()),
        protocol_revision: c.revision.into(),
        chat_messages: c.chat_messages.clone(),
        room: c.room.clone(),
        rooms: c.rooms.clone(),
        history: c.history.clone(),
        recent_servers: c.recent_servers.clone(),
        room_cursor: c.room_cursor.clone(),
        next_room_id: c.next_room_id.clone(),
        status_error: c.status_error.clone(),
        handoff: match p {
            Some(p) if p.loaded => "World loaded",
            Some(p) if p.accepted => "Seed accepted",
            Some(_) => "Choose normal New Game in Noita",
            None => "No native preparation",
        }
        .into(),
        seed: p.as_ref().map(|p| p.seed),
        mock: c.info.as_ref().and_then(|i| i.mock_provider) == Some(true),
        limits: "Development profile; native preparation requires a matching supported game/mod/bridge installation",
    }
}
fn trusted<R: tauri::Runtime>(w: &tauri::WebviewWindow<R>) -> Result<(), String> {
    let u = w.url().map_err(|_| "Unavailable origin")?;
    origin(w.label(), u.as_str())
}
fn origin(label: &str, url: &str) -> Result<(), String> {
    let u = reqwest::Url::parse(url).map_err(|_| "Invalid origin")?;
    if label != "main"
        || !(u.scheme() == "tauri" && u.host_str() == Some("localhost")
            || ["http", "https"].contains(&u.scheme())
                && u.host_str() == Some("tauri.localhost")
                && u.port().is_none())
        || !u.username().is_empty()
        || u.password().is_some()
    {
        return Err("Command origin denied".into());
    }
    Ok(())
}
pub(super) fn navigation(url: &str) -> bool {
    origin("main", url).is_ok()
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Login {
    server: String,
    service_id: String,
    remember: bool,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Action {
    game_path: Option<String>,
    action: ActionKind,
    room: Option<String>,
    subject: Option<String>,
    text: Option<String>,
    ready: Option<bool>,
    password: Option<String>,
    selected: Option<Vec<String>>,
    excluded: Option<Vec<String>>,
}
#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum ActionKind {
    ConfigureGame,
    ApproveMock,
    CreateFixture,
    Join,
    OpenWebsite,
    Ready,
    ChatSend,
    NextRooms,
    FirstRooms,
    Start,
    End,
    Refresh,
    Cancel,
}
#[tauri::command]
pub(super) async fn g1_login<R: tauri::Runtime>(
    w: tauri::WebviewWindow<R>,
    state: tauri::State<'_, App>,
    input: Login,
) -> Result<View, String> {
    trusted(&w)?;
    let _permit = permit(&state.slots)?;
    let generation = state.generation.load(Ordering::Acquire);
    let mut c = state.core.lock().await;
    if state.generation.load(Ordering::Acquire) != generation {
        return Err("Client action cancelled".into());
    }
    c.select(&input.server, &input.service_id, input.remember)
        .await?;
    if state.generation.load(Ordering::Acquire) != generation {
        return Err("Client action cancelled".into());
    }
    Ok(view(&c, &state.pending.lock().unwrap()))
}
#[tauri::command]
pub(super) async fn g1_status<R: tauri::Runtime>(
    w: tauri::WebviewWindow<R>,
    state: tauri::State<'_, App>,
) -> Result<View, String> {
    trusted(&w)?;
    let _permit = permit(&state.slots)?;
    let generation = state.generation.load(Ordering::Acquire);
    let c = state.core.lock().await;
    if state.generation.load(Ordering::Acquire) != generation {
        return Err("Client action cancelled".into());
    }
    // Service/liveness work is owned by the Rust loop, independently of renderer polling.
    Ok(view(&c, &state.pending.lock().unwrap()))
}
#[tauri::command]
pub(super) async fn g1_action<R: tauri::Runtime>(
    w: tauri::WebviewWindow<R>,
    state: tauri::State<'_, App>,
    input: Action,
) -> Result<View, String> {
    trusted(&w)?;
    if matches!(input.action, ActionKind::Cancel) {
        state.generation.fetch_add(1, Ordering::AcqRel);
        *state.pending.lock().unwrap() = None;
    }
    let _permit = if matches!(input.action, ActionKind::Cancel) {
        permit_limit(&state.slots, 18)?
    } else {
        permit(&state.slots)?
    };
    let generation = state.generation.load(Ordering::Acquire);
    let mut c = state.core.lock().await;
    if state.generation.load(Ordering::Acquire) != generation {
        return Err("Client action cancelled".into());
    }
    if input.selected.as_ref().is_some_and(|v| v.len() > 32)
        || input.excluded.as_ref().is_some_and(|v| v.len() > 32)
    {
        return Err("Roster exceeds 32 desktops".into());
    }
    match input.action {
        ActionKind::ConfigureGame => {
            if state.pending.lock().unwrap().is_some() {
                return Err("Cancel preparation before changing installation".into());
            }
            c.configure_game(
                input
                    .game_path
                    .as_deref()
                    .ok_or("Installation folder required")?,
            )?;
        }
        ActionKind::ApproveMock => {
            c.mock_approve(input.subject.as_deref().unwrap_or("host"))
                .await?
        }
        ActionKind::CreateFixture => c.fixture_room().await?,
        ActionKind::Join => {
            c.join_with_password(input.room.as_deref().ok_or("Select room")?, input.password)
                .await?
        }
        ActionKind::OpenWebsite => crate::browser::open_service(&c.base)?,
        ActionKind::Ready => {
            c.set_ready(input.ready.ok_or("Readiness required")?)
                .await?
        }
        ActionKind::ChatSend => c.chat_send(input.text.ok_or("Message required")?).await?,
        ActionKind::Refresh => c.rotate().await?,
        ActionKind::NextRooms => c.room_page(true).await?,
        ActionKind::FirstRooms => c.room_page(false).await?,
        ActionKind::Cancel => c.cancel(&state.pending).await?,
        ActionKind::End => {
            c.clear_game(&state.pending);
            let mut r = c.request(false);
            r.end_run = Some(wire::Empty {});
            c.desktop_rpc(r).await?;
        }
        ActionKind::Start => {
            if state.pending.lock().unwrap().is_some() {
                return Err("Preparation already exists".into());
            }
            c.start(
                input.selected.ok_or("Explicit selected roster required")?,
                input.excluded.ok_or("Explicit excluded roster required")?,
            )
            .await?;
            if state.generation.load(Ordering::Acquire) != generation {
                return Err("Preparation cancelled".into());
            }
            if c.room
                .as_ref()
                .and_then(|r| r.members.as_ref())
                .is_some_and(|members| {
                    members.iter().any(|m| {
                        m.selected == Some(true)
                            && m.desktop_id.as_deref() == c.desktop.as_ref().map(|d| d.id.as_str())
                    })
                })
            {
                if c.revision == G2_REV {
                    c.auto_arm(&state.pending)?;
                } else {
                    let p = c.preparation()?;
                    *state.pending.lock().unwrap() = Some(p);
                }
            }
        }
    }
    if state.generation.load(Ordering::Acquire) != generation {
        return Err("Client action cancelled".into());
    }
    Ok(view(&c, &state.pending.lock().unwrap()))
}
#[tauri::command]
pub(super) async fn g1_logout<R: tauri::Runtime>(
    w: tauri::WebviewWindow<R>,
    state: tauri::State<'_, App>,
) -> Result<View, String> {
    trusted(&w)?;
    state.generation.fetch_add(1, Ordering::AcqRel);
    *state.pending.lock().unwrap() = None;
    let _permit = permit_limit(&state.slots, 18)?;
    let mut c = state.core.lock().await;
    c.logout(&state.pending).await?;
    Ok(view(&c, &None))
}

#[cfg(test)]
#[path = "g1_test.rs"]
mod tests;

pub(super) fn start_background<S: CredentialStore + 'static>(
    core: Arc<tokio::sync::Mutex<Core<S>>>,
    pending: Arc<Mutex<Option<handoff::Pending>>>,
    stop: Arc<std::sync::atomic::AtomicBool>,
) -> tauri::async_runtime::JoinHandle<()> {
    tauri::async_runtime::spawn(async move {
        while !stop.load(Ordering::Acquire) {
            {
                let mut c = core.lock().await;
                if stop.load(Ordering::Acquire) {
                    break;
                }
                let generation = c.generation.load(Ordering::Acquire);
                if c.info.is_some() && Instant::now() >= c.retry_until {
                    let mut result = c.sync(&pending).await;
                    if result.is_ok() {
                        result = c.retry_chat().await;
                    }
                    if result.is_ok() {
                        result = c.chat_list().await;
                    }
                    if generation == c.generation.load(Ordering::Acquire) {
                        c.status_error = result.err();
                    }
                }
            }
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
    })
}
