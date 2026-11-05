// Private composition instrumentation. Delay is applied once AFTER the complete
// real TLS response body is read. It exercises HttpOwner's total ticket deadline,
// not reqwest's socket/send/body-read timeout. Lost reply uses the Server fixture.
use super::*;
use std::sync::{LazyLock,Mutex};
use serde_json::{json,Value};
struct Profile{generation:u64,origin:String,principals:Vec<String>,rtt_ms:u64,faults:bool,prior_cooldown:bool,started:Instant,events:Vec<Value>,stages:Vec<Value>}
static PROFILE:LazyLock<Mutex<Option<Profile>>>=LazyLock::new(||Mutex::new(None));
static GENERATION:AtomicU64=AtomicU64::new(1);
fn configure(origin:String,principals:Vec<String>,rtt_ms:u64,faults:bool,prior_cooldown:bool){assert!(origin.starts_with("https://127.0.0.1:")&&principals.len()==2&&matches!(rtt_ms,0|250|1000));*PROFILE.lock().unwrap()=Some(Profile{generation:GENERATION.fetch_add(1,Ordering::AcqRel),origin,principals,rtt_ms,faults,prior_cooldown,started:Instant::now(),events:Vec::new(),stages:Vec::new()});}
pub fn arm(origin:String,principals:Vec<String>,rtt_ms:u64){assert!(matches!(rtt_ms,250|1000));configure(origin,principals,rtt_ms,true,false);}
pub fn arm_clean(origin:String,principals:Vec<String>,rtt_ms:u64,prior_cooldown:bool){assert!(matches!(rtt_ms,0|250));configure(origin,principals,rtt_ms,false,prior_cooldown);}
pub fn snapshot()->Value{let p=PROFILE.lock().unwrap();p.as_ref().map(|p|json!({"injectedRttMs":p.rtt_ms,"faults":p.faults,"priorCooldownOnly":p.prior_cooldown,"actualTls":true,"delayAppliedOncePerResponse":true,"delayAfterBodyRead":true,"exercisesReqwestTransportTimeout":false,"events":p.events,"stages":p.stages})).unwrap_or(Value::Null)}
pub fn disarm(){*PROFILE.lock().unwrap()=None;}
fn matches(p:&Profile,binding:&Binding)->bool{p.origin==binding.service_origin&&p.principals.contains(&binding.principal_id)}
pub(crate) fn prior_cooldown(binding:&Binding)->bool{PROFILE.lock().unwrap().as_ref().is_some_and(|p|matches(p,binding)&&p.prior_cooldown)}
pub(crate) fn stage(binding:&Binding,kind:&str,id:Option<&str>){let mut p=PROFILE.lock().unwrap();if let Some(p)=p.as_mut().filter(|p|matches(p,binding)){if p.stages.len()<1024{p.stages.push(json!({"elapsedMs":p.started.elapsed().as_millis(),"stage":kind,"id":id,"actor":p.principals.iter().position(|v|v==&binding.principal_id)}));}}}
pub(super) struct Observation{generation:u64,index:usize,delay:u64,finished:bool}
impl Drop for Observation{fn drop(&mut self){if self.finished{return}let mut p=PROFILE.lock().unwrap();if let Some(p)=p.as_mut().filter(|p|p.generation==self.generation){let elapsed=p.started.elapsed().as_millis();p.events[self.index]["finishedMs"]=json!(elapsed);p.events[self.index]["completeTlsResponse"]=json!(false);}}}
pub(super) fn before_request(binding:&Binding,request:&[u8])->Option<Observation>{
 let q=protocol_g3::request(request).ok()?;
 let kind=if q.bank_batch.is_some(){"bankBatch"}else if q.gold_bank.is_some(){"goldBank"}else if q.gold_status.is_some(){"goldStatus"}else if q.gold_receipt.is_some(){"goldReceipt"}else if q.gold_intent.is_some(){"goldIntent"}else if q.item_bank.is_some(){"itemBank"}else if q.item_status.is_some(){"itemStatus"}else if q.item_receipt.is_some(){"itemReceipt"}else if q.item_deposit.is_some(){"itemDeposit"}else if q.item_claim.is_some(){"itemClaim"}else if q.item_inspect.is_some(){"itemInspect"}else if q.inventory_publish.is_some(){"inventoryPublish"}else if q.inventory_inspect.is_some(){"inventoryInspect"}else{return None};
 let mut p=PROFILE.lock().unwrap();let p=p.as_mut().filter(|p|matches(p,binding))?;assert!(p.events.len()<512,"Bounded optional timing fixture exhausted");let index=p.events.len();
 let jitter=if p.faults{[-40i64,20,0,40,-20][(index+1)%5]}else{0};let stall=if p.faults&&(index+1)%11==0{2400}else{0};let delay=(p.rtt_ms as i64+jitter)as u64+stall;
 p.events.push(json!({"ordinal":index+1,"actor":p.principals.iter().position(|v|v==&binding.principal_id),"kind":kind,"requestBytes":request.len(),"batchOperationIds":q.bank_batch.as_ref().and_then(|v|v.entries.as_ref()).map(|entries|entries.iter().filter_map(|e|e.item_status.as_ref().and_then(|v|v.operation_id.clone()).or_else(||e.item_deposit.as_ref().and_then(|v|v.operation_id.clone()))).collect::<Vec<_>>()),"batchEntries":q.bank_batch.as_ref().and_then(|v|v.entries.as_ref()).map(Vec::len),"requestId":q.request_id,"operationId":q.item_deposit.as_ref().and_then(|v|v.operation_id.clone()).or_else(||q.item_status.as_ref().and_then(|v|v.operation_id.clone())).or_else(||q.item_receipt.as_ref().and_then(|v|v.operation_id.clone())).or_else(||q.gold_status.as_ref().and_then(|v|v.operation_id.clone())).or_else(||q.gold_intent.as_ref().and_then(|v|v.operation_id.clone())).or_else(||q.gold_receipt.as_ref().and_then(|v|v.operation_id.clone())),"startedMs":p.started.elapsed().as_millis(),"injectedDelayMs":delay,"jitterMs":jitter,"stallMs":stall}));
 Some(Observation{generation:p.generation,index,delay,finished:false})
}
pub(super) async fn after_response(observation:Option<Observation>,body:&[u8]){if let Some(mut o)=observation{
 {let mut p=PROFILE.lock().unwrap();if let Some(p)=p.as_mut().filter(|p|p.generation==o.generation){let elapsed=p.started.elapsed().as_millis();p.events[o.index]["tlsBodyReadMs"]=json!(elapsed);p.events[o.index]["responseBytes"]=json!(body.len());p.events[o.index]["responseError"]=serde_json::from_slice::<Value>(body).ok().and_then(|v|v["error"]["code"].as_str().filter(|s|s.len()<=64&&s.bytes().all(|b|b.is_ascii_uppercase()||b==b'_')).map(str::to_owned)).map(Value::String).unwrap_or(Value::Null);let response=serde_json::from_slice::<wire::Response>(body).ok();
 if let Some(response)=response{
  p.events[o.index]["bankRevision"]=json!(response.bank_revision);
  let operations=response.item_operation.into_iter().chain(response.bank_batch.as_ref().and_then(|v|v.entries.as_ref()).into_iter().flatten().filter_map(|e|e.item_operation.clone())).map(|op|json!({"operationId":op.operation_id,"itemId":op.item_id,"state":op.state,"bankRevision":op.bank_revision})).collect::<Vec<_>>();p.events[o.index]["itemResults"]=json!(operations);
  p.events[o.index]["entryErrors"]=json!(response.bank_batch.into_iter().filter_map(|v|v.entries).flatten().filter_map(|e|e.error.and_then(|v|v.code)).collect::<Vec<_>>());
 }
 }}
 tokio::time::sleep(Duration::from_millis(o.delay)).await;
 {let mut p=PROFILE.lock().unwrap();if let Some(p)=p.as_mut().filter(|p|p.generation==o.generation){let elapsed=p.started.elapsed().as_millis();p.events[o.index]["finishedMs"]=json!(elapsed);p.events[o.index]["completeTlsResponse"]=json!(true);}}
 o.finished=true;
}}
