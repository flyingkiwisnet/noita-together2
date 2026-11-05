//! One request's transport identity over the existing lifecycle/auth owners.
//! No retry controller, bearer history, authority grant or deadline restamp.
use super::*;
#[derive(Clone)]pub(super) struct Call{
 context:String,auth:auth_owner::RequestOwner,fence:Arc<AtomicU64>,generation:u64,owner_fence:Arc<AtomicU64>,owner_generation:u64,
 request:Arc<Mutex<Option<w::Request>>>,used:Arc<Mutex<Option<u64>>>,
}
impl Call{
 #[cfg(test)]pub(super) fn request_for_test(&self)->Option<w::Request>{self.request.lock().unwrap().clone()}
 fn owned(&self)->bool{self.fence.load(Ordering::Acquire)==self.generation&&self.owner_fence.load(Ordering::Acquire)==self.owner_generation}
 pub(super) fn current(&self,context:&str)->bool{self.context==context&&self.owned()&&self.auth.current()}
 pub(super) fn bind(&self,q:&mut w::Request){
  let mut slot=self.request.lock().unwrap();if let Some(old)=slot.as_ref(){let mut same=q.clone();same.request_id=old.request_id.clone();if serde_json::to_vec(&same).unwrap()==serde_json::to_vec(old).unwrap(){q.request_id=old.request_id.clone();}}
  *slot=Some(q.clone());
 }
 pub(super) async fn send(&self,http:&reqwest::Client,origin:&str,q:&w::Request)->Result<(w::Response,Session),String>{
  if !self.owned(){return Err("Lifecycle request context retired".into())}
  let(session,used)=self.auth.read(origin,q.service_id.as_deref().ok_or("Lifecycle service absent")?,q.service_epoch.as_deref().ok_or("Lifecycle epoch absent")?,q.session_epoch.as_deref().ok_or("Lifecycle desktop epoch absent")?)?;
  *self.used.lock().unwrap()=Some(used);
  let response=rpc(http,origin,&session,q).await?;Ok((response,session))
 }
 pub(super) fn stale_auth(&self,error:&str)->bool{error=="UNAUTHENTICATED"&&self.used.lock().unwrap().is_some_and(|g|self.auth.newer_verified(g))}
 pub(super) fn view(&self)->Value{let q=self.request.lock().unwrap();json!({"operation":q.as_ref().map(|q|if q.prepare.is_some(){"prepare"}else{"reconcile"}),"requestId":q.as_ref().and_then(|q|q.request_id.as_ref()),"usedCredentialGeneration":*self.used.lock().unwrap(),"currentCredentialGeneration":self.auth.generation()})}
}
impl<S:CredentialStore> Core<S>{
 pub(super) fn lc1_call_context(&self,prepare:bool)->String{json!([self.base,self.service,self.epoch,self.device,format!("{:?}",self.profile),self.remember,self.desktop.as_ref().map(|d|(&d.account,&d.id,&d.epoch)),self.room.as_ref().map(|r|(&r.room_id,&r.run_id,&r.run_epoch,r.config_revision)),self.generation.load(Ordering::Acquire),self.world.preparation_fence.0.load(Ordering::Acquire),self.world.native,self.world.coordinator.ticket.as_ref().map(|t|(t.world,t.revision,t.native,t.attempt)),if prepare{self.world.draft.as_ref().map(|r|(&r.identity,r.world))}else{self.world.record.as_ref().map(|r|(&r.identity,r.world))}]).to_string()}
 pub(super) fn lc1_call(&mut self,prepare:bool)->Result<Call,String>{
  self.publish_auth_owner();let context=self.lc1_call_context(prepare);let old=if prepare{&self.world.prepare_call}else{&self.world.network_call};if let Some(call)=old.as_ref().filter(|c|c.current(&context)){return Ok(call.clone())}
  let auth=self.lifecycle_request_owner()?;let fence=if prepare{self.world.preparation_fence.0.clone()}else{self.generation.clone()};let generation=fence.load(Ordering::Acquire);
  Ok(Call{context,auth,fence,generation,owner_fence:self.generation.clone(),owner_generation:self.generation.load(Ordering::Acquire),request:Default::default(),used:Default::default()})
 }
 pub(super) fn lc1_call_result(&mut self,call:&Call,disposition:&str,error:Option<&str>){let mut v=call.view();v["disposition"]=json!(disposition);if let Some(e)=error{v["code"]=json!(if e.len()<=64&&e.bytes().all(|b|b.is_ascii_uppercase()||b==b'_'){e}else{"other"});self.world.credential_failure=v.clone();}self.world.credential_result=v;}
}
