#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod local_bridge;
mod local_owner;
mod browser;
#[allow(dead_code)] // Successor sign-in integration awaits reviewed generated contract.
mod auth_return;
#[allow(dead_code)] // Rust-owned candidate sign-in lifecycle; reviewed wire integration follows.
mod signin;
#[cfg(test)]
#[allow(dead_code)]
mod client;
mod g1_client;
mod handoff;
mod installation;
mod lifecycle;
#[allow(dead_code)] // Candidate local framing; successor service integration follows schema review.
mod native_report;
mod pipe;
mod preferences;
mod protocol;
mod protocol_g2;
mod protocol_g2_next;
#[cfg(test)]
mod protocol_g3;
#[cfg(test)]
mod g3_movement;
#[allow(dead_code)] // Isolated candidate product entry; installed application retains its accepted Core.
mod next_client;
mod secrets;
mod transport;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use tauri::Manager;
fn main() {
    let pending = Arc::new(Mutex::new(None));
    let stop = Arc::new(AtomicBool::new(false));
    let expected_game = Arc::new(Mutex::new(None));
    let worker = pipe::start_g2(pending.clone(), stop.clone(), expected_game.clone())
        .expect("Cannot acquire NT2 local bridge");
    let network_worker = Arc::new(Mutex::new(None));
    let setup_network_worker = network_worker.clone();
    let network_stop = stop.clone();
    let app = tauri::Builder::default()
        .setup(move |app| {
            let device = g1_client::device_at(&app.path().app_config_dir()?.join("device-id"))
                .map_err(std::io::Error::other)?;
            let mut core = g1_client::new_g2_core(secrets::WindowsCredentialStore, device);
            core.installation_at(app.path().app_config_dir()?, expected_game.clone());
            // Fixed public native endpoint ownership hint; never carries session credentials.
            let local = std::env::var_os("LOCALAPPDATA")
                .ok_or_else(|| std::io::Error::from(std::io::ErrorKind::NotFound))?;
            let bridge_config = std::path::PathBuf::from(local).join("NoitaTogether2");
            std::fs::create_dir_all(&bridge_config)?;
            std::fs::write(
                bridge_config.join("bridge-app.txt"),
                std::env::current_exe()?.to_string_lossy().as_bytes(),
            )?;
            core.preferences_at(app.path().app_config_dir()?.join("servers.json"))
                .map_err(std::io::Error::other)?;
            let generation = core.generation.clone();
            let core = Arc::new(tokio::sync::Mutex::new(core));
            *setup_network_worker.lock().unwrap() = Some(g1_client::start_background(
                core.clone(),
                pending.clone(),
                network_stop.clone(),
            ));
            app.manage(g1_client::App {
                core,
                pending: pending.clone(),
                generation,
                slots: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            });
            tauri::WebviewWindowBuilder::from_config(app, &app.config().app.windows[0])?
                .on_navigation(|url| g1_client::navigation(url.as_str()))
                .build()?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            g1_client::g1_status,
            g1_client::g1_login,
            g1_client::g1_action,
            g1_client::g1_logout
        ])
        .on_page_load(|window, payload| {
            if let Some(state) = window.try_state::<g1_client::App>() {
                state.generation.fetch_add(1, Ordering::AcqRel);
                *state.pending.lock().unwrap() = None;
            }
            if payload.url().host_str() != Some("tauri.localhost")
                && payload.url().scheme() != "tauri"
            {
                let _ = window.close();
            }
        })
        .build(tauri::generate_context!())
        .expect("Tauri runtime");
    app.run(move |handle, event| {
        if matches!(event, tauri::RunEvent::Exit) {
            let state = handle.state::<g1_client::App>();
            state.generation.fetch_add(1, Ordering::AcqRel);
            *state.pending.lock().unwrap() = None;
            stop.store(true, Ordering::Relaxed);
        }
    });
    if let Some(network_worker) = network_worker.lock().unwrap().take() {
        let _ = tauri::async_runtime::block_on(network_worker);
    }
    let _ = worker.join();
}
