// GENERATED from schema.json. Do not edit.
export const HISTORY_LIMITS = {"bytes":32768,"depth":8} as const;
export interface Discover {
  afterSequence?: string;
  throughSequence?: string;
  limit?: number;
}
export interface RunScope {
  roomId?: string;
  runId?: string;
  runEpoch?: string;
  configRevision?: number;
}
export interface Run {
  sequence?: string;
  scope?: RunScope;
  roomName?: string;
  createdAt?: string;
  state?: string;
  mode?: string;
  canDelete?: boolean;
  retentionDays?: number;
  collectionState?: string;
  latestExpiresAt?: string;
  deletedAt?: string;
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
  discover?: Discover;
}
export interface Response {
  major?: number;
  revision?: string;
  serviceId?: string;
  serviceEpoch?: string;
  sessionEpoch?: string;
  requestId?: string;
  runs?: Run[];
  nextSequence?: string;
  highSequence?: string;
  error?: Error;
}
