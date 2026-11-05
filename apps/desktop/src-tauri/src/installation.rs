//! Local installation verification. No executable selection or credentials reach Lua.
#[path="beta_setup.rs"]pub(crate) mod beta;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
pub const PROFILE: &str = "{}";
pub fn root(value: &str) -> Result<PathBuf, String> {
    if value.len() > 512 || value.starts_with(r"\\") || value.contains(['\n', '\r', '\0']) {
        return Err("Choose a local Noita installation folder".into());
    }
    let path = PathBuf::from(value);
    if !path.is_absolute() {
        return Err("Installation path must be absolute".into());
    }
    let mut at = PathBuf::new();
    for part in path.components() {
        if matches!(part, std::path::Component::ParentDir) {
            return Err("Parent traversal rejected".into());
        }
        at.push(part);
        if let Ok(meta) = std::fs::symlink_metadata(&at) {
            use std::os::windows::fs::MetadataExt;
            if meta.file_attributes() & 0x400 != 0 {
                return Err("Installation reparse points rejected".into());
            }
        }
    }
    if !path.is_dir() {
        return Err("Installation folder unavailable".into());
    }
    Ok(path)
}
pub fn hash(path: &Path) -> Result<String, String> {
    use std::io::Read;
    let metadata = std::fs::symlink_metadata(path).map_err(|_| "Installation file unavailable")?;
    use std::os::windows::fs::MetadataExt;
    if !metadata.is_file()
        || metadata.file_attributes() & 0x400 != 0
        || metadata.len() > 80 * 1024 * 1024
    {
        return Err("Installation file rejected".into());
    }
    let mut reader = std::fs::File::open(path).map_err(|_| "Installation file unavailable")?;
    let mut digest = Sha256::new();
    let mut buffer = [0; 65536];
    let mut count = 0;
    loop {
        let n = reader
            .read(&mut buffer)
            .map_err(|_| "Installation read failed")?;
        if n == 0 {
            break;
        }
        count += n;
        if count > 80 * 1024 * 1024 {
            return Err("Installation file oversized".into());
        }
        digest.update(&buffer[..n]);
    }
    Ok(digest
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect())
}
pub fn verify(
    path: &Path,
    service: Option<&crate::protocol_g2::wire::CompatibilityProfile>,
) -> Result<(), String> {
    root(path.to_str().ok_or("Invalid installation path")?)?;
    let profile: serde_json::Value =
        serde_json::from_str(PROFILE).map_err(|_| "Invalid built G2 profile")?;
    if let Some(s) = service {
        if !["candidate", "candidate-engine-evidence-pending", "verified"]
            .contains(&s.evidence_status.as_deref().unwrap_or(""))
            || s.id.as_deref() != profile["id"].as_str()
        {
            return Err("Service native profile is simulated or does not match this build".into());
        }
        for (actual, key) in [
            (&s.game_exe_sha256, "gameExeSha256"),
            (&s.original_lua_sha256, "originalLuaSha256"),
            (&s.bridge_sha256, "bridgeSha256"),
            (&s.mod_sha256, "modSha256"),
        ] {
            if actual.as_deref() != profile[key].as_str() {
                return Err("Service/native artifact hashes differ".into());
            }
        }
    }
    for (file, key) in [
        ("noita.exe", "gameExeSha256"),
        ("lua51.dll", "originalLuaSha256"),
        (
            "mods/noita_together2/files/nt2_g2_bridge.dll",
            "bridgeSha256",
        ),
        ("mods/noita_together2/init.lua", "modSha256"),
    ] {
        // Verify each directory too, so junctions cannot redirect a matched leaf.
        if let Some(parent) = path.join(file).parent() {
            root(parent.to_str().ok_or("Invalid installation path")?)?;
        }
        if Some(hash(&path.join(file))?.as_str()) != profile[key].as_str() {
            return Err(format!("Installed {file} does not match this source build"));
        }
    }
    Ok(())
}
pub fn discover() -> Option<PathBuf> {
    discover_candidates().into_iter().next()
}
pub fn discover_candidates() -> Vec<PathBuf> {
    let mut paths=if cfg!(feature="beta-portable"){crate::installation::beta::discovered()}else{Vec::new()};
    paths.extend([
        r"C:\Program Files\GOG Galaxy\Games\Noita",
        r"C:\GOG Games\Noita",
        r"C:\Program Files (x86)\Steam\steamapps\common\Noita",
    ]
    .iter()
    .filter_map(|p| root(p).ok().filter(|p| p.join("noita.exe").is_file())));paths.sort();paths.dedup();paths
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_remote_relative_and_traversal() {
        for p in [
            r"\\server\share\Noita",
            "Noita",
            r"C:\Noita\..\Windows",
            "invalid\n",
        ] {
            assert!(root(p).is_err());
        }
    }
}

