#[test]fn grouped_deposits_flush_first_keep_siblings_and_same_original_oversize_fallback(){tauri::async_runtime::block_on(async{
 let(mut c,mut s,dir,_,_)=claim_context();let payload=json!({"catalogueRevision":"nt2-items-base-2","spell":{"actionId":"LIGHT_BULLET","usesRemaining":"-1"}});
 for _ in 0..4{submit(&mut c,&mut s,uuid::Uuid::new_v4().to_string(),"deposit",payload.clone()).await.unwrap();}
 assert_eq!(s.intents.len(),4);assert_eq!(c.consumer.journal.pending().filter(|p|p.record.phase==Phase::Requested).count(),5);
 let mut ops=BTreeMap::new();for p in c.consumer.journal.pending().filter(|p|p.record.original.direction=="deposit"){
  let o=&p.record.original;ops.insert(o.operation_id.clone(),wire::ItemOperation{operation_id:Some(o.operation_id.clone()),principal_id:Some(o.principal_id.clone()),original_scope:Some(c.binding.scope.clone()),direction:Some("deposit".into()),item_id:Some(uuid::Uuid::new_v4().to_string()),delivery_id:Some(uuid::Uuid::new_v4().to_string()),payload_digest:Some(crate::g3_journal::operation_digest(o).unwrap()),state:Some("awaiting_local".into()),item:Some(serde_json::from_value(payload.clone()).unwrap()),effect_allowed:Some(true),save_persistence:Some("unproven".into()),bank_revision:Some("4".into()),..Default::default()});
 }
 let seen=Arc::new(Mutex::new(Vec::new()));let observed=seen.clone();c.owner.test_transport(move|bytes|{
  let q=protocol_g3::request(bytes).unwrap();let mut response=wire::Response{major:Some(1),revision:Some(protocol_g3::REVISION.into()),service_id:q.service_id.clone(),service_epoch:q.service_epoch.clone(),request_id:q.request_id.clone(),bank_revision:Some("4".into()),..Default::default()};
  if let Some(batch)=&q.bank_batch{let entries=batch.entries.as_ref().unwrap();assert_eq!(entries.len(),4);let mut results=Vec::new();
   for(index,e)in entries.iter().enumerate(){let id=e.item_deposit.as_ref().unwrap().operation_id.clone().unwrap();observed.lock().unwrap().push(id.clone());let code=match index{1=>Some("FORBIDDEN"),2=>Some("BATCH_TOO_LARGE"),_=>None};results.push(wire::BankBatchEntryResult{index:Some(index as u32),operation_id:Some(id.clone()),item_operation:code.is_none().then(||ops[&id].clone()),error:code.map(|code|wire::Error{code:Some(code.into()),..Default::default()}),..Default::default()});}
   response.bank_batch=Some(wire::BankBatchResult{entries:Some(results)});
  }else{let id=q.item_deposit.as_ref().unwrap().operation_id.as_ref().unwrap();observed.lock().unwrap().push(id.clone());response.item_operation=Some(ops[id].clone());}
  Ok(serde_json::to_vec(&response).unwrap())
 });
 flush_intents(&mut c,&mut s).await;assert!(s.intents.is_empty());assert_eq!(s.submissions.iter().filter(|v|v["status"]=="accepted").count(),3);assert_eq!(s.submissions.iter().filter(|v|v["status"]=="refused").count(),1);
 let calls=seen.lock().unwrap();assert_eq!(calls.len(),5);assert_eq!(calls[2],calls[4],"Oversize uses exact same flushed original");drop(calls);
 assert_eq!(c.consumer.journal.pending().filter(|p|p.record.phase==Phase::ServerAccepted).count(),3);assert_eq!(c.consumer.journal.pending().filter(|p|p.record.phase==Phase::Refused).count(),1);assert!(s.permissions.is_empty()&&s.effect.is_none());
 drop(c);std::fs::remove_file(dir.join("originals.jsonl")).unwrap();std::fs::remove_dir(dir).unwrap();
});}
#[test]fn full_copy_queue_keeps_latest_confirmed_page_without_losing_invalidation(){
 let mut s=State::default();for i in 0..4{s.queue(i.to_string(),"detail",json!({"value":i})).unwrap();}
 accept_page(&mut s,wire::ItemBankSnapshot{revision:Some("3".into()),items:Some(vec![]),..Default::default()});assert!(s.bank.is_some()&&s.pending_bank.is_some());
 accept_page(&mut s,wire::ItemBankSnapshot{revision:Some("4".into()),items:Some(vec![]),..Default::default()});assert_eq!(s.pending_bank.as_ref().unwrap().1["revision"],"4");s.outgoing.pop_front();queue_cached_page(&mut s);assert!(s.pending_bank.is_none());assert_eq!(s.outgoing.len(),4);
}
#[test]fn grouped_status_turn_releases_receipt_streak_even_when_transport_fails(){tauri::async_runtime::block_on(async{
 let(mut c,mut s,dir,_,_)=claim_context();c.consumer.begin_item_with_id(crate::g3_items::Original{kind:"claim".into(),payload_json:json!({"itemId":uuid::Uuid::new_v4().to_string(),"expectedRevision":"2"}).to_string()},c.consumer.generation(),uuid::Uuid::new_v4().to_string()).unwrap();
 c.owner.test_transport(|bytes|{assert_eq!(protocol_g3::request(bytes).unwrap().bank_batch.unwrap().entries.unwrap().len(),2);Err("CAPACITY".into())});
 s.recovery_streak=3;s.page_schedule.next=Some(Instant::now()+Duration::from_secs(60));pump(&mut c,&mut s).await;assert_eq!(s.recovery_streak,0,"A failed grouped status turn still releases receipt priority");assert!(s.permissions.is_empty());
 drop(c);std::fs::remove_file(dir.join("originals.jsonl")).unwrap();std::fs::remove_dir(dir).unwrap();
});}
