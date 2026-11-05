#[test]fn batch_correlates_every_index_original_owner_and_individual_refusal(){
 let o=owner();let mut q=request(&o);q.ghost_list=None;
 let ids=[uuid::Uuid::new_v4().to_string(),uuid::Uuid::new_v4().to_string()];
 q.bank_batch=Some(wire::BankBatch{entries:Some(ids.iter().map(|id|wire::BankBatchEntry{scope:q.scope.clone(),item_status:Some(wire::ItemStatus{operation_id:Some(id.clone())}),..Default::default()}).collect())});
 let good=wire::BankBatchEntryResult{index:Some(0),operation_id:Some(ids[0].clone()),item_operation:Some(wire::ItemOperation{operation_id:Some(ids[0].clone()),principal_id:Some(o.binding.principal_id.clone()),original_scope:q.scope.clone(),..Default::default()}),..Default::default()};
 let refused=wire::BankBatchEntryResult{index:Some(1),operation_id:Some(ids[1].clone()),error:Some(wire::Error{code:Some("CLAIM_LOST".into()),..Default::default()}),..Default::default()};
 let mut r=wire::Response{bank_batch:Some(wire::BankBatchResult{entries:Some(vec![good,refused])}),..Default::default()};
 assert!(batch::correlate(&o.binding,&q,&r).is_ok());
 for fault in 0..5{let mut bad=r.clone();let entries=bad.bank_batch.as_mut().unwrap().entries.as_mut().unwrap();match fault{
  0=>entries.swap(0,1),1=>entries[0].operation_id=Some(ids[1].clone()),2=>entries[0].item_operation.as_mut().unwrap().principal_id=Some("other".into()),3=>entries[0].item_operation.as_mut().unwrap().original_scope.as_mut().unwrap().world_token=Some("other".into()),_=>{entries.pop();}
 }assert!(batch::correlate(&o.binding,&q,&bad).is_err(),"Every sibling validated before journal writes");}
 r.bank_batch.as_mut().unwrap().entries.as_mut().unwrap()[1].error.as_mut().unwrap().code=Some("BATCH_TOO_LARGE".into());assert!(batch::correlate(&o.binding,&q,&r).is_ok(),"Oversize sibling has independent same-original fallback");
}
#[test]fn batch_plan_rejects_cross_world_and_counts_each_entry(){
 let mut o=owner();let mut q=request(&o);q.ghost_list=None;let entry=wire::BankBatchEntry{scope:q.scope.clone(),item_status:Some(wire::ItemStatus{operation_id:Some(uuid::Uuid::new_v4().to_string())}),..Default::default()};
 q.bank_batch=Some(wire::BankBatch{entries:Some(vec![entry;4])});let now=Instant::now();let bytes=serde_json::to_vec(&q).unwrap();let ticket=o.plan(&bytes,now).unwrap();assert_eq!(o.count,4);o.in_flight=None;drop(ticket);
 q.bank_batch.as_mut().unwrap().entries.as_mut().unwrap()[2].scope.as_mut().unwrap().run_id=Some("other".into());assert!(o.plan(&serde_json::to_vec(&q).unwrap(),now).is_err());
}
