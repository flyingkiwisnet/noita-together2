//! Isolated actual WebView test. No service, credentials, game pipe or installed files.
use serde_json::{Value, json};
fn snapshot() -> Value {
    json!({"profile_selected":true,"signed_in":true,"account_id":"host","desktop_id":"desktop-one","device_id":"device-one","website":"https://fixture.invalid","game_path":"original folder","activation_code":null,"seed":null,"handoff":"fixture","limits":"isolated renderer fixture","protocol_revision":"g2-draft-1","history":[],"rooms":[],"chat_messages":[{"sequence":1,"accountId":"host","text":"selectable text"}],"room":{"roomId":"room-one","name":"fixture room","state":"waiting","hostAccountId":"host","configRevision":1,"members":[{"desktopId":"desktop-one","accountId":"host","present":true,"ready":false}]}})
}
#[tauri::command]
fn g1_status() -> Value {
    snapshot()
}
#[tauri::command]
fn g1_login() -> Value {
    snapshot()
}
#[tauri::command]
fn g1_logout() -> Value {
    snapshot()
}
#[tauri::command]
fn g1_action(app: tauri::AppHandle, input: Value) -> Value {
    if input["action"] == "probe_results" {
        let passed = input["passed"].as_bool() == Some(true);
        let path = std::env::var_os("NT2_RENDERER_PROBE_RESULT").expect("test result path");
        std::fs::write(path, serde_json::to_vec(&input).unwrap()).expect("write test result");
        app.exit(if passed { 0 } else { 1 });
    }
    snapshot()
}
fn main() {
    let profile = std::env::var_os("NT2_RENDERER_PROBE_PROFILE").expect("isolated test profile");
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            g1_status, g1_login, g1_action, g1_logout
        ])
        .setup(move |app| {
            tauri::WebviewWindowBuilder::from_config(app, &app.config().app.windows[0])?
                .title("NT2 isolated renderer verification")
                .visible(false)
                .data_directory(profile.clone().into())
                .initialization_script(include_str!("../../tools/renderer-webview-probe.js"))
                .build()?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("isolated WebView runtime");
}
