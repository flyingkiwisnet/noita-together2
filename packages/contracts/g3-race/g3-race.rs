// GENERATED from schema.json. Uses serde derive; no transport or secret storage.

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Empty {
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RaceGoal {
    #[serde(rename = "instanceId", default, skip_serializing_if = "Option::is_none")]
    pub instance_id: Option<String>,
    #[serde(rename = "id", default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(rename = "target", default, skip_serializing_if = "Option::is_none")]
    pub target: Option<u32>,
    #[serde(rename = "bossIds", default, skip_serializing_if = "Option::is_none")]
    pub boss_ids: Option<Vec<String>>,
    #[serde(rename = "biomeId", default, skip_serializing_if = "Option::is_none")]
    pub biome_id: Option<String>,
    #[serde(rename = "direction", default, skip_serializing_if = "Option::is_none")]
    pub direction: Option<String>,
    #[serde(rename = "index", default, skip_serializing_if = "Option::is_none")]
    pub index: Option<u32>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RaceConfig {
    #[serde(rename = "registryRevision", default, skip_serializing_if = "Option::is_none")]
    pub registry_revision: Option<String>,
    #[serde(rename = "goals", default, skip_serializing_if = "Option::is_none")]
    pub goals: Option<Vec<RaceGoal>>,
    #[serde(rename = "durationMs", default, skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u32>,
}

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
pub struct Observation {
    #[serde(rename = "kind", default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(rename = "orbId", default, skip_serializing_if = "Option::is_none")]
    pub orb_id: Option<u32>,
    #[serde(rename = "bossId", default, skip_serializing_if = "Option::is_none")]
    pub boss_id: Option<String>,
    #[serde(rename = "biomeId", default, skip_serializing_if = "Option::is_none")]
    pub biome_id: Option<String>,
    #[serde(rename = "direction", default, skip_serializing_if = "Option::is_none")]
    pub direction: Option<String>,
    #[serde(rename = "index", default, skip_serializing_if = "Option::is_none")]
    pub index: Option<u32>,
    #[serde(rename = "value", default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(rename = "ending", default, skip_serializing_if = "Option::is_none")]
    pub ending: Option<String>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Commit {
    #[serde(rename = "eventId", default, skip_serializing_if = "Option::is_none")]
    pub event_id: Option<String>,
    #[serde(rename = "sequence", default, skip_serializing_if = "Option::is_none")]
    pub sequence: Option<u32>,
    #[serde(rename = "observations", default, skip_serializing_if = "Option::is_none")]
    pub observations: Option<Vec<Observation>>,
    #[serde(rename = "terminal", default, skip_serializing_if = "Option::is_none")]
    pub terminal: Option<String>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Read {
    #[serde(rename = "locator", default, skip_serializing_if = "Option::is_none")]
    pub locator: Option<String>,
    #[serde(rename = "afterRow", default, skip_serializing_if = "Option::is_none")]
    pub after_row: Option<String>,
    #[serde(rename = "limit", default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    #[serde(rename = "runKey", default, skip_serializing_if = "Option::is_none")]
    pub run_key: Option<String>,
    #[serde(rename = "snapshotRevision", default, skip_serializing_if = "Option::is_none")]
    pub snapshot_revision: Option<String>,
    #[serde(rename = "self", default, skip_serializing_if = "Option::is_none")]
    pub self_row_requested: Option<bool>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Credit {
    #[serde(rename = "id", default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(rename = "firstAtMs", default, skip_serializing_if = "Option::is_none")]
    pub first_at_ms: Option<String>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Progress {
    #[serde(rename = "instanceId", default, skip_serializing_if = "Option::is_none")]
    pub instance_id: Option<String>,
    #[serde(rename = "value", default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(rename = "currentValue", default, skip_serializing_if = "Option::is_none")]
    pub current_value: Option<String>,
    #[serde(rename = "target", default, skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    #[serde(rename = "complete", default, skip_serializing_if = "Option::is_none")]
    pub complete: Option<bool>,
    #[serde(rename = "completedAtMs", default, skip_serializing_if = "Option::is_none")]
    pub completed_at_ms: Option<String>,
    #[serde(rename = "credits", default, skip_serializing_if = "Option::is_none")]
    pub credits: Option<Vec<Credit>>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Entrant {
    #[serde(rename = "rowKey", default, skip_serializing_if = "Option::is_none")]
    pub row_key: Option<String>,
    #[serde(rename = "displayName", default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(rename = "status", default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(rename = "completedGoals", default, skip_serializing_if = "Option::is_none")]
    pub completed_goals: Option<u32>,
    #[serde(rename = "finalCountAtMs", default, skip_serializing_if = "Option::is_none")]
    pub final_count_at_ms: Option<String>,
    #[serde(rename = "finishOrder", default, skip_serializing_if = "Option::is_none")]
    pub finish_order: Option<u32>,
    #[serde(rename = "rank", default, skip_serializing_if = "Option::is_none")]
    pub rank: Option<u32>,
    #[serde(rename = "progress", default, skip_serializing_if = "Option::is_none")]
    pub progress: Option<Vec<Progress>>,
    #[serde(rename = "acceptedSequence", default, skip_serializing_if = "Option::is_none")]
    pub accepted_sequence: Option<u32>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Snapshot {
    #[serde(rename = "locator", default, skip_serializing_if = "Option::is_none")]
    pub locator: Option<String>,
    #[serde(rename = "runKey", default, skip_serializing_if = "Option::is_none")]
    pub run_key: Option<String>,
    #[serde(rename = "revision", default, skip_serializing_if = "Option::is_none")]
    pub revision: Option<String>,
    #[serde(rename = "registryRevision", default, skip_serializing_if = "Option::is_none")]
    pub registry_revision: Option<String>,
    #[serde(rename = "config", default, skip_serializing_if = "Option::is_none")]
    pub config: Option<RaceConfig>,
    #[serde(rename = "status", default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(rename = "startedAt", default, skip_serializing_if = "Option::is_none")]
    pub started_at: Option<String>,
    #[serde(rename = "deadlineAt", default, skip_serializing_if = "Option::is_none")]
    pub deadline_at: Option<String>,
    #[serde(rename = "endedAt", default, skip_serializing_if = "Option::is_none")]
    pub ended_at: Option<String>,
    #[serde(rename = "elapsedMs", default, skip_serializing_if = "Option::is_none")]
    pub elapsed_ms: Option<String>,
    #[serde(rename = "collectionState", default, skip_serializing_if = "Option::is_none")]
    pub collection_state: Option<String>,
    #[serde(rename = "entrants", default, skip_serializing_if = "Option::is_none")]
    pub entrants: Option<Vec<Entrant>>,
    #[serde(rename = "nextRow", default, skip_serializing_if = "Option::is_none")]
    pub next_row: Option<String>,
    #[serde(rename = "lobbyName", default, skip_serializing_if = "Option::is_none")]
    pub lobby_name: Option<String>,
    #[serde(rename = "selfRow", default, skip_serializing_if = "Option::is_none")]
    pub self_row: Option<String>,
    #[serde(rename = "endReason", default, skip_serializing_if = "Option::is_none")]
    pub end_reason: Option<String>,
    #[serde(rename = "lobbyMode", default, skip_serializing_if = "Option::is_none")]
    pub lobby_mode: Option<String>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Receipt {
    #[serde(rename = "eventId", default, skip_serializing_if = "Option::is_none")]
    pub event_id: Option<String>,
    #[serde(rename = "sequence", default, skip_serializing_if = "Option::is_none")]
    pub sequence: Option<u32>,
    #[serde(rename = "acceptedAtMs", default, skip_serializing_if = "Option::is_none")]
    pub accepted_at_ms: Option<String>,
    #[serde(rename = "revision", default, skip_serializing_if = "Option::is_none")]
    pub revision: Option<String>,
    #[serde(rename = "status", default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(rename = "completedGoals", default, skip_serializing_if = "Option::is_none")]
    pub completed_goals: Option<u32>,
    #[serde(rename = "finishOrder", default, skip_serializing_if = "Option::is_none")]
    pub finish_order: Option<u32>,
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
    #[serde(rename = "scope", default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<Scope>,
    #[serde(rename = "ownerGeneration", default, skip_serializing_if = "Option::is_none")]
    pub owner_generation: Option<u32>,
    #[serde(rename = "commit", default, skip_serializing_if = "Option::is_none")]
    pub commit: Option<Commit>,
    #[serde(rename = "read", default, skip_serializing_if = "Option::is_none")]
    pub read: Option<Read>,
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
    #[serde(rename = "receipt", default, skip_serializing_if = "Option::is_none")]
    pub receipt: Option<Receipt>,
    #[serde(rename = "snapshot", default, skip_serializing_if = "Option::is_none")]
    pub snapshot: Option<Snapshot>,
    #[serde(rename = "error", default, skip_serializing_if = "Option::is_none")]
    pub error: Option<Error>,
}
