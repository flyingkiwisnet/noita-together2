//! Candidate same-PC browser return transport. Not a credential redemption endpoint.
//! Only Rust receives the one-use grant; the service exchange must also require initiator proof.
use crate::secrets::SecretString;
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
    time::{Duration, Instant},
};
const HEADER_LIMIT: usize = 4096;
const BODY_LIMIT: usize = 4096;
pub struct ReturnGrant {
    pub grant: SecretString,
    pub attempt_id: uuid::Uuid,
    pub generation: u64,
}
pub struct BrowserReturn {
    pub url: String,
    pub context: String,
    attempt: Arc<Mutex<Option<uuid::Uuid>>>,
    expires: Instant,
    origin_refused: Arc<AtomicBool>,
    stop: Arc<AtomicBool>,
    receiver: mpsc::Receiver<ReturnGrant>,
    worker: Option<thread::JoinHandle<()>>,
}
impl BrowserReturn {
    pub fn start(origin: &str, generation: u64, lifetime: Duration) -> Result<Self, String> {
        let selected = reqwest::Url::parse(origin).map_err(|_| "Invalid sign-in origin")?;
        if selected.origin().ascii_serialization() != origin
            || !selected.username().is_empty()
            || selected.password().is_some()
            || (selected.scheme() != "https"
                && !(selected.scheme() == "http" && selected.host_str() == Some("127.0.0.1")))
        {
            return Err("Invalid sign-in origin".into());
        }
        if lifetime.is_zero() || lifetime > Duration::from_secs(300) {
            return Err("Invalid sign-in lifetime".into());
        }
        let listener =
            TcpListener::bind(("127.0.0.1", 0)).map_err(|_| "Cannot open local sign-in return")?;
        listener
            .set_nonblocking(true)
            .map_err(|_| "Cannot bound local sign-in return")?;
        let port = listener
            .local_addr()
            .map_err(|_| "No local return address")?
            .port();
        let context = random_state()?;
        let path = "/nt2-return".to_owned();
        let url = format!("http://127.0.0.1:{port}{path}");
        let host = format!("127.0.0.1:{port}");
        let stop = Arc::new(AtomicBool::new(false));
        let worker_stop = stop.clone();
        let (sender, receiver) = mpsc::sync_channel(1);
        let origin = origin.to_owned();
        let nonce = context.clone();
        let expires = Instant::now() + lifetime;
        let attempt = Arc::new(Mutex::new(None));
        let worker_attempt = attempt.clone();
        let origin_refused = Arc::new(AtomicBool::new(false));
        let worker_refused = origin_refused.clone();
        let worker = thread::spawn(move || {
            let mut attempts = 0;
            while !worker_stop.load(Ordering::Acquire) && Instant::now() < expires && attempts < 64
            {
                match listener.accept() {
                    Ok((mut stream, peer)) => {
                        attempts += 1;
                        let result = if peer.ip().is_loopback() {
                            let bound = *worker_attempt.lock().unwrap();
                            receive(
                                &mut stream,
                                &host,
                                &path,
                                &origin,
                                &nonce,
                                bound,
                                &worker_refused,
                            )
                        } else {
                            Err(())
                        };
                        let success = result.is_ok()
                            && !worker_stop.load(Ordering::Acquire)
                            && Instant::now() < expires;
                        let message = if success {
                            "Sign-in return received. Return to Noita Together 2."
                        } else {
                            "Sign-in return refused. Retry from the initiating app."
                        };
                        let response = format!(
                            "HTTP/1.1 {}\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\nCache-Control: no-store\r\nContent-Security-Policy: default-src 'none'; frame-ancestors 'none'\r\nConnection: close\r\n\r\n{}",
                            if success { "200 OK" } else { "400 Bad Request" },
                            message.len(),
                            message
                        );
                        let _ = stream.set_write_timeout(Some(Duration::from_millis(100)));
                        let _ = stream.write_all(response.as_bytes());
                        if success {
                            let (grant, attempt_id) = result.unwrap();
                            let _ = sender.send(ReturnGrant {
                                grant,
                                attempt_id,
                                generation,
                            });
                            break;
                        }
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(10))
                    }
                    Err(_) => break,
                }
            }
        });
        Ok(Self {
            url,
            context,
            attempt,
            expires,
            origin_refused,
            stop,
            receiver,
            worker: Some(worker),
        })
    }
    pub fn take(&self) -> Option<ReturnGrant> {
        if Instant::now() >= self.expires {
            // Discard and zeroize any queued grant after this initiating attempt expires.
            let _ = self.receiver.try_recv();
            return None;
        }
        self.receiver.try_recv().ok()
    }
    pub fn origin_refused(&self) -> bool {
        self.origin_refused.load(Ordering::Acquire)
    }
    /// Bind the service-issued attempt before opening its launch URL. Never rebind.
    pub fn bind_attempt(&self, id: uuid::Uuid) -> Result<(), String> {
        if id.is_nil() || Instant::now() >= self.expires {
            return Err("Invalid sign-in attempt".into());
        }
        let mut attempt = self
            .attempt
            .lock()
            .map_err(|_| "Sign-in return unavailable")?;
        if attempt.is_some() {
            return Err("Sign-in return already bound".into());
        }
        *attempt = Some(id);
        Ok(())
    }
}
pub(crate) fn random_state() -> Result<String, String> {
    let mut bytes = zeroize::Zeroizing::new([0u8; 32]);
    #[cfg(windows)]
    {
        #[link(name = "bcrypt")]
        unsafe extern "system" {
            fn BCryptGenRandom(
                algorithm: *mut std::ffi::c_void,
                bytes: *mut u8,
                count: u32,
                flags: u32,
            ) -> i32;
        }
        if unsafe { BCryptGenRandom(std::ptr::null_mut(), bytes.as_mut_ptr(), 32, 2) } < 0 {
            return Err("Cannot generate sign-in context".into());
        }
    }
    #[cfg(not(windows))]
    {
        return Err("Sign-in return requires Windows".into());
    }
    // Unpadded base64url: exactly 43 characters for 32 CSPRNG bytes.
    let alphabet = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut encoded = String::with_capacity(43);
    for block in bytes.chunks(3) {
        let value = ((block[0] as u32) << 16)
            | ((block.get(1).copied().unwrap_or(0) as u32) << 8)
            | block.get(2).copied().unwrap_or(0) as u32;
        for shift in [18, 12, 6, 0].into_iter().take(block.len() + 1) {
            encoded.push(alphabet[((value >> shift) & 63) as usize] as char);
        }
    }
    Ok(encoded)
}
impl Drop for BrowserReturn {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
fn receive(
    stream: &mut TcpStream,
    host: &str,
    path: &str,
    origin: &str,
    context: &str,
    attempt: Option<uuid::Uuid>,
    origin_refused: &AtomicBool,
) -> Result<(SecretString, uuid::Uuid), ()> {
    let attempt = attempt.ok_or(())?;
    // Accepted sockets can inherit the nonblocking listener mode on Windows.
    // Apply the existing bounded read timeout to a blocking stream so a browser
    // that sends just after accept is not spuriously refused.
    stream.set_nonblocking(false).map_err(|_| ())?;
    stream
        .set_read_timeout(Some(Duration::from_millis(100)))
        .map_err(|_| ())?;
    let deadline = Instant::now() + Duration::from_millis(300);
    let mut bytes = zeroize::Zeroizing::new(Vec::with_capacity(HEADER_LIMIT + BODY_LIMIT + 4));
    let mut chunk = zeroize::Zeroizing::new([0u8; 1024]);
    loop {
        if Instant::now() >= deadline {
            return Err(());
        }
        let n = stream.read(&mut *chunk).map_err(|_| ())?;
        if n == 0 {
            return Err(());
        }
        bytes.extend_from_slice(&chunk[..n]);
        if bytes.len() > HEADER_LIMIT + BODY_LIMIT + 4 {
            return Err(());
        }
        if let Some(end) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
            if end > HEADER_LIMIT {
                return Err(());
            }
            let header = std::str::from_utf8(&bytes[..end]).map_err(|_| ())?;
            let length = check_headers(header, host, path, origin, origin_refused)?;
            let offset = end + 4;
            if bytes.len() > offset + length {
                return Err(());
            }
            if bytes.len() == offset + length {
                return parse_form(&bytes[offset..], context, attempt);
            }
        } else if bytes.len() > HEADER_LIMIT {
            return Err(());
        }
    }
}
fn check_headers(
    header: &str,
    host: &str,
    path: &str,
    origin: &str,
    origin_refused: &AtomicBool,
) -> Result<usize, ()> {
    let mut lines = header.split("\r\n");
    if lines.next() != Some(format!("POST {path} HTTP/1.1").as_str()) {
        return Err(());
    }
    let mut headers = std::collections::HashMap::new();
    for line in lines {
        let (key, value) = line.split_once(':').ok_or(())?;
        if key.is_empty() || key.bytes().any(|b| !b.is_ascii_alphanumeric() && b != b'-') {
            return Err(());
        }
        if headers
            .insert(key.to_ascii_lowercase(), value.trim())
            .is_some()
        {
            return Err(());
        }
    }
    if headers.get("origin") != Some(&origin) {
        origin_refused.store(true, Ordering::Release);
        return Err(());
    }
    if headers.get("host") != Some(&host)
        || headers.contains_key("transfer-encoding")
        || headers.contains_key("expect")
    {
        return Err(());
    }
    let content_type = headers
        .get("content-type")
        .ok_or(())?
        .split(';')
        .next()
        .ok_or(())?;
    if content_type != "application/x-www-form-urlencoded" {
        return Err(());
    }
    let length = headers.get("content-length").ok_or(())?;
    if length.is_empty() || !length.bytes().all(|b| b.is_ascii_digit()) {
        return Err(());
    }
    let length = length.parse::<usize>().map_err(|_| ())?;
    if length == 0 || length > BODY_LIMIT {
        return Err(());
    }
    Ok(length)
}
fn parse_form(
    body: &[u8],
    context: &str,
    expected_attempt: uuid::Uuid,
) -> Result<(SecretString, uuid::Uuid), ()> {
    if body.len() > BODY_LIMIT || !body.is_ascii() {
        return Err(());
    }
    // Strict percent encoding; URL decoding otherwise tolerates malformed escapes/UTF-8.
    for field in body.split(|b| *b == b'&') {
        let mut i = 0;
        while i < field.len() {
            if field[i] == b'%' {
                if i + 2 >= field.len()
                    || !field[i + 1].is_ascii_hexdigit()
                    || !field[i + 2].is_ascii_hexdigit()
                {
                    return Err(());
                }
                i += 3;
            } else {
                i += 1;
            }
        }
    }
    let mut grant = None;
    let mut state = None;
    let mut attempt = None;
    for field in body.split(|b| *b == b'&') {
        let split = field.iter().position(|b| *b == b'=').ok_or(())?;
        let key = decode_form(&field[..split])?;
        let value = decode_form(&field[split + 1..])?;
        match key.as_str() {
            "completionGrant" if grant.is_none() => grant = Some(value),
            "callbackState" if state.is_none() => state = Some(value),
            "attemptId" if attempt.is_none() => attempt = Some(value),
            _ => return Err(()),
        }
    }
    if state.as_ref().map(|s| s.as_str()) != Some(context)
        || attempt.as_ref().map(|s| s.as_str()) != Some(expected_attempt.to_string().as_str())
    {
        return Err(());
    }
    let grant = grant.ok_or(())?;
    if grant.is_empty()
        || grant.len() > 2048
        || !grant
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
    {
        return Err(());
    }
    Ok((SecretString::from(grant.as_str()), expected_attempt))
}
fn decode_form(bytes: &[u8]) -> Result<zeroize::Zeroizing<String>, ()> {
    let mut decoded = zeroize::Zeroizing::new(Vec::with_capacity(bytes.len()));
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' => {
                let hex =
                    std::str::from_utf8(bytes.get(i + 1..i + 3).ok_or(())?).map_err(|_| ())?;
                decoded.push(u8::from_str_radix(hex, 16).map_err(|_| ())?);
                i += 3;
            }
            b'+' => {
                decoded.push(b' ');
                i += 1;
            }
            b => {
                decoded.push(b);
                i += 1;
            }
        }
    }
    let text = std::str::from_utf8(&decoded).map_err(|_| ())?;
    Ok(zeroize::Zeroizing::new(text.to_owned()))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]fn auth1_browser_return_waits_for_headers_after_accept(){
        let callback=BrowserReturn::start("https://service.test",1,Duration::from_secs(5)).unwrap();let attempt=uuid::Uuid::new_v4();callback.bind_attempt(attempt).unwrap();let url=reqwest::Url::parse(&callback.url).unwrap();let host=format!("127.0.0.1:{}",url.port().unwrap());let mut stream=TcpStream::connect(&host).unwrap();stream.set_read_timeout(Some(Duration::from_secs(2))).unwrap();thread::sleep(Duration::from_millis(35));let body=format!("completionGrant=fixture-grant&callbackState={}&attemptId={attempt}",callback.context);stream.write_all(format!("POST {} HTTP/1.1\r\nHost: {host}\r\nOrigin: https://service.test\r\nContent-Type: application/x-www-form-urlencoded\r\nContent-Length: {}\r\n\r\n{body}",url.path(),body.len()).as_bytes()).unwrap();let mut response=String::new();stream.read_to_string(&mut response).unwrap();assert!(response.starts_with("HTTP/1.1 200"));let until=Instant::now()+Duration::from_secs(1);loop{if let Some(grant)=callback.take(){assert_eq!(grant.attempt_id,attempt);break}assert!(Instant::now()<until);thread::sleep(Duration::from_millis(5));}
    }
    fn post(callback: &BrowserReturn, origin: &str, context: &str, grant: &str) -> String {
        let url = reqwest::Url::parse(&callback.url).unwrap();
        let host = format!("127.0.0.1:{}", url.port().unwrap());
        let attempt = callback.attempt.lock().unwrap().unwrap();
        let body = format!("completionGrant={grant}&callbackState={context}&attemptId={attempt}");
        let mut stream = TcpStream::connect(&host).unwrap();
        stream.write_all(format!("POST {} HTTP/1.1\r\nHost: {host}\r\nOrigin: {origin}\r\nContent-Type: application/x-www-form-urlencoded\r\nContent-Length: {}\r\n\r\n{body}",url.path(),body.len()).as_bytes()).unwrap();
        let mut response = String::new();
        // Refused headers can leave unread request bytes; Windows may reset after
        // delivering the HTTP refusal. Require the response below, tolerate that EOF.
        if let Err(error)=stream.read_to_string(&mut response){assert_eq!(error.kind(),std::io::ErrorKind::ConnectionReset);assert!(response.starts_with("HTTP/1.1 400"));}
        response
    }
    #[test]
    fn loopback_return_origin_context_single_use_and_generation() {
        let callback =
            BrowserReturn::start("https://service.test", 7, Duration::from_secs(5)).unwrap();
        assert_eq!(callback.context.len(), 43);
        let attempt = uuid::Uuid::new_v4();
        callback.bind_attempt(attempt).unwrap();
        assert!(callback.bind_attempt(uuid::Uuid::new_v4()).is_err());
        assert!(
            post(
                &callback,
                "https://wrong.test",
                &callback.context,
                "one-use-grant"
            )
            .contains("400")
        );
        assert!(callback.take().is_none());
        assert!(
            post(
                &callback,
                "https://service.test",
                "wrong-context",
                "one-use-grant"
            )
            .contains("400")
        );
        assert!(callback.take().is_none());
        assert!(
            post(
                &callback,
                "https://service.test",
                &callback.context,
                "one-use-grant"
            )
            .contains("200")
        );
        let result = callback.take().unwrap();
        assert_eq!(result.grant.as_str(), "one-use-grant");
        assert_eq!(result.generation, 7);
        assert_eq!(result.attempt_id, attempt);
        assert!(callback.take().is_none());
    }
    #[test]
    fn form_requires_matching_attempt_and_exact_three_fields() {
        let id = uuid::Uuid::new_v4();
        let body = format!("completionGrant=grant%2Dvalue&callbackState=state&attemptId={id}");
        let (grant, actual) = parse_form(body.as_bytes(), "state", id).unwrap();
        assert_eq!(grant.as_str(), "grant-value");
        assert_eq!(actual, id);
        assert!(parse_form(body.as_bytes(), "state", uuid::Uuid::new_v4()).is_err());
        for bad in [
            "completionGrant=x&callbackState=state".to_owned(),
            format!("{body}&attemptId={id}"),
            format!("{body}&extra=x"),
            format!("completionGrant=%FF&callbackState=state&attemptId={id}"),
        ] {
            assert!(parse_form(bad.as_bytes(), "state", id).is_err());
        }
    }
    #[test]
    fn parallel_apps_do_not_accept_each_others_return() {
        let first =
            BrowserReturn::start("https://service.test", 1, Duration::from_secs(5)).unwrap();
        let second =
            BrowserReturn::start("https://service.test", 2, Duration::from_secs(5)).unwrap();
        let id = uuid::Uuid::new_v4();
        first.bind_attempt(id).unwrap();
        second.bind_attempt(id).unwrap();
        assert_ne!(first.context, second.context);
        assert!(post(&first, "https://service.test", &second.context, "grant").contains("400"));
        assert!(first.take().is_none());
        assert!(second.take().is_none());
        assert!(post(&second, "https://service.test", &second.context, "grant").contains("200"));
        assert_eq!(second.take().unwrap().generation, 2);
        assert!(first.take().is_none());
    }
    #[test]
    fn queued_return_cannot_outlive_initiating_attempt() {
        let callback =
            BrowserReturn::start("https://service.test", 9, Duration::from_millis(150)).unwrap();
        callback.bind_attempt(uuid::Uuid::new_v4()).unwrap();
        assert!(
            post(
                &callback,
                "https://service.test",
                &callback.context,
                "grant"
            )
            .contains("200")
        );
        thread::sleep(Duration::from_millis(160));
        assert!(callback.take().is_none());
        assert!(callback.bind_attempt(uuid::Uuid::new_v4()).is_err());
    }
    #[test]
    fn form_headers_and_bounded_cancel() {
        for body in [
            "completionGrant=x&callbackState=c&completionGrant=y",
            "completionGrant=x&callbackState=c&extra=x",
            "completionGrant=%GG&callbackState=c",
            "completionGrant=%FF&callbackState=c",
            "completionGrant=x&callbackState=wrong",
        ] {
            assert!(parse_form(body.as_bytes(), "c", uuid::Uuid::new_v4()).is_err());
        }
        assert!(BrowserReturn::start("http://192.168.0.104", 1, Duration::from_secs(5)).is_err());
        assert!(
            BrowserReturn::start("https://service.test/path", 1, Duration::from_secs(5)).is_err()
        );
        let callback =
            BrowserReturn::start("https://service.test", 1, Duration::from_secs(5)).unwrap();
        let url = reqwest::Url::parse(&callback.url).unwrap();
        let _slow = TcpStream::connect(("127.0.0.1", url.port().unwrap())).unwrap();
        let started = Instant::now();
        drop(callback);
        assert!(started.elapsed() < Duration::from_millis(500));
    }
}
