// GENERATED from schema.json. Do not edit.
export const SOCIAL_LIMITS = {"bytes":32768,"fields":2048,"depth":8} as const;
export interface Scope {
  roomId?: string;
  runId?: string;
  runEpoch?: string;
  configRevision?: number;
  attachmentId?: string;
  worldToken?: string;
}
export interface Error {
  code?: string;
}
export interface Hud {
  hpMilli?: number;
  maxHpMilli?: number;
  sampo?: boolean;
  hpKnown?: boolean;
  sampoKnown?: boolean;
  biome?: string;
}
export interface Emote {
  emoteId?: string;
  active?: boolean;
}
export interface Cosmetic {
  robeId?: string;
  hat?: boolean;
  amulet?: boolean;
  gem?: boolean;
}
export interface Poi {
  text?: string;
  xMilli?: string;
  yMilli?: string;
}
export interface Publish {
  eventId?: string;
  sequence?: number;
  hud?: Hud;
  emote?: Emote;
  cosmetic?: Cosmetic;
  poi?: Poi;
}
export interface Page {
  afterSequence?: string;
  limit?: number;
}
export interface Snapshot {
  afterKey?: string;
  limit?: number;
}
export interface Dismiss {
  poiSequence?: string;
}
export interface Projection {
  principalId?: string;
  gameSessionId?: string;
  displayName?: string;
  scope?: Scope;
  eventId?: string;
  sequence?: number;
  capturedAt?: string;
  expiresAt?: string;
  hud?: Hud;
  emote?: EmoteState;
  cosmetic?: Cosmetic;
  provenance?: string;
  hudStateRevision?: number;
  presentationRevision?: number;
}
export interface Marker {
  poiSequence?: string;
  eventId?: string;
  principalId?: string;
  gameSessionId?: string;
  displayName?: string;
  scope?: Scope;
  poi?: Poi;
  capturedAt?: string;
  provenance?: string;
}
export interface Request {
  major?: number;
  revision?: string;
  serviceId?: string;
  serviceEpoch?: string;
  sessionEpoch?: string;
  requestId?: string;
  scope?: Scope;
  publish?: Publish;
  snapshot?: Snapshot;
  poiSnapshot?: Page;
  poiOutbox?: Page;
  dismissPoi?: Dismiss;
  roster?: Snapshot;
  poiOutcome?: PoiLookup;
}
export interface Response {
  major?: number;
  revision?: string;
  serviceId?: string;
  serviceEpoch?: string;
  sessionEpoch?: string;
  requestId?: string;
  scope?: Scope;
  acceptedEventId?: string;
  duplicate?: boolean;
  projections?: Projection[];
  markers?: Marker[];
  nextKey?: string;
  nextSequence?: string;
  highSequence?: string;
  error?: Error;
  players?: Player[];
  poiOutcome?: PoiOutcome;
  rosterComplete?: boolean;
}
export interface EmoteState {
  emoteId?: string;
  active?: boolean;
  durationFrames?: number;
  ageMs?: number;
}
export interface Player {
  playerId?: string;
  principalId?: string;
  displayName?: string;
  gameSessionId?: string;
  worldToken?: string;
  state?: string;
  inventoryRevision?: number;
  heldWandSpriteId?: string;
  raceRowKey?: string;
  raceStatus?: string;
  completedGoals?: number;
}
export interface PoiLookup {
  eventId?: string;
}
export interface PoiOutcome {
  eventId?: string;
  status?: string;
  marker?: Marker;
}
