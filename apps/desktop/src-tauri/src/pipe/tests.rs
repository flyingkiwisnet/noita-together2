use super::*;
use std::sync::atomic::AtomicU32;
#[link(name = "kernel32")]
unsafe extern "system" {
    fn CreateFileW(n: *const u16, a: u32, s: u32, sa: Handle, c: u32, f: u32, t: Handle) -> Handle;
    fn SetNamedPipeHandleState(h: Handle, m: *const u32, c: *const u32, t: *const u32) -> i32;
}
static COUNTER: AtomicU32 = AtomicU32::new(0);
#[test]fn a3_initial_contact_trace_releases_pending_before_diagnostics_contention(){
 let pending=Arc::new(Mutex::new(Some(super::super::handoff::new(424242,"diagnostic-lock".into(),1))));let diagnostics=Arc::new(Mutex::new(NativeDiagnostics::default()));
 diagnostics.lock().unwrap().initial_contact_delay=Some((5,7));let held=diagnostics.lock().unwrap();let(p,d)=(pending.clone(),diagnostics.clone());let(tx,rx)=std::sync::mpsc::channel();
 let worker=std::thread::spawn(move||record_initial_contact(&p,&d,5,true,||{tx.send(()).unwrap();}));
 let reached=rx.recv_timeout(Duration::from_secs(2)).is_ok();let pending_available=pending.try_lock().is_ok();
 // Release before asserting/joining, so even a regressed lock order cannot
 // strand a disposable unit-test thread during a failing regression.
 drop(held);let delay=worker.join().unwrap();assert!(reached,"Trace waited for diagnostics before completing the pending snapshot");assert!(pending_available,"Trace retained pending while waiting for diagnostics");assert_eq!(delay,Some(7));let d=diagnostics.lock().unwrap();assert_eq!(d.initial_contact_trace.len(),1);assert_eq!(d.initial_contact_trace[0]["transitionAcknowledged"],true);assert!(d.initial_contact_delay.is_none());
}
#[test]fn lc1_final_delivery_rechecks_current_play_eligibility_after_reply_construction(){
 for revoked in ["release","accepted","loaded"]{let generation=Arc::new(std::sync::atomic::AtomicU64::new(7));let mut p=super::super::handoff::new(424242,"delivery".into(),1);p.owner=Some((nt2_local_ws::owner::Id::OsProcess(123),[1;16]));p.native_v2=true;p.accepted=true;p.loaded=true;p.release=true;p.generation=Some((generation,7));p.authority_until=Instant::now()+Duration::from_secs(4);p.collection_until=Some(Instant::now()+Duration::from_secs(6));let common=Arc::new(Mutex::new(Some(p)));let mut q=[0u8;32];q[..4].copy_from_slice(&super::super::handoff::MAGIC.to_le_bytes());q[4..8].copy_from_slice(&4u32.to_le_bytes());q[8..12].copy_from_slice(&5u32.to_le_bytes());q[16..32].fill(1);let mut reply=c2_transition(&mut common.lock().unwrap(),nt2_local_ws::owner::Id::OsProcess(123),&q);assert_eq!(reply[8],1);assert_eq!(reply[12],2);
  {let mut slot=common.lock().unwrap();let p=slot.as_mut().unwrap();match revoked{"release"=>p.release=false,"accepted"=>p.accepted=false,_=>p.loaded=false};assert!(p.authority_until>Instant::now()&&p.collection_until.unwrap()>Instant::now());}c2_delivery_fence(&common,None,nt2_local_ws::owner::Id::OsProcess(123),&q,&mut reply);assert_eq!(reply[8],1);assert_eq!(reply[12],1,"Already-built play reply escaped current {revoked} revocation");assert!(u32::from_le_bytes(reply[32..36].try_into().unwrap())>0,"Current held collection stays bounded/live");
 }
}
#[test]fn lc1_saved_selector_poll_preserves_matching_contact_after_bootstrap_context(){
 let marker=nt2_desktop_g0::world_local::Marker{kind:"nt2-world".into(),version:1,local_world:uuid::Uuid::new_v4(),bundle_digest:"a".repeat(64),actual_seed:"424242".into(),loot:"shared:0:424242".into()};let body=marker.encode().unwrap();let mut q=vec![0;32];q[..4].copy_from_slice(&super::super::handoff::MAGIC.to_le_bytes());q[4..8].copy_from_slice(&8u32.to_le_bytes());q[8..12].copy_from_slice(&14u32.to_le_bytes());q[12..16].copy_from_slice(&(body.len() as u32).to_le_bytes());q.extend(body);let mut mailbox=rejoin::Mailbox::default();assert_eq!(mailbox.transition(&None,nt2_local_ws::owner::Id::OsProcess(123),&q)[12],4);
 let contact=rejoin::Contact{pid:nt2_local_ws::owner::Id::OsProcess(123),context:[1;16],association:*marker.local_world.as_bytes(),actual:424242};mailbox.contact=Some(contact.clone());mailbox.admitted=Some(contact.clone());mailbox.lookup=2;let mut p=super::super::handoff::new(424242,"saved".into(),1);p.owner=Some((nt2_local_ws::owner::Id::OsProcess(123),[1;16]));p.collection_until=Some(Instant::now()+Duration::from_secs(6));q[16..32].fill(1);assert_eq!(mailbox.transition(&Some(p),nt2_local_ws::owner::Id::OsProcess(123),&q)[12],2);assert!(mailbox.contact==Some(contact.clone())&&mailbox.admitted==Some(contact));assert_eq!(mailbox.world_context,Some([0;16]),"Selector identity must not rebind an existing native owner");
 mailbox.lookup=3;mailbox.terminal=true;assert_eq!(mailbox.transition(&None,nt2_local_ws::owner::Id::OsProcess(123),&q)[12],3);assert!(mailbox.terminal);q[16..32].fill(2);assert_eq!(mailbox.transition(&None,nt2_local_ws::owner::Id::OsProcess(123),&q)[12],4);assert!(mailbox.contact.is_none()&&mailbox.admitted.is_none());assert_eq!(mailbox.world_context,Some([2;16]));
}
#[test]fn lc1_collection_contact_cannot_extend_play_or_ignore_close_and_scope_fences(){
 let generation=Arc::new(std::sync::atomic::AtomicU64::new(7));let mut p=super::super::handoff::new(424242,"lc1-contact".into(),1);p.owner=Some((nt2_local_ws::owner::Id::OsProcess(123),[1;16]));p.native_v2=true;p.loaded=true;p.accepted=true;p.actual_seed=Some(424242);p.generation=Some((generation.clone(),7));p.collection_until=Some(Instant::now()+Duration::from_secs(6));let mut slot=Some(p);let mut q=[0u8;32];q[..4].copy_from_slice(&super::super::handoff::MAGIC.to_le_bytes());q[4..8].copy_from_slice(&4u32.to_le_bytes());q[8..12].copy_from_slice(&5u32.to_le_bytes());q[16..32].fill(1);
 let r=c2_transition(&mut slot,nt2_local_ws::owner::Id::OsProcess(123),&q);assert_eq!(r[8],1);assert_eq!(r[12],1);assert!(u32::from_le_bytes(r[32..36].try_into().unwrap())>0);assert!(!g3_playing(&slot,nt2_local_ws::owner::Id::OsProcess(123),&[1;16]));
 {let p=slot.as_mut().unwrap();p.release=true;p.authority_until=Instant::now()+Duration::from_millis(120);}let r=c2_transition(&mut slot,nt2_local_ws::owner::Id::OsProcess(123),&q);assert_eq!(r[12],2);assert!(u32::from_le_bytes(r[32..36].try_into().unwrap())<=120,"Collection cannot extend canonical play duration");slot.as_mut().unwrap().authority_until=Instant::now();let r=c2_transition(&mut slot,nt2_local_ws::owner::Id::OsProcess(123),&q);assert_eq!(r[8],1);assert_eq!(r[12],1);assert!(u32::from_le_bytes(r[32..36].try_into().unwrap())>0);assert!(!g3_playing(&slot,nt2_local_ws::owner::Id::OsProcess(123),&[1;16]));
 assert_eq!(c2_transition(&mut slot,nt2_local_ws::owner::Id::OsProcess(124),&q)[8],0);q[16]=2;assert_eq!(c2_transition(&mut slot,nt2_local_ws::owner::Id::OsProcess(123),&q)[8],0);q[16]=1;generation.store(8,Ordering::Release);assert_eq!(c2_transition(&mut slot,nt2_local_ws::owner::Id::OsProcess(123),&q)[8],0);generation.store(7,Ordering::Release);slot.as_mut().unwrap().collection_until=None;assert_eq!(c2_transition(&mut slot,nt2_local_ws::owner::Id::OsProcess(123),&q)[8],0,"Terminal/retired collection does not recover authority");slot.as_mut().unwrap().collection_until=Some(Instant::now()+Duration::from_secs(6));
 let marker=nt2_desktop_g0::world_local::Marker{kind:"nt2-world".into(),version:1,local_world:uuid::Uuid::new_v4(),bundle_digest:"a".repeat(64),actual_seed:"424242".into(),loot:"shared:0:424242".into()}.encode().unwrap();let mut selector=q.to_vec();selector[4..8].copy_from_slice(&8u32.to_le_bytes());selector[8..12].copy_from_slice(&14u32.to_le_bytes());selector[12..16].copy_from_slice(&(marker.len() as u32).to_le_bytes());selector.extend(marker);let mut mailbox=rejoin::Mailbox::default();assert_eq!(mailbox.transition(&slot,nt2_local_ws::owner::Id::OsProcess(124),&selector)[8],0,"Expired remote authority does not let another PID replace locally held contact");selector[16]=2;assert_eq!(mailbox.transition(&slot,nt2_local_ws::owner::Id::OsProcess(123),&selector)[8],0);selector[16]=1;assert_eq!(mailbox.transition(&slot,nt2_local_ws::owner::Id::OsProcess(123),&selector)[8],1);assert!(mailbox.admitted.is_none()&&!g3_playing(&slot,nt2_local_ws::owner::Id::OsProcess(123),&[1;16]),"A selector/local collection never grants gameplay");
 q[8..12].copy_from_slice(&6u32.to_le_bytes());c2_transition(&mut slot,nt2_local_ws::owner::Id::OsProcess(124),&q);assert!(slot.is_some());c2_transition(&mut slot,nt2_local_ws::owner::Id::OsProcess(123),&q);assert!(slot.is_none(),"Actual matching close retires held local world");
}
#[test]fn lc1_exited_process_with_retained_handle_is_not_live_owner(){
 use std::os::windows::process::CommandExt;
 let mut child=std::process::Command::new("cmd.exe").args(["/d","/c","exit /b 0"]).creation_flags(0x08000000).spawn().unwrap();let pid=child.id();assert!(child.wait().unwrap().success());
 // Keep Child (and its Windows process handle) alive. This was precisely the
 // replacement/cold/running fixture pattern; dropping it masks the defect.
 unsafe{let h=OpenProcess(0x1000,0,pid);assert!(!h.is_null());let h=Owned(h);let(mut c,mut e,mut k,mut u)=(0,0,0,0);assert_ne!(GetProcessTimes(h.0,&mut c,&mut e,&mut k,&mut u),0);assert_ne!(c,0,"The old birth-only helper falsely returned this exited instance");}
 assert!(owner_instance(crate::local_owner::Id::os_process(pid)).is_none(),"An exited native must lose liveness even while its parent retains Child");assert!(owner_instance(crate::local_owner::Id::os_process(std::process::id())).is_some());drop(child);
}
#[test]fn lc1_early_collection_delivery_survives_empty_common_and_fences_promotion(){
 let early=Arc::new(world::Bridge::new());let common=Arc::new(Mutex::new(None));let generation=Arc::new(std::sync::atomic::AtomicU64::new(7));let mut p=super::super::handoff::new(424242,"lc1-early".into(),1);p.native_v2=true;p.owner=Some((nt2_local_ws::owner::Id::OsProcess(123),[1;16]));p.authority_until=Instant::now()+Duration::from_secs(12);p.generation=Some((generation.clone(),7));*early.pending.lock().unwrap()=Some(p);
 let mut request=[0u8;40];request[..4].copy_from_slice(&super::super::handoff::MAGIC.to_le_bytes());request[4..8].copy_from_slice(&6u32.to_le_bytes());request[8..12].copy_from_slice(&11u32.to_le_bytes());request[12..16].copy_from_slice(&424242u32.to_le_bytes());request[16..32].fill(1);
 let mut reply=early.frame(nt2_local_ws::owner::Id::OsProcess(123),&request);assert_eq!(reply[8],1);let accepted=reply.clone();c2_delivery_fence(&common,Some(&early),nt2_local_ws::owner::Id::OsProcess(123),&request,&mut reply);assert_eq!(reply[8],1);assert_eq!(reply[36],7);assert!(early.pending.lock().unwrap().as_ref().unwrap().accepted);
 // The old common-only check reproduces the first native refusal.
 let mut old=accepted.clone();c2_fence(&common.lock().unwrap(),nt2_local_ws::owner::Id::OsProcess(123),&request,&mut old);assert_eq!(old[8],0);
 let mut promoted=early.pending.lock().unwrap().take().unwrap();promoted.loaded=true;promoted.native_epoch=Some(7);let current=Arc::new(std::sync::atomic::AtomicU64::new(12));promoted.generation=Some((current.clone(),12));*common.lock().unwrap()=Some(promoted);reply=accepted.clone();c2_delivery_fence(&common,Some(&early),nt2_local_ws::owner::Id::OsProcess(123),&request,&mut reply);assert_eq!(reply[8],1);assert_eq!(reply[36],7,"Broker epoch survives common promotion");
 current.store(13,Ordering::Release);reply=accepted.clone();c2_delivery_fence(&common,Some(&early),nt2_local_ws::owner::Id::OsProcess(123),&request,&mut reply);assert_eq!(reply[8],0);
 let mut held=common.lock().unwrap().take().unwrap();held.generation=Some((generation.clone(),7));*early.pending.lock().unwrap()=Some(held);reply=accepted.clone();c2_delivery_fence(&common,None,nt2_local_ws::owner::Id::OsProcess(123),&request,&mut reply);assert_eq!(reply[8],0,"Common source must never fall back to an old early slot");
 for (pid,ctx) in [(124,1),(123,2)]{request[16..32].fill(ctx);reply=accepted.clone();c2_delivery_fence(&common,Some(&early),nt2_local_ws::owner::Id::OsProcess(pid),&request,&mut reply);assert_eq!(reply[8],0);}request[16..32].fill(1);generation.store(8,Ordering::Release);reply=accepted;c2_delivery_fence(&common,Some(&early),nt2_local_ws::owner::Id::OsProcess(123),&request,&mut reply);assert_eq!(reply[8],0);
}
#[test]fn seedloot01_full_width_seed_envelope_preserves_owner_generation_and_actual_binding(){
 let generation=Arc::new(std::sync::atomic::AtomicU64::new(7));let mut p=super::super::handoff::new(0,"individual".into(),1);
 p.native_v2=true;p.world_policy=2;p.owner=Some((nt2_local_ws::owner::Id::OsProcess(123),[1;16]));p.authority_until=Instant::now()+Duration::from_secs(12);p.generation=Some((generation.clone(),7));let mut pending=Some(p);
 let mut request=[0u8;40];request[..4].copy_from_slice(&super::super::handoff::MAGIC.to_le_bytes());request[4..8].copy_from_slice(&6u32.to_le_bytes());request[8..12].copy_from_slice(&11u32.to_le_bytes());request[12..16].fill(255);request[16..32].fill(1);request[32..36].fill(255);
 assert_eq!(c2_transition(&mut pending,nt2_local_ws::owner::Id::OsProcess(124),&request)[8],0);assert_eq!(c2_transition(&mut pending,nt2_local_ws::owner::Id::OsProcess(123),&request)[8],1);
 assert_eq!(pending.as_ref().unwrap().actual_seed,Some(u64::MAX));assert!(pending.as_ref().unwrap().matches_seed(Some("18446744073709551615")));
 request[12]=254;assert_eq!(c2_transition(&mut pending,nt2_local_ws::owner::Id::OsProcess(123),&request)[8],0);request[12]=255;
 request[36]=1;assert_eq!(c2_transition(&mut pending,nt2_local_ws::owner::Id::OsProcess(123),&request)[8],0);request[36]=0;
 generation.store(8,Ordering::Release);assert_eq!(c2_transition(&mut pending,nt2_local_ws::owner::Id::OsProcess(123),&request)[8],0);
}
#[test]fn c2_original_local_lease_generation_and_delivery_fences(){
 let generation=Arc::new(std::sync::atomic::AtomicU64::new(7));let mut p=super::super::handoff::new(424242,"c2-private".into(),1);
 p.native_v2=true;p.owner=Some((nt2_local_ws::owner::Id::OsProcess(123),[1;16]));p.accepted=true;p.loaded=true;p.release=true;p.authority_until=Instant::now()+Duration::from_millis(150);p.generation=Some((generation.clone(),7));let until=p.authority_until;
 let mut pending=Some(p);let mut request=[0;32];request[..4].copy_from_slice(&super::super::handoff::MAGIC.to_le_bytes());request[4..8].copy_from_slice(&4u32.to_le_bytes());request[8..12].copy_from_slice(&5u32.to_le_bytes());request[16..32].fill(1);
 let mut reply=c2_transition(&mut pending,nt2_local_ws::owner::Id::OsProcess(123),&request);assert_eq!(reply.len(),48);assert_eq!(reply[8],1);assert_eq!(reply[12],2);let remaining=u32::from_le_bytes(reply[32..36].try_into().unwrap());assert!(remaining>0&&remaining<=150);assert_eq!(reply[36],7);assert_eq!(pending.as_ref().unwrap().authority_until,until);
 thread::sleep(Duration::from_millis(3));c2_fence(&pending,nt2_local_ws::owner::Id::OsProcess(123),&request,&mut reply);assert!(u32::from_le_bytes(reply[32..36].try_into().unwrap())<remaining);
 let accepted=reply.clone();generation.store(8,Ordering::Release);c2_fence(&pending,nt2_local_ws::owner::Id::OsProcess(123),&request,&mut reply);assert_eq!(&reply[8..16],&[0;8]);assert_eq!(&reply[32..40],&[0;8]);generation.store(7,Ordering::Release);
 reply=accepted.clone();c2_fence(&pending,nt2_local_ws::owner::Id::OsProcess(124),&request,&mut reply);assert_eq!(reply[8],0);
 reply=accepted.clone();pending.as_mut().unwrap().loaded=false;pending.as_mut().unwrap().expires=Instant::now();c2_fence(&pending,nt2_local_ws::owner::Id::OsProcess(123),&request,&mut reply);assert_eq!(reply[8],0);
 reply=accepted.clone();pending.as_mut().unwrap().loaded=true;pending.as_mut().unwrap().authority_until=Instant::now();c2_fence(&pending,nt2_local_ws::owner::Id::OsProcess(123),&request,&mut reply);assert_eq!(reply[8],0);
 reply=accepted;pending=None;c2_fence(&pending,nt2_local_ws::owner::Id::OsProcess(123),&request,&mut reply);assert_eq!(reply[8],0);assert_eq!(c2_transition(&mut pending,nt2_local_ws::owner::Id::OsProcess(123),&[0;3]),vec![0;48]);
}
#[test]
#[ignore = "explicit isolated initial-contact DLL/actual Lua callbacks; injected engine facts"]
fn g3_initial_contact_actual_callbacks_controlled_reply_delay() {
    use std::process::{Command, Stdio};
    use std::os::windows::process::CommandExt;
    let repo = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..").canonicalize().unwrap();
    let repo = std::path::PathBuf::from(repo.to_str().unwrap().trim_start_matches(r"\\?\"));
    let runner = repo.join("spikes/noita-entry/.build/Lua51Runner.exe");
    let lua = repo.join("spikes/noita-entry/.build/g3-inspection-build19-cc26b9c694054c1dac73888617aefc3f/social-Steam/lua51.dll");
    let dll = std::env::var("NT2_INITIAL_DLL").unwrap();
    let before = std::env::var("NT2_INITIAL_BEFORE").as_deref()==Ok("1");
    let base = repo.join("spikes/noita-entry/.build/g3-initial-contact").join(uuid::Uuid::new_v4().to_string());
    std::fs::create_dir_all(&base).unwrap();
    let mut evidence = Vec::new();
    let mut cases = if before {vec!["delay1","delay3","delay4","busy"]} else {vec!["delay1","delay3","delay4","busy","availability-race","heartbeat","over-bound","invalid-reply","expired","expires-in-flight","authority-expired","consumed","wrong-process","wrong-app","saved-run","generation","generation-in-flight","terminal","terminal-in-flight","wrong-seed","no-pipe"]};
    if !before{cases.extend(["heartbeat-negative","heartbeat-malformed","heartbeat-wrong-app","heartbeat-generation","heartbeat-terminal","heartbeat-exhausted","heartbeat-expired"]);}
    if let Ok(selected)=std::env::var("NT2_INITIAL_CASES"){cases.retain(|case|selected.split(',').any(|name|name==*case));assert!(!cases.is_empty());}
    for primary in [true,false] {for case in &cases {
        let own = base.join(format!("{}-{case}",if primary{"primary"}else{"social"}));
        let roaming = own.join("Roaming");let local = own.join("Local");
        std::fs::create_dir_all(&roaming).unwrap();std::fs::create_dir_all(local.join("NoitaTogether2/g3-dev")).unwrap();
        let image = verified_process_path(&std::env::current_exe().unwrap()).unwrap();
        std::fs::write(local.join("NoitaTogether2/g3-dev/bridge-app.txt"),if *case=="wrong-app" {"C:\\private-wrong-app.exe"} else {image.to_str().unwrap()}).unwrap();
        if *case=="saved-run" {let save=own.join("LocalLow/Nolla_Games_Noita/save00");std::fs::create_dir_all(&save).unwrap();std::fs::write(save.join("world_state.xml"),b"private sentinel").unwrap();}
        let generation=Arc::new(std::sync::atomic::AtomicU64::new(1));
        let mut p=super::super::handoff::new(424242,"owned-fixture-run".into(),1);
        p.native_v2=true;p.authority_until=Instant::now()+Duration::from_secs(12);
        p.generation=Some((generation.clone(),1));p.claim_path=Some(own.join("claims/world.bin"));
        if *case=="expired" {p.expires=Instant::now();}
        if *case=="expires-in-flight" {p.expires=Instant::now()+Duration::from_millis(50);}
        if *case=="authority-expired" {p.authority_until=Instant::now();}
        if *case=="consumed" {std::fs::create_dir_all(own.join("claims")).unwrap();std::fs::write(p.claim_path.as_ref().unwrap(),b"private previously consumed sentinel").unwrap();}
        let expires=p.expires;let authority=p.authority_until;
        let pending=Arc::new(Mutex::new(Some(p)));let stop=Arc::new(AtomicBool::new(false));
        let diagnostics=Arc::new(Mutex::new(NativeDiagnostics::default()));
        let delay=match *case {"delay1"|"expires-in-flight"|"generation-in-flight"|"terminal-in-flight"=>Some((1,100)),"delay3"=>Some((3,100)),"delay4"=>Some((4,100)),"over-bound"=>Some((1,350)),_=>None};
        diagnostics.lock().unwrap().initial_contact_delay=delay;
        if *case=="busy" {diagnostics.lock().unwrap().initial_contact_busy=Some((3,100));}
        if *case=="invalid-reply" {diagnostics.lock().unwrap().initial_contact_invalid_reply=true;}
        if *case=="heartbeat" {diagnostics.lock().unwrap().delay_next_heartbeat=true;}
        let expected=Arc::new(Mutex::new(Some(if *case=="wrong-process" {own.join("wrong-runner.exe")} else {runner.clone()})));
        let worker=if *case=="no-pipe" {None} else {Some(start_g3_named(format!(r"\\.\pipe\nt2-g3-initial-private-v1-{}",user_sid().unwrap()),pending.clone(),stop.clone(),expected,diagnostics.clone(),Arc::new(Mutex::new(nt2_desktop_g0::g3_runtime::Exchange::default()))).unwrap())};
        let log=std::fs::File::create(own.join("native-output.txt")).unwrap();
        let mut command=Command::new(&runner);command.arg(&lua).arg(repo.join("apps/desktop/tools/g3-initial-contact-test.lua"))
            .env("LOCALAPPDATA",&local).env("APPDATA",&roaming).env("NT2_INITIAL_DLL",&dll)
            .env("NT2_G3_MODULES",repo.join("mod/noita_together2/files/g3"))
            .env("NT2_INITIAL_FINAL_GATE",&own)
            .env("NT2_INITIAL_SPACING",if before&&*case!="busy" {"120"} else {"0"})
            .env("NT2_INITIAL_PRIMARY",if primary{"1"}else{"0"}).stdout(Stdio::from(log.try_clone().unwrap())).stderr(Stdio::from(log)).creation_flags(0x08000000);
        if matches!(*case,"generation"|"terminal") {command.env("NT2_INITIAL_GATE",&own);}
        if *case=="wrong-seed" {command.env("NT2_INITIAL_WRONG_SEED","1");}
        if *case=="heartbeat" {command.env("NT2_INITIAL_HEARTBEAT","1");}
        if case.starts_with("heartbeat-"){command.env("NT2_INITIAL_HEARTBEAT","1").env("NT2_INITIAL_HEARTBEAT_GUARD","1");}
        if *case=="heartbeat-exhausted"{command.env("NT2_HEARTBEAT_CONNECT_EXHAUST","1");}
        if *case=="availability-race" {command.env("NT2_INITIAL_CONNECT_RACE","1");}
        let mut child=command.spawn().unwrap();
        if case.starts_with("heartbeat-"){
            let end=Instant::now()+Duration::from_secs(3);while !own.join("heartbeat-ready").exists(){assert!(Instant::now()<end);thread::sleep(Duration::from_millis(5));}
            match *case{
                "heartbeat-negative"=>diagnostics.lock().unwrap().deny_next_heartbeat=true,
                "heartbeat-malformed"=>diagnostics.lock().unwrap().malform_next_heartbeat=true,
                "heartbeat-wrong-app"=>std::fs::write(local.join("NoitaTogether2/g3-dev/bridge-app.txt"),b"C:\\private-wrong-app.exe").unwrap(),
                "heartbeat-generation"=>generation.store(2,Ordering::Release),
                "heartbeat-terminal"=>*pending.lock().unwrap()=None,
                "heartbeat-expired"=>pending.lock().unwrap().as_mut().unwrap().authority_until=Instant::now(),
                _=>(),
            }
            std::fs::write(own.join("heartbeat-continue"),b"fixture guard applied").unwrap();
        }
        if matches!(*case,"generation-in-flight"|"terminal-in-flight") {
            let end=Instant::now()+Duration::from_secs(3);while !diagnostics.lock().unwrap().initial_contact_trace.iter().any(|e|e["op"]==1){assert!(Instant::now()<end);thread::sleep(Duration::from_millis(2));}
            if *case=="generation-in-flight" {generation.store(2,Ordering::Release);}else{*pending.lock().unwrap()=None;}
        }
        if matches!(*case,"generation"|"terminal") {let end=Instant::now()+Duration::from_secs(3);while !own.join("prepared").exists(){assert!(Instant::now()<end);thread::sleep(Duration::from_millis(5));}if *case=="generation" {generation.store(2,Ordering::Release);}else{*pending.lock().unwrap()=None;}std::fs::write(own.join("continue"),b"fixture gate").unwrap();}
        let end=Instant::now()+Duration::from_secs(5);while !own.join("done").exists(){if Instant::now()>=end{let _=child.kill();panic!("Private callback completion timeout: {}",own.display())}thread::sleep(Duration::from_millis(5));}
        thread::sleep(Duration::from_millis(if *case=="over-bound" {150}else{30}));
        let snapshot=pending.lock().unwrap().clone();
        std::fs::write(own.join("finish"),b"fixture gate").unwrap();
        let end=Instant::now()+Duration::from_secs(5);let status=loop {if let Some(s)=child.try_wait().unwrap(){break s}if Instant::now()>=end{let _=child.kill();panic!("Private callback timeout: {}",own.display())}thread::sleep(Duration::from_millis(5));};
        assert!(status.success());
        // Allow the intentionally late server response to finish before stopping its listener.
        thread::sleep(Duration::from_millis(if *case=="over-bound" {150}else{30}));
        stop.store(true,Ordering::Release);if let Some(worker)=worker {worker.join().unwrap();}
        let output=std::fs::read_to_string(own.join("native-output.txt")).unwrap();
        let d=diagnostics.lock().unwrap();let p=&snapshot;
        let complete=output.contains("prepare=true;accepted=true;loaded=true;held=true");
        let is_delay=case.starts_with("delay")||case.starts_with("heartbeat-")||matches!(*case,"busy"|"availability-race"|"heartbeat");
        assert_eq!(complete,is_delay&&!before,"{case}: {output}");
        if before&&*case=="delay1" {assert!(output.contains("op=1 stage=verified-read-timeout"));assert!(own.join("claims/world.bin").exists());assert!(p.as_ref().unwrap().owner.is_some());}
        if before&&*case=="delay3" {assert!(output.contains("op=3 stage=verified-read-timeout"));assert!(p.as_ref().unwrap().accepted);assert!(!p.as_ref().unwrap().loaded);}
        if before&&*case=="delay4" {assert!(output.contains("op=4 stage=verified-read-timeout"));assert!(p.as_ref().unwrap().loaded);}
        if before&&*case=="busy" {assert!(output.contains("op=4 stage=pipe-busy"));}
        if *case=="heartbeat" {assert!(output.contains("op=5 stage=verified-read-timeout"));assert!(output.contains("HEARTBEAT_RETAINED status=1"));assert!(output.contains("HEARTBEAT_DIAGNOSTIC reason=3"));}
        if case.starts_with("heartbeat-"){
            let reason=match *case{"heartbeat-negative"|"heartbeat-generation"|"heartbeat-terminal"=>9,"heartbeat-malformed"=>5,"heartbeat-wrong-app"=>2,"heartbeat-exhausted"=>6,_=>0};
            assert!(output.contains(&format!("HEARTBEAT_DIAGNOSTIC reason={reason}")),"{case}: {output}");
            assert!(output.contains(if *case=="heartbeat-expired"{"HEARTBEAT_RETAINED status=1"}else{"HEARTBEAT_RETAINED status=0"}),"Guard must keep the native world held: {case}: {output}");
            if *case=="heartbeat-exhausted"{assert!(output.contains("op=5 stage=test-connection-exhausted"));assert!(output.contains("budget_ms=80"));}
        }
        if *case=="availability-race" {assert!(output.contains("stage=test-availability-race-injected"));assert!(output.contains("stage=pipe-availability-raced"));}
        if !before&&*case=="over-bound" {assert!(output.contains("op=1 stage=verified-read-timeout"));}
        if matches!(*case,"expired"|"authority-expired"|"consumed"|"wrong-process"|"wrong-app"|"saved-run"|"no-pipe") {assert!(!output.contains("prepare=true"));}
        if *case=="wrong-app" {assert!(output.contains("stage=app-identity-refused"));}
        if *case=="no-pipe" {assert!(output.contains("stage=pipe-unavailable"));}
        if *case=="invalid-reply" {assert!(output.contains("stage=invalid-reply"));assert!(own.join("claims/world.bin").exists());}
        if *case=="saved-run" {assert!(output.contains("event=prepare|frame=1|detail=-2"));assert!(!own.join("claims/world.bin").exists());}
        if matches!(*case,"generation"|"terminal"|"wrong-seed"|"expires-in-flight"|"generation-in-flight"|"terminal-in-flight") {assert!(!output.contains("accepted=true"));}
        if let Some(p)=p.as_ref() {assert_eq!(p.expires,expires);if *case=="heartbeat-expired"{assert!(p.authority_until<=Instant::now());}else{assert_eq!(p.authority_until,authority);}assert!(!p.release);}
        let claims=d.initial_contact_trace.iter().filter(|e|e["op"]==1).count();assert!(claims<=1,"Actual entry must not call prepare again");
        let item=serde_json::json!({"entry":if primary{"primary"}else{"social"},"case":case,"delayMs":delay.map(|(_,ms)|ms),"callbacksComplete":complete,"op1Count":claims,"trace":d.initial_contact_trace,"originalExpiryUnchanged":true,"releaseGranted":false,"actualNoita":false});
        std::fs::write(own.join("result.json"),serde_json::to_vec_pretty(&item).unwrap()).unwrap();evidence.push(item);
        println!("INITIAL_CONTACT {} {case} complete={complete} op1={claims}",if primary{"primary"}else{"social"});
    }}
    std::fs::write(base.join("result.json"),serde_json::to_vec_pretty(&evidence).unwrap()).unwrap();
    println!("INITIAL_CONTACT_EVIDENCE {}",base.display());
}
#[test]
fn verified_process_normalizes_valid_alias_without_relaxing_installation_guards() {
    let executable = std::env::current_exe().unwrap();
    let canonical = verified_process_path(&executable).unwrap();
    let parent = canonical
        .parent()
        .unwrap()
        .to_str()
        .unwrap()
        .replace('\\', "/");
    let alias = std::path::PathBuf::from(format!(
        "{parent}/./{}",
        canonical.file_name().unwrap().to_str().unwrap()
    ));
    // Prior case-only comparison refused this otherwise valid local installation spelling.
    assert!(
        !canonical
            .to_string_lossy()
            .eq_ignore_ascii_case(&alias.to_string_lossy())
    );
    assert!(crate::installation::root(&format!("{parent}/.")).is_ok());
    assert!(matches_local_owner(crate::local_owner::Id::os_process(std::process::id()), &alias));
    assert!(!matches_local_owner(
        crate::local_owner::Id::os_process(std::process::id()),
        &canonical.parent().unwrap().join("different.exe")
    ));
    for path in [r"\\server\share\Noita", r"C:\Noita\..\Windows"] {
        assert!(crate::installation::root(path).is_err());
    }
}
#[test]
#[ignore = "requires test-production-inspection.ps1; actual production hint/save checks and original LuaJIT, isolated pipe/environment; no engine claim"]
fn production_inspection_hint_save_gate_and_verified_process() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .unwrap();
    let dll = std::path::PathBuf::from(std::env::var("NT2_INSPECT_PRODUCTION_TEST_DLL").unwrap());
    let quote = |path: &std::path::Path| {
        serde_json::to_string(
            &path
                .to_string_lossy()
                .trim_start_matches(r"\\?\")
                .replace('\\', "/"),
        )
        .unwrap()
    };
    assert!(
        dll.canonicalize()
            .unwrap()
            .starts_with(root.join("spikes/noita-entry/.build/g2-inspect-production-test"))
    );
    let mut checks = Vec::new();
    for scenario in [
        "valid",
        "alias",
        "saved",
        "missing-hint",
        "wrong-hint",
        "unverified",
        "unarmed",
    ] {
        let owned = root.join(format!(
            "spikes/noita-entry/.build/g2-inspect-production-test/{}",
            uuid::Uuid::new_v4()
        ));
        let local = owned.join("Local");
        let roaming = owned.join("Roaming");
        let hint = local.join("NoitaTogether2/g2-next/bridge-app.txt");
        std::fs::create_dir_all(hint.parent().unwrap()).unwrap();
        std::fs::create_dir_all(&roaming).unwrap();
        if scenario != "missing-hint" {
            std::fs::write(
                &hint,
                if scenario == "wrong-hint" {
                    verified_process_path(&root.join("spikes/noita-entry/.build/Lua51Runner.exe"))
                        .unwrap()
                } else {
                    std::env::current_exe().unwrap()
                }
                .to_string_lossy()
                .as_bytes(),
            )
            .unwrap();
        }
        let saved = owned.join("LocalLow/Nolla_Games_Noita/save00/player.xml");
        if scenario == "saved" {
            std::fs::create_dir_all(saved.parent().unwrap()).unwrap();
            std::fs::write(&saved, b"saved-world-sentinel").unwrap();
        }
        let claim = owned.join("claims/one-use");
        let mut p = crate::handoff::new(424242, "production-fixture".into(), 1);
        p.native_v2 = true;
        p.claim_path = Some(claim.clone());
        p.authority_until = Instant::now() + Duration::from_secs(30);
        let pending = Arc::new(Mutex::new(if scenario == "unarmed" {
            None
        } else {
            Some(p)
        }));
        let stop = Arc::new(AtomicBool::new(false));
        let diagnostics = Arc::new(Mutex::new(NativeDiagnostics::default()));
        let runner =
            verified_process_path(&root.join("spikes/noita-entry/.build/Lua51Runner.exe")).unwrap();
        let expected = if scenario == "unverified" {
            None
        } else if scenario == "alias" {
            Some(std::path::PathBuf::from(format!(
                "{}/./Lua51Runner.exe",
                runner
                    .parent()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .replace('\\', "/")
            )))
        } else {
            Some(runner.clone())
        };
        let worker = start_inspection_named(
            format!(
                r"\\.\pipe\nt2-g2-inspect-production-test-{}",
                user_sid().unwrap()
            ),
            pending.clone(),
            stop.clone(),
            Arc::new(Mutex::new(expected)),
            diagnostics.clone(),
        )
        .unwrap();
        let script = owned.join("worker.lua");
        let native = quote(&dll);
        let wanted = if matches!(scenario, "valid" | "alias") {
            1
        } else if scenario == "saved" {
            -2
        } else {
            0
        };
        std::fs::write(&script,format!(r#"local ffi=require('ffi');ffi.cdef[[void __stdcall Sleep(uint32_t);int nt2_g2i_prepare(uint32_t*,uint8_t*);int nt2_g2i_seed_accepted(const uint8_t*,uint32_t);int nt2_g2i_world_loaded(const uint8_t*);int nt2_g2i_status(const uint8_t*);int nt2_g2i_close(const uint8_t*);]]
local native=ffi.load({native});local kernel=ffi.load('kernel32');local seed=ffi.new('uint32_t[1]');local token=ffi.new('uint8_t[16]');local status=native.nt2_g2i_prepare(seed,token);assert(status=={wanted},'prepare result '..status)
if status==1 then assert(seed[0]==424242);assert(native.nt2_g2i_seed_accepted(token,seed[0])==1);assert(native.nt2_g2i_world_loaded(token)==1);assert(native.nt2_g2i_status(token)==1);kernel.Sleep(250);assert(native.nt2_g2i_close(token)==1) end
print('PASS production-path {scenario}: status '..status)
"#)).unwrap();
        let mut child = std::process::Command::new(&runner)
            .arg(r"C:\Noita\lua51.dll")
            .arg(script.to_string_lossy().trim_start_matches(r"\\?\"))
            .env("LOCALAPPDATA", &local)
            .env("APPDATA", &roaming)
            .spawn()
            .unwrap();
        let started = Instant::now();
        let mut loaded = false;
        let result = loop {
            loaded |= pending
                .lock()
                .unwrap()
                .as_ref()
                .is_some_and(|p| p.accepted && p.loaded);
            if let Some(result) = child.try_wait().unwrap() {
                break result;
            }
            if started.elapsed() > Duration::from_secs(8) {
                child.kill().unwrap();
                stop.store(true, Ordering::Release);
                worker.join().unwrap();
                panic!("Production fixture deadline");
            }
            thread::sleep(Duration::from_millis(5));
        };
        assert!(result.success());
        assert_eq!(claim.exists(), wanted == 1);
        assert_eq!(loaded, wanted == 1);
        // Give the still-running app listener its normal next poll after an early close.
        thread::sleep(Duration::from_millis(40));
        let view = diagnostics.lock().unwrap().view();
        assert!(view["contacts"].as_u64().unwrap() > 0);
        if scenario == "saved" {
            assert!(pending.lock().unwrap().is_none());
            assert_eq!(std::fs::read(saved).unwrap(), b"saved-world-sentinel");
            assert!(view["status"].as_str().unwrap().contains("Continue"));
        }
        if scenario == "unverified" {
            assert!(view["status"].as_str().unwrap().contains("not verified"));
        }
        if scenario == "unarmed" {
            assert!(
                view["status"]
                    .as_str()
                    .unwrap()
                    .contains("no current armed grant")
            );
        }
        if matches!(scenario, "missing-hint" | "wrong-hint") {
            assert!(view["status"].as_str().unwrap().contains("no handshake"));
            std::fs::write(
                &hint,
                std::env::current_exe()
                    .unwrap()
                    .to_string_lossy()
                    .as_bytes(),
            )
            .unwrap();
            let body = std::fs::read_to_string(&script)
                .unwrap()
                .replace("assert(status==0,", "assert(status==1,");
            std::fs::write(&script, body).unwrap();
            let mut retry = std::process::Command::new(&runner)
                .arg(r"C:\Noita\lua51.dll")
                .arg(script.to_string_lossy().trim_start_matches(r"\\?\"))
                .env("LOCALAPPDATA", &local)
                .env("APPDATA", &roaming)
                .spawn()
                .unwrap();
            let started = Instant::now();
            loop {
                if let Some(status) = retry.try_wait().unwrap() {
                    assert!(status.success());
                    break;
                }
                if started.elapsed() > Duration::from_secs(8) {
                    retry.kill().unwrap();
                    stop.store(true, Ordering::Release);
                    worker.join().unwrap();
                    panic!("Hint recovery deadline");
                }
                thread::sleep(Duration::from_millis(5));
            }
            assert!(claim.exists()); // Initial refusal did not consume; repaired hint can connect to the same listener.
        }
        stop.store(true, Ordering::Release);
        worker.join().unwrap();
        checks.push(scenario);
    }
    let result = serde_json::json!({"passed":true,"checks":checks,"productionAppHint":true,"productionSaveGate":true,"testAppPathBypass":false,"noSaveBypass":false,"actualNoita":false,"scope":"original LuaJIT/production bridge code, isolated child env and pipe; authority simulated"});
    std::fs::write(
        root.join("spikes/noita-entry/.build/g2-inspect-production-test/result.json"),
        serde_json::to_vec_pretty(&result).unwrap(),
    )
    .unwrap();
}
#[test]
#[ignore = "requires test-native-inspection.ps1; candidate mod, game APIs stubbed, authority simulated"]
fn candidate_mod_pre_start_revalidation_and_restore() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .unwrap();
    let dll = std::path::PathBuf::from(std::env::var("NT2_INSPECT_TEST_DLL").unwrap());
    assert!(
        dll.canonicalize()
            .unwrap()
            .starts_with(root.join("spikes/noita-entry/.build/g2-inspect-test"))
    );
    let quote = |path: &std::path::Path| {
        serde_json::to_string(
            &path
                .to_string_lossy()
                .trim_start_matches(r"\\?\")
                .replace('\\', "/"),
        )
        .unwrap()
    };
    for scenario in [
        "after-release",
        "before-release",
        "wrong-seed",
        "missing-mods",
    ] {
        let scenario_json = serde_json::to_string(scenario).unwrap();
        let script = root.join("spikes/noita-entry/.build/g2-inspect-test/mod-worker.lua");
        let native = quote(&dll);
        let collector = quote(&root.join("mod/noita_together2/files/inspection.lua"));
        let entry = quote(&root.join("mod/noita_together2/init-inspect.lua"));
        let stubs = quote(&root.join("apps/desktop/tools/mod-protection-stubs.lua"));
        std::fs::write(&script,format!(r#"
local ffi=require('ffi');ffi.cdef[[void __stdcall Sleep(uint32_t);]]
local native=ffi.load({native});local kernel=ffi.load('kernel32');local real_load=ffi.load;local real_dofile=dofile
ffi.load=function(path) if path=='mods/noita_together2/files/nt2_g2_inspect_bridge.dll' then return native end return real_load(path) end
dofile=function(path) if path=='mods/noita_together2/files/inspection.lua' then return real_dofile({collector}) end return real_dofile(path) end
dofile({stubs})
local mods={{'noita_together2','other'}}
local scenario={scenario_json}
function StatsGetValue() return scenario=='wrong-seed' and '424243' or '424242' end
function ModGetActiveModIDs() if scenario=='missing-mods' then error('collector unavailable') end return mods end
function GameIsBetaBuild() return false end
dofile({entry});OnModPreInit();OnMagicNumbersAndWorldSeedInitialized();OnPlayerSpawned(99)
local s=NT2_FIXTURE
assert(not s.components[1].enabled and not s.enabled[5] and not s.enabled[6] and not s.enabled[7] and s.poly and s.effect)
kernel.Sleep(2200);OnWorldPreUpdate();assert(not s.components[1].enabled)
if scenario=='before-release' then mods={{'other','noita_together2'}} end
kernel.Sleep(2300);OnWorldPreUpdate()
if scenario=='after-release' then
assert(s.components[1].enabled and s.enabled[5] and s.enabled[6] and not s.enabled[7] and not s.poly and not s.effect)
mods={{'other','noita_together2'}};s.frame=300;OnWorldPreUpdate()
assert(not s.components[1].enabled and s.poly and s.effect and not s.enabled[5])
kernel.Sleep(50)
mods={{'noita_together2','other'}};s.frame=600;OnWorldPreUpdate()
assert(not s.components[1].enabled) -- post-authorization change cannot silently self-reapprove
else assert(not s.components[1].enabled and s.poly and s.effect) end
s.button=true;OnWorldPostUpdate()
assert(s.components[1].enabled and not s.poly and not s.effect and not s.components[3] and s.enabled[5] and s.enabled[6] and not s.enabled[7])
print('PASS candidate mod '..scenario..': protection, fresh observations and explicit offline restoration; engine APIs stubbed, authority simulated')
"#)).unwrap();
        let claim = root.join(format!(
            "spikes/noita-entry/.build/g2-inspect-test/claim-{}",
            uuid::Uuid::new_v4()
        ));
        let mut p = crate::handoff::new(424242, "candidate-mod-run".into(), 1);
        p.native_v2 = true;
        p.claim_path = Some(claim.clone());
        p.authority_until = Instant::now() + Duration::from_secs(30);
        let pending = Arc::new(Mutex::new(Some(p)));
        let stop = Arc::new(AtomicBool::new(false));
        let worker = start_inner_mode(
            format!(r"\\.\pipe\nt2-g2-inspect-test-v1-{}", user_sid().unwrap()),
            pending.clone(),
            stop.clone(),
            |_| true,
            true,
        )
        .unwrap();
        let mut child =
            std::process::Command::new(root.join("spikes/noita-entry/.build/Lua51Runner.exe"))
                .arg(r"C:\Noita\lua51.dll")
                .arg(script.to_string_lossy().trim_start_matches(r"\\?\"))
                .spawn()
                .unwrap();
        let started = Instant::now();
        let mut authorized = false;
        let mut changed = false;
        let mut ineligible = false;
        let result = loop {
            if let Some(p) = pending.lock().unwrap().as_mut() {
                if let Some(report) = p.observation.as_ref() {
                    ineligible |= !report.observation.complete
                        || report.observation.actual_seed.as_deref() != Some("424242");
                    if report.sequence == 1
                        && started.elapsed() > Duration::from_millis(2350)
                        && !authorized
                        && report.observation.complete
                        && report.observation.actual_seed.as_deref() == Some("424242")
                    {
                        assert!(p.accepted && p.loaded);
                        assert!(report.observation.protected);
                        assert!(!p.release);
                        p.release = true;
                        authorized = true;
                    }
                    if report
                        .observation
                        .mods
                        .first()
                        .is_some_and(|id| id == "other")
                    {
                        changed = true;
                        assert!(!p.release);
                    }
                }
            }
            if let Some(result) = child.try_wait().unwrap() {
                break result;
            }
            if started.elapsed() > Duration::from_secs(12) {
                child.kill().unwrap();
                stop.store(true, Ordering::Release);
                worker.join().unwrap();
                panic!("Candidate mod test deadline");
            }
            thread::sleep(Duration::from_millis(10));
        };
        stop.store(true, Ordering::Release);
        worker.join().unwrap();
        let _ = std::fs::remove_file(claim);
        assert!(result.success());
        if matches!(scenario, "wrong-seed" | "missing-mods") {
            assert!(!authorized && ineligible);
        } else {
            assert!(authorized && changed);
        }
    }
}
#[test]
#[ignore = "requires test-native-inspection.ps1; installed LuaJIT, game APIs stubbed, authority simulated"]
fn native_inspection_report_and_paused_snapshot() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .unwrap();
    let dll = std::path::PathBuf::from(std::env::var("NT2_INSPECT_TEST_DLL").unwrap());
    assert!(
        dll.canonicalize()
            .unwrap()
            .starts_with(root.join("spikes/noita-entry/.build/g2-inspect-test"))
    );
    let quote = |path: &std::path::Path| {
        serde_json::to_string(
            &path
                .to_string_lossy()
                .trim_start_matches(r"\\?\")
                .replace('\\', "/"),
        )
        .unwrap()
    };
    let script = root.join("spikes/noita-entry/.build/g2-inspect-test/worker.lua");
    let native = quote(&dll);
    let collector = quote(&root.join("mod/noita_together2/files/inspection.lua"));
    std::fs::write(&script,format!(r#"
local ffi=require('ffi')
ffi.cdef[[void __stdcall Sleep(uint32_t);int nt2_g2i_prepare(uint32_t*,uint8_t*);int nt2_g2i_seed_accepted(const uint8_t*,uint32_t);int nt2_g2i_world_loaded(const uint8_t*);int nt2_g2i_status(const uint8_t*);int nt2_g2i_close(const uint8_t*);int nt2_g2i_report(const uint8_t*,const uint8_t*,uint32_t,uint64_t);]]
local native=ffi.load({native});local kernel=ffi.load('kernel32');local collect=dofile({collector})
local seed=ffi.new('uint32_t[1]');local token=ffi.new('uint8_t[16]')
function StatsGetValue() return '424242' end
local mods={{'noita_together2','other'}}
function ModGetActiveModIDs() return mods end
function GameIsBetaBuild() return false end
assert(native.nt2_g2i_prepare(seed,token)==1 and seed[0]==424242)
assert(native.nt2_g2i_seed_accepted(token,seed[0])==1)
assert(native.nt2_g2i_world_loaded(token)==1)
local report=collect.encode(collect.collect(true,true))
assert(native.nt2_g2i_report(token,report,#report,1)==1)
kernel.Sleep(2200) -- Only native heartbeat runs: it must not recapture observations or authorize play.
assert(native.nt2_g2i_status(token)==1)
assert(native.nt2_g2i_report(token,report,#report,1)==1) -- exact retransmission, not fresh collection
assert(native.nt2_g2i_report(token,'{{}}',2,2)==0)
local wrong=ffi.new('uint8_t[16]');assert(native.nt2_g2i_report(wrong,report,#report,2)==0)
kernel.Sleep(2300)
assert(native.nt2_g2i_status(token)==2) -- isolated controller explicitly simulates host authorization
mods={{'other','noita_together2'}}
local changed=collect.encode(collect.collect(true,true))
assert(native.nt2_g2i_report(token,changed,#changed,1)==0) -- same sequence conflict
assert(native.nt2_g2i_report(token,changed,#changed,2)==1)
assert(native.nt2_g2i_status(token)==1) -- the changed report invalidates cached release immediately
kernel.Sleep(2200);assert(native.nt2_g2i_status(token)==1)
assert(native.nt2_g2i_close(token)==1)
print('PASS bounded native report framing, atomic/replay/context validation, paused snapshot retained, loaded-before-host hold and content-change hold; authority simulated, engine APIs stubbed')
"#)).unwrap();
    let claim = root.join(format!(
        "spikes/noita-entry/.build/g2-inspect-test/claim-{}",
        uuid::Uuid::new_v4()
    ));
    let mut p = crate::handoff::new(424242, "inspection-run".into(), 1);
    p.native_v2 = true;
    p.claim_path = Some(claim.clone());
    p.authority_until = Instant::now() + Duration::from_secs(30);
    let pending = Arc::new(Mutex::new(Some(p)));
    let stop = Arc::new(AtomicBool::new(false));
    let worker = start_inner_mode(
        format!(r"\\.\pipe\nt2-g2-inspect-test-v1-{}", user_sid().unwrap()),
        pending.clone(),
        stop.clone(),
        |_| true,
        true,
    )
    .unwrap();
    let mut child =
        std::process::Command::new(root.join("spikes/noita-entry/.build/Lua51Runner.exe"))
            .arg(r"C:\Noita\lua51.dll")
            .arg(script.to_string_lossy().trim_start_matches(r"\\?\"))
            .spawn()
            .unwrap();
    let started = Instant::now();
    let mut observed = None;
    let mut captured = None;
    let mut pulse = false;
    let mut authorized = false;
    let result = loop {
        if let Some(p) = pending.lock().unwrap().as_mut() {
            if let Some(report) = p.observation.as_ref() {
                if report.sequence == 1 {
                    if observed.is_none() {
                        observed = Some(report.observed_at);
                        captured = Some(report.digest);
                    }
                    assert_eq!(observed, Some(report.observed_at));
                    if started.elapsed() > Duration::from_millis(2250) && !authorized {
                        assert!(!p.release);
                        assert!(p.loaded);
                        p.release = true;
                        authorized = true;
                    }
                } else {
                    assert_eq!(report.sequence, 2);
                    assert_ne!(captured, Some(report.digest));
                    assert!(!p.release);
                }
            }
            pulse |= p
                .last_seen
                .is_some_and(|seen| seen.duration_since(started) > Duration::from_millis(1800));
        }
        if let Some(result) = child.try_wait().unwrap() {
            break result;
        }
        if started.elapsed() > Duration::from_secs(12) {
            child.kill().unwrap();
            stop.store(true, Ordering::Release);
            worker.join().unwrap();
            panic!("Native report test deadline");
        }
        thread::sleep(Duration::from_millis(10));
    };
    stop.store(true, Ordering::Release);
    worker.join().unwrap();
    let _ = std::fs::remove_file(claim);
    assert!(result.success());
    assert!(pulse && authorized);
}
#[test]
#[ignore = "requires test-native-g2.ps1 and original installed LuaJIT; game APIs are stubs"]
fn g2_native_context_release_and_pause() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .unwrap();
    let dll = std::path::PathBuf::from(std::env::var("NT2_G2_NATIVE_TEST_DLL").unwrap());
    assert!(
        dll.canonicalize()
            .unwrap()
            .starts_with(root.join("spikes/noita-entry/.build/g2-native-test"))
    );
    let quoted = serde_json::to_string(
        &dll.to_string_lossy()
            .trim_start_matches(r"\\?\")
            .replace('\\', "/"),
    )
    .unwrap();
    let mod_file = serde_json::to_string(
        &root
            .join("mod/noita_together2/init.lua")
            .to_string_lossy()
            .trim_start_matches(r"\\?\")
            .replace('\\', "/"),
    )
    .unwrap();
    let script = root.join("spikes/noita-entry/.build/g2-native-test/worker.lua");
    std::fs::write(&script,format!(r#"
local ffi=require('ffi');ffi.cdef[[void __stdcall Sleep(uint32_t);]]
local kernel=ffi.load('kernel32');local native=ffi.load({quoted});local original_load=ffi.load;local original_new=ffi.new;local old
ffi.load=function(path) if path=='mods/noita_together2/files/nt2_g2_bridge.dll' then return native end return original_load(path) end
ffi.new=function(kind,...) local result=original_new(kind,...);if kind=='uint8_t[16]' then old=result end return result end
local components={{[1]={{enabled=true}},[2]={{wait_for_kill_flag_on_death=false}},[4]={{frames=1}}}}
local poly,effect,button=false,false,false;local enabled={{[5]=true,[6]=true,[7]=false}}
function dofile_once() end;function ModMagicNumbersFileAdd() end;function GameGetFrameNum() return 0 end
function SetWorldSeed(v) assert(v==424242) end;function StatsGetValue() return '424242' end
function EntityGetIsAlive(id) return id==99 or id==999 and effect end
function EntityGetFirstComponentIncludingDisabled(id,kind) if id==999 then return kind=='GameEffectComponent' and 4 end return kind=='ControlsComponent' and 1 or kind=='DamageModelComponent' and 2 or kind=='InventoryGuiComponent' and 5 or kind=='Inventory2Component' and 6 or kind=='ItemPickUpperComponent' and 7 end
function EntityGetComponentIncludingDisabled(_,kind) if kind=='VariableStorageComponent' and components[3] then return {{3}} end return {{}} end
function ComponentGetIsEnabled(id) return enabled[id] end
function EntitySetComponentIsEnabled(_,id,value) enabled[id]=value end
function ComponentGetValue2(id,key) return components[id][key] end
function ComponentSetValue2(id,key,value) components[id][key]=value end
function EntityAddComponent2(_,_,values) components[3]=values;return 3 end
function EntityRemoveComponent(_,id) components[id]=nil end
function EntityGetTransform() return 227,-85 end;function EntitySetTransform() end
function EntityHasTag(id) return id==99 and poly or id==999 and effect end
function EntityAddTag(id) if id==99 then poly=true end end;function EntityRemoveTag(id) if id==99 then poly=false end end
function LoadGameEffectEntityTo() effect=true;return 999 end
function EntityKill() effect=false end;function EntityGetAllChildren() return effect and {{999}} or {{}} end
function GuiCreate() return 1 end;function GuiStartFrame() end;function GuiText() end;function GuiButton() return button end
dofile({mod_file});OnModPreInit();local stale=old;OnMagicNumbersAndWorldSeedInitialized();OnPlayerSpawned(99)
assert(components[1].enabled==false and poly and effect);assert(components[3].value_string=='101110' and not enabled[5] and not enabled[6] and not enabled[7])
kernel.Sleep(2700) -- native lease renews with no Lua/game callback
assert(native.nt2_g2_status(stale)==2);OnWorldPreUpdate();assert(components[1].enabled and not poly and not effect and not components[3] and enabled[5] and enabled[6] and not enabled[7])
kernel.Sleep(2700);OnWorldPreUpdate();assert(not components[1].enabled and poly and effect)
button=true;OnWorldPostUpdate();assert(components[1].enabled and not poly and not effect and not components[3] and enabled[5] and enabled[6] and not enabled[7])
kernel.Sleep(600)
local context=original_new('uint8_t[16]');local seed=original_new('uint32_t[1]');assert(native.nt2_g2_prepare(seed,context)==1)
assert(native.nt2_g2_close(stale)==0);assert(native.nt2_g2_status(context)==1)
assert(native.nt2_g2_seed_accepted(context,1)==0);assert(native.nt2_g2_world_loaded(context)==0)
assert(native.nt2_g2_close(context)==1)
-- Simulate a saved held world, then run the normal player-spawn restoration path.
components[1].enabled=false;components[2].wait_for_kill_flag_on_death=true;components[3]={{name='nt2_g2_recovery',value_string='101'}};poly=true;effect=true
OnPlayerSpawned(99);assert(components[1].enabled and not components[2].wait_for_kill_flag_on_death and not poly and not effect)
print('PASS G2 source-built native lease/release/suspension, stale token rejection, mod protection restore and saved marker recovery; stub game APIs')
"#)).unwrap();
    let claim_root = root.join(format!(
        "spikes/noita-entry/.build/g2-native-test/claims-{}",
        uuid::Uuid::new_v4()
    ));
    let make = |name: &str| {
        let mut p = crate::handoff::new(424242, name.into(), 1);
        p.native_v2 = true;
        p.claim_path = Some(claim_root.join(name));
        p.authority_until = Instant::now() + Duration::from_secs(30);
        p
    };
    let pending = Arc::new(Mutex::new(Some(make("first"))));
    let stop = Arc::new(AtomicBool::new(false));
    let worker = start_inner(
        format!(r"\\.\pipe\nt2-g2-test-v1-{}", user_sid().unwrap()),
        pending.clone(),
        stop.clone(),
        |_| true,
    )
    .unwrap();
    let mut child =
        std::process::Command::new(root.join("spikes/noita-entry/.build/Lua51Runner.exe"))
            .arg(r"C:\Noita\lua51.dll")
            .arg(script.to_string_lossy().trim_start_matches(r"\\?\"))
            .spawn()
            .unwrap();
    let start = Instant::now();
    let mut renewed = false;
    let mut replaced = false;
    let status = loop {
        {
            let mut local = pending.lock().unwrap();
            if let Some(p) = local.as_mut() {
                if !replaced && p.loaded {
                    p.release = start.elapsed() < Duration::from_millis(3000);
                    renewed |= p
                        .last_seen
                        .is_some_and(|t| t.duration_since(start) > Duration::from_millis(1900));
                }
            } else if !replaced {
                *local = Some(make("second"));
                replaced = true;
            }
        }
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if start.elapsed() > Duration::from_secs(15) {
            child.kill().unwrap();
            panic!("G2 native harness timed out");
        }
        thread::sleep(Duration::from_millis(10));
    };
    stop.store(true, Ordering::Release);
    worker.join().unwrap();
    for file in ["first", "second"] {
        let _ = std::fs::remove_file(claim_root.join(file));
    }
    let _ = std::fs::remove_dir(claim_root);
    assert!(status.success());
    assert!(renewed);
    assert!(replaced);
}
#[test]
#[ignore = "requires separately built x86 test DLL and original installed LuaJIT"]
fn native_worker_renews_without_lua_updates() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .unwrap();
    let dll = std::path::PathBuf::from(
        std::env::var("NT2_NATIVE_TEST_DLL").expect("run test-native-g1.ps1"),
    );
    assert!(
        dll.canonicalize()
            .unwrap()
            .starts_with(root.join("spikes/noita-entry/.build/g1-native-test"))
    );
    let script = root.join("spikes/noita-entry/.build/g1-native-test/worker.lua");
    let quoted = serde_json::to_string(&dll.to_string_lossy().replace('\\', "/")).unwrap();
    let mod_file = serde_json::to_string(
        &root
            .join("mod/nt2_g0_handoff/init.lua")
            .to_string_lossy()
            .trim_start_matches(r"\\?\")
            .replace('\\', "/"),
    )
    .unwrap();
    std::fs::write(&script,format!(r#"local ffi=require('ffi')
ffi.cdef[[int nt2_g0_prepare(uint32_t*);int nt2_g0_seed_accepted(void);int nt2_g0_world_loaded(void);int nt2_g1_context_closed(void);void __stdcall Sleep(uint32_t);]]
local bridge=ffi.load({quoted});local kernel=ffi.load('kernel32');local seed=ffi.new('uint32_t[1]')
local original_load=ffi.load
ffi.load=function(path) if path=='mods/nt2_g0_handoff/files/nt2_g0_bridge.dll' then return bridge end return original_load(path) end
function ModMagicNumbersFileAdd() end
function GameGetFrameNum() return 0 end
function SetWorldSeed(value) assert(value==424242) end
function StatsGetValue() return '424242' end
dofile({mod_file})
OnModPreInit();OnMagicNumbersAndWorldSeedInitialized();OnPlayerSpawned(99)
kernel.Sleep(6500) -- no Lua/game update callbacks throughout this pause
OnModPreInit=nil;OnMagicNumbersAndWorldSeedInitialized=nil;OnPlayerSpawned=nil;OnWorldPostUpdate=nil
collectgarbage('collect');collectgarbage('collect')
print('PASS native worker lease without Lua updates, then context closed')
"#)).unwrap();
    let pending = Arc::new(Mutex::new(Some(crate::handoff::new(
        424242,
        "native-test".into(),
        1,
    ))));
    let stop = Arc::new(AtomicBool::new(false));
    let name = format!(r"\\.\pipe\nt2-g1-test-v1-{}", user_sid().unwrap());
    let worker = start_inner(name, pending.clone(), stop.clone(), |_| true).unwrap();
    let start = Instant::now();
    let mut child =
        std::process::Command::new(root.join("spikes/noita-entry/.build/Lua51Runner.exe"))
            .arg(r"C:\Noita\lua51.dll")
            .arg(script.to_string_lossy().trim_start_matches(r"\\?\"))
            .spawn()
            .unwrap();
    let mut renewed = false;
    let status = loop {
        if pending.lock().unwrap().as_ref().is_some_and(|p| {
            p.loaded
                && p.last_seen
                    .is_some_and(|t| t.duration_since(start) > Duration::from_secs(4))
        }) {
            renewed = true;
        }
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if start.elapsed() > Duration::from_secs(12) {
            child.kill().unwrap();
            panic!("native fixture timeout")
        }
        thread::sleep(Duration::from_millis(10));
    };
    stop.store(true, Ordering::Release);
    worker.join().unwrap();
    assert!(status.success());
    assert!(renewed, "heartbeat did not renew without Lua callbacks");
    assert!(
        pending.lock().unwrap().is_none(),
        "closed nonce still owned the handoff"
    );
}
fn test_name() -> String {
    format!(
        r"\\.\pipe\nt2-g0-test-{}-{}",
        crate::local_owner::Id::os_process(std::process::id()),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    )
}
fn connect(name: &str) -> Owned {
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        let h = unsafe {
            CreateFileW(
                wide(name).as_ptr(),
                0xc0000000,
                0,
                null_mut(),
                3,
                0,
                null_mut(),
            )
        };
        if h as isize != -1 {
            // Nonblocking client reads keep the harness bounded too.
            assert_ne!(
                unsafe { SetNamedPipeHandleState(h, &1, null_mut(), null_mut()) },
                0
            );
            return Owned(h);
        }
        assert!(
            Instant::now() < deadline,
            "test pipe did not become available"
        );
        thread::sleep(Duration::from_millis(2));
    }
}
fn exchange(name: &str, bytes: &[u8]) -> Option<[u8; 32]> {
    let h = connect(name);
    let mut n = 0;
    if unsafe {
        WriteFile(
            h.0,
            bytes.as_ptr().cast(),
            bytes.len() as u32,
            &mut n,
            null_mut(),
        )
    } == 0
    {
        return None;
    }
    let deadline = Instant::now() + Duration::from_millis(300);
    while Instant::now() < deadline {
        let mut reply = [0u8; 32];
        if unsafe { ReadFile(h.0, reply.as_mut_ptr().cast(), 32, &mut n, null_mut()) } != 0 {
            return (n == 32).then_some(reply);
        }
        if unsafe { GetLastError() } != 232 {
            return None;
        }
        thread::sleep(Duration::from_millis(1));
    }
    None
}
fn request() -> [u8; 32] {
    let mut b = [0; 32];
    b[0..4].copy_from_slice(&super::super::handoff::MAGIC.to_le_bytes());
    b[4..8].copy_from_slice(&1u32.to_le_bytes());
    b[8..12].copy_from_slice(&1u32.to_le_bytes());
    b[16..32].fill(7);
    b
}
#[test]
fn actual_single_owner_and_bounded_shutdown() {
    let name = test_name();
    let pending = Arc::new(Mutex::new(None));
    let stop = Arc::new(AtomicBool::new(false));
    let worker = start_inner(name.clone(), pending.clone(), stop.clone(), |_| true).unwrap();
    assert!(start_inner(name, pending, Arc::new(AtomicBool::new(false)), |_| true).is_err());
    let now = Instant::now();
    stop.store(true, Ordering::Relaxed);
    worker.join().unwrap();
    assert!(now.elapsed() < Duration::from_millis(500));
}
#[test]
fn malformed_short_oversized_burst_then_valid() {
    let name = test_name();
    let pending = Arc::new(Mutex::new(Some(super::super::handoff::new(
        424242,
        "run".into(),
        1,
    ))));
    let stop = Arc::new(AtomicBool::new(false));
    let worker = start_inner(name.clone(), pending.clone(), stop.clone(), |_| true).unwrap();
    for bytes in [&[0u8; 3][..], &[0u8; 64][..]] {
        assert!(exchange(&name, bytes).is_none())
    }
    for _ in 0..128 {
        let reply = exchange(&name, &[0u8; 32]).unwrap();
        assert_eq!(&reply[8..32], &[0; 24])
    }
    assert!(pending.lock().unwrap().as_ref().unwrap().owner.is_none());
    let reply = exchange(&name, &request()).unwrap();
    assert_eq!(reply[8], 1);
    assert_eq!(
        u32::from_le_bytes(reply[12..16].try_into().unwrap()),
        424242
    );
    stop.store(true, Ordering::Relaxed);
    worker.join().unwrap();
}
#[test]
fn actual_process_rejection_and_idle_shutdown() {
    let name = test_name();
    let pending = Arc::new(Mutex::new(Some(super::super::handoff::new(
        1,
        "r".into(),
        1,
    ))));
    let stop = Arc::new(AtomicBool::new(false));
    let worker = start_inner(name.clone(), pending.clone(), stop.clone(), is_game).unwrap();
    assert!(exchange(&name, &request()).is_none());
    assert!(pending.lock().unwrap().as_ref().unwrap().owner.is_none());
    stop.store(true, Ordering::Relaxed);
    worker.join().unwrap();
    let name = test_name();
    let stop = Arc::new(AtomicBool::new(false));
    let worker = start_inner(
        name.clone(),
        Arc::new(Mutex::new(None)),
        stop.clone(),
        |_| true,
    )
    .unwrap();
    let _idle = connect(&name);
    thread::sleep(Duration::from_millis(20));
    let now = Instant::now();
    stop.store(true, Ordering::Relaxed);
    worker.join().unwrap();
    assert!(now.elapsed() < Duration::from_millis(500));
}
#[test]
#[ignore = "requires test-g3-transport.ps1; real x86 DLL/OS pipe, injected game and service facts"]
fn actual_g3_transport_owner_protected_start_pose_pause_and_cleanup(){
    use nt2_desktop_g0::{g3_runtime::{Exchange,Runtime},g3_journal::Journal,g3_local_consumer::Binding,protocol_g3::wire};
    let canonical=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..").canonicalize().unwrap();
    let repo=std::path::PathBuf::from(canonical.to_str().unwrap().trim_start_matches(r"\\?\"));
    let dll=std::env::var("NT2_G3_TRANSPORT_DLL").expect("explicit candidate DLL required");
    let dir=repo.join("spikes/noita-entry/.build/g3-transport-tests").join(uuid::Uuid::new_v4().to_string());
    let local=dir.join("Local");let roaming=dir.join("Roaming");
    std::fs::create_dir_all(local.join("NoitaTogether2/g3-dev")).unwrap();std::fs::create_dir_all(&roaming).unwrap();
    std::fs::write(local.join("NoitaTogether2/g3-dev/bridge-app.txt"),std::env::current_exe().unwrap().canonicalize().unwrap().to_str().unwrap().trim_start_matches(r"\\?\")).unwrap();
    let runner=repo.join("spikes/noita-entry/.build/Lua51Runner.exe").canonicalize().unwrap();
    let mut p=super::super::handoff::new(424242,"run-fixture".into(),1);p.epoch="run-epoch-fixture".into();p.native_v2=true;
    p.authority_until=Instant::now()+Duration::from_secs(20);
    p.claim_path=Some(dir.join("claims/world-fixture.bin"));
    let generation=Arc::new(std::sync::atomic::AtomicU64::new(1));p.generation=Some((generation,1));
    let pending=Arc::new(Mutex::new(Some(p)));let stop=Arc::new(AtomicBool::new(false));let shared=Arc::new(Mutex::new(Exchange::default()));
    let worker=start_g3(pending.clone(),stop.clone(),Arc::new(Mutex::new(Some(runner.clone()))),Arc::new(Mutex::new(NativeDiagnostics::default())),shared.clone()).unwrap();
    // Same Windows SID is insufficient: the verified executable must match.
    assert!(exchange(&format!(r"\\.\pipe\nt2-g3-dev-v1-{}",user_sid().unwrap()),&request()).is_none());
    let lua=repo.join("apps/desktop/tools/g3-transport-test.lua");
    let output=std::fs::File::create(dir.join("native-output.txt")).unwrap();
    let mut child=std::process::Command::new(runner).arg(r"C:\Noita\lua51.dll").arg(lua)
        .env("NT2_G3_TRANSPORT_DLL",dll).env("NT2_G3_TRANSPORT_DIR",&dir).env("LOCALAPPDATA",&local).env("APPDATA",&roaming)
        .stdout(output.try_clone().unwrap()).stderr(output).spawn().unwrap();
    let result=(||->Result<(),String>{
        let wait=|file:&str|->Result<(),String>{let end=Instant::now()+Duration::from_secs(8);while !dir.join(file).exists(){
            if Instant::now()>end{return Err(format!("Timed out {file}"))}thread::sleep(Duration::from_millis(10));}Ok(())};
        wait("protected")?;
        {let p=pending.lock().unwrap();let p=p.as_ref().ok_or("missing protected pending")?;
            assert_eq!(p.owner.unwrap().0,child.id());assert!(p.loaded&&p.accepted&&!p.release);
            assert!(p.observation.as_ref().unwrap().observation.protected);}
        pending.lock().unwrap().as_mut().unwrap().release=true;std::fs::write(dir.join("start"),b"fixture Start") .unwrap();
        wait("playing")?;
        let context={let p=pending.lock().unwrap();let p=p.as_ref().unwrap();assert!(!p.observation.as_ref().unwrap().observation.protected);p.owner.unwrap().1};
        let binding=Binding{context,service_origin:"https://fixture.invalid".into(),service_id:"service-fixture".into(),
            service_epoch:"service-epoch-fixture".into(),session_epoch:"session-fixture".into(),principal_id:"principal-fixture".into(),
            scope:wire::Scope{room_id:Some("room-fixture".into()),run_id:Some("run-fixture".into()),run_epoch:Some("run-epoch-fixture".into()),
                config_revision:Some(1),attachment_id:Some("attachment-fixture".into()),world_token:Some(context.iter().map(|b|format!("{b:02x}")).collect())}};
        let mut runtime=Runtime::new(Journal::open(&dir.join("journal.jsonl"))?,shared.clone());
        runtime.authorize(binding,zeroize::Zeroizing::new("fixture-only-secret".into()),nt2_local_ws::owner::Id::OsProcess(child.id()),Instant::now()+Duration::from_secs(20),reqwest::Client::new())?;
        std::fs::write(dir.join("authorized"),b"fixture authority") .unwrap();wait("pose")?;
        let end=Instant::now()+Duration::from_secs(3);let frame=loop{
            if let Some(frame)=shared.lock().unwrap().take_pose(){break frame;}if Instant::now()>end{return Err("No native worker pose".into())}thread::sleep(Duration::from_millis(10));};
        let pose=nt2_desktop_g0::g3_native_frame::decode(&frame,&context,0)?;assert_eq!(pose.x_milli.as_deref(),Some("-1250"));assert_eq!(pose.sequence,Some(1));
        wait("peers")?; // Actual 864-byte poll reply reaches x86 on its worker.
        pending.lock().unwrap().as_mut().unwrap().authority_until=Instant::now();
        std::fs::write(dir.join("pause"),b"hold") .unwrap();wait("closed")?;
        assert!(pending.lock().unwrap().is_none());assert!(shared.lock().unwrap().take_pose().is_none());
        assert_eq!(runtime.view()["native_playing"],false);
        // Original production Continue guard reads only this child-owned save tree.
        let save=dir.join("LocalLow/Nolla_Games_Noita/save00");std::fs::create_dir_all(&save).unwrap();
        std::fs::write(save.join("player.xml"),b"fixture sentinel; no game data").unwrap();
        let log=std::fs::File::create(dir.join("saved-guard-output.txt")).unwrap();
        let status=std::process::Command::new(repo.join("spikes/noita-entry/.build/Lua51Runner.exe"))
            .arg(r"C:\Noita\lua51.dll").arg(repo.join("apps/desktop/tools/g3-transport-test.lua"))
            .env("NT2_G3_TRANSPORT_DLL",std::env::var("NT2_G3_TRANSPORT_DLL").unwrap()).env("NT2_G3_TRANSPORT_DIR",&dir)
            .env("NT2_G3_SAVE_REFUSAL","1").env("LOCALAPPDATA",&local).env("APPDATA",&roaming)
            .stdout(log.try_clone().unwrap()).stderr(log).status().map_err(|_|"Saved guard fixture launch failed")?;
        assert!(status.success());assert!(dir.join("saved-refused").exists());
        Ok(())
    })();
    if result.is_err(){let _=child.kill();}let status=child.wait().unwrap();stop.store(true,Ordering::Relaxed);worker.join().unwrap();
    assert!(result.is_ok(),"{:?}; evidence {}",result,dir.display());assert!(status.success(),"native evidence {}",dir.display());
    std::fs::write(dir.join("result.json"),r#"{"passed":true,"actualNoita":false,"serviceFacts":"injected","nativeDll":"production separate uninstalled G3","checks":["SID scoped pipe","OS PID executable rejection","protected seed inspection","selected Start fixture","64 byte native pose decoded by Rust","864 byte Rust reply read by x86","expired lease and pause worker heartbeat","world close cleanup","production Continue guard with child-owned saved sentinel"]}"#).unwrap();
    println!("G3 transport evidence {}",dir.display());
}

#[test]fn protected_lobby_frame_is_bounded_fenced_and_read_only(){
 let fence=Arc::new(std::sync::atomic::AtomicU64::new(4));let ctx=[7;16];
 let mut d=LobbyDisplay{code:2,name:"\u{96ea} lobby".into(),until:Instant::now()+Duration::from_secs(1),generation:4,fence:fence.clone()};
 let frame=lobby_display_frame(Some(&d),&ctx);assert_eq!(frame.len(),256);assert_eq!(&frame[16..32],&ctx);assert_eq!(frame[8],2);assert_eq!(&frame[64..64+d.name.len()],d.name.as_bytes());
 fence.fetch_add(1,Ordering::AcqRel);assert_eq!(lobby_display_frame(Some(&d),&ctx)[8],0);
 d.generation=5;d.until=Instant::now()-Duration::from_secs(1);assert_eq!(lobby_display_frame(Some(&d),&ctx)[8],0);
 d.until=Instant::now()+Duration::from_secs(1);d.name="stale\nname".into();assert_eq!(lobby_display_frame(Some(&d),&ctx)[8],0);
 d.name="x".repeat(193);assert_eq!(lobby_display_frame(Some(&d),&ctx)[8],0);assert_eq!(lobby_display_frame(None,&ctx)[8],0);
}

#[test]fn protected_compatibility_diagnostic_cannot_outlive_context_or_lease(){
 let fence=Arc::new(std::sync::atomic::AtomicU64::new(9));let mut d=LobbyDisplay{code:5,name:"App/runtime versions differ; see app".into(),until:Instant::now()+Duration::from_secs(1),generation:9,fence:fence.clone()};
 let a=lobby_display_frame(Some(&d),&[1;16]);assert_eq!(a[8],5);assert_eq!(&a[16..32],&[1;16]);
 fence.store(10,Ordering::Release);assert_eq!(lobby_display_frame(Some(&d),&[2;16])[8],0);
 d.generation=10;d.until=Instant::now()-Duration::from_secs(1);assert_eq!(lobby_display_frame(Some(&d),&[2;16])[8],0);
 d.code=2;d.name="New lobby".into();d.until=Instant::now()+Duration::from_secs(1);assert_eq!(lobby_display_frame(Some(&d),&[2;16])[8],2);
 assert_eq!(lobby_display_frame(None,&[2;16])[8],0,"Terminal/Race no-lobby projection has no old diagnostic");
}

#[test]#[ignore]
fn protected_lobby_actual_local_websocket_native_lua_display_without_authority(){
 let native=std::path::PathBuf::from(std::env::var_os("NT2_PROTECTED_LOBBY_NATIVE").expect("owned private native output"));
 let repo=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..").canonicalize().unwrap();let own=native.join(format!("display-flow-{}",uuid::Uuid::new_v4()));std::fs::create_dir(&own).unwrap();
 let stop=Arc::new(AtomicBool::new(false));let pending=Arc::new(Mutex::new(None));let diagnostics=Arc::new(Mutex::new(NativeDiagnostics::default()));let fence=Arc::new(std::sync::atomic::AtomicU64::new(1));
 let game=own.join("game");let files=game.join("mods/noita_together2/files");std::fs::create_dir_all(&files).unwrap();
 for name in ["nt2_g3_ipc.dll","nt2_g3_transport.dll","nt2_g3_social.dll"]{std::fs::copy(native.join(name),files.join(name)).unwrap();}
 let dispatcher=Dispatcher{pending:pending.clone(),diagnostics:Some(diagnostics.clone()),g3:None,social:None,startup:None};
 let(control,worker)=crate::local_bridge::fixture(dispatcher,Arc::new(Mutex::new(Some(game.clone()))),stop.clone()).unwrap();control.select_game(game).unwrap();
 let deadline=Instant::now()+Duration::from_secs(2);while control.view()["setup"]["ready"]!=true{assert!(Instant::now()<deadline,"private bootstrap");thread::sleep(Duration::from_millis(5));}
 let script=own.join("child.lua");std::fs::write(&script,r#"
local ffi=require('ffi');ffi.cdef[[void Sleep(unsigned long);int nt2_g3_lobby_status(uint8_t*,uint32_t);]]
local dll=ffi.load(assert(os.getenv('NT2_PROTECTED_LOBBY_NATIVE'))..'/nt2_g3_transport.dll')
local repo=assert(os.getenv('NT2_ITEM_REPO'));local display=dofile(repo..'/mod/noita_together2/files/rejoin/lobby_status.lua')
local root=assert(os.getenv('NT2_PROTECTED_LOBBY_FLOW'));local bridge={nt2_g3_lobby_status=dll.nt2_g3_lobby_status}
for step=1,10 do
 local found=false
 for i=1,1000 do
  local v=display.read(bridge,ffi)
  local expected=step<=5 and step or step==7 and 2 or step==9 and 4 or 0
  if (expected>0 and v and v.code==expected and v.name=='Verified fixture')or(expected==0 and not v)then
   local f=assert(io.open(root..'/step-'..step,'w'));f:write(display.message(v));f:close();found=true;break
  end
  ffi.C.Sleep(5)
 end
 assert(found,'snapshot step '..step)
 local done=false;for i=1,1000 do local f=io.open(root..'/ack-'..step,'r');if f then f:close();done=true;break end;ffi.C.Sleep(5)end;assert(done,'parent ack')
end
print('PASS actual app pipe / native worker / Lua membership messages; no world prepared')
"#).unwrap();
 use std::os::windows::process::CommandExt;
 let child=std::process::Command::new(repo.join("spikes/noita-entry/.build/Lua51Runner.exe")).creation_flags(0x08000000).arg("C:/Noita/lua51.dll").arg(&script).env("NT2_PROTECTED_LOBBY_NATIVE",&files).env("NT2_PROTECTED_LOBBY_FLOW",&own).env("NT2_ITEM_REPO",repo.to_string_lossy().trim_start_matches(r"\\?\").replace('\\',"/")).stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::piped()).spawn().unwrap();
 let result=(||->Result<(),String>{for step in 1..=10{
  let code=match step{1..=6=>step.min(5),7=>2,8=>5,9=>4,_=>0};
  diagnostics.lock().unwrap().lobby_display=(code>0).then(||LobbyDisplay{code,name:"Verified fixture".into(),until:if step==6{Instant::now()-Duration::from_secs(1)}else{Instant::now()+Duration::from_secs(2)},generation:if step==8{0}else{1},fence:fence.clone()});
  let until=Instant::now()+Duration::from_secs(4);while !own.join(format!("step-{step}")).exists(){if Instant::now()>=until{return Err(format!("missing native/Lua step{step}"))}thread::sleep(Duration::from_millis(5));}
  let text=std::fs::read_to_string(own.join(format!("step-{step}"))).unwrap();assert!(!text.contains("replacement world"));
  std::fs::write(own.join(format!("ack-{step}")),b"continue").unwrap();
 }Ok(())})();
 stop.store(true,Ordering::Relaxed);worker.join().unwrap();let output=child.wait_with_output().unwrap();std::fs::write(own.join("stdout.txt"),&output.stdout).unwrap();std::fs::write(own.join("stderr.txt"),&output.stderr).unwrap();assert!(result.is_ok(),"{result:?}");assert!(output.status.success(),"{}",String::from_utf8_lossy(&output.stderr));assert!(pending.lock().unwrap().is_none(),"display cannot prepare a world");
 println!("PROTECTED_LOBBY_WS_NATIVE_LUA {}",own.display());
}
