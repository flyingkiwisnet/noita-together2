use super::*;
use std::io::{Read,Write};
use super::super::tests::{MemoryStore,release_case};
pub(crate) enum Reply{Build(Box<dyn Fn(&Value)->Value+Send>),Entry(Value),OutcomeTerminal,Data(Value),Lost,Error(&'static str),Fence(Value,Arc<AtomicU64>),Recovered(Value,Arc<AtomicU64>,Arc<Mutex<Option<handoff::Pending>>>,handoff::Pending),ConsumeError(Arc<Mutex<Option<handoff::Pending>>>,u32)}
#[test]fn seedloot01_canonical_limited_preview_is_displayed_without_enrollment(){tauri::async_runtime::block_on(async{
 let (mut c,_,_)=client();let golden:Value=serde_json::from_str(include_str!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../packages/contracts/g2-next/golden.json"))).unwrap();
 let example=golden["fixtures"].as_array().unwrap().iter().find(|f|f["json"]["admission"].is_object()&&f["json"]["room"]["config"].is_object()).unwrap()["json"].clone();
 let room=example["room"]["roomId"].as_str().unwrap().to_owned();let expected=example["room"]["config"].clone();
 let(origin,worker)=network(vec![Reply::Data(example)]);set_origin(&mut c,&origin);c.setup.preview=None;c.preview_lobby(&room).await.unwrap();
 assert_eq!(serde_json::to_value(&c.setup.preview.as_ref().unwrap().config).unwrap(),expected);assert!(c.setup.intent.is_none()&&c.pending.lock().unwrap().is_none()&&c.room.is_none());worker.join().unwrap();clean(c);
});}
pub(crate) fn network(replies:Vec<Reply>)->(String,std::thread::JoinHandle<Vec<Value>>){
    let listener=std::net::TcpListener::bind("127.0.0.1:0").unwrap();let origin=format!("http://{}",listener.local_addr().unwrap());
    let worker=std::thread::spawn(move||{let mut calls=vec![];listener.set_nonblocking(true).unwrap();
        for reply in replies {
            let deadline=Instant::now()+Duration::from_secs(30);
            let mut socket=loop{match listener.accept(){Ok((s,_))=>break s,Err(e) if e.kind()==std::io::ErrorKind::WouldBlock=>{assert!(Instant::now()<deadline,"Missing fixture request");std::thread::sleep(Duration::from_millis(5));},Err(e)=>panic!("{e}")}};
            socket.set_nonblocking(false).unwrap();socket.set_read_timeout(Some(Duration::from_secs(3))).unwrap();let mut head=vec![];let mut byte=[0];while !head.ends_with(b"\r\n\r\n"){socket.read_exact(&mut byte).unwrap();head.push(byte[0]);assert!(head.len()<8192);}
            let header=String::from_utf8(head).unwrap();let size=header.lines().find_map(|l|l.to_lowercase().strip_prefix("content-length:").and_then(|v|v.trim().parse::<usize>().ok())).unwrap_or(0);assert!(size<=65536);
            let mut body=vec![0;size];socket.read_exact(&mut body).unwrap();let request:Value=serde_json::from_slice(&body).unwrap();calls.push(request.clone());
            let mut response=match reply{Reply::Build(build)=>build(&request),Reply::Entry(v)=>v,Reply::OutcomeTerminal=>json!({"outcome":{"original":request["outcome"].clone(),"originalOutcome":"unresolved","coverage":"unknown","currentDisposition":"offline","currentGeneration":1,"currentReason":"offline","retryAfterMs":2000}}),Reply::Lost=>continue,Reply::Data(v)=>v,Reply::Error(code)=>json!({"error":{"code":code}}),Reply::Fence(v,g)=>{g.fetch_add(1,Ordering::AcqRel);v},Reply::Recovered(v,g,p,world)=>{g.fetch_add(1,Ordering::AcqRel);*p.lock().unwrap()=Some(world);v},Reply::ConsumeError(p,seed)=>{handoff::transition(&mut p.lock().unwrap(),nt2_local_ws::owner::Id::OsProcess(7),&frame(1,0));handoff::transition(&mut p.lock().unwrap(),nt2_local_ws::owner::Id::OsProcess(7),&frame(3,seed));handoff::transition(&mut p.lock().unwrap(),nt2_local_ws::owner::Id::OsProcess(7),&frame(4,1));json!({"error":{"code":"EXPIRED"}})}};
            if response.is_null(){continue} for field in ["major","revision","serviceId","serviceEpoch","requestId"]{response[field]=request[field].clone();}
            if response["entryPreview"].is_object(){let token=header.lines().find_map(|l|l.strip_prefix("authorization: Bearer ").or_else(||l.strip_prefix("Authorization: Bearer "))).unwrap();let credential=super::super::admission::token_id(token).unwrap();response["entryPreview"]["sample"]["originRequestId"]=request["requestId"].clone();response["entryPreview"]["sample"]["credentialId"]=json!(credential);response["credentialValidity"]["originRequestId"]=request["requestId"].clone();response["credentialValidity"]["credentialId"]=json!(credential);response["credentialValidity"]["sessionEpoch"]=request["sessionEpoch"].clone();} if response["admission"]["scope"].is_object(){let token=header.lines().find_map(|l|l.strip_prefix("authorization: Bearer ").or_else(||l.strip_prefix("Authorization: Bearer "))).unwrap();let credential=super::super::admission::token_id(token).unwrap();response["admission"]["sample"]["originRequestId"]=request["requestId"].clone();response["admission"]["sample"]["credentialId"]=json!(credential);response["credentialValidity"]["originRequestId"]=request["requestId"].clone();response["credentialValidity"]["credentialId"]=json!(credential);} let body=serde_json::to_vec(&response).unwrap();let _=write!(socket,"HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",body.len());let _=socket.write_all(&body);
        } calls
    });(origin,worker)
}
fn later(ms:i64)->String{
    let total=(now() as i64+ms) as u64;let z=(total/86400000) as i64+719468;let era=z/146097;let doy=z-era*146097;
    let yoe=(doy-doy/1460+doy/36524-doy/146096)/365;let day=doy-(365*yoe+yoe/4-yoe/100);let mp=(5*day+2)/153;
    let d=day-(153*mp+2)/5+1;let m=mp+if mp<10{3}else{-9};let y=yoe+era*400+i64::from(m<=2);let s=total/1000;
    format!("{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}.{:03}Z",s/3600%24,s/60%60,s%60,total%1000)
}
pub(crate) fn client()->(Core<MemoryStore>,wire::Room,wire::Preparation){
    let(mut c,_,_)=release_case();let mut room=c.room.clone().unwrap();let mut prep=c.preparation.clone().unwrap();c.clear_game();
    // Current run and authenticated actor identities use the real UUID format.
    // The old release golden intentionally used symbolic historical identifiers.
    c.epoch=uuid::Uuid::new_v4().to_string();room.run_id=Some(uuid::Uuid::new_v4().to_string());room.run_epoch=Some(uuid::Uuid::new_v4().to_string());
    {let d=c.desktop.as_mut().unwrap();d.id=uuid::Uuid::new_v4().to_string();d.epoch=uuid::Uuid::new_v4().to_string();d.account=uuid::Uuid::new_v4().to_string();d.principal.principal_id=Some(d.account.clone());prep.desktop_id=Some(d.id.clone());prep.desktop_session_epoch=Some(d.epoch.clone());for m in room.members.as_mut().unwrap(){m.desktop_id=Some(d.id.clone());m.account_id=Some(d.account.clone());}}
    prep.service_epoch=Some(c.epoch.clone());prep.run_id=room.run_id.clone();prep.run_epoch=room.run_epoch.clone();
    room.state=Some("staging".into());room.revision=Some(3);room.name=Some("Reviewed lobby".into());
    room.config.as_mut().unwrap().mode=Some("core".into());room.review_expires_at=Some(later(1800000));
    for member in room.members.as_mut().unwrap(){member.ready=Some(false);member.readiness=Some("waiting_noita".into());member.consented_config_revision=room.config_revision;member.included=Some(true);}
    c.service="nt2-g3-c2-potion-correction".into();prep.service_id=Some(c.service.clone());prep.state=Some("issued".into());prep.expires_at=Some(later(600000));prep.review_expires_at=room.review_expires_at.clone();
    c.room=None;c.info=Some(wire::Info{connection_flow_revision:Some("g3-flow-1".into()),profile:Some(wire::CompatibilityProfile::default()),..Default::default()});
    c.open_browser=false;c.enable_automatic_setup();std::fs::create_dir_all(&c.claims_root).unwrap();c.public_preferences(c.claims_root.join("servers.json")).unwrap();
    let game=std::path::PathBuf::from(std::env::var_os("NT2_FLOW_GAME_FIXTURE").expect("Run test-g3-connection-flow.ps1"));
    let repo=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..").canonicalize().unwrap();
    assert!(game.canonicalize().unwrap().starts_with(repo.join("spikes/noita-entry/.build")));
    c.preferred_game_path=Some(game);c.enable_g3(&c.claims_root.join("journal.jsonl"),Arc::new(Mutex::new(nt2_desktop_g0::g3_runtime::Exchange::default()))).unwrap();
    c.request_social(Arc::new(Mutex::new(nt2_desktop_g0::g3_social_stream::Exchange::default()))).unwrap();
    c.social_profile=Some(nt2_desktop_g0::g3_social_installation::development_profile().unwrap()["profile"].clone());
    c.setup.preview=Some(Preview{room:room.room_id.clone().unwrap(),revision:room.revision.unwrap(),config_revision:room.config_revision.unwrap(),config:room.config.clone().unwrap(),run:room.run_id.clone().zip(room.run_epoch.clone()),state:room.state.clone(),context:c.preview_context(),sequence:c.setup.preview_sequence.load(Ordering::Acquire)});
    (c,room,prep)
}
pub(crate) fn set_origin(c:&mut Core<MemoryStore>,origin:&str){c.base=origin.into();c.profile=Some(c.credential_profile(origin,&c.service).unwrap());
 // These HTTP action fixtures inject verified service timing along with their
 // synthetic credentials; the production CR2 path never falls back to UTC now.
 #[cfg(feature="g3-rejoin")] {if let Some(d)=c.desktop.as_ref(){c.lifecycle.lock().unwrap().service_time=Some(super::super::admission::ServiceTime{scope:json!([c.base,c.service,c.epoch,d.account]).to_string(),stamp:nt2_desktop_g0::connection_time::Stamp{sequence:1,started:Instant::now()},upper_ms:now()});}}
 inject_credential_timing(c);
 let context=c.preview_context();if let Some(p)=c.setup.preview.as_mut(){p.context=context;}
}
fn inject_credential_timing(c:&mut Core<MemoryStore>){
 #[cfg(feature="g3-rejoin")]{use base64::Engine;let Some(mut s)=c.desktop.clone()else{return};let id=uuid::Uuid::new_v4().to_string();s.access=SecretString::from(format!("fixture.{}.unsigned",base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(json!({"jti":id}).to_string())));c.desktop=Some(s.clone());let mut coordinator=c.lifecycle.lock().unwrap();coordinator.request_sequence+=1;let stamp=nt2_desktop_g0::connection_time::Stamp{sequence:coordinator.request_sequence,started:Instant::now()};let request=uuid::Uuid::new_v4().to_string();let v=nt2_desktop_g0::protocol_connection::wire::ConnectionCredential{principal_id:Some(s.account.clone()),session_id:Some(s.id.clone()),session_epoch:Some(s.epoch.clone()),credential_id:Some(id),origin_request_id:Some(request.clone()),sample_key:Some("e".repeat(64)),access_remaining_ms:Some(900000)};super::super::admission::credential_sample(&mut coordinator.credentials,&c.base,&c.service,&s,&v,&request,stamp).unwrap();}
 #[cfg(not(feature="g3-rejoin"))]let _=c;
}
pub(crate) fn data(room:&wire::Room,prep:&wire::Preparation)->Value{json!({"room":room,"preparation":prep})}
// Source fixture: the canonical attach/enrollment acknowledgement has settled.
// No admission/permit/release is supplied. Real service composition owns proof.
pub(crate) fn r8_settled_lineage(c:&mut Core<MemoryStore>){
 let mut model=release_case().0;let mut game=model.game.take().unwrap();let mut pending=c.pending.lock().unwrap();let p=pending.as_mut().unwrap();let b=p.startup_bundle.as_ref().unwrap();let s=b.scope();let owner=p.owner.unwrap();let attachment=uuid::Uuid::new_v4().to_string();game.owner=owner;game.session.account=s.account.clone();game.session.attachment=attachment.clone();game.instance=uuid::Uuid::new_v4().to_string();game.entered=false;game.permit=None;
 p.startup_attach=Some(nt2_desktop_g0::startup_local::AttachEvidence{request_id:uuid::Uuid::new_v4().to_string(),fingerprint:"a".repeat(64),session_id:s.desktop.clone(),session_epoch:s.desktop_epoch.clone(),service_epoch:s.service_epoch.clone(),preparation:s.preparation.clone(),attachment,instance:game.instance.clone()});
 let marker=nt2_desktop_g0::startup_local::WaitingMarker{kind:"nt2-waiting".into(),version:1,local_world:b.world(),bundle_digest:b.digest().into(),actual_seed:p.seed.to_string(),loot:format!("shared:{}:{}",u8::from(p.world_policy==5),p.seed)};drop(pending);c.game=Some(game);c.lifecycle.lock().unwrap().waiting_metadata=Some(marker);
}
#[test]fn se1_r8_preview_transition_retires_inflight_read_and_fresh_retry_uses_settled_base(){tauri::async_runtime::block_on(async{
 let(mut c,room,_)=client();let gate=c.replacement_lineage.clone();let first=replacement(&c,&room);let mut fresh=first.clone();fresh["entryPreview"]["scope"]["baseFingerprint"]=json!("d".repeat(64));let(origin,server)=network(vec![Reply::Build(Box::new(move|_|{assert!(gate.block("fixture recovery begins during preview"));first.clone()})),Reply::Entry(fresh)]);set_origin(&mut c,&origin);
 assert!(c.prepare_replacement_preview().await.unwrap_err().contains("Late replacement preview"));assert!(c.provisional.is_none());c.prepare_replacement_preview().await.unwrap();let view=super::super::entry_transport::settle(c.provisional.as_ref().unwrap()).await.unwrap();let r=nt2_desktop_g0::provisional_local::load(&super::super::rejoin::ResumeStore(c.store.clone()),&view.scope,view.world).unwrap().unwrap();assert_eq!(r.base()["baseFingerprint"],"d".repeat(64));let epoch=c.replacement_lineage.snapshot();c.desktop.as_mut().unwrap().access=SecretString::from("same-session-renewed-token");assert_eq!(c.sync_replacement_lineage(),epoch,"credential renewal must not invent a base transition");assert_eq!(server.join().unwrap().len(),2);clean(c);
});}
pub(crate) fn replacement(c:&Core<MemoryStore>,room:&wire::Room)->Value{
 let d=c.desktop.as_ref().unwrap();let config=room.config.as_ref().unwrap();let digest=nt2_desktop_g0::protocol_connection::canonical_fingerprint(&serde_json::to_vec(config).unwrap()).unwrap();
 json!({"entryPreview":{"scope":{"serviceId":c.service,"serviceEpoch":c.epoch,"accountId":d.account,"deviceId":c.device.to_string(),"roomId":room.room_id,"runId":room.run_id,"runEpoch":room.run_epoch,"configRevision":room.config_revision,"configDigest":digest,"basePreparationId":uuid::Uuid::new_v4().to_string(),"baseFingerprint":"a".repeat(64),"seedPolicy":config.seed_policy.clone().unwrap_or("shared-fixed".into()),"randomizeLoot":config.randomize_loot.unwrap_or(false),"actualSeed":config.seed.unwrap().to_string()},"sample":{"channel":"entry-http","originRequestId":"retagged","credentialId":"retagged","sampleKey":"a".repeat(64)},"reviewRemainingMs":60000,"mayPlay":false},"credentialValidity":{"principalId":d.account,"sessionId":d.id,"sessionEpoch":d.epoch,"credentialId":"retagged","originRequestId":"retagged","sampleKey":"b".repeat(64),"accessRemainingMs":900000}})
}
fn r5_review(room:&wire::Room)->Value{json!({"room":room,"admission":{"roomId":room.room_id,"revision":room.revision,"configRevision":room.config_revision}})}
fn r5_saved(c:&Core<MemoryStore>,room:&wire::Room)->nt2_desktop_g0::rejoin_local::Association{let mut a=self::continue_cf05::prior(c);a.room=room.room_id.clone().unwrap();a.run=room.run_id.clone().unwrap();self::continue_cf05::remember(c,&a);a}
#[test]fn se1_r5_confirmed_initial_join_can_review_newly_opened_staging_run(){tauri::async_runtime::block_on(async{
 let(mut c,room,prep)=client();let mut waiting=room.clone();waiting.state=Some("waiting".into());waiting.run_id=None;waiting.run_epoch=None;let(origin,server)=network(vec![Reply::Data(r5_review(&waiting)),Reply::Data(data(&room,&prep)),Reply::Data(r5_review(&room)),Reply::Entry(replacement(&c,&room))]);set_origin(&mut c,&origin);c.preview_lobby(room.room_id.as_deref().unwrap()).await.unwrap();join(&mut c).await.unwrap();assert_eq!(c.setup.preview.as_ref().unwrap().state.as_deref(),Some("waiting"));assert!(c.setup.preview.as_ref().unwrap().run.is_none());c.pending.lock().unwrap().as_mut().unwrap().owner=Some((nt2_local_ws::owner::Id::OsProcess(7),[61;16]));r8_settled_lineage(&mut c);c.entry_tick().await.unwrap();assert!(c.provisional.as_ref().and_then(|w|w.view().1).is_some());assert_eq!(c.setup.preview.as_ref().unwrap().run,room.run_id.clone().zip(room.run_epoch.clone()));assert_eq!(c.pending.lock().unwrap().as_ref().unwrap().owner,Some((crate::local_owner::Id::os_process(7),[61;16])));let calls=server.join().unwrap();assert_eq!(calls.len(),4);assert!(calls[1]["roomJoin"].is_object()&&calls[2]["roomPreview"].is_object()&&calls[3]["entryPreview"].is_object());clean(c);
});}
#[test]fn se1_r5_cold_selection_refreshes_unused_b_without_room_after_generation_or_auth_change(){tauri::async_runtime::block_on(async{
 for change in ["generation","authentication"]{
  let(mut c,room,_)=client();let initial=replacement(&c,&room);let mut updated=initial.clone();updated["entryPreview"]["scope"]["baseFingerprint"]=json!("c".repeat(64));let next_id=uuid::Uuid::new_v4().to_string();if change=="authentication"{updated["credentialValidity"]["sessionId"]=json!(next_id);}let(origin,server)=network(vec![Reply::Data(r5_review(&room)),Reply::Entry(initial),Reply::Data(r5_review(&room)),Reply::Entry(updated)]);set_origin(&mut c,&origin);let a=r5_saved(&c,&room);c.preview_lobby(&a.room).await.unwrap();join(&mut c).await.unwrap();assert!(c.room.is_none());let old=c.provisional.as_ref().unwrap().view().1.unwrap();let old_context=c.setup.preview.as_ref().unwrap().context.clone();
  let generation=c.generation.fetch_add(1,Ordering::AcqRel)+1;if change=="authentication"{let d=c.desktop.as_mut().unwrap();d.id=next_id;d.epoch=uuid::Uuid::new_v4().to_string();inject_credential_timing(&mut c);}
  let mut owner=handoff::new(424242,a.run.clone(),1);owner.owner=Some((nt2_local_ws::owner::Id::OsProcess(7),[39;16]));owner.generation=Some((c.generation.clone(),generation));*c.pending.lock().unwrap()=Some(owner);let mut recovered=release_case().0.game.take().unwrap();recovered.owner=(nt2_local_ws::owner::Id::OsProcess(7),[39;16]);recovered.session.account=c.desktop.as_ref().unwrap().account.clone();recovered.session.id=uuid::Uuid::new_v4().to_string();recovered.session.epoch=uuid::Uuid::new_v4().to_string();recovered.session.attachment=uuid::Uuid::new_v4().to_string();recovered.permit=None;recovered.entered=false;c.game=Some(recovered);assert_ne!(old_context,c.preview_context());c.entry_tick().await.unwrap();
  let current=c.provisional.as_ref().unwrap().view().1.unwrap();assert_ne!(current.world,old.world,"{change}: {:?}",c.entry.error);assert_eq!(c.setup.preview.as_ref().unwrap().context,c.preview_context());assert!(c.room.is_none());assert_eq!(c.generation.load(Ordering::Acquire),generation);assert_eq!(c.pending.lock().unwrap().as_ref().unwrap().owner,Some((crate::local_owner::Id::os_process(7),[39;16])));assert_eq!(c.resume_intent.association(),Some(&a));assert!(c.entry.error.is_none());let adapter=self::continue_cf05::StoreBridge(&c.store);let archived=nt2_desktop_g0::provisional_local::load(&adapter,&old.scope,old.world).unwrap().unwrap();assert!(archived.activation_original().is_none());
  let calls=server.join().unwrap();assert_eq!(calls.len(),4);assert!(calls[2]["roomPreview"].is_object()&&calls[3]["entryPreview"].is_object());assert!(calls.iter().all(|q|q["roomJoin"].is_null()&&q["reserve"].is_null()));clean(c);
 }
});}
#[test]fn se1_r5_changed_run_config_or_state_never_relabels_or_replaces_unused_original(){tauri::async_runtime::block_on(async{
 for mismatch in ["run","revision","configuration","running"]{
  let(mut c,room,_)=client();let mut changed=room.clone();match mismatch{"run"=>changed.run_id=Some(uuid::Uuid::new_v4().to_string()),"revision"=>changed.config_revision=Some(room.config_revision.unwrap()+1),"configuration"=>changed.config.as_mut().unwrap().seed=Some(7),_=>changed.state=Some("running".into())};let(origin,server)=network(vec![Reply::Data(r5_review(&room)),Reply::Entry(replacement(&c,&room)),Reply::Data(r5_review(&changed))]);set_origin(&mut c,&origin);let a=r5_saved(&c,&room);c.preview_lobby(&a.room).await.unwrap();join(&mut c).await.unwrap();let old=c.provisional.as_ref().unwrap().view().1.unwrap();let context=c.setup.preview.as_ref().unwrap().context.clone();c.generation.fetch_add(1,Ordering::AcqRel);c.entry_tick().await.unwrap();assert_eq!(c.provisional.as_ref().unwrap().view().1.unwrap().world,old.world);assert_eq!(c.setup.preview.as_ref().unwrap().context,context);assert!(c.entry.error.as_deref().unwrap().contains("run/configuration changed"));assert!(c.room.is_none());assert_eq!(server.join().unwrap().len(),3);clean(c);
 }
});}
#[test]fn se1_r5_late_reviews_and_native_offer_during_either_read_preserve_original(){tauri::async_runtime::block_on(async{
 for race in ["selection","generation","entry-generation","room-offer","entry-offer"]{
  let(mut c,room,_)=client();let generation=c.generation.clone();let selection=c.setup.preview_sequence.clone();let worker:Arc<Mutex<Option<Arc<nt2_desktop_g0::provisional_local::Worker>>>>=Default::default();let slot=worker.clone();let public=r5_review(&room);let entry=replacement(&c,&room);let payload=if race.starts_with("entry-"){entry.clone()}else{public.clone()};let action=race.to_owned();let late=Reply::Build(Box::new(move|_|{match action.as_str(){"selection"=>{selection.fetch_add(1,Ordering::AcqRel);},"generation"|"entry-generation"=>{generation.fetch_add(1,Ordering::AcqRel);},_=>{let w=slot.lock().unwrap().clone().unwrap();let v=w.view().1.unwrap();w.begin_fresh(v.world).unwrap();}}payload.clone()}));let mut replies=vec![Reply::Data(public.clone()),Reply::Entry(entry)];if race.starts_with("entry-"){replies.push(Reply::Data(public));}replies.push(late);let(origin,server)=network(replies);set_origin(&mut c,&origin);let a=r5_saved(&c,&room);c.preview_lobby(&a.room).await.unwrap();join(&mut c).await.unwrap();*worker.lock().unwrap()=c.provisional.clone();let old=c.provisional.as_ref().unwrap().view().1.unwrap();c.generation.fetch_add(1,Ordering::AcqRel);c.entry_tick().await.unwrap();let retained=c.provisional.as_ref().unwrap().view().1.unwrap();assert_eq!(retained.world,old.world);assert_eq!(retained.digest,old.digest);assert!(c.entry.error.is_some());let calls=server.join().unwrap();assert_eq!(calls.len(),if race.starts_with("entry-"){4}else{3});assert!(calls.iter().all(|q|q["roomJoin"].is_null()&&q["reserve"].is_null()));clean(c);
 }
});}
#[test]fn se1_r5_offered_claimed_or_uncertain_b_skips_passive_refresh(){tauri::async_runtime::block_on(async{
 for phase in ["offered","claimed","submitted"]{let(mut c,room,_)=client();let(origin,server)=network(vec![Reply::Data(r5_review(&room)),Reply::Entry(replacement(&c,&room))]);set_origin(&mut c,&origin);let a=r5_saved(&c,&room);c.preview_lobby(&a.room).await.unwrap();join(&mut c).await.unwrap();server.join().unwrap();let worker=c.provisional.clone().unwrap();let old=worker.view().1.unwrap();worker.begin_fresh(old.world).unwrap();if phase!="offered"{worker.claim(old.world,nt2_local_ws::owner::Id::OsProcess(7),100).unwrap();super::super::entry_transport::settle(&worker).await.unwrap();}if phase=="submitted"{let r=nt2_desktop_g0::provisional_local::load(&self::continue_cf05::StoreBridge(&c.store),&old.scope,old.world).unwrap().unwrap();let q=json!({"major":1,"revision":nt2_desktop_g0::protocol_waiting::REVISION,"serviceId":c.service,"serviceEpoch":c.epoch,"sessionEpoch":c.desktop.as_ref().unwrap().epoch,"requestId":uuid::Uuid::new_v4().to_string(),"entryActivate":{"scope":r.entry_scope().unwrap(),"identity":r.entry_identity(),"recoveryProof":r.future_secret().as_str(),"gameVersion":"fixture","modVersion":"fixture","bridgeVersion":"fixture","capabilities":["core.seed"],"observation":{"gameInstanceId":r.game_instance().to_string(),"worldToken":old.world.simple().to_string(),"observationRevision":1,"actualSeed":old.seed.to_string(),"activeMods":["noita_together2"],"inventoryComplete":true,"beta":"false","protected":true,"loaded":true,"seedStatus":"known","modVersion":"fixture","bridgeVersion":"fixture","compatibilityProvenance":"game-thread-client-observation"}}});worker.seal_activation(old.world,serde_json::to_vec(&q).unwrap(),c.desktop.as_ref().unwrap().id.clone()).unwrap();super::super::entry_transport::settle(&worker).await.unwrap();}let before=nt2_desktop_g0::provisional_local::load(&self::continue_cf05::StoreBridge(&c.store),&old.scope,old.world).unwrap().unwrap().activation_original();c.generation.fetch_add(1,Ordering::AcqRel);c.entry_tick().await.unwrap();let refused=c.prepare_replacement_preview().await.unwrap_err();assert!(refused.contains("Original fresh entry retained"));assert_eq!(worker.view().1.unwrap().world,old.world);assert_eq!(worker.view().1.unwrap().digest,old.digest);assert_eq!(worker.view().1.unwrap().phase,match phase{"offered"=>nt2_desktop_g0::provisional_local::Phase::Staged,"claimed"=>nt2_desktop_g0::provisional_local::Phase::Claimed,_=>nt2_desktop_g0::provisional_local::Phase::ActivationSubmitted});assert_eq!(nt2_desktop_g0::provisional_local::load(&self::continue_cf05::StoreBridge(&c.store),&old.scope,old.world).unwrap().unwrap().activation_original(),before);clean(c);}
});}
async fn join(c:&mut Core<MemoryStore>)->Result<(),String>{let p=c.setup.preview.clone().unwrap();c.join_displayed(&p.room,None,p.revision,p.config_revision).await}
fn wake(c:&mut Core<MemoryStore>){c.setup.intent.as_mut().unwrap().next=Instant::now();}
pub(crate) fn retain_waiting_setup(c:&mut Core<MemoryStore>,s:&nt2_desktop_g0::startup_local::Scope){
 c.setup.enabled=true;let d=c.desktop.as_ref().unwrap();
 c.setup.intent=Some(Intent{review_context:c.preview_context(),review_sequence:c.setup.preview_sequence.load(Ordering::Acquire),generation:c.generation.load(Ordering::Acquire),origin:c.base.clone(),service:c.service.clone(),epoch:c.epoch.clone(),desktop:d.id.clone(),session:d.epoch.clone(),room:s.room.clone(),config_revision:s.config.try_into().unwrap(),config:serde_json::to_vec(c.room.as_ref().unwrap().config.as_ref().unwrap()).unwrap(),run:Some((s.run.clone(),s.run_epoch.clone())),operation:None,renewing:false,renewal_baseline:None,renewal_sent:false,next:Instant::now(),failures:0,blocked:false,entered:false});
}
// Current immutable complete/native facts injected directly; this fixture does
// not use the obsolete synchronous v2 claim to manufacture local consumption.
fn se1_consumed()->(Core<MemoryStore>,Intent){
 let(mut c,room,mut prep)=client();set_origin(&mut c,"https://se1.invalid");let desktop=c.desktop.as_ref().unwrap();
 let intent=Intent{review_context:c.preview_context(),review_sequence:c.setup.preview_sequence.load(Ordering::Acquire),generation:c.generation.load(Ordering::Acquire),origin:c.base.clone(),service:c.service.clone(),epoch:c.epoch.clone(),desktop:desktop.id.clone(),session:desktop.epoch.clone(),room:room.room_id.clone().unwrap(),config_revision:room.config_revision.unwrap(),config:serde_json::to_vec(room.config.as_ref().unwrap()).unwrap(),run:room.run_id.clone().zip(room.run_epoch.clone()),operation:None,renewing:false,renewal_baseline:None,renewal_sent:false,next:Instant::now(),failures:0,blocked:false,entered:false};
 let mut pending=handoff::new(room.config.as_ref().unwrap().seed.unwrap(),room.run_id.clone().unwrap(),u64::from(room.config_revision.unwrap()));pending.epoch=room.run_epoch.clone().unwrap();pending.preparation_id=prep.preparation_id.clone();pending.owner=Some((nt2_local_ws::owner::Id::OsProcess(7),[1;16]));pending.accepted=true;pending.loaded=true;pending.actual_seed=Some(u64::from(pending.seed));pending.generation=Some((c.generation.clone(),intent.generation));pending.authority_until=Instant::now()+Duration::from_secs(6);
 pending.native_v2=true;pending.world_policy=seed_policy::resolved(room.config.as_ref().unwrap()).unwrap().bits;pending.last_seen=Some(Instant::now());
 *c.pending.lock().unwrap()=Some(pending);prep.state=Some("consumed".into());c.preparation=Some(prep);c.room=Some(room);(c,intent)
}
#[test]fn se1_consumed_ambiguous_attachment_retains_exact_original_and_held_world(){tauri::async_runtime::block_on(async{
 let(mut c,intent)=se1_consumed();c.setup.intent=Some(intent);let owner=c.pending.lock().unwrap().as_ref().unwrap().owner;let mut request=c.request(false);request.game_attach=Some(wire::GameAttach{game_instance_id:Some(uuid::Uuid::new_v4().to_string()),..Default::default()});let original=serde_json::to_value(&request).unwrap();
 c.attach_retry=Some(Retry{request,until:Instant::now()-Duration::from_secs(1)});c.attach_cycle=Some(("original-instance".into(),Instant::now()-Duration::from_secs(1)));
 assert!(c.game_tick().await.unwrap_err().contains("explicit restart required"));assert_eq!(serde_json::to_value(&c.setup.original.as_ref().unwrap().request).unwrap(),original);assert!(c.setup.intent.as_ref().unwrap().blocked);
 let pending=c.pending.lock().unwrap().clone().unwrap();assert_eq!(pending.owner,owner);assert!(!pending.release);assert!(c.game.is_none());clean(c);
});}
#[test]fn se1_consumed_original_retires_unsent_renewal_without_network_or_replacement(){tauri::async_runtime::block_on(async{
 let(mut c,mut intent)=se1_consumed();let original=c.pending.lock().unwrap().clone().unwrap();let mut request=c.request(false);request.preparation_renew=Some(wire::PreparationRenew{preparation_id:original.preparation_id.clone()});intent.operation=Some(Retry{request,until:Instant::now()+Duration::from_secs(30)});intent.renewing=true;intent.renewal_sent=false;
 c.setup_step(&mut intent).await.unwrap();assert!(intent.operation.is_none()&&!intent.renewing&&!intent.blocked);let current=c.pending.lock().unwrap().clone().unwrap();assert_eq!(current.preparation_id,original.preparation_id);assert_eq!(current.owner,original.owner);assert_eq!(current.actual_seed,original.actual_seed);assert!(!current.release);clean(c);
});}
#[test]fn se1_consumed_renewal_snapshot_cannot_substitute_run_original_or_session(){
 let(c,intent)=se1_consumed();let room=c.room.clone().unwrap();let prep=c.preparation.clone().unwrap();let response:wire::Response=serde_json::from_value(data(&room,&prep)).unwrap();assert!(c.validate_renewal_snapshot(&intent,&response).is_ok());
 for fault in ["preparation","run","session","membership","terminal"]{let mut response=response.clone();match fault{
  "preparation"=>response.preparation.as_mut().unwrap().preparation_id=Some(uuid::Uuid::new_v4().to_string()),
  "run"=>response.room.as_mut().unwrap().run_epoch=Some(uuid::Uuid::new_v4().to_string()),
  "session"=>response.preparation.as_mut().unwrap().desktop_session_epoch=Some(uuid::Uuid::new_v4().to_string()),
  "membership"=>response.room.as_mut().unwrap().members=Some(vec![]),
  "terminal"=>response.preparation.as_mut().unwrap().state=Some("cancelled".into()),_=>unreachable!()}
  assert!(c.validate_renewal_snapshot(&intent,&response).is_err(),"{fault}");
 }clean(c);
}
pub(crate) fn clean(c:Core<MemoryStore>){
    let root=c.claims_root.canonicalize().unwrap();assert!(root.starts_with(std::env::temp_dir().canonicalize().unwrap())&&root.file_name().unwrap().to_string_lossy().starts_with("nt2-next-test-"));drop(c);std::fs::remove_dir_all(root).unwrap();
}
fn frame(op:u32,field:u32)->[u8;32]{let mut v=[1;32];v[..4].copy_from_slice(&handoff::MAGIC.to_le_bytes());v[4..8].copy_from_slice(&2u32.to_le_bytes());v[8..12].copy_from_slice(&op.to_le_bytes());v[12..16].copy_from_slice(&field.to_le_bytes());v}
#[cfg(feature="g3-rejoin")]
#[test]
fn se1_reauthentication_preview_preserves_saved_owner_before_room_join(){tauri::async_runtime::block_on(async{
    use self::continue_cf05::{prior,remember};
    let(mut c,room,prep)=client();let mut response=data(&room,&prep);response["admission"]=json!({"roomId":room.room_id,"revision":room.revision,"configRevision":room.config_revision});
    let replacement=replacement(&c,&room);let(origin,worker)=network(vec![Reply::Data(response),Reply::Entry(replacement)]);set_origin(&mut c,&origin);
    let mut a=prior(&c);a.room=room.room_id.clone().unwrap();a.run=room.run_id.clone().unwrap();remember(&c,&a);
    // New authenticated desktop session, same account/device: index is a selector,
    // never credentials/authority and never a reason for fresh roomJoin.
    c.desktop.as_mut().unwrap().epoch=uuid::Uuid::new_v4().to_string();inject_credential_timing(&mut c);
    c.preview_lobby(&a.room).await.unwrap();join(&mut c).await.unwrap();
    assert_eq!(c.resume_intent.association(),Some(&a));assert!(c.setup.intent.is_none()&&c.pending.lock().unwrap().is_none());
    let calls=worker.join().unwrap();assert_eq!(calls.len(),2);assert!(calls[0]["roomPreview"].is_object()&&calls[1]["entryPreview"].is_object());assert!(calls.iter().all(|q|q["roomJoin"].is_null()&&q["reserve"].is_null()));clean(c);
});}
#[test]fn se1_r4_cold_running_lobby_selection_keeps_saved_continue_without_fresh_mutation(){tauri::async_runtime::block_on(async{
 use self::continue_cf05::{prior,remember};let(mut c,mut room,prep)=client();room.state=Some("running".into());let mut response=data(&room,&prep);response["admission"]=json!({"roomId":room.room_id,"revision":room.revision,"configRevision":room.config_revision});let(origin,server)=network(vec![Reply::Data(response)]);set_origin(&mut c,&origin);let mut a=prior(&c);a.room=room.room_id.clone().unwrap();a.run=room.run_id.clone().unwrap();remember(&c,&a);assert!(c.room.is_none()&&c.resume_intent.association().is_none());let generation=c.generation.load(Ordering::Acquire);
 c.preview_lobby(&a.room).await.unwrap();join(&mut c).await.unwrap();assert_eq!(c.resume_intent.association(),Some(&a));assert_eq!(c.setup_view()["stage"],"continue_world");assert!(c.provisional.is_none()&&c.setup.intent.is_none()&&c.pending.lock().unwrap().is_none());assert_eq!(c.generation.load(Ordering::Acquire),generation);let calls=server.join().unwrap();assert_eq!(calls.len(),1);assert!(calls[0]["roomPreview"].is_object());clean(c);
});}
#[cfg(feature="g3-rejoin")]
#[test]
fn se1_loaded_waiting_selector_blocks_reauthenticated_join_without_new_preparation(){tauri::async_runtime::block_on(async{
    use nt2_desktop_g0::startup_local as startup;use self::continue_cf05::StoreBridge;
    let(mut c,room,prep)=client();let mut public=data(&room,&prep);public["admission"]=json!({"roomId":room.room_id,"revision":room.revision,"configRevision":room.config_revision});let(origin,worker)=network(vec![Reply::Data(public),Reply::Entry(replacement(&c,&room))]);set_origin(&mut c,&origin);let d=c.desktop.as_ref().unwrap();
    let scope=startup::Scope{origin:c.base.clone(),service:c.service.clone(),service_epoch:c.epoch.clone(),account:d.account.clone(),device:c.device,desktop:d.id.clone(),desktop_epoch:d.epoch.clone(),room:room.room_id.clone().unwrap(),run:room.run_id.clone().unwrap(),run_epoch:room.run_epoch.clone().unwrap(),config:room.config_revision.unwrap().into(),config_digest:"a".repeat(64),preparation:uuid::Uuid::new_v4().to_string()};
    let bundle=startup::Prepared::stage(scope.clone(),424242,5,1,Instant::now()+Duration::from_secs(60)).unwrap();let mut claim=7u32.to_le_bytes().to_vec();claim.extend([7;16]);
    let marker=startup::persist(&StoreBridge(&c.store),&bundle,424242,nt2_local_ws::owner::Id::OsProcess(7),[7;16],&claim).unwrap();
    c.desktop.as_mut().unwrap().epoch=uuid::Uuid::new_v4().to_string();
    c.preview_lobby(room.room_id.as_deref().unwrap()).await.unwrap();join(&mut c).await.unwrap();assert!(c.setup.intent.is_none()&&c.pending.lock().unwrap().is_none()&&c.preparation.is_none());assert_eq!(worker.join().unwrap().len(),2);
    assert!(startup::recall(&StoreBridge(&c.store),&scope.origin,&scope.service,&scope.account,scope.device,&marker).unwrap().is_some());clean(c);
});}
#[test]fn se1_current_new_run_preview_archives_old_selector_then_ordinary_join(){tauri::async_runtime::block_on(async{
 use nt2_desktop_g0::startup_local as startup;use self::continue_cf05::StoreBridge;
 let(mut c,room,prep)=client();let mut public=data(&room,&prep);public["admission"]=json!({"roomId":room.room_id,"revision":room.revision,"configRevision":room.config_revision});let(origin,server)=network(vec![Reply::Data(public),Reply::Data(data(&room,&prep))]);set_origin(&mut c,&origin);let d=c.desktop.as_ref().unwrap();
 let old=startup::Scope{origin:origin.clone(),service:c.service.clone(),service_epoch:c.epoch.clone(),account:d.account.clone(),device:c.device,desktop:uuid::Uuid::new_v4().to_string(),desktop_epoch:uuid::Uuid::new_v4().to_string(),room:room.room_id.clone().unwrap(),run:uuid::Uuid::new_v4().to_string(),run_epoch:uuid::Uuid::new_v4().to_string(),config:1,config_digest:"a".repeat(64),preparation:uuid::Uuid::new_v4().to_string()};let bundle=startup::Prepared::stage(old.clone(),424242,1,1,Instant::now()+Duration::from_secs(60)).unwrap();let mut claim=7u32.to_le_bytes().to_vec();claim.extend([7;16]);let marker=startup::persist(&StoreBridge(&c.store),&bundle,424242,nt2_local_ws::owner::Id::OsProcess(7),[7;16],&claim).unwrap();
 c.preview_lobby(room.room_id.as_deref().unwrap()).await.unwrap();assert!(!startup::has_recoverable_room(&StoreBridge(&c.store),&origin,&c.service,&old.account,c.device,&old.room).unwrap());join(&mut c).await.unwrap();
 assert!(startup::recall(&StoreBridge(&c.store),&origin,&c.service,&old.account,c.device,&marker).unwrap().is_some());assert!(c.provisional.is_none());let calls=server.join().unwrap();assert_eq!(calls.len(),2);assert!(calls[0]["roomPreview"].is_object()&&calls[1]["roomJoin"].is_object());assert!(calls.iter().all(|q|q["resume"].is_null()&&q["pendingOffline"].is_null()&&q["entryCancel"].is_null()&&q["entryPreview"].is_null()));clean(c);
});}
#[test]
fn se1_stale_setup_does_not_revoke_recovered_owner(){tauri::async_runtime::block_on(async{
    let(mut c,room,prep)=client();let(origin,worker)=network(vec![Reply::Data(data(&room,&prep))]);set_origin(&mut c,&origin);join(&mut c).await.unwrap();worker.join().unwrap();
    let generation=c.generation.fetch_add(1,Ordering::AcqRel)+1;
    {let mut pending=c.pending.lock().unwrap();let p=pending.as_mut().unwrap();p.owner=Some((nt2_local_ws::owner::Id::OsProcess(7),[91;16]));p.loaded=true;p.accepted=true;p.generation=Some((c.generation.clone(),generation));}
    c.native_revoked=false;wake(&mut c);c.setup_tick().await.unwrap();
    assert!(!c.native_revoked);assert_eq!(c.pending.lock().unwrap().as_ref().unwrap().owner,Some((crate::local_owner::Id::os_process(7),[91;16])));clean(c);
});}
#[test]
fn se1_late_room_join_reply_cannot_absorb_over_recovered_owner(){tauri::async_runtime::block_on(async{
    let(mut c,room,prep)=client();let(other,mut p,_)=release_case();std::fs::create_dir_all(&other.claims_root).unwrap();clean(other);
    p.owner=Some((nt2_local_ws::owner::Id::OsProcess(7),[92;16]));p.loaded=true;p.accepted=true;p.generation=Some((c.generation.clone(),c.generation.load(Ordering::Acquire)+2));
    let(origin,worker)=network(vec![Reply::Recovered(data(&room,&prep),c.generation.clone(),c.pending.clone(),p)]);set_origin(&mut c,&origin);
    c.native_revoked=false;assert!(join(&mut c).await.unwrap_err().contains("Late setup"));worker.join().unwrap();
    assert!(!c.native_revoked);assert_eq!(c.pending.lock().unwrap().as_ref().unwrap().owner,Some((crate::local_owner::Id::os_process(7),[92;16])));assert!(c.room.is_none());clean(c);
});}
#[test]
fn flow01_discovery_skips_invalid_candidate_and_preview_requires_configuration(){tauri::async_runtime::block_on(async{
    let(mut c,room,_)=client();c.preferred_game_path=None;
    c.discover_candidates(vec![c.claims_root.join("missing-game"),std::path::PathBuf::from(std::env::var_os("NT2_FLOW_GAME_FIXTURE").unwrap())]);
    assert!(c.game_path.is_some());
    let(origin,worker)=network(vec![Reply::Data(json!({"admission":{"roomId":room.room_id,"revision":room.revision,"configRevision":room.config_revision}}))]);
    set_origin(&mut c,&origin);c.setup.preview=None;assert!(c.preview_lobby(room.room_id.as_deref().unwrap()).await.unwrap_err().contains("configuration"));
    assert!(c.setup.preview.is_none()&&c.setup.intent.is_none());worker.join().unwrap();clean(c);
});}
#[test]
fn flow01_start_requires_complete_roster_readiness_and_captured_report(){tauri::async_runtime::block_on(async{
 for case in ["ready","offline","membership","config","report","excluded"]{
  let(mut c,p)=super::super::start::tests::fixture();let mut room=c.room.clone().unwrap();let config=room.config_revision.unwrap();let mut report=c.inspections[0].clone();let mut excluded=vec![];
  if case=="excluded"{room.members.as_mut().unwrap().push(wire::Member{desktop_id:Some("slow-desktop".into()),account_id:Some("slow-account".into()),readiness:Some("offline".into()),..Default::default()});excluded.push("slow-desktop".into());c.room=Some(room.clone());}
  if case=="offline"{room.members.as_mut().unwrap()[0].ready=Some(false);}
  if case=="membership"{room.members.as_mut().unwrap().clear();}
  if case=="config"{room.config_revision=Some(config+1);}
  if case=="report"{report.content_digest=Some("b".repeat(64));}
  let success=matches!(case,"ready"|"excluded");let mut replies=vec![Reply::Data(json!({"room":room}))];
  if !matches!(case,"membership"|"config"){replies.push(Reply::Data(json!({"inspections":[report]})));if case!="report"{replies.push(Reply::Data(json!({"room":room})));}}
  if success{replies.push(Reply::Data(json!({})));}
  let count=replies.len();let(origin,worker)=network(replies);c.base=origin;let result=c.start_displayed(p.clone(),excluded.clone(),config).await;assert_eq!(result.is_ok(),success,"{case}: {result:?}");let calls=worker.join().unwrap();assert_eq!(calls.len(),count);if success{assert_eq!(calls.last().unwrap()["finalStart"]["participants"][0],json!(p[0]));assert_eq!(calls.last().unwrap()["finalStart"]["excludedDesktopIds"],json!(excluded));}
 }
});}
#[test]
fn flow01_join_arms_without_ready_prepare_load_and_native_seed_is_guarded(){tauri::async_runtime::block_on(async{
    let(mut c,room,prep)=client();let mut preview=data(&room,&prep);preview["admission"]=json!({"roomId":room.room_id,"revision":room.revision,"configRevision":room.config_revision});
    let(origin,worker)=network(vec![Reply::Data(preview),Reply::Data(data(&room,&prep))]);set_origin(&mut c,&origin);
    c.setup.preview=None;c.preview_lobby(room.room_id.as_deref().unwrap()).await.unwrap();join(&mut c).await.unwrap();
    assert_eq!(c.setup_view()["stage"],"waiting_noita");assert!(c.game_path.is_some());let id=c.pending.lock().unwrap().as_ref().unwrap().preparation_id.clone();
    // Refresh/repeat Join uses the same Core intent and grant; no preparation mutation.
    join(&mut c).await.unwrap();wake(&mut c);c.setup_tick().await.unwrap();assert_eq!(c.pending.lock().unwrap().as_ref().unwrap().preparation_id,id);
    let seed=room.config.as_ref().unwrap().seed.unwrap();let reply=handoff::transition(&mut c.pending.lock().unwrap(),nt2_local_ws::owner::Id::OsProcess(7),&frame(1,0));assert_eq!(u32::from_le_bytes(reply[12..16].try_into().unwrap()),seed);
    handoff::transition(&mut c.pending.lock().unwrap(),nt2_local_ws::owner::Id::OsProcess(7),&frame(3,seed));handoff::transition(&mut c.pending.lock().unwrap(),nt2_local_ws::owner::Id::OsProcess(7),&frame(4,1));
    assert_eq!(c.setup_view()["stage"],"checking_report");assert_eq!(c.room.as_ref().unwrap().members.as_ref().unwrap()[0].ready,Some(false));
    let calls=worker.join().unwrap();assert_eq!(calls.len(),2);assert!(calls.iter().all(|r|r["prepare"].is_null()&&r["ready"].is_null()));clean(c);
});}
#[test]
fn flow01_ambiguous_join_replays_exact_identity_and_expired_reply_recovers_read_only(){tauri::async_runtime::block_on(async{
    for expire in [false,true]{let(mut c,room,prep)=client();let(origin,worker)=network(vec![Reply::Lost,Reply::Data(data(&room,&prep))]);set_origin(&mut c,&origin);
        join(&mut c).await.unwrap_err();let original=serde_json::to_value(&c.setup.intent.as_ref().unwrap().operation.as_ref().unwrap().request).unwrap();
        if expire{c.setup.intent.as_mut().unwrap().operation.as_mut().unwrap().until=Instant::now()-Duration::from_secs(1);}
        wake(&mut c);c.setup_tick().await.unwrap();assert!(c.setup.intent.is_some()&&c.pending.lock().unwrap().is_some());
        let calls=worker.join().unwrap();assert_eq!(calls[0],original);if expire{assert!(calls[1]["roomGet"].is_object()&&calls[1]["roomJoin"].is_null());}else{assert_eq!(calls[1],original);}clean(c);
    }
});}
#[test]
fn flow01_definite_stale_join_refreshes_same_config_before_new_request(){tauri::async_runtime::block_on(async{
    let(mut c,mut room,prep)=client();room.revision=Some(4);let(origin,worker)=network(vec![Reply::Error("STALE_REVISION"),Reply::Data(data(&room,&prep)),Reply::Data(data(&room,&prep))]);set_origin(&mut c,&origin);
    join(&mut c).await.unwrap_err();wake(&mut c);c.setup_tick().await.unwrap();let calls=worker.join().unwrap();
    assert!(calls[1]["roomPreview"].is_object());assert_ne!(calls[0]["requestId"],calls[2]["requestId"]);assert_eq!(calls[2]["expectedRevision"],4);assert_eq!(calls[0]["configRevision"],calls[2]["configRevision"]);clean(c);
});}
#[test]
fn flow01_waiting_renewal_preserves_same_preparation_and_consumed_world_is_not_renewed(){tauri::async_runtime::block_on(async{
    let(mut c,room,prep)=client();let mut renewed=prep.clone();renewed.expires_at=Some(later(700000));
    let(origin,worker)=network(vec![Reply::Data(data(&room,&prep)),Reply::Data(data(&room,&renewed))]);set_origin(&mut c,&origin);join(&mut c).await.unwrap();
    c.preparation.as_mut().unwrap().expires_at=Some(later(240000));wake(&mut c);c.setup_tick().await.unwrap();let id=c.preparation.as_ref().unwrap().preparation_id.clone();
    wake(&mut c);c.setup_tick().await.unwrap();assert_eq!(c.preparation.as_ref().unwrap().preparation_id,id);assert_eq!(c.pending.lock().unwrap().as_ref().unwrap().preparation_id,id);
    handoff::transition(&mut c.pending.lock().unwrap(),nt2_local_ws::owner::Id::OsProcess(7),&frame(1,0));c.preparation.as_mut().unwrap().state=Some("consumed".into());c.preparation.as_mut().unwrap().expires_at=Some(later(100000));
    wake(&mut c);c.setup_tick().await.unwrap();assert!(c.setup.intent.as_ref().unwrap().operation.is_none());let calls=worker.join().unwrap();assert_eq!(calls[1]["preparationRenew"]["preparationId"],id.unwrap());clean(c);
});}
#[test]
fn flow01_uncertain_renewal_reconciles_read_only_then_expires_without_replacement(){tauri::async_runtime::block_on(async{
    let(mut c,room,prep)=client();let mut waiting=prep.clone();waiting.expires_at=Some(later(240000));let mut expired=waiting.clone();expired.expires_at=Some(later(-1000));
    let(origin,worker)=network(vec![Reply::Data(data(&room,&prep)),Reply::Lost,Reply::Data(data(&room,&waiting)),Reply::Data(data(&room,&expired))]);set_origin(&mut c,&origin);join(&mut c).await.unwrap();
    c.preparation=Some(waiting);wake(&mut c);c.setup_tick().await.unwrap();wake(&mut c);c.setup_tick().await.unwrap_err();
    let original=c.setup.intent.as_ref().unwrap().operation.as_ref().unwrap().request.request_id.clone();c.setup.intent.as_mut().unwrap().operation.as_mut().unwrap().until=Instant::now()-Duration::from_secs(1);
    wake(&mut c);assert!(c.setup_tick().await.unwrap_err().contains("uncertain"));assert_eq!(c.setup.intent.as_ref().unwrap().operation.as_ref().unwrap().request.request_id,original);
    wake(&mut c);assert!(c.setup_tick().await.unwrap_err().contains("expiry"));assert!(c.setup.intent.as_ref().unwrap().blocked);assert!(c.pending.lock().unwrap().is_none());
    let calls=worker.join().unwrap();assert!(calls[1]["preparationRenew"].is_object());assert!(calls[2]["roomGet"].is_object()&&calls[3]["roomGet"].is_object());clean(c);
});}
#[test]
fn flow01_changed_config_late_reply_and_continue_stop_automatic_work(){tauri::async_runtime::block_on(async{
    let(mut c,room,prep)=client();let(origin,worker)=network(vec![Reply::Fence(data(&room,&prep),c.generation.clone())]);set_origin(&mut c,&origin);join(&mut c).await.unwrap_err();assert!(c.setup.intent.is_none()&&c.pending.lock().unwrap().is_none());worker.join().unwrap();clean(c);
    for continuation in [false,true]{let(mut c,room,prep)=client();let(origin,worker)=network(vec![Reply::Data(data(&room,&prep))]);set_origin(&mut c,&origin);join(&mut c).await.unwrap();worker.join().unwrap();
        if continuation{handoff::transition(&mut c.pending.lock().unwrap(),nt2_local_ws::owner::Id::OsProcess(7),&frame(2,0));c.native_diagnostics.lock().unwrap().fixture_continue();}
        else{c.room.as_mut().unwrap().config_revision=Some(room.config_revision.unwrap()+1);}
        wake(&mut c);c.setup_tick().await.unwrap_err();assert_eq!(c.setup_view()["stage"],"attention");assert_eq!(c.setup_view()["blocked"],true);assert!(c.pending.lock().unwrap().is_none());clean(c);
    }
});}
#[test]
fn flow01_expired_ambiguous_attachment_keeps_intent_and_original_identity_without_replacement(){tauri::async_runtime::block_on(async{
    let(mut c,room,prep)=client();let(origin,worker)=network(vec![Reply::Data(data(&room,&prep))]);set_origin(&mut c,&origin);join(&mut c).await.unwrap();worker.join().unwrap();
    handoff::transition(&mut c.pending.lock().unwrap(),nt2_local_ws::owner::Id::OsProcess(7),&frame(1,0));let seed=room.config.as_ref().unwrap().seed.unwrap();handoff::transition(&mut c.pending.lock().unwrap(),nt2_local_ws::owner::Id::OsProcess(7),&frame(3,seed));handoff::transition(&mut c.pending.lock().unwrap(),nt2_local_ws::owner::Id::OsProcess(7),&frame(4,1));
    let mut request=c.request(false);request.game_attach=Some(wire::GameAttach{game_instance_id:Some(uuid::Uuid::new_v4().to_string()),..Default::default()});let request_id=request.request_id.clone();
    c.attach_retry=Some(Retry{request,until:Instant::now()-Duration::from_secs(1)});c.attach_cycle=Some(("immutable-instance".into(),Instant::now()-Duration::from_secs(1)));
    assert!(c.game_tick().await.unwrap_err().contains("explicit restart required"));assert_eq!(c.setup.original.as_ref().unwrap().request.request_id,request_id);assert!(c.setup.intent.is_some());assert!(c.setup_view()["blocked"].as_bool().unwrap());assert!(!c.pending.lock().unwrap().as_ref().unwrap().release);
    assert!(join(&mut c).await.unwrap_err().contains("already consumed"));clean(c);
});}

