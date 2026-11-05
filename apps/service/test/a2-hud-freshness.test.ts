// Superseding retained-state policy; HUD sample age is no longer display validity.
import test from 'node:test';import assert from 'node:assert/strict';
import {mergeRetainedHud,type RetainedHud} from '../../../packages/domain/src/g3-social.js';
import {encode,decode,schema} from '../../../packages/contracts/src/g3-social.js';
const missing:RetainedHud={hpMilli:0,maxHpMilli:0,sampo:false,hpKnown:false,sampoKnown:false};
test('retained facts merge missing, named, explicitly unnamed and known zero observations',()=>{
 assert.deepEqual(mergeRetainedHud(undefined,missing),missing);
 const valid={...missing,hpKnown:true,hpMilli:800,maxHpMilli:1000,sampoKnown:true,sampo:true,biome:'$biome_mountain'};
 assert.deepEqual(mergeRetainedHud(valid,missing),valid);
 const unnamed=mergeRetainedHud(valid,{...missing,biome:'_EMPTY_'});assert.equal(unnamed.biome,'_EMPTY_');assert.equal(unnamed.hpMilli,800);
 assert.equal(mergeRetainedHud(unnamed,missing).biome,'_EMPTY_');assert.equal(mergeRetainedHud(unnamed,{...missing,biome:'$biome_coalmine'}).biome,'$biome_coalmine');
 const zero=mergeRetainedHud(valid,{...missing,hpKnown:true,maxHpMilli:1000,hpMilli:0,sampoKnown:true,sampo:false});assert.equal(zero.hpMilli,0);assert.equal(zero.sampo,false);
});
test('draft3 preserves retained HUD revision and forbids transient expiry/event/sequence',()=>{
 assert.equal(schema.revision,'g3-social-draft-3');
 const p={hudStateRevision:42,capturedAt:'1970-01-01T00:00:00.000Z',hud:{hpKnown:true,hpMilli:0,maxHpMilli:1000,biome:'_EMPTY_'}};
 assert.equal(decode('Projection',encode('Projection',p)).hudStateRevision,42);
 for(const bad of [{...p,hudStateRevision:0},{...p,hudStateRevision:undefined},{...p,expiresAt:'2099-01-01T00:00:00.000Z'},{...p,sequence:1},{...p,eventId:'x'},{...p,hudFreshForMs:2000}])assert.throws(()=>encode('Projection',bad));
 assert.doesNotThrow(()=>encode('Projection',{presentationRevision:1,emote:{emoteId:'wave'}}));
 assert.throws(()=>encode('Projection',{emote:{emoteId:'wave'},hudStateRevision:1}));
});
