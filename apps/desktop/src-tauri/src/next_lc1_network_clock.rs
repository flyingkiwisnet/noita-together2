// Fixture clock contract only. Production authority and native clocks are unchanged.
pub(super) struct ClockProbe {initial:Value,last:Value,root:Option<std::path::PathBuf>}
fn clock_number(v:&Value,key:&str)->Result<u64,String>{v[key].as_u64().ok_or_else(||format!("Missing/noninteger fixture clock field: {key}"))}
fn clock_shape(v:&Value,mode:&str)->Result<(),String>{
 if v["revision"]!=1||v["mode"]!=mode{return Err("Fixture clock revision/mode differs".into())}
 for key in ["modeRevision","sampleSequence","monotonicSampleMs","elapsedSinceAnchorMs","setupAdvanceMs","explicitInjectedMs","ignoredWallAdvanceMs"]{clock_number(v,key)?;}
 if clock_number(v,"sampleSequence")?==0||v["elapsedRequiresAdvanceKind"]!=json!(mode=="elapsed")||v["ordinaryWallAdvance"]!="kind=wall; ignored in elapsed mode"||v["expiryAdvance"]!="kind=injected; adds offset in either mode"{return Err("Fixture clock counter/advance semantics differ".into())}
 Ok(())
}
fn clock_progress(before:&Value,after:&Value,injected:u64,wall:u64)->Result<(),String>{
 clock_shape(before,"elapsed")?;clock_shape(after,"elapsed")?;
 if before["modeRevision"]!=after["modeRevision"]||after["modeRevision"]!=1||clock_number(after,"sampleSequence")?<=clock_number(before,"sampleSequence")?{return Err("Fixture clock mode changed or sample sequence did not advance".into())}
 for key in ["monotonicSampleMs","elapsedSinceAnchorMs","setupAdvanceMs","explicitInjectedMs","ignoredWallAdvanceMs"]{if clock_number(after,key)?<clock_number(before,key)?{return Err(format!("Fixture clock regressed: {key}"))}}
 if after["setupAdvanceMs"]!=before["setupAdvanceMs"]||clock_number(after,"explicitInjectedMs")?-clock_number(before,"explicitInjectedMs")?!=injected||clock_number(after,"ignoredWallAdvanceMs")?-clock_number(before,"ignoredWallAdvanceMs")?!=wall{return Err("Unexpected setup/injected/duplicate wall advancement".into())}
 let observed=clock_number(after,"monotonicSampleMs")?-clock_number(before,"monotonicSampleMs")?;
 let elapsed=clock_number(after,"elapsedSinceAnchorMs")?-clock_number(before,"elapsedSinceAnchorMs")?;
 // status() samples monotonic time twice. This2ms allowance is sampling skew,
 // not an authority/request/observation/pose deadline or a simulated offset.
 if observed.abs_diff(elapsed.checked_add(injected).ok_or("Fixture clock offset overflow")?)>2{return Err("Fixture clock advanced beyond elapsed time plus explicit injection".into())}
 Ok(())
}
impl ClockProbe {
 fn new(clock:&Value)->Result<Self,String>{clock_shape(clock,"elapsed")?;if clock["modeRevision"]!=1||clock["explicitInjectedMs"]!=0||clock["ignoredWallAdvanceMs"]!=0{return Err("Network elapsed mode must start before wall/injected advances".into())}Ok(Self{initial:clock.clone(),last:clock.clone(),root:None})}
 pub(super) fn observe(&mut self,request:&Value,response:&Value)->Result<(),String>{
  let check=(||{
   let (injected,wall)=match request["op"].as_str(){
    Some("advance")=>{let ms=clock_number(request,"ms")?;if ms>3600000{return Err("Clock advance outside fixture bound".into())}match request["kind"].as_str(){Some("wall")=>{if response["advanced"]!=false{return Err("Elapsed wall request was advanced rather than ignored".into())}(0,ms)},Some("injected")=>{if response["advanced"]!=true{return Err("Explicit injected clock offset was ignored".into())}(ms,0)},_=>return Err("Elapsed advance requires explicit wall/injected kind".into())}},
    Some("clockStatus")=>(0,0),Some("clockMode")if request["mode"]=="elapsed"=>(0,0),_=>return Err("Unexpected clock response operation".into()),
   };clock_progress(&self.last,&response["clock"],injected,wall)
  })();
  if let Some(root)=&self.root{
   write_row(root,"network-clock-samples.jsonl",&json!({"request":request,"clock":response["clock"],"accepted":check.is_ok()}));
   if let Err(reason)=&check{let path=root.join("network-clock-first-loss.json");if !path.exists(){std::fs::write(path,serde_json::to_vec_pretty(&json!({"reason":reason,"initial":self.initial,"previous":self.last,"request":request,"response":response,"authority_deadlines_changed":false})).unwrap()).unwrap();}}
  }
  check?;self.last=response["clock"].clone();Ok(())
 }
}
pub(super) fn enable_elapsed_clock(service:&mut Service,root:&std::path::Path){
 assert!(service.clock_probe.is_none());
 let result=service.command(json!({"op":"clockMode","mode":"elapsed"}));
 let mut probe=ClockProbe::new(&result["clock"]).unwrap();probe.root=Some(root.to_path_buf());
 write_row(root,"network-clock-samples.jsonl",&json!({"phase":"elapsed selected immediately after Service start before Client/native traffic","clock":result["clock"]}));
 service.clock_probe=Some(probe);
}
fn clock_phase(before:&Value,after:&Value)->Result<Value,String>{
 let wall=clock_number(after,"ignoredWallAdvanceMs")?.checked_sub(clock_number(before,"ignoredWallAdvanceMs")?).ok_or("Ignored wall counter regressed")?;
 clock_progress(before,after,0,wall)?;
 Ok(json!({"revision":1,"mode":"elapsed","before":before,"after":after,"sample_sequence_delta":clock_number(after,"sampleSequence")?-clock_number(before,"sampleSequence")?,"monotonic_delta_ms":clock_number(after,"monotonicSampleMs")?-clock_number(before,"monotonicSampleMs")?,"elapsed_delta_ms":clock_number(after,"elapsedSinceAnchorMs")?-clock_number(before,"elapsedSinceAnchorMs")?,"setup_advance_delta_ms":0,"explicit_injected_delta_ms":0,"ignored_wall_advance_delta_ms":wall,"duplicate_wall_advancement":false}))
}

