// Windows G0 only. Message size fixed at 32; user DACL; reject remote clients.
use std::{
    ffi::c_void,
    ptr::null_mut,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant},
};
#[path="rejoin_pipe.rs"]pub(crate) mod rejoin;
#[path="startup_transaction.rs"]pub(crate) mod startup;
#[path="entry_bridge.rs"]pub(crate) mod entry;
#[path="world_bridge.rs"]pub(crate) mod world;
type Handle = *mut c_void;
/// Presentation only, fenced by the existing desktop selection generation.
pub(crate) struct LobbyDisplay {
    pub code:u32,pub name:String,pub until:Instant,
    pub generation:u64,pub fence:Arc<std::sync::atomic::AtomicU64>,
}
fn lobby_display_frame(display:Option<&LobbyDisplay>,context:&[u8])->Vec<u8>{
    let mut out=vec![0;256];out[..4].copy_from_slice(b"NLD1");out[4..8].copy_from_slice(&1u32.to_le_bytes());out[16..32].copy_from_slice(context);
    if let Some(d)=display.filter(|d|d.fence.load(Ordering::Acquire)==d.generation&&Instant::now()<d.until&&(1..=5).contains(&d.code)&&!d.name.is_empty()&&d.name.len()<=192&&!d.name.chars().any(char::is_control)){
        let remaining=d.until.saturating_duration_since(Instant::now()).as_millis().min(2000)as u32;
        if remaining>0{out[8..12].copy_from_slice(&d.code.to_le_bytes());out[12..16].copy_from_slice(&remaining.to_le_bytes());out[32..36].copy_from_slice(&(d.name.len()as u32).to_le_bytes());out[64..64+d.name.len()].copy_from_slice(d.name.as_bytes());}
    }out
}
pub struct NativeDiagnostics {
    pub(crate) local_ws:Option<Arc<crate::local_bridge::Control>>,
    #[cfg(test)]pub(crate) local_drop_bank_reply_once:bool,
    #[cfg(test)]pub(crate) local_dropped_bank_replies:u64,
    pub(crate) lobby_display:Option<LobbyDisplay>,
    pub(crate) world:Option<Arc<world::Bridge>>,
    pub(crate) world_fence:Option<Arc<std::sync::atomic::AtomicU64>>,
    pub(crate) provisional:Option<Arc<entry::Bridge>>,
    pub(crate) rejoin:rejoin::Mailbox,
    continue_detected: bool,
    status: String,
    contacts: u64,
    last_contact: Option<Instant>,
    #[cfg(test)]
    pub(crate) delay_next_heartbeat: bool,
    #[cfg(test)]
    pub(crate) deny_next_heartbeat: bool,
    #[cfg(test)]
    pub(crate) malform_next_heartbeat: bool,
    #[cfg(test)]
    pub(crate) initial_contact_delay: Option<(u32, u64)>,
    #[cfg(test)]
    pub(crate) initial_contact_busy: Option<(u32, u64)>,
    #[cfg(test)]
    pub(crate) initial_contact_invalid_reply: bool,
    #[cfg(test)]
    pub(crate) initial_contact_trace: Vec<serde_json::Value>,
    #[cfg(test)] pub(crate) startup_claim_delay_ms:u64,
    #[cfg(test)] pub(crate) startup_drop_ready_once:bool,
    #[cfg(test)] pub(crate) lc1_collection_trace:Vec<serde_json::Value>,
    #[cfg(test)] pub(crate) lc1_collection_recent:Vec<serde_json::Value>,
    #[cfg(test)] pub(crate) lc1_retirement_trace:Vec<serde_json::Value>,
}
impl Default for NativeDiagnostics {
    fn default() -> Self {
        Self {
            local_ws:None,
            #[cfg(test)]local_drop_bank_reply_once:false,
            #[cfg(test)]local_dropped_bank_replies:0,
            lobby_display:None,
            world:None,
            world_fence:None,
            provisional:None,
            rejoin:Default::default(),
            continue_detected: false,
            status: "No game contact yet; Load game before normal New Game".into(),
            contacts: 0,
            last_contact: None,
            #[cfg(test)]
            delay_next_heartbeat: false,
            #[cfg(test)]
            deny_next_heartbeat: false,
            #[cfg(test)]
            malform_next_heartbeat: false,
            #[cfg(test)]
            initial_contact_delay: None,
            #[cfg(test)]
            initial_contact_busy: None,
            #[cfg(test)]
            initial_contact_invalid_reply: false,
            #[cfg(test)]
            initial_contact_trace: Vec::new(),
            #[cfg(test)] startup_claim_delay_ms:0,
            #[cfg(test)] startup_drop_ready_once:false,
            #[cfg(test)] lc1_collection_trace:Vec::new(),
            #[cfg(test)] lc1_collection_recent:Vec::new(),
            #[cfg(test)] lc1_retirement_trace:Vec::new(),
        }
    }
}
pub(crate) fn retire_lc1_bridge(diagnostics:&Arc<Mutex<NativeDiagnostics>>){let(fence,bridge)={let d=diagnostics.lock().unwrap();(d.world_fence.clone(),d.world.clone())};if let Some(fence)=fence{fence.fetch_add(1,Ordering::AcqRel);}if let Some(bridge)=bridge{bridge.lane.retire();}}
impl NativeDiagnostics {
    pub fn continue_detected(&self)->bool{self.continue_detected}
    pub fn clear_continue_notice(&mut self){self.continue_detected=false;}
    #[cfg(test)] pub fn fixture_continue(&mut self){self.continue_detected=true;}
    fn note(&mut self, status: &str) {
        self.status = status.into();
        self.last_contact = Some(Instant::now());
    }
    pub fn view(&self) -> serde_json::Value {
        let early=self.world.as_ref().map(|b|b.pending.try_lock().map(|p|collection_view(&p)).unwrap_or_else(|_|serde_json::json!({"busy":true})));
        let mut view=serde_json::json!({"status":self.status,"contacts":self.contacts,"last_contact_seconds":self.last_contact.map(|t|t.elapsed().as_secs()),"early_world":early});
        view["local_bridge"]=self.local_ws.as_ref().map(|bridge|bridge.view()).unwrap_or(serde_json::Value::Null);
        #[cfg(test)]{view["lc1_collection_trace"]=serde_json::json!(self.lc1_collection_trace);view["lc1_collection_recent"]=serde_json::json!(self.lc1_collection_recent);view["lc1_retirement_trace"]=serde_json::json!(self.lc1_retirement_trace);}view
    }
}
pub(crate) fn collection_view(slot:&Option<crate::handoff::Pending>)->serde_json::Value{let Some(p)=slot else{return serde_json::json!({"present":false})};serde_json::json!({"present":true,"owner_pid":p.owner.map(|o|o.0),"accepted":p.accepted,"loaded":p.loaded,"release":p.release,"report_sequence":p.observation.as_ref().map(|r|r.sequence),"generation_expected":p.generation.as_ref().map(|(_,n)|*n),"generation_current":p.generation.as_ref().map(|(g,_)|g.load(Ordering::Acquire)),"native_epoch":p.native_epoch.or_else(||p.generation.as_ref().and_then(|(_,n)|(*n).try_into().ok())),"lease_remaining_ms":p.authority_until.saturating_duration_since(Instant::now()).as_millis().min(12000),"collection_remaining_ms":p.collection_until.map(|at|at.saturating_duration_since(Instant::now()).as_millis().min(12000)),"unexpired":Instant::now()<p.expires,"startup_ready":p.startup_ready,"startup_claimed":p.startup_transaction.is_some()})}
#[cfg(test)]
fn record_initial_contact(pending:&Mutex<Option<crate::handoff::Pending>>,diagnostics:&Mutex<NativeDiagnostics>,op:u32,acknowledged:bool,before_diagnostics:impl FnOnce())->Option<u64>{
    // Complete the owned facts before entering the independent diagnostic lock.
    // Core/status readers must never contend with a nested broker trace lock.
    let trace={let p=pending.lock().unwrap();serde_json::json!({
        "op":op,"trustedProcess":true,"transitionAcknowledged":acknowledged,
        "ownerClaimed":p.as_ref().is_some_and(|p|p.owner.is_some()),
        "durableClaimExists":p.as_ref().and_then(|p|p.claim_path.as_ref()).is_some_and(|p|p.exists()),
        "expired":p.as_ref().is_some_and(|p|Instant::now()>=p.expires),
        "generationCurrent":p.as_ref().is_some_and(|p|p.generation.as_ref().is_none_or(|(g,n)|g.load(Ordering::Acquire)==*n))
    })};
    before_diagnostics();
    let mut d=diagnostics.lock().unwrap();d.initial_contact_trace.push(trace);
    if d.initial_contact_delay.is_some_and(|(target,_)|target==op){d.initial_contact_delay.take().map(|(_,ms)|ms)}else{None}
}
// Each startup conversation chooses one source on its first offer. Passive B
// never intercepts canonical A's bootstrap, claim, poll or scoped close.
pub(crate) fn dispatch_startup(lane:&startup::Lane,provisional:Option<&Arc<entry::Bridge>>,pid:crate::local_owner::Id,start:u64,request:&[u8],delay:Duration)->Vec<u8>{
    if lane.bound(pid,start,request){return lane.handle_instance(pid,start,request,delay)}
    if let Some(b)=provisional{if b.bound(pid,start,request)||request.len()==32&&request[8..12]==19u32.to_le_bytes(){return b.handle(pid,start,request)}}
    if provisional.is_none()&&request.len()==32&&request[8..12]==19u32.to_le_bytes(){return lane.handle_instance(pid,start,request,delay)}
    startup::failure(request,startup::OWNER)
}
pub(crate) fn entry_collection(diagnostics:&Arc<Mutex<NativeDiagnostics>>,contact:&rejoin::Contact)->Option<crate::handoff::Pending>{
    let bridge=diagnostics.lock().unwrap().provisional.clone();bridge.and_then(|b|b.collection(contact))
}
pub(crate) fn dispatch_rejoin(diagnostics:&Arc<Mutex<NativeDiagnostics>>,pending:&Arc<Mutex<Option<crate::handoff::Pending>>>,pid:crate::local_owner::Id,request:&[u8])->Vec<u8>{
    // Never acquire diagnostics while holding a pending-slot mutex (or vice
    // versa). The snapshot retains the live atomic lifecycle fence.
    let current=pending.lock().unwrap().clone();diagnostics.lock().unwrap().rejoin.transition(&current,pid,request)
}
// Called only after candidate artifact verification. Normalize valid Windows aliases
// (forward slashes/dot segments/extended prefix) before comparing the OS image path.
pub fn verified_process_path(file: &std::path::Path) -> Result<std::path::PathBuf, String> {
    let canonical =
        std::fs::canonicalize(file).map_err(|_| "Verified executable path unavailable")?;
    let text = canonical.to_str().ok_or("Unsupported executable path")?;
    let text = text.strip_prefix(r"\\?\").unwrap_or(text);
    let bytes = text.as_bytes();
    if bytes.len() < 3 || !bytes[0].is_ascii_alphabetic() || bytes[1] != b':' || bytes[2] != b'\\' {
        return Err("Use a local executable path".into());
    }
    Ok(std::path::PathBuf::from(text))
}
pub(crate) fn matches_local_owner(owner:crate::local_owner::Id,expected:&std::path::Path)->bool{
 if matches!(owner,crate::local_owner::Id::Socket{..}){return crate::local_owner::matches_installation(owner,expected)}
 let crate::local_owner::Id::OsProcess(pid)=owner else{return false};
 let Ok(expected)=verified_process_path(expected)else{return false};
 unsafe{let h=OpenProcess(0x1000,0,pid);if h.is_null(){return false}let h=Owned(h);let mut image=[0u16;1024];let mut count=1024;QueryFullProcessImageNameW(h.0,0,image.as_mut_ptr(),&mut count)!=0&&String::from_utf16_lossy(&image[..count as usize]).eq_ignore_ascii_case(&expected.to_string_lossy())}
}
// A retained process handle keeps an exited process queryable on Windows.
// Creation time alone proves identity, not a live native owner.
pub(crate) fn owner_instance(owner:crate::local_owner::Id)->Option<u64>{if matches!(owner,crate::local_owner::Id::Socket{..}){return crate::local_owner::instance(owner)}let crate::local_owner::Id::OsProcess(pid)=owner else{return None};unsafe{let h=OpenProcess(0x00101000,0,pid);if h.is_null(){return None}let h=Owned(h);if WaitForSingleObject(h.0,0)!=258{return None}let(mut c,mut e,mut k,mut u)=(0u64,0u64,0u64,0u64);(GetProcessTimes(h.0,&mut c,&mut e,&mut k,&mut u)!=0&&c!=0&&WaitForSingleObject(h.0,0)==258).then_some(c)}}
#[repr(C)]
struct Security {
    len: u32,
    descriptor: Handle,
    inherit: i32,
}
#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetCurrentProcess() -> Handle;
    fn GetProcessTimes(h:Handle,c:*mut u64,e:*mut u64,k:*mut u64,u:*mut u64)->i32;
    fn WaitForSingleObject(h:Handle,milliseconds:u32)->u32;
    fn CloseHandle(h: Handle) -> i32;
    fn LocalFree(h: Handle) -> Handle;
    fn CreateNamedPipeW(
        n: *const u16,
        a: u32,
        m: u32,
        i: u32,
        o: u32,
        r: u32,
        t: u32,
        s: *const Security,
    ) -> Handle;
    fn ConnectNamedPipe(h: Handle, o: Handle) -> i32;
    fn DisconnectNamedPipe(h: Handle) -> i32;
    fn GetNamedPipeClientProcessId(h: Handle, p: *mut u32) -> i32;
    fn ReadFile(h: Handle, b: *mut c_void, n: u32, r: *mut u32, o: Handle) -> i32;
    fn WriteFile(h: Handle, b: *const c_void, n: u32, r: *mut u32, o: Handle) -> i32;
    fn PeekNamedPipe(h: Handle, b: Handle, n: u32, r: *mut u32, a: *mut u32, l: *mut u32) -> i32;
    fn GetLastError() -> u32;
    fn OpenProcess(a: u32, i: i32, p: u32) -> Handle;
    fn QueryFullProcessImageNameW(h: Handle, f: u32, b: *mut u16, n: *mut u32) -> i32;
}
#[link(name = "advapi32")]
unsafe extern "system" {
    fn OpenProcessToken(p: Handle, a: u32, t: *mut Handle) -> i32;
    fn GetTokenInformation(t: Handle, c: u32, b: Handle, n: u32, r: *mut u32) -> i32;
    fn ConvertSidToStringSidW(s: Handle, out: *mut *mut u16) -> i32;
    fn ConvertStringSecurityDescriptorToSecurityDescriptorW(
        s: *const u16,
        r: u32,
        out: *mut Handle,
        n: *mut u32,
    ) -> i32;
}
fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}
pub fn user_sid() -> Result<String, String> {
    unsafe {
        let mut token = null_mut();
        if OpenProcessToken(GetCurrentProcess(), 8, &mut token) == 0 {
            return Err("User token unavailable".into());
        }
        let mut n = 0;
        GetTokenInformation(token, 1, null_mut(), 0, &mut n);
        let mut buffer = vec![0usize; (n as usize).div_ceil(std::mem::size_of::<usize>())];
        let ok = GetTokenInformation(token, 1, buffer.as_mut_ptr().cast(), n, &mut n);
        CloseHandle(token);
        if ok == 0 {
            return Err("User identity unavailable".into());
        }
        let sid = *(buffer.as_ptr().cast::<Handle>());
        let mut text = null_mut();
        if ConvertSidToStringSidW(sid, &mut text) == 0 {
            return Err("SID unavailable".into());
        }
        let mut len = 0;
        while *text.add(len) != 0 {
            len += 1;
        }
        let result = String::from_utf16_lossy(std::slice::from_raw_parts(text, len));
        LocalFree(text.cast());
        Ok(result)
    }
}
#[cfg(test)]
#[allow(dead_code)]
pub fn path(sid: &str) -> String {
    format!(r"\\.\pipe\nt2-g0-v1-{sid}")
}
struct Owned(Handle);
impl Drop for Owned {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.0);
        }
    }
}
#[cfg(test)]
fn is_game(owner:crate::local_owner::Id) -> bool {
    let crate::local_owner::Id::OsProcess(pid)=owner else{return false};
    unsafe {
        let h = OpenProcess(0x1000, 0, pid);
        if h.is_null() {
            return false;
        }
        let h = Owned(h);
        let mut b = [0u16; 1024];
        let mut n = 1024;
        if QueryFullProcessImageNameW(h.0, 0, b.as_mut_ptr(), &mut n) == 0 {
            return false;
        }
        String::from_utf16_lossy(&b[..n as usize])
            .eq_ignore_ascii_case(r"C:\Noita\noita.exe")
    }
}
#[cfg(test)]
#[allow(dead_code)]
pub fn start(
    pending: Arc<Mutex<Option<super::handoff::Pending>>>,
    stop: Arc<AtomicBool>,
) -> Result<thread::JoinHandle<()>, String> {
    start_inner(path(&user_sid()?), pending, stop, is_game)
}
pub fn start_g2(
    pending: Arc<Mutex<Option<super::handoff::Pending>>>,
    stop: Arc<AtomicBool>,
    expected: Arc<Mutex<Option<std::path::PathBuf>>>,
) -> Result<thread::JoinHandle<()>, String> {
    start_inner(
        format!(r"\\.\pipe\nt2-g2-v1-{}", user_sid()?),
        pending,
        stop,
        move |pid| {
            let Some(expected) = expected.lock().unwrap().clone() else {
                return false;
            };
            unsafe {
                let crate::local_owner::Id::OsProcess(pid)=pid else{return false};
                let h = OpenProcess(0x1000, 0, pid);
                if h.is_null() {
                    return false;
                }
                let h = Owned(h);
                let mut b = [0u16; 1024];
                let mut n = 1024;
                if QueryFullProcessImageNameW(h.0, 0, b.as_mut_ptr(), &mut n) == 0 {
                    return false;
                }
                String::from_utf16_lossy(&b[..n as usize])
                    .eq_ignore_ascii_case(&expected.to_string_lossy())
            }
        },
    )
}
fn start_inner<F: Fn(crate::local_owner::Id) -> bool + Send + 'static>(
    name: String,
    pending: Arc<Mutex<Option<super::handoff::Pending>>>,
    stop: Arc<AtomicBool>,
    trust: F,
) -> Result<thread::JoinHandle<()>, String> {
    start_inner_mode(name, pending, stop, trust, false)
}
#[allow(dead_code)] // Activated only by the successor client, not the current LAN client.
pub fn start_inspection(
    pending: Arc<Mutex<Option<super::handoff::Pending>>>,
    stop: Arc<AtomicBool>,
    expected: Arc<Mutex<Option<std::path::PathBuf>>>,
    diagnostics: Arc<Mutex<NativeDiagnostics>>,
) -> Result<thread::JoinHandle<()>, String> {
    start_inspection_named(
        format!(r"\\.\pipe\nt2-g2-inspect-v1-{}", user_sid()?),
        pending,
        stop,
        expected,
        diagnostics,
    )
}
fn start_inspection_named(
    name: String,
    pending: Arc<Mutex<Option<super::handoff::Pending>>>,
    stop: Arc<AtomicBool>,
    expected: Arc<Mutex<Option<std::path::PathBuf>>>,
    diagnostics: Arc<Mutex<NativeDiagnostics>>,
) -> Result<thread::JoinHandle<()>, String> {
    let trust_diagnostics = diagnostics.clone();
    start_inner_diagnostics(
        name,
        pending,
        stop,
        move |pid| {
            let Some(expected) = expected.lock().unwrap().clone() else {
                trust_diagnostics.lock().unwrap().note("Game contacted app but installation is not verified; verify the Noita folder before Load game");
                return false;
            };
            let trusted = matches_local_owner(pid, &expected);
            if !trusted {
                trust_diagnostics.lock().unwrap().note("Game connection refused: executable differs from the verified Noita installation");
            }
            trusted
        },
        true,
        Some(diagnostics),
        None,
        None,
    )
}
/// Separately named uninstalled G3 candidate pipe; the accepted inspection pipe is unchanged.
pub fn start_g3(pending:Arc<Mutex<Option<super::handoff::Pending>>>,stop:Arc<AtomicBool>,
    expected:Arc<Mutex<Option<std::path::PathBuf>>>,diagnostics:Arc<Mutex<NativeDiagnostics>>,
    exchange:Arc<Mutex<nt2_desktop_g0::g3_runtime::Exchange>>)->Result<thread::JoinHandle<()>,String>{
    start_g3_named(format!(r"\\.\pipe\nt2-g3-dev-v1-{}",user_sid()?),pending,stop,expected,diagnostics,exchange)
}
pub(crate) fn start_g3_named(name:String,pending:Arc<Mutex<Option<super::handoff::Pending>>>,stop:Arc<AtomicBool>,
    expected:Arc<Mutex<Option<std::path::PathBuf>>>,diagnostics:Arc<Mutex<NativeDiagnostics>>,
    exchange:Arc<Mutex<nt2_desktop_g0::g3_runtime::Exchange>>)->Result<thread::JoinHandle<()>,String>{
    start_inner_diagnostics(name,pending,stop,move|pid|expected.lock().unwrap().as_ref()
        .is_some_and(|path|matches_local_owner(pid,path)),true,Some(diagnostics),Some(exchange),None)
}
pub(crate) fn start_g3_social_named(name:String,pending:Arc<Mutex<Option<super::handoff::Pending>>>,stop:Arc<AtomicBool>,
 expected:Arc<Mutex<Option<std::path::PathBuf>>>,diagnostics:Arc<Mutex<NativeDiagnostics>>,exchange:Arc<Mutex<nt2_desktop_g0::g3_runtime::Exchange>>,
 social:Arc<Mutex<nt2_desktop_g0::g3_social_stream::Exchange>>)->Result<thread::JoinHandle<()>,String>{
 start_inner_diagnostics(name,pending,stop,move|pid|expected.lock().unwrap().as_ref().is_some_and(|path|matches_local_owner(pid,path)),true,Some(diagnostics),Some(exchange),Some(social))
}
fn g3_playing(p:&Option<super::handoff::Pending>,pid:crate::local_owner::Id,context:&[u8;16])->bool{
    p.as_ref().is_some_and(|p|p.native_v2&&p.owner==Some((pid,*context))&&p.accepted&&p.loaded&&p.release
        &&Instant::now()<p.authority_until&&p.last_seen.is_some_and(|at|at.elapsed()<Duration::from_secs(12))
        &&p.generation.as_ref().is_none_or(|(g,n)|g.load(Ordering::Acquire)==*n)
        &&p.observation.as_ref().is_some_and(|r|r.context==*context&&!r.observation.protected&&r.observation.complete
            &&r.observation.loaded&&p.matches_seed(r.observation.actual_seed.as_deref())))
}
// op13 keeps its1072-byte identity frame and unchanged JSON at offset48.
// Formerly reserved32..48: display version/code/remaining-ms/native-epoch.
fn metadata_display(p:&Option<super::handoff::Pending>,pid:crate::local_owner::Id,context:&[u8;16],reply:&mut[u8]){
 if reply.len()!=1072{return}reply[32..48].fill(0);
 if reply[8..12]!=1u32.to_le_bytes()||reply[16..32]!=*context{return}
 let Some(p)=p.as_ref().filter(|p|p.native_v2&&p.owner==Some((pid,*context))&&p.last_seen.is_some_and(|at|at.elapsed()<Duration::from_secs(12)))else{return};
 let Some((fence,generation))=p.generation.as_ref().filter(|(g,n)|g.load(Ordering::Acquire)==*n)else{return};let _=fence;
 let Some(display)=p.display.as_ref().filter(|d|d.owner==(pid,*context)&&d.generation==*generation&&(1..=5).contains(&d.code))else{return};
 let Some(epoch)=p.native_epoch.filter(|n|*n>0)else{return};let mut until=display.until;
 if matches!(display.code,2|4){let Some(report)=p.observation.as_ref().filter(|r|r.context==*context&&r.observation.complete&&r.observation.loaded&&r.observation.error.is_none()&&p.matches_seed(r.observation.actual_seed.as_deref()))else{return};until=until.min(p.authority_until).min(report.observed_at+Duration::from_secs(6));}
 let remaining=until.saturating_duration_since(Instant::now()).as_millis().min(12000)as u32;if remaining==0{return}
 for(offset,value)in[(32,1u32),(36,display.code),(40,remaining),(44,epoch)]{reply[offset..offset+4].copy_from_slice(&value.to_le_bytes());}
}
pub(crate) fn report_transition(
    pending: &mut Option<super::handoff::Pending>,
    pid:crate::local_owner::Id,
    request: &[u8],
) -> [u8; 32] {
    let mut reply = [0u8; 32];
    reply[..4].copy_from_slice(&super::handoff::MAGIC.to_le_bytes());
    reply[4..8].copy_from_slice(&3u32.to_le_bytes());
    let Ok(frame) = crate::native_report::decode(request) else {
        return reply;
    };
    let Some(p) = pending else {
        return reply;
    };
    if !p.native_v2
        || p.owner != Some((pid, frame.context))
        || p.generation
            .as_ref()
            .is_some_and(|(g, n)| g.load(Ordering::Acquire) != *n)
        || (!p.loaded && Instant::now() >= p.expires)
    {
        return reply;
    }
    let context = frame.context;
    let old_digest = p.observation.as_ref().map(|r| r.digest);
    if crate::native_report::Latest::update(&mut p.observation, context, frame).is_err() {
        return reply;
    }
    let report = p.observation.as_ref().unwrap();
    if old_digest.is_some_and(|digest| digest != report.digest)
        || !report.observation.complete
        || !report.observation.loaded
        || !p.matches_seed(report.observation.actual_seed.as_deref())
    {
        p.release = false;
        p.invalidated_observation = report.sequence;
    }
    reply[8..12].copy_from_slice(&1u32.to_le_bytes());
    let state = if p.release && p.accepted && p.loaded && Instant::now() < p.authority_until {
        2u32
    } else {
        1u32
    };
    reply[12..16].copy_from_slice(&state.to_le_bytes());
    reply[16..32].copy_from_slice(&context);
    reply
}

