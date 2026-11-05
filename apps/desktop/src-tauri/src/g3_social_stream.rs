//! UNAPPROVED candidate stream. Only Core arms owner/lease; native listener checks PID.
use crate::g3_social_frame::{Queue,Command};
use std::{time::Instant,collections::BTreeMap};
pub const BANK_FOOTER:usize=64+32768;
pub const REPLY:usize=BANK_FOOTER+16+8192;
// HUD encoding reserves these existing transport bounds; no TTL is renewed by
// pending ACK retransmission or the DLL's cached reply lease.
pub const MAX_ACK_AGE_MS:u64=100;
pub const MAX_LEASE_MS:u64=75;
pub struct Exchange{race_copy_last:&'static str,race_serial:u64,race_owner:Option<(crate::local_owner::Id,[u8;16],u32,Instant)>,race_output:serde_json::Value,race_pending:Option<(u64,u64,Vec<u8>)>,pub(crate) bank_wake:std::sync::Arc<tokio::sync::Notify>,owner:Option<(crate::local_owner::Id,[u8;16],u32)>,until:Instant,serial:u64,queue:Option<Queue>,pending:Option<Vec<u8>>,pending_at:Instant,ack:u32,aborted:bool,terminal:bool,terminal_context:Option<[u8;16]>,input:Option<(u64,Vec<u8>)>,input_ack:u64,rescan:bool,input_at:Option<Instant>,input_received:u64,input_acknowledged:u64,input_rejected:u64,chat_frames:BTreeMap<u32,u32>,chat_receipt_high:u32,ack_reset_required:bool,debug_enabled:std::sync::Arc<std::sync::atomic::AtomicBool>,debug_report:Option<crate::debug_diagnostics::ModReport>,debug_ack:u64,debug_rejected:u64,d1_pending:Option<(u64,Vec<u8>)>,bank_pending:Option<(u64,Vec<u8>)>,bank_ack:u64,bank_output:Vec<u8>,d1_output:serde_json::Value,c2_output:serde_json::Value,optional_turn:u8,c1_output:serde_json::Value,c1_completion:bool,c1_pending:Option<(u64,Vec<u8>)>,c1_ack:u64,c1_bootstrap:Option<(crate::local_owner::Id,[u8;16],u32,u32,Instant)>}
impl Default for Exchange{fn default()->Self{Self{race_copy_last:"none",race_serial:1,race_owner:None,race_output:serde_json::Value::Null,race_pending:None,bank_wake:std::sync::Arc::new(tokio::sync::Notify::new()),owner:None,until:Instant::now(),serial:1,queue:None,pending:None,pending_at:Instant::now(),ack:0,aborted:false,terminal:false,terminal_context:None,input:None,input_ack:0,rescan:true,input_at:None,input_received:0,input_acknowledged:0,input_rejected:0,chat_frames:BTreeMap::new(),chat_receipt_high:0,ack_reset_required:false,debug_enabled:std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true)),debug_report:None,debug_ack:0,debug_rejected:0,d1_pending:None,bank_pending:None,bank_ack:0,bank_output:Vec::new(),d1_output:serde_json::Value::Null,c2_output:serde_json::Value::Null,optional_turn:0,c1_output:serde_json::Value::Null,c1_completion:false,c1_pending:None,c1_ack:0,c1_bootstrap:None}}}
impl Exchange{
 pub fn race_arm(&mut self,pid:crate::local_owner::Id,context:[u8;16],generation:u32,until:Instant)->Result<(),String>{
  if pid==0||context==[0;16]||generation==0||until<=Instant::now(){self.race_owner=None;self.c1_ack=0;return Err("Race metadata owner invalid".into())}
  if self.race_owner.is_none_or(|(p,c,g,_)|(p,c,g)!=(pid,context,generation)){self.race_serial=self.race_serial.checked_add(1).ok_or("Race copy generation exhausted")?;self.race_output=serde_json::Value::Null;self.race_pending=None;self.c1_ack=0;}
  self.race_owner=Some((pid,context,generation,until));Ok(())
 }
 pub fn race_disarm(&mut self){if self.race_owner.take().is_some(){self.c1_ack=0;}self.race_output=serde_json::Value::Null;}
 pub fn race_view_for(&mut self,context:[u8;16],view:serde_json::Value){if self.race_owner.is_some_and(|(_,c,_,until)|c==context&&until>Instant::now()){self.race_output=view;}}
 pub fn race_input(&self)->Option<(u64,u64,crate::race_work::Input)>{let(serial,sequence,body)=self.race_pending.as_ref()?;Some((*serial,*sequence,crate::race_work::Input::decode(body)?))}
 pub fn acknowledge_race(&mut self,serial:u64,sequence:u64){if self.race_pending.as_ref().is_some_and(|(s,q,_)|(*s,*q)==(serial,sequence)){self.race_pending=None;if self.race_serial==serial&&self.race_owner.is_some(){self.c1_ack=sequence;}}}
 pub fn c1_bootstrap_arm(&mut self,pid:crate::local_owner::Id,context:[u8;16],generation:u32,ownership:u32,until:Instant)->Result<(),String>{
  if self.terminal_context==Some(context)||pid==0||context==[0;16]||generation==0||ownership==0||until<=Instant::now(){self.clear_c1_bootstrap();return Err("Invalid C1 metadata owner".into())}
  if self.c1_completion{self.c1_output=serde_json::Value::Null;}self.c1_completion=false;
  let same=self.c1_bootstrap.is_some_and(|(p,c,g,o,_)|(p,c,g,o)==(pid,context,generation,ownership));
  // Copy receipts and uncaptured inputs belong to this exact native owner.
  // A retained app must not send its previous native ACK to a fresh sequence.
  if !same{self.c1_output=serde_json::Value::Null;self.c2_output=serde_json::Value::Null;self.c1_pending=None;self.c1_ack=0;}self.c1_bootstrap=Some((pid,context,generation,ownership,until));Ok(())
 }
 pub fn c1_completion_arm(&mut self,pid:crate::local_owner::Id,context:[u8;16],generation:u32,ownership:u32,until:Instant)->Result<(),String>{
  let current=self.c1_bootstrap;let mode=self.c1_completion;let view=self.c1_output.clone();self.c1_bootstrap_arm(pid,context,generation,ownership,until)?;
  if !mode||current.is_none_or(|(p,c,g,o,_)|(p,c,g,o)!=(pid,context,generation,ownership)){self.c1_pending=None;self.c1_ack=0;}else{self.c1_output=view;}self.c1_completion=true;Ok(())
 }
 // Scoped observation only. These fields never grant or extend authority.
 pub fn c1_metadata_diagnostics(&self,pid:crate::local_owner::Id,context:[u8;16],native_epoch:u32)->Option<serde_json::Value>{
  let(owner,current,epoch,_,until)=self.c1_bootstrap?;
  if owner!=pid||current!=context||until<=Instant::now()||self.terminal_context==Some(context){return None}
  Some(serde_json::json!({"bootstrap":!self.c1_completion,"metadata_ready":self.c1_output["initialized"]==true,"native_epoch_matches":epoch==native_epoch}))
 }
 pub fn clear_c1_bootstrap(&mut self){if self.c1_bootstrap.take().is_some(){self.c1_output=serde_json::Value::Null;}self.c1_completion=false;self.c1_pending=None;}
 pub fn c1_bootstrap_view(&mut self,context:[u8;16],ownership:u32,view:serde_json::Value){
  if self.c1_bootstrap.is_some_and(|(_,c,_,o,until)|c==context&&o==ownership&&until>Instant::now())&&self.terminal_context!=Some(context){
   // Only inert policy/projection metadata can reach a protected engine.
   self.c1_output=serde_json::json!({"enabled":view["enabled"],"world":view["world"],"snapshot":view["snapshot"],"memberCount":view["memberCount"],"initialized":view["initialized"],"sequence":view["sequence"],"status":view["status"],"event":null,"permit":null,"completion":self.c1_completion,"localAccepted":if self.c1_completion{view["localAccepted"].clone()}else{serde_json::Value::Null}});
  }
 }
 pub fn c1_input(&self,protected:bool)->Option<(u64,crate::c1_work::Input,bool)>{
  if let Some((seq,bytes))=&self.c1_pending{return Some((*seq,crate::c1_work::Input::decode(bytes)?,true))}
  if protected{return None}let(seq,bytes)=self.bank_pending.as_ref()?;Some((*seq,crate::c1_work::Input::decode(bytes)?,false))
 }
 pub fn acknowledge_c1(&mut self,context:[u8;16],sequence:u64,lane:bool){
  if !lane{self.acknowledge_bank(context,sequence);return}
  let readonly=self.c1_bootstrap.is_some_and(|(_,c,_,_,until)|c==context&&until>Instant::now())&&self.terminal_context!=Some(context)&&self.c1_pending.as_ref().is_some_and(|(_,body)|crate::c2_work::Input::decode(body).is_some_and(|i|i.protected_allowed(false)));
  let allowed=readonly||self.owner.is_some_and(|(_,c,_)|c==context)&&!self.terminal&&Instant::now()<self.until||self.c1_completion&&self.c1_bootstrap.is_some_and(|(_,c,_,_,until)|c==context&&until>Instant::now())&&self.terminal_context!=Some(context);
  if allowed&&self.c1_pending.as_ref().is_some_and(|(s,_)|*s==sequence){self.c1_ack=sequence;self.c1_pending=None;}
 }
 pub fn c2_input(&self,protected:bool,completion:bool)->Option<(u64,crate::c2_work::Input,bool)>{
  if let Some((seq,body))=&self.c1_pending{let input=crate::c2_work::Input::decode(body)?;if !protected||input.protected_allowed(completion){return Some((*seq,input,true))}return None}
  if protected{return None}let(seq,body)=self.bank_pending.as_ref()?;Some((*seq,crate::c2_work::Input::decode(body)?,false))
 }
 pub fn c2_view_for(&mut self,context:[u8;16],mut view:serde_json::Value,protected:bool){
  let current=if protected{self.c1_bootstrap.is_some_and(|(_,c,_,_,until)|c==context&&until>Instant::now())&&self.terminal_context!=Some(context)}else{self.owner.is_some_and(|(_,c,_)|c==context)&&!self.terminal&&Instant::now()<self.until};
  if current{if protected{view["event"]=serde_json::Value::Null;view["permit"]=serde_json::Value::Null;}self.c2_output=view;}
 }
 fn c1_copy(&mut self,sequence:u64,body:&[u8],completion_allowed:bool)->bool{
  let race=body.starts_with(br#"{"kind":"race_completion","#);
  if race{
   self.race_copy_last=match (self.race_owner,crate::race_work::Input::decode(body)){
    (None,_)=>"no-owner",(Some((_,_,_,until)),_)if until<=Instant::now()=>"expired-owner",(_,None)=>"invalid-input",
    (Some((_,context,_,_)),Some(input))if input.context!=context.iter().map(|b|format!("{b:02x}")).collect::<String>()=>"context-mismatch",
    _ if sequence<=self.c1_ack=>"already-copied",_ if self.race_pending.is_some()=>"occupied",
    _=>{self.race_pending=Some((self.race_serial,sequence,body.to_vec()));self.bank_wake.notify_one();"copied"}};return true
  }
  let c1=body.starts_with(br#"{"kind":"c1_completion","#);let c2=body.starts_with(br#"{"kind":"c2_completion","#);
  let allowed=c1&&completion_allowed&&crate::c1_work::Input::decode(body).is_some_and(|i|i.completion_allowed())||c2&&crate::c2_work::Input::decode(body).is_some_and(|i|i.protected_allowed(completion_allowed));
  if allowed&&sequence>self.c1_ack&&self.c1_pending.is_none(){self.c1_pending=Some((sequence,body.to_vec()));self.bank_wake.notify_one();}c1||c2
 }
 pub fn c1_view_for(&mut self,context:[u8;16],view:serde_json::Value){if self.owner.is_some_and(|(_,c,_)|c==context)&&!self.terminal&&Instant::now()<self.until{self.c1_output=view;}}
 pub fn bank_view(&mut self,bytes:Vec<u8>){if bytes.len()<=8192{self.bank_output=bytes;}}
 pub fn bank_view_for(&mut self,context:[u8;16],bytes:Vec<u8>){if self.owner.is_some_and(|(_,c,_)|c==context)&&!self.terminal&&Instant::now()<self.until{self.bank_view(bytes);}}
 pub fn clear_deleted_bank(&mut self){if let Some((sequence,_))=self.bank_pending.take(){self.bank_ack=self.bank_ack.max(sequence);}self.bank_output=br#"{"run":"deleted","fresh":false,"enabled":false,"queued":0,"version":"0","balance":null,"status":"Lobby deleted; transfers removed.","notices":[]}"#.to_vec();}
 pub fn d1_input(&self)->Option<(u64,u64,crate::d1_work::Input)>{let(seq,body)=self.d1_pending.as_ref()?;Some((self.serial,*seq,crate::d1_work::Input::decode(body)?))}
 pub fn acknowledge_d1(&mut self,serial:u64,sequence:u64){if self.serial==serial&&self.d1_pending.as_ref().is_some_and(|(seq,_)|*seq==sequence){self.d1_pending=None;}}
 pub fn d1_view_for(&mut self,context:[u8;16],view:serde_json::Value){if self.owner.is_some_and(|(_,c,_)|c==context)&&!self.terminal&&Instant::now()<self.until{self.d1_output=view;}}
 pub fn bank_input(&self)->Option<(u64,crate::g3_bank::Input)>{let(seq,bytes)=self.bank_pending.as_ref()?;Some((*seq,crate::g3_bank::Input::decode(bytes).ok()?))}
 pub fn acknowledge_bank(&mut self,context:[u8;16],sequence:u64){if self.owner.is_some_and(|(_,c,_)|c==context)&&self.bank_pending.as_ref().is_some_and(|(s,_)|*s==sequence){self.bank_ack=sequence;self.bank_pending=None;}}
 pub fn debug_control(&mut self,enabled:std::sync::Arc<std::sync::atomic::AtomicBool>){self.debug_enabled=enabled;}
 pub fn take_debug_report(&mut self)->Option<crate::debug_diagnostics::ModReport>{self.debug_report.take()}

 /// Bounded read-only fixture diagnostics: no identity, payload or authority grant.
 pub fn progress(&self)->serde_json::Value{serde_json::json!({"race_copy":self.race_copy_last,"race_copy_ack":self.c1_ack,"race_copy_pending":self.race_pending.as_ref().map(|(_,q,_)|q),"race_copy_owner_live":self.race_owner.is_some_and(|(_,_,_,until)|until>Instant::now()),"ready":self.ready(),"queued":self.queue.as_ref().is_some_and(Queue::queued),"pending_frame":self.pending.as_ref().map(|f|u32::from_le_bytes(f[28..32].try_into().unwrap())),"pending_bytes":self.pending.as_ref().map_or(0,Vec::len),"pending_age_ms":self.pending.as_ref().map(|_|self.pending_at.elapsed().as_millis()),"ack":self.ack,"aborted":self.aborted,"rescan":self.rescan,"terminal":self.terminal,"c1_bootstrap":self.c1_bootstrap.is_some(),"c1_terminal_context":self.terminal_context.is_some(),"input_pending":self.input.is_some(),"input_age_ms":self.input_at.map(|t|t.elapsed().as_millis()),"inputs_received":self.input_received,"inputs_acknowledged":self.input_acknowledged,"inputs_rejected":self.input_rejected})}
 pub fn arm(&mut self,pid:crate::local_owner::Id,context:[u8;16],generation:u32,until:Instant)->Result<(),String>{
  self.clear_c1_bootstrap();
  if pid==0||context==[0;16]||generation==0||until<=Instant::now(){self.pause();return Err("Social owner/lease".into());}
  if self.terminal&&self.owner.is_some_and(|(_,old,_)|old==context){return Err("Terminal social world needs fresh context".into());}
  if self.owner!=Some((pid,context,generation)){
   let same_context=self.owner.is_some_and(|(_,old,_)|old==context);
   let sequence=if same_context{self.queue.as_ref().map_or(self.ack,|q|q.sequence().max(self.ack))}else{0};
   self.serial=self.serial.checked_add(1).ok_or("Social owner exhausted")?;
   // Requests have no generation field. Do not reuse frame IDs in the same
   // context: an old generation's ACK must never acknowledge a new chat.
   self.bank_pending=None;self.d1_pending=None;self.bank_ack=0;self.bank_output.clear();self.c1_output=serde_json::Value::Null;self.c2_output=serde_json::Value::Null;self.d1_output=serde_json::Value::Null;self.debug_report=None;self.debug_ack=0;self.queue=Some(Queue::native_after(context,generation,sequence));self.pending=None;self.ack=0;self.input=None;self.input_at=None;self.input_ack=0;self.chat_frames.clear();if !same_context{self.chat_receipt_high=0;}self.ack_reset_required=same_context;
  }
  self.owner=Some((pid,context,generation));self.until=until;self.terminal=false;Ok(())
 }
 pub fn pause(&mut self){if self.terminal{return;}self.until=Instant::now();self.pending=None;self.rescan=true;self.chat_frames.clear();self.ack_reset_required=true;if let Some(q)=&mut self.queue{q.pause();}}
 pub fn take_rescan(&mut self)->bool{std::mem::take(&mut self.rescan)}
 pub fn end(&mut self){self.terminal_context=self.c1_bootstrap.map(|(_,c,_,_,_)|c).or_else(||self.owner.map(|(_,c,_)|c)).or(self.terminal_context);self.clear_c1_bootstrap();if self.terminal{return;}self.terminal=true;self.chat_frames.clear();self.until=Instant::now()+std::time::Duration::from_secs(2);self.pending=self.queue.as_mut().and_then(Queue::terminal);}
 pub fn ready(&self)->bool{!self.terminal&&Instant::now()<self.until&&self.pending.is_none()&&self.queue.as_ref().is_some_and(|q|!q.queued())}
 pub fn publish(&mut self,now:f64,batches:Vec<Vec<Command>>)->Result<(),String>{
  if !self.ready(){return Err("Social stream backpressure/lease".into());}
  if batches.len()>12||batches.iter().any(|b|b.len()>32){return Err("Local queue capacity".into());}
  let mut hints=BTreeMap::new();let mut sequence=self.queue.as_ref().unwrap().sequence();let mut previous_chat=None;
  for batch in &batches{sequence=sequence.checked_add(1).ok_or("Social frame exhausted")?;for command in batch{
   if let Command::A2(crate::g3_a2_frame::Command::Chat{id,..})=command{
    if let Some((_,raw))=id.rsplit_once(':'){if let Ok(high)=raw.parse::<u32>(){if high>0&&high.to_string()==raw{
     if previous_chat.is_some_and(|old|high<=old){return Err("Chat notification order".into());}
     previous_chat=Some(high);hints.insert(sequence,high);
    }}}
   }
  }}
  self.queue.as_mut().unwrap().publish(now,batches)?;self.chat_frames=hints;self.aborted=false;Ok(())
 }
 /// Native byte receipt only, not evidence of Lua/GamePrint/screen display.
 pub fn acknowledged_chat_high(&self,pid:crate::local_owner::Id,context:[u8;16],generation:u32)->Option<u32>{
  (self.owner==Some((pid,context,generation))).then_some(self.chat_receipt_high)
 }
 pub fn input(&self)->Option<(u64,crate::g3_social_input::Input)>{let(seq,bytes)=self.input.as_ref()?;Some((*seq,crate::g3_social_input::Input::decode(bytes).ok()?))}
 /// Called only after a durable action record (or coalesced transient sample).
 pub fn acknowledge_input(&mut self,context:[u8;16],generation:u32,sequence:u64){if self.owner.is_some_and(|(_,c,g)|c==context&&g==generation)&&self.input.as_ref().is_some_and(|(s,_)|*s==sequence){self.input_ack=sequence;self.input=None;self.input_at=None;self.input_acknowledged=self.input_acknowledged.saturating_add(1);}}
 pub fn transition(&mut self,pid:crate::local_owner::Id,request:&[u8],playing:bool)->Vec<u8>{
  let mut reply=vec![0;REPLY];reply[..5].copy_from_slice(b"NSR1\x01");reply[6]=u8::from(self.debug_enabled.load(std::sync::atomic::Ordering::Acquire));
  if !playing{if let Some((owner,context,generation,until))=self.race_owner{
   if owner==pid&&until>Instant::now()&&request.len()>=64&&request[..8]==*b"NSP1\x01\0\0\0"&&request[8..24]==context{
    let input=u32::from_le_bytes(request[56..60].try_into().unwrap())as usize;let debug=u32::from_le_bytes(request[28..32].try_into().unwrap())as usize;let size=u32::from_le_bytes(request[60..64].try_into().unwrap())as usize;let seq=u64::from_le_bytes(request[40..48].try_into().unwrap());
    if self.race_output["displayOnly"]!=true&&input<=4096&&debug<=4096&&size>0&&size<=4096&&seq>0&&request.len()==64+input+debug+size{self.c1_copy(seq,&request[64+input+debug..],false);}
    let bytes=serde_json::to_vec(&serde_json::json!({"race":self.race_output})).unwrap();
    if bytes.len()<=8192{reply[5]=5;reply[8..24].copy_from_slice(&context);reply[24..28].copy_from_slice(&generation.to_le_bytes());reply[28..32].copy_from_slice(&(MAX_LEASE_MS as u32).to_le_bytes());reply[BANK_FOOTER..BANK_FOOTER+8].copy_from_slice(&self.c1_ack.to_le_bytes());reply[BANK_FOOTER+8..BANK_FOOTER+12].copy_from_slice(&(bytes.len()as u32).to_le_bytes());reply[BANK_FOOTER+16..BANK_FOOTER+16+bytes.len()].copy_from_slice(&bytes);}return reply;
   }
  }}
  if !playing{if let Some((owner,context,generation,_,until))=self.c1_bootstrap{
   if until>Instant::now()&&pid==owner&&request.len()>=64&&request.len()<=32768&&request[..8]==*b"NSP1\x01\0\0\0"&&request[8..24]==context{
    let bytes=serde_json::to_vec(&serde_json::json!({"c1":self.c1_output,"c2":self.c2_output})).unwrap();
    if !self.c1_output.is_null()&&bytes.len()<=8192{
     if self.c1_completion||!self.c2_output.is_null(){
      let input_size=u32::from_le_bytes(request[56..60].try_into().unwrap())as usize;let debug_size=u32::from_le_bytes(request[28..32].try_into().unwrap())as usize;let size=u32::from_le_bytes(request[60..64].try_into().unwrap())as usize;let seq=u64::from_le_bytes(request[40..48].try_into().unwrap());
      if input_size<=4096&&debug_size<=4096&&size>0&&size<=4096&&seq>0&&request.len()==64+input_size+debug_size+size{self.c1_copy(seq,&request[64+input_size+debug_size..],self.c1_completion);}
      reply[BANK_FOOTER..BANK_FOOTER+8].copy_from_slice(&self.c1_ack.to_le_bytes());
     }
     reply[5]=if self.c1_completion||!self.c2_output.is_null(){5}else{4};reply[8..24].copy_from_slice(&context);reply[24..28].copy_from_slice(&generation.to_le_bytes());reply[28..32].copy_from_slice(&(MAX_LEASE_MS as u32).to_le_bytes());
     reply[BANK_FOOTER+8..BANK_FOOTER+12].copy_from_slice(&(bytes.len() as u32).to_le_bytes());reply[BANK_FOOTER+16..BANK_FOOTER+16+bytes.len()].copy_from_slice(&bytes);
    }
    return reply; // Metadata only: no input ACK, queue, delivery or permit.
   }
  }}
  let Some((owner,context,generation))=self.owner else{return reply};
  if request.len()<64||request.len()>32768||request[..8]!=*b"NSP1\x01\0\0\0"||request[8..24]!=context||pid!=owner{return reply;}
  let input_sequence=u64::from_le_bytes(request[48..56].try_into().unwrap());let input_size=u32::from_le_bytes(request[56..60].try_into().unwrap()) as usize;
  if input_size>4096||request.len()<64+input_size||(input_size==0)!=(input_sequence==0){return reply;}
  // Diagnostic footer is independent of the gameplay input. Bad optional bytes
  // never reject, substitute or acknowledge a gameplay original.
  let debug_size=u32::from_le_bytes(request[28..32].try_into().unwrap()) as usize;
  let debug_sequence=u64::from_le_bytes(request[32..40].try_into().unwrap());
  let bank_size=u32::from_le_bytes(request[60..64].try_into().unwrap()) as usize;
  let bank_sequence=u64::from_le_bytes(request[40..48].try_into().unwrap());
  let optional_valid=bank_size<=4096&&debug_size<=4096&&request.len()==64+input_size+debug_size+bank_size&&(bank_size==0)==(bank_sequence==0);
  if debug_size>0||debug_sequence>0{
   if debug_sequence>self.debug_ack{
    let valid=optional_valid&&debug_size>0;
    if valid{if let Some(report)=crate::debug_diagnostics::ModReport::decode(&request[64+input_size..64+input_size+debug_size]){if self.debug_enabled.load(std::sync::atomic::Ordering::Acquire){self.debug_report=Some(report);}}else{self.debug_rejected=self.debug_rejected.saturating_add(1);}}
    else{self.debug_rejected=self.debug_rejected.saturating_add(1);}
    self.debug_ack=debug_sequence;
   }
  }
  reply[8..24].copy_from_slice(&context);reply[24..28].copy_from_slice(&generation.to_le_bytes());reply[56..64].copy_from_slice(&self.debug_ack.to_le_bytes());
  if self.terminal{
   if Instant::now()>=self.until{return reply;}let ack=u32::from_le_bytes(request[24..28].try_into().unwrap());
   if self.pending.as_ref().is_some_and(|frame|u32::from_le_bytes(frame[28..32].try_into().unwrap())==ack){self.pending=None;self.ack=ack;}
   reply[5]=2;reply[8..24].copy_from_slice(&context);reply[24..28].copy_from_slice(&generation.to_le_bytes());reply[28..32].copy_from_slice(&(MAX_LEASE_MS as u32).to_le_bytes());reply[40..48].copy_from_slice(&self.serial.to_le_bytes());
   if let Some(frame)=&self.pending{reply[32..36].copy_from_slice(&(frame.len() as u32).to_le_bytes());reply[64..64+frame.len()].copy_from_slice(frame);}return reply; // CONTROL ONLY, no playing authority.
  }
  if !playing||Instant::now()>=self.until{self.pause();return reply;}
  let completion_body=optional_valid&&bank_size>0&&self.c1_copy(bank_sequence,&request[64+input_size+debug_size..],true);
  if optional_valid&&!completion_body&&bank_sequence>self.bank_ack{
   let body=&request[64+input_size+debug_size..];
   let stats=serde_json::from_slice::<serde_json::Value>(body).ok().is_some_and(|v|v["kind"]=="d1");
   if stats{if crate::d1_work::Input::decode(body).is_some(){if self.d1_pending.is_none(){self.d1_pending=Some((bank_sequence,body.to_vec()));self.bank_wake.notify_one();}}else{self.input_rejected=self.input_rejected.saturating_add(1);}self.bank_ack=bank_sequence;}
   else if self.bank_pending.is_none()&&(crate::g3_bank::Input::decode(body).is_ok()||crate::c1_work::Input::decode(body).is_some()||crate::c2_work::Input::decode(body).is_some()){self.bank_pending=Some((bank_sequence,body.to_vec()));self.bank_wake.notify_one();}
  }
  reply[BANK_FOOTER..BANK_FOOTER+8].copy_from_slice(&if completion_body{self.c1_ack}else{self.bank_ack}.to_le_bytes());
  let mut output=self.bank_output.clone();
  let mut combined=serde_json::from_slice::<serde_json::Value>(&output).unwrap_or_else(|_|serde_json::json!({}));
  if !combined.is_object(){combined=serde_json::json!({});}
  if !self.c1_output.is_null(){combined["c1"]=self.c1_output.clone();}
  if !self.c2_output.is_null(){combined["c2"]=self.c2_output.clone();}
  if !self.d1_output.is_null(){combined["d1"]=self.d1_output.clone();}
  if !self.race_output.is_null(){combined["race"]=self.race_output.clone();}
  let all=serde_json::to_vec(&combined).unwrap();
  if all.len()<=8192{output=all;}else{
   // Every optional lane gets a turn even when snapshots cannot fit together.
   self.optional_turn=(self.optional_turn+1)%5;
   if self.optional_turn==1&&!self.c1_output.is_null(){output=serde_json::to_vec(&serde_json::json!({"c1":self.c1_output})).unwrap();}
   if self.optional_turn==2&&!self.c2_output.is_null(){output=serde_json::to_vec(&serde_json::json!({"c2":self.c2_output})).unwrap();}
   if self.optional_turn==3&&!self.d1_output.is_null(){output=serde_json::to_vec(&serde_json::json!({"d1":self.d1_output})).unwrap();}
   if self.optional_turn==4&&!self.race_output.is_null(){output=serde_json::to_vec(&serde_json::json!({"race":self.race_output})).unwrap();}
   if output.len()>8192{output.clear();}
  }
  reply[BANK_FOOTER+8..BANK_FOOTER+12].copy_from_slice(&(output.len() as u32).to_le_bytes());
  reply[BANK_FOOTER+16..BANK_FOOTER+16+output.len()].copy_from_slice(&output);
  let ack=u32::from_le_bytes(request[24..28].try_into().unwrap());
  if self.ack_reset_required{
   if ack!=0{reply[5]=3;reply[8..24].copy_from_slice(&context);reply[24..28].copy_from_slice(&generation.to_le_bytes());reply[40..48].copy_from_slice(&self.serial.to_le_bytes());return reply;}
   self.ack=0;self.ack_reset_required=false;
  }
  if input_sequence>self.input_ack{
   if self.input.as_ref().is_some_and(|(old,bytes)|*old!=input_sequence||bytes.as_slice()!=&request[64..64+input_size]){self.input_rejected=self.input_rejected.saturating_add(1);return reply;}
   if self.input.is_none(){if crate::g3_social_input::Input::decode(&request[64..64+input_size]).is_err(){self.input_rejected=self.input_rejected.saturating_add(1);return reply;}self.input=Some((input_sequence,request[64..64+input_size].to_vec()));self.input_at=Some(Instant::now());self.input_received=self.input_received.saturating_add(1);}
  }
  if let Some(frame)=&self.pending{let sequence=u32::from_le_bytes(frame[28..32].try_into().unwrap());if ack==sequence{if let Some(high)=self.chat_frames.remove(&sequence){self.chat_receipt_high=self.chat_receipt_high.max(high);}self.ack=ack;self.pending=None;}else if ack>self.ack&&ack<sequence{self.ack=ack;}else if ack!=self.ack{return reply;}}
  else if ack>self.queue.as_ref().unwrap().sequence(){reply[5]=3;reply[8..24].copy_from_slice(&context);reply[24..28].copy_from_slice(&generation.to_le_bytes());reply[40..48].copy_from_slice(&self.serial.to_le_bytes());return reply;}else if ack!=self.ack{return reply;}
  if self.pending.is_some()&&self.pending_at.elapsed()>std::time::Duration::from_millis(MAX_ACK_AGE_MS){self.chat_frames.clear();self.pending=self.queue.as_mut().unwrap().abort();self.pending_at=Instant::now();self.aborted=true;self.rescan=true;if self.pending.is_none(){self.pause();return reply;}}
  if self.pending.is_none(){let q=self.queue.as_mut().unwrap();let queued=q.queued();self.pending=q.take(context,generation,true);self.pending_at=Instant::now();if queued&&self.pending.is_none(){self.chat_frames.clear();self.pending=q.abort();self.aborted=true;self.rescan=true;if self.pending.is_none(){self.pause();return reply;}}}
  reply[5]=1;reply[8..24].copy_from_slice(&context);reply[24..28].copy_from_slice(&generation.to_le_bytes());
  reply[48..56].copy_from_slice(&self.input_ack.to_le_bytes());
  let lease=(self.until.saturating_duration_since(Instant::now()).as_millis().min(MAX_LEASE_MS as u128) as u32).max(1);reply[28..32].copy_from_slice(&lease.to_le_bytes());reply[40..48].copy_from_slice(&self.serial.to_le_bytes());
  if let Some(frame)=&self.pending{reply[32..36].copy_from_slice(&(frame.len() as u32).to_le_bytes());reply[64..64+frame.len()].copy_from_slice(frame);}
  reply
 }
}
#[cfg(test)]mod tests{use super::*;use std::time::Duration;
 #[test]fn d1_stopped_optional_consumer_does_not_hold_bank_copy_and_old_ack_is_fenced(){
  let ctx=[1;16];let owner=nt2_local_ws::owner::Id::OsProcess(42);let mut x=Exchange::default();x.arm(owner,ctx,1,Instant::now()+Duration::from_secs(3)).unwrap();
  let packet=|body:serde_json::Value,seq:u64|{let body=serde_json::to_vec(&body).unwrap();let mut req=poll(ctx,0).to_vec();req[40..48].copy_from_slice(&seq.to_le_bytes());req[60..64].copy_from_slice(&(body.len()as u32).to_le_bytes());req.extend(body);req};
  let observation=serde_json::json!({"kind":"d1","world":"a".repeat(64),"observation":{"eventId":"11111111-1111-4111-8111-111111111111","kind":"heart","progression":0}});
  let reply=x.transition(owner,&packet(observation.clone(),1),true);assert_eq!(u64::from_le_bytes(reply[BANK_FOOTER..BANK_FOOTER+8].try_into().unwrap()),1);assert!(x.d1_input().is_some()&&x.bank_input().is_none());let old_serial=x.serial;
  let reply=x.transition(owner,&packet(serde_json::json!({"kind":"submit","id":"22222222-2222-4222-8222-222222222222","direction":"deposit","amount":5}),2),true);assert!(x.bank_input().is_some()&&x.d1_input().is_some());assert_eq!(u64::from_le_bytes(reply[BANK_FOOTER..BANK_FOOTER+8].try_into().unwrap()),1);
  x.arm(owner,ctx,2,Instant::now()+Duration::from_secs(3)).unwrap();x.transition(owner,&packet(observation,1),true);x.acknowledge_d1(old_serial,1);assert!(x.d1_input().is_some());x.acknowledge_d1(x.serial,1);assert!(x.d1_input().is_none());
  let reply=x.transition(owner,&packet(serde_json::json!({"kind":"d1","token":"forbidden"}),2),true);assert_eq!(u64::from_le_bytes(reply[BANK_FOOTER..BANK_FOOTER+8].try_into().unwrap()),2);assert!(x.d1_input().is_none());
 }
 #[test]fn c2_optional_snapshot_rotation_preserves_bank_and_c1_within_native_bound(){
  let ctx=[1;16];let mut x=Exchange::default();x.arm(nt2_local_ws::owner::Id::OsProcess(7),ctx,1,Instant::now()+Duration::from_secs(3)).unwrap();x.bank_view(serde_json::to_vec(&serde_json::json!({"bank":"b".repeat(4000)})).unwrap());x.c1_view_for(ctx,serde_json::json!({"text":"c".repeat(4000)}));x.c2_view_for(ctx,serde_json::json!({"text":"d".repeat(4000)}),false);
  let mut seen=std::collections::BTreeSet::new();for _ in 0..3{let reply=x.transition(nt2_local_ws::owner::Id::OsProcess(7),&poll(ctx,0),true);let size=u32::from_le_bytes(reply[BANK_FOOTER+8..BANK_FOOTER+12].try_into().unwrap())as usize;assert!(size>0&&size<=8192);let body:serde_json::Value=serde_json::from_slice(&reply[BANK_FOOTER+16..BANK_FOOTER+16+size]).unwrap();for key in ["bank","c1","c2"]{if body.get(key).is_some(){seen.insert(key);}}}assert_eq!(seen.len(),3);
 }
 #[test]fn c2_protected_readonly_metadata_never_grants_effect_or_accepts_steve_publish(){
  let ctx=[9;16];let mut x=Exchange::default();x.c1_bootstrap_arm(nt2_local_ws::owner::Id::OsProcess(42),ctx,3,8,Instant::now()+Duration::from_secs(3)).unwrap();x.c1_bootstrap_view(ctx,8,serde_json::json!({"initialized":true}));x.c2_view_for(ctx,serde_json::json!({"initialized":true,"event":{"unsafe":true},"permit":"unsafe"}),true);
  let packet=|body:&[u8],seq:u64|{let mut request=poll(ctx,0).to_vec();request[40..48].copy_from_slice(&seq.to_le_bytes());request[60..64].copy_from_slice(&(body.len()as u32).to_le_bytes());request.extend_from_slice(body);request};
  let req=packet(br#"{"kind":"c2_completion","state":{"progression":0}}"#,1);assert_eq!(x.transition(nt2_local_ws::owner::Id::OsProcess(43),&req,false)[5],0);let reply=x.transition(nt2_local_ws::owner::Id::OsProcess(42),&req,false);assert_eq!(reply[5],5);assert!(x.c2_input(true,false).is_some());assert!(x.input().is_none()&&x.bank_pending.is_none());assert!(x.c2_output["event"].is_null()&&x.c2_output["permit"].is_null());x.acknowledge_c1(ctx,1,true);assert!(x.c2_input(true,false).is_none());
  let denied=packet(br#"{"kind":"c2_completion","publish":{"eventId":"11111111-1111-4111-8111-111111111111","rulesRevision":1,"progression":0,"senderDisposition":"applied","payload":{"steve":{"progression":0}}}}"#,2);x.transition(nt2_local_ws::owner::Id::OsProcess(42),&denied,false);assert!(x.c1_pending.is_none());
 }

 #[test]fn c1_bootstrap_diagnostics_are_scoped_read_only_and_epoch_specific(){
  let mut x=Exchange::default();let ctx=[7;16];x.c1_bootstrap_arm(nt2_local_ws::owner::Id::OsProcess(42),ctx,3,8,Instant::now()+Duration::from_secs(2)).unwrap();
  assert_eq!(x.c1_metadata_diagnostics(nt2_local_ws::owner::Id::OsProcess(42),ctx,3).unwrap()["bootstrap"],true);assert_eq!(x.c1_metadata_diagnostics(nt2_local_ws::owner::Id::OsProcess(42),ctx,4).unwrap()["native_epoch_matches"],false);
  assert!(x.c1_metadata_diagnostics(nt2_local_ws::owner::Id::OsProcess(43),ctx,3).is_none()&&x.c1_metadata_diagnostics(nt2_local_ws::owner::Id::OsProcess(42),[8;16],3).is_none());
  assert_eq!(x.c1_metadata_diagnostics(nt2_local_ws::owner::Id::OsProcess(42),ctx,3).unwrap()["metadata_ready"],false);x.c1_bootstrap_view(ctx,8,serde_json::json!({"initialized":true}));assert_eq!(x.c1_metadata_diagnostics(nt2_local_ws::owner::Id::OsProcess(42),ctx,3).unwrap()["metadata_ready"],true);
  assert!(!x.ready()&&x.owner.is_none()&&x.bank_input().is_none());
  x.c1_bootstrap=Some((nt2_local_ws::owner::Id::OsProcess(42),ctx,3,8,Instant::now()));assert!(x.c1_metadata_diagnostics(nt2_local_ws::owner::Id::OsProcess(42),ctx,3).is_none());x.c1_bootstrap=None;assert!(x.c1_metadata_diagnostics(nt2_local_ws::owner::Id::OsProcess(42),ctx,3).is_none());
 }
 #[test]fn c1_bootstrap_rotated_native_does_not_inherit_previous_copy_ack(){
  let mut x=Exchange::default();let old=nt2_local_ws::owner::Id::OsProcess(42);let fresh=nt2_local_ws::owner::Id::OsProcess(43);let old_ctx=[7;16];let ctx=[8;16];let until=Instant::now()+Duration::from_secs(2);
  let metadata=|x:&mut Exchange,context,ownership|{x.c1_bootstrap_view(context,ownership,serde_json::json!({"enabled":true,"initialized":true,"world":"a".repeat(64)}));x.c2_output=serde_json::json!({"enabled":true,"initialized":true});};
  let packet=|context,sequence:u64|{let body=br#"{"kind":"c2_completion","state":{"progression":0}}"#;let mut request=poll(context,0).to_vec();request[40..48].copy_from_slice(&sequence.to_le_bytes());request[60..64].copy_from_slice(&(body.len()as u32).to_le_bytes());request.extend(body);request};
  x.c1_bootstrap_arm(old,old_ctx,3,8,until).unwrap();metadata(&mut x,old_ctx,8);x.transition(old,&packet(old_ctx,1),false);assert!(x.c2_input(true,false).is_some());x.acknowledge_c1(old_ctx,1,true);assert_eq!(u64::from_le_bytes(x.transition(old,&poll(old_ctx,0),false)[BANK_FOOTER..BANK_FOOTER+8].try_into().unwrap()),1);
  x.c1_bootstrap_arm(old,old_ctx,3,8,until).unwrap();metadata(&mut x,old_ctx,8);assert_eq!(u64::from_le_bytes(x.transition(old,&poll(old_ctx,0),false)[BANK_FOOTER..BANK_FOOTER+8].try_into().unwrap()),1,"same owner retains its receipt");
  x.transition(old,&packet(old_ctx,2),false);assert!(x.c2_input(true,false).is_some());x.c1_bootstrap_arm(fresh,ctx,4,9,until).unwrap();metadata(&mut x,ctx,9);let reply=x.transition(fresh,&poll(ctx,0),false);assert_eq!(reply[5],5);assert_eq!(u64::from_le_bytes(reply[BANK_FOOTER..BANK_FOOTER+8].try_into().unwrap()),0,"new native sequence starts at zero");assert!(x.c2_input(true,false).is_none(),"old owner pending copy cannot attach to new native");
  x.acknowledge_c1(old_ctx,2,true);assert_eq!(u64::from_le_bytes(x.transition(fresh,&poll(ctx,0),false)[BANK_FOOTER..BANK_FOOTER+8].try_into().unwrap()),0,"late old receipt is fenced");x.transition(fresh,&packet(ctx,1),false);assert!(x.c2_input(true,false).is_some());x.acknowledge_c1(ctx,1,true);assert_eq!(u64::from_le_bytes(x.transition(fresh,&poll(ctx,0),false)[BANK_FOOTER..BANK_FOOTER+8].try_into().unwrap()),1);assert!(!x.ready()&&x.owner.is_none());
 }
 #[test]fn c1_bootstrap_is_metadata_only_and_does_not_arm_or_ack_gameplay(){
  let mut x=Exchange::default();let ctx=[7;16];x.c1_bootstrap_arm(nt2_local_ws::owner::Id::OsProcess(42),ctx,3,8,Instant::now()+Duration::from_secs(2)).unwrap();
  x.c1_bootstrap_view(ctx,8,serde_json::json!({"enabled":true,"world":"a".repeat(64),"initialized":true,"snapshot":{"revision":1},"event":{"payload":{"heart":{}}},"permit":"must-not-cross"}));
  let mut request=poll(ctx,0).to_vec();let body=br#"{"kind":"c1","publish":{"eventId":"11111111-1111-4111-8111-111111111111","rulesRevision":1,"payload":{"heart":{}},"senderDisposition":"applied"}}"#;
  request[40..48].copy_from_slice(&1u64.to_le_bytes());request[60..64].copy_from_slice(&(body.len()as u32).to_le_bytes());request.extend(body);
  let reply=x.transition(nt2_local_ws::owner::Id::OsProcess(42),&request,false);assert_eq!(reply[5],4);assert_eq!(u32::from_le_bytes(reply[32..36].try_into().unwrap()),0);assert_eq!(&reply[BANK_FOOTER..BANK_FOOTER+8],&[0;8]);
  let len=u32::from_le_bytes(reply[BANK_FOOTER+8..BANK_FOOTER+12].try_into().unwrap())as usize;
  let view:serde_json::Value=serde_json::from_slice(&reply[BANK_FOOTER+16..BANK_FOOTER+16+len]).unwrap();assert_eq!(view["c1"]["initialized"],true);assert!(view["c1"]["event"].is_null());assert!(view["c1"]["permit"].is_null());assert!(x.bank_input().is_none()&&x.c1_input(false).is_none()&&x.input().is_none());assert!(!x.ready());assert!(x.owner.is_none()&&x.queue.is_none());
  assert_eq!(x.transition(nt2_local_ws::owner::Id::OsProcess(43),&request,false)[5],0);assert_eq!(x.transition(nt2_local_ws::owner::Id::OsProcess(42),&poll([8;16],0),false)[5],0);
 }
 #[test]fn c1_bootstrap_wrong_generation_expiry_terminal_and_late_reply_are_fenced(){
  let mut x=Exchange::default();let ctx=[7;16];let view=serde_json::json!({"enabled":true,"initialized":true,"world":"a".repeat(64)});
  x.c1_bootstrap_arm(nt2_local_ws::owner::Id::OsProcess(42),ctx,3,8,Instant::now()+Duration::from_secs(2)).unwrap();x.c1_bootstrap_view(ctx,7,view.clone());assert_eq!(x.transition(nt2_local_ws::owner::Id::OsProcess(42),&poll(ctx,0),false)[5],0);
  x.c1_bootstrap_view([8;16],8,view.clone());assert_eq!(x.transition(nt2_local_ws::owner::Id::OsProcess(42),&poll(ctx,0),false)[5],0);
  x.c1_bootstrap_view(ctx,8,view.clone());assert_eq!(x.transition(nt2_local_ws::owner::Id::OsProcess(42),&poll(ctx,0),false)[5],4);
  x.c1_bootstrap_arm(nt2_local_ws::owner::Id::OsProcess(42),ctx,4,9,Instant::now()+Duration::from_secs(2)).unwrap();x.c1_bootstrap_view(ctx,8,view.clone());assert_eq!(x.transition(nt2_local_ws::owner::Id::OsProcess(42),&poll(ctx,0),false)[5],0);
  x.c1_bootstrap_view(ctx,9,view.clone());assert_eq!(x.transition(nt2_local_ws::owner::Id::OsProcess(42),&poll(ctx,0),false)[5],4);
  x.c1_bootstrap=Some((nt2_local_ws::owner::Id::OsProcess(42),ctx,4,9,Instant::now()));assert_eq!(x.transition(nt2_local_ws::owner::Id::OsProcess(42),&poll(ctx,0),false)[5],0);
  x.terminal=true;x.terminal_context=Some(ctx);assert!(x.c1_bootstrap_arm(nt2_local_ws::owner::Id::OsProcess(42),ctx,4,9,Instant::now()+Duration::from_secs(2)).is_err());x.c1_bootstrap_view(ctx,9,view);assert_eq!(x.transition(nt2_local_ws::owner::Id::OsProcess(42),&poll(ctx,0),false)[5],0);
 }
 #[test]fn c1_protected_completion_drains_only_closed_facts_after_durability_without_gameplay(){
  let mut x=Exchange::default();let ctx=[7;16];x.c1_completion_arm(nt2_local_ws::owner::Id::OsProcess(42),ctx,3,8,Instant::now()+Duration::from_secs(2)).unwrap();
  x.c1_bootstrap_view(ctx,8,serde_json::json!({"initialized":true,"event":{"payload":{"heart":{}}},"permit":"forbidden","localAccepted":[]}));
  let packet=|payload:serde_json::Value|{let bytes=serde_json::to_vec(&payload).unwrap();let mut request=poll(ctx,0).to_vec();request.resize(64+bytes.len(),0);request[40..48].copy_from_slice(&1u64.to_le_bytes());request[60..64].copy_from_slice(&(bytes.len()as u32).to_le_bytes());request[64..].copy_from_slice(&bytes);request};
  let death=packet(serde_json::json!({"kind":"c1_completion","publish":{"eventId":"11111111-1111-4111-8111-111111111111","rulesRevision":1,"payload":{"death":{"state":"respawning","cause":"normal"}},"senderDisposition":"applied"}}));
  assert_eq!(x.transition(nt2_local_ws::owner::Id::OsProcess(43),&death,false)[5],0);assert!(x.c1_input(true).is_none());
  let heart=packet(serde_json::json!({"kind":"c1_completion","publish":{"eventId":"11111111-1111-4111-8111-111111111111","rulesRevision":1,"payload":{"heart":{}},"senderDisposition":"applied"}}));
  let reply=x.transition(nt2_local_ws::owner::Id::OsProcess(42),&heart,false);assert_eq!(reply[5],5);assert!(x.c1_input(true).is_none());assert_eq!(&reply[BANK_FOOTER..BANK_FOOTER+8],&[0;8]);
  let reply=x.transition(nt2_local_ws::owner::Id::OsProcess(42),&death,false);assert_eq!(reply[5],5);assert!(x.c1_input(true).is_some());assert_eq!(&reply[BANK_FOOTER..BANK_FOOTER+8],&[0;8]);assert!(!x.ready());assert!(x.owner.is_none()&&x.queue.is_none());assert_eq!(&reply[32..64],&[0;32]);
  x.acknowledge_c1([8;16],1,true);assert!(x.c1_input(true).is_some());x.acknowledge_c1(ctx,1,true);assert!(x.c1_input(true).is_none());assert_eq!(&x.transition(nt2_local_ws::owner::Id::OsProcess(42),&death,false)[BANK_FOOTER..BANK_FOOTER+8],&1u64.to_le_bytes());
  x.c1_bootstrap=Some((nt2_local_ws::owner::Id::OsProcess(42),ctx,3,8,Instant::now()));assert_eq!(x.transition(nt2_local_ws::owner::Id::OsProcess(42),&death,false)[5],0);
 }
 #[test]fn c1_new_world_metadata_survives_setup_terminal_without_reopening_old_world(){
  let mut x=Exchange::default();x.end();let ctx=[8;16];x.c1_bootstrap_arm(nt2_local_ws::owner::Id::OsProcess(42),ctx,3,8,Instant::now()+Duration::from_secs(2)).unwrap();
  x.c1_bootstrap_view(ctx,8,serde_json::json!({"initialized":true}));assert_eq!(x.transition(nt2_local_ws::owner::Id::OsProcess(42),&poll(ctx,0),false)[5],4);assert!(!x.ready());assert!(x.owner.is_none());assert!(x.terminal);
  x.end();assert!(x.c1_bootstrap_arm(nt2_local_ws::owner::Id::OsProcess(42),ctx,3,8,Instant::now()+Duration::from_secs(2)).is_err());
  x.c1_bootstrap_arm(nt2_local_ws::owner::Id::OsProcess(43),[9;16],4,9,Instant::now()+Duration::from_secs(2)).unwrap();assert!(x.terminal);assert!(!x.ready());
 }
 #[test]fn bank_publication_is_fenced_by_context_terminal_and_lease(){let mut x=Exchange::default();x.arm(nt2_local_ws::owner::Id::OsProcess(1),[1;16],1,Instant::now()+Duration::from_secs(2)).unwrap();x.bank_view_for([1;16],b"confirmed".to_vec());assert_eq!(x.bank_output,b"confirmed");x.bank_view_for([2;16],b"wrong-context".to_vec());assert_eq!(x.bank_output,b"confirmed");x.terminal=true;x.bank_view_for([1;16],b"terminal".to_vec());assert_eq!(x.bank_output,b"confirmed");x.terminal=false;x.until=Instant::now();x.bank_view_for([1;16],b"expired".to_vec());assert_eq!(x.bank_output,b"confirmed");}

 #[test]fn lobby_delete_clears_bank_queue_and_effect_without_acknowledging_social_input(){
  let mut x=Exchange::default();x.arm(nt2_local_ws::owner::Id::OsProcess(7),[1;16],1,Instant::now()+Duration::from_secs(3)).unwrap();
  let social=br#"{"kind":"dismiss","sequence":"1"}"#.to_vec();x.input=Some((4,social.clone()));x.bank_pending=Some((9,serde_json::to_vec(&serde_json::json!({"kind":"submit","id":uuid::Uuid::new_v4().to_string(),"direction":"withdraw","amount":5})).unwrap()));x.bank_view(br#"{"effect":{"stage":"permit"},"queued":5,"balance":100,"fresh":true}"#.to_vec());
  x.clear_deleted_bank();assert!(x.bank_input().is_none());assert_eq!(x.bank_ack,9);assert_eq!(x.input,Some((4,social)));assert_eq!(x.input_ack,0);let view:serde_json::Value=serde_json::from_slice(&x.bank_output).unwrap();assert!(view["effect"].is_null());assert_eq!(view["queued"],0);assert_eq!(view["fresh"],false);assert!(view["balance"].is_null());x.clear_deleted_bank();assert_eq!(x.bank_ack,9);
 }
 fn receipt(x:&Exchange)->u32{let(pid,context,generation)=x.owner.unwrap();x.acknowledged_chat_high(pid,context,generation).unwrap()}
 fn chat(high:u32)->Vec<Command>{vec![Command::A2(crate::g3_a2_frame::Command::Chat{id:format!("room:{high}"),name:"A".into(),text:"hello".into()})]}
 fn frame(reply:&[u8])->u32{u32::from_le_bytes(reply[92..96].try_into().unwrap())}
 #[test]fn a2_chat_receipt_requires_exact_ordered_native_ack(){
  let ctx=[1;16];let mut x=Exchange::default();x.arm(nt2_local_ws::owner::Id::OsProcess(7),ctx,1,Instant::now()+Duration::from_secs(3)).unwrap();x.publish(0.,vec![chat(1),chat(2)]).unwrap();
  assert_eq!(x.transition(nt2_local_ws::owner::Id::OsProcess(7),&poll(ctx,2),true)[5],0,"Unsent larger ACK accepted");assert_eq!(receipt(&x),0);
  let first=x.transition(nt2_local_ws::owner::Id::OsProcess(7),&poll(ctx,0),true);let one=frame(&first);assert_eq!(receipt(&x),0);assert_eq!(first,x.transition(nt2_local_ws::owner::Id::OsProcess(7),&poll(ctx,0),true),"Lost ACK must repeat original frame");
  assert_eq!(x.transition(nt2_local_ws::owner::Id::OsProcess(8),&poll(ctx,one),true)[5],0);assert_eq!(x.transition(nt2_local_ws::owner::Id::OsProcess(7),&poll([2;16],one),true)[5],0);assert_eq!(x.transition(nt2_local_ws::owner::Id::OsProcess(7),&poll(ctx,one+1),true)[5],0);assert_eq!(receipt(&x),0);
  let second=x.transition(nt2_local_ws::owner::Id::OsProcess(7),&poll(ctx,one),true);assert_eq!(receipt(&x),1);assert_eq!(frame(&second),one+1);x.transition(nt2_local_ws::owner::Id::OsProcess(7),&poll(ctx,one),true);assert_eq!(receipt(&x),1);x.transition(nt2_local_ws::owner::Id::OsProcess(7),&poll(ctx,one+1),true);assert_eq!(receipt(&x),2);assert!(x.chat_frames.is_empty());
 }
 #[test]fn a2_chat_abort_reset_generation_and_stale_ack_do_not_commit(){
  let ctx=[1;16];let mut x=Exchange::default();x.arm(nt2_local_ws::owner::Id::OsProcess(7),ctx,1,Instant::now()+Duration::from_secs(3)).unwrap();x.publish(0.,vec![chat(1),chat(2)]).unwrap();let first=x.transition(nt2_local_ws::owner::Id::OsProcess(7),&poll(ctx,0),true);let old=frame(&first);
  x.pending_at=Instant::now()-Duration::from_millis(101);let aborted=x.transition(nt2_local_ws::owner::Id::OsProcess(7),&poll(ctx,0),true);assert_eq!(aborted[104],5);assert_eq!(receipt(&x),0);assert!(x.chat_frames.is_empty());x.transition(nt2_local_ws::owner::Id::OsProcess(7),&poll(ctx,old),true);assert_eq!(receipt(&x),0);
  x.arm(nt2_local_ws::owner::Id::OsProcess(7),ctx,2,Instant::now()+Duration::from_secs(3)).unwrap();x.publish(0.,vec![chat(1)]).unwrap();assert_eq!(x.transition(nt2_local_ws::owner::Id::OsProcess(7),&poll(ctx,old),true)[5],3);assert_eq!(receipt(&x),0);let fresh=x.transition(nt2_local_ws::owner::Id::OsProcess(7),&poll(ctx,0),true);let current=frame(&fresh);assert!(current>old);x.transition(nt2_local_ws::owner::Id::OsProcess(7),&poll(ctx,old),true);assert_eq!(receipt(&x),0);x.transition(nt2_local_ws::owner::Id::OsProcess(7),&poll(ctx,current),true);assert_eq!(receipt(&x),1);
  x.pause();assert_eq!(receipt(&x),1,"Already received prefix survives temporary pause");x.arm(nt2_local_ws::owner::Id::OsProcess(7),ctx,3,Instant::now()+Duration::from_secs(3)).unwrap();assert_eq!(receipt(&x),1,"Same-context recovery preserves accepted receipt, not pending hints");x.arm(nt2_local_ws::owner::Id::OsProcess(7),[2;16],4,Instant::now()+Duration::from_secs(3)).unwrap();assert_eq!(receipt(&x),0,"Foreign context cannot inherit chat receipt");
 }
 #[test]fn a2_chat_out_of_order_high_water_rejected_atomically(){let mut x=Exchange::default();x.arm(nt2_local_ws::owner::Id::OsProcess(7),[1;16],1,Instant::now()+Duration::from_secs(3)).unwrap();assert!(x.publish(0.,vec![chat(2),chat(1)]).is_err());assert!(x.ready());assert!(x.chat_frames.is_empty());assert_eq!(receipt(&x),0);}
 #[test]fn a2_input_diagnostics_distinguish_occupied_rejected_and_drained_without_payload(){
  let ctx=[1;16];let mut x=Exchange::default();x.arm(nt2_local_ws::owner::Id::OsProcess(7),ctx,1,Instant::now()+Duration::from_secs(3)).unwrap();let body=br#"{"kind":"dismiss","sequence":"128"}"#;let mut request=poll(ctx,0).to_vec();request[48..56].copy_from_slice(&1u64.to_le_bytes());request[56..60].copy_from_slice(&(body.len()as u32).to_le_bytes());request.extend(body);
  assert_eq!(x.progress()["input_pending"],false);x.transition(nt2_local_ws::owner::Id::OsProcess(7),&request,true);let view=x.progress();assert_eq!(view["input_pending"],true);assert_eq!(view["inputs_received"],1);assert!(view["input_age_ms"].is_number());assert!(!view.to_string().contains("dismiss"));x.transition(nt2_local_ws::owner::Id::OsProcess(7),&request,true);assert_eq!(x.progress()["inputs_received"],1,"Replay does not enqueue another operation");
  let mut different=request.clone();*different.last_mut().unwrap()=b' ';x.transition(nt2_local_ws::owner::Id::OsProcess(7),&different,true);assert_eq!(x.progress()["inputs_rejected"],1);assert!(x.input().is_some());x.acknowledge_input(ctx,1,1);let view=x.progress();assert_eq!(view["input_pending"],false);assert!(view["input_age_ms"].is_null());assert_eq!(view["inputs_acknowledged"],1);x.transition(nt2_local_ws::owner::Id::OsProcess(7),&request,true);assert_eq!(x.progress()["inputs_received"],1);
 }
 #[test]fn a2_input_lost_ack_replay_owner_and_durable_ack_fences(){
  let ctx=[1;16];let mut x=Exchange::default();x.arm(nt2_local_ws::owner::Id::OsProcess(7),ctx,1,Instant::now()+Duration::from_secs(3)).unwrap();let body=br#"{"kind":"dismiss","sequence":"1"}"#;let mut request=poll(ctx,0).to_vec();request[48..56].copy_from_slice(&1u64.to_le_bytes());request[56..60].copy_from_slice(&(body.len()as u32).to_le_bytes());request.extend(body);
  assert_eq!(x.transition(nt2_local_ws::owner::Id::OsProcess(8),&request,true)[5],0);assert!(x.input().is_none());assert_eq!(x.transition(nt2_local_ws::owner::Id::OsProcess(7),&request,true)[48..56],[0;8]);assert_eq!(x.input().unwrap().0,1);assert_eq!(x.transition(nt2_local_ws::owner::Id::OsProcess(7),&request,true)[48..56],[0;8]);
  let mut substituted=request.clone();*substituted.last_mut().unwrap()=b' ';assert_eq!(x.transition(nt2_local_ws::owner::Id::OsProcess(7),&substituted,true)[5],0);x.acknowledge_input(ctx,2,1);assert!(x.input().is_some());x.acknowledge_input(ctx,1,1);assert!(x.input().is_none());assert_eq!(x.transition(nt2_local_ws::owner::Id::OsProcess(7),&request,true)[48..56],1u64.to_le_bytes());assert!(x.input().is_none(),"Lost ACK repeats no operation");
  x.arm(nt2_local_ws::owner::Id::OsProcess(7),[2;16],2,Instant::now()+Duration::from_secs(3)).unwrap();assert_eq!(x.transition(nt2_local_ws::owner::Id::OsProcess(7),&request,true)[5],0);assert!(x.input().is_none());
 }
 fn poll(ctx:[u8;16],ack:u32)->[u8;64]{let mut p=[0;64];p[..8].copy_from_slice(b"NSP1\x01\0\0\0");p[8..24].copy_from_slice(&ctx);p[24..28].copy_from_slice(&ack.to_le_bytes());p}
 #[test]fn optional_debug_footer_never_owns_gameplay_input(){
  let ctx=[1;16];let body=br#"{"kind":"dismiss","sequence":"128"}"#;
  let report=crate::debug_diagnostics::ModReport{revision:1,enabled:true,logger:true,..Default::default()};
  for case in 0..7{
   let mut x=Exchange::default();x.arm(nt2_local_ws::owner::Id::OsProcess(7),ctx,1,Instant::now()+Duration::from_secs(3)).unwrap();
   let enabled=std::sync::Arc::new(std::sync::atomic::AtomicBool::new(case!=1));x.debug_control(enabled);
   let mut request=poll(ctx,0).to_vec();request[48..56].copy_from_slice(&1u64.to_le_bytes());request[56..60].copy_from_slice(&(body.len()as u32).to_le_bytes());request.extend(body);
   let bytes=match case{2=>br#"{"token":"secret"}"#.to_vec(),3=>vec![b' ';4097],_=>serde_json::to_vec(&report).unwrap()};
   if case!=6{request[28..32].copy_from_slice(&(bytes.len()as u32).to_le_bytes());request[32..40].copy_from_slice(&1u64.to_le_bytes());request.extend(bytes);}
   if case==4{request[40]=1;}if case==5{request.pop();}
   let reply=x.transition(nt2_local_ws::owner::Id::OsProcess(7),&request,true);assert_eq!(reply[5],1,"case {case}");assert_eq!(x.input().unwrap().0,1);assert_eq!(x.progress()["inputs_received"],1);assert_eq!(x.progress()["inputs_rejected"],0);
   assert_eq!(x.take_debug_report().is_some(),case==0);assert_eq!(reply[6],u8::from(case!=1));
   x.acknowledge_input(ctx,1,1);let reply=x.transition(nt2_local_ws::owner::Id::OsProcess(7),&request,true);assert_eq!(reply[48..56],1u64.to_le_bytes());assert!(x.input().is_none());
  }
 }
 #[test]fn b1_bank_mailbox_is_independent_bounded_and_owner_fenced(){
  let ctx=[1;16];let mut x=Exchange::default();x.arm(nt2_local_ws::owner::Id::OsProcess(7),ctx,1,Instant::now()+std::time::Duration::from_secs(3)).unwrap();
  let social=br#"{"kind":"dismiss","sequence":"128"}"#;let bank=serde_json::to_vec(&serde_json::json!({"kind":"submit","id":uuid::Uuid::new_v4().to_string(),"direction":"withdraw","amount":5})).unwrap();
  let mut request=poll(ctx,0).to_vec();request[48..56].copy_from_slice(&1u64.to_le_bytes());request[56..60].copy_from_slice(&(social.len() as u32).to_le_bytes());request[40..48].copy_from_slice(&1u64.to_le_bytes());request[60..64].copy_from_slice(&(bank.len() as u32).to_le_bytes());request.extend(social);request.extend(&bank);
  assert_eq!(x.transition(nt2_local_ws::owner::Id::OsProcess(8),&request,true)[5],0);assert!(x.bank_input().is_none());assert_eq!(x.transition(nt2_local_ws::owner::Id::OsProcess(7),&request,true)[5],1);assert!(x.bank_input().is_some());assert!(x.input().is_some());
  // Slow bank action retains its own slot while HUD/chat can acknowledge.
  x.acknowledge_input(ctx,1,1);assert!(x.input().is_none());assert!(x.bank_input().is_some());x.acknowledge_bank([2;16],1);assert!(x.bank_input().is_some());x.acknowledge_bank(ctx,1);assert!(x.bank_input().is_none());
  x.bank_view(b"{\"fresh\":true}".to_vec());let reply=x.transition(nt2_local_ws::owner::Id::OsProcess(7),&request,true);assert_eq!(&reply[BANK_FOOTER..BANK_FOOTER+8],&1u64.to_le_bytes());assert!(x.bank_input().is_none(),"lost native ACK cannot resubmit an original");
  let mut corrupt=request;corrupt[40..48].copy_from_slice(&2u64.to_le_bytes());corrupt[60..64].copy_from_slice(&4097u32.to_le_bytes());assert_eq!(x.transition(nt2_local_ws::owner::Id::OsProcess(7),&corrupt,true)[5],1,"bad optional bank does not starve social");assert!(x.bank_input().is_none());
  x.arm(nt2_local_ws::owner::Id::OsProcess(7),[2;16],2,Instant::now()+std::time::Duration::from_secs(3)).unwrap();assert!(x.bank_output.is_empty());assert_eq!(x.transition(nt2_local_ws::owner::Id::OsProcess(7),&corrupt,true)[5],0);
 }
 #[test]fn debug_footer_cannot_bypass_owner_or_ack_a_gameplay_original(){
  let ctx=[1;16];let mut x=Exchange::default();x.arm(nt2_local_ws::owner::Id::OsProcess(7),ctx,1,Instant::now()+Duration::from_secs(3)).unwrap();let report=crate::debug_diagnostics::ModReport{revision:1,enabled:true,logger:true,..Default::default()};let bytes=serde_json::to_vec(&report).unwrap();let mut request=poll(ctx,0).to_vec();request[28..32].copy_from_slice(&(bytes.len()as u32).to_le_bytes());request[32..40].copy_from_slice(&999u64.to_le_bytes());request.extend(bytes);
  assert_eq!(x.transition(nt2_local_ws::owner::Id::OsProcess(8),&request,true)[5],0);assert!(x.take_debug_report().is_none());let reply=x.transition(nt2_local_ws::owner::Id::OsProcess(7),&request,false);assert_eq!(reply[48..56],[0;8]);assert!(x.input().is_none());assert!(x.take_debug_report().is_some());
 }
 #[test]fn acknowledged_stream_pid_hold_replay_generation_and_abort(){
  let mut x=Exchange::default();x.arm(nt2_local_ws::owner::Id::OsProcess(7),[1;16],1,Instant::now()+Duration::from_secs(2)).unwrap();x.publish(0.,vec![vec![Command::Pause],vec![Command::End]]).unwrap();
  assert_eq!(x.transition(nt2_local_ws::owner::Id::OsProcess(8),&poll([1;16],0),true)[5],0);let first=x.transition(nt2_local_ws::owner::Id::OsProcess(7),&poll([1;16],0),true);assert_eq!(first,x.transition(nt2_local_ws::owner::Id::OsProcess(7),&poll([1;16],0),true));
  assert_eq!(x.transition(nt2_local_ws::owner::Id::OsProcess(7),&poll([1;16],99),true)[5],0);let next=x.transition(nt2_local_ws::owner::Id::OsProcess(7),&poll([1;16],1),true);assert_eq!(u32::from_le_bytes(next[92..96].try_into().unwrap()),2);
  x.transition(nt2_local_ws::owner::Id::OsProcess(7),&poll([1;16],2),false);x.arm(nt2_local_ws::owner::Id::OsProcess(7),[1;16],1,Instant::now()+Duration::from_secs(2)).unwrap();assert!(!x.queue.as_ref().unwrap().queued());
  x.publish(0.,vec![vec![Command::Pause]]).unwrap();assert_eq!(x.transition(nt2_local_ws::owner::Id::OsProcess(7),&poll([1;16],1),true)[5],3);std::thread::sleep(Duration::from_millis(1010));let abort=x.transition(nt2_local_ws::owner::Id::OsProcess(7),&poll([1;16],0),true);assert_eq!(abort[5],1);assert!(x.aborted);assert_eq!(abort[104],5);
  x.arm(nt2_local_ws::owner::Id::OsProcess(9),[2;16],2,Instant::now()+Duration::from_secs(2)).unwrap();assert_eq!(x.transition(nt2_local_ws::owner::Id::OsProcess(7),&poll([1;16],0),true)[5],0);
 }
 #[test]fn reset_negotiates_old_ack_and_terminal_is_control_only_fresh_context(){
  let mut x=Exchange::default();x.arm(nt2_local_ws::owner::Id::OsProcess(7),[1;16],1,Instant::now()+Duration::from_secs(3)).unwrap();x.publish(0.,vec![vec![Command::Pause]]).unwrap();x.transition(nt2_local_ws::owner::Id::OsProcess(7),&poll([1;16],0),true);x.transition(nt2_local_ws::owner::Id::OsProcess(7),&poll([1;16],1),true);
  x.arm(nt2_local_ws::owner::Id::OsProcess(7),[1;16],2,Instant::now()+Duration::from_secs(3)).unwrap();let reset=x.transition(nt2_local_ws::owner::Id::OsProcess(7),&poll([1;16],1),true);assert_eq!(reset[5],3);assert_eq!(&reset[28..36],&[0;8]);assert_eq!(x.transition(nt2_local_ws::owner::Id::OsProcess(7),&poll([1;16],0),true)[5],1);
  x.end();x.pause();let terminal=x.transition(nt2_local_ws::owner::Id::OsProcess(7),&poll([1;16],0),false);assert_eq!(terminal[5],2);assert_eq!(terminal[104],6);assert!(!x.ready());assert_eq!(x.transition(nt2_local_ws::owner::Id::OsProcess(8),&poll([1;16],0),false)[5],0);
  assert!(x.arm(nt2_local_ws::owner::Id::OsProcess(7),[1;16],3,Instant::now()+Duration::from_secs(1)).is_err());let seq=u32::from_le_bytes(terminal[92..96].try_into().unwrap());assert_eq!(&x.transition(nt2_local_ws::owner::Id::OsProcess(7),&poll([1;16],seq),false)[32..36],&[0;4]);
  x.until=Instant::now();assert_eq!(x.transition(nt2_local_ws::owner::Id::OsProcess(7),&poll([1;16],seq),true)[5],0);x.arm(nt2_local_ws::owner::Id::OsProcess(7),[2;16],3,Instant::now()+Duration::from_secs(1)).unwrap();assert_eq!(x.transition(nt2_local_ws::owner::Id::OsProcess(7),&poll([1;16],0),true)[5],0);
 }
 #[test]fn twelve_frame_capture_drains_under_slow_bounded_polling(){
  let mut x=Exchange::default();x.arm(nt2_local_ws::owner::Id::OsProcess(7),[1;16],1,Instant::now()+Duration::from_secs(2)).unwrap();x.publish(0.,(0..12).map(|_|vec![Command::Pause]).collect()).unwrap();let mut ack=0;
  for _ in 0..12{let reply=x.transition(nt2_local_ws::owner::Id::OsProcess(7),&poll([1;16],ack),true);assert_eq!(reply[5],1);assert!(u32::from_le_bytes(reply[32..36].try_into().unwrap())>0);ack=u32::from_le_bytes(reply[92..96].try_into().unwrap());std::thread::sleep(Duration::from_millis(40));}
  x.transition(nt2_local_ws::owner::Id::OsProcess(7),&poll([1;16],ack),true);assert_eq!(ack,12);assert!(x.ready());assert!(!x.aborted);
 }
}

