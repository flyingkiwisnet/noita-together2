import test from 'node:test';import assert from 'node:assert/strict';import {createHash,randomUUID} from 'node:crypto';
import {startLifecycleLc1Fixture} from '../scripts/lifecycle-lc1-fixture.js';
import {persistentRejoinIdentity} from '../src/rejoin/host-identity.js';
import {RejoinDatabase} from '../src/rejoin/database.js';
const digest=(v:unknown)=>createHash('sha256').update(JSON.stringify(v)).digest('hex');
test('Persistent signer startup skips redundant migrations; membership no-op and durable world eligibility survive restart',async()=>{
 const x=await startLifecycleLc1Fixture({persistentIdentity:true}),f=x.fixture;try{
  const loaded=await x.send(x.desktop,x.request());assert.equal(loaded.error,undefined);await x.start();
  const keys=digest(f.service.auth.security.keys),epoch=f.service.db.epoch;
  const status=async()=>JSON.parse((await f.call('/v3/rejoin/rpc',{major:1,revision:'g3-rejoin-1',serviceId:f.service.db.serviceId,serviceEpoch:f.service.db.epoch,sessionEpoch:x.desktop.sessionEpoch,requestId:randomUUID(),status:{}},x.desktop.accessToken)).body);
  const before=await status();assert.ok(before.resumeProof);
  // Leave one already-reset membership alongside an active one. Recovery must
  // clear the latter without creating a new tuple for the former.
  const member=x.desktop.sessionId;
  await f.service.db.pool.query('UPDATE memberships SET presence_until=NULL,ready_config_revision=NULL,selected=false WHERE desktop_id=$1',[member]);
  const tuple=(await f.service.db.pool.query('SELECT xmin::text AS version FROM memberships WHERE desktop_id=$1',[member])).rows[0];assert.ok(tuple);
  const other=await f.service.db.transaction(tx=>f.service.auth.family(tx,x.desktop.accountId!,randomUUID(),'desktop'));
  await f.service.db.pool.query('INSERT INTO memberships(desktop_id,room_id,account_id,presence_until,ready_config_revision,selected) SELECT $1,room_id,account_id,now(),1,true FROM memberships WHERE desktop_id=$2',[other.sessionId,member]);
  const migrate=RejoinDatabase.prototype.migrate;let traversals=0;RejoinDatabase.prototype.migrate=async function(){traversals++;return migrate.call(this);};
  try{await f.restart();assert.equal(traversals,1,'only service startup checks the full migration chain');}finally{RejoinDatabase.prototype.migrate=migrate;}
  assert.notEqual(f.service.db.epoch,epoch);assert.equal(digest(f.service.auth.security.keys),keys);
  assert.equal((await f.service.db.pool.query('SELECT xmin::text AS version FROM memberships WHERE desktop_id=$1',[member])).rows[0].version,tuple.version);
  assert.equal((await f.service.db.pool.query('SELECT count(*)::int AS stale FROM memberships WHERE presence_until IS NOT NULL OR ready_config_revision IS NOT NULL OR selected IS DISTINCT FROM false')).rows[0].stale,0);
  const state=await x.state();assert.equal(state.participants.length,1);assert.equal(state.participants[0].state,'reconnecting');assert.equal(state.participants[0].terminal,false);
  const recovered=await status();assert.equal(recovered.error,undefined);assert.equal(recovered.participant.returnEligible,true);assert.equal(recovered.participant.state,'reconnecting');assert.equal(digest(recovered.resumeProof),digest(before.resumeProof));
  const url='postgres://nt2_g1:local-fixture-only@127.0.0.1:54328/nt2_g1';await assert.rejects(persistentRejoinIdentity(url,f.service.db.serviceId,f.namespace,'https://other.invalid'),/never reset/);
  assert.equal(digest(await persistentRejoinIdentity(url,f.service.db.serviceId,f.namespace,f.origin)),keys);
 }finally{await x.close();}
});