fn consume(c:&mut Core<MemoryStore>,room:&wire::Room){
 handoff::transition(&mut c.pending.lock().unwrap(),nt2_local_ws::owner::Id::OsProcess(7),&frame(1,0));
 handoff::transition(&mut c.pending.lock().unwrap(),nt2_local_ws::owner::Id::OsProcess(7),&frame(3,room.config.as_ref().unwrap().seed.unwrap()));
 handoff::transition(&mut c.pending.lock().unwrap(),nt2_local_ws::owner::Id::OsProcess(7),&frame(4,1));
}
fn attached_credentials(c:&Core<MemoryStore>)->Value{
 json!({"credentials":{"accessToken":"a".repeat(43),"refreshSecret":"r".repeat(43),"accountId":c.desktop.as_ref().unwrap().account,"sessionId":"original-game-session","sessionEpoch":"original-game-epoch","attachmentId":"original-attachment","kind":"game","deviceId":c.device.to_string(),"expiresAt":later(900000),"principal":c.desktop.as_ref().unwrap().principal}})
}
#[test]
fn flow01_fix01_unsent_renewal_is_retired_and_lost_attachment_recovers_original(){tauri::async_runtime::block_on(async{
 let(mut c,room,prep)=client();let credentials=attached_credentials(&c);
 let(origin,worker)=network(vec![Reply::Data(data(&room,&prep)),Reply::Data(data(&room,&prep)),Reply::Lost,Reply::Data(credentials)]);
 set_origin(&mut c,&origin);join(&mut c).await.unwrap();c.preparation.as_mut().unwrap().expires_at=Some(later(240000));wake(&mut c);c.setup_tick().await.unwrap();
 assert!(c.setup.intent.as_ref().unwrap().renewing);consume(&mut c,&room);
 c.game_tick().await.unwrap_err();let original=serde_json::to_value(&c.attach_retry.as_ref().unwrap().request).unwrap();
 wake(&mut c);c.setup_tick().await.unwrap();assert!(!c.setup.intent.as_ref().unwrap().blocked);assert!(c.setup.intent.as_ref().unwrap().operation.is_none());
 assert_eq!(serde_json::to_value(&c.attach_retry.as_ref().unwrap().request).unwrap(),original);
 c.game_tick().await.unwrap();assert_eq!(c.game.as_ref().unwrap().session.attachment,"original-attachment");assert!(c.pending.lock().unwrap().as_ref().unwrap().owner.is_some());
 let calls=worker.join().unwrap();assert_eq!(calls[2],calls[3]);assert!(calls.iter().all(|r|r["preparationRenew"].is_null()&&r["prepare"].is_null()));clean(c);
});}
#[test]
fn flow01_fix01_uncertain_renewal_consumption_reconciles_same_scope_without_replay(){tauri::async_runtime::block_on(async{
 for state in ["issued","consumed","loaded"]{
 let(mut c,room,prep)=client();let mut consumed=prep.clone();consumed.state=Some(state.into());
 let(origin,worker)=network(vec![Reply::Data(data(&room,&prep)),Reply::Lost,Reply::Data(data(&room,&consumed))]);set_origin(&mut c,&origin);join(&mut c).await.unwrap();
 c.preparation.as_mut().unwrap().expires_at=Some(later(240000));wake(&mut c);c.setup_tick().await.unwrap();wake(&mut c);c.setup_tick().await.unwrap_err();
 consume(&mut c,&room);let mut request=c.request(false);request.game_attach=Some(wire::GameAttach{game_instance_id:Some("immutable-original".into()),..Default::default()});let original=serde_json::to_value(&request).unwrap();c.attach_retry=Some(Retry{request,until:Instant::now()+Duration::from_secs(30)});
 wake(&mut c);c.setup_tick().await.unwrap();assert!(!c.native_revoked&&c.setup.intent.as_ref().unwrap().operation.is_none());assert_eq!(serde_json::to_value(&c.attach_retry.as_ref().unwrap().request).unwrap(),original);
 let calls=worker.join().unwrap();assert!(calls[1]["preparationRenew"].is_object());assert!(calls[2]["roomGet"].is_object()&&calls[2]["preparationRenew"].is_null());clean(c);
 }
});}
#[test]
fn flow01_fix01_consumed_reconciliation_refuses_expiry_terminal_and_mismatched_scope(){tauri::async_runtime::block_on(async{
 for fault in ["expired","cancelled","issued-expiry","review","id","run","service","session","config","membership"]{
 let(mut c,room,prep)=client();let mut snapshot_room=room.clone();let mut snapshot_prep=prep.clone();snapshot_prep.state=Some("consumed".into());
 match fault{
 "expired"|"cancelled"=>snapshot_prep.state=Some(fault.into()),
 "issued-expiry"=>{snapshot_prep.state=Some("issued".into());snapshot_prep.expires_at=Some(later(-1000));},
 "review"=>snapshot_prep.review_expires_at=Some(later(-1000)),
 "id"=>snapshot_prep.preparation_id=Some(uuid::Uuid::new_v4().to_string()),
 "run"=>snapshot_room.run_epoch=Some("replacement".into()),"service"=>snapshot_prep.service_id=Some("another".into()),
 "session"=>snapshot_prep.desktop_session_epoch=Some("another".into()),"config"=>snapshot_room.config.as_mut().unwrap().seed=Some(99),
 _=>snapshot_room.members=Some(vec![]),
 }
 let(origin,worker)=network(vec![Reply::Data(data(&room,&prep)),Reply::Lost,Reply::Data(data(&snapshot_room,&snapshot_prep))]);set_origin(&mut c,&origin);join(&mut c).await.unwrap();
 c.preparation.as_mut().unwrap().expires_at=Some(later(240000));wake(&mut c);c.setup_tick().await.unwrap();wake(&mut c);c.setup_tick().await.unwrap_err();consume(&mut c,&room);
 wake(&mut c);assert!(c.setup_tick().await.is_err(),"{fault}");assert!(c.setup.intent.as_ref().unwrap().blocked&&c.native_revoked,"{fault}");assert!(c.setup.intent.as_ref().unwrap().operation.is_some());
 let calls=worker.join().unwrap();assert!(calls[2]["roomGet"].is_object());clean(c);
 }
});}
#[test]fn flow01_fix01_automatic_handoff_has_no_manual_ready_load_prompt(){
 let(mut c,_,_)=client();for path in [None,Some(c.claims_root.clone())]{c.game_path=path;let handoff=c.view()["handoff"].as_str().unwrap().to_lowercase();assert!(!handoff.contains("load game")&&!handoff.contains("join and ready"));}clean(c);
}

