use std::time::{Duration, Instant};
pub const MAGIC: u32 = 0x3247544e;
// Display only: exact local owner/fence and an existing evidence deadline.
// Codes:1 checking,2 confirmed host wait,3 reconnecting,4 confirmed entering.
#[derive(Clone,Debug)]
pub struct Display {pub code:u32,pub owner:(crate::local_owner::Id,[u8;16]),pub generation:u64,pub until:Instant}
#[derive(Clone, Debug)]
pub struct Pending {
    pub seed: u32,
    // Local adapter bits: 1 shared, 2 individual, 4 shared-loot variation.
    // Numeric seed zero alone never authorizes individual generation.
    pub world_policy: u32,
    pub actual_seed: Option<u64>,
    pub run: String,
    pub epoch: String,
    pub config: u64,
    pub expires: Instant,
    pub owner: Option<(crate::local_owner::Id, [u8; 16])>,
    pub accepted: bool,
    pub loaded: bool,
    pub local_rebind: bool,
    pub last_seen: Option<Instant>,
    pub native_v2: bool,
    pub release: bool,
    pub authority_until: Instant,
    // LC1 local collection contact is independent of canonical play authority.
    // This can keep an owned loaded conversation held, never grant gameplay.
    pub collection_until: Option<Instant>,
    pub display:Option<Display>,
    pub claim_path: Option<std::path::PathBuf>,
    pub generation: Option<(std::sync::Arc<std::sync::atomic::AtomicU64>, u64)>,
    // Provisional-to-canonical promotion keeps one native conversation. Its
    // reply epoch is separate from the lifecycle fence, which still must match.
    pub native_epoch: Option<u32>,
    pub observation: Option<crate::native_report::Latest>,
    // Content/eligibility invalidation is monotonic; protection phase is separate.
    pub invalidated_observation: u64,
    // Socket recovery needs a fresh sample without invalidating frozen content.
    pub transport_observation_floor: u64,
    pub preparation_id: Option<String>,
    pub startup_bundle: Option<std::sync::Arc<nt2_desktop_g0::startup_local::Prepared>>,
    pub startup_ready:bool,
    pub startup_transaction:Option<std::sync::Arc<crate::pipe::startup::Attempt>>,
    pub startup_attach: Option<nt2_desktop_g0::startup_local::AttachEvidence>,
}
pub fn transition(p: &mut Option<Pending>, pid:crate::local_owner::Id, request: &[u8; 32]) -> [u8; 32] {
    let mut reply = [0; 32];
    reply[0..4].copy_from_slice(&MAGIC.to_le_bytes());
    reply[4..8].copy_from_slice(&1u32.to_le_bytes());
    let field = |i| u32::from_le_bytes(request[i..i + 4].try_into().unwrap());
    let nonce: [u8; 16] = request[16..32].try_into().unwrap();
    if field(0) == MAGIC && field(4) == 2 {
        return transition_v2(p, pid, request);
    }
    if p.as_ref().is_some_and(|p| p.native_v2) {
        return reply;
    }
    if field(0) != MAGIC || field(4) != 1 || field(12) != 0 {
        return reply;
    }
    if field(8) == 2 || field(8) == 6 {
        // A second game cannot invalidate a handoff already owned by another context.
        if p.as_ref()
            .is_some_and(|p| p.owner == Some((pid, nonce)))
        {
            *p = None;
        }
        return reply;
    }
    if let Some(p) = p {
        if !p.loaded && Instant::now() >= p.expires {
            return reply;
        }
        if field(8) == 1 {
            if p.owner.is_none() {
                p.owner = Some((pid, nonce));
            }
            if p.owner != Some((pid, nonce)) {
                return reply;
            }
        } else if p.owner != Some((pid, nonce)) {
            return reply;
        }
        match field(8) {
            1 => {}
            3 => p.accepted = true,
            4 if p.accepted => p.loaded = true,
            5 => {}
            _ => return reply,
        }
        p.last_seen = Some(Instant::now());
        reply[8..12].copy_from_slice(&1u32.to_le_bytes());
        reply[12..16].copy_from_slice(&p.seed.to_le_bytes());
        reply[16..32].copy_from_slice(&nonce);
    }
    reply
}
pub fn new(seed: u32, run: String, config: u64) -> Pending {
    Pending {
        seed,
        world_policy: 1,
        actual_seed: None,
        run,
        epoch: String::new(),
        config,
        expires: Instant::now() + Duration::from_secs(120),
        owner: None,
        accepted: false,
        loaded: false,
        local_rebind: false,
        last_seen: None,
        native_v2: false,
        release: false,
        authority_until: Instant::now(),
        collection_until: None,
        display:None,
        claim_path: None,
        generation: None,
        native_epoch:None,
        observation: None,
        invalidated_observation: 0,
        transport_observation_floor: 0,
        preparation_id: None,
        startup_bundle: None,
        startup_attach: None,
        startup_transaction:None,
        startup_ready:false,
    }
}
fn transition_v2(pending: &mut Option<Pending>, pid:crate::local_owner::Id, request: &[u8; 32]) -> [u8; 32] {
    let field = |i| u32::from_le_bytes(request[i..i + 4].try_into().unwrap());
    let op = field(8);
    let nonce: [u8; 16] = request[16..32].try_into().unwrap();
    let mut reply = [0u8; 32];
    reply[..4].copy_from_slice(&MAGIC.to_le_bytes());
    reply[4..8].copy_from_slice(&2u32.to_le_bytes());
    let Some(p) = pending else {
        return reply;
    };
    if !p.native_v2 || nonce == [0; 16] || !matches!(p.world_policy,1|2|5) {
        return reply;
    }
    if op == 2 || op == 6 {
        if field(12) == 0 && (p.owner == Some((pid, nonce))) {
            *pending = None;
        }
        return reply;
    }
    if p.startup_bundle.as_ref().is_some_and(|b|!b.matches(&p.run,&p.epoch,p.config,p.preparation_id.as_deref(),p.seed,p.world_policy)) {return reply;}
    if p.generation
        .as_ref()
        .is_some_and(|(g, n)| g.load(std::sync::atomic::Ordering::Acquire) != *n)
    {
        return reply;
    }
    if !p.loaded && Instant::now() >= p.expires {
        return reply;
    }
    if op == 1 {
        if p.startup_bundle.is_some(){return reply;} // production startup uses the off-lock complete transaction
        if field(12) != 0 {
            return reply;
        }
        if p.owner.is_none() {
            if Instant::now() >= p.authority_until {
                return reply;
            }
            // Commit consumption before granting a seed. Crash/restart cannot replay this world.
            let Some(path) = p.claim_path.as_ref() else {
                return reply;
            };
            let result = (|| -> std::io::Result<()> {
                use std::io::Write;
                let parent = path.parent().ok_or(std::io::ErrorKind::InvalidInput)?;
                std::fs::create_dir_all(parent)?;
                if std::fs::read_dir(parent)?.take(129).count() >= 128 {
                    return Err(std::io::ErrorKind::StorageFull.into());
                }
                let mut file = std::fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(path)?;
                file.write_all(&pid.claim_prefix())?;
                file.write_all(&nonce)?;
                file.sync_all()
            })();
            if result.is_err() {
                return reply;
            }
            p.owner = Some((pid, nonce));
        }
    }
    if p.owner != Some((pid, nonce)) {
        return reply;
    }
    match op {
        1 => {}
        3 if (p.world_policy==1||p.world_policy==5)&&field(12)==p.seed
            || p.world_policy==2&&p.actual_seed.is_none_or(|actual|actual==u64::from(field(12))) => {
                p.actual_seed=Some(u64::from(field(12)));p.accepted=true;
            },
        10 if field(12)==0 && matches!(p.world_policy,1|2|5) => {},
        4 if field(12) == 1 && p.accepted => p.loaded = true,
        5 if field(12) == 0 => {}
        _ => return reply,
    }
    p.last_seen = Some(Instant::now());
    reply[8..12].copy_from_slice(&1u32.to_le_bytes());
    let value = if op == 10 { p.world_policy } else if op == 1 {
        p.seed
    } else if p.loaded && p.accepted && p.release && Instant::now() < p.authority_until {
        2
    } else {
        1
    };
    reply[12..16].copy_from_slice(&value.to_le_bytes());
    reply[16..32].copy_from_slice(&nonce);
    reply
}
pub(crate) fn known_seed(pending:&mut Option<Pending>,pid:crate::local_owner::Id,nonce:[u8;16],actual:u64)->[u8;32] {
    let mut reply=[0u8;32];reply[..4].copy_from_slice(&MAGIC.to_le_bytes());reply[4..8].copy_from_slice(&2u32.to_le_bytes());
    let Some(p)=pending else{return reply};
    if !p.native_v2||nonce==[0;16]||p.owner!=Some((pid,nonce))||!matches!(p.world_policy,1|2|5)
        ||p.generation.as_ref().is_some_and(|(g,n)|g.load(std::sync::atomic::Ordering::Acquire)!=*n)
        ||(!p.loaded&&Instant::now()>=p.expires)
        ||(p.world_policy!=2&&actual!=u64::from(p.seed))||p.actual_seed.is_some_and(|known|known!=actual){return reply;}
    p.actual_seed=Some(actual);p.accepted=true;p.last_seen=Some(Instant::now());
    reply[8..12].copy_from_slice(&1u32.to_le_bytes());reply[12..16].copy_from_slice(&1u32.to_le_bytes());reply[16..32].copy_from_slice(&nonce);reply
}
impl Pending {
    pub fn matches_seed(&self,value:Option<&str>)->bool {
        let expected=if self.world_policy==2 {self.actual_seed} else if matches!(self.world_policy,1|5){Some(u64::from(self.seed))}else{None};
        expected.is_some_and(|seed|value==Some(seed.to_string().as_str()))
    }
}
/// Historical ST1 fixture only. Production now uses pipe::startup::Lane.
#[cfg(test)]
#[derive(Debug,PartialEq,Eq)]
pub enum StartupFailure {Unavailable,Retired,Expired,PreparationMismatch,ConsumedOrUncertain}
#[derive(Debug)]
#[cfg(test)]
pub struct CoherentStartup {
    pub bundle:std::sync::Arc<nt2_desktop_g0::startup_local::Prepared>,
    pub owner:(crate::local_owner::Id,[u8;16]),pub consumption_confirmed:bool,pub elapsed:Duration,
}
#[cfg(test)]
pub fn coherent_startup(pending:&mut Option<Pending>,pid:crate::local_owner::Id,context:[u8;16])->Result<CoherentStartup,StartupFailure>{
    let began=Instant::now();let p=pending.as_ref().ok_or(StartupFailure::Unavailable)?;
    let b=p.startup_bundle.clone().ok_or(StartupFailure::Unavailable)?;
    let generation=p.generation.as_ref().map(|(g,_)|g.load(std::sync::atomic::Ordering::Acquire)).ok_or(StartupFailure::Retired)?;
    if generation!=b.generation(){return Err(StartupFailure::Retired)}
    if !b.live(generation){return Err(StartupFailure::Expired)}
    if !b.matches(&p.run,&p.epoch,p.config,p.preparation_id.as_deref(),p.seed,p.world_policy){return Err(StartupFailure::PreparationMismatch)}
    let mut request=[0u8;32];request[..4].copy_from_slice(&MAGIC.to_le_bytes());request[4..8].copy_from_slice(&2u32.to_le_bytes());request[8..12].copy_from_slice(&1u32.to_le_bytes());request[16..32].copy_from_slice(&context);
    // Preserve accepted historical fixtures without exposing their blocking
    // legacy operation to a production complete-bundle preparation.
    let bundle=pending.as_mut().unwrap().startup_bundle.take();let reply=transition(pending,pid,&request);if let Some(p)=pending.as_mut(){p.startup_bundle=bundle;}
    if reply[8..12]!=1u32.to_le_bytes(){return Err(StartupFailure::ConsumedOrUncertain)}
    Ok(CoherentStartup{bundle:b,owner:(pid,context),consumption_confirmed:true,elapsed:began.elapsed()})
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]fn st1_coherent_prepare_commits_once_and_lost_reply_cannot_rearm(){
        let root=std::env::temp_dir().join(format!("nt2-st1-handoff-{}",uuid::Uuid::new_v4()));
        let scope=nt2_desktop_g0::startup_local::Scope{origin:"https://fixture.invalid".into(),service:"service".into(),service_epoch:"epoch".into(),account:"account".into(),device:uuid::Uuid::new_v4(),desktop:"desktop".into(),desktop_epoch:"desktop-epoch".into(),room:"room".into(),run:"run".into(),run_epoch:"run-epoch".into(),config:1,config_digest:"a".repeat(64),preparation:"preparation".into()};
        let generation=std::sync::Arc::new(std::sync::atomic::AtomicU64::new(7));let bundle=std::sync::Arc::new(nt2_desktop_g0::startup_local::Prepared::stage(scope,424242,5,7,Instant::now()+Duration::from_secs(60)).unwrap());
        let mut p=new(424242,"run".into(),1);p.epoch="run-epoch".into();p.preparation_id=Some("preparation".into());p.native_v2=true;p.world_policy=5;p.authority_until=Instant::now()+Duration::from_secs(12);p.claim_path=Some(root.join("claim"));p.generation=Some((generation.clone(),7));p.startup_bundle=Some(bundle.clone());let original=p.clone();let mut current=Some(p);
        let reply=coherent_startup(&mut current,nt2_local_ws::owner::Id::OsProcess(9),[1;16]).unwrap();assert!(reply.consumption_confirmed&&std::sync::Arc::ptr_eq(&reply.bundle,&bundle));assert_eq!((reply.bundle.seed(),reply.bundle.policy()),(424242,5));let bytes=std::fs::read(root.join("claim")).unwrap();assert_eq!(bytes.len(),20);
        let secondary=coherent_startup(&mut current,nt2_local_ws::owner::Id::OsProcess(9),[1;16]).unwrap();assert_eq!(reply.bundle.digest(),secondary.bundle.digest());assert_eq!(std::fs::read(root.join("claim")).unwrap(),bytes);
        assert_eq!(coherent_startup(&mut Some(original),nt2_local_ws::owner::Id::OsProcess(9),[1;16]).unwrap_err(),StartupFailure::ConsumedOrUncertain);
        current.as_mut().unwrap().world_policy=1;assert_eq!(coherent_startup(&mut current,nt2_local_ws::owner::Id::OsProcess(9),[1;16]).unwrap_err(),StartupFailure::PreparationMismatch);
        let mut invalid=current.clone();let mut close=[0;32];close[..4].copy_from_slice(&MAGIC.to_le_bytes());close[4..8].copy_from_slice(&2u32.to_le_bytes());close[8..12].copy_from_slice(&6u32.to_le_bytes());close[16..32].copy_from_slice(&[1;16]);transition(&mut invalid,nt2_local_ws::owner::Id::OsProcess(9),&close);assert!(invalid.is_none(),"Current scoped close must still retire a mismatched preparation");current.as_mut().unwrap().world_policy=5;
        generation.store(8,std::sync::atomic::Ordering::Release);assert_eq!(coherent_startup(&mut current,nt2_local_ws::owner::Id::OsProcess(9),[1;16]).unwrap_err(),StartupFailure::Retired);
        println!("ST1 source helper durable claim+coherent reply elapsed_us={}; not actual Noita cold loader timing",reply.elapsed.as_micros());
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn seedloot01_individual_policy_freezes_known_seed_and_preserves_consumption_scope() {
        let root=std::env::temp_dir().join(format!("nt2-seed-loot-{}",uuid::Uuid::new_v4()));
        let mut p=new(0,"immutable-run".into(),1);p.native_v2=true;p.world_policy=2;
        p.claim_path=Some(root.join("claim"));p.authority_until=Instant::now()+Duration::from_secs(12);
        let generation=std::sync::Arc::new(std::sync::atomic::AtomicU64::new(1));p.generation=Some((generation.clone(),1));let mut p=Some(p);
        let mut frame=[1u8;32];frame[..4].copy_from_slice(&MAGIC.to_le_bytes());frame[4..8].copy_from_slice(&2u32.to_le_bytes());
        let mut send=|op:u32,field:u32,pid:crate::local_owner::Id|{frame[8..12].copy_from_slice(&op.to_le_bytes());frame[12..16].copy_from_slice(&field.to_le_bytes());transition(&mut p,pid,&frame)};
        assert_eq!(send(1,0,nt2_local_ws::owner::Id::OsProcess(7))[8],1);assert_eq!(u32::from_le_bytes(send(10,0,nt2_local_ws::owner::Id::OsProcess(7))[12..16].try_into().unwrap()),2);
        assert_eq!(send(3,777,nt2_local_ws::owner::Id::OsProcess(8))[8],0);assert_eq!(send(4,1,nt2_local_ws::owner::Id::OsProcess(7))[8],0);
        assert_eq!(send(3,777,nt2_local_ws::owner::Id::OsProcess(7))[8],1);assert_eq!(send(3,888,nt2_local_ws::owner::Id::OsProcess(7))[8],0);assert_eq!(send(4,1,nt2_local_ws::owner::Id::OsProcess(7))[8],1);
        generation.store(2,std::sync::atomic::Ordering::Release);assert_eq!(send(10,0,nt2_local_ws::owner::Id::OsProcess(7))[8],0);
        drop(send);let p=p.unwrap();assert!(p.matches_seed(Some("777")));assert!(!p.matches_seed(None)&&!p.matches_seed(Some("888")));assert!(root.join("claim").exists());
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn seedloot01_zero_without_individual_policy_cannot_accept_an_arbitrary_seed(){
        let mut p=new(0,"run".into(),1);p.native_v2=true;p.owner=Some((nt2_local_ws::owner::Id::OsProcess(7),[1;16]));
        let mut frame=[1u8;32];frame[..4].copy_from_slice(&MAGIC.to_le_bytes());frame[4..8].copy_from_slice(&2u32.to_le_bytes());frame[8..12].copy_from_slice(&3u32.to_le_bytes());frame[12..16].copy_from_slice(&777u32.to_le_bytes());let mut pending=Some(p);
        assert_eq!(transition(&mut pending,nt2_local_ws::owner::Id::OsProcess(7),&frame)[8],0);
        pending.as_mut().unwrap().world_policy=3;frame[8..12].copy_from_slice(&10u32.to_le_bytes());frame[12..16].fill(0);assert_eq!(transition(&mut pending,nt2_local_ws::owner::Id::OsProcess(7),&frame)[8],0);
    }
    fn request(op: u32, nonce: u8) -> [u8; 32] {
        let mut b = [0; 32];
        b[0..4].copy_from_slice(&MAGIC.to_le_bytes());
        b[4..8].copy_from_slice(&1u32.to_le_bytes());
        b[8..12].copy_from_slice(&op.to_le_bytes());
        b[16..32].fill(nonce);
        b
    }
    fn granted(b: [u8; 32]) -> bool {
        b[8] == 1
    }
    #[test]
    fn durable_v2_owner_seed_release_expiry_and_restart_guard() {
        let root = std::env::temp_dir().join(format!("nt2-world-guard-{}", uuid::Uuid::new_v4()));
        let make = || {
            let mut p = new(424242, "run".into(), 1);
            p.native_v2 = true;
            p.claim_path = Some(root.join("one"));
            p.authority_until = Instant::now() + Duration::from_secs(12);
            p
        };
        let req = |op: u32, value: u32, nonce: u8| {
            let mut r = request(op, nonce);
            r[4..8].copy_from_slice(&2u32.to_le_bytes());
            r[12..16].copy_from_slice(&value.to_le_bytes());
            r
        };
        let mut p = Some(make());
        assert!(granted(transition(&mut p, nt2_local_ws::owner::Id::OsProcess(7), &req(1, 0, 1))));
        assert!(root.join("one").exists());
        assert!(!granted(transition(&mut p, nt2_local_ws::owner::Id::OsProcess(7), &req(3, 42, 1))));
        assert!(!granted(transition(&mut p, nt2_local_ws::owner::Id::OsProcess(7), &req(4, 1, 1))));
        assert!(granted(transition(&mut p, nt2_local_ws::owner::Id::OsProcess(7), &req(3, 424242, 1))));
        assert!(granted(transition(&mut p, nt2_local_ws::owner::Id::OsProcess(7), &req(4, 1, 1))));
        assert_eq!(transition(&mut p, nt2_local_ws::owner::Id::OsProcess(7), &req(5, 0, 1))[12], 1);
        p.as_mut().unwrap().release = true;
        assert_eq!(transition(&mut p, nt2_local_ws::owner::Id::OsProcess(7), &req(5, 0, 1))[12], 2);
        p.as_mut().unwrap().authority_until = Instant::now();
        assert_eq!(transition(&mut p, nt2_local_ws::owner::Id::OsProcess(7), &req(5, 0, 1))[12], 1);
        transition(&mut p, nt2_local_ws::owner::Id::OsProcess(7), &req(6, 0, 2));
        assert!(p.is_some());
        transition(&mut p, nt2_local_ws::owner::Id::OsProcess(7), &req(6, 0, 1));
        assert!(p.is_none());
        p = Some(make());
        assert!(!granted(transition(&mut p, nt2_local_ws::owner::Id::OsProcess(7), &req(1, 0, 2))));
        assert!(p.as_ref().unwrap().owner.is_none());
        std::fs::remove_file(root.join("one")).unwrap();
        std::fs::remove_dir(root).unwrap();
    }
    #[test]
    fn ownership_duplicate_and_stale() {
        let mut p = Some(new(424242, "run".into(), 1));
        assert!(granted(transition(&mut p, nt2_local_ws::owner::Id::OsProcess(7), &request(1, 1))));
        assert!(granted(transition(&mut p, nt2_local_ws::owner::Id::OsProcess(7), &request(1, 1))));
        assert!(!granted(transition(&mut p, nt2_local_ws::owner::Id::OsProcess(8), &request(1, 1))));
        assert!(!granted(transition(&mut p, nt2_local_ws::owner::Id::OsProcess(7), &request(1, 2))));
    }
    #[test]
    fn continue_cancel_and_expiry() {
        let mut p = Some(new(1, "r".into(), 1));
        assert!(!granted(transition(&mut p, nt2_local_ws::owner::Id::OsProcess(7), &request(2, 1))));
        assert!(p.is_none());
        p = Some(new(1, "r".into(), 1));
        p.as_mut().unwrap().expires = Instant::now();
        assert!(!granted(transition(&mut p, nt2_local_ws::owner::Id::OsProcess(7), &request(1, 1))));
        p = None;
        assert!(!granted(transition(&mut p, nt2_local_ws::owner::Id::OsProcess(7), &request(1, 1))));
    }
    #[test]
    fn load_requires_seed_and_owner() {
        let mut p = Some(new(1, "r".into(), 1));
        transition(&mut p, nt2_local_ws::owner::Id::OsProcess(7), &request(1, 1));
        assert!(!granted(transition(&mut p, nt2_local_ws::owner::Id::OsProcess(7), &request(4, 1))));
        assert!(granted(transition(&mut p, nt2_local_ws::owner::Id::OsProcess(7), &request(3, 1))));
        assert!(!granted(transition(&mut p, nt2_local_ws::owner::Id::OsProcess(8), &request(4, 1))));
        assert!(granted(transition(&mut p, nt2_local_ws::owner::Id::OsProcess(7), &request(4, 1))));
        assert!(p.unwrap().loaded);
    }
    #[test]
    fn malformed_is_inert() {
        let mut p = Some(new(1, "r".into(), 1));
        assert!(!granted(transition(&mut p, nt2_local_ws::owner::Id::OsProcess(7), &[0; 32])));
        assert!(p.unwrap().owner.is_none());
    }
    #[test]
    fn foreign_continue_cannot_cancel_owner() {
        let mut p = Some(new(1, "r".into(), 1));
        transition(&mut p, nt2_local_ws::owner::Id::OsProcess(7), &request(1, 1));
        transition(&mut p, nt2_local_ws::owner::Id::OsProcess(8), &request(2, 2));
        assert!(p.is_some());
        transition(&mut p, nt2_local_ws::owner::Id::OsProcess(7), &request(2, 1));
        assert!(p.is_none());
    }
    #[test]
    fn heartbeat_is_pause_safe_bound_to_context_and_survives_loaded_deadline() {
        let mut p = Some(new(1, "r".into(), 1));
        assert!(!granted(transition(&mut p, nt2_local_ws::owner::Id::OsProcess(7), &request(5, 1))));
        transition(&mut p, nt2_local_ws::owner::Id::OsProcess(7), &request(1, 1));
        assert!(granted(transition(&mut p, nt2_local_ws::owner::Id::OsProcess(7), &request(5, 1))));
        assert!(!granted(transition(&mut p, nt2_local_ws::owner::Id::OsProcess(7), &request(5, 2))));
        transition(&mut p, nt2_local_ws::owner::Id::OsProcess(7), &request(3, 1));
        transition(&mut p, nt2_local_ws::owner::Id::OsProcess(7), &request(4, 1));
        p.as_mut().unwrap().expires = Instant::now();
        assert!(granted(transition(&mut p, nt2_local_ws::owner::Id::OsProcess(7), &request(5, 1))));
        transition(&mut p, nt2_local_ws::owner::Id::OsProcess(7), &request(6, 2));
        assert!(p.is_some());
        transition(&mut p, nt2_local_ws::owner::Id::OsProcess(7), &request(6, 1));
        assert!(p.is_none());
    }
}
