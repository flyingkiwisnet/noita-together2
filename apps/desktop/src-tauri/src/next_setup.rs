//! Single Core-owned setup intent. Network deadlines never supply world authority.
use super::*;
#[derive(Clone)]
pub(super) struct Preview { pub room:String, pub revision:u32, pub config_revision:u32, pub config:wire::Config,pub run:Option<(String,String)>,pub state:Option<String>,pub context:String,pub sequence:u64 }
struct Intent {
    generation:u64, origin:String, service:String, epoch:String, desktop:String, session:String,
    room:String, config_revision:u32, config:Vec<u8>, run:Option<(String,String)>,review_context:String,review_sequence:u64,
    operation:Option<Retry>, renewing:bool, renewal_baseline:Option<String>, renewal_sent:bool,
    next:Instant, failures:u32, blocked:bool, entered:bool,
}
#[derive(Default)]
pub(super) struct Setup {
    pub enabled:bool, pub preview:Option<Preview>, pub(super) join_unknown:Option<(String,String)>, intent:Option<Intent>, error:Option<String>, original:Option<Retry>,
    preview_sequence:Arc<AtomicU64>,
}
impl<S:CredentialStore> Core<S> {
    pub(crate) fn lobby_review_fence(&self)->Arc<AtomicU64>{self.setup.preview_sequence.clone()}
    pub(super) fn preview_context(&self)->String{json!([self.base,self.service,self.epoch,self.device,self.desktop.as_ref().map(|d|(&d.id,&d.epoch,&d.account)),self.generation.load(Ordering::Acquire)]).to_string()}
    pub(super) fn replacement_review_selection(&self)->(u64,String){let sequence=self.setup.preview_sequence.load(Ordering::Acquire);(sequence,json!([sequence,self.setup.preview.as_ref().map(|p|json!([p.room,p.run,p.config_revision,p.config,p.state,p.context,p.sequence,p.revision])),self.room.as_ref().map(|r|json!([r.room_id,r.run_id,r.run_epoch,r.config_revision,r.config,r.state])),self.setup.intent.as_ref().map(|i|json!([i.room,i.run,i.config_revision,i.config,i.desktop,i.session,i.generation,i.blocked,i.operation.as_ref().and_then(|r|r.request.request_id.as_ref())]))]).to_string())}
    pub(super) fn replacement_review_target(&self)->Option<Preview>{
        let sequence=self.setup.preview_sequence.load(Ordering::Acquire);
        if let Some(r)=self.room.as_ref(){let run=r.run_id.clone().zip(r.run_epoch.clone());let joined=self.setup.intent.as_ref().is_some_and(|i|self.setup_context_current(i)&&!i.blocked&&i.operation.is_none()&&i.run==run&&r.room_id.as_deref()==Some(&i.room)&&r.config_revision==Some(i.config_revision)&&r.config.as_ref().and_then(|c|serde_json::to_vec(c).ok()).as_ref()==Some(&i.config))&&self.setup.preview.as_ref().is_none_or(|p|Some(&p.room)==r.room_id.as_ref());
            if joined||self.setup.preview.is_none(){return Some(Preview{room:r.room_id.clone()?,revision:r.revision?,config_revision:r.config_revision?,config:r.config.clone()?,run,state:r.state.clone(),context:self.preview_context(),sequence})}
        }self.setup.preview.clone()
    }
    // Local source seam only. The reviewed entry-preview consumer owns validation;
    // staging this separate worker never clears/revokes A or changes generation.
    pub(super) fn stage_provisional_preview(&mut self,preview:nt2_desktop_g0::provisional_local::Preview)->Result<(),String> where S:'static {
        let epoch=self.sync_replacement_lineage().ok_or("Local base transition is not settled")?;self.stage_provisional_at(preview,Some(epoch))
    }
    pub(super) fn stage_provisional_at(&mut self,preview:nt2_desktop_g0::provisional_local::Preview,epoch:Option<u64>)->Result<(),String> where S:'static {
        let worker=self.provisional.get_or_insert_with(||Arc::new(nt2_desktop_g0::provisional_local::Worker::coherent(
            Arc::new(super::rejoin::ResumeStore(self.store.clone())),self.credential_writer.clone(),self.replacement_lineage.clone())));
        worker.stage_at(preview,epoch)
    }
    pub fn enable_automatic_setup(&mut self){self.setup.enabled=true;}
    pub(super) fn review_created_lobby(&mut self)->Result<(),String>{
        let context=self.preview_context();let sequence=self.setup.preview_sequence.fetch_add(1,Ordering::AcqRel)+1;
        let room=self.room.as_ref().ok_or("Created lobby unavailable")?;
        self.setup.preview=Some(Preview{room:required(room.room_id.clone())?,revision:room.revision.ok_or("Lobby revision unavailable")?,
            config_revision:room.config_revision.ok_or("Config revision unavailable")?,config:room.config.clone().ok_or("Created config unavailable")?,run:room.run_id.clone().zip(room.run_epoch.clone()),state:room.state.clone(),context,sequence});Ok(())
    }
    pub fn automatic_setup_enabled(&self)->bool{
        self.setup.enabled && self.info.as_ref().and_then(|i|i.connection_flow_revision.as_deref())==Some("g3-flow-1")
    }
    pub(super) fn stop_setup(&mut self,reason:&str){self.setup.intent=None;self.setup.preview=None;self.setup.error=Some(reason.into());}
    // A Join gesture selects the latest authoritative lobby snapshot internally.
    // This is admission/configuration fencing, not an extra user consent screen.
    pub fn join_selected<'a>(&'a mut self,room:&'a str,password:Option<String>)->std::pin::Pin<Box<dyn std::future::Future<Output=Result<(),String>>+Send+'a>> where S:'static {Box::pin(async move{
        if !self.automatic_setup_enabled(){return Box::pin(self.join(room,password)).await}
        if self.setup.join_unknown.as_ref().is_some_and(|(context,_)|*context==self.preview_context()){return Err("The previous Join outcome is unconfirmed. Reconnect to confirm your lobby membership".into())}
        self.refresh_lobby_preview(room).await?;let selected=self.setup.preview.clone().ok_or("Selected lobby unavailable")?;
        if self.lc1{self.lc1_join_snapshot(room,password,selected.revision,selected.config_revision,true).await}else{Box::pin(self.join_displayed(room,password,selected.revision,selected.config_revision)).await}
    })}
    pub async fn preview_lobby(&mut self,room:&str)->Result<(),String> where S:'static {self.refresh_lobby_preview(room).await.map(|_|())}
    pub(super) async fn refresh_lobby_preview(&mut self,room:&str)->Result<bool,String> where S:'static {
        self.setup.preview=None;
        let context=self.preview_context();let sequence=self.setup.preview_sequence.fetch_add(1,Ordering::AcqRel)+1;let expected=self.generation.load(Ordering::Acquire);
        let account=self.desktop.as_ref().ok_or("Sign in before lobby preview")?.account.clone();
        let (store,writer,origin,service,device,selected)=(self.store.clone(),self.credential_writer.clone(),self.base.clone(),self.service.clone(),self.device,room.to_owned());
        let snapshot=if self.lc1{None}else{Some(tokio::task::spawn_blocking(move||{let _guard=writer.lock().unwrap();nt2_desktop_g0::startup_local::selector_snapshot(&super::rejoin::ResumeStore(store),&origin,&service,&account,device,&selected)}).await.map_err(|_|"Lobby selector snapshot worker cancelled")??)};
        let mut request=self.request(false);request.room_id=Some(room.into());request.expected_revision=None;
        request.config_revision=None;request.run_id=None;request.run_epoch=None;request.room_preview=Some(wire::Empty{});
        let response=self.rpc(&request,false,false).await?;
        if context!=self.preview_context()||sequence!=self.setup.preview_sequence.load(Ordering::Acquire){return Err("Late lobby preview refused".into())}
        let admission=response.admission.ok_or("Lobby preview unavailable")?;
        let public=response.room.ok_or("Service must provide the lobby configuration before Join")?;
        if admission.room_id.as_deref()!=Some(room)||public.room_id!=admission.room_id||public.config_revision!=admission.config_revision||public.revision!=admission.revision{
            return Err("Lobby preview context changed; select it again".into());
        }
        let run=match (public.run_id.clone(),public.run_epoch.clone()){(Some(run),Some(epoch)) if uuid::Uuid::parse_str(&run).is_ok()&&uuid::Uuid::parse_str(&epoch).is_ok()=>Some((run,epoch)),(None,None)=>None,_=>return Err("Lobby preview run identity incomplete".into())};
        if let (Some((run,epoch)),Some(snapshot))=(&run,snapshot){if matches!(public.state.as_deref(),Some("staging"|"running")){
            let (store,writer,fence,run,epoch,id)=(self.store.clone(),self.credential_writer.clone(),self.generation.clone(),run.clone(),epoch.clone(),request.request_id.clone().ok_or("Preview correlation absent")?);
            tokio::task::spawn_blocking(move||{let _guard=writer.lock().unwrap();nt2_desktop_g0::startup_local::reconcile_completed_runs(&super::rejoin::ResumeStore(store),&snapshot,&run,&epoch,&id,&fence,expected)}).await.map_err(|_|"Completed-run selector worker cancelled")??;
        }}
        if context!=self.preview_context()||sequence!=self.setup.preview_sequence.load(Ordering::Acquire){return Err("Lobby preview retired during selector reconciliation".into())}
        self.setup.preview=Some(Preview{room:room.into(),revision:admission.revision.ok_or("Missing lobby revision")?,
            config_revision:admission.config_revision.ok_or("Missing configuration revision")?,config:public.config.ok_or("Missing lobby configuration")?,run,state:public.state,context,sequence});
        Ok(admission.join_allowed==Some(true))
    }
    pub async fn join_displayed(&mut self,room:&str,password:Option<String>,revision:u32,config_revision:u32)->Result<(),String> where S:'static {
        if self.lc1{return self.lc1_join_displayed(room,password,revision,config_revision).await}
        if !self.automatic_setup_enabled(){return Err("Automatic setup requires the reviewed connection-flow service".into());}
        let preview=self.setup.preview.clone().ok_or("Review the lobby configuration before Join")?;
        if preview.room!=room||preview.revision!=revision||preview.config_revision!=config_revision{return Err("Displayed lobby review changed; review it again".into());}
        let desktop=self.desktop.as_ref().ok_or("Sign in first")?.clone();
        let config=serde_json::to_vec(&preview.config).map_err(|_|"Configuration encoding failed")?;
        if self.setup.intent.as_ref().is_some_and(|i|self.setup_context_current(i)&&!i.blocked&&i.room==room&&i.config_revision==config_revision&&i.config==config){return Ok(());}
        if preview.context!=self.preview_context()||preview.sequence!=self.setup.preview_sequence.load(Ordering::Acquire){return Err("Lobby review belongs to an older session/entry; review it again".into())}
        if self.setup.intent.as_ref().is_some_and(|i|!i.blocked&&i.operation.is_some()){
            let reason="Original lobby request outcome is unresolved; the new lobby setup has not started";
            self.setup.error=Some(reason.into());return Err(reason.into());
        }
        // Explicit unrelated-room setup parks the exact old operation first.
        let (store,writer)=(self.store.clone(),self.credential_writer.clone());
        let selected_index=self.rejoin_index_profile()?;
        let (origin,service,account,device,selected)=(self.base.clone(),self.service.clone(),desktop.account.clone(),self.device,room.to_owned());
        let (saved,restored)=tokio::task::spawn_blocking(move||{let _guard=writer.lock().unwrap();
            let selected_record=store.load(&selected_index).map_err(|_|"Saved selected index unavailable")?;
            let adapter=super::rejoin::ResumeStore(store);
            let selected_record=selected_record.map(|b|nt2_desktop_g0::rejoin_local::Association::decode(b.bytes())).transpose()?
                .filter(|a|a.origin==origin&&a.service==service&&a.account==account&&a.room==selected);
            let restored=if let Some(a)=selected_record{
                if nt2_desktop_g0::rejoin_local::recall(&adapter,&a).map_err(|_|"Saved selected proof unavailable")?.is_some()
                    && nt2_desktop_g0::rejoin_local::retired_proof(&adapter,&a).map_err(|_|"Saved selected retirement unavailable")?.is_none(){Some(a)}else{None}
            }else{None};
            Ok::<_,String>((nt2_desktop_g0::startup_local::has_recoverable_room(&adapter,&origin,&service,&account,device,&selected)?,restored))
        }).await.map_err(|_|"Saved startup lookup worker cancelled")??;
        if preview.context!=self.preview_context()||preview.sequence!=self.setup.preview_sequence.load(Ordering::Acquire){return Err("Lobby review retired during saved lookup".into())}
        self.park_foreign_recovery()?;
        if self.resume_intent.association().is_none(){if let Some(a)=restored{self.resume_intent.offer(a,true)?;}}
        let admitted=self.resume_intent.association().is_some_and(|a|a.room==room&&preview.run.as_ref().is_none_or(|(run,_)|a.run==*run))
            && !matches!(self.resume_intent.phase(),nt2_desktop_g0::rejoin_local::Phase::Offline|nt2_desktop_g0::rejoin_local::Phase::Terminal);
        if saved||admitted{
            self.entry.selected=true;
            if preview.state.as_deref()==Some("staging"){self.prepare_replacement_preview().await?;}
            self.setup.error=None;return Ok(());
        }
        if preview.state.as_deref()==Some("staging"){if let Some((run,epoch))=&preview.run{self.park_completed_run_recovery(room,run,epoch)?;}}
        if self.provisional.as_ref().is_some_and(|w|w.fresh_started()){return Err("Original fresh entry is still owned; keep its protected world".into())}
        self.entry.selected=false;self.provisional=None;self.native_diagnostics.lock().unwrap().provisional=None;
        self.park_for_fresh_room(room)?;
        if let Some(reason)=self.fresh_join_recovery_blocker()?{
            self.setup.error=Some(reason.into());return Err(reason.into());
        }
        // Hold the pipe lock while fencing an unconsumed previous grant.
        {let mut pending=self.pending.lock().unwrap();
            let completed=preview.state.as_deref()==Some("staging")&&preview.run.as_ref().is_some_and(|(run,epoch)|pending.as_ref().is_some_and(|p|p.run!=*run&&p.epoch!=*epoch));
            if pending.as_ref().is_some_and(|p|p.owner.is_some())&&!completed{
                return Err("A world is already consumed; leave it and choose a fresh New Game before changing lobby".into());
            }
            self.generation.fetch_add(1,Ordering::AcqRel);*pending=None;
        }
        self.clear_game();
        if let Some(issuer)=self.lifecycle_issuer(){self.lifecycle.lock().unwrap().fresh(issuer,room.into());}
        // Explicit fresh Join supersedes only transient native routing. Saved
        // associations/proofs and chosen/terminal intent remain intact. Do this
        // before awaiting Join, so an ambiguous reply cannot restore an old
        // mailbox/contact ahead of the original setup retry.
        self.rejoin_transport=Default::default();
        self.native_diagnostics.lock().unwrap().rejoin=Default::default();
        self.native_diagnostics.lock().unwrap().clear_continue_notice();
        let mut request=self.request(false);request.room_id=Some(room.into());request.expected_revision=Some(revision);
        request.config_revision=Some(config_revision);request.run_id=None;request.run_epoch=None;request.room_join=Some(wire::RoomJoin{password});
        self.setup.intent=Some(Intent{generation:self.generation.load(Ordering::Acquire),origin:self.base.clone(),service:self.service.clone(),epoch:self.epoch.clone(),
            desktop:desktop.id,session:desktop.epoch,room:room.into(),config_revision,config,run:None,review_context:preview.context,review_sequence:preview.sequence,
            operation:Some(Retry{request,until:Instant::now()+Duration::from_secs(30)}),renewing:false,renewal_baseline:None,renewal_sent:false,next:Instant::now(),failures:0,blocked:false,entered:false});
        self.setup.error=None;
        // The intent is established even if the first bounded request fails.
        self.setup_tick().await
    }
    pub(super) fn fresh_setup_owns_world(&self)->bool{
        self.setup.intent.as_ref().is_some_and(|i|self.setup_context_current(i)&&!i.blocked&&!i.entered
            &&self.game.as_ref().is_none_or(|g|!g.entered)
            &&(i.operation.as_ref().is_some_and(|r|r.request.room_join.is_some())
                ||self.preparation.as_ref().is_some_and(|p|p.room_id.as_deref()==Some(&i.room))))
    }
    pub(super) fn setup_matches_saved_waiting(&self,s:&nt2_desktop_g0::startup_local::Scope)->bool{
        let Some(i)=&self.setup.intent else{return false};
        self.setup_context_current(i)&&i.operation.is_none()&&!i.blocked&&!i.entered
            &&s.origin==i.origin&&s.service==i.service&&s.service_epoch==i.epoch&&s.device==self.device&&s.desktop==i.desktop&&s.desktop_epoch==i.session
            &&self.desktop.as_ref().is_some_and(|d|d.account==s.account)
            &&s.room==i.room&&Some((s.run.clone(),s.run_epoch.clone()))==i.run&&s.config==u64::from(i.config_revision)
            &&nt2_desktop_g0::protocol_connection::canonical_fingerprint(&i.config).ok().as_deref()==Some(&s.config_digest)
            &&self.pending.lock().unwrap().as_ref().is_some_and(|p|p.run==s.run&&p.epoch==s.run_epoch&&p.config==s.config
                &&p.preparation_id.as_deref()==Some(&s.preparation)
                &&p.generation.as_ref().is_some_and(|(g,n)|Arc::ptr_eq(g,&self.generation)&&*n==i.generation&&g.load(Ordering::Acquire)==*n)
                &&p.startup_bundle.as_ref().is_some_and(|b|b.scope()==s))
    }
    // Only a confirmed current Join plus its coherent local canonical bundle
    // distinguishes a next run. A preview alone cannot retire a source.
    pub(super) fn fresh_setup_supersedes_entry(&self,old:&nt2_desktop_g0::startup_local::Scope)->bool{
        let (Some(i),Some(review),Some(room),Some(prep))=(&self.setup.intent,&self.setup.preview,&self.room,&self.preparation)else{return false};
        if !self.setup_context_current(i)||i.blocked||i.entered||i.operation.is_some()||self.native_revoked
            ||review.context!=i.review_context||review.sequence!=i.review_sequence||review.sequence!=self.setup.preview_sequence.load(Ordering::Acquire)
            ||review.room!=i.room||review.config_revision!=i.config_revision||serde_json::to_vec(&review.config).ok().as_ref()!=Some(&i.config)
            ||!(review.run==i.run||review.run.is_none()&&review.state.as_deref()==Some("waiting"))
            ||room.state.as_deref()!=Some("staging")||room.room_id.as_deref()!=Some(&i.room)||room.run_id.clone().zip(room.run_epoch.clone())!=i.run
            ||room.config_revision!=Some(i.config_revision)||room.config.as_ref().and_then(|c|serde_json::to_vec(c).ok()).as_ref()!=Some(&i.config){return false}
        let pending=self.pending.lock().unwrap();let Some(p)=pending.as_ref()else{return false};let Some(bundle)=&p.startup_bundle else{return false};let s=bundle.scope();
        old.origin==s.origin&&old.service==s.service&&old.account==s.account&&old.device==s.device&&old.run!=s.run&&old.run_epoch!=s.run_epoch
            &&s.origin==i.origin&&s.service==i.service&&s.service_epoch==i.epoch&&s.account==self.desktop.as_ref().unwrap().account&&s.device==self.device
            &&s.desktop==i.desktop&&s.desktop_epoch==i.session&&s.room==i.room&&Some((s.run.clone(),s.run_epoch.clone()))==i.run&&s.config==u64::from(i.config_revision)
            &&nt2_desktop_g0::protocol_connection::canonical_fingerprint(&i.config).ok().as_deref()==Some(&s.config_digest)&&p.owner.is_none()&&p.run==s.run&&p.epoch==s.run_epoch&&p.config==s.config
            &&p.generation.as_ref().is_some_and(|(g,n)|Arc::ptr_eq(g,&self.generation)&&*n==i.generation&&g.load(Ordering::Acquire)==*n)
            &&p.preparation_id.as_deref()==Some(&s.preparation)&&prep.preparation_id==p.preparation_id&&prep.state.as_deref()==Some("issued")
            &&prep.service_id.as_deref()==Some(&s.service)&&prep.service_epoch.as_deref()==Some(&s.service_epoch)&&prep.desktop_id.as_deref()==Some(&s.desktop)&&prep.desktop_session_epoch.as_deref()==Some(&s.desktop_epoch)
            &&prep.room_id.as_deref()==Some(&s.room)&&prep.run_id.as_deref()==Some(&s.run)&&prep.run_epoch.as_deref()==Some(&s.run_epoch)&&prep.config_revision.map(u64::from)==Some(s.config)
    }
    pub(super) fn park_setup_for_entry(&mut self){if let Some(i)=&mut self.setup.intent{i.blocked=true;}}
    pub(super) fn note_native_entry(&mut self,local:&handoff::Pending){
        if self.game.as_ref().is_some_and(|g|g.entered){
            self.complete_verified_setup_entry(local);self.record_health_entry();
        }
    }
    // Only local native consumption or a matching private health checkpoint
    // supplies this fact, never a service phase/timestamp.
    pub(super) fn complete_verified_setup_entry(&mut self,local:&handoff::Pending){
        let current=self.setup.intent.as_ref().is_some_and(|i|self.setup_context_current(i)
            &&i.run.as_ref()==Some(&(local.run.clone(),local.epoch.clone()))
            &&self.game.as_ref().is_some_and(|g|Some(g.owner)==local.owner));
        if current{self.setup.intent.as_mut().unwrap().entered=true;}
    }
    fn setup_context_current(&self,i:&Intent)->bool{
        i.generation==self.generation.load(Ordering::Acquire)&&i.origin==self.base&&i.service==self.service&&i.epoch==self.epoch
            &&self.desktop.as_ref().is_some_and(|d|d.id==i.desktop&&d.epoch==i.session)
    }
    async fn setup_snapshot(&mut self,i:&Intent)->Result<wire::Response,String>{
        let mut request=self.request(false);request.room_id=Some(i.room.clone());request.expected_revision=None;
        request.config_revision=None;request.run_id=None;request.run_epoch=None;request.room_get=Some(wire::Empty{});
        let response=self.rpc(&request,false,false).await?;
        if !self.setup_context_current(i){return Err("Late setup snapshot refused".into())}Ok(response)
    }
    async fn setup_preview_snapshot(&mut self,i:&Intent)->Result<wire::Response,String>{
        let mut request=self.request(false);request.room_id=Some(i.room.clone());request.expected_revision=None;
        request.config_revision=None;request.run_id=None;request.run_epoch=None;request.room_preview=Some(wire::Empty{});
        let response=self.rpc(&request,false,false).await?;
        if !self.setup_context_current(i){return Err("Late setup preview refused".into())}Ok(response)
    }
    pub(super) async fn setup_tick(&mut self)->Result<(),String>{
        let Some(mut intent)=self.setup.intent.take() else{return Ok(());};
        // Completed setup never reacquires a world after recovery clears game.
        // A sent original operation, if any, still has to be reconciled.
        if intent.entered&&intent.operation.is_none(){self.setup.intent=Some(intent);return Ok(());}
        if !self.setup_context_current(&intent){self.setup.error=Some("Setup cancelled because service/session changed; join again".into());return Ok(());}
        if intent.blocked||Instant::now()<intent.next{self.setup.intent=Some(intent);return Ok(());}
        let result=tokio::time::timeout(Duration::from_secs(3),self.setup_step(&mut intent)).await.unwrap_or_else(|_|Err("Whole setup attempt deadline".into()));
        if !self.setup_context_current(&intent){self.setup.error=Some("Late setup response refused; join again".into());return Err("Late setup response refused; join again".into());}
        match &result {
            Ok(())=>{intent.failures=0;self.setup.error=None;intent.next=Instant::now()+Duration::from_secs(2);},
            Err(error)=>{intent.failures=intent.failures.saturating_add(1);intent.next=Instant::now()+Duration::from_secs(1u64<<intent.failures.min(3));self.setup.error=Some(error.clone());}
        }
        self.setup.intent=Some(intent);result
    }
    fn setup_consumed(&self)->bool{self.pending.lock().unwrap().as_ref().is_some_and(|p|p.owner.is_some())}
    fn validate_renewal_snapshot(&self,i:&Intent,response:&wire::Response)->Result<(),String>{
        let room=response.room.as_ref().ok_or("Renewal reconciliation has no lobby")?;
        let prep=response.preparation.as_ref().ok_or("Renewal reconciliation has no original preparation")?;
        let original=self.preparation.as_ref().ok_or("Original preparation unavailable")?;
        let pending=self.pending.lock().unwrap();
        if !self.setup_context_current(i)||room.room_id.as_deref()!=Some(&i.room)||room.config_revision!=Some(i.config_revision)
            ||room.config.as_ref().and_then(|c|serde_json::to_vec(c).ok()).as_ref()!=Some(&i.config)
            ||!matches!(room.state.as_deref(),Some("staging"|"running"))
            ||i.run.as_ref()!=room.run_id.clone().zip(room.run_epoch.clone()).as_ref()
            ||!room.members.as_ref().is_some_and(|ms|ms.iter().any(|m|m.desktop_id.as_deref()==Some(&i.desktop)&&m.consented_config_revision==Some(i.config_revision)))
            ||prep.preparation_id.is_none()||prep.preparation_id!=original.preparation_id
            ||prep.service_id.as_deref()!=Some(&i.service)||prep.service_epoch.as_deref()!=Some(&i.epoch)
            ||prep.desktop_id.as_deref()!=Some(&i.desktop)||prep.desktop_session_epoch.as_deref()!=Some(&i.session)
            ||prep.room_id.as_deref()!=Some(&i.room)||prep.config_revision!=Some(i.config_revision)
            ||prep.run_id!=room.run_id||prep.run_epoch!=room.run_epoch
            ||!pending.as_ref().is_some_and(|p|p.preparation_id==prep.preparation_id&&p.run==room.run_id.as_deref().unwrap_or("")&&p.epoch==room.run_epoch.as_deref().unwrap_or("")&&p.config==u64::from(i.config_revision)){
            return Err("Original renewal scope changed; world remains held".into());
        }
        if !matches!(prep.state.as_deref(),Some("issued"|"consumed"|"loaded"))
            ||self.preparation_remaining(prep,true)?==0
            ||(prep.state.as_deref()==Some("issued")&&self.preparation_remaining(prep,false)?==0){
            return Err("Original waiting preparation reached expiry or ended; world remains held".into());
        }
        Ok(())
    }
    fn finish_renewal(i:&mut Intent){i.operation=None;i.renewing=false;i.renewal_baseline=None;i.renewal_sent=false;}
    async fn reconcile_consumed_renewal(&mut self,i:&mut Intent)->Result<(),String>{
        // An uncertain issued-only mutation is never replayed after local consumption.
        // A snapshot resolves only this original scope, not an attachment or a replacement.
        let response=match self.setup_snapshot(i).await{
            Ok(response)=>response,
            Err(error)=>{
                if matches!(error.as_str(),"FORBIDDEN"|"UNAUTHORIZED"|"ROOM_UNAVAILABLE"|"NOT_FOUND"|"INVALID_STATE"|"STALE_EPOCH"|"RESTART_REQUIRED"|"EXPIRED"|"CONFLICT"){i.blocked=true;self.revoke_native();}
                return Err(error);
            }
        };
        if let Err(error)=self.validate_renewal_snapshot(i,&response){i.blocked=true;self.revoke_native();return Err(error);}
        self.absorb(&response);if self.native_revoked{i.blocked=true;return Err("Original consumed world authority expired; world remains held".into());}Self::finish_renewal(i);Ok(())
    }
    async fn setup_step(&mut self,i:&mut Intent)->Result<(),String>{
        if self.native_diagnostics.lock().unwrap().continue_detected(){
            i.blocked=true;self.revoke_native();return Err("Continue keeps the saved world offline; leave it, review the lobby and explicitly choose a fresh New Game".into());
        }
        if i.renewing&&self.setup_consumed(){
            if i.renewal_sent{self.reconcile_consumed_renewal(i).await?;}else{Self::finish_renewal(i);}
        }
        if let Some(operation)=i.operation.clone(){
            let reply=if Instant::now()<operation.until {if i.renewing{i.renewal_sent=true;}self.rpc(&operation.request,false,false).await}
                else {self.setup_snapshot(i).await};
            if !self.setup_context_current(i){return Err("Late setup original reply refused".into())}
            match reply {
                Ok(response)=>{
                    if i.renewing{
                        if let Err(error)=self.validate_renewal_snapshot(i,&response){i.blocked=true;self.revoke_native();return Err(error);}
                    }
                    let room=response.room.as_ref().ok_or("Setup reply has no current lobby")?;
                    if room.room_id.as_deref()!=Some(&i.room)||room.config_revision!=Some(i.config_revision)
                        || room.config.as_ref().and_then(|c|serde_json::to_vec(c).ok()).as_ref()!=Some(&i.config){
                        i.blocked=true;self.revoke_native();return Err("Lobby configuration changed; review and Join explicitly again".into());
                    }
                    if !room.members.as_ref().is_some_and(|members|members.iter().any(|m|m.desktop_id.as_deref()==Some(&i.desktop)&&m.consented_config_revision==Some(i.config_revision))){
                        i.blocked=true;return Err("Join was not confirmed; review the lobby and Join again".into());
                    }
                    if i.renewing&&Instant::now()>=operation.until {
                        if response.preparation.as_ref().is_none_or(|p|matches!(p.state.as_deref(),Some("expired"|"cancelled"))||(p.state.as_deref()==Some("issued")&&self.preparation_remaining(p,false).map_or(true,|v|v==0))||self.preparation_remaining(p,true).map_or(true,|v|v==0)) {
                            i.blocked=true;self.revoke_native();return Err("Uncertain waiting renewal reached expiry; explicitly review a fresh run".into());
                        }
                        if response.preparation.as_ref().and_then(|p|p.expires_at.as_ref())==i.renewal_baseline.as_ref(){return Err("Waiting renewal reply is uncertain; reconciling the same preparation".into());}
                    }
                    self.absorb(&response);Self::finish_renewal(i);
                }
                Err(_) if i.renewing&&self.setup_consumed()=>{
                    return self.reconcile_consumed_renewal(i).await;
                }
                Err(error) if error=="STALE_REVISION"=>{
                    // Definite rejection permits a new operation only after fresh state.
                    let response=if i.renewing{self.setup_snapshot(i).await?}else{self.setup_preview_snapshot(i).await?};
                    let room=response.room.as_ref().ok_or("Fresh lobby unavailable")?;
                    if room.config_revision!=Some(i.config_revision)||room.config.as_ref().and_then(|c|serde_json::to_vec(c).ok()).as_ref()!=Some(&i.config){
                        i.blocked=true;self.revoke_native();return Err("Configuration changed; review it before joining".into());
                    }
                    if i.renewing{
                        if let Err(error)=self.validate_renewal_snapshot(i,&response){i.blocked=true;self.revoke_native();return Err(error);}
                        self.absorb(&response);
                    }let mut request=operation.request.clone();request.request_id=Some(uuid::Uuid::new_v4().to_string());request.expected_revision=room.revision;
                    i.operation=Some(Retry{request,until:Instant::now()+Duration::from_secs(30)});i.renewal_sent=false;return Err("Lobby changed; retrying the same reviewed configuration".into());
                }
                Err(error)=>{
                    if matches!(error.as_str(),"FORBIDDEN"|"UNAUTHORIZED"|"PASSWORD_REQUIRED"|"CAPACITY"|"ROOM_UNAVAILABLE"|"PREPARATION_EXPIRED"|"PREPARATION_USED"|"NOT_FOUND"|"ACCOUNT_REQUIRED"|"INVALID_STATE"|"STALE_EPOCH"|"RESTART_REQUIRED"|"EXPIRED"|"CONFLICT") {i.blocked=true;self.revoke_native();}
                    return Err(error);
                }
            }
        }
        let room=self.room.as_ref().ok_or("Waiting for the joined lobby")?.clone();
        if room.room_id.as_deref()!=Some(&i.room)||room.config_revision!=Some(i.config_revision)
            ||room.config.as_ref().and_then(|c|serde_json::to_vec(c).ok()).as_ref()!=Some(&i.config){
            i.blocked=true;self.revoke_native();return Err("Configuration changed; review the displayed lobby and Join again".into());
        }
        let run=(required(room.run_id.clone())?,required(room.run_epoch.clone())?);
        if i.run.as_ref().is_some_and(|old|old!=&run)||!matches!(room.state.as_deref(),Some("staging"|"running")){
            i.blocked=true;self.revoke_native();return Err("Lobby run ended or changed; choose a fresh New Game after explicit Join".into());
        }
        i.run=Some(run);
        if room.members.as_ref().is_none_or(|members|!members.iter().any(|m|m.desktop_id.as_deref()==Some(&i.desktop)&&m.consented_config_revision==Some(i.config_revision))){
            i.blocked=true;self.revoke_native();return Err("Membership/configuration consent changed; review the lobby".into());
        }
        if self.native_revoked{i.blocked=true;return Err("World authority revoked; leave this world and choose a fresh New Game".into());}
        if self.preparation.is_none(){let response=self.setup_snapshot(i).await?;self.absorb(&response);}
        let prep=self.preparation.as_ref().ok_or("Service has not confirmed own preparation; waiting without creating another")?.clone();
        if prep.service_id.as_deref()!=Some(&self.service)||prep.service_epoch.as_deref()!=Some(&self.epoch)||prep.desktop_id.as_deref()!=Some(&i.desktop)
            ||prep.desktop_session_epoch.as_deref()!=Some(&i.session)||prep.room_id.as_deref()!=Some(&i.room)||prep.config_revision!=Some(i.config_revision)
            ||prep.run_id!=room.run_id||prep.run_epoch!=room.run_epoch {i.blocked=true;self.revoke_native();return Err("Preparation scope differs; automatic setup refused".into());}
        if self.preparation_remaining(&prep,true)?==0{i.blocked=true;self.revoke_native();return Err("Lobby review expired; explicitly start a fresh lobby run".into());}
        if !matches!(prep.state.as_deref(),Some("issued"|"consumed"|"loaded")){i.blocked=true;self.revoke_native();return Err("Preparation expired or ended; choose a fresh New Game after lobby review".into());}
        let consumed=self.pending.lock().unwrap().as_ref().is_some_and(|p|p.owner.is_some());
        if !consumed&&prep.state.as_deref()==Some("issued"){
            let remaining=self.preparation_remaining(&prep,false)?;
            if remaining==0{i.blocked=true;self.revoke_native();return Err("Waiting preparation expired; explicitly Join for a fresh New Game".into());}
            if remaining<=300000{
                let mut request=self.request(false);request.preparation_renew=Some(wire::PreparationRenew{preparation_id:prep.preparation_id.clone()});
                i.operation=Some(Retry{request,until:Instant::now()+Duration::from_secs(30)});i.renewing=true;i.renewal_baseline=prep.expires_at.clone();i.renewal_sent=false;return Ok(());
            }
        }
        let pending=self.pending.lock().unwrap().clone();
        if let Some(p)=pending{
            if p.preparation_id!=prep.preparation_id {i.blocked=true;self.revoke_native();return Err("Replacement preparation refused; choose a fresh New Game".into());}
            if p.owner.is_none(){self.pending.lock().unwrap().as_mut().unwrap().expires=Instant::now()+Duration::from_millis(self.preparation_remaining(&prep,false)?);}
            return Ok(());
        }
        if prep.state.as_deref()!=Some("issued"){i.blocked=true;return Err("Continue or already-consumed world cannot be enrolled; choose a fresh New Game".into());}
        self.discover_game();
        let root=self.game_path.clone().ok_or("Choose a supported Noita folder; setup intent is retained")?;
        if let Err(error)=self.verify_game(&root){self.installation_state=format!("Installation needs attention: {error}");return Err(error);}
        if self.social_requested {let metadata=self.social_profile.clone().ok_or("Matching social runtime required")?;self.enable_social(&root,&metadata,self.social_stream.clone().ok_or("No social owner")?)?;}
        self.arm()
    }
    pub(super) fn setup_restart_required(&mut self,message:&str){
        if self.automatic_setup_enabled(){
            if let Some(i)=self.setup.intent.as_mut(){i.blocked=true;}
            self.setup.original=self.attach_retry.clone().or_else(||self.game.as_ref().and_then(|g|g.report_retry.clone()));
            self.setup.error=Some(message.into());self.revoke_native();
        }
    }
    pub fn setup_view(&self)->Value{
        let enabled=self.automatic_setup_enabled();let pending=self.pending.lock().unwrap();
        let member=self.room.as_ref().and_then(|r|r.members.as_ref()).and_then(|members|members.iter().find(|m|self.desktop.as_ref().is_some_and(|d|m.desktop_id.as_deref()==Some(&d.id))));
        let active=self.setup.intent.as_ref().is_some_and(|i|!i.entered||i.operation.is_some());
        let revoked=self.native_revoked&&active;
        let stage=if self.setup.error.is_some()||revoked{"attention"}else if self.entry.selected&&pending.is_none()&&self.game.is_none(){"continue_world"}else if self.setup.intent.is_none()&&!self.entry.selected{"choose_lobby"}
            else if member.is_some_and(|m|m.readiness.as_deref()==Some("ready")&&m.ready==Some(true)){"ready"}
            else if member.is_some_and(|m|m.readiness.as_deref()==Some("playing")){"playing"}
            else if pending.as_ref().is_some_and(|p|p.loaded){"checking_report"}
            else if pending.as_ref().is_some_and(|p|p.owner.is_some()){ "loading_world" }
            else if pending.as_ref().is_some_and(|p|p.startup_ready){"waiting_noita"}else if pending.is_some(){"preparing"}else{"preparing"};
        json!({"enabled":enabled,"stage":stage,"error":self.setup.error.as_deref().or(if revoked{Some("World authority revoked; keep it held and choose a fresh New Game after explicit lobby review")}else{None}),"intent_retained":active,"blocked":revoked||self.setup.intent.as_ref().is_some_and(|i|i.blocked),
            "installation_required":self.game_path.is_none()||self.installation_state.contains("needs attention")||self.installation_state.contains("needs verification"),
            "replacement":self.entry_view(),"preview":self.setup.preview.as_ref().map(|p|json!({"room_id":p.room,"revision":p.revision,"config_revision":p.config_revision,"config":p.config}))})
    }
    pub async fn leave_lobby(&mut self)->Result<(),String>{
        if self.lc1{if self.world.record.is_some(){self.lc1_choose(true)?;}self.lc1_retire_preparation();let mut request=self.request(false);request.expected_revision=None;request.config_revision=None;request.run_id=None;request.run_epoch=None;request.room_leave=Some(wire::Empty{});self.rpc(&request,false,false).await?;self.room=None;self.inspections.clear();self.chat.clear();self.inspection_cursor.clear();return Ok(())}
        self.resume_intent.terminal();
        let target=self.setup.intent.as_ref().map(|i|i.room.clone()).or_else(||self.room.as_ref().and_then(|r|r.room_id.clone()));
        self.stop_setup("Left lobby");self.generation.fetch_add(1,Ordering::AcqRel);self.clear_game();
        let mut request=self.request(false);request.room_id=target;request.expected_revision=None;request.config_revision=None;request.run_id=None;request.run_epoch=None;
        request.room_leave=Some(wire::Empty{});let response=self.rpc(&request,false,false).await?;
        self.absorb(&response);self.room=None;self.inspections.clear();self.chat.clear();self.inspection_cursor.clear();Ok(())
    }
    pub async fn start_displayed(&mut self,participants:Vec<wire::InspectedParticipant>,excluded:Vec<String>,config_revision:u32)->Result<(),String> where S:'static {
        let capture=self.capture_start()?;
        self.start_captured(capture,participants,excluded,config_revision).await
    }
}
#[cfg(test)] #[path="next_setup_test.rs"] pub(super) mod tests;
