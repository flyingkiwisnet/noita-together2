import test from 'node:test';import assert from 'node:assert/strict';import {readFile} from 'node:fs/promises';
import {verifyNativeA1DevelopmentArtifacts,nativeA1RuntimeResources,nativeA1SourceReviewResource} from '../src/g3/native-a1-profile.js';
import {matchGameBuild} from '../src/g2-next/build-registry.js';
test('A1 source closures retain exact pairs and narrow caps; every independent frozen dependency refuses drift',async()=>{
 const cache=new Map<string,Buffer>(),reader=async(url:URL)=>{let b=cache.get(url.href);if(!b){b=await readFile(url);cache.set(url.href,b);}return b;};
 for(const social of [false,true]){
  const m=await verifyNativeA1DevelopmentArtifacts(social,reader);assert.equal(m.requiredLocalFiles.length,social?20:14);assert.equal(m.installApproved,false);assert.equal(m.sourceReview.usableCandidate,false);
  assert.deepEqual(m.compatibility.capabilities,social?['core.seed','core.ghost','core.hud','core.poi']:['core.seed','core.ghost']);
  for(const v of m.buildRegistry.supportedBuilds){assert.equal(matchGameBuild(m.buildRegistry,v.gameExeSha256,v.originalLuaSha256)?.id,v.id);assert.equal(matchGameBuild(m.buildRegistry,v.gameExeSha256,'0'.repeat(64)),undefined);}
  assert.equal(matchGameBuild(m.buildRegistry,'0'.repeat(64),m.buildRegistry.originalLuaSha256),undefined);
  for(const p of m.runtimeMappings.filter((p:any)=>p.path.endsWith('.dll'))){const b=await reader(new URL('../../../'+p.sourcePath,import.meta.url));assert.equal(b.toString('ascii',0,2),'MZ');const pe=b.readUInt32LE(60);assert.equal(b.toString('ascii',pe,pe+4),'PE\0\0');assert.equal(b.readUInt16LE(pe+4),0x14c);assert.equal(b.readUInt16LE(pe+24),0x10b);assert.ok(b.readUInt16LE(pe+22)&0x2000);}
  const targets=[...new Set([nativeA1RuntimeResources[social?'social':'primary'].path,nativeA1SourceReviewResource.path,m.sourceReview.snapshotRequest.path,...m.sourceReview.pins.map((p:any)=>p.path),...m.provenance.pins.map((p:any)=>p.path)])];
  for(const target of targets){let reached=false;await assert.rejects(verifyNativeA1DevelopmentArtifacts(social,async url=>{if(url.pathname.endsWith('/'+target)){reached=true;return Buffer.from('changed');}return reader(url);}),/changed/);assert.equal(reached,true,target);}
  console.log('A1 '+(social?'social':'primary')+' baseline + '+targets.length+' individually reached refusals');
 }
});
