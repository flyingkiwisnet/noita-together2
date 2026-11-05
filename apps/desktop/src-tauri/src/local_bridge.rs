//! Current app/game transport. One listener and one authenticated native owner.
use std::{path::PathBuf,sync::{Arc,Mutex,Weak,atomic::{AtomicBool,Ordering}},thread,time::Duration};
use nt2_local_ws::{server::{Server,Handler,Peer},private::Bootstrap};
use crate::pipe::Dispatcher;
// Linearize conversation retirement with dispatch commits, never with socket I/O.
struct Adapter{dispatcher:Dispatcher,expected:Arc<Mutex<Option<PathBuf>>>,gate:Mutex<()>}
impl Handler for Adapter{
 fn connected(&self,peer:Peer)->bool{let _gate=self.gate.lock().unwrap();let Some(game)=self.expected.lock().unwrap().clone()else{return false};crate::local_owner::connected(peer,&game).is_ok()}
 fn request(&self,peer:Peer,request:&[u8])->Option<Vec<u8>>{let _gate=self.gate.lock().unwrap();if !crate::local_owner::current(peer){return None}let expected=self.expected.lock().unwrap().clone()?;let id=crate::local_owner::Id::from(peer);if !crate::local_owner::matches_installation(id,&expected){return None}self.dispatcher.prepare(id,request).map(|(mut reply,early)|{reply.insert(0,u8::from(early));reply})}
 fn delivery(&self,peer:Peer,request:&[u8],reply:Vec<u8>)->Option<Vec<u8>>{let _gate=self.gate.lock().unwrap();if !crate::local_owner::current(peer){return None}let expected=self.expected.lock().unwrap().clone()?;let id=crate::local_owner::Id::from(peer);if !crate::local_owner::matches_installation(id,&expected){return None}let (&source,payload)=reply.split_first()?;if source>1{return None}let reply=self.dispatcher.delivery(id,request,payload.to_vec(),source==1)?;
 #[cfg(test)]if request.len()>=64&&request[..8]==*b"NSP1\x01\0\0\0"&&reply.len()==41040{let sequence=u64::from_le_bytes(request[40..48].try_into().unwrap());let ack=u64::from_le_bytes(reply[32832..32840].try_into().unwrap());let bank_size=u32::from_le_bytes(request[60..64].try_into().unwrap())as usize;if bank_size>0&&bank_size<=4096&&bank_size<=request.len()-64&&sequence!=0&&ack==sequence{let body=&request[request.len()-bank_size..];if serde_json::from_slice::<serde_json::Value>(body).ok().is_some_and(|v|v["kind"]=="submit"){if let Some(d)=&self.dispatcher.diagnostics{let mut d=d.lock().unwrap();if std::mem::take(&mut d.local_drop_bank_reply_once){d.local_dropped_bank_replies+=1;return Some(Vec::new()); /* Invalid internal reply closes the real socket before any frame is sent. */}}}}}
 Some(reply)}
 fn disconnected(&self,peer:Peer){let _gate=self.gate.lock().unwrap();crate::local_owner::disconnected(peer)}
}
#[derive(Default,Clone,serde::Serialize)]struct Setup{ready:bool,error:Option<String>}
pub(crate) struct Control{select:std::sync::mpsc::SyncSender<PathBuf>,state:Arc<Mutex<Setup>>,server:Weak<Server>}
impl Control{
 pub(crate) fn select_game(&self,game:PathBuf)->Result<(),String>{self.select.try_send(game).map_err(|_|"Local bridge setup is busy or stopped".into())}
 pub(crate) fn error(&self)->Option<String>{self.state.lock().unwrap().error.as_ref().map(|e|format!("Local bridge setup failed: {e}"))}
 pub(crate) fn view(&self)->serde_json::Value{let state=self.state.lock().unwrap().clone();serde_json::json!({"transport":"authenticated-loopback-websocket","setup":state,"listener":self.server.upgrade().map(|s|s.stats()),"executableAttestation":false})}
 #[cfg(test)]pub(crate) fn owner(&self)->Option<crate::local_owner::Id>{self.server.upgrade()?.current().map(Into::into)}
 #[cfg(test)]pub(crate) fn outage(&self,blocked:bool){self.server.upgrade().unwrap().fixture_outage(blocked).unwrap();}
}
pub(crate) fn start(dispatcher:Dispatcher,expected:Arc<Mutex<Option<PathBuf>>>,stop:Arc<AtomicBool>)->Result<thread::JoinHandle<()>,String>{start_at(nt2_local_ws::protocol::PORT,false,dispatcher,expected,stop).map(|(_,task)|task)}
#[cfg(test)]pub(crate) fn fixture(dispatcher:Dispatcher,expected:Arc<Mutex<Option<PathBuf>>>,stop:Arc<AtomicBool>)->Result<(Arc<Control>,thread::JoinHandle<()>),String>{start_at(0,true,dispatcher,expected,stop)}
fn start_at(port:u16,private:bool,dispatcher:Dispatcher,expected:Arc<Mutex<Option<PathBuf>>>,stop:Arc<AtomicBool>)->Result<(Arc<Control>,thread::JoinHandle<()>),String>{
 let diagnostics=dispatcher.diagnostics.clone();let server=Arc::new(Server::bind(port,private,Arc::new(Adapter{dispatcher,expected,gate:Mutex::new(())}))?);let(tx,rx)=std::sync::mpsc::sync_channel::<PathBuf>(1);let state=Arc::new(Mutex::new(Setup::default()));let control=Arc::new(Control{select:tx,state:state.clone(),server:Arc::downgrade(&server)});
 if let Some(diagnostics)=diagnostics{diagnostics.lock().unwrap().local_ws=Some(control.clone());}
 let thread=thread::Builder::new().name("nt2-private-bootstrap".into()).spawn(move||{
  let descriptor=server.descriptor();let mut published:Option<Bootstrap>=None;let mut selected=None;
  while !stop.load(Ordering::Acquire){match rx.recv_timeout(Duration::from_millis(50)){
   Ok(game)=>{if selected.as_ref()==Some(&game)&&published.as_ref().is_some_and(|b|b.path().is_file()){if published.as_ref().is_some_and(|b|nt2_local_ws::private::read(b.path()).is_ok_and(|d|d.app==descriptor.app)){continue}else{*state.lock().unwrap()=Setup{ready:false,error:Some("Another or invalid app pairing occupies this mod folder; it was preserved. Close other NT2 instances normally and review setup.".into())};continue}}let result=(||{let root=game.join("mods/noita_together2").canonicalize().map_err(|_|"Verified mod directory unavailable".to_string())?;if published.as_ref().is_some_and(|b|b.path().is_file()&&b.path()==root.join(nt2_local_ws::private::DIRECTORY).join(nt2_local_ws::private::FILE)){return Ok(None)}if let Ok(other)=nt2_local_ws::private::read(&root.join(nt2_local_ws::private::DIRECTORY).join(nt2_local_ws::private::FILE)){if other.app!=descriptor.app&&other.endpoint!=descriptor.endpoint{if let Ok((address,_,_))=other.validate(private){if std::net::TcpStream::connect_timeout(&address.into(),Duration::from_millis(100)).is_ok(){return Err("Another live NT2 app owns this mod pairing; it was preserved. Close the other app normally before setup.".into())}}}}drop(published.take());Bootstrap::publish(&root,&descriptor).map(Some)})();
    match result{Ok(value)=>{if let Some(value)=value{published=Some(value);}selected=Some(game);*state.lock().unwrap()=Setup{ready:true,error:None}},Err(error)=>{*state.lock().unwrap()=Setup{ready:false,error:Some(error)}}}
   },Err(std::sync::mpsc::RecvTimeoutError::Timeout)=>{},Err(_)=>break
  }}
  // A late old app's cleanup cannot remove a descriptor owned by its successor.
  drop(published);let app=nt2_local_ws::protocol::unhex::<16>(&descriptor.app).ok();drop(server);if let Some(app)=app{crate::local_owner::retire_app(app);}
 }).map_err(|_|"Local bootstrap worker unavailable")?;Ok((control,thread))
}

