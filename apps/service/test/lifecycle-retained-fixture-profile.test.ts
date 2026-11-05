// Read-only PE/source preflight. No service/native launch, DB or normal profiles.
import test from 'node:test';import assert from 'node:assert/strict';import {readFile,writeFile,mkdtemp,rm} from 'node:fs/promises';import {createHash} from 'node:crypto';import {resolve,join,relative,sep} from 'node:path';
import {verifyLifecycleFixtureRuntime} from '../src/rejoin/lifecycle-fixture-profile.js';
test('reviewed retained A2 selector verifies only with matching private compile selectors and source pins',async()=>{
 const report=JSON.parse(await readFile('apps/desktop/a2-retained-information-checks-20261022.json','utf8')),pin=report.fixture.profile;
 const original=JSON.parse(await readFile(pin.path,'utf8')),verified=await verifyLifecycleFixtureRuntime(pin);
 assert.equal(verified.runtimeMappings.length,66);assert.equal(verified.fixtureNative.pipePrefix,'\\\\.\\pipe\\nt2-a2-retained-20261022-01-');assert.equal(verified.fixtureNative.private,true);assert.equal(verified.installApproved,false);
 const build=resolve('spikes/noita-entry/.build'),folder=await mkdtemp(join(build,'retained-selector-check-'));let n=0;
 const candidate=async(v:any)=>{const path=join(folder,++n+'.json'),b=JSON.stringify(v);await writeFile(path,b);return {path:relative(process.cwd(),path).replaceAll('\\','/'),sha256:createHash('sha256').update(b).digest('hex')};};
 try{
  for(const prefix of ['\\\\.\\pipe\\nt2-g3-c2-dev-v1-','\\\\.\\pipe\\nt2-a2-other-','\\\\.\\pipe\\nt2-a2-retained-20261022-02-','\\\\server\\pipe\\nt2-a2-retained-20261022-01-']){const v=structuredClone(original);v.fixtureNative.pipePrefix=prefix;await assert.rejects(verifyLifecycleFixtureRuntime(await candidate(v)),/private LC1\/A2 pipe selector/);}
  const previous=structuredClone(original);previous.fixtureNative.pipePrefix='\\\\.\\pipe\\nt2-a2-client-20261020-01-';await assert.rejects(verifyLifecycleFixtureRuntime(await candidate(previous)),/private compiled selectors/);
  const wrongApp=structuredClone(original);wrongApp.fixtureNative.testAppPath=join(build,'different-reviewed-input.exe');await assert.rejects(verifyLifecycleFixtureRuntime(await candidate(wrongApp)),/private compiled selectors/);
  const wrongSource=structuredClone(original);wrongSource.fixtureNative.sourcePins[0].sha256='0'.repeat(64);await assert.rejects(verifyLifecycleFixtureRuntime(await candidate(wrongSource)),/changed/);
  const normal=structuredClone(original);normal.nativeArtifacts[0].private=false;await assert.rejects(verifyLifecycleFixtureRuntime(await candidate(normal)),/explicit native flags/);
  console.log(JSON.stringify({retainedFixturePreflight:true,runtimeFiles:66,negativeCases:8,sourceAndCompiledSelectorFences:true,serviceStarted:false,databaseTouched:false}));
 }finally{assert.ok(resolve(folder).startsWith(build+sep));await rm(folder,{recursive:true,force:true});}
});
