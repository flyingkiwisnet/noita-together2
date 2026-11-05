//! Disposable public prefs, scoped credential store and bounded loopback HTTP only.
use super::*;
use std::io::{Read, Write};
const OLD: &str = "nt2-g3-c2-potion-diagnostic";
const NEW: &str = "nt2-g3-c2-potion-correction";
#[derive(Default)]
struct Store { events: Mutex<Vec<(String, ServiceProfile)>>, credential_id:Mutex<Option<uuid::Uuid>> }
impl CredentialStore for Store {
    fn load(&self, p:&ServiceProfile)->Result<Option<SecretBlob>,crate::secrets::StoreError>{
        // Recovery journals have their own profile. Only loading the desktop
        // credential constitutes the forbidden remembered-session restore.
        if Some(p.profile_id())!=*self.credential_id.lock().unwrap(){return Ok(None)}
        self.events.lock().unwrap().push(("load".into(),p.clone()));
        Err(crate::secrets::StoreError::Corrupt) // Any restore is a test failure.
    }
    fn save(&self,p:&ServiceProfile,_:&SecretBlob)->Result<(),crate::secrets::StoreError>{
        self.events.lock().unwrap().push(("save".into(),p.clone()));Ok(())
    }
    fn delete(&self,p:&ServiceProfile)->Result<(),crate::secrets::StoreError>{
        self.events.lock().unwrap().push(("delete".into(),p.clone()));Ok(())
    }
}
fn client(origin:&str)->Core<Store>{
    let mut c=Core::new(Store::default(),uuid::Uuid::new_v4(),Arc::new(Mutex::new(None)),
        std::env::temp_dir().join(format!("nt2-signin01-{}",uuid::Uuid::new_v4())));
    c.open_browser=false;
    c.public_preferences(c.claims_root.join("servers.json")).unwrap();
    let unrelated=c.credential_profile("http://127.0.0.1:4321","unrelated-service").unwrap();
    c.remember_server(&unrelated,"unrelated-service").unwrap();
    let selected=c.credential_profile(origin,OLD).unwrap();c.remember_server(&selected,OLD).unwrap();
    *c.store.credential_id.lock().unwrap()=Some(selected.profile_id());
    c.base=origin.into();c.service=OLD.into();c.profile=Some(selected);
    c.game_path=Some(c.claims_root.join("preserved-game"));
    std::fs::write(c.claims_root.join("journal-sentinel"),b"untouched").unwrap();
    c
}
fn clean(c:&Core<Store>){std::fs::remove_dir_all(&c.claims_root).unwrap();}
// Scripted responses; capture complete requests to prove fresh/anonymous traffic.
fn server(replies:Vec<(String,String,String)>)->(String,std::thread::JoinHandle<Vec<String>>){
    let listener=std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let origin=format!("http://{}",listener.local_addr().unwrap());let base=origin.clone();
    let worker=std::thread::spawn(move||{
        let mut requests=vec![];listener.set_nonblocking(true).unwrap();
        for (status,headers,body) in replies {
            let deadline=Instant::now()+Duration::from_secs(10);
            let mut socket=loop{match listener.accept(){Ok((s,_))=>break s,Err(e) if e.kind()==std::io::ErrorKind::WouldBlock=>{assert!(Instant::now()<deadline,"No expected fixture request");std::thread::sleep(Duration::from_millis(5));},Err(e)=>panic!("{e}")}};
            socket.set_nonblocking(false).unwrap();
            socket.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
            let mut bytes=vec![];let mut byte=[0];
            while !bytes.ends_with(b"\r\n\r\n"){socket.read_exact(&mut byte).unwrap();bytes.push(byte[0]);assert!(bytes.len()<8192);}
            let head=String::from_utf8(bytes.clone()).unwrap();
            let length=head.lines().find_map(|l|l.to_lowercase().strip_prefix("content-length:").and_then(|v|v.trim().parse::<usize>().ok())).unwrap_or(0);
            assert!(length<16384);let mut request_body=vec![0;length];socket.read_exact(&mut request_body).unwrap();bytes.extend(request_body);
            let body=if body=="guest" || body=="browser" {
                let request:Value=serde_json::from_slice(&bytes[head.len()..]).unwrap();
                let mut value=json!({"major":1,"revision":protocol::REVISION,"serviceId":NEW,"serviceEpoch":"test-epoch","requestId":request["requestId"]});
                if body=="guest" {value["credentials"]=json!({"accessToken":"a".repeat(43),"refreshSecret":"r".repeat(43),"accountId":"fresh-account","sessionId":"fresh-desktop","sessionEpoch":"fresh-epoch","kind":"desktop","deviceId":request["guestIssue"]["deviceId"],"expiresAt":"2099-01-01T00:00:00.000Z","principal":{"principalId":"fresh-account","kind":"guest","displayName":"Fresh guest","source":"guest"}});}
                else {let id=uuid::Uuid::new_v4();value["authAttempt"]=json!({"attemptId":id,"state":"pending","provider":"website","launchUrl":format!("{base}/signin/{id}"),"expiresAt":"2099-01-01T00:00:00.000Z"});}
                value.to_string()
            } else {body};
            let response=format!("HTTP/1.1 {status}\r\n{headers}Content-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len());
            socket.write_all(response.as_bytes()).unwrap();requests.push(String::from_utf8(bytes).unwrap());
        } requests
    });(origin,worker)
}
fn info()->(String,String,String){let mut value:Value=serde_json::from_str(&discovery_info(NEW)).unwrap();value["mockProvider"]=true.into();("200 OK".into(),"".into(),value.to_string())}
fn challenge<S: CredentialStore>(c:&Core<S>)->IdentityRecovery{serde_json::from_value(c.view()["identity_mismatch"].clone()).unwrap()}
#[test]
fn signin01_explicit_fresh_guest_remember_and_selected_history_only(){
    tauri::async_runtime::block_on(async{
        for remember in [false,true] {
            let (origin,worker)=server(vec![info(),info(),("200 OK".into(),"".into(),"guest".into())]);
            let mut c=client(&origin);let original_path=c.game_path.clone();let device=c.device;
            assert!(c.login(&origin,true,"website",None,None).await.unwrap_err().contains("identity mismatch"));
            assert!(c.store.events.lock().unwrap().is_empty());
            let r=challenge(&c);assert_eq!(r.expected,OLD);assert_eq!(r.discovered,NEW);
            assert_eq!(c.recent_servers[0].service_id,OLD);assert!(c.view()["login_error"].as_str().unwrap().contains("remembered credentials were not sent"));
            c.login(&origin,remember,"website",Some("Fresh guest"),Some(r.clone())).await.unwrap();
            assert_eq!(c.desktop.as_ref().unwrap().account,"fresh-account");assert_eq!(c.service,NEW);
            assert_eq!(c.game_path,original_path);assert_eq!(c.device,device);assert!(c.view()["identity_mismatch"].is_null());assert!(c.view()["login_error"].is_null());
            let saved=crate::preferences::Preferences::at(c.claims_root.join("servers.json")).load(device).unwrap();
            assert_eq!(saved.len(),2);assert_eq!(saved[0].service_id,NEW);assert_eq!(saved[1].service_id,"unrelated-service");
            assert_eq!(std::fs::read(c.claims_root.join("journal-sentinel")).unwrap(),b"untouched");
            let events=c.store.events.lock().unwrap();assert_eq!(events.len(),1);assert_eq!(events[0].0,if remember{"save"}else{"delete"});assert_eq!(events[0].1,c.credential_profile(&origin,NEW).unwrap());drop(events);
            assert!(c.recover_identity(&origin,true,r).await.is_err());
            let requests=worker.join().unwrap();assert_eq!(requests.len(),3);
            assert!(requests.iter().all(|r|!r.to_lowercase().contains("authorization:")&&!r.contains("refresh")&&!r.contains("cookie:")));
            assert!(requests[2].contains("guestIssue"));
            // Logout leaves the selected public history; it is not identity recovery.
            c.desktop=None;c.logout().await.unwrap();assert_eq!(c.recent_servers[0].service_id,NEW);clean(&c);
        }
    });
}
#[test]
fn signin01_stale_modified_origin_generation_context_or_history_refuses_before_contact(){
    tauri::async_runtime::block_on(async{
        let(origin,worker)=server(vec![info()]);let mut c=client(&origin);
        c.login(&origin,true,"website",None,None).await.unwrap_err();let r=challenge(&c);worker.join().unwrap();
        for field in 0..4 {let mut altered=r.clone();match field{0=>altered.origin="http://127.0.0.1:4444".into(),1=>altered.expected="wrong".into(),2=>altered.discovered="wrong".into(),_=>altered.ticket="wrong".into()};assert!(c.recover_identity(&origin,true,altered).await.unwrap_err().contains("changed"));}
        assert!(c.recover_identity("http://127.0.0.1:4444",true,r.clone()).await.unwrap_err().contains("changed"));
        c.base="http://127.0.0.1:4444".into();assert!(c.recover_identity(&origin,true,r.clone()).await.unwrap_err().contains("changed"));c.base=origin.clone();
        c.service="another".into();assert!(c.recover_identity(&origin,true,r.clone()).await.unwrap_err().contains("changed"));c.service=OLD.into();
        c.recent_servers[0].service_id="another".into();assert!(c.recover_identity(&origin,true,r.clone()).await.unwrap_err().contains("Saved identity changed"));c.recent_servers[0].service_id=OLD.into();
        c.generation.fetch_add(1,Ordering::AcqRel);assert!(c.recover_identity(&origin,true,r).await.unwrap_err().contains("changed"));
        assert!(c.store.events.lock().unwrap().is_empty());clean(&c);
    });
}
#[test]
fn signin01_rediscovery_refuses_new_identity_redirect_incompatible_bounds(){
    tauri::async_runtime::block_on(async{
        for reply in [
            ("200 OK".into(),"".into(),discovery_info("unexpected-third-service")),
            ("302 Found".into(),"Location: http://127.0.0.1:9/v1/info\r\n".into(),discovery_info(NEW)),
            ("200 OK".into(),"".into(),discovery_info(NEW).replace(protocol::REVISION,"g2-draft-1")),
            ("200 OK".into(),"".into(),"x".repeat(65537)),
            ("200 OK".into(),"".into(),discovery_info("invalid identity!")),
        ] {
            let(origin,worker)=server(vec![info(),reply]);let mut c=client(&origin);
            c.login(&origin,true,"website",None,None).await.unwrap_err();let r=challenge(&c);
            assert!(c.login(&origin,true,"website",None,Some(r)).await.is_err());
            assert!(c.store.events.lock().unwrap().is_empty());assert_eq!(c.recent_servers[0].service_id,OLD);
            assert!(c.desktop.is_none()&&c.auth.is_none());assert!(c.view()["login_error"].is_string());worker.join().unwrap();clean(&c);
        }
    });
}
#[test]
fn signin01_active_native_authority_retired_without_world_migration(){
    tauri::async_runtime::block_on(async{
        let(mut c,_,_)=release_case();let(origin,worker)=server(vec![info(),info()]);
        c.base=origin.clone();c.service=OLD.into();c.profile=Some(c.credential_profile(&origin,OLD).unwrap());
        c.recent_servers.push(crate::preferences::RecentServer{remembered_name:None,has_saved_account:None,origin:origin.clone(),service_id:OLD.into()});
        assert!(c.game.is_some()&&c.pending.lock().unwrap().is_some());
        c.login(&origin,true,"website",None,None).await.unwrap_err();let r=challenge(&c);
        assert!(c.desktop.is_none()&&c.game.is_none()&&c.preparation.is_none());assert!(c.native_revoked);
        assert!(!c.pending.lock().unwrap().as_ref().unwrap().release);
        c.recover_identity(&origin,true,r).await.unwrap();assert!(c.pending.lock().unwrap().is_none());
        assert!(c.desktop.is_none()&&c.room.is_none()&&c.preparation.is_none());worker.join().unwrap();
    });
}
#[test]
fn signin01_browser_recovery_begins_new_anonymous_attempt(){
    tauri::async_runtime::block_on(async{
        let(origin,worker)=server(vec![info(),info(),("200 OK".into(),"".into(),"browser".into())]);let mut c=client(&origin);
        c.login(&origin,true,"website",None,None).await.unwrap_err();let r=challenge(&c);
        c.login(&origin,true,"website",None,Some(r)).await.unwrap();
        assert!(c.auth.is_some());assert!(c.view()["auth_pending"].as_bool().unwrap());assert!(c.store.events.lock().unwrap().is_empty());
        let requests=worker.join().unwrap();assert!(requests[2].contains("authBegin"));assert!(!requests[2].contains("refresh")&&!requests[2].to_lowercase().contains("authorization:"));clean(&c);
    });
}
#[test]
fn signin01_incompatible_social_runtime_preserves_saved_identity(){
    tauri::async_runtime::block_on(async{
        let(origin,worker)=server(vec![info(),info(),("200 OK".into(),"".into(),"{}".into())]);let mut c=client(&origin);
        c.social_requested=true;
        c.login(&origin,true,"website",None,None).await.unwrap_err();let r=challenge(&c);
        assert!(c.login(&origin,true,"website",None,Some(r)).await.unwrap_err().contains("runtime metadata differs"));
        assert_eq!(c.recent_servers[0].service_id,OLD);assert!(c.store.events.lock().unwrap().is_empty());
        assert!(c.desktop.is_none()&&c.auth.is_none());
        let requests=worker.join().unwrap();assert!(requests[2].starts_with("GET /v3/social/info "));clean(&c);
    });
}
#[test]
fn signin01_recovery_uses_normal_tls_before_any_credential_or_history_change(){
    tauri::async_runtime::block_on(async{
        let listener=std::net::TcpListener::bind("127.0.0.1:0").unwrap();let origin=format!("https://{}",listener.local_addr().unwrap());
        let worker=std::thread::spawn(move||{let(mut socket,_)=listener.accept().unwrap();socket.set_read_timeout(Some(Duration::from_secs(5))).unwrap();let mut bytes=[0;8192];let n=socket.read(&mut bytes).unwrap();let _=socket.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\n{}");bytes[..n].to_vec()});
        let mut c=client(&origin);
        // Inject only a captured public challenge to isolate recovery's TLS contact.
        let r=IdentityRecovery{origin:origin.clone(),expected:OLD.into(),discovered:NEW.into(),ticket:uuid::Uuid::new_v4().to_string()};
        c.identity_mismatch=Some(IdentityMismatch{recovery:r.clone(),generation:c.generation.load(Ordering::Acquire),base:c.base.clone(),service:c.service.clone()});
        assert_eq!(c.login(&origin,true,"website",None,Some(r)).await.unwrap_err(),"Service TLS/connection unavailable");
        assert!(!worker.join().unwrap().starts_with(b"GET "));assert!(c.store.events.lock().unwrap().is_empty());
        assert_eq!(c.recent_servers[0].service_id,OLD);clean(&c);
    });
}
#[test]
fn signin01_pending_auth_retry_errors_stay_inline_and_success_clears_them(){
    tauri::async_runtime::block_on(async{
        let failure=("500 Internal Server Error".into(),"".into(),"{}".into());
        let(origin,worker)=server(vec![info(),info(),failure.clone(),failure,("200 OK".into(),"".into(),"guest".into())]);let mut c=client(&origin);
        c.login(&origin,true,"website",None,None).await.unwrap_err();let r=challenge(&c);
        c.login(&origin,true,"website",Some("Fresh guest"),Some(r)).await.unwrap_err();
        c.login_error=None;c.next_work=Instant::now();c.tick().await.unwrap_err();
        assert!(c.view()["login_error"].as_str().unwrap().contains("HTTP refused"));assert!(c.view()["auth_pending"].as_bool().unwrap());
        c.auth_tick().await.unwrap();assert!(c.view()["login_error"].is_null());assert!(c.desktop.is_some());worker.join().unwrap();clean(&c);
    });
}
