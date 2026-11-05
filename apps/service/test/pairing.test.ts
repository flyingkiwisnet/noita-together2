import test from 'node:test';
import assert from 'node:assert/strict';
import { randomUUID } from 'node:crypto';
import { createMockApp } from '../src/app.js';
import { pairingRevision } from '@nt2/contracts/pairing';
import { serverId, type Session } from '@nt2/contracts';
const base = {contractRevision:pairingRevision,serviceId:serverId};
// All provisioning and lifecycle actions below cross the actual HTTP receiving boundary.
async function tauriScenario(now:()=>number=Date.now) {
  const fixture=await createMockApp({now});
  const post=(path:string,payload:unknown,token?:string)=>fixture.app.inject({method:'POST',url:`/g0/${path}`,payload:payload as Record<string,unknown>,headers:token?{authorization:`Bearer ${token}`}:{}});
  async function pair(persona:'host'|'guest',deviceId:string) {
    const browser=(await post('sessions',{persona,kind:'browser'})).json();
    const attempt=(await post('pairings/initiate',{...base,deviceId,requestId:randomUUID()})).json();
    assert.equal((await post('pairings/approve',{...base,activationCode:attempt.activationCode},browser.token)).statusCode,200);
    const response=await post('pairings/exchange',{...base,attemptId:attempt.attemptId,deviceId,pollingSecret:attempt.pollingSecret});
    assert.equal(response.statusCode,200,response.body); return response.json();
  }
  let room=fixture.pairing.snapshot('g0-desktop-room-forest');
  function envelope(actor:{session:Session},kind:string,body:unknown={}) {
    return {...base,serviceEpoch:fixture.pairing.serviceEpoch,requestId:randomUUID(),roomId:room.id,runId:room.runId,runEpoch:room.runEpoch,configRevision:room.configRevision,connectionEpoch:actor.session.epoch,expectedRevision:room.revision,kind,body};
  }
  async function command(actor:{session:Session;token:string},kind:string,body:unknown={}) {
    const response=await post('desktop/commands',envelope(actor,kind,body),actor.token);
    assert.equal(response.statusCode,200,response.body); room=response.json().room; return room;
  }
  async function attach(actor:{session:Session;token:string},gameInstanceId:string) {
    const {requestId,kind,body,...scope}=envelope(actor,'desktop.join');
    const response=await post('desktop/attach',{...scope,gameInstanceId},actor.token);
    assert.equal(response.statusCode,200,response.body); return response.json();
  }
  return {...fixture,post,pair,envelope,command,attach,room:()=>room};
}

test('two paired desktops and attached games require every frozen seed and loading report',async t=>{
  const f=await tauriScenario(); t.after(()=>f.app.close());
  const host=await f.pair('host','two-host'),guest=await f.pair('guest','two-guest');
  await f.command(host,'desktop.join'); await f.command(guest,'desktop.join');
  await f.command(host,'desktop.ready',{ready:true}); await f.command(guest,'desktop.ready',{ready:true});
  await f.command(host,'run.start');
  assert.equal(f.room().state,'preparing'); assert.equal(f.room().roster.length,2);
  const hostGame=await f.attach(host,'host-world'),guestGame=await f.attach(guest,'guest-world');
  const report=(game:typeof hostGame)=>({attachmentId:game.binding.attachmentId,gameInstanceId:game.binding.gameInstanceId});
  const before=f.pairing.snapshot(f.room().id);
  const spoof=await f.post('desktop/commands',f.envelope(guestGame,'game.seedAccepted',{...report(hostGame),seed:f.room().seed}),guestGame.token);
  assert.equal(spoof.statusCode,403); assert.deepEqual(f.pairing.snapshot(f.room().id),before);
  await f.command(hostGame,'game.seedAccepted',{...report(hostGame),seed:f.room().seed});
  assert.equal(f.room().state,'preparing');
  await f.command(guestGame,'game.seedAccepted',{...report(guestGame),seed:f.room().seed});
  assert.equal(f.room().state,'loading');
  await f.command(hostGame,'game.loaded',report(hostGame)); assert.equal(f.room().state,'loading');
  await f.command(guestGame,'game.loaded',report(guestGame)); assert.equal(f.room().state,'running');
  assert.ok(f.room().players.every(p=>p.attachedGame?.worldLoaded));
  assert.equal(f.pairing.snapshot('g0-desktop-room-caverns').state,'waiting');
  assert.deepEqual(f.pairing.snapshot('g0-desktop-room-caverns').players,[]);
});

