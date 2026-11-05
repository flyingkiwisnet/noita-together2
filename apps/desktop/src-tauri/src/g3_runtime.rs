//! Owned desktop gameplay pump. Credentials originate in accepted Core; pipe facts
//! are separately verified by its native listener. No renderer-supplied authority.
use crate::{g3_http::HttpOwner, g3_journal::Journal, g3_local_consumer::{Binding, LocalConsumer}, protocol_g3::{self, wire}};
use std::{sync::{Arc, Mutex, atomic::{AtomicU64, Ordering}}, time::{Duration, Instant}};
use sha2::{Digest, Sha256};
use zeroize::Zeroizing;
pub const REPLY_BYTES: usize = 864;
/// Volatile bounded history; no arbitrary service/error text is retained here.
#[derive(Clone,Copy,serde::Serialize)]
#[serde(rename_all="snake_case")]
pub enum FailureStage { Authentication, Session, Presence, Room, Chat, Inspection, Game, Binding, Publication, Peers }
#[derive(Clone,Copy,serde::Serialize)]
#[serde(rename_all="snake_case")]
pub enum FailureCategory { ConnectionUnavailable, ResponseInterrupted, Capacity, StaleEpoch, StaleRevision, InvalidState, Forbidden, InvalidMessage, IncompatibleGame, Conflict, ExpiredAuthority, AuthorityChanged, InvalidResponse, Refused }
#[derive(Clone,Default,serde::Serialize)]
pub struct FailureSnapshot { count:u32, stage:Option<FailureStage>, category:Option<FailureCategory> }
impl FailureSnapshot {
    pub fn record(&mut self,stage:FailureStage,error:&str) {
        use FailureCategory::*;
        let category=match error {
            "Candidate connection unavailable"|"G3 connection unavailable; retain original IDs"=>ConnectionUnavailable,
            "Candidate response unavailable"|"G3 response interrupted; retain original IDs"=>ResponseInterrupted,
            "CAPACITY"|"G3 polling rate capacity"|"G3 owner rate capacity"=>Capacity,
            "STALE_EPOCH"=>StaleEpoch,"CONFLICT"=>Conflict,
            "STALE_REVISION"=>StaleRevision,"INVALID_STATE"=>InvalidState,"FORBIDDEN"=>Forbidden,
            "INVALID_MESSAGE"=>InvalidMessage,"INCOMPATIBLE_GAME"=>IncompatibleGame,
            "Expired gameplay authority"|"Expired movement authority"=>ExpiredAuthority,
            "Late G3 response discarded"|"Candidate response context changed"|"Retired G3 owner generation"|"Movement authority retired after publication"|"Movement authority retired across await"|"Native authority retired during ghost request"|"Movement owner changed across await"=>AuthorityChanged,
            "Invalid canonical G3 response"|"Candidate response violates canonical schema"|"Unexpected G3 response payload"=>InvalidResponse,
            _=>Refused,
        };
        self.count=self.count.saturating_add(1);self.stage=Some(stage);self.category=Some(category);
    }
}
pub struct Exchange {
    pub fence: Arc<AtomicU64>,
    c2_revision:u64,c1_revision:u64,bank_generation:u64,bank_revision:u64,bank_connection:Option<String>,bank_seen:Option<Instant>,
    pub(crate) owner: Option<(crate::local_owner::Id, [u8;16])>,
    pub(crate) until: Instant,
    pub(crate) pose: Option<[u8;64]>,
    sequence: u32,
    sequence_context:Option<[u8;16]>,
    pub(crate) peers: Vec<([u8;16], [u8;64])>,
    pub(crate) peers_at: Instant,
    pub(crate) peer_times:std::collections::BTreeMap<[u8;16],Instant>,
    cursor: usize,
    roster_trace: std::collections::VecDeque<(Instant,serde_json::Value)>,
    last_projection: Option<(Instant,serde_json::Value)>,
    last_reply_count: usize,
}
impl Default for Exchange {
    fn default() -> Self { Self { fence:Arc::new(AtomicU64::new(1)),c2_revision:0,c1_revision:0,bank_generation:0,bank_revision:0,bank_connection:None,bank_seen:None,owner:None,until:Instant::now(),pose:None,
        sequence:0,sequence_context:None,peers:vec![],peers_at:Instant::now(),peer_times:Default::default(),cursor:0,roster_trace:Default::default(),last_projection:None,last_reply_count:0 } }
}
impl Exchange {
    pub(crate) fn begin_bank_stream(&mut self,context:[u8;16])->Option<u64>{
        if !self.live_context(context){return None}
        self.bank_generation=self.bank_generation.checked_add(1)?;self.bank_connection=None;self.bank_revision=0;self.c1_revision=0;self.c2_revision=0;self.bank_seen=None;Some(self.bank_generation)
    }
    pub(crate) fn bank_gap(&mut self,context:[u8;16],generation:u64)->Option<u64>{
        if generation!=self.bank_generation{return None}self.begin_bank_stream(context)
    }
    pub(crate) fn bank_frame(&mut self,context:[u8;16],generation:u64,connection:&str,revision:Option<u64>){
        if !self.live_context(context)||generation!=self.bank_generation{return}
        if self.bank_connection.as_deref().is_some_and(|current|current!=connection){return}
        self.bank_connection=Some(connection.into());self.bank_revision=self.bank_revision.max(revision.unwrap_or(0));self.bank_seen=Some(Instant::now());
    }
    pub(crate) fn c1_frame(&mut self,context:[u8;16],generation:u64,sequence:u64){if self.live_context(context)&&generation==self.bank_generation{self.c1_revision=self.c1_revision.max(sequence);}}
    pub(crate) fn c2_frame(&mut self,context:[u8;16],generation:u64,sequence:u64){if self.live_context(context)&&generation==self.bank_generation{self.c2_revision=self.c2_revision.max(sequence);}}
    pub(crate) fn c2_signal(&self,context:[u8;16])->Option<(u64,u64)>{self.bank_signal(context).map(|(g,_)|(g,self.c2_revision))}
    pub(crate) fn c1_signal(&self,context:[u8;16])->Option<(u64,u64)>{self.bank_signal(context).map(|(g,_)|(g,self.c1_revision))}
    pub(crate) fn bank_signal(&self,context:[u8;16])->Option<(u64,u64)>{
        (self.live_context(context)&&self.bank_seen.is_some_and(|at|at.elapsed()<Duration::from_secs(3))).then_some((self.bank_generation,self.bank_revision))
    }
    fn trace_roster(&mut self,value:serde_json::Value){if self.roster_trace.len()>=32{self.roster_trace.pop_front();}self.roster_trace.push_back((Instant::now(),value));}
    pub(crate) fn note_projection(&mut self,value:serde_json::Value){self.last_projection=Some((Instant::now(),value.clone()));self.trace_roster(value);}
    pub(crate) fn roster_diagnostics(&self)->serde_json::Value{serde_json::json!({"cached_peers":self.peers.len(),"snapshot_age_ms":self.peers_at.elapsed().as_millis().min(60000),"trace":self.roster_trace.iter().map(|(at,value)|serde_json::json!({"age_ms":at.elapsed().as_millis().min(60000),"event":value})).collect::<Vec<_>>()})}
    fn live_context(&self,context:[u8;16])->bool{self.owner.is_some_and(|(_,current)|current==context)&&Instant::now()<self.until}
    pub fn pause(&mut self) {
        if self.owner.is_some(){self.trace_roster(serde_json::json!({"kind":"client_pause","cached_peers":self.peers.len()}));}self.last_reply_count=0;
        if self.owner.take().is_some() { self.fence.fetch_add(1,Ordering::AcqRel); }
        self.bank_generation=self.bank_generation.saturating_add(1);self.bank_seen=None;self.bank_connection=None;self.bank_revision=0;self.c1_revision=0;self.c2_revision=0;
        self.pose=None;self.peers.clear();self.peer_times.clear();self.cursor=0;
    }
    /// Rust owner drains one coalesced frame; this does not grant authority.
    pub fn take_pose(&mut self)->Option<[u8;64]>{self.pose.take()}
    fn arm(&mut self,pid:crate::local_owner::Id,context:[u8;16],until:Instant) {
        if self.sequence_context!=Some(context){self.sequence=0;self.sequence_context=Some(context);}
        if self.owner != Some((pid,context)) { self.pause();self.owner=Some((pid,context)); }
        self.until=until;
    }
    pub fn transition(&mut self,pid:crate::local_owner::Id,frame:&[u8],verified_playing:bool)->[u8;REPLY_BYTES] {
        let mut reply=[0u8;REPLY_BYTES];reply[..4].copy_from_slice(b"NG3R");reply[4]=2;
        let Some((owner,context))=self.owner else{return reply};
        if owner!=pid || frame.len()!=64 || frame[..4]!=*b"NTG3" || frame[4]!=1 || frame[5]!=0
            || frame[8..24]!=context {return reply;}
        if !verified_playing || Instant::now()>=self.until { self.pause();return reply; }
        match u16::from_le_bytes(frame[6..8].try_into().unwrap()) {
            1=>{
                let Ok(pose)=crate::g3_native_frame::decode(frame,&context,self.sequence) else{return reply};
                self.sequence=pose.sequence.unwrap();self.pose=Some(frame.try_into().unwrap());
            },
            2=>{if frame[24..].iter().any(|b|*b!=0){return reply;}},
            _=>return reply,
        }
        reply[5]=1;reply[8..24].copy_from_slice(&context);
        let lease=self.until.saturating_duration_since(Instant::now()).as_millis().min(200) as u32;
        if lease==0{return [0;REPLY_BYTES]}
        reply[24..28].copy_from_slice(&lease.to_le_bytes());
        let before_filter=self.peers.len();
        if !self.peer_times.is_empty(){let now=Instant::now();self.peers.retain(|(actor,_)|self.peer_times.get(actor).is_some_and(|at|now.saturating_duration_since(*at)<Duration::from_secs(2)));}
        if self.peers_at.elapsed()<=Duration::from_secs(2) && !self.peers.is_empty() {
            reply[28]=self.peers.len() as u8;
            for (i,(actor,_)) in self.peers.iter().enumerate(){reply[32+i*16..48+i*16].copy_from_slice(actor);}
        }
        if self.peers_at.elapsed()<=Duration::from_millis(500) && !self.peers.is_empty() {
            let count=self.peers.len().min(4);reply[6]=count as u8;
            for i in 0..count {let index=(self.cursor+i)%self.peers.len();let offset=544+i*80;
                reply[offset..offset+16].copy_from_slice(&self.peers[index].0);
                reply[offset+16..offset+80].copy_from_slice(&self.peers[index].1);
            }
            self.cursor=(self.cursor+count)%self.peers.len();
        }
        let count=usize::from(reply[28]);if count!=self.last_reply_count{
            let reason=if count>0{"nonempty"}else if before_filter>0&&self.peers.is_empty(){"peer_age_expired"}else if self.peers_at.elapsed()>Duration::from_secs(2){"snapshot_age_expired"}else{"cached_empty"};
            self.trace_roster(serde_json::json!({"kind":"native_reply_roster","reason":reason,"count":count,"cached_peers":self.peers.len(),"filtered_peers":before_filter-self.peers.len(),"snapshot_age_ms":self.peers_at.elapsed().as_millis().min(60000),"projection":self.last_projection.as_ref().map(|(at,v)|serde_json::json!({"age_ms":at.elapsed().as_millis().min(60000),"event":v}))}));self.last_reply_count=count;
        }
        reply
    }
}
pub struct Runtime {
    shared:Arc<Mutex<MovementState>>,
    cancellation:Arc<AtomicU64>,
    fixture_runner:Option<MovementRunner>,
    pub exchange:Arc<Mutex<Exchange>>,
    pub consumer:LocalConsumer,
    binding:Option<Binding>,
    statuses:std::collections::BTreeMap<String,String>,
    pub recovery_after:Instant,
}
impl Runtime {
    pub fn enable_live(&mut self){self.shared.lock().unwrap().live_enabled=true;}
    pub fn bind_live_scope(&self,scope:crate::protocol_g3_live::wire::Scope){self.shared.lock().unwrap().live_scope=Some(scope);}
    pub fn new(journal:Journal,exchange:Arc<Mutex<Exchange>>)->Self {
        Self{shared:Arc::new(Mutex::new(MovementState::default())),cancellation:Arc::new(AtomicU64::new(1)),fixture_runner:None,exchange,consumer:LocalConsumer::new(journal),binding:None,
            statuses:Default::default(),recovery_after:Instant::now()}
    }
    pub fn retire(&mut self) {
        self.shared.lock().unwrap().grant=None;
        self.exchange.lock().unwrap().pause();
        if self.binding.take().is_some(){let _=self.consumer.stop();}
    }
    pub fn pause(&mut self){self.shared.lock().unwrap().grant=None;self.exchange.lock().unwrap().pause();}
    pub fn trusted_depth(&self,principal:&str,session:&str,world:&str)->Option<f64>{
        let exchange=self.exchange.lock().unwrap();
        let shared=self.shared.lock().unwrap();if shared.grant.as_ref().is_none_or(|grant|!grant.active())||self.binding.as_ref().is_none_or(|b|!exchange.live_context(b.context)){return None;}
        let (_,ghosts)=shared.last_ghosts.as_ref()?;
        if shared.ghost_times.get(session)?.elapsed()>Duration::from_millis(500){return None;}
        let ghost=ghosts.iter().find(|g|g.principal_id.as_deref()==Some(principal)&&g.game_session_id.as_deref()==Some(session)&&g.world_token.as_deref()==Some(world))?;
        let frame=ghost.frames.as_ref()?.last()?;frame.y_milli.as_ref()?.parse::<i32>().ok().map(|y|y as f64/1000.)
    }
    pub fn trusted_ghost_actor(&self,principal:&str,session:&str,world:&str)->Option<String>{
        self.trusted_depth(principal,session,world)?;
        Some(Sha256::digest(serde_json::to_vec(&(principal,session,world)).ok()?)[..16].iter().map(|b|format!("{b:02x}")).collect())
    }
    /// Called only by accepted Core after matching service/Start/native-playing validation.
    pub fn authorize(&mut self,binding:Binding,token:Zeroizing<String>,pid:crate::local_owner::Id,until:Instant,http:reqwest::Client)->Result<(),String>{
        self.authorize_at_generation(binding,token,pid,until,http,self.cancellation.load(Ordering::Acquire))
    }
    pub fn authorize_recovered(&mut self,binding:Binding,token:Zeroizing<String>,pid:crate::local_owner::Id,until:Instant,http:reqwest::Client,generation:u64,ownership_generation:u32)->Result<(),String>{
        if self.cancellation.load(Ordering::Acquire)!=generation||until<=Instant::now(){return Err("Expired recovered gameplay authority".into())}
        if self.binding.as_ref().is_some_and(|old|old.context!=binding.context){self.retire();}
        self.consumer.bind_recovered(binding.clone(),ownership_generation)?;
        self.authorize_at_generation(binding,token,pid,until,http,generation)
    }
    /// LC1's validated admission may name a newly created world or a new
    /// connection to the retained world. Recovery generations are per world.
    pub fn authorize_lifecycle(&mut self,binding:Binding,token:Zeroizing<String>,pid:crate::local_owner::Id,until:Instant,http:reqwest::Client,generation:u64,ownership_generation:u32)->Result<(),String>{
        let result=(||{
            if self.cancellation.load(Ordering::Acquire)!=generation||until<=Instant::now()||ownership_generation==0{return Err("Expired recovered gameplay authority".into())}
            if self.consumer.lifecycle_owner_replacement(&binding,ownership_generation)?{
                // Cancel the old socket/grant and queued pose before rebinding.
                // Same native-context sequence floors remain monotonic.
                self.pause();self.consumer.bind_lifecycle_owner(binding.clone(),ownership_generation)?;
                self.authorize_at_generation(binding,token,pid,until,http,generation)
            }else if self.consumer.retained_world()!=binding.scope.world_token.as_deref(){let initial=binding.clone();self.authorize_at_generation(binding,token,pid,until,http,generation)?;self.consumer.bind_recovered(initial,ownership_generation)}else{self.authorize_recovered(binding,token,pid,until,http,generation,ownership_generation)}
        })();if result.is_err(){self.pause();}result
    }
    pub fn authorize_at_generation(&mut self,binding:Binding,token:Zeroizing<String>,pid:crate::local_owner::Id,until:Instant,http:reqwest::Client,generation:u64)->Result<(),String>{
        if self.cancellation.load(Ordering::Acquire)!=generation{return Err("Retired gameplay generation".into());}
        if until<=Instant::now(){self.pause();return Err("Expired gameplay authority".into());}
        if self.binding.as_ref().is_some_and(|old|old.context!=binding.context){self.retire();}
        self.consumer.bind(binding.clone())?;
        let fence={let mut exchange=self.exchange.lock().unwrap();
            if self.binding.is_none(){exchange.sequence=0;}
            exchange.arm(pid,binding.context,until);exchange.fence.clone()};
        let serial=fence.load(Ordering::Acquire);
        self.shared.lock().unwrap().grant=Some(MovementGrant{binding:binding.clone(),token,pid,until,http,fence,serial,cancellation:self.cancellation.clone(),generation});        self.binding=Some(binding);Ok(())
    }
    /// Explicit fixture path; the public app runs a separate runner without Core's lock.
    pub async fn pump(&mut self)->Result<(),String>{
        let mut runner=self.fixture_runner.take().unwrap_or_else(||self.runner());
        let result=runner.pump().await;self.fixture_runner=Some(runner);result
    }
    pub fn runner(&self)->MovementRunner{let enabled=self.shared.lock().unwrap().live_enabled;MovementRunner{shared:self.shared.clone(),exchange:self.exchange.clone(),owner:None,binding:None,next:Instant::now(),sequence:0,live:if enabled{Some(Default::default())}else{None}}}
    pub fn bank_context(&self)->Option<crate::g3_bank::Context>{
        let grant=self.shared.lock().unwrap().grant.clone().filter(|g|g.active())?;
        let mut owner=HttpOwner::from_verified_client(grant.binding.clone(),grant.token.clone(),grant.fence.clone(),grant.http.clone()).ok()?;
        owner.guard(grant.cancellation.clone(),grant.generation,grant.until);
        Some(crate::g3_bank::Context{binding:grant.binding,consumer:self.consumer.clone(),owner,events:Some(self.exchange.clone()),items:false,item_players:serde_json::json!([])})
    }
    pub fn health_publisher(&self)->HealthPublisher{HealthPublisher{shared:self.shared.clone(),exchange:self.exchange.clone(),cancellation:self.cancellation.clone()}}
    pub fn take_live_renewal(&self,binding:&Binding,token:&str,pid:crate::local_owner::Id,generation:u64)->bool{self.health_publisher().take_live_renewal(binding,token,pid,generation)}
    pub fn cancellation(&mut self,fence:Arc<AtomicU64>){self.cancellation=fence;}
    pub fn reset_diagnostics(&mut self){let mut shared=self.shared.lock().unwrap();shared.failure=FailureSnapshot::default();shared.first_failure=FailureSnapshot::default();shared.authority_loss=None;shared.first_authority_loss=None;}
    pub fn view(&self)->serde_json::Value {
        let records:Vec<_>=self.consumer.journal.pending().filter(|p|self.binding.as_ref().is_some_and(|b|p.record.original.service_origin==b.service_origin&&p.record.original.service_id==b.service_id&&p.record.original.principal_id==b.principal_id)).map(|p|{
            let scope=serde_json::from_str::<wire::Scope>(&p.record.original.scope_json).ok();let selected=self.binding.as_ref();
            let current=selected.zip(scope.as_ref()).is_some_and(|(b,s)|p.record.original.service_origin==b.service_origin&&p.record.original.service_id==b.service_id&&p.record.original.principal_id==b.principal_id&&s.room_id==b.scope.room_id&&s.run_id==b.scope.run_id&&s.run_epoch==b.scope.run_epoch&&s.config_revision==b.scope.config_revision&&s.world_token==b.scope.world_token);
            let phase=format!("{:?}",p.record.phase);let state=if p.restored{"retained_unverified"}else{match p.record.phase{crate::g3_journal::Phase::AppliedSession=>"applied_session",crate::g3_journal::Phase::Refused=>"refused",crate::g3_journal::Phase::Uncertain|crate::g3_journal::Phase::EffectIntent=>"uncertain",_=>"pending"}};
            serde_json::json!({"operation_id":p.record.original.operation_id,"direction":p.record.original.direction,"amount":p.record.original.amount,"asset":if p.record.original.item.is_some(){"item"}else{"gold"},"state":state,"phase":phase,"restored":p.restored,"current_scope":current,"room_id":scope.as_ref().and_then(|s|s.room_id.clone()),"run_id":scope.and_then(|s|s.run_id),"requires_status":true,"server_status":self.statuses.get(&p.record.original.operation_id)})
        }).collect();
        let (playing,roster)={let x=self.exchange.lock().unwrap();(x.owner.is_some(),x.roster_diagnostics())};let shared=self.shared.lock().unwrap();
        serde_json::json!({"enabled":true,"native_playing":playing,"movement_roster":roster,"operations":records,"error":shared.error,"movement":shared.metrics,"movement_failure":shared.failure,"movement_first_failure":shared.first_failure,"movement_authority_loss":shared.authority_loss,"movement_first_authority_loss":shared.first_authority_loss,"saved_world_proof":false,"live_transport":{"enabled":shared.live_enabled,"scope_bound":shared.live_scope.is_some(),"grant_present":shared.grant.is_some(),"grant_active":shared.grant.as_ref().is_some_and(|g|g.active()),"attempts":shared.live_attempts,"failures":shared.live_failures,"last_error":shared.live_error,"last_receive_failure":shared.live_receive_failure,"nonempty_snapshots":shared.live_nonempty}})
    }
    pub fn fixture_peers(&self)->serde_json::Value{let shared=self.shared.lock().unwrap();serde_json::json!(shared.last_ghosts.as_ref().map(|(_,ghosts)|ghosts.iter().map(|g|serde_json::json!({"session":g.game_session_id,"world":g.world_token,"sequence":g.frames.as_ref().and_then(|f|f.last()).and_then(|f|f.sequence),"age_ms":g.game_session_id.as_ref().and_then(|id|shared.ghost_times.get(id)).map(|at|at.elapsed().as_millis().min(60000))})).collect::<Vec<_>>()).unwrap_or_default())}
    /// Read-only network status is displayed, never promoted to loaded-world proof.
    pub fn record_status(&mut self,operation:&wire::Operation)->Result<(),String>{
        self.consumer.journal.validate_status(operation)?;
        self.statuses.insert(operation.operation_id.clone().ok_or("Missing original status ID")?,operation.state.clone().ok_or("Missing original status state")?);Ok(())
    }
    pub fn prune_transfer_statuses(&mut self){self.statuses.retain(|id,_|self.consumer.journal.pending().any(|p|p.record.original.operation_id==*id));}
    pub fn record_item_status(&mut self,operation:&wire::ItemOperation)->Result<(),String>{self.consumer.journal.validate_item_status(operation)?;self.statuses.insert(operation.operation_id.clone().ok_or("Missing item status ID")?,operation.state.clone().ok_or("Missing item status state")?);Ok(())}
}
/// Trusted Core health lane only. No network or journal work under these locks.
#[derive(Clone)]
pub struct HealthPublisher{shared:Arc<Mutex<MovementState>>,exchange:Arc<Mutex<Exchange>>,cancellation:Arc<AtomicU64>}
impl HealthPublisher{
    pub fn take_live_renewal(&self,binding:&Binding,token:&str,pid:crate::local_owner::Id,generation:u64)->bool{
        let mut shared=self.shared.lock().unwrap();let Some(current)=shared.grant.as_ref().filter(|g|g.active())else{return false};
        if current.pid!=pid||current.generation!=generation||current.token.as_str()!=token||current.renewal_key()!=renewal_key(binding,pid,generation,current.serial){return false;}
        let key=current.renewal_key();let token=current.token.clone();
        let consumed=shared.live_renewal.take().is_some_and(|(requested,rejected)|requested==key&&rejected.as_str()==token.as_str());if consumed{shared.metrics.live_renewals_consumed=shared.metrics.live_renewals_consumed.saturating_add(1);}consumed
    }
    pub fn bind_live_scope(&self,scope:crate::protocol_g3_live::wire::Scope){self.shared.lock().unwrap().live_scope=Some(scope);}
    pub fn pause(&self){self.shared.lock().unwrap().grant=None;self.exchange.lock().unwrap().pause();}
    pub fn authorize(&self,binding:Binding,token:Zeroizing<String>,pid:crate::local_owner::Id,until:Instant,http:reqwest::Client,generation:u64)->Result<(),String>{
        if self.cancellation.load(Ordering::Acquire)!=generation{return Err("Retired gameplay generation".into());}
        if until<=Instant::now(){self.pause();return Err("Expired gameplay authority".into());}
        let fence={let mut exchange=self.exchange.lock().unwrap();exchange.arm(pid,binding.context,until);exchange.fence.clone()};
        let serial=fence.load(Ordering::Acquire);
        self.shared.lock().unwrap().grant=Some(MovementGrant{binding,token,pid,until,http,fence,serial,cancellation:self.cancellation.clone(),generation});Ok(())
    }
}
#[derive(Clone)]
pub(crate) struct MovementGrant{pub(crate) binding:Binding,pub(crate) token:Zeroizing<String>,pub(crate) pid:crate::local_owner::Id,pub(crate) until:Instant,pub(crate) http:reqwest::Client,fence:Arc<AtomicU64>,pub(crate) serial:u64,cancellation:Arc<AtomicU64>,pub(crate) generation:u64}
#[derive(Clone,Copy,serde::Serialize,Debug,PartialEq)]
#[serde(rename_all="snake_case")]
enum AuthorityLoss{Deadline,NativeFence,Cancellation}
impl MovementGrant{
    fn renewal_key(&self)->String{renewal_key(&self.binding,self.pid,self.generation,self.serial)}
    fn inactive_reason(&self)->Option<AuthorityLoss>{
        if Instant::now()>=self.until{Some(AuthorityLoss::Deadline)}
        else if self.fence.load(Ordering::Acquire)!=self.serial{Some(AuthorityLoss::NativeFence)}
        else if self.cancellation.load(Ordering::Acquire)!=self.generation{Some(AuthorityLoss::Cancellation)}
        else{None}
    }
    pub(crate) fn active(&self)->bool{self.inactive_reason().is_none()}
}
fn renewal_key(binding:&Binding,pid:crate::local_owner::Id,generation:u64,serial:u64)->String{serde_json::to_string(&(&binding.service_origin,&binding.service_id,&binding.service_epoch,&binding.session_epoch,&binding.principal_id,&binding.scope,binding.context,pid,generation,serial)).unwrap()}
#[derive(Default,Clone,serde::Serialize)]
struct MovementMetrics{receives:u64,max_receive_gap_ms:u64,publications:u64,max_publication_gap_ms:u64,missed:u64,publication_attempts:u64,peer_attempts:u64,live_connections:u64,live_reauthentications:u64,live_admission_renewal_retries:u64,live_renewal_requests:u64,live_renewals_consumed:u64,receive_gap_p50_ms:u64,receive_gap_p95_ms:u64,remote_age_p50_ms:u64,remote_age_p95_ms:u64}
#[derive(Default)]
pub(crate) struct MovementState{pub(crate) grant:Option<MovementGrant>,pub(crate) live_scope:Option<crate::protocol_g3_live::wire::Scope>,live_enabled:bool,live_renewal:Option<(String,Zeroizing<String>)>,pub(crate) live_attempts:u64,live_failures:u64,live_error:Option<String>,pub(crate) live_receive_failure:Option<serde_json::Value>,live_nonempty:u64,pub(crate) ghost_times:std::collections::BTreeMap<String,Instant>,last_ghosts:Option<(Instant,Vec<wire::Ghost>)>,error:Option<String>,failure:FailureSnapshot,first_failure:FailureSnapshot,authority_loss:Option<AuthorityLoss>,first_authority_loss:Option<AuthorityLoss>,stage:Option<FailureStage>,metrics:MovementMetrics,received_at:Option<Instant>,published_at:Option<Instant>,gap_histogram:Vec<u64>,age_histogram:Vec<u64>}
fn histogram(hist:&mut Vec<u64>,ms:u64)->(u64,u64){if hist.is_empty(){hist.resize(81,0);}let index=(ms/25).min(80) as usize;hist[index]=hist[index].saturating_add(1);let total=hist.iter().fold(0u64,|n,v|n.saturating_add(*v));let quantile=|percent:u64|{let target=total.saturating_mul(percent).div_ceil(100);let mut count=0u64;for(i,n)in hist.iter().enumerate(){count=count.saturating_add(*n);if count>=target{return (i as u64+1)*25;}}2025};(quantile(50),quantile(95))}
impl MovementState {
    fn live_failed(&mut self,error:&str){self.live_failures=self.live_failures.saturating_add(1);self.live_error=Some(match error{"CAPACITY"|"EXPIRED"|"FORBIDDEN"|"INVALID_MESSAGE"|"REAUTH_REQUIRED"|"RETIRED"|"STALE_EPOCH"|"UNAUTHENTICATED"|"No verified live scope"|"Live auth deadline"|"Live connect deadline"|"Live connection unavailable"|"Live control send deadline"|"Live generation exhausted"|"Live owner cancelled"|"Live peer closed"|"Live receive deadline"|"Live receive retired"|"Live send deadline"|"Live stale acknowledgement"|"Live upgrade deadline"|"Live upgrade refused"=>error,_=>"Live transport failure"}.into());if self.first_failure.count==0{self.first_failure.record(FailureStage::Publication,error);}self.failure.record(FailureStage::Publication,error);}
    pub(crate) fn request_live_renewal(&mut self,rejected:&MovementGrant)->bool{
        let Some(current)=self.grant.as_ref().filter(|g|g.active()&&g.renewal_key()==rejected.renewal_key()&&g.token.as_str()==rejected.token.as_str())else{return false};
        let request=(current.renewal_key(),current.token.clone());if self.live_renewal.as_ref().is_none_or(|old|old.0!=request.0||old.1.as_str()!=request.1.as_str()){self.live_renewal=Some(request);self.metrics.live_renewal_requests=self.metrics.live_renewal_requests.saturating_add(1);}true
    }
    pub(crate) fn live_admission_renewed(&mut self){self.metrics.live_admission_renewal_retries+=1;}
    pub(crate) fn live_sample_ages(&mut self,ages:Vec<u64>){for age in ages{(self.metrics.remote_age_p50_ms,self.metrics.remote_age_p95_ms)=histogram(&mut self.age_histogram,age);}}
    pub(crate) fn live_authenticated(&mut self,initial:bool){if initial{self.metrics.live_connections+=1;}else{self.metrics.live_reauthentications+=1;}}
    pub(crate) fn live_publication(&mut self){let at=Instant::now();self.metrics.publications+=1;self.metrics.publication_attempts+=1;if let Some(old)=self.published_at.replace(at){self.metrics.max_publication_gap_ms=self.metrics.max_publication_gap_ms.max(at.duration_since(old).as_millis() as u64);}}
    pub(crate) fn accept_ghosts(&mut self,ghosts:Vec<wire::Ghost>)->Instant {
        let now=Instant::now();if !ghosts.is_empty(){self.live_nonempty=self.live_nonempty.saturating_add(1);}self.ghost_times=ghosts.iter().filter_map(|g|g.game_session_id.as_ref().map(|id|(id.clone(),now))).collect();self.last_ghosts=Some((now,ghosts));self.error=None;
        self.metrics.receives+=1;if let Some(at)=self.received_at.replace(now){let gap=now.duration_since(at).as_millis() as u64;self.metrics.max_receive_gap_ms=self.metrics.max_receive_gap_ms.max(gap);(self.metrics.receive_gap_p50_ms,self.metrics.receive_gap_p95_ms)=histogram(&mut self.gap_histogram,gap);}
        now
    }
}
/// A single serialized movement lane. It never acquires the asynchronous Core lock.
/// Core publishes grants synchronously; the native listener and HTTP owner fence them.
pub struct MovementRunner{shared:Arc<Mutex<MovementState>>,exchange:Arc<Mutex<Exchange>>,owner:Option<HttpOwner>,binding:Option<([u8;16],u64,u64)>,next:Instant,sequence:u32,live:Option<crate::g3_live::Lane>}
fn base(binding:&Binding)->wire::Request{wire::Request{major:Some(1),revision:Some(protocol_g3::REVISION.into()),service_id:Some(binding.service_id.clone()),service_epoch:Some(binding.service_epoch.clone()),session_epoch:Some(binding.session_epoch.clone()),request_id:Some(uuid::Uuid::new_v4().to_string()),scope:Some(binding.scope.clone()),..Default::default()}}
impl MovementRunner{
    fn fail(&mut self,grant:&MovementGrant,error:&str){
        self.next=Instant::now()+Duration::from_millis(100);
        let transient=matches!(error,"G3 connection unavailable; retain original IDs"|"G3 response interrupted; retain original IDs"|"Late G3 response discarded"|"CAPACITY"|"G3 polling rate capacity"|"G3 owner rate capacity");
        let mut exchange=self.exchange.lock().unwrap();
        // A delayed old request cannot pause a newly selected owner.
        if (!transient||!grant.active())&&exchange.owner==Some((grant.pid,grant.binding.context))&&exchange.fence.load(Ordering::Acquire)==grant.serial{exchange.pause();}
        drop(exchange);let mut shared=self.shared.lock().unwrap();
        if shared.grant.as_ref().is_some_and(|current|current.serial==grant.serial&&current.generation==grant.generation){
            let stage=shared.stage.unwrap_or(FailureStage::Binding);
            if shared.first_failure.count==0{shared.first_failure.record(stage,error);}
            shared.failure.record(stage,error);shared.error=Some(error.into());shared.metrics.missed+=1;
        }
    }
    pub async fn pump(&mut self)->Result<(),String>{
        if let Some(live)=&mut self.live{let result=live.pump(self.shared.clone(),self.exchange.clone()).await;if let Err(error)=&result{self.shared.lock().unwrap().live_failed(error);}return result;}
        let grant=self.shared.lock().unwrap().grant.clone();
        let result=self.pump_inner().await;
        if let (Err(error),Some(grant))=(&result,grant){self.fail(&grant,error);}
        result
    }
    async fn pump_inner(&mut self)->Result<(),String>{
        if Instant::now()<self.next{return Ok(())}self.next=Instant::now()+Duration::from_millis(100);
        let Some(grant)=self.shared.lock().unwrap().grant.clone() else{self.owner=None;return Ok(())};
        self.shared.lock().unwrap().stage=Some(FailureStage::Binding);
        if let Some(reason)=grant.inactive_reason(){
            {let mut shared=self.shared.lock().unwrap();shared.authority_loss=Some(reason);shared.first_authority_loss.get_or_insert(reason);}
            self.fail(&grant,"Expired movement authority");return Ok(())
        }
        let changed=self.binding.as_ref().is_none_or(|(context,serial,generation)|*context!=grant.binding.context||*serial!=grant.serial||*generation!=grant.generation);
        if changed{self.owner=Some(HttpOwner::from_verified_client(grant.binding.clone(),grant.token.clone(),grant.fence.clone(),grant.http.clone())?);self.binding=Some((grant.binding.context,grant.serial,grant.generation));self.sequence=0;}
        let owner=self.owner.as_mut().unwrap();owner.guard(grant.cancellation.clone(),grant.generation,grant.until);owner.refresh_verified_token(grant.token.clone())?;
        let pose={let mut exchange=self.exchange.lock().unwrap();if exchange.owner!=Some((grant.pid,grant.binding.context))||!grant.active(){return Ok(())}exchange.pose.take()};        if let Some(pose)=pose {
            self.shared.lock().unwrap().stage=Some(FailureStage::Publication);
            let decoded=crate::g3_native_frame::decode(&pose,&grant.binding.context,self.sequence)?;
            let mut movement=base(&grant.binding);movement.movement=Some(wire::Movement{frames:Some(vec![decoded.clone()])});
            self.sequence=decoded.sequence.unwrap();let bytes=serde_json::to_vec(&movement).unwrap();
            {let mut shared=self.shared.lock().unwrap();shared.metrics.publication_attempts=shared.metrics.publication_attempts.saturating_add(1);}
            if let Err(error)=owner.call(bytes).await{return Err(error);}
            if !grant.active(){return Err("Movement authority retired after publication".into());}
            let now=Instant::now();let mut shared=self.shared.lock().unwrap();shared.metrics.publications+=1;
            if let Some(at)=shared.published_at.replace(now){shared.metrics.max_publication_gap_ms=shared.metrics.max_publication_gap_ms.max(now.duration_since(at).as_millis() as u64);}
        }
        let binding=&grant.binding;
        self.shared.lock().unwrap().stage=Some(FailureStage::Peers);
        let request=wire::Request{major:Some(1),revision:Some(protocol_g3::REVISION.into()),service_id:Some(binding.service_id.clone()),
            service_epoch:Some(binding.service_epoch.clone()),session_epoch:Some(binding.session_epoch.clone()),
            request_id:Some(uuid::Uuid::new_v4().to_string()),scope:Some(binding.scope.clone()),ghost_list:Some(wire::GhostList::default()),..Default::default()};
        {let mut shared=self.shared.lock().unwrap();shared.metrics.peer_attempts=shared.metrics.peer_attempts.saturating_add(1);}
        let result=owner.call(serde_json::to_vec(&request).unwrap()).await;
        let response=match result{Ok(response)=>response,Err(error)=>{return Err(error)}};
        let trusted_ghosts=response.ghosts.clone().unwrap_or_default();
        let mut peers=Vec::new();
        for ghost in response.ghosts.unwrap_or_default(){
            let principal=ghost.principal_id.as_deref().ok_or("Missing trusted ghost principal")?;
            let session=ghost.game_session_id.as_deref().ok_or("Missing trusted ghost session")?;
            let world=ghost.world_token.as_deref().ok_or("Missing trusted ghost world")?;
            if principal.is_empty()||session.is_empty()||world.is_empty(){return Err("Incomplete trusted ghost identity".into());}
            let digest=Sha256::digest(serde_json::to_vec(&(principal,session,world)).unwrap());
            let actor:[u8;16]=digest[..16].try_into().unwrap();
            let Some(pose)=ghost.frames.and_then(|frames|frames.last().cloned()) else {continue};
            let mut frame=[0u8;64];frame[..8].copy_from_slice(b"NTG3\x01\x00\x01\x00");frame[8..24].copy_from_slice(&binding.context);
            frame[24..28].copy_from_slice(&pose.sequence.ok_or("Missing ghost sequence")?.to_le_bytes());
            for (offset,value) in [(28,pose.x_milli),(32,pose.y_milli),(36,pose.vx_milli),(40,pose.vy_milli)]{
                let value=value.ok_or("Missing ghost coordinate")?;let number=value.parse::<i32>().map_err(|_|"Invalid ghost coordinate")?;
                if number.to_string()!=value{return Err("Noncanonical ghost coordinate".into());}
                frame[offset..offset+4].copy_from_slice(&number.to_le_bytes());
            }
            frame[44]=u8::from(pose.facing_left.unwrap_or(false));
            crate::g3_native_frame::decode(&frame,&binding.context,0)?;
            if peers.iter().any(|(prior,_)|prior==&actor){return Err("Duplicate trusted ghost actor".into());}
            peers.push((actor,frame));
        }
        if !grant.active(){return Err("Movement authority retired across await".into());}
        let mut exchange=self.exchange.lock().unwrap();
        if exchange.owner.is_none()||exchange.until<=Instant::now(){exchange.pause();return Err("Native authority retired during ghost request".into());}
        if !grant.active()||exchange.owner!=Some((grant.pid,grant.binding.context)){return Err("Movement owner changed across await".into());}
        exchange.peers=peers;exchange.peers_at=Instant::now();drop(exchange);
        let now=self.shared.lock().unwrap().accept_ghosts(trusted_ghosts);
        self.next=now+Duration::from_millis(100);Ok(())
    }
}
#[cfg(test)]
mod tests {
    #[test]fn bank_stream_revision_is_coalesced_context_and_retired_connection_fenced(){
        let mut e=Exchange::default();e.arm(nt2_local_ws::owner::Id::OsProcess(7),[1;16],Instant::now()+Duration::from_secs(5));
        let first=e.begin_bank_stream([1;16]).unwrap();e.bank_frame([1;16],first,"first",Some(9));e.bank_frame([1;16],first,"first",Some(7));assert_eq!(e.bank_signal([1;16]),Some((first,9)));
        let next=e.begin_bank_stream([1;16]).unwrap();assert!(e.bank_signal([1;16]).is_none());e.bank_frame([1;16],next,"next",Some(9));e.bank_frame([1;16],first,"first",Some(99));e.bank_frame([2;16],next,"next",Some(99));e.bank_frame([1;16],next,"wrong-connection",Some(99));assert_eq!(e.bank_signal([1;16]),Some((next,9)));
        let resync=e.bank_gap([1;16],next).unwrap();assert!(resync>next);assert!(e.bank_gap([1;16],next).is_none());assert!(e.bank_signal([1;16]).is_none());
        e.pause();e.bank_frame([1;16],next,"next",Some(100));assert!(e.bank_signal([1;16]).is_none());
    }
    use super::*;
    #[test]fn bank_history_labels_preserve_phase_and_never_claim_loaded_save_completion(){
        let root=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../spikes/noita-entry/.build/bank-isolation-client-20261026-01").join(uuid::Uuid::new_v4().to_string());std::fs::create_dir_all(&root).unwrap();let canonical=root.canonicalize().unwrap();let root=std::path::PathBuf::from(canonical.to_str().unwrap().trim_start_matches(r"\\?\"));let path=root.join("journal.jsonl");
        let mut runtime=Runtime::new(Journal::open(&path).unwrap(),Arc::new(Mutex::new(Exchange::default())));let binding=Binding{context:[1;16],service_origin:"https://fixture.invalid".into(),service_id:"service".into(),service_epoch:uuid::Uuid::new_v4().to_string(),session_epoch:uuid::Uuid::new_v4().to_string(),principal_id:uuid::Uuid::new_v4().to_string(),scope:wire::Scope{room_id:Some(uuid::Uuid::new_v4().to_string()),run_id:Some(uuid::Uuid::new_v4().to_string()),run_epoch:Some(uuid::Uuid::new_v4().to_string()),config_revision:Some(1),attachment_id:Some(uuid::Uuid::new_v4().to_string()),world_token:Some(uuid::Uuid::new_v4().to_string())}};
        runtime.binding=Some(binding.clone());runtime.consumer.bind(binding.clone()).unwrap();runtime.consumer.begin_gold("deposit",10,runtime.consumer.generation()).unwrap();let row=runtime.view()["operations"][0].clone();assert_eq!(row["phase"],"Requested");assert_eq!(row["current_scope"],true);let id=row["operation_id"].as_str().unwrap().to_owned();let original=runtime.consumer.journal.pending().next().unwrap().record.original;let digest=crate::g3_journal::operation_digest(&original).unwrap();runtime.consumer.journal.accept_operation(&wire::Operation{operation_id:Some(id.clone()),principal_id:Some(binding.principal_id.clone()),run_id:binding.scope.run_id.clone(),attachment_id:binding.scope.attachment_id.clone(),world_token:binding.scope.world_token.clone(),direction:Some("deposit".into()),amount:Some(10),payload_digest:Some(digest),delivery_id:Some(uuid::Uuid::new_v4().to_string()),state:Some("awaiting_local".into()),..Default::default()}).unwrap();let permit=runtime.consumer.journal.prepare_effect(&id).unwrap();runtime.consumer.journal.finish_session_effect(permit,true).unwrap();assert_eq!(runtime.view()["operations"][0]["state"],"applied_session");drop(runtime);
        let mut runtime=Runtime::new(Journal::open(&path).unwrap(),Arc::new(Mutex::new(Exchange::default())));runtime.binding=Some(binding.clone());assert_eq!(runtime.view()["operations"][0]["state"],"retained_unverified");assert_eq!(runtime.view()["operations"][0]["phase"],"AppliedSession");runtime.binding.as_mut().unwrap().scope.room_id=Some(uuid::Uuid::new_v4().to_string());assert_eq!(runtime.view()["operations"][0]["current_scope"],false);runtime.binding.as_mut().unwrap().principal_id=uuid::Uuid::new_v4().to_string();assert!(runtime.view()["operations"].as_array().unwrap().is_empty());drop(runtime);std::fs::remove_file(path).unwrap();std::fs::remove_dir(root).unwrap();
    }
    #[test]fn lc1_ghost_same_context_new_canonical_owner_rebinds_without_losing_journal(){
        let root=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../spikes/noita-entry/.build/lc1-ghost-recovery-client-20261020-01").join(format!("binding-{}",uuid::Uuid::new_v4()));std::fs::create_dir_all(&root).unwrap();let root=root.canonicalize().unwrap();let root=std::path::PathBuf::from(root.to_str().unwrap().trim_start_matches(r"\\?\"));let path=root.join("operations.jsonl");let exchange=Arc::new(Mutex::new(Exchange::default()));let mut runtime=Runtime::new(Journal::open(&path).unwrap(),exchange);let mut binding=Binding{context:[1;16],service_origin:"https://fixture.invalid".into(),service_id:"service".into(),service_epoch:"epoch".into(),session_epoch:"old-session".into(),principal_id:"account".into(),scope:wire::Scope{room_id:Some("room".into()),run_id:Some("run".into()),run_epoch:Some("run-epoch".into()),config_revision:Some(1),attachment_id:Some("old-attachment".into()),world_token:Some("world".into())}};
        let until=Instant::now()+Duration::from_secs(20);let http=reqwest::Client::new();for _ in 0..2{runtime.authorize_lifecycle(binding.clone(),Zeroizing::new("fixture-original".into()),nt2_local_ws::owner::Id::OsProcess(7),until,http.clone(),1,1).unwrap();}runtime.consumer.begin_gold("withdraw",10,runtime.consumer.generation()).unwrap();let original=std::fs::read(&path).unwrap();let old=runtime.shared.lock().unwrap().grant.clone().unwrap();let mut pose=[0u8;64];pose[..8].copy_from_slice(b"NTG3\x01\x00\x01\x00");pose[8..24].copy_from_slice(&binding.context);pose[24..28].copy_from_slice(&120u32.to_le_bytes());assert_eq!(runtime.exchange.lock().unwrap().transition(nt2_local_ws::owner::Id::OsProcess(7),&pose,true)[5],1);binding.session_epoch="new-session".into();binding.scope.attachment_id=Some("new-attachment".into());
        runtime.authorize_lifecycle(binding.clone(),Zeroizing::new("fixture-renewed".into()),nt2_local_ws::owner::Id::OsProcess(7),until,http.clone(),1,2).expect("Fresh validated canonical owner was rejected solely because the native context survived");
        runtime.authorize_lifecycle(binding.clone(),Zeroizing::new("fixture-renewed-again".into()),nt2_local_ws::owner::Id::OsProcess(7),until,http.clone(),1,2).unwrap();assert!(!old.active());assert!(runtime.exchange.lock().unwrap().pose.is_none());assert_eq!(runtime.exchange.lock().unwrap().transition(nt2_local_ws::owner::Id::OsProcess(7),&pose,true)[5],0);pose[24..28].copy_from_slice(&121u32.to_le_bytes());assert_eq!(runtime.exchange.lock().unwrap().transition(nt2_local_ws::owner::Id::OsProcess(7),&pose,true)[5],1);for field in 0..10{let mut bad=binding.clone();let mut owner_generation=3;match field{0=>owner_generation=2,1=>owner_generation=1,2=>bad.principal_id="other".into(),3=>bad.service_epoch="other".into(),4=>bad.scope.run_epoch=Some("other".into()),5=>bad.scope.world_token=Some("other".into()),6=>bad.scope.room_id=Some("other".into()),7=>bad.scope.config_revision=Some(2),_=>{}}if field!=8{bad.session_epoch="third-session".into();}if field!=9{bad.scope.attachment_id=Some("third-attachment".into());}assert!(runtime.authorize_lifecycle(bad,Zeroizing::new("fixture-refused".into()),nt2_local_ws::owner::Id::OsProcess(7),until,http.clone(),1,owner_generation).is_err(),"Changed field {field} escaped strict canonical rebind");assert!(runtime.shared.lock().unwrap().grant.is_none());runtime.authorize_lifecycle(binding.clone(),Zeroizing::new("fixture-current".into()),nt2_local_ws::owner::Id::OsProcess(7),until,http.clone(),1,2).unwrap();}assert_eq!(std::fs::read(&path).unwrap(),original);assert_eq!(runtime.consumer.journal.pending().count(),1);assert!(runtime.shared.lock().unwrap().grant.as_ref().is_some_and(|g|g.active()));let retired=runtime.shared.lock().unwrap().grant.clone().unwrap();runtime.cancellation.store(2,Ordering::Release);binding.context=[2;16];binding.session_epoch="reopened-session".into();binding.scope.attachment_id=Some("reopened-attachment".into());runtime.authorize_lifecycle(binding,Zeroizing::new("fixture-reopened".into()),nt2_local_ws::owner::Id::OsProcess(8),until,reqwest::Client::new(),2,3).unwrap();assert!(!retired.active());assert_eq!(runtime.exchange.lock().unwrap().transition(nt2_local_ws::owner::Id::OsProcess(7),&pose,true)[5],0);pose[8..24].fill(2);pose[24..28].copy_from_slice(&1u32.to_le_bytes());assert_eq!(runtime.exchange.lock().unwrap().transition(nt2_local_ws::owner::Id::OsProcess(8),&pose,true)[5],1);assert_eq!(std::fs::read(&path).unwrap(),original);assert_eq!(runtime.consumer.journal.pending().count(),1);
    }
    #[test]fn a3_roster_provenance_distinguishes_local_expiry_and_bounded_pause(){
        let mut x=Exchange::default();let context=[1;16];x.arm(nt2_local_ws::owner::Id::OsProcess(7),context,Instant::now()+Duration::from_secs(5));let mut poll=[0u8;64];poll[..8].copy_from_slice(b"NTG3\x01\0\x02\0");poll[8..24].copy_from_slice(&context);
        for peer_expired in [false,true]{x.peers=vec![([2;16],pose(context,1))];x.peers_at=Instant::now();x.peer_times.clear();x.note_projection(serde_json::json!({"kind":"test_projection","accepted_peers":1}));assert_eq!(x.transition(nt2_local_ws::owner::Id::OsProcess(7),&poll,true)[28],1);
            if peer_expired{x.peer_times.insert([2;16],Instant::now()-Duration::from_secs(3));}else{x.peers_at=Instant::now()-Duration::from_secs(3);}
            let reply=x.transition(nt2_local_ws::owner::Id::OsProcess(7),&poll,true);assert_eq!(reply[5],1);assert_eq!(reply[28],0);let event=&x.roster_trace.back().unwrap().1;assert_eq!(event["reason"],if peer_expired{"peer_age_expired"}else{"snapshot_age_expired"});
        }
        for n in 0..100{x.note_projection(serde_json::json!({"kind":"test_projection","accepted_peers":n%2}));}assert_eq!(x.roster_trace.len(),32);x.pause();assert_eq!(x.roster_trace.len(),32);assert_eq!(x.roster_trace.back().unwrap().1["kind"],"client_pause");assert_eq!(x.transition(nt2_local_ws::owner::Id::OsProcess(7),&poll,true)[5],0);
    }
    #[test]fn c2_snapshot_uses_renewed_health_and_refuses_late_end(){
        let exchange=Arc::new(Mutex::new(Exchange::default()));let shared=Arc::new(Mutex::new(MovementState::default()));
        let cancellation=Arc::new(AtomicU64::new(1));let publisher=HealthPublisher{shared:shared.clone(),exchange:exchange.clone(),cancellation:cancellation.clone()};
        let binding=Binding{context:[1;16],service_origin:"https://fixture.invalid".into(),service_id:"fixture".into(),service_epoch:"epoch".into(),session_epoch:"session".into(),principal_id:"principal".into(),scope:wire::Scope::default()};
        let scope=crate::protocol_g3_live::wire::Scope::default();publisher.bind_live_scope(scope.clone());
        publisher.authorize(binding.clone(),Zeroizing::new("old".into()),nt2_local_ws::owner::Id::OsProcess(7),Instant::now()+Duration::from_millis(20),reqwest::Client::new(),1).unwrap();
        let original=shared.lock().unwrap().grant.clone().unwrap();
        publisher.authorize(binding,Zeroizing::new("renewed".into()),nt2_local_ws::owner::Id::OsProcess(7),Instant::now()+Duration::from_secs(1),reqwest::Client::new(),1).unwrap();
        std::thread::sleep(Duration::from_millis(25));assert!(!original.active());
        crate::g3_live::commit_snapshot(&shared,&exchange,&original,&scope,vec![([2;16],pose([1;16],1))],vec![]).unwrap();assert_eq!(exchange.lock().unwrap().peers.len(),1);
        let origin=shared.lock().unwrap().grant.as_ref().unwrap().binding.service_origin.clone();shared.lock().unwrap().grant.as_mut().unwrap().binding.service_origin="https://other.invalid".into();
        assert!(crate::g3_live::commit_snapshot(&shared,&exchange,&original,&scope,vec![],vec![]).is_err());shared.lock().unwrap().grant.as_mut().unwrap().binding.service_origin=origin;
        cancellation.fetch_add(1,Ordering::AcqRel);
        assert!(crate::g3_live::commit_snapshot(&shared,&exchange,&original,&scope,vec![],vec![]).is_err());assert_eq!(exchange.lock().unwrap().peers.len(),1);
        publisher.pause();assert!(exchange.lock().unwrap().peers.is_empty());
    }
    #[test]fn c1_health_publication_preserves_sequence_and_refuses_retired_generation(){
        let exchange=Arc::new(Mutex::new(Exchange::default()));let shared=Arc::new(Mutex::new(MovementState::default()));
        let cancellation=Arc::new(AtomicU64::new(1));let publisher=HealthPublisher{shared:shared.clone(),exchange:exchange.clone(),cancellation:cancellation.clone()};
        let binding=Binding{context:[1;16],service_origin:"https://fixture.invalid".into(),service_id:"fixture".into(),service_epoch:"epoch".into(),session_epoch:"session".into(),principal_id:"principal".into(),scope:wire::Scope::default()};
        let until=Instant::now()+Duration::from_secs(1);
        publisher.authorize(binding.clone(),Zeroizing::new("old".into()),nt2_local_ws::owner::Id::OsProcess(7),until,reqwest::Client::new(),1).unwrap();
        let serial=exchange.lock().unwrap().fence.load(Ordering::Acquire);assert_eq!(exchange.lock().unwrap().transition(nt2_local_ws::owner::Id::OsProcess(7),&pose([1;16],1),true)[5],1);
        publisher.authorize(binding.clone(),Zeroizing::new("renewed".into()),nt2_local_ws::owner::Id::OsProcess(7),until,reqwest::Client::new(),1).unwrap();
        assert_eq!(exchange.lock().unwrap().sequence,1);assert_eq!(exchange.lock().unwrap().until,until);assert_eq!(exchange.lock().unwrap().fence.load(Ordering::Acquire),serial);
        assert_eq!(shared.lock().unwrap().grant.as_ref().unwrap().token.as_str(),"renewed");
        cancellation.fetch_add(1,Ordering::AcqRel);
        assert!(publisher.authorize(binding,Zeroizing::new("late".into()),nt2_local_ws::owner::Id::OsProcess(7),until,reqwest::Client::new(),1).is_err());
        assert!(!shared.lock().unwrap().grant.as_ref().unwrap().active());
        assert_eq!(shared.lock().unwrap().grant.as_ref().unwrap().token.as_str(),"renewed");
    }
    fn pose(context:[u8;16],sequence:u32)->[u8;64]{let mut frame=[0;64];frame[..8].copy_from_slice(b"NTG3\x01\x00\x01\x00");
        frame[8..24].copy_from_slice(&context);frame[24..28].copy_from_slice(&sequence.to_le_bytes());frame}
    #[test]
    fn verified_owner_generation_lease_pause_and_frame_fences(){
        let mut exchange=Exchange::default();let frame=pose([1;16],1);
        assert_eq!(exchange.transition(nt2_local_ws::owner::Id::OsProcess(7),&frame,true)[5],0);
        exchange.arm(nt2_local_ws::owner::Id::OsProcess(7),[1;16],Instant::now()+Duration::from_secs(1));
        assert!(exchange.live_context([1;16]));assert!(!exchange.live_context([2;16]));
        let generation=exchange.fence.load(Ordering::Acquire);
        assert_eq!(exchange.transition(nt2_local_ws::owner::Id::OsProcess(8),&frame,false)[5],0);assert!(exchange.owner.is_some());
        assert_eq!(exchange.transition(nt2_local_ws::owner::Id::OsProcess(7),&pose([2;16],1),false)[5],0);assert!(exchange.owner.is_some());
        assert_eq!(exchange.transition(nt2_local_ws::owner::Id::OsProcess(7),&frame,true)[5],1);
        assert_eq!(exchange.transition(nt2_local_ws::owner::Id::OsProcess(7),&frame,true)[5],0);
        let mut malformed=pose([1;16],2);malformed[63]=1;assert_eq!(exchange.transition(nt2_local_ws::owner::Id::OsProcess(7),&malformed,true)[5],0);
        assert_eq!(exchange.transition(nt2_local_ws::owner::Id::OsProcess(7),&pose([1;16],2),false)[5],0);
        assert!(exchange.owner.is_none());assert!(exchange.pose.is_none());assert!(exchange.fence.load(Ordering::Acquire)>generation);
        assert!(!exchange.live_context([1;16]));
        exchange.arm(nt2_local_ws::owner::Id::OsProcess(7),[1;16],Instant::now()+Duration::from_secs(1));
        assert_eq!(exchange.transition(nt2_local_ws::owner::Id::OsProcess(7),&frame,true)[5],0); // pause cannot reset sequence history
        assert_eq!(exchange.transition(nt2_local_ws::owner::Id::OsProcess(7),&pose([1;16],2),true)[5],1);
        exchange.until=Instant::now();assert_eq!(exchange.transition(nt2_local_ws::owner::Id::OsProcess(7),&pose([1;16],3),true)[5],0);assert!(exchange.owner.is_none());
        assert!(!exchange.live_context([1;16]));
    }
    #[test]
    fn bounded_peer_batches_fairness_freshness_and_terminal_cleanup(){
        let mut exchange=Exchange::default();exchange.arm(nt2_local_ws::owner::Id::OsProcess(7),[1;16],Instant::now()+Duration::from_secs(2));
        exchange.peers=(1u8..=32).map(|id|([id;16],pose([1;16],id.into()))).collect();exchange.peers_at=Instant::now();
        let mut poll=pose([1;16],0);poll[6]=2;
        let mut seen=std::collections::BTreeSet::new();for _ in 0..8{let reply=exchange.transition(nt2_local_ws::owner::Id::OsProcess(7),&poll,true);assert_eq!(reply[6],4);
            assert_eq!(reply[28],32);for i in 0..4{seen.insert(reply[544+i*80]);}}
        assert_eq!(seen.len(),32);
        exchange.peers_at=Instant::now()-Duration::from_secs(1);let gap=exchange.transition(nt2_local_ws::owner::Id::OsProcess(7),&poll,true);assert_eq!(gap[6],0);assert_eq!(gap[28],32);
        exchange.peers.clear();let removal=exchange.transition(nt2_local_ws::owner::Id::OsProcess(7),&poll,true);assert_eq!(removal[28],0);
        exchange.pause();assert_eq!(exchange.transition(nt2_local_ws::owner::Id::OsProcess(7),&poll,true)[5],0);assert!(exchange.peers.is_empty());
    }
    #[test]
    fn transient_gap_never_renews_authority_and_old_failure_cannot_pause_new_owner(){
        let exchange=Arc::new(Mutex::new(Exchange::default()));let until=Instant::now()+Duration::from_secs(1);
        exchange.lock().unwrap().arm(nt2_local_ws::owner::Id::OsProcess(7),[1;16],until);
        let fence=exchange.lock().unwrap().fence.clone();let cancellation=Arc::new(AtomicU64::new(1));
        let grant=MovementGrant{binding:Binding{context:[1;16],service_origin:"https://fixture.invalid".into(),service_id:"fixture".into(),service_epoch:"epoch".into(),session_epoch:"session".into(),principal_id:"principal".into(),scope:wire::Scope::default()},token:Zeroizing::new("fixture-only".into()),pid:nt2_local_ws::owner::Id::OsProcess(7),until,http:reqwest::Client::new(),serial:fence.load(Ordering::Acquire),fence,cancellation:cancellation.clone(),generation:1};
        let shared=Arc::new(Mutex::new(MovementState::default()));shared.lock().unwrap().grant=Some(grant.clone());
        let mut runner=MovementRunner{shared,exchange:exchange.clone(),owner:None,binding:None,next:Instant::now(),sequence:0,live:None};
        runner.fail(&grant,"G3 connection unavailable; retain original IDs");
        let captured=serde_json::to_value(&runner.shared.lock().unwrap().failure).unwrap();
        assert_eq!(captured["count"],1);assert_eq!(captured["category"],"connection_unavailable");
        let first=serde_json::to_value(&runner.shared.lock().unwrap().first_failure).unwrap();
        assert_eq!(first,captured);
        runner.shared.lock().unwrap().accept_ghosts(vec![]); // production accepted-reply commit
        assert!(runner.shared.lock().unwrap().error.is_none());
        assert_eq!(serde_json::to_value(&runner.shared.lock().unwrap().failure).unwrap(),captured);
        assert!(exchange.lock().unwrap().live_context([1;16]));assert_eq!(exchange.lock().unwrap().until,until);
        cancellation.fetch_add(1,Ordering::AcqRel);runner.fail(&grant,"Late G3 response discarded");assert!(exchange.lock().unwrap().owner.is_none());
        assert_eq!(serde_json::to_value(&runner.shared.lock().unwrap().first_failure).unwrap(),first);
        exchange.lock().unwrap().arm(nt2_local_ws::owner::Id::OsProcess(9),[2;16],until);runner.fail(&grant,"FORBIDDEN");assert!(exchange.lock().unwrap().live_context([2;16]));
    }
    #[test]
    fn movement_grant_reports_exact_lost_guard_and_retains_first_failure(){
        let exchange=Arc::new(Mutex::new(Exchange::default()));let until=Instant::now()+Duration::from_secs(5);
        exchange.lock().unwrap().arm(nt2_local_ws::owner::Id::OsProcess(7),[1;16],until);
        let fence=exchange.lock().unwrap().fence.clone();let cancellation=Arc::new(AtomicU64::new(1));
        let grant=MovementGrant{binding:Binding{context:[1;16],service_origin:"https://fixture.invalid".into(),service_id:"fixture".into(),service_epoch:"epoch".into(),session_epoch:"session".into(),principal_id:"principal".into(),scope:wire::Scope::default()},token:Zeroizing::new("fixture-only".into()),pid:nt2_local_ws::owner::Id::OsProcess(7),until,http:reqwest::Client::new(),serial:fence.load(Ordering::Acquire),fence,cancellation:cancellation.clone(),generation:1};
        assert!(grant.active());assert_eq!(grant.inactive_reason(),None);
        let mut cancelled=grant.clone();cancelled.cancellation=Arc::new(AtomicU64::new(2));assert_eq!(cancelled.inactive_reason(),Some(AuthorityLoss::Cancellation));assert!(!cancelled.active());
        let mut expired=grant.clone();expired.until=Instant::now();assert_eq!(expired.inactive_reason(),Some(AuthorityLoss::Deadline));assert!(!expired.active());
        let shared=Arc::new(Mutex::new(MovementState::default()));shared.lock().unwrap().grant=Some(grant.clone());shared.lock().unwrap().stage=Some(FailureStage::Publication);
        let mut runner=MovementRunner{shared:shared.clone(),exchange:exchange.clone(),owner:None,binding:None,next:Instant::now(),sequence:0,live:None};
        // The real failure handler fences the native owner after a refusal.
        runner.fail(&grant,"FORBIDDEN");assert_eq!(grant.inactive_reason(),Some(AuthorityLoss::NativeFence));assert!(exchange.lock().unwrap().owner.is_none());
        runner.next=Instant::now();tauri::async_runtime::block_on(runner.pump()).unwrap();
        let state=shared.lock().unwrap();let first=serde_json::to_value(&state.first_failure).unwrap();let last=serde_json::to_value(&state.failure).unwrap();
        assert_eq!(first,serde_json::json!({"count":1,"stage":"publication","category":"forbidden"}));
        assert_eq!(last,serde_json::json!({"count":2,"stage":"binding","category":"expired_authority"}));
        assert_eq!(serde_json::to_value(state.authority_loss).unwrap(),"native_fence");
        assert_eq!(state.first_authority_loss,Some(AuthorityLoss::NativeFence));
        assert_eq!(state.metrics.publication_attempts,0);assert_eq!(state.metrics.peer_attempts,0);
        drop(state);shared.lock().unwrap().grant.as_mut().unwrap().until=Instant::now();runner.next=Instant::now();tauri::async_runtime::block_on(runner.pump()).unwrap();
        assert_eq!(shared.lock().unwrap().authority_loss,Some(AuthorityLoss::Deadline));assert_eq!(shared.lock().unwrap().first_authority_loss,Some(AuthorityLoss::NativeFence));
    }
    #[test]
    fn diagnostic_snapshot_has_only_fixed_values_and_saturates(){
        let mut snapshot=FailureSnapshot::default();
        snapshot.record(FailureStage::Peers,"secret-token principal/context E:/private/path");
        let value=serde_json::to_value(&snapshot).unwrap();
        assert_eq!(value,serde_json::json!({"count":1,"stage":"peers","category":"refused"}));
        snapshot.count=u32::MAX;snapshot.record(FailureStage::Publication,"STALE_EPOCH");
        assert_eq!(snapshot.count,u32::MAX);
        assert_eq!(serde_json::to_value(snapshot).unwrap()["category"],"stale_epoch");
        for (code,category) in [("STALE_REVISION","stale_revision"),("INVALID_STATE","invalid_state"),("FORBIDDEN","forbidden"),("INVALID_MESSAGE","invalid_message"),("INCOMPATIBLE_GAME","incompatible_game")] {
            let mut snapshot=FailureSnapshot::default();snapshot.record(FailureStage::Game,code);
            assert_eq!(serde_json::to_value(snapshot).unwrap()["category"],category);
        }
    }
    #[test]
    fn diagnostic_movement_reset_clears_only_failure_history(){
        let dir=std::env::temp_dir().join(format!("nt2-diag-{}",uuid::Uuid::new_v4()));std::fs::create_dir(&dir).unwrap();
        let exchange=Arc::new(Mutex::new(Exchange::default()));
        let mut runtime=Runtime::new(Journal::open(&dir.join("journal.jsonl")).unwrap(),exchange.clone());
        let until=Instant::now()+Duration::from_secs(1);exchange.lock().unwrap().arm(nt2_local_ws::owner::Id::OsProcess(7),[1;16],until);
        runtime.shared.lock().unwrap().failure.record(FailureStage::Peers,"INVALID_STATE");
        runtime.shared.lock().unwrap().first_failure.record(FailureStage::Peers,"INVALID_STATE");runtime.shared.lock().unwrap().authority_loss=Some(AuthorityLoss::NativeFence);runtime.shared.lock().unwrap().first_authority_loss=Some(AuthorityLoss::NativeFence);
        runtime.shared.lock().unwrap().accept_ghosts(vec![]);
        let received_at=runtime.shared.lock().unwrap().received_at;
        assert_eq!(runtime.view()["movement_failure"]["count"],1);
        runtime.pause();assert_eq!(runtime.view()["movement_failure"]["count"],1);
        runtime.reset_diagnostics();assert_eq!(runtime.view()["movement_failure"]["count"],0);
        assert_eq!(runtime.view()["movement_first_failure"]["count"],0);assert!(runtime.view()["movement_authority_loss"].is_null());
        assert!(runtime.view()["movement_first_authority_loss"].is_null());
        assert_eq!(runtime.view()["movement"]["receives"],1); // App-history totals require capture deltas.
        assert_eq!(runtime.shared.lock().unwrap().received_at,received_at);
        assert!(exchange.lock().unwrap().owner.is_none());assert_eq!(exchange.lock().unwrap().until,until);
        drop(runtime);std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]fn cr2_trusted_depth_counts_remote_age_before_local_receipt(){
        let dir=std::env::temp_dir().join(format!("nt2-aged-{}",uuid::Uuid::new_v4()));std::fs::create_dir(&dir).unwrap();let(grant,shared,exchange)=cr1_live_fixture(crate::protocol_g3_live::wire::Scope::default());
        let mut runtime=Runtime::new(Journal::open(&dir.join("journal.jsonl")).unwrap(),exchange);runtime.shared=shared.clone();runtime.binding=Some(grant.binding.clone());
        shared.lock().unwrap().accept_ghosts(vec![wire::Ghost{principal_id:Some("peer".into()),game_session_id:Some("peer-session".into()),world_token:Some("peer-world".into()),frames:Some(vec![wire::MovementFrame{y_milli:Some("2500".into()),..Default::default()}]),..Default::default()}]);
        shared.lock().unwrap().ghost_times.insert("peer-session".into(),Instant::now()-Duration::from_millis(600));assert_eq!(runtime.trusted_depth("peer","peer-session","peer-world"),None);
        shared.lock().unwrap().ghost_times.insert("peer-session".into(),Instant::now()-Duration::from_millis(100));assert_eq!(runtime.trusted_depth("peer","peer-session","peer-world"),Some(2.5));
        drop(runtime);drop(shared);std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]fn cr2_correction_live_renewal_is_coalesced_and_owner_scoped(){
        let(grant,shared,exchange)=cr1_live_fixture(crate::protocol_g3_live::wire::Scope::default());let publisher=HealthPublisher{shared:shared.clone(),exchange,cancellation:grant.cancellation.clone()};
        assert!(shared.lock().unwrap().request_live_renewal(&grant));assert!(shared.lock().unwrap().request_live_renewal(&grant));
        let mut other=grant.binding.clone();other.service_epoch="replacement".into();assert!(!publisher.take_live_renewal(&other,grant.token.as_str(),grant.pid,grant.generation));
        assert!(publisher.take_live_renewal(&grant.binding,grant.token.as_str(),grant.pid,grant.generation));assert!(!publisher.take_live_renewal(&grant.binding,grant.token.as_str(),grant.pid,grant.generation));
        assert!(shared.lock().unwrap().request_live_renewal(&grant));grant.cancellation.fetch_add(1,Ordering::AcqRel);assert!(!publisher.take_live_renewal(&grant.binding,grant.token.as_str(),grant.pid,grant.generation));assert!(!shared.lock().unwrap().request_live_renewal(&grant));
    }
    #[test]fn lc1_runtime_renewal_fences_delayed_denial_and_does_not_grant_authority(){
        let dir=std::env::temp_dir().join(format!("nt2-lc1-renewal-{}",uuid::Uuid::new_v4()));std::fs::create_dir(&dir).unwrap();
        let(grant,shared,exchange)=cr1_live_fixture(crate::protocol_g3_live::wire::Scope::default());let mut runtime=Runtime::new(Journal::open(&dir.join("journal.jsonl")).unwrap(),exchange.clone());runtime.shared=shared.clone();
        let until=exchange.lock().unwrap().until;let serial=exchange.lock().unwrap().fence.load(Ordering::Acquire);
        assert!(shared.lock().unwrap().request_live_renewal(&grant));assert!(shared.lock().unwrap().request_live_renewal(&grant));assert_eq!(shared.lock().unwrap().metrics.live_renewal_requests,1);assert!(runtime.take_live_renewal(&grant.binding,grant.token.as_str(),grant.pid,grant.generation));assert!(!runtime.take_live_renewal(&grant.binding,grant.token.as_str(),grant.pid,grant.generation));assert_eq!(shared.lock().unwrap().metrics.live_renewals_consumed,1);
        for change in 0..5{
            *shared.lock().unwrap()=MovementState{grant:Some(grant.clone()),..Default::default()};assert!(shared.lock().unwrap().request_live_renewal(&grant));
            {let mut s=shared.lock().unwrap();let g=s.grant.as_mut().unwrap();match change{0=>g.binding.scope.world_token=Some("replacement".into()),1=>g.binding.context=[8;16],2=>g.token=Zeroizing::new("replacement-token".into()),3=>g.pid=crate::local_owner::Id::os_process(999),_=>g.generation+=1}}
            assert!(!runtime.take_live_renewal(&grant.binding,grant.token.as_str(),grant.pid,grant.generation),"delayed old denial crossed scope {change}");assert!(!shared.lock().unwrap().request_live_renewal(&grant));
        }
        *shared.lock().unwrap()=MovementState{grant:Some(grant.clone()),..Default::default()};assert!(shared.lock().unwrap().request_live_renewal(&grant));runtime.pause();assert!(!runtime.take_live_renewal(&grant.binding,grant.token.as_str(),grant.pid,grant.generation));
        assert_eq!(exchange.lock().unwrap().until,until);assert_eq!(exchange.lock().unwrap().fence.load(Ordering::Acquire),serial+1);assert!(exchange.lock().unwrap().owner.is_none(),"renewal cannot release a terminal/offline world");
        let mut s=shared.lock().unwrap();s.live_failed("secret-token E:/private/path");assert_eq!(s.live_error.as_deref(),Some("Live transport failure"));drop(s);assert!(!runtime.view().to_string().contains("secret-token"));drop(runtime);std::fs::remove_dir_all(dir).unwrap();
    }
}


