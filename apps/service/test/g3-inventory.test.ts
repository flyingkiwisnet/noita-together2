import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {createHash} from 'node:crypto';
import {operationDigest} from '../src/g3/authority.js';
test('G3 retains every reviewed player acceptance case in the single feature inventory',()=>{
 const root=new URL('../../../',import.meta.url),read=(p:string)=>JSON.parse(readFileSync(new URL(p,root),'utf8'));
 const inventory=read('docs/g0/feature-inventory.json'),audit=read('docs/rewrite/legacy-player-experience.json');
 assert.equal(inventory.playerExperienceCases.length,audit.cases.length);
 for(const c of audit.cases){
  const mapped=inventory.playerExperienceCases.find((v:{id:string})=>v.id===c.id);assert.ok(mapped,c.id);
  assert.deepEqual(mapped.featureIds,c.feature_ids);assert.deepEqual(mapped.sourceRefs,c.source_refs);assert.equal(mapped.acceptance,c.rewrite_acceptance);
  for(const id of c.feature_ids){const feature=inventory.features.find((v:{id:string})=>v.id===id);assert.ok(feature,id);assert.ok(feature.g3.sourceAcceptanceCases.includes(c.id));}
 }
 assert.equal(inventory.g3Checkpoint.schemaSha256,createHash('sha256').update(readFileSync(new URL('packages/contracts/g3/schema.json',root))).digest('hex'));
});
test('G3 operation digest matches fixed cross-language canonical UTF-8 bytes',()=>{
 const f=JSON.parse(readFileSync(new URL('../../../packages/contracts/g3/operation-digest.json',import.meta.url),'utf8'));
 assert.equal(Buffer.from(f.canonicalJson).toString('hex'),f.utf8Hex);
 assert.equal(createHash('sha256').update(f.canonicalJson).digest('hex'),f.sha256);
 assert.equal(operationDigest(f.principalId,f.scope,f.direction,f.amount),f.sha256);
 assert.notEqual(operationDigest(f.principalId,{...f.scope,worldToken:'replacement'},f.direction,f.amount),f.sha256);
});
