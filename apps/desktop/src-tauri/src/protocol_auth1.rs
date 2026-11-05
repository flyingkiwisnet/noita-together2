//! Canonical AUTH1 consumer. Account metadata is a current UI projection only.
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::sync::LazyLock;
pub mod wire{include!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../packages/contracts/g3-auth1/g3-auth1.rs"));}
static SCHEMA:LazyLock<Value>=LazyLock::new(||serde_json::from_str(include_str!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../packages/contracts/g3-auth1/schema.json"))).unwrap());
fn typed<T:DeserializeOwned>(name:&str,bytes:&[u8])->Result<T,String>{
 fn validate(name:&str,value:&Value,depth:u64,count:&mut usize)->Result<(),String>{
  if depth>SCHEMA["limits"]["depth"].as_u64().unwrap(){return Err("Account nesting bound".into())}
  let object=value.as_object().ok_or("Account object required")?;*count+=object.len();
  if *count>SCHEMA["limits"]["fields"].as_u64().unwrap()as usize{return Err("Account field bound".into())}
  for(key,value)in object{
   let field=&SCHEMA["messages"][name]["fields"][key];let kind=field["type"].as_str().ok_or("Unknown account field")?;
   let check=|value:&Value,count:&mut usize|->Result<(),String>{match kind{
    "string"=>{let text=value.as_str().ok_or("Account string required")?;if text.len()as u64>field["max"].as_u64().unwrap_or(0)||field["values"].as_array().is_some_and(|values|!values.iter().any(|v|v.as_str()==Some(text))){return Err("Account string bound/enum".into())}Ok(())},
    "uint32"=>if value.as_u64().is_some_and(|n|n<=field["max"].as_u64().unwrap_or(u32::MAX as u64)){Ok(())}else{Err("Account integer bound".into())},
    "bool"=>if value.is_boolean(){Ok(())}else{Err("Account boolean required".into())},
    _=>validate(kind,value,depth+1,count),
   }};
   if field["repeated"]==true{let values=value.as_array().ok_or("Account array required")?;if values.len()as u64>field["max"].as_u64().unwrap_or(0){return Err("Account array bound".into())}for value in values{check(value,count)?;}}else{check(value,count)?;}
  }Ok(())
 }
 let value=crate::protocol_g2_next::strict_json(bytes)?;validate(name,&value,0,&mut 0)?;
 serde_json::from_value(value).map_err(|_|"Invalid canonical account value".into())
}
fn identity(revision:Option<&str>,service:Option<&str>,epoch:Option<&str>,expected_service:&str,expected_epoch:&str)->Result<(),String>{
 if revision!=Some("auth1-1")||service!=Some(expected_service)||epoch!=Some(expected_epoch)||expected_service.is_empty()||expected_epoch.is_empty(){return Err("Account service identity changed".into())}Ok(())
}
pub fn discovery(bytes:&[u8],service:&str,epoch:&str)->Result<wire::Discovery,String>{
 let value:wire::Discovery=typed("Discovery",bytes).map_err(|_|"Invalid sign-in options")?;
 identity(value.revision.as_deref(),value.service_id.as_deref(),value.service_epoch.as_deref(),service,epoch)?;
 if value.sign_in_seconds!=Some(300)||value.account_endpoint.as_deref()!=Some("/v1/auth/account")||value.password_change_url.as_deref()!=Some("/auth/password"){return Err("Unsupported account routes or sign-in allowance".into())}
 let providers=value.providers.as_ref().ok_or("Sign-in options absent")?;
 if providers.len()!=3||["twitch","google","local"].iter().any(|id|providers.iter().filter(|p|p.id.as_deref()==Some(*id)&&p.available.is_some()).count()!=1){return Err("Invalid sign-in providers".into())}
 Ok(value)
}
pub fn account(bytes:&[u8],service:&str,epoch:&str,account:&str)->Result<wire::Account,String>{
 let value:wire::AccountResponse=typed("AccountResponse",bytes).map_err(|_|"Invalid account response")?;
 identity(value.revision.as_deref(),value.service_id.as_deref(),value.service_epoch.as_deref(),service,epoch)?;
 match(value.account,value.error){
  (None,Some(error))=>Err(error.code.filter(|c|!c.is_empty()).unwrap_or("Account request refused".into())),
  (Some(a),None)=>{
   if a.account_id.as_deref()!=Some(account)||account.is_empty()||a.enabled!=Some(true)||a.revision.is_none()||a.display_name.as_ref().is_none_or(|n|n.is_empty())||a.provider.as_ref().is_none_or(|p|p.is_empty())||a.last_sign_in_at.as_ref().is_some_and(|at|crate::g1_client::timestamp(at).is_err()){return Err("Account context changed".into())}
   let role=a.role.as_deref().ok_or("Account role missing")?;if !["user","power-user","admin","dev"].contains(&role){return Err("Unknown account role".into())}
   let e=a.entitlements.as_ref().ok_or("Account entitlements missing")?;
   let(cap,standard,power)=(e.capacity_limit.ok_or("Capacity missing")?,e.standard_capacity_limit.ok_or("Capacity missing")?,e.power_capacity_limit.ok_or("Capacity missing")?);
   if !(1..=32).contains(&standard)||!(standard..=32).contains(&power)||cap==0||cap>power||role=="user"&&cap>standard
    ||e.custom_lobby_name!=Some(role!="user")||e.manage_users!=Some(matches!(role,"admin"|"dev"))||e.manage_all_roles!=Some(role=="dev")
    ||e.testing_mode.is_none()||role!="dev"&&e.testing_mode==Some(true)||e.generated_lobby_name.as_ref().is_none_or(|n|n.is_empty()){
     return Err("Invalid current account entitlements".into())
   }Ok(a)
  },_=>Err("Account/error response shape differs".into())
 }
}
#[cfg(test)]mod tests{
 use super::*;
 fn fixtures()->Vec<Value>{serde_json::from_str(include_str!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../packages/contracts/g3-auth1/fixture-inputs.json"))).unwrap()}
 #[test]fn auth1_exact_discovery_account_and_untrusted_routes(){
  let f=fixtures();let d=f[0]["json"].clone();let a=f[1]["json"].clone();let service=d["serviceId"].as_str().unwrap();let epoch=d["serviceEpoch"].as_str().unwrap();let id=a["account"]["accountId"].as_str().unwrap();
  assert!(discovery(&serde_json::to_vec(&d).unwrap(),service,epoch).is_ok());assert!(account(&serde_json::to_vec(&a).unwrap(),service,epoch,id).is_ok());
  for (field,value)in[("revision","other"),("serviceId","other"),("serviceEpoch","other"),("accountEndpoint","https://evil.invalid/account"),("passwordChangeUrl","//evil.invalid/password")]{let mut bad=d.clone();bad[field]=value.into();assert!(discovery(&serde_json::to_vec(&bad).unwrap(),service,epoch).is_err());}
  let mut bad=d.clone();bad["providers"][1]=bad["providers"][0].clone();assert!(discovery(&serde_json::to_vec(&bad).unwrap(),service,epoch).is_err());
  for field in["accountId","role"]{let mut bad=a.clone();bad["account"][field]="other".into();assert!(account(&serde_json::to_vec(&bad).unwrap(),service,epoch,id).is_err());}
  let mut bad=a.clone();bad["error"]=serde_json::json!({"code":"FORBIDDEN"});assert!(account(&serde_json::to_vec(&bad).unwrap(),service,epoch,id).is_err());
  let mut bad=a.clone();bad["account"]["entitlements"]["capacityLimit"]=33.into();assert!(account(&serde_json::to_vec(&bad).unwrap(),service,epoch,id).is_err());
  let mut bad=a.clone();bad["account"]["entitlements"]["testingMode"]=true.into();assert!(account(&serde_json::to_vec(&bad).unwrap(),service,epoch,id).is_err());
  assert!(discovery(br#"{"revision":"auth1-1","revision":"auth1-1"}"#,service,epoch).is_err());assert!(discovery(&vec![b' ';65537],service,epoch).is_err());
 }
}
