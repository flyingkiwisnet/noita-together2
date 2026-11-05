// GENERATED from schema.json. Do not edit.
export const WAITING_LIMITS = {"bytes":65536,"fields":2048,"depth":12} as const;
export interface ConnectionOwner {
  familyId?: string;
  desktopId?: string;
  desktopSessionEpoch?: string;
  gameSessionId?: string;
  gameSessionEpoch?: string;
  attachmentId?: string;
  gameInstanceId?: string;
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
export interface ConnectionStart {
  requestId?: string;
  authorizedAt?: string;
  selectedInspection?: ConnectionInspection;
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
export interface ConnectionImmutable {
  authorizationId?: string;
  startRequestId?: string;
  authorizedAt?: string;
  selectedInspection?: ConnectionInspection;
}
export interface ConnectionDecision {
  authorized?: boolean;
  playing?: boolean;
  terminal?: boolean;
  reason?: string;
  remainingMs?: number;
  initialReviewRemainingMs?: number;
}
export interface Credentials {
  accessToken?: string;
  refreshSecret?: string;
  accountId?: string;
  sessionId?: string;
  sessionEpoch?: string;
  kind?: string;
  deviceId?: string;
  expiresAt?: string;
  attachmentId?: string;
  principal?: Principal;
}
export interface Principal {
  principalId?: string;
  kind?: string;
  displayName?: string;
  source?: string;
}
export interface GameObservation {
  attachmentId?: string;
  gameInstanceId?: string;
  worldToken?: string;
  observationRevision?: number;
  actualSeed?: string;
  activeMods?: string[];
  inventoryComplete?: boolean;
  beta?: string;
  protected?: boolean;
  loaded?: boolean;
  seedStatus?: string;
  collectorFailure?: string;
  modVersion?: string;
  bridgeVersion?: string;
  compatibilityProvenance?: string;
}
export interface Empty {
}
export interface WaitingIntent {
  waitingId?: string;
  serviceId?: string;
  accountId?: string;
  deviceId?: string;
  preparationId?: string;
  originalAttachmentId?: string;
  originalAttachRequestId?: string;
  originalAttachFingerprint?: string;
  originalDesktopId?: string;
  originalDesktopEpoch?: string;
  roomId?: string;
  runId?: string;
  runEpoch?: string;
  preparationServiceEpoch?: string;
  configDigest?: string;
  worldToken?: string;
  actualSeed?: string;
  seedPolicy?: string;
  originalGameInstanceId?: string;
  contextId?: string;
  contentDigest?: string;
  configRevision?: number;
  randomizeLoot?: boolean;
  originalObservationRevision?: number;
  pendingReservationId?: string;
}
export interface WaitingState {
  intent?: WaitingIntent;
  generation?: number;
  owner?: ConnectionOwner;
  state?: string;
  reason?: string;
  reviewRemainingMs?: number;
  ownerRemainingMs?: number;
  mayPlay?: boolean;
  sample?: ConnectionSample;
  currentReport?: ConnectionReport;
  admittedParticipantId?: string;
  currentContextId?: string;
}
export interface Enroll {
  preparationId?: string;
  attachmentId?: string;
  originalAttachRequestId?: string;
  originalAttachFingerprint?: string;
  worldToken?: string;
  gameInstanceId?: string;
  observationRevision?: number;
  contentDigest?: string;
  contextId?: string;
}
export interface Recover {
  waitingProof?: string;
  gameVersion?: string;
  modVersion?: string;
  bridgeVersion?: string;
  capabilities?: string[];
  observation?: GameObservation;
  contextId?: string;
}
export interface Proof {
  waitingProof?: string;
}
export interface WaitingOriginal {
  accountId?: string;
  deviceId?: string;
  originalSessionId?: string;
  originalSessionEpoch?: string;
  originalServiceEpoch?: string;
  requestId?: string;
  kind?: string;
  fingerprint?: string;
  waitingId?: string;
}
export interface WaitingOutcome {
  original?: WaitingOriginal;
  originalOutcome?: string;
  coverage?: string;
  waitingId?: string;
  effectId?: string;
  currentDisposition?: string;
  retryAfterMs?: number;
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
  waitingId?: string;
  generation?: number;
  enroll?: Enroll;
  recover?: Recover;
  status?: Empty;
  outcome?: WaitingOriginal;
  offline?: Proof;
  pendingId?: string;
  reserve?: Reserve;
  pendingRecover?: PendingRecover;
  pendingStatus?: Empty;
  pendingOffline?: PendingOffline;
  pendingOutcome?: WaitingOriginal;
  entryPreview?: EntryPreviewRequest;
  entryActivate?: EntryActivate;
  entryCancel?: EntryCancel;
  entryStatus?: EntryLookup;
  entryOutcome?: EntryOriginal;
}
export interface Response {
  major?: number;
  revision?: string;
  serviceId?: string;
  serviceEpoch?: string;
  requestId?: string;
  waiting?: WaitingState;
  waitingProof?: string;
  credentials?: Credentials;
  credentialValidity?: ConnectionCredential;
  canonicalAdmission?: ConnectionAdmission;
  outcome?: WaitingOutcome;
  error?: Error;
  pending?: PendingState;
  pendingProof?: string;
  pendingOutcome?: PendingOutcome;
  resumeProof?: string;
  participant?: Participant;
  entryPreview?: EntryPreview;
  entry?: EntryState;
  entryOutcome?: EntryOutcome;
}
export interface Reserve {
  preparationId?: string;
  worldToken?: string;
  contextId?: string;
}
export interface PendingIntent {
  pendingId?: string;
  preparationId?: string;
  serviceId?: string;
  serviceEpoch?: string;
  roomId?: string;
  runId?: string;
  runEpoch?: string;
  configRevision?: number;
  configDigest?: string;
  accountId?: string;
  deviceId?: string;
  originalDesktopId?: string;
  originalDesktopEpoch?: string;
  worldToken?: string;
  contextId?: string;
  seedPolicy?: string;
  randomizeLoot?: boolean;
  actualSeed?: string;
}
export interface PendingState {
  intent?: PendingIntent;
  generation?: number;
  state?: string;
  reason?: string;
  mayPlay?: boolean;
  waitingId?: string;
  sample?: ConnectionSample;
  preparationRemainingMs?: number;
  reviewRemainingMs?: number;
  boundActualSeed?: string;
  preparationState?: string;
}
export interface PendingRecover {
  gameVersion?: string;
  modVersion?: string;
  bridgeVersion?: string;
  capabilities?: string[];
  observation?: GameObservation;
  contextId?: string;
  pendingProof?: string;
}
export interface PendingOffline {
  pendingProof?: string;
}
export interface PendingOutcome {
  original?: WaitingOriginal;
  originalOutcome?: string;
  coverage?: string;
  pendingId?: string;
  effectId?: string;
}
export interface Association {
  serviceId?: string;
  roomId?: string;
  runId?: string;
  participantId?: string;
  worldToken?: string;
  accountId?: string;
  originalStartId?: string;
  configRevision?: number;
  actualSeed?: string;
  seedPolicy?: string;
  randomizeLoot?: boolean;
}
export interface Participant {
  association?: Association;
  state?: string;
  reason?: string;
  generation?: number;
  serviceEpoch?: string;
  runEpoch?: string;
  desktopId?: string;
  gameSessionId?: string;
  attachmentId?: string;
  leaseExpiresAt?: string;
  returnEligible?: boolean;
  reviewRequired?: boolean;
  pendingObservationRevision?: number;
  pendingContentDigest?: string;
}
export interface EntryPreviewRequest {
  roomId?: string;
  basePreparationId?: string;
}
export interface EntryScope {
  serviceId?: string;
  serviceEpoch?: string;
  accountId?: string;
  deviceId?: string;
  roomId?: string;
  runId?: string;
  runEpoch?: string;
  configRevision?: number;
  configDigest?: string;
  basePreparationId?: string;
  baseFingerprint?: string;
  seedPolicy?: string;
  randomizeLoot?: boolean;
  actualSeed?: string;
}
export interface EntryIdentity {
  preparationId?: string;
  pendingId?: string;
  worldToken?: string;
  contextId?: string;
  proofCommitment?: string;
}
export interface EntryPreview {
  scope?: EntryScope;
  sample?: ConnectionSample;
  reviewRemainingMs?: number;
  mayPlay?: boolean;
}
export interface EntryLookup {
  scope?: EntryScope;
  identity?: EntryIdentity;
}
export interface EntryActivate {
  scope?: EntryScope;
  identity?: EntryIdentity;
  recoveryProof?: string;
  gameVersion?: string;
  modVersion?: string;
  bridgeVersion?: string;
  capabilities?: string[];
  observation?: GameObservation;
}
export interface EntryCancel {
  scope?: EntryScope;
  identity?: EntryIdentity;
  recoveryProof?: string;
  worldOffline?: boolean;
}
export interface EntryOriginal {
  accountId?: string;
  deviceId?: string;
  originalSessionId?: string;
  originalSessionEpoch?: string;
  originalServiceEpoch?: string;
  requestId?: string;
  kind?: string;
  fingerprint?: string;
  waitingId?: string;
  runId?: string;
  pendingId?: string;
}
export interface EntryState {
  scope?: EntryScope;
  identity?: EntryIdentity;
  state?: string;
  reason?: string;
  mayPlay?: boolean;
  attachmentId?: string;
  waitingId?: string;
}
export interface EntryOutcome {
  original?: EntryOriginal;
  originalOutcome?: string;
  coverage?: string;
  effectId?: string;
}
