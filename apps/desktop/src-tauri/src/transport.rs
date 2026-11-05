//! Windows platform trust, with normal certificate/hostname verification intact.
pub fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .tls_backend_native()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(std::time::Duration::from_secs(3))
        .build()
        .expect("HTTP runtime")
}
