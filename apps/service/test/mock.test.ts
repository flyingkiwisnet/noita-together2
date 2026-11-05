import test from 'node:test';
import assert from 'node:assert/strict';
import { once } from 'node:events';
import WebSocket from 'ws';
import { createMockApp } from '../src/app.js';
import { MockProvider, MockWorld, command } from '@nt2/test-kit';
import { BoundaryError, limits, type Room, type Session } from '@nt2/contracts';

function errorCode(code: string) { return (error: unknown) => error instanceof BoundaryError && error.code === code; }
test('mock provider separates browser/game identity, server scope and expiry', () => {
  let now = 0; const provider = new MockProvider(() => now);
  const browser = provider.issue('host','browser'); const game = provider.issue('host','game');
  assert.equal(browser.session.accountId, game.session.accountId);
  assert.notEqual(browser.session.connectionId,game.session.connectionId);
  assert.throws(() => new MockProvider().verify(browser.token),errorCode('UNAUTHENTICATED'));
  now = 900000; assert.throws(() => provider.verify(game.token),errorCode('UNAUTHENTICATED'));
});
test('browser and two games simulate one frozen seed/run; browser cannot acknowledge loading', () => {
  const world = new MockWorld(); const provider = new MockProvider();
  const browser = provider.issue('host','browser').session;
  const host = provider.issue('host','game').session; const guest = provider.issue('guest','game').session;
  let room = world.snapshot('g0-room-forest');
  const perform = (session:Session, kind:Parameters<typeof command>[2], body:Record<string,unknown> = {}) => {
    room = world.execute(session,command(session,room,kind,body));
  };
  assert.throws(() => perform(browser,'room.join'),errorCode('FORBIDDEN'));
  perform(host,'room.join'); perform(guest,'room.join');
  assert.throws(() => perform(browser,'run.start'),errorCode('NOT_READY'));
  perform(host,'game.ready',{ready:true}); perform(guest,'game.ready',{ready:true});
  perform(browser,'run.start'); const runId = room.runId; assert.ok(runId); assert.equal(room.state,'preparing');
  assert.throws(() => perform(browser,'game.seedAccepted',{seed:room.seed}),errorCode('FORBIDDEN'));
  assert.throws(() => perform(host,'game.seedAccepted',{seed:room.seed+1}),errorCode('CONFLICT'));
  perform(host,'game.seedAccepted',{seed:room.seed}); perform(guest,'game.seedAccepted',{seed:room.seed});
  assert.equal(room.state,'loading'); perform(host,'game.loaded'); assert.equal(room.state,'loading');
  perform(guest,'game.loaded'); assert.equal(room.state,'running'); assert.equal(room.runId,runId);
  assert.equal(world.snapshot('g0-room-caverns').state,'waiting');
});
test('stale revisions/epochs/runs, non-host starts and room switching preserve state', () => {
  const world = new MockWorld(); const provider = new MockProvider();
  const guest = provider.issue('guest','game').session;
  let room = world.snapshot('g0-room-forest'); const join = command(guest,room,'room.join');
  room = world.execute(guest,join); const before = world.snapshot(room.id);
  for (const [value, code] of [
    [command(guest,room,'run.start'),'FORBIDDEN'],
    [{...command(guest,room,'game.ready',{ready:true}),connectionEpoch:'old'},'STALE_EPOCH'],
    [{...command(guest,room,'game.ready',{ready:true}),runId:'old'},'WRONG_RUN'],
    [command(guest,{...room,revision:1},'game.ready',{ready:true}),'STALE_REVISION'],
    [command(guest,world.snapshot('g0-room-caverns'),'room.join'),'CONFLICT']
  ] as const) assert.throws(() => world.execute(guest,value),errorCode(code));
  assert.deepEqual(world.snapshot(room.id),before);
});
test('identical retries return original result; conflicting ID reuse has no mutation', () => {
  const world = new MockWorld(); const session = new MockProvider().issue('host','game').session;
  const join = command(session,world.snapshot('g0-room-forest'),'room.join');
  const first = world.execute(session,join); assert.deepEqual(world.execute(session,join),first);
  assert.throws(() => world.execute(session,{...join,body:{ready:true},kind:'game.ready'}),errorCode('CONFLICT'));
  assert.equal(world.snapshot(first.id).players.length,1);
});
test('competing starts produce one preparation; denied capacity admission preserves both rooms', () => {
  const world = new MockWorld(); const provider = new MockProvider();
  const browser = provider.issue('host','browser').session;
  let room = world.snapshot('g0-room-forest');
  for (let i=0;i<4;i++) {
    const session = provider.issue('guest','game').session;
    room = world.execute(session,command(session,room,'room.join'));
    room = world.execute(session,command(session,room,'game.ready',{ready:true}));
  }
  const denied = provider.issue('observer','game').session;
  assert.throws(() => world.execute(denied,command(denied,room,'room.join')),errorCode('ROOM_FULL'));
  const startA = command(browser,room,'run.start'); const startB = command(browser,room,'run.start');
  const started = world.execute(browser,startA);
  assert.throws(() => world.execute(browser,startB),errorCode('STALE_REVISION'));
  assert.equal(world.snapshot(room.id).runId,started.runId);
  assert.equal(world.snapshot(room.id).players.length,4);
  assert.equal(world.snapshot('g0-room-caverns').players.length,0);
});
test('bank effects remain unavailable and chat is attributed/bounded', () => {
  const world = new MockWorld(); const session = new MockProvider().issue('host','browser').session;
  let room = world.snapshot('g0-room-forest');
  assert.throws(() => world.execute(session,command(session,room,'transfer.status',{operationId:'op-1'})),errorCode('UNSUPPORTED_G0'));
  for (let i=0; i<60; i++) room = world.execute(session,command(session,room,'chat.send',{text:`message ${i}`}));
  assert.equal(room.chat.length,limits.chatHistory); assert.equal(room.chat[0]?.text,'message 10');
  assert.ok(room.chat.every(c => c.accountId === session.accountId));
});
test('HTTP boundaries reject unauthenticated, cross-origin, token URL and oversized traffic', async t => {
  const {app} = await createMockApp(); t.after(() => app.close());
  assert.equal((await app.inject('/g0/rooms')).statusCode,401);
  assert.equal((await app.inject({url:'/health',headers:{origin:'https://untrusted.example'}})).statusCode,403);
  assert.equal((await app.inject('/health?token=secret')).statusCode,400);
  assert.equal((await app.inject({method:'POST',url:'/g0/sessions',payload:'x'.repeat(70000),headers:{'content-type':'application/json'}})).statusCode,400);
  const response = await app.inject({method:'POST',url:'/g0/sessions',payload:{persona:'host',kind:'browser'}});
  assert.equal(response.statusCode,200); const {token} = response.json();
  const list = await app.inject({url:'/g0/rooms',headers:{authorization:`Bearer ${token}`}});
  assert.equal(list.json().rooms.length,2); assert.equal(list.json().mock,true);
});
test('real WebSocket authenticates outside URL and delivers HTTP command snapshots', async t => {
  const {app,provider,world} = await createMockApp(); const address = await app.listen({host:'127.0.0.1',port:0});
  const socket = new WebSocket(address.replace('http:','ws:') + '/g0/events');
  t.after(async () => {socket.terminate(); await app.close();});
  await once(socket,'open'); const {token,session} = provider.issue('host','browser');
  const initial = once(socket,'message'); socket.send(JSON.stringify({kind:'session.authenticate',token,roomId:'g0-room-forest'}));
  const [data] = await initial; const room:Room = JSON.parse(data.toString()).room;
  assert.equal(room.id,'g0-room-forest'); const event = once(socket,'message');
  const result = await app.inject({method:'POST',url:'/g0/commands',headers:{authorization:`Bearer ${token}`},payload:command(session,room,'chat.send',{text:'<script>fixture</script>'})});
  assert.equal(result.statusCode,200); const [next] = await event;
  assert.equal(JSON.parse(next.toString()).room.chat[0].accountId,session.accountId);
  assert.equal(world.snapshot(room.id).chat[0]?.text,'<script>fixture</script>');
});
test('WebSocket unauthenticated deadline and malformed frames close safely', async t => {
  const {app} = await createMockApp({authTimeoutMs:40}); const address = await app.listen({host:'127.0.0.1',port:0});
  t.after(() => app.close());
  for (const frame of [undefined, 'null']) {
    const socket = new WebSocket(address.replace('http:','ws:') + '/g0/events');
    socket.on('error', () => {}); await once(socket,'open'); const closed = once(socket,'close');
    if (frame) socket.send(frame); const [code] = await closed; assert.equal(code,1008);
  }
});
test('session flooding and per-session command rate stay bounded', async t => {
  const {app,provider} = await createMockApp({now:() => 0}); t.after(() => app.close());
  const {token} = provider.issue('host','browser');
  for (let i=0;i<limits.messagesPerSecond;i++) assert.equal((await app.inject({url:'/g0/rooms',headers:{authorization:`Bearer ${token}`}})).statusCode,200);
  assert.equal((await app.inject({url:'/g0/rooms',headers:{authorization:`Bearer ${token}`}})).statusCode,429);
  for (let i=1;i<limits.sessions;i++) provider.issue('guest','game');
  assert.throws(() => provider.issue('guest','game'),errorCode('RATE_LIMITED'));
});