#[test]fn lc1_network_clock_consumer_preserves_manual_default_and_elapsed_offsets_without_shared_service(){
 let output=hidden("C:/Program Files/nodejs/node.exe").current_dir(repo()).args(["--import","tsx","apps/desktop/tools/lc1-clock-consumer-check.ts"]).output().unwrap();
 assert!(output.status.success(),"Pure fixture clock consumer failed: {}",String::from_utf8_lossy(&output.stderr));
 let trace:Value=serde_json::from_slice(&output.stdout).unwrap();let manual=trace["manual"].as_array().unwrap();
 for row in manual{clock_shape(&row["response"]["clock"],"manual").unwrap();}
 assert_eq!(manual[0]["response"]["clock"]["monotonicSampleMs"],10000,"Manual default must ignore unsent wall passage");
 assert_eq!(manual[1]["response"]["clock"]["monotonicSampleMs"],10120,"Manual setup step is preserved");
 assert_eq!(manual[2]["response"]["clock"]["monotonicSampleMs"],10270,"Manual wall tag keeps old advancement");
 assert_eq!(manual[3]["response"]["clock"]["monotonicSampleMs"],41270,"Manual explicit expiry offset is preserved");
 let elapsed=trace["elapsed"].as_array().unwrap();let initial=&elapsed[0]["response"]["clock"];let mut probe=ClockProbe::new(initial).unwrap();
 for row in &elapsed[1..3]{probe.observe(&row["request"],&row["response"]).unwrap();}
 let ordinary=&elapsed[2]["response"]["clock"];let measured=clock_phase(initial,ordinary).unwrap();assert_eq!(measured["monotonic_delta_ms"],300);assert_eq!(measured["ignored_wall_advance_delta_ms"],150);assert_eq!(measured["explicit_injected_delta_ms"],0);
 assert_eq!(elapsed[3]["error"],"FIXTURE_CLOCK_ARGUMENT","Elapsed untyped offset must be rejected");
 probe.observe(&elapsed[4]["request"],&elapsed[4]["response"]).unwrap();assert_eq!(elapsed[4]["response"]["clock"]["explicitInjectedMs"],31000);
 assert!(clock_phase(ordinary,&elapsed[4]["response"]["clock"]).is_err(),"Ordinary phases cannot conceal explicit expiry offset");
 assert_eq!(elapsed[5]["error"],"FIXTURE_CLOCK_MODE");
 let root=repo().join("spikes/noita-entry/.build/lc1-network-client-20261020-11");std::fs::create_dir_all(&root).unwrap();std::fs::write(root.join("clock-consumer-check.json"),serde_json::to_vec_pretty(&trace).unwrap()).unwrap();
 for fault in 0..7{let mut bad=elapsed[1]["response"].clone();match fault{
  0=>bad["clock"]["monotonicSampleMs"]=json!(10420),
  1=>bad["clock"]["setupAdvanceMs"]=json!(240),
  2=>bad["clock"]["modeRevision"]=json!(2),
  3=>bad["clock"]["explicitInjectedMs"]=json!(1),
  4=>bad["clock"]["ignoredWallAdvanceMs"]=json!(300),
  5=>bad["advanced"]=json!(true),
  _=>bad["clock"]["sampleSequence"]=initial["sampleSequence"].clone(),
 };assert!(ClockProbe::new(initial).unwrap().observe(&elapsed[1]["request"],&bad).is_err(),"Invalid clock fault{fault} accepted");}
}

