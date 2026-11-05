//! Local resource ownership only. This module never grants admission or invents server recovery policy.
use crate::secrets::{CredentialStore, SecretBlob, ServiceProfile, StoreError};
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};
const MAX_TASKS: usize = 16;

struct Scope {
    id: uuid::Uuid,
    cancelled: AtomicBool,
    active: AtomicUsize,
}
pub struct TaskLease {
    scope: Arc<Scope>,
    total: Arc<AtomicUsize>,
}
impl TaskLease {
    pub fn cancelled(&self) -> bool {
        self.scope.cancelled.load(Ordering::Acquire)
    }
}
impl Drop for TaskLease {
    fn drop(&mut self) {
        self.scope.active.fetch_sub(1, Ordering::AcqRel);
        self.total.fetch_sub(1, Ordering::AcqRel);
    }
}
#[derive(Debug, PartialEq, Eq)]
pub enum LocalError {
    NoProfile,
    Cancelled,
    Busy,
    Stale,
    Storage(StoreError),
}
impl From<StoreError> for LocalError {
    fn from(e: StoreError) -> Self {
        Self::Storage(e)
    }
}

pub struct SessionResources<S: CredentialStore> {
    store: S,
    profile: Option<ServiceProfile>,
    scope: Option<Arc<Scope>>,
    total: Arc<AtomicUsize>,
}
impl<S: CredentialStore> SessionResources<S> {
    pub fn new(store: S) -> Self {
        Self {
            store,
            profile: None,
            scope: None,
            total: Arc::new(AtomicUsize::new(0)),
        }
    }
    pub fn select(&mut self, profile: ServiceProfile) {
        self.dispose();
        self.profile = Some(profile);
        self.scope = Some(Arc::new(Scope {
            id: uuid::Uuid::new_v4(),
            cancelled: AtomicBool::new(false),
            active: AtomicUsize::new(0),
        }));
    }
    pub fn task(&self) -> Result<TaskLease, LocalError> {
        let scope = self.scope.as_ref().ok_or(LocalError::NoProfile)?;
        if scope.cancelled.load(Ordering::Acquire) {
            return Err(LocalError::Cancelled);
        }
        self.total
            .try_update(Ordering::AcqRel, Ordering::Acquire, |n| {
                (n < MAX_TASKS).then_some(n + 1)
            })
            .map_err(|_| LocalError::Busy)?;
        scope.active.fetch_add(1, Ordering::AcqRel);
        Ok(TaskLease {
            scope: scope.clone(),
            total: self.total.clone(),
        })
    }
    pub fn require_current(&self, lease: &TaskLease) -> Result<(), LocalError> {
        if lease.cancelled() || self.scope.as_ref().is_none_or(|s| s.id != lease.scope.id) {
            return Err(LocalError::Stale);
        }
        Ok(())
    }
    pub fn remember(&self, lease: &TaskLease, secret: &SecretBlob) -> Result<(), LocalError> {
        self.require_current(lease)?;
        self.store
            .save(self.profile.as_ref().ok_or(LocalError::NoProfile)?, secret)?;
        Ok(())
    }
    pub fn remembered(&self) -> Result<Option<SecretBlob>, LocalError> {
        self.store
            .load(self.profile.as_ref().ok_or(LocalError::NoProfile)?)
            .map_err(Into::into)
    }
    // Local deletion never claims remote revocation. The reviewed auth adapter supplies that separately.
    pub fn clear_local(&mut self) -> Result<(), LocalError> {
        let profile = self.profile.take();
        self.dispose();
        if let Some(p) = profile {
            self.store.delete(&p)?
        }
        Ok(())
    }
    pub fn dispose(&mut self) {
        if let Some(s) = self.scope.take() {
            s.cancelled.store(true, Ordering::Release)
        }
        self.profile = None;
    }
    #[cfg(test)]
    pub fn active_tasks(&self) -> usize {
        self.scope
            .as_ref()
            .map_or(0, |s| s.active.load(Ordering::Acquire))
    }
}
impl<S: CredentialStore> Drop for SessionResources<S> {
    fn drop(&mut self) {
        self.dispose();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    #[derive(Default)]
    struct Store {
        value: Mutex<Option<Vec<u8>>>,
        fail_delete: bool,
    }
    impl CredentialStore for Store {
        fn save(&self, _: &ServiceProfile, s: &SecretBlob) -> Result<(), StoreError> {
            *self.value.lock().unwrap() = Some(s.bytes().to_vec());
            Ok(())
        }
        fn load(&self, _: &ServiceProfile) -> Result<Option<SecretBlob>, StoreError> {
            self.value
                .lock()
                .unwrap()
                .as_ref()
                .map(|s| SecretBlob::new(s.clone()))
                .transpose()
        }
        fn delete(&self, _: &ServiceProfile) -> Result<(), StoreError> {
            if self.fail_delete {
                return Err(StoreError::Unavailable);
            }
            *self.value.lock().unwrap() = None;
            Ok(())
        }
    }
    fn profile() -> ServiceProfile {
        ServiceProfile::new("https://fixture.invalid", "fixture", uuid::Uuid::new_v4()).unwrap()
    }
    #[test]
    fn hundred_cycles_dispose_tasks_and_reject_late_commits() {
        let mut m = SessionResources::new(Store::default());
        for _ in 0..100 {
            m.select(profile());
            let task = m.task().unwrap();
            assert_eq!(m.active_tasks(), 1);
            m.dispose();
            assert!(task.cancelled());
            m.select(profile());
            assert_eq!(m.require_current(&task), Err(LocalError::Stale));
            assert_eq!(m.active_tasks(), 0);
            drop(task);
            m.dispose();
            assert_eq!(m.active_tasks(), 0);
        }
    }
    #[test]
    fn bounded_work_and_clear_local() {
        let mut m = SessionResources::new(Store::default());
        m.select(profile());
        let tasks = (0..MAX_TASKS)
            .map(|_| m.task().unwrap())
            .collect::<Vec<_>>();
        assert!(matches!(m.task(), Err(LocalError::Busy)));
        m.remember(
            &tasks[0],
            &SecretBlob::new(b"fixture-secret".to_vec()).unwrap(),
        )
        .unwrap();
        assert!(m.remembered().unwrap().is_some());
        m.clear_local().unwrap();
        assert!(tasks.iter().all(TaskLease::cancelled));
        assert!(matches!(m.task(), Err(LocalError::NoProfile)));
    }
    #[test]
    fn storage_failure_does_not_leave_active_local_owner() {
        let mut m = SessionResources::new(Store {
            fail_delete: true,
            ..Default::default()
        });
        m.select(profile());
        let task = m.task().unwrap();
        assert_eq!(
            m.clear_local(),
            Err(LocalError::Storage(StoreError::Unavailable))
        );
        assert!(task.cancelled());
        assert!(matches!(m.task(), Err(LocalError::NoProfile)));
    }
    #[test]
    fn replacement_cannot_escape_global_work_budget() {
        let mut m = SessionResources::new(Store::default());
        m.select(profile());
        let tasks = (0..MAX_TASKS)
            .map(|_| m.task().unwrap())
            .collect::<Vec<_>>();
        m.select(profile());
        assert!(matches!(m.task(), Err(LocalError::Busy)));
        drop(tasks);
        assert!(m.task().is_ok());
    }
}
