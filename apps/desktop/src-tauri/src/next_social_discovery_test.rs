//! Production discovery over bounded loopback HTTP; metadata is injected.
//! This does not exercise TLS trust, service SQL or Noita.
use super::*;
use std::io::{Read,Write};
#[test]fn a2_canonical_core_discovery_accepts_current_contract_and_rejects_substitutions(){tauri::async_runtime::block_on(async{
 let raw=include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../packages/contracts/g3-social/schema.json"));
 let schema:Value=serde_json::from_slice(raw).unwrap();let policy:Value=serde_json::from_slice(include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../packages/contracts/g3-social/replay-policy.json"))).unwrap();
 let pinned=nt2_desktop_g0::g3_social_installation::development_profile().unwrap();
 use sha2::{Digest,Sha256};
 let good=json!({"major":schema["major"],"revision":schema["revision"],"schemaSha256":format!("{:x}",Sha256::digest(raw)),"replayPolicy":policy["policy"],"serviceId":pinned["serviceId"],"serviceEpoch":"discovery-test-epoch","sourceApproved":true,"developmentOnly":true,"installApproved":false,"runtimeApproval":{"sha256":nt2_desktop_g0::g3_social_installation::DEVELOPMENT_SHA},"runtimeProfile":pinned["profile"]});
 let mutations=[("revision",json!("g3-social-draft-1")),("schemaSha256",json!("829dea7c1363cd588ddbd6aa7515d401f394c71ecb997a6437180b3e1ddf7c6e")),("serviceId",json!("foreign")),("serviceEpoch",json!("retired")),("replayPolicy",json!("foreign")),("sourceApproved",json!(false)),("developmentOnly",json!(false)),("installApproved",json!(true)),("runtimeApproval",json!({"sha256":"wrong"})),("runtimeProfile",json!({"id":"wrong"}))];
 let mut replies=vec![good.clone()];for(field,bad)in &mutations{let mut changed=good.clone();changed[*field]=bad.clone();replies.push(changed);}replies.push(good.clone());let mut rejected=good.clone();rejected["revision"]=json!("g3-social-draft-1");replies.push(rejected);
 let listener=std::net::TcpListener::bind("127.0.0.1:0").unwrap();listener.set_nonblocking(true).unwrap();let origin=format!("http://{}",listener.local_addr().unwrap());
 let server=std::thread::spawn(move||{for reply in replies{let deadline=Instant::now()+Duration::from_secs(5);let mut socket=loop{match listener.accept(){Ok((s,_))=>break s,Err(e)if e.kind()==std::io::ErrorKind::WouldBlock=>{assert!(Instant::now()<deadline,"Expected discovery request");std::thread::sleep(Duration::from_millis(2));},Err(e)=>panic!("{e}")}};socket.set_read_timeout(Some(Duration::from_secs(2))).unwrap();let mut head=vec![];let mut byte=[0];while !head.ends_with(b"\r\n\r\n"){socket.read_exact(&mut byte).unwrap();head.push(byte[0]);assert!(head.len()<8192);}let head=String::from_utf8(head).unwrap();assert!(head.starts_with("GET /v3/social/info HTTP/1.1\r\n"));assert!(!head.to_ascii_lowercase().contains("authorization:"));let body=reply.to_string();write!(socket,"HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",body.len(),body).unwrap();}});
 let mut c=core();c.base=origin;c.service=pinned["serviceId"].as_str().unwrap().into();c.epoch="discovery-test-epoch".into();c.http=reqwest::Client::builder().no_proxy().redirect(reqwest::redirect::Policy::none()).build().unwrap();
 c.refresh_social_profile().await.unwrap();assert_eq!(c.social_profile.as_ref(),Some(&pinned["profile"]));
 for(field,_)in mutations{assert!(c.refresh_social_profile().await.unwrap_err().contains("runtime metadata differs"),"Discovery accepted substituted {field}");}
 c.refresh_social_profile_checked().await.unwrap();c.social_enabled=true;assert!(c.refresh_social_profile_checked().await.is_err());assert!(!c.social_enabled&&c.social_profile.is_none()&&c.social_verified.is_none());assert!(c.native_failure.is_some());server.join().unwrap();
});}
