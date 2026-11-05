//! Narrow browser operations for the selected service; no renderer-supplied URL opening.
pub fn open_service(origin: &str) -> Result<(), String> {
    let url = reqwest::Url::parse(origin).map_err(|_| "Invalid selected service")?;
    if !["http", "https"].contains(&url.scheme())
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || url.path() != "/"
    {
        return Err("Invalid selected service".into());
    }
    open_url(&url)
}
pub(crate) fn account_password_url(origin:&str)->Result<reqwest::Url,String>{
    let base=reqwest::Url::parse(origin).map_err(|_|"Invalid account service")?;
    if base.origin().ascii_serialization()!=origin||!base.username().is_empty()||base.password().is_some()
        ||!(base.scheme()=="https"||base.scheme()=="http"&&base.host_str()==Some("127.0.0.1")&&base.port().is_some()){
        return Err("Invalid account service".into())
    }
    reqwest::Url::parse(&format!("{origin}/auth/password")).map_err(|_|"Invalid account route".into())
}
pub fn open_account_password(origin:&str)->Result<(),String>{open_url(&account_password_url(origin)?)}
#[allow(dead_code)]
pub fn open_signin(origin: &str, launch_url: &str, attempt: uuid::Uuid) -> Result<(), String> {
    let url = signin_url(origin, launch_url, attempt)?;
    open_url(&url)
}
pub(crate) fn signin_url(
    origin: &str,
    launch_url: &str,
    attempt: uuid::Uuid,
) -> Result<reqwest::Url, String> {
    let base = reqwest::Url::parse(origin).map_err(|_| "Invalid sign-in service")?;
    let url = reqwest::Url::parse(launch_url).map_err(|_| "Invalid sign-in launch")?;
    if attempt.is_nil()
        || base.origin().ascii_serialization() != origin
        || !base.username().is_empty()
        || base.password().is_some()
        || !(base.scheme() == "https"
            || (base.scheme() == "http"
                && base.host_str() == Some("127.0.0.1")
                && base.port().is_some()))
        || url.origin() != base.origin()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || launch_url != format!("{origin}/signin/{attempt}")
    {
        return Err("Invalid sign-in launch".into());
    }
    Ok(url)
}
fn open_url(url: &reqwest::Url) -> Result<(), String> {
    #[cfg(windows)]
    {
        use std::ffi::c_void;
        #[link(name = "shell32")]
        unsafe extern "system" {
            fn ShellExecuteW(
                window: *mut c_void,
                operation: *const u16,
                file: *const u16,
                parameters: *const u16,
                directory: *const u16,
                show: i32,
            ) -> isize;
        }
        let wide = |text: &str| text.encode_utf16().chain(Some(0)).collect::<Vec<_>>();
        let operation = wide("open");
        let file = wide(url.as_str());
        let result = unsafe {
            ShellExecuteW(
                std::ptr::null_mut(),
                operation.as_ptr(),
                file.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                1,
            )
        };
        if result <= 32 {
            return Err("Could not open the selected service website".into());
        }
        Ok(())
    }
    #[cfg(not(windows))]
    {
        Err("Browser opening is unavailable on this platform".into())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]fn auth1_password_route_is_fixed_to_the_selected_service(){
        assert_eq!(account_password_url("https://nt2.flyingkiwis.net").unwrap().as_str(),"https://nt2.flyingkiwis.net/auth/password");
        assert!(account_password_url("http://127.0.0.1:4358").is_ok());
        for origin in["https://good.invalid/path","https://good.invalid?next=evil","https://user:secret@good.invalid","https://good.invalid#evil","http://192.168.0.104:4358"]{assert!(account_password_url(origin).is_err());}
    }
    #[test]
    fn signin_launch_is_exact_origin_and_attempt_without_secrets() {
        let id = uuid::Uuid::new_v4();
        let expected = format!("https://service.test/signin/{id}");
        assert!(signin_url("https://service.test", &expected, id).is_ok());
        for url in [
            format!("{expected}?grant=x"),
            format!("{expected}#x"),
            format!("https://other.test/signin/{id}"),
            format!("https://user@service.test/signin/{id}"),
            format!("https://service.test/signin/{}", uuid::Uuid::new_v4()),
            format!("https://service.test/signin/%{id}"),
        ] {
            assert!(signin_url("https://service.test", &url, id).is_err());
        }
        assert!(
            signin_url(
                "http://192.168.0.104:4352",
                &format!("http://192.168.0.104:4352/signin/{id}"),
                id
            )
            .is_err()
        );
    }
}
