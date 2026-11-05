import { randomUUID, randomBytes, createHash, timingSafeEqual } from 'node:crypto';
import { BoundaryError, exactKeys, parseCommand, revision, serverId, type Persona, type Session, type Command, type Room } from '@nt2/contracts';
import { pairingLimits, pairingRevision, type PairingState, type GameBinding } from '@nt2/contracts/pairing';
import { accounts, MockProvider, MockWorld, roomFixtures } from './index.js';
type Attempt = {id:string; deviceId:string; requestId:string; activationCode:string; proof:Buffer; expiresAt:number; state:PairingState; persona:Persona | null};
const digest = (secret:string) => createHash('sha256').update(secret).digest();

// Isolated desktop fixture namespace. Reuses the existing room domain, never a production policy.
export class MockPairing {
  readonly serviceEpoch = randomUUID();
  readonly world = new MockWorld(roomFixtures().map(room => ({...room,id:room.id.replace('g0-room','g0-desktop-room')})));
  private attempts = new Map<string,Attempt>();
  private desktops = new Map<string,{persona:Persona; deviceId:string; present:boolean}>();
  private runs = new Map<string,{epoch:string; expiresAt:number}>();
  private games = new Map<string,GameBinding>();
  constructor(private readonly provider:MockProvider, private readonly now:() => number = Date.now) {}
  checkService(id: unknown, epoch?:unknown) {
    if (id !== serverId) throw new BoundaryError('WRONG_SERVICE');
    if (epoch !== undefined && epoch !== this.serviceEpoch) throw new BoundaryError('STALE_EPOCH');
  }
  initiate(deviceId:string, requestId:string) {
    if (this.attempts.size >= pairingLimits.attempts) throw new BoundaryError('RATE_LIMITED');
    if ([...this.attempts.values()].some(a => a.deviceId === deviceId && a.requestId === requestId)) throw new BoundaryError('CONFLICT');
    const pollingSecret = randomBytes(32).toString('base64url');
    const attempt:Attempt = {id:randomUUID(),deviceId,requestId,activationCode:randomBytes(8).toString('hex'),proof:digest(pollingSecret),expiresAt:this.now()+pairingLimits.lifetimeMs,state:'pending',persona:null};
    this.attempts.set(attempt.id,attempt);
    return {...this.view(attempt),activationCode:attempt.activationCode,pollingSecret};
  }
  private view(attempt:Attempt) {
    if (attempt.expiresAt <= this.now() && !['cancelled','exchanged'].includes(attempt.state)) attempt.state = 'expired';
    return {attemptId:attempt.id, deviceId:attempt.deviceId, expiresAt:attempt.expiresAt,state:attempt.state};
  }
  private requireOpen(attempt:Attempt) {
    this.view(attempt);
    if (attempt.state === 'expired') throw new BoundaryError('PAIRING_EXPIRED');
    if (attempt.state === 'cancelled') throw new BoundaryError('PAIRING_CANCELLED');
    if (attempt.state === 'exchanged') throw new BoundaryError('ALREADY_EXCHANGED');
  }
  private prove(id:string, deviceId:string, secret:string) {
    const attempt = this.attempts.get(id);
    if (!attempt || attempt.deviceId !== deviceId || !timingSafeEqual(attempt.proof,digest(secret))) throw new BoundaryError('UNAUTHENTICATED');
    return attempt;
  }
  approve(code:string, browser:Session) {
    if (browser.kind !== 'browser') throw new BoundaryError('FORBIDDEN');
    const attempt = [...this.attempts.values()].find(a => a.activationCode === code);
    if (!attempt) throw new BoundaryError('NOT_FOUND');
    this.requireOpen(attempt);
    const persona = (Object.keys(accounts) as Persona[]).find(key => accounts[key].id === browser.accountId)!;
    if (attempt.persona && attempt.persona !== persona) throw new BoundaryError('CONFLICT');
    attempt.persona = persona; attempt.state = 'approved'; return this.view(attempt);
  }
  status(id:string,device:string,secret:string) {return this.view(this.prove(id,device,secret));}
  cancel(id:string,device:string,secret:string) {
    const attempt = this.prove(id,device,secret);
    if (attempt.state === 'cancelled') return this.view(attempt);
    this.requireOpen(attempt); attempt.state = 'cancelled'; return this.view(attempt);
  }
  exchange(id:string,device:string,secret:string) {
    const attempt = this.prove(id,device,secret); this.requireOpen(attempt);
    if (attempt.state !== 'approved' || !attempt.persona) throw new BoundaryError('PAIRING_PENDING');
    const result = this.provider.issue(attempt.persona,'desktop');
    this.desktops.set(result.session.connectionId,{persona:attempt.persona,deviceId:device,present:true});
    attempt.state = 'exchanged';
    return {...result,contractRevision:pairingRevision,serviceEpoch:this.serviceEpoch,deviceId:device,desktopPresent:true,attachedGame:null};
  }
  private desktop(session:Session) {
    const desktop = this.desktops.get(session.connectionId);
    if (session.kind !== 'desktop' || !desktop) throw new BoundaryError('FORBIDDEN');
    return desktop;
  }
  presence(session:Session,present:boolean) {this.desktop(session).present=present; return {desktopConnectionId:session.connectionId,desktopPresent:present};}
  snapshot(roomId:string) {
    const room = this.world.snapshot(roomId); const run = room.runId ? this.runs.get(room.runId) : undefined;
    return {...room, runEpoch:run?.epoch ?? null, preparationExpiresAt:run?.expiresAt ?? null,
      players:room.players.map(player => ({...player,participationKind:'desktop',desktopPresent:this.desktops.get(player.connectionId)?.present ?? false,desktopReady:player.ready,
        attachedGame:[...this.games.entries()].filter(([_,game]) => game.desktopConnectionId===player.connectionId && game.runId===room.runId && game.expiresAt>this.now() && !['cancelled','waiting'].includes(room.state)).map(([connectionId,game]) => ({connectionId,...game,seedAccepted:player.seedAccepted,worldLoaded:player.loaded}))[0] ?? null}))};
  }
  list(session:Session) {this.desktop(session); return this.world.list();}
  read(session:Session,roomId:string) {
    if (session.kind === 'desktop') this.desktop(session);
    else if (!this.games.has(session.connectionId)) throw new BoundaryError('FORBIDDEN');
    if (session.kind === 'game' && this.games.get(session.connectionId)?.roomId !== roomId) throw new BoundaryError('FORBIDDEN');
    return this.snapshot(roomId);
  }
  attach(session:Session, binding:Record<string,unknown>) {
    const desktop = this.desktop(session);
    const room = this.world.snapshot(binding.roomId as string); const run = room.runId ? this.runs.get(room.runId) : undefined;
    this.match(room,binding,run?.epoch);
    if (binding.expectedRevision!==room.revision) throw new BoundaryError('STALE_REVISION');
    if (binding.connectionEpoch!==session.epoch) throw new BoundaryError('STALE_EPOCH');
    if (room.state !== 'preparing' || !run || run.expiresAt <= this.now()) throw new BoundaryError('CONFLICT');
    if (!room.roster.includes(session.connectionId) || !desktop.present) throw new BoundaryError('FORBIDDEN');
    if ([...this.games.values()].some(game => game.desktopConnectionId===session.connectionId && game.runId===room.runId)) throw new BoundaryError('CONFLICT');
    const issued = this.provider.attach(desktop.persona,session,run.expiresAt);
    const game:GameBinding = {serviceEpoch:this.serviceEpoch,roomId:room.id,runId:room.runId!,runEpoch:run.epoch,configRevision:room.configRevision,desktopConnectionId:session.connectionId,attachmentId:randomUUID(),gameInstanceId:binding.gameInstanceId as string,expiresAt:issued.session.expiresAt};
    this.games.set(issued.session.connectionId,game);
    return {...issued,contractRevision:pairingRevision,serviceEpoch:this.serviceEpoch,binding:game};
  }
  private match(room:Room, value:Record<string,unknown>, runEpoch:string | undefined) {
    this.checkService(value.serviceId,value.serviceEpoch);
    if (value.configRevision !== room.configRevision) throw new BoundaryError('STALE_REVISION');
    if (value.runId !== room.runId || value.runEpoch !== (runEpoch ?? null)) throw new BoundaryError('WRONG_RUN');
  }
  execute(session:Session,value:Record<string,unknown>) {
    const room = this.world.snapshot(value.roomId as string);
    this.match(room,value,room.runId ? this.runs.get(room.runId)?.epoch : undefined);
    if (value.connectionEpoch !== session.epoch) throw new BoundaryError('STALE_EPOCH');
    let kind = value.kind as string; let body = value.body;
    if (session.kind === 'desktop') {
      const desktop = this.desktop(session);
      if (!desktop.present) throw new BoundaryError('FORBIDDEN');
      const mapping:Record<string,Command['kind']> = {'desktop.join':'room.join','desktop.leave':'room.leave','desktop.ready':'game.ready','chat.send':'chat.send','run.start':'run.start','run.cancel':'run.cancel'};
      if (!Object.hasOwn(mapping,kind)) throw new BoundaryError('FORBIDDEN');
      kind = mapping[kind]!;
    } else {
      const game = this.games.get(session.connectionId);
      if (!game || game.expiresAt<=this.now()) throw new BoundaryError('UNAUTHENTICATED');
      if (!['game.seedAccepted','game.loaded'].includes(kind)) throw new BoundaryError('FORBIDDEN');
      exactKeys(body,kind==='game.seedAccepted' ? ['attachmentId','gameInstanceId','seed'] : ['attachmentId','gameInstanceId']);
      if (body.attachmentId !== game.attachmentId || body.gameInstanceId !== game.gameInstanceId) throw new BoundaryError('FORBIDDEN');
      if (game.roomId!==room.id || game.runId!==room.runId || game.runEpoch!==value.runEpoch || game.configRevision!==value.configRevision) throw new BoundaryError('WRONG_RUN');
      body = kind==='game.seedAccepted' ? {seed:body.seed} : {};
    }
    const command = parseCommand({contractRevision:revision,requestId:value.requestId,roomId:room.id,runId:value.runId,connectionEpoch:session.epoch,expectedRevision:value.expectedRevision,kind,body});
    const result = this.world.execute(session,command);
    if (command.kind==='run.start' && result.runId && !this.runs.has(result.runId)) this.runs.set(result.runId,{epoch:randomUUID(),expiresAt:this.now()+pairingLimits.preparationMs});
    return this.snapshot(room.id);
  }
}
