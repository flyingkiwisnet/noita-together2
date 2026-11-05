import test from 'node:test';import assert from 'node:assert/strict';import {randomUUID} from 'node:crypto';import {SocialRatePrecondition} from '../scripts/lc1-social-rate-precondition.js';
test('synthetic rate precondition is bounded, explicit, privately scoped and cancellable',()=>{
 const service={db:{namespace:'g3_rejoin_'+'a'.repeat(32)}},p={target:'consumer',worldId:randomUUID(),activeEventId:randomUUID()},x=new SocialRatePrecondition(service,()=>true);
 for(const bad of [null,[],{...p,target:'all'},{...p,activeEventId:undefined},{...p,worldId:'wrong'},{...p,count:9}])assert.throws(()=>x.arm(bad),/FIXTURE_SOCIAL_RATE_ARGUMENT/);
 for(let i=0;i<8;i++){x.arm(p);assert.throws(()=>x.arm(p),/FIXTURE_SOCIAL_RATE_BUSY/);x.release();}assert.equal(x.status().armsUsed,8);assert.equal(x.status().active,null);assert.throws(()=>x.arm(p),/FIXTURE_SOCIAL_RATE_BUSY/);x.close();
 const closed=new SocialRatePrecondition(service,()=>true);closed.close();assert.throws(()=>closed.arm(p),/FIXTURE_SOCIAL_RATE_BUSY/);assert.throws(()=>new SocialRatePrecondition({db:{namespace:'live'}},()=>true).arm(p),/FIXTURE_SOCIAL_RATE_ARGUMENT/);
});
