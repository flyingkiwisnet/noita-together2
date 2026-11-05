//! Sole LC1 encrypted file backend. Authentication and other protected payload
//! consumers retain their existing credential store. Call only from I/O workers.
use crate::world_local::{self, Identity, Record, Store};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    fs::{self, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::Mutex,
};
use zeroize::Zeroizing;
pub const MAX_PLAINTEXT: usize = 73728;
pub const FAILURE_PREFIX: &str = "LC1 world storage: ";
pub fn is_failure(error: &str) -> bool {
    error.starts_with(FAILURE_PREFIX)
}
fn tagged(error: String) -> String {
    format!("{FAILURE_PREFIX}{error}")
}
pub(super) const MAX_CIPHER: usize = 98304;
#[cfg(windows)]
#[path = "world_store_windows.rs"]
mod os;
#[derive(Default)]
struct Stats {
    reads: u64,
    writes: u64,
    cipher_bytes: usize,
    known: BTreeSet<String>,
}
pub struct Files {
    root: Option<PathBuf>,
    fixture: Option<uuid::Uuid>,
    stats: Mutex<Stats>,
    cleanup_on_drop: bool,
    #[cfg(test)]
    fault: std::sync::atomic::AtomicUsize,
}
fn error(stage: &str, e: std::io::Error) -> String {
    format!(
        "World store {stage} failed (os={})",
        e.raw_os_error().unwrap_or(0)
    )
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
fn owner_bytes(root: &Path) -> Result<Vec<u8>, String> {
    let path = root.join("fixture-owner");
    ordinary(&path)?;
    let mut bytes = Vec::new();
    fs::File::open(path)
        .map_err(|e| error("fixture owner open", e))?
        .take(65)
        .read_to_end(&mut bytes)
        .map_err(|e| error("fixture owner read", e))?;
    if bytes.len() > 64 {
        return Err("World fixture owner bound".into());
    }
    Ok(bytes)
}
fn encode(record: &Record) -> Result<Zeroizing<Vec<u8>>, String> {
    struct Bounded(Zeroizing<Vec<u8>>);
    impl Write for Bounded {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            if self.0.len().saturating_add(bytes.len()) > MAX_PLAINTEXT {
                return Err(std::io::Error::other("World plaintext bound"));
            }
            self.0.extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut out = Bounded(Zeroizing::new(Vec::new()));
    serde_json::to_writer(&mut out, record).map_err(|_| "World record plaintext bound/encoding")?;
    Ok(out.0)
}
pub(crate) fn binding(id: &Identity, world: uuid::Uuid) -> Result<[u8; 32], String> {
    let id = id.normalized()?;
    if world.is_nil() {
        return Err("World store world invalid".into());
    }
    let bytes = serde_json::to_vec(&(
        "nt2-lc1-world-dpapi-v1",
        id.origin,
        id.service,
        id.account,
        id.device,
        world,
    ))
    .map_err(|_| "World store scope encoding")?;
    Ok(Sha256::digest(bytes).into())
}
fn ordinary(path: &Path) -> Result<(), String> {
    for ancestor in path.ancestors() {
        match fs::symlink_metadata(ancestor) {
            Ok(m) => {
                #[cfg(windows)]
                {
                    use std::os::windows::fs::MetadataExt;
                    if m.file_attributes() & 0x400 != 0 {
                        return Err("World store reparse path refused".into());
                    }
                }
                if m.file_type().is_symlink() {
                    return Err("World store symbolic path refused".into());
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(error("path metadata", e)),
        }
    }
    Ok(())
}
fn local_root(path: &Path) -> Result<(), String> {
    if !path.is_absolute() {
        return Err("World store root must be absolute".into());
    }
    #[cfg(windows)]
    {
        use std::path::{Component, Prefix};
        if !matches!(path.components().next(),Some(Component::Prefix(p)) if matches!(p.kind(),Prefix::Disk(_)|Prefix::VerbatimDisk(_)))
        {
            return Err("World store requires a local disk path".into());
        }
    }
    Ok(())
}
impl Files {
    fn configured(
        root: Option<PathBuf>,
        fixture: Option<uuid::Uuid>,
        cleanup_on_drop: bool,
    ) -> Self {
        Self {
            root,
            fixture,
            stats: Mutex::new(Stats::default()),
            cleanup_on_drop,
            #[cfg(test)]
            fault: std::sync::atomic::AtomicUsize::new(0),
        }
    }
    pub fn user() -> Self {
        Self::configured(None, None, false)
    }
    pub fn ephemeral_fixture(root: PathBuf) -> Result<Self, String> {
        let mut f = Self::fixture_new(root)?;
        f.cleanup_on_drop = true;
        Ok(f)
    }
    pub fn fixture_new(root: PathBuf) -> Result<Self, String> {
        local_root(&root)?;
        if !root.is_absolute() {
            return Err("World fixture root must be absolute".into());
        }
        ordinary(&root)?;
        if let Some(parent) = root.parent() {
            fs::create_dir_all(parent).map_err(|e| error("fixture parent", e))?;
        }
        fs::create_dir(&root).map_err(|e| error("fresh fixture root", e))?;
        let id = uuid::Uuid::new_v4();
        let mut f = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(root.join("fixture-owner"))
            .map_err(|e| error("fixture owner", e))?;
        f.write_all(id.to_string().as_bytes())
            .map_err(|e| error("fixture owner write", e))?;
        f.sync_all().map_err(|e| error("fixture owner flush", e))?;
        Ok(Self::configured(Some(root), Some(id), false))
    }
    pub fn fixture_reopen(root: PathBuf) -> Result<Self, String> {
        local_root(&root)?;
        if !root.is_absolute() {
            return Err("World fixture root must be absolute".into());
        }
        ordinary(&root)?;
        let bytes = owner_bytes(&root)?;
        let id = std::str::from_utf8(&bytes)
            .ok()
            .and_then(|s| uuid::Uuid::parse_str(s).ok())
            .ok_or("World fixture owner invalid")?;
        Ok(Self::configured(Some(root), Some(id), false))
    }
    pub fn fixture_root(&self) -> Result<PathBuf, String> {
        self.fixture.ok_or("Not a fixture world store")?;
        self.root.clone().ok_or("No fixture root".into())
    }
    fn root(&self) -> Result<PathBuf, String> {
        let root = match &self.root {
            Some(p) => p.clone(),
            None => {
                #[cfg(windows)]
                {
                    os::user_root()?
                }
                #[cfg(not(windows))]
                {
                    return Err("Windows world store unavailable".into());
                }
            }
        };
        if !root.is_absolute() {
            return Err("World store root must be absolute".into());
        }
        local_root(&root)?;
        ordinary(&root)?;
        fs::create_dir_all(&root).map_err(|e| error("root create", e))?;
        ordinary(&root)?;
        let resolved = fs::canonicalize(root).map_err(|e| error("root resolve", e))?;
        local_root(&resolved)?;
        Ok(resolved)
    }
    #[cfg(windows)]
    fn locked<T>(
        &self,
        allow_closed: bool,
        f: impl FnOnce(&Path) -> Result<T, String>,
    ) -> Result<T, String> {
        let root = self.root()?;
        let name = format!(
            "Global\\NoitaTogether2.LC1.World.{}",
            hex(&Sha256::digest(
                root.as_os_str().to_string_lossy().to_lowercase().as_bytes()
            ))
        );
        let _guard = os::lock(&name)?;
        ordinary(&root)?;
        if let Some(owner) = self.fixture {
            let bytes = owner_bytes(&root)?;
            if bytes != owner.to_string().as_bytes() {
                return Err("World fixture ownership changed".into());
            }
        }
        ordinary(&root.join("fixture-closed"))?;
        if !allow_closed && root.join("fixture-closed").exists() {
            return Err("World fixture store closed".into());
        }
        f(&root)
    }
    #[cfg(not(windows))]
    fn locked<T>(&self, _: bool, _: impl FnOnce(&Path) -> Result<T, String>) -> Result<T, String> {
        Err("Windows world store unavailable".into())
    }
    fn decode(
        &self,
        root: &Path,
        id: &Identity,
        world: uuid::Uuid,
    ) -> Result<Option<Record>, String> {
        let key = binding(id, world)?;
        let name = format!("{}.world", hex(&key));
        let path = root.join(&name);
        ordinary(&path)?;
        let file = match fs::File::open(&path) {
            Ok(f) => f,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(error("committed open", e)),
        };
        let len = file
            .metadata()
            .map_err(|e| error("committed size", e))?
            .len();
        if len == 0 || len > MAX_CIPHER as u64 {
            return Err("World committed ciphertext bound".into());
        }
        let mut cipher = Vec::new();
        file.take((MAX_CIPHER + 1) as u64)
            .read_to_end(&mut cipher)
            .map_err(|e| error("committed read", e))?;
        if cipher.len() > MAX_CIPHER {
            return Err("World committed ciphertext bound".into());
        }
        #[cfg(windows)]
        let plain = os::crypt(&cipher, &key, false)?;
        #[cfg(not(windows))]
        let plain: Zeroizing<Vec<u8>> = return Err("Windows world store unavailable".into());
        if plain.is_empty() || plain.len() > MAX_PLAINTEXT {
            return Err("World committed plaintext bound".into());
        }
        let record: Record =
            serde_json::from_slice(&plain).map_err(|_| "Protected world record corrupt")?;
        record.validate()?;
        if binding(&record.identity, record.world)? != key {
            return Err("Protected world scope differs".into());
        }
        let mut stats = self.stats.lock().unwrap();
        stats.reads += 1;
        stats.known.insert(name);
        Ok(Some(record))
    }
    fn commit(
        &self,
        root: &Path,
        id: &Identity,
        world: uuid::Uuid,
        next: &Record,
    ) -> Result<(), String> {
        let key = binding(id, world)?;
        if binding(&next.identity, next.world)? != key {
            return Err("World update scope differs".into());
        }
        let plain = encode(next)?;
        if plain.is_empty() || plain.len() > MAX_PLAINTEXT {
            return Err("World record plaintext bound".into());
        }
        #[cfg(windows)]
        let cipher = os::crypt(&plain, &key, true)?;
        #[cfg(not(windows))]
        let cipher: Zeroizing<Vec<u8>> = return Err("Windows world store unavailable".into());
        if cipher.len() > MAX_CIPHER {
            return Err("World encrypted record bound".into());
        }
        let stem = hex(&key);
        self.remove_stages(root, &stem)?;
        let stage = root.join(format!("{stem}.{}.stage", uuid::Uuid::new_v4().simple()));
        let target = root.join(format!("{stem}.world"));
        ordinary(&target)?;
        let mut cleanup = Staged(Some(stage.clone()));
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&stage)
            .map_err(|e| error("stage create", e))?;
        let written = (|| {
            self.cut(1)?;
            file.write_all(&cipher)
                .map_err(|e| error("stage write", e))?;
            self.cut(2)?;
            file.sync_all().map_err(|e| error("stage flush", e))?;
            self.cut(3)?;
            Ok::<(), String>(())
        })();
        drop(file);
        if let Err(e) = written {
            cleanup.discard().map_err(|c| format!("{e}; {c}"))?;
            return Err(e);
        }
        let publication = (|| {
            self.cut(4)?;
            #[cfg(windows)]
            os::publish(&stage, &target)?;
            Ok::<(), String>(())
        })();
        if let Err(e) = publication {
            cleanup.discard().map_err(|c| format!("{e}; {c}"))?;
            return Err(e);
        }
        cleanup.0 = None;
        self.cut(5)?;
        // A failure here is explicitly uncertain-after-publication. Never roll back
        // to an older Rejoin record; the next operation rereads this committed file.
        OpenOptions::new()
            .read(true)
            .write(true)
            .open(&target)
            .map_err(|e| error("published open", e))?
            .sync_all()
            .map_err(|e| error("published flush", e))?;
        let mut stats = self.stats.lock().unwrap();
        stats.writes += 1;
        stats.cipher_bytes = cipher.len();
        stats.known.insert(format!("{stem}.world"));
        Ok(())
    }
    fn remove_stages(&self, root: &Path, stem: &str) -> Result<(), String> {
        let mut count = 0;
        for entry in fs::read_dir(root).map_err(|e| error("stage scan", e))? {
            let entry = entry.map_err(|e| error("stage entry", e))?;
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with(&format!("{stem}.")) && name.ends_with(".stage") {
                if !managed(&name) {
                    return Err("World stage name refused".into());
                }
                count += 1;
                if count > 32 {
                    return Err("World orphan stage bound".into());
                }
                ordinary(&entry.path())?;
                fs::remove_file(entry.path()).map_err(|e| error("orphan stage cleanup", e))?;
            }
        }
        Ok(())
    }
    fn cut(&self, stage: usize) -> Result<(), String> {
        #[cfg(test)]
        if self
            .fault
            .compare_exchange(
                stage,
                0,
                std::sync::atomic::Ordering::AcqRel,
                std::sync::atomic::Ordering::Acquire,
            )
            .is_ok()
        {
            return Err(format!("Injected world publication failure stage={stage}"));
        }
        let _ = stage;
        Ok(())
    }
    pub fn snapshot(&self) -> serde_json::Value {
        let s = self.stats.lock().unwrap();
        serde_json::json!({"reads":s.reads,"commits":s.writes,"known_record_files":s.known.len(),"last_cipher_bytes":s.cipher_bytes,"credential_entries_for_worlds":0})
    }
    pub fn cleanup_fixture(&self) -> Result<serde_json::Value, String> {
        self.fixture.ok_or("World cleanup requires owned fixture")?;
        self.locked(true, |root| {
            let closed = root.join("fixture-closed");
            ordinary(&closed)?;
            let previous = match fs::File::open(&closed) {
                Ok(file) => { let mut bytes=Vec::new(); file.take(4097).read_to_end(&mut bytes).map_err(|e|error("fixture close read",e))?; if bytes.len()>4096{return Err("World fixture close receipt bound".into())} serde_json::from_slice::<serde_json::Value>(&bytes).ok().filter(|v|v["passed"]==true) },
                Err(e) if e.kind()==std::io::ErrorKind::NotFound => None,
                Err(e) => return Err(error("fixture close open",e)),
            };
            // Fence first, including cleanup paths that discover an unknown
            // file or fail midway. Independent late instances see this gate.
            let mut gate=if previous.is_none(){let mut file=OpenOptions::new().write(true).create(true).truncate(true).open(&closed).map_err(|e|error("fixture close",e))?;file.write_all(b"closed").map_err(|e|error("fixture close write",e))?;file.sync_all().map_err(|e|error("fixture close flush",e))?;Some(file)}else{None};
            let mut paths = Vec::new();
            for entry in fs::read_dir(root).map_err(|e|error("fixture file scan",e))? {
                let entry=entry.map_err(|e|error("fixture file entry",e))?;
                let name=entry.file_name().to_string_lossy().into_owned();
                if matches!(name.as_str(),"fixture-owner"|"fixture-closed"){continue}
                if !managed(&name){return Err("Unknown file in owned world fixture; cleanup stopped".into())}
                ordinary(&entry.path())?;
                if paths.len()>=4096{return Err("World fixture cleanup count bound".into())}
                paths.push(entry.path());
            }
            if let Some(previous)=previous {
                if !paths.is_empty(){return Err("Encrypted data reappeared in closed fixture".into())}
                return Ok(previous)
            }
            let mut gate=gate.take().ok_or("World fixture close state changed")?;
            for path in &paths {
                fs::remove_file(path).map_err(|e|error("fixture encrypted cleanup",e))?;
                if path.exists(){return Err("Owned encrypted world file remains".into())}
            }
            let result=serde_json::json!({"encrypted_files_removed":paths.len(),"encrypted_files_verified_absent":paths.len(),"closed_to_writes":true,"control_markers_retained":true,"passed":true});
            gate.set_len(0).map_err(|e|error("fixture receipt truncate",e))?;
            use std::io::{Seek,SeekFrom};gate.seek(SeekFrom::Start(0)).map_err(|e|error("fixture receipt seek",e))?;
            gate.write_all(&serde_json::to_vec(&result).map_err(|_|"Fixture receipt encoding")?).map_err(|e|error("fixture receipt write",e))?;
            gate.sync_all().map_err(|e|error("fixture receipt flush",e))?;
            Ok(result)
        })
    }
}
fn managed(name: &str) -> bool {
    let hexpart = |s: &str| {
        s.len() == 64
            && s.bytes()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
    };
    if let Some(stem) = name.strip_suffix(".world") {
        return hexpart(stem);
    }
    if let Some(value) = name.strip_suffix(".stage") {
        if let Some((stem, id)) = value.split_once('.') {
            return hexpart(stem) && id.len() == 32 && uuid::Uuid::parse_str(id).is_ok();
        }
    }
    false
}
impl Drop for Files {
    fn drop(&mut self) {
        if self.cleanup_on_drop {
            let _ = self.cleanup_fixture();
        }
    }
}
struct Staged(Option<PathBuf>);
impl Drop for Staged {
    fn drop(&mut self) {
        if let Some(path) = &self.0 {
            let _ = fs::remove_file(path);
        }
    }
}
impl Staged {
    fn discard(&mut self) -> Result<(), String> {
        if let Some(path) = &self.0 {
            fs::remove_file(path).map_err(|e| error("failed stage cleanup", e))?;
        }
        self.0 = None;
        Ok(())
    }
}
impl Store for Files {
    fn read(&self, id: &Identity, world: uuid::Uuid) -> Result<Option<Record>, String> {
        self.locked(false, |root| self.decode(root, id, world))
            .map_err(tagged)
    }
    fn update(
        &self,
        id: &Identity,
        world: uuid::Uuid,
        f: &mut dyn FnMut(Option<Record>) -> Result<Record, String>,
    ) -> Result<Record, String> {
        self.locked(false, |root| {
            let old = self.decode(root, id, world)?;
            let next = f(old.clone())?;
            next.validate()?;
            world_local::validate_update(old.as_ref(), &next)?;
            self.commit(root, id, world, &next)?;
            Ok(next)
        })
        .map_err(tagged)
    }
}
#[cfg(test)]
#[path = "world_store_test.rs"]
mod tests;
