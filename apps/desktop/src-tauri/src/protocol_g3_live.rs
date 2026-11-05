//! Canonical generated DTOs and bounded strict protobuf binary codec for C2 WSS.
pub const REVISION:&str="g3-live-cr2-4";
pub const ENDPOINT:&str="/v3/live";
pub const SUBPROTOCOL:&str="nt2.g3.live.v1";
pub mod wire{include!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../packages/contracts/g3-live/g3-live.rs"));}
use serde::de::DeserializeOwned;
use serde_json::Value;
static SCHEMA:std::sync::LazyLock<Value>=std::sync::LazyLock::new(||serde_json::from_str(include_str!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../packages/contracts/g3-live/schema.json"))).unwrap());
include!("protocol_live_codec.rs");
pub fn encode<T:serde::Serialize>(name:&str,value:&T)->Result<Vec<u8>,ProtocolError>{
 let value=serde_json::to_value(value).map_err(|_|ProtocolError::InvalidMessage)?;validate(name,&value,0,&mut 0)?;
 fn variable(mut n:u32,out:&mut Vec<u8>){while n>=128{out.push((n as u8&127)|128);n>>=7;}out.push(n as u8);}
 fn message(name:&str,value:&Value,out:&mut Vec<u8>)->Result<(),ProtocolError>{
  let mut fields=SCHEMA["messages"][name]["fields"].as_object().ok_or(ProtocolError::InvalidMessage)?.iter().collect::<Vec<_>>();fields.sort_by_key(|(_,f)|f["id"].as_u64().unwrap());
  for (key,f) in fields{let Some(v)=value.get(key)else{continue};let values=if f["repeated"]==true{v.as_array().unwrap().iter().collect::<Vec<_>>()}else{vec![v]};
   for v in values{let kind=f["type"].as_str().unwrap();let tag=(f["id"].as_u64().unwrap() as u32)<<3;
    if matches!(kind,"bool"|"uint32"){variable(tag,out);variable(if kind=="bool"{u32::from(v.as_bool().unwrap())}else{v.as_u64().unwrap() as u32},out);}
    else{let mut body=Vec::new();if kind=="string"{body.extend_from_slice(v.as_str().unwrap().as_bytes());}else{message(kind,v,&mut body)?;}variable(tag|2,out);variable(body.len() as u32,out);out.extend(body);}
   }
  }Ok(())
 }
 let mut bytes=Vec::new();message(name,&value,&mut bytes)?;if bytes.len()>32768{return Err(ProtocolError::TooLarge);}Ok(bytes)
}
pub fn decode<T:DeserializeOwned>(name:&str,bytes:&[u8])->Result<T,ProtocolError>{serde_json::from_value(binary_value(name,bytes)?).map_err(|_|ProtocolError::InvalidMessage)}
#[cfg(test)]mod tests{
 use super::*;use sha2::{Digest,Sha256};
 #[test]fn c2_canonical_negative_and_binary_boundaries(){
  let negatives:Value=serde_json::from_str(include_str!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../packages/contracts/g3-live/negative-inputs.json"))).unwrap();
  for n in negatives.as_array().unwrap(){assert!(encode(n["type"].as_str().unwrap(),&n["json"]).is_err());}
  assert!(binary_value("ClientFrame",&vec![0;32769]).is_err());
  let golden:Value=serde_json::from_str(include_str!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../packages/contracts/g3-live/golden.json"))).unwrap();
  let mut bytes=encode("ClientFrame",&golden["fixtures"][0]["json"]).unwrap();
  let original=bytes.clone();bytes.extend_from_slice(&[8,1]);assert!(binary_value("ClientFrame",&bytes).is_err()); // duplicate singular
  bytes=original.clone();bytes.extend_from_slice(&[0xf8,0x07,1]);assert!(binary_value("ClientFrame",&bytes).is_err()); // unknown field
  bytes=original;bytes[1]=0x81;bytes.insert(2,0);assert!(binary_value("ClientFrame",&bytes).is_err()); // overlong varint
  assert!(json_value("ClientFrame",br#"{"major":1,"major":1}"#).is_err());
 }
 #[test]fn canonical_c1_live_dtos_and_pins(){
  let schema=include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../packages/contracts/g3-live/schema.json"));
  let hex=Sha256::digest(schema).iter().map(|b|format!("{b:02x}")).collect::<String>();
  assert_eq!(hex,"f8edcfb8714f6115c3097faa5bef2b73c89bb2c9196bf158bfaf5ef51b9bf97d");
  let golden:serde_json::Value=serde_json::from_slice(include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../packages/contracts/g3-live/golden.json"))).unwrap();
  assert_eq!(golden["revision"],REVISION);
  for fixture in golden["fixtures"].as_array().unwrap(){
   let value=&fixture["json"];let round=match fixture["type"].as_str().unwrap(){
    "ClientFrame"=>serde_json::to_value(serde_json::from_value::<wire::ClientFrame>(value.clone()).unwrap()).unwrap(),
    "ServerFrame"=>serde_json::to_value(serde_json::from_value::<wire::ServerFrame>(value.clone()).unwrap()).unwrap(),
    _=>panic!("Unexpected canonical fixture type")};assert_eq!(&round,value);
   let bytes=fixture["hex"].as_str().unwrap().as_bytes().chunks_exact(2).map(|v|u8::from_str_radix(std::str::from_utf8(v).unwrap(),16).unwrap()).collect::<Vec<_>>();
   assert_eq!(binary_value(fixture["type"].as_str().unwrap(),&bytes).unwrap(),*value);
   assert_eq!(encode(fixture["type"].as_str().unwrap(),value).unwrap(),bytes);
  }
  let mut bad=golden["fixtures"][0]["json"].clone();bad["inventedTransportField"]=serde_json::json!(true);
  assert!(serde_json::from_value::<wire::ClientFrame>(bad).is_err());
 }
}
