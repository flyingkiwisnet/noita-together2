import test from 'node:test';import assert from 'node:assert/strict';import {readFile} from 'node:fs/promises';
import {verifyRejoinPackage,rejoinRuntimePath,rejoinPackageInput} from '../src/rejoin/package-profile.js';
import {verifyRejoinCompositionInput} from '../src/rejoin/composition-profile.js';
const root=new URL('../../../',import.meta.url);
test('RJ3 final36 normal profile matches current resources, ABI2, new namespace and no private images',async()=>{const r=await verifyRejoinPackage();assert.equal(r.requiredLocalFiles.length,36);assert.equal(r.id,'nt2-g3-rejoin-dev-1');assert.equal(r.namespacePrefix,'g3_rejoin_');assert.equal(r.localProtocol.brokerAbiVersion,2);assert.ok(r.nativeArtifacts.every((a:any)=>!a.testFlags&&!a.loaded));assert.ok(!Object.hasOwn(r,'privateArtifacts'));assert.equal(r.installApproved,false);assert.equal(r.actualNoitaEvidence,false);});
test('RJ3 altered resource reader refuses rather than widening profile or touching staged files',async()=>{await assert.rejects(verifyRejoinPackage(async url=>{const b=await readFile(url);return url.pathname.endsWith('/'+rejoinRuntimePath)?Buffer.concat([b,Buffer.from('drift')]):b;}),/resource drift/);});

test('CF01 frozen current input and broker source drift refuse without modifying shared files',async()=>{
 for(const path of [rejoinPackageInput.path,'native/g3-ipc/broker.cpp']){
  let injected=false;
  await assert.rejects(verifyRejoinCompositionInput(rejoinPackageInput,async url=>{
   const bytes=await readFile(url);if(url.pathname.endsWith('/'+path)){injected=true;return Buffer.concat([bytes,Buffer.from('drift')]);}return bytes;
  }),/Rejoin composition:.*changed/);
  assert.equal(injected,true);
 }
});
