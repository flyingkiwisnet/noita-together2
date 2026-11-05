//! Typed UI operations on the existing session, revision and lifecycle owners.
use super::*;
static CONTRACT:std::sync::LazyLock<Value>=std::sync::LazyLock::new(||serde_json::from_str(include_str!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../docs/g3/ux1-service-contract.json"))).unwrap());
#[cfg(test)]#[path="next_ux1_test.rs"]mod tests;
#[path="next_ux1_outcome.rs"]mod outcomes;
pub(super) fn definite_outcome(error:&str)->bool{outcomes::definite(error)}
#[derive(Default)]pub(super) struct State{context:String,binding:String,unconfirmed:bool,original:Option<Retry>,result:Value,error:Option<String>,rule_original:Option<(String,Value,Instant)>,rules:Value,access:Option<wire::RoomAccess>,review:Option<outcomes::Review>,acknowledged:Value,#[cfg(test)]lose_reply:bool}
#[derive(Deserialize)]#[serde(deny_unknown_fields)]pub(crate) struct Create{
 pub name:Option<String>,pub config:wire::Config,pub capacity:u32,pub password:Option<String>,pub visibility:String,pub allow_guests:bool,
}
impl<S:CredentialStore> Core<S>{
 #[cfg(test)]pub(super) fn ux1_fixture_lose_next_reply(&mut self){self.ux1_prepare().unwrap();self.ux1.lose_reply=true;}
 #[cfg(test)]pub(super) fn ux1_fixture_local_deadline(&mut self){self.ux1.original.as_mut().unwrap().until=Instant::now();}
 // Isolated composition can discard a validated committed reply; never compiled into a normal app.
 pub(super) fn ux1_observe_reply(&mut self,response:Result<wire::Response,String>)->Result<wire::Response,String>{#[cfg(test)]if response.is_ok()&&std::mem::take(&mut self.ux1.lose_reply){return Err("Fixture committed reply lost".into())}response}
 pub(super) fn remember_account_name(&mut self){let name=if self.remember{self.desktop.as_ref().and_then(|d|d.principal.display_name.clone())}else{None};if let Some(server)=self.recent_servers.iter_mut().find(|s|s.origin==self.base&&s.service_id==self.service){server.remembered_name=name;server.has_saved_account=Some(self.remember&&self.desktop.is_some());}if let Some(preferences)=self.preferences.as_ref(){let _=preferences.save(&self.recent_servers,self.device);}}
 pub async fn restore_session(&mut self,origin:&str,remember:bool)->Result<(),String>{let result=async{self.select(origin,"",true).await?;if self.desktop.is_none(){return Err("No saved session is available for this service. Choose a sign-in method.".into())}self.remember=remember;if !remember{if let Some(profile)=self.profile.as_ref(){self.store.delete(profile).map_err(|e|e.to_string())?;}}self.remember_account_name();if self.info.as_ref().and_then(|i|i.mock_provider)!=Some(true){self.auth1.active=true;self.poll_auth1();}Ok(())}.await;self.login_result(&result);result}
 pub async fn fresh_login(&mut self,origin:&str,remember:bool,provider:&str,recovery:Option<IdentityRecovery>)->Result<(),String>{let result=async{if let Some(recovery)=recovery{self.recover_identity(origin,false,recovery).await?;}else{self.select(origin,"",false).await?;}self.remember=remember;if self.desktop.is_some(){return Err("Sign out before choosing a different account".into())}self.sign_in(provider).await}.await;self.login_result(&result);result}
 pub fn forget_saved(&mut self,origin:&str)->Result<(),String>{if self.desktop.is_some()||self.auth.is_some()||self.guest_retry.is_some(){return Err("Finish or cancel sign-in before forgetting the saved account".into())}let address=self.credential_profile(origin,"discovery")?;let selected=self.recent_servers.iter().find(|s|s.origin==address.origin()).ok_or("No saved account at this address")?;let profile=self.credential_profile(&selected.origin,&selected.service_id)?;{let _guard=self.credential_writer.lock().unwrap();self.store.delete(&profile).map_err(|e|e.to_string())?;}if let Some(server)=self.recent_servers.iter_mut().find(|s|s.origin==address.origin()){server.remembered_name=None;server.has_saved_account=Some(false);}if let Some(preferences)=self.preferences.as_ref(){preferences.save(&self.recent_servers,self.device)?;}Ok(())}
 pub(super) fn ux1_projection(&mut self,response:&wire::Response){self.ux1_sync_context();if let Some(access)=response.access.as_ref(){self.ux1.access=Some(access.clone());}}
 pub fn ux1_guard_room(&self,room:Option<&str>)->Result<(),String>{if room.is_none()||room!=self.room.as_ref().and_then(|r|r.room_id.as_deref()){Err("Lobby selection changed; this action was not sent".into())}else{Ok(())}}
 fn ux1_sync_context(&mut self){let context=self.ux1_context();if self.ux1.context==context{return}let binding=self.ux1_binding();if self.ux1.binding==binding&&(self.ux1.original.is_some()||self.ux1.rule_original.is_some()){self.ux1.context=context;self.ux1.unconfirmed=true;self.ux1.review=None;self.ux1.error=Some("Lobby run changed; earlier action outcome remains unknown. Review current state before a separate new action".into());}else{self.ux1=State{context,binding,..Default::default()};}}
 fn ux1_binding(&self)->String{json!([self.base,self.service,self.epoch,self.desktop.as_ref().map(|s|(&s.id,&s.epoch,&s.account)),self.room.as_ref().map(|r|&r.room_id),self.generation.load(Ordering::Acquire)]).to_string()}
 fn ux1_context(&self)->String{json!([self.base,self.service,self.epoch,self.desktop.as_ref().map(|s|(&s.id,&s.epoch,&s.account)),self.room.as_ref().map(|r|(&r.room_id,&r.run_id,&r.run_epoch)),self.generation.load(Ordering::Acquire)]).to_string()}
 pub(super) fn ux1_view(&self)->Value{
  let current=self.ux1.context==self.ux1_context();let contract=&*CONTRACT;
  json!({"access":if current{self.ux1.access.as_ref()}else{None},"compatibility":self.info.as_ref().and_then(|i|i.compatibility.as_ref()),"chat":contract["chat"],"result":if current{self.ux1.result.clone()}else{Value::Null},"rules":if current{self.ux1.rules.clone()}else{Value::Null},"pending":current&&(self.ux1.original.is_some()||self.ux1.rule_original.is_some()),"error":if current{self.ux1.error.as_ref()}else{None},"blocked":current&&self.ux1.unconfirmed,"chat_status":if self.chat_feedback["room"].as_str()==self.room.as_ref().and_then(|r|r.room_id.as_deref())&&self.chat_feedback["session"].as_str()==self.desktop.as_ref().map(|d|d.epoch.as_str()){self.chat_feedback.clone()}else{Value::Null},"outcome":if current{self.ux1_outcome("lobby")}else{Value::Null},"chat_outcome":self.ux1_outcome("chat"),"outcome_review":if current{self.ux1_review_view()}else{Value::Null},"acknowledged":if current{self.ux1.acknowledged.clone()}else{Value::Null},"chat_pending":self.chat_retry.is_some(),"chat_request":self.chat_retry.as_ref().and_then(|r|r.request.request_id.as_ref())})
 }
 fn ux1_prepare(&mut self)->Result<(),String>{
  if self.desktop.is_none(){return Err("Sign in first".into())}
  self.ux1_sync_context();Ok(())
 }
}
impl<S:CredentialStore+'static> Core<S>{
 pub(super) async fn ux1_send(&mut self,request:wire::Request,mutation:bool)->Result<(),String>{
  self.ux1_prepare()?;if mutation&&(self.ux1.unconfirmed||self.ux1.original.is_some()||self.ux1.rule_original.is_some()){return Err("Previous lobby action is unconfirmed. Retry that same action before making another change".into())}
  if !mutation{let response=self.rpc(&request,false,false).await?;self.absorb(&response);self.ux1.result=json!({"playerDetails":response.player_details,"roomBans":response.room_bans,"nextRoomBanAccountId":response.next_room_ban_account_id});return Ok(())}
  if request.end_run.is_some(){if self.lc1{self.lc1_retire_preparation();if let Some(p)=self.pending.lock().unwrap().as_mut(){p.release=false;p.authority_until=Instant::now();}self.g3_pause();}else{self.resume_intent.terminal();self.stop_setup("Host ended the run; review and Join for a fresh New Game");self.clear_game();}self.ux1.context=self.ux1_context();}
  self.ux1.original=Some(Retry{request,until:Instant::now()+Duration::from_secs(30)});self.ux1_retry().await
 }
 pub async fn ux1_retry(&mut self)->Result<(),String>{
  self.ux1_prepare()?;
  if let Some((path,request,until))=self.ux1.rule_original.take(){if self.ux1.unconfirmed||Instant::now()>=until{self.ux1.rule_original=Some((path,request,until));self.ux1.unconfirmed=true;self.ux1.error=Some("Earlier outcome unknown. Refresh current state for review, then acknowledge before a separate new action".into());return Err(self.ux1.error.clone().unwrap())}return self.ux1_rules_send(&path,request,true,until).await;}
  let retry=self.ux1.original.take().ok_or("No lobby action needs retry")?;
  if self.ux1.unconfirmed||(Instant::now()>=retry.until&&outcomes::local_receipt_window(&retry.request)){self.ux1.original=Some(retry);self.ux1.unconfirmed=true;self.ux1.error=Some("Earlier outcome unknown. Refresh current state for review, then acknowledge before a separate new action".into());return Err(self.ux1.error.clone().unwrap())}
  let response=self.rpc(&retry.request,false,false).await;let response=self.ux1_observe_reply(response);match response{
   Ok(response)=>{
    let deleted=response.deleted_room_id.clone();self.absorb(&response);self.ux1.context=self.ux1_context();self.ux1.error=None;
    if let Some(id)=deleted.as_ref(){if self.room.as_ref().and_then(|r|r.room_id.as_deref())==Some(id.as_str()){self.stop_setup("Lobby deleted");self.clear_game();self.room=None;}self.rooms.retain(|r|r.room_id.as_deref()!=Some(id.as_str()));self.ux1.context=self.ux1_context();}
    self.ux1.result=json!({"accepted":true,"created":retry.request.room_create.is_some(),"deletedRoomId":deleted});
    if retry.request.end_run.is_some(){self.lc1_terminal("Host ended the run");}
    if retry.request.cancel_run.is_some(){self.lc1_retire_preparation();}
    if retry.request.room_create.is_some(){let password=retry.request.room_create.as_ref().and_then(|c|c.password.clone());self.review_created_lobby()?;let room=self.room.as_ref().and_then(|r|r.room_id.clone()).ok_or("Created lobby unavailable")?;self.join(&room,password).await?;self.ux1.context=self.ux1_context();}
    Ok(())
   },Err(error)=>{
    let definite=outcomes::definite(&error);if error=="EXPIRED"{self.ux1.unconfirmed=true;}self.ux1.error=Some(if definite{error.clone()}else{"Outcome unconfirmed. Retry keeps the same action identity".into()});if !definite{self.ux1.original=Some(retry);}Err(error)
   }
  }
 }
 pub async fn ux1_create(&mut self,input:Create)->Result<(),String>{
  if self.room.is_some(){return Err("Leave the current lobby before creating another".into())}
  let mut q=self.request(false);q.room_id=None;q.expected_revision=None;q.config_revision=None;q.run_id=None;q.run_epoch=None;
  q.room_create=Some(wire::RoomCreate{name:input.name,config:Some(input.config),visibility:Some(input.visibility),allow_guests:Some(input.allow_guests),capacity:Some(input.capacity),password:input.password});self.ux1_send(q,true).await
 }
 pub async fn ux1_room_action(&mut self,kind:&str,revision:u32,config:Option<wire::Config>,policy:Option<wire::RoomPolicy>,access:Option<wire::RoomAccess>,moderation:Option<wire::RoomModerate>)->Result<(),String>{
  let room=self.room.as_ref().ok_or("Choose a lobby")?;if room.revision!=Some(revision){return Err("Lobby changed. Review the latest state before saving".into())}
  let mut q=self.request(false);match kind{
   "configure"=>q.configure=Some(config.ok_or("Configuration required")?),"policy"=>q.room_policy=Some(policy.ok_or("Admission policy required")?),"access"=>q.room_access=Some(access.ok_or("Access policy required")?),
   "moderate"=>q.room_moderate=Some(moderation.ok_or("Named player action required")?),"cancel"=>q.cancel_run=Some(wire::Empty{}),"end"=>q.end_run=Some(wire::Empty{}),"delete"=>q.room_delete=Some(wire::Empty{}),_=>return Err("Unsupported lobby action".into()),
  };self.ux1_send(q,true).await
 }
 pub async fn ux1_refresh(&mut self)->Result<(),String>{let mut q=self.request(false);q.room_get=Some(wire::Empty{});self.ux1_send(q,false).await}
 pub async fn ux1_player(&mut self,target:wire::PlayerTarget)->Result<(),String>{let mut q=self.request(false);q.room_player_details=Some(target);self.ux1_send(q,false).await?;let details=self.ux1.result.clone();let mut refresh=self.request(false);refresh.room_get=Some(wire::Empty{});let response=self.rpc(&refresh,false,false).await?;self.absorb(&response);self.ux1.result=details;Ok(())}
 pub async fn ux1_bans(&mut self,page:wire::RoomBanList)->Result<(),String>{let mut q=self.request(false);q.room_ban_list=Some(page);self.ux1_send(q,false).await}
 async fn ux1_rules_send(&mut self,path:&str,request:Value,mutation:bool,until:Instant)->Result<(),String>{
  self.ux1_prepare()?;let context=self.ux1.context.clone();let token=self.desktop.as_ref().unwrap().access.clone();let bytes=zeroize::Zeroizing::new(serde_json::to_vec(&request).map_err(|_|"Rules encoding failed")?);
  match path{"/v3/c1/rpc"=>{nt2_desktop_g0::protocol_c1::request(&bytes).map_err(|_|"Invalid player rules")?;},"/v3/c2/rpc"=>{nt2_desktop_g0::protocol_c2::request(&bytes).map_err(|_|"Invalid world rules")?;},"/v3/rejoin/rpc"=>{serde_json::from_slice::<nt2_desktop_g0::protocol_rejoin::wire::Request>(&bytes).map_err(|_|"Invalid recovery review")?;},_=>return Err("Unsupported rules route".into())}
  let result=tokio::time::timeout(Duration::from_secs(4),async{
   let mut response=self.http.post(format!("{}{path}",self.base)).bearer_auth(token.as_str()).header("content-type","application/json").body(bytes.to_vec()).send().await.map_err(|_|"Rules service unavailable")?;
   if !response.status().is_success()||response.url().as_str()!=format!("{}{path}",self.base){return Err(format!("Rules service refused (HTTP {})",response.status().as_u16()))}let mut body=Vec::new();while let Some(chunk)=response.chunk().await.map_err(|_|"Rules reply interrupted")?{if body.len()+chunk.len()>65536{return Err("Rules reply exceeds bounds".into())}body.extend_from_slice(&chunk);}
   let value=if path=="/v3/c1/rpc"{serde_json::to_value(nt2_desktop_g0::protocol_c1::response(&body).map_err(|_|"Invalid player rules reply")?).unwrap()}else if path=="/v3/c2/rpc"{serde_json::to_value(nt2_desktop_g0::protocol_c2::response(&body).map_err(|_|"Invalid world rules reply")?).unwrap()}else{serde_json::to_value(nt2_desktop_g0::protocol_rejoin::response(&body).map_err(|_|"Invalid recovery review reply")?).unwrap()};
   if value["serviceId"]!=request["serviceId"]||value["serviceEpoch"]!=request["serviceEpoch"]||value["requestId"]!=request["requestId"]{return Err("Rules reply context changed".into())}if value["error"].is_object(){return Err(value["error"]["code"].as_str().unwrap_or("Rules request refused").to_owned())}Ok(value)
  }).await.unwrap_or_else(|_|Err("Rules service deadline".into()));
  if context!=self.ux1_context(){return Err("Rules action belongs to the previous lobby".into())}
  match result{Ok(value)=>{if path=="/v3/rejoin/rpc"{if value["participants"].is_array(){self.ux1.rules["recovery"]=value["participants"].clone();}if value["participant"].is_object(){self.ux1.rules["recoveryReport"]=value["participant"].clone();}}else{self.ux1.rules[if path=="/v3/c1/rpc"{"player"}else{"world"}]=value["snapshot"].clone();}self.ux1.error=None;Ok(())},Err(e)=>{if mutation&&!outcomes::definite(&e){if e=="EXPIRED"{self.ux1.unconfirmed=true;}self.ux1.rule_original=Some((path.into(),request,until));}self.ux1.error=Some(e.clone());Err(e)}}
 }
 pub async fn ux1_rules(&mut self,player:bool,player_rules:Option<nt2_desktop_g0::protocol_c1::wire::RuleSet>,world_rules:Option<nt2_desktop_g0::protocol_c2::wire::RulesSet>)->Result<(),String>{
  self.ux1_prepare()?;if (player_rules.is_some()||world_rules.is_some())&&(self.ux1.unconfirmed||self.ux1.original.is_some()||self.ux1.rule_original.is_some()){return Err("Confirm the previous lobby action first".into())}
  let room=self.room.as_ref().and_then(|r|r.room_id.clone()).ok_or("Choose a lobby")?;let mut request=json!({"major":1,"revision":if player{"g3-c1-1"}else{"g3-c2-1"},"serviceId":self.service,"serviceEpoch":self.epoch,"sessionEpoch":self.desktop.as_ref().unwrap().epoch,"requestId":uuid::Uuid::new_v4(),"roomId":room});let mutation=player_rules.is_some()||world_rules.is_some();
  if mutation{request["rulesSet"]=if player{json!(player_rules.ok_or("Player rules required")?)}else{json!(world_rules.ok_or("World rules required")?)};}else{request["rulesGet"]=json!({});}
  self.ux1_rules_send(if player{"/v3/c1/rpc"}else{"/v3/c2/rpc"},request,mutation,Instant::now()+Duration::from_secs(30)).await
 }
 pub async fn ux1_recovery(&mut self,target:Option<nt2_desktop_g0::protocol_rejoin::wire::Participant>,report:Option<&str>)->Result<(),String>{
  self.ux1_prepare()?;let room=self.room.as_ref().ok_or("Choose a lobby")?;let mut q=json!({"major":1,"revision":nt2_desktop_g0::protocol_rejoin::REVISION,"serviceId":self.service,"serviceEpoch":self.epoch,"sessionEpoch":self.desktop.as_ref().unwrap().epoch,"requestId":uuid::Uuid::new_v4(),"roomId":room.room_id});let mutation=target.is_some()&&report.is_none();
  if let Some(target)=target{if mutation&&(self.ux1.unconfirmed||self.ux1.original.is_some()||self.ux1.rule_original.is_some()){return Err("Confirm the previous lobby action first".into())}let association=target.association.as_ref().ok_or("Current recovery association required")?;if association.room_id!=room.room_id||association.run_id!=room.run_id||association.config_revision!=room.config_revision||target.review_required!=Some(true){return Err("Recovery review changed; refresh the current lobby".into())}q["runId"]=json!(association.run_id);q["participantId"]=json!(association.participant_id);q["worldToken"]=json!(association.world_token);q["configRevision"]=json!(association.config_revision);q["generation"]=json!(target.generation);if let Some(report)=report{if !["original","pending"].contains(&report){return Err("Unsupported inspection selector".into())}q["status"]=json!({});q["observationReport"]=json!(report);}else{q["review"]=json!({"participantId":association.participant_id,"observationRevision":target.pending_observation_revision,"contentDigest":target.pending_content_digest});}}else{q["list"]=json!({});}
  self.ux1_rules_send("/v3/rejoin/rpc",q,mutation,Instant::now()+Duration::from_secs(30)).await
 }
}