// C2 local IPC only. The existing strict transitions still own consumption and
// report validation; this envelope adds the remaining original permission lease
// and exact cancellation/report identity. It never creates authority.
fn c2_transition(pending:&mut Option<super::handoff::Pending>,pid:crate::local_owner::Id,request:&[u8])->Vec<u8>{
    if request.len()<32||request.len()>crate::native_report::HEADER_BYTES+crate::native_report::REPORT_BYTES{return vec![0;48];}
    let version=u32::from_le_bytes(request[4..8].try_into().unwrap());
    let mut old=request.to_vec();let mut reply=if version==4&&request.len()==32{
        old[4..8].copy_from_slice(&2u32.to_le_bytes());super::handoff::transition(pending,pid,old[..32].try_into().unwrap()).to_vec()
    }else if version==6&&request.len()==40&&request[8..12]==11u32.to_le_bytes()&&request[36..40]==[0;4]{
        let actual=u64::from(u32::from_le_bytes(request[12..16].try_into().unwrap()))|(u64::from(u32::from_le_bytes(request[32..36].try_into().unwrap()))<<32);
        super::handoff::known_seed(pending,pid,request[16..32].try_into().unwrap(),actual).to_vec()
    }else if version==5&&request.len()>=crate::native_report::HEADER_BYTES{
        old[4..8].copy_from_slice(&3u32.to_le_bytes());report_transition(pending,pid,&old).to_vec()
    }else{return vec![0;48]};
    reply.resize(48,0);reply[4..8].copy_from_slice(&version.to_le_bytes());
    if version==5{reply[40..48].copy_from_slice(&request[16..24]);}
    c2_fence(pending,pid,request,&mut reply);reply
}
fn c2_fence(pending:&Option<super::handoff::Pending>,pid:crate::local_owner::Id,request:&[u8],reply:&mut [u8]){
    let op=u32::from_le_bytes(request[8..12].try_into().unwrap());let version=u32::from_le_bytes(request[4..8].try_into().unwrap());
    let context:&[u8]=if version==5{&request[24..40]}else{&request[16..32]};
    if op==6{reply[32..40].fill(0);return;}
    let valid=pending.as_ref().filter(|p|(p.loaded||Instant::now()<p.expires)&&p.owner.is_some_and(|(owner,c)|owner==pid&&c==context)&&p.generation.as_ref().is_some_and(|(g,n)|g.load(Ordering::Acquire)==*n));
    if let Some(p)=valid{
        let now=Instant::now();let collection=p.collection_until.unwrap_or(p.authority_until);let mut until=if p.loaded{collection}else{collection.min(p.expires)};
        // State2 carries gameplay permission. Its IPC lease must never outlive
        // the original canonical authority even if local collection stays live.
        if matches!(op,4|5|7)&&u32::from_le_bytes(reply[12..16].try_into().unwrap())==2{
            if p.release&&p.accepted&&p.loaded&&p.authority_until.saturating_duration_since(now).as_millis()>0{until=until.min(p.authority_until);}else{reply[12..16].copy_from_slice(&1u32.to_le_bytes());}
        }
        let remaining=until.saturating_duration_since(now).as_millis().min(12000) as u32;
        let generation=p.native_epoch.unwrap_or_else(||u32::try_from(p.generation.as_ref().unwrap().1).unwrap_or(0));
        if remaining>0&&generation>0{reply[32..36].copy_from_slice(&remaining.to_le_bytes());reply[36..40].copy_from_slice(&generation.to_le_bytes());return;}
    }
    reply[8..16].fill(0);reply[32..40].fill(0);
}
// Recheck the source which actually handled collection. Promotion may move the
// same conversation into common, but an empty common slot cannot retire a valid
// early-world reply. A reply initially handled by common never falls back.
fn c2_delivery_fence(common:&Arc<Mutex<Option<super::handoff::Pending>>>,early:Option<&Arc<world::Bridge>>,pid:crate::local_owner::Id,request:&[u8],reply:&mut [u8]){
 let version=u32::from_le_bytes(request[4..8].try_into().unwrap());let context=if version==5{&request[24..40]}else{&request[16..32]};let current=common.lock().unwrap();
 if early.is_none()||current.as_ref().is_some_and(|p|p.owner.is_some_and(|o|o.0==pid&&o.1.as_slice()==context)){c2_fence(&current,pid,request,reply);return}drop(current);c2_fence(&early.unwrap().pending.lock().unwrap(),pid,request,reply);
}
fn start_inner_mode<F: Fn(crate::local_owner::Id) -> bool + Send + 'static>(
    name: String,
    pending: Arc<Mutex<Option<super::handoff::Pending>>>,
    stop: Arc<AtomicBool>,
    trust: F,
    inspection: bool,
) -> Result<thread::JoinHandle<()>, String> {
    start_inner_diagnostics(name, pending, stop, trust, inspection, None, None, None)
}
fn start_inner_diagnostics<F: Fn(crate::local_owner::Id) -> bool + Send + 'static>(
    name:String,pending:Arc<Mutex<Option<super::handoff::Pending>>>,stop:Arc<AtomicBool>,trust:F,inspection:bool,
    diagnostics:Option<Arc<Mutex<NativeDiagnostics>>>,g3:Option<Arc<Mutex<nt2_desktop_g0::g3_runtime::Exchange>>>,social:Option<Arc<Mutex<nt2_desktop_g0::g3_social_stream::Exchange>>>,
)->Result<thread::JoinHandle<()>,String>{start_inner_diagnostics_routed(name,pending,stop,trust,inspection,diagnostics,g3,social,#[cfg(test)]None)}

#[cfg(test)]
#[derive(Clone)]pub(crate) struct FixtureParticipant{
 pub pending:Arc<Mutex<Option<super::handoff::Pending>>>,pub diagnostics:Arc<Mutex<NativeDiagnostics>>,
 pub gameplay:Arc<Mutex<nt2_desktop_g0::g3_runtime::Exchange>>,pub social:Arc<Mutex<nt2_desktop_g0::g3_social_stream::Exchange>>,
 pub(crate) startup:Arc<startup::Lane>,
}
#[cfg(test)]impl FixtureParticipant{pub fn new(pending:Arc<Mutex<Option<super::handoff::Pending>>>,diagnostics:Arc<Mutex<NativeDiagnostics>>,gameplay:Arc<Mutex<nt2_desktop_g0::g3_runtime::Exchange>>,social:Arc<Mutex<nt2_desktop_g0::g3_social_stream::Exchange>>)->Self{Self{startup:Arc::new(startup::Lane::new(pending.clone())),pending,diagnostics,gameplay,social}}}
#[cfg(test)]#[derive(Default)]pub(crate) struct FixtureRoutes{entries:Mutex<Vec<(crate::local_owner::Id,u64,FixtureParticipant)>>}
#[cfg(test)]impl FixtureRoutes{
 pub fn register(&self,pid:crate::local_owner::Id,participant:FixtureParticipant)->Result<u64,String>{let birth=owner_instance(pid).ok_or("Fixture process instance unavailable")?;let mut entries=self.entries.lock().unwrap();if entries.len()>=2||entries.iter().any(|e|e.0==pid){return Err("Fixture route bound/duplicate".into())}entries.push((pid,birth,participant));Ok(birth)}
 pub fn remove(&self,pid:crate::local_owner::Id,birth:u64){self.entries.lock().unwrap().retain(|e|e.0!=pid||e.1!=birth);}
 fn select(&self,pid:crate::local_owner::Id,birth:Option<u64>)->Option<FixtureParticipant>{let entries=self.entries.lock().unwrap();entries.iter().find(|e|e.0==pid&&Some(e.1)==birth).map(|e|e.2.clone())}
}
#[cfg(test)]#[test]fn lc1_fixture_route_rejects_unknown_removed_and_reused_owner_instance(){
 let routes=FixtureRoutes::default();let pid=std::process::id();let p=FixtureParticipant::new(Arc::new(Mutex::new(None)),Arc::new(Mutex::new(Default::default())),Arc::new(Mutex::new(Default::default())),Arc::new(Mutex::new(Default::default())));assert!(routes.select(nt2_local_ws::owner::Id::OsProcess(pid),owner_instance(nt2_local_ws::owner::Id::OsProcess(pid))).is_none());let birth=routes.register(nt2_local_ws::owner::Id::OsProcess(pid),p.clone()).unwrap();assert!(routes.select(nt2_local_ws::owner::Id::OsProcess(pid),Some(birth)).is_some());assert!(routes.select(nt2_local_ws::owner::Id::OsProcess(pid),Some(birth.wrapping_add(1))).is_none());assert!(routes.select(nt2_local_ws::owner::Id::OsProcess(pid),None).is_none());assert!(routes.select(nt2_local_ws::owner::Id::OsProcess(pid.wrapping_add(1)),Some(birth)).is_none());assert!(routes.register(nt2_local_ws::owner::Id::OsProcess(pid),p).is_err());routes.remove(nt2_local_ws::owner::Id::OsProcess(pid),birth.wrapping_add(1));assert!(routes.select(nt2_local_ws::owner::Id::OsProcess(pid),Some(birth)).is_some());routes.remove(nt2_local_ws::owner::Id::OsProcess(pid),birth);assert!(routes.select(nt2_local_ws::owner::Id::OsProcess(pid),Some(birth)).is_none());
}
#[cfg(test)]pub(crate) fn start_lc1_fixture_routes(name:String,expected:std::path::PathBuf,routes:Arc<FixtureRoutes>,stop:Arc<AtomicBool>)->Result<thread::JoinHandle<()>,String>{start_inner_diagnostics_routed(name,Arc::new(Mutex::new(None)),stop,move|pid|matches_local_owner(pid,&expected),true,None,Some(Arc::new(Mutex::new(Default::default()))),Some(Arc::new(Mutex::new(Default::default()))),Some(routes))}

