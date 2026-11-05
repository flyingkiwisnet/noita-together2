// GENERATED from schema.json. Do not edit.
export const C2_LIMITS = {"bytes":32768,"fields":4096,"depth":12} as const;
export interface Scope {
  roomId?: string;
  runId?: string;
  runEpoch?: string;
  configRevision?: number;
  attachmentId?: string;
  worldToken?: string;
}
export interface Rules {
  NT_sync_steve?: boolean;
}
export interface Empty {
}
export interface RulesSet {
  expectedRevision?: number;
  rules?: Rules;
}
export interface RulesSnapshot {
  revision?: number;
  sequence?: string;
  rules?: Rules;
}
export interface ArenaBind {
  progression?: number;
  arenaToken?: string;
}
export interface State {
  progression?: number;
}
export interface Near {
  encounterId?: string;
  arenaToken?: string;
  near?: boolean;
  readinessEpoch?: string;
}
export interface Pickup {
  encounterId?: string;
  arenaToken?: string;
  orbCount?: number;
}
export interface Steve {
  progression?: number;
}
export interface Hourglass {
  effectId?: string;
  materialId?: string;
  catalogueRevision?: string;
}
export interface Completed {
  progression?: number;
}
export interface Payload {
  pickup?: Pickup;
  steve?: Steve;
  hourglass?: Hourglass;
  completed?: Completed;
}
export interface Publish {
  eventId?: string;
  rulesRevision?: number;
  payload?: Payload;
  senderDisposition?: string;
  progression?: number;
}
export interface Deliveries {
  limit?: number;
}
export interface Receipt {
  eventId?: string;
  sequence?: string;
  state?: string;
  reason?: string;
}
export interface Participant {
  principalId?: string;
  worldToken?: string;
  displayName?: string;
  status?: string;
  near?: boolean;
  pickedUp?: boolean;
  orbCount?: number;
  completed?: boolean;
  readinessEpoch?: string;
}
export interface Encounter {
  encounterId?: string;
  progression?: number;
  stage?: string;
  revision?: string;
  participants?: Participant[];
}
export interface Snapshot {
  sequence?: string;
  rules?: RulesSnapshot;
  encounter?: Encounter;
  completions?: Completion[];
  diagnostic?: string;
}
export interface Delivery {
  eventId?: string;
  sequence?: string;
  rulesRevision?: number;
  sourcePrincipalId?: string;
  sourceWorldToken?: string;
  sourceName?: string;
  payload?: Payload;
  capturedRules?: Rules;
  senderDisposition?: string;
  state?: string;
  reason?: string;
  sourceProgression?: number;
  recipientProgression?: number;
}
export interface Request {
  major?: number;
  revision?: string;
  serviceId?: string;
  serviceEpoch?: string;
  sessionEpoch?: string;
  requestId?: string;
  roomId?: string;
  scope?: Scope;
  ownerGeneration?: number;
  rulesGet?: Empty;
  rulesSet?: RulesSet;
  state?: State;
  arenaBind?: ArenaBind;
  near?: Near;
  publish?: Publish;
  deliveries?: Deliveries;
  receipt?: Receipt;
  sourceOutcome?: SourceOutcome;
}
export interface Error {
  code?: string;
}
export interface Response {
  major?: number;
  revision?: string;
  serviceId?: string;
  serviceEpoch?: string;
  requestId?: string;
  scope?: Scope;
  ownerGeneration?: number;
  snapshot?: Snapshot;
  event?: Delivery;
  deliveries?: Delivery[];
  more?: boolean;
  duplicate?: boolean;
  error?: Error;
  sourceStatus?: SourceStatus;
}
export interface SourceOriginal {
  roomId?: string;
  runId?: string;
  worldToken?: string;
  configRevision?: number;
  publish?: Publish;
}
export interface SourceReceipt {
  state?: string;
  reason?: string;
}
export interface SourceOutcome {
  original?: SourceOriginal;
  receipt?: SourceReceipt;
}
export interface SourceStatus {
  original?: SourceOriginal;
  coverage?: string;
  committed?: boolean;
  sequence?: string;
  disposition?: string;
  reason?: string;
  effectAuthority?: boolean;
  selfReported?: boolean;
  savedEffectProof?: boolean;
}
export interface Completion {
  principalId?: string;
  worldToken?: string;
  progression?: number;
  eventId?: string;
  displayName?: string;
}
