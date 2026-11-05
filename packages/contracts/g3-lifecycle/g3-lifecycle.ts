// GENERATED from schema.json. Do not edit.
export const LIFECYCLE_LIMITS = {"bytes":65536,"fields":2048,"depth":12} as const;
export interface PrepareWorld {
  worldId?: string;
  roomId?: string;
  proof?: string;
  configRevision?: number;
  configDigest?: string;
}
export interface PreparedWorld {
  association?: WorldAssociation;
  config?: Config;
  runEpoch?: string;
  reviewRemainingMs?: number;
}
export interface WorldAssociation {
  worldId?: string;
  roomId?: string;
  runId?: string;
  preparationId?: string;
  configRevision?: number;
  configDigest?: string;
}
export interface Reconcile {
  association?: WorldAssociation;
  proof?: string;
  attempt?: number;
  connectionId?: string;
  action?: string;
  gameVersion?: string;
  modVersion?: string;
  bridgeVersion?: string;
  capabilities?: string[];
  observation?: GameObservation;
}
export interface WorldState {
  association?: WorldAssociation;
  state?: string;
  reason?: string;
  attempt?: number;
  generation?: number;
  mayPlay?: boolean;
  retryAfterMs?: number;
  ownerRemainingMs?: number;
  reviewRemainingMs?: number;
  runEpoch?: string;
  participantId?: string;
  currentReport?: ConnectionReport;
  connectionId?: string;
}
export interface Request {
  major?: number;
  revision?: string;
  serviceId?: string;
  serviceEpoch?: string;
  sessionEpoch?: string;
  requestId?: string;
  reconcile?: Reconcile;
  prepare?: PrepareWorld;
}
export interface Response {
  major?: number;
  revision?: string;
  serviceId?: string;
  serviceEpoch?: string;
  requestId?: string;
  world?: WorldState;
  credentials?: Credentials;
  credentialValidity?: ConnectionCredential;
  canonicalAdmission?: ConnectionAdmission;
  prepared?: PreparedWorld;
  error?: Error;
}
export interface Config {
  seed?: number;
  mode?: string;
  settings?: Setting[];
  requiredCapabilities?: string[];
  seedPolicy?: string;
  randomizeLoot?: boolean;
  race?: RaceConfig;
}
export interface Setting {
  key?: string;
  value?: string;
}
export interface RaceConfig {
  registryRevision?: string;
  goals?: RaceGoal[];
  durationMs?: number;
}
export interface RaceGoal {
  instanceId?: string;
  id?: string;
  target?: number;
  bossIds?: string[];
  biomeId?: string;
  direction?: string;
  index?: number;
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
export interface ConnectionReport {
  revision?: number;
  contentDigest?: string;
  capturedAt?: string;
  complete?: boolean;
  loaded?: boolean;
  protected?: boolean;
  contentValid?: boolean;
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
export interface ConnectionOwner {
  familyId?: string;
  desktopId?: string;
  desktopSessionEpoch?: string;
  gameSessionId?: string;
  gameSessionEpoch?: string;
  attachmentId?: string;
  gameInstanceId?: string;
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
export interface ConnectionSample {
  originRequestId?: string;
  channel?: string;
  credentialId?: string;
  sampleKey?: string;
}
export interface Error {
  code?: string;
  retryAfterMs?: number;
}