#[test]
fn flow01_fix01_consumption_during_sent_renewal_reconciles_refusal_before_recovery(){tauri::async_runtime::block_on(async{
 let(mut c,room,prep)=client();let mut consumed=prep.clone();consumed.state=Some("consumed".into());
 let(origin,worker)=network(vec![Reply::Data(data(&room,&prep)),Reply::ConsumeError(c.pending.clone(),room.config.as_ref().unwrap().seed.unwrap()),Reply::Data(data(&room,&consumed))]);set_origin(&mut c,&origin);join(&mut c).await.unwrap();
 c.preparation.as_mut().unwrap().expires_at=Some(later(240000));wake(&mut c);c.setup_tick().await.unwrap();wake(&mut c);c.setup_tick().await.unwrap();
 assert!(c.setup.intent.as_ref().unwrap().operation.is_none()&&!c.native_revoked);assert!(c.pending.lock().unwrap().as_ref().unwrap().owner.is_some());
 let calls=worker.join().unwrap();assert!(calls[1]["preparationRenew"].is_object()&&calls[2]["roomGet"].is_object());clean(c);
});}
#[test]
fn flow01_fix01_established_playing_uses_health_route_after_review_deadline(){tauri::async_runtime::block_on(async{
 let(mut c,_,mut release)=release_case();std::fs::create_dir_all(&c.claims_root).unwrap();c.enable_automatic_setup();c.info=Some(wire::Info{connection_flow_revision:Some("g3-flow-1".into()),..Default::default()});
 {let mut pending=c.pending.lock().unwrap();let p=pending.as_mut().unwrap();p.release=true;let report=p.observation.as_mut().unwrap();report.observation.protected=false;report.digest=report.observation.content_digest().unwrap();release.content_digest=Some(hex(&report.digest));}
 c.game.as_mut().unwrap().entered=true;c.game.as_mut().unwrap().permit=Some(release.clone());release.phase=Some("playing".into());release.entered_playing_at=Some("2000-01-01T00:00:02Z".into());
 c.preparation.as_mut().unwrap().review_expires_at=Some(later(-1000));c.room.as_mut().unwrap().review_expires_at=Some(later(-1000));
 let room=c.room.clone().unwrap();let prep=c.preparation.clone().unwrap();
 c.setup.intent=Some(Intent{review_context:c.preview_context(),review_sequence:c.setup.preview_sequence.load(Ordering::Acquire),generation:c.generation.load(Ordering::Acquire),origin:String::new(),service:c.service.clone(),epoch:c.epoch.clone(),desktop:c.desktop.as_ref().unwrap().id.clone(),session:c.desktop.as_ref().unwrap().epoch.clone(),room:room.room_id.clone().unwrap(),config_revision:room.config_revision.unwrap(),config:serde_json::to_vec(room.config.as_ref().unwrap()).unwrap(),run:room.run_id.clone().zip(room.run_epoch.clone()),operation:None,renewing:false,renewal_baseline:None,renewal_sent:false,next:Instant::now(),failures:0,blocked:false,entered:true});
 let mut response=data(&room,&prep);response["release"]=serde_json::to_value(release).unwrap();
 let(origin,worker)=network((0..6).map(|_|Reply::Data(response.clone())).collect());set_origin(&mut c,&origin);c.setup.intent.as_mut().unwrap().origin=origin;
 let mut health=c.health_runner();assert!(c.health_active());assert!(c.native_authority_valid(c.pending.lock().unwrap().as_ref().unwrap()));
 c.next_work=Instant::now();c.tick_inner().await.unwrap();assert!(!c.setup.intent.as_ref().unwrap().blocked);
 health.pump().await.unwrap();tokio::time::sleep(Duration::from_millis(100)).await;c.sync_health();assert!(c.health_active()&&!c.native_revoked&&c.game.as_ref().unwrap().entered);
 let calls=worker.join().unwrap();assert_eq!(calls.iter().filter(|r|r["gamePulse"].is_object()).count(),1);assert!(calls.iter().all(|r|r["preparationRenew"].is_null()&&r["prepare"].is_null()));drop(health);clean(c);
});}

