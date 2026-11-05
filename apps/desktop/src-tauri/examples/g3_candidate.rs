#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![allow(dead_code)]
#[path="../src/local_bridge.rs"] mod local_bridge;
#[path="../src/local_owner.rs"] mod local_owner;
#[path="../src/auth_return.rs"] mod auth_return;
#[path="../src/browser.rs"] mod browser;
#[path="../src/signin.rs"] mod signin;
#[path="../src/g1_client.rs"] mod g1_client;
#[path="../src/handoff.rs"] mod handoff;
#[path="../src/installation.rs"] mod installation;
#[path="../src/lifecycle.rs"] mod lifecycle;
#[path="../src/native_report.rs"] mod native_report;
#[path="../src/pipe.rs"] mod pipe;
#[path="../src/preferences.rs"] mod preferences;
#[path="../src/protocol.rs"] mod protocol;
#[path="../src/protocol_g2.rs"] mod protocol_g2;
#[path="../src/protocol_g2_next.rs"] mod protocol_g2_next;
#[path="../src/secrets.rs"] mod secrets;
#[path="../src/transport.rs"] mod transport;
#[path="../src/next_client.rs"] mod next_client;
#[path="../src/next_app.rs"] mod next_app;
fn main(){
 let mut args=std::env::args().skip(1);
 if args.next().as_deref()==Some("--verify-kit-root"){
  let result=(||{let root=std::path::PathBuf::from(args.next().ok_or("Local verification folder required")?);
   let approval=nt2_desktop_g0::g3_installation::profile()?;
   nt2_desktop_g0::g3_installation::verify(&root,&approval["profile"])?;Ok::<(),String>(())})();
  let output=serde_json::json!({"passed":result.is_ok(),"error":result.err(),"readOnly":true,"actualNoita":false,"installApproved":false});
  if let Some(path)=args.next(){std::fs::write(path,serde_json::to_vec(&output).unwrap()).unwrap();}
  std::process::exit(if output["passed"]==true{0}else{1});
 }
 next_app::run_g3();
}
