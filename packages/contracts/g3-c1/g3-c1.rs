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
pub struct Empty {
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Rules {
    #[serde(rename = "NT_sync_hearts", default, skip_serializing_if = "Option::is_none")]
    pub nt_sync_hearts: Option<bool>,
    #[serde(rename = "NT_sync_orbs", default, skip_serializing_if = "Option::is_none")]
    pub nt_sync_orbs: Option<bool>,
    #[serde(rename = "NT_sync_shift", default, skip_serializing_if = "Option::is_none")]
    pub nt_sync_shift: Option<bool>,
    #[serde(rename = "NT_team_perks", default, skip_serializing_if = "Option::is_none")]
    pub nt_team_perks: Option<bool>,
    #[serde(rename = "NT_sync_perks", default, skip_serializing_if = "Option::is_none")]
    pub nt_sync_perks: Option<bool>,
    #[serde(rename = "NT_alt_heartache", default, skip_serializing_if = "Option::is_none")]
    pub nt_alt_heartache: Option<bool>,
    #[serde(rename = "NT_ban_perks", default, skip_serializing_if = "Option::is_none")]
    pub nt_ban_perks: Option<bool>,
    #[serde(rename = "NT_sync_orbs_no_curse", default, skip_serializing_if = "Option::is_none")]
    pub nt_sync_orbs_no_curse: Option<bool>,
    #[serde(rename = "NT_ondeath_kick", default, skip_serializing_if = "Option::is_none")]
    pub nt_ondeath_kick: Option<bool>,
    #[serde(rename = "NT_sync_orb_count", default, skip_serializing_if = "Option::is_none")]
    pub nt_sync_orb_count: Option<u32>,
    #[serde(rename = "NT_death_penalty", default, skip_serializing_if = "Option::is_none")]
    pub nt_death_penalty: Option<String>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuleSnapshot {
    #[serde(rename = "revision", default, skip_serializing_if = "Option::is_none")]
    pub revision: Option<u32>,
    #[serde(rename = "sequence", default, skip_serializing_if = "Option::is_none")]
    pub sequence: Option<String>,
    #[serde(rename = "rules", default, skip_serializing_if = "Option::is_none")]
    pub rules: Option<Rules>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuleSet {
    #[serde(rename = "expectedRevision", default, skip_serializing_if = "Option::is_none")]
    pub expected_revision: Option<u32>,
    #[serde(rename = "rules", default, skip_serializing_if = "Option::is_none")]
    pub rules: Option<Rules>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Heart {
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Orb {
    #[serde(rename = "orbId", default, skip_serializing_if = "Option::is_none")]
    pub orb_id: Option<u32>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Perk {
    #[serde(rename = "perkId", default, skip_serializing_if = "Option::is_none")]
    pub perk_id: Option<String>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Fungal {
    #[serde(rename = "fromMaterials", default, skip_serializing_if = "Option::is_none")]
    pub from_materials: Option<Vec<String>>,
    #[serde(rename = "toMaterial", default, skip_serializing_if = "Option::is_none")]
    pub to_material: Option<String>,
    #[serde(rename = "displayMaterial", default, skip_serializing_if = "Option::is_none")]
    pub display_material: Option<String>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Death {
    #[serde(rename = "state", default, skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    #[serde(rename = "cause", default, skip_serializing_if = "Option::is_none")]
    pub cause: Option<String>,
    #[serde(rename = "progression", default, skip_serializing_if = "Option::is_none")]
    pub progression: Option<u32>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Payload {
    #[serde(rename = "heart", default, skip_serializing_if = "Option::is_none")]
    pub heart: Option<Heart>,
    #[serde(rename = "orb", default, skip_serializing_if = "Option::is_none")]
    pub orb: Option<Orb>,
    #[serde(rename = "perk", default, skip_serializing_if = "Option::is_none")]
    pub perk: Option<Perk>,
    #[serde(rename = "fungal", default, skip_serializing_if = "Option::is_none")]
    pub fungal: Option<Fungal>,
    #[serde(rename = "death", default, skip_serializing_if = "Option::is_none")]
    pub death: Option<Death>,
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
    #[serde(rename = "state", default, skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    #[serde(rename = "reason", default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(rename = "senderDisposition", default, skip_serializing_if = "Option::is_none")]
    pub sender_disposition: Option<String>,
    #[serde(rename = "capturedRules", default, skip_serializing_if = "Option::is_none")]
    pub captured_rules: Option<Rules>,
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
pub struct Page {
    #[serde(rename = "limit", default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LifeUpdate {
    #[serde(rename = "deathEventId", default, skip_serializing_if = "Option::is_none")]
    pub death_event_id: Option<String>,
    #[serde(rename = "state", default, skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Life {
    #[serde(rename = "principalId", default, skip_serializing_if = "Option::is_none")]
    pub principal_id: Option<String>,
    #[serde(rename = "worldToken", default, skip_serializing_if = "Option::is_none")]
    pub world_token: Option<String>,
    #[serde(rename = "state", default, skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    #[serde(rename = "deathEventId", default, skip_serializing_if = "Option::is_none")]
    pub death_event_id: Option<String>,
    #[serde(rename = "rulesRevision", default, skip_serializing_if = "Option::is_none")]
    pub rules_revision: Option<u32>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Member {
    #[serde(rename = "principalId", default, skip_serializing_if = "Option::is_none")]
    pub principal_id: Option<String>,
    #[serde(rename = "displayName", default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Error {
    #[serde(rename = "code", default, skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
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
    pub rules_set: Option<RuleSet>,
    #[serde(rename = "publish", default, skip_serializing_if = "Option::is_none")]
    pub publish: Option<Publish>,
    #[serde(rename = "deliveries", default, skip_serializing_if = "Option::is_none")]
    pub deliveries: Option<Page>,
    #[serde(rename = "receipt", default, skip_serializing_if = "Option::is_none")]
    pub receipt: Option<Receipt>,
    #[serde(rename = "life", default, skip_serializing_if = "Option::is_none")]
    pub life: Option<LifeUpdate>,
    #[serde(rename = "sourceOutcome", default, skip_serializing_if = "Option::is_none")]
    pub source_outcome: Option<SourceOutcomeRequest>,
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
    pub snapshot: Option<RuleSnapshot>,
    #[serde(rename = "event", default, skip_serializing_if = "Option::is_none")]
    pub event: Option<Delivery>,
    #[serde(rename = "deliveries", default, skip_serializing_if = "Option::is_none")]
    pub deliveries: Option<Vec<Delivery>>,
    #[serde(rename = "duplicate", default, skip_serializing_if = "Option::is_none")]
    pub duplicate: Option<bool>,
    #[serde(rename = "members", default, skip_serializing_if = "Option::is_none")]
    pub members: Option<Vec<Member>>,
    #[serde(rename = "memberCount", default, skip_serializing_if = "Option::is_none")]
    pub member_count: Option<u32>,
    #[serde(rename = "lives", default, skip_serializing_if = "Option::is_none")]
    pub lives: Option<Vec<Life>>,
    #[serde(rename = "error", default, skip_serializing_if = "Option::is_none")]
    pub error: Option<Error>,
    #[serde(rename = "more", default, skip_serializing_if = "Option::is_none")]
    pub more: Option<bool>,
    #[serde(rename = "sequence", default, skip_serializing_if = "Option::is_none")]
    pub sequence: Option<String>,
    #[serde(rename = "sourceOutcome", default, skip_serializing_if = "Option::is_none")]
    pub source_outcome: Option<SourceOutcome>,
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
pub struct SourceDisposition {
    #[serde(rename = "state", default, skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    #[serde(rename = "reason", default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceOutcomeRequest {
    #[serde(rename = "original", default, skip_serializing_if = "Option::is_none")]
    pub original: Option<SourceOriginal>,
    #[serde(rename = "receipt", default, skip_serializing_if = "Option::is_none")]
    pub receipt: Option<SourceDisposition>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceOutcome {
    #[serde(rename = "original", default, skip_serializing_if = "Option::is_none")]
    pub original: Option<SourceOriginal>,
    #[serde(rename = "outcome", default, skip_serializing_if = "Option::is_none")]
    pub outcome: Option<String>,
    #[serde(rename = "coverage", default, skip_serializing_if = "Option::is_none")]
    pub coverage: Option<String>,
    #[serde(rename = "sourceDisposition", default, skip_serializing_if = "Option::is_none")]
    pub source_disposition: Option<String>,
    #[serde(rename = "sourceReason", default, skip_serializing_if = "Option::is_none")]
    pub source_reason: Option<String>,
    #[serde(rename = "acceptedSequence", default, skip_serializing_if = "Option::is_none")]
    pub accepted_sequence: Option<String>,
    #[serde(rename = "rulesRevision", default, skip_serializing_if = "Option::is_none")]
    pub rules_revision: Option<u32>,
    #[serde(rename = "effectAuthority", default, skip_serializing_if = "Option::is_none")]
    pub effect_authority: Option<bool>,
    #[serde(rename = "selfReported", default, skip_serializing_if = "Option::is_none")]
    pub self_reported: Option<bool>,
    #[serde(rename = "savedEffectProof", default, skip_serializing_if = "Option::is_none")]
    pub saved_effect_proof: Option<bool>,
    #[serde(rename = "duplicate", default, skip_serializing_if = "Option::is_none")]
    pub duplicate: Option<bool>,
}