#[cfg(test)]mod race_copy_tests{
 use super::*;
 fn body(context:[u8;16])->Vec<u8>{format!(r#"{{"kind":"race_completion","world":"{}","context":"{}","commit":{{"eventId":"11111111-1111-4111-8111-111111111111","sequence":1,"observations":[],"terminal":"dead"}}}}"#,"a".repeat(64),context.iter().map(|b|format!("{b:02x}")).collect::<String>()).into_bytes()}
 #[test]fn race_lobby_display_metadata_cannot_copy_a_gameplay_original(){
  let mut x=Exchange::default();let pid=crate::local_owner::Id::os_process(7);let context=[1;16];x.race_arm(pid,context,1,Instant::now()+std::time::Duration::from_secs(2)).unwrap();x.race_view_for(context,serde_json::json!({"displayOnly":true,"lobbyOnly":true}));let body=body(context);let mut request=vec![0;64];request[..8].copy_from_slice(b"NSP1\x01\0\0\0");request[8..24].copy_from_slice(&context);request[40..48].copy_from_slice(&1u64.to_le_bytes());request[60..64].copy_from_slice(&(body.len()as u32).to_le_bytes());request.extend(body);let reply=x.transition(pid,&request,false);assert_eq!(reply[5],5);assert!(x.race_input().is_none());assert_eq!(x.c1_ack,0);assert!(!x.ready()&&x.owner.is_none());
 }
 #[test]fn race_terminal_copy_survives_end_and_is_not_acknowledged_before_durable_work(){
  let mut x=Exchange::default();let until=Instant::now()+std::time::Duration::from_secs(5);x.race_arm(crate::local_owner::Id::os_process(7),[1;16],1,until).unwrap();
  assert!(x.c1_copy(1,&body([2;16]),false));assert!(x.race_input().is_none());assert!(x.c1_copy(1,&body([1;16]),false));let(serial,sequence,_)=x.race_input().unwrap();assert_eq!(x.c1_ack,0);x.end();assert!(x.race_input().is_some());x.acknowledge_race(serial,sequence);assert!(x.race_input().is_none());assert_eq!(x.c1_ack,1);
 }
 #[test]fn obsolete_race_copy_ack_cannot_acknowledge_replacement_context(){
  let mut x=Exchange::default();let until=Instant::now()+std::time::Duration::from_secs(5);x.race_arm(crate::local_owner::Id::os_process(7),[1;16],1,until).unwrap();x.c1_copy(1,&body([1;16]),false);let(old,_,_)=x.race_input().unwrap();
  x.race_arm(crate::local_owner::Id::os_process(8),[2;16],2,until).unwrap();x.c1_copy(1,&body([2;16]),false);x.acknowledge_race(old,1);assert!(x.race_input().is_some());assert_eq!(x.c1_ack,0);
  let(current,sequence,_)=x.race_input().unwrap();x.acknowledge_race(current,sequence);assert_eq!(x.c1_ack,1);
 }
 #[test]fn race_to_core_bootstrap_does_not_export_an_ack_from_the_old_native(){
  let mut x=Exchange::default();let until=Instant::now()+std::time::Duration::from_secs(5);let old=crate::local_owner::Id::os_process(7);let new=crate::local_owner::Id::os_process(8);
  x.arm(old,[1;16],1,until).unwrap();  x.race_arm(old,[1;16],1,until).unwrap();x.c1_copy(1,&body([1;16]),false);let(serial,sequence,_)=x.race_input().unwrap();x.acknowledge_race(serial,sequence);
  x.c1_copy(2,&body([1;16]),false);let pending=x.race_input().unwrap();
  assert!(x.race_arm(old,[1;16],1,Instant::now()).is_err(),"The old collection lease may expire before the mode changes");
  x.end();assert!(x.pending.is_some(),"Retain the old terminal frame until its own scope is retired");x.race_disarm();assert!(x.race_input().is_some(),"Retain the old original until durable work owns it");
  x.c1_bootstrap_arm(new,[2;16],2,1,until).unwrap();x.c1_bootstrap_view([2;16],1,serde_json::json!({"initialized":true}));
  x.c2_view_for([2;16],serde_json::json!({"initialized":true}),true);
  x.acknowledge_race(pending.0,pending.1);let mut request=vec![0;64];request[..5].copy_from_slice(b"NSP1\x01");request[8..24].copy_from_slice(&[2;16]);
  let reply=x.transition(new,&request,false);assert_eq!(reply[5],5);assert_eq!(u64::from_le_bytes(reply[BANK_FOOTER..BANK_FOOTER+8].try_into().unwrap()),0,"The new broker has not published a copy sequence yet");
 }
}
