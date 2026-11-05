//! Semantic checks for an already envelope-correlated desktop metadata reply.
use crate::protocol_c1::wire::{Response,SourceOriginal,SourceDisposition,SourceOutcome};
fn skip_reason(reason:&str)->bool{matches!(reason,"sharing_disabled"|"perk_banned"|"orb_owned"|"orb_cap"|"fungal_cooldown"|"fungal_limit"|"fungal_noop"|"gold_world"|"unsupported_content"|"recipient_terminal"|"source_terminal"|"run_ended"|"effect_not_applicable")}
pub(crate) fn validate(original:&SourceOriginal,receipt:Option<&SourceDisposition>,reply:Response)->Result<SourceOutcome,String>{
 if reply.error.is_some()||reply.scope.is_some()||reply.owner_generation.is_some()||reply.snapshot.is_some()||reply.event.is_some()||reply.deliveries.is_some()||reply.duplicate.is_some()||reply.members.is_some()||reply.member_count.is_some()||reply.lives.is_some()||reply.more.is_some()||reply.sequence.is_some(){return Err("C1 source outcome carried unrelated authority".into())}
 let value=reply.source_outcome.ok_or("C1 original outcome absent")?;
 if serde_json::to_value(&value.original).map_err(|_|"C1 outcome encoding")?!=serde_json::to_value(Some(original)).map_err(|_|"C1 original encoding")?
  ||value.effect_authority!=Some(false)||value.self_reported!=Some(true)||value.saved_effect_proof!=Some(false){return Err("C1 source original or authority mismatch".into())}
 match value.outcome.as_deref(){
  Some("unresolved")=>{
   if value.coverage.as_deref()!=Some("absent_expired_or_uncovered")||value.source_disposition.is_some()||value.source_reason.is_some()||value.accepted_sequence.is_some()||value.rules_revision.is_some()||value.duplicate.is_some(){return Err("C1 unresolved outcome contradiction".into())}
  },
  Some("committed")=>{
   let expected_revision=original.publish.as_ref().and_then(|p|p.rules_revision).filter(|r|*r>0).ok_or("C1 original rules revision absent")?;
   let sequence=value.accepted_sequence.as_deref().ok_or("C1 accepted sequence absent")?;
   let number=sequence.parse::<u64>().map_err(|_|"C1 accepted sequence invalid")?;
   let disposition=value.source_disposition.as_deref().ok_or("C1 source disposition absent")?;
   let reason=value.source_reason.as_deref().ok_or("C1 source reason absent")?;
   if value.coverage.as_deref()!=Some("exact_durable_record")||number==0||number.to_string()!=sequence||value.rules_revision!=Some(expected_revision)||value.duplicate.is_none()
    ||!matches!(disposition,"pending"|"applied"|"skipped"|"uncertain")||(if disposition=="skipped"{!skip_reason(reason)}else{!reason.is_empty()})
    ||disposition=="pending"&&value.duplicate==Some(true){return Err("C1 committed outcome contradiction".into())}
   if let Some(receipt)=receipt{
    let requested=receipt.state.as_deref().ok_or("C1 source receipt state absent")?;
    if !matches!(requested,"applied"|"skipped"|"uncertain")||disposition!=requested||reason!=receipt.reason.as_deref().unwrap_or(""){return Err("C1 source receipt outcome mismatch".into())}
   }
  },_=>return Err("C1 source outcome absent or unsupported".into())
 }
 Ok(value)
}

