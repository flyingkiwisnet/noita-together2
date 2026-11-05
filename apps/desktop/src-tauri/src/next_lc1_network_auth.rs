// Reviewed fixture-only authentication attribution. No scope, token or actor ID.
fn auth_shape(value:&Value)->Result<(),String>{
 if !matches!(value["revision"].as_u64(),Some(1|2))||value["attempts"].as_u64().is_none(){return Err("Authentication diagnostic revision/count differs".into())}
 // Revision 2 separates retirement attribution from authentication failures.
 // An eligible outage phase permits neither; no stale-scope exemption is added.
 if value["revision"]==2{match value.get("firstRetire"){Some(Value::Null)=>{},Some(_)=>return Err("Unexpected service authentication retirement".into()),None=>return Err("Missing authentication retirement slot".into())}}
 let events=value["events"].as_array().ok_or("Missing authentication diagnostic events")?;if events.len()>64{return Err("Authentication event bound exceeded".into())}
 let event=|e:&Value|->Result<(),String>{
  let object=e.as_object().ok_or("Invalid authentication event")?;
  let fields=["elapsedMs","attempt","socket","connectionTag","renew","clientGeneration","frameAuthorityGeneration","stage","identity","projection","fixtureRefusalConsumed","canonical","member","outcome","bound"];
  if object.keys().any(|k|!fields.contains(&k.as_str()))||e["elapsedMs"].as_u64().is_none(){return Err("Authentication event contains unknown/private fields".into())}
  if !["token-authenticate","token-scope","projection-authorize","transaction-prepare","transaction-token-check","post-transaction-member","outside-authentication"].contains(&e["stage"].as_str().unwrap_or(""))||!["accepted","retired-other","FORBIDDEN"].contains(&e["outcome"].as_str().unwrap_or("")){return Err("Unknown authentication stage/outcome".into())}
  if e["stage"]!="outside-authentication"{
   for key in ["attempt","socket","clientGeneration"]{if e[key].as_u64().is_none_or(|v|v==0){return Err("Missing authentication connection/attempt ordinal".into())}}
   if e["attempt"].as_u64().unwrap()>value["attempts"].as_u64().unwrap()||!e["connectionTag"].as_str().is_some_and(|s|s.len()==16&&s.bytes().all(|b|b.is_ascii_digit()||(b'a'..=b'f').contains(&b))){return Err("Invalid authentication attempt/tag correlation".into())}
  }
  for key in ["renew","fixtureRefusalConsumed","bound"]{if e.get(key).is_some()&&!e[key].is_boolean(){return Err("Invalid authentication boolean".into())}}
  if e.get("frameAuthorityGeneration").is_some()&&e["frameAuthorityGeneration"].as_u64().is_none(){return Err("Invalid authentication authority generation".into())}
  for (key,allowed) in [("identity",&["game","principal","session","epoch"][..]),("projection",&["terminal","exactScope","sameSession"][..]),("member",&["present","digest","parent","family"][..])]{if let Some(v)=e.get(key){let m=v.as_object().ok_or("Invalid authentication predicate group")?;if m.iter().any(|(k,v)|!allowed.contains(&k.as_str())||!v.is_boolean()){return Err("Unknown/private authentication predicate".into())}}}
  if let Some(c)=e.get("canonical"){
   let c=c.as_object().ok_or("Invalid canonical authentication predicates")?;
   let booleans=["terminal","worldPresent","exactScope","sameSession","sameAttachment","authorized","playing","decisionTerminal","reportProtected","reportComplete","reportLoaded","reportContentValid","coreMode","ghostCapability","diagnosticReadFailed"];
   for (key,v) in c{if booleans.contains(&key.as_str()){if !v.is_boolean(){return Err("Invalid canonical authentication boolean".into())}}else if ["generation","reportRevision"].contains(&key.as_str()){if !v.is_null()&&v.as_u64().is_none(){return Err("Invalid canonical authentication revision".into())}}else if key=="reason"{if !["","host_ban","account_disabled","access_revoked","retired","owner_binding_changed","inspection_required","report_invalid","initial_review_expired","lease_expired","entry_required","other"].contains(&v.as_str().unwrap_or("invalid")){return Err("Unknown canonical authentication reason".into())}}else{return Err("Unknown/private canonical authentication field".into())}}
  }
  Ok(())
 };
 for e in events{event(e)?;}
 match value.get("firstFailure"){Some(Value::Null)=>{},Some(v)=>{event(v)?;if v["outcome"]!="FORBIDDEN"{return Err("First authentication failure lost its outcome".into())}},None=>return Err("Missing first authentication failure slot".into())}
 Ok(())
}
fn auth_phase(before:&Value,after:&Value)->Result<Value,String>{
 auth_shape(before)?;auth_shape(after)?;
 if !before["firstFailure"].is_null()||!after["firstFailure"].is_null(){return Err("Unexpected FORBIDDEN authentication; no retired-scope exemption".into())}
 let attempts=after["attempts"].as_u64().unwrap().checked_sub(before["attempts"].as_u64().unwrap()).ok_or("Authentication attempts regressed")?;
 Ok(json!({"before":before,"after":after,"authentication_attempt_delta":attempts,"first_forbidden_observed":false,"includes_successful_same_socket_reauthentication":true}))
}
#[test]fn lc1_network_auth_diagnostic_is_bounded_correlated_and_never_excuses_current_forbidden(){
 let clean=json!({"revision":1,"attempts":0,"events":[],"firstFailure":null});auth_phase(&clean,&clean).unwrap();
 let mut current=clean.clone();current["revision"]=json!(2);current["firstRetire"]=Value::Null;auth_phase(&current,&current).unwrap();let mut retired=current.clone();retired["firstRetire"]=json!({"outcome":"FORBIDDEN"});assert!(auth_phase(&current,&retired).is_err());let mut missing=current.clone();missing.as_object_mut().unwrap().remove("firstRetire");assert!(auth_phase(&current,&missing).is_err());
 let event=json!({"elapsedMs":10,"attempt":1,"socket":1,"connectionTag":"0123456789abcdef","renew":false,"clientGeneration":4,"stage":"projection-authorize","identity":{"game":true,"principal":true,"session":true,"epoch":true},"projection":{"terminal":false,"exactScope":false,"sameSession":false},"canonical":{"terminal":false,"worldPresent":true,"exactScope":true,"sameSession":true,"sameAttachment":true,"generation":2,"authorized":true,"playing":false,"reportProtected":true,"reportRevision":28,"reason":"entry_required"},"outcome":"FORBIDDEN","bound":false});
 let rejected=json!({"revision":1,"attempts":1,"events":[event.clone()],"firstFailure":event});auth_shape(&rejected).unwrap();assert!(auth_phase(&clean,&rejected).is_err());
 let mut wrong=rejected.clone();wrong["events"][0]["connectionTag"]=json!("raw-private-connection");assert!(auth_shape(&wrong).is_err());wrong=rejected.clone();wrong["events"][0]["canonical"]["sessionId"]=json!("private");assert!(auth_shape(&wrong).is_err());wrong=rejected.clone();wrong["events"]=Value::Array(vec![rejected["firstFailure"].clone();65]);assert!(auth_shape(&wrong).is_err());
 let mut consumed=rejected;consumed["firstFailure"]["fixtureRefusalConsumed"]=json!(true);assert!(auth_phase(&clean,&consumed).is_err(),"No one-shot refusal is armed by ordinary network/reopen phases");
}
