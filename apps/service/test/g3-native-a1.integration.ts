import test from 'node:test';import assert from 'node:assert/strict';import {randomUUID,createHash} from 'node:crypto';import {readFile} from 'node:fs/promises';
import {startG3Fixture,provisionG3Fixture} from '../scripts/g3-fixture.js';
import {verifyNativeA1DevelopmentArtifacts} from '../src/g3/native-a1-profile.js';import {createG3CandidateApp} from '../src/g3/candidate.js';
for(const social of [false,true])test('A1 '+(social?'social':'primary')+' exact TLS metadata/Start/playing/movement/End/restart and runtime drift',async()=>{
 let target='',reached=false;const f=await startG3Fixture({tls:true,a1Runtime:true,...(social?{nativeSocial:true}:{native:true}),artifactReader:async url=>{if(target&&url.pathname.endsWith('/'+target)){reached=true;return Buffer.from('changed');}return readFile(url);}});
 try{
  const info=JSON.parse((await f.call('/v1/info')).body);assert.deepEqual(info.profile,f.profile.buildRegistry);
  if(social){const r=await f.call('/v3/social/runtime');assert.equal(r.headers['cache-control'],'no-store');assert.equal(createHash('sha256').update(r.body).digest('hex'),f.profile.runtimeApproval.sha256);const i=JSON.parse((await f.call('/v3/social/info')).body);assert.deepEqual(i.runtimeProfile,f.profile.profile);assert.equal(i.installApproved,false);}
  const s=await provisionG3Fixture(f),call=async(i:number,payload:object)=>{const c=s.games[i]!;const r=await f.call('/v3/rpc',{major:1,revision:'g3-draft-1',serviceId:f.service.db.serviceId,serviceEpoch:f.service.db.epoch,sessionEpoch:c.sessionEpoch,requestId:randomUUID(),scope:s.scopes[i],...payload},c.accessToken);assert.equal(r.status,200);return JSON.parse(r.body);};
  const move={movement:{frames:[{sequence:1,xMilli:'1000',yMilli:'2000',vxMilli:'0',vyMilli:'0',facingLeft:false}]}};
  assert.ok((await call(0,move)).error);await s.start();assert.ok((await call(0,move)).error);await s.playing();assert.equal((await call(0,move)).error,undefined);assert.equal((await call(1,{ghostList:{}})).ghosts.length,1);assert.deepEqual((await call(0,{ghostList:{}})).ghosts,[]);
  assert.equal((await call(0,{goldIntent:{operationId:randomUUID(),direction:'deposit',amount:1}})).error.code,'FORBIDDEN');assert.equal((await f.call('/v3/nemesis/info')).status,404);
  await s.rpc(s.host,{endRun:{}});assert.ok((await call(1,{ghostList:{}})).error);await f.restart();assert.deepEqual(JSON.parse((await f.call('/v1/info')).body).profile,info.profile);assert.ok((await call(0,move)).error);
  target=f.profile.runtimeMappings.find((p:any)=>p.path.endsWith('/native_owner.lua')).sourcePath;await assert.rejects(f.restart(),/changed/);assert.equal(reached,true);
 }finally{await f.close();}
});
test('A1 selector refuses old profile namespace, wrong identity/caps and missing TLS before database work',async()=>{
 const m=await verifyNativeA1DevelopmentArtifacts();const base:any={a1Runtime:true,native:true,databaseUrl:'invalid-database-not-reached',namespace:'g3_a1_unreached',serviceId:m.serviceId,mockEnabled:true,compatibility:m.compatibility,keys:{},tls:{pfx:Buffer.alloc(0),passphrase:''}};
 for(const patch of [{namespace:'g3_fixture_old'},{serviceId:'nt2-g3-native-development'},{tls:undefined},{native:false},{profile:m.profile},{compatibility:{...m.compatibility,capabilities:[...m.compatibility.capabilities,'core.gold']}}])await assert.rejects(createG3CandidateApp({...base,...patch}),/FORBIDDEN/);
});
