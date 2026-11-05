use super::*;
#[path="next_social_discovery_test.rs"] mod social_discovery;
#[cfg(feature="g3-rejoin")]
#[path="next_connection_cr2_test.rs"] mod connection_cr2;
#[path="next_joined_test.rs"] mod joined;
#[path="next_c1_test.rs"] mod connection_c1;
#[path="next_signin_test.rs"] mod signin_recovery;
#[derive(Clone, Default)]
pub(super) struct MemoryStore(Arc<Mutex<Option<Vec<u8>>>>,Arc<Mutex<std::collections::BTreeMap<String,Vec<u8>>>>);
impl CredentialStore for MemoryStore {
    fn save(&self, p: &ServiceProfile, b: &SecretBlob) -> Result<(), crate::secrets::StoreError> {
        self.1.lock().unwrap().insert(format!("{p:?}"),b.bytes().to_vec());
        *self.0.lock().unwrap() = Some(b.bytes().to_vec());
        Ok(())
    }
    fn load(&self,p:&ServiceProfile)->Result<Option<SecretBlob>,crate::secrets::StoreError>{
        let records=self.1.lock().unwrap();
        let bytes=if records.is_empty(){self.0.lock().unwrap().clone()}else{records.get(&format!("{p:?}")).cloned()};
        bytes.map(SecretBlob::new).transpose()
    }
    fn delete(&self, p: &ServiceProfile) -> Result<(), crate::secrets::StoreError> {
        self.1.lock().unwrap().remove(&format!("{p:?}"));
        self.0.lock().unwrap().take();
        Ok(())
    }
}
fn core() -> Core<MemoryStore> {
    let mut c = Core::new(
        MemoryStore::default(),
        uuid::Uuid::new_v4(),
        Arc::new(Mutex::new(None)),
        std::env::temp_dir().join(format!("nt2-next-test-{}", uuid::Uuid::new_v4())),
    );
    c.world_files=Arc::new(nt2_desktop_g0::world_store::Files::ephemeral_fixture(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../spikes/noita-entry/.build/lifecycle-lc1-client-20261018-01").join(format!("unit-world-{}",uuid::Uuid::new_v4()))).unwrap());
    c.open_browser = false;
    c
}
#[test]
fn social_launcher_request_never_approves_or_prepares_runtime(){
 let mut c=core();let stream=Arc::new(Mutex::new(nt2_desktop_g0::g3_social_stream::Exchange::default()));
 assert!(c.request_social(stream.clone()).is_err());
 let journal=std::env::temp_dir().join(format!("nt2-social-request-{}.jsonl",uuid::Uuid::new_v4()));
 c.enable_g3(&journal,Arc::new(Mutex::new(nt2_desktop_g0::g3_runtime::Exchange::default()))).unwrap();c.request_social(stream).unwrap();
 assert_eq!(c.social_view()["requested"],true);assert_eq!(c.social_view()["enabled"],false);
 assert!(c.enable_social(std::path::Path::new("E:/unapproved"),&json!({}),c.social_stream.clone().unwrap()).is_err());
 tauri::async_runtime::block_on(async{assert!(c.load_game().await.is_err());c.social_scheduler_tick().await.unwrap();});
 assert!(c.pending.lock().unwrap().is_none());assert!(c.preparation.is_none());assert!(c.social_owner.is_none());
 assert!(!c.social_view().to_string().contains("access_token"));
}
#[test]
fn load_game_refuses_unverified_installation_before_any_service_preparation() {
    tauri::async_runtime::block_on(async {
        let mut c = core();
        assert!(
            c.load_game()
                .await
                .unwrap_err()
                .contains("no preparation was issued")
        );
        assert!(c.preparation.is_none() && c.pending.lock().unwrap().is_none());
        assert!(c.expected_game.lock().unwrap().is_none());
        let view = c.view();
        assert!(view["handoff"].as_str().unwrap().contains("Verify"));
        assert!(
            view["native_connection"]["status"]
                .as_str()
                .unwrap()
                .contains("No game contact")
        );
    });
}
#[test]
fn unsupported_installed_pair_never_prepares_a_server_grant() {
    tauri::async_runtime::block_on(async {
        let mut c = core();
        let dir =
            std::env::temp_dir().join(format!("nt2 unsupported game {}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&dir).unwrap();
        std::fs::write(dir.join("noita.exe"), "unsupported executable").unwrap();
        std::fs::write(dir.join("lua51.dll"), "unsupported runtime").unwrap();
        c.info = Some(wire::Info {
            profile: Some(crate::installation::candidate_test_profile()),
            ..Default::default()
        });
        assert!(
            c.configure_game(dir.to_str().unwrap())
                .unwrap_err()
                .contains("Unsupported Noita")
        );
        assert!(c.game_path.is_none() && c.expected_game.lock().unwrap().is_none());
        assert!(
            c.view()["installation_diagnostics"]
                .as_str()
                .unwrap()
                .contains("noita.exe:")
        );
        c.game_path = Some(dir.clone());
        assert!(
            c.load_game()
                .await
                .unwrap_err()
                .contains("Unsupported Noita")
        );
        assert!(c.preparation.is_none() && c.pending.lock().unwrap().is_none());
        std::fs::remove_dir_all(dir).unwrap();
    });
}
#[test]
#[ignore = "requires copied originals and artifacts in NT2_MULTIBUILD_FIXTURE; never executes supplied binaries"]
fn candidate_multibuild_copied_files_and_tampered_artifacts() {
    let dir = std::path::PathBuf::from(std::env::var("NT2_MULTIBUILD_FIXTURE").unwrap());
    let profile = crate::installation::candidate_test_profile();
    let label = crate::installation::verify_candidate(&dir, &profile).unwrap();
    assert!(label.starts_with("GOG"));
    let original_lua = crate::installation::hash(&dir.join("lua51.dll")).unwrap();
    let original_exe = crate::installation::hash(&dir.join("noita.exe")).unwrap();
    let mut c = core();
    c.info = Some(wire::Info {
        profile: Some(profile.clone()),
        ..Default::default()
    });
    c.configure_game(dir.to_str().unwrap()).unwrap();
    assert!(
        c.view()["installation_state"]
            .as_str()
            .unwrap()
            .contains("GOG")
    );
    for name in [
        "lua51.dll",
        "mods/noita_together2/init.lua",
        "mods/noita_together2/files/inspection.lua",
        "mods/noita_together2/files/nt2_g2_inspect_bridge.dll",
    ] {
        let path = dir.join(name);
        let bytes = std::fs::read(&path).unwrap();
        std::fs::write(&path, "modified fixture").unwrap();
        assert!(
            crate::installation::verify_candidate(&dir, &profile).is_err(),
            "{name}"
        );
        tauri::async_runtime::block_on(async {
            assert!(c.load_game().await.is_err());
        });
        assert!(c.preparation.is_none() && c.pending.lock().unwrap().is_none());
        std::fs::write(path, bytes).unwrap();
    }
    assert_eq!(
        crate::installation::hash(&dir.join("noita.exe")).unwrap(),
        original_exe
    );
    assert_eq!(
        crate::installation::hash(&dir.join("lua51.dll")).unwrap(),
        original_lua
    );
}
#[test]
fn armed_view_does_not_claim_a_protected_world_before_native_consumption() {
    let c = core();
    let p = handoff::new(424242, "test".into(), 1);
    *c.pending.lock().unwrap() = Some(p);
    assert!(c.view()["handoff"].as_str().unwrap().contains("Seed armed"));
    c.pending.lock().unwrap().as_mut().unwrap().owner = Some((nt2_local_ws::owner::Id::OsProcess(123), [1; 16]));
    assert!(
        c.view()["handoff"]
            .as_str()
            .unwrap()
            .contains("waiting for seed confirmation")
    );
}
#[test]
fn skipped_scheduler_wake_retains_the_last_poll_error() {
    tauri::async_runtime::block_on(async {
        let mut c = core();
        c.status_error = Some("Chat polling: INVALID_MESSAGE".into());
        c.tick().await.unwrap();
        assert_eq!(c.view()["status_error"], "Chat polling: INVALID_MESSAGE");
    });
}
#[test]
#[ignore = "read-only verification of the local matching installed inspection candidate; requires test-production-inspection.ps1"]
fn candidate_detects_installed_game_and_reverifies_same_path_while_armed() {
    let mut c = core();
    let built: Value = serde_json::from_str("{}")
    .unwrap();
    c.info = Some(wire::Info {
        profile: Some(wire::CompatibilityProfile {
            id: Some(built["id"].as_str().unwrap().into()),
            evidence_status: Some(built["evidenceStatus"].as_str().unwrap().into()),
            game_exe_sha256: Some(built["gameExeSha256"].as_str().unwrap().into()),
            original_lua_sha256: Some(built["originalLuaSha256"].as_str().unwrap().into()),
            bridge_sha256: Some(built["bridgeSha256"].as_str().unwrap().into()),
            mod_sha256: Some(built["modSha256"].as_str().unwrap().into()),
            ..Default::default()
        }),
        ..Default::default()
    });
    c.discover_game();
    let root = c
        .game_path
        .clone()
        .expect("Matching installed candidate should be detected/verified read-only");
    assert!(c.expected_game.lock().unwrap().is_some());
    assert!(
        c.view()["installation_state"]
            .as_str()
            .unwrap()
            .contains("Detected Noita installation and verified")
    );
    *c.pending.lock().unwrap() = Some(handoff::new(424242, "read-only-verification".into(), 1));
    let alias = root.to_str().unwrap().replace('\\', "/") + "/.";
    let executable = c.configure_game(&alias).unwrap();
    assert_eq!(Some(&executable), c.expected_game.lock().unwrap().as_ref());
    assert!(c.pending.lock().unwrap().as_ref().unwrap().owner.is_none());
}
#[test]
#[ignore = "requires test-next-core.ps1; routed post-join polling and real Rust handoff, synthetic native participant"]
fn next_core_post_join_poll_reaches_native_authority() {
    tauri::async_runtime::block_on(async {
        let mut c = core();
        c.select("http://127.0.0.1:4355", "", false).await.unwrap();
        registered(&mut c).await;
        c.create_fixture(false).await.unwrap();
        c.ready(true).await.unwrap();
        c.prepare().await.unwrap();
        let grant = c.preparation.clone().unwrap();
        let room = c.room.clone().unwrap();
        let mut p = handoff::new(
            room.config.as_ref().unwrap().seed.unwrap(),
            room.run_id.clone().unwrap(),
            u64::from(room.config_revision.unwrap()),
        );
        p.native_v2 = true;
        p.epoch = room.run_epoch.clone().unwrap();
        p.preparation_id = grant.preparation_id.clone();
        p.generation = Some((c.generation.clone(), c.generation.load(Ordering::Acquire)));
        p.claim_path = Some(c.claims_root.join("poll-one-use"));
        p.authority_until = Instant::now() - Duration::from_millis(1);
        *c.pending.lock().unwrap() = Some(p);
        let mut bad = c.request(false);
        bad.chat_list = Some(wire::ChatList {
            limit: Some(64),
            ..Default::default()
        });
        // The same canonical request accepted locally is refused by the actual inherited chat policy.
        assert!(
            c.rpc(&bad, false, false)
                .await
                .unwrap_err()
                .contains("INVALID_MESSAGE")
        );
        c.chat_send("polling reached chat".into()).await.unwrap();
        c.next_work = Instant::now();
        c.tick().await.unwrap();
        assert!(c.status_error.is_none());
        assert_eq!(c.chat.len(), 1);
        assert!(c.pending.lock().unwrap().as_ref().unwrap().authority_until > Instant::now());
        // Real handoff now obtains the seed; no synthetic loaded/report shortcuts precede it.
        let frame = seed_request(&c, 1, 0);
        let reply = handoff::transition(&mut c.pending.lock().unwrap(), nt2_local_ws::owner::Id::OsProcess(987), &frame);
        assert_eq!(u32::from_le_bytes(reply[8..12].try_into().unwrap()), 1);
        assert_eq!(
            u32::from_le_bytes(reply[12..16].try_into().unwrap()),
            424242
        );
        assert!(
            c.pending
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .claim_path
                .as_ref()
                .unwrap()
                .exists()
        );
        c.cancel().await.unwrap();
        std::fs::remove_file(c.claims_root.join("poll-one-use")).unwrap();
        std::fs::remove_dir(&c.claims_root).unwrap();
    });
}
// One bounded HTTP Info response; records the actual first-contact wire request.
fn info_server(
    status: &str,
    headers: &str,
    body: String,
) -> (String, std::thread::JoinHandle<String>) {
    use std::io::{Read, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let response = format!(
        "HTTP/1.1 {status}\r\n{headers}Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let handle = std::thread::spawn(move || {
        let (mut socket, _) = listener.accept().unwrap();
        socket
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let mut request = Vec::new();
        while !request.ends_with(b"\r\n\r\n") {
            let mut byte = [0];
            if socket.read(&mut byte).unwrap_or(0) == 0 {
                break;
            }
            request.push(byte[0]);
            assert!(request.len() < 8192);
        }
        let _ = socket.write_all(response.as_bytes());
        String::from_utf8(request).unwrap()
    });
    (origin, handle)
}
fn discovery_info(service: &str) -> String {
    json!({"major":1,"revision":protocol::REVISION,"serviceId":service,"serviceEpoch":"test-epoch"})
        .to_string()
}
#[test]
fn candidate_first_contact_refuses_invalid_tls_before_http_or_credentials() {
    tauri::async_runtime::block_on(async {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let origin = format!("https://{}", listener.local_addr().unwrap());
        let server = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            socket
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let mut bytes = [0; 8192];
            let count = socket.read(&mut bytes).unwrap();
            // A clear-text server cannot satisfy normal native TLS. No fallback allowed.
            let _ = socket.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\n{}");
            bytes[..count].to_vec()
        });
        let mut c = core();
        *c.store.0.lock().unwrap() = Some(b"token".to_vec());
        assert_eq!(
            c.select(&origin, "", true).await.unwrap_err(),
            "Service TLS/connection unavailable"
        );
        let bytes = server.join().unwrap();
        assert!(!bytes.starts_with(b"GET "));
        assert!(c.profile.is_none() && c.recent_servers.is_empty());
    });
}
#[test]
fn candidate_address_discovery_restart_and_identity_pin_before_credentials() {
    tauri::async_runtime::block_on(async {
        let mut c = core();
        let device = c.device;
        let path = c.claims_root.join("servers.json");
        c.public_preferences(path.clone()).unwrap();
        assert_eq!(c.view()["suggested_server"], "");
        let (origin, server) = info_server("200 OK", "", discovery_info("first-service"));
        c.select(&origin, "", false).await.unwrap();
        let request = server.join().unwrap().to_lowercase();
        assert!(request.starts_with("get /v1/info "));
        assert!(!request.contains("authorization") && !request.contains("cookie"));
        assert_eq!(c.service, "first-service");
        assert_eq!(c.view()["history"][0], origin);
        let mut restarted = Core::new(
            MemoryStore::default(),
            device,
            Arc::new(Mutex::new(None)),
            c.claims_root.clone(),
        );
        restarted.public_preferences(path.clone()).unwrap();
        assert_eq!(restarted.view()["suggested_server"], origin);
        assert_eq!(
            restarted.view()["recent_servers"][0]["service_id"],
            "first-service"
        );
        // Same origin reappears with a new identity. An invalid stored record would fail
        // differently if selection consulted the credential store before checking Info.
        *restarted.store.0.lock().unwrap() = Some(b"remembered-token-must-not-be-loaded".to_vec());
        let address = reqwest::Url::parse(&origin).unwrap();
        let listener = std::net::TcpListener::bind(("127.0.0.1", address.port().unwrap())).unwrap();
        let body = discovery_info("changed-service");
        let changed = std::thread::spawn(move || {
            use std::io::{Read, Write};
            let (mut socket, _) = listener.accept().unwrap();
            let mut bytes = [0; 8192];
            let count = socket.read(&mut bytes).unwrap();
            write!(
                socket,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            )
            .unwrap();
            String::from_utf8_lossy(&bytes[..count]).to_string()
        });
        let error = restarted.select(&origin, "", true).await.unwrap_err();
        assert!(error.contains("identity mismatch"));
        assert!(
            !changed
                .join()
                .unwrap()
                .to_lowercase()
                .contains("authorization")
        );
        assert!(restarted.profile.is_none() && restarted.desktop.is_none());
        assert_eq!(restarted.recent_servers[0].service_id, "first-service");
        assert_eq!(
            crate::preferences::Preferences::at(path.clone())
                .load(device)
                .unwrap()[0]
                .service_id,
            "first-service"
        );
        std::fs::remove_file(path).unwrap();
        std::fs::remove_dir(c.claims_root).unwrap();
    });
}
#[test]
fn candidate_discovery_rejects_redirect_incompatible_oversize_and_invalid_identity() {
    tauri::async_runtime::block_on(async {
        for (status, headers, body) in [
            (
                "302 Found",
                "Location: http://127.0.0.1:1/elsewhere\r\n",
                discovery_info("fixture"),
            ),
            (
                "200 OK",
                "",
                discovery_info("fixture").replace(protocol::REVISION, "g2-draft-1"),
            ),
            ("200 OK", "", "x".repeat(65537)),
            ("200 OK", "", discovery_info("bad identity!")),
            (
                "200 OK",
                "",
                json!({"major":1,"revision":protocol::REVISION,"serviceId":"fixture"}).to_string(),
            ),
        ] {
            let mut c = core();
            *c.store.0.lock().unwrap() = Some(b"token".to_vec());
            let (origin, server) = info_server(status, headers, body);
            assert!(c.select(&origin, "", true).await.is_err());
            assert!(
                !server
                    .join()
                    .unwrap()
                    .to_lowercase()
                    .contains("authorization")
            );
            assert!(c.profile.is_none() && c.recent_servers.is_empty());
        }
        assert!(
            core()
                .select("http://192.168.0.104:4356", "", true)
                .await
                .is_err()
        );
        assert!(
            core()
                .select("https://example.invalid/?redirect=secret", "", true)
                .await
                .is_err()
        );
    });
}
#[test]
fn candidate_public_profiles_are_bounded_and_last_selection_survives_restart() {
    let mut c = core();
    let path = c.claims_root.join("servers.json");
    c.public_preferences(path.clone()).unwrap();
    for port in 4400..4418 {
        let p = c
            .credential_profile(&format!("http://127.0.0.1:{port}"), "custom-service")
            .unwrap();
        c.remember_server(&p, "custom-service").unwrap();
    }
    let earlier = c
        .credential_profile("http://127.0.0.1:4405", "custom-service")
        .unwrap();
    c.remember_server(&earlier, "custom-service").unwrap();
    assert_eq!(c.recent_servers.len(), 16);
    let mut restarted = Core::new(
        MemoryStore::default(),
        c.device,
        Arc::new(Mutex::new(None)),
        c.claims_root.clone(),
    );
    restarted.public_preferences(path.clone()).unwrap();
    assert_eq!(
        restarted.view()["suggested_server"],
        "http://127.0.0.1:4405"
    );
    assert_eq!(restarted.recent_servers.len(), 16);
    assert!(restarted.game_path.is_none());
    std::fs::write(&path, b"invalid public config").unwrap();
    assert!(restarted.public_preferences(path.clone()).is_err());
    std::fs::remove_file(path).unwrap();
    std::fs::remove_dir(c.claims_root).unwrap();
}
#[test]
fn discovered_active_identity_mismatch_stops_ticks_but_failed_switch_preserves_session() {
    tauri::async_runtime::block_on(async {
        for active in [true, false] {
            let (mut c, _, _) = release_case();
            let (origin, server) = info_server("200 OK", "", discovery_info("changed-service"));
            c.base = if active {
                origin.clone()
            } else {
                "http://127.0.0.1:4321".into()
            };
            c.service = "original-service".into();
            c.profile = Some(c.credential_profile(&c.base, "original-service").unwrap());
            c.recent_servers.push(crate::preferences::RecentServer {
                remembered_name:None,has_saved_account:None,
                origin: origin.clone(),
                service_id: "original-service".into(),
            });
            assert!(
                c.select(&origin, "", true)
                    .await
                    .unwrap_err()
                    .contains("identity mismatch")
            );
            assert!(
                !server
                    .join()
                    .unwrap()
                    .to_lowercase()
                    .contains("authorization")
            );
            if active {
                assert!(c.desktop.is_none() && c.game.is_none() && c.profile.is_none());
                assert!(c.native_revoked);
                assert!(!c.pending.lock().unwrap().as_ref().unwrap().release);
                c.tick().await.unwrap(); // No session remains that could send another request.
            } else {
                assert!(c.desktop.is_some() && c.game.is_some() && c.profile.is_some());
                assert!(!c.native_revoked);
            }
        }
    });
}
#[test]
fn late_guest_retry_cannot_commit_after_cancellation_generation() {
    tauri::async_runtime::block_on(async {
        let mut c = core();
        c.guest_generation = c.generation.load(Ordering::Acquire);
        let mut request = c.request(false);
        request.guest_issue = Some(wire::GuestIssue {
            device_id: Some(c.device.to_string()),
            display_name: Some("late".into()),
            issuance_proof: Some("p".repeat(43)),
        });
        c.guest_retry = Some(Retry {
            request,
            until: Instant::now() + Duration::from_secs(30),
        });
        c.generation.fetch_add(1, Ordering::AcqRel);
        c.auth_tick().await.unwrap();
        assert!(c.guest_retry.is_none());
        assert!(c.desktop.is_none());
    });
}
#[test]
fn renderer_reload_disposes_pending_auth_without_network() {
    tauri::async_runtime::block_on(async {
        let mut c = core();
        c.auth = Some(
            SignIn::new(
                "http://127.0.0.1:4355",
                c.device,
                c.generation.load(Ordering::Acquire),
            )
            .unwrap(),
        );
        c.generation.fetch_add(1, Ordering::AcqRel);
        c.auth_tick().await.unwrap();
        assert!(c.auth.is_none());
        assert!(!c.view()["auth_pending"].as_bool().unwrap());
    });
}
#[test]
fn expired_refresh_recovery_cannot_issue_a_fresh_request_or_keep_native_owner() {
    tauri::async_runtime::block_on(async {
        let (mut c, _, _) = release_case();
        c.profile = Some(
            c.credential_profile("http://127.0.0.1:4355", "fixture")
                .unwrap(),
        );
        c.remember = true;
        c.save("r".repeat(43).as_str(), "old-request", now())
            .unwrap();
        let request = c.request(false);
        c.refresh_retry = Some(Retry {
            request,
            until: Instant::now(),
        });
        assert!(c.rotate().await.is_err());
        assert!(c.desktop.is_none());
        assert!(c.pending.lock().unwrap().is_none());
        assert!(c.refresh_retry.is_none());
        assert!(c.store.0.lock().unwrap().is_none());
        assert!(c.rotate().await.is_err());
    });
}
pub(super) fn release_case() -> (Core<MemoryStore>, handoff::Pending, wire::GameRelease) {
    let mut c = core();
    let goldens: Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../packages/contracts/g2-next/golden.json"
    )))
    .unwrap();
    let fixture = goldens["fixtures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["json"]["release"].is_object())
        .unwrap();
    let response = protocol::response(&serde_json::to_vec(&fixture["json"]).unwrap()).unwrap();
    let mut r = response.release.unwrap();
    r.authorization_id = Some(uuid::Uuid::new_v4().to_string());
    r.start_request_id = Some(uuid::Uuid::new_v4().to_string());
    r.captured_at = Some("2000-01-01T00:00:00Z".into());
    r.authorized_at = Some("2000-01-01T00:00:01Z".into());
    r.release_expires_at = Some("2099-01-01T00:00:00Z".into());
    r.phase = Some("authorized".into());
    r.entered_playing_at = None;
    c.service = r.service_id.clone().unwrap();
    c.epoch = r.service_epoch.clone().unwrap();
    c.room = Some(wire::Room {
        state: Some("running".into()),
        config: Some(wire::Config {
            seed: Some(424242),
            ..Default::default()
        }),
        members: Some(vec![wire::Member {
            desktop_id: r.desktop_id.clone(),
            account_id: Some("trusted-account".into()),
            present: Some(true),
            selected: Some(true),
            ..Default::default()
        }]),
        room_id: r.room_id.clone(),
        run_id: r.run_id.clone(),
        run_epoch: r.run_epoch.clone(),
        config_revision: r.config_revision,
        ..Default::default()
    });
    let session = |id: String, epoch: String, attachment: String| Session {
        access: SecretString::from("private-access"),
        refresh: SecretString::from("private-refresh"),
        id,
        epoch,
        attachment,
        account: "trusted-account".into(),
        principal: wire::Principal {
            principal_id: Some("trusted-account".into()),
            kind: Some("registered".into()),
            ..Default::default()
        },
        expires: now() + 900000,
    };
    c.desktop = Some(session(
        r.desktop_id.clone().unwrap(),
        r.desktop_session_epoch.clone().unwrap(),
        String::new(),
    ));
    c.preparation = Some(wire::Preparation {
        preparation_id: Some(uuid::Uuid::new_v4().to_string()),
        service_id: r.service_id.clone(),
        service_epoch: r.service_epoch.clone(),
        room_id: r.room_id.clone(),
        run_id: r.run_id.clone(),
        run_epoch: r.run_epoch.clone(),
        config_revision: r.config_revision,
        desktop_id: r.desktop_id.clone(),
        desktop_session_epoch: r.desktop_session_epoch.clone(),
        expires_at: Some("2099-01-01T00:00:00Z".into()),
        review_expires_at: Some("2099-01-01T00:00:00Z".into()),
        state: Some("loaded".into()),
    });
    c.game = Some(Game {
        session: session(
            r.game_session_id.clone().unwrap(),
            r.game_session_epoch.clone().unwrap(),
            r.attachment_id.clone().unwrap(),
        ),
        instance: r.game_instance_id.clone().unwrap(),
        owner: (nt2_local_ws::owner::Id::OsProcess(7), [1; 16]),
        reported: 1,
        report_retry: None,
        renew_retry: None,
        permit: None,
        entered: false,
        offered: false,
    });
    native(&mut c, 1, true, false);
    let local = c.pending.lock().unwrap().clone().unwrap();
    r.world_token = Some(hex(&local.owner.unwrap().1));
    r.content_digest = Some(hex(&local.observation.as_ref().unwrap().digest));
    (c, local, r)
}
#[test]
fn release_requires_exact_world_content_scope_and_first_deadline() {
    let (c, p, r) = release_case();
    assert!(c.validate_release(&r, &p).is_ok());
    for mutate in [0, 1, 2, 3, 4, 5] {
        let mut changed = r.clone();
        match mutate {
            0 => changed.world_token = Some("different-world".into()),
            1 => changed.game_session_epoch = Some("different-epoch".into()),
            2 => changed.content_digest = Some("0".repeat(64)),
            3 => changed.observation_revision = Some(2),
            4 => changed.release_expires_at = Some("2021-01-01T00:00:00Z".into()),
            _ => changed.desktop_id = Some("different-desktop".into()),
        };
        assert!(c.validate_release(&changed, &p).is_err());
    }
    let mut incomplete = p.clone();
    incomplete
        .observation
        .as_mut()
        .unwrap()
        .observation
        .complete = false;
    assert!(c.validate_release(&r, &incomplete).is_err());
}
// Lifecycle authority facts are injected here; OS process matching is real.
fn g3_playing_case() -> (Core<MemoryStore>,std::path::PathBuf) {
    let (mut c,_,mut release)=release_case();
    let dir=std::env::temp_dir().join(format!("nt2-g3-core-{}",uuid::Uuid::new_v4()));
    std::fs::create_dir(&dir).unwrap();
    c.pending.lock().unwrap().take();
    c.enable_g3(&dir.join("journal.jsonl"),Arc::new(Mutex::new(nt2_desktop_g0::g3_runtime::Exchange::default()))).unwrap();
    c.base="https://fixture.invalid".into();
    *c.expected_game.lock().unwrap()=Some(std::env::current_exe().unwrap().canonicalize().unwrap());
    c.game.as_mut().unwrap().owner.0=nt2_local_ws::owner::Id::OsProcess(std::process::id());
    c.game.as_mut().unwrap().entered=true;
    c.game.as_mut().unwrap().permit=Some(release.clone());
    release.phase=Some("playing".into());release.entered_playing_at=Some("2000-01-01T00:00:02Z".into());
    c.g3_release=Some(release);
    native(&mut c,1,false,false);
    {let mut pending=c.pending.lock().unwrap();let p=pending.as_mut().unwrap();p.release=true;p.authority_until=Instant::now()+Duration::from_secs(30);}
    (c,dir)
}
#[test]
fn g3_core_requires_selected_start_actual_playing_and_current_native_authority(){
    for fault in 0..10 {
        let (mut c,dir)=g3_playing_case();
        c.g3_sync().unwrap();assert_eq!(c.g3_view()["native_playing"],true);
        match fault {
            0=>c.pending.lock().unwrap().as_mut().unwrap().observation.as_mut().unwrap().observation.protected=true,
            1=>c.pending.lock().unwrap().as_mut().unwrap().release=false,
            2=>c.pending.lock().unwrap().as_mut().unwrap().authority_until=Instant::now(),
            3=>c.pending.lock().unwrap().as_mut().unwrap().last_seen=Some(Instant::now()-Duration::from_secs(13)),
            4=>c.game.as_mut().unwrap().reported=0,
            5=>c.game.as_mut().unwrap().entered=false,
            6=>c.room.as_mut().unwrap().members.as_mut().unwrap()[0].selected=Some(false),
            7=>{c.generation.fetch_add(1,Ordering::AcqRel);},
            8=>c.pending.lock().unwrap().as_mut().unwrap().loaded=false,
            _=>{c.pending.lock().unwrap().take();},
        }
        let _=c.g3_sync();assert_eq!(c.g3_view()["native_playing"],false,"fault {fault}");
        let view=c.g3_view().to_string();assert!(!view.contains("private-access")&&!view.contains("private-refresh"));
        drop(c);std::fs::remove_dir_all(dir).unwrap();
    }
}
#[test]
fn g3_core_refuses_changed_process_world_service_and_frozen_release(){
    for fault in 0..4 {
        let (mut c,dir)=g3_playing_case();c.g3_sync().unwrap();
        match fault {
            0=>*c.expected_game.lock().unwrap()=Some(dir.join("wrong.exe")),
            1=>c.pending.lock().unwrap().as_mut().unwrap().owner=Some((nt2_local_ws::owner::Id::OsProcess(std::process::id()),[9;16])),
            2=>c.epoch="changed-service-epoch".into(),
            _=>c.g3_release.as_mut().unwrap().authorization_id=Some(uuid::Uuid::new_v4().to_string()),
        }
        let result=c.g3_sync();if result.is_err(){c.g3_pause();}
        assert_eq!(c.g3_view()["native_playing"],false,"fault {fault}");
        drop(c);std::fs::remove_dir_all(dir).unwrap();
    }
}
#[test]
fn g3_core_end_retains_original_and_restart_view_stays_uncertain(){
    let (mut c,dir)=g3_playing_case();c.g3_sync().unwrap();
    let runtime=c.g3.as_mut().unwrap();runtime.consumer.begin_gold("withdraw",17,runtime.consumer.generation()).unwrap();
    let original=runtime.consumer.journal.pending().next().unwrap().record.original.clone();
    c.clear_game();assert_eq!(c.g3_view()["native_playing"],false);assert_eq!(c.g3_view()["operations"][0]["amount"],17);
    assert_eq!(c.g3_view()["operations"][0]["operation_id"],original.operation_id);
    drop(c);
    let mut recovered=core();recovered.enable_g3(&dir.join("journal.jsonl"),Arc::new(Mutex::new(nt2_desktop_g0::g3_runtime::Exchange::default()))).unwrap();
    assert_eq!(recovered.g3_view()["operations"][0]["state"],"uncertain");assert_eq!(recovered.g3_view()["saved_world_proof"],false);
    assert!(recovered.g3.as_mut().unwrap().consumer.journal.prepare_effect(&original.operation_id).is_err());
    tauri::async_runtime::block_on(async{assert!(recovered.g3_original_status(&original.operation_id).await.is_err());});
    drop(recovered);std::fs::remove_dir_all(dir).unwrap();
}
#[test]
fn g3_original_status_refuses_changed_account_without_journal_promotion(){
    let (mut c,dir)=g3_playing_case();c.g3_sync().unwrap();
    let runtime=c.g3.as_mut().unwrap();runtime.consumer.begin_gold("deposit",11,runtime.consumer.generation()).unwrap();
    let id=runtime.consumer.journal.pending().next().unwrap().record.original.operation_id.clone();
    let before=std::fs::read(dir.join("journal.jsonl")).unwrap();c.desktop.as_mut().unwrap().account="another-account".into();
    tauri::async_runtime::block_on(async{assert!(c.g3_original_status(&id).await.unwrap_err().contains("does not own ORIGINAL"));});
    assert_eq!(before,std::fs::read(dir.join("journal.jsonl")).unwrap());assert_eq!(c.g3_view()["saved_world_proof"],false);
    assert!(c.g3.as_mut().unwrap().consumer.journal.prepare_effect(&id).is_err());
    drop(c);std::fs::remove_dir_all(dir).unwrap();
}
#[test]
fn restoring_requires_locally_consumed_frozen_authorization_and_phase() {
    let (mut c, p, mut r) = release_case();
    r.release_expires_at = Some("2021-01-01T00:00:00Z".into());
    c.game.as_mut().unwrap().permit = Some(r.clone());
    r.phase = Some("restoring".into());
    r.entered_playing_at = Some("2000-01-01T00:00:02Z".into());
    assert!(c.validate_release(&r, &p).is_err());
    c.game.as_mut().unwrap().entered = true;
    assert!(c.validate_release(&r, &p).is_ok());
    let mut changed = r.clone();
    changed.authorization_id = Some(uuid::Uuid::new_v4().to_string());
    assert!(c.validate_release(&changed, &p).is_err());
    changed = r.clone();
    changed.captured_at = Some("2000-01-01T00:00:01Z".into());
    assert!(c.validate_release(&changed, &p).is_err());
    let mut phase = p.clone();
    phase.observation.as_mut().unwrap().sequence = 2;
    phase.observation.as_mut().unwrap().observation.protected = false;
    c.game.as_mut().unwrap().reported = 2;
    assert!(c.validate_release(&r, &phase).is_err());
    r.phase = Some("playing".into());
    assert!(c.validate_release(&r, &phase).is_ok());
}
fn field<'a>(html: &'a str, name: &str) -> &'a str {
    html.split(&format!("name=\"{name}\" value=\""))
        .nth(1)
        .expect("Fixture field")
        .split('"')
        .next()
        .unwrap()
}
fn native_frame(
    c: &mut Core<MemoryStore>,
    sequence: u64,
    change: impl FnOnce(&mut native_report::Observation),
) {
    let mut pending = c.pending.lock().unwrap();
    let p = pending.as_ref().unwrap();
    let owner = p.owner.unwrap();
    let mut observation = p.observation.as_ref().unwrap().observation.clone();
    change(&mut observation);
    let frame = native_report::encode(owner.1, sequence, &observation).unwrap();
    let reply = crate::pipe::report_transition(&mut pending, owner.0, &frame);
    assert_eq!(reply[8], 1);
}
#[test]
fn stale_network_release_cannot_override_current_native_content_or_eligibility() {
    for failure in 0..3 {
        let (mut c, old, r) = release_case();
        c.apply_release(Some(r.clone()), &old).unwrap();
        native_frame(&mut c, 2, |o| match failure {
            0 => o.mods.push("changed-during-status-await".into()),
            1 => {
                o.complete = false;
                o.error = Some("collection_failed".into());
            }
            _ => o.loaded = false,
        });
        c.apply_release(Some(r.clone()), &old).unwrap();
        assert!(!c.pending.lock().unwrap().as_ref().unwrap().release);
        assert_eq!(
            c.game
                .as_ref()
                .unwrap()
                .permit
                .as_ref()
                .unwrap()
                .authorization_id,
            r.authorization_id
        );
        c.game.as_mut().unwrap().reported = 2;
        assert!(c.apply_release(Some(r), &old).is_err());
        assert!(!c.pending.lock().unwrap().as_ref().unwrap().release);
    }
}
#[test]
fn changed_then_reverted_content_cannot_restore_an_older_release() {
    let (mut c, old, r) = release_case();
    c.apply_release(Some(r.clone()), &old).unwrap();
    native_frame(&mut c, 2, |o| o.mods.push("changed".into()));
    native_frame(&mut c, 3, |o| {
        o.mods.pop();
    });
    c.game.as_mut().unwrap().reported = 3;
    assert_eq!(
        c.pending
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .observation
            .as_ref()
            .unwrap()
            .digest,
        old.observation.as_ref().unwrap().digest
    );
    assert!(c.apply_release(Some(r), &old).is_err());
    assert!(!c.pending.lock().unwrap().as_ref().unwrap().release);
}
#[test]
fn release_commit_refuses_cancellation_replacement_and_generation_changes() {
    for change in 0..3 {
        let (mut c, old, r) = release_case();
        match change {
            0 => {
                *c.pending.lock().unwrap() = None;
            }
            1 => {
                c.pending.lock().unwrap().as_mut().unwrap().owner = Some((nt2_local_ws::owner::Id::OsProcess(8), [2; 16]));
            }
            _ => {
                c.generation.fetch_add(1, Ordering::AcqRel);
            }
        }
        assert!(c.apply_release(Some(r), &old).is_err());
        assert!(
            c.pending
                .lock()
                .unwrap()
                .as_ref()
                .is_none_or(|p| !p.release)
        );
        assert!(c.game.as_ref().unwrap().permit.is_none());
    }
}
#[test]
fn newer_phase_preserves_valid_release_but_holds_on_incompatible_phase() {
    let (mut c, old, mut r) = release_case();
    c.apply_release(Some(r.clone()), &old).unwrap();
    c.game.as_mut().unwrap().entered = true;
    r.phase = Some("playing".into());
    r.entered_playing_at = Some("2000-01-01T00:00:02Z".into());
    native_frame(&mut c, 2, |o| o.protected = false);
    c.apply_release(Some(r.clone()), &old).unwrap();
    assert!(c.pending.lock().unwrap().as_ref().unwrap().release);
    assert_eq!(
        c.pending
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .invalidated_observation,
        0
    );
    c.game.as_mut().unwrap().reported = 2;
    c.apply_release(Some(r.clone()), &old).unwrap();
    assert!(c.pending.lock().unwrap().as_ref().unwrap().release);
    native_frame(&mut c, 3, |o| o.protected = true);
    c.apply_release(Some(r.clone()), &old).unwrap();
    assert!(!c.pending.lock().unwrap().as_ref().unwrap().release);
    c.game.as_mut().unwrap().reported = 3;
    r.phase = Some("restoring".into());
    c.apply_release(Some(r.clone()), &old).unwrap();
    assert!(c.pending.lock().unwrap().as_ref().unwrap().release);
    assert_eq!(
        c.game
            .as_ref()
            .unwrap()
            .permit
            .as_ref()
            .unwrap()
            .observation_revision,
        Some(1)
    );
}
#[test]
fn runtime_newer_same_phase_keeps_existing_validated_release() {
    let (mut c, old, mut r) = release_case();
    c.apply_release(Some(r.clone()), &old).unwrap();
    c.game.as_mut().unwrap().entered = true;
    r.phase=Some("playing".into());
    r.entered_playing_at=Some("2000-01-01T00:00:02Z".into());
    native_frame(&mut c,2,|o|o.protected=false);
    c.game.as_mut().unwrap().reported=2;
    c.apply_release(Some(r.clone()),&old).unwrap();
    let deadline=c.pending.lock().unwrap().as_ref().unwrap().authority_until;
    native_frame(&mut c,3,|_|{});
    c.apply_release(Some(r.clone()),&old).unwrap();
    {
        let pending=c.pending.lock().unwrap();let p=pending.as_ref().unwrap();
        assert!(p.release,"A newer eligible same-content playing report must not revoke existing validated authority");
        assert_eq!(p.authority_until,deadline,"Pending observation does not renew authority");
    }
    c.pending.lock().unwrap().as_mut().unwrap().authority_until=Instant::now();
    c.apply_release(Some(r),&old).unwrap();
    assert!(!c.pending.lock().unwrap().as_ref().unwrap().release,"Pending report cannot revive an expired grant");
}
#[test]
fn live_scheduler_pending_same_playing_report_keeps_original_movement_binding() {
    let (mut c,dir)=g3_playing_case();
    c.g3_sync().unwrap();assert_eq!(c.g3_view()["native_playing"],true);
    let until=c.pending.lock().unwrap().as_ref().unwrap().authority_until;
    native_frame(&mut c,2,|_|{});
    // Production social scheduler calls binding between the two-second Core polls.
    assert_eq!(c.game.as_ref().unwrap().reported,1);
    assert!(c.g3_binding().unwrap().is_some(),"Unacknowledged same-content playing report must not revoke a previously validated playing binding");
    c.g3_sync().unwrap();assert_eq!(c.g3_view()["native_playing"],true);
    assert_eq!(c.pending.lock().unwrap().as_ref().unwrap().authority_until,until);
    tauri::async_runtime::block_on(async {
        // Exact production control calls while the two-second poll is not due.
        c.next_work=Instant::now()+Duration::from_secs(2);
        c.social_fixture=true;c.social_enabled=true;
        c.social_stream=Some(Arc::new(Mutex::new(nt2_desktop_g0::g3_social_stream::Exchange::default())));
        c.tick().await.unwrap();
        assert!(c.social_scheduler_tick().await.is_err()); // unreachable fixture.invalid
        assert_eq!(c.g3_view()["native_playing"],true);
        assert_eq!(c.pending.lock().unwrap().as_ref().unwrap().authority_until,until);
    });
    drop(c);std::fs::remove_dir_all(dir).unwrap();
}
#[test]
fn diagnostic_control_failure_survives_success_and_fresh_preparation_clears_capture() {
    let (mut c,dir)=g3_playing_case();
    c.info=Some(wire::Info::default());
    tauri::async_runtime::block_on(async {
        c.g3_sync().unwrap();
        c.next_work=Instant::now();
        assert!(c.tick().await.is_err()); // fixture.invalid; actual presence poll failure
        let failed=c.g3_view()["control"].clone();
        assert_eq!(failed["failure"]["count"],1);
        assert_eq!(failed["failure"]["stage"],"presence");
        assert_eq!(failed["failure"]["category"],"connection_unavailable");
        assert_eq!(failed["failure_context"]["phase"],"playing");
        assert_eq!(failed["failure_context"]["entered_locally"],true);
        assert_eq!(failed["failure_context"]["local_revision"],1);
        assert_eq!(failed["failure_context"]["service_revision"],1);
        // Exact tick success path, with no authenticated owner/network work due.
        c.desktop=None;c.next_work=Instant::now();c.tick().await.unwrap();
        assert!(c.status_error.is_none());
        assert_eq!(c.g3_view()["control"]["failure"],failed["failure"]);
        assert_eq!(c.g3_view()["control"]["failure_context"],failed["failure_context"]);
        c.record_control_failure(FailureStage::Game,"secret-token principal/context E:/private/path");
        let capture=c.g3_view()["control"].to_string();
        for private in ["secret-token","principal/context","E:/private/path"] {assert!(!capture.contains(private));}
        assert_eq!(c.g3_view()["control"]["failure"]["category"],"refused");
        c.reset_capture_diagnostics(); // same reset invoked only after accepted prepare
        assert_eq!(c.g3_view()["control"]["failure"]["count"],0);
        assert!(c.g3_view()["control"]["failure_context"].is_null());
        assert_eq!(c.g3_view()["movement_failure"]["count"],0);
    });
    drop(c);std::fs::remove_dir_all(dir).unwrap();
}
#[test]
fn runtime_first_playing_report_during_release_await_keeps_original_grant() {
    let (mut c,old,r)=release_case();
    c.apply_release(Some(r.clone()),&old).unwrap();
    let deadline=c.pending.lock().unwrap().as_ref().unwrap().authority_until;
    native_frame(&mut c,2,|o|o.protected=false);
    c.apply_release(Some(r),&old).unwrap();
    let p=c.pending.lock().unwrap();let p=p.as_ref().unwrap();
    assert!(p.release,"First playing phase arriving during the protected snapshot await must not revoke its prior validated grant");
    assert_eq!(p.authority_until,deadline);
    assert!(!c.game.as_ref().unwrap().entered,"Pending local phase is not service acceptance or new consumption");
    assert_eq!(c.game.as_ref().unwrap().reported,1);
}
#[test]
fn runtime_social_capture_failure_does_not_pause_valid_movement() {
    let (mut c,dir)=g3_playing_case();
    tauri::async_runtime::block_on(async {
        c.g3_sync().unwrap();assert_eq!(c.g3_view()["native_playing"],true);
        c.social_fixture=true;c.social_enabled=true;
        c.social_stream=Some(Arc::new(Mutex::new(nt2_desktop_g0::g3_social_stream::Exchange::default())));
        for _ in 0..3{
            assert!(c.social_native_tick().await.is_err());
            assert_eq!(c.g3_view()["native_playing"],true,"Social discovery/capture failure cannot revoke an independently validated movement lease");
        }
        c.pending.lock().unwrap().as_mut().unwrap().authority_until=Instant::now();
        assert!(c.social_native_tick().await.is_err());
        assert_eq!(c.g3_view()["native_playing"],false,"Expired shared authority must pause both lanes");
    });
    drop(c);std::fs::remove_dir_all(dir).unwrap();
}
#[test]
fn newer_unchanged_report_does_not_replace_or_strand_the_first_frozen_permit() {
    let (mut c, old, r) = release_case();
    native_frame(&mut c, 2, |_| {});
    c.apply_release(Some(r.clone()), &old).unwrap();
    assert!(!c.pending.lock().unwrap().as_ref().unwrap().release);
    assert!(c.game.as_ref().unwrap().permit.is_none());
    c.game.as_mut().unwrap().reported = 2;
    c.apply_release(Some(r), &old).unwrap();
    assert!(c.pending.lock().unwrap().as_ref().unwrap().release);
    assert_eq!(
        c.game
            .as_ref()
            .unwrap()
            .permit
            .as_ref()
            .unwrap()
            .observation_revision,
        Some(1)
    );
}
fn seed_request(c: &Core<MemoryStore>, op: u32, value: u32) -> [u8; 32] {
    let mut request = [0; 32];
    request[..4].copy_from_slice(&handoff::MAGIC.to_le_bytes());
    request[4..8].copy_from_slice(&2u32.to_le_bytes());
    request[8..12].copy_from_slice(&op.to_le_bytes());
    request[12..16].copy_from_slice(&value.to_le_bytes());
    request[16..32].copy_from_slice(c.device.as_bytes());
    request
}
fn unconsumed(c: &mut Core<MemoryStore>) {
    let room = c.room.as_ref().unwrap();
    let mut p = handoff::new(
        424242,
        room.run_id.clone().unwrap(),
        u64::from(room.config_revision.unwrap()),
    );
    p.epoch = room.run_epoch.clone().unwrap();
    p.native_v2 = true;
    p.preparation_id = c
        .preparation
        .as_ref()
        .and_then(|grant| grant.preparation_id.clone());
    p.generation = Some((c.generation.clone(), c.generation.load(Ordering::Acquire)));
    p.claim_path = Some(
        c.claims_root.join(
            c.preparation
                .as_ref()
                .unwrap()
                .preparation_id
                .as_ref()
                .unwrap(),
        ),
    );
    *c.pending.lock().unwrap() = Some(p);
    c.game = None;
    c.native_revoked = false;
}
#[test]
fn cached_identifiers_cannot_renew_an_unconsumed_seed_after_authority_loss() {
    tauri::async_runtime::block_on(async {
        for change in 0..8 {
            let (mut c, _, _) = release_case();
            c.room.as_mut().unwrap().state = Some("staging".into());
            c.preparation.as_mut().unwrap().state = Some("issued".into());
            unconsumed(&mut c);
            c.game_tick().await.unwrap();
            match change {
                0 => c.room.as_mut().unwrap().state = Some("waiting".into()),
                1 => c.room.as_mut().unwrap().state = Some("ended".into()),
                2 => c.room.as_mut().unwrap().state = Some("suspended".into()),
                3 => c.room.as_mut().unwrap().members = Some(vec![]),
                4 => c.room.as_mut().unwrap().config_revision = Some(2),
                5 => c.preparation.as_mut().unwrap().state = Some("cancelled".into()),
                6 => {
                    c.preparation.as_mut().unwrap().preparation_id =
                        Some(uuid::Uuid::new_v4().to_string())
                }
                _ => {
                    c.generation.fetch_add(1, Ordering::AcqRel);
                }
            };
            assert!(c.game_tick().await.is_err());
            let request = seed_request(&c, 1, 0);
            let reply = handoff::transition(&mut c.pending.lock().unwrap(), nt2_local_ws::owner::Id::OsProcess(7), &request);
            assert_eq!(reply[8], 0);
            assert!(c.pending.lock().unwrap().is_none());
            assert!(c.native_revoked);
        }
    });
}
#[test]
fn attachment_first_deadline_clears_once_and_loss_reason_survives_unrelated_poll(){
 tauri::async_runtime::block_on(async{
  let (mut c,_,_)=release_case();
  c.game=None;c.attach_cycle=Some((uuid::Uuid::new_v4().to_string(),Instant::now()-Duration::from_millis(1)));
  assert_eq!(c.game_tick().await.unwrap_err(),"Game attachment retry expired; cancel and prepare again");
  assert!(c.pending.lock().unwrap().is_none()&&c.preparation.is_none()&&c.attach_cycle.is_none()&&c.game.is_none());
  let reason=c.view()["inspection_status"]["failure"].clone();assert!(reason.as_str().unwrap().contains("retry expired"));
  c.info=None;c.status_error=None;c.tick().await.unwrap();c.game_tick().await.unwrap();
  assert_eq!(c.view()["inspection_status"]["failure"],reason);
  assert_eq!(c.view()["handoff"],reason);
  assert!(!c.view().to_string().contains("world_token"));
 });
}
#[test]
#[ignore = "requires test-next-core.ps1; routed cancellation/end and actual Rust handoff, synthetic game observations"]
fn next_core_remote_terminal_authority() {
    tauri::async_runtime::block_on(async {
        let mut host = core();
        let mut guest = core();
        for c in [&mut host, &mut guest] {
            c.select(
                "http://127.0.0.1:4355",
                "nt2-client-browser-candidate",
                false,
            )
            .await
            .unwrap();
        }
        registered(&mut host).await;
        guest.guest("Terminal regression guest").await.unwrap();
        host.create_fixture(true).await.unwrap();
        guest
            .join(host.room.as_ref().unwrap().room_id.as_ref().unwrap(), None)
            .await
            .unwrap();
        for loaded in [false, true] {
            for end in [false, true] {
                room(&mut host).await;
                host.prepare().await.unwrap();
                room(&mut guest).await;
                guest.prepare().await.unwrap();
                unconsumed(&mut guest);
                guest.game_tick().await.unwrap();
                let grant = guest.preparation.as_ref().unwrap().clone();
                let marker = guest
                    .pending
                    .lock()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .claim_path
                    .clone()
                    .unwrap();
                let old_run = guest.room.as_ref().unwrap().run_id.clone();
                if loaded {
                    for (op, value) in [(1, 0), (3, 424242), (4, 1)] {
                        let request = seed_request(&guest, op, value);
                        let reply =
                            handoff::transition(&mut guest.pending.lock().unwrap(), nt2_local_ws::owner::Id::OsProcess(7), &request);
                        assert_eq!(reply[8], 1);
                    }
                    native(&mut guest, 1, true, false);
                    guest.pending.lock().unwrap().as_mut().unwrap().claim_path =
                        Some(marker.clone());
                    guest.game_tick().await.unwrap();
                    assert!(marker.exists());
                }
                room(&mut host).await;
                if end {
                    host.end().await.unwrap();
                } else {
                    let mut request = host.request(false);
                    request.cancel_run = Some(wire::Empty {});
                    let response = host.rpc(&request, false, false).await.unwrap();
                    host.absorb(&response);
                }
                room(&mut guest).await;
                assert_eq!(guest.room.as_ref().unwrap().run_id, old_run);
                assert_eq!(
                    guest.room.as_ref().unwrap().state.as_deref(),
                    Some(if end { "ended" } else { "waiting" })
                );
                guest.game_tick().await.unwrap();
                assert!(guest.native_revoked);
                assert!(guest.preparation.is_none());
                let request = seed_request(&guest, if loaded { 5 } else { 1 }, 0);
                let reply = handoff::transition(&mut guest.pending.lock().unwrap(), nt2_local_ws::owner::Id::OsProcess(7), &request);
                if loaded {
                    assert_eq!(reply[8], 1);
                    assert_eq!(reply[12], 1);
                    let p = guest.pending.lock().unwrap().clone().unwrap();
                    assert!(p.loaded && !p.release && Instant::now() >= p.authority_until);
                    assert!(marker.exists());
                    assert_eq!(std::fs::read(&marker).unwrap().len(), 20);
                    guest.game_tick().await.unwrap();
                    assert!(!guest.pending.lock().unwrap().as_ref().unwrap().release);
                    std::fs::remove_file(&marker).unwrap();
                    std::fs::remove_dir(marker.parent().unwrap()).unwrap();
                    guest.clear_game();
                } else {
                    assert_eq!(reply[8], 0);
                    assert!(!marker.exists());
                    assert!(guest.pending.lock().unwrap().is_none());
                }
                assert!(grant.preparation_id.is_some());
            }
        }
        host.logout().await.unwrap();
        guest.logout().await.unwrap();
    });
}
async fn registered<S:CredentialStore+'static>(c: &mut Core<S>) {
    registered_as(c,"alex").await;
}
async fn registered_as<S:CredentialStore+'static>(c: &mut Core<S>,subject:&str) {
    c.sign_in("website").await.unwrap();
    let launch = c.pending_launch.as_ref().unwrap();
    let page = c.http.get(launch).send().await.unwrap();
    let cookie = page.headers()["set-cookie"]
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_owned();
    let html = page.text().await.unwrap();
    let csrf = field(&html, "csrf");
    let page = c
        .http
        .post(launch)
        .header("Origin", &c.base)
        .header("Cookie", cookie)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(format!("csrf={csrf}&subject={subject}"))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .text()
        .await
        .unwrap();
    let url = c
        .auth
        .as_ref()
        .unwrap()
        .return_url(c.generation.load(Ordering::Acquire))
        .unwrap();
    let url=url.to_owned();
    let body = format!(
        "attemptId={}&completionGrant={}&callbackState={}",
        field(&page, "attemptId"),
        field(&page, "completionGrant"),
        field(&page, "callbackState")
    );
    // The loopback handler may close after acceptance. Retain the exact
    // callback form and confirm only actual Core's authenticated exchange.
    let deadline=Instant::now()+Duration::from_secs(12);
    while c.desktop.is_none(){
        if let Ok(reply)=c.http.post(&url).header("Origin",&c.base).header("Content-Type","application/x-www-form-urlencoded").body(body.clone()).send().await{let _=reply.error_for_status();}
        c.auth_tick().await.unwrap();
        assert!(Instant::now()<deadline,"actual callback/auth exchange did not complete");
        if c.desktop.is_none(){tokio::time::sleep(Duration::from_millis(100)).await;}
    }
    assert_eq!(
        c.desktop.as_ref().unwrap().principal.kind.as_deref(),
        Some("registered")
    );
}
async fn room(c: &mut Core<MemoryStore>) {
    let mut req = c.request(false);
    req.room_get = Some(wire::Empty {});
    let response = c.rpc(&req, false, false).await.unwrap();
    c.absorb(&response);
}
async fn presence(c: &mut Core<MemoryStore>) {
    let mut req = c.request(false);
    req.presence = Some(wire::Presence {
        present: Some(true),
    });
    let response = c.rpc(&req, false, false).await.unwrap();
    c.absorb(&response);
}
fn native(c: &mut Core<MemoryStore>, sequence: u64, protected: bool, changed: bool) {
    let room = c.room.as_ref().unwrap();
    let owner = c
        .game
        .as_ref()
        .map(|g| g.owner)
        .unwrap_or((nt2_local_ws::owner::Id::OsProcess(7), *c.device.as_bytes()));
    let observation = native_report::Observation {
        actual_seed: Some("424242".into()),
        mods: if changed {
            vec!["changed".into()]
        } else {
            vec![
                "noita_together2".into(),
                "duplicate".into(),
                "duplicate".into(),
            ]
        },
        complete: true,
        error: None,
        beta: None,
        loaded: true,
        protected,
        mod_version: "nt2-g2-inspect-1".into(),
        bridge_version: "g2-inspect-1".into(),
    ..Default::default()};
    let digest = observation.content_digest().unwrap();
    let mut p = handoff::new(
        424242,
        room.run_id.clone().unwrap(),
        u64::from(room.config_revision.unwrap()),
    );
    p.epoch = room.run_epoch.clone().unwrap();
    p.native_v2 = true;
    p.preparation_id = c
        .preparation
        .as_ref()
        .and_then(|grant| grant.preparation_id.clone());
    p.loaded = true;
    p.accepted = true;
    p.owner = Some(owner);
    p.last_seen = Some(Instant::now());
    p.generation = Some((c.generation.clone(), c.generation.load(Ordering::Acquire)));
    p.observation = Some(native_report::Latest {
        context: owner.1,
        sequence,
        observation,
        digest,
        observed_at: Instant::now(),
    });
    *c.pending.lock().unwrap() = Some(p);
}
#[test]
#[ignore = "requires test-next-core.ps1; actual service, synthetic native facts, no engine claim"]
fn next_core_service_lifecycle() {
    tauri::async_runtime::block_on(async {
        let mut host = core();
        let mut guest = core();
        let mut outsider = core();
        for c in [&mut host, &mut guest, &mut outsider] {
            c.select(
                "http://127.0.0.1:4355",
                "nt2-client-browser-candidate",
                true,
            )
            .await
            .unwrap();
        }
        registered(&mut host).await;
        guest.guest("Remote guest").await.unwrap();
        outsider.guest("Outside").await.unwrap();
        let guest_id = guest.desktop.as_ref().unwrap().id.clone();
        let guest_account = guest.desktop.as_ref().unwrap().account.clone();
        assert!(guest.create_fixture(false).await.is_err());
        host.create_fixture(false).await.unwrap();
        let denied = host.room.as_ref().unwrap().room_id.clone().unwrap();
        assert!(guest.join(&denied, None).await.is_err());
        assert_eq!(guest.desktop.as_ref().unwrap().account, guest_account);
        assert!(guest.room.is_none());
        let mut policy = host.request(false);
        policy.room_policy = Some(wire::RoomPolicy {
            allow_guests: Some(true),
            capacity: Some(32),
            ..Default::default()
        });
        let response = host.rpc(&policy, false, false).await.unwrap();
        host.absorb(&response);
        let id = host.room.as_ref().unwrap().room_id.clone().unwrap();
        guest.join(&id, None).await.unwrap();
        room(&mut host).await;
        host.ready(true).await.unwrap();
        room(&mut guest).await;
        guest.ready(true).await.unwrap();
        room(&mut host).await;
        host.prepare().await.unwrap();
        room(&mut guest).await;
        guest.prepare().await.unwrap();
        room(&mut host).await;
        // Test-only synthetic local worlds: the production arm path still rejects simulated profiles.
        assert!(
            host.configure_game(r"C:\Noita")
                .is_err()
        );
        native(&mut host, 1, true, false);
        native(&mut guest, 1, true, false);
        host.game_tick().await.unwrap();
        guest.game_tick().await.unwrap();
        room(&mut host).await;
        room(&mut guest).await;
        for _ in 0..3 {
            host.collect_inspection().await.unwrap();
            guest.collect_inspection().await.unwrap();
        }
        assert_eq!(host.inspections.len(), 2);
        assert_eq!(guest.inspections.len(), 2);
        assert!(
            guest
                .inspections
                .iter()
                .all(|r| r.active_mods.as_ref().unwrap().len() == 3)
        );
        assert!(!host.pending.lock().unwrap().as_ref().unwrap().release);
        let mut denied = outsider.request(false);
        denied.room_id = Some(id.clone());
        denied.inspection_list = Some(wire::InspectionList {
            limit: Some(1),
            after_desktop_id: None,
        });
        assert!(outsider.rpc(&denied, false, false).await.is_err());
        let host_id = host.desktop.as_ref().unwrap().id.clone();
        host.final_start(vec![host_id, guest_id], vec![])
            .await
            .unwrap();
        room(&mut guest).await;
        host.game_tick().await.unwrap();
        guest.game_tick().await.unwrap();
        assert!(host.pending.lock().unwrap().as_ref().unwrap().release);
        assert!(guest.pending.lock().unwrap().as_ref().unwrap().release);
        native(&mut host, 2, false, false);
        native(&mut guest, 2, false, false);
        host.game_tick().await.unwrap();
        guest.game_tick().await.unwrap();
        assert!(host.game.as_ref().unwrap().entered);
        assert!(guest.game.as_ref().unwrap().entered);
        let original = host.game.as_ref().unwrap().permit.clone().unwrap();
        let game_id = host.game.as_ref().unwrap().session.id.clone();
        let desktop_id = host.desktop.as_ref().unwrap().id.clone();
        for _ in 0..92 {
            host.http
                .post("http://127.0.0.1:4355/fixture/advance")
                .header("x-fixture-step", "20000")
                .send()
                .await
                .unwrap()
                .error_for_status()
                .unwrap();
            for c in [&mut host, &mut guest] {
                c.rotate().await.unwrap();
                presence(c).await;
                c.game.as_mut().unwrap().session.expires = now();
                c.pending.lock().unwrap().as_mut().unwrap().last_seen = Some(Instant::now());
                c.game_tick().await.unwrap();
            }
        }
        assert_eq!(host.game.as_ref().unwrap().session.id, game_id);
        assert_eq!(host.desktop.as_ref().unwrap().id, desktop_id);
        assert_eq!(
            host.game
                .as_ref()
                .unwrap()
                .permit
                .as_ref()
                .unwrap()
                .captured_at,
            original.captured_at
        );
        native(&mut host, 3, true, false);
        host.game_tick().await.unwrap();
        assert!(host.pending.lock().unwrap().as_ref().unwrap().release);
        assert_eq!(
            host.game
                .as_ref()
                .unwrap()
                .permit
                .as_ref()
                .unwrap()
                .observation_revision,
            Some(1)
        );
        let exposed = host.view().to_string();
        for secret in [
            host.desktop.as_ref().unwrap().access.as_str(),
            host.desktop.as_ref().unwrap().refresh.as_str(),
            host.game.as_ref().unwrap().session.access.as_str(),
        ] {
            assert!(!exposed.contains(secret));
        }
        native(&mut host, 4, true, true);
        assert!(host.game_tick().await.is_err());
        assert!(!host.pending.lock().unwrap().as_ref().unwrap().release);
        host.cancel().await.unwrap();
        assert!(host.pending.lock().unwrap().is_none());
        guest.logout().await.unwrap();
        assert!(guest.desktop.is_none());
        host.logout().await.unwrap();
        outsider.rotate().await.unwrap();
        outsider.logout().await.unwrap();
    });
}


#[cfg(feature="g3-rejoin")]
#[path="next_continue_cf04_test.rs"] mod continue_cf04;
