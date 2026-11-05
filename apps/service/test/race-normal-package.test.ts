import test from 'node:test';import assert from 'node:assert/strict';
import {readFile,writeFile,unlink} from 'node:fs/promises';import {createHash,randomUUID} from 'node:crypto';
import {verifyApprovedNormalPackage} from '../src/rejoin/approved-package.js';
const sha=(v:Buffer|string)=>createHash('sha256').update(v).digest('hex');
test('Normal pinned Race approval verifies324 dependencies and refuses partial/aliased/private profiles',async()=>{
 const root=new URL('../../../',import.meta.url),runtime=JSON.parse(await readFile(new URL('deploy/profiles/g3-testing-loadout-runtime.20261030.json',root),'utf8'));
 const prefix='mods/noita_together2/files/race/',files=['catalogue.lua','hook_api.lua','local.lua','panel.lua','player_death.lua','register.lua','sample.lua','sha256.lua','storage.lua','append/boss.lua','append/orb.lua'];
 runtime.runtimeMappings=runtime.runtimeMappings.filter((m:any)=>!m.path.startsWith(prefix));runtime.requiredLocalFiles=runtime.requiredLocalFiles.filter((m:any)=>!m.path.startsWith(prefix));
 for(const file of files){const sourcePath='mod/noita_together2/files/race/'+file,path=prefix+file,sha256=sha(await readFile(new URL(sourcePath,root)));runtime.runtimeMappings.push({path,sourcePath,sha256});runtime.requiredLocalFiles.push({path,sha256});}
 runtime.compatibility.modes=['core','race'];runtime.compatibility.capabilities=[...new Set([...runtime.compatibility.capabilities,'race.goals.v1'])];runtime.profile.capabilities=[...runtime.compatibility.capabilities];assert.equal(runtime.runtimeMappings.length,324);
 const id=randomUUID(),runtimePath=`deploy/profiles/race-normal-verifier-${id}.json`,sourcePath=`deploy/profiles/race-normal-source-verifier-${id}.json`;
 const verify=async(r:any,mutateSource?:(s:any)=>void)=>{
  const bytes=JSON.stringify(r),runtimePin={path:runtimePath,sha256:sha(bytes)},source={sourceApproved:true,developmentOnly:true,installApproved:false,productionApproved:false,packaged:false,actualNoitaEvidence:false,runtimeApproval:runtimePin};mutateSource?.(source);const review=JSON.stringify(source);
  await writeFile(new URL(runtimePath,root),bytes,{flag:'w'});await writeFile(new URL(sourcePath,root),review,{flag:'w'});
  return verifyApprovedNormalPackage({runtime:runtimePin,source:{path:sourcePath,sha256:sha(review)}});
 };
 let negativeCases=0;
 const reject=async(mutate:(r:any)=>void)=>{const r=structuredClone(runtime);mutate(r);await assert.rejects(verify(r));negativeCases++;};
 try{
  const accepted=await verify(runtime);assert.equal(accepted.runtimeMappings.length,324);assert.ok(accepted.compatibility.capabilities.includes('race.goals.v1'));
  for(const mutate of [(r:any)=>{r.compatibility.modes=['core']},(r:any)=>{r.compatibility.modes=['race']},(r:any)=>{r.compatibility.capabilities=r.compatibility.capabilities.filter((c:string)=>c!=='race.goals.v1')},(r:any)=>{r.profile.capabilities=r.profile.capabilities.filter((c:string)=>c!=='race.goals.v1')},(r:any)=>{r.profile.bridgeVersion='other'},(r:any)=>{r.buildRegistry.capabilities=['race.goals.v1']},(r:any)=>{r.nativeArtifacts[0].private=true},(r:any)=>{r.nativeArtifacts[0].testFlags=true},(r:any)=>{r.fixtureOnly=true},(r:any)=>{r.raceFixture={revision:'race-first-release-private-1'}},(r:any)=>{r.a3Fixture={runtimeFiles:324}},(r:any)=>{r.runtimeMappings.pop();r.requiredLocalFiles.pop()}])await reject(mutate);
  for(const file of files)await reject(r=>{const path=prefix+file,m=r.runtimeMappings.find((m:any)=>m.path===path);m.sourcePath=m.sourcePath.replace('/race/','/race_alias/');});
  await reject(r=>{const m=r.runtimeMappings.find((m:any)=>m.path===prefix+'local.lua');m.sha256='0'.repeat(64);});
  await assert.rejects(verify(runtime,s=>{s.runtimeApproval.sha256='0'.repeat(64)}));negativeCases++;
  console.log(JSON.stringify({normalRaceClosure:324,exactRaceDependencies:11,negativeCases,dbUsed:false,nativeExecuted:false}));
 }finally{await Promise.all([unlink(new URL(runtimePath,root)),unlink(new URL(sourcePath,root))]);}
});
