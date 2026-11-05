// File/PE preflight only: no service, native process, DB or normal profile write.
import test from 'node:test';import assert from 'node:assert/strict';import {readFile,writeFile,mkdtemp,rm} from 'node:fs/promises';import {createHash} from 'node:crypto';import {resolve,join,relative,sep} from 'node:path';
import {verifyLifecycleFixtureRuntime} from '../src/rejoin/lifecycle-fixture-profile.js';
const pinFile='spikes/noita-entry/.build/a2-client-20261021-04/fixture-profile-f81490491e7d4fe69e4669f4b55debdb/pin.json';
test('reviewed66-file private A2 input verifies, legacy50 path remains, and changed/unknown additions fail closed',async()=>{
 const pin=JSON.parse(await readFile(pinFile,'utf8')),original=JSON.parse(await readFile(pin.path,'utf8')),verified=await verifyLifecycleFixtureRuntime(pin);assert.equal(verified.runtimeMappings.length,66);assert.equal(verified.fixtureNative.private,true);assert.equal(verified.installApproved,false);
 await assert.rejects(verifyLifecycleFixtureRuntime({...pin,sha256:'0'.repeat(64)}),/changed/);
 const build=resolve('spikes/noita-entry/.build'),folder=await mkdtemp(join(build,'lc1-a2-verifier-'));assert.ok(folder.startsWith(build+sep));let number=0;
 const candidate=async(value:any)=>{const path=join(folder,String(++number)+'.json'),bytes=JSON.stringify(value);await writeFile(path,bytes);return {path:relative(process.cwd(),path).replaceAll('\\','/'),sha256:createHash('sha256').update(bytes).digest('hex')};};
 try{
  const normal=JSON.parse(await readFile('deploy/profiles/g3-lc1-recovery-runtime.20261020.json','utf8')),legacy=structuredClone(original),basePaths=new Set(normal.runtimeMappings.map((m:any)=>m.path));delete legacy.a2Fixture;delete legacy.fixtureNative;legacy.nativeArtifacts=normal.nativeArtifacts;legacy.runtimeMappings=legacy.runtimeMappings.filter((m:any)=>basePaths.has(m.path)).map((m:any)=>m.path.endsWith('.dll')?normal.runtimeMappings.find((n:any)=>n.path===m.path):m);legacy.requiredLocalFiles=legacy.runtimeMappings.map((m:any)=>({path:m.path,sha256:m.sha256}));legacy.buildRegistry.bridgeSha256=normal.buildRegistry.bridgeSha256;legacy.profile.bridgeSha256=normal.profile.bridgeSha256;
  assert.equal((await verifyLifecycleFixtureRuntime(await candidate(legacy))).runtimeMappings.length,50);
  const bads:((r:any)=>void)[]=[
   r=>{r.fixtureOnly=false;},r=>{r.installApproved=true;},r=>{r.fixtureNative.private=false;},r=>{r.a2Fixture.installed=true;},r=>{r.a2Fixture.serviceMocked=true;},
   r=>{r.fixtureNative.pipePrefix='\\\\.\\pipe\\nt2-a2-other-';},r=>{r.fixtureNative.testAppPath='C:/Windows/other.exe';},
   r=>{r.a2Fixture.addedFiles[0]='unknown.lua';},r=>{r.requiredLocalFiles.pop();},
   r=>{const m=r.runtimeMappings.find((m:any)=>m.path.endsWith('/a2_text.lua'));m.sha256='0'.repeat(64);},
   r=>{const m=r.runtimeMappings.find((m:any)=>m.path.endsWith('/a2_text.lua'));const old=m.path;m.path='mods/noita_together2/files/g3/unknown.lua';r.requiredLocalFiles.find((p:any)=>p.path===old).path=m.path;},
   r=>{const m=r.runtimeMappings.find((m:any)=>m.path.endsWith('/a2_text.lua')),other=r.runtimeMappings.find((m:any)=>m.path.endsWith('/a2_capture.lua'));m.sourcePath=other.sourcePath;m.sha256=other.sha256;r.requiredLocalFiles.find((p:any)=>p.path===m.path).sha256=m.sha256;},
   r=>{const m=r.runtimeMappings.find((m:any)=>m.path.endsWith('/settings.lua')),old=m.path;m.path='mods/noita_together2/files/g3/unknown-base.lua';r.requiredLocalFiles.find((p:any)=>p.path===old).path=m.path;},
   r=>{const m=r.runtimeMappings.find((m:any)=>m.path.endsWith('/a2_text.lua'));m.sourcePath='../outside.lua';}
  ];
  for(const change of bads){const value=structuredClone(original);change(value);await assert.rejects(verifyLifecycleFixtureRuntime(await candidate(value)));}
  const noSelector=structuredClone(original);delete noSelector.a2Fixture;await assert.rejects(verifyLifecycleFixtureRuntime(await candidate(noSelector)),/current50-file closure/);
  console.log(JSON.stringify({reviewedA2Files:66,baseFiles:50,explicitAdditions:16,legacy50Verified:true,negativeCases:bads.length+2,databaseTouched:false}));
 }finally{assert.ok(resolve(folder).startsWith(build+sep));await rm(folder,{recursive:true,force:true});}
});
