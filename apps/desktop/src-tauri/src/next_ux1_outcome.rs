//! Unknown outcomes are observations, never proof that a mutation failed.
use super::*;
pub(super) fn definite(error:&str)->bool{!error.is_empty()&&!matches!(error,"EXPIRED"|"SERVICE_UNAVAILABLE"|"UNAUTHENTICATED"|"UNAUTHORIZED"|"STALE_EPOCH")&&error.bytes().all(|b|b.is_ascii_uppercase()||b==b'_')}
pub(super) fn local_receipt_window(q:&wire::Request)->bool{q.configure.is_some()||q.final_start.is_some()||q.cancel_run.is_some()||q.end_run.is_some()}
pub(super) struct Review{token:String,lane:String,identity:String,context:String,stamp:String,until:Instant,summary:Value}
impl<S:CredentialStore> Core<S>{
 fn ux1_pending_outcome(&self,lane:&str)->Option<(String,String,bool)>{
  if lane=="chat"{let r=self.chat_retry.as_ref()?;if r.request.room_id!=self.room.as_ref().and_then(|r|r.room_id.clone())||r.request.session_epoch!=self.desktop.as_ref().map(|d|d.epoch.clone()){return None}return Some((r.request.request_id.clone()?,"chatSend".into(),Instant::now()>=r.until||self.chat_feedback["status"]=="expired"))}
  if lane!="lobby"{return None}if let Some((path,q,until))=&self.ux1.rule_original{return Some((q["requestId"].as_str()?.into(),if path=="/v3/rejoin/rpc"{"recovery review"}else if path=="/v3/c1/rpc"{"player rules"}else{"world rules"}.into(),self.ux1.unconfirmed||Instant::now()>=*until))}
  let r=self.ux1.original.as_ref()?;let q=&r.request;let kind=if q.room_create.is_some(){"create lobby"}else if q.configure.is_some(){"configuration"}else if q.room_policy.is_some(){"admission settings"}else if q.room_access.is_some(){"invitations/access"}else if q.room_moderate.is_some(){"player moderation"}else if q.final_start.is_some(){"Start"}else if q.cancel_run.is_some(){"cancel preparation"}else if q.end_run.is_some(){"End run"}else{"delete lobby"};
  Some((q.request_id.clone()?,kind.into(),self.ux1.unconfirmed||Instant::now()>=r.until&&local_receipt_window(q)))
 }
 pub(super) fn ux1_outcome(&self,lane:&str)->Value{self.ux1_pending_outcome(lane).map(|(id,kind,expired)|json!({"id":id,"kind":kind,"expired":expired})).unwrap_or(Value::Null)}
 fn ux1_outcome_stamp(&self)->String{json!([self.room,self.ux1.access,self.ux1.rules,self.rooms]).to_string()}
 pub(super) fn ux1_review_view(&self)->Value{let Some(r)=&self.ux1.review else{return Value::Null};if r.context!=self.ux1_context()||Instant::now()>=r.until||r.stamp!=self.ux1_outcome_stamp()||self.ux1_pending_outcome(&r.lane).is_none_or(|(id,_,expired)|id!=r.identity||!expired){return Value::Null}json!({"token":r.token,"lane":r.lane,"identity":r.identity,"summary":r.summary})}
 pub fn ux1_acknowledge_outcome(&mut self,lane:&str,token:&str)->Result<(),String>{
  let current=self.ux1_review_view();if current.is_null()||current["token"]!=token||current["lane"]!=lane{return Err("Current state review changed or expired. Refresh and review again".into())}
  let (id,kind,expired)=self.ux1_pending_outcome(lane).ok_or("Earlier outcome no longer selected")?;if !expired{return Err("Retry the retained original action first".into())}
  if lane=="chat"{self.chat_retry=None;self.chat_feedback["status"]=json!("acknowledged_unknown");}else if lane=="lobby"{self.ux1.original=None;self.ux1.rule_original=None;self.ux1.unconfirmed=false;}else{return Err("Unsupported outcome lane".into())}
  self.ux1.acknowledged[lane]=json!({"id":id,"kind":kind,"unknown":true});self.ux1.review=None;self.ux1.error=None;Ok(())
 }
}
impl<S:CredentialStore+'static> Core<S>{
 pub async fn ux1_review_outcome(&mut self,lane:&str,identity:&str)->Result<(),String>{
  self.ux1_prepare()?;let(id,kind,expired)=self.ux1_pending_outcome(lane).ok_or("Select the retained uncertain action")?;if id!=identity{return Err("Earlier action selection changed".into())}if !expired{return Err("Retry the retained original action first; its receipt may still be available".into())}
  self.ux1.review=None;let binding=self.ux1_binding();let path=self.ux1.rule_original.as_ref().map(|r|r.0.clone());let mut summary=json!({"earlierAction":kind,"originalRequest":id,"outcome":"unknown","warning":"The earlier action may already have succeeded. Current state, visible lists and retained chat history can be incomplete. A new submission can repeat an earlier action."});
  if lane=="chat"{let mut q=self.request(false);q.chat_list=Some(wire::ChatList{limit:Some(32),after_sequence:None});let response=self.rpc(&q,false,false).await?;summary["retainedChatPage"]=json!(response.chat_messages.unwrap_or_default());}
  else{if self.room.is_some(){let mut q=self.request(false);q.room_get=Some(wire::Empty{});match self.rpc(&q,false,false).await{Ok(response)=>{self.absorb(&response);summary["currentLobby"]=json!(self.room);summary["currentAccess"]=json!(self.ux1.access);},Err(e)if matches!(e.as_str(),"NOT_FOUND"|"ROOM_DELETED")=>summary["currentLobbyUnavailable"]=json!(e),Err(e)=>return Err(e)}}else{self.room_page(false).await?;summary["visibleLobbyPage"]=json!(self.rooms);}
   if let Some(path)=path{match path.as_str(){"/v3/c1/rpc"=>self.ux1_rules(true,None,None).await?,"/v3/c2/rpc"=>self.ux1_rules(false,None,None).await?,"/v3/rejoin/rpc"=>self.ux1_recovery(None,None).await?,_=>return Err("Unsupported uncertain action".into())}summary["currentRulesOrRecovery"]=self.ux1.rules.clone();}
  }
  if binding!=self.ux1_binding()||self.ux1_pending_outcome(lane).is_none_or(|(current,_,expired)|current!=id||!expired){return Err("Action context changed while reading current state; earlier action was not replayed".into())}
  self.ux1.review=Some(Review{token:uuid::Uuid::new_v4().to_string(),lane:lane.into(),identity:id,context:self.ux1_context(),stamp:self.ux1_outcome_stamp(),until:Instant::now()+Duration::from_secs(30),summary});Ok(())
 }
}
