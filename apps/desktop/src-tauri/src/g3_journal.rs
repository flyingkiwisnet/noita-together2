//! Local original-operation journal. A flushed app record is never loaded-world proof.
use crate::protocol_g3::{self, wire};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap,BTreeSet},
    fs::{File, OpenOptions},
    io::{Read, Write},
    path::{Path,PathBuf},
};

pub const MAX_OPERATIONS: usize = 256;
pub const MAX_RECORDS: usize = 2048;
pub const MAX_RECORD_BYTES: usize = 196608;
const MAX_FILE_BYTES: usize = 40*1024*1024;
const MAX_DELETED_LOBBIES:usize=1024;

/// Original service/account/lobby identity, independent of run/session epochs.
#[derive(Debug,Clone,PartialEq,Eq,PartialOrd,Ord,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LobbyIdentity{pub service_origin:String,pub service_id:String,pub principal_id:String,pub room_id:String}
impl LobbyIdentity{
    pub fn from_original(original:&Original)->Result<Self,String>{let scope=validate_retained_original(original)?;Ok(Self{service_origin:original.service_origin.clone(),service_id:original.service_id.clone(),principal_id:original.principal_id.clone(),room_id:scope.room_id.unwrap()})}
    fn valid(&self)->bool{let Ok(origin)=reqwest::Url::parse(&self.service_origin)else{return false};self.service_origin.len()<=2048&&origin.scheme()=="https"&&origin.host_str().is_some()&&origin.username().is_empty()&&origin.password().is_none()&&origin.path()=="/"&&origin.query().is_none()&&origin.fragment().is_none()&&origin.as_str().trim_end_matches('/')==self.service_origin&&[&self.service_id,&self.principal_id,&self.room_id].iter().all(|s|identifier(s))}
}
/// Constructed inside the authenticated/correlated HTTP seam, never from UI,
/// discovery absence, End/leave or an arbitrary operation/status error string.
pub struct DeletedLobby{identity:LobbyIdentity}
impl DeletedLobby{
    pub(crate) fn authenticated(identity:LobbyIdentity)->Result<Self,String>{if !identity.valid(){return Err("Invalid deleted lobby identity".into())}Ok(Self{identity})}
    pub fn identity(&self)->&LobbyIdentity{&self.identity}
}
#[derive(Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
struct Checkpoint{version:u32,sequence:u32,kind:String,#[serde(skip_serializing_if="Option::is_none")]deleted:Option<LobbyIdentity>,#[serde(skip_serializing_if="Option::is_none")]original:Option<Original>,#[serde(skip_serializing_if="Option::is_none")]original_id:Option<String>,#[serde(skip_serializing_if="Option::is_none")]delivery:Option<Delivery>,#[serde(skip_serializing_if="Option::is_none")]phase:Option<Phase>}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Original {
    #[serde(default,skip_serializing_if="Option::is_none")]
    pub item:Option<crate::g3_items::Original>,
    pub service_origin: String,
    pub service_id: String,
    pub service_epoch: String,
    pub session_epoch: String,
    pub principal_id: String,
    /// Validated canonical Scope serialized once; immutable across every status query.
    pub scope_json: String,
    pub operation_id: String,
    pub direction: String,
    pub amount: u32,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Delivery {
    pub delivery_id: String,
    pub payload_digest: String,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Phase {
    Requested,
    ServerAccepted,
    EffectIntent,
    AppliedSession,
    Uncertain,
    Refused,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Record {
    version: u32,
    sequence: u32,
    pub original: Original,
    pub delivery: Option<Delivery>,
    pub phase: Phase,
}
#[derive(Debug, Clone)]
pub struct Pending {
    pub record: Record,
    pub restored: bool,
}
/// Single-use, in-process permission produced only after sync_all of EffectIntent.
/// This is not a bank receipt and must not be reconstructed after restart.
pub struct EffectPermit {
    operation_id: String,
}
pub struct Journal {
    file: Option<File>,
    path:PathBuf,
    _owner:File,
    entries: BTreeMap<String, Pending>,
    deleted:BTreeSet<LobbyIdentity>,
    records: usize,
    poisoned: bool,
}

/// One file owner shared by optional workers. Readers use a memory snapshot:
/// lifecycle/movement/UI never wait for a bank fsync. Mutations belong off Core.
#[derive(Clone)]
pub struct SharedJournal {
    owner: std::sync::Arc<std::sync::Mutex<Journal>>,
    snapshot: std::sync::Arc<std::sync::RwLock<(Vec<Pending>, bool,BTreeSet<LobbyIdentity>)>>,
}
impl SharedJournal {
    /// Optional C1 worker uses the same owned storage directory, outside Core.
    pub(crate) fn c2_path(&self)->PathBuf{self.owner.lock().unwrap().path.with_file_name("c2-effects.jsonl")}
    pub(crate) fn c1_path(&self)->PathBuf{self.owner.lock().unwrap().path.with_file_name("c1-effects.jsonl")}
    pub fn new(journal: Journal) -> Self {
        let snapshot=(journal.pending().cloned().collect(),journal.storage_blocked(),journal.deleted.clone());
        Self{owner:std::sync::Arc::new(std::sync::Mutex::new(journal)),snapshot:std::sync::Arc::new(std::sync::RwLock::new(snapshot))}
    }
    pub fn pending(&self)->std::vec::IntoIter<Pending>{self.snapshot.read().unwrap().0.clone().into_iter()}
    pub fn storage_blocked(&self)->bool{self.snapshot.read().unwrap().1}
    pub(crate) fn deleted_lobbies(&self)->Vec<LobbyIdentity>{self.snapshot.read().unwrap().2.iter().cloned().collect()}
    pub fn lobby_deleted(&self,identity:&LobbyIdentity)->bool{self.snapshot.read().unwrap().2.contains(identity)}
    pub fn retire_lobby(&self,proof:DeletedLobby)->Result<usize,String>{self.update(|j|j.retire_lobby(proof))}
    pub fn uncertain(&self,id:&str)->bool{self.pending().any(|p|p.record.original.operation_id==id&&(p.restored||matches!(p.record.phase,Phase::EffectIntent|Phase::Uncertain)))}
    fn update<T>(&self,work:impl FnOnce(&mut Journal)->Result<T,String>)->Result<T,String>{
        let mut owner=self.owner.lock().unwrap();let result=work(&mut owner);
        *self.snapshot.write().unwrap()=(owner.pending().cloned().collect(),owner.storage_blocked(),owner.deleted.clone());result
    }
    pub fn begin(&self,original:Original)->Result<(),String>{self.update(|j|j.begin(original))}
    pub fn refuse(&self,id:&str)->Result<(),String>{self.update(|j|{let p=j.entries.get(id).ok_or("Missing original")?.clone();if p.record.phase!=Phase::Requested{return Err("Cannot refuse an accepted original".into())}j.append(p.record.original,None,Phase::Refused)})}
    pub fn accept_operation(&self,op:&wire::Operation)->Result<(),String>{self.update(|j|j.accept_operation(op))}
    pub fn accept_item_operation(&self,op:&wire::ItemOperation)->Result<(),String>{self.update(|j|j.accept_item_operation(op))}
    pub fn validate_item_status(&self,op:&wire::ItemOperation)->Result<(),String>{let id=op.operation_id.as_deref().ok_or("Missing item operation")?;let p=self.pending().find(|p|p.record.original.operation_id==id).ok_or("Missing item original")?;crate::g3_items::operation(&p.record.original,op)?;if p.record.delivery.as_ref().is_some_and(|d|Some(&d.delivery_id)!=op.delivery_id.as_ref()||Some(&d.payload_digest)!=op.payload_digest.as_ref()){return Err("Item original delivery changed".into())}Ok(())}
    pub fn recover_item_unissued(&self,op:&wire::ItemOperation)->Result<(),String>{self.update(|j|j.recover_item_unissued(op))}
    pub fn validate_status(&self,op:&wire::Operation)->Result<(),String>{let id=op.operation_id.as_deref().ok_or("Missing operation")?;let pending=self.pending().find(|p|p.record.original.operation_id==id).ok_or("Missing original")?;validate_pending(&pending,op)}
    pub fn recover_unissued(&self,op:&wire::Operation)->Result<(),String>{self.update(|j|j.recover_unissued(op))}
    pub fn prepare_effect(&self,id:&str)->Result<EffectPermit,String>{self.update(|j|j.prepare_effect(id))}
    pub fn finish_session_effect(&self,permit:EffectPermit,applied:bool)->Result<(),String>{self.update(|j|j.finish_session_effect(permit,applied))}
}
pub trait JournalView {
    fn originals(&self)->Vec<Pending>;
    fn check_status(&self,operation:&wire::Operation)->Result<(),String>;
    fn check_item_status(&self,operation:&wire::ItemOperation)->Result<(),String>;
}
impl JournalView for Journal{
    fn originals(&self)->Vec<Pending>{self.pending().cloned().collect()}
    fn check_status(&self,op:&wire::Operation)->Result<(),String>{self.validate_status(op)}
    fn check_item_status(&self,op:&wire::ItemOperation)->Result<(),String>{self.validate_item_status(op)}
}
impl JournalView for SharedJournal{
    fn originals(&self)->Vec<Pending>{self.pending().collect()}
    fn check_status(&self,op:&wire::Operation)->Result<(),String>{self.validate_status(op)}
    fn check_item_status(&self,op:&wire::ItemOperation)->Result<(),String>{self.validate_item_status(op)}
}

fn uuid(value: &str) -> bool {
    uuid::Uuid::parse_str(value).is_ok_and(|id| id.to_string() == value)
}
fn identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 80
        && value
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'-' | b'_' | b'.'))
}
fn validate_original(original: &Original) -> Result<wire::Scope, String> {validate_original_policy(original,true)}
fn validate_retained_original(original: &Original) -> Result<wire::Scope,String>{validate_original_policy(original,false)}
fn validate_original_policy(original: &Original,current:bool) -> Result<wire::Scope, String> {
    let origin =
        reqwest::Url::parse(&original.service_origin).map_err(|_| "Invalid service origin")?;
    if origin.scheme() != "https"
        || origin.host_str().is_none()
        || !origin.username().is_empty()
        || origin.password().is_some()
        || origin.query().is_some()
        || origin.fragment().is_some()
        || origin.path() != "/"
        || origin.as_str().trim_end_matches('/') != original.service_origin
    {
        return Err("Expected normalized HTTPS origin".into());
    }
    if !identifier(&original.service_epoch)
        || !identifier(&original.session_epoch)
        || !identifier(&original.service_id)
        || !identifier(&original.principal_id)
        || !uuid(&original.operation_id)
        || if let Some(item)=&original.item{original.direction!=item.kind||original.amount!=1||(if current{crate::g3_items::validate_original(item)}else{crate::g3_items::retained_original(item)}).is_err()}else{!matches!(original.direction.as_str(),"deposit"|"withdraw")||!(1..=1_000_000).contains(&original.amount)}
    {
        return Err("Invalid original operation".into());
    }
    let scope: wire::Scope = protocol_g3::typed("Scope", original.scope_json.as_bytes())
        .map_err(|_| "Invalid canonical scope")?;
    if [
        &scope.room_id,
        &scope.run_id,
        &scope.run_epoch,
        &scope.attachment_id,
    ]
    .iter()
    .any(|id| id.as_ref().is_none_or(|id| !identifier(id)))
        || scope.config_revision.is_none()
        // Canonical world tokens are opaque UTF-8, at most 80 bytes, without normalization.
        || scope.world_token.as_ref().is_none_or(|token| token.is_empty() || token.len() > 80)
    {
        return Err("Incomplete canonical scope".into());
    }
    Ok(scope)
}
/// Server-owned operation-digest-1: sorted compact JSON and exact UTF-8 bytes.
/// No operation/request/delivery ID is part of this digest.
pub fn operation_digest(original: &Original) -> Result<String, String> {
    use sha2::{Digest, Sha256};
    Ok(Sha256::digest(operation_digest_bytes(original)?).iter().map(|b| format!("{b:02x}")).collect())
}
fn retained_operation_digest(original:&Original)->Result<String,String>{
    use sha2::{Digest,Sha256};
    Ok(Sha256::digest(digest_bytes_policy(original,false)?).iter().map(|b|format!("{b:02x}")).collect())
}
fn operation_digest_bytes(original: &Original) -> Result<Vec<u8>, String> {digest_bytes_policy(original,true)}
fn digest_bytes_policy(original:&Original,current:bool)->Result<Vec<u8>,String>{
    let scope = validate_original_policy(original,current)?;
    if let Some(item)=&original.item{return if current{crate::g3_items::digest_bytes(&original.principal_id,&scope,item)}else{crate::g3_items::retained_digest_bytes(&original.principal_id,&scope,item)}}
    // Explicit field order is independent of serde_json's preserve_order feature.
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct DigestScope<'a> {
        attachment_id: &'a str, config_revision: u32, room_id: &'a str,
        run_epoch: &'a str, run_id: &'a str, world_token: &'a str,
    }
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Canonical<'a> {
        amount: u32, direction: &'a str, principal_id: &'a str, scope: DigestScope<'a>, v: u32,
    }
    let canonical = Canonical {
        amount: original.amount, direction: &original.direction, principal_id: &original.principal_id,
        scope: DigestScope {
            attachment_id: scope.attachment_id.as_deref().unwrap(), config_revision: scope.config_revision.unwrap(),
            room_id: scope.room_id.as_deref().unwrap(), run_epoch: scope.run_epoch.as_deref().unwrap(),
            run_id: scope.run_id.as_deref().unwrap(), world_token: scope.world_token.as_deref().unwrap(),
        }, v: 1,
    };
    serde_json::to_vec(&canonical).map_err(|_| "Digest encode failed".into())
}
fn validate_delivery(delivery: &Delivery) -> bool {
    uuid(&delivery.delivery_id)
        && delivery.payload_digest.len() == 64
        && delivery
            .payload_digest
            .bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
}
pub(crate) fn plain_path(path: &Path) -> Result<(), String> {
    if !path.is_absolute()
        || path.to_string_lossy().starts_with(r"\\")
        || path
            .components()
            .any(|part| matches!(part, std::path::Component::ParentDir))
    {
        return Err("Journal must use an absolute local path".into());
    }
    for parent in path.ancestors() {
        if let Ok(meta) = std::fs::symlink_metadata(parent) {
            if meta.file_type().is_symlink() {
                return Err("Journal links are refused".into());
            }
            #[cfg(windows)]
            {
                use std::os::windows::fs::MetadataExt;
                if meta.file_attributes() & 0x400 != 0 {
                    return Err("Journal reparse paths are refused".into());
                }
            }
        } else {
            return Err("Journal parent must already exist".into());
        }
    }
    Ok(())
}
fn validate_pending(pending:&Pending,operation:&wire::Operation)->Result<(),String>{
        let original = &pending.record.original;
        let scope = validate_original(original)?;
        let delivery = Delivery { delivery_id: operation.delivery_id.clone().ok_or("Missing delivery")?,
            payload_digest: operation.payload_digest.clone().ok_or("Missing digest")? };
        if operation.principal_id.as_deref() != Some(&original.principal_id)
            || operation.run_id != scope.run_id || operation.attachment_id != scope.attachment_id
            || operation.world_token != scope.world_token || operation.direction.as_deref() != Some(&original.direction)
            || operation.amount != Some(original.amount) || !validate_delivery(&delivery)
            || delivery.payload_digest != operation_digest(original)?
            || pending.record.delivery.as_ref().is_some_and(|prior| prior != &delivery)
            || !matches!(operation.state.as_deref(), Some("awaiting_local" | "reserved" | "uncertain" | "completed" | "cancelled")) {
            return Err("Status differs from immutable ORIGINAL".into());
        }
        Ok(())
}
pub(crate) fn owned_file(path:&Path,create_new:bool)->Result<File,String>{
    let mut options=OpenOptions::new();options.read(true).append(true);
    if create_new{options.create_new(true);}else{options.create(true);}
    #[cfg(windows)]{use std::os::windows::fs::OpenOptionsExt;options.share_mode(1|4);}
    options.open(path).map_err(|_|"Journal unavailable or already owned".into())
}
pub(crate) fn journal_owner(path:&Path)->Result<File,String>{
    let owner=path.with_file_name(format!("{}.owner",path.file_name().ok_or("Journal name absent")?.to_string_lossy()));
    if owner.exists(){plain_path(&owner)?;}
    let mut options=OpenOptions::new();options.read(true).write(true).create(true);
    // A separate path lock survives atomic replacement of the data file. The
    // Windows handle is exclusive and deletes only this owned lock on close,
    // including process death; ownership is not tied to a worker thread.
    #[cfg(windows)]{use std::os::windows::fs::OpenOptionsExt;options.share_mode(0).access_mode(0xc0010000).custom_flags(0x04000000);}
    let file=options.open(owner).map_err(|_|"Journal unavailable or already owned")?;
    #[cfg(not(windows))]{file.try_lock().map_err(|_|"Journal unavailable or already owned")?;}
    Ok(file)
}
pub(crate) fn replace_journal(temp:&Path,target:&Path)->Result<(),String>{
    #[cfg(windows)]{
        use std::os::windows::ffi::OsStrExt;
        #[link(name="kernel32")]unsafe extern "system"{fn MoveFileExW(from:*const u16,to:*const u16,flags:u32)->i32;}
        let from:Vec<u16>=temp.as_os_str().encode_wide().chain(Some(0)).collect();let to:Vec<u16>=target.as_os_str().encode_wide().chain(Some(0)).collect();
        // Same-directory atomic replacement after flushing and closing data
        // handles; the independent path owner remains exclusive throughout.
        if unsafe{MoveFileExW(from.as_ptr(),to.as_ptr(),1|8)}==0{return Err(format!("Journal deletion commit failed ({})",std::io::Error::last_os_error().raw_os_error().unwrap_or(0)))}
    }
    #[cfg(not(windows))]{std::fs::rename(temp,target).map_err(|_|"Journal deletion commit failed")?;File::open(target.parent().ok_or("Journal parent absent")?).and_then(|f|f.sync_all()).map_err(|_|"Journal deletion directory flush failed")?;}
    Ok(())
}
impl Journal {
    /// Directory creation belongs to the candidate storage owner, not this effect guard.
    pub fn open(path: &Path) -> Result<Self, String> {
        plain_path(path.parent().ok_or("Missing journal parent")?)?;
        if path.exists() {
            plain_path(path)?;
        }
        let owner=journal_owner(path)?;
        let mut file=owned_file(path,false)?;
        if file
            .metadata()
            .map_err(|_| "Journal metadata unavailable")?
            .len()
            > MAX_FILE_BYTES as u64
        {
            return Err("Journal storage limit".into());
        }
        let mut bytes = Vec::new();
        (&mut file)
            .take(MAX_FILE_BYTES as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| "Journal read failed")?;
        if bytes.len() > MAX_FILE_BYTES || (!bytes.is_empty() && bytes.last() != Some(&b'\n')) {
            return Err("Journal torn or oversized; preserve it for reconciliation".into());
        }
        let mut journal = Self {
            file:Some(file),
            path:path.to_path_buf(),
            _owner:owner,
            entries: BTreeMap::new(),
            deleted:BTreeSet::new(),
            records: 0,
            poisoned: false,
        };
        let contents = bytes.strip_suffix(b"\n").unwrap_or(&bytes);
        if !bytes.is_empty() && contents.is_empty() {
            return Err("Empty journal record".into());
        }
        for line in contents
            .split(|b| *b == b'\n')
            .filter(|_| !contents.is_empty())
        {
            if line.len() + 1 > MAX_RECORD_BYTES || journal.records >= MAX_RECORDS {
                return Err("Journal record limit".into());
            }
            let shape:serde_json::Value=serde_json::from_slice(line).map_err(|_|"Invalid journal record")?;
            if shape.get("kind").is_some(){journal.load_checkpoint(line)?;continue}
            let record: Record = serde_json::from_value(shape).map_err(|_| "Invalid journal record")?;
            journal.check(&record)?;
            journal.records += 1;
            journal.entries.insert(
                record.original.operation_id.clone(),
                Pending {
                    record,
                    restored: true,
                },
            );
        }
        Ok(journal)
    }
    pub fn pending(&self) -> impl Iterator<Item = &Pending> {
        self.entries.values().filter(|p|LobbyIdentity::from_original(&p.record.original).is_ok_and(|k|!self.deleted.contains(&k)))
    }
pub fn lobby_deleted(&self,identity:&LobbyIdentity)->bool{self.deleted.contains(identity)}
    fn load_checkpoint(&mut self,line:&[u8])->Result<(),String>{
        let value:Checkpoint=serde_json::from_slice(line).map_err(|_|"Invalid journal checkpoint")?;
        if value.version!=2||value.sequence as usize!=self.records+1{return Err("Invalid journal checkpoint sequence".into())}
        match value.kind.as_str(){
            "lobby_deleted"=>{
                let key=value.deleted.ok_or("Missing deleted lobby identity")?;
                if !key.valid()||value.original.is_some()||value.original_id.is_some()||value.delivery.is_some()||value.phase.is_some()||self.deleted.len()>=MAX_DELETED_LOBBIES||self.deleted.contains(&key)||!self.entries.is_empty(){return Err("Invalid lobby deletion checkpoint".into())}
                self.deleted.insert(key);
            },
            "snapshot"=>{
                if value.deleted.is_some()||value.original_id.is_some(){return Err("Invalid operation checkpoint".into())}
                let original=value.original.ok_or("Missing checkpoint original")?;let key=LobbyIdentity::from_original(&original)?;
                let phase=value.phase.ok_or("Missing checkpoint phase")?;
                let delivery_required=!matches!(phase,Phase::Requested|Phase::Refused);
                if self.deleted.contains(&key)||self.entries.contains_key(&original.operation_id)||self.entries.len()>=MAX_OPERATIONS||delivery_required!=value.delivery.is_some()
                    ||value.delivery.as_ref().is_some_and(|d|!validate_delivery(d)||retained_operation_digest(&original).as_deref()!=Ok(d.payload_digest.as_str())){return Err("Invalid operation checkpoint state".into())}
                self.entries.insert(original.operation_id.clone(),Pending{record:Record{version:1,sequence:value.sequence,original,delivery:value.delivery,phase},restored:true});
            },
            "transition"=>{
                if value.deleted.is_some()||value.original.is_some(){return Err("Invalid item transition".into())}
                let id=value.original_id.ok_or("Missing transition original")?;let prior=self.entries.get(&id).ok_or("Unknown transition original")?;
                if prior.record.original.item.is_none(){return Err("Transition requires item original".into())}
                let record=Record{version:1,sequence:value.sequence,original:prior.record.original.clone(),delivery:value.delivery,phase:value.phase.ok_or("Missing transition phase")?};self.check(&record)?;self.entries.insert(id,Pending{record,restored:true});
            },_=>return Err("Unknown journal checkpoint".into())
        }
        self.records+=1;Ok(())
    }
    /// The only destructive journal operation. A verified terminal lobby proof
    /// atomically replaces history with minimal deletion fences and surviving
    /// original snapshots. Other lobbies keep their current in-process phases;
    /// every reopened original remains restored/status-only as before.
    pub fn retire_lobby(&mut self,proof:DeletedLobby)->Result<usize,String>{
        if self.poisoned{return Err("Journal durability uncertain; deletion not confirmed".into())}
        let key=proof.identity;if self.deleted.contains(&key){return Ok(0)}
        if self.deleted.len()>=MAX_DELETED_LOBBIES{return Err("Deleted lobby fence capacity".into())}
        let count=self.entries.values().filter(|p|LobbyIdentity::from_original(&p.record.original).as_ref()==Ok(&key)).count();
        let mut deleted=self.deleted.clone();deleted.insert(key.clone());
        let mut kept:Vec<Pending>=self.entries.values().filter(|p|LobbyIdentity::from_original(&p.record.original).as_ref()!=Ok(&key)).cloned().collect();
        let mut bytes=Vec::new();let mut sequence=0u32;
        for identity in &deleted{
            sequence+=1;let record=Checkpoint{version:2,sequence,kind:"lobby_deleted".into(),deleted:Some(identity.clone()),original:None,original_id:None,delivery:None,phase:None};
            serde_json::to_writer(&mut bytes,&record).map_err(|_|"Deletion checkpoint encode")?;bytes.push(b'\n');
        }
        for pending in &mut kept{
            sequence+=1;pending.record.sequence=sequence;
            let record=Checkpoint{version:2,sequence,kind:"snapshot".into(),deleted:None,original:Some(pending.record.original.clone()),original_id:None,delivery:pending.record.delivery.clone(),phase:Some(pending.record.phase)};
            serde_json::to_writer(&mut bytes,&record).map_err(|_|"Original checkpoint encode")?;bytes.push(b'\n');
        }
        if bytes.len()>MAX_FILE_BYTES||bytes.split(|b|*b==b'\n').any(|line|line.len()+1>MAX_RECORD_BYTES){return Err("Deletion checkpoint size".into())}
        let temp=self.path.with_file_name(format!(".journal-delete-{}.tmp",uuid::Uuid::new_v4()));
        let result=(||{let mut file=owned_file(&temp,true)?;file.write_all(&bytes).and_then(|_|file.sync_all()).map_err(|_|"Deletion checkpoint flush failed")?;drop(file);
            // The separate path owner remains locked while Windows closes its
            // data handles for replacement; no second journal can enter here.
            drop(self.file.take());replace_journal(&temp,&self.path)?;self.file=Some(owned_file(&self.path,false)?);Ok::<_,String>(())})();
        match result{
            Ok(())=>{self.deleted=deleted;self.entries=kept.into_iter().map(|p|(p.record.original.operation_id.clone(),p)).collect();self.records=sequence as usize;Ok(count)},
            Err(error)=>{self.poisoned=true;self.deleted.insert(key);let _=std::fs::remove_file(temp);Err(error)}
        }
    }
    pub fn storage_blocked(&self) -> bool {
        self.poisoned || self.records >= MAX_RECORDS || self.entries.len() >= MAX_OPERATIONS
    }
    /// Read-only reconciliation. Does not change restored flags/phases or grant an effect.
    pub fn validate_status(&self, operation: &wire::Operation) -> Result<(), String> {
        let id = operation.operation_id.as_deref().ok_or("Missing original operation")?;
        let pending = self.entries.get(id).ok_or("Status has no retained ORIGINAL")?;
        validate_pending(pending,operation)
    }
    pub fn validate_item_status(&self,op:&wire::ItemOperation)->Result<(),String>{
        let id=op.operation_id.as_deref().ok_or("Missing item operation")?;let p=self.pending().find(|p|p.record.original.operation_id==id).ok_or("Missing item original")?;crate::g3_items::operation(&p.record.original,op)?;
        if p.record.delivery.as_ref().is_some_and(|d|Some(&d.delivery_id)!=op.delivery_id.as_ref()||Some(&d.payload_digest)!=op.payload_digest.as_ref()){return Err("Item original delivery changed".into())}Ok(())
    }
    pub fn uncertain(&self, id: &str) -> bool {
        self.entries.get(id).is_some_and(|entry| {
            entry.restored || matches!(entry.record.phase, Phase::EffectIntent | Phase::Uncertain)
        })
    }
    fn check(&self, record: &Record) -> Result<(), String> {
        validate_retained_original(&record.original)?;
        if self.deleted.contains(&LobbyIdentity::from_original(&record.original)?){return Err("Deleted lobby original is fenced".into())}
        if record.version != 1
            || record.sequence as usize != self.records + 1
            || record
                .delivery
                .as_ref()
                .is_some_and(|delivery| !validate_delivery(delivery)
                    || retained_operation_digest(&record.original).as_deref() != Ok(delivery.payload_digest.as_str()))
        {
            return Err("Invalid journal identity".into());
        }
        match self.entries.get(&record.original.operation_id) {
            None => {
                if self.entries.len() >= MAX_OPERATIONS
                    || record.phase != Phase::Requested
                    || record.delivery.is_some()
                {
                    return Err("Invalid initial record or operation capacity".into());
                }
            }
            Some(prior) => {
                if record.original != prior.record.original {
                    return Err("Original operation conflict".into());
                }
                let permitted = match (prior.record.phase, record.phase) {
                    (Phase::Requested, Phase::ServerAccepted) => record.delivery.is_some(),
                    (Phase::Requested, Phase::Refused) => record.delivery.is_none(),
                    (Phase::ServerAccepted, Phase::EffectIntent) => {
                        record.delivery == prior.record.delivery
                    }
                    (Phase::EffectIntent, Phase::AppliedSession | Phase::Uncertain) => {
                        record.delivery == prior.record.delivery
                    }
                    _ => false,
                };
                if !permitted {
                    return Err("Invalid local phase transition".into());
                }
            }
        }
        Ok(())
    }
    fn append(
        &mut self,
        original: Original,
        delivery: Option<Delivery>,
        phase: Phase,
    ) -> Result<(), String> {
        if self.poisoned || self.records >= MAX_RECORDS {
            return Err("Journal disabled; retain originals for status".into());
        }
        let record = Record {
            version: 1,
            sequence: (self.records + 1) as u32,
            original,
            delivery,
            phase,
        };
        self.check(&record)?;
        let mut bytes=if record.original.item.is_some()&&self.entries.contains_key(&record.original.operation_id){serde_json::to_vec(&Checkpoint{version:2,sequence:record.sequence,kind:"transition".into(),deleted:None,original:None,original_id:Some(record.original.operation_id.clone()),delivery:record.delivery.clone(),phase:Some(phase)})}else{serde_json::to_vec(&record)}.map_err(|_|"Journal encode failed")?;
        bytes.push(b'\n');
        if bytes.len() > MAX_RECORD_BYTES {
            return Err("Journal record limit".into());
        }
        // Any partial write or flush failure poisons this owner. Never truncate/retry an effect.
        let file=self.file.as_mut().ok_or("Journal append owner unavailable")?;
        if file.write_all(&bytes)
            .and_then(|_| file.sync_all())
            .is_err()
        {
            self.poisoned = true;
            return Err("Journal durability uncertain; no effect permission issued".into());
        }
        self.records += 1;
        self.entries.insert(
            record.original.operation_id.clone(),
            Pending {
                record,
                restored: false,
            },
        );
        Ok(())
    }
    /// Persist before sending GoldIntent. Duplicate status queries retain this original.
    pub fn begin(&mut self, original: Original) -> Result<(), String> {
        validate_original(&original)?;
        if self.deleted.contains(&LobbyIdentity::from_original(&original)?){return Err("Deleted lobby original is fenced".into())}
        if let Some(prior) = self.entries.get(&original.operation_id) {
            return if prior.record.original == original {
                Ok(())
            } else {
                Err("Original operation conflict".into())
            };
        }
        self.append(original, None, Phase::Requested)
    }
    /// Caller must authenticate and correlate the canonical service response first.
    /// This guard checks immutable identities; it does not authenticate an arbitrary Operation.
    pub fn accept_operation(&mut self, operation: &wire::Operation) -> Result<(), String> {
        let id = operation
            .operation_id
            .as_deref()
            .ok_or("Missing operation")?;
        let prior = self.entries.get(id).ok_or("No durable original")?.clone();
        if prior.restored {
            return Err("Restored operation requires status/reconciliation; no replay".into());
        }
        let scope = validate_original(&prior.record.original)?;
        let original = &prior.record.original;
        if operation.principal_id.as_deref() != Some(&original.principal_id)
            || operation.run_id != scope.run_id
            || operation.attachment_id != scope.attachment_id
            || operation.world_token != scope.world_token
            || operation.direction.as_deref() != Some(&original.direction)
            || operation.amount != Some(original.amount)
            || operation.state.as_deref()
                != Some(if original.direction == "deposit" {
                    "awaiting_local"
                } else {
                    "reserved"
                })
        {
            return Err("Service operation does not match original".into());
        }
        let delivery = Delivery {
            delivery_id: operation.delivery_id.clone().ok_or("Missing delivery")?,
            payload_digest: operation.payload_digest.clone().ok_or("Missing digest")?,
        };
        if !validate_delivery(&delivery) || delivery.payload_digest != operation_digest(original)? {
            return Err("Service digest does not match immutable original".into());
        }
        if prior.record.delivery.as_ref() == Some(&delivery) {
            return Ok(());
        }
        self.append(prior.record.original, Some(delivery), Phase::ServerAccepted)
    }
    pub fn prepare_effect(&mut self, id: &str) -> Result<EffectPermit, String> {
        let entry = self.entries.get(id).ok_or("Unknown operation")?.clone();
        validate_original(&entry.record.original)?;
        if entry.restored || entry.record.phase != Phase::ServerAccepted {
            return Err("No replay or uncertain compensation permitted".into());
        }
        self.append(
            entry.record.original,
            entry.record.delivery,
            Phase::EffectIntent,
        )?;
        Ok(EffectPermit {
            operation_id: id.to_owned(),
        })
    }
    pub fn accept_item_operation(&mut self,op:&wire::ItemOperation)->Result<(),String>{
        self.validate_item_status(op)?;let id=op.operation_id.as_deref().ok_or("Missing item operation")?;let prior=self.entries.get(id).ok_or("Missing item original")?.clone();
        if prior.restored{return Err("Restored item original requires status; no replay".into())}
        let direction=&prior.record.original.item.as_ref().ok_or("Not item original")?.kind;
        if op.state.as_deref()!=Some(if direction=="deposit"{"awaiting_local"}else{"reserved"}){return Err("Item original not pending local effect".into())}
        let delivery=Delivery{delivery_id:op.delivery_id.clone().ok_or("Item delivery absent")?,payload_digest:op.payload_digest.clone().ok_or("Item digest absent")?};
        if prior.record.delivery.as_ref()==Some(&delivery){return Ok(())}self.append(prior.record.original,Some(delivery),Phase::ServerAccepted)
    }
    pub fn recover_item_unissued(&mut self,op:&wire::ItemOperation)->Result<(),String>{
        self.validate_item_status(op)?;let id=op.operation_id.as_deref().ok_or("Missing item operation")?;let p=self.entries.get_mut(id).ok_or("Missing item original")?;
        if op.effect_allowed!=Some(true)||!matches!(p.record.phase,Phase::Requested|Phase::ServerAccepted){return Err("Item effect outcome requires reconciliation".into())}p.restored=false;self.accept_item_operation(op)
    }
    /// Authenticated current same-world authority plus a complete durable prefix
    /// ending BEFORE EffectIntent proves this app never issued the mutation.
    /// No wallet value, server ACK or AppliedSession record is save proof.
    pub fn recover_unissued(&mut self, operation:&wire::Operation)->Result<(),String>{
        self.validate_status(operation)?;
        let id=operation.operation_id.as_deref().ok_or("Missing operation")?;
        let entry=self.entries.get_mut(id).ok_or("Missing original")?;
        if operation.effect_allowed!=Some(true)||!matches!(entry.record.phase,Phase::Requested|Phase::ServerAccepted){return Err("Effect outcome requires reconciliation".into())}
        entry.restored=false;
        self.accept_operation(operation)
    }
    pub fn finish_session_effect(
        &mut self,
        permit: EffectPermit,
        observed_applied: bool,
    ) -> Result<(), String> {
        let entry = self
            .entries
            .get(&permit.operation_id)
            .ok_or("Unknown permit")?
            .clone();
        if entry.restored {
            return Err("Permit cannot cross process recovery".into());
        }
        self.append(
            entry.record.original,
            entry.record.delivery,
            if observed_applied {
                Phase::AppliedSession
            } else {
                Phase::Uncertain
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    mod lobby_deletion{use super::*;include!("g3_journal_delete_test.rs");}
    mod items{use super::*;include!("g3_journal_item_test.rs");}
    mod startup{use super::*;include!("g3_journal_startup_test.rs");}
    struct Fixture {
        dir: std::path::PathBuf,
        path: std::path::PathBuf,
    }
    impl Fixture {
        fn new() -> Self {
            let dir = Path::new(env!("CARGO_MANIFEST_DIR")).ancestors().nth(3).unwrap().join("spikes/noita-entry/.build")
                .join(format!("nt2-g3-journal-{}", uuid::Uuid::new_v4()));
            std::fs::create_dir_all(&dir).unwrap();
            Self {
                path: dir.join("operations.jsonl"),
                dir,
            }
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.path);
            let _ = std::fs::remove_dir(&self.dir);
        }
    }
    fn original() -> Original {
        Original { item:None, service_origin: "https://fixture.invalid".into(), service_id: "g3-fixture".into(), service_epoch: "service-epoch".into(), session_epoch: "game-epoch".into(), principal_id: "account-a".into(),
            scope_json: r#"{"roomId":"room-a","runId":"run-a","runEpoch":"run-epoch","configRevision":1,"attachmentId":"attachment-a","worldToken":"world-a"}"#.into(),
            operation_id: uuid::Uuid::new_v4().to_string(), direction: "withdraw".into(), amount: 100 }
    }
    #[test]
    fn canonical_unicode_vector_bytes_digest_and_effect_gate() {
        let vector: serde_json::Value = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"), "/../../../packages/contracts/g3/operation-digest.json"
        ))).unwrap();
        let mut original = original();
        original.principal_id = vector["principalId"].as_str().unwrap().into();
        original.scope_json = vector["scope"].to_string();
        original.direction = vector["direction"].as_str().unwrap().into();
        original.amount = vector["amount"].as_u64().unwrap() as u32;
        let bytes = operation_digest_bytes(&original).unwrap();
        assert_eq!(std::str::from_utf8(&bytes).unwrap(), vector["canonicalJson"].as_str().unwrap());
        assert_eq!(bytes.iter().map(|b| format!("{b:02x}")).collect::<String>(), vector["utf8Hex"]);
        assert_eq!(operation_digest(&original).unwrap(), vector["sha256"]);
        let scope = validate_original(&original).unwrap();
        let mut operation = operation(&original);
        operation.run_id = scope.run_id;
        operation.attachment_id = scope.attachment_id;
        operation.world_token = scope.world_token;
        let fixture = Fixture::new();
        let mut journal = Journal::open(&fixture.path).unwrap();
        journal.begin(original.clone()).unwrap();
        let mut wrong = operation.clone();
        wrong.payload_digest = Some("a".repeat(64));
        assert!(journal.accept_operation(&wrong).is_err());
        assert!(journal.prepare_effect(&original.operation_id).is_err());
        for field in ["principalId", "runId", "attachmentId", "worldToken", "direction", "amount"] {
            let mut wrong = serde_json::to_value(&operation).unwrap();
            wrong[field] = if field == "amount" { serde_json::json!(1) } else { serde_json::json!("substitution") };
            assert!(journal.accept_operation(&serde_json::from_value(wrong).unwrap()).is_err());
        }
        journal.accept_operation(&operation).unwrap();
        let permit = journal.prepare_effect(&original.operation_id).unwrap();
        journal.finish_session_effect(permit, true).unwrap();
        drop(journal);
        let mut restored = Journal::open(&fixture.path).unwrap();
        assert!(restored.uncertain(&original.operation_id));
        assert!(restored.prepare_effect(&original.operation_id).is_err());
    }
    #[test]
    fn unicode_only_relaxes_world_token_with_exact_byte_bounds() {
        let mut original = original();
        let mut scope: serde_json::Value = serde_json::from_str(&original.scope_json).unwrap();
        scope["worldToken"] = serde_json::json!("雪".repeat(26) + "ab"); // 80 UTF-8 bytes
        original.scope_json = scope.to_string();
        assert!(validate_original(&original).is_ok());
        scope["worldToken"] = serde_json::json!("雪".repeat(27));
        original.scope_json = scope.to_string();
        assert!(validate_original(&original).is_err());
        scope["worldToken"] = serde_json::json!("");
        original.scope_json = scope.to_string();
        assert!(validate_original(&original).is_err());
        scope["worldToken"] = serde_json::json!("world-雪");
        for field in ["roomId", "runId", "runEpoch", "attachmentId"] {
            let mut wrong = scope.clone();
            wrong[field] = serde_json::json!("雪");
            original.scope_json = wrong.to_string();
            assert!(validate_original(&original).is_err());
        }
    }
    fn operation(original: &Original) -> wire::Operation {
        wire::Operation {
            operation_id: Some(original.operation_id.clone()),
            principal_id: Some(original.principal_id.clone()),
            run_id: Some("run-a".into()),
            attachment_id: Some("attachment-a".into()),
            world_token: Some("world-a".into()),
            direction: Some(original.direction.clone()),
            amount: Some(original.amount),
            payload_digest: Some(operation_digest(original).unwrap()),
            delivery_id: Some(uuid::Uuid::new_v4().to_string()),
            state: Some(
                if original.direction == "deposit" {
                    "awaiting_local"
                } else {
                    "reserved"
                }
                .into(),
            ),
            ..Default::default()
        }
    }
    #[test]
    fn b1_only_durable_pre_effect_prefix_can_resume_after_restore(){
        for phase in [Phase::Requested,Phase::ServerAccepted,Phase::EffectIntent,Phase::AppliedSession,Phase::Uncertain]{
            let fixture=Fixture::new();let original=original();let id=original.operation_id.clone();let mut op=operation(&original);op.effect_allowed=Some(true);
            let mut journal=Journal::open(&fixture.path).unwrap();journal.begin(original).unwrap();
            if phase!=Phase::Requested{journal.accept_operation(&op).unwrap();}
            if matches!(phase,Phase::EffectIntent|Phase::AppliedSession|Phase::Uncertain){let permit=journal.prepare_effect(&id).unwrap();if phase!=Phase::EffectIntent{journal.finish_session_effect(permit,phase==Phase::AppliedSession).unwrap();}}
            drop(journal);let mut restored=Journal::open(&fixture.path).unwrap();assert!(restored.prepare_effect(&id).is_err());
            op.effect_allowed=Some(false);assert!(restored.recover_unissued(&op).is_err());op.effect_allowed=Some(true);
            if matches!(phase,Phase::Requested|Phase::ServerAccepted){restored.recover_unissued(&op).unwrap();assert!(!restored.uncertain(&id));let permit=restored.prepare_effect(&id).unwrap();restored.finish_session_effect(permit,true).unwrap();assert!(restored.prepare_effect(&id).is_err());}
            else{assert!(restored.recover_unissued(&op).is_err());assert!(restored.prepare_effect(&id).is_err());}
        }
    }
    #[test]
    fn b1_cached_readers_do_not_take_the_disk_owner_lock(){
        let fixture=Fixture::new();let original=original();let mut op=operation(&original);let journal=SharedJournal::new(Journal::open(&fixture.path).unwrap());journal.begin(original).unwrap();journal.accept_operation(&op).unwrap();
        let held=journal.owner.lock().unwrap();assert_eq!(journal.pending().count(),1);assert!(!journal.storage_blocked());journal.validate_status(&op).unwrap();op.amount=Some(999);assert!(journal.validate_status(&op).is_err());drop(held);
    }
    #[test]
    fn restored_app_phases_never_authorize_an_effect() {
        for phase in [
            Phase::Requested,
            Phase::ServerAccepted,
            Phase::EffectIntent,
            Phase::AppliedSession,
            Phase::Uncertain,
        ] {
            let fixture = Fixture::new();
            let original = original();
            let operation = operation(&original);
            let mut journal = Journal::open(&fixture.path).unwrap();
            journal.begin(original.clone()).unwrap();
            if phase != Phase::Requested {
                journal.accept_operation(&operation).unwrap();
            }
            if matches!(
                phase,
                Phase::EffectIntent | Phase::AppliedSession | Phase::Uncertain
            ) {
                let permit = journal.prepare_effect(&original.operation_id).unwrap();
                if phase != Phase::EffectIntent {
                    journal
                        .finish_session_effect(permit, phase == Phase::AppliedSession)
                        .unwrap();
                }
            }
            drop(journal);
            let mut restored = Journal::open(&fixture.path).unwrap();
            assert!(restored.uncertain(&original.operation_id));
            assert!(restored.prepare_effect(&original.operation_id).is_err());
            assert!(restored.accept_operation(&operation).is_err());
            restored.begin(original.clone()).unwrap();
            assert!(restored.pending().next().unwrap().restored);
            assert_eq!(restored.pending().next().unwrap().record.phase, phase);
        }
    }
    #[test]
    fn originals_and_delivery_conflicts_are_immutable() {
        let fixture = Fixture::new();
        let original = original();
        let mut journal = Journal::open(&fixture.path).unwrap();
        journal.begin(original.clone()).unwrap();
        journal.begin(original.clone()).unwrap();
        let mut conflict = original.clone();
        conflict.amount += 1;
        assert!(journal.begin(conflict).is_err());
        let operation = operation(&original);
        let mut wrong = operation.clone();
        wrong.world_token = Some("world-b".into());
        assert!(journal.accept_operation(&wrong).is_err());
        journal.accept_operation(&operation).unwrap();
        journal.accept_operation(&operation).unwrap();
        wrong = operation.clone();
        wrong.payload_digest = Some("b".repeat(64));
        assert!(journal.accept_operation(&wrong).is_err());
        let permit = journal.prepare_effect(&original.operation_id).unwrap();
        assert!(journal.prepare_effect(&original.operation_id).is_err());
        journal.finish_session_effect(permit, true).unwrap();
        assert!(!journal.uncertain(&original.operation_id)); // In-process observation only, never positive bank proof.
        assert!(journal.prepare_effect(&original.operation_id).is_err());
    }
    #[test]
    fn terminal_status_is_read_only_and_never_loaded_world_proof() {
        let fixture = Fixture::new(); let original = original();
        let mut journal = Journal::open(&fixture.path).unwrap(); journal.begin(original.clone()).unwrap();
        let operation = operation(&original); journal.accept_operation(&operation).unwrap();
        let permit = journal.prepare_effect(&original.operation_id).unwrap(); journal.finish_session_effect(permit, true).unwrap();
        drop(journal); let mut restored = Journal::open(&fixture.path).unwrap();
        let before = std::fs::read(&fixture.path).unwrap();
        let mut status = operation.clone(); status.state = Some("completed".into());
        restored.validate_status(&status).unwrap();
        assert_eq!(restored.pending().next().unwrap().record.phase, Phase::AppliedSession);
        assert!(restored.uncertain(&original.operation_id)); assert!(restored.prepare_effect(&original.operation_id).is_err());
        let mut substituted = status.clone(); substituted.payload_digest = Some("a".repeat(64));
        assert!(restored.validate_status(&substituted).is_err());
        substituted = status.clone(); substituted.delivery_id = Some(uuid::Uuid::new_v4().to_string());
        assert!(restored.validate_status(&substituted).is_err());
        assert_eq!(std::fs::read(&fixture.path).unwrap(), before);
    }
    #[test]
    fn malformed_torn_and_oversize_files_are_preserved() {
        let fixture = Fixture::new();
        for bytes in [b"{\"version\":1}".as_slice(), b"\n", b"{}\n", b"invalid\n"] {
            std::fs::write(&fixture.path, bytes).unwrap();
            assert!(Journal::open(&fixture.path).is_err());
            assert_eq!(std::fs::read(&fixture.path).unwrap(), bytes);
        }
        let file = File::create(&fixture.path).unwrap();
        file.set_len(MAX_FILE_BYTES as u64 + 1).unwrap();
        drop(file);
        assert!(Journal::open(&fixture.path).is_err());
        assert_eq!(
            std::fs::metadata(&fixture.path).unwrap().len(),
            MAX_FILE_BYTES as u64 + 1
        );
    }
    #[test]
    fn capacity_keeps_all_originals_across_contexts() {
        let fixture = Fixture::new();
        let mut journal = Journal::open(&fixture.path).unwrap();
        for i in 0..MAX_OPERATIONS {
            let mut original = original();
            original.scope_json = original
                .scope_json
                .replace("world-a", &format!("world-{i}"));
            journal.begin(original).unwrap();
        }
        assert!(journal.begin(original()).is_err());
        assert_eq!(journal.pending().count(), MAX_OPERATIONS);
        drop(journal);
        let restored = Journal::open(&fixture.path).unwrap();
        assert_eq!(restored.pending().count(), MAX_OPERATIONS);
        assert!(restored.pending().all(|entry| entry.restored));
    }
    #[test]
    fn invalid_inputs_and_poisoned_storage_cannot_issue_permits() {
        let fixture = Fixture::new();
        let mut journal = Journal::open(&fixture.path).unwrap();
        let mut invalid = original();
        invalid.service_origin = "https://user:password@fixture.invalid".into();
        assert!(journal.begin(invalid).is_err());
        invalid = original();
        invalid.scope_json = "{}".into();
        assert!(journal.begin(invalid).is_err());
        let original = original();
        journal.begin(original.clone()).unwrap();
        journal.accept_operation(&operation(&original)).unwrap();
        // An actual read-only file handle makes the pre-effect append fail.
        journal.file = Some(File::open(&fixture.path).unwrap());
        assert!(journal.prepare_effect(&original.operation_id).is_err());
        assert!(journal.storage_blocked());
        assert!(journal.prepare_effect(&original.operation_id).is_err());
        assert_eq!(
            journal.pending().next().unwrap().record.phase,
            Phase::ServerAccepted
        );
        assert!(Journal::open(Path::new("relative.jsonl")).is_err());
    }
    #[test]
    #[ignore = "Only invoked by the owned temporary journal subprocess test"]
    fn child_write_fixture() {
        let path = std::env::var_os("NT2_G3_JOURNAL_TEST_PATH").expect("explicit owned fixture");
        let original: Original =
            serde_json::from_str(&std::env::var("NT2_G3_JOURNAL_TEST_ORIGINAL").unwrap()).unwrap();
        let mut journal = Journal::open(Path::new(&path)).unwrap();
        journal.begin(original.clone()).unwrap();
        journal.accept_operation(&operation(&original)).unwrap();
        let permit = journal.prepare_effect(&original.operation_id).unwrap();
        journal.finish_session_effect(permit, true).unwrap();
    }
    #[test]
    fn flushed_original_survives_a_separate_process_without_becoming_world_proof() {
        let fixture = Fixture::new();
        let original = original();
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "g3_journal::tests::child_write_fixture",
                "--ignored",
            ])
            .env("NT2_G3_JOURNAL_TEST_PATH", &fixture.path)
            .env(
                "NT2_G3_JOURNAL_TEST_ORIGINAL",
                serde_json::to_string(&original).unwrap(),
            )
            .stdout(std::process::Stdio::null())
            .status()
            .unwrap();
        assert!(status.success());
        let mut journal = Journal::open(&fixture.path).unwrap();
        assert_eq!(journal.pending().next().unwrap().record.original, original);
        assert_eq!(
            journal.pending().next().unwrap().record.phase,
            Phase::AppliedSession
        );
        assert!(journal.uncertain(&original.operation_id));
        assert!(journal.prepare_effect(&original.operation_id).is_err());
    }
}
