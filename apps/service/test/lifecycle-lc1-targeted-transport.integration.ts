import test from 'node:test';import assert from 'node:assert/strict';import {randomUUID} from 'node:crypto';
import {startLifecycleLc1Fixture} from '../scripts/lifecycle-lc1-fixture.js';
import {livePeer} from '../scripts/g3-live-fixture.js';
import {schema as lobby} from '../../../packages/contracts/src/g2-next.js';
const sleep=(ms:number)=>new Promise<void>(r=>setTimeout(r,ms));
test('LC1 scoped transport rejects invalid kind/target/boolean and unbootstrapped peer',async()=>{
 const x=await startLifecycleLc1Fixture();try{
  for(const args of [['http',true,'unknown'],['invalid',true],['http','false'],['wss',true,'peer']])await assert.rejects(x.transport(...args as ['http',boolean]),/FIXTURE_TRANSPORT_ARGUMENT/);
  await x.transport('http',true,'consumer');await x.rpc(x.host,{roomGet:{}});await x.transport('http',false,'consumer');
 }finally{await x.close();}
});
test('LC1 targeted actual HTTP/refresh and first-frame WSS cuts isolate a healthy peer across rotation/recovery',async()=>{
 const x=await startLifecycleLc1Fixture(),sockets:Awaited<ReturnType<typeof livePeer>>[]=[];
 try{
  let consumer=x.desktop;const peer=await x.bootstrapPeer();let peerDesktop=peer.desktop;
  const obsA={...x.observation,observationRevision:2,protected:false},obsB={...peer.observation,observationRevision:2,protected:false};
  const peerRequest=(observation=peer.observation)=>{const q=x.request(peerDesktop,1,'peer',observation);q.reconcile!.association=peer.association;q.reconcile!.proof=peer.proof;return q;};
  await x.send(consumer,x.request());await x.send(peerDesktop,peerRequest());await x.start();
  let a=await x.send(consumer,x.request(consumer,1,'lc1-connection-1',obsA)),b=await x.send(peerDesktop,peerRequest(obsB));assert.equal(a.canonicalAdmission.decision.playing,true);assert.equal(b.canonicalAdmission.decision.playing,true);
  const open=async(value:any)=>{const p=await livePeer(x.fixture as any,{games:[value.credentials],scopes:[value.canonicalAdmission.scope]} as any,0);sockets.push(p);return p;};
  const blocked=async(value:any)=>assert.rejects(open(value),/Live frame timeout/);
  let liveA=await open(a),liveB=await open(b);
  const closed=async(p:typeof liveA)=>{for(let i=0;i<100&&p.socket.readyState!==3;i++)await sleep(10);assert.equal(p.socket.readyState,3);};
  const ping=async(p:typeof liveA)=>{x.advance(1100);const sequence=Math.max(0,...p.received.map(r=>r.frame.sequence??0));p.send({ping:{}});await p.wait(v=>!!v.pong&&(v.sequence??0)>sequence);assert.equal(p.socket.readyState,1);};
  // Actual same-session access rotation, then actual WSS reauthentication.
  consumer=(await x.rpc(undefined,{refresh:{secret:consumer.refreshSecret,deviceId:consumer.deviceId}})).credentials;
  x.advance(2000);a=await x.send(consumer,x.request(consumer,1,'lc1-connection-1',obsA));liveA.send({reauthenticate:{accessToken:a.credentials.accessToken,authRevision:2}});await liveA.wait(v=>v.welcome?.authRevision===2);
  await x.transport('wss',true,'consumer');await closed(liveA);await ping(liveB);await blocked(a);
  assert.equal((await x.send(consumer,x.request(consumer,1,'lc1-connection-1',obsA))).world.mayPlay,true,'WSS-only cut leaves lifecycle HTTP working');await x.rpc(x.host,{roomGet:{}});
  await x.transport('wss',false,'consumer');liveA=await open(a);await ping(liveA);await ping(liveB);
  // Targeted HTTP does not close either existing WSS connection.
  await x.transport('http',true,'consumer');await ping(liveA);await ping(liveB);
  assert.equal((await x.send(consumer,x.request(consumer,1,'lc1-connection-1',obsA))).error.code,'SERVICE_UNAVAILABLE');
  const refreshRequest={major:1,revision:lobby.revision,serviceId:x.fixture.service.db.serviceId,serviceEpoch:x.fixture.service.db.epoch,requestId:randomUUID(),refresh:{secret:consumer.refreshSecret,deviceId:consumer.deviceId}};
  const cutRefresh=await x.fixture.call('/v1/rpc',refreshRequest);assert.equal(cutRefresh.status,503);assert.equal(JSON.parse(cutRefresh.body).error.code,'SERVICE_UNAVAILABLE');
  // A false device claim cannot reclassify the durable refresh secret as peer.
  const falseClaim=await x.fixture.call('/v1/rpc',{...refreshRequest,requestId:randomUUID(),refresh:{...refreshRequest.refresh,deviceId:peerDesktop.deviceId}});assert.equal(falseClaim.status,503);
  peerDesktop=(await x.rpc(undefined,{refresh:{secret:peerDesktop.refreshSecret,deviceId:peerDesktop.deviceId}})).credentials;
  b=await x.send(peerDesktop,peerRequest(obsB));assert.equal(b.world.mayPlay,true);await x.rpc(x.host,{roomGet:{}});
  await x.transport('wss',true,'consumer');await closed(liveA);await ping(liveB);
  // A new current desktop family is matched by verified account/device, never
  // the original token/session snapshot. No scheduler/expiry claim is made.
  const current=await x.signIn();const newObs={...x.observation,gameInstanceId:'new-consumer',observationRevision:1};const next=x.request(current,2,'new-consumer',newObs);
  assert.equal((await x.send(current,next)).error.code,'SERVICE_UNAVAILABLE');
  for(let i=0;i<3;i++){x.advance(11000);await x.rpc(peerDesktop,{presence:{present:true}});await x.rpc(b.credentials,{gamePulse:{attachmentId:b.credentials.attachmentId,gameInstanceId:peer.observation.gameInstanceId,worldToken:peer.association.worldId}});await ping(liveB);}
  await x.transport('http',false,'consumer');const restored=await x.send(current,next);assert.equal(restored.error,undefined);assert.equal(restored.world.mayPlay,true);
  // The blocked refresh was not consumed; normal API can perform it after cut.
  assert.ok(JSON.parse((await x.fixture.call('/v1/rpc',{...refreshRequest,requestId:randomUUID()})).body).credentials);
  const newGame=await x.send(current,x.request(current,2,'new-consumer',{...newObs,observationRevision:2,protected:false}));assert.equal(newGame.canonicalAdmission.decision.playing,true);await blocked(newGame);await ping(liveB);
  await x.transport('wss',false,'consumer');liveA=await open(newGame);await ping(liveA);await ping(liveB);
  // Reverse target and omitted/global behavior preserve separate semantics.
  await x.transport('wss',true,'peer');await closed(liveB);await ping(liveA);await blocked(b);await x.transport('wss',false,'peer');liveB=await open(b);
  await x.transport('http',true);await assert.rejects(x.rpc(x.host,{roomGet:{}}),/SERVICE_UNAVAILABLE/);await ping(liveA);await ping(liveB);await x.transport('http',false);
  await x.transport('wss',true);await closed(liveA);await closed(liveB);await x.transport('wss',false);
  liveA=await open(newGame);liveB=await open(b);await ping(liveA);await ping(liveB);
 }finally{for(const s of sockets)await s.close();await x.close();}
});