fn start_inner_diagnostics_routed<F: Fn(crate::local_owner::Id) -> bool + Send + 'static>(
    name: String,
    pending: Arc<Mutex<Option<super::handoff::Pending>>>,
    stop: Arc<AtomicBool>,
    trust: F,
    inspection: bool,
    diagnostics: Option<Arc<Mutex<NativeDiagnostics>>>,
    g3: Option<Arc<Mutex<nt2_desktop_g0::g3_runtime::Exchange>>>,
    social: Option<Arc<Mutex<nt2_desktop_g0::g3_social_stream::Exchange>>>,
    #[cfg(test)] routes:Option<Arc<FixtureRoutes>>,
) -> Result<thread::JoinHandle<()>, String> {
    let capacity = if social.is_some(){nt2_desktop_g0::g3_social_stream::REPLY} else if inspection {
        crate::native_report::HEADER_BYTES + crate::native_report::REPORT_BYTES
    } else {
        32
    };
    let sid = user_sid()?;
    let sddl = wide(&format!("D:P(A;;GA;;;{sid})"));
    let mut descriptor = null_mut();
    let raw = unsafe {
        if ConvertStringSecurityDescriptorToSecurityDescriptorW(
            sddl.as_ptr(),
            1,
            &mut descriptor,
            null_mut(),
        ) == 0
        {
            return Err("Pipe ACL unavailable".into());
        }
        let security = Security {
            len: std::mem::size_of::<Security>() as u32,
            descriptor,
            inherit: 0,
        };
        let h = CreateNamedPipeW(
            wide(&name).as_ptr(),
            3 | 0x80000,
            4 | 2 | 1 | 8,
            1,
            capacity as u32,
            capacity as u32,
            0,
            &security,
        );
        LocalFree(descriptor);
        h
    };
    if raw as isize == -1 {
        return Err("Another app owns the local bridge or pipe creation failed".into());
    }
    let handle = raw as usize;
    Ok(thread::spawn(move || {
        let h = Owned(handle as Handle);
        let startup=inspection.then(||Arc::new(startup::Lane::new(pending.clone())));
        while !stop.load(Ordering::Relaxed) {
            let connected =
                unsafe { ConnectNamedPipe(h.0, null_mut()) != 0 || GetLastError() == 535 };
            if !connected {
                // A production hint refusal can close the client before the
                // nonblocking listener observes it. Reset that closed instance.
                if inspection && unsafe { GetLastError() } == 232 {
                    unsafe {
                        DisconnectNamedPipe(h.0);
                    }
                    if let Some(d) = &diagnostics {
                        let mut d = d.lock().unwrap();
                        d.contacts = d.contacts.saturating_add(1);
                        d.note("Game briefly contacted app but sent no handshake; check the selected candidate app and installation");
                    }
                }
                thread::sleep(Duration::from_millis(10));
                continue;
            }
            let mut os_pid=0;
            let identified=unsafe{GetNamedPipeClientProcessId(h.0,&mut os_pid)!=0};
            let pid=crate::local_owner::Id::os_process(os_pid);let trusted=identified&&trust(pid);
            #[cfg(test)]let selected=if trusted{routes.as_ref().and_then(|r|r.select(pid,owner_instance(pid)))}else{None};
            #[cfg(test)]let trusted=trusted&&(routes.is_none()||selected.is_some());
            #[cfg(test)]let (pending,diagnostics,g3,social,startup)=if let Some(p)=selected{(p.pending,Some(p.diagnostics),Some(p.gameplay),Some(p.social),Some(p.startup))}else{(pending.clone(),if routes.is_none(){diagnostics.clone()}else{None},g3.clone(),social.clone(),startup.clone())};
            if trusted{if let Some(d)=&diagnostics{let mut d=d.lock().unwrap();d.contacts=d.contacts.saturating_add(1);d.last_contact=Some(Instant::now());}}
            #[cfg(test)]
            if !trusted {if let Some(d)=&diagnostics {d.lock().unwrap().initial_contact_trace.push(serde_json::json!({"op":0,"trustedProcess":false}));}}
            if trusted {
                let mut received = false;
                let mut request = vec![0u8; capacity];
                let mut n = 0;
                let deadline = Instant::now() + Duration::from_millis(100);
                while Instant::now() < deadline && !stop.load(Ordering::Relaxed) {
                    let ok = unsafe {
                        ReadFile(
                            h.0,
                            request.as_mut_ptr().cast(),
                            capacity as u32,
                            &mut n,
                            null_mut(),
                        )
                    };
                    if ok != 0 {
                        if n == 32 || inspection && (n==40&&u32::from_le_bytes(request[4..8].try_into().unwrap())==6 || n as usize >= crate::native_report::HEADER_BYTES)
                        {
                            received = true;
                            #[cfg(test)]
                            if n==32&&u32::from_le_bytes(request[8..12].try_into().unwrap())==5{
                                if let Some(d)=&diagnostics{
                                    let delay={let mut d=d.lock().unwrap();std::mem::take(&mut d.delay_next_heartbeat)};
                                    if delay{thread::sleep(Duration::from_millis(100));}
                                }
                            }
                            let dispatcher=Dispatcher{pending:pending.clone(),diagnostics:diagnostics.clone(),g3:g3.clone(),social:social.clone(),startup:startup.clone()};
                            let Some(reply)=dispatcher.reply(pid,&request[..n as usize])else{break};
                            let sent = unsafe {
                                WriteFile(h.0, reply.as_ptr().cast(), reply.len() as u32, &mut n, null_mut())
                            };
                            if sent != 0 && n as usize == reply.len() {
                                #[cfg(test)]
                                if request[..4] == super::handoff::MAGIC.to_le_bytes() {
                                    let op=u32::from_le_bytes(request[8..12].try_into().unwrap());
                                    if let Some(d)=&diagnostics {
                                        let delay={let mut d=d.lock().unwrap();if d.initial_contact_busy.is_some_and(|(target,_)|target==op){d.initial_contact_busy.take().map(|(_,ms)|ms)}else{None}};
                                        if let Some(ms)=delay {thread::sleep(Duration::from_millis(ms));}
                                    }
                                }
                                // Disconnect discards unread replies. Await client closure, never blocking on flush.
                                let delivery = Instant::now() + Duration::from_millis(80);
                                while Instant::now() < delivery && !stop.load(Ordering::Relaxed) {
                                    if unsafe {
                                        PeekNamedPipe(
                                            h.0,
                                            null_mut(),
                                            0,
                                            null_mut(),
                                            null_mut(),
                                            null_mut(),
                                        )
                                    } == 0
                                    {
                                        break;
                                    }
                                    thread::sleep(Duration::from_millis(1));
                                }
                            }
                        }
                        break;
                    }
                    if unsafe { GetLastError() } != 232 {
                        break;
                    }
                    thread::sleep(Duration::from_millis(1));
                }
                if !received {
                    if let Some(d) = &diagnostics {
                        d.lock().unwrap().note("Game connected but sent no handshake; check that this candidate app is the selected bridge target");
                    }
                }
            }
            unsafe {
                DisconnectNamedPipe(h.0);
            }
            thread::sleep(Duration::from_millis(1));
        }
    }))
}
#[cfg(test)]
#[path="pipe/tests.rs"]
mod tests;

include!("pipe_dispatch.rs");
