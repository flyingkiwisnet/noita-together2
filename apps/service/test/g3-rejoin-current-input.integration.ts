import test from 'node:test';import assert from 'node:assert/strict';import {readFile} from 'node:fs/promises';import {createHash} from 'node:crypto';
import {startG3Fixture,provisionG3Fixture} from '../scripts/g3-loot01-fixture.js';
import {materializeRejoinComposition} from '../src/rejoin/composition-profile.js';
const dependency=new URL('../../../docs/g3/server-rejoin-integration-dependency.json',import.meta.url),sha=(b:Buffer)=>createHash('sha256').update(b).digest('hex');
test('RJ2 actual pinned current normal artifacts materialize idempotently and select exact disposable consumer-owned runtime',async()=>{
 const d=JSON.parse(await readFile(dependency,'utf8'));assert.ok(d.currentInput,'Current Client snapshot required');
 const r=await materializeRejoinComposition(d.currentInput);assert.equal(r.runtimeApproval.sha256,d.runtimeApproval.sha256);assert.equal(r.desktopApproval.sha256,d.desktopApproval.sha256);assert.equal(r.requiredLocalFiles.length,36);assert.deepEqual(r.nativeArtifacts.map((a:any)=>a.testFlags),[false,false,false]);
 const f=await startG3Fixture({rejoin:true,loot01Runtime:true,rejoinInput:d.currentInput,live:true,tls:true});try{
  const remote=await f.call('/v3/social/runtime');assert.equal(sha(Buffer.from(remote.body)),r.runtimeApproval.sha256);assert.equal(JSON.parse(remote.body).id,r.id);
  const info=JSON.parse((await f.call('/v3/social/info')).body);assert.equal(info.sourceApproved,true);assert.deepEqual(info.runtimeApproval,r.runtimeApproval);assert.deepEqual(info.desktopApproval,r.desktopApproval);
  const s=await provisionG3Fixture(f,{consumerPlayer:0});assert.equal(s.games.length,1);await assert.rejects(s.start(),/ROSTER_MISMATCH|NOT_READY/);
  const epoch=f.service.db.epoch;await f.restart();assert.notEqual(f.service.db.epoch,epoch);assert.equal(f.profile.id,r.id);assert.equal(sha(Buffer.from((await f.call('/v3/social/runtime')).body)),r.runtimeApproval.sha256);
 }finally{await f.close();}
});
