//! Fixed, local bank metadata only. Never journal payloads or authority tokens.
use super::*;
#[derive(Clone,Serialize,Deserialize)]#[serde(rename_all="camelCase",deny_unknown_fields)]
pub struct BankOperation{pub id:String,pub room:Option<String>,pub run:Option<String>,pub asset:String,pub direction:String,pub phase:String,pub restored:bool,pub current:bool,pub server:Option<String>}
#[derive(Clone,Serialize,Deserialize)]#[serde(rename_all="camelCase",deny_unknown_fields)]
pub struct BankData{pub operations:Vec<BankOperation>,pub truncated:bool,pub item_permissions:u64,pub item_prepares:u64,pub worker_pending:bool}
#[derive(Serialize,Deserialize)]#[serde(rename_all="camelCase",deny_unknown_fields)]
pub struct BankFile{pub revision:String,pub runtime_id:String,pub runtime_sha256:String,pub session_id:String,pub scope:Scope,pub data:BankData}
fn identifier(v:&Value)->Option<String>{let s=v.as_str()?;(uuid::Uuid::parse_str(s).is_ok()&&s.len()==36).then(||s.to_ascii_lowercase())}
pub fn snapshot(view:&Value)->BankData{
 let all=view.pointer("/g3/operations").and_then(Value::as_array).cloned().unwrap_or_default();
 let mut all:Vec<_>=all.iter().collect();all.sort_by_key(|o|(o["current_scope"]!=true,o["restored"]==true));
 let operations=all.iter().take(32).filter_map(|o|{
  let phase=o["phase"].as_str().filter(|s|matches!(*s,"Requested"|"ServerAccepted"|"EffectIntent"|"AppliedSession"|"Uncertain"|"Refused"))?;
  let asset=o["asset"].as_str().filter(|s|matches!(*s,"gold"|"item"))?;let direction=o["direction"].as_str().filter(|s|matches!(*s,"deposit"|"withdraw"|"claim"))?;
  Some(BankOperation{id:identifier(&o["operation_id"] )?,room:identifier(&o["room_id"]),run:identifier(&o["run_id"]),asset:asset.into(),direction:direction.into(),phase:phase.into(),restored:o["restored"]==true,current:o["current_scope"]==true,server:o["server_status"].as_str().filter(|s|matches!(*s,"awaiting_local"|"reserved"|"completed"|"uncertain"|"cancelled")).map(str::to_owned)})
 }).collect();
 BankData{operations,truncated:all.len()>32,item_permissions:view.pointer("/g3/bank/items/permissions").and_then(Value::as_u64).unwrap_or(0).min(4),item_prepares:view.pointer("/g3/bank/items/prepare_waiting").and_then(Value::as_u64).unwrap_or(0).min(4),worker_pending:view.pointer("/g3/bank/pending")==Some(&Value::Bool(true))}
}
#[cfg(test)]mod tests{use super::*;
 #[test]fn bank_diagnostics_only_fixed_bounded_metadata(){let id=uuid::Uuid::new_v4().to_string();let o=serde_json::json!({"operation_id":id,"room_id":id,"run_id":"secret","asset":"item","direction":"deposit","phase":"AppliedSession","restored":false,"current_scope":true,"server_status":"completed","payload":"unrelated-secret","world_token":"unrelated-secret"});let view=serde_json::json!({"g3":{"operations":vec![o;40],"bank":{"items":{"permissions":999,"prepare_waiting":999}}}});let data=snapshot(&view);assert_eq!(data.operations.len(),32);assert!(data.truncated);assert_eq!(data.item_permissions,4);let text=serde_json::to_string(&data).unwrap();assert!(!text.contains("secret")&&!text.contains("world_token")&&!text.contains("payload"));assert!(text.len()<16384);}
}
