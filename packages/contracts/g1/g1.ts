// GENERATED from schema.json. Do not edit.
export const G1_LIMITS = {"bytes":65536,"fields":2048,"depth":8,"rooms":32,"members":32,"capabilities":128,"configBytes":8192} as const;
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
  pairBegin?: PairBegin;
  pairApprove?: PairApprove;
  pairPoll?: PairProof;
  pairExchange?: PairProof;
  pairCancel?: PairProof;
  refresh?: Refresh;
  logout?: Empty;
  roomCreate?: RoomCreate;
  roomList?: ListRooms;
  roomGet?: Empty;
  roomJoin?: Empty;
  roomLeave?: Empty;
  presence?: Presence;
  ready?: Ready;
  configure?: Config;
  start?: Start;
  cancelRun?: Empty;
  endRun?: Empty;
  gameAttach?: GameAttach;
  gameSeedApplied?: GameFact;
  gameLoaded?: GameFact;
  gameResume?: GameFact;
  roomPreview?: Empty;
  roomAccess?: RoomAccess;
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
}
export interface RoomSummary {
  roomId?: string;
  name?: string;
  state?: string;
  revision?: number;
  configRevision?: number;
  visibility?: string;
  joinAllowed?: boolean;
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
}
