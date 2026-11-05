use super::*;
#[test]
fn endpoints() {
    assert!(endpoint("http://127.0.0.1:4317").is_ok());
    for u in [
        "http://localhost:4317",
        "http://127.0.0.1:4317/other",
        "http://user@127.0.0.1:4317",
        "https://evil.test",
    ] {
        assert!(endpoint(u).is_err())
    }
}
#[test]
fn strict_payload() {
    assert!(
        serde_json::from_str::<Login>(
            r#"{"server":"http://127.0.0.1:4317","persona":"host","shell":"x"}"#
        )
        .is_err()
    );
}
fn fixture(body: String) -> (String, std::thread::JoinHandle<()>) {
    use std::io::{Read, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let worker = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(2)))
            .unwrap();
        let mut b = [0; 4096];
        let _ = stream.read(&mut b);
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        let _ = stream.write_all(response.as_bytes());
    });
    (base, worker)
}
fn connected(base: String) -> Core {
    let mut c = new_core();
    c.base = base;
    c.service_epoch = Some("old-epoch".into());
    c.session = Some(Session {
        token: "test-only-secret".into(),
        epoch: "desktop-epoch".into(),
    });
    c.room = Some(json!({"id":"g0-desktop-room-forest"}));
    c
}
#[test]
fn service_restart_invalidates_preparation() {
    tauri::async_runtime::block_on(async {
        let (base,worker)=fixture(json!({"mock":true,"contractRevision":REV,"serviceId":SERVICE,"serviceEpoch":"new-epoch","room":{}}).to_string());
        let mut c = connected(base);
        let pending = Mutex::new(Some(handoff::new(1, "run".into(), 1)));
        assert!(
            sync_core(&mut c, &pending)
                .await
                .unwrap_err()
                .contains("Service restarted")
        );
        worker.join().unwrap();
        assert!(pending.lock().unwrap().is_none());
        assert!(c.session.is_none() && c.attachment.is_none());
    });
}
#[test]
fn service_loss_invalidates_preparation() {
    tauri::async_runtime::block_on(async {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        drop(listener);
        let mut c = connected(base);
        let pending = Mutex::new(Some(handoff::new(1, "run".into(), 1)));
        assert!(sync_core(&mut c, &pending).await.is_err());
        assert!(pending.lock().unwrap().is_none());
        assert!(c.session.is_none());
    });
}
#[test]
fn oversized_service_response_rejected() {
    tauri::async_runtime::block_on(async {
        let (base, worker) = fixture("x".repeat(65537));
        assert_eq!(
            response(new_core().http.get(base), false)
                .await
                .unwrap_err(),
            "Oversized service response"
        );
        worker.join().unwrap();
    });
}
#[test]
#[ignore = "requires a fresh G0 mock fixture on port 4318"]
fn rust_consumer_against_mock_service() {
    tauri::async_runtime::block_on(async {
        let mut c = new_core();
        begin(
            &mut c,
            endpoint("http://127.0.0.1:4318").unwrap(),
            "host".into(),
        )
        .await
        .unwrap();
        let secret = c.attempt.as_ref().unwrap().secret.clone();
        let renderer = serde_json::to_string(&view(&c, &None)).unwrap();
        assert!(!renderer.contains(&secret));
        let original_proof = proof(&c).unwrap();
        assert!(
            response(
                c.http
                    .post(format!("{}/g0/pairings/exchange", c.base))
                    .json(&proof(&c).unwrap()),
                false
            )
            .await
            .is_err()
        );
        approve(&c).await.unwrap();
        poll_pairing(&mut c).await.unwrap();
        assert!(c.attempt.is_none());
        assert_eq!(
            response(
                c.http
                    .post(format!("{}/g0/pairings/exchange", c.base))
                    .json(&original_proof),
                false
            )
            .await
            .unwrap_err(),
            "ALREADY_EXCHANGED"
        );
        let token = c.session.as_ref().unwrap().token.clone();
        assert!(
            !serde_json::to_string(&view(&c, &None))
                .unwrap()
                .contains(&token)
        );
        join(&mut c, "g0-desktop-room-forest".into()).await.unwrap();
        command(
            &mut c,
            "chat.send",
            json!({"text":"Rust paired consumer test"}),
            false,
        )
        .await
        .unwrap();
        command(&mut c, "run.start", json!({}), false)
            .await
            .unwrap();
        let mut p = preparation(&c).unwrap();
        assert_eq!(p.seed, 424242);
        assert!(
            command(&mut c, "game.loaded", json!({}), false)
                .await
                .is_err()
        );
        // Simulated local facts; no claim of real Noita integration with this revision.
        p.owner = Some((7, [8; 16]));
        p.accepted = true;
        p.loaded = true;
        let pending = Mutex::new(Some(p));
        sync_core(&mut c, &pending).await.unwrap();
        assert_eq!(c.room.as_ref().unwrap()["state"], "running");
        assert!(c.sent_seed && c.sent_loaded);
        let game_token = c.attachment.as_ref().unwrap().session.token.clone();
        assert!(
            game_token != token,
            "desktop and game credentials must differ"
        );
        let renderer = serde_json::to_string(&view(&c, &pending.lock().unwrap())).unwrap();
        assert!(
            !renderer.contains(&token)
                && !renderer.contains(&game_token)
                && !renderer.contains(&secret)
        );
        let mut other = new_core();
        begin(&mut other, c.base.clone(), "guest".into())
            .await
            .unwrap();
        let proof = proof(&other).unwrap();
        cancel_pairing(&mut other).await.unwrap();
        assert!(
            response(
                other
                    .http
                    .post(format!("{}/g0/pairings/exchange", other.base))
                    .json(&proof),
                false
            )
            .await
            .is_err()
        );
    });
}
#[test]
fn expired_pairing_discards_proof() {
    tauri::async_runtime::block_on(async {
        let(base,worker)=fixture(json!({"mock":true,"contractRevision":REV,"serviceId":SERVICE,"serviceEpoch":"old-epoch","state":"expired"}).to_string());
        let mut c = connected(base);
        c.session = None;
        c.room = None;
        c.attempt = Some(Attempt {
            id: "attempt".into(),
            device: "device".into(),
            secret: "private-proof".into(),
            code: "public-code".into(),
            persona: "host".into(),
        });
        let pending = Mutex::new(None);
        assert!(sync_core(&mut c, &pending).await.is_err());
        worker.join().unwrap();
        assert!(c.attempt.is_none());
        assert!(view(&c, &None).activation_code.is_none());
    });
}
