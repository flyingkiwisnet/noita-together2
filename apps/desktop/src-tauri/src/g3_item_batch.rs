// Bounded compatible stages. Local effects and flushed originals stay individual.
fn batch_entry(q:wire::Request)->wire::BankBatchEntry{
 wire::BankBatchEntry{scope:q.scope,current_scope:q.current_scope,gold_intent:q.gold_intent,gold_status:q.gold_status,gold_receipt:q.gold_receipt,item_deposit:q.item_deposit,item_claim:q.item_claim,item_status:q.item_status,item_receipt:q.item_receipt}
}
async fn flush_intents(c:&mut Context,s:&mut State){
 let mut request=c.request();let mut selected=Vec::new();let mut entries=Vec::new();
 for(id,bytes)in &s.intents{
  let Ok(q)=protocol_g3::request(bytes)else{continue};let entry=batch_entry(q);let mut trial=entries.clone();trial.push(entry.clone());request.bank_batch=Some(wire::BankBatch{entries:Some(trial)});
  if serde_json::to_vec(&request).unwrap().len()>196608{break}
  entries.push(entry);selected.push((id.clone(),bytes.clone()));if entries.len()==4{break}
 }
 if selected.is_empty(){return}
 request.bank_batch=Some(wire::BankBatch{entries:Some(entries)});
 let response=c.owner.submit_bank_batch(&mut c.consumer,serde_json::to_vec(&request).unwrap()).await;
 let result=match response{Ok(response)=>Ok(response.bank_batch.unwrap().entries.unwrap()),Err(error)=>Err(error)};
 // Transport ambiguity leaves the flushed original for the sole recovery owner.
 s.submissions.clear();
 for(index,(id,bytes))in selected.into_iter().enumerate(){
  s.intents.remove(&id);
  let outcome=match &result{
   Ok(results)=>{let r=&results[index];if r.error.as_ref().and_then(|e|e.code.as_deref())==Some("BATCH_TOO_LARGE"){
    // Savepoint rolled this sibling back. Retry the SAME flushed request once.
    c.owner.submit_item(&mut c.consumer,bytes).await
   }else if let Some(error)=&r.error{Err(error.code.clone().unwrap_or("SERVICE_UNAVAILABLE".into()))}
   else{Ok(wire::Response{item_operation:r.item_operation.clone(),..Default::default()})}},
   Err(error)=>Err(error.clone()),
  };
  match intent_result(c,s,&id,"deposit",outcome){
   Ok(response)=>{if let Some(op)=response.item_operation{recovered_submission(s,&op);s.submissions.push(json!({"id":id,"status":"accepted","direction":"deposit"}));s.operations.insert(id,op);s.cache.request();s.page_schedule.request_now();}},
   Err(error)=>{let refused=c.consumer.journal.pending().find(|p|p.record.original.operation_id==id).is_some_and(|p|p.record.phase==Phase::Refused);s.submissions.push(json!({"id":id,"status":if refused{"refused"}else{"unresolved"},"direction":"deposit"}));s.foreground_error(&id,&error);}
  }
 }
 s.intent_due=None;
}
async fn recover_statuses(c:&mut Context,s:&mut State,pending:&[&crate::g3_journal::Pending]){
 let mut request=c.request();let mut selected=Vec::new();
 for offset in 0..pending.len().min(4){
  let p=pending[(s.scan+offset)%pending.len()];let id=&p.record.original.operation_id;
  if let Ok(mut q)=c.original(id){q.item_status=Some(wire::ItemStatus{operation_id:Some(id.clone())});selected.push((p,batch_entry(q)));}
 }
 if selected.is_empty(){return}s.scan=(s.scan+selected.len())%pending.len();
 request.bank_batch=Some(wire::BankBatch{entries:Some(selected.iter().map(|(_,e)|e.clone()).collect())});
 let response=c.call(request).await;s.recovery_schedule.complete(Instant::now(),response.is_ok(),600);
 match response{
  Ok(response)=>for((p,_),r)in selected.into_iter().zip(response.bank_batch.unwrap().entries.unwrap()){
   let id=&p.record.original.operation_id;
   if let Some(op)=r.item_operation{if c.consumer.journal.validate_item_status(&op).is_ok(){recovered_submission(s,&op);s.operations.insert(id.clone(),op);}}
   else if let Some(error)=r.error{match error.code.as_deref(){
    Some("BATCH_TOO_LARGE")=>{if let Err(error)=status(c,s,id).await{s.foreground_error(&id,&error);}},
    Some("NOT_FOUND")=>{if let Err(error)=retry_unissued_with(c,s,p,"NOT_FOUND",|owner,consumer,req|Box::pin(owner.submit_item(consumer,req))).await{s.foreground_error(&id,&error);}},
    Some(error)=>s.foreground_error(id,error),None=>s.foreground_error(id,"SERVICE_UNAVAILABLE"),
   }}
  },
  Err(error)=>{for(p,_)in &selected{s.foreground_error(&p.record.original.operation_id,&error);}},
 }
}
