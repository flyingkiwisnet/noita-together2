use super::*;
use crate::world_local::{self as local, Intent};
fn fixture() -> Files {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../spikes/noita-entry/.build/lifecycle-lc1-client-20261018-01")
        .join(format!("unit-store-{}", uuid::Uuid::new_v4()));
    Files::ephemeral_fixture(root).unwrap()
}
fn record() -> Record {
    let id = Identity {
        origin: "https://example.invalid".into(),
        service: "lc1-test".into(),
        account: uuid::Uuid::new_v4().to_string(),
        device: uuid::Uuid::new_v4(),
        room: "room".into(),
        run: "run".into(),
        run_epoch: "epoch".into(),
        config: 1,
        config_digest: "a".repeat(64),
    };
    let mut r = Record::prepared(
        id,
        uuid::Uuid::new_v4(),
        "b".repeat(64),
        Some("424242".into()),
        1,
        serde_json::json!({"fixture":"payload"}),
    )
    .unwrap();
    r.proof = Some("p".repeat(43));
    r.consumed = true;
    r.next_attempt([1; 16]).unwrap();
    r
}
fn target(files: &Files, r: &Record) -> PathBuf {
    files.root().unwrap().join(format!(
        "{}.world",
        hex(&binding(&r.identity, r.world).unwrap())
    ))
}
#[test]
fn lc1_file_atomic_cuts_sharing_failure_and_corrupt_offline_never_restore_rejoin() {
    let files = fixture();
    let mut r = record();
    r.intent = Intent::Offline;
    local::save(&files, &r).unwrap();
    let mut next = r.clone();
    next.canonical_generation = 1;
    for cut in 1..=4 {
        files.fault.store(cut, std::sync::atomic::Ordering::Release);
        assert!(
            local::save(&files, &next)
                .err()
                .expect("Expected protected store failure")
                .contains("Injected")
        );
        let old = local::load(&files, &r.identity, r.world).unwrap().unwrap();
        assert_eq!(old.intent, Intent::Offline);
        assert_eq!(old.canonical_generation, 0);
        assert_eq!(
            fs::read_dir(files.root().unwrap())
                .unwrap()
                .filter_map(Result::ok)
                .filter(|e| e.file_name().to_string_lossy().ends_with(".stage"))
                .count(),
            0
        );
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        let held = OpenOptions::new()
            .read(true)
            .share_mode(3)
            .open(target(&files, &r))
            .unwrap();
        let error = local::save(&files, &next)
            .err()
            .expect("Expected protected store failure");
        assert!(error.contains("atomic publication"));
        drop(held);
        assert_eq!(
            local::load(&files, &r.identity, r.world)
                .unwrap()
                .unwrap()
                .canonical_generation,
            0
        );
    }
    files.fault.store(5, std::sync::atomic::Ordering::Release);
    assert!(
        local::save(&files, &next).is_err(),
        "Post-publication failure must be explicit"
    );
    let published = local::load(&files, &r.identity, r.world).unwrap().unwrap();
    assert_eq!(published.intent, Intent::Offline);
    assert_eq!(
        published.canonical_generation, 1,
        "Never fall back to previous committed data"
    );
    let path = target(&files, &r);
    let mut cipher = fs::read(&path).unwrap();
    assert!(
        !cipher
            .windows(43)
            .any(|p| p == r.proof.as_ref().unwrap().as_bytes())
    );
    let n = cipher.len() / 2;
    cipher[n] ^= 1;
    fs::write(&path, &cipher).unwrap();
    assert!(local::load(&files, &r.identity, r.world).is_err());
    assert!(
        local::select(
            &files,
            &r.identity.origin,
            &r.identity.service,
            &r.identity.account,
            r.identity.device,
            &r.marker().unwrap()
        )
        .is_err()
    );
    next.intent = Intent::Rejoin;
    assert!(local::save(&files, &next).is_err());
    assert_eq!(fs::read(&path).unwrap(), cipher);
}
#[test]
fn lc1_file_scope_entropy_normalization_size_bounds_and_closed_root() {
    #[cfg(windows)]
    assert!(
        local_root(Path::new(r"\\unopened.invalid\share\fixture")).is_err(),
        "Reject network roots before any filesystem access"
    );
    let files = fixture();
    let mut r = record();
    local::save(&files, &r).unwrap();
    let mut alias = r.identity.clone();
    alias.origin = "https://EXAMPLE.INVALID:443/".into();
    assert!(local::load(&files, &alias, r.world).unwrap().is_some());
    let cipher = fs::read(target(&files, &r)).unwrap();
    for which in 0..5 {
        let mut id = r.identity.clone();
        let mut world = r.world;
        match which {
            0 => id.origin = "https://other.invalid".into(),
            1 => id.service = "another".into(),
            2 => id.account = "another".into(),
            3 => id.device = uuid::Uuid::new_v4(),
            _ => world = uuid::Uuid::new_v4(),
        };
        assert!(files.read(&id, world).unwrap().is_none());
        let path = files
            .root()
            .unwrap()
            .join(format!("{}.world", hex(&binding(&id, world).unwrap())));
        fs::write(&path, &cipher).unwrap();
        assert!(files.read(&id, world).is_err());
        fs::remove_file(path).unwrap();
    }
    r.config = serde_json::json!({"fixture":""});
    let base = Zeroizing::new(serde_json::to_vec(&r).unwrap()).len();
    r.config["fixture"] = serde_json::json!("x".repeat(MAX_PLAINTEXT - base));
    assert_eq!(
        Zeroizing::new(serde_json::to_vec(&r).unwrap()).len(),
        MAX_PLAINTEXT
    );
    local::save(&files, &r).unwrap();
    let mut too_large = r.clone();
    too_large.config["fixture"] = serde_json::json!("x".repeat(MAX_PLAINTEXT - base + 1));
    assert!(
        local::save(&files, &too_large)
            .err()
            .expect("Expected protected store failure")
            .contains("bound")
    );
    assert_eq!(
        Zeroizing::new(
            serde_json::to_vec(&local::load(&files, &r.identity, r.world).unwrap().unwrap())
                .unwrap()
        )
        .len(),
        MAX_PLAINTEXT
    );
    let reopened = Files::fixture_reopen(files.fixture_root().unwrap()).unwrap();
    assert!(
        local::load(&reopened, &r.identity, r.world)
            .unwrap()
            .is_some()
    );
    let unknown = files.root().unwrap().join("owned-test-note");
    fs::write(&unknown, b"nonsecret fixture note").unwrap();
    assert!(
        files.cleanup_fixture().is_err(),
        "Unknown files cannot be swept"
    );
    assert!(
        local::save(&reopened, &r).err().unwrap().contains("closed"),
        "Even failed cleanup must fence late writers"
    );
    assert!(unknown.is_file());
    fs::remove_file(unknown).unwrap();
    let cleanup = files.cleanup_fixture().unwrap();
    assert_eq!(
        files.cleanup_fixture().unwrap(),
        cleanup,
        "Repeated cleanup lost its verified count"
    );
    assert_eq!(
        reopened.cleanup_fixture().unwrap(),
        cleanup,
        "Independent cleanup must preserve the same receipt"
    );
    assert_eq!(cleanup["encrypted_files_removed"], 1);
    assert_eq!(cleanup["passed"], true);
    assert!(
        local::save(&reopened, &r)
            .err()
            .expect("Expected protected store failure")
            .contains("closed")
    );
    assert!(
        local::load(&reopened, &r.identity, r.world)
            .err()
            .expect("Expected protected store failure")
            .contains("closed")
    );
}
fn child(files: &Files, r: &Record, mode: &str) {
    use std::process::{Command, Stdio};
    let root = files.fixture_root().unwrap();
    let parent = root.parent().unwrap();
    let scope = parent.join(format!("child-scope-{}.json", uuid::Uuid::new_v4()));
    fs::write(
        &scope,
        serde_json::to_vec(&(r.identity.clone(), r.world)).unwrap(),
    )
    .unwrap();
    let log = fs::File::create(scope.with_extension("log")).unwrap();
    let mut command = Command::new(std::env::current_exe().unwrap());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    let mut child = command
        .args([
            "--exact",
            "world_store::tests::lc1_world_store_child",
            "--ignored",
            "--nocapture",
        ])
        .env("NT2_LC1_WORLD_ROOT", root)
        .env("NT2_LC1_WORLD_SCOPE", &scope)
        .env("NT2_LC1_WORLD_MODE", mode)
        .stdout(Stdio::from(log.try_clone().unwrap()))
        .stderr(Stdio::from(log))
        .spawn()
        .unwrap();
    let until = std::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            assert!(
                status.success(),
                "World store child failed; retained fixture log"
            );
            break;
        }
        assert!(
            std::time::Instant::now() < until,
            "Bounded child did not exit; no termination"
        );
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}
#[test]
fn lc1_file_cross_process_lock_terminal_stale_sequence_generation_and_preprepare_fences() {
    let files = fixture();
    let r = record();
    local::save(&files, &r).unwrap();
    files
        .locked(false, |_| {
            child(&files, &r, "deadline");
            Ok(())
        })
        .unwrap();
    assert_eq!(
        local::load(&files, &r.identity, r.world)
            .unwrap()
            .unwrap()
            .intent,
        Intent::Rejoin
    );
    child(&files, &r, "offline");
    assert_eq!(
        local::load(&files, &r.identity, r.world)
            .unwrap()
            .unwrap()
            .intent,
        Intent::Offline
    );
    assert!(
        local::save(&files, &r)
            .err()
            .expect("Expected protected store failure")
            .contains("superseded")
    );
    let mut terminal = r.clone();
    terminal.intent = Intent::Terminal;
    assert!(local::save(&files, &terminal).is_err());
    let independent = Files::fixture_reopen(files.fixture_root().unwrap()).unwrap();
    let mut current = record();
    current.canonical_generation = 5;
    current.next_attempt([2; 16]).unwrap();
    local::save(&files, &current).unwrap();
    let mut stale = current.clone();
    stale.sequence = 1;
    stale.attempt.as_mut().unwrap().sequence = 1;
    assert!(local::save(&independent, &stale).is_err());
    stale = current.clone();
    stale.canonical_generation = 4;
    assert!(local::save(&independent, &stale).is_err());
    stale = current.clone();
    stale.attempt.as_mut().unwrap().id = uuid::Uuid::new_v4();
    assert!(local::save(&independent, &stale).is_err());
    let mut preparation = record();
    preparation.identity.run = "preparing".into();
    preparation.identity.run_epoch = "preparing".into();
    preparation.consumed = false;
    preparation.sequence = 0;
    preparation.attempt = None;
    local::save(&files, &preparation).unwrap();
    let mut prepared = preparation.clone();
    prepared.identity.run = "prepared-run".into();
    prepared.identity.run_epoch = "prepared-epoch".into();
    prepared.association = Some(serde_json::json!({"fixture":true}));
    local::save(&independent, &prepared).unwrap();
    assert!(local::save(&files, &preparation).is_err());
}
#[test]
#[ignore = "Only invoked by the bounded parent with fresh owned file scope, no credentials in environment"]
fn lc1_world_store_child() {
    let root = PathBuf::from(std::env::var_os("NT2_LC1_WORLD_ROOT").unwrap());
    let scope = fs::read(std::env::var_os("NT2_LC1_WORLD_SCOPE").unwrap()).unwrap();
    let (id, world): (Identity, uuid::Uuid) = serde_json::from_slice(&scope).unwrap();
    let files = Files::fixture_reopen(root).unwrap();
    if std::env::var("NT2_LC1_WORLD_MODE").unwrap() == "deadline" {
        let began = std::time::Instant::now();
        assert!(
            local::load(&files, &id, world)
                .err()
                .expect("Expected protected store failure")
                .contains("lock deadline")
        );
        assert!(began.elapsed() < std::time::Duration::from_secs(4));
    } else {
        let mut r = local::load(&files, &id, world).unwrap().unwrap();
        r.intent = Intent::Offline;
        local::set_intent(&files, &r).unwrap();
    }
}