#[test]fn lc1_network_full_matrix_elapsed_clock_accounts_all_seven_injection_sites(){
 let output=hidden("C:/Program Files/nodejs/node.exe").current_dir(repo()).args(["--import","tsx","apps/desktop/tools/lc1-clock-consumer-check.ts"]).output().unwrap();assert!(output.status.success());let trace:Value=serde_json::from_slice(&output.stdout).unwrap();let matrix=&trace["matrix"];assert_eq!(matrix["sites"],json!([901000,901000,31000,31000,31000,31000,31000]));let rows=matrix["rows"].as_array().unwrap();assert_eq!(rows.len(),22);let initial=&rows[0]["response"]["clock"];let mut probe=ClockProbe::new(initial).unwrap();let mut total=0;
 for chunk in rows[1..].chunks_exact(3){let before=probe.last.clone();probe.observe(&chunk[0]["request"],&chunk[0]["response"]).unwrap();let phase=clock_phase(&before,&chunk[0]["response"]["clock"]).unwrap();assert_eq!(phase["explicit_injected_delta_ms"],0);assert_eq!(phase["monotonic_delta_ms"],100);let amount=chunk[1]["request"]["ms"].as_u64().unwrap();total+=amount;probe.observe(&chunk[1]["request"],&chunk[1]["response"]).unwrap();assert_eq!(clock_number(&probe.last,"explicitInjectedMs").unwrap(),total);assert!(clock_phase(&before,&probe.last).is_err(),"Ordinary phase concealed deliberate expiry");let injected=probe.last.clone();probe.observe(&chunk[2]["request"],&chunk[2]["response"]).unwrap();assert_eq!(clock_phase(&injected,&probe.last).unwrap()["monotonic_delta_ms"],100);}
 assert_eq!(total,1957000);assert_eq!(clock_number(&probe.last,"setupAdvanceMs").unwrap(),clock_number(initial,"setupAdvanceMs").unwrap());assert_eq!(clock_number(&probe.last,"ignoredWallAdvanceMs").unwrap(),1750);
 let root=repo().join("spikes/noita-entry/.build/lc1-network-client-20261020-11");std::fs::create_dir_all(&root).unwrap();std::fs::write(root.join("matrix-clock-consumer-check.json"),serde_json::to_vec_pretty(matrix).unwrap()).unwrap();
}