/// Client approvals are compiled independently of service-advertised compatibility metadata.
/// A server cannot add an executable or a Lua runtime to this list.
struct ApprovedBuild {
    label: String,
    exe: String,
    lua: String,
}
static CANDIDATE_BUILDS: std::sync::LazyLock<Vec<ApprovedBuild>> = std::sync::LazyLock::new(|| {
    let built: serde_json::Value =
        serde_json::from_str(CANDIDATE_PROFILE).expect("compiled candidate manifest");
    let variants = built["supportedBuilds"]
        .as_array()
        .expect("compiled approved pairs");
    assert!(!variants.is_empty() && variants.len() <= 8);
    variants
        .iter()
        .map(|v| ApprovedBuild {
            label: format!(
                "{} · {}",
                match v["store"].as_str().unwrap() {
                    "gog" => "GOG",
                    "steam" => "Steam",
                    _ => "Reviewed build",
                },
                v["buildIdentity"].as_str().unwrap()
            ),
            exe: v["gameExeSha256"].as_str().unwrap().into(),
            lua: v["originalLuaSha256"].as_str().unwrap().into(),
        })
        .collect()
});
pub fn approved_candidate_pair(exe: &str, lua: &str) -> Option<&'static str> {
    CANDIDATE_BUILDS
        .iter()
        .find(|v| exe == v.exe && lua == v.lua)
        .map(|v| v.label.as_str())
}
/// Only fixed public artifact names are fingerprinted. No credentials or save contents.
pub fn candidate_diagnostics(root_path: &Path) -> String {
    if root(root_path.to_str().unwrap_or("")).is_err() {
        return "Noita installation diagnostics: choose an accessible local folder without junctions".into();
    }
    let mut output = String::from("NT2 candidate installation fingerprints\n");
    for name in [
        "noita.exe",
        "lua51.dll",
        "mods/noita_together2/init.lua",
        "mods/noita_together2/files/inspection.lua",
        "mods/noita_together2/files/nt2_g2_inspect_bridge.dll",
    ] {
        let file = root_path.join(name);
        let value = file
            .parent()
            .and_then(|p| root(p.to_str()?).ok())
            .and_then(|_| hash(&file).ok());
        output.push_str(name);
        output.push_str(": ");
        output.push_str(value.as_deref().unwrap_or("unavailable or rejected path"));
        output.push('\n');
    }
    output
}
#[cfg(test)]
mod candidate_build_tests {
    use super::*;
    #[test]
    fn accepts_only_compiled_executable_runtime_pairs() {
        for build in CANDIDATE_BUILDS.iter() {
            let (label, exe, lua) = (&build.label, build.exe.as_str(), build.lua.as_str());
            assert_eq!(approved_candidate_pair(exe, lua), Some(label.as_str()));
            assert_eq!(approved_candidate_pair(exe, &"a".repeat(64)), None);
            assert_eq!(approved_candidate_pair(&"b".repeat(64), lua), None);
            assert_eq!(approved_candidate_pair(&exe.to_uppercase(), lua), None);
        }
        assert_eq!(approved_candidate_pair("", ""), None);
    }
    #[test]
    fn diagnostics_rejects_remote_paths_without_inspecting_them() {
        let value = candidate_diagnostics(Path::new(r"\\server\share\Noita"));
        assert!(value.len() < 256);
        assert!(!value.contains("server"));
    }
}

