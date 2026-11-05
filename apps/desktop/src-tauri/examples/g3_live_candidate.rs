#![cfg_attr(windows, windows_subsystem = "windows")]
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
 let command=args.next();
 if command.as_deref()==Some("--verify-kit-root"){
  let result=(||{let root=std::path::PathBuf::from(args.next().ok_or("Local verification folder required")?);
   let approval=nt2_desktop_g0::g3_social_installation::development_profile()?;
   nt2_desktop_g0::g3_social_installation::verify_development(&root,&approval["profile"])?;Ok::<(),String>(())})();
  let output=serde_json::json!({"passed":result.is_ok(),"error":result.err(),"readOnly":true,"actualNoita":false,"installApproved":false});
  if let Some(path)=args.next(){std::fs::write(path,serde_json::to_vec(&output).unwrap()).unwrap();}
  std::process::exit(if output["passed"]==true{0}else{1});
 }
 if command.as_deref()==Some("--verify-public-info"){
  let result=tauri::async_runtime::block_on(async{
   let origin=args.next().ok_or("HTTPS service origin required")?;let ca=args.next().ok_or("Use system trust or provide a public CA file")?;
   let url=reqwest::Url::parse(&origin).map_err(|_|"Invalid public service origin")?;
   if url.scheme()!="https"||!url.username().is_empty()||url.password().is_some()||url.query().is_some()||url.fragment().is_some()||url.path()!="/"{return Err("Exact HTTPS origin required".to_string())}
   let target=format!("{}/v1/info",url.as_str().trim_end_matches('/'));
   let mut transport=reqwest::Client::builder().use_native_tls().no_proxy().https_only(true).redirect(reqwest::redirect::Policy::none()).timeout(std::time::Duration::from_secs(5));
   if ca!="system"{let bytes=std::fs::read(&ca).map_err(|_|"Public CA unavailable")?;let certificate=reqwest::Certificate::from_der(&bytes).or_else(|_|reqwest::Certificate::from_pem(&bytes)).map_err(|_|"Invalid public CA")?;transport=transport.add_root_certificate(certificate);}
   let http=transport.build().map_err(|_|"Public discovery transport unavailable")?;
   let mut response=http.get(&target).send().await.map_err(|_|"Public Info TLS/connection unavailable")?.error_for_status().map_err(|_|"Public Info refused")?;
   if response.url().as_str()!=target||response.content_length().is_some_and(|n|n>65536){return Err("Public Info redirect or size refused".into())}
   let mut body=Vec::new();while let Some(chunk)=response.chunk().await.map_err(|_|"Public Info body unavailable")?{if body.len()+chunk.len()>65536{return Err("Public Info exceeds bounds".into())}body.extend_from_slice(&chunk);}
   let info=protocol_g2_next::info(&body).map_err(|_|"Unsupported candidate contract")?;
   if info.major!=Some(1)||info.revision.as_deref()!=Some(protocol_g2_next::REVISION){return Err("Service/contract mismatch".into())}
   Ok::<_,String>(serde_json::json!({"serviceId":info.service_id,"profileId":info.profile.and_then(|p|p.id),"responseBytes":body.len(),"systemTrustOnly":ca=="system"}))
  });
  let output=serde_json::json!({"passed":result.is_ok(),"discovery":result.as_ref().ok(),"error":result.as_ref().err(),"readOnly":true,"credentialsSent":false,"trustChanged":false,"serviceStarted":false,"normalGuiExecuted":false});
  if let Some(path)=args.next(){std::fs::write(path,serde_json::to_vec(&output).unwrap()).unwrap();}
  std::process::exit(if result.is_ok(){0}else{1});
 }
 next_app::run_g3_social();
}
