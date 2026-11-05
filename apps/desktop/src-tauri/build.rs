// Bind discovery/request identity to the exact canonical bytes compiled below.
fn social_contract_identity() {
    use sha2::{Digest, Sha256};
    let root = std::path::PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap())
        .join("../../../packages/contracts/g3-social");
    for (file, field, value_env, hash_env) in [
        ("schema.json", "revision", "NT2_G3_SOCIAL_REVISION", "NT2_G3_SOCIAL_SCHEMA_SHA"),
        ("replay-policy.json", "policy", "NT2_G3_SOCIAL_POLICY", "NT2_G3_SOCIAL_POLICY_SHA"),
    ] {
        let path = root.join(file);
        println!("cargo:rerun-if-changed={}", path.display());
        let bytes = std::fs::read(path).expect("Canonical social contract is missing");
        let value: serde_json::Value = serde_json::from_slice(&bytes).expect("Canonical social contract is invalid");
        let text = value[field].as_str().expect("Canonical social identity is missing");
        assert!(!text.is_empty() && !text.chars().any(char::is_control), "Invalid canonical social identity");
        println!("cargo:rustc-env={value_env}={text}");
        println!("cargo:rustc-env={hash_env}={:x}", Sha256::digest(bytes));
    }
}
fn main() {
    social_contract_identity();
    // One canonical presentation vocabulary, served as an unchanged ESM asset.
    let desktop = std::path::PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let hints = desktop.join("../../../packages/contracts/src/lobby-setting-hints.js");
    let asset = desktop.join("../ui/lobby-setting-hints.js");
    println!("cargo:rerun-if-changed={}", hints.display());
    let bytes = std::fs::read(&hints).expect("Canonical lobby setting hints missing");
    if std::fs::read(&asset).ok().as_deref() != Some(bytes.as_slice()) {
        std::fs::write(&asset, bytes).expect("Cannot copy lobby setting hints asset");
    }
    for (source,destination) in [
        ("src/race-goal-ui.js","race-goal-ui.js"),
        ("src/race-goal-editor.js","race-goal-editor.js"),
        ("g3-race-goals/schema.json","race-goal-schema.json"),
        ("g3-race-goals/catalogue.json","race-goal-catalogue.json"),
    ] {
        let source=desktop.join("../../../packages/contracts").join(source);
        println!("cargo:rerun-if-changed={}",source.display());
        let bytes=std::fs::read(source).expect("Canonical Race presentation asset missing");
        let destination=desktop.join("../ui").join(destination);
        if std::fs::read(&destination).ok().as_deref()!=Some(bytes.as_slice()) {std::fs::write(destination,bytes).expect("Cannot copy canonical Race presentation asset");}
    }
    // Catalogue data uses the script asset path: the desktop CSP deliberately
    // permits IPC connections only, so same-origin JSON fetch is unavailable.
    let schema=std::fs::read(desktop.join("../../../packages/contracts/g3-race-goals/schema.json")).unwrap();
    let content=std::fs::read(desktop.join("../../../packages/contracts/g3-race-goals/catalogue.json")).unwrap();
    let catalogue=format!("export const catalogue = {};\n",serde_json::json!({"schema":serde_json::from_slice::<serde_json::Value>(&schema).unwrap(),"content":serde_json::from_slice::<serde_json::Value>(&content).unwrap()}));
    let asset=desktop.join("../ui/race-goal-data.js");
    if std::fs::read_to_string(&asset).ok().as_deref()!=Some(&catalogue){std::fs::write(asset,catalogue).unwrap();}
    // MSVC examples below use Tauri's resource, including common-controls v6.
    // Keep the existing explicit manifest fallback for other Windows toolchains.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows")
        && std::env::var("CARGO_CFG_TARGET_ENV").as_deref() != Ok("msvc")
    {
        println!("cargo:rustc-link-arg-examples=/MANIFEST:EMBED");
        let manifest = std::path::PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("examples/renderer-probe.manifest");
        println!("cargo:rustc-link-arg-examples=/MANIFESTINPUT:{}", manifest.display());
        println!("cargo:rerun-if-changed=examples/renderer-probe.manifest");
    }
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&[
            "g1_status",
            "g1_login",
            "g1_action",
            "g1_logout",
            "g3_original_status",
            "g1_browse_game",
            "g1_beta_setup",
            "g1_beta_discover",
            "g1_beta_export",
        ]),
    ))
    .expect("Tauri build configuration");
    // tauri-build / embed-resource emit rustc-link-arg-bins for this resource.
    // The distributed interactive client is a Cargo example, so link the same
    // generated icon/version resource explicitly into that target as well.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows")
        && std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc")
    {
        let resource = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap())
            .join("resource.lib");
        assert!(resource.is_file(), "Tauri Windows resource is missing");
        println!("cargo:rustc-link-arg-examples={}", resource.display());
        println!("cargo:rerun-if-changed=icons/icon.ico");
    }
}