#[cfg(test)]mod tests{
 use super::*;use serde_json::{json,Value};
 fn original()->SourceOriginal{serde_json::from_value(json!({"roomId":"11111111-1111-4111-8111-111111111111","runId":"22222222-2222-4222-8222-222222222222","worldToken":"33333333-3333-4333-8333-333333333333","configRevision":3,"publish":{"eventId":"44444444-4444-4444-8444-444444444444","rulesRevision":7,"payload":{"death":{"state":"terminal","cause":"normal"}},"senderDisposition":"pending"}})).unwrap()}
 fn committed()->Value{json!({"major":1,"revision":crate::protocol_c1::REVISION,"serviceId":"service","serviceEpoch":"epoch","requestId":"55555555-5555-4555-8555-555555555555","sourceOutcome":{"original":original(),"outcome":"committed","coverage":"exact_durable_record","sourceDisposition":"applied","sourceReason":"","acceptedSequence":"19","rulesRevision":7,"effectAuthority":false,"selfReported":true,"savedEffectProof":false,"duplicate":false}})}
 fn unresolved()->Value{let mut v=committed();v["sourceOutcome"]=json!({"original":original(),"outcome":"unresolved","coverage":"absent_expired_or_uncovered","effectAuthority":false,"selfReported":true,"savedEffectProof":false});v}
 fn receipt(state:&str,reason:Option<&str>)->SourceDisposition{SourceDisposition{state:Some(state.into()),reason:reason.map(str::to_owned)}}
 fn checked(v:&Value,r:Option<&SourceDisposition>)->Result<SourceOutcome,String>{let reply=crate::protocol_c1::response(&serde_json::to_vec(v).unwrap()).map_err(|_|"C1 test canonical response")?;validate(&original(),r,reply)}
 fn malformed()->Vec<Value>{
  let mut all=Vec::new();
  for key in ["original","outcome","coverage","sourceDisposition","sourceReason","acceptedSequence","rulesRevision","effectAuthority","selfReported","savedEffectProof","duplicate"]{let mut v=committed();v["sourceOutcome"].as_object_mut().unwrap().remove(key);all.push(v);}
  for(key,value)in [("effectAuthority",json!(true)),("selfReported",json!(false)),("savedEffectProof",json!(true)),("coverage",json!("absent_expired_or_uncovered")),("rulesRevision",json!(8)),("acceptedSequence",json!("0")),("acceptedSequence",json!("019")),("acceptedSequence",json!("18446744073709551616")),("sourceReason",json!("source_terminal")),("outcome",json!("unresolved"))]{let mut v=committed();v["sourceOutcome"][key]=value;all.push(v);}
  for(key,value)in [("roomId",json!("99999999-9999-4999-8999-999999999999")),("runId",json!("99999999-9999-4999-8999-999999999999")),("worldToken",json!("99999999-9999-4999-8999-999999999999")),("configRevision",json!(4))]{let mut v=committed();v["sourceOutcome"]["original"][key]=value;all.push(v);}
  for(key,value)in [("eventId",json!("99999999-9999-4999-8999-999999999999")),("rulesRevision",json!(8)),("senderDisposition",json!("applied")),("payload",json!({"death":{"state":"won","cause":"normal"}}))]{let mut v=committed();v["sourceOutcome"]["original"]["publish"][key]=value;all.push(v);}
  for(key,value)in [("scope",json!({})),("ownerGeneration",json!(1)),("snapshot",json!({})),("event",json!({})),("deliveries",json!([])),("duplicate",json!(false)),("members",json!([])),("memberCount",json!(0)),("lives",json!([])),("more",json!(false)),("sequence",json!("1"))]{let mut v=committed();v[key]=value;all.push(v);}
  for(key,value)in [("sourceDisposition",json!("pending")),("sourceReason",json!("")),("acceptedSequence",json!("19")),("rulesRevision",json!(7)),("duplicate",json!(false))]{let mut v=unresolved();v["sourceOutcome"][key]=value;all.push(v);}
  let mut v=unresolved();v["sourceOutcome"]["coverage"]=json!("exact_durable_record");all.push(v);
  all
 }
 #[test]fn c1_source_outcome_accepts_exact_committed_and_non_authoritative_unresolved(){
  for state in ["pending","applied","skipped","uncertain"]{let mut v=committed();v["sourceOutcome"]["sourceDisposition"]=json!(state);v["sourceOutcome"]["sourceReason"]=json!(if state=="skipped"{"effect_not_applicable"}else{""});assert!(checked(&v,None).is_ok());if state!="pending"{assert!(checked(&v,Some(&receipt(state,if state=="skipped"{Some("effect_not_applicable")}else{None}))).is_ok());}}
  for r in [None,Some(receipt("applied",None))]{let v=checked(&unresolved(),r.as_ref()).unwrap();assert_eq!(v.effect_authority,Some(false));assert_eq!(v.outcome.as_deref(),Some("unresolved"));}
 }
 #[test]fn c1_source_outcome_rejects_malformed_but_correlated_canonical_replies(){
  for v in malformed(){assert!(crate::protocol_c1::response(&serde_json::to_vec(&v).unwrap()).is_ok(),"Test must reach semantics: {v}");assert!(checked(&v,None).is_err(),"Unexpected semantic acceptance: {v}");}
  let mut v=committed();v["sourceOutcome"]["sourceDisposition"]=json!("pending");v["sourceOutcome"]["duplicate"]=json!(true);assert!(checked(&v,None).is_err());
 }
 #[test]fn c1_source_outcome_receipt_requires_exact_returned_disposition_and_reason(){
  for r in [receipt("uncertain",None),receipt("skipped",Some("effect_not_applicable"))]{assert!(checked(&committed(),Some(&r)).is_err());}
  let mut v=committed();v["sourceOutcome"]["sourceDisposition"]=json!("pending");assert!(checked(&v,Some(&receipt("applied",None))).is_err());
  v["sourceOutcome"]["sourceDisposition"]=json!("skipped");v["sourceOutcome"]["sourceReason"]=json!("source_terminal");assert!(checked(&v,Some(&receipt("skipped",Some("effect_not_applicable")))).is_err());
  v["sourceOutcome"]["sourceReason"]=json!("arbitrary");assert!(checked(&v,None).is_err());
 }
 fn retained(state:&str)->(std::path::PathBuf,crate::c1_journal::Journal,crate::c1_journal::Record){
  let root=std::env::temp_dir().join(format!("nt2-c1-source-semantic-{}",uuid::Uuid::new_v4()));std::fs::create_dir(&root).unwrap();let mut journal=crate::c1_journal::Journal::open(&root.join("c1-effects.jsonl")).unwrap();let source=serde_json::to_value(original()).unwrap();
  let row=crate::c1_journal::Record{ordinal:0,source:Some(source.clone()),lobby:crate::g3_journal::LobbyIdentity{service_origin:"https://fixture.invalid".into(),service_id:"service".into(),principal_id:"principal".into(),room_id:original().room_id.unwrap()},world:"a".repeat(64),id:original().publish.unwrap().event_id.unwrap(),sequence:"0".into(),state:state.into(),reason:"applied".into(),original:source["publish"].clone()};journal.put(row.clone()).unwrap();(root,journal,row)
 }
 #[test]fn c1_source_outcome_malformed_replies_never_remove_pending_or_uncertain_original(){
  for state in ["pending","uncertain"]{let(root,mut journal,row)=retained(state);for v in malformed(){assert!(crate::c1_work::finish_retired_source(&mut journal,row.clone(),checked(&v,None)).is_err());let kept=journal.get(&row.world,&row.id,"0").unwrap();assert_eq!(kept.state,state);assert_eq!(kept.original,row.original);assert_eq!(kept.source,row.source);}drop(journal);std::fs::remove_dir_all(root).unwrap();}
 }
 #[test]fn c1_source_outcome_validated_final_results_control_durable_acknowledgement(){
  use crate::c1_work::{finish_retired_source,RetiredDisposition};
  for(kind,expected)in [("committed",RetiredDisposition::Acknowledged),("unresolved",RetiredDisposition::Unresolved),("contradiction",RetiredDisposition::Uncertain)]{
   let(root,mut journal,row)=retained("pending");let mut v=if kind=="unresolved"{unresolved()}else{committed()};if kind=="contradiction"{v["sourceOutcome"]["sourceDisposition"]=json!("uncertain");}
   let requested=receipt("applied",None);let result=checked(&v,if kind=="contradiction"{None}else{Some(&requested)});assert_eq!(finish_retired_source(&mut journal,row.clone(),result).unwrap(),expected);
   let kept=journal.get(&row.world,&row.id,"0");match expected{RetiredDisposition::Acknowledged=>assert!(kept.is_none()),RetiredDisposition::Unresolved=>assert_eq!(kept.unwrap().state,"unresolved"),RetiredDisposition::Uncertain=>assert_eq!(kept.unwrap().state,"uncertain")};drop(journal);std::fs::remove_dir_all(root).unwrap();
  }
 }
}
