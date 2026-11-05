// Root alone runs this ignored real-component test. Only engine observations,
// physical input and a labelled authenticated rate prefill are injected.
mod debug_capture {use super::*;include!("next_debug_composition_test.rs");}
fn a3_view(n:&Native)->Value{fixture_signal::read(n.root.join("a3-view.json")).unwrap().map(|s|serde_json::from_str(&s).expect("Invalid complete A3 fixture snapshot")).unwrap_or(Value::Null)}
fn a3_peer(n:&Native,world:&Value)->Value{a3_view(n)["peers"][world.as_str().unwrap()].clone()}
fn a3_selection_identity(view:&Value,icon:&str,original:&Value,known:&std::collections::BTreeSet<String>,state:&nt2_desktop_g0::g3_a2_local::State)->Result<Option<String>,String>{
 let mut ids=std::collections::BTreeSet::<String>::new();let field=if icon.starts_with("color_"){"lastCosmeticOriginal"}else{"lastOriginal"};let selection=&view["copyEvidence"]["lastSelection"];
 for value in [&view[field],&selection["copiedIdentity"]]{if value!=original{if let Some(id)=value.as_str(){if !id.is_empty()&&!known.contains(id){if id.len()!=32||uuid::Uuid::parse_str(id).is_err(){return Err("Invalid copied A3 identity".into())}ids.insert(id.into());}}}}
 for(id,op)in &state.social{if known.contains(id){continue}let matching=if let Some(robe)=icon.strip_prefix("color_"){op.publish.cosmetic.as_ref().is_some_and(|p|p.robe_id.as_deref()==Some(robe))}else{op.publish.emote.as_ref().is_some_and(|p|p.emote_id.as_deref()==Some(icon)&&p.active==Some(true))};if matching{ids.insert(id.clone());}}
 if ids.len()>1{return Err("Multiple A3 originals for one physical selection".into())}Ok(ids.into_iter().next())
}
fn a3_selection_no_copy(view:&Value,icon:&str,armed_frame:u64)->bool{let s=&view["copyEvidence"]["lastSelection"];s["token"]==icon&&s["frame"].as_u64().is_some_and(|v|v>armed_frame)&&s["completed"]==true&&s["copied"]==false}
#[test]fn a3_menu_selection_waits_for_complete_no_copy_and_stops_on_either_original(){
 let id="a".repeat(32);let known=std::collections::BTreeSet::new();let mut state=nt2_desktop_g0::g3_a2_local::State::default();let mut v=json!({"lastOriginal":"old","copyEvidence":{"lastSelection":{"token":"heart","frame":12,"completed":false,"copied":false}}});
 assert!(!a3_selection_no_copy(&v,"heart",10),"Consumed file without completed callback cannot authorize another click");v["copyEvidence"]["lastSelection"]["completed"]=json!(true);assert!(a3_selection_no_copy(&v,"heart",10));assert!(!a3_selection_no_copy(&v,"heart",12),"Stale selection is not the current attempt");
 v["lastOriginal"]=json!(id);assert_eq!(a3_selection_identity(&v,"heart",&json!("old"),&known,&state).unwrap(),Some(id.clone()),"Native copy ends clicking before journal arrival");v["lastOriginal"]=json!("old");
 let mut publish=nt2_desktop_g0::protocol_g3_social::wire::Publish::default();publish.emote=Some(nt2_desktop_g0::protocol_g3_social::wire::Emote{emote_id:Some("heart".into()),active:Some(true)});state.social.insert(id.clone(),nt2_desktop_g0::g3_a2_local::SocialOperation{publish,status:2,reason:"real refusal".into(),uncertain:false});
 assert_eq!(a3_selection_identity(&v,"heart",&json!("old"),&known,&state).unwrap(),Some(id.clone()),"A definitive refusal is still an original and cannot be reclicked");v["copyEvidence"]["lastSelection"]["copiedIdentity"]=json!("b".repeat(32));assert!(a3_selection_identity(&v,"heart",&json!("old"),&known,&state).is_err());
}
async fn a3_click_copy(service:&mut Service,c:&Client,n:&Native,icon:&str)->String{
 // One5s budget includes opening, physical selection and observed release.
 // A blocked Q press has no queued click; always release it before repressing.
 enum Phase{Release(u64),Open,Select(u64),Copied(String,u64)}
 let first=a3_view(n);let field=if icon.starts_with("color_"){"lastCosmeticOriginal"}else{"lastOriginal"};let original=first[field].clone();let store=a3_store(c).await;let known=store.read().unwrap().social.keys().cloned().collect();let deadline=Instant::now()+Duration::from_secs(5);
 fixture_signal::remove(n.root.join("a3-menu")).unwrap();fixture_signal::remove(n.root.join("a3-select")).unwrap();let mut phase=Phase::Release(first["frame"].as_u64().unwrap_or(0));let(mut attempts,mut represses)=(0,0);
 let failure=loop{
  service.wall_clock();let view=a3_view(n);let frame=view["frame"].as_u64().unwrap_or(0);let state=store.read().unwrap();let selected=match a3_selection_identity(&view,icon,&original,&known,&state){Ok(id)=>id,Err(reason)=>break reason};
  if !matches!(&phase,Phase::Copied(..)){if let Some(id)=selected{fixture_signal::remove(n.root.join("a3-menu")).unwrap();fixture_signal::remove(n.root.join("a3-select")).unwrap();phase=Phase::Copied(id,frame);}}
  match &phase{
   Phase::Release(at)if frame>at+1&&view["menu"]==false&&view["menuLocked"]==false=>{fixture_signal::write(n.root.join("a3-menu"),b"injected Q pressed").unwrap();phase=Phase::Open;represses+=1;},
   Phase::Open if view["menuLocked"]==true||view["pendingStop"]==true=>{fixture_signal::remove(n.root.join("a3-menu")).unwrap();phase=Phase::Release(frame);},
   Phase::Open if view["menu"]==true=>{fixture_signal::write(n.root.join("a3-select"),icon).unwrap();phase=Phase::Select(frame);attempts+=1;},
   Phase::Select(_)if (view["menuLocked"]==true||view["pendingStop"]==true)&&fixture_signal::present(n.root.join("a3-select")).unwrap()=>{fixture_signal::remove(n.root.join("a3-menu")).unwrap();fixture_signal::remove(n.root.join("a3-select")).unwrap();phase=Phase::Release(frame);},
   Phase::Select(at)if a3_selection_no_copy(&view,icon,*at)=>{fixture_signal::remove(n.root.join("a3-menu")).unwrap();phase=Phase::Release(frame);},
   Phase::Copied(id,at)if frame>at+1&&view["menu"]==false&&view["menuLocked"]==false=>return id.clone(),
   _=>{}
  }
  if Instant::now()>=deadline{break "A3 physical menu/copy/release unchanged5s deadline".into()}
  tokio::time::sleep(Duration::from_millis(25)).await;
 };
 let runtime=a3_runtime_evidence(c).await;fixture_signal::write(n.root.join("a3-copy-failure.json"),serde_json::to_vec_pretty(&json!({"boundary":"before copy assertion/failure unwind","reason":failure,"icon":icon,"originalBefore":original,"physicalAttempts":attempts,"bindingPresses":represses,"view":a3_view(n),"activity":native_activity(n),"runtime":runtime,"stream":c.handle.view()["g3"]["fixture_social_stream"],"worker":c.handle.view()["g3"]["fixture_social"],"engineFactsInjected":true,"serviceMocked":false})).unwrap()).unwrap();fixture_signal::remove(n.root.join("a3-menu")).unwrap();fixture_signal::remove(n.root.join("a3-select")).unwrap();panic!("{failure}; inspect a3-copy-failure.json");
}
async fn a3_select_outcome(service:&mut Service,c:&Client,n:&Native,icon:&str,expected:u8)->String{
 let id=a3_click_copy(service,c,n,icon).await;let store=a3_store(c).await;
 until(service,"exact A3 native UI original has a definitive durable outcome",||store.read().unwrap().social.get(&id).is_some_and(|op|op.status!=0)).await;
 let op=store.read().unwrap().social[&id].clone();assert_eq!(op.status,expected,"A3 {icon} original {id}: {}; optimistic presentation is not an acknowledgement",op.reason);id
}
async fn a3_select(service:&mut Service,c:&Client,n:&Native,icon:&str)->String{a3_select_outcome(service,c,n,icon,1).await}
async fn a3_store(c:&Client)->Arc<nt2_desktop_g0::g3_a2_local::Store>{let slot=Arc::new(Mutex::new(None));let out=slot.clone();c.handle.command(move|c|Box::pin(async move{*out.lock().unwrap()=c.social_work.fixture_store();Ok(json!({"captured":true}))})).await.unwrap();let store=slot.lock().unwrap().take().expect("ordinary social store");store}
async fn a3_runtime_evidence(c:&Client)->Value{c.handle.command(|c|Box::pin(async move{Ok(c.g3.as_ref().map(|g|g.view()).unwrap_or(Value::Null))})).await.unwrap()}
fn a3_new_poi_identity(view:&Value,known:&std::collections::BTreeSet<String>,state:&nt2_desktop_g0::g3_a2_local::State)->Result<Option<String>,String>{
 let mut ids:std::collections::BTreeSet<String>=state.operations.keys().filter(|id|!known.contains(*id)).cloned().collect();
 for value in [&view["copyEvidence"]["lastPoiCopied"]["id"],&view["copyEvidence"]["lastPoiSubmit"]["pending"]]{if let Some(id)=value.as_str(){if id.len()!=32||uuid::Uuid::parse_str(id).is_err(){return Err("Invalid copied POI identity".into())}if !known.contains(id){ids.insert(id.into());}}}
 if ids.len()>1{return Err("Multiple POI originals for one logical placement".into())}Ok(ids.into_iter().next())
}
fn a3_poi_busy_before_copy(view:&Value,before:u64)->bool{
 let submit=&view["copyEvidence"]["lastPoiSubmit"];submit["count"].as_u64().is_some_and(|n|n>before)&&submit["copied"]==false&&submit["pending"].is_null()&&submit["feedback"]=="Optional message lane is busy; keep the draft and try again"
}
#[test]fn a3_poi_copy_boundary_stops_on_native_or_durable_original_before_retry(){
 let known=std::collections::BTreeSet::new();let mut state=nt2_desktop_g0::g3_a2_local::State::default();let mut native=json!({"copyEvidence":{"lastPoiSubmit":{"count":1,"copied":false,"pending":null,"feedback":"Optional message lane is busy; keep the draft and try again"}}});
 assert!(a3_new_poi_identity(&native,&known,&state).unwrap().is_none()&&a3_poi_busy_before_copy(&native,0));assert!(!a3_poi_busy_before_copy(&native,1),"A stale busy result cannot authorize a new click");
 let id="a".repeat(32);native["copyEvidence"]["lastPoiCopied"]=json!({"id":id});assert_eq!(a3_new_poi_identity(&native,&known,&state).unwrap(),Some(id.clone()),"Copied identity stops clicking even before durable acknowledgement");
 native["copyEvidence"]["lastPoiCopied"]=Value::Null;native["copyEvidence"]["lastPoiSubmit"]["pending"]=json!(id);assert_eq!(a3_new_poi_identity(&native,&known,&state).unwrap(),Some(id.clone()));native["copyEvidence"]["lastPoiSubmit"]["pending"]=Value::Null;
 state.operations.insert(id.clone(),nt2_desktop_g0::g3_a2_local::Operation{publish:Default::default(),text:"fixture".into(),status:0,reason:String::new(),uncertain:true});assert_eq!(a3_new_poi_identity(&native,&known,&state).unwrap(),Some(id.clone()),"Durable original stops clicking even if native observations lag");
 for reason in ["Message refused","Enter a message","Random operation identity unavailable"]{native["copyEvidence"]["lastPoiSubmit"]["feedback"]=json!(reason);assert!(!a3_poi_busy_before_copy(&native,0));}
 native["copyEvidence"]["lastPoiCopied"]=json!({"id":"b".repeat(32)});assert!(a3_new_poi_identity(&native,&known,&state).is_err(),"Conflicting originals must surface, not select a new retry target");
}
async fn a3_place_poi(service:&mut Service,c:&Client,n:&Native,peer:&Native,draft:&str){
 // One unchanged25s budget covers physical attempts, durable outcome and both
 // real renderers. Explicit retries are permitted only before any copy/original.
 let deadline=Instant::now()+Duration::from_secs(25);let store=a3_store(c).await;let initial=store.read().unwrap();assert!(initial.pending.is_none(),"A3 POI smoke already has an unresolved original");
 let mut known:std::collections::BTreeSet<String>=initial.operations.keys().cloned().collect();let first=a3_view(n);
 for value in [&first["copyEvidence"]["lastPoiCopied"]["id"],&first["copyEvidence"]["lastPoiSubmit"]["pending"]]{if let Some(id)=value.as_str(){known.insert(id.into());}}
 let(mut copied,mut armed,mut before_submit,mut attempts)=(None::<String>,false,0u64,0u64);
 let failure=loop{
  service.wall_clock();let native=a3_view(n);let state=store.read().unwrap();
  if copied.is_none(){match a3_new_poi_identity(&native,&known,&state){Ok(id)=>copied=id,Err(reason)=>break reason}}
  if let Some(id)=&copied{
   if armed{fixture_signal::remove(n.root.join("a2-click")).unwrap();armed=false;}
   if let Some(op)=state.operations.get(id){
    if op.text!=draft{break "Copied POI original substituted the draft".into()}
    if op.status==2{break format!("Definitive POI refusal for copied original {id}: {}",op.reason)}
    if op.status==1&&view(n)["markers"]==1&&view(peer)["markers"]==1{
     assert_eq!(state.operations.keys().filter(|key|!known.contains(*key)).count(),1,"Repeated UI placement created another original");
     fixture_signal::write(n.root.join("a3-poi-smoke.json"),serde_json::to_vec_pretty(&json!({"copiedIdentity":id,"acceptedOriginal":op.publish,"explicitAttempts":attempts,"preCopyBusyOnly":true,"noClickAfterCopy":true,"bothRenderers":true,"engineFactsInjected":true,"serviceMocked":false})).unwrap()).unwrap();return
    }
   }
  }else{
   if armed&&!fixture_signal::present(n.root.join("a2-click")).unwrap(){
    let submit=&native["copyEvidence"]["lastPoiSubmit"];
    if submit["count"].as_u64().is_some_and(|n|n>before_submit){if a3_poi_busy_before_copy(&native,before_submit){armed=false;}else{break format!("POI click produced a non-busy/no-copy outcome: {submit}")}}
   }
   if !armed&&Instant::now()<deadline{before_submit=native["copyEvidence"]["poiSubmits"].as_u64().unwrap_or(0);fixture_signal::write(n.root.join("a2-click"),"Place message").unwrap();armed=true;attempts+=1;}
  }
  if Instant::now()>=deadline{break "A3 POI copy/outcome/both-renderers unchanged25s deadline".into()}
  tokio::time::sleep(Duration::from_millis(50)).await;
 };
 let runtime=a3_runtime_evidence(c).await;fixture_signal::write(n.root.join("a3-poi-failure.json"),serde_json::to_vec_pretty(&json!({"boundary":"before POI assertion/failure unwind","reason":failure,"copiedIdentity":copied,"explicitAttempts":attempts,"native":a3_view(n),"peer":a3_view(peer),"activity":native_activity(n),"runtime":runtime,"operations":store.read().unwrap().operations,"stream":c.handle.view()["g3"]["fixture_social_stream"],"worker":c.handle.view()["g3"]["fixture_social"],"engineFactsInjected":true,"serviceMocked":false})).unwrap()).unwrap();panic!("{failure}; inspect a3-poi-failure.json");
}
fn a3_finite_observation(view:&Value,actor:&Value,start:&str,stops:&std::collections::BTreeSet<String>)->Option<Value>{
 for row in view["emoteHistory"].as_object()?.values(){
  if row["actor"]!=*actor||row["id"]!="heart"||row["duration"]!=101||row["status"]!=1||row["entity"].as_u64().unwrap_or(0)==0{continue}
  let phase=if row["event"]==start&&row["rendered"]=="heart"&&row["active"]==true{"renderer-start-observed"}
   else if row["event"]==start&&row["rendered"]==""&&row["active"]==false&&row["elapsed"].as_u64().is_some_and(|v|v>=103){"delivered-complete-no-replay"}
   else if row["rendered"]==""&&row["active"]==false&&row["event"].as_str().is_some_and(|event|stops.contains(event)){"accepted-start-followed-by-accepted-finite-stop"}else{continue};
  return Some(json!({"phase":phase,"observation":row}));
 }None
}
#[test]fn a3_finite_delivery_evidence_requires_exact_original_or_accepted_stop(){
 let actor=json!("actor");let stop="accepted-stop".to_string();let stops=std::collections::BTreeSet::from([stop.clone()]);let base=json!({"actor":"actor","id":"heart","duration":101,"status":1,"entity":1001,"event":"start","rendered":"heart","active":true,"elapsed":60});
 let view=|row:Value|json!({"emoteHistory":{"1":row}});assert_eq!(a3_finite_observation(&view(base.clone()),&actor,"start",&stops).unwrap()["phase"],"renderer-start-observed");
 let mut done=base.clone();done["rendered"]=json!("");done["active"]=json!(false);done["elapsed"]=json!(103);assert_eq!(a3_finite_observation(&view(done.clone()),&actor,"start",&stops).unwrap()["phase"],"delivered-complete-no-replay");done["elapsed"]=json!(0);done["event"]=json!(stop);assert_eq!(a3_finite_observation(&view(done),&actor,"start",&stops).unwrap()["phase"],"accepted-start-followed-by-accepted-finite-stop");
 for(field,value)in[("actor",json!("other")),("event",json!("foreign")),("duration",json!(240)),("entity",json!(0)),("status",json!(2))]{let mut bad=base.clone();bad[field]=value;assert!(a3_finite_observation(&view(bad),&actor,"start",&stops).is_none(),"Unrelated or ineligible {field} cannot count as finite delivery");}
}
async fn a3_input_until(service:&mut Service,own:&std::path::Path,phase:&str,a:&Client,na:&Native,b:&Client,nb:&Native,predicate:impl Fn()->bool){
 let deadline=Instant::now()+Duration::from_secs(25);let mut sample=Instant::now();
 loop{service.wall_clock();if predicate(){return}if Instant::now()>=sample{sample=Instant::now()+Duration::from_secs(2);let rows=[(a,na),(b,nb)].into_iter().map(|(c,n)|{let view=c.handle.view();json!({"native":a3_view(n),"activity":native_activity(n),"worker":view["g3"]["fixture_social"],"stream":view["g3"]["fixture_social_stream"],"optional":view["g3"]["optional_failures"],"roster":view["g3"]["movement_roster"],"movement":view["g3"]["movement"],"connection":view["connection"]["state"]})}).collect::<Vec<_>>();let mut out=std::fs::OpenOptions::new().create(true).append(true).open(own.join("a3-input-observations.jsonl")).unwrap();writeln!(out,"{}",json!({"phase":phase,"beforeFailureUnwind":true,"sides":rows})).unwrap();}
 assert!(Instant::now()<deadline,"A3 {phase} deadline; a3-input-observations.jsonl");tokio::time::sleep(Duration::from_millis(50)).await;}
}
#[test]#[ignore="Root serialized A3 TLS/SQL/ordinary scheduler/native/Lua; Noita APIs/input injected"]
fn a3_social_controls_ordinary_scheduler_composition(){
 assert_eq!(std::env::var("NT2_C3_COMPOSITION").as_deref(),Ok("1"));assert_eq!(std::env::var("NT2_A3_FIXTURE").as_deref(),Ok("1"));assert_eq!(std::env::var("NT2_POLYMORPH_FIXTURE").as_deref(),Ok("1"));
 std::thread::Builder::new().name("a3-social".into()).stack_size(16*1024*1024).spawn(||tauri::async_runtime::block_on(async{
 verify_current_fixture_inputs();let own=repo().join("spikes/noita-entry/.build/a3-client-20261022-01").join(format!("composition-{}",uuid::Uuid::new_v4()));std::fs::create_dir_all(&own).unwrap();eprintln!("A3_COMPOSITION {}",own.display());
 let mut service=Service::start(&own);network_profile::enable_elapsed_clock(&mut service,&own);let router=PipeHarness::new();let mut a=Client::start(&service.private(),&own.join("host"),None,false,true,router.routes.clone(),service.watches.clone(),service.stores.clone()).await;bundle(&mut service,&a).await;let mut na=Native::start(&a,None,false);wait(&mut service,&a,"A3 host Ready",30,|v|current_state_is(v,"Ready")&&na.root.join("associated").is_file()).await;let world=a.handle.view()["rejoin"]["current_world"].clone();
 service.command(json!({"op":"peer"}));let mut b=Client::start(&service.private(),&own.join("peer"),None,true,true,router.routes.clone(),service.watches.clone(),service.stores.clone()).await;bundle(&mut service,&b).await;let mut nb=Native::start(&b,None,false);wait(&mut service,&b,"A3 peer Ready",30,|v|current_state_is(v,"Ready")&&nb.root.join("associated").is_file()).await;let peer_world=b.handle.view()["rejoin"]["current_world"].clone();
 // Persistent unlock observations exist before the first eligible cosmetic sample.
 fixture_signal::write(na.root.join("a3-flags"),"secret_hat,secret_amulet_gem").unwrap();fixture_signal::write(nb.root.join("a3-flags"),"secret_amulet").unwrap();
 two_peer_ghost_with_start(&mut service,&a,&na,&world,&b,&nb,&peer_world,"startReviewed").await;
 until(&mut service,"A3 current production roster",||a3_view(&na)["peers"][peer_world.as_str().unwrap()].is_object()&&a3_view(&nb)["peers"][world.as_str().unwrap()].is_object()).await;
 if std::env::var("NT2_A3_INPUT_ONLY").as_deref()==Ok("1"){
  for n in [&na,&nb]{fixture_signal::write(n.root.join("a3-watch-retained"),"capture first ordinary ghost removal before cosmetic wait").unwrap();}
  let cosmetic_deadline=Instant::now()+Duration::from_secs(25);let mut cosmetic_sample=Instant::now();
  loop{service.wall_clock();if a3_peer(&nb,&world)["robe"]=="purple"&&a3_peer(&na,&peer_world)["robe"]=="purple"{break}
   if Instant::now()>=cosmetic_sample{cosmetic_sample=Instant::now()+Duration::from_secs(2);let rows=[(&a,&na),(&b,&nb)].into_iter().map(|(c,n)|json!({"native":a3_view(n),"activity":native_activity(n),"worker":c.handle.view()["g3"]["fixture_social"],"stream":c.handle.view()["g3"]["fixture_social_stream"],"optional":c.handle.view()["g3"]["optional_failures"]})).collect::<Vec<_>>();fixture_signal::write(own.join("a3-initial-cosmetics.json"),serde_json::to_vec_pretty(&json!({"phase":"before failure unwind","sides":rows})).unwrap()).unwrap();}
   assert!(Instant::now()<cosmetic_deadline,"initial automatic cosmetics did not reach renderer; a3-initial-cosmetics.json");tokio::time::sleep(Duration::from_millis(50)).await;
  }
  let store=a3_store(&a).await;let baseline=store.read().unwrap();
  fixture_signal::write(na.root.join("a3-copy-block"),"bounded explicit copy refusal; HUD/movement/service unchanged").unwrap();
  fixture_signal::write(na.root.join("a3-menu"),"injected Q press").unwrap();until(&mut service,"held menu before single emote choice",||a3_view(&na)["menu"]==true).await;
  fixture_signal::write(na.root.join("a3-select"),"wave").unwrap();
  until(&mut service,"single emote choice waits before actual native copy",||fixture_signal::present(na.root.join("a3-copy-blocked")).unwrap()&&a3_view(&na)["localOriginal"].as_str().is_some_and(|id|id.len()==32)).await;
  let intent=a3_view(&na)["localOriginal"].as_str().unwrap().to_owned();assert!(!store.read().unwrap().social.contains_key(&intent));
  let delay=Instant::now()+Duration::from_secs(4);while Instant::now()<delay{service.wall_clock();assert_eq!(a3_view(&na)["localOriginal"],intent);tokio::time::sleep(Duration::from_millis(50)).await;}
  fixture_signal::remove(na.root.join("a3-copy-block")).unwrap();fixture_signal::remove(na.root.join("a3-menu")).unwrap();
  a3_input_until(&mut service,&own,"same one-click emote original accepted and rendered remotely",&a,&na,&b,&nb,||store.read().unwrap().social.get(&intent).is_some_and(|op|op.status==1)&&a3_peer(&nb,&world)["emote"]=="wave").await;
  assert_eq!(store.read().unwrap().social.iter().filter(|(id,op)|!baseline.social.contains_key(*id)&&op.publish.emote.as_ref().is_some_and(|e|e.active==Some(true)&&e.emote_id.as_deref()==Some("wave"))).count(),1);
  click(&mut service,&na,"World message").await;fixture_signal::write(na.root.join("a2-text"),"Single POI").unwrap();until(&mut service,"one-click POI draft",||text(&na,"a2-ui.txt").contains("10/99")).await;
  until(&mut service,"composer cleanup Stop copied before POI contention",||a3_view(&na)["pendingStop"]==false).await;
  fixture_signal::remove(na.root.join("a3-copy-blocked")).unwrap();fixture_signal::write(na.root.join("a3-copy-block"),"bounded POI copy refusal").unwrap();
  fixture_signal::write(na.root.join("a2-click"),"Place message").unwrap();
  until(&mut service,"one POI click retained before copy",||fixture_signal::present(na.root.join("a3-copy-blocked")).unwrap()&&a3_view(&na)["copyEvidence"]["lastPoiSubmit"]["pending"].as_str().is_some_and(|id|id.len()==32)).await;
  let poi=a3_view(&na)["copyEvidence"]["lastPoiSubmit"]["pending"].as_str().unwrap().to_owned();assert!(!store.read().unwrap().operations.contains_key(&poi));
  let delay=Instant::now()+Duration::from_secs(4);while Instant::now()<delay{service.wall_clock();tokio::time::sleep(Duration::from_millis(50)).await;}
  fixture_signal::remove(na.root.join("a3-copy-block")).unwrap();
  a3_input_until(&mut service,&own,"same POI original accepted and visible on both PCs",&a,&na,&b,&nb,||store.read().unwrap().operations.get(&poi).is_some_and(|op|op.status==1)&&view(&na)["markers"].as_u64().unwrap_or(0)>0&&view(&nb)["markers"].as_u64().unwrap_or(0)>0).await;
  assert_eq!(store.read().unwrap().operations.len(),baseline.operations.len()+1);assert_eq!(a3_view(&na)["copyEvidence"]["poiSubmits"],1);
  fresh_bidirectional_ghosts(&mut service,&a,&na,&b,&nb,"input contention preserves fresh two-way movement").await;
  fixture_signal::write(own.join("a3-input-progress.json"),serde_json::to_vec_pretty(&json!({"passed":true,"emoteLocalId":intent,"poiLocalId":poi,"emotePhysicalSelections":1,"poiPhysicalSubmits":1,"injectedExplicitCopyRefusalMs":4000,"duplicateOriginals":0,"serviceMocked":false,"engineAndInputInjected":true,"hostWorker":a.handle.view()["g3"]["fixture_social"],"peerWorker":b.handle.view()["g3"]["fixture_social"]})).unwrap()).unwrap();
  na.close();nb.close();a.close().await;b.close().await;service.close();checkpoint(&own,"bounded-one-click-input-progress-and-owned-schema-cleanup");return;
 }
 a3_select(&mut service,&a,&na,"color_red").await;a3_select(&mut service,&b,&nb,"color_blue").await;
 until(&mut service,"per-actor robe/actual flags through service and native",||a3_peer(&nb,&world)["robe"]=="red"&&a3_peer(&nb,&world)["hat"]==true&&a3_peer(&nb,&world)["gem"]==true&&a3_peer(&na,&peer_world)["robe"]=="blue"&&a3_peer(&na,&peer_world)["amulet"]==true).await;
 for n in [&na,&nb]{fixture_signal::write(n.root.join("a3-watch-retained"),b"capture first actual ghost removal before failure unwind").unwrap();}
 a3_select(&mut service,&a,&na,"wave").await;until(&mut service,"loop through actual native mailbox",||a3_peer(&nb,&world)["emote"]=="wave"&&a3_view(&na)["motion"]==0).await;let holder=a3_peer(&nb,&world)["holder"].clone();let original=a3_peer(&nb,&world)["event"].clone();
 let retained=Instant::now();while retained.elapsed()<Duration::from_secs(32){service.wall_clock();let host=a3_view(&na);let peer=a3_view(&nb);let host_remote=&host["peers"][peer_world.as_str().unwrap()];let peer_remote=&peer["peers"][world.as_str().unwrap()];
  if peer_remote["emote"]!="wave"||peer_remote["holder"]!=holder||peer_remote["robe"]!="red"||host_remote["robe"]!="blue"{
   let first_loss=|n:&Native|fixture_signal::read(n.root.join("a3-first-ghost-loss.json")).unwrap().map(|s|serde_json::from_str::<Value>(&s).unwrap());
   let host_runtime=a3_runtime_evidence(&a).await;let peer_runtime=a3_runtime_evidence(&b).await;
   fixture_signal::write(own.join("a3-retained-failure.json"),serde_json::to_vec_pretty(&json!({"boundary":"before assertion and failure unwind","elapsedMs":retained.elapsed().as_millis(),"host":host,"peer":peer,"hostActivity":native_activity(&na),"peerActivity":native_activity(&nb),"hostFirstGhostLoss":first_loss(&na),"peerFirstGhostLoss":first_loss(&nb),"hostRuntime":host_runtime,"peerRuntime":peer_runtime})).unwrap()).unwrap();
  }
  assert_eq!(peer_remote["emote"],"wave");assert_eq!(peer_remote["holder"],holder,"Duplicate current snapshots restarted the same original");assert_eq!(peer_remote["robe"],"red");assert_eq!(host_remote["robe"],"blue");tokio::time::sleep(Duration::from_millis(200)).await;
 }
 for n in [&na,&nb]{fixture_signal::remove(n.root.join("a3-watch-retained")).unwrap();}
 checkpoint(&own,"ordinary-loop-over10s-cosmetics-over30s-distinct-colours-flags-duplicate-holder-stable");
 let mut reader=authenticated_owner(&b).await;let response=reader.timed_snapshot("").await.unwrap().response;let delayed=response.projections.unwrap().into_iter().find(|p|p.emote.is_some()&&p.scope.as_ref().unwrap().world_token.as_deref()==world.as_str()).unwrap();
 a3_select(&mut service,&a,&na,"wave").await;until(&mut service,"intentional same emote has new original and holder",||a3_peer(&nb,&world)["emote"]=="wave"&&a3_peer(&nb,&world)["event"]!=original&&a3_peer(&nb,&world)["holder"]!=holder).await;
 // Pause only the injected game callbacks. Native workers and app scheduler
 // remain alive; held local motion is released safely on resume.
 let probe=na.root.join("capture-first-loss");if probe.is_file(){fixture_signal::remove(probe).unwrap();}paused_wait_phase(&mut service,&a,&na,&world,2,false).await;fixture_signal::remove(na.root.join("pause-game-thread")).unwrap();ready_owner(&mut service,&a,&na,&world,true).await;until(&mut service,"resumed callback cancels transient emote and restores motion",||a3_view(&na)["controls"]==true&&a3_view(&na)["localEmote"]==""&&a3_view(&na)["motion"]==287).await;
 // A finite heart must complete in the producer/remote adapters and must not
 // restart from subsequent retained snapshots.
 let heart_ui=a3_select(&mut service,&a,&na,"heart").await;let heart_store=a3_store(&a).await;let heart=heart_store.read().unwrap().social[&heart_ui].clone();let heart_event=heart.publish.event_id.clone().unwrap();let heart_sequence=heart.publish.sequence.unwrap();let finite_evidence=Arc::new(Mutex::new(Value::Null));
 until(&mut service,"real remote finite start/completed delivery or correlated accepted stop",||{
  let state=heart_store.read().unwrap();let stops=state.social.values().filter(|op|op.status==1&&op.publish.sequence.is_some_and(|s|s>heart_sequence)&&op.publish.emote.as_ref().is_some_and(|e|e.emote_id.as_deref()==Some("heart")&&e.active==Some(false))).filter_map(|op|op.publish.event_id.clone()).collect();
  if let Some(evidence)=a3_finite_observation(&a3_view(&nb),&world,&heart_event,&stops){*finite_evidence.lock().unwrap()=evidence;true}else{false}
 }).await;
 until(&mut service,"finite effect ends and actual motion restored",||a3_peer(&nb,&world)["emote"]==""&&a3_view(&na)["localEmote"]==""&&a3_view(&na)["motion"]==287).await;elapsed(&mut service,2).await;assert_eq!(a3_peer(&nb,&world)["emote"],"");
 fixture_signal::write(own.join("a3-finite-heart.json"),serde_json::to_vec_pretty(&json!({"acceptedStart":heart.publish,"remote":*finite_evidence.lock().unwrap(),"localAndRemoteRestored":true,"noReplay":true,"engineFactsInjected":true,"serviceMocked":false})).unwrap()).unwrap();
 let players=reader.roster().await.unwrap().players.unwrap();let recipient=response.scope.unwrap();let deadline=Instant::now()+Duration::from_secs(5);loop{let p=delayed.clone();let players=players.clone();let recipient=recipient.clone();let replay=b.handle.command(move|c|Box::pin(async move{let binding=c.g3_binding()?.ok_or("A3 current binding absent")?;let scope=serde_json::from_value(serde_json::to_value(&binding.scope).unwrap()).unwrap();if serde_json::to_vec(&scope).unwrap()!=serde_json::to_vec(&recipient).unwrap(){return Err("A3 recipient changed".into())}Ok(json!({"accepted":c.social_work.fixture_delayed_presentation(&scope,&players,&p)?}))})).await.unwrap();if replay["accepted"]==false{break}assert!(replay["accepted"].is_null()&&Instant::now()<deadline,"Older authenticated start replaced newer stop");tokio::time::sleep(Duration::from_millis(20)).await;}
 checkpoint(&own,"deliberate-same-id-restart-finite-completion-cold-repeat-and-authenticated-reordered-start-rejected");
 // Actual normal GUI wave establishes the presentation to clean up. Only
 // the private per-request budget setup is synthetic; policy/refusal/Stop and
 // dropped accepted reply all remain real, with no HTTP timing race.
 let rate_store=a3_store(&a).await;let prior_ui=a3_select(&mut service,&a,&na,"wave").await;
 let prior_event=rate_store.read().unwrap().social[&prior_ui].publish.event_id.clone().unwrap();
 until(&mut service,"accepted real GUI wave before synthetic budget precondition",||a3_peer(&nb,&world)["emote"]=="wave"&&a3_peer(&nb,&world)["event"]==prior_event).await;
 service.command(json!({"op":"socialRateArm","plan":{"target":"consumer","worldId":world,"activeEventId":prior_event}}));
 service.command(json!({"op":"socialArm","plan":{"target":"consumer","worldId":world,"operation":"publishEmote","stopOnly":true,"phase":"reply","effect":"drop"}}));
 let refused_ui=a3_select_outcome(&mut service,&a,&na,"heart",2).await;until(&mut service,"rate-limited selection returns visible native outcome",||text(&na,"a2-notices.txt").contains("Social selection capacity reached")).await;
 until(&mut service,"cleanup survives exhausted start budget and dropped reply",||{let peer=a3_peer(&nb,&world);peer["entity"].as_u64().unwrap_or(0)>0&&peer["canonicalEmote"]=="heart"&&peer["canonicalActive"]==false&&peer["emote"]==""&&a3_view(&na)["motion"]==287&&!a3_view(&na)["pendingStop"].as_bool().unwrap_or(true)}).await;
 let faults=service.command(json!({"op":"socialStatus"}));let dropped=faults["social"]["events"].as_array().unwrap().iter().find(|v|v["operation"]=="publishEmote"&&v["kind"]=="committed-reply-drop").cloned().expect("Actual accepted stop reply must be destroyed");let event=dropped["eventId"].as_str().unwrap();let store=a3_store(&a).await;until(&mut service,"same durable stop original accepted after actual reply loss",||store.read().unwrap().social.values().any(|op|op.publish.event_id.as_deref()==Some(event)&&op.status==1&&op.uncertain&&op.publish.emote.as_ref().is_some_and(|e|e.active==Some(false)))).await;
 let refused_event=rate_store.read().unwrap().social[&refused_ui].publish.event_id.clone().unwrap();let rate=&faults["social"]["ratePrecondition"];
 assert!(rate["active"].is_null());let rate_events=rate["events"].as_array().unwrap();
 assert!(rate_events.iter().any(|v|v["kind"]=="start-refused-real-policy"&&v["eventId"]==refused_event&&v["synthetic"]==true),"Actual targeted native Start must hit real CAPACITY before Stop setup");
 assert!(rate_events.iter().any(|v|v["kind"]=="stop-committed"&&v["eventId"]==event&&v["synthetic"]==true),"Actual same Stop original must commit at the injected exhausted budget");
 fixture_signal::write(own.join("a3-lost-stop.json"),serde_json::to_vec_pretty(&json!({"fault":dropped,"ratePrecondition":rate,"priorAcceptedWave":prior_event,"refusedNativeStart":refused_event,"prefill":"synthetic counter4/current window at actual Start transaction and actual Stop transaction; normal policy/refusal/cleanup/reply-drop/original reconciliation","sameOriginalReconciled":true,"serviceMocked":false})).unwrap()).unwrap();checkpoint(&own,"native-rate-refusal-and-cleanup-at-exhausted-budget-real-stop-commit-reply-drop-original-reconciliation");
 a3_select(&mut service,&a,&na,"wave").await;until(&mut service,"pre-form loop",||a3_peer(&nb,&world)["emote"]=="wave").await;change_form(&mut service,&na,199,"poly",210).await;until(&mut service,"form replacement cancels animation and restores new actual motion",||a3_view(&na)["subject"]==199&&a3_view(&na)["motion"]==287&&a3_peer(&nb,&world)["emote"]=="").await;change_form(&mut service,&na,299,"normal",220).await;fresh_bidirectional_ghosts(&mut service,&a,&na,&b,&nb,"A3 preserves movement after forms").await;
 a3_select(&mut service,&a,&na,"wave").await;until(&mut service,"pre-disable loop",||a3_peer(&nb,&world)["emote"]=="wave").await;fixture_signal::write(nb.root.join("a3-setting-NT_SHOW_EMOTES"),"false").unwrap();until(&mut service,"display off restores ordinary remote presentation",||a3_peer(&nb,&world)["emote"]==""&&a3_peer(&nb,&world)["robe"]=="").await;fixture_signal::write(nb.root.join("a3-setting-NT_SHOW_EMOTES"),"true").unwrap();until(&mut service,"display on reapplies allowed current loop/robe",||a3_peer(&nb,&world)["emote"]=="wave"&&a3_peer(&nb,&world)["robe"]=="red").await;
 a3_select(&mut service,&b,&nb,"t_pose").await;until(&mut service,"independent peer loop before other actor outage",||a3_peer(&na,&peer_world)["emote"]=="t_pose"&&a3_view(&nb)["localEmote"]=="t_pose").await;let peer_original=a3_view(&nb)["localOriginal"].clone();let peer_event=a3_peer(&na,&peer_world)["event"].clone();
 // Real scoped transport outage; untouched peer retains its local state.
 for n in [&na,&nb]{let p=n.root.join("capture-first-loss");if p.is_file(){fixture_signal::remove(p).unwrap();}}
 for kind in ["http","wss"]{service.command(json!({"op":"transport","kind":kind,"enabled":true,"target":"consumer"}));}until(&mut service,"A3 outage obeys existing local hold",||native_activity(&na)["native_owned_hold"]==true&&a3_view(&na)["localEmote"]==""&&a3_view(&na)["motion"]==287).await;assert_eq!(a3_view(&nb)["robe"],"blue");assert_eq!(a3_view(&nb)["localEmote"],"t_pose");assert_eq!(a3_view(&nb)["localOriginal"],peer_original,"Healthy actor effect was restarted by another actor's outage");
 for kind in ["http","wss"]{service.command(json!({"op":"transport","kind":kind,"enabled":false,"target":"consumer"}));}ready_owner(&mut service,&a,&na,&world,true).await;fresh_bidirectional_ghosts(&mut service,&a,&na,&b,&nb,"A3 outage recovery preserves fresh ghosts").await;until(&mut service,"current cosmetics restore after revalidation",||a3_peer(&nb,&world)["robe"]=="red"&&a3_peer(&na,&peer_world)["robe"]=="blue").await;assert_eq!(a3_view(&na)["localEmote"],"");
 same_process_continue(&mut service,&a,&na,&world,&b,&nb).await;until(&mut service,"same-run Continue restores robe without resuming own emote",||a3_view(&na)["robe"]=="red"&&a3_view(&na)["localEmote"]==""&&a3_peer(&nb,&world)["robe"]=="red"&&a3_peer(&na,&peer_world)["emote"]=="t_pose"&&a3_peer(&na,&peer_world)["event"]==peer_event).await;fresh_bidirectional_ghosts(&mut service,&a,&na,&b,&nb,"A3 Continue fresh movement").await;checkpoint(&own,"pause-forms-display-setting-scoped-outage-cold-revalidation-and-same-run-Continue");
 // Affected A2 smoke uses the existing real native UI/service paths.
 click(&mut service,&na,"Players").await;until_info(&mut service,"A3 retains A2 HUD",&a,&na,&b,&nb,||view(&na)["hpBars"]==2).await;service.command(json!({"op":"chat","target":"host","text":"A3 retained chat 雪"}));until(&mut service,"A3 retains chat delivery",||text(&nb,"a2-notices.txt").contains("A3 retained chat 雪")).await;
 let move_file=na.root.join("move-source");if move_file.is_file(){fixture_signal::remove(move_file).unwrap();}click(&mut service,&na,"World message").await;fixture_signal::write(na.root.join("a2-text"),"A3 retained POI").unwrap();until(&mut service,"A3 POI draft",||text(&na,"a2-ui.txt").contains("15/99")).await;a3_place_poi(&mut service,&a,&na,&nb,"A3 retained POI").await;
 // Session turnover: cold app/native over the same owned save uses fresh
 // authority and restores cosmetics without a pre-save local animation.
 na.close();a.close().await;let store=a.store.clone();let mut cold=Client::start(&service.private(),&own.join("host-cold"),Some(store),false,false,router.routes.clone(),service.watches.clone(),service.stores.clone()).await;let mut nc=Native::start(&cold,Some(&na.root),true);
 wait(&mut service,&cold,"A3 cold saved-world explicit choice",15,|v|v["connection"]["state"]=="Choice").await;fixture_signal::write(nc.root.join("choose-rejoin"),b"explicit user rejoin choice after cold app restore").unwrap();
 ready_owner(&mut service,&cold,&nc,&world,true).await;fresh_bidirectional_ghosts(&mut service,&cold,&nc,&b,&nb,"A3 owner-session turnover restores fresh ghosts").await;until(&mut service,"cold current robe and aged peer loop restored",||a3_view(&nc)["robe"]=="red"&&a3_view(&nc)["localEmote"]==""&&a3_view(&nc)["particles"]==0&&a3_peer(&nb,&world)["robe"]=="red"&&a3_peer(&nb,&world)["hat"]==true&&a3_peer(&nb,&world)["gem"]==true&&a3_peer(&nc,&peer_world)["emote"]=="t_pose"&&a3_peer(&nc,&peer_world)["event"]==peer_event).await;
 assert!(!nc.root.join("seed-before-generation").exists(),"Saved-world launch must not reseed");
 service.command(json!({"op":"moderate","target":"peer","action":"ban"}));until(&mut service,"removed actor clears effects by canonical roster",||a3_view(&nc)["peers"][peer_world.as_str().unwrap()].is_null()&&a3_view(&nc)["active"].as_object().is_some_and(|v|v.is_empty())&&native_activity(&nc)["ghosts"]==0).await;let probe=nc.root.join("capture-first-loss");if probe.is_file(){fixture_signal::remove(probe).unwrap();}service.command(json!({"op":"end"}));until(&mut service,"End restores owned presentation and protects current player",||cold.handle.view()["connection"]["state"]=="Ended"&&native_activity(&nc)["native_owned_hold"]==true&&a3_view(&nc)["motion"]==287&&a3_view(&nc)["bodyVisible"]==true&&a3_view(&nc)["active"].as_object().is_some_and(|v|v.is_empty())).await;
 nc.close();nb.close();cold.close().await;b.close().await;service.close();checkpoint(&own,"A2-HUD-chat-POI-survive-owner-turnover-removal-End-and-owned-schema-store-cleanup");
 })).unwrap().join().unwrap();
}
