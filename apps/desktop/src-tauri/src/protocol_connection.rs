//! Server-owned canonical connection codec, no independent wire definitions.
pub const REVISION:&str="g3-connection-cr2-1";
pub const ENDPOINT:&str="/v3/connection/rpc";
pub mod wire{include!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../packages/contracts/g3-connection/g3-connection.rs"));}
static SCHEMA:std::sync::LazyLock<serde_json::Value>=std::sync::LazyLock::new(||serde_json::from_str(include_str!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../packages/contracts/g3-connection/schema.json"))).unwrap());
pub fn validate<T:serde::Serialize>(name:&str,value:&T)->Result<(),String>{
 fn visit(name:&str,value:&serde_json::Value,depth:u64,fields:&mut usize)->Result<(),String>{
  let bad=||"Invalid canonical connection shape/bounds".to_string();let object=value.as_object().ok_or_else(bad)?;let definition=SCHEMA["messages"][name]["fields"].as_object().ok_or_else(bad)?;*fields=fields.checked_add(object.len()).ok_or_else(bad)?;
  if depth>SCHEMA["limits"]["depth"].as_u64().unwrap()||*fields>SCHEMA["limits"]["fields"].as_u64().unwrap() as usize{return Err(bad());}
  for (key,field) in definition{if field["required"]==true&&!object.contains_key(key){return Err(bad());}}
  for (key,item) in object{let field=definition.get(key).ok_or_else(bad)?;let kind=field["type"].as_str().ok_or_else(bad)?;let check=|v:&serde_json::Value,fields:&mut usize,max:Option<u64>|match kind{
   "string"=>if v.as_str().is_some_and(|s|s.len() as u64<=max.unwrap_or(0)){Ok(())}else{Err(bad())},
   "uint32"=>if v.as_u64().is_some_and(|n|n<=field["maxValue"].as_u64().unwrap_or(u64::from(u32::MAX))&&n>=field["minValue"].as_u64().unwrap_or(0)){Ok(())}else{Err(bad())},
   "bool"=>if v.is_boolean(){Ok(())}else{Err(bad())},_=>visit(kind,v,depth+1,fields)};
   if field["repeated"]==true{let values=item.as_array().ok_or_else(bad)?;if values.len() as u64>field["max"].as_u64().unwrap_or(0){return Err(bad());}for value in values{check(value,fields,field["itemMax"].as_u64())?;}}else{check(item,fields,field["max"].as_u64())?;}
  }
  if let Some(oneof)=SCHEMA["messages"][name]["oneof"].as_array(){if oneof.iter().filter(|key|object.contains_key(key.as_str().unwrap())).count()!=1{return Err(bad());}}Ok(())
 }
 let value=serde_json::to_value(value).map_err(|_|"Connection value encoding failed")?;visit(name,&value,0,&mut 0)
}
pub fn response(bytes:&[u8])->Result<wire::Response,String>{
 if bytes.len()>65536{return Err("Connection response exceeds bound".into());}
 let r:wire::Response=serde_json::from_slice(bytes).map_err(|_|"Invalid canonical connection response")?;
 if r.major!=Some(1)||r.revision.as_deref()!=Some(REVISION){return Err("Connection revision changed".into());}validate("Response",&r)?;Ok(r)
}
pub fn canonical_fingerprint(bytes:&[u8])->Result<String,String>{
 fn sorted(v:serde_json::Value)->serde_json::Value{match v{serde_json::Value::Object(m)=>{let mut pairs:Vec<_>=m.into_iter().collect();pairs.sort_by(|a,b|a.0.cmp(&b.0));serde_json::Value::Object(pairs.into_iter().map(|(k,v)|(k,sorted(v))).collect())},serde_json::Value::Array(a)=>serde_json::Value::Array(a.into_iter().map(sorted).collect()),v=>v}}
 let value:serde_json::Value=serde_json::from_slice(bytes).map_err(|_|"Invalid original JSON")?;use sha2::Digest;let canonical=zeroize::Zeroizing::new(serde_json::to_vec(&sorted(value)).map_err(|_|"Original canonicalization failed")?);Ok(format!("{:x}",sha2::Sha256::digest(canonical.as_slice())))
}

pub fn credential_id(token:&str)->Result<String,String>{
 use base64::Engine;let payload=token.split('.').nth(1).ok_or("Credential identity unavailable")?;
 let bytes=zeroize::Zeroizing::new(base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(payload).map_err(|_|"Invalid credential identity")?);
 let v:serde_json::Value=serde_json::from_slice(&bytes).map_err(|_|"Invalid credential identity")?;v["jti"].as_str().filter(|v|!v.is_empty()&&v.len()<=128).map(String::from).ok_or("Missing credential identity".into())
}

#[cfg(test)]mod tests{
 use super::*;
 #[test]fn cr2_connection_consumer_uses_canonical_required_fields_and_bounds(){
  let v:serde_json::Value=serde_json::from_str(include_str!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../packages/contracts/g3-connection/golden.json"))).unwrap();for fixture in v["fixtures"].as_array().unwrap(){validate(fixture["type"].as_str().unwrap(),&fixture["json"]).unwrap();}
  let a=v["fixtures"].as_array().unwrap().iter().find(|v|v["type"]=="ConnectionAdmission").unwrap()["json"].clone();let mut bad=a.clone();bad["decision"]["remainingMs"]=serde_json::json!(30001);assert!(validate("ConnectionAdmission",&bad).is_err());let mut bad=a;bad["owner"].as_object_mut().unwrap().remove("familyId");assert!(validate("ConnectionAdmission",&bad).is_err());
 }
 #[test]fn cr2_original_fingerprint_matches_server_vectors(){let v:serde_json::Value=serde_json::from_str(include_str!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../packages/contracts/g3-connection/fingerprint-vectors.json"))).unwrap();for vector in v["vectors"].as_array().unwrap(){let bytes=zeroize::Zeroizing::new(serde_json::to_vec(&vector["original"]).unwrap());assert_eq!(canonical_fingerprint(&bytes).unwrap(),vector["sha256"].as_str().unwrap());}}
}