test('attached preparation cancellation invalidates facts and exact expiry blocks attachment',async t=>{
  let now=1000; const f=await tauriScenario(()=>now); t.after(()=>f.app.close());
  const host=await f.pair('host','cancel-attached');
  await f.command(host,'desktop.join'); await f.command(host,'desktop.ready',{ready:true}); await f.command(host,'run.start');
  const game=await f.attach(host,'cancelled-world');
  const old=f.envelope(game,'game.seedAccepted',{attachmentId:game.binding.attachmentId,gameInstanceId:game.binding.gameInstanceId,seed:f.room().seed});
  await f.command(host,'run.cancel');
  assert.equal(f.room().state,'cancelled'); assert.equal(f.room().players[0]?.attachedGame,null);
  const cancelled=f.pairing.snapshot(f.room().id);
  assert.notEqual((await f.post('desktop/commands',old,game.token)).statusCode,200);
  assert.deepEqual(f.pairing.snapshot(f.room().id),cancelled);
  // A separate preparation tests the exact deadline, without waiting or bypass endpoints.
  const a=f.pairing.initiate('expiry-device','expiry-request');
  f.pairing.approve(a.activationCode,f.provider.issue('host','browser').session);
  const desktop=f.pairing.exchange(a.attemptId,'expiry-device',a.pollingSecret);
  let room=f.pairing.snapshot('g0-desktop-room-caverns');
  for (const [kind,body] of [['desktop.join',{}],['desktop.ready',{ready:true}],['run.start',{}]] as const) {
    const response=await f.post('desktop/commands',{...base,serviceEpoch:f.pairing.serviceEpoch,requestId:randomUUID(),roomId:room.id,runId:room.runId,runEpoch:room.runEpoch,configRevision:room.configRevision,connectionEpoch:desktop.session.epoch,expectedRevision:room.revision,kind,body},desktop.token);
    assert.equal(response.statusCode,200,response.body); room=response.json().room;
  }
  now=room.preparationExpiresAt!;
  const response=await f.post('desktop/attach',{...base,serviceEpoch:f.pairing.serviceEpoch,roomId:room.id,runId:room.runId,runEpoch:room.runEpoch,configRevision:room.configRevision,connectionEpoch:desktop.session.epoch,expectedRevision:room.revision,gameInstanceId:'expired-world'},desktop.token);
  assert.equal(response.json().error.code,'CONFLICT');
  assert.deepEqual(f.pairing.snapshot(room.id),room);
});
test('HTTP pairing initiation, browser approval, proof exchange and duplicates are scoped', async t => {
  const {app,provider} = await createMockApp(); t.after(() => app.close());
  const post = async (path:string,payload:Record<string,unknown>,token?:string) => app.inject({method:'POST',url:`/g0/${path}`,payload,headers:token ? {authorization:`Bearer ${token}`} : {}});
  const initiated = await post('pairings/initiate',{...base,deviceId:'device-a',requestId:'pair-1'});
  assert.equal(initiated.statusCode,200); const attempt = initiated.json();
  const proof = {...base,attemptId:attempt.attemptId,deviceId:'device-a',pollingSecret:attempt.pollingSecret};
  assert.equal((await post('pairings/exchange',proof)).json().error.code,'PAIRING_PENDING');
  for (const changed of [{deviceId:'device-b'},{pollingSecret:'wrong'},{attemptId:randomUUID()}]) {
    assert.equal((await post('pairings/exchange',{...proof,...changed})).statusCode,401);
  }
  assert.equal((await post('pairings/status',{...proof,serviceId:'foreign-server'})).json().error.code,'WRONG_SERVICE');
  const browser = provider.issue('host','browser'); const game = provider.issue('host','game');
  const approval = {...base,activationCode:attempt.activationCode};
  assert.equal((await post('pairings/approve',approval,game.token)).statusCode,403);
  assert.equal((await post('pairings/approve',approval,browser.token)).json().state,'approved');
  assert.equal((await post('pairings/approve',approval,provider.issue('guest','browser').token)).json().error.code,'CONFLICT');
  const exchanged = await post('pairings/exchange',proof); assert.equal(exchanged.statusCode,200);
  assert.equal(exchanged.json().session.kind,'desktop'); assert.equal(exchanged.json().desktopPresent,true);
  assert.equal(exchanged.json().attachedGame,null); assert.notEqual(exchanged.json().session.connectionId,browser.session.connectionId);
  assert.equal((await post('pairings/exchange',proof)).json().error.code,'ALREADY_EXCHANGED');
  assert.equal((await post('pairings/cancel',proof)).json().error.code,'ALREADY_EXCHANGED');
  assert.equal((await post('pairings/initiate',{...base,deviceId:'device-a',requestId:'pair-1'})).json().error.code,'CONFLICT');
  assert.equal((await app.inject({url:'/g0/rooms',headers:{authorization:`Bearer ${exchanged.json().token}`}})).statusCode,403);
});
test('pairing cancellation and expiry prevent issuance, with deterministic clock', async t => {
  let now = 1000; const {app} = await createMockApp({now:() => now}); t.after(() => app.close());
  const post = async (action:string,payload:Record<string,unknown>) => app.inject({method:'POST',url:`/g0/pairings/${action}`,payload});
  for (const expired of [false,true]) {
    const a = (await post('initiate',{...base,deviceId:'device',requestId:randomUUID()})).json();
    const proof = {...base,attemptId:a.attemptId,deviceId:'device',pollingSecret:a.pollingSecret};
    if (expired) now = a.expiresAt;
    else {
      assert.equal((await post('cancel',proof)).json().state,'cancelled');
      assert.equal((await post('cancel',proof)).json().state,'cancelled');
    }
    assert.equal((await post('status',proof)).json().state,expired ? 'expired' : 'cancelled');
    assert.equal((await post('exchange',proof)).json().error.code,expired ? 'PAIRING_EXPIRED' : 'PAIRING_CANCELLED');
  }
});
test('service restart rejects old pairing proof and desktop token and changes service epoch', async t => {
  const old = await createMockApp(); const browser = old.provider.issue('host','browser');
  const a = old.pairing.initiate('device','request'); old.pairing.approve(a.activationCode,browser.session);
  const desktop = old.pairing.exchange(a.attemptId,'device',a.pollingSecret); await old.app.close();
  const fresh = await createMockApp(); t.after(() => fresh.app.close());
  assert.notEqual(old.pairing.serviceEpoch,fresh.pairing.serviceEpoch);
  assert.equal((await fresh.app.inject({method:'POST',url:'/g0/pairings/exchange',payload:{...base,attemptId:a.attemptId,deviceId:'device',pollingSecret:a.pollingSecret}})).statusCode,401);
  assert.equal((await fresh.app.inject({url:'/g0/desktop/rooms',headers:{authorization:`Bearer ${desktop.token}`}})).statusCode,401);
});
test('paired desktop presence/readiness differs from bound attached-game seed/loading facts', async t => {
  let now = 0; const {app,provider,pairing} = await createMockApp({now:() => now}); t.after(() => app.close());
  const browser = provider.issue('host','browser'); const a = pairing.initiate('desktop-a','pair-a');
  pairing.approve(a.activationCode,browser.session); const desktop = pairing.exchange(a.attemptId,'desktop-a',a.pollingSecret);
  const post = async (path:string,payload:Record<string,unknown>,token=desktop.token) => app.inject({method:'POST',url:`/g0/desktop/${path}`,payload,headers:{authorization:`Bearer ${token}`}});
  let room = pairing.snapshot('g0-desktop-room-forest');
  const envelope = (session:Session,kind:string,body:unknown={}) => ({...base,serviceEpoch:pairing.serviceEpoch,requestId:randomUUID(),roomId:room.id,runId:room.runId,runEpoch:room.runEpoch,configRevision:room.configRevision,connectionEpoch:session.epoch,expectedRevision:room.revision,kind,body});
  const perform = async (kind:string,body:unknown={}) => {
    const response = await post('commands',envelope(desktop.session,kind,body)); assert.equal(response.statusCode,200, response.body); room=response.json().room;
  };
  await perform('desktop.join'); await perform('desktop.ready',{ready:true});
  assert.equal(room.players[0]!.attachedGame,null); assert.equal(room.players[0]!.desktopReady,true);
  await perform('run.start'); assert.equal(room.state,'preparing');
  assert.equal((await post('commands',envelope(desktop.session,'game.loaded',{attachmentId:'fake',gameInstanceId:'pid-1'}))).statusCode,403);
  const attach = {...base,serviceEpoch:pairing.serviceEpoch,roomId:room.id,runId:room.runId,runEpoch:room.runEpoch,configRevision:room.configRevision,connectionEpoch:desktop.session.epoch,expectedRevision:room.revision,gameInstanceId:'pid-123-context-abc'};
  for (const changed of [{runEpoch:'old'},{configRevision:99},{serviceEpoch:'old'},{connectionEpoch:'old'}]) assert.equal((await post('attach',{...attach,...changed})).statusCode,400);
  const response = await post('attach',attach); assert.equal(response.statusCode,200,response.body); const game = response.json();
  assert.equal(game.session.kind,'game'); assert.notEqual(game.session.connectionId,desktop.session.connectionId);
  assert.equal((await post('attach',attach)).json().error.code,'CONFLICT');
  assert.equal((await post('commands',envelope(game.session,'game.loaded',{attachmentId:game.binding.attachmentId,gameInstanceId:game.binding.gameInstanceId}),game.token)).statusCode,400);
  assert.equal((await post('commands',envelope(game.session,'game.seedAccepted',{attachmentId:game.binding.attachmentId,gameInstanceId:'another-context',seed:room.seed}),game.token)).statusCode,403);
  for (const [kind,body] of [['game.seedAccepted',{seed:room.seed}],['game.loaded',{}]] as const) {
    const result = await post('commands',envelope(game.session,kind,{...body,attachmentId:game.binding.attachmentId,gameInstanceId:game.binding.gameInstanceId}),game.token);
    assert.equal(result.statusCode,200,result.body); room=result.json().room;
  }
  assert.equal(room.state,'running'); assert.equal(room.players[0]!.attachedGame!.worldLoaded,true);
  assert.equal((await post('presence',{...base,serviceEpoch:pairing.serviceEpoch,present:false})).json().desktopPresent,false);
  assert.equal(pairing.snapshot(room.id).players[0]!.attachedGame!.worldLoaded,true);
  now = game.binding.expiresAt;
  assert.equal((await post('commands',envelope(game.session,'game.loaded',{attachmentId:game.binding.attachmentId,gameInstanceId:game.binding.gameInstanceId}),game.token)).statusCode,401);
});
test('cancelled preparation and expired preparation cannot attach/replay a game', async t => {
  let now=0; const {app,provider,pairing} = await createMockApp({now:() => now}); t.after(() => app.close());
  for (const cancel of [true,false]) {
    const a=pairing.initiate('device',randomUUID()); pairing.approve(a.activationCode,provider.issue('host','browser').session);
    const d=pairing.exchange(a.attemptId,'device',a.pollingSecret); let room=pairing.snapshot(cancel?'g0-desktop-room-forest':'g0-desktop-room-caverns');
    const envelope=(kind:string,body:unknown={})=>({...base,serviceEpoch:pairing.serviceEpoch,requestId:randomUUID(),roomId:room.id,runId:room.runId,runEpoch:room.runEpoch,configRevision:room.configRevision,connectionEpoch:d.session.epoch,expectedRevision:room.revision,kind,body});
    for (const [kind,body] of [['desktop.join',{}],['desktop.ready',{ready:true}],['run.start',{}]] as const) room=pairing.execute(d.session,envelope(kind,body));
    if (cancel) room=pairing.execute(d.session,envelope('run.cancel'));
    else now=room.preparationExpiresAt!;
    const result=await app.inject({method:'POST',url:'/g0/desktop/attach',headers:{authorization:`Bearer ${d.token}`},payload:{...base,serviceEpoch:pairing.serviceEpoch,roomId:room.id,runId:room.runId,runEpoch:room.runEpoch,configRevision:room.configRevision,connectionEpoch:d.session.epoch,expectedRevision:room.revision,gameInstanceId:'game-context'}});
    assert.equal(result.json().error.code,'CONFLICT');
  }
});