#[cfg(test)]mod beta_tests{
 use super::*;
 #[test]fn beta_setup_republishes_private_pairing_after_owned_mod_replacement(){
  let own=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../spikes/noita-entry/.build").join(format!("beta-pairing-{}",uuid::Uuid::new_v4()));std::fs::create_dir_all(own.join("game/mods/noita_together2")).unwrap();let own=own.canonicalize().unwrap();let game=own.join("game");let root=game.join("mods/noita_together2");let stop=Arc::new(AtomicBool::new(false));let pending=Arc::new(Mutex::new(None));let dispatcher=Dispatcher{pending:pending.clone(),diagnostics:None,g3:None,social:None,startup:None};let(control,worker)=fixture(dispatcher,Arc::new(Mutex::new(Some(game.clone()))),stop.clone()).unwrap();
  let wait=||{let end=std::time::Instant::now()+Duration::from_secs(3);loop{if let Ok(d)=nt2_local_ws::private::read(&root.join(".nt2-local/bridge.json")){return d}assert!(std::time::Instant::now()<end,"Private descriptor was not regenerated");thread::sleep(Duration::from_millis(10));}};
  control.select_game(game.clone()).unwrap();let old=wait();std::fs::rename(&root,own.join("previous-mod")).unwrap();std::fs::create_dir(&root).unwrap();control.select_game(game.clone()).unwrap();let mut new=wait();assert_eq!(old.app,new.app);thread::sleep(Duration::from_millis(80));assert!(root.join(".nt2-local/bridge.json").is_file(),"Dropping the previous same-app descriptor removed its replacement");assert!(pending.lock().unwrap().is_none());new.app=uuid::Uuid::new_v4().simple().to_string();let foreign=nt2_local_ws::private::Bootstrap::publish(&root,&new).unwrap();control.select_game(game).unwrap();let end=std::time::Instant::now()+Duration::from_secs(2);while control.view()["setup"]["error"].is_null(){assert!(std::time::Instant::now()<end);thread::sleep(Duration::from_millis(10));}assert_eq!(nt2_local_ws::private::read(&root.join(".nt2-local/bridge.json")).unwrap().app,new.app,"Same-app reselection overwrote another pairing");drop(foreign);stop.store(true,Ordering::Release);worker.join().unwrap();assert!(!root.join(".nt2-local/bridge.json").exists());std::fs::remove_dir_all(own).unwrap();
 }
}
