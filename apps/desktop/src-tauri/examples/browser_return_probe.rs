//! Isolated candidate mock sign-in through actual trusted HTTPS WebView2 and Rust return.
#[path = "../src/auth_return.rs"]
#[allow(dead_code)]
mod auth_return;
#[path = "../src/browser.rs"]
#[allow(dead_code)]
mod browser;
#[path = "../src/secrets.rs"]
#[allow(dead_code)]
mod secrets;
#[path = "../src/signin.rs"]
#[allow(dead_code)]
mod signin;
use serde_json::{Value, json};
use std::time::{Duration, Instant};
async fn rpc(
    client: &reqwest::Client,
    origin: &str,
    base: &Value,
    command: Value,
    request_id: uuid::Uuid,
) -> Result<Value, String> {
    let mut request = base.clone();
    request["requestId"] = json!(request_id.to_string());
    for (key, value) in command.as_object().ok_or("Bad probe command")? {
        request[key] = value.clone();
    }
    let response = client
        .post(format!("{origin}/v1/rpc"))
        .json(&request)
        .send()
        .await
        .map_err(|_| "Candidate transport refused")?;
    let response: Value = response
        .error_for_status()
        .map_err(|_| "Candidate HTTP refused")?
        .json()
        .await
        .map_err(|_| "Candidate response invalid")?;
    if response.get("error").is_some() {
        return Err("Candidate command refused".into());
    }
    Ok(response)
}
fn main() {
    let origin = std::env::var("NT2_BROWSER_PROBE_ORIGIN").expect("isolated candidate origin");
    assert!(["https://192.168.0.104:4355", "http://127.0.0.1:4355"].contains(&origin.as_str()));
    let https = origin.starts_with("https:");
    let client = reqwest::Client::builder()
        .use_native_tls()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap();
    let info: Result<Value, reqwest::Error> = tauri::async_runtime::block_on(async {
        client
            .get(format!("{origin}/v1/info"))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await
    });
    let info = match info {
        Ok(info) => info,
        Err(_) => {
            let result = std::env::var_os("NT2_BROWSER_PROBE_RESULT").unwrap();
            std::fs::write(result, serde_json::to_vec(&json!({"passed":false,"stage":"trusted HTTPS candidate Info","reason":"Normal TLS or HTTP validation refused; no browser authentication attempted","trustChanges":false,"workingServiceMutation":false,"engineEvidence":false})).unwrap()).unwrap();
            std::process::exit(1);
        }
    };
    let revision = std::env::var("NT2_BROWSER_PROBE_REVISION").expect("explicit candidate revision");
    assert!(["g2-draft-2", "g2-draft-3", "g2-draft-4"].contains(&revision.as_str()));
    assert_eq!(info["revision"], revision);
    assert_eq!(info["serviceId"], "nt2-client-browser-candidate");
    let base = json!({"major":1,"revision":revision,"serviceId":info["serviceId"],"serviceEpoch":info["serviceEpoch"]});
    let provider = std::env::var("NT2_BROWSER_PROBE_PROVIDER").unwrap();
    assert!(["website", "twitch"].contains(&provider.as_str()));
    let mut flow = signin::SignIn::new(&origin, uuid::Uuid::new_v4(), 43).unwrap();
    let device = flow.begin.device_id.to_string();
    let instance = flow.begin.app_instance_id.to_string();
    let begin_request = json!({"authBegin":{"provider":provider,"deviceId":device,"appInstanceId":instance,"proofChallenge":flow.begin.proof_challenge,"callbackState":flow.begin.callback_state,"callbackPort":flow.begin.callback_port}});
    let begin = tauri::async_runtime::block_on(rpc(&client, &origin, &base, begin_request.clone(), flow.begin.request_id)).unwrap();
    let again = tauri::async_runtime::block_on(rpc(&client, &origin, &base, begin_request, flow.begin.request_id)).unwrap();
    assert_eq!(begin["authAttempt"]["attemptId"], again["authAttempt"]["attemptId"]);
    let attempt =
        uuid::Uuid::parse_str(begin["authAttempt"]["attemptId"].as_str().unwrap()).unwrap();
    let target = flow.accept_attempt(
        43,
        attempt,
        begin["authAttempt"]["launchUrl"].as_str().unwrap(),
    )
    .unwrap();
    let expected = target.as_str().to_owned();
    let callback_url = flow.return_url(43).unwrap().to_owned();
    let initial = expected.clone();
    let result = std::path::PathBuf::from(std::env::var_os("NT2_BROWSER_PROBE_RESULT").unwrap());
    let profile = std::path::PathBuf::from(std::env::var_os("NT2_BROWSER_PROBE_PROFILE").unwrap());
    tauri::Builder::default().setup(move|app|{
  tauri::WebviewWindowBuilder::new(app,"main",tauri::WebviewUrl::External(target))
   .title("NT2 isolated candidate browser verification").visible(false).data_directory(profile.clone())
   .on_navigation(move|url|url.as_str()==expected||url.as_str()==callback_url)
   .on_page_load(move|window,payload|{if payload.url().as_str()==initial && payload.event()==tauri::webview::PageLoadEvent::Finished {let _=window.eval("if(!document.getElementById('return')){const button=document.querySelector('button[name=subject][value=alex]');if(button)button.form.requestSubmit(button);}");}}).build()?;
  let app=app.handle().clone();std::thread::spawn(move||{
   let started=Instant::now();let mut passed=false;
   while started.elapsed()<Duration::from_secs(30){
    if let Ok(Some((proof,receipt)))=flow.completion(43){
     if receipt.attempt_id==attempt {
      let exchange=json!({"authExchange":{"attemptId":attempt.to_string(),"deviceId":device,"appInstanceId":instance,"initiatorProof":proof.as_str(),"completionGrant":receipt.completion_grant.as_str()}});
      if let Ok(response)=tauri::async_runtime::block_on(rpc(&client,&origin,&base,exchange.clone(),receipt.request_id)){
       let retry=tauri::async_runtime::block_on(rpc(&client,&origin,&base,exchange,receipt.request_id));
       passed=response["credentials"]["principal"]["source"]==format!("mock:{provider}")&&response["credentials"]["principal"]["displayName"]=="Alex"&&response["credentials"]["kind"]=="desktop"&&retry.is_ok_and(|retried|retried["credentials"]==response["credentials"]);
      }
     }break;
    }std::thread::sleep(Duration::from_millis(20));
   }
   flow.cancel();drop(flow);std::fs::write(result,serde_json::to_vec(&json!({"passed":passed,"test":"actual candidate mock choice, CSRF/cookie, automatic loopback POST through Rust sign-in lifecycle, exact Begin/Exchange retries","https":https,"provider":provider,"seconds":started.elapsed().as_secs_f64(),"isolatedCandidateMutation":true,"workingServiceMutation":false,"trustChanges":false,"engineEvidence":false})).unwrap()).unwrap();app.exit(if passed{0}else{1});
  });Ok(())
 }).run(tauri::generate_context!()).expect("isolated browser runtime");
}