test('attached-game scope rejects another room and survives no service restart', async t => {
  const old = await createMockApp(); t.after(() => old.app.close());
  const a = old.pairing.initiate('restart-desktop','restart-pair');
  old.pairing.approve(a.activationCode,old.provider.issue('host','browser').session);
  const desktop = old.pairing.exchange(a.attemptId,'restart-desktop',a.pollingSecret);
  let room = old.pairing.snapshot('g0-desktop-room-forest');
  const envelope = (kind:string, body:unknown={}) => ({...base,serviceEpoch:old.pairing.serviceEpoch,requestId:randomUUID(),roomId:room.id,runId:room.runId,runEpoch:room.runEpoch,configRevision:room.configRevision,connectionEpoch:desktop.session.epoch,expectedRevision:room.revision,kind,body});
  for (const [kind,body] of [['desktop.join',{}],['desktop.ready',{ready:true}],['run.start',{}]] as const) {
    const response = await old.app.inject({method:'POST',url:'/g0/desktop/commands',headers:{authorization:`Bearer ${desktop.token}`},payload:envelope(kind,body)});
    assert.equal(response.statusCode,200,response.body); room=response.json().room;
  }
  const attach = {...base,serviceEpoch:old.pairing.serviceEpoch,roomId:room.id,runId:room.runId,runEpoch:room.runEpoch,configRevision:room.configRevision,connectionEpoch:desktop.session.epoch,expectedRevision:room.revision,gameInstanceId:'restart-game-context'};
  const response = await old.app.inject({method:'POST',url:'/g0/desktop/attach',headers:{authorization:`Bearer ${desktop.token}`},payload:attach});
  assert.equal(response.statusCode,200,response.body); const game=response.json();
  const headers={authorization:`Bearer ${game.token}`};
  assert.equal((await old.app.inject({url:`/g0/desktop/rooms/${room.id}`,headers})).statusCode,200);
  assert.equal((await old.app.inject({url:'/g0/desktop/rooms/g0-desktop-room-caverns',headers})).statusCode,403);
  assert.equal(old.pairing.snapshot('g0-desktop-room-caverns').runId,null);
  await old.app.close();
  const fresh = await createMockApp(); t.after(() => fresh.app.close());
  assert.notEqual(fresh.pairing.serviceEpoch,game.binding.serviceEpoch);
  assert.equal((await fresh.app.inject({url:`/g0/desktop/rooms/${room.id}`,headers})).statusCode,401);
  assert.equal((await fresh.app.inject({method:'POST',url:'/g0/desktop/commands',headers,payload:{...envelope('game.loaded',{attachmentId:game.binding.attachmentId,gameInstanceId:game.binding.gameInstanceId}),connectionEpoch:game.session.epoch}})).statusCode,401);
  assert.equal((await fresh.app.inject({method:'POST',url:'/g0/desktop/attach',headers:{authorization:`Bearer ${desktop.token}`},payload:attach})).statusCode,401);
  const reset=fresh.pairing.snapshot(room.id);
  assert.equal(reset.state,'waiting'); assert.equal(reset.runId,null);
  assert.equal(reset.runEpoch,null); assert.equal(reset.preparationExpiresAt,null);
  assert.deepEqual(reset.players,[]);
});
