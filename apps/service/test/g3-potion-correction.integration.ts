import test from 'node:test';import assert from 'node:assert/strict';import {createHash} from 'node:crypto';
import {startG3Fixture,provisionG3Fixture} from '../scripts/g3-fixture.js';
import {nativePotionCorrectionRuntimeResource,nativePotionCorrectionSourceResource} from '../src/g3/native-potion-correction-profile.js';
test('separate disposable correction fixture serves exact resource and frozen fresh room through existing inspected Start/End',async()=>{
 let now=Date.now();const f=await startG3Fixture({potionCorrection:true,live:true,tls:true,now:()=>now});
 try{
  assert.match(f.namespace,/^g3_c2_potion_correction_fx_[0-9a-f]{32}$/);
  const info=JSON.parse((await f.call('/v1/info')).body),social=JSON.parse((await f.call('/v3/social/info')).body),live=JSON.parse((await f.call('/v3/live/info')).body);
  assert.equal(info.serviceId,'nt2-g3-c2-potion-correction');assert.equal(info.profile.id,'nt2-g3-c2-potion-correction-1');assert.deepEqual(info.roomSemantics.requiredCapabilities,['core.seed','core.ghost','core.hud','core.poi']);
  assert.deepEqual(social.runtimeApproval,nativePotionCorrectionRuntimeResource);assert.deepEqual(social.desktopApproval,nativePotionCorrectionSourceResource);assert.equal(live.endpoint,'/v3/live');
  const document=await f.call('/v3/social/runtime');assert.equal(createHash('sha256').update(document.body).digest('hex'),nativePotionCorrectionRuntimeResource.sha256);assert.equal(JSON.parse(document.body).correction.freshRoomRequired,true);
  const s=await provisionG3Fixture(f,{beforeRpc:()=>{now+=100;}});assert.deepEqual(s.room.config!.requiredCapabilities,info.roomSemantics.requiredCapabilities);await s.start();await s.playing();await s.pulse();await s.rpc(s.host,{endRun:{}});
 }finally{await f.close();}
});
