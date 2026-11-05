// GENERATED from schema.json. Uses serde derive; no transport or secret storage.

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Scope {
    #[serde(rename = "roomId", default, skip_serializing_if = "Option::is_none")]
    pub room_id: Option<String>,
    #[serde(rename = "runId", default, skip_serializing_if = "Option::is_none")]
    pub run_id: Option<String>,
    #[serde(rename = "runEpoch", default, skip_serializing_if = "Option::is_none")]
    pub run_epoch: Option<String>,
    #[serde(rename = "configRevision", default, skip_serializing_if = "Option::is_none")]
    pub config_revision: Option<u32>,
    #[serde(rename = "attachmentId", default, skip_serializing_if = "Option::is_none")]
    pub attachment_id: Option<String>,
    #[serde(rename = "worldToken", default, skip_serializing_if = "Option::is_none")]
    pub world_token: Option<String>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Rules {
    #[serde(rename = "NT_sync_steve", default, skip_serializing_if = "Option::is_none")]
    pub nt_sync_steve: Option<bool>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Empty {
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RulesSet {
    #[serde(rename = "expectedRevision", default, skip_serializing_if = "Option::is_none")]
    pub expected_revision: Option<u32>,
    #[serde(rename = "rules", default, skip_serializing_if = "Option::is_none")]
    pub rules: Option<Rules>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RulesSnapshot {
    #[serde(rename = "revision", default, skip_serializing_if = "Option::is_none")]
    pub revision: Option<u32>,
    #[serde(rename = "sequence", default, skip_serializing_if = "Option::is_none")]
    pub sequence: Option<String>,
    #[serde(rename = "rules", default, skip_serializing_if = "Option::is_none")]
    pub rules: Option<Rules>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ArenaBind {
    #[serde(rename = "progression", default, skip_serializing_if = "Option::is_none")]
    pub progression: Option<u32>,
    #[serde(rename = "arenaToken", default, skip_serializing_if = "Option::is_none")]
    pub arena_token: Option<String>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct State {
    #[serde(rename = "progression", default, skip_serializing_if = "Option::is_none")]
    pub progression: Option<u32>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Near {
    #[serde(rename = "encounterId", default, skip_serializing_if = "Option::is_none")]
    pub encounter_id: Option<String>,
    #[serde(rename = "arenaToken", default, skip_serializing_if = "Option::is_none")]
    pub arena_token: Option<String>,
    #[serde(rename = "near", default, skip_serializing_if = "Option::is_none")]
    pub near: Option<bool>,
    #[serde(rename = "readinessEpoch", default, skip_serializing_if = "Option::is_none")]
    pub readiness_epoch: Option<String>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Pickup {
    #[serde(rename = "encounterId", default, skip_serializing_if = "Option::is_none")]
    pub encounter_id: Option<String>,
    #[serde(rename = "arenaToken", default, skip_serializing_if = "Option::is_none")]
    pub arena_token: Option<String>,
    #[serde(rename = "orbCount", default, skip_serializing_if = "Option::is_none")]
    pub orb_count: Option<u32>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Steve {
    #[serde(rename = "progression", default, skip_serializing_if = "Option::is_none")]
    pub progression: Option<u32>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Hourglass {
    #[serde(rename = "effectId", default, skip_serializing_if = "Option::is_none")]
    pub effect_id: Option<String>,
    #[serde(rename = "materialId", default, skip_serializing_if = "Option::is_none")]
    pub material_id: Option<String>,
    #[serde(rename = "catalogueRevision", default, skip_serializing_if = "Option::is_none")]
    pub catalogue_revision: Option<String>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Completed {
    #[serde(rename = "progression", default, skip_serializing_if = "Option::is_none")]
    pub progression: Option<u32>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Payload {
    #[serde(rename = "pickup", default, skip_serializing_if = "Option::is_none")]
    pub pickup: Option<Pickup>,
    #[serde(rename = "steve", default, skip_serializing_if = "Option::is_none")]
    pub steve: Option<Steve>,
    #[serde(rename = "hourglass", default, skip_serializing_if = "Option::is_none")]
    pub hourglass: Option<Hourglass>,
    #[serde(rename = "completed", default, skip_serializing_if = "Option::is_none")]
    pub completed: Option<Completed>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Publish {
    #[serde(rename = "eventId", default, skip_serializing_if = "Option::is_none")]
    pub event_id: Option<String>,
    #[serde(rename = "rulesRevision", default, skip_serializing_if = "Option::is_none")]
    pub rules_revision: Option<u32>,
    #[serde(rename = "payload", default, skip_serializing_if = "Option::is_none")]
    pub payload: Option<Payload>,
    #[serde(rename = "senderDisposition", default, skip_serializing_if = "Option::is_none")]
    pub sender_disposition: Option<String>,
    #[serde(rename = "progression", default, skip_serializing_if = "Option::is_none")]
    pub progression: Option<u32>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Deliveries {
    #[serde(rename = "limit", default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Receipt {
    #[serde(rename = "eventId", default, skip_serializing_if = "Option::is_none")]
    pub event_id: Option<String>,
    #[serde(rename = "sequence", default, skip_serializing_if = "Option::is_none")]
    pub sequence: Option<String>,
    #[serde(rename = "state", default, skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    #[serde(rename = "reason", default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Participant {
    #[serde(rename = "principalId", default, skip_serializing_if = "Option::is_none")]
    pub principal_id: Option<String>,
    #[serde(rename = "worldToken", default, skip_serializing_if = "Option::is_none")]
    pub world_token: Option<String>,
    #[serde(rename = "displayName", default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(rename = "status", default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(rename = "near", default, skip_serializing_if = "Option::is_none")]
    pub near: Option<bool>,
    #[serde(rename = "pickedUp", default, skip_serializing_if = "Option::is_none")]
    pub picked_up: Option<bool>,
    #[serde(rename = "orbCount", default, skip_serializing_if = "Option::is_none")]
    pub orb_count: Option<u32>,
    #[serde(rename = "completed", default, skip_serializing_if = "Option::is_none")]
    pub completed: Option<bool>,
    #[serde(rename = "readinessEpoch", default, skip_serializing_if = "Option::is_none")]
    pub readiness_epoch: Option<String>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Encounter {
    #[serde(rename = "encounterId", default, skip_serializing_if = "Option::is_none")]
    pub encounter_id: Option<String>,
    #[serde(rename = "progression", default, skip_serializing_if = "Option::is_none")]
    pub progression: Option<u32>,
    #[serde(rename = "stage", default, skip_serializing_if = "Option::is_none")]
    pub stage: Option<String>,
    #[serde(rename = "revision", default, skip_serializing_if = "Option::is_none")]
    pub revision: Option<String>,
    #[serde(rename = "participants", default, skip_serializing_if = "Option::is_none")]
    pub participants: Option<Vec<Participant>>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Snapshot {
    #[serde(rename = "sequence", default, skip_serializing_if = "Option::is_none")]
    pub sequence: Option<String>,
    #[serde(rename = "rules", default, skip_serializing_if = "Option::is_none")]
    pub rules: Option<RulesSnapshot>,
    #[serde(rename = "encounter", default, skip_serializing_if = "Option::is_none")]
    pub encounter: Option<Encounter>,
    #[serde(rename = "completions", default, skip_serializing_if = "Option::is_none")]
    pub completions: Option<Vec<Completion>>,
    #[serde(rename = "diagnostic", default, skip_serializing_if = "Option::is_none")]
    pub diagnostic: Option<String>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Delivery {
    #[serde(rename = "eventId", default, skip_serializing_if = "Option::is_none")]
    pub event_id: Option<String>,
    #[serde(rename = "sequence", default, skip_serializing_if = "Option::is_none")]
    pub sequence: Option<String>,
    #[serde(rename = "rulesRevision", default, skip_serializing_if = "Option::is_none")]
    pub rules_revision: Option<u32>,
    #[serde(rename = "sourcePrincipalId", default, skip_serializing_if = "Option::is_none")]
    pub source_principal_id: Option<String>,
    #[serde(rename = "sourceWorldToken", default, skip_serializing_if = "Option::is_none")]
    pub source_world_token: Option<String>,
    #[serde(rename = "sourceName", default, skip_serializing_if = "Option::is_none")]
    pub source_name: Option<String>,
    #[serde(rename = "payload", default, skip_serializing_if = "Option::is_none")]
    pub payload: Option<Payload>,
    #[serde(rename = "capturedRules", default, skip_serializing_if = "Option::is_none")]
    pub captured_rules: Option<Rules>,
    #[serde(rename = "senderDisposition", default, skip_serializing_if = "Option::is_none")]
    pub sender_disposition: Option<String>,
    #[serde(rename = "state", default, skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    #[serde(rename = "reason", default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(rename = "sourceProgression", default, skip_serializing_if = "Option::is_none")]
    pub source_progression: Option<u32>,
    #[serde(rename = "recipientProgression", default, skip_serializing_if = "Option::is_none")]
    pub recipient_progression: Option<u32>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Request {
    #[serde(rename = "major", default, skip_serializing_if = "Option::is_none")]
    pub major: Option<u32>,
    #[serde(rename = "revision", default, skip_serializing_if = "Option::is_none")]
    pub revision: Option<String>,
    #[serde(rename = "serviceId", default, skip_serializing_if = "Option::is_none")]
    pub service_id: Option<String>,
    #[serde(rename = "serviceEpoch", default, skip_serializing_if = "Option::is_none")]
    pub service_epoch: Option<String>,
    #[serde(rename = "sessionEpoch", default, skip_serializing_if = "Option::is_none")]
    pub session_epoch: Option<String>,
    #[serde(rename = "requestId", default, skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    #[serde(rename = "roomId", default, skip_serializing_if = "Option::is_none")]
    pub room_id: Option<String>,
    #[serde(rename = "scope", default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<Scope>,
    #[serde(rename = "ownerGeneration", default, skip_serializing_if = "Option::is_none")]
    pub owner_generation: Option<u32>,
    #[serde(rename = "rulesGet", default, skip_serializing_if = "Option::is_none")]
    pub rules_get: Option<Empty>,
    #[serde(rename = "rulesSet", default, skip_serializing_if = "Option::is_none")]
    pub rules_set: Option<RulesSet>,
    #[serde(rename = "state", default, skip_serializing_if = "Option::is_none")]
    pub state: Option<State>,
    #[serde(rename = "arenaBind", default, skip_serializing_if = "Option::is_none")]
    pub arena_bind: Option<ArenaBind>,
    #[serde(rename = "near", default, skip_serializing_if = "Option::is_none")]
    pub near: Option<Near>,
    #[serde(rename = "publish", default, skip_serializing_if = "Option::is_none")]
    pub publish: Option<Publish>,
    #[serde(rename = "deliveries", default, skip_serializing_if = "Option::is_none")]
    pub deliveries: Option<Deliveries>,
    #[serde(rename = "receipt", default, skip_serializing_if = "Option::is_none")]
    pub receipt: Option<Receipt>,
    #[serde(rename = "sourceOutcome", default, skip_serializing_if = "Option::is_none")]
    pub source_outcome: Option<SourceOutcome>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Error {
    #[serde(rename = "code", default, skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Response {
    #[serde(rename = "major", default, skip_serializing_if = "Option::is_none")]
    pub major: Option<u32>,
    #[serde(rename = "revision", default, skip_serializing_if = "Option::is_none")]
    pub revision: Option<String>,
    #[serde(rename = "serviceId", default, skip_serializing_if = "Option::is_none")]
    pub service_id: Option<String>,
    #[serde(rename = "serviceEpoch", default, skip_serializing_if = "Option::is_none")]
    pub service_epoch: Option<String>,
    #[serde(rename = "requestId", default, skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    #[serde(rename = "scope", default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<Scope>,
    #[serde(rename = "ownerGeneration", default, skip_serializing_if = "Option::is_none")]
    pub owner_generation: Option<u32>,
    #[serde(rename = "snapshot", default, skip_serializing_if = "Option::is_none")]
    pub snapshot: Option<Snapshot>,
    #[serde(rename = "event", default, skip_serializing_if = "Option::is_none")]
    pub event: Option<Delivery>,
    #[serde(rename = "deliveries", default, skip_serializing_if = "Option::is_none")]
    pub deliveries: Option<Vec<Delivery>>,
    #[serde(rename = "more", default, skip_serializing_if = "Option::is_none")]
    pub more: Option<bool>,
    #[serde(rename = "duplicate", default, skip_serializing_if = "Option::is_none")]
    pub duplicate: Option<bool>,
    #[serde(rename = "error", default, skip_serializing_if = "Option::is_none")]
    pub error: Option<Error>,
    #[serde(rename = "sourceStatus", default, skip_serializing_if = "Option::is_none")]
    pub source_status: Option<SourceStatus>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceOriginal {
    #[serde(rename = "roomId", default, skip_serializing_if = "Option::is_none")]
    pub room_id: Option<String>,
    #[serde(rename = "runId", default, skip_serializing_if = "Option::is_none")]
    pub run_id: Option<String>,
    #[serde(rename = "worldToken", default, skip_serializing_if = "Option::is_none")]
    pub world_token: Option<String>,
    #[serde(rename = "configRevision", default, skip_serializing_if = "Option::is_none")]
    pub config_revision: Option<u32>,
    #[serde(rename = "publish", default, skip_serializing_if = "Option::is_none")]
    pub publish: Option<Publish>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceReceipt {
    #[serde(rename = "state", default, skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    #[serde(rename = "reason", default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceOutcome {
    #[serde(rename = "original", default, skip_serializing_if = "Option::is_none")]
    pub original: Option<SourceOriginal>,
    #[serde(rename = "receipt", default, skip_serializing_if = "Option::is_none")]
    pub receipt: Option<SourceReceipt>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceStatus {
    #[serde(rename = "original", default, skip_serializing_if = "Option::is_none")]
    pub original: Option<SourceOriginal>,
    #[serde(rename = "coverage", default, skip_serializing_if = "Option::is_none")]
    pub coverage: Option<String>,
    #[serde(rename = "committed", default, skip_serializing_if = "Option::is_none")]
    pub committed: Option<bool>,
    #[serde(rename = "sequence", default, skip_serializing_if = "Option::is_none")]
    pub sequence: Option<String>,
    #[serde(rename = "disposition", default, skip_serializing_if = "Option::is_none")]
    pub disposition: Option<String>,
    #[serde(rename = "reason", default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(rename = "effectAuthority", default, skip_serializing_if = "Option::is_none")]
    pub effect_authority: Option<bool>,
    #[serde(rename = "selfReported", default, skip_serializing_if = "Option::is_none")]
    pub self_reported: Option<bool>,
    #[serde(rename = "savedEffectProof", default, skip_serializing_if = "Option::is_none")]
    pub saved_effect_proof: Option<bool>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Completion {
    #[serde(rename = "principalId", default, skip_serializing_if = "Option::is_none")]
    pub principal_id: Option<String>,
    #[serde(rename = "worldToken", default, skip_serializing_if = "Option::is_none")]
    pub world_token: Option<String>,
    #[serde(rename = "progression", default, skip_serializing_if = "Option::is_none")]
    pub progression: Option<u32>,
    #[serde(rename = "eventId", default, skip_serializing_if = "Option::is_none")]
    pub event_id: Option<String>,
    #[serde(rename = "displayName", default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
}
