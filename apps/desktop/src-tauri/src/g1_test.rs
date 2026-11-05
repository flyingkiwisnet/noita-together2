use super::*;
#[derive(Default, Clone)]
struct MemoryStore {
    bytes: Arc<Mutex<Option<Vec<u8>>>>,
}
impl CredentialStore for MemoryStore {
    fn save(&self, _: &ServiceProfile, b: &SecretBlob) -> Result<(), crate::secrets::StoreError> {
        *self.bytes.lock().unwrap() = Some(b.bytes().to_vec());
        Ok(())
    }
    fn load(&self, _: &ServiceProfile) -> Result<Option<SecretBlob>, crate::secrets::StoreError> {
        self.bytes
            .lock()
            .unwrap()
            .as_ref()
            .map(|b| SecretBlob::new(b.clone()))
            .transpose()
    }
    fn delete(&self, _: &ServiceProfile) -> Result<(), crate::secrets::StoreError> {
        *self.bytes.lock().unwrap() = None;
        Ok(())
    }
}
#[test]
fn origin_and_command_payload_denial() {
    for url in [
        "https://evil.invalid",
        "file:///C:/test.html",
        "tauri://evil",
        "http://tauri.localhost:81",
        "https://user@tauri.localhost",
    ] {
        assert!(origin("main", url).is_err(), "{url}")
    }
    assert!(origin("other", "http://tauri.localhost").is_err());
    assert!(origin("main", "http://tauri.localhost").is_ok());
    assert!(
        serde_json::from_str::<Action>(r#"{"action":"read_file","room":"C:/secret"}"#).is_err()
    );
    assert!(serde_json::from_str::<Login>(r#"{"server":"http://127.0.0.1:4321","service_id":"fixture","remember":false,"token":"secret"}"#).is_err());
}
#[test]
fn timestamps_and_journal_budget() {
    assert_eq!(timestamp("1970-01-01T00:00:00.000Z").unwrap(), 0);
    assert_eq!(
        timestamp("2026-10-06T00:00:00.123Z").unwrap(),
        1791244800123
    );
    for t in ["bad", "2026-99-01T00:00:00Z", "2026-10-06T00:00:00+01:00"] {
        assert!(timestamp(t).is_err())
    }
    let mut j = Journal::new(uuid());
    j.refresh = "r".repeat(256);
    j.proof = "p".repeat(256);
    j.attempt = "a".repeat(128);
    j.request_id = "i".repeat(128);
    j.operation = "exchange".into();
    j.operation_revision = REV.into();
    assert!(SecretBlob::new(serde_json::to_vec(&j).unwrap()).is_ok());
    assert!(j.validate(&j.device, REV).is_ok());
    j.operation_revision = "g1-draft-1".into();
    assert!(j.validate(&j.device, REV).is_err());
    j.operation.clear();
    let mut legacy = serde_json::to_value(&j).unwrap();
    legacy.as_object_mut().unwrap().remove("operation_revision");
    let mut old: Journal = serde_json::from_value(legacy).unwrap();
    assert!(old.validate(&old.device, REV).is_ok());
    old.operation = "refresh".into();
    assert!(old.validate(&old.device, REV).is_err());
}
#[test]
fn renderer_never_contains_credentials() {
    let mut c = new_core(MemoryStore::default(), uuid::Uuid::new_v4());
    c.journal.proof = "fixture-private-proof".into();
    c.journal.refresh = "fixture-private-refresh".into();
    let v = serde_json::to_string(&view(&c, &None)).unwrap();
    assert!(!v.contains("fixture-private"));
}
#[test]
#[ignore = "requires test-tls-g2.ps1 with temporary current-user certificate trust"]
fn g2_tls_private_ip_current_user_roots() {
    tauri::async_runtime::block_on(async {
        let base = std::env::var("NT2_TLS_TEST_ORIGIN").unwrap();
        let expected = std::env::var("NT2_TLS_TEST_EXPECT").unwrap();
        let device = uuid::Uuid::new_v4();
        let profile = ServiceProfile::new(&base, "nt2-g2-tls-test", device).unwrap();
        assert!(profile.origin().starts_with("https://192.168.0.104:"));
        assert!(
            ServiceProfile::new(
                &base.replacen("https://", "http://", 1),
                "nt2-g2-tls-test",
                device
            )
            .is_err()
        );
        let core = new_g2_core(MemoryStore::default(), device);
        let response = core
            .http
            .get(format!("{}/probe", profile.origin()))
            .send()
            .await;
        if expected == "trusted" {
            let response = response.unwrap().error_for_status().unwrap();
            assert_eq!(response.text().await.unwrap(), "nt2-private-ip-tls-ok");
        } else {
            let failure = response.expect_err("Untrusted/wrong-IP TLS must not connect");
            assert!(failure.is_connect());
        }
        println!(
            "PASS production Core native-TLS transport: {expected}; private-IP HTTPS verification and HTTP LAN denial"
        );
    });
}
#[test]
fn public_view_roster_controls_across_sessions() {
    let device = uuid::Uuid::new_v4();
    let first = uuid();
    let replacement = uuid();
    assert_ne!(device.to_string(), first);
    assert_ne!(first, replacement);
    let mut c = new_g2_core(MemoryStore::default(), device);
    c.base = "https://service.test".into();
    c.service = "service-scope-test".into();
    c.info = Some(wire::Info {
        service_epoch: Some("service-epoch-test".into()),
        ..Default::default()
    });
    c.desktop = Some(Session {
        access: SecretString::from("private-test-access"),
        refresh: SecretString::from("private-test-refresh"),
        account: uuid(),
        id: first.clone(),
        epoch: uuid(),
        device: device.to_string(),
        expires: now() + 60000,
        attachment: String::new(),
    });
    c.room = Some(wire::Room {
        room_id: Some(uuid()),
        state: Some("waiting".into()),
        members: Some(vec![wire::Member {
            desktop_id: Some(first.clone()),
            present: Some(true),
            ready: Some(false),
            ..Default::default()
        }]),
        ..Default::default()
    });
    let joined = serde_json::to_value(view(&c, &None)).unwrap();
    assert_eq!(joined["device_id"], device.to_string());
    assert_eq!(joined["desktop_id"], first);
    assert_eq!(joined["server_origin"], "https://service.test");
    assert_eq!(joined["service_id"], "service-scope-test");
    assert_eq!(joined["service_epoch"], "service-epoch-test");
    assert_eq!(joined["session_epoch"], c.desktop.as_ref().unwrap().epoch);
    c.room.as_mut().unwrap().members = Some(vec![]);
    let nonmember = serde_json::to_value(view(&c, &None)).unwrap();
    c.room.as_mut().unwrap().members = Some(vec![wire::Member {
        desktop_id: Some(first),
        present: Some(true),
        ready: Some(true),
        ..Default::default()
    }]);
    c.desktop.as_mut().unwrap().id = replacement.clone();
    let replaced = serde_json::to_value(view(&c, &None)).unwrap();
    assert_eq!(replaced["device_id"], device.to_string());
    assert_eq!(replaced["desktop_id"], replacement);
    c.room.as_mut().unwrap().members.as_mut().unwrap()[0].desktop_id = Some(replacement);
    let rejoined = serde_json::to_value(view(&c, &None)).unwrap();
    c.desktop = None; // Even a stale roster cannot re-enable controls after local logout.
    let logged_out = serde_json::to_value(view(&c, &None)).unwrap();
    assert!(logged_out["desktop_id"].is_null());
    assert!(logged_out["session_epoch"].is_null());
    assert_eq!(logged_out["device_id"], device.to_string());
    let cases = serde_json::json!({"joined":joined,"nonmember":nonmember,"replaced":replaced,"rejoined":rejoined,"loggedOut":logged_out});
    assert!(!cases.to_string().contains("private-test"));
    println!("NT2_RENDERER_VIEWS={cases}");
}
#[test]
fn command_queue_is_bounded_before_mutex_wait() {
    let slots = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let leases = (0..16).map(|_| permit(&slots).unwrap()).collect::<Vec<_>>();
    assert!(permit(&slots).is_err());
    drop(leases);
    assert!(permit(&slots).is_ok());
}
#[test]
fn tauri_ipc_capabilities_reject_remote_and_arbitrary_operations() {
    use tauri::Manager;
    let mut c = new_core(WindowsCredentialStore, uuid::Uuid::new_v4());
    c.journal.proof = "private-IPC-proof-fixture".into();
    c.journal.refresh = "private-IPC-refresh-fixture".into();
    let generation = c.generation.clone();
    let app = tauri::test::mock_builder()
        .manage(App {
            core: Arc::new(tokio::sync::Mutex::new(c)),
            pending: Arc::new(Mutex::new(None)),
            generation,
            slots: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        })
        .invoke_handler(tauri::generate_handler![
            g1_status, g1_login, g1_action, g1_logout
        ])
        .build(tauri::generate_context!())
        .unwrap();
    let webview = app.get_webview_window("main").unwrap_or_else(|| {
        tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
            .build()
            .unwrap()
    });
    let invoke = |cmd: &str, url: &str, body: serde_json::Value| {
        tauri::test::get_ipc_response(
            &webview,
            tauri::webview::InvokeRequest {
                cmd: cmd.into(),
                callback: tauri::ipc::CallbackFn(0),
                error: tauri::ipc::CallbackFn(1),
                url: url.parse().unwrap(),
                body: tauri::ipc::InvokeBody::Json(body),
                headers: Default::default(),
                invoke_key: tauri::test::INVOKE_KEY.into(),
            },
        )
    };
    let v = invoke("g1_status", "http://tauri.localhost", serde_json::json!({}))
        .unwrap()
        .deserialize::<serde_json::Value>()
        .unwrap();
    assert!(!v.to_string().contains("private-IPC"));
    assert!(
        invoke(
            "g1_status",
            "https://attacker.invalid",
            serde_json::json!({})
        )
        .is_err()
    );
    for command in [
        "get_credentials",
        "read_file",
        "plugin:fs|read_file",
        "plugin:shell|execute",
        "plugin:http|fetch",
        "open_socket",
        "g0_login",
    ] {
        assert!(invoke(command,"http://tauri.localhost",serde_json::json!({"path":"C:/private","url":"https://attacker.invalid","program":"cmd.exe"})).is_err());
    }
    assert!(
        invoke(
            "g1_action",
            "http://tauri.localhost",
            serde_json::json!({"input":{"action":"read_file","room":"C:/private"}})
        )
        .is_err()
    );
    assert!(invoke("g1_login","http://tauri.localhost",serde_json::json!({"input":{"server":"https://example.invalid","service_id":"fixture","remember":false,"refreshSecret":"injected-secret"}})).is_err());
    app.state::<App>().slots.store(16, Ordering::Release);
    assert!(
        invoke(
            "g1_action",
            "http://tauri.localhost",
            serde_json::json!({"input":{"action":"cancel"}})
        )
        .is_ok()
    );
    assert!(app.state::<App>().generation.load(Ordering::Acquire) > 0);
    app.state::<App>().slots.store(0, Ordering::Release);
}

#[test]
fn cancellation_rejects_late_network_result() {
    use std::io::{Read, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let mut c = new_core(MemoryStore::default(), uuid::Uuid::new_v4());
    let generation = c.generation.clone();
    let worker = std::thread::spawn(move || {
        let (mut socket, _) = listener.accept().unwrap();
        socket
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        let mut input = [0; 2048];
        assert!(socket.read(&mut input).unwrap() > 0);
        generation.fetch_add(1, Ordering::AcqRel);
        let body = r#"{"major":1,"revision":"g1-draft-2","serviceId":"fixture","serviceEpoch":"epoch","mockProvider":false}"#;
        write!(socket,"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",body.len(),body).unwrap();
    });
    let outcome = tauri::async_runtime::block_on(c.select(&base, "fixture", false));
    worker.join().unwrap();
    assert_eq!(outcome.err().unwrap(), "Client action cancelled");
    assert!(c.info.is_none());
    assert!(c.desktop.is_none());
    assert!(c.journal.proof.is_empty());
}

#[test]
#[ignore = "requires isolated client-owned fault fixture on port4323"]
fn persisted_g1_client_lifecycle() {
    tauri::async_runtime::block_on(async {
        const BASE: &str = "http://127.0.0.1:4323";
        const SERVICE: &str = "nt2-client-g1-test";
        let control = |body: serde_json::Value| async move {
            reqwest::Client::new()
                .post(format!("{BASE}/fixture/control"))
                .json(&body)
                .send()
                .await
                .unwrap()
                .error_for_status()
                .unwrap();
        };
        control(serde_json::json!({"reset":true})).await;
        let device = uuid::Uuid::new_v4();
        let store = MemoryStore::default();
        let pending = Arc::new(Mutex::new(None));
        let mut c = new_core(store.clone(), device);
        c.select(BASE, SERVICE, true).await.unwrap();
        assert!(c.desktop.is_none());
        assert!(!c.journal.proof.is_empty());
        // Wrong proof cannot exchange; failed request must not expose secrets in its error.
        let mut wrong = c.request(false);
        let mut proof = c.proof();
        proof.proof = Some("wrong-proof".into());
        wrong.pair_exchange = Some(proof);
        assert_eq!(c.send(&wrong, None).await.err().unwrap(), "FORBIDDEN");
        control(serde_json::json!({"provider":false})).await;
        assert!(c.mock_approve("host").await.is_err());
        assert!(c.desktop.is_none());
        control(serde_json::json!({"provider":true})).await;
        // Exchange committed at service, reply deliberately lost; app state recreated from remembered journal.
        control(serde_json::json!({"drop":"pairExchange"})).await;
        assert!(c.mock_approve("host").await.is_err());
        let exchange_id = c.journal.request_id.clone();
        assert_eq!(c.journal.operation, "exchange");
        drop(c);
        let mut c = new_core(store.clone(), device);
        c.select(BASE, SERVICE, true).await.unwrap();
        assert!(c.desktop.is_some());
        assert!(c.journal.operation.is_empty());
        assert!(!exchange_id.is_empty());
        assert!(c.journal.proof.is_empty());
        let desktop_id = c.desktop.as_ref().unwrap().id.clone();
        let old_refresh = SecretString::from(c.desktop.as_ref().unwrap().refresh.as_str());
        // Refresh committed, response lost, service restarted, exact original request recovered.
        control(serde_json::json!({"drop":"refresh"})).await;
        assert!(c.rotate().await.is_err());
        let request_id = c.journal.request_id.clone();
        control(serde_json::json!({"restart":true})).await;
        drop(c);
        let mut c = new_core(store.clone(), device);
        c.select(BASE, SERVICE, true).await.unwrap();
        assert_eq!(c.desktop.as_ref().unwrap().id, desktop_id);
        assert!(c.desktop.as_ref().unwrap().refresh.as_str() != old_refresh.as_str());
        assert!(!request_id.is_empty());
        // An independent host browser creates rooms; guests receive only public discovery/preview before joining.
        c.browser = None;
        let mut host_browser = new_core(MemoryStore::default(), uuid::Uuid::new_v4());
        host_browser.select(BASE, SERVICE, false).await.unwrap();
        host_browser.mock_approve("host").await.unwrap();
        host_browser.fixture_room().await.unwrap();
        let room_id = host_browser.room.as_ref().unwrap().room_id.clone().unwrap();
        let mut access = host_browser.request(false);
        access.session_epoch = Some(host_browser.browser.as_ref().unwrap().epoch.clone());
        access.room_access = Some(wire::RoomAccess {
            visibility: Some("private".into()),
            admitted_account_ids: Some(vec![]),
        });
        let token = SecretString::from(host_browser.browser.as_ref().unwrap().access.as_str());
        host_browser.room = host_browser
            .send(&access, Some(token.as_str()))
            .await
            .unwrap()
            .room;
        c.join(&room_id).await.unwrap();
        let mut guest = new_core(MemoryStore::default(), uuid::Uuid::new_v4());
        guest.select(BASE, SERVICE, false).await.unwrap();
        guest.mock_approve("guest").await.unwrap();
        assert_ne!(
            guest.desktop.as_ref().unwrap().account,
            c.desktop.as_ref().unwrap().account
        );
        assert_eq!(guest.join(&room_id).await.err().unwrap(), "NOT_FOUND");
        assert!(guest.room.is_none());
        guest.list().await.unwrap();
        assert!(
            !guest
                .rooms
                .iter()
                .any(|r| r.room_id.as_deref() == Some(&room_id))
        );
        // Guessed full reads do not bypass preview/admission authorization.
        let mut private = guest.request(false);
        private.room_id = Some(room_id.clone());
        private.room_get = Some(wire::Empty {});
        assert_eq!(guest.desktop_rpc(private).await.err().unwrap(), "FORBIDDEN");
        let mut access = c.request(false);
        access.room_access = Some(wire::RoomAccess {
            visibility: Some("public".into()),
            admitted_account_ids: Some(vec![]),
        });
        c.desktop_rpc(access).await.unwrap();
        guest.list().await.unwrap();
        assert!(
            guest
                .rooms
                .iter()
                .any(|r| r.room_id.as_deref() == Some(&room_id) && r.join_allowed == Some(true))
        );
        let admission = guest.preview(&room_id).await.unwrap();
        assert!(guest.room.is_none());
        let mut change = c.request(false);
        change.ready = Some(wire::Ready { ready: Some(false) });
        c.desktop_rpc(change).await.unwrap();
        assert!(
            guest
                .join_admission(admission, None)
                .await
                .err()
                .unwrap()
                .contains("Room changed")
        );
        assert!(guest.room.is_none());
        guest.join(&room_id).await.unwrap();
        c.join(&room_id).await.unwrap();
        let mut observer = new_core(MemoryStore::default(), uuid::Uuid::new_v4());
        observer.select(BASE, SERVICE, false).await.unwrap();
        observer.mock_approve("observer").await.unwrap();
        observer.join(&room_id).await.unwrap();
        let mut not_ready = observer.request(false);
        not_ready.ready = Some(wire::Ready { ready: Some(false) });
        observer.desktop_rpc(not_ready).await.unwrap();
        let mut read = c.request(false);
        read.room_get = Some(wire::Empty {});
        c.desktop_rpc(read).await.unwrap();
        let selected = vec![
            c.desktop.as_ref().unwrap().id.clone(),
            guest.desktop.as_ref().unwrap().id.clone(),
        ];
        let excluded = vec![observer.desktop.as_ref().unwrap().id.clone()];
        assert_eq!(
            c.start(selected.clone(), vec![]).await.err().unwrap(),
            "ROSTER_MISMATCH"
        );
        c.start(selected, excluded).await.unwrap();
        assert_eq!(c.room.as_ref().unwrap().state.as_deref(), Some("preparing"));
        assert!(c.preparation().unwrap_err().contains("simulated"));
        // Explicitly simulated native facts: never use fixture version IDs in real provisioning.
        tokio::time::sleep(Duration::from_millis(1050)).await;
        let room = c.room.as_ref().unwrap();
        let mut p = handoff::new(
            424242,
            room.run_id.clone().unwrap(),
            room.config_revision.unwrap() as u64,
        );
        p.epoch = room.run_epoch.clone().unwrap();
        p.owner = Some((nt2_local_ws::owner::Id::OsProcess(123), [7; 16]));
        p.accepted = true;
        p.loaded = true;
        p.last_seen = Some(Instant::now());
        *pending.lock().unwrap() = Some(p.clone());
        control(serde_json::json!({"drop":"gameAttach"})).await;
        assert!(c.sync(&pending).await.is_err());
        assert!(c.game.is_none());
        let attachment_request = c.attach_retry.as_ref().unwrap().request.clone();
        c.sync(&pending).await.unwrap();
        assert!(c.attach_retry.is_none());
        let audit: serde_json::Value = reqwest::get(format!("{BASE}/fixture/attachment-audit"))
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        let calls = audit.as_array().unwrap();
        assert_eq!(calls.len(), 2);
        assert_eq!(
            calls[0]["requestId"],
            attachment_request.request_id.clone().unwrap()
        );
        assert_eq!(calls[0]["requestId"], calls[1]["requestId"]);
        assert_eq!(calls[0]["bodySha256"], calls[1]["bodySha256"]);
        assert_eq!(calls[0]["sessionId"], calls[1]["sessionId"]);
        assert_eq!(calls[0]["attachmentId"], calls[1]["attachmentId"]);
        assert_eq!(calls[1]["sessionId"], c.game.as_ref().unwrap().session.id);
        assert_eq!(
            c.game.as_ref().unwrap().session.device,
            c.device.to_string()
        );
        assert_eq!(
            c.game.as_ref().unwrap().session.account,
            c.desktop.as_ref().unwrap().account
        );
        let mut read = guest.request(false);
        read.room_get = Some(wire::Empty {});
        guest.desktop_rpc(read).await.unwrap();
        let mut p2 = p.clone();
        p2.owner = Some((nt2_local_ws::owner::Id::OsProcess(124), [9; 16]));
        let guest_pending = Mutex::new(Some(p2.clone()));
        guest
            .attach(
                &p2,
                &guest_pending,
                guest.info.as_ref().unwrap().compatibility.clone().unwrap(),
            )
            .await
            .unwrap();
        assert_ne!(
            c.game.as_ref().unwrap().session.id,
            guest.game.as_ref().unwrap().session.id
        );
        let mut read = c.request(false);
        read.room_get = Some(wire::Empty {});
        c.desktop_rpc(read).await.unwrap();
        c.fact(false).await.unwrap();
        assert_eq!(c.room.as_ref().unwrap().state.as_deref(), Some("preparing"));
        let mut read = guest.request(false);
        read.room_get = Some(wire::Empty {});
        guest.desktop_rpc(read).await.unwrap();
        guest.fact(false).await.unwrap();
        assert_eq!(
            guest.room.as_ref().unwrap().state.as_deref(),
            Some("loading")
        );
        let mut read = c.request(false);
        read.room_get = Some(wire::Empty {});
        c.desktop_rpc(read).await.unwrap();
        c.fact(true).await.unwrap();
        assert_eq!(c.room.as_ref().unwrap().state.as_deref(), Some("loading"));
        let mut read = guest.request(false);
        read.room_get = Some(wire::Empty {});
        guest.desktop_rpc(read).await.unwrap();
        guest.fact(true).await.unwrap();
        assert_eq!(
            guest.room.as_ref().unwrap().state.as_deref(),
            Some("running")
        );
        let mut read = c.request(false);
        read.room_get = Some(wire::Empty {});
        c.desktop_rpc(read).await.unwrap();
        // A second room cannot receive facts/reads from an attached game bound to the first.
        let old_observer_room = observer.room.clone();
        observer.fixture_room().await.unwrap();
        let second = observer.room.as_ref().unwrap().room_id.clone();
        let mut foreign = c.request(true);
        foreign.room_id = second;
        foreign.room_get = Some(wire::Empty {});
        let token = SecretString::from(c.game.as_ref().unwrap().session.access.as_str());
        assert_eq!(
            c.send(&foreign, Some(token.as_str())).await.err().unwrap(),
            "FORBIDDEN"
        );
        observer.room = old_observer_room;
        *pending.lock().unwrap() = Some(p.clone());
        let game_epoch = c.game.as_ref().unwrap().session.epoch.clone();
        control(serde_json::json!({"drop":"gameResume"})).await;
        assert!(c.resume().await.is_err());
        c.resume().await.unwrap();
        assert_ne!(c.game.as_ref().unwrap().session.epoch, game_epoch);
        // New world nonce cannot renew old context. Pause independence is supplied by native heartbeats.
        pending.lock().unwrap().as_mut().unwrap().owner = Some((nt2_local_ws::owner::Id::OsProcess(123), [8; 16]));
        assert!(c.sync(&pending).await.is_err());
        assert!(c.game.is_none());
        assert!(pending.lock().unwrap().is_none());
        // Service restart invalidates old game facts and rotates run epoch; no auto replay.
        control(serde_json::json!({"restart":true})).await;
        assert!(c.sync(&pending).await.is_err());
        assert!(c.game.is_none());
        assert!(c.room.is_none());
        c.rotate().await.unwrap();
        c.list().await.unwrap();
        assert!(
            c.rooms
                .iter()
                .any(|r| r.room_id.as_deref() == Some(&room_id)
                    && r.state.as_deref() == Some("suspended"))
        );
        let mut read = c.request(false);
        read.room_id = Some(room_id.clone());
        read.room_get = Some(wire::Empty {});
        c.desktop_rpc(read).await.unwrap();
        let mut end = c.request(false);
        end.end_run = Some(wire::Empty {});
        c.desktop_rpc(end).await.unwrap();
        assert_eq!(c.room.as_ref().unwrap().state.as_deref(), Some("ended"));
        c.join(&room_id).await.unwrap();
        let me = c
            .room
            .as_ref()
            .unwrap()
            .members
            .as_ref()
            .unwrap()
            .iter()
            .find(|m| m.desktop_id.as_deref() == Some(&c.desktop.as_ref().unwrap().id))
            .unwrap();
        assert_eq!(me.present, Some(true));
        assert_eq!(me.ready, Some(true));
        // Real Core::sync owns attachment recovery. Never retry a committed attachment in a new scope.
        for failure in [
            "manual_refresh",
            "automatic_refresh",
            "nonce",
            "cancel",
            "deadline",
            "run",
            "restart",
            "late_cancel",
            "late_nonce",
        ] {
            tokio::time::sleep(Duration::from_millis(1050)).await;
            c.join(&room_id).await.unwrap();
            let own = c.desktop.as_ref().unwrap().id.clone();
            let excluded = c
                .room
                .as_ref()
                .unwrap()
                .members
                .as_ref()
                .unwrap()
                .iter()
                .filter_map(|m| m.desktop_id.clone())
                .filter(|id| id != &own)
                .collect();
            c.start(vec![own], excluded).await.unwrap();
            let room = c.room.as_ref().unwrap();
            let mut local = handoff::new(
                424242,
                room.run_id.clone().unwrap(),
                room.config_revision.unwrap() as u64,
            );
            local.epoch = room.run_epoch.clone().unwrap();
            local.owner = Some((nt2_local_ws::owner::Id::OsProcess(321), [10; 16]));
            local.last_seen = Some(Instant::now());
            *pending.lock().unwrap() = Some(local);
            let before: Vec<serde_json::Value> =
                reqwest::get(format!("{BASE}/fixture/attachment-audit"))
                    .await
                    .unwrap()
                    .json()
                    .await
                    .unwrap();
            if failure.starts_with("late_") {
                control(serde_json::json!({"delay":"gameAttach"})).await;
                let generation = c.generation.clone();
                let mutation_pending = pending.clone();
                let before_count = before.len();
                let mutate = tauri::async_runtime::spawn(async move {
                    for _ in 0..100 {
                        let audit: Vec<serde_json::Value> =
                            reqwest::get(format!("{BASE}/fixture/attachment-audit"))
                                .await
                                .unwrap()
                                .json()
                                .await
                                .unwrap();
                        if audit.len() > before_count {
                            if failure == "late_cancel" {
                                generation.fetch_add(1, Ordering::AcqRel);
                                *mutation_pending.lock().unwrap() = None;
                            } else {
                                mutation_pending.lock().unwrap().as_mut().unwrap().owner =
                                    Some((nt2_local_ws::owner::Id::OsProcess(322), [11; 16]));
                            }
                            return;
                        }
                        tokio::time::sleep(Duration::from_millis(5)).await;
                    }
                    panic!("Attachment did not commit before the bounded observation deadline");
                });
                let result = c.sync(&pending).await;
                mutate.await.unwrap();
                assert!(result.is_err(), "{failure}");
            } else {
                control(serde_json::json!({"drop":"gameAttach"})).await;
                assert!(c.sync(&pending).await.is_err());
                assert!(c.attach_retry.is_some());
                if failure == "manual_refresh" || failure == "automatic_refresh" {
                    let retry = c.attach_retry.as_ref().unwrap();
                    let original_request = serde_json::to_vec(&retry.request).unwrap();
                    let original_deadline = retry.deadline;
                    let desktop = c.desktop.as_ref().unwrap();
                    let identity = (
                        desktop.id.clone(),
                        desktop.epoch.clone(),
                        desktop.account.clone(),
                        desktop.device.clone(),
                    );
                    let original_access = SecretString::from(desktop.access.as_str());
                    if failure == "manual_refresh" {
                        c.rotate().await.unwrap();
                    } else {
                        // Only force the client-side renewal threshold. Server issuance time stays valid.
                        c.desktop.as_mut().unwrap().expires = now() + 59000;
                        control(serde_json::json!({"drop":"gameAttach"})).await;
                        assert!(c.sync(&pending).await.is_err());
                    }
                    let desktop = c.desktop.as_ref().unwrap();
                    assert_eq!(
                        (
                            desktop.id.clone(),
                            desktop.epoch.clone(),
                            desktop.account.clone(),
                            desktop.device.clone()
                        ),
                        identity
                    );
                    assert!(desktop.access.as_str() != original_access.as_str());
                    assert!(desktop.expires > now() + 60000);
                    let retry = c.attach_retry.as_ref().unwrap();
                    assert_eq!(retry.deadline, original_deadline);
                    assert_eq!(
                        serde_json::to_vec(&retry.request).unwrap(),
                        original_request
                    );
                    c.sync(&pending).await.unwrap();
                    assert!(c.game.is_some());
                    assert!(c.attach_retry.is_none());
                    assert_eq!(c.game.as_ref().unwrap().owner, (crate::local_owner::Id::os_process(321), [10; 16]));
                    assert_eq!(c.game.as_ref().unwrap().session.account, identity.2);
                    assert_eq!(c.game.as_ref().unwrap().session.device, identity.3);
                    assert!(pending.lock().unwrap().is_some());
                    c.list().await.unwrap(); // Fresh desktop authentication remains valid after recovery.
                    let after: Vec<serde_json::Value> =
                        reqwest::get(format!("{BASE}/fixture/attachment-audit"))
                            .await
                            .unwrap()
                            .json()
                            .await
                            .unwrap();
                    let attempts = &after[before.len()..];
                    assert_eq!(
                        attempts.len(),
                        if failure == "manual_refresh" { 2 } else { 3 }
                    );
                    for call in attempts {
                        assert!(call["error"].is_null());
                        assert_eq!(call["requestId"], attempts[0]["requestId"]);
                        assert_eq!(call["bodySha256"], attempts[0]["bodySha256"]);
                        assert_eq!(call["sessionId"], attempts[0]["sessionId"]);
                        assert_eq!(call["attachmentId"], attempts[0]["attachmentId"]);
                    }
                    assert_eq!(
                        attempts[0]["sessionId"],
                        c.game.as_ref().unwrap().session.id
                    );
                    c.cancel(&pending).await.unwrap();
                    continue;
                }
                match failure {
                    "nonce" => {
                        pending.lock().unwrap().as_mut().unwrap().owner = Some((nt2_local_ws::owner::Id::OsProcess(322), [11; 16]))
                    }
                    "cancel" => c.cancel(&pending).await.unwrap(),
                    "deadline" => c.attach_retry.as_mut().unwrap().deadline = Instant::now(),
                    "run" => pending.lock().unwrap().as_mut().unwrap().epoch = uuid(),
                    "restart" => control(serde_json::json!({"restart":true})).await,
                    _ => unreachable!(),
                }
                if failure == "cancel" || failure == "run" {
                    c.sync(&pending).await.unwrap();
                } else {
                    assert!(c.sync(&pending).await.is_err(), "{failure}");
                }
            }
            assert!(c.game.is_none(), "{failure}");
            // Cancel after an in-flight generation fence clears the retained local recovery too.
            if failure == "late_cancel" {
                c.cancel(&pending).await.unwrap();
            }
            assert!(c.attach_retry.is_none(), "{failure}");
            assert!(pending.lock().unwrap().is_none(), "{failure}");
            let after: Vec<serde_json::Value> =
                reqwest::get(format!("{BASE}/fixture/attachment-audit"))
                    .await
                    .unwrap()
                    .json()
                    .await
                    .unwrap();
            assert_eq!(
                after.len(),
                before.len() + 1,
                "No fresh or stale retry after {failure}"
            );
            if failure == "restart" {
                c.rotate().await.unwrap();
                let mut r = c.request(false);
                r.room_id = Some(room_id.clone());
                r.room_get = Some(wire::Empty {});
                c.desktop_rpc(r).await.unwrap();
                assert_eq!(c.room.as_ref().unwrap().state.as_deref(), Some("waiting"));
                c.join(&room_id).await.unwrap();
            } else {
                c.cancel(&pending).await.unwrap();
            }
        }
        // New same-device login supersedes old session. Old access fails; local logout still clears store.
        tauri::async_runtime::spawn_blocking(|| std::thread::sleep(Duration::from_millis(1050)))
            .await
            .unwrap();
        let mut replacement = new_core(MemoryStore::default(), device);
        replacement.select(BASE, SERVICE, false).await.unwrap();
        replacement.mock_approve("host").await.unwrap();
        assert_eq!(c.list().await.err().unwrap(), "UNAUTHENTICATED");
        c.logout(&pending).await.unwrap();
        assert!(c.desktop.is_none());
        assert!(store.bytes.lock().unwrap().is_none());
        control(serde_json::json!({"drop":"logout"})).await;
        replacement.logout(&pending).await.unwrap();
        guest.logout(&pending).await.unwrap();
        observer.logout(&pending).await.unwrap();
        // Cancellation and expiry discard proof, and an expired retry never invents a new request ID.
        let mut exp = new_core(MemoryStore::default(), uuid::Uuid::new_v4());
        exp.select(BASE, SERVICE, false).await.unwrap();
        exp.journal.expires = now() - 1;
        assert!(exp.poll().await.is_err());
        assert!(exp.journal.proof.is_empty());
        exp.journal.operation = "refresh".into();
        exp.journal.request_id = "fixed-expired-request".into();
        exp.journal.started = now() - 30001;
        assert!(exp.recover().await.is_err());
        assert_eq!(exp.journal.request_id, "fixed-expired-request");
        // Conflicting old refresh reuse revokes the family and clears locally remembered recovery material.
        let reuse_store = MemoryStore::default();
        let mut reuse = new_core(reuse_store.clone(), uuid::Uuid::new_v4());
        reuse.select(BASE, SERVICE, true).await.unwrap();
        reuse.mock_approve("host").await.unwrap();
        let old = SecretString::from(reuse.journal.refresh.as_str());
        reuse.rotate().await.unwrap();
        let access = SecretString::from(reuse.desktop.as_ref().unwrap().access.as_str());
        let mut read = reuse.request(false);
        read.room_list = Some(wire::ListRooms {
            limit: Some(32),
            after_room_id: None,
        });
        let mut conflict = reuse.request(false);
        conflict.refresh = Some(wire::Refresh {
            secret: Some(old.as_str().into()),
            device_id: Some(reuse.device.to_string()),
        });
        assert_eq!(
            reuse.send(&conflict, None).await.err().unwrap(),
            "REFRESH_REUSED"
        );
        assert!(reuse.desktop.is_none());
        assert!(reuse.journal.refresh.is_empty());
        assert_eq!(
            reuse
                .send(&read, Some(access.as_str()))
                .await
                .err()
                .unwrap(),
            "UNAUTHENTICATED"
        );
        reuse.logout(&pending).await.unwrap();
        assert!(reuse_store.bytes.lock().unwrap().is_none());
        println!(
            "PASS persisted G1 Rust lifecycle: pairing, lost exchange, refresh/restart/reuse, explicit roster, two simulated game sessions, multiple-room isolation, resume retry, context fencing, restart invalidation, supersession/logout retry, expiry; no actual-game claim."
        );
    });
}

#[test]
#[ignore = "requires isolated client-owned G2 persisted fixture on port4326"]
fn persisted_g2_lobby() {
    tauri::async_runtime::block_on(async {
        const BASE: &str = "http://127.0.0.1:4326";
        const SERVICE: &str = "nt2-client-g2-test";
        reqwest::Client::new()
            .post(format!("{BASE}/fixture/control"))
            .json(&serde_json::json!({"reset":true}))
            .send()
            .await
            .unwrap()
            .error_for_status()
            .unwrap();
        let mut host = new_g2_core(MemoryStore::default(), uuid::Uuid::new_v4());
        host.select(BASE, SERVICE, false).await.unwrap();
        host.mock_approve("host").await.unwrap();
        host.fixture_room().await.unwrap();
        let room = host.room.as_ref().unwrap().room_id.clone().unwrap();
        host.join(&room).await.unwrap();
        assert_eq!(
            host.room.as_ref().unwrap().members.as_ref().unwrap()[0].ready,
            Some(false)
        );
        host.set_ready(true).await.unwrap();
        let mut policy = host.request(false);
        policy.room_policy = Some(wire::RoomPolicy {
            capacity: Some(4),
            password: Some("fixture-password".into()),
        });
        host.desktop_rpc(policy).await.unwrap();
        let mut guest = new_g2_core(MemoryStore::default(), uuid::Uuid::new_v4());
        guest.select(BASE, SERVICE, false).await.unwrap();
        guest.mock_approve("guest").await.unwrap();
        guest.list().await.unwrap();
        assert_eq!(guest.rooms[0].password_required, Some(true));
        assert_eq!(guest.join(&room).await.err().unwrap(), "PASSWORD_REQUIRED");
        assert!(guest.room.is_none());
        guest
            .join_with_password(&room, Some("fixture-password".into()))
            .await
            .unwrap();
        guest.set_ready(true).await.unwrap();
        let attack = "<img src=x onerror=window.stolen=true>";
        guest.chat_send(attack.into()).await.unwrap();
        host.room = None; // Restore an authorized snapshot, never copy a guest's private snapshot.
        let mut read = host.request(false);
        read.room_id = Some(room.clone());
        read.room_get = Some(wire::Empty {});
        host.desktop_rpc(read).await.unwrap();
        host.chat_send("Ready when you are".into()).await.unwrap();
        guest.chat_list().await.unwrap();
        assert_eq!(guest.chat_messages.len(), 2);
        assert_ne!(
            guest.chat_messages[0].account_id,
            guest.chat_messages[1].account_id
        );
        assert_eq!(guest.chat_messages[0].text.as_deref(), Some(attack));
        let identity = (
            host.desktop.as_ref().unwrap().id.clone(),
            host.desktop.as_ref().unwrap().epoch.clone(),
        );
        let old_access = SecretString::from(host.desktop.as_ref().unwrap().access.as_str());
        host.desktop.as_mut().unwrap().expires = now() + 59000;
        host.last_presence = Instant::now() - Duration::from_secs(11);
        let core = Arc::new(tokio::sync::Mutex::new(host));
        let pending = Arc::new(Mutex::new(None));
        let stop = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let background = start_background(core.clone(), pending.clone(), stop.clone());
        tokio::time::sleep(Duration::from_millis(1100)).await;
        {
            let host = core.lock().await;
            assert!(host.status_error.is_none());
            assert_eq!(
                (
                    host.desktop.as_ref().unwrap().id.clone(),
                    host.desktop.as_ref().unwrap().epoch.clone()
                ),
                identity
            );
            assert!(host.desktop.as_ref().unwrap().access.as_str() != old_access.as_str());
            assert!(host.last_presence.elapsed() < Duration::from_secs(3));
            assert_eq!(host.chat_messages.len(), 2);
        }
        stop.store(true, Ordering::Release);
        background.await.unwrap();
        assert_eq!(core.lock().await.resources.active_tasks(), 0);
        reqwest::Client::new()
            .post(format!("{BASE}/fixture/control"))
            .json(&serde_json::json!({"drop":"chatSend"}))
            .send()
            .await
            .unwrap()
            .error_for_status()
            .unwrap();
        assert!(
            guest
                .chat_send("Exact lost chat reply".into())
                .await
                .is_err()
        );
        let chat_id = guest
            .chat_retry
            .as_ref()
            .unwrap()
            .request
            .request_id
            .clone();
        guest.rotate().await.unwrap();
        guest.retry_chat().await.unwrap();
        assert!(guest.chat_retry.is_none());
        assert!(chat_id.is_some());
        guest.chat_list().await.unwrap();
        assert_eq!(
            guest
                .chat_messages
                .iter()
                .filter(|m| m.text.as_deref() == Some("Exact lost chat reply"))
                .count(),
            1
        );
        // Discovery retains a bounded page/cursor across background snapshots.
        let mut maker = new_g2_core(MemoryStore::default(), uuid::Uuid::new_v4());
        maker.select(BASE, SERVICE, false).await.unwrap();
        maker.mock_approve("observer").await.unwrap();
        for _ in 0..32 {
            maker.fixture_room().await.unwrap();
            tokio::time::sleep(Duration::from_millis(55)).await;
        }
        guest.list().await.unwrap();
        assert_eq!(guest.rooms.len(), 32);
        assert!(guest.next_room_id.is_some());
        let first: std::collections::HashSet<_> = guest
            .rooms
            .iter()
            .filter_map(|r| r.room_id.clone())
            .collect();
        guest.room_page(true).await.unwrap();
        assert_eq!(guest.rooms.len(), 1);
        assert!(!first.contains(guest.rooms[0].room_id.as_ref().unwrap()));
        assert!(guest.next_room_id.is_none());
        guest.room_page(false).await.unwrap();
        assert_eq!(guest.rooms.len(), 32);
        // Authoritative snapshots reach selected guests; simulated profiles cannot arm native code.
        let guest_id = guest.desktop.as_ref().unwrap().id.clone();
        let host_id = core.lock().await.desktop.as_ref().unwrap().id.clone();
        core.lock()
            .await
            .start(vec![guest_id], vec![host_id])
            .await
            .unwrap();
        let guest_pending = Mutex::new(None);
        assert!(
            guest
                .sync(&guest_pending)
                .await
                .err()
                .unwrap()
                .contains("simulated")
        );
        assert!(guest_pending.lock().unwrap().is_none());
        core.lock().await.sync(&pending).await.unwrap();
        assert!(pending.lock().unwrap().is_none());
        guest.clear_game(&guest_pending);
        guest.sync(&guest_pending).await.unwrap();
        assert!(guest_pending.lock().unwrap().is_none());
        core.lock().await.cancel(&pending).await.unwrap();
        guest.sync(&guest_pending).await.unwrap();
        guest.join(&room).await.unwrap();
        guest.set_ready(true).await.unwrap();
        {
            let mut host = core.lock().await;
            host.sync(&pending).await.unwrap();
            host.set_ready(true).await.unwrap();
            let mut start = host.request(false);
            start.session_epoch = Some(host.browser.as_ref().unwrap().epoch.clone());
            start.start = Some(wire::Start {
                participant_desktop_ids: Some(vec![
                    host.desktop.as_ref().unwrap().id.clone(),
                    guest.desktop.as_ref().unwrap().id.clone(),
                ]),
                excluded_desktop_ids: Some(vec![]),
            });
            let token = SecretString::from(host.browser.as_ref().unwrap().access.as_str());
            host.room = host.send(&start, Some(token.as_str())).await.unwrap().room;
            // Explicit simulated native callbacks. Production preparation still refuses this profile.
            let room = host.room.as_ref().unwrap();
            let mut p = handoff::new(
                424242,
                room.run_id.clone().unwrap(),
                room.config_revision.unwrap() as u64,
            );
            p.epoch = room.run_epoch.clone().unwrap();
            p.native_v2 = true;
            p.owner = Some((nt2_local_ws::owner::Id::OsProcess(123), [7; 16]));
            p.accepted = true;
            p.last_seen = Some(Instant::now());
            *pending.lock().unwrap() = Some(p.clone());
            p.owner = Some((nt2_local_ws::owner::Id::OsProcess(124), [9; 16]));
            *guest_pending.lock().unwrap() = Some(p);
            host.sync(&pending).await.unwrap();
            assert!(!pending.lock().unwrap().as_ref().unwrap().release);
        }
        guest.sync(&guest_pending).await.unwrap();
        assert!(!guest_pending.lock().unwrap().as_ref().unwrap().release);
        pending.lock().unwrap().as_mut().unwrap().loaded = true;
        core.lock().await.sync(&pending).await.unwrap();
        assert!(!pending.lock().unwrap().as_ref().unwrap().release);
        guest_pending.lock().unwrap().as_mut().unwrap().loaded = true;
        guest.sync(&guest_pending).await.unwrap();
        assert!(guest_pending.lock().unwrap().as_ref().unwrap().release);
        core.lock().await.sync(&pending).await.unwrap();
        assert!(pending.lock().unwrap().as_ref().unwrap().release);
        reqwest::Client::new()
            .post(format!("{BASE}/fixture/control"))
            .json(&serde_json::json!({"drop":"roomGet"}))
            .send()
            .await
            .unwrap()
            .error_for_status()
            .unwrap();
        assert!(core.lock().await.sync(&pending).await.is_err());
        assert!(!pending.lock().unwrap().as_ref().unwrap().release);
        core.lock().await.sync(&pending).await.unwrap();
        assert!(pending.lock().unwrap().as_ref().unwrap().release);
        {
            let mut host = core.lock().await;
            let mut end = host.request(false);
            end.end_run = Some(wire::Empty {});
            host.desktop_rpc(end).await.unwrap();
            host.clear_game(&pending);
        }
        guest.sync(&guest_pending).await.unwrap();
        assert!(guest_pending.lock().unwrap().is_none());
        guest.logout(&pending).await.unwrap();
        maker.logout(&pending).await.unwrap();
        core.lock().await.logout(&pending).await.unwrap();
        println!(
            "PASS G2 Core lobby: independent pairing/admission/password/readiness/chat, bounded paginated discovery, background renewal/presence without renderer calls; no actual-game claim."
        );
    });
}
