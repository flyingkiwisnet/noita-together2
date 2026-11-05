//! Saved waiting consumer. Uses the existing mailbox and initial admission owner.
use super::*;
use nt2_desktop_g0::{startup_local as startup,waiting_local as journal,protocol_waiting::{self as protocol,wire as w}};
#[derive(Clone)]struct Loaded{scope:startup::Scope,journal:journal::Journal}
fn published_selector(bytes:&[u8],saved:&startup::WaitingMarker)->bool{
 let Ok(published)=startup::WaitingMarker::decode(bytes)else{return false};if published==*saved{return true}
 // A save can retain its early pending selector after local enrollment updates
 // the published one. Protected load_saved_selector verifies BOTH originals
 // and their identical claim before this alias can reach lookup2.
 if saved.kind!="nt2-pending"{return false}let mut promoted=saved.clone();promoted.kind="nt2-waiting".into();published==promoted
}
#[derive(Debug)]pub(super) struct Failure{pub(super) code:String,not_before:Option<Instant>}
impl From<String> for Failure{fn from(code:String)->Self{Self{code,not_before:None}}}
impl From<&str> for Failure{fn from(code:&str)->Self{code.to_string().into()}}
pub(super) struct Recovered{loaded:Loaded,session:Session,instance:String,validity:admission::CredentialLease}
#[derive(Default)]pub(super) struct Transport{
 loaded:Option<Loaded>,load:Option<tokio::task::JoinHandle<Result<Loaded,String>>>,recover:Option<tokio::task::JoinHandle<Result<Recovered,Failure>>>,
 pub(super) contact:Option<crate::pipe::rejoin::Contact>,key:String,nonce:u64,generation:u64,ticket:Option<lifecycle::Ticket>,next:Option<Instant>,setup_scope:Option<startup::Scope>,
}
impl Drop for Transport{fn drop(&mut self){if let Some(t)=&self.load{t.abort()}if let Some(t)=&self.recover{t.abort()}}}
#[derive(Default)]pub(super) struct RetirementRunner{key:String,task:Option<tokio::task::JoinHandle<Result<(),Failure>>>,next:Option<Instant>,pub(super) error:Option<String>}
impl Drop for RetirementRunner{fn drop(&mut self){if let Some(t)=&self.task{t.abort()}}}
impl<S:CredentialStore> Core<S>{
 async fn poll_waiting_retirement(&mut self) where S:'static{
  let Some(desktop)=self.desktop.clone()else{return};let key=json!([self.base,self.service,self.epoch,self.device,desktop.account,desktop.id,desktop.epoch]).to_string();
  if self.waiting_retirement.key!=key{self.waiting_retirement=Default::default();self.waiting_retirement.key=key;}
  if self.waiting_retirement.task.as_ref().is_some_and(|t|t.is_finished()){
   // Join only a completed task; never wait on network/storage from Core.
   if let Some(task)=self.waiting_retirement.task.take(){match task.await{Ok(Err(e))=>{self.waiting_retirement.error=Some(e.code);self.waiting_retirement.next=Some(e.not_before.unwrap_or_else(||Instant::now()+Duration::from_secs(2)));},_=>{self.waiting_retirement.error=None;}}}
  }
  if self.waiting_retirement.task.is_some()||self.waiting_retirement.next.is_some_and(|at|Instant::now()<at){return}
  self.waiting_retirement.next=Some(Instant::now()+Duration::from_secs(2));let store=self.store.clone();let writer=self.credential_writer.clone();let http=self.http.clone();let origin=self.base.clone();let service=self.service.clone();let epoch=self.epoch.clone();let device=self.device;let nonce=self.health_nonce.clone();let expected=nonce.load(Ordering::Acquire);
  self.waiting_retirement.task=Some(tokio::spawn(async move{retirement_step(store,writer,http,origin,service,epoch,device,desktop,nonce,expected).await}));
 }
 pub(super) fn publish_waiting_metadata(&self){
  let Some(marker)=self.waiting_startup_marker()else{return};let Some(game)=self.game.as_ref()else{return};let Some(p)=self.pending.lock().unwrap().clone()else{return};if p.owner!=Some(game.owner)||p.generation.as_ref().is_none_or(|(g,n)|g.load(Ordering::Acquire)!=*n)||!p.loaded||!p.accepted{return}
  let Ok(record)=marker.encode()else{return};let mut d=self.native_diagnostics.lock().unwrap();if d.rejoin.metadata.as_ref().is_some_and(|(pid,context,m)|(*pid,*context)==game.owner&&nt2_desktop_g0::rejoin_local::Association::decode(m).is_ok()){return}
  d.rejoin.metadata=Some((game.owner.0,game.owner.1,record));
 }
 pub(super) fn waiting_local_world(&self)->Option<uuid::Uuid>{Some(self.waiting.loaded.as_ref()?.journal.marker.local_world)}
 pub(super) fn waiting_world(&self)->Option<&str>{if self.lc1{return self.world.ctx.as_ref().map(|c|c.world.as_str())}let l=self.waiting.loaded.as_ref()?;l.journal.intent.as_ref().and_then(|i|i.world_token.as_deref()).or_else(||l.journal.operation.as_ref().and_then(|op|op.request.pending_recover.as_ref()).and_then(|r|r.observation.as_ref()).and_then(|o|o.world_token.as_deref()))}
 pub(super) fn waiting_authority_valid(&self,p:&handoff::Pending)->bool{
  let Some(c)=self.waiting.contact.as_ref()else{return false};let Some(l)=self.waiting.loaded.as_ref()else{return false};
  !self.native_revoked&&!l.journal.terminal&&l.scope.origin==self.base&&l.scope.service==self.service&&self.desktop.as_ref().is_some_and(|d|d.account==l.scope.account)&&p.owner==Some((c.pid,c.context))&&p.run==l.scope.run&&p.epoch==l.scope.run_epoch&&p.config==l.scope.config&&p.actual_seed==Some(c.actual)&&p.generation.as_ref().is_some_and(|(g,n)|Arc::ptr_eq(g,&self.generation)&&g.load(Ordering::Acquire)==*n)
 }
 pub(super) fn select_waiting_for_retirement(&mut self,scope:&startup::Scope,marker:&startup::WaitingMarker)->Result<(),String>{
  if scope.origin!=self.base||scope.service!=self.service||scope.device!=self.device||self.desktop.as_ref().is_none_or(|d|d.account!=scope.account){return Err("Waiting retirement belongs to another current issuer/account/device".into())}
  let _guard=self.credential_writer.lock().unwrap();let adapter=rejoin::ResumeStore(self.store.clone());if startup::recall(&adapter,&scope.origin,&scope.service,&scope.account,scope.device,marker)?.is_none(){return Ok(())}let mut j=journal::load_saved_selector(&adapter,scope,marker)?;if j.is_none()&&marker.kind=="nt2-pending"{let record=nt2_desktop_g0::startup_pending::load(&adapter,scope,marker.local_world)?.ok_or("Pending original absent for retirement")?;if !record.ready(){return Err("Pending original proof unavailable for retirement".into())}let original=journal::Journal::new(marker.clone());journal::save(&adapter,scope,&original)?;j=Some(original)}if let Some(j)=j{self.waiting.loaded=Some(Loaded{scope:scope.clone(),journal:j})}Ok(())
 }
 pub(super) fn retire_waiting_local(&mut self)->Result<(),String>{
  let Some(mut l)=self.waiting.loaded.clone()else{return Ok(())};let _guard=self.credential_writer.lock().unwrap();let adapter=rejoin::ResumeStore(self.store.clone());
  if !l.journal.terminal{l.journal.terminal=true;let saved=if startup::recall(&adapter,&l.scope.origin,&l.scope.service,&l.scope.account,l.scope.device,&l.journal.marker)?.is_some(){journal::save(&adapter,&l.scope,&l.journal)}else{journal::save_retired(&adapter,&l.scope,&l.journal)};let retired=startup::retire(&adapter,&l.scope,&l.journal.marker);retired?;saved?;}
  journal::queue_retirement(&adapter,&l.scope,&l.journal.marker)?;self.waiting.loaded=Some(l);self.waiting.ticket=None;Ok(())
 }
 pub(super) async fn waiting_tick(&mut self)->Result<bool,String> where S:'static {
  self.poll_waiting_retirement().await;
  let Some(desktop)=self.desktop.clone()else{return Ok(false)};
  let (offered,lookup_context)={let n=self.native_diagnostics.lock().unwrap();(n.rejoin.waiting_offered.clone(),n.rejoin.waiting_context)};let Some((pid,marker))=offered else{return Ok(false)};
  if marker.kind=="nt2-provisional"{return Ok(false)}
  // Setup remains unentered while staging. After Save & Quit its published
  // marker still names the same durable world, but the new game has another
  // PID/context. The mailbox already fences a competing live native lease.
  // Select by exact marker here, then verify its protected scope before lookup2.
  let key=json!([self.base,self.service,self.epoch,desktop.account,desktop.id,desktop.epoch,self.device,pid,lookup_context,marker]).to_string();
  let setup_scope=if self.fresh_setup_owns_world()&&(self.waiting.contact.is_none()||self.waiting.key!=key){
   if !self.native_diagnostics.lock().unwrap().rejoin.metadata.as_ref().is_some_and(|(_,_,bytes)|published_selector(bytes,&marker)){return Ok(false)}
   let scope=self.pending.lock().unwrap().as_ref().and_then(|p|p.startup_bundle.as_ref().map(|b|b.scope().clone())).ok_or("Current setup bundle absent; world remains held")?;
   if !self.setup_matches_saved_waiting(&scope){return Err("Current setup scope differs; world remains held".into())}Some(scope)
  }else{None};
  if self.waiting.key!=key{if !self.waiting.key.is_empty(){self.cancel_health();self.lifecycle.lock().unwrap().retire_owner();self.generation.fetch_add(1,Ordering::AcqRel);self.game=None;*self.pending.lock().unwrap()=None;}self.waiting=Transport::default();self.waiting.key=key;self.waiting.setup_scope=setup_scope;self.waiting.nonce=self.health_nonce.load(Ordering::Acquire);self.waiting.generation=self.generation.load(Ordering::Acquire);}
  if self.native_diagnostics.lock().unwrap().rejoin.offline{
   if !self.native_revoked{self.lifecycle.lock().unwrap().offline();self.revoke_native();}if self.waiting.loaded.as_ref().is_some_and(|l|!l.journal.terminal){self.retire_waiting_local()?;}return Ok(true)
  }
  if self.waiting.load.as_ref().is_some_and(|t|t.is_finished()){
   let result=self.waiting.load.take().unwrap().await.map_err(|_|"Waiting lookup worker cancelled")?;match result{Ok(l)=>{
    if self.waiting.setup_scope.as_ref().is_some_and(|s|*s!=l.scope){return Err("Saved waiting world differs from current setup; world remains held".into())}
    if self.waiting.nonce!=self.health_nonce.load(Ordering::Acquire)||self.waiting.generation!=self.generation.load(Ordering::Acquire){return Err("Saved lookup owner retired; world remains held".into())}
    if self.pending.lock().unwrap().as_ref().is_some_and(|p|p.authority_until>Instant::now()&&p.owner.is_some_and(|owner|owner!=(pid,lookup_context.unwrap_or([0;16])))){return Err("Saved lookup conflicts with current live owner; world remains held".into())}
    let mut n=self.native_diagnostics.lock().unwrap();if n.rejoin.waiting_offered.as_ref()!=Some(&(pid,marker.clone()))||n.rejoin.waiting_context!=lookup_context||n.rejoin.offline||n.rejoin.terminal{return Ok(true)}self.waiting.loaded=Some(l);n.rejoin.lookup=2;
   },Err(e)=>{self.waiting.next=Some(Instant::now()+Duration::from_secs(2));return Err(e)}}
  }
  if self.waiting.loaded.is_none(){
   if self.waiting.load.is_none()&&self.waiting.next.is_none_or(|at|Instant::now()>=at){
    let store=self.store.clone();let writer=self.credential_writer.clone();let origin=self.base.clone();let service=self.service.clone();let device=self.device;let account=desktop.account.clone();let marker=marker.clone();
    self.waiting.load=Some(tokio::task::spawn_blocking(move||{let _guard=writer.lock().unwrap();let adapter=rejoin::ResumeStore(store);let parked=startup::recall(&adapter,&origin,&service,&account,device,&marker)?.ok_or("Saved waiting world missing or explicitly offline")?;let mut j=journal::load_saved_selector(&adapter,&parked.original,&marker)?;
     if marker.kind=="nt2-pending"&&j.as_ref().is_none_or(|j|j.intent.is_none()||j.proof.is_none()) {pending::require_source(&adapter,&parked.original,marker.local_world)?;if j.is_none(){let original=journal::Journal::new(marker.clone());journal::save(&adapter,&parked.original,&original)?;j=Some(original)} }
     let j=j.ok_or("Canonical waiting enrollment/proof was not saved")?;if marker.kind!="nt2-pending"&&!pending::has_source(&adapter,&parked.original,marker.local_world)?&&!j.ready()&&!(j.intent.is_some()&&j.proof.is_some()&&j.generation>0&&!j.terminal&&j.operation.as_ref().is_some_and(|op|op.request.recover.is_some())){return Err("Waiting enrollment remains uncertain; current outcome reconciliation required".into())}Ok(Loaded{scope:parked.original,journal:j})}));
   }return Ok(true)
  }
  let contact=self.native_diagnostics.lock().unwrap().rejoin.contact.clone();let Some(contact)=contact else{return Ok(true)};
  let l=self.waiting.loaded.as_ref().unwrap().clone();if contact.pid!=pid||contact.association!=*marker.local_world.as_bytes()||contact.actual.to_string()!=marker.actual_seed{return Err("Waiting contact differs from saved world".into())}
  if l.scope.service_epoch!=self.epoch{return Err("Waiting service epoch changed; world remains held".into())}
  if self.waiting.contact.as_ref()!=Some(&contact){
   if !self.replacement_lineage.block("saved waiting ownership recovery"){return Ok(true)}
   if self.resume_intent.association().is_some(){if self.parked_recovery.len()>=16{return Err("Parked recovery capacity reached".into())}self.persist_recovery()?;self.parked_recovery.push(rejoin::Parked{intent:std::mem::take(&mut self.resume_intent),transport:std::mem::take(&mut self.rejoin_transport)});}

   self.cancel_health();self.g3_retire();self.game=None;self.preparation=None;self.native_revoked=false;
   let generation=self.generation.fetch_add(1,Ordering::AcqRel)+1;self.waiting.generation=generation;self.waiting.nonce=self.health_nonce.load(Ordering::Acquire);self.waiting.contact=Some(contact.clone());
   let world=lifecycle::StableWorld{issuer:self.lifecycle_issuer().ok_or("Waiting issuer absent")?,room:l.scope.room.clone(),run:l.scope.run.clone(),participant:String::new(),world:l.journal.intent.as_ref().and_then(|i|i.world_token.clone()).unwrap_or_else(||marker.local_world.simple().to_string())};
   let mut coordinator=self.lifecycle.lock().unwrap();coordinator.saved(world.clone());self.waiting.ticket=coordinator.bind(lifecycle::Owner{world,native:(pid,contact.context),session:desktop.id.clone(),generation});drop(coordinator);
   let mut p=handoff::new(if marker.loot=="individual:0"{0}else{contact.actual.try_into().map_err(|_|"Waiting shared seed overflow")?},l.scope.run.clone(),l.scope.config);p.epoch=l.scope.run_epoch.clone();p.owner=Some((pid,contact.context));p.actual_seed=Some(contact.actual);p.world_policy=if marker.loot=="individual:0"{2}else if marker.loot.starts_with("shared:1:"){5}else{1};p.native_v2=true;p.accepted=true;p.generation=Some((self.generation.clone(),generation));p.authority_until=Instant::now()+Duration::from_secs(6);p.last_seen=Some(Instant::now());if let Some(source)=crate::pipe::entry_collection(&self.native_diagnostics,&contact){p.native_epoch=source.native_epoch;p.loaded=source.loaded;p.accepted=source.accepted;p.observation=source.observation;p.invalidated_observation=source.invalidated_observation;p.last_seen=source.last_seen;}*self.pending.lock().unwrap()=Some(p);
   self.native_diagnostics.lock().unwrap().rejoin.admitted=Some(contact.clone());
  }
  if self.waiting.recover.as_ref().is_some_and(|t|t.is_finished()){
   let result=self.waiting.recover.take().unwrap().await.map_err(|_|"Waiting recovery worker cancelled")?;let r=match result{Ok(r)=>r,Err(e)=>{self.waiting.next=Some(e.not_before.unwrap_or_else(||Instant::now()+Duration::from_secs(2)));return Err(e.code)}};
   if self.waiting.nonce!=self.health_nonce.load(Ordering::Acquire)||self.waiting.generation!=self.generation.load(Ordering::Acquire)||!self.waiting.ticket.as_ref().is_some_and(|t|self.lifecycle.lock().unwrap().accepts(t)){return Err("Late waiting owner retired".into())}
   let session=r.session;let k=admission::credential_key(&self.base,&self.service,&session)?;self.lifecycle.lock().unwrap().credentials.insert(k,r.validity);self.game=Some(Game{session,instance:r.instance,owner:(pid,contact.context),reported:0,report_retry:None,renew_retry:None,permit:None,entered:false,offered:false});self.waiting.loaded=Some(r.loaded);self.publish_health();
  }
  if self.waiting.nonce!=self.health_nonce.load(Ordering::Acquire)||self.waiting.generation!=self.generation.load(Ordering::Acquire){return Err("Waiting local owner retired".into())}
  if self.game.is_some(){self.publish_health();return Ok(true)}
  // Finite local collection lease only: release remains false until the existing
  // canonical initial admission sees actual later host Start.
  let local=self.pending.lock().unwrap().clone().ok_or("Waiting native owner absent")?;
  if local.last_seen.is_none_or(|at|at.elapsed()>Duration::from_secs(12))||local.owner!=Some((pid,contact.context))||local.generation.as_ref().is_none_or(|(g,n)|g.load(Ordering::Acquire)!=*n){return Err("Waiting native owner retired".into())}
  if let Some(p)=self.pending.lock().unwrap().as_mut(){p.authority_until=Instant::now()+Duration::from_secs(6);p.release=false;}
  let Some(report)=local.observation.clone().filter(|r|r.observed_at.elapsed()<Duration::from_secs(6)&&r.context==contact.context&&r.observation.loaded&&r.observation.complete&&r.observation.protected&&r.observation.actual_seed.as_deref()==Some(&marker.actual_seed))else{return Ok(true)};
  if self.waiting.recover.is_none()&&self.waiting.next.is_none_or(|at|Instant::now()>=at){
   let ticket=self.waiting.ticket.clone().ok_or("Waiting ticket absent")?;let store=self.store.clone();let writer=self.credential_writer.clone();let c=self.lifecycle.clone();let nonce=self.health_nonce.clone();let generation=self.generation.clone();let expected=(self.waiting.nonce,self.waiting.generation);let http=self.http.clone();let device=self.device;let contact=contact.clone();let game_version=self.info.as_ref().and_then(|i|i.compatibility.as_ref()).and_then(|c|c.game_version.clone()).ok_or("Waiting game compatibility missing")?;let capabilities=self.local_game_capabilities();
   self.waiting.recover=Some(tokio::spawn(async move{recover(store,writer,c,nonce,generation,expected,ticket,http,desktop,device,l,contact,report,game_version,capabilities).await}));
  }Ok(true)
 }
}
fn current(c:&Arc<Mutex<lifecycle::Coordinator>>,n:&AtomicU64,g:&AtomicU64,expected:(u64,u64),t:&lifecycle::Ticket)->bool{n.load(Ordering::Acquire)==expected.0&&g.load(Ordering::Acquire)==expected.1&&c.lock().unwrap().accepts(t)}
pub(super) async fn rpc(http:&reqwest::Client,s:&startup::Scope,desktop:&Session,r:&w::Request)->Result<w::Response,Failure>{
 protocol::validate("Request",r)?;let url=format!("{}{}",s.origin,protocol::ENDPOINT);let body=zeroize::Zeroizing::new(serde_json::to_vec(r).map_err(|_|"Waiting request encoding")?);
 tokio::time::timeout(Duration::from_secs(3),async{let mut response=http.post(&url).bearer_auth(desktop.access.as_str()).header("content-type","application/json").body(body.to_vec()).send().await.map_err(|_|"Waiting transport uncertain")?;if response.url().as_str()!=url||!response.status().is_success(){return Err("Waiting HTTP refused; original retained".into())}let mut bytes=zeroize::Zeroizing::new(Vec::new());while let Some(chunk)=response.chunk().await.map_err(|_|"Waiting reply uncertain")?{if bytes.len()+chunk.len()>65536{return Err("Waiting response capacity".into())}bytes.extend_from_slice(&chunk)}let reply=protocol::response(&bytes)?;if reply.service_id.as_deref()!=Some(&s.service)||reply.service_epoch.as_deref()!=Some(&s.service_epoch)||reply.request_id!=r.request_id{return Err("Waiting reply scope changed".into())}if let Some(e)=&reply.error{let ms=e.retry_after_ms.unwrap_or(0);if ms>60000{return Err("Waiting retry duration bound".into())}return Err(Failure{code:e.code.clone().unwrap_or("Waiting refused".into()),not_before:(ms>0).then(||Instant::now()+Duration::from_millis(u64::from(ms)))})}Ok(reply)}).await.map_err(|_|"Waiting whole request deadline")?
}
fn envelope(s:&startup::Scope,d:&Session)->w::Request{w::Request{major:Some(1),revision:Some(protocol::REVISION.into()),service_id:Some(s.service.clone()),service_epoch:Some(s.service_epoch.clone()),session_epoch:Some(d.epoch.clone()),request_id:Some(uuid::Uuid::new_v4().to_string()),..Default::default()}}
async fn retirement_step<S:CredentialStore+'static>(store:Arc<S>,writer:Arc<Mutex<()>>,http:reqwest::Client,origin:String,service:String,epoch:String,device:uuid::Uuid,desktop:Session,nonce:Arc<AtomicU64>,expected:u64)->Result<(),Failure>{
 let selected={let (store,writer)=(store.clone(),writer.clone());let account=desktop.account.clone();tokio::task::spawn_blocking(move||{let _guard=writer.lock().unwrap();let adapter=rejoin::ResumeStore(store);for marker in journal::retirement_markers(&adapter,&origin,&service,&account,device)?{let original=startup::recall_retired(&adapter,&origin,&service,&account,device,&marker)?.ok_or("Retirement original absent")?;let j=journal::load_retired(&adapter,&original.original,&marker)?.ok_or("Retirement journal absent")?;if !j.retirement_done{return Ok::<_,String>(Some(Loaded{scope:original.original,journal:j}))}journal::finish_retirement(&adapter,&original.original,&marker)?;}Ok(None)}).await.map_err(|_|"Retirement lookup worker cancelled")??};
 let Some(mut l)=selected else{return Ok(())};if l.scope.service_epoch!=epoch{return Err("Waiting retirement service epoch changed; original retained".into())}
 let pending_reservation={let(store,writer,s,world)=(store.clone(),writer.clone(),l.scope.clone(),l.journal.marker.local_world);tokio::task::spawn_blocking(move||{let _guard=writer.lock().unwrap();pending::has_source(&rejoin::ResumeStore(store),&s,world)}).await.map_err(|_|"Retirement reservation lookup canceled")??};
 // A new startup's pending reservation must also become explicitly offline
 // after promotion; pendingOffline forwards the exact waiting retirement.
 if pending_reservation{return pending::retire(store,writer,http,desktop,nonce,expected,l).await}
 if nonce.load(Ordering::Acquire)!=expected{return Err("Waiting retirement authentication retired".into())}
 if let Some(op)=&l.journal.operation{
  let mut request=envelope(&l.scope,&desktop);request.outcome=Some(op.original.clone());let result=rpc(&http,&l.scope,&desktop,&request).await?;
  let outcome=result.outcome.as_ref().ok_or("Retirement original outcome absent")?;let state=result.waiting.as_ref().ok_or("Retirement current state absent")?;
  if result.credentials.is_some()||result.waiting_proof.is_some()||outcome.original_outcome.as_deref()!=Some("committed")||outcome.coverage.as_deref()!=Some("exact_durable_record")||serde_json::to_value(&outcome.original).ok()!=serde_json::to_value(&Some(&op.original)).ok()||serde_json::to_value(&state.intent).ok()!=serde_json::to_value(&l.journal.intent).ok(){return Err("Retirement prior effect remains uncertain".into())}
  l.journal.generation=state.generation.filter(|g|*g>=l.journal.generation).ok_or("Retirement generation regressed")?;l.journal.current_attachment=state.owner.as_ref().and_then(|o|o.attachment_id.clone());l.journal.operation=None;
 }
 if l.journal.retirement.is_none(){let mut r=envelope(&l.scope,&desktop);r.waiting_id=l.journal.intent.as_ref().and_then(|i|i.waiting_id.clone());r.generation=Some(l.journal.generation);r.offline=Some(w::Proof{waiting_proof:l.journal.proof.clone()});l.journal.retirement=Some(journal::Mutation::capture(&l.scope,r,&desktop.id)?);}
 let l={let (store,writer,l,nonce)=(store.clone(),writer.clone(),l.clone(),nonce.clone());tokio::task::spawn_blocking(move||{let _guard=writer.lock().unwrap();if nonce.load(Ordering::Acquire)!=expected{return Err::<Loaded,String>("Retirement authentication retired before journal".into())}journal::save_retired(&rejoin::ResumeStore(store),&l.scope,&l.journal)?;Ok(l)}).await.map_err(|_|"Retirement journal worker cancelled")??};
 let op=l.journal.retirement.as_ref().ok_or("Retirement exact original absent")?;if nonce.load(Ordering::Acquire)!=expected{return Err("Retirement authentication retired before send".into())}
 let mut l=if op.permits(&l.scope,&desktop.id,&desktop.epoch){let result=rpc(&http,&l.scope,&desktop,&op.request).await?;let state=result.waiting.as_ref().ok_or("Retirement state absent")?;if result.credentials.is_some()||result.waiting_proof.is_some()||state.may_play!=Some(false)||state.state.as_deref()!=Some("terminal")||state.reason.as_deref()!=Some("explicit_offline")||serde_json::to_value(&state.intent).ok()!=serde_json::to_value(&l.journal.intent).ok(){return Err("Retirement response differs".into())}l}else{
  let mut r=envelope(&l.scope,&desktop);r.outcome=Some(op.original.clone());let result=rpc(&http,&l.scope,&desktop,&r).await?;let o=result.outcome.ok_or("Retirement outcome absent")?;if result.credentials.is_some()||result.waiting_proof.is_some()||o.original_outcome.as_deref()!=Some("committed")||o.coverage.as_deref()!=Some("exact_durable_record")||serde_json::to_value(&o.original).ok()!=serde_json::to_value(&Some(&op.original)).ok(){return Err("Retirement authentication changed; exact effect remains uncertain".into())}l
 };
 l.journal.retirement=None;l.journal.retirement_done=true;
 tokio::task::spawn_blocking(move||{let _guard=writer.lock().unwrap();if nonce.load(Ordering::Acquire)!=expected{return Err::<(),String>("Retirement authentication retired before commit".into())}let adapter=rejoin::ResumeStore(store);journal::save_retired(&adapter,&l.scope,&l.journal)?;journal::finish_retirement(&adapter,&l.scope,&l.journal.marker)}).await.map_err(|_|"Retirement commit worker cancelled")??;Ok(())
}
async fn recover<S:CredentialStore+'static>(store:Arc<S>,writer:Arc<Mutex<()>>,c:Arc<Mutex<lifecycle::Coordinator>>,nonce:Arc<AtomicU64>,generation:Arc<AtomicU64>,expected:(u64,u64),ticket:lifecycle::Ticket,http:reqwest::Client,desktop:Session,device:uuid::Uuid,l:Loaded,contact:crate::pipe::rejoin::Contact,report:native_report::Latest,game_version:String,capabilities:Vec<String>)->Result<Recovered,Failure>{
 if l.journal.intent.is_none()||l.journal.proof.is_none(){return pending::recover_pending(store,writer,c,nonce,generation,expected,ticket,http,desktop,device,l,contact,report,game_version,capabilities).await}
 let intent=l.journal.intent.as_ref().ok_or("Waiting canonical intent missing")?;if intent.content_digest.as_deref()!=Some(&hex(&report.digest)){return Err("Waiting current content differs".into())}
 let mut r=envelope(&l.scope,&desktop);r.waiting_id=intent.waiting_id.clone();r.generation=Some(l.journal.generation);r.recover=Some(w::Recover{waiting_proof:l.journal.proof.clone(),game_version:Some(game_version),mod_version:Some(report.observation.mod_version.clone()),bridge_version:Some(report.observation.bridge_version.clone()),capabilities:Some(capabilities),context_id:Some(hex(&contact.context)),observation:Some(w::GameObservation{attachment_id:l.journal.current_attachment.clone(),game_instance_id:Some(uuid::Uuid::new_v4().to_string()),world_token:intent.world_token.clone(),observation_revision:Some(report.sequence.try_into().map_err(|_|"Waiting report revision bound")?),actual_seed:report.observation.actual_seed.clone(),active_mods:Some(report.observation.mods.clone()),inventory_complete:Some(true),beta:Some(match report.observation.beta{Some(true)=>"true",Some(false)=>"false",None=>"unknown"}.into()),protected:Some(true),loaded:Some(true),seed_status:Some("known".into()),collector_failure:None,mod_version:Some(report.observation.mod_version.clone()),bridge_version:Some(report.observation.bridge_version.clone()),compatibility_provenance:Some("game-thread-client-observation".into())})});
 let captured=journal::Mutation::capture(&l.scope,r,&desktop.id)?;
 let mut l={let (store,writer,c,nonce,generation,ticket,l)=(store.clone(),writer.clone(),c.clone(),nonce.clone(),generation.clone(),ticket.clone(),l.clone());tokio::task::spawn_blocking(move||{let _guard=writer.lock().unwrap();if !current(&c,&nonce,&generation,expected,&ticket){return Err("Waiting recovery owner retired".into())}let adapter=rejoin::ResumeStore(store);let mut j=journal::load(&adapter,&l.scope,&l.journal.marker)?.ok_or("Waiting protected journal absent")?;if j.terminal{return Err("Waiting explicitly retired".into())}if j.operation.is_none(){j.operation=Some(captured);journal::save(&adapter,&l.scope,&j)?}Ok::<_,String>(Loaded{scope:l.scope.clone(),journal:j})}).await.map_err(|_|"Waiting recovery journal worker cancelled")??};
 let op=l.journal.operation.as_ref().ok_or("Waiting exact original absent")?;
 if !op.permits(&l.scope,&desktop.id,&desktop.epoch)||op.request.recover.as_ref().and_then(|r|r.context_id.as_deref())!=Some(hex(&contact.context).as_str()){
  let mut r=envelope(&l.scope,&desktop);r.outcome=Some(op.original.clone());let result=rpc(&http,&l.scope,&desktop,&r).await?;
  if result.credentials.is_some()||result.waiting_proof.is_some()||result.canonical_admission.is_some(){return Err("Unexpected authority in waiting outcome".into())}
  let outcome=result.outcome.as_ref().ok_or("Waiting original outcome absent")?;
  if outcome.original_outcome.as_deref()!=Some("committed")||outcome.coverage.as_deref()!=Some("exact_durable_record")||serde_json::to_value(&outcome.original).ok()!=serde_json::to_value(&Some(&op.original)).ok(){return Err("Waiting original remains explicitly uncertain".into())}
  let state=result.waiting.as_ref().ok_or("Waiting current state absent")?;
  if state.may_play!=Some(false)||serde_json::to_value(&state.intent).ok()!=serde_json::to_value(&l.journal.intent).ok(){return Err("Waiting outcome world differs".into())}
  // A historical outcome is never current authority. Retain the original
  // proof, advance only the observed generation, then make a fresh protected
  // recovery request. Server preserves the original selected participant.
  l.journal.generation=state.generation.filter(|g|*g>=l.journal.generation).ok_or("Waiting outcome generation regressed")?;l.journal.current_attachment=state.owner.as_ref().and_then(|o|o.attachment_id.clone());l.journal.operation=None;l.journal.terminal=state.state.as_deref()==Some("terminal");
  tokio::task::spawn_blocking(move||{let _guard=writer.lock().unwrap();if !current(&c,&nonce,&generation,expected,&ticket){return Err::<(),String>("Waiting outcome owner retired".into())}journal::save(&rejoin::ResumeStore(store),&l.scope,&l.journal)}).await.map_err(|_|"Waiting outcome writer cancelled")??;
  return Err("Waiting original reconciled without credentials; current ownership recovery still required".into())
 }
 let stamp=c.lock().unwrap().request_stamp(&json!([l.scope.origin,l.scope.service,desktop.id,desktop.epoch,"waiting"]).to_string(),op.request.request_id.as_deref().ok_or("Waiting original request absent")?)?;
 if !current(&c,&nonce,&generation,expected,&ticket){return Err("Waiting recovery owner retired before send".into())}
 let response=rpc(&http,&l.scope,&desktop,&op.request).await?;let request=op.request.clone();complete_recovery(store,writer,c,nonce,generation,expected,ticket,desktop,device,l,contact,report,request,stamp,response).await
}
async fn complete_recovery<S:CredentialStore+'static>(store:Arc<S>,writer:Arc<Mutex<()>>,c:Arc<Mutex<lifecycle::Coordinator>>,nonce:Arc<AtomicU64>,generation:Arc<AtomicU64>,expected:(u64,u64),ticket:lifecycle::Ticket,desktop:Session,device:uuid::Uuid,mut l:Loaded,contact:crate::pipe::rejoin::Contact,report:native_report::Latest,request:w::Request,stamp:nt2_desktop_g0::connection_time::Stamp,mut response:w::Response)->Result<Recovered,Failure>{
 let state=response.waiting.as_ref().ok_or("Waiting recovery state missing")?;
 let mut failed=vec![];for (valid,name) in [(state.may_play==Some(false),"mayPlay=false"),(matches!(state.state.as_deref(),Some("waiting"|"started")),"waiting-or-started"),(state.state.as_deref()==Some("started")||response.canonical_admission.is_none(),"staging-admission-absent"),(state.current_context_id.as_deref()==Some(&hex(&contact.context)),"current-context"),(serde_json::to_value(&state.intent).ok()==serde_json::to_value(&l.journal.intent).ok(),"immutable-intent")]{if !valid{failed.push(name)}}if !failed.is_empty(){return Err(format!("Recovered waiting immutable world/state differs: {}",failed.join(",")).into())}
 let owner=state.owner.as_ref().ok_or("Waiting current owner missing")?;let actual=request.recover.as_ref().and_then(|r|r.observation.as_ref()).or_else(||request.pending_recover.as_ref().and_then(|r|r.observation.as_ref())).ok_or("Waiting original recover observation missing")?;let instance=actual.game_instance_id.clone().ok_or("Waiting instance absent")?;
 let credentials:wire::Credentials=serde_json::from_value(serde_json::to_value(response.credentials.take().ok_or("Waiting returned no current game credentials")?).map_err(|_|"Waiting credentials conversion")?).map_err(|_|"Waiting credentials shape")?;let session=Session::read(credentials,&device.to_string(),"game")?;
 if session.account!=l.scope.account||owner.desktop_id.as_deref()!=Some(&desktop.id)||owner.desktop_session_epoch.as_deref()!=Some(&desktop.epoch)||owner.game_session_id.as_deref()!=Some(&session.id)||owner.game_session_epoch.as_deref()!=Some(&session.epoch)||owner.attachment_id.as_deref()!=Some(&session.attachment)||owner.game_instance_id.as_deref()!=Some(&instance){return Err("Waiting returned credential owner differs".into())}
 if state.state.as_deref()==Some("started"){validate_selected_bridge(state,response.participant.as_ref().ok_or("Started waiting recovery did not return its selected participant")?,&session,&desktop,&l.scope)?;if response.resume_proof.as_ref().is_none_or(|p|p.len()!=43){return Err("Selected waiting recovery proof absent".into())}}
 else if response.participant.is_some()||response.resume_proof.is_some(){return Err("Staging waiting recovery unexpectedly carried selected authority".into())}
 // The existing health/admission owner must independently obtain and validate
 // the current canonical admission. Neither this response nor waiting's local
 // generation sets release/entered or manufactures a Start. The durable waiting
 // proof remains sufficient if the app quits before admitted proof capture.
 if let Some(proof)=response.resume_proof.as_mut(){proof.zeroize()}
 let sample=state.sample.as_ref().ok_or("Waiting sample missing")?;let report_state=state.current_report.as_ref().ok_or("Waiting protected observation missing")?;
 if state.owner_remaining_ms.is_none_or(|ms|ms>30000)||sample.sample_key.as_ref().is_none_or(|v|v.len()!=64||!v.bytes().all(|b|b.is_ascii_hexdigit()&&!b.is_ascii_uppercase()))||sample.channel.as_deref()!=Some("waiting-http")||sample.origin_request_id!=request.request_id||sample.credential_id.as_deref()!=Some(&admission::token_id(desktop.access.as_str())?)||report_state.content_digest!=actual.actual_seed.as_ref().map(|_|hex(&report.digest))||report_state.revision!=actual.observation_revision||report_state.loaded!=Some(true)||report_state.complete!=Some(true)||report_state.protected!=Some(true)||report_state.content_valid!=Some(true){return Err("Waiting current sample/report differs".into())}
 validate_review(state,stamp,Instant::now())?;
 let v:nt2_desktop_g0::protocol_connection::wire::ConnectionCredential=serde_json::from_value(serde_json::to_value(response.credential_validity.as_ref().ok_or("Waiting current credential validity absent")?).map_err(|_|"Waiting validity conversion")?).map_err(|_|"Waiting validity shape")?;
 let mut leases=std::collections::BTreeMap::new();admission::credential_sample(&mut leases,&l.scope.origin,&l.scope.service,&session,&v,request.request_id.as_deref().unwrap(),stamp)?;let validity=leases.remove(&admission::credential_key(&l.scope.origin,&l.scope.service,&session)?).ok_or("Waiting validity not sampled")?;
 if state.state.as_deref()==Some("started"){
  let participant=response.participant.as_ref().ok_or("Selected participant absent")?;let association=participant.association.as_ref().ok_or("Selected original association absent")?;let intent=state.intent.as_ref().ok_or("Selected original waiting intent absent")?;
  let ctx=admission::Context{issuer:lifecycle::Issuer{origin:l.scope.origin.clone(),service:l.scope.service.clone(),account:l.scope.account.clone()},epoch:l.scope.service_epoch.clone(),desktop:desktop.clone(),game:session.clone(),instance:instance.clone(),room:l.scope.room.clone(),run:l.scope.run.clone(),run_epoch:l.scope.run_epoch.clone(),config:l.scope.config.try_into().map_err(|_|"Selected config bound")?,world:intent.world_token.clone().ok_or("Selected world absent")?,participant:association.participant_id.clone(),ownership:participant.generation,original_start:association.original_start_id.clone(),seed_policy:intent.seed_policy.clone().ok_or("Selected seed policy absent")?,loot:intent.randomize_loot.ok_or("Selected loot policy absent")?,recovered:true};
  let mut native=handoff::new(if l.journal.marker.loot=="individual:0"{0}else{contact.actual.try_into().map_err(|_|"Selected shared seed bound")?},l.scope.run.clone(),l.scope.config);native.epoch=l.scope.run_epoch.clone();native.owner=Some((contact.pid,contact.context));native.actual_seed=Some(contact.actual);native.world_policy=if l.journal.marker.loot=="individual:0"{2}else if ctx.loot{5}else{1};native.loaded=report.observation.loaded;native.accepted=native.matches_seed(report.observation.actual_seed.as_deref());native.last_seen=Some(report.observed_at);native.observation=Some(report.clone());
  let canonical:nt2_desktop_g0::protocol_connection::wire::ConnectionAdmission=serde_json::from_value(serde_json::to_value(response.canonical_admission.as_ref().ok_or("Selected recovery canonical admission absent")?).map_err(|_|"Selected admission encoding")?).map_err(|_|"Selected admission shape")?;
  // Validate through the sole existing admission consumer, without publishing
  // its temporary result. Health still owns the current release decision.
  admission::State::default().install(canonical,&ctx,&native,request.request_id.as_deref().unwrap(),&admission::token_id(session.access.as_str())?,stamp,validity.deadline.until().ok_or("Selected credential lease absent")?)?;
 }
 l.journal.current_attachment=owner.attachment_id.clone();l.journal.generation=state.generation.filter(|g|*g>l.journal.generation).ok_or("Waiting recovery generation did not advance")?;l.journal.operation=None;
 let l=tokio::task::spawn_blocking(move||{let _guard=writer.lock().unwrap();if !current(&c,&nonce,&generation,expected,&ticket){return Err("Waiting recovery retired before commit".into())}journal::save(&rejoin::ResumeStore(store),&l.scope,&l.journal)?;if !current(&c,&nonce,&generation,expected,&ticket){return Err("Waiting recovery retired during commit".into())}Ok::<_,String>(l)}).await.map_err(|_|"Waiting recovery commit worker cancelled")??;
 Ok(Recovered{loaded:l,session,instance,validity})
}

