//! Separately compiled G3 HTTP owner seam. Frozen binding/credentials must come from
//! accepted sign-in + verified native inspection + selected Start, never the renderer.
//! Constructing this seam does not authenticate a game or approve a native profile.
use crate::{g3_local_consumer::Binding, protocol_g3::{self, wire}};
use std::{sync::{Arc, atomic::{AtomicU64, Ordering}}, time::{Duration, Instant}};
use zeroize::Zeroizing;

#[cfg(any(test,feature="g3-bank-latency-fixture"))]#[path="g3_http_bank_timing_test.rs"]pub mod bank_timing;
const MAX_BYTES: usize = 262144;
const DEADLINE: Duration = Duration::from_secs(3);
const GHOST_DEADLINE: Duration = Duration::from_millis(500);
#[path="g3_http_batch.rs"]mod batch;
#[path="g3_http_c1_source.rs"]pub(crate) mod c1_source;

pub struct HttpOwner {
    #[cfg(test)]
    test_exchange: Option<Box<dyn Fn(&[u8])->Result<Vec<u8>,String>+Send+Sync>>,
    http: reqwest::Client,
    binding: Binding,
    token: Zeroizing<String>,
    fence: Arc<AtomicU64>,
    generation: u64,
    in_flight: Option<String>,
    movement_at: Option<Instant>,
    ghost_at: Option<Instant>,
    window: Instant,
    count: u32,
    authority_guard: Option<(Arc<AtomicU64>, u64, Instant)>,
}
struct Ticket {
    request: wire::Request,
    started: Instant,
}
impl HttpOwner {
    pub async fn statistics(&mut self,request:crate::protocol_stats::wire::Request)->Result<crate::protocol_stats::wire::Response,String>{
        use crate::protocol_stats as p;
        self.active()?;
        if request.observe.is_none()||request.discover.is_some()||request.read.is_some()||request.service_id.as_deref()!=Some(&self.binding.service_id)||request.service_epoch.as_deref()!=Some(&self.binding.service_epoch)||request.session_epoch.as_deref()!=Some(&self.binding.session_epoch)||serde_json::to_value(&request.scope).unwrap()!=serde_json::to_value(&Some(&self.binding.scope)).unwrap(){return Err("Statistics owner mismatch".into())}
        let bytes=serde_json::to_vec(&request).map_err(|_|"Statistics encoding")?;p::request(&bytes).map_err(|_|"Statistics request schema")?;
        let expected=format!("{}/v3/stats/rpc",self.binding.service_origin);
        tokio::time::timeout(DEADLINE,async{
            let mut response=self.http.post(&expected).header("content-type","application/json").bearer_auth(self.token.as_str()).body(bytes).send().await.map_err(|_|"Statistics connection unavailable")?;
            self.active()?;if response.url().as_str()!=expected||!response.status().is_success()||response.headers().get("content-type").and_then(|v|v.to_str().ok()).is_none_or(|v|v.split(';').next()!=Some("application/json")){return Err("Statistics HTTP response refused".into())}
            let mut bytes=Vec::new();while let Some(part)=response.chunk().await.map_err(|_|"Statistics response interrupted")?{self.active()?;if bytes.len()+part.len()>32768{return Err("Statistics response bound".into())}bytes.extend_from_slice(&part)}
            let r=p::response(&bytes).map_err(|_|"Statistics response schema")?;self.active()?;if r.service_id!=request.service_id||r.service_epoch!=request.service_epoch||r.request_id!=request.request_id{return Err("Statistics correlation".into())}
            if let Some(e)=r.error{return Err(e.code.unwrap_or("Statistics refused".into()))}if r.accepted!=Some(true){return Err("Statistics observation not accepted".into())}Ok(r)
        }).await.map_err(|_|"Statistics whole request deadline")?
    }
    /// Same frozen session, renewed credential from Core; keep rate/replay state.
    pub fn refresh_verified_token(&mut self,token:Zeroizing<String>)->Result<(),String>{
        self.active()?;
        if self.in_flight.is_some()||token.is_empty()||token.len()>4096||token.bytes().any(|b|b<=32||b>=127){return Err("Invalid G3 credential renewal".into());}
        self.token=token;Ok(())
    }
    /// Accepted Rust Core owns this verified TLS client; never a renderer argument.
    pub fn from_verified_client(binding: Binding, token: Zeroizing<String>, fence: Arc<AtomicU64>, http:reqwest::Client)->Result<Self,String>{
        Self::validate_endpoint(&binding,&token)?;
        Ok(Self::with_client(binding,token,fence,http))
    }
    /// Rust owner integration seam only. No renderer command exposes this constructor.
    pub fn new(binding: Binding, token: Zeroizing<String>, fence: Arc<AtomicU64>) -> Result<Self, String> {
        Self::with_roots(binding, token, fence, None)
    }
    fn with_roots(binding: Binding, token: Zeroizing<String>, fence: Arc<AtomicU64>, roots: Option<Vec<reqwest::Certificate>>) -> Result<Self, String> {
        Self::validate_endpoint(&binding,&token)?;
        let mut builder = reqwest::Client::builder().tls_backend_native().no_proxy()
            .redirect(reqwest::redirect::Policy::none()).timeout(DEADLINE);
        if let Some(roots) = roots { builder = builder.tls_certs_only(roots); }
        let http = builder.build().map_err(|_| "G3 HTTP runtime unavailable")?;
        Ok(Self::with_client(binding,token,fence,http))
    }
    fn validate_endpoint(binding:&Binding,token:&str)->Result<(),String>{
        let origin = reqwest::Url::parse(&binding.service_origin).map_err(|_| "Invalid G3 origin")?;
        if origin.scheme() != "https" || origin.host_str().is_none()
            || !origin.username().is_empty() || origin.password().is_some()
            || origin.path() != "/" || origin.query().is_some() || origin.fragment().is_some()
            || origin.as_str().trim_end_matches('/') != binding.service_origin
            || token.is_empty() || token.len() > 4096 || token.bytes().any(|b| b <= 32 || b >= 127)
        { return Err("Invalid G3 endpoint or credential".into()); }
        Ok(())
    }
    fn with_client(binding:Binding,token:Zeroizing<String>,fence:Arc<AtomicU64>,http:reqwest::Client)->Self{
        Self { #[cfg(test)] test_exchange:None, http, binding, token, generation: fence.load(Ordering::Acquire), fence,
            in_flight: None, movement_at: None, ghost_at: None, window: Instant::now(), count: 0, authority_guard: None }
    }
    pub(crate) fn active(&self) -> Result<(), String> {
        if self.fence.load(Ordering::Acquire) != self.generation
            || self.authority_guard.as_ref().is_some_and(|(fence, generation, until)| fence.load(Ordering::Acquire) != *generation || Instant::now() >= *until) {
            Err("Retired G3 owner generation".into())
        } else { Ok(()) }
    }
    /// C1 uses this same authenticated client, lease and cancellation fence.
    /// Called only by the optional off-Core worker, never the Noita frame path.
    pub async fn c1(&mut self,request:crate::protocol_c1::wire::Request)->Result<crate::protocol_c1::wire::Response,String>{
        use crate::protocol_c1 as p;
        self.active()?;
        let bytes=serde_json::to_vec(&request).map_err(|_|"C1 encoding")?;
        p::request(&bytes).map_err(|_|"Invalid canonical C1 request")?;
        if request.service_id.as_deref()!=Some(&self.binding.service_id)||request.service_epoch.as_deref()!=Some(&self.binding.service_epoch)||request.session_epoch.as_deref()!=Some(&self.binding.session_epoch){return Err("C1 owner mismatch".into())}
        let scope=serde_json::to_value(&self.binding.scope).map_err(|_|"C1 scope")?;
        if request.source_outcome.is_none()&&request.scope.as_ref().map(serde_json::to_value).transpose().map_err(|_|"C1 scope")?!=Some(scope){return Err("C1 scope mismatch".into())}
        let mut response=self.http.post(format!("{}/v3/c1/rpc",self.binding.service_origin)).header("content-type","application/json").bearer_auth(self.token.as_str()).body(bytes).send().await.map_err(|_|"C1 connection unavailable")?;
        self.active()?;
        if !response.status().is_success()||response.headers().get("content-type").and_then(|v|v.to_str().ok()).is_none_or(|v|v.split(';').next()!=Some("application/json")){return Err("C1 HTTP response refused".into())}
        let mut bytes=Vec::new();while let Some(chunk)=response.chunk().await.map_err(|_|"C1 response interrupted")?{self.active()?;if bytes.len()+chunk.len()>32768{return Err("C1 response too large".into())}bytes.extend_from_slice(&chunk);}
        let reply=p::response(&bytes).map_err(|_|"C1 response schema")?;self.active()?;
        if reply.service_id!=request.service_id||reply.service_epoch!=request.service_epoch||reply.request_id!=request.request_id{return Err("C1 response correlation".into())}
        if let Some(error)=&reply.error{return Err(error.code.clone().unwrap_or_else(||"C1 refused".into()))}
        // Canonical rulesGet/Set responses are room metadata and omit the
        // gameplay scope echo. The authenticated bound request, UUID and guard
        // still correlate them; such a response can carry no gameplay effect.
        let metadata=request.rules_get.is_some()||request.rules_set.is_some()||request.source_outcome.is_some();
        if metadata {
            if reply.event.is_some()||reply.deliveries.is_some()||reply.scope.is_some()||reply.owner_generation.is_some(){return Err("C1 metadata response correlation".into())}
        }else if reply.owner_generation!=request.owner_generation||serde_json::to_value(&reply.scope).unwrap()!=serde_json::to_value(&request.scope).unwrap(){return Err("C1 response correlation".into())}
        Ok(reply)
    }
    pub async fn c2(&mut self,request:crate::protocol_c2::wire::Request)->Result<crate::protocol_c2::wire::Response,String>{
        use crate::protocol_c2 as p;
        self.active()?;
        let bytes=serde_json::to_vec(&request).map_err(|_|"C2 encoding")?;
        p::request(&bytes).map_err(|_|"Invalid canonical C2 request")?;
        if request.service_id.as_deref()!=Some(&self.binding.service_id)||request.service_epoch.as_deref()!=Some(&self.binding.service_epoch)||request.session_epoch.as_deref()!=Some(&self.binding.session_epoch){return Err("C2 owner mismatch".into())}
        let scope=serde_json::to_value(&self.binding.scope).map_err(|_|"C2 scope")?;
        if request.source_outcome.is_none()&&request.scope.as_ref().map(serde_json::to_value).transpose().map_err(|_|"C2 scope")?!=Some(scope){return Err("C2 scope mismatch".into())}
        let mut response=self.http.post(format!("{}/v3/c2/rpc",self.binding.service_origin)).header("content-type","application/json").bearer_auth(self.token.as_str()).body(bytes).send().await.map_err(|_|"C2 connection unavailable")?;
        self.active()?;
        if !response.status().is_success()||response.headers().get("content-type").and_then(|v|v.to_str().ok()).is_none_or(|v|v.split(';').next()!=Some("application/json")){return Err("C2 HTTP response refused".into())}
        let mut bytes=Vec::new();while let Some(chunk)=response.chunk().await.map_err(|_|"C2 response interrupted")?{self.active()?;if bytes.len()+chunk.len()>32768{return Err("C2 response too large".into())}bytes.extend_from_slice(&chunk);}
        let reply=p::response(&bytes).map_err(|_|"C2 response schema")?;self.active()?;
        if reply.service_id!=request.service_id||reply.service_epoch!=request.service_epoch||reply.request_id!=request.request_id{return Err("C2 response correlation".into())}
        if let Some(error)=&reply.error{return Err(error.code.clone().unwrap_or_else(||"C2 refused".into()))}
        // Canonical rulesGet/Set responses are room metadata and omit the
        // gameplay scope echo. The authenticated bound request, UUID and guard
        // still correlate them; such a response can carry no gameplay effect.
        let metadata=request.rules_get.is_some()||request.rules_set.is_some()||request.source_outcome.is_some();
        if metadata {
            if reply.event.is_some()||reply.deliveries.is_some()||reply.scope.is_some()||reply.owner_generation.is_some(){return Err("C2 metadata response correlation".into())}
        }else if reply.owner_generation!=request.owner_generation||serde_json::to_value(&reply.scope).unwrap()!=serde_json::to_value(&request.scope).unwrap(){return Err("C2 response correlation".into())}
        Ok(reply)
    }
    /// Core's independent cancellation generation and existing native lease.
    /// Playback and a successful HTTP response cannot renew either authority.
    pub(crate) fn guard(&mut self, fence: Arc<AtomicU64>, generation: u64, until: Instant) {
        self.authority_guard = Some((fence, generation, until));
    }
    pub fn c1_guard(&mut self,fence:Arc<AtomicU64>,generation:u64,until:Instant){self.guard(fence,generation,until);}
    fn plan(&mut self, bytes: &[u8], now: Instant) -> Result<Ticket, String> {
        self.active()?;
        if self.in_flight.is_some() { return Err("G3 request already in flight".into()); }
        let request = protocol_g3::request(bytes).map_err(|_| "Invalid canonical G3 request")?;
        if request.service_id.as_deref() != Some(&self.binding.service_id)
            || request.service_epoch.as_deref() != Some(&self.binding.service_epoch)
            || request.session_epoch.as_deref() != Some(&self.binding.session_epoch)
            || serde_json::to_vec(&request.current_scope.as_ref().or(request.scope.as_ref())).unwrap() != serde_json::to_vec(&Some(&self.binding.scope)).unwrap()
            || request.request_id.as_deref().is_none_or(|id| uuid::Uuid::parse_str(id).is_err())
        { return Err("G3 request substituted frozen authority".into()); }
        // No arbitrary canonical command is granted by this first gameplay adapter.
        if request.movement.is_none() && request.ghost_list.is_none() && request.gold_intent.is_none()
            && request.gold_status.is_none() && request.outbox_list.is_none() && request.gold_bank.is_none() && request.gold_receipt.is_none()
            &&request.item_deposit.is_none()&&request.item_claim.is_none()&&request.item_status.is_none()&&request.item_receipt.is_none()&&request.item_bank.is_none()&&request.item_inspect.is_none()&&request.inventory_publish.is_none()&&request.inventory_inspect.is_none()&&request.bank_batch.is_none()
        { return Err("G3 command not enabled in this adapter".into()); }
        if let Some(original)=&request.scope{
            let current=&self.binding.scope;
            if request.current_scope.is_some() && (request.gold_status.is_none()&&request.gold_receipt.is_none()&&request.item_status.is_none()&&request.item_receipt.is_none()
                ||original.room_id!=current.room_id||original.run_id!=current.run_id||original.run_epoch!=current.run_epoch
                ||original.config_revision!=current.config_revision||original.world_token!=current.world_token){return Err("Original recovery crossed world".into())}
        }
        if let Some(batch)=&request.bank_batch{
            let entries=batch.entries.as_deref().ok_or("Missing bank batch entries")?;
            if !(1..=4).contains(&entries.len())||bytes.len()>196608{return Err("Bank batch capacity".into())}
            for e in entries{
                let original=e.scope.as_ref().ok_or("Missing batch original scope")?;let current=&self.binding.scope;
                if original.room_id!=current.room_id||original.run_id!=current.run_id||original.run_epoch!=current.run_epoch||original.config_revision!=current.config_revision||original.world_token!=current.world_token||serde_json::to_vec(&e.current_scope.as_ref().or(e.scope.as_ref())).unwrap()!=serde_json::to_vec(&Some(current)).unwrap(){return Err("Bank batch crossed frozen authority".into())}
            }
        }
        if now.saturating_duration_since(self.window) >= Duration::from_secs(1) {
            self.window = now; self.count = 0;
        }
        let cost=request.bank_batch.as_ref().and_then(|v|v.entries.as_ref()).map_or(1,|v|v.len() as u32);
        if self.count.saturating_add(cost)>20 { return Err("G3 owner rate capacity".into()); }
        for (enabled, last) in [(request.movement.is_some(), self.movement_at), (request.ghost_list.is_some(), self.ghost_at)] {
            if enabled && last.is_some_and(|at| now.saturating_duration_since(at) < Duration::from_millis(100)) {
                return Err("G3 polling rate capacity".into());
            }
        }
        self.count += cost;
        if request.movement.is_some() { self.movement_at = Some(now); }
        if request.ghost_list.is_some() { self.ghost_at = Some(now); }
        self.in_flight = request.request_id.clone();
        Ok(Ticket { request, started: now })
    }
    fn complete(&mut self, ticket: Ticket, bytes: Result<Vec<u8>, String>, now: Instant) -> Result<wire::Response, String> {
        let expected = self.in_flight.take();
        self.active()?;
        if expected != ticket.request.request_id || expected.is_none() {
            return Err("G3 reply has no matching in-flight request".into());
        }
        let elapsed = now.saturating_duration_since(ticket.started);
        if elapsed > DEADLINE || (ticket.request.ghost_list.is_some() && elapsed > GHOST_DEADLINE) {
            return Err("Late G3 response discarded".into());
        }
        let bytes = bytes?;
        let response: wire::Response = protocol_g3::response_as(&bytes).map_err(|_| "Invalid canonical G3 response")?;
        if response.request_id != ticket.request.request_id
            || response.service_id != ticket.request.service_id
            || response.service_epoch != ticket.request.service_epoch
        { return Err("G3 response correlation or service changed".into()); }
        // A canonical type alone grants no effect; callers must verify the operation
        // against the flushed ORIGINAL journal before obtaining any local effect permit.
        let has_operation = ticket.request.gold_intent.is_some() || ticket.request.gold_status.is_some() || ticket.request.gold_receipt.is_some();
        let has_item=ticket.request.item_deposit.is_some()||ticket.request.item_claim.is_some()||ticket.request.item_status.is_some()||ticket.request.item_receipt.is_some();
        if (response.error.is_none() && has_operation && response.operation.is_none())
            || (!has_operation && response.operation.is_some())
            || (ticket.request.ghost_list.is_none() && response.ghosts.is_some())
            || (ticket.request.outbox_list.is_none() && (response.outbox.is_some() || response.next_sequence.is_some()))
            || response.statistics.is_some() || response.aggregates.is_some()
            || (ticket.request.gold_bank.is_some() != response.bank.is_some() && response.error.is_none())
            ||(response.error.is_none()&&has_item!=response.item_operation.is_some())
            ||(response.error.is_none()&&ticket.request.item_bank.is_some()!=response.item_bank.is_some())
            ||(response.error.is_none()&&ticket.request.item_inspect.is_some()!=response.item_detail.is_some())
            ||(response.error.is_none()&&ticket.request.inventory_inspect.is_some()!=response.inventory.is_some())
            || (response.error.is_some() && (response.operation.is_some() || response.ghosts.is_some() || response.outbox.is_some() || response.bank.is_some()||response.item_operation.is_some()||response.item_bank.is_some()||response.item_detail.is_some()||response.inventory.is_some()))
        { return Err("Unexpected G3 response payload".into()); }
        batch::correlate(&self.binding,&ticket.request,&response)?;
        if let Some(error) = &response.error {
            return Err(error.code.clone().unwrap_or_else(|| "G3 request refused".into()));
        }
        if let Some(operation) = &response.operation {
            let expected_id = ticket.request.gold_intent.as_ref().and_then(|v| v.operation_id.as_ref())
                .or_else(|| ticket.request.gold_status.as_ref().and_then(|v| v.operation_id.as_ref()))
                .or_else(|| ticket.request.gold_receipt.as_ref().and_then(|v|v.operation_id.as_ref()));
            let original=ticket.request.scope.as_ref().ok_or("Missing original scope")?;
            if operation.operation_id.as_ref() != expected_id || expected_id.is_none()
                || operation.principal_id.as_deref() != Some(&self.binding.principal_id)
                || operation.run_id != original.run_id
                || operation.attachment_id != original.attachment_id
                || operation.world_token != original.world_token
            { return Err("G3 operation substituted original owner/world".into()); }
        }
        if let Some(operation)=&response.item_operation{
            let expected=ticket.request.item_deposit.as_ref().and_then(|q|q.operation_id.as_ref()).or_else(||ticket.request.item_claim.as_ref().and_then(|q|q.operation_id.as_ref())).or_else(||ticket.request.item_status.as_ref().and_then(|q|q.operation_id.as_ref())).or_else(||ticket.request.item_receipt.as_ref().and_then(|q|q.operation_id.as_ref()));
            if operation.operation_id.as_ref()!=expected||operation.principal_id.as_deref()!=Some(&self.binding.principal_id)||serde_json::to_vec(&operation.original_scope).unwrap()!=serde_json::to_vec(&ticket.request.scope).unwrap(){return Err("Item operation substituted original owner/world".into())}
        }
        Ok(response)
    }
    #[cfg(test)]
    pub(crate) fn test_transport(&mut self,transport:impl Fn(&[u8])->Result<Vec<u8>,String>+Send+Sync+'static){self.test_exchange=Some(Box::new(transport));}
    async fn exchange(&self, bytes: Vec<u8>) -> Result<Vec<u8>, String> {
        self.active()?;
        #[cfg(test)] if let Some(transport)=&self.test_exchange{return transport(&bytes)}
        #[cfg(any(test,feature="g3-bank-latency-fixture"))]let timing=bank_timing::before_request(&self.binding,&bytes);
        let mut response = self.http.post(format!("{}/v3/rpc", self.binding.service_origin))
            .header("content-type", "application/json").bearer_auth(self.token.as_str()).body(bytes)
            .send().await.map_err(|_| "G3 connection unavailable; retain original IDs")?;
        if !response.status().is_success()
            || response.headers().get("content-type").and_then(|v| v.to_str().ok())
                .is_none_or(|v| v.split(';').next() != Some("application/json"))
        { return Err("G3 HTTP response refused".into()); }
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(|_| "G3 response interrupted; retain original IDs")? {
            self.active()?;
            if bytes.len() + chunk.len() > MAX_BYTES { return Err("G3 response too large".into()); }
            bytes.extend_from_slice(&chunk);
        }
        #[cfg(any(test,feature="g3-bank-latency-fixture"))]bank_timing::after_response(timing,&bytes).await;
        Ok(bytes)
    }
    /// Serialized by &mut ownership: one request, no hidden queue and no automatic retry.
    /// Cancellation of the future leaves this owner blocked; retire it via the owner fence.
    pub async fn call(&mut self, bytes: Vec<u8>) -> Result<wire::Response, String> {
        let request = protocol_g3::request(&bytes).map_err(|_| "Invalid canonical G3 request")?;
        if request.gold_intent.is_some()||request.item_deposit.is_some()||request.item_claim.is_some()||request.bank_batch.as_ref().and_then(|v|v.entries.as_ref()).is_some_and(|entries|entries.iter().any(|e|e.gold_intent.is_some()||e.item_deposit.is_some()||e.item_claim.is_some())) {
            return Err("Gold intents require the flushed ORIGINAL submission path".into());
        }
        let ticket = self.plan(&bytes, Instant::now())?;
        let response = self.exchange(bytes).await;
        self.complete(ticket, response, Instant::now())
    }
    pub async fn submit_item(&mut self,consumer:&mut crate::g3_local_consumer::LocalConsumer,bytes:Vec<u8>)->Result<wire::Response,String>{
        self.submit_item_via(consumer,bytes,|owner,bytes|Box::pin(owner.exchange(bytes))).await
    }
    pub(crate) async fn submit_item_via<F>(&mut self,consumer:&mut crate::g3_local_consumer::LocalConsumer,bytes:Vec<u8>,transport:F)->Result<wire::Response,String>
    where F:for<'a>FnOnce(&'a Self,Vec<u8>)->std::pin::Pin<Box<dyn std::future::Future<Output=Result<Vec<u8>,String>>+Send+'a>>{
        let request=protocol_g3::request(&bytes).map_err(|_|"Invalid canonical item intent")?;consumer.validate_item_request(&self.binding,&request)?;let generation=consumer.generation();let ticket=self.plan(&bytes,Instant::now())?;let result=transport(self,bytes).await;let response=self.complete(ticket,result,Instant::now())?;
        self.active()?;if consumer.generation()!=generation{return Err("Retired item consumer response".into())}consumer.journal.accept_item_operation(response.item_operation.as_ref().ok_or("Missing item operation")?)?;Ok(response)
    }
    /// LocalConsumer.begin_gold has already flushed ORIGINAL before this request.
    /// No effect permit or positive receipt is produced by a network success.
    pub async fn submit_gold(&mut self, consumer: &mut crate::g3_local_consumer::LocalConsumer,
        bytes: Vec<u8>) -> Result<wire::Response, String> {
        self.submit_gold_via(consumer, bytes, |owner, bytes| Box::pin(owner.exchange(bytes))).await
    }
    async fn submit_gold_via<F>(&mut self, consumer: &mut crate::g3_local_consumer::LocalConsumer,
        bytes: Vec<u8>, transport: F) -> Result<wire::Response, String>
    where F: for<'a> FnOnce(&'a Self, Vec<u8>) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<u8>, String>> + Send + 'a>> {
        let request = protocol_g3::request(&bytes).map_err(|_| "Invalid gold request")?;
        consumer.validate_gold_request(&self.binding, &request)?;
        let generation = consumer.generation();
        let ticket = self.plan(&bytes, Instant::now())?;
        let result = transport(self, bytes).await;
        let response = self.complete(ticket, result, Instant::now())?;
        self.accept_gold_response(consumer, &response, generation)?;
        Ok(response)
    }
    fn accept_gold_response(&self, consumer: &mut crate::g3_local_consumer::LocalConsumer,
        response: &wire::Response, generation: u64) -> Result<(), String> {
        self.active()?;
        consumer.accept_operation(response.operation.as_ref().ok_or("Missing gold operation")?, generation)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    include!("g3_http_delete_test.rs");
    include!("g3_http_item_test.rs");
    include!("g3_http_batch_test.rs");
    fn owner() -> HttpOwner {
        HttpOwner::new(Binding {
            context: [1; 16], service_origin: "https://fixture.invalid".into(), service_id: "service".into(),
            service_epoch: "epoch".into(), session_epoch: "session".into(), principal_id: "principal".into(),
            scope: wire::Scope { room_id: Some("room".into()), run_id: Some("run".into()),
                run_epoch: Some("run-epoch".into()), config_revision: Some(1),
                attachment_id: Some("attachment".into()), world_token: Some("world-é›ª".into()) },
        }, Zeroizing::new("fixture-token".into()), Arc::new(AtomicU64::new(1))).unwrap()
    }
    fn request(owner: &HttpOwner) -> wire::Request {
        wire::Request { major: Some(1), revision: Some(protocol_g3::REVISION.into()),
            service_id: Some(owner.binding.service_id.clone()), service_epoch: Some(owner.binding.service_epoch.clone()),
            session_epoch: Some(owner.binding.session_epoch.clone()), request_id: Some(uuid::Uuid::new_v4().to_string()),
            scope: Some(owner.binding.scope.clone()), ghost_list: Some(wire::GhostList::default()), ..Default::default() }
    }
    fn reply(request: &wire::Request) -> Vec<u8> {
        serde_json::to_vec(&wire::Response { major: Some(1), revision: Some(protocol_g3::REVISION.into()),
            service_id: request.service_id.clone(), service_epoch: request.service_epoch.clone(),
            request_id: request.request_id.clone(), ghosts: Some(vec![]), ..Default::default() }).unwrap()
    }
    #[test]
    fn frozen_request_authority_and_endpoint_restrictions() {
        let mut owner = owner();
        let request = request(&owner);
        for field in ["serviceId", "serviceEpoch", "sessionEpoch"] {
            let mut value = serde_json::to_value(&request).unwrap(); value[field] = serde_json::json!("substituted");
            assert!(owner.plan(&serde_json::to_vec(&value).unwrap(), Instant::now()).is_err());
        }
        for field in ["roomId", "runId", "runEpoch", "attachmentId", "worldToken", "configRevision"] {
            let mut value = serde_json::to_value(&request).unwrap();
            value["scope"][field] = if field == "configRevision" { serde_json::json!(2) } else { serde_json::json!("substituted") };
            assert!(owner.plan(&serde_json::to_vec(&value).unwrap(), Instant::now()).is_err());
        }
        for origin in ["http://fixture.invalid", "https://user@fixture.invalid", "https://fixture.invalid/path", "https://fixture.invalid?token=secret"] {
            let mut binding = owner.binding.clone(); binding.service_origin = origin.into();
            assert!(HttpOwner::new(binding.clone(), Zeroizing::new("fixture-token".into()), owner.fence.clone()).is_err());
            assert!(HttpOwner::from_verified_client(binding, Zeroizing::new("fixture-token".into()), owner.fence.clone(),owner.http.clone()).is_err());
        }
        let verified=HttpOwner::from_verified_client(owner.binding.clone(),Zeroizing::new("fixture-token".into()),owner.fence.clone(),owner.http.clone()).unwrap();
        assert!(verified.active().is_ok());owner.fence.fetch_add(1,Ordering::AcqRel);assert!(verified.active().is_err());
    }
    #[test]
    fn correlation_generation_late_reply_and_unexpected_payload_are_inert() {
        for field in ["requestId", "serviceId", "serviceEpoch", "revision"] {
            let mut owner = owner(); let req = request(&owner); let now = Instant::now();
            let ticket = owner.plan(&serde_json::to_vec(&req).unwrap(), now).unwrap();
            let mut response: serde_json::Value = serde_json::from_slice(&reply(&req)).unwrap();
            response[field] = serde_json::json!("substituted");
            assert!(owner.complete(ticket, Ok(serde_json::to_vec(&response).unwrap()), now).is_err());
            assert!(owner.in_flight.is_none());
        }
        let mut owner = owner(); let req = request(&owner); let now = Instant::now();
        let ticket = owner.plan(&serde_json::to_vec(&req).unwrap(), now).unwrap();
        assert!(owner.complete(ticket, Ok(reply(&req)), now + GHOST_DEADLINE + Duration::from_millis(1)).is_err());
        let ticket = owner.plan(&serde_json::to_vec(&req).unwrap(), now + Duration::from_secs(1)).unwrap();
        owner.fence.fetch_add(1, Ordering::AcqRel);
        assert!(owner.complete(ticket, Ok(reply(&req)), now + Duration::from_secs(1)).is_err());
        assert!(owner.plan(&serde_json::to_vec(&req).unwrap(), now + Duration::from_secs(2)).is_err());
    }
    #[test]
    fn no_queue_rate_bounds_and_lost_response_keep_request_identity() {
        let mut owner = owner(); let mut req = request(&owner); let now = Instant::now();
        let bytes = serde_json::to_vec(&req).unwrap();
        let ticket = owner.plan(&bytes, now).unwrap();
        assert!(owner.plan(&bytes, now).is_err());
        assert!(owner.complete(ticket, Err("lost response".into()), now).is_err());
        assert!(owner.plan(&bytes, now + Duration::from_millis(99)).is_err());
        let ticket = owner.plan(&bytes, now + Duration::from_millis(100)).unwrap();
        assert!(owner.complete(ticket, Ok(reply(&req)), now + Duration::from_millis(100)).is_ok());
        req.ghost_list = None; req.outbox_list = Some(wire::OutboxList::default());
        let bytes = serde_json::to_vec(&req).unwrap();
        for _ in 2..20 {
            let ticket = owner.plan(&bytes, now + Duration::from_millis(200)).unwrap();
            owner.complete(ticket, Err("injected refusal".into()), now + Duration::from_millis(200)).unwrap_err();
        }
        assert!(owner.plan(&bytes, now + Duration::from_millis(999)).is_err());
        assert!(owner.plan(&bytes, now + Duration::from_secs(1)).is_ok());
    }
    #[test]
    fn independent_core_generation_and_existing_lease_fence_late_reply() {
        let mut owner=owner();let core=Arc::new(AtomicU64::new(7));
        owner.guard(core.clone(),7,Instant::now()+Duration::from_secs(1));
        let request=request(&owner);let now=Instant::now();
        let ticket=owner.plan(&serde_json::to_vec(&request).unwrap(),now).unwrap();
        core.fetch_add(1,Ordering::AcqRel);
        assert!(owner.complete(ticket,Ok(reply(&request)),now).is_err());
        owner.guard(core,8,Instant::now());assert!(owner.active().is_err());
    }
    #[test]
    fn renewed_core_lease_can_refresh_same_owner_without_resetting_rate_budget(){
        let mut owner=owner();let core=Arc::new(AtomicU64::new(1));
        owner.guard(core.clone(),1,Instant::now());assert!(owner.active().is_err());
        owner.guard(core,1,Instant::now()+Duration::from_secs(1));
        owner.refresh_verified_token(Zeroizing::new("renewed-fixture-token".into())).unwrap();
        let request=request(&owner);let now=Instant::now();let bytes=serde_json::to_vec(&request).unwrap();
        let ticket=owner.plan(&bytes,now).unwrap();owner.complete(ticket,Ok(reply(&request)),now).unwrap();
        assert!(owner.plan(&bytes,now+Duration::from_millis(99)).is_err());
    }
    #[test]
    fn schema_bounded_error_and_wrong_payload_responses_are_refused() {
        for extra in [serde_json::json!({"operation": {}}), serde_json::json!({"statistics": []}),
            serde_json::json!({"error": {"code": "FORBIDDEN"}}), serde_json::json!({"outbox": []})] {
            let mut owner = owner(); let req = request(&owner); let now = Instant::now();
            let ticket = owner.plan(&serde_json::to_vec(&req).unwrap(), now).unwrap();
            let mut response: serde_json::Value = serde_json::from_slice(&reply(&req)).unwrap();
            response.as_object_mut().unwrap().extend(extra.as_object().unwrap().clone());
            assert!(owner.complete(ticket, Ok(serde_json::to_vec(&response).unwrap()), now).is_err());
        }
        let mut owner = owner(); let req = request(&owner); let now = Instant::now();
        let ticket = owner.plan(&serde_json::to_vec(&req).unwrap(), now).unwrap();
        assert!(owner.complete(ticket, Ok(vec![b' '; MAX_BYTES + 1]), now).is_err());
    }
    #[test]
    fn correlated_gold_digest_and_local_generation_gate_precede_any_effect() {
        use crate::{g3_journal::{Journal, operation_digest}, g3_local_consumer::LocalConsumer};
        let dir = std::env::temp_dir().join(format!("nt2-g3-http-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&dir).unwrap();
        let path = dir.join("operations.jsonl");
        let mut owner = owner();
        let mut consumer = LocalConsumer::new(Journal::open(&path).unwrap());
        consumer.bind(owner.binding.clone()).unwrap();
        let generation = consumer.generation();
        let bytes = consumer.begin_gold("withdraw", 100, generation).unwrap();
        let original = consumer.journal.pending().next().unwrap().record.original.clone();
        let now = Instant::now();
        let ticket = owner.plan(&bytes, now).unwrap();
        let mut operation = wire::Operation {
            operation_id: Some(original.operation_id.clone()), principal_id: Some(original.principal_id.clone()),
            run_id: owner.binding.scope.run_id.clone(), attachment_id: owner.binding.scope.attachment_id.clone(),
            world_token: owner.binding.scope.world_token.clone(), direction: Some(original.direction.clone()),
            amount: Some(original.amount), state: Some("reserved".into()),
            delivery_id: Some(uuid::Uuid::new_v4().to_string()), payload_digest: Some("a".repeat(64)), ..Default::default() };
        let mut response: wire::Response = serde_json::from_slice(&reply(&ticket.request)).unwrap();
        response.ghosts = None; response.operation = Some(operation.clone());
        let response = owner.complete(ticket, Ok(serde_json::to_vec(&response).unwrap()), now).unwrap();
        assert!(owner.accept_gold_response(&mut consumer, &response, generation).is_err());
        assert!(consumer.journal.prepare_effect(&original.operation_id).is_err());
        operation.payload_digest = Some(operation_digest(&original).unwrap());
        let response = wire::Response { operation: Some(operation), ..response };
        owner.accept_gold_response(&mut consumer, &response, generation).unwrap();
        // The accepted network response itself does not apply anything or create a bank receipt.
        assert_eq!(consumer.journal.pending().next().unwrap().record.phase, crate::g3_journal::Phase::ServerAccepted);
        consumer.stop().unwrap();
        assert!(owner.accept_gold_response(&mut consumer, &response, generation).is_err());
        drop(consumer);
        let mut restored = Journal::open(&path).unwrap();
        assert!(restored.uncertain(&original.operation_id));
        assert!(restored.prepare_effect(&original.operation_id).is_err());
        drop(restored);
        std::fs::remove_file(path).unwrap(); std::fs::remove_dir(dir).unwrap();
    }
    #[test]
    fn changed_or_unjournalled_gold_never_reaches_transport_or_rate_state() {
        use crate::{g3_journal::Journal, g3_local_consumer::LocalConsumer};
        let dir = std::env::temp_dir().join(format!("nt2-g3-preflight-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&dir).unwrap(); let path = dir.join("operations.jsonl");
        let mut owner = owner(); let mut consumer = LocalConsumer::new(Journal::open(&path).unwrap());
        consumer.bind(owner.binding.clone()).unwrap();
        let bytes = consumer.begin_gold("withdraw", 100, consumer.generation()).unwrap();
        let original: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        let sent = Arc::new(AtomicU64::new(0));
        let mut mutations = Vec::new();
        for field in ["direction", "amount", "operationId"] {
            let mut value = original.clone();
            value["goldIntent"][field] = match field { "amount" => serde_json::json!(101),
                "direction" => serde_json::json!("deposit"), _ => serde_json::json!(uuid::Uuid::new_v4().to_string()) };
            mutations.push(value);
        }
        for field in ["serviceId", "serviceEpoch", "sessionEpoch"] {
            let mut value = original.clone(); value[field] = serde_json::json!("changed"); mutations.push(value);
        }
        for field in ["roomId", "runId", "runEpoch", "configRevision", "attachmentId", "worldToken"] {
            let mut value = original.clone();
            value["scope"][field] = if field == "configRevision" { serde_json::json!(2) } else { serde_json::json!("changed") };
            mutations.push(value);
        }
        for value in mutations {
            let spy = sent.clone();
            let result = tauri::async_runtime::block_on(owner.submit_gold_via(&mut consumer,
                serde_json::to_vec(&value).unwrap(), move |_, _| {
                    spy.fetch_add(1, Ordering::AcqRel); Box::pin(async { Err("transport spy".into()) })
                }));
            assert!(result.is_err()); assert_eq!(sent.load(Ordering::Acquire), 0);
            assert_eq!(owner.count, 0); assert!(owner.in_flight.is_none());
        }
        // Even a valid flushed intent cannot bypass the dedicated guarded path.
        assert!(tauri::async_runtime::block_on(owner.call(bytes.clone())).is_err());
        assert_eq!(owner.count, 0);
        let spy = sent.clone();
        assert!(tauri::async_runtime::block_on(owner.submit_gold_via(&mut consumer, bytes.clone(), move |_, _| {
            spy.fetch_add(1, Ordering::AcqRel); Box::pin(async { Err("lost response".into()) })
        })).is_err());
        assert_eq!(sent.load(Ordering::Acquire), 1); assert_eq!(consumer.journal.pending().count(), 1);
        // A replacement owner/consumer must not retag the old original into a new world.
        consumer.stop().unwrap(); let mut next = owner.binding.clone(); next.context = [2; 16];
        next.scope.world_token = Some("next-world".into()); consumer.bind(next.clone()).unwrap();
        let mut next_owner = HttpOwner::new(next, Zeroizing::new("fixture-token".into()), owner.fence.clone()).unwrap();
        let spy = sent.clone();
        assert!(tauri::async_runtime::block_on(next_owner.submit_gold_via(&mut consumer, bytes, move |_, _| {
            spy.fetch_add(1, Ordering::AcqRel); Box::pin(async { Err("transport spy".into()) })
        })).is_err());
        assert_eq!(sent.load(Ordering::Acquire), 1); assert_eq!(next_owner.count, 0);
        drop(consumer); std::fs::remove_file(path).unwrap(); std::fs::remove_dir(dir).unwrap();
    }
}

#[cfg(test)]
#[path = "g3_https_test.rs"]
pub(crate) mod https_test;

/// Authenticated desktop original-status recovery, including scoped terminal
/// deletion evidence. No game rejoin, receipts or effect-permit API.
pub struct RecoveryOwner {
    origin: String, service_id: String, service_epoch: String, session_epoch: String,
    principal_id: String, token: Zeroizing<String>, fence: Arc<AtomicU64>, generation: u64,
    roots: Option<Vec<reqwest::Certificate>>,
    last_call: Option<Instant>,
    verified_client: Option<reqwest::Client>,
}
pub enum OriginalStatus{Operation(wire::Operation),ItemOperation(wire::ItemOperation),LobbyDeleted(crate::g3_journal::DeletedLobby)}
impl RecoveryOwner {
    /// Current desktop credentials, retained Race world. This sends observations
    /// or reads only; it cannot enter/release/resurrect a game actor.
    pub async fn race(&mut self,lobby:&crate::g3_journal::LobbyIdentity,scope:Option<crate::protocol_race::wire::Scope>,generation:u32,commit:Option<crate::protocol_race::wire::Commit>,read:Option<crate::protocol_race::wire::Read>)->Result<crate::protocol_race::wire::Response,String>{
        use crate::protocol_race::{self as p,wire as r};
        let active=||self.fence.load(Ordering::Acquire)==self.generation;
        if !active()||lobby.service_origin!=self.origin||lobby.service_id!=self.service_id||lobby.principal_id!=self.principal_id||commit.is_some()==read.is_some()||commit.is_some()&&(generation==0||scope.as_ref().and_then(|s|s.room_id.as_deref())!=Some(&lobby.room_id)){return Err("Retired or mismatched Race desktop owner".into())}
        let request=r::Request{major:Some(1),revision:Some(p::REVISION.into()),service_id:Some(self.service_id.clone()),service_epoch:Some(self.service_epoch.clone()),session_epoch:Some(self.session_epoch.clone()),request_id:Some(uuid::Uuid::new_v4().to_string()),scope,owner_generation:commit.as_ref().map(|_|generation),commit,read};
        let bytes=serde_json::to_vec(&request).map_err(|_|"Race request encoding")?;p::request(&bytes).map_err(|_|"Race canonical request")?;
        let http=self.verified_client.as_ref().ok_or("Verified Race HTTP owner absent")?;let expected=format!("{}/rpc/g3/race",self.origin);
        tokio::time::timeout(DEADLINE,async{
            let mut response=http.post(&expected).bearer_auth(self.token.as_str()).header("content-type","application/json").body(bytes).send().await.map_err(|_|"Race connection unavailable")?;
            if !active()||response.url().as_str()!=expected||!response.status().is_success()||response.headers().get("content-type").and_then(|v|v.to_str().ok()).is_none_or(|v|v.split(';').next()!=Some("application/json")){return Err("Race HTTP response refused".into())}
            let mut bytes=Vec::new();while let Some(part)=response.chunk().await.map_err(|_|"Race response interrupted")?{if !active()||bytes.len()+part.len()>262144{return Err("Race response owner/bound".into())}bytes.extend_from_slice(&part)}
            let response=p::response(&bytes).map_err(|_|"Race canonical response")?;
            if !active()||response.service_id!=request.service_id||response.service_epoch!=request.service_epoch||response.request_id!=request.request_id{return Err("Race response correlation".into())}
            if let Some(e)=response.error{return Err(e.code.unwrap_or("Race refused".into()))}
            if request.commit.is_some()&&(response.receipt.is_none()||response.snapshot.is_some())||request.read.is_some()&&(response.snapshot.is_none()||response.receipt.is_some()){return Err("Race unexpected response payload".into())}Ok(response)
        }).await.map_err(|_|"Race whole request deadline")?
    }
    /// Inputs come from current authenticated desktop/service discovery, not saved app authority.
    pub fn new(origin: String, service_id: String, service_epoch: String, session_epoch: String,
        principal_id: String, token: Zeroizing<String>, fence: Arc<AtomicU64>) -> Self {
        Self { origin, service_id, service_epoch, session_epoch, principal_id, token,
            generation: fence.load(Ordering::Acquire), fence, roots: None, last_call: None, verified_client:None }
    }
    pub fn with_verified_client(mut self,http:reqwest::Client)->Self{self.verified_client=Some(http);self}
    pub fn owns_original(&self,o:&crate::g3_journal::Original)->bool{self.fence.load(Ordering::Acquire)==self.generation&&o.service_origin==self.origin&&o.service_id==self.service_id&&o.principal_id==self.principal_id}
    pub(crate) fn cleanup_owner_key(&self)->String{serde_json::to_string(&(&self.origin,&self.service_id,&self.principal_id,&self.session_epoch,self.generation)).unwrap()}
    pub async fn c2_source_outcome(&mut self,lobby:&crate::g3_journal::LobbyIdentity,original:serde_json::Value,receipt:Option<crate::protocol_c2::wire::SourceReceipt>)->Result<crate::protocol_c2::wire::SourceStatus,String>{
        use crate::protocol_c2::{self as p,wire as c};
        if self.fence.load(Ordering::Acquire)!=self.generation||lobby.service_origin!=self.origin||lobby.service_id!=self.service_id||lobby.principal_id!=self.principal_id{return Err("Retired C2 desktop owner".into())}
        let original:c::SourceOriginal=p::typed("SourceOriginal",&serde_json::to_vec(&original).map_err(|_|"C2 retained encoding")?).map_err(|_|"C2 retained original")?;
        if original.room_id.as_deref()!=Some(&lobby.room_id){return Err("C2 retained room differs".into())}
        let binding=Binding{context:[0;16],service_origin:self.origin.clone(),service_id:self.service_id.clone(),service_epoch:self.service_epoch.clone(),session_epoch:self.session_epoch.clone(),principal_id:self.principal_id.clone(),scope:wire::Scope{room_id:original.room_id.clone(),run_id:original.run_id.clone(),world_token:original.world_token.clone(),config_revision:original.config_revision,..Default::default()}};
        let mut owner=HttpOwner::from_verified_client(binding,Zeroizing::new(self.token.to_string()),self.fence.clone(),self.verified_client.clone().ok_or("Verified C2 HTTP owner absent")?)?;owner.generation=self.generation;
        let response=owner.c2(c::Request{major:Some(1),revision:Some(p::REVISION.into()),service_id:Some(self.service_id.clone()),service_epoch:Some(self.service_epoch.clone()),session_epoch:Some(self.session_epoch.clone()),request_id:Some(uuid::Uuid::new_v4().to_string()),source_outcome:Some(c::SourceOutcome{original:Some(original.clone()),receipt:receipt.clone()}),..Default::default()}).await?;
        let status=response.source_status.ok_or("C2 source status absent")?;
        if serde_json::to_value(&status.original).unwrap()!=serde_json::to_value(&original).unwrap()||status.effect_authority!=Some(false)||status.saved_effect_proof!=Some(false)||status.self_reported!=Some(true){return Err("C2 retained outcome authority/original differs".into())}
        if status.coverage.as_deref()==Some("durable")&&status.committed==Some(true){
            if status.sequence.as_deref().and_then(|v|v.parse::<u64>().ok()).is_none_or(|v|v==0){return Err("C2 committed source sequence absent".into())}
            if let Some(receipt)=receipt{if status.disposition!=receipt.state{return Err("C2 source receipt differs".into())}}
        }else if status.coverage.as_deref()!=Some("absent")||status.committed!=Some(false){return Err("C2 inconsistent source coverage".into())}
        Ok(status)
    }
    pub async fn c1_source_outcome(&mut self,lobby:&crate::g3_journal::LobbyIdentity,original:serde_json::Value,receipt:Option<crate::protocol_c1::wire::SourceDisposition>)->Result<crate::protocol_c1::wire::SourceOutcome,String>{
        use crate::protocol_c1::{self as p,wire as c};
        if self.fence.load(Ordering::Acquire)!=self.generation||lobby.service_origin!=self.origin||lobby.service_id!=self.service_id||lobby.principal_id!=self.principal_id{return Err("Retired C1 desktop owner".into())}
        let original:c::SourceOriginal=serde_json::from_value(original).map_err(|_|"C1 retained original encoding")?;
        if original.room_id.as_deref()!=Some(&lobby.room_id)||original.publish.as_ref().and_then(|p|p.payload.as_ref()).and_then(|p|p.death.as_ref()).is_none(){return Err("C1 retained death original scope".into())}
        let binding=Binding{context:[0;16],service_origin:self.origin.clone(),service_id:self.service_id.clone(),service_epoch:self.service_epoch.clone(),session_epoch:self.session_epoch.clone(),principal_id:self.principal_id.clone(),scope:wire::Scope{room_id:original.room_id.clone(),run_id:original.run_id.clone(),world_token:original.world_token.clone(),config_revision:original.config_revision,..Default::default()}};
        let mut owner=HttpOwner::from_verified_client(binding,Zeroizing::new(self.token.to_string()),self.fence.clone(),self.verified_client.clone().ok_or("Verified C1 desktop HTTP owner absent")?)?;owner.generation=self.generation;
        let response=owner.c1(c::Request{major:Some(1),revision:Some(p::REVISION.into()),service_id:Some(self.service_id.clone()),service_epoch:Some(self.service_epoch.clone()),session_epoch:Some(self.session_epoch.clone()),request_id:Some(uuid::Uuid::new_v4().to_string()),source_outcome:Some(c::SourceOutcomeRequest{original:Some(original.clone()),receipt:receipt.clone()}),..Default::default()}).await?;
        c1_source::validate(&original,receipt.as_ref(),response)
    }
    fn for_original(&mut self, journal: &impl crate::g3_journal::JournalView, id: &str) -> Result<HttpOwner, String> {
        if self.fence.load(Ordering::Acquire) != self.generation { return Err("Retired desktop recovery".into()); }
        let original = &journal.originals().into_iter().find(|p| p.record.original.operation_id == id)
            .ok_or("Recovery has no retained ORIGINAL")?.record.original;
        if original.service_origin != self.origin || original.service_id != self.service_id
            || original.principal_id != self.principal_id { return Err("Recovery desktop does not own ORIGINAL".into()); }
        let scope = protocol_g3::typed("Scope", original.scope_json.as_bytes()).map_err(|_| "Invalid original scope")?;
        let binding=Binding { context: [0; 16], service_origin: self.origin.clone(),
            service_id: self.service_id.clone(), service_epoch: self.service_epoch.clone(),
            session_epoch: self.session_epoch.clone(), principal_id: self.principal_id.clone(), scope };
        let token=Zeroizing::new(self.token.to_string());
        let mut owner=if let Some(http)=&self.verified_client{
            HttpOwner::from_verified_client(binding,token,self.fence.clone(),http.clone())?
        }else{HttpOwner::with_roots(binding,token,self.fence.clone(),self.roots.clone())?};
        // Keep the recovery owner's initiating generation, even if cancellation races construction.
        owner.generation = self.generation;
        let now = Instant::now();
        if self.last_call.is_some_and(|at| now.saturating_duration_since(at) < Duration::from_millis(100)) {
            return Err("Recovery polling rate capacity".into());
        }
        self.last_call = Some(now);
        Ok(owner)
    }
    fn request(owner: &HttpOwner) -> wire::Request {
        wire::Request { major: Some(1), revision: Some(protocol_g3::REVISION.into()),
            service_id: Some(owner.binding.service_id.clone()), service_epoch: Some(owner.binding.service_epoch.clone()),
            session_epoch: Some(owner.binding.session_epoch.clone()), request_id: Some(uuid::Uuid::new_v4().to_string()),
            scope: Some(owner.binding.scope.clone()), ..Default::default() }
    }
    pub async fn status(&mut self, journal: &impl crate::g3_journal::JournalView, id: &str) -> Result<wire::Operation, String> {
        match self.status_or_deleted(journal,id).await?{OriginalStatus::Operation(op)=>Ok(op),OriginalStatus::ItemOperation(_)=>Err("Expected gold original".into()),OriginalStatus::LobbyDeleted(_)=>Err("ROOM_DELETED".into())}
    }
    /// Only a correlated authenticated goldStatus refusal can construct the
    /// deletion proof. NOT_FOUND/absence/other errors have no destructive path.
    pub async fn status_or_deleted(&mut self,journal:&impl crate::g3_journal::JournalView,id:&str)->Result<OriginalStatus,String>{
        self.status_or_deleted_via(journal,id,|owner,bytes|Box::pin(owner.exchange(bytes))).await
    }
    async fn status_or_deleted_via<F>(&mut self,journal:&impl crate::g3_journal::JournalView,id:&str,transport:F)->Result<OriginalStatus,String>
    where F:for<'a> FnOnce(&'a HttpOwner,Vec<u8>)->std::pin::Pin<Box<dyn std::future::Future<Output=Result<Vec<u8>,String>>+Send+'a>>{
        let mut owner = self.for_original(journal, id)?;
        let mut request = Self::request(&owner);
        let item=journal.originals().into_iter().find(|p|p.record.original.operation_id==id).is_some_and(|p|p.record.original.item.is_some());
        if item{request.item_status=Some(wire::ItemStatus{operation_id:Some(id.into())});}else{request.gold_status = Some(wire::GoldStatus { operation_id: Some(id.into()) });}
        let bytes=serde_json::to_vec(&request).map_err(|_|"Recovery encode failed")?;
        let ticket=owner.plan(&bytes,Instant::now())?;
        let bytes=transport(&owner,bytes).await;
        let response = match owner.complete(ticket,bytes,Instant::now()){
            Ok(response)=>response,
            Err(code)if code=="ROOM_DELETED"=>{owner.active()?;let key=crate::g3_journal::LobbyIdentity{service_origin:owner.binding.service_origin.clone(),service_id:owner.binding.service_id.clone(),principal_id:owner.binding.principal_id.clone(),room_id:owner.binding.scope.room_id.clone().ok_or("Deleted status room absent")?};return Ok(OriginalStatus::LobbyDeleted(crate::g3_journal::DeletedLobby::authenticated(key)?))},
            Err(error)=>return Err(error),
        };
        if item{let op=response.item_operation.ok_or("Missing recovered item operation")?;journal.check_item_status(&op)?;return Ok(OriginalStatus::ItemOperation(op))}
        let operation = response.operation.ok_or("Missing recovered operation")?;
        journal.check_status(&operation)?;
        Ok(OriginalStatus::Operation(operation))
    }
    pub async fn outbox(&mut self, journal: &impl crate::g3_journal::JournalView, original_id: &str,
        after: &str) -> Result<wire::Response, String> {
        // Canonical decimal cursor; bound page size, no implicit polling loop.
        if after.is_empty() || !after.bytes().all(|b| b.is_ascii_digit()) || after.parse::<u64>().is_err()
            || after.len() > 20 || (after.len() > 1 && after.starts_with('0')) {
            return Err("Invalid recovery cursor".into());
        }
        let mut owner = self.for_original(journal, original_id)?;
        let mut request = Self::request(&owner);
        request.outbox_list = Some(wire::OutboxList { after_sequence: Some(after.into()), limit: Some(32) });
        let response = owner.call(serde_json::to_vec(&request).map_err(|_| "Recovery encode failed")?).await?;
        let mut last = after.parse::<u64>().unwrap();
        for entry in response.outbox.as_deref().unwrap_or(&[]) {
            let raw_sequence = entry.sequence.as_deref().ok_or("Missing outbox sequence")?;
            let sequence = raw_sequence.parse::<u64>().map_err(|_| "Invalid outbox sequence")?;
            if sequence.to_string() != raw_sequence { return Err("Noncanonical outbox sequence".into()); }
            if sequence <= last { return Err("Outbox sequence replay".into()); } last = sequence;
            if let Some(operation) = &entry.operation { journal.check_status(operation)?; }
            // Event consumption is outside this bank-only recovery path.
            if entry.event.is_some() { return Err("Unsupported recovery event; retain cursor".into()); }
        }
        if response.next_sequence.as_deref() != Some(last.to_string().as_str()) {
            return Err("Recovery cursor mismatch".into());
        }
        Ok(response)
    }
}
