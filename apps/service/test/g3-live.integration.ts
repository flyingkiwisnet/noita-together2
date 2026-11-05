import test from 'node:test';
import assert from 'node:assert/strict';
import {randomUUID} from 'node:crypto';
import {WebSocket} from 'ws';
import {checkServerIdentity} from 'node:tls';
import {startG3Fixture,provisionG3Fixture} from '../scripts/g3-fixture.js';
import {encode,decode,schema} from '../../../packages/contracts/src/g3-live.js';
import type {ClientFrame,ServerFrame,Scope} from '../../../packages/contracts/g3-live/g3-live.js';
const sleep=(ms:number)=>new Promise<void>(r=>setTimeout(r,ms));

import {livePeer} from '../scripts/g3-live-fixture.js';
import {PolicyError} from '../src/g1/security.js';

test('real one-issuer WSS: SQL-free remote peers during durable lock, scope/reauth and committed retirement',async()=>{
 let now=Date.now();const f=await startG3Fixture({live:true,tls:true,a1Runtime:true,a1HeartbeatBusy:true,nativeSocial:true,now:()=>now});const peers:Awaited<ReturnType<typeof livePeer>>[]=[];
 try{
  const s=await provisionG3Fixture(f,{beforeRpc:()=>{now+=100;}});await s.start();await s.playing();await s.pulse();
  peers.push(await livePeer(f,s,0),await livePeer(f,s,1));const [a,b]=peers;
  let release!:()=>void,entered!:()=>void;const enteredPromise=new Promise<void>(r=>{entered=r;}),held=new Promise<void>(r=>{release=r;});
  const durable=f.service.db.transaction(async()=>{entered();await held;});await enteredPromise;
  // A pose path taking the durable mutation lock could not deliver here.
  const sentAt=Date.now();a!.send({pose:{sequence:1,xMilli:'1000',yMilli:'2000',vxMilli:'0',vyMilli:'0'}});
  const snapshot=await b!.wait(v=>v.snapshot?.peers?.some(p=>p.sessionId===a!.scope.sessionId)??false,700);assert.equal(snapshot.snapshot!.peers![0]!.poses![0]!.xMilli,'1000');assert.ok(Date.now()-sentAt<700);
  release();await durable;
  assert.equal(f.service.live!.stats.receivedPoses,1);
  await assert.rejects(f.service.db.transaction(async tx=>{await tx.query('UPDATE accounts SET disabled=true WHERE id=$1',[a!.scope.principalId]);throw new PolicyError('FORBIDDEN');}),/FORBIDDEN/);
  assert.equal(a!.socket.readyState,1,'rolled-back revocation cannot change committed live authority');
  // An observer read can abort PostgreSQL's transaction. COMMIT then reports
  // ROLLBACK; it must never publish a projection or claim the reliable mutation.
  const observer=f.service.db.transactionProjection!;
  f.service.db.transactionProjection={...observer,prepare:async tx=>{try{await tx.query('SELECT * FROM deliberate_missing_live_projection_table');}catch{}}};
  await assert.rejects(f.service.db.transaction(tx=>tx.query('UPDATE accounts SET disabled=true WHERE id=$1',[a!.scope.principalId])),/SERVICE_UNAVAILABLE/);
  f.service.db.transactionProjection=observer;assert.equal((await f.service.db.pool.query('SELECT disabled FROM accounts WHERE id=$1',[a!.scope.principalId])).rows[0].disabled,false);
  assert.equal(a!.socket.readyState,1,'known rollback does not retire another valid authority');
  // Renew through the existing issuer/desktop operation, preserving scope and pose sequence.
  const renewed=(await s.rpc(s.desktops[0]!,{gameRenew:{attachmentId:s.games[0]!.attachmentId!,gameSessionId:s.games[0]!.sessionId!,gameSessionEpoch:s.games[0]!.sessionEpoch!,gameInstanceId:s.observations[0]!.gameInstanceId!,worldToken:s.scopes[0]!.worldToken!}})).credentials!;
  a!.send({reauthenticate:{accessToken:renewed.accessToken!,authRevision:2}});await a!.wait(v=>v.welcome?.authRevision===2);
  a!.send({pose:{sequence:2,xMilli:'3000',yMilli:'2000',vxMilli:'0',vyMilli:'0'}});await b!.wait(v=>v.snapshot?.peers?.some(p=>p.poses?.at(-1)?.sequence===2)??false);
  await s.rpc(s.games[0]!,{gameObservation:{...s.observations[0]!,observationRevision:3,protected:true}});await a!.wait(v=>!!v.retire);assert.equal(a!.socket.readyState===1,false);
  await s.rpc(s.games[0]!,{gameObservation:{...s.observations[0]!,observationRevision:4,protected:false}});
  const replay=await livePeer(f,s,0);peers.push(replay);replay.send({pose:{sequence:2,xMilli:'3000',yMilli:'2000',vxMilli:'0',vyMilli:'0'}});assert.equal((await replay.wait(v=>!!v.retire)).retire!.code,'STALE_EPOCH','same-world restoration cannot replay an old pose');await replay.close();
  const restored=await livePeer(f,s,0);peers.push(restored);assert.ok(restored.received[0]!.frame.authorityGeneration!>a!.received[0]!.frame.authorityGeneration!);restored.send({pose:{sequence:3,xMilli:'4000',yMilli:'2000',vxMilli:'0',vyMilli:'0'}});await b!.wait(v=>v.snapshot?.peers?.some(p=>p.poses?.at(-1)?.sequence===3)??false);
  await s.rpc(s.host,{endRun:{}});await b!.wait(v=>!!v.retire);assert.equal(f.service.live!.stats.samples,0);
 }finally{for(const p of peers)await p.close();await f.close();}
});
