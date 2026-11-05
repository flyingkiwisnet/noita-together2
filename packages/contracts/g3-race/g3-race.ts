// GENERATED from schema.json. Do not edit.
export const RACE_LIMITS = {"bytes":262144,"fields":20000,"depth":8,"page":8,"batch":32,"eventsPerRun":65536} as const;
export interface Empty {
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
export interface Scope {
  roomId?: string;
  runId?: string;
  runEpoch?: string;
  configRevision?: number;
  attachmentId?: string;
  worldToken?: string;
}
export interface Observation {
  kind?: string;
  orbId?: number;
  bossId?: string;
  biomeId?: string;
  direction?: string;
  index?: number;
  value?: string;
  ending?: string;
}
export interface Commit {
  eventId?: string;
  sequence?: number;
  observations?: Observation[];
  terminal?: string;
}
export interface Read {
  locator?: string;
  afterRow?: string;
  limit?: number;
  runKey?: string;
  snapshotRevision?: string;
  self?: boolean;
}
export interface Credit {
  id?: string;
  firstAtMs?: string;
}
export interface Progress {
  instanceId?: string;
  value?: string;
  currentValue?: string;
  target?: string;
  complete?: boolean;
  completedAtMs?: string;
  credits?: Credit[];
}
export interface Entrant {
  rowKey?: string;
  displayName?: string;
  status?: string;
  completedGoals?: number;
  finalCountAtMs?: string;
  finishOrder?: number;
  rank?: number;
  progress?: Progress[];
  acceptedSequence?: number;
}
export interface Snapshot {
  locator?: string;
  runKey?: string;
  revision?: string;
  registryRevision?: string;
  config?: RaceConfig;
  status?: string;
  startedAt?: string;
  deadlineAt?: string;
  endedAt?: string;
  elapsedMs?: string;
  collectionState?: string;
  entrants?: Entrant[];
  nextRow?: string;
  lobbyName?: string;
  selfRow?: string;
  endReason?: string;
  lobbyMode?: string;
}
export interface Receipt {
  eventId?: string;
  sequence?: number;
  acceptedAtMs?: string;
  revision?: string;
  status?: string;
  completedGoals?: number;
  finishOrder?: number;
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
  commit?: Commit;
  read?: Read;
}
export interface Response {
  major?: number;
  revision?: string;
  serviceId?: string;
  serviceEpoch?: string;
  requestId?: string;
  receipt?: Receipt;
  snapshot?: Snapshot;
  error?: Error;
}
