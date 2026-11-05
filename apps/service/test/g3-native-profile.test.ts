import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import {verifyNativeDevelopmentArtifacts,runtimeApprovalResource,desktopReviewResource,nativeDevelopmentProfile} from '../src/g3/native-profile.js';

test('split runtime approval remains exact while request9 desktop closure changes; defective supplemental social runtime withheld; every dependency refuses drift',async()=>{
 const root=new URL('../../../',import.meta.url),cache=new Map<string,Buffer>();
 const reader=async(url:URL)=>{let bytes=cache.get(url.href);if(!bytes){bytes=await readFile(url);cache.set(url.href,bytes);}return bytes;};
 const metadata=await verifyNativeDevelopmentArtifacts(reader);
 assert.deepEqual(metadata.profile,nativeDevelopmentProfile.profile);
 assert.deepEqual(metadata.compatibility,nativeDevelopmentProfile.compatibility);
 assert.equal(metadata.id,'nt2-g3-native-development-2');assert.equal(metadata.installApproved,false);
 assert.equal(metadata.provenance.pins.length,185);
 assert.equal(metadata.provenance.supplementalRuntime.approved,false);assert.deepEqual(metadata.provenance.supplementalRuntime.capabilities,[]);assert.match(metadata.provenance.supplementalRuntime.finding,/G3-R12/);assert.deepEqual(metadata.compatibility.capabilities,['core.seed','core.ghost']);
 const runtime=JSON.parse((await reader(new URL(runtimeApprovalResource.path,root))).toString());
 assert.equal(runtime.provenance.pins.length,18);
 assert.equal(runtime.requiredLocalFiles.length,14);assert.ok(runtime.requiredLocalFiles.some((p:{path:string})=>p.path.endsWith('/logging.xml')));
 assert.ok(runtime.provenance.pins.every((p:{path:string})=>!p.path.includes('/apps/desktop/')));
 assert.ok(!JSON.stringify(runtime).includes('g3_candidate.exe'));
 const paths=[...new Set([runtimeApprovalResource.path,desktopReviewResource.path,runtime.provenance.originalReview.path,...runtime.provenance.pins.map((p:{path:string})=>p.path),metadata.provenance.manifest.path,metadata.provenance.snapshotRequest.path,...metadata.provenance.pins.map((p:{path:string})=>p.path)])];
 let refused=0;
 for(const target of paths){let reached=false;await assert.rejects(verifyNativeDevelopmentArtifacts(async url=>{if(url.pathname.endsWith('/'+target)){reached=true;return Buffer.from('changed');}return reader(url);}),/changed/);assert.equal(reached,true,target);refused++;}
 console.log('Split native positive baseline + '+refused+' individual targeted refusals');
});
