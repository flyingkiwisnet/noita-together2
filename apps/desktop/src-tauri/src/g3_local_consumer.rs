//! Candidate local/native -> canonical request consumer. No network or bearer authority.
//! The future authenticated adapter must provide the frozen Start binding, own credentials,
//! correlate responses/generation and verify service authorization before any journal effect.
use crate::{
    g3_journal::{Journal, SharedJournal, Original},
    g3_native_frame,
    protocol_g3::{self, wire},
};

#[derive(Clone)]
pub struct Binding {
    pub context: [u8; 16],
    pub service_origin: String,
    pub service_id: String,
    pub service_epoch: String,
    pub session_epoch: String,
    pub principal_id: String,
    pub scope: wire::Scope,
}
#[derive(Clone)]
pub struct LocalConsumer {
    pub journal: SharedJournal,
    binding: Option<Binding>,
    last_context: Option<[u8; 16]>,
    last_world: Option<String>,
    last_binding:Option<Binding>,
    recovery_generation:u32,
    sequence: u32,
    generation: u64,
}
impl LocalConsumer {
    pub fn new(journal: Journal) -> Self {
        Self {
            journal: SharedJournal::new(journal),
            binding: None,
            last_context: None,
            last_world: None,
            last_binding:None,
            recovery_generation:0,
            sequence: 0,
            generation: 0,
        }
    }
    pub fn ownership_generation(&self)->u32{self.recovery_generation.max(1)}
    pub fn generation(&self) -> u64 {
        self.generation
    }
    /// Must run before transport or rate mutation. A restored original is status-only.
    pub(crate) fn validate_gold_request(&self, owner: &Binding, request: &wire::Request) -> Result<(), String> {
        let active = self.binding.as_ref().ok_or("No active gold owner")?;
        if active.context != owner.context || active.service_origin != owner.service_origin
            || active.principal_id != owner.principal_id
            || serde_json::to_vec(&Self::base(active)).unwrap() != serde_json::to_vec(&Self::base(owner)).unwrap() {
            return Err("Gold consumer and HTTP owner differ".into());
        }
        let intent = request.gold_intent.as_ref().ok_or("Expected original gold intent")?;
        let id = intent.operation_id.as_deref().ok_or("Missing original gold ID")?;
        let pending = self.journal.pending().find(|p| p.record.original.operation_id == id)
            .ok_or("Gold intent has no flushed original")?;
        if pending.restored { return Err("Restored gold original requires status only".into()); }
        let original = &pending.record.original;
        if original.item.is_some(){return Err("Item original cannot become a gold intent".into())}
        let scope: wire::Scope = protocol_g3::typed("Scope", original.scope_json.as_bytes()).map_err(|_| "Invalid original scope")?;
        if original.service_origin != owner.service_origin || original.principal_id != owner.principal_id
            || request.service_id.as_deref() != Some(&original.service_id)
            || request.service_epoch.as_deref() != Some(&original.service_epoch)
            || request.session_epoch.as_deref() != Some(&original.session_epoch)
            || serde_json::to_vec(&request.scope).unwrap() != serde_json::to_vec(&Some(scope)).unwrap()
            || intent.direction.as_deref() != Some(&original.direction) || intent.amount != Some(original.amount) {
            return Err("Gold intent differs from flushed ORIGINAL".into());
        }
        Ok(())
    }
    /// Only call after the owner HTTP adapter authenticates/correlates the response.
    pub fn accept_operation(&mut self, operation: &wire::Operation, generation: u64) -> Result<(), String> {
        if generation != self.generation || self.binding.is_none() {
            return Err("Retired local operation response".into());
        }
        self.journal.accept_operation(operation)
    }
    /// Trusted local owner input only: not a network entry point or authentication result.
    pub fn bind(&mut self, binding: Binding) -> Result<(), String> {
        if binding.context.iter().all(|b| *b == 0) {
            return Err("Empty native owner context".into());
        }
        let request = Self::base(&binding);
        let mut check = request.clone();
        check.ghost_list = Some(wire::GhostList::default());
        Self::encode(&check)?;
        if let Some(current) = &self.binding {
            if current.context == binding.context {
                if serde_json::to_vec(&Self::base(current)).unwrap()
                    != serde_json::to_vec(&request).unwrap()
                    || current.service_origin != binding.service_origin
                    || current.principal_id != binding.principal_id
                {
                    return Err("Native owner context cannot change frozen authority".into());
                }
                return Ok(());
            }
        } else if self.last_context == Some(binding.context) {
            return Err("Stopped context cannot be reopened".into());
        }
        if self.last_world == binding.scope.world_token
            || binding
                .scope
                .world_token
                .as_ref()
                .is_none_or(|id| id.is_empty())
        {
            return Err(
                "Sequence reset requires a genuinely new world; same-run recovery is not enabled"
                    .into(),
            );
        }
        self.generation = self
            .generation
            .checked_add(1)
            .ok_or("Generation exhausted")?;
        self.last_context = Some(binding.context);
        self.last_world = binding.scope.world_token.clone();
        self.recovery_generation=0;
        self.last_binding=Some(binding.clone());self.binding = Some(binding);
        self.sequence = 0;
        Ok(())
    }
    /// Trusted Core only, after fresh canonical admission and actual inspection.
    pub fn retained_world(&self)->Option<&str>{self.last_world.as_deref()}
    // LC1 alone can validate a new canonical game owner without replacing the
    // still-live native context. Legacy bind/rejoin rules remain unchanged.
    pub(crate) fn lifecycle_owner_replacement(&self,binding:&Binding,generation:u32)->Result<bool,String>{
        let Some(old)=self.binding.as_ref().filter(|old|old.context==binding.context)else{return Ok(false)};
        if old.service_origin==binding.service_origin&&old.principal_id==binding.principal_id&&serde_json::to_vec(&Self::base(old)).unwrap()==serde_json::to_vec(&Self::base(binding)).unwrap(){return Ok(false)}
        if generation<=self.recovery_generation||generation==0||old.service_origin!=binding.service_origin||old.service_id!=binding.service_id||old.service_epoch!=binding.service_epoch||old.principal_id!=binding.principal_id||old.scope.room_id!=binding.scope.room_id||old.scope.run_id!=binding.scope.run_id||old.scope.run_epoch!=binding.scope.run_epoch||old.scope.config_revision!=binding.scope.config_revision||old.scope.world_token!=binding.scope.world_token||old.scope.attachment_id==binding.scope.attachment_id||old.session_epoch==binding.session_epoch{return Err("LC1 replacement differs from current participation or canonical generation".into())}
        let mut check=Self::base(binding);check.ghost_list=Some(wire::GhostList::default());Self::encode(&check)?;self.generation.checked_add(1).ok_or("Generation exhausted")?;Ok(true)
    }
    pub(crate) fn bind_lifecycle_owner(&mut self,binding:Binding,generation:u32)->Result<(),String>{
        if !self.lifecycle_owner_replacement(&binding,generation)?{return Err("LC1 owner replacement not current".into())}
        self.generation+=1;self.sequence=0;self.recovery_generation=generation;self.last_context=Some(binding.context);self.last_world=binding.scope.world_token.clone();self.last_binding=Some(binding.clone());self.binding=Some(binding);Ok(())
    }
    /// Existing ledger originals remain untouched; only movement sequence resets.
    pub fn bind_recovered(&mut self,binding:Binding,generation:u32)->Result<(),String>{
        if generation==0||generation<self.recovery_generation{return Err("Stale recovered consumer generation".into())}
        if self.binding.as_ref().is_some_and(|old|old.context==binding.context){self.bind(binding)?;self.recovery_generation=generation;return Ok(());}
        if generation==0||generation<=self.recovery_generation{return Err("Stale recovered consumer generation".into())}
        if let Some(old)=&self.last_binding{
            if old.context==binding.context||old.service_origin!=binding.service_origin||old.service_id!=binding.service_id
                ||old.principal_id!=binding.principal_id||old.scope.room_id!=binding.scope.room_id||old.scope.run_id!=binding.scope.run_id
                ||old.scope.config_revision!=binding.scope.config_revision||old.scope.world_token!=binding.scope.world_token
                ||old.scope.attachment_id==binding.scope.attachment_id||old.session_epoch==binding.session_epoch{
                return Err("Recovered consumer differs from retained participation".into())
            }
        }
        let prior_world=self.last_world.take();
        if let Err(error)=self.bind(binding){self.last_world=prior_world;return Err(error)}
        self.recovery_generation=generation;Ok(())
    }
    pub fn stop(&mut self) -> Result<(), String> {
        self.generation = self
            .generation
            .checked_add(1)
            .ok_or("Generation exhausted")?;
        self.binding = None;
        self.sequence = 0;
        // Journal deliberately survives End, world replacement and installation switching.
        Ok(())
    }
    fn base(binding: &Binding) -> wire::Request {
        wire::Request {
            major: Some(1),
            revision: Some(protocol_g3::REVISION.into()),
            service_id: Some(binding.service_id.clone()),
            service_epoch: Some(binding.service_epoch.clone()),
            session_epoch: Some(binding.session_epoch.clone()),
            request_id: Some("local-validation".into()),
            scope: Some(binding.scope.clone()),
            ..Default::default()
        }
    }
    fn encode(request: &wire::Request) -> Result<Vec<u8>, String> {
        let bytes = serde_json::to_vec(request).map_err(|_| "Canonical request encode failed")?;
        protocol_g3::request(&bytes).map_err(|_| "Canonical request refused")?;
        Ok(bytes)
    }
    pub fn movement(&mut self, frame: &[u8], generation: u64) -> Result<Vec<u8>, String> {
        if generation != self.generation {
            return Err("Retired local work".into());
        }
        let binding = self.binding.as_ref().ok_or("No active Start binding")?;
        let pose = g3_native_frame::decode(frame, &binding.context, self.sequence)?;
        let sequence = pose.sequence.unwrap();
        let mut request = Self::base(binding);
        request.request_id = Some(uuid::Uuid::new_v4().to_string());
        request.movement = Some(wire::Movement {
            frames: Some(vec![pose]),
        });
        let bytes = Self::encode(&request)?;
        self.sequence = sequence;
        Ok(bytes)
    }
    pub fn begin_gold(
        &mut self,
        direction: &str,
        amount: u32,
        generation: u64,
    ) -> Result<Vec<u8>, String> {
        self.begin_gold_with_id(direction,amount,generation,uuid::Uuid::new_v4().to_string())
    }
    pub fn begin_gold_with_id(&mut self,direction:&str,amount:u32,generation:u64,operation_id:String)->Result<Vec<u8>,String>{
        if generation != self.generation {
            return Err("Retired local work".into());
        }
        let binding = self.binding.as_ref().ok_or("No active Start binding")?;
        let mut request = Self::base(binding);
        request.request_id = Some(uuid::Uuid::new_v4().to_string());
        request.gold_intent = Some(wire::GoldIntent {
            operation_id: Some(operation_id.clone()),
            direction: Some(direction.into()),
            amount: Some(amount),
        });
        let bytes = Self::encode(&request)?;
        self.journal.begin(Original { item:None,
            service_origin: binding.service_origin.clone(),
            service_id: binding.service_id.clone(),
            principal_id: binding.principal_id.clone(),
            service_epoch: binding.service_epoch.clone(),
            session_epoch: binding.session_epoch.clone(),
            scope_json: serde_json::to_string(&binding.scope).map_err(|_| "Scope encode failed")?,
            operation_id,
            direction: direction.into(),
            amount,
        })?;
        // Only return a request after its original is flushed. No timeout replacement API.
        Ok(bytes)
    }
    pub fn begin_item_with_id(&mut self,item:crate::g3_items::Original,generation:u64,operation_id:String)->Result<Vec<u8>,String>{
        if generation!=self.generation{return Err("Retired item work".into())}let binding=self.binding.as_ref().ok_or("No active item owner")?;
        let payload=crate::g3_items::validate_original(&item)?;let item=crate::g3_items::Original{kind:item.kind,payload_json:String::from_utf8(crate::g3_items::canonical(&payload)?).map_err(|_|"Item payload UTF8")?};
        let mut request=Self::base(binding);request.request_id=Some(uuid::Uuid::new_v4().to_string());
        if item.kind=="deposit"{request.item_deposit=Some(wire::ItemDeposit{operation_id:Some(operation_id.clone()),item:Some(serde_json::from_value(payload).map_err(|_|"Invalid deposit descriptor")?)});}
        else{request.item_claim=Some(wire::ItemClaim{operation_id:Some(operation_id.clone()),item_id:Some(payload["itemId"].as_str().unwrap().into()),expected_revision:Some(payload["expectedRevision"].as_str().unwrap().into())});}
        let bytes=Self::encode(&request)?;
        self.journal.begin(Original{item:Some(item.clone()),service_origin:binding.service_origin.clone(),service_id:binding.service_id.clone(),principal_id:binding.principal_id.clone(),service_epoch:binding.service_epoch.clone(),session_epoch:binding.session_epoch.clone(),scope_json:serde_json::to_string(&binding.scope).map_err(|_|"Item scope encoding")?,operation_id,direction:item.kind,amount:1})?;Ok(bytes)
    }
    pub(crate) fn validate_item_request(&self,owner:&Binding,request:&wire::Request)->Result<(),String>{
        let active=self.binding.as_ref().ok_or("No active item owner")?;
        if active.context!=owner.context||active.service_origin!=owner.service_origin||active.principal_id!=owner.principal_id||serde_json::to_vec(&Self::base(active)).unwrap()!=serde_json::to_vec(&Self::base(owner)).unwrap(){return Err("Item consumer and HTTP owner differ".into())}
        let id=request.item_deposit.as_ref().and_then(|q|q.operation_id.as_deref()).or_else(||request.item_claim.as_ref().and_then(|q|q.operation_id.as_deref())).ok_or("Missing item intent ID")?;
        let p=self.journal.pending().find(|p|p.record.original.operation_id==id).ok_or("Item intent has no flushed original")?;if p.restored{return Err("Restored item original is status only".into())}
        let original=&p.record.original;let item=original.item.as_ref().ok_or("Gold original cannot become item intent")?;
        let payload=if let Some(deposit)=&request.item_deposit{if item.kind!="deposit"{return Err("Item direction changed".into())}serde_json::to_value(crate::g3_items::descriptor(deposit.item.as_ref().ok_or("Item descriptor absent")?)?).map_err(|_|"Item encode")?}
            else{let claim=request.item_claim.as_ref().unwrap();if item.kind!="claim"{return Err("Item direction changed".into())}serde_json::json!({"itemId":claim.item_id,"expectedRevision":claim.expected_revision})};
        let scope:wire::Scope=serde_json::from_str(&original.scope_json).map_err(|_|"Original item scope")?;
        if original.service_origin!=owner.service_origin||original.principal_id!=owner.principal_id||request.service_id.as_deref()!=Some(&original.service_id)||request.service_epoch.as_deref()!=Some(&original.service_epoch)||request.session_epoch.as_deref()!=Some(&original.session_epoch)||serde_json::to_vec(&request.scope).unwrap()!=serde_json::to_vec(&Some(scope)).unwrap()||crate::g3_items::canonical(&payload)?!=crate::g3_items::canonical(&crate::g3_items::validate_original(item)?)?{return Err("Item intent differs from immutable original".into())}Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_to_generated_request_and_end_preserves_original() {
        let dir = std::env::temp_dir().join(format!("nt2-g3-consumer-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&dir).unwrap();
        let path = dir.join("operations.jsonl");
        let binding = Binding {
            context: [1; 16],
            service_origin: "https://fixture.invalid".into(),
            service_id: "g3-fixture".into(),
            service_epoch: "service-epoch".into(),
            session_epoch: "game-epoch".into(),
            principal_id: "account-a".into(),
            scope: wire::Scope {
                room_id: Some("room-a".into()),
                run_id: Some("run-a".into()),
                run_epoch: Some("run-epoch".into()),
                config_revision: Some(1),
                attachment_id: Some("attachment-a".into()),
                world_token: Some("world-a".into()),
            },
        };
        let mut consumer = LocalConsumer::new(Journal::open(&path).unwrap());
        consumer.bind(binding.clone()).unwrap();
        let generation = consumer.generation();
        let mut frame = [0; 64];
        frame[..8].copy_from_slice(b"NTG3\x01\x00\x01\x00");
        frame[8..24].fill(1);
        frame[24] = 1;
        let bytes = consumer.movement(&frame, generation).unwrap();
        assert_eq!(
            protocol_g3::request(&bytes)
                .unwrap()
                .movement
                .unwrap()
                .frames
                .unwrap()[0]
                .sequence,
            Some(1)
        );
        consumer.bind(binding.clone()).unwrap();
        assert!(consumer.movement(&frame, generation).is_err());
        let mut conflict = binding.clone();
        conflict.session_epoch = "other-epoch".into();
        assert!(consumer.bind(conflict).is_err());
        let gold = consumer.begin_gold("withdraw", 100, generation).unwrap();
        let intent = protocol_g3::request(&gold).unwrap().gold_intent.unwrap();
        assert_eq!(
            consumer
                .journal
                .pending()
                .next()
                .unwrap()
                .record
                .original
                .operation_id,
            intent.operation_id.unwrap()
        );
        assert!(consumer.begin_gold("withdraw", 0, generation).is_err());
        assert_eq!(consumer.journal.pending().count(), 1);
        consumer.stop().unwrap();
        assert!(consumer.bind(binding.clone()).is_err());
        assert!(consumer.movement(&frame, generation).is_err());
        let mut next = binding;
        next.context = [2; 16];
        next.scope.world_token = Some("world-b".into());
        consumer.bind(next).unwrap();
        assert_eq!(consumer.journal.pending().count(), 1);
        frame[8..24].fill(2);
        let generation = consumer.generation();
        consumer.movement(&frame, generation).unwrap();
        drop(consumer);
        let mut restored = Journal::open(&path).unwrap();
        let id = restored
            .pending()
            .next()
            .unwrap()
            .record
            .original
            .operation_id
            .clone();
        assert!(restored.uncertain(&id));
        assert!(restored.prepare_effect(&id).is_err());
        drop(restored);
        std::fs::remove_file(path).unwrap();
        std::fs::remove_dir(dir).unwrap();
    }
}
#[cfg(test)]mod rejoin_recovery_tests{
 use super::*;
 #[test]fn lc1_new_world_resets_recovery_fence_without_touching_transfer_journal(){
  let dir=std::env::temp_dir().join(format!("nt2-lc1-world-consumer-{}",uuid::Uuid::new_v4()));std::fs::create_dir(&dir).unwrap();let path=dir.join("operations.jsonl");let mut b=Binding{context:[1;16],service_origin:"https://fixture.invalid".into(),service_id:"service".into(),service_epoch:"epoch".into(),session_epoch:"session".into(),principal_id:"account".into(),scope:wire::Scope{room_id:Some("room".into()),run_id:Some("run".into()),run_epoch:Some("epoch".into()),config_revision:Some(1),attachment_id:Some("attachment".into()),world_token:Some("world".into())}};let mut c=LocalConsumer::new(Journal::open(&path).unwrap());c.bind(b.clone()).unwrap();c.begin_gold("withdraw",10,c.generation()).unwrap();let journal=std::fs::read(&path).unwrap();c.bind_recovered(b.clone(),17).unwrap();assert_eq!(c.ownership_generation(),17);assert_ne!(c.ownership_generation() as u64,c.generation());c.stop().unwrap();b.context=[2;16];b.scope.run_id=Some("new-run".into());b.scope.run_epoch=Some("new-epoch".into());b.scope.world_token=Some("new-world".into());b.scope.attachment_id=Some("new-attachment".into());b.session_epoch="new-session".into();c.bind(b.clone()).unwrap();c.bind_recovered(b.clone(),1).unwrap();c.stop().unwrap();b.context=[3;16];b.scope.attachment_id=Some("returned-attachment".into());b.session_epoch="returned-session".into();c.bind_recovered(b,2).unwrap();assert_eq!(std::fs::read(&path).unwrap(),journal);assert_eq!(c.journal.pending().count(),1);drop(c);std::fs::remove_file(path).unwrap();std::fs::remove_dir(dir).unwrap();
 }
 #[test]fn rejoin_movement_reset_preserves_original_uncertain_ledger_and_fences_old_owner(){
  let dir=std::env::temp_dir().join(format!("nt2-rejoin-consumer-{}",uuid::Uuid::new_v4()));std::fs::create_dir(&dir).unwrap();let path=dir.join("operations.jsonl");
  let original=Binding{context:[1;16],service_origin:"https://fixture.invalid".into(),service_id:"service".into(),service_epoch:"epoch".into(),session_epoch:"session".into(),principal_id:"account".into(),scope:wire::Scope{room_id:Some("room".into()),run_id:Some("run".into()),run_epoch:Some("run-epoch".into()),config_revision:Some(1),attachment_id:Some("attachment".into()),world_token:Some("world".into())}};
  let mut c=LocalConsumer::new(Journal::open(&path).unwrap());c.bind(original.clone()).unwrap();let before_generation=c.generation();c.begin_gold("withdraw",10,before_generation).unwrap();let bytes=std::fs::read(&path).unwrap();
  let mut next=original.clone();next.context=[2;16];next.session_epoch="fresh-session".into();next.scope.attachment_id=Some("fresh-attachment".into());
  for field in 0..4{let mut bad=next.clone();match field{0=>bad.principal_id="another".into(),1=>bad.scope.world_token=Some("other-world".into()),2=>bad.scope.attachment_id=original.scope.attachment_id.clone(),_=>bad.context=original.context};assert!(c.bind_recovered(bad,2).is_err());}
  c.stop().unwrap();c.bind_recovered(next.clone(),2).unwrap();assert_eq!(std::fs::read(&path).unwrap(),bytes);assert_eq!(c.journal.pending().count(),1);
  let mut stale=next.clone();stale.context=[3;16];stale.session_epoch="another-session".into();stale.scope.attachment_id=Some("another-attachment".into());assert!(c.bind_recovered(stale,2).is_err());assert!(c.bind_recovered(next,1).is_err());
  let mut frame=[0u8;64];frame[..8].copy_from_slice(b"NTG3\x01\x00\x01\x00");frame[8..24].fill(1);frame[24]=1;assert!(c.movement(&frame,before_generation).is_err());frame[8..24].fill(2);c.movement(&frame,c.generation()).unwrap();
  drop(c);let mut restored=Journal::open(&path).unwrap();let id=restored.pending().next().unwrap().record.original.operation_id.clone();assert!(restored.uncertain(&id));assert!(restored.prepare_effect(&id).is_err());
 }
}
