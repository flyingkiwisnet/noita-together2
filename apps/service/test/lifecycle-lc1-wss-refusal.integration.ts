import test from 'node:test';import assert from 'node:assert/strict';import {randomUUID} from 'node:crypto';import {Agent} from 'node:https';import {checkServerIdentity} from 'node:tls';import {WebSocket} from 'ws';
import {startLifecycleLc1Fixture} from '../scripts/lifecycle-lc1-fixture.js';import {livePeer} from '../scripts/g3-live-fixture.js';
import {encode,decode,schema} from '../../../packages/contracts/src/g3-live.js';import type {Scope,ServerFrame} from '../../../packages/contracts/g3-live/g3-live.js';
const sleep=(ms:number)=>new Promise<void>(r=>setTimeout(r,ms));
async function probe(x:Awaited<ReturnType<typeof startLifecycleLc1Fixture>>,token:string,scope:Scope){
 const agent=new Agent({ca:x.fixture.ca!,checkServerIdentity:(_host,cert)=>checkServerIdentity('127.0.0.1',cert)}),socket=new WebSocket(x.fixture.origin.replace('https:','wss:')+'/v3/live','nt2.g3.live.v1',{agent,perMessageDeflate:false}),frames:ServerFrame[]=[];
 let closeCode:number|undefined;socket.on('error',()=>{});socket.on('close',code=>{closeCode=code;});socket.on('message',data=>frames.push(decode('ServerFrame',data as Buffer) as ServerFrame));
 const close=async()=>{socket.terminate();agent.destroy();await sleep(10);};
 try{await new Promise<void>((resolve,reject)=>{socket.once('open',resolve);socket.once('error',reject);});socket.send(Buffer.from(encode('ClientFrame',{major:1,revision:schema.revision,serviceId:x.fixture.service.db.serviceId,serviceEpoch:x.fixture.service.db.epoch,connectionId:randomUUID(),clientGeneration:1,authorityGeneration:0,sequence:1,scope,authenticate:{accessToken:token}})),{binary:true});
  for(let i=0;i<350&&!frames.some(f=>f.welcome||f.retire)&&closeCode===undefined;i++)await sleep(10);
  const first=frames.find(f=>f.welcome||f.retire);if(first?.retire)for(let i=0;i<100&&closeCode===undefined;i++)await sleep(10);
  return {socket,frames,first,closeCode,close};
 }catch(e){await close();throw e;}
}
test('LC1 WSS refusal control is consumer-only, bounded, non-overwriting and sanitized',async()=>{
 const x=await startLifecycleLc1Fixture();try{
  for(const target of [undefined,null,'peer','other',false])assert.throws(()=>x.denyWssOnce(target as 'consumer'),/FIXTURE_WSS_REFUSAL_ARGUMENT/);
  for(let i=0;i<8;i++){const armed=x.denyWssOnce('consumer');assert.equal(armed.armed,true);assert.equal(armed.armsUsed,i+1);assert.equal(armed.consumedCount,0);assert.throws(()=>x.denyWssOnce('consumer'),/FIXTURE_WSS_REFUSAL_ARGUMENT/);x.clearWssRefusal();}
  assert.throws(()=>x.denyWssOnce('consumer'),/FIXTURE_WSS_REFUSAL_ARGUMENT/);assert.equal(x.wssRefusalStatus().totalConsumed,0);
 }finally{await x.close();}
});
test('LC1 one verified current consumer initial WSS auth receives canonical FORBIDDEN once; peer/invalid/reauth traffic does not consume',async()=>{
 const x=await startLifecycleLc1Fixture(),connections:{close:()=>Promise<void>}[]=[];
 try{
  const peer=await x.bootstrapPeer(),peerRequest=(protectedWorld=true)=>{const q=x.request(peer.desktop,1,'peer',{...peer.observation,observationRevision:protectedWorld?1:2,protected:protectedWorld});q.reconcile!.association=peer.association;q.reconcile!.proof=peer.proof;return q;};
  await x.send(x.desktop,x.request());await x.send(peer.desktop,peerRequest());await x.start();
  let a=await x.send(x.desktop,x.request(x.desktop,1,'lc1-connection-1',{...x.observation,observationRevision:2,protected:false}));const b=await x.send(peer.desktop,peerRequest(false));
  const liveA=await livePeer(x.fixture as any,{games:[a.credentials],scopes:[a.canonicalAdmission.scope]} as any,0);connections.push(liveA);
  const armed=x.denyWssOnce('consumer'),status=()=>{assert.equal(x.wssRefusalStatus().armId,armed.armId);assert.equal(x.wssRefusalStatus().armed,true);assert.equal(x.wssRefusalStatus().consumedCount,0);};
  await x.rpc(x.host,{roomGet:{}});await x.send(peer.desktop,peerRequest(false));status();
  const bad=await probe(x,'not-a-valid-token',a.canonicalAdmission.scope);connections.push(bad);assert.equal(bad.first!.retire!.code,'UNAUTHENTICATED');status();
  const foreign=await probe(x,b.credentials.accessToken,a.canonicalAdmission.scope);connections.push(foreign);assert.equal(foreign.first!.retire!.code,'FORBIDDEN');status();
  const wrongScope=await probe(x,a.credentials.accessToken,{...a.canonicalAdmission.scope,worldToken:randomUUID()});connections.push(wrongScope);assert.equal(wrongScope.first!.retire!.code,'FORBIDDEN');status();
  const liveB=await livePeer(x.fixture as any,{games:[b.credentials],scopes:[b.canonicalAdmission.scope]} as any,0);connections.push(liveB);assert.ok(liveB.received.some(f=>f.frame.welcome));status();
  x.advance(2000);a=await x.send(x.desktop,x.request(x.desktop,1,'lc1-connection-1',{...x.observation,observationRevision:2,protected:false}));liveA.send({reauthenticate:{accessToken:a.credentials.accessToken,authRevision:2}});await liveA.wait(f=>f.welcome?.authRevision===2);status();
  await x.transport('wss',true,'consumer');const cut=await probe(x,a.credentials.accessToken,a.canonicalAdmission.scope);connections.push(cut);assert.equal(cut.closeCode,1012);assert.equal(cut.first,undefined);status();await x.transport('wss',false,'consumer');
  const refused=await probe(x,a.credentials.accessToken,a.canonicalAdmission.scope);connections.push(refused);assert.equal(refused.first!.retire!.code,'FORBIDDEN');assert.equal(refused.first!.retire!.retryAfterMs,0);assert.equal(refused.closeCode,1008);assert.equal(refused.frames.some(f=>!!f.welcome),false);
  const consumed=x.wssRefusalStatus();assert.equal(consumed.armed,false);assert.equal(consumed.consumedCount,1);assert.equal(consumed.totalConsumed,1);
  const later=await probe(x,a.credentials.accessToken,a.canonicalAdmission.scope);connections.push(later);assert.ok(later.first!.welcome);assert.equal(x.wssRefusalStatus().totalConsumed,1);
  liveB.send({ping:{}});await liveB.wait(f=>!!f.pong);assert.equal(liveB.socket.readyState,1);
  console.log(JSON.stringify({wssRefusalAssertions:{...consumed,canonicalRetire:'FORBIDDEN',canonicalClose:1008,peerUnaffected:true,laterEligibleWelcome:true,invalidAndForeignDidNotConsume:true,reauthDidNotConsume:true}}));
 }finally{for(const c of connections)await c.close();await x.close();}
});