#[test]fn flow01_fix01_definite_stale_renewal_new_unsent_request_stops_on_consumption(){tauri::async_runtime::block_on(async{
 let(mut c,room,prep)=client();let(origin,worker)=network(vec![Reply::Data(data(&room,&prep)),Reply::Error("STALE_REVISION"),Reply::Data(data(&room,&prep))]);set_origin(&mut c,&origin);join(&mut c).await.unwrap();
 c.preparation.as_mut().unwrap().expires_at=Some(later(240000));wake(&mut c);c.setup_tick().await.unwrap();let original=c.setup.intent.as_ref().unwrap().operation.as_ref().unwrap().request.request_id.clone();
 wake(&mut c);c.setup_tick().await.unwrap_err();assert!(!c.setup.intent.as_ref().unwrap().renewal_sent);assert_ne!(original,c.setup.intent.as_ref().unwrap().operation.as_ref().unwrap().request.request_id);
 consume(&mut c,&room);wake(&mut c);c.setup_tick().await.unwrap();assert!(c.setup.intent.as_ref().unwrap().operation.is_none()&&!c.native_revoked);let calls=worker.join().unwrap();assert_eq!(calls.len(),3);clean(c);
});}

#[cfg(feature="g3-rejoin")] #[path="next_continue_cf05_test.rs"] mod continue_cf05;

