// GENERATED from schema.json. Do not edit.
export const NEMESIS_LIMITS = {"bytes":32768,"fields":2048,"depth":8} as const;
export interface Scope {
  roomId?: string;
  runId?: string;
  runEpoch?: string;
  configRevision?: number;
  attachmentId?: string;
  worldToken?: string;
}
export interface Source {
  principalId?: string;
  name?: string;
  gameSessionId?: string;
  attachmentId?: string;
  worldToken?: string;
}
export interface Error {
  code?: string;
}
export interface Purchase {
  operationId?: string;
  offerId?: string;
}
export interface Status {
  operationId?: string;
  afterRecipientKey?: string;
  limit?: number;
}
export interface Pending {
  afterSequence?: number;
  throughSequence?: number;
  limit?: number;
}
export interface Receipt {
  deliveryId?: string;
  receiptId?: string;
  outcome?: string;
}
export interface PurchaseResult {
  operationId?: string;
  offerId?: string;
  groupId?: string;
  abilityId?: string;
  cost?: number;
  pointsAfter?: number;
  recipientCount?: number;
  provenance?: string;
}
export interface Original {
  operationId?: string;
  offerId?: string;
  groupId?: string;
  abilityId?: string;
  cost?: number;
  pointsAfter?: number;
  digest?: string;
  catalogueRevision?: string;
  localHandlerId?: string;
  effectKind?: string;
  durationFrames?: number;
  source?: Source;
  scope?: Scope;
  recipientCount?: number;
  provenance?: string;
}
export interface RecipientStatus {
  deliveryId?: string;
  gameSessionId?: string;
  attachmentId?: string;
  worldToken?: string;
  outcome?: string;
  receiptId?: string;
}
export interface Delivery {
  sequence?: number;
  operationId?: string;
  deliveryId?: string;
  abilityId?: string;
  catalogueRevision?: string;
  localHandlerId?: string;
  effectKind?: string;
  durationFrames?: number;
  source?: Source;
  scope?: Scope;
  recipientGameSessionId?: string;
  outcome?: string;
  provenance?: string;
}
export interface Request {
  major?: number;
  revision?: string;
  serviceId?: string;
  serviceEpoch?: string;
  sessionEpoch?: string;
  requestId?: string;
  scope?: Scope;
  catalogueRevision?: string;
  purchase?: Purchase;
  status?: Status;
  pending?: Pending;
  receipt?: Receipt;
}
export interface Response {
  major?: number;
  revision?: string;
  serviceId?: string;
  serviceEpoch?: string;
  requestId?: string;
  error?: Error;
  purchase?: PurchaseResult;
  original?: Original;
  recipientStatuses?: RecipientStatus[];
  nextRecipientKey?: string;
  highRecipientKey?: string;
  deliveries?: Delivery[];
  nextSequence?: number;
  highSequence?: number;
}
