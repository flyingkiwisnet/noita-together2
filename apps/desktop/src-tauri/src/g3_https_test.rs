//! Explicitly gated actual HTTPS client test. SERVER-owned lifecycle factory;
//! every game observation is simulated and no native/mod profile is approved.
use super::*;
use crate::{g3_journal::Journal, g3_local_consumer::LocalConsumer};
use serde::Deserialize;
use std::{io::{BufRead, BufReader, Write}, path::PathBuf, process::{Child, Command, Stdio}, sync::mpsc};
use zeroize::Zeroize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Credential { pub(crate) access_token: String, pub(crate) session_epoch: String, pub(crate) account_id: String, pub(crate) session_id: String, refresh_secret: Option<String> }
impl Drop for Credential {
    fn drop(&mut self) { self.access_token.zeroize(); if let Some(secret) = &mut self.refresh_secret { secret.zeroize(); } }
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Bundle { pub(crate) origin: String, pub(crate) service_id: String, pub(crate) service_epoch: String, pub(crate) ca_path: PathBuf,
    pub(crate) evidence_status: String, desktops: Vec<Credential>, pub(crate) games: Vec<Credential>, pub(crate) scopes: Vec<wire::Scope> }
pub(crate) struct Fixture { child: Child, events: mpsc::Receiver<String>, bundle_path: PathBuf }
impl Fixture {
    fn start() -> Self {
        Self::start_mode(false)
    }
    pub(crate) fn start_social()->Self{Self::start_mode(true)}
    fn start_mode(social:bool)->Self{
        let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..");
        let mut command = Command::new("C:/Program Files/nodejs/node.exe");
        command.current_dir(&repo).args(["--import", "tsx", "apps/desktop/tools/g3-https-fixture.mjs"])
            .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::inherit());
        if social{command.arg("--social");}
        #[cfg(windows)] { use std::os::windows::process::CommandExt; command.creation_flags(0x08000000); }
        let mut child = command.spawn().expect("Start explicit disposable fixture");
        let output = child.stdout.take().unwrap(); let (send, events) = mpsc::channel();
        std::thread::spawn(move || { for line in BufReader::new(output).lines() { if let Ok(line) = line { let _ = send.send(line); } } });
        let mut fixture = Self { child, events, bundle_path: PathBuf::new() };
        let ready = fixture.event(); assert_eq!(ready["event"], "ready", "Disposable fixture startup failed");
        fixture.bundle_path = ready["bundlePath"].as_str().unwrap().into(); fixture
    }
    fn event(&self) -> serde_json::Value {
        let line = self.events.recv_timeout(Duration::from_secs(40)).expect("Bounded fixture event deadline");
        serde_json::from_str(&line).expect("Fixture event metadata")
    }
    pub(crate) fn command(&mut self, command: &str) {
        writeln!(self.child.stdin.as_mut().unwrap(), "{command}").unwrap();
        assert_eq!(self.event()["event"], command, "Fixture lifecycle command failed");
    }
    pub(crate) fn bundle(&self) -> Bundle {
        let bytes = Zeroizing::new(std::fs::read(&self.bundle_path).unwrap());
        assert!(bytes.len() < 65536); serde_json::from_slice(&bytes).expect("Private fixture bundle")
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        if let Some(input) = self.child.stdin.as_mut() { let _ = writeln!(input, "quit"); }
        self.child.stdin.take();
        let deadline = Instant::now() + Duration::from_secs(15);
        while Instant::now() < deadline {
            if self.child.try_wait().ok().flatten().is_some() { return; }
            std::thread::sleep(Duration::from_millis(50));
        }
        let _ = self.child.kill(); let _ = self.child.wait();
    }
}
fn binding(bundle: &Bundle, index: usize) -> Binding {
    Binding { context: [index as u8 + 1; 16], service_origin: bundle.origin.clone(), service_id: bundle.service_id.clone(),
        service_epoch: bundle.service_epoch.clone(), session_epoch: bundle.games[index].session_epoch.clone(),
        principal_id: bundle.games[index].account_id.clone(), scope: bundle.scopes[index].clone() }
}
fn owner(bundle: &Bundle, index: usize) -> HttpOwner {
    let root = reqwest::Certificate::from_pem(&std::fs::read(&bundle.ca_path).unwrap()).unwrap();
    HttpOwner::with_roots(binding(bundle, index), Zeroizing::new(bundle.games[index].access_token.clone()),
        Arc::new(AtomicU64::new(1)), Some(vec![root])).unwrap()
}
fn ghost_request(owner: &HttpOwner) -> Vec<u8> {
    let mut request = RecoveryOwner::request(owner); request.ghost_list = Some(wire::GhostList::default());
    serde_json::to_vec(&request).unwrap()
}
fn recovery(bundle: &Bundle, index: usize) -> RecoveryOwner {
    let credential = &bundle.desktops[index];
    let mut owner = RecoveryOwner::new(bundle.origin.clone(), bundle.service_id.clone(), bundle.service_epoch.clone(),
        credential.session_epoch.clone(), credential.account_id.clone(), Zeroizing::new(credential.access_token.clone()),
        Arc::new(AtomicU64::new(1)));
    owner.roots = Some(vec![reqwest::Certificate::from_pem(&std::fs::read(&bundle.ca_path).unwrap()).unwrap()]); owner
}
#[test]
#[ignore = "Explicit isolated HTTPS/server database fixture only; no game/native acceptance"]
fn actual_https_start_ghosts_original_gold_and_desktop_recovery() {
    assert_eq!(std::env::var("NT2_G3_HTTPS_TEST").as_deref(), Ok("1"), "Use the owned gated runner");
    let mut fixture = Fixture::start(); let bundle = fixture.bundle();
    assert_eq!(bundle.evidence_status, "simulated-game-facts");
    let mut first = owner(&bundle, 0); let mut peer = owner(&bundle, 1);
    // Independently trusted fixture-only root; normal system trust and wrong hostname reject.
    let mut untrusted = HttpOwner::new(binding(&bundle, 0), Zeroizing::new(bundle.games[0].access_token.clone()), Arc::new(AtomicU64::new(1))).unwrap();
    assert!(tauri::async_runtime::block_on(untrusted.call(ghost_request(&untrusted))).is_err());
    let mut wrong_host = owner(&bundle, 0); wrong_host.binding.service_origin = bundle.origin.replace("127.0.0.1", "localhost");
    assert!(tauri::async_runtime::block_on(wrong_host.call(ghost_request(&wrong_host))).is_err());
    assert!(tauri::async_runtime::block_on(first.call(ghost_request(&first))).is_err());
    fixture.command("start");
    let mut protected = owner(&bundle, 0);
    assert!(tauri::async_runtime::block_on(protected.call(ghost_request(&protected))).is_err());
    fixture.command("playing");
    let mut movement = RecoveryOwner::request(&first);
    movement.movement = Some(wire::Movement { frames: Some(vec![wire::MovementFrame { sequence:Some(1), x_milli:Some("1000".into()),
        y_milli:Some("2000".into()), vx_milli:Some("0".into()), vy_milli:Some("0".into()), facing_left:Some(false) }]) });
    tauri::async_runtime::block_on(first.call(serde_json::to_vec(&movement).unwrap())).expect("Authenticated fixture movement");
    let ghosts = tauri::async_runtime::block_on(peer.call(ghost_request(&peer))).expect("Authenticated fixture ghosts").ghosts.unwrap();
    assert_eq!(ghosts.len(), 1); assert_eq!(ghosts[0].principal_id.as_deref(), Some(bundle.games[0].account_id.as_str()));
    assert_eq!(ghosts[0].game_session_id.as_deref(), Some(bundle.games[0].session_id.as_str()));
    assert_eq!(ghosts[0].world_token, bundle.scopes[0].world_token);
    let directory = std::env::temp_dir().join(format!("nt2-g3-https-journal-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir(&directory).unwrap(); let path = directory.join("originals.jsonl");
    let mut consumer = LocalConsumer::new(Journal::open(&path).unwrap()); consumer.bind(binding(&bundle, 0)).unwrap();
    let bytes = consumer.begin_gold("deposit", 10, consumer.generation()).unwrap();
    let id = consumer.journal.pending().next().unwrap().record.original.operation_id.clone();
    // Actual commit followed by a deliberately discarded HTTP response, then exact original retry.
    let lost = tauri::async_runtime::block_on(first.submit_gold_via(&mut consumer, bytes.clone(), |owner, bytes| Box::pin(async move {
        let _committed = owner.exchange(bytes).await?;
        Err("Injected lost response after service commit".into())
    })));
    assert!(lost.is_err()); assert_eq!(consumer.journal.pending().count(), 1);
    let mut before_retry = recovery(&bundle, 0);
    let committed = tauri::async_runtime::block_on(before_retry.status(&consumer.journal, &id)).expect("Original status after lost response");
    let response = tauri::async_runtime::block_on(first.submit_gold(&mut consumer, bytes)).expect("Exact flushed original retry over authenticated HTTPS");
    let operation = response.operation.unwrap(); assert_eq!(operation.state.as_deref(), Some("awaiting_local")); assert_eq!(operation.balance, Some(0));
    assert_eq!(operation.delivery_id, committed.delivery_id);
    // Cancellation after actual service commit rejects the response and cannot mutate local phase.
    let cancelled = consumer.begin_gold("deposit", 11, consumer.generation()).unwrap();
    let cancelled_id = protocol_g3::request(&cancelled).unwrap().gold_intent.unwrap().operation_id.unwrap();
    let result = tauri::async_runtime::block_on(first.submit_gold_via(&mut consumer, cancelled, |owner, bytes| Box::pin(async move {
        let response = owner.exchange(bytes).await?;
        owner.fence.fetch_add(1, Ordering::AcqRel);
        Ok(response)
    })));
    assert!(result.is_err());
    assert_eq!(consumer.journal.pending().find(|p| p.record.original.operation_id == cancelled_id).unwrap().record.phase,
        crate::g3_journal::Phase::Requested);
    // No receipt/effect. End then reopen journal: status/outbox reads preserve ORIGINAL.
    fixture.command("end"); drop(consumer);
    let mut journal = Journal::open(&path).unwrap(); assert!(journal.uncertain(&id));
    let mut desktop = recovery(&bundle, 0);
    let status = tauri::async_runtime::block_on(desktop.status(&journal, &id)).expect("Owning desktop status after End");
    assert_eq!(status.delivery_id, operation.delivery_id); assert_eq!(status.payload_digest, operation.payload_digest);
    // A fresh recovery owner models a separately authenticated reader; wrong principal is local-denied.
    let mut foreign = recovery(&bundle, 1); assert!(tauri::async_runtime::block_on(foreign.status(&journal, &id)).is_err());
    let mut outbox = recovery(&bundle, 0);
    let page = tauri::async_runtime::block_on(outbox.outbox(&journal, &id, "0")).expect("Owning original outbox after End");
    assert_eq!(page.outbox.unwrap()[0].operation.as_ref().unwrap().operation_id.as_deref(), Some(id.as_str()));
    fixture.command("restart"); let restarted = fixture.bundle(); assert_ne!(restarted.service_epoch, bundle.service_epoch);
    let mut desktop = recovery(&restarted, 0);
    let status = tauri::async_runtime::block_on(desktop.status(&journal, &id)).expect("Owning original status after service restart");
    assert_eq!(status.delivery_id, operation.delivery_id); assert_eq!(status.balance, Some(0));
    let mut cancelled_reader = recovery(&restarted, 0);
    let cancelled_status = tauri::async_runtime::block_on(cancelled_reader.status(&journal, &cancelled_id)).expect("Cancelled original after restart");
    assert_eq!(cancelled_status.amount, Some(11)); assert_eq!(cancelled_status.balance, Some(0));
    assert!(journal.uncertain(&cancelled_id)); assert!(journal.prepare_effect(&cancelled_id).is_err());
    assert!(journal.uncertain(&id)); assert!(journal.prepare_effect(&id).is_err());
    let bundle_path = fixture.bundle_path.clone(); let ca_path = restarted.ca_path.clone(); drop(fixture);
    assert!(!bundle_path.exists()); assert!(!ca_path.exists());
    drop(journal); std::fs::remove_file(path).unwrap(); std::fs::remove_dir(directory).unwrap();
}
