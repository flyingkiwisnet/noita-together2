// GENERATED from schema.json. Do not edit.
export const G2_NEXT_LIMITS = {"bytes":65536,"fields":2048,"depth":8,"rooms":32,"members":32,"capabilities":128,"configBytes":8192} as const;
export interface Info {
  major?: number;
  revision?: string;
  serviceId?: string;
  serviceEpoch?: string;
  capabilities?: string[];
  compatibility?: Compatibility;
  limits?: Limits;
  mockProvider?: boolean;
  contractStatus?: string;
  profile?: CompatibilityProfile;
  connectionFlowRevision?: string;
}
export interface Compatibility {
  gameVersion?: string;
  modVersion?: string;
  bridgeVersion?: string;
  capabilities?: string[];
  modes?: string[];
  settingKeys?: string[];
}
export interface Limits {
  bytes?: number;
  fields?: number;
  depth?: number;
  rooms?: number;
  members?: number;
  capabilities?: number;
  configBytes?: number;
  requestWork?: number;
  databaseWork?: number;
  acquireMs?: number;
  statementMs?: number;
  lockMs?: number;
  transactionMs?: number;
  requestMs?: number;
}
export interface BrowserLoginResponse {
  credentials?: Credentials;
  error?: Error;
  credentialValidity?: ConnectionCredential;
}
export interface BrowserLogin {
  subject?: string;
  deviceId?: string;
  requestId?: string;
}
export interface Request {
  major?: number;
  revision?: string;
  serviceId?: string;
  requestId?: string;
  serviceEpoch?: string;
  sessionEpoch?: string;
  roomId?: string;
  expectedRevision?: number;
  configRevision?: number;
  runId?: string;
  runEpoch?: string;
  refresh?: Refresh;
  logout?: Empty;
  roomCreate?: RoomCreate;
  roomList?: ListRooms;
  roomGet?: Empty;
  roomJoin?: RoomJoin;
  roomLeave?: Empty;
  presence?: Presence;
  ready?: Ready;
  configure?: Config;
  cancelRun?: Empty;
  endRun?: Empty;
  gameAttach?: GameAttach;
  roomPreview?: Empty;
  roomAccess?: RoomAccess;
  chatSend?: ChatSend;
  chatList?: ChatList;
  roomPolicy?: RoomPolicy;
  roomModerate?: RoomModerate;
  guestIssue?: GuestIssue;
  authBegin?: AuthBegin;
  authStatus?: AuthProof;
  authCancel?: AuthProof;
  authExchange?: AuthExchange;
  prepare?: Prepare;
  gameObservation?: GameObservation;
  inspectionList?: InspectionList;
  finalStart?: FinalStart;
  gamePulse?: GamePulse;
  gameRenew?: GameRenew;
  preparationCancel?: PreparationCancel;
  preparationRenew?: PreparationRenew;
  roomDelete?: Empty;
  roomPlayerDetails?: PlayerTarget;
  roomBanList?: RoomBanList;
}
export interface Empty {
}
export interface PairBegin {
  deviceId?: string;
}
export interface PairApprove {
  attemptId?: string;
  code?: string;
}
export interface PairProof {
  attemptId?: string;
  proof?: string;
  deviceId?: string;
}
export interface Refresh {
  secret?: string;
  deviceId?: string;
}
export interface RoomCreate {
  name?: string;
  config?: Config;
  visibility?: string;
  allowGuests?: boolean;
  capacity?: number;
  password?: string;
}
export interface RoomAccess {
  visibility?: string;
  admittedAccountIds?: string[];
}
export interface ListRooms {
  limit?: number;
  afterRoomId?: string;
}
export interface Setting {
  key?: string;
  value?: string;
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
export interface Presence {
  present?: boolean;
}
export interface Ready {
  ready?: boolean;
}
export interface Start {
  participantDesktopIds?: string[];
  excludedDesktopIds?: string[];
}
export interface GameAttach {
  gameInstanceId?: string;
  gameVersion?: string;
  modVersion?: string;
  bridgeVersion?: string;
  capabilities?: string[];
  preparationId?: string;
}
export interface GameFact {
  attachmentId?: string;
  gameInstanceId?: string;
}
export interface Response {
  major?: number;
  revision?: string;
  serviceId?: string;
  serviceEpoch?: string;
  requestId?: string;
  error?: Error;
  pairing?: Pairing;
  credentials?: Credentials;
  room?: Room;
  rooms?: RoomSummary[];
  admission?: RoomSummary;
  nextRoomId?: string;
  chatMessages?: ChatMessage[];
  chatNextSequence?: number;
  chatReset?: boolean;
  authAttempt?: AuthAttempt;
  inspections?: Inspection[];
  inspectionNextDesktopId?: string;
  preparation?: Preparation;
  release?: GameRelease;
  gameStatus?: GameStatus;
  credentialValidity?: ConnectionCredential;
  canonicalAdmission?: ConnectionAdmission;
  deletedRoomId?: string;
  playerDetails?: PlayerDetails;
  roomBans?: PlayerDetails[];
  nextRoomBanAccountId?: string;
  access?: RoomAccess;
}
export interface Error {
  code?: string;
  message?: string;
  retryAfterMs?: number;
}
export interface Pairing {
  attemptId?: string;
  code?: string;
  proof?: string;
  state?: string;
  expiresAt?: string;
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
export interface RoomSummary {
  roomId?: string;
  name?: string;
  state?: string;
  revision?: number;
  configRevision?: number;
  visibility?: string;
  joinAllowed?: boolean;
  capacity?: number;
  passwordRequired?: boolean;
  allowGuests?: boolean;
  host?: Principal;
  mode?: string;
  playerCount?: number;
  joinLocked?: boolean;
  publicStats?: boolean;
  statsLocator?: string;
  statsAllowed?: boolean;
}
export interface Member {
  accountId?: string;
  desktopId?: string;
  present?: boolean;
  ready?: boolean;
  selected?: boolean;
  attached?: boolean;
  seedApplied?: boolean;
  loaded?: boolean;
  principal?: Principal;
  readiness?: string;
  readinessReason?: string;
  consentedConfigRevision?: number;
  included?: boolean;
  chatMuted?: boolean;
}
export interface Room {
  roomId?: string;
  name?: string;
  hostAccountId?: string;
  state?: string;
  revision?: number;
  configRevision?: number;
  config?: Config;
  runId?: string;
  runEpoch?: string;
  preparationExpiresAt?: string;
  members?: Member[];
  capacity?: number;
  passwordRequired?: boolean;
  allowGuests?: boolean;
  reviewExpiresAt?: string;
  host?: Principal;
  joinLocked?: boolean;
  visibility?: string;
  publicStats?: boolean;
  statsLocator?: string;
}
export interface RoomJoin {
  password?: string;
}
export interface ChatSend {
  text?: string;
}
export interface ChatList {
  afterSequence?: number;
  limit?: number;
}
export interface ChatMessage {
  sequence?: number;
  accountId?: string;
  sessionKind?: string;
  sentAt?: string;
  text?: string;
  principal?: Principal;
}
export interface RoomPolicy {
  capacity?: number;
  password?: string;
  allowGuests?: boolean;
  name?: string;
  joinLocked?: boolean;
  publicStats?: boolean;
}
export interface RoomModerate {
  accountId?: string;
  action?: string;
  desktopId?: string;
}
export interface CompatibilityProfile {
  id?: string;
  evidenceStatus?: string;
  gameExeSha256?: string;
  originalLuaSha256?: string;
  bridgeSha256?: string;
  modSha256?: string;
  gameFamilyId?: string;
  supportedBuilds?: GameBuildVariant[];
}
export interface Principal {
  principalId?: string;
  kind?: string;
  displayName?: string;
  source?: string;
}
export interface GuestIssue {
  deviceId?: string;
  displayName?: string;
  issuanceProof?: string;
}
export interface AuthBegin {
  deviceId?: string;
  appInstanceId?: string;
  provider?: string;
  proofChallenge?: string;
  callbackPort?: number;
  callbackState?: string;
}
export interface AuthProof {
  attemptId?: string;
  deviceId?: string;
  appInstanceId?: string;
  initiatorProof?: string;
}
export interface AuthExchange {
  attemptId?: string;
  deviceId?: string;
  appInstanceId?: string;
  initiatorProof?: string;
  completionGrant?: string;
}
export interface AuthAttempt {
  attemptId?: string;
  state?: string;
  provider?: string;
  launchUrl?: string;
  expiresAt?: string;
  grantExpiresAt?: string;
}
export interface Prepare {
  reviewMinutes?: number;
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
export interface Inspection {
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
  desktopId?: string;
  contentDigest?: string;
  capturedAt?: string;
  presenceUntil?: string;
  validUntil?: string;
  compatibility?: string;
  seedStatus?: string;
  collectorFailure?: string;
  modVersion?: string;
  bridgeVersion?: string;
  compatibilityProvenance?: string;
}
export interface InspectionList {
  afterDesktopId?: string;
  limit?: number;
}
export interface InspectedParticipant {
  desktopId?: string;
  attachmentId?: string;
  worldToken?: string;
  observationRevision?: number;
  contentDigest?: string;
}
export interface FinalStart {
  participants?: InspectedParticipant[];
  excludedDesktopIds?: string[];
}
export interface GamePulse {
  attachmentId?: string;
  gameInstanceId?: string;
  worldToken?: string;
}
export interface Preparation {
  preparationId?: string;
  serviceId?: string;
  serviceEpoch?: string;
  roomId?: string;
  runId?: string;
  runEpoch?: string;
  configRevision?: number;
  desktopId?: string;
  desktopSessionEpoch?: string;
  expiresAt?: string;
  reviewExpiresAt?: string;
  state?: string;
}
export interface PreparationCancel {
  preparationId?: string;
}
export interface GameRenew {
  attachmentId?: string;
  gameInstanceId?: string;
  worldToken?: string;
  gameSessionId?: string;
  gameSessionEpoch?: string;
}
export interface GameRelease {
  authorizationId?: string;
  startRequestId?: string;
  contractRevision?: string;
  serviceId?: string;
  serviceEpoch?: string;
  roomId?: string;
  runId?: string;
  runEpoch?: string;
  configRevision?: number;
  desktopId?: string;
  desktopSessionEpoch?: string;
  gameSessionId?: string;
  gameSessionEpoch?: string;
  attachmentId?: string;
  gameInstanceId?: string;
  worldToken?: string;
  observationRevision?: number;
  contentDigest?: string;
  capturedAt?: string;
  authorizedAt?: string;
  releaseExpiresAt?: string;
  phase?: string;
  enteredPlayingAt?: string;
  actualSeed?: string;
  seedPolicy?: string;
  randomizeLoot?: boolean;
}
export interface GameStatus {
  phase?: string;
  reason?: string;
}
export interface GameBuildVariant {
  id?: string;
  store?: string;
  branch?: string;
  buildIdentity?: string;
  gameExeSha256?: string;
  originalLuaSha256?: string;
  evidenceStatus?: string;
}
export interface PreparationRenew {
  preparationId?: string;
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
export interface PlayerTarget {
  accountId?: string;
  desktopId?: string;
}
export interface PlayerDetails {
  accountId?: string;
  desktopId?: string;
  principal?: Principal;
  role?: string;
  chatMuted?: boolean;
  banned?: boolean;
  roomRevision?: number;
  actions?: string[];
}
export interface RoomBanList {
  afterAccountId?: string;
  limit?: number;
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
export interface RaceConfig {
  registryRevision?: string;
  goals?: RaceGoal[];
  durationMs?: number;
}
