use nt2_desktop_g0::debug_diagnostics::{safe_report,MAX_FILE};
use serde_json::json;

#[test]
fn export_keeps_bounded_events_and_omits_free_text_and_session_identity(){
 let mut envelope=json!({"revision":"nt2-client-debug-1","component":"app","runtimeId":"test-beta","runtimeSha256":"a".repeat(64),"sessionId":"PRIVATE_SESSION","scope":{},"startedAtUtcMs":1,"updatedAtUtcMs":2,"closed":false,"enabled":true,"reason":"PRIVATE_REASON","truncated":false,"dropped":2,"firstLoss":null,"recovery":null,"beforeLoss":[],"afterLoss":[],"recent":[{"sequence":1,"utcMs":2,"monotonicMs":1,"kind":"sample","fields":{},"flags":{},"scope":{}}]});
 let value=safe_report(&serde_json::to_vec(&envelope).unwrap()).unwrap();assert_eq!(value["records"].as_array().unwrap().len(),1);assert_eq!(value["dropped"],2);let text=value.to_string();assert!(!text.contains("PRIVATE_SESSION")&&!text.contains("PRIVATE_REASON"));
 envelope["recent"][0]["scope"]=json!({"serviceTag":"PRIVATE_SCOPE"});let value=safe_report(&serde_json::to_vec(&envelope).unwrap()).unwrap();assert!(value["records"].as_array().unwrap().is_empty());assert!(!value.to_string().contains("PRIVATE_SCOPE"));
 envelope["credentials"]=json!("PRIVATE_CREDENTIAL");assert!(safe_report(&serde_json::to_vec(&envelope).unwrap()).is_none());
}

#[test]
fn export_refuses_oversized_or_unrecognized_diagnostic_files(){assert!(safe_report(&vec![b' ';MAX_FILE+1]).is_none());assert!(safe_report(br#"{"refreshSecret":"not-a-diagnostic"}"#).is_none());}
