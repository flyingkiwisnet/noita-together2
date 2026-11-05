//! Public client preferences only. Credential material stays in the OS store.
use crate::secrets::ServiceProfile;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecentServer {
    pub origin: String,
    pub service_id: String,
    /// Display hint only. Never proof of a valid session or account authority.
    #[serde(default,skip_serializing_if="Option::is_none")]
    pub remembered_name:Option<String>,
    #[serde(default,skip_serializing_if="Option::is_none")]
    pub has_saved_account:Option<bool>,
}
#[derive(Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    servers: Vec<RecentServer>,
}
pub struct Preferences {
    path: PathBuf,
}
/// Separate public installation preference; updating servers cannot erase it.
pub struct InstallationPreferences { path: PathBuf }
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct InstallationDocument { path: String }
impl InstallationPreferences {
    pub fn at(path: PathBuf) -> Self { Self { path } }
    pub fn load(&self) -> Result<Option<PathBuf>, String> {
        if !self.path.exists() { return Ok(None); }
        let metadata=std::fs::symlink_metadata(&self.path).map_err(|_|"Installation preference unavailable")?;
        use std::os::windows::fs::MetadataExt;
        if !metadata.is_file() || metadata.file_attributes() & 0x400 != 0 || metadata.len()>2048 {
            return Err("Installation preference rejected".into());
        }
        let value: InstallationDocument=serde_json::from_slice(&std::fs::read(&self.path).map_err(|_|"Installation preference unavailable")?)
            .map_err(|_|"Invalid installation preference")?;
        Ok(Some(installation_path(&value.path)?))
    }
    pub fn save(&self, path: &std::path::Path) -> Result<(), String> {
        let value=path.to_str().ok_or("Local installation path required")?;
        installation_path(value)?;
        let parent=self.path.parent().ok_or("Installation preference path unavailable")?;
        std::fs::create_dir_all(parent).map_err(|_|"Installation preference unavailable")?;
        if self.path.exists() {
            use std::os::windows::fs::MetadataExt;
            let metadata=std::fs::symlink_metadata(&self.path).map_err(|_|"Installation preference unavailable")?;
            if !metadata.is_file()||metadata.file_attributes()&0x400!=0||metadata.len()>2048{return Err("Installation preference rejected".into());}
        }
        let temporary=parent.join(format!("installation-{}.tmp",uuid::Uuid::new_v4()));
        use std::io::Write;
        let result=(||{
            let mut output=std::fs::OpenOptions::new().create_new(true).write(true).open(&temporary).map_err(|_|"Installation preference unavailable")?;
            output.write_all(&serde_json::to_vec(&InstallationDocument{path:value.into()}).map_err(|_|"Installation preference encoding failed")?)
                .and_then(|_|output.sync_all()).map_err(|_|"Installation preference write failed")?;
            drop(output);
            std::fs::rename(&temporary,&self.path).map_err(|_|"Installation preference replacement failed")
        })();
        if result.is_err(){let _=std::fs::remove_file(&temporary);}result.map_err(String::from)
    }
}
fn installation_path(value:&str)->Result<PathBuf,String>{
    let path=PathBuf::from(value);
    if value.len()>512 || value.starts_with(r"\\") || value.contains(['\n','\r','\0']) || !path.is_absolute()
        || path.components().any(|p|matches!(p,std::path::Component::ParentDir)) {
        return Err("Choose an absolute local Noita folder".into());
    }
    Ok(path)
}
impl Preferences {
    pub fn at(path: PathBuf) -> Self {
        Self { path }
    }
    pub fn load(&self, device: uuid::Uuid) -> Result<Vec<RecentServer>, String> {
        if !self.path.exists() {
            return Ok(vec![]);
        }
        let metadata =
            std::fs::symlink_metadata(&self.path).map_err(|_| "Server preferences unavailable")?;
        if !metadata.is_file() || metadata.file_type().is_symlink() || metadata.len() > 16384 {
            return Err("Server preferences rejected".into());
        }
        let bytes = std::fs::read(&self.path).map_err(|_| "Server preferences unavailable")?;
        let value: Document =
            serde_json::from_slice(&bytes).map_err(|_| "Invalid server preferences")?;
        validate(&value.servers, device)?;
        Ok(value.servers)
    }
    pub fn save(&self, servers: &[RecentServer], device: uuid::Uuid) -> Result<(), String> {
        validate(servers, device)?;
        let bytes = serde_json::to_vec(&Document {
            servers: servers.to_vec(),
        })
        .map_err(|_| "Server preferences encoding failed")?;
        if bytes.len() > 16384 {
            return Err("Server preferences too large".into());
        }
        let parent = self.path.parent().ok_or("Invalid preferences path")?;
        std::fs::create_dir_all(parent).map_err(|_| "Server preferences unavailable")?;
        if self.path.exists() {
            let metadata = std::fs::symlink_metadata(&self.path)
                .map_err(|_| "Server preferences unavailable")?;
            if !metadata.is_file() || metadata.file_type().is_symlink() {
                return Err("Server preferences rejected".into());
            }
        }
        let temporary = parent.join(format!("servers-{}.tmp", uuid::Uuid::new_v4()));
        use std::io::Write;
        let result = (|| {
            let mut output = std::fs::OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&temporary)
                .map_err(|_| "Server preferences unavailable")?;
            output
                .write_all(&bytes)
                .and_then(|_| output.sync_all())
                .map_err(|_| "Server preferences write failed")?;
            drop(output);
            std::fs::rename(&temporary, &self.path)
                .map_err(|_| "Server preferences replacement failed")
        })();
        if result.is_err() {
            let _ = std::fs::remove_file(&temporary);
        }
        result.map_err(String::from)
    }
}
fn validate(servers: &[RecentServer], device: uuid::Uuid) -> Result<(), String> {
    if servers.len() > 16 {
        return Err("Too many recent servers".into());
    }
    let mut seen = std::collections::HashSet::new();
    for server in servers {
        let profile = ServiceProfile::new(&server.origin, &server.service_id, device)
            .map_err(|_| "Invalid recent server")?;
        if profile.origin() != server.origin || !seen.insert(&server.origin) || server.remembered_name.as_ref().is_some_and(|s|s.len()>80||s.chars().any(char::is_control)) {
            return Err("Invalid recent server".into());
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn flow01_installation_roundtrip_history_isolation_and_invalid_recovery() {
        let root=std::env::temp_dir().join(format!("nt2-preferences-{}",uuid::Uuid::new_v4()));
        let install=InstallationPreferences::at(root.join("installation.json"));
        assert!(install.load().unwrap().is_none());
        let steam=PathBuf::from(r"C:\Noita");
        let gog=PathBuf::from(r"C:\GOG Games\Noita");
        install.save(&steam).unwrap();install.save(&gog).unwrap();assert_eq!(install.load().unwrap(),Some(gog.clone()));
        let device=uuid::Uuid::new_v4();let history=Preferences::at(root.join("servers.json"));
        history.save(&[RecentServer{remembered_name:None,has_saved_account:None,origin:"http://127.0.0.1:4321".into(),service_id:"nt2-g1-local".into()}],device).unwrap();
        assert_eq!(install.load().unwrap(),Some(gog.clone()));let original=std::fs::read(root.join("servers.json")).unwrap();
        for path in ["Noita",r"\\server\share\Noita",r"C:\Noita\..\Windows","C:\\Noita\n"]{assert!(install.save(std::path::Path::new(path)).is_err());}
        std::fs::write(&install.path,b"invalid json").unwrap();assert!(install.load().is_err());install.save(&steam).unwrap();
        assert_eq!(std::fs::read(root.join("servers.json")).unwrap(),original);
        std::fs::write(&install.path,vec![b'x';2049]).unwrap();assert!(install.load().is_err());assert!(install.save(&gog).is_err());
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn public_history_roundtrip_and_bounded_validation() {
        let root = std::env::temp_dir().join(format!("nt2-preferences-{}", uuid::Uuid::new_v4()));
        let preferences = Preferences::at(root.join("servers.json"));
        let device = uuid::Uuid::new_v4();
        let mut servers = vec![RecentServer {
            origin: "http://127.0.0.1:4321".into(),
            service_id: "nt2-g1-local".into(),
            remembered_name:None,
            has_saved_account:None,
        }];
        preferences.save(&servers, device).unwrap();
        servers[0].service_id = "changed-service".into();
        preferences.save(&servers, device).unwrap();
        assert_eq!(
            preferences.load(device).unwrap()[0].service_id,
            "changed-service"
        );
        servers[0].origin = "https://example.invalid/?credential=secret".into();
        assert!(preferences.save(&servers, device).is_err());
        std::fs::write(&preferences.path, vec![b'x'; 16385]).unwrap();
        assert!(preferences.load(device).is_err());
        std::fs::remove_file(&preferences.path).unwrap();
        std::fs::remove_dir(root).unwrap();
    }
}
