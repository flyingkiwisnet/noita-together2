// Pure file/PE preflight; no service, SQL, native process or installed files.
import test from 'node:test';import assert from 'node:assert/strict';import {readFile,writeFile,mkdtemp,rm} from 'node:fs/promises';import {createHash} from 'node:crypto';import {resolve,join,relative} from 'node:path';
import {verifyLifecycleFixtureRuntime} from '../src/rejoin/lifecycle-fixture-profile.js';
const sha=(b:Buffer|string)=>createHash('sha256').update(b).digest('hex');
test('Private item fixture permits only exact current canonical Lua mapping and rejects substitution',async()=>{
 const pin=JSON.parse(await readFile(process.env.NT2_ITEM_FIXTURE_PIN??'spikes/noita-entry/.build/bank-events-client-20261026-01/fixture-profile-9793a87878e144d591c28174f7a01304/pin.json','utf8')),original=JSON.parse(await readFile(pin.path,'utf8'));
 // Verify the raw Client-produced pin first, including catalogue/social metadata.
 // No field or source pin is repaired before positive acceptance.
 const verified=await verifyLifecycleFixtureRuntime(pin);assert.equal(verified.itemFixture.revision,'items-private-1');
 const folder=await mkdtemp(join(resolve('spikes/noita-entry/.build'),'items-verifier-'));let n=0;
 const candidate=async(value:any)=>{const path=join(folder,++n+'.json'),bytes=JSON.stringify(value);await writeFile(path,bytes);return {path:relative(process.cwd(),path).replaceAll('\\','/'),sha256:sha(bytes)};};
 try{
  for(const mutate of [(m:any)=>{m.sourcePath='packages/contracts/g3-items/other.lua';},(m:any)=>{m.path='mods/noita_together2/files/g3/other_catalogue.lua';},(m:any)=>{m.sha256='0'.repeat(64);}]){const bad=structuredClone(original),m=bad.runtimeMappings.find((m:any)=>m.path==='mods/noita_together2/files/g3/item_catalogue_data.lua');mutate(m);await assert.rejects(verifyLifecycleFixtureRuntime(await candidate(bad)));}
  for(const mutate of [(r:any)=>{r.itemCatalogue.sha256=null;},(r:any)=>{r.socialRevision='g3-social-draft-3';},(r:any)=>{r.gameplayRevision='g3-draft-3';},(r:any)=>{r.schemaSha256='0'.repeat(64);}]){const bad=structuredClone(original);mutate(bad);await assert.rejects(verifyLifecycleFixtureRuntime(await candidate(bad)));}
 }finally{await rm(folder,{recursive:true,force:true});}
});
