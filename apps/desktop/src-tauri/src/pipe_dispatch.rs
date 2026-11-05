// Shared binary request handler. Transport adapters supply an authenticated local owner.
#[derive(Clone)]
pub(crate) struct Dispatcher {
 pub pending:Arc<Mutex<Option<super::handoff::Pending>>>,
 pub diagnostics:Option<Arc<Mutex<NativeDiagnostics>>>,
 pub g3:Option<Arc<Mutex<nt2_desktop_g0::g3_runtime::Exchange>>>,
 pub social:Option<Arc<Mutex<nt2_desktop_g0::g3_social_stream::Exchange>>>,
 pub startup:Option<Arc<startup::Lane>>,
}
impl Dispatcher {
 pub(crate) fn reply(&self,pid:crate::local_owner::Id,bytes:&[u8])->Option<Vec<u8>> {
  self.prepare(pid,bytes).map(|(reply,_)|reply)
 }
 pub(crate) fn prepare(&self,pid:crate::local_owner::Id,bytes:&[u8])->Option<(Vec<u8>,bool)> {
  if bytes.len()<32||bytes.len()>131072{return None}
  let request=bytes;let n=request.len() as u32;let inspection=true;
  let pending=self.pending.clone();let diagnostics=self.diagnostics.clone();let g3=self.g3.clone();let social=self.social.clone();let startup=self.startup.clone();
                            let provisional=diagnostics.as_ref().and_then(|d|d.lock().unwrap().provisional.clone());
                            let world=diagnostics.as_ref().and_then(|d|d.lock().unwrap().world.clone());
                            let mut early_reply=None;
                            #[cfg(test)]let retiring_before=if matches!(u32::from_le_bytes(request[8..12].try_into().unwrap()),2|6|17){let slot=pending.lock().unwrap();Some((collection_view(&slot),slot.as_ref().is_some_and(|p|p.owner==Some((pid,request[16..32].try_into().unwrap())))))}else{None};
                            let mut reply:Vec<u8> =
                                if n==32&&request[..4]==super::handoff::MAGIC.to_le_bytes()&&request[4..8]==10u32.to_le_bytes()&&request[8..12]==20u32.to_le_bytes()&&request[12..16]==[0;4]{
                                    lobby_display_frame(diagnostics.as_ref().and_then(|d|d.lock().ok()).as_ref().and_then(|d|d.lobby_display.as_ref()),&request[16..32])
                                }else if startup.is_some()&&request[..4]==super::handoff::MAGIC.to_le_bytes()&&request[4..8]==9u32.to_le_bytes(){
                                    #[cfg(not(test))]let delay=Duration::ZERO;
                                    #[cfg(test)]let delay=Duration::from_millis(diagnostics.as_ref().map(|d|d.lock().unwrap().startup_claim_delay_ms).unwrap_or(0));
                                    if let Some(world)=&world{world.lane.handle_instance(pid,owner_instance(pid).unwrap_or(0),&request[..n as usize],delay)}else{dispatch_startup(startup.as_ref().unwrap(),provisional.as_ref(),pid,owner_instance(pid).unwrap_or(0),&request[..n as usize],delay)}
                                }else if diagnostics.is_some() && request[..4]==super::handoff::MAGIC.to_le_bytes() && matches!(u32::from_le_bytes(request[4..8].try_into().unwrap()),7|8){
                                    let context:[u8;16]=request[16..32].try_into().unwrap();
                                    let canonical=pending.lock().unwrap().as_ref().is_some_and(|p|p.owner==Some((pid,context)));
                                    let b=provisional.as_ref().filter(|b|!canonical&&b.owns_context(pid,&context));
                                    if n==32&&request[4..8]==8u32.to_le_bytes()&&request[8..12]==13u32.to_le_bytes()&&world.as_ref().is_some_and(|b|b.owns(pid,&context)){
                                        let metadata=world.as_ref().unwrap().marker(pid,context);let mut value=vec![0;1072];value[..4].copy_from_slice(&super::handoff::MAGIC.to_le_bytes());value[4..8].copy_from_slice(&8u32.to_le_bytes());value[16..32].copy_from_slice(&context);if let Some(metadata)=metadata{value[8..12].copy_from_slice(&1u32.to_le_bytes());value[12..16].copy_from_slice(&(metadata.len() as u32).to_le_bytes());value[48..48+metadata.len()].copy_from_slice(&metadata);}value
                                    }else if !canonical&&n==32&&request[4..8]==8u32.to_le_bytes()&&request[8..12]==15u32.to_le_bytes()&&provisional.as_ref().is_some_and(|b|b.explicit_offline(pid,owner_instance(pid).unwrap_or(0),context)){
                                        let mut value=vec![0;48];value[..4].copy_from_slice(&super::handoff::MAGIC.to_le_bytes());value[4..8].copy_from_slice(&8u32.to_le_bytes());value[8..12].copy_from_slice(&1u32.to_le_bytes());value[16..32].copy_from_slice(&context);value
                                    }else if request[4..8]==8u32.to_le_bytes()&&request[8..12]==13u32.to_le_bytes()&&n==32&&b.is_some(){
                                        if let Some(metadata)=b.unwrap().metadata(pid,context){let mut value=vec![0;1072];value[..4].copy_from_slice(&super::handoff::MAGIC.to_le_bytes());value[4..8].copy_from_slice(&8u32.to_le_bytes());value[8..12].copy_from_slice(&1u32.to_le_bytes());value[12..16].copy_from_slice(&(metadata.len() as u32).to_le_bytes());value[16..32].copy_from_slice(&context);value[48..48+metadata.len()].copy_from_slice(&metadata);value}
                                        else{vec![0;1072]}
                                    }else if let Some(b)=b{dispatch_rejoin(diagnostics.as_ref().unwrap(),&b.pending,pid,&request[..n as usize])}
                                    else{dispatch_rejoin(diagnostics.as_ref().unwrap(),&pending,pid,&request[..n as usize])}
                                }else if cfg!(feature="g3-c2-live")&&g3.is_some()&&social.is_some()&&request[..4]==super::handoff::MAGIC.to_le_bytes()&&matches!(u32::from_le_bytes(request[4..8].try_into().unwrap()),4|5|6){
                                    let bytes=&request[..n as usize];let context=if request[4..8]==5u32.to_le_bytes(){bytes.get(24..40)}else{bytes.get(16..32)};
                                    let canonical=context.is_some_and(|ctx|pending.lock().unwrap().as_ref().is_some_and(|p|p.owner.is_some_and(|o|o.0==pid&&o.1.as_slice()==ctx)));
                                    if !canonical&&context.is_some_and(|ctx|world.as_ref().is_some_and(|b|b.owns(pid,ctx))){early_reply=world.clone();world.as_ref().unwrap().frame(pid,bytes)}
                                    else if !canonical{provisional.as_ref().and_then(|b|b.frame(pid,bytes)).unwrap_or_else(||c2_transition(&mut pending.lock().unwrap(),pid,bytes))}
                                    else{c2_transition(&mut pending.lock().unwrap(),pid,bytes)}
                                }else if social.is_some() && request[..4]==*b"NSP1" {
                                    let context:[u8;16]=request[8..24].try_into().unwrap();let playing=g3_playing(&pending.lock().unwrap(),pid,&context);
                                    social.as_ref().unwrap().lock().unwrap().transition(pid,&request[..n as usize],playing)
                                } else if g3.is_some() && request[..4]==*b"NTG3" {
                                    let context:[u8;16]=request[8..24].try_into().unwrap();
                                    let playing=g3_playing(&pending.lock().unwrap(),pid,&context);
                                    g3.as_ref().unwrap().lock().unwrap().transition(pid,&request[..n as usize],playing).to_vec()
                                } else if inspection && n as usize >= crate::native_report::HEADER_BYTES {
                                    report_transition(
                                        &mut pending.lock().unwrap(),
                                        pid,
                                        &request[..n as usize],
                                    ).to_vec()
                                } else {
                                    super::handoff::transition(
                                        &mut pending.lock().unwrap(),
                                        pid,
                                        request[..32].try_into().unwrap(),
                                    ).to_vec()
                                };
                            #[cfg(test)]
                            if n==32&&u32::from_le_bytes(request[8..12].try_into().unwrap())==5{
                                if let Some(d)=&diagnostics{let mut d=d.lock().unwrap();if std::mem::take(&mut d.deny_next_heartbeat){reply[8..12].fill(0);}if std::mem::take(&mut d.malform_next_heartbeat){reply[0]^=1;}}
                            }
                            if let Some(g3)=&g3{
                                let current=pending.lock().unwrap();
                                if current.as_ref().and_then(|p|p.owner).is_none_or(|(owner,context)|!g3_playing(&current,owner,&context)){
                                    g3.lock().unwrap().pause();
                                }
                            }
                            if let Some(d) = &diagnostics {
                                let op = u32::from_le_bytes(request[8..12].try_into().unwrap());
                                let acknowledged =
                                    u32::from_le_bytes(reply[8..12].try_into().unwrap()) == 1;
                                let mut d = d.lock().unwrap();
                                if op==1&&acknowledged {d.continue_detected=false;}
                                let status = match (op, acknowledged) {
                                    (1, true) => Some(
                                        "Game handshake accepted; waiting for seed confirmation",
                                    ),
                                    (1, false) => Some(
                                        "Game prepare refused: no current armed grant or grant already consumed/expired; use a fresh host preparation and Load game before New Game",
                                    ),
                                    (2, _) => Some(
                                        "Legacy cancellation ignored without exact current owner; engine world mode unresolved",
                                    ),
                                    (3, true) => Some(
                                        "Game confirmed the injected seed; waiting for player/report",
                                    ),
                                    (4, true) => {
                                        Some("Game loaded; waiting for actual seed/mod inspection")
                                    }
                                    (7, true) => Some(
                                        "Local game inspection received; service upload/acceptance is shown separately",
                                    ),
                                    (7, false) => {
                                        Some("Game inspection refused; current world remains held")
                                    }
                                    (6, _) => Some(
                                        "Game connection closed; a replacement world needs a fresh preparation",
                                    ),
                                    _ => None,
                                };
                                if let Some(status) = status {
                                    d.note(status);
                                }
                            }
                            #[cfg(test)]
                            if n == 32 && request[..4] == super::handoff::MAGIC.to_le_bytes() && request[4..8]!=9u32.to_le_bytes() {
                                let op = u32::from_le_bytes(request[8..12].try_into().unwrap());
                                if let Some(d) = &diagnostics {
                                    let delay=record_initial_contact(&pending,d,op,reply[8]==1,||{});
                                    if let Some(ms) = delay {thread::sleep(Duration::from_millis(ms));}
                                    if op==1 && std::mem::take(&mut d.lock().unwrap().initial_contact_invalid_reply) {reply[0]^=1;}
                                }
                            }
                            #[cfg(test)]if let (Some(d),Some((before,owner_matches)))=(&diagnostics,retiring_before){let after=collection_view(&pending.lock().unwrap());let mut d=d.lock().unwrap();if d.lc1_retirement_trace.len()==32{d.lc1_retirement_trace.remove(0);}d.lc1_retirement_trace.push(serde_json::json!({"op":u32::from_le_bytes(request[8..12].try_into().unwrap()),"version":u32::from_le_bytes(request[4..8].try_into().unwrap()),"pid":pid,"owner_context_matches":owner_matches,"before":before,"after":after}));}
                            // Fence a social reply again at delivery, after all listener work.
                            if let Some(social)=&social{if request[..4]==*b"NSP1"{let context:[u8;16]=request[8..24].try_into().unwrap();if !g3_playing(&pending.lock().unwrap(),pid,&context){reply=social.lock().unwrap().transition(pid,&request[..n as usize],false);}}}
                            if reply.len()==48&&matches!(u32::from_le_bytes(request[4..8].try_into().unwrap()),4|5|6){
                                #[cfg(test)]let before=(reply[8],u32::from_le_bytes(reply[32..36].try_into().unwrap()),u32::from_le_bytes(reply[36..40].try_into().unwrap()));
                                c2_delivery_fence(&pending,early_reply.as_ref(),pid,&request[..n as usize],&mut reply);
                                #[cfg(test)]if let Some(d)=&diagnostics{let slot=if let Some(b)=&early_reply{b.pending.clone()}else{pending.clone()};let slot=slot.lock().unwrap();let version=u32::from_le_bytes(request[4..8].try_into().unwrap());let ctx=if version==5{&request[24..40]}else{&request[16..32]};let snapshot=collection_view(&slot);let owner=slot.as_ref().is_some_and(|p|p.owner.is_some_and(|o|o.0==pid));let context=slot.as_ref().is_some_and(|p|p.owner.is_some_and(|o|o.1.as_slice()==ctx));drop(slot);let mut d=d.lock().unwrap();let event=serde_json::json!({"op":u32::from_le_bytes(request[8..12].try_into().unwrap()),"version":version,"source":if early_reply.is_some(){"early-world"}else{"common"},"owner_matches":owner,"context_matches":context,"ack_before":before.0,"lease_before_ms":before.1,"epoch_before":before.2,"ack_delivered":reply[8],"lease_delivered_ms":u32::from_le_bytes(reply[32..36].try_into().unwrap()),"epoch_delivered":u32::from_le_bytes(reply[36..40].try_into().unwrap()),"slot":snapshot});if d.lc1_collection_trace.len()<32{d.lc1_collection_trace.push(event.clone());}if d.lc1_collection_recent.len()==16{d.lc1_collection_recent.remove(0);}d.lc1_collection_recent.push(event);}
                            }
                            #[cfg(test)] if request[4..8]==9u32.to_le_bytes()&&reply[12..16]==1u32.to_le_bytes(){if let Some(d)=&diagnostics{if std::mem::take(&mut d.lock().unwrap().startup_drop_ready_once){return None;}}}
                            if request[..4]==super::handoff::MAGIC.to_le_bytes()&&request[4..8]==8u32.to_le_bytes()&&request[8..12]==13u32.to_le_bytes()&&n==32{let context:[u8;16]=request[16..32].try_into().unwrap();metadata_display(&pending.lock().unwrap(),pid,&context,&mut reply);}

 Some((reply,early_reply.is_some()))
 }
 // `early` records the original dispatch source. Retirement must never turn a
 // reply from the common slot into a grant from an unrelated replacement slot.
 pub(crate) fn delivery(&self,pid:crate::local_owner::Id,request:&[u8],mut reply:Vec<u8>,early:bool)->Option<Vec<u8>> {
  if request.len()<32||reply.len()<32{return None}
  let world=self.diagnostics.as_ref().and_then(|d|d.lock().unwrap().world.clone());
  if reply.len()==48&&request[..4]==super::handoff::MAGIC.to_le_bytes()&&matches!(u32::from_le_bytes(request[4..8].try_into().unwrap()),4|5|6){
   if early&&world.is_none(){return None}
   c2_delivery_fence(&self.pending,if early{world.as_ref()}else{None},pid,request,&mut reply);
  }
  if request[..4]==*b"NSP1"{let context=request[8..24].try_into().unwrap();if !g3_playing(&self.pending.lock().unwrap(),pid,&context){reply=self.social.as_ref()?.lock().unwrap().transition(pid,request,false);}}
  if request[..4]==*b"NTG3"{let context=request[8..24].try_into().unwrap();if !g3_playing(&self.pending.lock().unwrap(),pid,&context){reply=self.g3.as_ref()?.lock().unwrap().transition(pid,request,false).to_vec();}}
  if request[..4]==super::handoff::MAGIC.to_le_bytes()&&request[4..8]==9u32.to_le_bytes()&&reply[12..16]==1u32.to_le_bytes(){
   let instance=owner_instance(pid)?;
   reply=if let Some(world)=world{world.lane.delivery(pid,instance,request)}else{self.startup.as_ref()?.delivery(pid,instance,request)};
  }
  Some(reply)
 }
}
