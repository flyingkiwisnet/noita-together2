import test from 'node:test';import assert from 'node:assert/strict';
import {seedPolicy,effectiveLoot,seedMatches,policyCapabilities} from '../../../packages/domain/src/seed-policy.js';
import {contentFailure} from '../src/g2-next/observation.js';
const profile={modVersion:'fixture',bridgeVersion:'fixture'};
const report={seedStatus:'known',actualSeed:'123',inventoryComplete:true,loaded:true,modVersion:'fixture',bridgeVersion:'fixture'};
test('U1-S-SEED omitted configurations and numeric0 never authorize individual seed or hidden loot',()=>{
 for(const seed of [0,123]){assert.equal(seedPolicy({seed}),'shared-fixed');assert.equal(effectiveLoot({seed}),false);assert.deepEqual(policyCapabilities({seed}),[]);assert.equal(seedMatches({seed},'987'),false);}
 assert.equal(contentFailure(report,{seed:0},profile),'seed_mismatch');assert.equal(contentFailure(report,{seed:123},profile),undefined);
});
test('U1-S-SEED only explicit individual policy permits known distinct seeds and still requires complete loaded version report',()=>{
 const config={seed:0,seedPolicy:'individual-random',randomizeLoot:false};assert.equal(contentFailure(report,config,profile),undefined);
 assert.equal(contentFailure({...report,seedStatus:'unknown'},config,profile),'seed_unknown');assert.equal(contentFailure({...report,inventoryComplete:false},config,profile),'inventory_incomplete');assert.equal(contentFailure({...report,modVersion:'old'},config,profile),'observed_version_mismatch');assert.equal(contentFailure({...report,loaded:false},config,profile),'not_loaded');
 assert.equal(effectiveLoot({...config,randomizeLoot:true}),false);
});
