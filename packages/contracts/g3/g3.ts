// GENERATED from schema.json. Do not edit.
export const G3_LIMITS = {"bytes":262144,"fields":32768,"depth":12} as const;
export interface Scope {
  roomId?: string;
  runId?: string;
  runEpoch?: string;
  configRevision?: number;
  attachmentId?: string;
  worldToken?: string;
}
export interface MovementFrame {
  sequence?: number;
  xMilli?: string;
  yMilli?: string;
  vxMilli?: string;
  vyMilli?: string;
  facingLeft?: boolean;
}
export interface Movement {
  frames?: MovementFrame[];
}
export interface GoldIntent {
  operationId?: string;
  direction?: string;
  amount?: number;
}
export interface GoldStatus {
  operationId?: string;
}
export interface GoldReceipt {
  operationId?: string;
  deliveryId?: string;
  payloadDigest?: string;
  receiptId?: string;
  outcome?: string;
  evidenceDigest?: string;
}
export interface Operation {
  operationId?: string;
  principalId?: string;
  runId?: string;
  attachmentId?: string;
  worldToken?: string;
  direction?: string;
  amount?: number;
  payloadDigest?: string;
  deliveryId?: string;
  state?: string;
  balance?: number;
  localOutcome?: string;
  provenance?: string;
  originalScope?: Scope;
  balanceVersion?: string;
  effectAllowed?: boolean;
  savePersistence?: string;
}
export interface Ghost {
  principalId?: string;
  gameSessionId?: string;
  worldToken?: string;
  frames?: MovementFrame[];
  capturedAt?: string;
}
export interface GhostList {
}
export interface GameEvent {
  eventId?: string;
  catalogId?: string;
  count?: number;
  targetGameSessionId?: string;
}
export interface OutboxList {
  afterSequence?: string;
  limit?: number;
}
export interface OutboxEntry {
  sequence?: string;
  kind?: string;
  operation?: Operation;
  event?: TrustedEvent;
  notice?: GoldNotice;
  itemOperation?: ItemOperation;
  itemNotice?: ItemNotice;
}
export interface StatisticsList {
  afterSequence?: string;
  limit?: number;
}
export interface StatisticsAggregate {
}
export interface StatisticsDelete {
}
export interface Statistic {
  sequence?: string;
  version?: number;
  kind?: string;
  principalId?: string;
  displayName?: string;
  count?: number;
  provenance?: string;
  capturedAt?: string;
}
export interface Aggregate {
  kind?: string;
  count?: string;
  provenance?: string;
}
export interface Error {
  code?: string;
}
export interface Request {
  major?: number;
  revision?: string;
  serviceId?: string;
  serviceEpoch?: string;
  sessionEpoch?: string;
  requestId?: string;
  scope?: Scope;
  movement?: Movement;
  ghostList?: GhostList;
  goldIntent?: GoldIntent;
  goldStatus?: GoldStatus;
  goldReceipt?: GoldReceipt;
  gameEvent?: GameEvent;
  outboxList?: OutboxList;
  statisticsList?: StatisticsList;
  statisticsAggregate?: StatisticsAggregate;
  statisticsDelete?: StatisticsDelete;
  currentScope?: Scope;
  goldBank?: GoldBank;
  itemDeposit?: ItemDeposit;
  itemClaim?: ItemClaim;
  itemStatus?: ItemStatus;
  itemReceipt?: ItemReceipt;
  itemInspect?: ItemInspect;
  itemBank?: ItemBank;
  inventoryPublish?: InventoryPublish;
  inventoryInspect?: InventoryInspect;
  bankBatch?: BankBatch;
}
export interface Response {
  major?: number;
  revision?: string;
  serviceId?: string;
  serviceEpoch?: string;
  requestId?: string;
  operation?: Operation;
  ghosts?: Ghost[];
  outbox?: OutboxEntry[];
  statistics?: Statistic[];
  aggregates?: Aggregate[];
  nextSequence?: string;
  error?: Error;
  bank?: GoldBankSnapshot;
  itemOperation?: ItemOperation;
  itemBank?: ItemBankSnapshot;
  itemDetail?: ItemDetail;
  inventory?: InventoryView;
  bankRevision?: string;
  bankBatch?: BankBatchResult;
}
export interface TrustedEvent {
  eventId?: string;
  catalogId?: string;
  principalId?: string;
  gameSessionId?: string;
  worldToken?: string;
  targetGameSessionId?: string;
  count?: number;
  provenance?: string;
}
export interface GoldBank {
  afterNoticeSequence?: string;
  afterOperationId?: string;
  limit?: number;
}
export interface GoldNotice {
  sequence?: string;
  principalId?: string;
  displayName?: string;
  amount?: number;
  balance?: number;
  balanceVersion?: string;
  acceptedAt?: string;
}
export interface GoldBankSnapshot {
  availableBalance?: number;
  balanceVersion?: string;
  sendGoldEnabled?: boolean;
  notices?: GoldNotice[];
  noticeSequence?: string;
  noticeReset?: boolean;
  operations?: Operation[];
  nextOperationId?: string;
}
export interface ItemSpell {
  actionId?: string;
  usesRemaining?: string;
}
export interface ItemCard {
  slot?: number;
  spell?: ItemSpell;
}
export interface ItemWand {
  named?: boolean;
  uiName?: string;
  spriteId?: string;
  gripX?: string;
  gripY?: string;
  tipX?: string;
  tipY?: string;
  manaMax?: string;
  manaChargeSpeed?: string;
  reloadTime?: string;
  actionsPerRound?: string;
  deckCapacity?: string;
  shuffleDeckWhenEmpty?: boolean;
  spreadDegrees?: string;
  speedMultiplier?: string;
  fireRateWait?: string;
  deck?: ItemCard[];
  permanent?: ItemCard[];
}
export interface ItemMaterial {
  materialId?: string;
  amount?: string;
}
export interface ItemContainer {
  containerType?: string;
  color?: string;
  contents?: ItemMaterial[];
}
export interface ItemObject {
  objectId?: string;
}
export interface ItemDescriptor {
  catalogueRevision?: string;
  spell?: ItemSpell;
  wand?: ItemWand;
  container?: ItemContainer;
  object?: ItemObject;
}
export interface ItemDeposit {
  operationId?: string;
  item?: ItemDescriptor;
}
export interface ItemClaim {
  operationId?: string;
  itemId?: string;
  expectedRevision?: string;
}
export interface ItemStatus {
  operationId?: string;
}
export interface ItemReceipt {
  operationId?: string;
  deliveryId?: string;
  payloadDigest?: string;
  receiptId?: string;
  outcome?: string;
  evidenceDigest?: string;
}
export interface ItemInspect {
  itemId?: string;
}
export interface ItemBank {
  tab?: string;
  search?: string;
  spellTypes?: number[];
  wandSort?: string[];
  sort?: string;
  page?: number;
  afterNoticeSequence?: string;
  afterOperationId?: string;
  useFilters?: boolean;
}
export interface ItemOperation {
  operationId?: string;
  principalId?: string;
  originalScope?: Scope;
  direction?: string;
  itemId?: string;
  deliveryId?: string;
  payloadDigest?: string;
  state?: string;
  localOutcome?: string;
  item?: ItemDescriptor;
  effectAllowed?: boolean;
  savePersistence?: string;
  bankRevision?: string;
}
export interface ItemSummary {
  itemId?: string;
  kind?: string;
  state?: string;
  revision?: string;
  displayName?: string;
  nameKey?: string;
  iconId?: string;
  usesRemaining?: string;
  spellType?: number;
  senderId?: string;
  senderName?: string;
  claimAllowed?: boolean;
}
export interface ItemDetail {
  summary?: ItemSummary;
  item?: ItemDescriptor;
}
export interface ItemNotice {
  sequence?: string;
  principalId?: string;
  displayName?: string;
  count?: number;
  kind?: string;
  acceptedAt?: string;
}
export interface ItemBankSnapshot {
  revision?: string;
  items?: ItemSummary[];
  total?: number;
  page?: number;
  pageCount?: number;
  notices?: ItemNotice[];
  noticeSequence?: string;
  noticeReset?: boolean;
  operations?: ItemOperation[];
  nextOperationId?: string;
}
export interface InventoryWand {
  slot?: number;
  wand?: ItemWand;
  unavailableReason?: string;
}
export interface InventoryPublish {
  revision?: number;
  wands?: InventoryWand[];
  heldSlot?: string;
}
export interface InventoryInspect {
  gameSessionId?: string;
  slot?: number;
}
export interface InventoryView {
  principalId?: string;
  gameSessionId?: string;
  revision?: number;
  wands?: InventoryWand[];
  heldWandSpriteId?: string;
  totalSlots?: number;
  nextSlot?: string;
  state?: string;
  unavailableReason?: string;
  heldSlot?: string;
}
export interface BankBatchEntry {
  scope?: Scope;
  currentScope?: Scope;
  goldIntent?: GoldIntent;
  goldStatus?: GoldStatus;
  goldReceipt?: GoldReceipt;
  itemDeposit?: ItemDeposit;
  itemClaim?: ItemClaim;
  itemStatus?: ItemStatus;
  itemReceipt?: ItemReceipt;
}
export interface BankBatch {
  entries?: BankBatchEntry[];
}
export interface BankBatchEntryResult {
  index?: number;
  operationId?: string;
  operation?: Operation;
  itemOperation?: ItemOperation;
  error?: Error;
}
export interface BankBatchResult {
  entries?: BankBatchEntryResult[];
}
