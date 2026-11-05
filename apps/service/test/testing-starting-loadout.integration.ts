import test from 'node:test';import assert from 'node:assert/strict';
import {startLifecycleLc1Fixture} from '../scripts/lifecycle-lc1-fixture.js';
import {testingStartingLoadout as enabled,TESTING_STARTING_LOADOUT as C} from '../../../packages/domain/src/testing-starting-loadout.js';
import {encode,decode} from '../../../packages/contracts/src/g3-lifecycle.js';
test('Explicit development Testing mode is host-only and frozen into the real prepared startup config, immutable for staged/running worlds',async()=>{
 const x=await startLifecycleLc1Fixture({testingStartingLoadout:true,testingMode:true});try{
  const info=JSON.parse((await x.fixture.call('/v1/info')).body);assert.equal(info.developmentFeatures.testingStartingLoadout.available,true);assert.ok(info.compatibility.settingKeys.includes(C.settingKey));
  const prepared=await x.prepareWorld(x.desktop,x.association.worldId,x.proof);assert.equal(enabled(prepared.prepared.config),true);assert.equal(prepared.association.configDigest,x.association.configDigest);const response={major:1,revision:'g3-lifecycle-lc1-2',serviceId:info.serviceId,serviceEpoch:info.serviceEpoch,requestId:'testing-startup-serialization',prepared:prepared.prepared};const decoded=decode('Response',encode('Response',response)) as any;assert.equal(enabled(decoded.prepared.config),true);
  const peer=await x.bootstrapPeer(),off={...x.room.config,settings:[{key:C.settingKey,value:'false'}]};await assert.rejects(x.rpc(peer.desktop,{configure:off}),/FORBIDDEN/);await assert.rejects(x.rpc(x.host,{configure:off}),/INVALID_STATE/);assert.equal(enabled(x.room.config),true);
  await x.send(x.desktop,x.request());const q=x.request(peer.desktop,1,'testing-peer',peer.observation);q.reconcile!.association=peer.association;q.reconcile!.proof=peer.proof;await x.send(peer.desktop,q);await x.start();await assert.rejects(x.rpc(x.host,{configure:off}),/INVALID_STATE/);assert.equal(enabled(x.room.config),true);
  const independent={...x.room.config,settings:[]};await x.rpc(x.host,{roomCreate:{name:'Ordinary independent room',visibility:'public',config:independent}});assert.equal(enabled(x.room.config),false);await x.rpc(x.host,{configure:{...x.room.config,settings:[{key:C.settingKey,value:'true'}]}});assert.equal(enabled(x.room.config),true);
 }finally{await x.close();}
});
test('Disabled development service hides Testing mode and rejects submitted false or true despite host authority',async()=>{
 const x=await startLifecycleLc1Fixture();try{const info=JSON.parse((await x.fixture.call('/v1/info')).body);assert.equal(info.developmentFeatures,undefined);assert.ok(!info.compatibility.settingKeys.includes(C.settingKey));assert.equal(enabled(x.room.config),false);for(const value of C.values)await assert.rejects(x.rpc(x.host,{roomCreate:{name:'Refused test room',config:{...x.room.config,settings:[{key:C.settingKey,value}]}}}),/FORBIDDEN/);}finally{await x.close();}
});
