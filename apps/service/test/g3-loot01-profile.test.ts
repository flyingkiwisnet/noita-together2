import test from 'node:test';import assert from 'node:assert/strict';import {readFile} from 'node:fs/promises';
import {verifyNativeLoot01Artifacts,nativeLoot01RuntimeResource} from '../src/g3/native-loot01-profile.js';
import {declareRoomSemantics,validateRoomSemantics} from '../../../packages/contracts/src/room-semantics.js';
import {matchGameBuild} from '../src/g2-next/build-registry.js';
test('LOOT01-S-MATCH exact35 native closure retains issuer/original Steam GOG pairs, reviewed capabilities and bounded nonblocking ABI',async()=>{
 const r=await verifyNativeLoot01Artifacts();assert.equal(r.runtimeMappings.length,35);assert.equal(r.serviceId,'nt2-g3-c2-potion-correction');assert.equal(r.installApproved,false);assert.equal(r.packaged,false);assert.equal(r.actualNoitaEvidence,false);assert.equal(r.nativeArtifacts.length,3);
 for(const b of r.profile.supportedBuilds)assert.equal(matchGameBuild(r.buildRegistry,b.gameExeSha256,b.originalLuaSha256)?.id,b.id);
 assert.equal(matchGameBuild(r.buildRegistry,'0'.repeat(64),r.profile.originalLuaSha256),undefined);assert.deepEqual(r.profile.supportedBuilds.map((b:any)=>b.store),['steam','gog']);
 assert.equal(r.localProtocol.actualSeedControlVersion,6);assert.equal(r.localProtocol.controlVersion,4);assert.equal(r.localProtocol.reportVersion,5);assert.equal(r.localProtocol.policyControlOperation,10);assert.equal(r.localProtocol.startupWaitMaximumMs,250);
 const semantics=declareRoomSemantics(r.compatibility);assert.equal(semantics.kind,'core-social-seed-loot');assert.deepEqual(validateRoomSemantics(semantics,r.compatibility),r.compatibility.capabilities);assert.throws(()=>validateRoomSemantics(semantics,{capabilities:[...r.compatibility.capabilities,'core.gold']}));
 assert.equal(declareRoomSemantics({capabilities:['core.seed','core.ghost','core.hud','core.poi']}).kind,'core-social');
});
test('LOOT01-S-MATCH source/runtime/native/loot/canonical drift refuses rather than mixing baseline and new closure',async()=>{
 const r=await verifyNativeLoot01Artifacts();for(const changed of [nativeLoot01RuntimeResource.path,r.runtimeMappings[0].sourcePath,r.nativeArtifacts[0].path,r.runtimeMappings.find((m:any)=>m.path.endsWith('/run-policy.txt')).sourcePath,'packages/contracts/src/room-semantics.js']){
  await assert.rejects(verifyNativeLoot01Artifacts(async url=>{const b=await readFile(url);return url.pathname.endsWith('/'+changed)?Buffer.concat([b,Buffer.from('drift')]):b;}),/changed/);
 }
});

test('LOOT01 fixed inert bootstrap missing refuses metadata selection',async()=>{await assert.rejects(verifyNativeLoot01Artifacts(async url=>{if(url.pathname.endsWith('/files/loot/run-policy.txt'))throw Error('missing inert bootstrap');return readFile(url);}),/missing inert bootstrap/);});
