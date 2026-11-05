//! Rust-only credential primitives. No serde implementation, renderer command or raw target access.
use sha2::{Digest, Sha256};
use zeroize::Zeroizing;

pub const MAX_SECRET_BYTES: usize = 2560;
pub struct SecretBlob(Zeroizing<Vec<u8>>);
impl SecretBlob {
    pub fn new(bytes: Vec<u8>) -> Result<Self, StoreError> {
        let bytes = Zeroizing::new(bytes);
        if bytes.is_empty() || bytes.len() > MAX_SECRET_BYTES {
            return Err(StoreError::InvalidSecret);
        }
        Ok(Self(bytes))
    }
    pub fn bytes(&self) -> &[u8] {
        self.0.as_slice()
    }
}
impl std::fmt::Debug for SecretBlob {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SecretBlob([redacted])")
    }
}

#[derive(Clone)]
pub struct SecretString(Zeroizing<String>);
impl SecretString {
    pub fn new(value: String) -> Self {
        Self(Zeroizing::new(value))
    }
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}
impl From<String> for SecretString {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}
impl From<&str> for SecretString {
    fn from(value: &str) -> Self {
        Self::new(value.to_string())
    }
}
impl std::fmt::Debug for SecretString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SecretString([redacted])")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceProfile {
    origin: String,
    service_id: String,
    profile_id: uuid::Uuid,
}
impl ServiceProfile {
    pub fn new(origin: &str, service_id: &str, profile_id: uuid::Uuid) -> Result<Self, StoreError> {
        if origin.len() > 512
            || service_id.is_empty()
            || service_id.len() > 128
            || !service_id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
        {
            return Err(StoreError::InvalidProfile);
        }
        let url = reqwest::Url::parse(origin).map_err(|_| StoreError::InvalidProfile)?;
        if !url.username().is_empty()
            || url.password().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
            || url.path() != "/"
            || url.host_str().is_none()
        {
            return Err(StoreError::InvalidProfile);
        }
        // Remote G1 profiles require TLS; explicit loopback ports are mock-only.
        if url.scheme() != "https"
            && !(url.scheme() == "http"
                && url.host_str() == Some("127.0.0.1")
                && url.port().is_some())
        {
            return Err(StoreError::InvalidProfile);
        }
        Ok(Self {
            origin: url.origin().ascii_serialization(),
            service_id: service_id.into(),
            profile_id,
        })
    }
    pub fn origin(&self) -> &str {
        &self.origin
    }
    pub fn service_id(&self) -> &str { &self.service_id }
    pub fn profile_id(&self) -> uuid::Uuid { self.profile_id }
    fn target(&self) -> String {
        // Unambiguous length-delimited scope; no credential or personal account value in target.
        let mut hash = Sha256::new();
        for part in [
            self.origin.as_bytes(),
            self.service_id.as_bytes(),
            self.profile_id.as_bytes(),
        ] {
            hash.update((part.len() as u64).to_le_bytes());
            hash.update(part);
        }
        format!(
            "NoitaTogether2/G1/{}",
            hash.finalize()
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreError {
    InvalidProfile,
    InvalidSecret,
    Unavailable,
    Corrupt,
    Windows { operation: &'static str, code: u32, blob_bytes: usize },
}
impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Self::Windows { operation, code, blob_bytes } = self {
            return write!(f,"Windows credential {operation} failed (os={code}, blob_bytes={blob_bytes})");
        }
        f.write_str(match self {
            Self::InvalidProfile => "Invalid service profile",
            Self::InvalidSecret => "Invalid remembered credential",
            Self::Unavailable => "Windows credential store unavailable",
            Self::Corrupt => "Remembered credential is invalid; sign in again",
            Self::Windows { .. } => unreachable!(),
        })
    }
}
pub trait CredentialStore: Send + Sync {
    fn save(&self, profile: &ServiceProfile, secret: &SecretBlob) -> Result<(), StoreError>;
    fn load(&self, profile: &ServiceProfile) -> Result<Option<SecretBlob>, StoreError>;
    fn delete(&self, profile: &ServiceProfile) -> Result<(), StoreError>;
}
impl<T:CredentialStore> CredentialStore for std::sync::Arc<T>{
    fn save(&self,p:&ServiceProfile,s:&SecretBlob)->Result<(),StoreError>{(**self).save(p,s)}
    fn load(&self,p:&ServiceProfile)->Result<Option<SecretBlob>,StoreError>{(**self).load(p)}
    fn delete(&self,p:&ServiceProfile)->Result<(),StoreError>{(**self).delete(p)}
}

pub struct WindowsCredentialStore;
#[cfg(windows)]
mod windows {
    use super::*;
    use std::{ffi::c_void, ptr::null_mut};
    #[repr(C)]
    struct FileTime {
        low: u32,
        high: u32,
    }
    #[repr(C)]
    struct Credential {
        flags: u32,
        kind: u32,
        target: *mut u16,
        comment: *mut u16,
        last_written: FileTime,
        blob_size: u32,
        blob: *mut u8,
        persist: u32,
        attribute_count: u32,
        attributes: *mut c_void,
        alias: *mut u16,
        username: *mut u16,
    }
    #[link(name = "advapi32")]
    unsafe extern "system" {
        fn CredWriteW(credential: *const Credential, flags: u32) -> i32;
        fn CredReadW(
            target: *const u16,
            kind: u32,
            flags: u32,
            credential: *mut *mut Credential,
        ) -> i32;
        fn CredDeleteW(target: *const u16, kind: u32, flags: u32) -> i32;
        fn CredFree(buffer: *mut c_void);
    }
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetLastError() -> u32;
    }
    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(Some(0)).collect()
    }
    struct Owned(*mut Credential);
    impl Drop for Owned {
        fn drop(&mut self) {
            unsafe {
                if !self.0.is_null() {
                    let c = &mut *self.0;
                    if !c.blob.is_null() && (c.blob_size as usize) <= MAX_SECRET_BYTES {
                        zeroize::Zeroize::zeroize(std::slice::from_raw_parts_mut(
                            c.blob,
                            c.blob_size as usize,
                        ));
                    }
                    CredFree(self.0.cast());
                }
            }
        }
    }
    impl CredentialStore for WindowsCredentialStore {
        fn save(&self, profile: &ServiceProfile, secret: &SecretBlob) -> Result<(), StoreError> {
            let mut target = wide(&profile.target());
            let mut user = wide("NT2 Rust session core");
            let c = Credential {
                flags: 0,
                kind: 1,
                target: target.as_mut_ptr(),
                comment: null_mut(),
                last_written: FileTime { low: 0, high: 0 },
                blob_size: secret.bytes().len() as u32,
                blob: secret.bytes().as_ptr() as *mut u8,
                persist: 2,
                attribute_count: 0,
                attributes: null_mut(),
                alias: null_mut(),
                username: user.as_mut_ptr(),
            };
            if unsafe { CredWriteW(&c, 0) } == 0 {
                let code=unsafe { GetLastError() };
                return Err(StoreError::Windows { operation:"write",code,blob_bytes:secret.bytes().len() });
            }
            Ok(())
        }
        fn load(&self, profile: &ServiceProfile) -> Result<Option<SecretBlob>, StoreError> {
            let mut out = null_mut();
            if unsafe { CredReadW(wide(&profile.target()).as_ptr(), 1, 0, &mut out) } == 0 {
                let code=unsafe { GetLastError() };
                return if code == 1168 {
                    Ok(None)
                } else {
                    Err(StoreError::Windows { operation:"read",code,blob_bytes:0 })
                };
            }
            if out.is_null() {
                return Err(StoreError::Corrupt);
            }
            let out = Owned(out);
            let c = unsafe { &*out.0 };
            if c.kind != 1
                || c.blob_size == 0
                || c.blob_size as usize > MAX_SECRET_BYTES
                || c.blob.is_null()
            {
                return Err(StoreError::Corrupt);
            }
            Ok(Some(SecretBlob::new(
                unsafe { std::slice::from_raw_parts(c.blob, c.blob_size as usize) }.to_vec(),
            )?))
        }
        fn delete(&self, profile: &ServiceProfile) -> Result<(), StoreError> {
            if unsafe { CredDeleteW(wide(&profile.target()).as_ptr(), 1, 0) } == 0 {
                let code=unsafe { GetLastError() };
                if code!=1168{return Err(StoreError::Windows { operation:"delete",code,blob_bytes:0 });}
            }
            Ok(())
        }
    }
}
#[cfg(not(windows))]
impl CredentialStore for WindowsCredentialStore {
    fn save(&self, _: &ServiceProfile, _: &SecretBlob) -> Result<(), StoreError> {
        Err(StoreError::Unavailable)
    }
    fn load(&self, _: &ServiceProfile) -> Result<Option<SecretBlob>, StoreError> {
        Err(StoreError::Unavailable)
    }
    fn delete(&self, _: &ServiceProfile) -> Result<(), StoreError> {
        Err(StoreError::Unavailable)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn profile(id: uuid::Uuid) -> ServiceProfile {
        ServiceProfile::new(
            "https://nt2-credential-test.invalid",
            "nt2-credential-test",
            id,
        )
        .unwrap()
    }
    #[test]
    fn profiles_are_canonical_and_service_scoped() {
        let id = uuid::Uuid::new_v4();
        let a = ServiceProfile::new("https://EXAMPLE.test:443/", "service", id).unwrap();
        let b = ServiceProfile::new("https://example.test", "service", id).unwrap();
        assert_eq!(a.target(), b.target());
        assert_ne!(
            a.target(),
            ServiceProfile::new("https://example.test", "other", id)
                .unwrap()
                .target()
        );
        assert_ne!(
            a.target(),
            ServiceProfile::new("https://elsewhere.test", "service", id)
                .unwrap()
                .target()
        );
        for s in [
            "http://remote.test",
            "http://localhost:4317",
            "https://x.test/path",
            "https://user:secret@x.test",
            "https://x.test?token=x",
        ] {
            assert!(ServiceProfile::new(s, "service", id).is_err());
        }
    }
    #[test]
    fn secrets_are_bounded_and_redacted() {
        assert!(SecretBlob::new(vec![]).is_err());
        assert!(SecretBlob::new(vec![1; MAX_SECRET_BYTES + 1]).is_err());
        let b = SecretBlob::new(b"test-private-value".to_vec()).unwrap();
        assert!(!format!("{b:?}").contains("test-private-value"));
        let s = SecretString::new("another-private-value".into());
        assert!(!format!("{s:?}").contains(s.as_str()));
    }
    #[test]
    #[ignore = "actual Windows credential store; only a generated test entry is touched"]
    fn os_store_roundtrip_across_process() {
        let id = uuid::Uuid::new_v4();
        let p = profile(id);
        let store = WindowsCredentialStore;
        struct Cleanup(ServiceProfile);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = WindowsCredentialStore.delete(&self.0);
            }
        }
        assert!(store.load(&p).unwrap().is_none());
        let _cleanup = Cleanup(p.clone());
        store
            .save(
                &p,
                &SecretBlob::new(b"nt2-fixture-not-a-real-token".to_vec()).unwrap(),
            )
            .unwrap();
        let wrong = ServiceProfile::new(
            "https://different-credential-test.invalid",
            "nt2-credential-test",
            id,
        )
        .unwrap();
        assert!(store.load(&wrong).unwrap().is_none());
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "secrets::tests::os_child_probe", "--ignored"])
            .env("NT2_STORE_TEST_PROFILE", id.to_string())
            .status()
            .unwrap();
        assert!(status.success());
        store.delete(&p).unwrap();
        assert!(store.load(&p).unwrap().is_none());
        store.delete(&p).unwrap();
    }
    #[test]
    #[ignore = "called only by generated-entry OS storage test"]
    fn os_child_probe() {
        let id = uuid::Uuid::parse_str(
            &std::env::var("NT2_STORE_TEST_PROFILE").expect("parent test required"),
        )
        .unwrap();
        let p = profile(id);
        let stored = WindowsCredentialStore.load(&p).unwrap().unwrap();
        assert!(stored.bytes() == b"nt2-fixture-not-a-real-token");
    }
}
