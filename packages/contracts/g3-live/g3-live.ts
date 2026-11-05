// GENERATED from schema.json. Do not edit.
export const LIVE_LIMITS = {"bytes":32768,"fields":2048,"depth":8} as const;
export interface Scope {
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
export interface Pose {
  sequence?: number;
  xMilli?: string;
  yMilli?: string;
  vxMilli?: string;
  vyMilli?: string;
  facingLeft?: boolean;
}
export interface Empty {
}
export interface Authenticate {
  accessToken?: string;
}
export interface Reauthenticate {
  accessToken?: string;
  authRevision?: number;
}
export interface Welcome {
  authRevision?: number;
  accessExpiresAt?: string;
  authorityUntil?: string;
  poseHz?: number;
  snapshotHz?: number;
}
export interface Peer {
  principalId?: string;
  sessionId?: string;
  attachmentId?: string;
  worldToken?: string;
  poses?: Pose[];
  acceptedAt?: string;
  peerAgeMsAtSend?: number;
  poseAgesMsAtSend?: number[];
}
export interface Snapshot {
  peers?: Peer[];
  acceptedAt?: string;
}
export interface Retire {
  code?: string;
  retryAfterMs?: number;
}
export interface RoomSemantics {
  revision?: string;
  kind?: string;
  requiredCapabilities?: string[];
}
export interface ClientFrame {
  major?: number;
  revision?: string;
  serviceId?: string;
  serviceEpoch?: string;
  connectionId?: string;
  clientGeneration?: number;
  authorityGeneration?: number;
  sequence?: number;
  scope?: Scope;
  authenticate?: Authenticate;
  reauthenticate?: Reauthenticate;
  pose?: Pose;
  ping?: Empty;
}
export interface ServerFrame {
  major?: number;
  revision?: string;
  serviceId?: string;
  serviceEpoch?: string;
  connectionId?: string;
  clientGeneration?: number;
  authorityGeneration?: number;
  sequence?: number;
  scope?: Scope;
  welcome?: Welcome;
  snapshot?: Snapshot;
  retire?: Retire;
  pong?: Empty;
  ackClientSequence?: number;
  authorityRemainingMs?: number;
  accessRemainingMs?: number;
  credentialId?: string;
  bankChanged?: BankChanged;
  c1Changed?: C1Changed;
  c2Changed?: C2Changed;
}
export interface BankChanged {
  revision?: string;
}
export interface C1Changed {
  sequence?: string;
}
export interface C2Changed {
  sequence?: string;
}