#[cfg(feature="g3-rejoin")] #[path="next_connection_cr1_test.rs"] mod connection_cr1;

#[cfg(feature="g3-rejoin")]
#[test]fn cr2_unrelated_normal_join_parks_exact_original_and_arms_new_world(){tauri::async_runtime::block_on(async{
 use nt2_desktop_g0::{rejoin_local as local,secrets as protected};let(mut c,room,prep)=client();let(origin,worker)=network(vec![Reply::Data(data(&room,&prep))]);set_origin(&mut c,&origin);let a=continue_cf05::prior(&c);continue_cf05::remember(&c,&a);c.offer_saved_world(&a.encode().unwrap()).unwrap();c.rejoin_previous().unwrap();c.resume_intent.world_loaded(&a).unwrap();
 let desktop=c.desktop.as_ref().unwrap();let id=uuid::Uuid::new_v4().to_string();let body=zeroize::Zeroizing::new(local::resume_request(&a,&protected::SecretBlob::new(vec![b'A';43]).unwrap(),&c.epoch,&desktop.epoch,&id,uuid::Uuid::new_v4()).unwrap());let mut request:nt2_desktop_g0::protocol_rejoin::wire::Request=serde_json::from_slice(&body).unwrap();request.generation=Some(1);let original=serde_json::to_vec(&request).unwrap();c.rejoin_transport.resume_scope=Some(super::super::lifecycle::OriginalScope::capture(&a,&c.epoch,&desktop.id,&desktop.epoch,&id,&original));c.resume_intent.bind_operation(c.resume_intent.generation(),original.clone()).unwrap();
 join(&mut c).await.unwrap();assert!(c.pending.lock().unwrap().as_ref().unwrap().owner.is_none());assert!(c.resume_intent.association().is_none());assert_eq!(c.parked_recovery.len(),1);assert_eq!(c.parked_recovery[0].intent.original_operation(),Some(original.as_slice()));assert!(local::recall(&continue_cf05::StoreBridge(&c.store),&a).unwrap().is_some());let calls=worker.join().unwrap();assert_eq!(calls.len(),1);assert!(calls[0]["roomJoin"].is_object()&&calls[0]["resume"].is_null()&&calls[0]["offline"].is_null());clean(c);
});}

#[path="next_join_revision_test.rs"]mod join_revision;