fn validate_selected_bridge(state:&w::WaitingState,p:&w::Participant,game:&Session,desktop:&Session,s:&startup::Scope)->Result<(),String>{
 let i=state.intent.as_ref().ok_or("Selected bridge waiting intent absent")?;let a=p.association.as_ref().ok_or("Selected bridge original association absent")?;
 if state.admitted_participant_id!=a.participant_id||a.participant_id.as_ref().is_none_or(String::is_empty)||a.original_start_id.as_ref().is_none_or(String::is_empty)||a.service_id.as_deref()!=Some(&s.service)||a.room_id.as_deref()!=Some(&s.room)||a.run_id.as_deref()!=Some(&s.run)||a.account_id.as_deref()!=Some(&s.account)||a.config_revision.map(u64::from)!=Some(s.config)||a.world_token!=i.world_token||a.actual_seed!=i.actual_seed||a.seed_policy!=i.seed_policy||a.randomize_loot!=i.randomize_loot||p.return_eligible!=Some(true)||p.state.as_deref()!=Some("active")||p.service_epoch.as_deref()!=Some(&s.service_epoch)||p.run_epoch.as_deref()!=Some(&s.run_epoch)||p.desktop_id.as_deref()!=Some(&desktop.id)||p.game_session_id.as_deref()!=Some(&game.id)||p.attachment_id.as_deref()!=Some(&game.attachment)||p.generation.is_none_or(|g|g==0){return Err("Selected bridge differs from original admitted world/current owner".into())}Ok(())
}
fn validate_review(state:&w::WaitingState,stamp:nt2_desktop_g0::connection_time::Stamp,now:Instant)->Result<(),String>{
 let remaining=state.review_remaining_ms.filter(|ms|*ms<=3600000).ok_or("Waiting review duration missing or unbounded")?;
 if state.state.as_deref()==Some("started"){return Ok(())}
 if state.state.as_deref()!=Some("waiting"){return Err("Waiting review state differs".into())}
 let mut review=nt2_desktop_g0::connection_time::Deadline::default();if review.install(stamp,remaining,3600000,3600000,now)?<=now{return Err("Waiting review expired in flight".into())}Ok(())
}

#[cfg(test)]#[path="next_waiting_test.rs"]mod tests;
#[path="next_pending_recovery.rs"]mod pending;

