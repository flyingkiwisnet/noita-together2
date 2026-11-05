import test from 'node:test';import assert from 'node:assert/strict';
import {startG3Fixture} from '../scripts/g3-fixture.js';
import {CommittedRoomAuthority} from '../src/g3/live-lifecycle.js';
import {afterCommit} from '../src/g1/commit-hooks.js';
test('committed room preparation publishes only after COMMIT, retains reliable result, fences rollback/retirement/late work',async()=>{
 let now=100;const f=await startG3Fixture();
 try{
  const cache=new CommittedRoomAuthority(f.service.db.epoch,()=>now),roomId='owned-room',nonce=cache.reserve(roomId,0);
  const p={serviceEpoch:f.service.db.epoch,roomId,version:1,retired:false,participants:[{sessionId:'owned-session',scopeDigest:'a'.repeat(64),authorityGeneration:1,until:1000,playing:true}]};
  await assert.rejects(f.service.db.transaction(async tx=>{cache.stage(tx,nonce,p);assert.equal(cache.authority(roomId,'owned-session'),undefined);throw Error('rollback');}));
  assert.equal(cache.authority(roomId,'owned-session'),undefined);
  const result=await f.service.db.transaction(async tx=>{cache.stage(tx,nonce,p);return 'committed reliable value';});assert.equal(result,'committed reliable value');assert.ok(cache.authority(roomId,'owned-session'));
  assert.equal(await f.service.db.transaction(async tx=>{afterCommit(tx,()=>{throw Error('projection failure');},()=>cache.retire(roomId,nonce));return 'durable success';}),'durable success');assert.equal(cache.authority(roomId,'owned-session'),undefined);
  assert.equal(cache.apply(nonce,{...p,version:2}),false);
  cache.release(roomId,nonce);const next=cache.reserve(roomId,2);assert.equal(cache.apply(nonce,{...p,version:3}),false);assert.equal(cache.apply(next,{...p,version:3}),true);assert.equal(cache.apply(next,{...p,version:2}),false);
  now=1000;assert.equal(cache.authority(roomId,'owned-session'),undefined);assert.equal(cache.apply(next,{...p,version:4,participants:[{...p.participants[0]!,until:2000}]}),false);
 }finally{await f.close();}
});
