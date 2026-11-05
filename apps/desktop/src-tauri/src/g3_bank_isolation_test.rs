// Real journal/owner scheduler with controlled transport; engine evidence is separate.
#[test]fn foreign_world_originals_never_enter_bank_workers_or_current_view(){tauri::async_runtime::block_on(async{
 let(mut c,mut s,dir)=gold_capacity_context();c.items=true;
 let gold=uuid::Uuid::new_v4().to_string();c.consumer.begin_gold_with_id("deposit",10,c.consumer.generation(),gold.clone()).unwrap();
 let item=uuid::Uuid::new_v4().to_string();c.consumer.begin_item_with_id(crate::g3_items::Original{kind:"claim".into(),payload_json:json!({"itemId":uuid::Uuid::new_v4().to_string(),"expectedRevision":"2"}).to_string()},c.consumer.generation(),item.clone()).unwrap();
 let original=c.binding.clone();s.world=world_key(&original);s.status="previous lobby warning".into();
 let work=Work{state:s,..Default::default()};assert!(!work.view_for(Some(&original)).is_null());assert!(work.view_for(None).is_null());
 for field in 0..5{let mut other=original.clone();match field{0=>other.scope.room_id=Some(uuid::Uuid::new_v4().to_string()),1=>other.scope.run_id=Some(uuid::Uuid::new_v4().to_string()),2=>other.scope.run_epoch=Some(uuid::Uuid::new_v4().to_string()),3=>other.scope.config_revision=Some(2),_=>other.scope.world_token=Some(uuid::Uuid::new_v4().to_string())};assert!(work.view_for(Some(&other)).is_null());c.binding=other;assert!(c.original(&gold).is_err()&&c.original(&item).is_err());}
 let calls=Arc::new(std::sync::atomic::AtomicU64::new(0));let observed=calls.clone();c.owner.test_transport(move|_|{observed.fetch_add(1,std::sync::atomic::Ordering::AcqRel);Err("Unexpected foreign recovery".into())});
 let mut s=State::default();s.cache.accept(0);s.bank_poll.next=Some(Instant::now()+Duration::from_secs(60));
 let stream=Arc::new(Mutex::new(Exchange::default()));for _ in 0..20{pump_gold(&mut c,&mut s,&stream).await;}
 assert_eq!(calls.load(std::sync::atomic::Ordering::Acquire),0);assert!(s.operations.is_empty());assert!(s.status.is_empty());assert!(s.recovery_poll.next.is_none());
 // Attachment renewal in the same world keeps the original status route.
 c.binding=original;c.binding.scope.attachment_id=Some(uuid::Uuid::new_v4().to_string());let req=c.original(&gold).unwrap();assert_ne!(req.scope.unwrap().attachment_id,req.current_scope.unwrap().attachment_id);assert!(c.original(&item).is_ok());
 assert_eq!(c.consumer.journal.pending().count(),2);gold_capacity_cleanup(c,dir);
});}

#[test]fn retained_gold_read_is_slow_readonly_and_cannot_replace_success(){tauri::async_runtime::block_on(async{
 let(mut c,mut s,dir)=gold_capacity_context();let id=uuid::Uuid::new_v4().to_string();c.consumer.begin_gold_with_id("deposit",10,c.consumer.generation(),id.clone()).unwrap();
 let Context{binding,consumer,owner,events,items,item_players}=c;drop(consumer);let mut consumer=LocalConsumer::new(crate::g3_journal::Journal::open(&dir.join("originals.jsonl")).unwrap());consumer.bind(binding.clone()).unwrap();let mut c=Context{binding,consumer,owner,events,items,item_players};
 let calls=Arc::new(std::sync::atomic::AtomicU64::new(0));let observed=calls.clone();let journal=c.consumer.journal.clone();c.owner.test_transport(move|bytes|{let q=protocol_g3::request(bytes).unwrap();assert!(q.gold_status.is_some()&&q.gold_intent.is_none()&&q.gold_receipt.is_none());observed.fetch_add(1,std::sync::atomic::Ordering::AcqRel);Ok(gold_capacity_reply(bytes,&journal,None))});
 s.status="Gold transfer completed in this session.".into();s.cache.accept(0);s.bank_poll.next=Some(Instant::now()+Duration::from_secs(60));let stream=Arc::new(Mutex::new(Exchange::default()));for _ in 0..20{pump_gold(&mut c,&mut s,&stream).await;}
 assert_eq!(calls.load(std::sync::atomic::Ordering::Acquire),1);assert_eq!(s.status,"Gold transfer completed in this session.");assert!(s.permissions.is_empty());assert!(s.effect.as_ref().is_none_or(|v|v["stage"]=="offer"),"Read metadata is never an effect permission");assert_eq!(c.consumer.journal.pending().next().unwrap().record.phase,Phase::Requested);gold_capacity_cleanup(c,dir);
});}