pub const CANDIDATE_PROFILE: &str = "{}";
#[cfg(test)]
pub fn candidate_test_profile() -> crate::protocol_g2_next::wire::CompatibilityProfile {
    let built: serde_json::Value = serde_json::from_str(CANDIDATE_PROFILE).unwrap();
    let mut selected = serde_json::Map::new();
    for key in [
        "id",
        "evidenceStatus",
        "gameExeSha256",
        "originalLuaSha256",
        "bridgeSha256",
        "modSha256",
        "gameFamilyId",
        "supportedBuilds",
    ] {
        selected.insert(key.into(), built[key].clone());
    }
    serde_json::from_value(selected.into()).unwrap()
}
#[cfg(test)]
mod registry_tests {
    use super::*;
    #[test]
    fn service_registry_intersects_compiled_pairs_and_never_falls_back() {
        let original = candidate_test_profile();
        for build in CANDIDATE_BUILDS.iter() {
            let (exe, lua) = (build.exe.as_str(), build.lua.as_str());
            assert!(candidate_service_pair(&original, exe, lua).is_ok());
        }
        let (exe, lua) = (
            CANDIDATE_BUILDS[0].exe.as_str(),
            CANDIDATE_BUILDS[0].lua.as_str(),
        );
        let mut legacy = original.clone();
        legacy.game_family_id = None;
        legacy.supported_builds = None;
        assert!(candidate_service_pair(&legacy, exe, lua).is_ok());
        assert!(candidate_service_pair(&legacy, CANDIDATE_BUILDS[1].exe.as_str(), lua).is_err());
        for mode in 0..7 {
            let mut wrong = original.clone();
            match mode {
                0 => wrong.game_family_id = None,
                1 => wrong.supported_builds = None,
                2 => wrong.supported_builds = Some(vec![]),
                3 => {
                    wrong.supported_builds.as_mut().unwrap()[1].game_exe_sha256 =
                        Some("a".repeat(64))
                }
                4 => {
                    wrong.supported_builds.as_mut().unwrap()[1].original_lua_sha256 =
                        Some("b".repeat(64))
                }
                5 => wrong.bridge_sha256 = Some("c".repeat(64)),
                _ => wrong.game_family_id = Some("unapproved-family".into()),
            }
            assert!(
                candidate_service_pair(&wrong, exe, lua).is_err(),
                "mode {mode}"
            );
        }
    }
}
/// Match metadata and both local hashes to compiled approvals; advertised hashes alone confer no trust.
pub fn candidate_service_pair(
    profile: &crate::protocol_g2_next::wire::CompatibilityProfile,
    exe: &str,
    lua: &str,
) -> Result<&'static str, String> {
    let built: serde_json::Value =
        serde_json::from_str(CANDIDATE_PROFILE).map_err(|_| "Invalid candidate manifest")?;
    for (value, key) in [
        (&profile.id, "id"),
        (&profile.bridge_sha256, "bridgeSha256"),
        (&profile.mod_sha256, "modSha256"),
        (&profile.game_exe_sha256, "gameExeSha256"),
        (&profile.original_lua_sha256, "originalLuaSha256"),
    ] {
        if value.as_deref() != built[key].as_str() {
            return Err(
                "Service/native artifact metadata differs from this approved client".into(),
            );
        }
    }
    if !["candidate", "candidate-engine-evidence-pending", "verified"]
        .contains(&profile.evidence_status.as_deref().unwrap_or(""))
    {
        return Err("Service native evidence is unsupported".into());
    }
    let label = approved_candidate_pair(exe, lua).ok_or(
        "Unsupported Noita executable/runtime pair; copy the diagnostic fingerprints for review",
    )?;
    match (&profile.game_family_id, &profile.supported_builds) {
        (None, None)
            if exe == built["gameExeSha256"].as_str().unwrap_or("")
                && lua == built["originalLuaSha256"].as_str().unwrap_or("") =>
        {
            Ok(label)
        }
        (Some(family), Some(variants))
            if family == built["gameFamilyId"].as_str().unwrap_or("")
                && !variants.is_empty()
                && variants.len() <= 8 =>
        {
            let advertised =
                serde_json::to_value(variants).map_err(|_| "Invalid service build registry")?;
            if advertised != built["supportedBuilds"] {
                return Err(
                    "Service supported-build registry differs from reviewed local approvals".into(),
                );
            }
            if !variants.iter().any(|v| {
                v.game_exe_sha256.as_deref() == Some(exe)
                    && v.original_lua_sha256.as_deref() == Some(lua)
            }) {
                return Err("Installed game pair is not advertised by this service".into());
            }
            Ok(label)
        }
        _ => Err("Service supported-build family/registry is missing or incompatible".into()),
    }
}
pub fn verify_candidate(
    root_path: &Path,
    profile: &crate::protocol_g2_next::wire::CompatibilityProfile,
) -> Result<&'static str, String> {
    root(root_path.to_str().ok_or("Invalid installation path")?)?;
    let exe = hash(&root_path.join("noita.exe"))?;
    let lua = hash(&root_path.join("lua51.dll"))?;
    let label = candidate_service_pair(profile, &exe, &lua)?;
    let built: serde_json::Value =
        serde_json::from_str(CANDIDATE_PROFILE).map_err(|_| "Invalid candidate manifest")?;
    for (name, key) in [
        (
            "mods/noita_together2/files/nt2_g2_inspect_bridge.dll",
            "bridgeSha256",
        ),
        ("mods/noita_together2/init.lua", "modSha256"),
        (
            "mods/noita_together2/files/inspection.lua",
            "collectorSha256",
        ),
    ] {
        let file = root_path.join(name);
        root(
            file.parent()
                .and_then(|p| p.to_str())
                .ok_or("Invalid artifact path")?,
        )?;
        if Some(hash(&file)?.as_str()) != built[key].as_str() {
            return Err(format!(
                "Wrong NT2 mod/native artifact: {name}; install the matching kit"
            ));
        }
    }
    Ok(label)
}
