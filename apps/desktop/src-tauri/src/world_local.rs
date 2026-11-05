//! LC1 protected world identity. Connection attempts replace a connection, never
//! the world. Storage is an explicit atomic protected-file boundary, separate from item
//! and gold journals. Callers serialize writers outside Core/coordinator locks.
use crate::{secrets::ServiceProfile, startup_local::Scope};
use serde::{Deserialize, Serialize};
use zeroize::Zeroize;
/// Explicit LC1 storage boundary. Implementations must serialize the complete
/// read/validation/update/publication transaction, including other processes.
pub trait Store: Send + Sync {
    fn read(&self, identity: &Identity, world: uuid::Uuid) -> Result<Option<Record>, String>;
    fn update(
        &self,
        identity: &Identity,
        world: uuid::Uuid,
        f: &mut dyn FnMut(Option<Record>) -> Result<Record, String>,
    ) -> Result<Record, String>;
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Identity {
    pub origin: String,
    pub service: String,
    pub account: String,
    pub device: uuid::Uuid,
    pub room: String,
    pub run: String,
    pub run_epoch: String,
    pub config: u64,
    pub config_digest: String,
}
impl Identity {
    pub fn normalized(&self) -> Result<Self, String> {
        self.validate()?;
        let mut id = self.clone();
        id.origin = ServiceProfile::new(&self.origin, &self.service, self.device)
            .map_err(|_| "World issuer invalid")?
            .origin()
            .into();
        Ok(id)
    }
    pub fn validate(&self) -> Result<(), String> {
        ServiceProfile::new(&self.origin, &self.service, self.device)
            .map_err(|_| "World issuer invalid")?;
        if self.device.is_nil()
            || self.config == 0
            || [&self.account, &self.room, &self.run, &self.run_epoch]
                .iter()
                .any(|s| s.is_empty() || s.len() > 128)
            || !digest(&self.config_digest)
        {
            return Err("World identity invalid".into());
        }
        Ok(())
    }
}
impl From<&Scope> for Identity {
    fn from(s: &Scope) -> Self {
        Self {
            origin: s.origin.clone(),
            service: s.service.clone(),
            account: s.account.clone(),
            device: s.device,
            room: s.room.clone(),
            run: s.run.clone(),
            run_epoch: s.run_epoch.clone(),
            config: s.config,
            config_digest: s.config_digest.clone(),
        }
    }
}
fn digest(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
}

/// Same selector and encoding before and after Start; contains no capability.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Marker {
    pub kind: String,
    pub version: u8,
    pub local_world: uuid::Uuid,
    pub bundle_digest: String,
    pub actual_seed: String,
    pub loot: String,
}
impl Marker {
    pub fn validate(&self) -> Result<(), String> {
        let actual = self
            .actual_seed
            .parse::<u64>()
            .map_err(|_| "World seed invalid")?;
        if self.kind != "nt2-world"
            || self.version != 1
            || self.local_world.is_nil()
            || !digest(&self.bundle_digest)
            || actual.to_string() != self.actual_seed
            || !(self.loot == "individual:0"
                || self.loot == format!("shared:0:{actual}")
                || self.loot == format!("shared:1:{actual}"))
        {
            return Err("Saved world selector invalid".into());
        }
        Ok(())
    }
    pub fn encode(&self) -> Result<Vec<u8>, String> {
        self.validate()?;
        serde_json::to_vec(self).map_err(|_| "World marker encoding".into())
    }
    pub fn decode(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() > 1024 {
            return Err("World marker bound".into());
        }
        let m: Self = serde_json::from_slice(bytes).map_err(|_| "World marker shape")?;
        m.validate()?;
        Ok(m)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Intent {
    Rejoin,
    Offline,
    Terminal,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Attempt {
    pub sequence: u64,
    pub id: uuid::Uuid,
    pub context: [u8; 16],
    pub request: Vec<u8>,
}
impl Drop for Attempt {
    fn drop(&mut self) {
        self.request.zeroize()
    }
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Record {
    pub version: u8,
    pub identity: Identity,
    pub world: uuid::Uuid,
    pub bundle_digest: String,
    pub seed: Option<String>,
    pub policy: u32,
    pub config: serde_json::Value,
    pub intent: Intent,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub terminal_reason: Option<String>,
    pub sequence: u64,
    pub attempt: Option<Attempt>,
    // Filled only by the reviewed canonical consumer; no successful receipt is
    // required to retain this record or to supersede a never-sent connection.
    pub proof: Option<String>,
    pub association: Option<serde_json::Value>,
    // WorldState/world_associations lifecycle high-water, including pre-Start
    // replacements. This is not ConnectionAdmission's participant generation.
    pub canonical_generation: u64,
    #[serde(default)]
    pub semantic: String,
    pub consumed: bool,
}
impl Drop for Record {
    fn drop(&mut self) {
        if let Some(s) = &mut self.proof {
            s.zeroize()
        }
    }
}
impl Record {
    pub fn prepared(
        identity: Identity,
        world: uuid::Uuid,
        bundle_digest: String,
        seed: Option<String>,
        policy: u32,
        config: serde_json::Value,
    ) -> Result<Self, String> {
        let r = Self {
            version: 1,
            identity,
            world,
            bundle_digest,
            seed,
            policy,
            config,
            intent: Intent::Rejoin,
            terminal_reason: None,
            sequence: 0,
            attempt: None,
            proof: None,
            association: None,
            canonical_generation: 0,
            semantic: String::new(),
            consumed: false,
        };
        r.validate()?;
        Ok(r)
    }
    pub fn validate(&self) -> Result<(), String> {
        self.identity.validate()?;
        if self.version != 1
            || self.world.is_nil()
            || self.terminal_reason.as_deref().is_some_and(|r| r != "player_death_end" || self.intent == Intent::Rejoin)
            || !digest(&self.bundle_digest)
            || !matches!(self.policy, 1 | 2 | 5)
            || !self.config.is_object()
            || self.proof.as_ref().is_some_and(|s| {
                s.len() != 43
                    || !s
                        .bytes()
                        .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
            })
            || self
                .seed
                .as_ref()
                .is_some_and(|s| s.parse::<u64>().ok().is_none_or(|n| n.to_string() != *s))
        {
            return Err("Protected world record invalid".into());
        }
        if let Some(a) = &self.attempt {
            if a.sequence != self.sequence
                || a.sequence == 0
                || a.id.is_nil()
                || a.context == [0; 16]
                || a.request.len() > 32768
            {
                return Err("World attempt invalid".into());
            }
        }
        Ok(())
    }
    pub fn marker(&self) -> Result<Marker, String> {
        let seed = self.seed.as_ref().ok_or("World seed not yet observed")?;
        let m = Marker {
            kind: "nt2-world".into(),
            version: 1,
            local_world: self.world,
            bundle_digest: self.bundle_digest.clone(),
            actual_seed: seed.clone(),
            loot: if self.policy == 2 {
                "individual:0".into()
            } else {
                format!("shared:{}:{seed}", u8::from(self.policy == 5))
            },
        };
        m.validate()?;
        Ok(m)
    }
    /// Persist the incremented record before dispatch. Supersession is handled by
    /// canonical reconciliation, never by a committed-original-only local gate.
    pub fn next_attempt(&mut self, context: [u8; 16]) -> Result<Attempt, String> {
        if self.intent != Intent::Rejoin || !self.consumed || context == [0; 16] {
            return Err("World is not reconnectable".into());
        }
        self.sequence = self
            .sequence
            .checked_add(1)
            .filter(|n| *n <= 2147483647)
            .ok_or("World attempt counter exhausted")?;
        let a = Attempt {
            sequence: self.sequence,
            id: uuid::Uuid::new_v4(),
            context,
            request: Vec::new(),
        };
        self.attempt = Some(a.clone());
        Ok(a)
    }
}
pub(crate) fn validate_update(old: Option<&Record>, next: &Record) -> Result<(), String> {
    let Some(old) = old else { return Ok(()) };
    let a = old.identity.normalized()?;
    let b = next.identity.normalized()?;
    if old.world != next.world
        || a.origin != b.origin
        || a.service != b.service
        || a.account != b.account
        || a.device != b.device
        || a.room != b.room
        || a.config != b.config
        || a.config_digest != b.config_digest
        || a != b
            && (old.association.is_some()
                || old.consumed
                || old.sequence != 0
                || a.run != "preparing"
                || a.run_epoch != "preparing")
    {
        return Err("Protected world writer identity differs".into());
    }
    if old.sequence > next.sequence
        || old.canonical_generation > next.canonical_generation
        || old.intent != Intent::Rejoin && next.intent == Intent::Rejoin
        || old.intent == Intent::Offline && next.intent != Intent::Offline
        || old.terminal_reason.is_some() && next.intent != Intent::Offline && old.terminal_reason != next.terminal_reason
    {
        return Err("World writer superseded by durable attempt/terminal intent".into());
    }
    if old.sequence == next.sequence
        && old.sequence > 0
        && old.attempt.as_ref().map(|a| a.id) != next.attempt.as_ref().map(|a| a.id)
    {
        return Err("World writer attempt differs".into());
    }
    Ok(())
}
pub fn save<S: Store + ?Sized>(store: &S, r: &Record) -> Result<Record, String> {
    r.validate()?;
    let mut next = r.clone();
    next.identity = next.identity.normalized()?;
    store.update(
        &next.identity.clone(),
        next.world,
        &mut |_| Ok(next.clone()),
    )
}
pub fn load<S: Store + ?Sized>(
    store: &S,
    identity: &Identity,
    world: uuid::Uuid,
) -> Result<Option<Record>, String> {
    let id = identity.normalized()?;
    let Some(r) = store.read(&id, world)? else {
        return Ok(None);
    };
    r.validate()?;
    if r.identity.normalized()? != id || r.world != world {
        return Err("Protected world identity differs".into());
    }
    Ok(Some(r))
}
/// Snapshot only. Every save revalidates this comparison under the store's OS
/// transaction; no caller's earlier snapshot can weaken concurrent fencing.
pub fn previous<S: Store + ?Sized>(store: &S, next: &Record) -> Result<Option<Record>, String> {
    let old = store.read(&next.identity, next.world)?;
    validate_update(old.as_ref(), next)?;
    Ok(old)
}
pub fn set_intent<S: Store + ?Sized>(store: &S, record: &Record) -> Result<Record, String> {
    store.update(&record.identity, record.world, &mut |old| {
        if old
            .as_ref()
            .is_some_and(|r| r.identity.normalized().ok() != record.identity.normalized().ok())
        {
            return Err("Protected world intent identity differs".into());
        }
        let mut current = old.unwrap_or_else(|| record.clone());
        current.intent = match (current.intent, record.intent) {
            (Intent::Offline, _) | (_, Intent::Offline) => Intent::Offline,
            (_, Intent::Terminal) => Intent::Terminal,
            (intent, _) => intent,
        };
        if current.intent == Intent::Terminal && record.terminal_reason.is_some() {
            current.terminal_reason = record.terminal_reason.clone();
        }
        Ok(current)
    })
}
/// A saved marker is a nonsecret selector; protection still binds the full
/// authenticated issuer/account/device/world realm and complete record.
pub fn select<S: Store + ?Sized>(
    store: &S,
    origin: &str,
    service: &str,
    account: &str,
    device: uuid::Uuid,
    marker: &Marker,
) -> Result<Option<Record>, String> {
    marker.validate()?;
    let lookup = Identity {
        origin: origin.into(),
        service: service.into(),
        account: account.into(),
        device,
        room: "lookup".into(),
        run: "lookup".into(),
        run_epoch: "lookup".into(),
        config: 1,
        config_digest: "0".repeat(64),
    }
    .normalized()?;
    let Some(mut r) = store.read(&lookup, marker.local_world)? else {
        return Ok(None);
    };
    r.validate()?;
    if r.policy == 2 && r.seed.is_none() {
        r.seed = Some(marker.actual_seed.clone());
    }
    let actual = r.identity.normalized()?;
    if actual.origin != lookup.origin
        || actual.service != lookup.service
        || actual.account != lookup.account
        || actual.device != lookup.device
        || r.world != marker.local_world
        || r.marker()? != *marker
    {
        return Err("Saved world does not match protected association".into());
    }
    Ok(Some(r))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{collections::BTreeMap, sync::Mutex};
    #[derive(Default)]
    struct Memory(Mutex<BTreeMap<[u8; 32], Record>>);
    impl Store for Memory {
        fn read(&self, id: &Identity, world: uuid::Uuid) -> Result<Option<Record>, String> {
            Ok(self
                .0
                .lock()
                .unwrap()
                .get(&crate::world_store::binding(id, world)?)
                .cloned())
        }
        fn update(
            &self,
            id: &Identity,
            world: uuid::Uuid,
            f: &mut dyn FnMut(Option<Record>) -> Result<Record, String>,
        ) -> Result<Record, String> {
            let key = crate::world_store::binding(id, world)?;
            let mut map = self.0.lock().unwrap();
            let old = map.get(&key).cloned();
            let next = f(old.clone())?;
            next.validate()?;
            validate_update(old.as_ref(), &next)?;
            map.insert(key, next.clone());
            Ok(next)
        }
    }
    #[test]
    fn lc1_world_restart_supersedes_never_sent_attempt_without_committed_receipt() {
        let store = Memory::default();
        let i = Identity {
            origin: "https://lc1.invalid".into(),
            service: "lc1".into(),
            account: "account".into(),
            device: uuid::Uuid::new_v4(),
            room: "room".into(),
            run: "run".into(),
            run_epoch: "epoch".into(),
            config: 1,
            config_digest: "a".repeat(64),
        };
        let mut r = Record::prepared(
            i.clone(),
            uuid::Uuid::new_v4(),
            "b".repeat(64),
            Some("3377934835".into()),
            1,
            serde_json::json!({"seed":3377934835u32}),
        )
        .unwrap();
        r.consumed = true;
        let marker = r.marker().unwrap();
        let first = r.next_attempt([1; 16]).unwrap();
        save(&store, &r).unwrap();
        drop(r);
        let mut cold = select(&store, &i.origin, &i.service, &i.account, i.device, &marker)
            .unwrap()
            .unwrap();
        let second = cold.next_attempt([2; 16]).unwrap();
        assert_eq!(second.sequence, first.sequence + 1);
        assert!(cold.association.is_none());
        save(&store, &cold).unwrap();
        assert_eq!(cold.marker().unwrap(), marker);
        assert!(
            select(&store, &i.origin, &i.service, "another", i.device, &marker)
                .unwrap()
                .is_none()
        );
        cold.intent = Intent::Offline;
        save(&store, &cold).unwrap();
        assert!(
            load(&store, &i, marker.local_world)
                .unwrap()
                .unwrap()
                .next_attempt([3; 16])
                .is_err()
        );
    }
}
