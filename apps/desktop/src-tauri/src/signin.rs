//! Rust-owned successor sign-in lifecycle. Wire adapter follows reviewed contract acceptance.
//! The renderer receives progress only; proof/grant and immutable retry IDs stay here.
use crate::{
    auth_return::{BrowserReturn, random_state},
    browser,
    secrets::SecretString,
};
use sha2::{Digest, Sha256};
use std::time::{Duration, Instant};

pub struct Begin {
    pub request_id: uuid::Uuid,
    pub device_id: uuid::Uuid,
    pub app_instance_id: uuid::Uuid,
    pub proof_challenge: String,
    pub callback_port: u16,
    pub callback_state: String,
}
pub struct Exchange {
    pub request_id: uuid::Uuid,
    pub attempt_id: uuid::Uuid,
    pub completion_grant: SecretString,
    until: Instant,
}
pub struct SignIn {
    pub begin: Begin,
    origin: String,
    generation: u64,
    proof: SecretString,
    callback: Option<BrowserReturn>,
    attempt: Option<uuid::Uuid>,
    exchange: Option<Exchange>,
    expires: Instant,
}
impl SignIn {
    pub fn new(origin: &str, device_id: uuid::Uuid, generation: u64) -> Result<Self, String> {
        if device_id.is_nil() {
            return Err("Invalid sign-in device".into());
        }
        let callback = BrowserReturn::start(origin, generation, Duration::from_secs(300))?;
        let proof = SecretString::new(random_state()?);
        let proof_challenge = Sha256::digest(proof.as_str().as_bytes())
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        let callback_port = reqwest::Url::parse(&callback.url)
            .map_err(|_| "Invalid local return")?
            .port()
            .ok_or("Invalid local return")?;
        Ok(Self {
            begin: Begin {
                request_id: uuid::Uuid::new_v4(),
                device_id,
                app_instance_id: uuid::Uuid::new_v4(),
                proof_challenge,
                callback_port,
                callback_state: callback.context.clone(),
            },
            origin: origin.into(),
            generation,
            proof,
            callback: Some(callback),
            attempt: None,
            exchange: None,
            expires: Instant::now() + Duration::from_secs(300),
        })
    }
    fn current(&self, generation: u64) -> Result<(), String> {
        if self.generation != generation
            || Instant::now() >= self.expires
            || self.callback.is_none()
        {
            return Err("Sign-in attempt no longer current".into());
        }
        Ok(())
    }
    /// Exact Begin retries may return the same attempt. A different result cannot replace it.
    pub fn accept_attempt(
        &mut self,
        generation: u64,
        id: uuid::Uuid,
        launch: &str,
    ) -> Result<reqwest::Url, String> {
        self.current(generation)?;
        let launch = browser::signin_url(&self.origin, launch, id)?;
        match self.attempt {
            Some(old) if old != id => return Err("Conflicting sign-in attempt".into()),
            Some(_) => {}
            None => {
                self.callback.as_ref().unwrap().bind_attempt(id)?;
                self.attempt = Some(id);
            }
        }
        Ok(launch)
    }
    pub fn proof(&self, generation: u64) -> Result<&SecretString, String> {
        self.current(generation)?;
        Ok(&self.proof)
    }
    pub fn cancellation(&self)->Option<(uuid::Uuid,&SecretString)>{self.attempt.map(|attempt|(attempt,&self.proof))}
    pub fn return_url(&self, generation: u64) -> Result<&str, String> {
        self.current(generation)?;
        Ok(self.callback.as_ref().unwrap().url.as_str())
    }
    pub fn completion(&mut self, generation: u64) -> Result<Option<(&SecretString, &Exchange)>, String> {
        self.exchange(generation)?;
        Ok(self.exchange.as_ref().map(|exchange| (&self.proof, exchange)))
    }
    /// Receipt begins one bounded exchange. Retries borrow the identical grant/request ID.
    pub fn exchange(&mut self, generation: u64) -> Result<Option<&Exchange>, String> {
        self.current(generation)?;
        if self.exchange.is_none() {
            if let Some(receipt) = self.callback.as_ref().unwrap().take() {
                if receipt.generation != generation || Some(receipt.attempt_id) != self.attempt {
                    return Err("Sign-in return context changed".into());
                }
                self.exchange = Some(Exchange {
                    request_id: uuid::Uuid::new_v4(),
                    attempt_id: receipt.attempt_id,
                    completion_grant: receipt.grant,
                    until: Instant::now() + Duration::from_secs(30),
                });
            }
        }
        if self
            .exchange
            .as_ref()
            .is_some_and(|e| Instant::now() >= e.until)
        {
            self.cancel();
            return Err("Sign-in exchange retry expired".into());
        }
        Ok(self.exchange.as_ref())
    }
    /// Logout/profile replacement/cancellation closes the return and erases pending secrets.
    pub fn cancel(&mut self) {
        self.callback.take();
        self.exchange.take();
        self.proof = SecretString::new(String::new());
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::{Read, Write},
        net::TcpStream,
    };
    #[test]fn auth1_human_window_and_independent_attempt_cancellation(){
        let device=uuid::Uuid::new_v4();let mut first=SignIn::new("https://service.test",device,1).unwrap();let second=SignIn::new("https://service.test",device,1).unwrap();
        assert!(first.expires.saturating_duration_since(Instant::now())>Duration::from_secs(299));
        assert_ne!(first.begin.callback_port,second.begin.callback_port);assert_ne!(first.begin.callback_state,second.begin.callback_state);assert_ne!(first.begin.proof_challenge,second.begin.proof_challenge);
        first.cancel();assert!(first.proof(1).is_err());assert!(second.proof(1).is_ok());
    }
    #[test]
    fn exact_begin_and_exchange_retry_and_generation_fence() {
        let mut flow = SignIn::new("https://service.test", uuid::Uuid::new_v4(), 4).unwrap();
        let id = uuid::Uuid::new_v4();
        let launch = format!("https://service.test/signin/{id}");
        let request = flow.begin.request_id;
        flow.accept_attempt(4, id, &launch).unwrap();
        flow.accept_attempt(4, id, &launch).unwrap();
        assert_eq!(flow.begin.request_id, request);
        assert_eq!(flow.begin.proof_challenge.len(), 64);
        assert!(flow.accept_attempt(5, id, &launch).is_err());
        let other = uuid::Uuid::new_v4();
        assert!(
            flow.accept_attempt(4, other, &format!("https://service.test/signin/{other}"))
                .is_err()
        );
        assert!(flow.exchange(4).unwrap().is_none());
        let host = format!("127.0.0.1:{}", flow.begin.callback_port);
        let body = format!(
            "attemptId={id}&completionGrant=one-use&callbackState={}",
            flow.begin.callback_state
        );
        let mut stream = TcpStream::connect(&host).unwrap();
        stream.write_all(format!("POST /nt2-return HTTP/1.1\r\nHost: {host}\r\nOrigin: https://service.test\r\nContent-Type: application/x-www-form-urlencoded\r\nContent-Length: {}\r\n\r\n{body}",body.len()).as_bytes()).unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).unwrap();
        assert!(response.contains("200"));
        let exchange = flow.exchange(4).unwrap().unwrap();
        let retry = exchange.request_id;
        assert_eq!(exchange.completion_grant.as_str(), "one-use");
        assert_eq!(flow.exchange(4).unwrap().unwrap().request_id, retry);
        assert!(flow.exchange(5).is_err());
        flow.cancel();
        assert!(flow.proof(4).is_err());
        assert!(flow.exchange(4).is_err());
    }
    #[test]
    fn expired_exchange_cannot_remint_or_reuse_a_received_grant() {
        let mut flow = SignIn::new("https://service.test", uuid::Uuid::new_v4(), 1).unwrap();
        flow.exchange = Some(Exchange {
            request_id: uuid::Uuid::new_v4(),
            attempt_id: uuid::Uuid::new_v4(),
            completion_grant: SecretString::from("grant"),
            until: Instant::now() - Duration::from_secs(1),
        });
        assert!(flow.exchange(1).is_err());
        assert!(flow.exchange.is_none());
        assert!(flow.callback.is_none());
        assert!(flow.proof.as_str().is_empty());
    }
}
