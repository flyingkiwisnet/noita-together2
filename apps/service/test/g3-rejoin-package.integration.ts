import test from 'node:test';import assert from 'node:assert/strict';import {createHash} from 'node:crypto';
import {verifyRejoinPackage} from '../src/rejoin/package-profile.js';import {startG3Fixture,provisionG3Fixture} from '../scripts/g3-loot01-fixture.js';
test('RJ3 final resource discovery and retained participant recover with DB-stored same-issuer identity',async()=>{
 const r=await verifyRejoinPackage(),f=await startG3Fixture({rejoin:true,rejoinPackage:true,loot01Runtime:true,persistentIdentity:true,live:true,tls:true});try{
  const hash=(s:string)=>createHash('sha256').update(s).digest('hex');assert.equal(hash((await f.call('/v3/social/runtime')).body),r.runtimeApproval.sha256);const info=JSON.parse((await f.call('/v3/social/info')).body);assert.deepEqual(info.runtimeApproval,r.runtimeApproval);
  const s=await provisionG3Fixture(f);await s.start();await s.playing();const auth=s.desktops[0]!,base={major:1,revision:'g3-rejoin-1',serviceId:f.service.db.serviceId,serviceEpoch:f.service.db.epoch,sessionEpoch:auth.sessionEpoch,requestId:'package-before',status:{}};
  const before=JSON.parse((await f.call('/v3/rejoin/rpc',base,auth.accessToken)).body);await f.restart();const after=JSON.parse((await f.call('/v3/rejoin/rpc',{...base,serviceEpoch:f.service.db.epoch,requestId:'package-after'},auth.accessToken)).body);assert.equal(hash(before.resumeProof),hash(after.resumeProof));assert.equal(after.participant.returnEligible,true);assert.equal(after.participant.state,'reconnecting');assert.equal(f.profile.id,r.id);
 }finally{await f.close();}
});