#[cfg(test)]
pub(crate) fn cr1_live_fixture(scope:crate::protocol_g3_live::wire::Scope)->(MovementGrant,Arc<Mutex<MovementState>>,Arc<Mutex<Exchange>>){
 let exchange=Arc::new(Mutex::new(Exchange::default()));let fence=exchange.lock().unwrap().fence.clone();
 let until=Instant::now()+Duration::from_secs(3600);let cancellation=Arc::new(AtomicU64::new(1));
 let grant=MovementGrant{binding:Binding{context:[1;16],service_origin:"https://127.0.0.1:9".into(),service_id:"fixture".into(),service_epoch:"epoch".into(),session_epoch:"session".into(),principal_id:"principal".into(),scope:wire::Scope::default()},token:Zeroizing::new("fixture-only".into()),pid:nt2_local_ws::owner::Id::OsProcess(7),until,http:reqwest::Client::new(),serial:fence.load(Ordering::Acquire),fence,cancellation,generation:1};
 {let mut x=exchange.lock().unwrap();x.owner=Some((grant.pid,grant.binding.context));x.until=until;}
 let shared=Arc::new(Mutex::new(MovementState::default()));{let mut s=shared.lock().unwrap();s.grant=Some(grant.clone());s.live_scope=Some(scope);}
 (grant,shared,exchange)
}
