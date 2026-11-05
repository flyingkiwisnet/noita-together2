import test from 'node:test';import assert from 'node:assert/strict';import {randomUUID,createHash} from 'node:crypto';import {readFile} from 'node:fs/promises';
import {appendNemesisKill,appendNemesisShop,nemesisEarnedPoints,nemesisF64Bits,nemesisF64,nemesisShopClosure,nemesisProvenanceDigest,type NemesisKillEvidence,type NemesisKillOriginal,type NemesisShopOriginal} from '../../../packages/domain/src/g3-nemesis-provenance.js';
import {NEMESIS_CATALOGUE,type NemesisActor} from '../../../packages/domain/src/g3-nemesis.js';
import {storeNemesisProvenance,loadNemesisProvenance} from '../src/g3/nemesis-provenance-storage.js';
const actor=():NemesisActor=>({principalId:randomUUID(),gameSessionId:randomUUID(),attachmentId:randomUUID(),roomId:randomUUID(),runId:randomUUID(),runEpoch:randomUUID(),configRevision:1,worldToken:'世界-😀',name:'literal',live:true,mode:'nemesis',abilitiesEnabled:true,capabilities:['nemesis.abilities'],catalogueRevision:NEMESIS_CATALOGUE.revision});
const kill=(hp=3.5):NemesisKillEvidence=>{const playerEntityId=randomUUID();return {killId:randomUUID(),victimId:randomUUID(),playerEntityId,responsibleEntityId:playerEntityId,responsibleParentId:null,maxHpBits:nemesisF64Bits(hp),nemesisEnemy:false};};
const shop=(x=0,y=1050)=>({groupId:randomUUID(),spawnKind:'eater' as const,xBits:nemesisF64Bits(x),yBits:nemesisF64Bits(y),choices:['grounded','shrooms','confusion'].map(abilityId=>({offerId:randomUUID(),abilityId}))});
test('NP source attribution, missing component and credit-before-NEMESIS_ENEMY preserve binary64 multiply-then-floor',()=>{
 const a=actor(),records:NemesisKillOriginal[]=[];assert.equal(nemesisEarnedPoints(records,a.runId,a.principalId,[]),0);
 for(const [hp,expected] of [[0,0],[0.3,3],[0.29999999999999993,2],[3.5,35],[429496729.5,4294967295]]){const list:NemesisKillOriginal[]=[];assert.equal(appendNemesisKill(a,kill(hp),list).points,expected);}
 const evidence=kill(),child={...evidence,responsibleEntityId:randomUUID(),responsibleParentId:evidence.playerEntityId,nemesisEnemy:true},r=appendNemesisKill(a,child,records);assert.equal(r.attribution,'child');assert.equal(r.points,35);assert.equal(r.enemyBroadcastEligible,false);
 assert.equal(appendNemesisKill(a,{...kill(),responsibleEntityId:randomUUID()},records).points,0);assert.equal(appendNemesisKill(a,{...kill(),maxHpBits:null},records).points,0);
 const before=structuredClone(records);assert.deepEqual(appendNemesisKill(a,child,records),r);assert.deepEqual(records,before);for(const patch of [{maxHpBits:nemesisF64Bits(4)},{nemesisEnemy:false},{victimId:randomUUID()}])assert.throws(()=>appendNemesisKill(a,{...child,...patch},records),/CONFLICT/);
 assert.throws(()=>appendNemesisKill({...a,attachmentId:randomUUID()},child,records),/CONFLICT/);assert.throws(()=>appendNemesisKill({...a,principalId:randomUUID()},child,records),/CONFLICT/);assert.throws(()=>appendNemesisKill(a,{...child,killId:randomUUID()},records),/CONFLICT/);
 assert.equal(nemesisEarnedPoints(records,a.runId,a.principalId,[{cost:35}]),0);assert.throws(()=>nemesisEarnedPoints(records,a.runId,a.principalId,[{cost:36}]),/CAPACITY/);
});
test('exact finite HP/identity/capability/capacity refusals never insert or accept caller NP grants',()=>{
 const a=actor(),records:NemesisKillOriginal[]=[];for(const bits of ['7ff0000000000000','7ff8000000000000','fff0000000000000','8000000000000000','BFF0000000000000','bad','bff0000000000000'])assert.throws(()=>appendNemesisKill(a,{...kill(),maxHpBits:bits},records));
 assert.throws(()=>appendNemesisKill(a,kill(429496730),records),/CAPACITY/);for(const patch of [{points:500},{amount:1},{entityPath:'evil'},{target:a.principalId}])assert.throws(()=>appendNemesisKill(a,{...kill(),...patch},records),/INVALID_MESSAGE/);
 for(const patch of [{live:false},{mode:'core'},{abilitiesEnabled:false},{capabilities:[]}])assert.throws(()=>appendNemesisKill({...a,...patch},kill(),records),/FORBIDDEN/);assert.equal(records.length,0);
 assert.throws(()=>appendNemesisKill({...a,worldToken:'\ud800'},kill(),records),/FORBIDDEN/);
 for(let i=0;i<2048;i++)appendNemesisKill(a,kill(0),records);assert.throws(()=>appendNemesisKill(a,kill(),records),/CAPACITY/);assert.deepEqual(appendNemesisKill(a,records[0]!.evidence,records),records[0]);assert.equal(records.length,2048);
 assert.equal(nemesisF64(nemesisF64Bits(Number.MIN_VALUE)),Number.MIN_VALUE);
});
test('shop provenance derives each shifted slot tier/price and retains per-context shared20/25 pool without inventing RNG',()=>{
 const a=actor(),records:NemesisShopOriginal[]=[],s=shop(1.5,1100),r=appendNemesisShop(a,s,records);assert.deepEqual(r.offers.map(o=>[o.x,o.y,o.level,o.cost]),[[3.5,1097,2,35],[3.5,1072,2,35],[3.5,1047,2,35]]);assert.equal(r.rngVerified,false);
 assert.deepEqual(appendNemesisShop(a,s,records),r);assert.throws(()=>appendNemesisShop(a,{...s,xBits:nemesisF64Bits(2)},records),/CONFLICT/);assert.throws(()=>appendNemesisShop(a,shop(2,1100),records),/CONFLICT/);
 const lower=shop(0,12900);appendNemesisShop(a,lower,records);assert.throws(()=>appendNemesisShop(a,shop(0,10340),records),/CONFLICT/); //20 and25 share tier6.
 const other=actor();assert.equal(appendNemesisShop(other,shop(2,1100),records).offers.length,3);
 const crossing=shop(0,1030);assert.throws(()=>appendNemesisShop(a,crossing,[])); //slot1 level2;slot2 level1 unsupported.
 assert.throws(()=>appendNemesisShop(a,{...s,choices:[...s.choices,{offerId:randomUUID(),abilityId:'trip'}]},[]));
 const injected={...s,price:0};assert.throws(()=>appendNemesisShop(a,injected,[]),/INVALID_MESSAGE/);
});
test('shop radius closure includes distinct nearby groups in same local world; recipient key uses floored coordinates across sources',()=>{
 const a=actor(),records:NemesisShopOriginal[]=[],r=appendNemesisShop(a,shop(1.5,1100),records),near=shop(100,1100);near.choices=['trip','blindness','noeat'].map(abilityId=>({offerId:randomUUID(),abilityId}));
 // Use positive-weight IDs from the actual tier, without guessing catalogue availability.
 const unused=NEMESIS_CATALOGUE.entries.filter(e=>e.depthTierWeights[0]!>0&&!r.offers.some(o=>o.abilityId===e.id));near.choices=unused.slice(0,3).map(e=>({offerId:randomUUID(),abilityId:e.id}));appendNemesisShop(a,near,records);
 const foreign=appendNemesisShop(actor(),shop(1.5,1100),records),closure=nemesisShopClosure(r,r.offers[0]!.offerId,records,randomUUID());assert.equal(closure.offerIds.length,6);assert.equal(closure.coordinateKey,'NEMESIS_USED_ABILITY_3_1097');assert.ok(!closure.offerIds.some(id=>foreign.offers.some(o=>o.offerId===id)));
 assert.equal(nemesisShopClosure(foreign,foreign.offers[0]!.offerId,records,closure.recipientGameSessionId).coordinateKey,closure.coordinateKey);
});
test('canonical provenance UTF8 digest vector and lossless bounded JSONB storage preserve literal context',async()=>{
 const vector=JSON.parse(await readFile(new URL('../../../packages/contracts/g3-nemesis/provenance-internal.json',import.meta.url),'utf8')).killDigestVector;const a={...actor(),...vector.context},records:NemesisKillOriginal[]=[],r=appendNemesisKill(a,vector.evidence,records);
 assert.equal(r.digest,vector.sha256);assert.equal(nemesisProvenanceDigest(JSON.parse(vector.canonicalJson)),vector.sha256);assert.equal(Buffer.from(vector.canonicalJson).toString('hex'),vector.utf8Hex);
 const escaped={...r,source:{...r.source,worldToken:'\u0000'.repeat(80)}};assert.deepEqual(loadNemesisProvenance(storeNemesisProvenance(escaped)),escaped);assert.ok(Buffer.byteLength(JSON.stringify(storeNemesisProvenance(escaped)))<8192);
 const spec=JSON.parse(await readFile(new URL('../../../packages/contracts/g3-nemesis/provenance-internal.json',import.meta.url),'utf8'));for(const p of spec.sourcePins)assert.equal(createHash('sha256').update(await readFile(new URL('../../../../noita-together-main/'+p.file,import.meta.url))).digest('hex'),p.sha256);
});
