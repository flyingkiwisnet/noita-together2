import test from 'node:test';import assert from 'node:assert/strict';import {readFile} from 'node:fs/promises';
import {verifyNativeSeedLootArtifacts,nativeSeedLootRuntimeResource} from '../src/g3/native-seed-loot-profile.js';
import {declareRoomSemantics,validateRoomSemantics} from '../../../packages/contracts/src/room-semantics.js';
import {matchGameBuild} from '../src/g2-next/build-registry.js';
test('U2-S-PREP exact34 native closure retains issuer/original Steam GOG pairs, reviewed capabilities and bounded nonblocking ABI',async()=>{
 const r=await verifyNativeSeedLootArtifacts();assert.equal(r.runtimeMappings.length,34);assert.equal(r.serviceId,'nt2-g3-c2-potion-correction');assert.equal(r.installApproved,false);assert.equal(r.packaged,false);assert.equal(r.actualNoitaEvidence,false);assert.equal(r.nativeArtifacts.length,3);
 for(const b of r.profile.supportedBuilds)assert.equal(matchGameBuild(r.buildRegistry,b.gameExeSha256,b.originalLuaSha256)?.id,b.id);
 assert.equal(matchGameBuild(r.buildRegistry,'0'.repeat(64),r.profile.originalLuaSha256),undefined);assert.deepEqual(r.profile.supportedBuilds.map((b:any)=>b.store),['steam','gog']);
 assert.equal(r.localProtocol.actualSeedControlVersion,6);assert.equal(r.localProtocol.controlVersion,4);assert.equal(r.localProtocol.reportVersion,5);assert.equal(r.localProtocol.policyControlOperation,10);assert.equal(r.localProtocol.startupWaitMaximumMs,250);
 const semantics=declareRoomSemantics(r.compatibility);assert.equal(semantics.kind,'core-social-seed-loot');assert.deepEqual(validateRoomSemantics(semantics,r.compatibility),r.compatibility.capabilities);assert.throws(()=>validateRoomSemantics(semantics,{capabilities:[...r.compatibility.capabilities,'core.gold']}));
 assert.equal(declareRoomSemantics({capabilities:['core.seed','core.ghost','core.hud','core.poi']}).kind,'core-social');
});
test('U2-S-PREP source/runtime/native/loot/canonical drift refuses rather than mixing baseline and new closure',async()=>{
 const r=await verifyNativeSeedLootArtifacts();for(const changed of [nativeSeedLootRuntimeResource.path,r.runtimeMappings[0].sourcePath,r.nativeArtifacts[0].path,r.runtimeMappings.find((m:any)=>m.path.includes('loot')).sourcePath,'packages/contracts/src/room-semantics.js']){
  await assert.rejects(verifyNativeSeedLootArtifacts(async url=>{const b=await readFile(url);return url.pathname.endsWith('/'+changed)?Buffer.concat([b,Buffer.from('drift')]):b;}),/changed/);
 }
});
