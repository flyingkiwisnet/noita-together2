// GENERATED from schema.json. Do not edit.
export const C1_LIMITS = {"bytes":32768,"fields":2048,"depth":10} as const;
export interface Scope {
  roomId?: string;
  runId?: string;
  runEpoch?: string;
  configRevision?: number;
  attachmentId?: string;
  worldToken?: string;
}
export interface Empty {
}
export interface Rules {
  NT_sync_hearts?: boolean;
  NT_sync_orbs?: boolean;
  NT_sync_shift?: boolean;
  NT_team_perks?: boolean;
  NT_sync_perks?: boolean;
  NT_alt_heartache?: boolean;
  NT_ban_perks?: boolean;
  NT_sync_orbs_no_curse?: boolean;
  NT_ondeath_kick?: boolean;
  NT_sync_orb_count?: number;
  NT_death_penalty?: string;
}
export interface RuleSnapshot {
  revision?: number;
  sequence?: string;
  rules?: Rules;
}
export interface RuleSet {
  expectedRevision?: number;
  rules?: Rules;
}
export interface Heart {
}
export interface Orb {
  orbId?: number;
}
export interface Perk {
  perkId?: string;
}
export interface Fungal {
  fromMaterials?: string[];
  toMaterial?: string;
  displayMaterial?: string;
}
export interface Death {
  state?: string;
  cause?: string;
  progression?: number;
}
export interface Payload {
  heart?: Heart;
  orb?: Orb;
  perk?: Perk;
  fungal?: Fungal;
  death?: Death;
}
export interface Publish {
  eventId?: string;
  rulesRevision?: number;
  payload?: Payload;
  senderDisposition?: string;
}
export interface Delivery {
  eventId?: string;
  sequence?: string;
  rulesRevision?: number;
  sourcePrincipalId?: string;
  sourceWorldToken?: string;
  sourceName?: string;
  payload?: Payload;
  state?: string;
  reason?: string;
  senderDisposition?: string;
  capturedRules?: Rules;
}
export interface Receipt {
  eventId?: string;
  sequence?: string;
  state?: string;
  reason?: string;
}
export interface Page {
  limit?: number;
}
export interface LifeUpdate {
  deathEventId?: string;
  state?: string;
}
export interface Life {
  principalId?: string;
  worldToken?: string;
  state?: string;
  deathEventId?: string;
  rulesRevision?: number;
}
export interface Member {
  principalId?: string;
  displayName?: string;
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
  roomId?: string;
  scope?: Scope;
  ownerGeneration?: number;
  rulesGet?: Empty;
  rulesSet?: RuleSet;
  publish?: Publish;
  deliveries?: Page;
  receipt?: Receipt;
  life?: LifeUpdate;
  sourceOutcome?: SourceOutcomeRequest;
}
export interface Response {
  major?: number;
  revision?: string;
  serviceId?: string;
  serviceEpoch?: string;
  requestId?: string;
  scope?: Scope;
  ownerGeneration?: number;
  snapshot?: RuleSnapshot;
  event?: Delivery;
  deliveries?: Delivery[];
  duplicate?: boolean;
  members?: Member[];
  memberCount?: number;
  lives?: Life[];
  error?: Error;
  more?: boolean;
  sequence?: string;
  sourceOutcome?: SourceOutcome;
}
export interface SourceOriginal {
  roomId?: string;
  runId?: string;
  worldToken?: string;
  configRevision?: number;
  publish?: Publish;
}
export interface SourceDisposition {
  state?: string;
  reason?: string;
}
export interface SourceOutcomeRequest {
  original?: SourceOriginal;
  receipt?: SourceDisposition;
}
export interface SourceOutcome {
  original?: SourceOriginal;
  outcome?: string;
  coverage?: string;
  sourceDisposition?: string;
  sourceReason?: string;
  acceptedSequence?: string;
  rulesRevision?: number;
  effectAuthority?: boolean;
  selfReported?: boolean;
  savedEffectProof?: boolean;
  duplicate?: boolean;
}
