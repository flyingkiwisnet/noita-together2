import test from 'node:test';import assert from 'node:assert/strict';
import {readFile,writeFile,mkdtemp,rm} from 'node:fs/promises';import {createHash} from 'node:crypto';import {resolve,join,relative,sep} from 'node:path';
import {verifyLifecycleFixtureRuntime} from '../src/rejoin/lifecycle-fixture-profile.js';
const sha=(b:string)=>createHash('sha256').update(b).digest('hex');
test('Private Race324 metadata verifies exact dependencies and rejects selector/hash/alias/native changes',{skip:!process.env.NT2_RACE_FIXTURE_PIN},async()=>{
 const pin=JSON.parse(await readFile(process.env.NT2_RACE_FIXTURE_PIN!,'utf8')),original=JSON.parse(await readFile(pin.path,'utf8'));
 const verified=await verifyLifecycleFixtureRuntime(pin);assert.equal(verified.runtimeMappings.length,324);assert.equal(verified.raceFixture.revision,'race-first-release-private-1');
 const build=resolve('spikes/noita-entry/.build'),folder=await mkdtemp(join(build,'race-verifier-'));assert.ok(resolve(folder).startsWith(build+sep));let serial=0,cases=0;
 const candidate=async(value:any)=>{const p=join(folder,++serial+'.json'),bytes=JSON.stringify(value);await writeFile(p,bytes);return {path:relative(process.cwd(),p).replaceAll('\\','/'),sha256:sha(bytes)};};
 const reject=async(mutate:(r:any)=>void)=>{const r=structuredClone(original);mutate(r);await assert.rejects(verifyLifecycleFixtureRuntime(await candidate(r)));cases++;};
 try{
  for(const mutate of [(r:any)=>{delete r.raceFixture},(r:any)=>{delete r.c2Fixture},(r:any)=>{r.raceFixture.installed=true},(r:any)=>{r.raceFixture.injectedEngine=false},(r:any)=>{r.raceFixture.revision='other'},(r:any)=>{r.raceFixture.serviceMocked=true},(r:any)=>{r.a3Fixture.runtimeFiles=323},(r:any)=>{r.fixtureNative.private=false},(r:any)=>{r.nativeArtifacts[0].testFlags=false},(r:any)=>{r.fixtureNative.pipePrefix='nt2-race-fixture-20261105'}])await reject(mutate);
  const paths=original.runtimeMappings.filter((m:any)=>m.path.startsWith('mods/noita_together2/files/race/')).map((m:any)=>m.path);assert.equal(paths.length,11);
  for(const path of paths){
   await reject(r=>{const m=r.runtimeMappings.find((m:any)=>m.path===path);m.sha256='0'.repeat(64);r.requiredLocalFiles.find((m:any)=>m.path===path).sha256=m.sha256;});
   await reject(r=>{const m=r.runtimeMappings.find((m:any)=>m.path===path),required=r.requiredLocalFiles.find((m:any)=>m.path===path);m.path=path.replace('/race/','/g3/race_alias/');required.path=m.path;});
  }
  await reject(r=>{delete r.raceFixture;r.compatibility.capabilities=r.compatibility.capabilities.filter((c:string)=>c!=='race.goals.v1');r.profile.capabilities=r.compatibility.capabilities;});
  await verifyLifecycleFixtureRuntime(pin);
  console.log(JSON.stringify({privateRaceClosure:324,exactRaceDependencies:11,negativeCases:cases,normalArtifactInputsChanged:false,dbUsed:false,nativeExecuted:false}));
 }finally{assert.ok(resolve(folder).startsWith(build+sep));await rm(folder,{recursive:true,force:true});}
});
