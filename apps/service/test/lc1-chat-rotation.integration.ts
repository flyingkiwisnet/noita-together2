// Actual isolated sign-in/refresh/chat/lifecycle RPCs; injected game observations.
import test from 'node:test';import assert from 'node:assert/strict';import {randomUUID} from 'node:crypto';import pg from 'pg';
import {startLifecycleLc1Fixture} from '../scripts/lifecycle-lc1-fixture.js';
test('fixture chat retains exact host/peer attribution across real Client-owned credential rotation without revoking gameplay',async()=>{
 const x=await startLifecycleLc1Fixture();try{
  const db=x.fixture.service.db,identity=await db.transaction(async tx=>(await tx.query('SELECT current_database() AS database,current_user AS owner,current_schema() AS schema')).rows[0]);assert.deepEqual(identity,{database:'nt2_g1',owner:'nt2_g1',schema:x.fixture.namespace});
  const peer=await x.bootstrapPeer(),a=x.request(),b=x.request(peer.desktop,1,'chat-peer',peer.observation);b.reconcile!.association=peer.association;b.reconcile!.proof=peer.proof;await x.send(x.desktop,a);await x.send(peer.desktop,b);await x.start();a.reconcile!.observation={...x.observation,protected:false,observationRevision:2};b.reconcile!.observation={...peer.observation,protected:false,observationRevision:2};assert.equal((await x.send(x.desktop,a)).canonicalAdmission.decision.playing,true);assert.equal((await x.send(peer.desktop,b)).canonicalAdmission.decision.playing,true);
  const rotate=async(c:any)=>(await x.rpc(undefined,{refresh:{secret:c.refreshSecret,deviceId:c.deviceId}})).credentials;
  const staleHost=x.host,stalePeer=peer.desktop;let host=await rotate(staleHost),consumer=await rotate(stalePeer);
  await assert.rejects(x.rpc(staleHost,{roomGet:{}}),/UNAUTHENTICATED/);await assert.rejects(x.rpc(stalePeer,{roomGet:{}}),/UNAUTHENTICATED/);
  for(let cycle=0;cycle<2;cycle++){
   const hm=await x.chat('host','host rotation '+cycle),pm=await x.chat('peer','peer rotation '+cycle);assert.equal(hm.chat.accountId,staleHost.accountId);assert.equal(hm.chat.principal.principalId,staleHost.accountId);assert.equal(pm.chat.accountId,stalePeer.accountId);assert.equal(pm.chat.principal.principalId,stalePeer.accountId);
   await x.rpc(host,{roomGet:{}});const messages=(await x.rpc(consumer,{chatList:{afterSequence:0,limit:32}})).chatMessages;assert.equal(messages.length,(cycle+1)*2);assert.equal(messages.at(-1).text,'peer rotation '+cycle);assert.equal(messages.at(-1).principal.principalId,consumer.accountId);
   b.requestId=randomUUID();b.sessionEpoch=consumer.sessionEpoch;const live=await x.send(consumer,b);assert.equal(live.canonicalAdmission.decision.playing,true);assert.equal(live.world.generation,1);
   const facts=await db.transaction(async tx=>({members:(await tx.query('SELECT count(*)::int AS count FROM memberships WHERE room_id=$1',[x.room.roomId])).rows[0].count,worlds:(await tx.query('SELECT count(*)::int AS count FROM world_associations WHERE run_id=$1',[x.room.runId])).rows[0].count,actors:(await tx.query("SELECT f.device_id FROM sessions s JOIN session_families f ON f.id=s.family_id WHERE f.device_id LIKE 'lc1-chat-%' AND s.revoked_at IS NULL AND f.revoked_at IS NULL")).rows}));assert.equal(facts.members,2);assert.equal(facts.worlds,2);assert.equal(facts.actors.length,2);assert.ok(facts.actors.every(a=>![x.desktop.deviceId,peer.desktop.deviceId].includes(a.device_id)));
   if(cycle===0){host=await rotate(host);consumer=await rotate(consumer);}
  }
 }finally{const namespace=x.fixture.namespace;await x.close();const c=new pg.Client({connectionString:'postgres://nt2_g1:local-fixture-only@127.0.0.1:54328/nt2_g1'});try{await c.connect();assert.equal((await c.query('SELECT 1 FROM pg_namespace WHERE nspname=$1',[namespace])).rowCount,0);console.log(JSON.stringify({chatRotationSchemaAbsent:namespace}));}finally{await c.end();}}
});
