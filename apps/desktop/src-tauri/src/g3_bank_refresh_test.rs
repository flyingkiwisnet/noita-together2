#[test]fn bank_refresh_capacity_without_original_then_success_and_outage_are_separate(){tauri::async_runtime::block_on(async{
 let(mut c,mut s,dir)=gold_capacity_context();let count=Arc::new(std::sync::atomic::AtomicU64::new(0));let calls=count.clone();
 c.owner.test_transport(move|bytes|{let q=protocol_g3::request(bytes).unwrap();assert!(q.gold_bank.is_some());let n=calls.fetch_add(1,std::sync::atomic::Ordering::AcqRel);
  if n==3{return Err("G3 connection unavailable; retain original IDs".into())}
  let error=matches!(n,0|2).then(||wire::Error{code:Some("CAPACITY".into()),..Default::default()});
  let bank=error.is_none().then(||wire::GoldBankSnapshot{available_balance:Some(19),balance_version:Some("2".into()),send_gold_enabled:Some(true),..Default::default()});
  Ok(serde_json::to_vec(&wire::Response{major:Some(1),revision:Some(protocol_g3::REVISION.into()),service_id:q.service_id,service_epoch:q.service_epoch,request_id:q.request_id,bank,error,..Default::default()}).unwrap())});
 let stream=Arc::new(Mutex::new(Exchange::default()));
 pump_gold(&mut c,&mut s,&stream).await;assert!(s.refresh_status.as_ref().unwrap().contains("refresh busy"));assert!(!s.status.contains("original"));assert!(!s.fresh&&s.balance.is_none());assert_eq!(c.consumer.journal.pending().count(),0);
 s.bank_poll.request_now();pump_gold(&mut c,&mut s,&stream).await;assert!(s.refresh_status.is_none()&&s.fresh);assert_eq!(s.balance,Some(19));assert_eq!(s.status,"Ready.");
 s.cursor="7".into();s.bank_poll.request_now();pump_gold(&mut c,&mut s,&stream).await;assert_eq!(s.cursor,"7","Admission refusal must retain the live notification cursor");assert!(s.fresh,"Admission refusal alone cannot erase a confirmed balance");assert_eq!(s.balance,Some(19));assert!(s.permissions.is_empty()&&s.effect.is_none());
 s.bank_poll.request_now();pump_gold(&mut c,&mut s,&stream).await;assert!(!s.fresh,"Actual refresh connection failure marks the display stale");assert_eq!(s.balance,Some(19),"Useful display retained through outage");
 s.bank_poll.request_now();pump_gold(&mut c,&mut s,&stream).await;assert!(s.refresh_status.is_none()&&s.fresh&&s.permissions.is_empty()&&s.effect.is_none());assert_eq!(c.consumer.journal.pending().count(),0);assert_eq!(count.load(std::sync::atomic::Ordering::Acquire),5);
 gold_capacity_cleanup(c,dir);
});}
#[test]fn bank_mailbox_foreground_has_bounded_fair_background_turns(){let mut passes=0;let turns:Vec<_>=(1..=20).filter(|_|mailbox_background_due(&mut passes,true)).collect();assert_eq!(turns,vec![4,8,12,16,20]);assert!(mailbox_background_due(&mut passes,false));assert_eq!(passes,0);}

#[test]fn bank_publication_rejects_deleted_lobby_and_retired_owner(){let(mut c,_,dir)=gold_capacity_context();let owned=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).ancestors().nth(3).unwrap().join("spikes/noita-entry/.build").join(format!("bank-publication-{}",uuid::Uuid::new_v4()));std::fs::create_dir(&owned).unwrap();let mut consumer=LocalConsumer::new(crate::g3_journal::Journal::open(&owned.join("originals.jsonl")).unwrap());consumer.bind(c.binding.clone()).unwrap();c.consumer=consumer;assert!(c.can_publish());let lobby=c.lobby().unwrap();c.consumer.journal.retire_lobby(crate::g3_journal::DeletedLobby::authenticated(lobby).unwrap()).unwrap();assert!(!c.can_publish(),"A late optional result cannot republish deleted-lobby data");gold_capacity_cleanup(c,dir);std::fs::remove_file(owned.join("originals.jsonl")).unwrap();std::fs::remove_dir(owned).unwrap();
 let(mut c,_,dir)=gold_capacity_context();c.owner.guard(Arc::new(std::sync::atomic::AtomicU64::new(2)),1,Instant::now()+Duration::from_secs(1));assert!(!c.can_publish(),"A retired background owner cannot publish a stale permit");gold_capacity_cleanup(c,dir);}
