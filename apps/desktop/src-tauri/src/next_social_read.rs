//! Independent current-information and historical-marker reads. Both use the
//! existing scoped Owner and bounded queue; neither can grant game authority.
use super::*;
use nt2_desktop_g0::protocol_g3_social::wire;
use std::future::Future;
pub(super) trait Source{
 fn discover(&mut self)->impl Future<Output=Result<(),String>>+Send;
 fn roster(&mut self)->impl Future<Output=Result<wire::Response,String>>+Send;
 fn latest(&mut self,after:&str)->impl Future<Output=Result<nt2_desktop_g0::g3_social::TimedSnapshot,String>>+Send;
 fn poi(&mut self,after:&str)->impl Future<Output=Result<wire::Response,String>>+Send;
}
impl Source for Owner{
 async fn discover(&mut self)->Result<(),String>{Owner::discover(self).await}
 async fn roster(&mut self)->Result<wire::Response,String>{Owner::roster(self).await}
 async fn latest(&mut self,after:&str)->Result<nt2_desktop_g0::g3_social::TimedSnapshot,String>{self.timed_snapshot(after).await}
 async fn poi(&mut self,after:&str)->Result<wire::Response,String>{self.poi_snapshot(after).await}
}
impl Latest{
 fn advance(&mut self,last:String,count:usize)->Result<(),String>{
  // The canonical endpoint returns at most16 rows. A short page exhausts the
  // current keyset; the next ordinary refresh must start at its beginning.
  if count<16{self.after.clear();self.pages=0;}else{self.pages+=1;if self.pages>8{return Err("A2 latest scan bound".into())}self.after=last;}Ok(())
 }
}
pub(super) async fn information(owner:&mut impl Source,latest:&mut Latest,scope:&wire::Scope,local_actor:String,store:Arc<Store>,chat:Vec<Value>,clock:Instant,hud:Arc<Mutex<a2::HudReplay>>,diagnostic:Arc<Mutex<Value>>)->Result<Vec<Vec<Command>>,String>{
 let started=Instant::now();
 {diagnostic.lock().unwrap()["information_read"]=json!({"phase":"discovery","at_ms":clock.elapsed().as_millis()});}
 owner.discover().await?;
 {diagnostic.lock().unwrap()["information_read"]=json!({"phase":"roster","at_ms":clock.elapsed().as_millis()});}
 let roster=owner.roster().await?;let players=a2::roster(&roster)?;let raw_players=roster.players.unwrap_or_default();hud.lock().unwrap().roster(&raw_players);
 diagnostic.lock().unwrap()["item_roster"]=json!(players.iter().filter_map(|p|raw_players.iter().find(|r|r.player_id.as_deref()==Some(&p.actor)).map(|r|json!({"actor":p.actor,"ghost":p.ghost,"session":r.game_session_id,"revision":r.inventory_revision,"sprite":r.held_wand_sprite_id,"raceRowKey":r.race_row_key,"raceStatus":r.race_status,"completedGoals":r.completed_goals}))).collect::<Vec<_>>());
 latest.presentation.roster(&raw_players);
 let roster_at=Instant::now();
 let run=format!("{}:{}:{}:{}",scope.room_id.as_deref().ok_or("Roster room")?,scope.run_id.as_deref().ok_or("Roster run")?,scope.run_epoch.as_deref().ok_or("Roster epoch")?,scope.config_revision.ok_or("Roster config")?);
 let mut commands=vec![optional_command(Display::Players{run,local:local_actor,players})];
 {diagnostic.lock().unwrap()["information_read"]=json!({"phase":"latest","at_ms":clock.elapsed().as_millis()});}
 let timed=owner.latest(&latest.after).await?;let observed=Instant::now();let elapsed=observed.duration_since(timed.request_started);let snapshot=timed.response;let values=snapshot.projections.unwrap_or_default();if values.len()>16{return Err("A2 latest page bound".into())}
 let mut accepted=0u32;let mut replayed=0u32;
 let mut last=latest.after.clone();for p in &values{let kind=if p.hud.is_some(){"hud"}else if p.emote.is_some(){"emote"}else if p.cosmetic.is_some(){"cosmetic"}else{return Err("A2 projection kind".into())};let next=format!("{}:{kind}",p.game_session_id.as_deref().ok_or("Missing latest actor")?);if next<=last{return Err("A2 latest page order".into())}last=next;
  if p.hud.is_some(){let value=a2::hud(scope,&raw_players,p)?;if let Some(h)=hud.lock().unwrap().accept(value)?{commands.push(optional_command(Display::Hud(h)));{accepted+=1;}}else{replayed+=1;}}
  else if let Some(value)=latest.presentation.accept(scope,&raw_players,p,elapsed,clock.elapsed().as_secs_f64())?{commands.push(Command::A3(value));super::actions::reconcile(store.clone(),p).await?;}
 }
 if snapshot.next_key.as_deref().unwrap_or("")!=last{return Err("A2 latest cursor".into())}latest.advance(last,values.len())?;
 {let mut d=diagnostic.lock().unwrap();let old=d["consumer"].clone();let rows=values.iter().filter(|p|p.hud.is_some()).count() as u64;let known=values.iter().filter(|p|p.hud.as_ref().is_some_and(|h|h.hp_known==Some(true))).count() as u64;d["consumer"]=json!({"queries":old["queries"].as_u64().unwrap_or(0).saturating_add(1),"hud_rows_total":old["hud_rows_total"].as_u64().unwrap_or(0).saturating_add(rows),"known_hp_rows_total":old["known_hp_rows_total"].as_u64().unwrap_or(0).saturating_add(known),"display_commands_total":old["display_commands_total"].as_u64().unwrap_or(0).saturating_add(u64::from(accepted)),"latest_rows":values.len(),"hud_rows":rows,"known_hp_rows":known,"request_elapsed_ms":elapsed.as_millis(),"display_commands":accepted,"ignored_revisions":replayed,"read_ms":started.elapsed().as_millis()});}
 let latest_at=Instant::now();
 {diagnostic.lock().unwrap()["information_read"]=json!({"phase":"journal","at_ms":clock.elapsed().as_millis()});}
 let saved=stored(store.clone()).await?;let room=scope.room_id.as_deref().ok_or("Chat room scope")?;let(_high,chats)=chat_commands(chat,saved.chat_high,room);
 // Capturing/queueing is not native receipt. The existing producer journal
 // worker commits only the exact output frame's accepted native ACK.
 {diagnostic.lock().unwrap()["information_read"]=json!({"phase":"complete","at_ms":clock.elapsed().as_millis(),"roster_ms":roster_at.duration_since(started).as_millis(),"latest_ms":latest_at.duration_since(roster_at).as_millis(),"journal_ms":latest_at.elapsed().as_millis(),"total_ms":started.elapsed().as_millis()});}
 let mut batches=vec![commands];batches.extend(chats.into_iter().map(|c|vec![c]));Ok(batches)
}
pub(super) async fn markers(owner:&mut impl Source,scan:&mut Scan,generation:u64,store:Arc<Store>,_clock:Instant,#[cfg(test)]progress:Arc<Mutex<Value>>)->Result<Vec<Vec<Command>>,String>{
 #[cfg(test)]let started=Instant::now();
 #[cfg(test)]{let mut p=progress.lock().unwrap();let history=p["history"].take().as_array().cloned().unwrap_or_default();let rescans=p["rescans"].clone();*p=json!({"phase":"poi-discovery","poi_after":scan.after,"at_ms":_clock.elapsed().as_millis(),"history":history,"rescans":rescans});}
 owner.discover().await?;
 #[cfg(test)]{progress.lock().unwrap()["phase"]=json!("poi-store");}
 let saved=stored(store).await?;for sequence in &saved.dismissed{scan.consumer.remember_dismissal(*sequence)?;}
 #[cfg(test)]let stored_at=Instant::now();
 #[cfg(test)]{progress.lock().unwrap()["phase"]=json!("poi-request");}
 let response=owner.poi(&scan.after).await?;let(begin,page)=poi_page(scan,generation,&response)?;
 #[cfg(test)]{let mut p=progress.lock().unwrap();p["phase"]=json!("captured");p["page_complete"]=json!(page.complete);p["page_rows"]=json!(page.markers.len());let row=json!({"after":p["poi_after"],"next":page.next,"high":page.high,"rows":page.markers.len(),"complete":page.complete,"at_ms":_clock.elapsed().as_millis(),"store_ms":stored_at.duration_since(started).as_millis(),"poi_ms":stored_at.elapsed().as_millis(),"total_ms":started.elapsed().as_millis()});let history=p["history"].as_array_mut().unwrap();if history.len()>=32{history.remove(0);}history.push(row);}
 let mut commands=Vec::new();if let Some(high)=begin{commands.push(Command::Begin(high));}commands.push(Command::Page(page));Ok(vec![commands])
}
#[cfg(test)]mod tests{use super::*;include!("next_social_read_test.rs");}
