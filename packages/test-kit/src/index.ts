import { randomUUID, randomBytes } from 'node:crypto';
import { applyMockCommand } from '@nt2/domain';
import { BoundaryError, limits, revision, serverId, type Command, type ConnectionKind, type Persona, type Room, type Session } from '@nt2/contracts';

export const accounts = {
  host: {id:'g0-account-host', name:'Fixture host'},
  guest: {id:'g0-account-guest', name:'Fixture guest'},
  observer: {id:'g0-account-observer', name:'Fixture observer'}
} as const;
export function roomFixtures(): Room[] {
  return ['forest','caverns'].map((name, index) => ({
    id: `g0-room-${name}`, name: index === 0 ? 'Seed 424242 — core fixture' : 'Independent second room',
    hostAccountId: accounts.host.id, revision:1, configRevision:1, capacity:4, seed:424242 + index,
    state:'waiting', runId:null, players:[], roster:[], chat:[]
  }));
}
export class MockProvider {
  private sessions = new Map<string, Session>();
  constructor(private readonly now: () => number = Date.now) {}
  issue(persona: Persona, kind: ConnectionKind) {
    for (const [token, session] of this.sessions) if (session.expiresAt <= this.now()) this.sessions.delete(token);
    if (this.sessions.size >= limits.sessions) throw new BoundaryError('RATE_LIMITED');
    const token = randomBytes(32).toString('base64url');
    const session: Session = {accountId:accounts[persona].id, connectionId:randomUUID(), epoch:randomUUID(), kind, expiresAt:this.now() + 900000};
    this.sessions.set(token, session);
    return {token, session, serverId, contractRevision:revision, mock:true};
  }
  verify(token: string): Session {
    const session = this.sessions.get(token);
    if (!session || session.expiresAt <= this.now()) throw new BoundaryError('UNAUTHENTICATED');
    return session;
  }
  attach(persona: Persona, desktop: Session, expiresAt: number) {
    const issued = this.issue(persona,'game');
    issued.session.attachedDesktopId = desktop.connectionId;
    issued.session.expiresAt = Math.min(expiresAt,desktop.expiresAt);
    return issued;
  }
}
export class MockWorld {
  private rooms: Room[];
  constructor(rooms: Room[] = roomFixtures()) { this.rooms = structuredClone(rooms); }
  private results = new Map<string, Map<string, {payload:string; result:Room}>>();
  list() { return this.rooms.map(({players, chat, roster, ...room}) => ({...room, playerCount:players.length})); }
  snapshot(id: string) {
    const room = this.rooms.find(room => room.id === id);
    if (!room) throw new BoundaryError('NOT_FOUND');
    return structuredClone(room);
  }
  execute(session: Session, command: Command): Room {
    let requests = this.results.get(session.connectionId);
    if (!requests) {
      if (this.results.size >= limits.sessions) throw new BoundaryError('RATE_LIMITED');
      requests = new Map(); this.results.set(session.connectionId, requests);
    }
    const payload = JSON.stringify(command);
    const previous = requests.get(command.requestId);
    if (previous) {
      if (previous.payload !== payload) throw new BoundaryError('CONFLICT');
      return structuredClone(previous.result);
    }
    if (requests.size >= limits.requestsPerSession) throw new BoundaryError('RATE_LIMITED');
    const room = this.snapshot(command.roomId);
    if (command.kind === 'room.join' && this.rooms.some(r => r.id !== room.id && r.players.some(p => p.connectionId === session.connectionId))) throw new BoundaryError('CONFLICT');
    const name = Object.values(accounts).find(a => a.id === session.accountId)?.name ?? 'Fixture player';
    applyMockCommand(room, session, command, name, randomUUID);
    this.rooms = this.rooms.map(r => r.id === room.id ? room : r);
    requests.set(command.requestId, {payload, result:structuredClone(room)});
    return structuredClone(room);
  }
}
export function command(session: Session, room: Room, kind: Command['kind'], body: Record<string, unknown> = {}): Command {
  return {contractRevision:revision, requestId:randomUUID(), roomId:room.id, runId:room.runId, connectionEpoch:session.epoch, expectedRevision:room.revision, kind, body};
}
