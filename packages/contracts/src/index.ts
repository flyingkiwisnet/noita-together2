// Temporary JSON fixture contract. Not a frozen production protocol or generated codec.
export const revision = 'g0-json-1' as const;
export const serverId = 'nt2-g0-local' as const;
export const limits = { frameBytes: 65536, chatChars: 500, chatHistory: 50, sessions: 64, requestsPerSession: 256, messagesPerSecond: 30, outboundBytes: 262144 } as const;
export type Persona = 'host' | 'guest' | 'observer';
export type ConnectionKind = 'browser' | 'game' | 'desktop';
export type RunState = 'waiting' | 'preparing' | 'loading' | 'running' | 'cancelled';
export interface Session { accountId: string; connectionId: string; epoch: string; kind: ConnectionKind; expiresAt: number; attachedDesktopId?: string; }
export interface Player { accountId: string; connectionId: string; epoch: string; displayName: string; ready: boolean; seedAccepted: boolean; loaded: boolean; }
export interface Room {
  id: string; name: string; hostAccountId: string; revision: number; configRevision: number;
  capacity: number; seed: number; state: RunState; runId: string | null;
  players: Player[]; roster: string[];
  chat: { accountId: string; text: string }[];
}
export type CommandKind = 'room.join' | 'room.leave' | 'game.ready' | 'chat.send' | 'run.start' | 'game.seedAccepted' | 'game.loaded' | 'run.cancel' | 'transfer.status';
export interface Command {
  contractRevision: typeof revision; requestId: string; roomId: string; runId: string | null;
  connectionEpoch: string; expectedRevision: number; kind: CommandKind;
  body: Record<string, unknown>;
}
export type ErrorCode = 'INVALID_MESSAGE' | 'INCOMPATIBLE_VERSION' | 'UNAUTHENTICATED' | 'FORBIDDEN' | 'NOT_FOUND' | 'STALE_REVISION' | 'STALE_EPOCH' | 'WRONG_RUN' | 'ROOM_FULL' | 'NOT_READY' | 'CONFLICT' | 'UNSUPPORTED_G0' | 'RATE_LIMITED' | 'PAIRING_PENDING' | 'PAIRING_EXPIRED' | 'PAIRING_CANCELLED' | 'ALREADY_EXCHANGED' | 'WRONG_SERVICE';
export class BoundaryError extends Error {
  constructor(public readonly code: ErrorCode) { super(code); }
}
function record(value: unknown): asserts value is Record<string, unknown> {
  if (!value || typeof value !== 'object' || Array.isArray(value)) throw new BoundaryError('INVALID_MESSAGE');
}
export function exactKeys(value: unknown, keys: readonly string[]): asserts value is Record<string, unknown> {
  record(value);
  if (Object.keys(value).some(key => !keys.includes(key)) || keys.some(key => !Object.hasOwn(value, key))) throw new BoundaryError('INVALID_MESSAGE');
}
export function boundedId(value: unknown): value is string { return typeof value === 'string' && /^[a-zA-Z0-9_-]{1,80}$/.test(value); }
const bodies: Record<CommandKind, readonly string[]> = {
  'room.join': [], 'room.leave': [], 'game.ready': ['ready'], 'chat.send': ['text'],
  'run.start': [], 'game.seedAccepted': ['seed'], 'game.loaded': [], 'run.cancel': [], 'transfer.status': ['operationId']
};
export function parseCommand(value: unknown): Command {
  exactKeys(value, ['contractRevision','requestId','roomId','runId','connectionEpoch','expectedRevision','kind','body']);
  if (value.contractRevision !== revision) throw new BoundaryError('INCOMPATIBLE_VERSION');
  if (!boundedId(value.requestId) || !boundedId(value.roomId) || !boundedId(value.connectionEpoch)
      || !(value.runId === null || boundedId(value.runId))
      || !Number.isSafeInteger(value.expectedRevision) || (value.expectedRevision as number) < 0
      || typeof value.kind !== 'string' || !Object.hasOwn(bodies, value.kind)) throw new BoundaryError('INVALID_MESSAGE');
  const kind = value.kind as CommandKind;
  exactKeys(value.body, bodies[kind]);
  if (kind === 'game.ready' && typeof value.body.ready !== 'boolean') throw new BoundaryError('INVALID_MESSAGE');
  if (kind === 'chat.send' && (typeof value.body.text !== 'string' || !value.body.text.trim() || value.body.text.length > limits.chatChars)) throw new BoundaryError('INVALID_MESSAGE');
  if (kind === 'game.seedAccepted' && (!Number.isSafeInteger(value.body.seed) || (value.body.seed as number) < 0 || (value.body.seed as number) > 4294967295)) throw new BoundaryError('INVALID_MESSAGE');
  if (kind === 'transfer.status' && !boundedId(value.body.operationId)) throw new BoundaryError('INVALID_MESSAGE');
  return value as unknown as Command;
}
