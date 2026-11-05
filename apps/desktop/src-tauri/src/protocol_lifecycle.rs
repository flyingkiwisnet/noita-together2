//! Server-owned LC1 lifecycle codec, no independent wire definitions.
pub const REVISION:&str="g3-lifecycle-lc1-2";
pub const ENDPOINT:&str="/v3/lifecycle/rpc";
pub mod wire{include!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../packages/contracts/g3-lifecycle/g3-lifecycle.rs"));}
static SCHEMA:std::sync::LazyLock<serde_json::Value>=std::sync::LazyLock::new(||serde_json::from_str(include_str!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../packages/contracts/g3-lifecycle/schema.json"))).unwrap());
pub fn validate<T:serde::Serialize>(name:&str,value:&T)->Result<(),String>{
 fn visit(name:&str,value:&serde_json::Value,depth:u64,fields:&mut usize)->Result<(),String>{
  let bad=||"Invalid lifecycle shape/bounds".to_string();let object=value.as_object().ok_or_else(bad)?;let definition=SCHEMA["messages"][name]["fields"].as_object().ok_or_else(bad)?;*fields=fields.checked_add(object.len()).ok_or_else(bad)?;
  if depth>SCHEMA["limits"]["depth"].as_u64().unwrap()||*fields>SCHEMA["limits"]["fields"].as_u64().unwrap() as usize{return Err(bad());}
  for (key,field) in definition{if field["required"]==true&&!object.contains_key(key){return Err(bad());}}
  for (key,item) in object{let field=definition.get(key).ok_or_else(bad)?;let kind=field["type"].as_str().ok_or_else(bad)?;let check=|v:&serde_json::Value,fields:&mut usize,max:Option<u64>|{if field["values"].as_array().is_some_and(|allowed|!allowed.contains(v)){return Err(bad())}match kind{
   "string"=>if v.as_str().is_some_and(|s|s.len() as u64<=max.unwrap_or(0)){Ok(())}else{Err(bad())},
   "uint32"=>if v.as_u64().is_some_and(|n|n<=field["maxValue"].as_u64().unwrap_or(u64::from(u32::MAX))&&n>=field["minValue"].as_u64().unwrap_or(0)){Ok(())}else{Err(bad())},
   "bool"=>if v.is_boolean(){Ok(())}else{Err(bad())},_=>visit(kind,v,depth+1,fields)}};
   if field["repeated"]==true{let values=item.as_array().ok_or_else(bad)?;if values.len() as u64>field["max"].as_u64().unwrap_or(0){return Err(bad());}for value in values{check(value,fields,field["itemMax"].as_u64())?;}}else{check(item,fields,field["max"].as_u64())?;}
  }
  if let Some(oneof)=SCHEMA["messages"][name]["oneof"].as_array(){if oneof.iter().filter(|key|object.contains_key(key.as_str().unwrap())).count()!=1{return Err(bad());}}Ok(())
 }
 if serde_json::to_vec(value).map_err(|_|"Lifecycle encoding")?.len()>65536{return Err("Lifecycle value exceeds bound".into());}
 let value=serde_json::to_value(value).map_err(|_|"Connection value encoding failed")?;visit(name,&value,0,&mut 0)
}
pub fn response(bytes:&[u8])->Result<wire::Response,String>{
 if bytes.len()>65536{return Err("Lifecycle response exceeds bound".into());}
 let r:wire::Response=serde_json::from_slice(bytes).map_err(|_|"Invalid lifecycle response")?;
 if r.major!=Some(1)||r.revision.as_deref()!=Some(REVISION){return Err("Lifecycle revision changed".into());}validate("Response",&r)?;Ok(r)
}


