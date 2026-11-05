//! Canonical bounded transport. Every result is correlated before any journal write.
use super::*;
pub(super) fn single(outer:&wire::Request,e:&wire::BankBatchEntry)->wire::Request{
 wire::Request{scope:e.scope.clone(),current_scope:e.current_scope.clone(),bank_batch:None,gold_intent:e.gold_intent.clone(),gold_status:e.gold_status.clone(),gold_receipt:e.gold_receipt.clone(),item_deposit:e.item_deposit.clone(),item_claim:e.item_claim.clone(),item_status:e.item_status.clone(),item_receipt:e.item_receipt.clone(),..outer.clone()}
}
pub(super) fn entry_id(e:&wire::BankBatchEntry)->Option<&str>{
 e.gold_intent.as_ref().and_then(|v|v.operation_id.as_deref()).or_else(||e.gold_status.as_ref().and_then(|v|v.operation_id.as_deref())).or_else(||e.gold_receipt.as_ref().and_then(|v|v.operation_id.as_deref())).or_else(||e.item_deposit.as_ref().and_then(|v|v.operation_id.as_deref())).or_else(||e.item_claim.as_ref().and_then(|v|v.operation_id.as_deref())).or_else(||e.item_status.as_ref().and_then(|v|v.operation_id.as_deref())).or_else(||e.item_receipt.as_ref().and_then(|v|v.operation_id.as_deref()))
}
pub(super) fn correlate(binding:&Binding,request:&wire::Request,response:&wire::Response)->Result<(),String>{
 let Some(batch)=&request.bank_batch else{if response.bank_batch.is_some(){return Err("Unexpected bank batch".into())}return Ok(())};
 if response.error.is_some(){if response.bank_batch.is_some(){return Err("Unexpected bank batch error payload".into())}return Ok(())}
 let expected=batch.entries.as_deref().ok_or("Missing bank batch entries")?;let actual=response.bank_batch.as_ref().and_then(|v|v.entries.as_deref()).ok_or("Missing bank batch results")?;
 if expected.len()!=actual.len(){return Err("Partial uncorrelated bank batch".into())}
 for(index,(e,r))in expected.iter().zip(actual).enumerate(){
  let id=entry_id(e).ok_or("Missing bank original")?;
  if r.index!=Some(index as u32)||r.operation_id.as_deref()!=Some(id){return Err("Bank batch index/original mismatch".into())}
  let item=e.item_deposit.is_some()||e.item_claim.is_some()||e.item_status.is_some()||e.item_receipt.is_some();
  if r.error.is_some(){if r.operation.is_some()||r.item_operation.is_some(){return Err("Bank batch refused payload".into())}continue}
  if item{
   let op=r.item_operation.as_ref().ok_or("Missing batch item operation")?;
   if r.operation.is_some()||op.operation_id.as_deref()!=Some(id)||op.principal_id.as_deref()!=Some(&binding.principal_id)||serde_json::to_vec(&op.original_scope).unwrap()!=serde_json::to_vec(&e.scope).unwrap(){return Err("Bank batch substituted item original".into())}
  }else{
   let op=r.operation.as_ref().ok_or("Missing batch gold operation")?;let scope=e.scope.as_ref().ok_or("Missing batch original scope")?;
   if r.item_operation.is_some()||op.operation_id.as_deref()!=Some(id)||op.principal_id.as_deref()!=Some(&binding.principal_id)||op.run_id!=scope.run_id||op.attachment_id!=scope.attachment_id||op.world_token!=scope.world_token{return Err("Bank batch substituted gold original".into())}
  }
 }
 Ok(())
}
impl HttpOwner{
 pub async fn submit_bank_batch(&mut self,consumer:&mut crate::g3_local_consumer::LocalConsumer,bytes:Vec<u8>)->Result<wire::Response,String>{
  let request=protocol_g3::request(&bytes).map_err(|_|"Invalid bank batch")?;
  for e in request.bank_batch.as_ref().and_then(|v|v.entries.as_ref()).ok_or("Missing bank batch")?{
   let q=single(&request,e);
   if e.item_deposit.is_some()||e.item_claim.is_some(){consumer.validate_item_request(&self.binding,&q)?;}
   if e.gold_intent.is_some(){consumer.validate_gold_request(&self.binding,&q)?;}
  }
  let generation=consumer.generation();let ticket=self.plan(&bytes,Instant::now())?;let result=self.exchange(bytes).await;let response=self.complete(ticket,result,Instant::now())?;
  self.active()?;if consumer.generation()!=generation{return Err("Retired batch consumer response".into())}
  // Validation covers all siblings before accepting even the first one.
  for(e,r)in request.bank_batch.as_ref().unwrap().entries.as_ref().unwrap().iter().zip(response.bank_batch.as_ref().and_then(|v|v.entries.as_ref()).ok_or("Missing bank batch")?){
   if let Some(op)=&r.item_operation{if e.item_deposit.is_some()||e.item_claim.is_some(){consumer.journal.accept_item_operation(op)?;}else{consumer.journal.validate_item_status(op)?;}}
   if let Some(op)=&r.operation{if e.gold_intent.is_some(){consumer.accept_operation(op,generation)?;}else{consumer.journal.validate_status(op)?;}}
  }
  Ok(response)
 }
}
