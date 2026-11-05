// GENERATED from schema.json. Do not edit.
export const CONNECTION_LIMITS = {"bytes":65536,"fields":2048,"depth":12} as const;
export interface Empty {
}
export interface ConnectionScope {
  principalId?: string;
  sessionId?: string;
  sessionEpoch?: string;
  roomId?: string;
  runId?: string;
  runEpoch?: string;
  configRevision?: number;
  attachmentId?: string;
  worldToken?: string;
}
export interface ConnectionOwner {
  familyId?: string;
  desktopId?: string;
  desktopSessionEpoch?: string;
  gameSessionId?: string;
  gameSessionEpoch?: string;
  attachmentId?: string;
  gameInstanceId?: string;
}
export interface ConnectionInspection {
  desktopId?: string;
  attachmentId?: string;
  worldToken?: string;
  observationRevision?: number;
  contentDigest?: string;
  capturedAt?: string;
  gameSessionId?: string;
  gameSessionEpoch?: string;
  desktopSessionEpoch?: string;
  gameInstanceId?: string;
  actualSeed?: string;
  seedPolicy?: string;
  randomizeLoot?: boolean;
}
export interface ConnectionStart {
  requestId?: string;
  authorizedAt?: string;
  selectedInspection?: ConnectionInspection;
}
export interface ConnectionImmutable {
  authorizationId?: string;
  startRequestId?: string;
  authorizedAt?: string;
  selectedInspection?: ConnectionInspection;
}
export interface ConnectionReport {
  revision?: number;
  contentDigest?: string;
  capturedAt?: string;
  complete?: boolean;
  loaded?: boolean;
  protected?: boolean;
  contentValid?: boolean;
}
export interface ConnectionDecision {
  authorized?: boolean;
  playing?: boolean;
  terminal?: boolean;
  reason?: string;
  remainingMs?: number;
  initialReviewRemainingMs?: number;
}
export interface ConnectionSample {
  originRequestId?: string;
  channel?: string;
  credentialId?: string;
  sampleKey?: string;
}
export interface ConnectionCredential {
  principalId?: string;
  sessionId?: string;
  sessionEpoch?: string;
  credentialId?: string;
  accessRemainingMs?: number;
  originRequestId?: string;
  sampleKey?: string;
}
export interface ConnectionAdmission {
  serviceId?: string;
  serviceEpoch?: string;
  participantId?: string;
  generation?: number;
  scope?: ConnectionScope;
  owner?: ConnectionOwner;
  originalStart?: ConnectionStart;
  immutable?: ConnectionImmutable;
  currentReport?: ConnectionReport;
  decision?: ConnectionDecision;
  sample?: ConnectionSample;
}
export interface OriginalOperation {
  requestId?: string;
  kind?: string;
  fingerprint?: string;
  originalSessionId?: string;
  originalSessionEpoch?: string;
  originalOwnerGeneration?: number;
  participantId?: string;
  roomId?: string;
  runId?: string;
  worldToken?: string;
  configRevision?: number;
  originalServiceEpoch?: string;
  accountId?: string;
  deviceId?: string;
}
export interface Outcome {
  original?: OriginalOperation;
  originalOutcome?: string;
  coverage?: string;
  currentDisposition?: string;
  currentReason?: string;
  currentGeneration?: number;
  effectId?: string;
  retryAfterMs?: number;
}
export interface AdmissionRead {
  roomId?: string;
  participantId?: string;
}
export interface Error {
  code?: string;
  retryAfterMs?: number;
}
export interface Request {
  major?: number;
  revision?: string;
  serviceId?: string;
  serviceEpoch?: string;
  sessionEpoch?: string;
  requestId?: string;
  admission?: AdmissionRead;
  outcome?: OriginalOperation;
}
export interface Response {
  major?: number;
  revision?: string;
  serviceId?: string;
  serviceEpoch?: string;
  requestId?: string;
  admission?: ConnectionAdmission;
  outcome?: Outcome;
  error?: Error;
  credentialValidity?: ConnectionCredential;
}
