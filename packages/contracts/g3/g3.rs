// GENERATED from schema.json. Uses serde derive; no transport or secret storage.

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Scope {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub room_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run_epoch: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub config_revision: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attachment_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub world_token: Option<String>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MovementFrame {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sequence: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub x_milli: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub y_milli: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vx_milli: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vy_milli: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub facing_left: Option<bool>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Movement {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frames: Option<Vec<MovementFrame>>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GoldIntent {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operation_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub direction: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub amount: Option<u32>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GoldStatus {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operation_id: Option<String>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GoldReceipt {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operation_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delivery_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payload_digest: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub receipt_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outcome: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence_digest: Option<String>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Operation {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operation_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub principal_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attachment_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub world_token: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub direction: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub amount: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payload_digest: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delivery_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub balance: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub local_outcome: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provenance: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub original_scope: Option<Scope>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub balance_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effect_allowed: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub save_persistence: Option<String>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Ghost {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub principal_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub game_session_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub world_token: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frames: Option<Vec<MovementFrame>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub captured_at: Option<String>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GhostList {
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GameEvent {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub catalog_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub count: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_game_session_id: Option<String>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OutboxList {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub after_sequence: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OutboxEntry {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sequence: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operation: Option<Operation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event: Option<TrustedEvent>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notice: Option<GoldNotice>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item_operation: Option<ItemOperation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item_notice: Option<ItemNotice>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StatisticsList {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub after_sequence: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StatisticsAggregate {
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StatisticsDelete {
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Statistic {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sequence: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub principal_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub count: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provenance: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub captured_at: Option<String>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Aggregate {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub count: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provenance: Option<String>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Error {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Request {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub major: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revision: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub service_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub service_epoch: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_epoch: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<Scope>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub movement: Option<Movement>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ghost_list: Option<GhostList>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gold_intent: Option<GoldIntent>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gold_status: Option<GoldStatus>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gold_receipt: Option<GoldReceipt>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub game_event: Option<GameEvent>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outbox_list: Option<OutboxList>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub statistics_list: Option<StatisticsList>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub statistics_aggregate: Option<StatisticsAggregate>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub statistics_delete: Option<StatisticsDelete>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_scope: Option<Scope>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gold_bank: Option<GoldBank>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item_deposit: Option<ItemDeposit>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item_claim: Option<ItemClaim>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item_status: Option<ItemStatus>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item_receipt: Option<ItemReceipt>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item_inspect: Option<ItemInspect>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item_bank: Option<ItemBank>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inventory_publish: Option<InventoryPublish>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inventory_inspect: Option<InventoryInspect>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bank_batch: Option<BankBatch>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Response {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub major: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revision: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub service_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub service_epoch: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operation: Option<Operation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ghosts: Option<Vec<Ghost>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outbox: Option<Vec<OutboxEntry>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub statistics: Option<Vec<Statistic>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub aggregates: Option<Vec<Aggregate>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_sequence: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<Error>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bank: Option<GoldBankSnapshot>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item_operation: Option<ItemOperation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item_bank: Option<ItemBankSnapshot>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item_detail: Option<ItemDetail>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inventory: Option<InventoryView>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bank_revision: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bank_batch: Option<BankBatchResult>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TrustedEvent {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub catalog_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub principal_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub game_session_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub world_token: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_game_session_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub count: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provenance: Option<String>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GoldBank {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub after_notice_sequence: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub after_operation_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GoldNotice {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sequence: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub principal_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub amount: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub balance: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub balance_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accepted_at: Option<String>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GoldBankSnapshot {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub available_balance: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub balance_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub send_gold_enabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notices: Option<Vec<GoldNotice>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notice_sequence: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notice_reset: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operations: Option<Vec<Operation>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_operation_id: Option<String>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ItemSpell {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uses_remaining: Option<String>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ItemCard {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slot: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spell: Option<ItemSpell>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ItemWand {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub named: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ui_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sprite_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grip_x: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grip_y: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tip_x: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tip_y: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mana_max: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mana_charge_speed: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reload_time: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actions_per_round: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deck_capacity: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shuffle_deck_when_empty: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spread_degrees: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub speed_multiplier: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fire_rate_wait: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deck: Option<Vec<ItemCard>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub permanent: Option<Vec<ItemCard>>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ItemMaterial {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub material_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub amount: Option<String>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ItemContainer {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub container_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contents: Option<Vec<ItemMaterial>>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ItemObject {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub object_id: Option<String>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ItemDescriptor {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub catalogue_revision: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spell: Option<ItemSpell>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wand: Option<ItemWand>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub container: Option<ItemContainer>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub object: Option<ItemObject>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ItemDeposit {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operation_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item: Option<ItemDescriptor>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ItemClaim {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operation_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_revision: Option<String>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ItemStatus {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operation_id: Option<String>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ItemReceipt {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operation_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delivery_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payload_digest: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub receipt_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outcome: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence_digest: Option<String>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ItemInspect {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item_id: Option<String>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ItemBank {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tab: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub search: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spell_types: Option<Vec<u32>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wand_sort: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sort: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub after_notice_sequence: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub after_operation_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub use_filters: Option<bool>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ItemOperation {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operation_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub principal_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub original_scope: Option<Scope>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub direction: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delivery_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payload_digest: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub local_outcome: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item: Option<ItemDescriptor>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effect_allowed: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub save_persistence: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bank_revision: Option<String>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ItemSummary {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revision: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uses_remaining: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spell_type: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sender_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sender_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub claim_allowed: Option<bool>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ItemDetail {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<ItemSummary>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item: Option<ItemDescriptor>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ItemNotice {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sequence: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub principal_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub count: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accepted_at: Option<String>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ItemBankSnapshot {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revision: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub items: Option<Vec<ItemSummary>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page_count: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notices: Option<Vec<ItemNotice>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notice_sequence: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notice_reset: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operations: Option<Vec<ItemOperation>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_operation_id: Option<String>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InventoryWand {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slot: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wand: Option<ItemWand>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unavailable_reason: Option<String>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InventoryPublish {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revision: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wands: Option<Vec<InventoryWand>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub held_slot: Option<String>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InventoryInspect {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub game_session_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slot: Option<u32>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InventoryView {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub principal_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub game_session_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revision: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wands: Option<Vec<InventoryWand>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub held_wand_sprite_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total_slots: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_slot: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unavailable_reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub held_slot: Option<String>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BankBatchEntry {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<Scope>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_scope: Option<Scope>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gold_intent: Option<GoldIntent>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gold_status: Option<GoldStatus>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gold_receipt: Option<GoldReceipt>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item_deposit: Option<ItemDeposit>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item_claim: Option<ItemClaim>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item_status: Option<ItemStatus>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item_receipt: Option<ItemReceipt>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BankBatch {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entries: Option<Vec<BankBatchEntry>>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BankBatchEntryResult {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub index: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operation_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operation: Option<Operation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item_operation: Option<ItemOperation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<Error>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BankBatchResult {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entries: Option<Vec<BankBatchEntryResult>>,
}
