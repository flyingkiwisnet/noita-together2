// GENERATED from schema.json. Do not edit.
export const STATS_LIMITS = {"bytes":32768,"fields":2048,"depth":8,"page":32,"eventsPerRun":65536,"readConcurrency":8} as const;
export interface Empty {
}
export interface Discover {
  afterLocator?: string;
  limit?: number;
}
export interface Read {
  locator?: string;
  afterRow?: string;
  limit?: number;
  runKey?: string;
  summaryRevision?: string;
}
export interface Scope {
  roomId?: string;
  runId?: string;
  runEpoch?: string;
  configRevision?: number;
  attachmentId?: string;
  worldToken?: string;
}
export interface Observe {
  eventId?: string;
  kind?: string;
  progression?: number;
  orbId?: string;
}
export interface Lobby {
  locator?: string;
  name?: string;
  mode?: string;
  occupancy?: number;
  capacity?: number;
  state?: string;
  publicStats?: boolean;
}
export interface Row {
  rowKey?: string;
  displayName?: string;
  hearts?: string;
  orbs?: string;
  steveKills?: string;
  deaths?: string;
  wins?: string;
  goldDeposited?: string;
  goldWithdrawn?: string;
  itemsDeposited?: string;
  itemsClaimed?: string;
}
export interface Summary {
  locator?: string;
  runKey?: string;
  summaryRevision?: string;
  collectionState?: string;
  runStatus?: string;
  startedAt?: string;
  endedAt?: string;
  elapsedSeconds?: string;
  rows?: Row[];
  nextRow?: string;
  lobbyName?: string;
  retentionDays?: number;
  gameReported?: boolean;
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
  ownerGeneration?: number;
  discover?: Discover;
  read?: Read;
  observe?: Observe;
}
export interface Response {
  major?: number;
  revision?: string;
  serviceId?: string;
  serviceEpoch?: string;
  requestId?: string;
  lobbies?: Lobby[];
  nextLocator?: string;
  summary?: Summary;
  accepted?: boolean;
  duplicate?: boolean;
  error?: Error;
}
