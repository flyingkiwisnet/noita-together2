import Fastify from 'fastify';
import websocket from '@fastify/websocket';
import type { WebSocket } from 'ws';
import { BoundaryError, exactKeys, limits, parseCommand, revision, serverId, type Persona, type ConnectionKind, type Session } from '@nt2/contracts';
import { MockProvider, MockWorld } from '@nt2/test-kit';
import { MockPairing } from '@nt2/test-kit/pairing';
import { pairingRevision, parsePairing } from '@nt2/contracts/pairing';

export async function createMockApp(options: {now?: () => number; authTimeoutMs?: number} = {}) {
  const provider = new MockProvider(options.now);
  const world = new MockWorld();
  const pairing = new MockPairing(provider,options.now);
  const app = Fastify({logger:false, bodyLimit:limits.frameBytes});
  await app.register(websocket, {options:{maxPayload:limits.frameBytes, perMessageDeflate:false}});
  const connections = new Set<WebSocket>();
  const subscriptions = new Map<WebSocket, {session:Session; roomId:string}>();
  const budgets = new Map<string, {second:number; count:number}>();
  const now = options.now ?? Date.now;
  const label = {mock:true, contractRevision:revision, serverId};
  function budget(session: Session) {
    const second = Math.floor(now()/1000);
    const previous = budgets.get(session.connectionId);
    const count = previous?.second === second ? previous.count + 1 : 1;
    if (count > limits.messagesPerSecond) throw new BoundaryError('RATE_LIMITED');
    if (budgets.size >= limits.sessions && !previous) throw new BoundaryError('RATE_LIMITED');
    budgets.set(session.connectionId, {second,count});
  }
  function authenticate(header: string | undefined) {
    if (!header?.startsWith('Bearer ')) throw new BoundaryError('UNAUTHENTICATED');
    const session = provider.verify(header.slice(7)); budget(session); return session;
  }
  function legacy(session:Session) {
    if (session.kind==='desktop' || session.attachedDesktopId) throw new BoundaryError('FORBIDDEN');
    return session;
  }
  function send(socket: WebSocket, value: unknown) {
    if (socket.readyState !== 1) return;
    if (socket.bufferedAmount > limits.outboundBytes) { socket.close(1008, 'slow recipient'); return; }
    socket.send(JSON.stringify({...label, ...(value as object)}));
  }
  function publish(roomId: string) {
    for (const [socket, subscription] of subscriptions) {
      if (subscription.session.expiresAt <= now()) { socket.close(1008,'session expired'); continue; }
      if (subscription.roomId === roomId) send(socket,{kind:'room.snapshot', room:world.snapshot(roomId)});
    }
  }
  app.addHook('onRequest', async (request, reply) => {
    // Native probes omit Origin. Browser access is intentionally not enabled at G0.
    if (request.headers.origin) return reply.code(403).send({...label,error:{code:'FORBIDDEN'}});
    if (request.url.includes('?')) return reply.code(400).send({...label,error:{code:'INVALID_MESSAGE'}});
  });
  const pairingLabel = {mock:true,contractRevision:pairingRevision,serviceId:serverId,serviceEpoch:pairing.serviceEpoch};
  app.setErrorHandler((error, request, reply) => {
    const code = error instanceof BoundaryError ? error.code : 'INVALID_MESSAGE';
    const status = code === 'UNAUTHENTICATED' ? 401 : code === 'FORBIDDEN' ? 403 : code === 'NOT_FOUND' ? 404 : code === 'RATE_LIMITED' ? 429 : 400;
    reply.code(status).send({...(/\/g0\/(pairings|desktop)/.test(request.url) ? pairingLabel : label),error:{code}});
  });
  app.get('/health', async () => ({...label, status:'ok', persistence:'in-memory mock', productionReady:false}));
  app.post('/g0/sessions', async request => {
    exactKeys(request.body,['persona','kind']);
    if (!['host','guest','observer'].includes(String(request.body.persona)) || !['browser','game'].includes(String(request.body.kind))) throw new BoundaryError('INVALID_MESSAGE');
    return provider.issue(request.body.persona as Persona, request.body.kind as ConnectionKind);
  });
  app.get('/g0/rooms', async request => { legacy(authenticate(request.headers.authorization)); return {...label, rooms:world.list()}; });
  app.get<{Params:{id:string}}>('/g0/rooms/:id', async request => {
    legacy(authenticate(request.headers.authorization)); return {...label, room:world.snapshot(request.params.id)};
  });
  app.post('/g0/commands', async request => {
    const session = legacy(authenticate(request.headers.authorization));
    const command = parseCommand(request.body);
    const room = world.execute(session,command); publish(room.id);
    return {...label, requestId:command.requestId, room};
  });
  // Additive G0 pairing/desktop fixture. Legacy g0-json-1 messages remain unchanged.
  app.post('/g0/pairings/initiate', async request => {
    const body = parsePairing(request.body,['deviceId','requestId']); pairing.checkService(body.serviceId);
    return {...pairingLabel,...pairing.initiate(body.deviceId as string,body.requestId as string)};
  });
  app.post('/g0/pairings/approve', async request => {
    const browser = authenticate(request.headers.authorization);
    const body = parsePairing(request.body,['activationCode']); pairing.checkService(body.serviceId);
    return {...pairingLabel,...pairing.approve(body.activationCode as string,browser)};
  });
  for (const action of ['status','exchange','cancel'] as const) {
    app.post(`/g0/pairings/${action}`, async request => {
      const body = parsePairing(request.body,['attemptId','deviceId','pollingSecret']); pairing.checkService(body.serviceId);
      return {...pairing[action](body.attemptId as string,body.deviceId as string,body.pollingSecret as string),...pairingLabel};
    });
  }
  app.get('/g0/desktop/rooms', async request => ({...pairingLabel,rooms:pairing.list(authenticate(request.headers.authorization))}));
  app.get<{Params:{id:string}}>('/g0/desktop/rooms/:id', async request => ({...pairingLabel,room:pairing.read(authenticate(request.headers.authorization),request.params.id)}));
  app.post('/g0/desktop/presence', async request => {
    const session = authenticate(request.headers.authorization);
    const body = parsePairing(request.body,['serviceEpoch','present']); pairing.checkService(body.serviceId,body.serviceEpoch);
    return {...pairingLabel,...pairing.presence(session,body.present as boolean)};
  });
  app.post('/g0/desktop/attach', async request => {
    const session = authenticate(request.headers.authorization);
    const body = parsePairing(request.body,['serviceEpoch','roomId','runId','runEpoch','configRevision','connectionEpoch','expectedRevision','gameInstanceId']);
    return {...pairing.attach(session,body),...pairingLabel};
  });
  app.post('/g0/desktop/commands', async request => {
    const session = authenticate(request.headers.authorization);
    const body = parsePairing(request.body,['serviceEpoch','requestId','roomId','runId','runEpoch','configRevision','connectionEpoch','expectedRevision','kind','body']);
    return {...pairingLabel,requestId:body.requestId,room:pairing.execute(session,body)};
  });
  app.get('/g0/events', {websocket:true}, socket => {
    if (connections.size >= limits.sessions) { socket.close(1008,'connection limit'); return; }
    connections.add(socket);
    let token: string | undefined;
    const timer = setTimeout(() => socket.close(1008,'authentication deadline'), options.authTimeoutMs ?? 3000);
    timer.unref();
    socket.on('close', () => {clearTimeout(timer); connections.delete(socket); subscriptions.delete(socket);});
    socket.on('error', () => {socket.close(1008,'connection error');});
    socket.on('message', (data, binary) => {
      try {
        if (binary) throw new BoundaryError('INVALID_MESSAGE');
        const value: unknown = JSON.parse(data.toString());
        if (!token) {
          exactKeys(value,['kind','token','roomId']);
          if (value.kind !== 'session.authenticate' || typeof value.token !== 'string' || typeof value.roomId !== 'string') throw new BoundaryError('INVALID_MESSAGE');
          const session = legacy(provider.verify(value.token)); budget(session);
          const room = world.snapshot(value.roomId);
          token = value.token; clearTimeout(timer);
          subscriptions.set(socket,{session,roomId:room.id});
          send(socket,{kind:'room.snapshot', room});
        } else {
          const session = legacy(provider.verify(token)); budget(session);
          const command = parseCommand(value);
          const subscription = subscriptions.get(socket)!;
          if (command.roomId !== subscription.roomId) throw new BoundaryError('FORBIDDEN');
          const room = world.execute(session,command);
          send(socket,{kind:'command.result',requestId:command.requestId,room}); publish(room.id);
        }
      } catch (error) {
        send(socket,{kind:'command.error',error:{code:error instanceof BoundaryError ? error.code : 'INVALID_MESSAGE'}});
        socket.close(1008,'rejected message');
      }
    });
  });
  app.addHook('preClose', async () => { for (const socket of connections) socket.terminate(); });
  return {app, provider, world, pairing};
}
