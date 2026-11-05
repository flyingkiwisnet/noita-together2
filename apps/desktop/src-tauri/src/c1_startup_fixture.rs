// Synthetic only. Independent server canonical vectors supply persisted delivery digests.
use serde_json::{json,Value};
pub fn mixed_history()->Vec<u8>{
 use sha2::{Digest,Sha256};
 let vectors:Value=serde_json::from_str(include_str!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../packages/contracts/g3-items/operation-digest.json"))).unwrap();
 let mut bytes=Vec::new();let mut sequence=0;
 for revision in ["nt2-items-base-1","nt2-items-base-2"]{
  for terminal in ["Requested","ServerAccepted","EffectIntent","AppliedSession","Uncertain","Refused"]{
   let v=&vectors["vectors"][0];let mut payload=v["original"]["payload"].clone();payload["catalogueRevision"]=json!(revision);
   let canonical=v["canonicalJson"].as_str().unwrap().replace("nt2-items-base-2",revision);
   let digest=Sha256::digest(canonical.as_bytes()).iter().map(|b|format!("{b:02x}")).collect::<String>();
   let original=json!({"service_origin":"https://fixture.invalid","service_id":"startup-fixture","service_epoch":"service-epoch","session_epoch":"game-epoch","principal_id":v["original"]["principalId"],"scope_json":v["original"]["scope"].to_string(),"operation_id":uuid::Uuid::new_v4().to_string(),"direction":"deposit","amount":1,"item":{"kind":"deposit","payload_json":payload.to_string()}});
   let delivery=json!({"delivery_id":uuid::Uuid::new_v4().to_string(),"payload_digest":digest});
   let phases=match terminal{"Requested"=>vec!["Requested"],"Refused"=>vec!["Requested","Refused"],"ServerAccepted"=>vec!["Requested","ServerAccepted"],"EffectIntent"=>vec!["Requested","ServerAccepted","EffectIntent"],end=>vec!["Requested","ServerAccepted","EffectIntent",end]};
   for phase in phases{
    sequence+=1;let delivery=if matches!(phase,"Requested"|"Refused"){Value::Null}else{delivery.clone()};
    // Mix original v1 records and compact v2 transitions, as on the real production path.
    let record=if phase=="Requested"{json!({"version":1,"sequence":sequence,"original":original,"delivery":delivery,"phase":phase})}else{json!({"version":2,"sequence":sequence,"kind":"transition","original_id":original["operation_id"],"delivery":delivery,"phase":phase})};
    serde_json::to_writer(&mut bytes,&record).unwrap();bytes.push(b'\n');
   }
  }
 }
 // Claims are catalogue-independent and remain mixed with both deposit revisions.
 let v=&vectors["vectors"][1];sequence+=1;
 let original=json!({"service_origin":"https://fixture.invalid","service_id":"startup-fixture","service_epoch":"service-epoch","session_epoch":"game-epoch","principal_id":v["original"]["principalId"],"scope_json":v["original"]["scope"].to_string(),"operation_id":uuid::Uuid::new_v4().to_string(),"direction":"claim","amount":1,"item":{"kind":"claim","payload_json":v["original"]["payload"].to_string()}});
 serde_json::to_writer(&mut bytes,&json!({"version":1,"sequence":sequence,"original":original,"delivery":null,"phase":"Requested"})).unwrap();bytes.push(b'\n');bytes
}
