// Read-only metadata/PE validation. No service, SQL or native execution.
import test from 'node:test';import assert from 'node:assert/strict';
import {readFile,writeFile,mkdtemp,rm} from 'node:fs/promises';import {createHash} from 'node:crypto';import {resolve,join,relative} from 'node:path';
import {verifyLifecycleFixtureRuntime} from '../src/rejoin/lifecycle-fixture-profile.js';
const sha=(b:Buffer|string)=>createHash('sha256').update(b).digest('hex');
test('Explicit private C1 fixture admits exact canonical catalogue and owned subtree; substitutions fail closed',{skip:!process.env.NT2_C1_FIXTURE_PIN},async()=>{
 const pin=JSON.parse(await readFile(process.env.NT2_C1_FIXTURE_PIN!,'utf8')),original=JSON.parse(await readFile(pin.path,'utf8'));
 // Raw immutable Client input must pass first; never repair source/native pins.
 const verified=await verifyLifecycleFixtureRuntime(pin);assert.equal(verified.c1Fixture.revision,'c1-player-private-1');assert.equal(verified.c1Fixture.installed,false);assert.equal(verified.runtimeMappings.length,original.a3Fixture.runtimeFiles);
 const json=await readFile('packages/contracts/g3-c1/catalogue.json'),catalogue=JSON.parse(json.toString());assert.deepEqual(verified.c1Catalogue,{revision:catalogue.revision,sha256:sha(json)});
 const folder=await mkdtemp(join(resolve('spikes/noita-entry/.build'),'c1-verifier-'));let count=0;
 const candidate=async(value:any)=>{const path=join(folder,++count+'.json'),bytes=JSON.stringify(value);await writeFile(path,bytes);return {path:relative(process.cwd(),path).replaceAll('\\','/'),sha256:sha(bytes)};};
 const catalogueMapping=(r:any)=>r.runtimeMappings.find((m:any)=>m.path==='mods/noita_together2/files/g3/c1_catalogue_data.lua');
 const cases:[string,(r:any)=>void][]=[
  ['selector missing',r=>{delete r.c1Fixture;}],['selector revision',r=>{r.c1Fixture.revision='other';}],['installed selector',r=>{r.c1Fixture.installed=true;}],
  ['mocked service',r=>{r.c1Fixture.serviceMocked=true;}],['normal profile mutation',r=>{r.c1Fixture.normalProfilesChanged=true;}],['missing catalogue',r=>{delete r.c1Catalogue;}],
  ['catalogue revision',r=>{r.c1Catalogue.revision='other';}],['catalogue JSON hash',r=>{r.c1Catalogue.sha256='0'.repeat(64);}],
  ['Lua source substitution',r=>{catalogueMapping(r).sourcePath='packages/contracts/g3-items/catalogue.lua';}],['Lua destination substitution',r=>{catalogueMapping(r).path='mods/noita_together2/files/g3/other_c1_data.lua';}],['Lua byte hash',r=>{catalogueMapping(r).sha256='0'.repeat(64);}],
  ['owned subtree destination',r=>{const m=r.runtimeMappings.find((m:any)=>m.path.startsWith('mods/noita_together2/files/c1/')),required=r.requiredLocalFiles.find((p:any)=>p.path===m.path);m.path='mods/noita_together2/files/not_c1/fake.lua';required.path=m.path;}],
  ['selectorless subtree',r=>{delete r.c1Fixture;delete r.c1Catalogue;const m=catalogueMapping(r);r.runtimeMappings=r.runtimeMappings.filter((p:any)=>p!==m);r.requiredLocalFiles=r.requiredLocalFiles.filter((p:any)=>p.path!==m.path);r.a3Fixture.runtimeFiles=r.runtimeMappings.length;}],
 ];
 try{
  for(const [label,mutate]of cases){const bad=structuredClone(original);mutate(bad);await assert.rejects(verifyLifecycleFixtureRuntime(await candidate(bad)),(error:unknown)=>error instanceof Error,label);}
  // Current C1 entry may require these exact support paths; the allowance is
  // not a general rejoin/startup/catalogue mapping permission.
  for(const destination of ['mods/noita_together2/files/g3/d1_hook_api.lua','mods/noita_together2/files/g3/d1_local.lua','mods/noita_together2/files/c1/append/steve_death.lua','mods/noita_together2/files/c2/storage.lua','mods/noita_together2/files/rejoin/lobby_status.lua','mods/noita_together2/files/startup/testing_loadout.lua','mods/noita_together2/files/startup/testing-starting-loadout.json']){
   if(!original.runtimeMappings.some((m:any)=>m.path===destination))continue;
   const corrupt=structuredClone(original),cm=corrupt.runtimeMappings.find((m:any)=>m.path===destination);cm.sha256='0'.repeat(64);corrupt.requiredLocalFiles.find((p:any)=>p.path===destination).sha256=cm.sha256;await assert.rejects(verifyLifecycleFixtureRuntime(await candidate(corrupt)),/changed/);
   const alias=structuredClone(original),am=alias.runtimeMappings.find((m:any)=>m.path===destination),required=alias.requiredLocalFiles.find((p:any)=>p.path===destination);am.path=destination+'.alias';required.path=am.path;await assert.rejects(verifyLifecycleFixtureRuntime(await candidate(alias)),/owned unique mapping|unknown A2\/A3 file/);
   const sourceAlias=structuredClone(original),sm=sourceAlias.runtimeMappings.find((m:any)=>m.path===destination);sm.sourcePath='mod/noita_together2/files/startup/settings_scope.lua';sm.sha256=sha(await readFile(sm.sourcePath));sourceAlias.requiredLocalFiles.find((p:any)=>p.path===destination).sha256=sm.sha256;await assert.rejects(verifyLifecycleFixtureRuntime(await candidate(sourceAlias)),/owned unique mapping|unknown A2\/A3 file/);
  }
  // Matching real bytes cannot authorize a source/destination alias.
  const alias=structuredClone(original),m=alias.runtimeMappings.find((m:any)=>m.path.startsWith('mods/noita_together2/files/c1/'));m.sourcePath='mod/noita_together2/files/g3/c1_gameplay.lua';m.sha256=sha(await readFile(m.sourcePath));alias.requiredLocalFiles.find((p:any)=>p.path===m.path).sha256=m.sha256;await assert.rejects(verifyLifecycleFixtureRuntime(await candidate(alias)),/unknown A2\/A3 file/);
 }finally{await rm(folder,{recursive:true,force:true});}
});
