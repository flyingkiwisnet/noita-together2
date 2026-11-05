// GENERATED from schema.json. Do not edit.
export const HOURGLASS2_LIMITS = {"bytes":32768,"fields":2048,"depth":8} as const;
export interface Scope {
  roomId?: string;
  runId?: string;
  runEpoch?: string;
  configRevision?: number;
  attachmentId?: string;
  worldToken?: string;
}
export interface Activate {
  activationId?: string;
  effectId?: string;
  materialId?: string;
}
export interface Status {
  activationId?: string;
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
export interface Source {
  principalId?: string;
  name?: string;
  gameSessionId?: string;
  attachmentId?: string;
  worldToken?: string;
}
export interface Delivery {
  deliveryId?: string;
  activationId?: string;
  effectId?: string;
  materialId?: string;
  catalogueRevision?: string;
  source?: Source;
  scope?: Scope;
  recipientGameSessionId?: string;
  recipientAttachmentId?: string;
  recipientWorldToken?: string;
  outcome?: string;
  provenance?: string;
  sequence?: number;
}
export interface Activation {
  activationId?: string;
  effectId?: string;
  materialId?: string;
  recipientCount?: number;
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
  catalogueRevision?: string;
  activate?: Activate;
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
  activation?: Activation;
  deliveries?: Delivery[];
  nextSequence?: number;
  highSequence?: number;
  original?: Original;
  recipientStatuses?: RecipientStatus[];
  nextRecipientKey?: string;
  highRecipientKey?: string;
}
export interface Original {
  activationId?: string;
  digest?: string;
  effectId?: string;
  materialId?: string;
  catalogueRevision?: string;
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
