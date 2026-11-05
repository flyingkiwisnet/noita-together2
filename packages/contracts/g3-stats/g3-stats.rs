// GENERATED from schema.json. Uses serde derive; no transport or secret storage.

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Empty {
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Discover {
    #[serde(rename = "afterLocator", default, skip_serializing_if = "Option::is_none")]
    pub after_locator: Option<String>,
    #[serde(rename = "limit", default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
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
    #[serde(rename = "summaryRevision", default, skip_serializing_if = "Option::is_none")]
    pub summary_revision: Option<String>,
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
pub struct Observe {
    #[serde(rename = "eventId", default, skip_serializing_if = "Option::is_none")]
    pub event_id: Option<String>,
    #[serde(rename = "kind", default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(rename = "progression", default, skip_serializing_if = "Option::is_none")]
    pub progression: Option<u32>,
    #[serde(rename = "orbId", default, skip_serializing_if = "Option::is_none")]
    pub orb_id: Option<String>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Lobby {
    #[serde(rename = "locator", default, skip_serializing_if = "Option::is_none")]
    pub locator: Option<String>,
    #[serde(rename = "name", default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "mode", default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
    #[serde(rename = "occupancy", default, skip_serializing_if = "Option::is_none")]
    pub occupancy: Option<u32>,
    #[serde(rename = "capacity", default, skip_serializing_if = "Option::is_none")]
    pub capacity: Option<u32>,
    #[serde(rename = "state", default, skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    #[serde(rename = "publicStats", default, skip_serializing_if = "Option::is_none")]
    pub public_stats: Option<bool>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Row {
    #[serde(rename = "rowKey", default, skip_serializing_if = "Option::is_none")]
    pub row_key: Option<String>,
    #[serde(rename = "displayName", default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(rename = "hearts", default, skip_serializing_if = "Option::is_none")]
    pub hearts: Option<String>,
    #[serde(rename = "orbs", default, skip_serializing_if = "Option::is_none")]
    pub orbs: Option<String>,
    #[serde(rename = "steveKills", default, skip_serializing_if = "Option::is_none")]
    pub steve_kills: Option<String>,
    #[serde(rename = "deaths", default, skip_serializing_if = "Option::is_none")]
    pub deaths: Option<String>,
    #[serde(rename = "wins", default, skip_serializing_if = "Option::is_none")]
    pub wins: Option<String>,
    #[serde(rename = "goldDeposited", default, skip_serializing_if = "Option::is_none")]
    pub gold_deposited: Option<String>,
    #[serde(rename = "goldWithdrawn", default, skip_serializing_if = "Option::is_none")]
    pub gold_withdrawn: Option<String>,
    #[serde(rename = "itemsDeposited", default, skip_serializing_if = "Option::is_none")]
    pub items_deposited: Option<String>,
    #[serde(rename = "itemsClaimed", default, skip_serializing_if = "Option::is_none")]
    pub items_claimed: Option<String>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Summary {
    #[serde(rename = "locator", default, skip_serializing_if = "Option::is_none")]
    pub locator: Option<String>,
    #[serde(rename = "runKey", default, skip_serializing_if = "Option::is_none")]
    pub run_key: Option<String>,
    #[serde(rename = "summaryRevision", default, skip_serializing_if = "Option::is_none")]
    pub summary_revision: Option<String>,
    #[serde(rename = "collectionState", default, skip_serializing_if = "Option::is_none")]
    pub collection_state: Option<String>,
    #[serde(rename = "runStatus", default, skip_serializing_if = "Option::is_none")]
    pub run_status: Option<String>,
    #[serde(rename = "startedAt", default, skip_serializing_if = "Option::is_none")]
    pub started_at: Option<String>,
    #[serde(rename = "endedAt", default, skip_serializing_if = "Option::is_none")]
    pub ended_at: Option<String>,
    #[serde(rename = "elapsedSeconds", default, skip_serializing_if = "Option::is_none")]
    pub elapsed_seconds: Option<String>,
    #[serde(rename = "rows", default, skip_serializing_if = "Option::is_none")]
    pub rows: Option<Vec<Row>>,
    #[serde(rename = "nextRow", default, skip_serializing_if = "Option::is_none")]
    pub next_row: Option<String>,
    #[serde(rename = "lobbyName", default, skip_serializing_if = "Option::is_none")]
    pub lobby_name: Option<String>,
    #[serde(rename = "retentionDays", default, skip_serializing_if = "Option::is_none")]
    pub retention_days: Option<u32>,
    #[serde(rename = "gameReported", default, skip_serializing_if = "Option::is_none")]
    pub game_reported: Option<bool>,
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
    #[serde(rename = "discover", default, skip_serializing_if = "Option::is_none")]
    pub discover: Option<Discover>,
    #[serde(rename = "read", default, skip_serializing_if = "Option::is_none")]
    pub read: Option<Read>,
    #[serde(rename = "observe", default, skip_serializing_if = "Option::is_none")]
    pub observe: Option<Observe>,
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
    #[serde(rename = "lobbies", default, skip_serializing_if = "Option::is_none")]
    pub lobbies: Option<Vec<Lobby>>,
    #[serde(rename = "nextLocator", default, skip_serializing_if = "Option::is_none")]
    pub next_locator: Option<String>,
    #[serde(rename = "summary", default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<Summary>,
    #[serde(rename = "accepted", default, skip_serializing_if = "Option::is_none")]
    pub accepted: Option<bool>,
    #[serde(rename = "duplicate", default, skip_serializing_if = "Option::is_none")]
    pub duplicate: Option<bool>,
    #[serde(rename = "error", default, skip_serializing_if = "Option::is_none")]
    pub error: Option<Error>,
}
