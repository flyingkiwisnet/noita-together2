import test from 'node:test';
import assert from 'node:assert/strict';
import {randomUUID} from 'node:crypto';
import {startConnectionCr2Fixture} from '../scripts/connection-cr2-fixture.js';
import {encode,decode} from '../../../packages/contracts/src/g3-connection.js';
import type {Response} from '../../../packages/contracts/g3-connection/g3-connection.js';

for(const binary of [false,true])test(`CR2 explicit participant admission preserves current-owner scope (${binary?'protobuf':'JSON'})`,async()=>{
 const now=Date.now(),{revision,fixture:f,scenario:s}=await startConnectionCr2Fixture({now:()=>now});
 try{
  await s.start();await s.playing();
  const own=(await s.rpc(s.games[0],{roomGet:{}})).canonicalAdmission!;
  const foreign=(await s.rpc(s.games[1],{roomGet:{}})).canonicalAdmission!;
  assert.ok(own.participantId);assert.ok(foreign.participantId);assert.notEqual(own.participantId,foreign.participantId);
  const base={major:1,revision,serviceId:f.service.db.serviceId,serviceEpoch:f.service.db.epoch,sessionEpoch:s.games[0]!.sessionEpoch,requestId:randomUUID()};
  const call=async(participantId?:string):Promise<Response>=>{
   const input={...base,admission:{roomId:s.room.roomId!,...(participantId?{participantId}:{})}};
   if(!binary)return JSON.parse((await f.call('/v3/connection/rpc',input,s.games[0]!.accessToken)).body) as Response;
   const response=await f.service.app.inject({method:'POST',url:'/v3/connection/rpc',headers:{host:new URL(f.origin).host,'content-type':'application/x-protobuf',authorization:'Bearer '+s.games[0]!.accessToken},payload:Buffer.from(encode('Request',input))});
   assert.equal(response.statusCode,200);assert.match(response.headers['content-type'] as string,/application\/x-protobuf/);
   return decode('Response',response.rawPayload) as Response;
  };
  const implicit=await call();assert.equal(implicit.error,undefined);assert.equal(implicit.admission!.participantId,own.participantId);
  const explicit=await call(own.participantId!);assert.equal(explicit.error,undefined);assert.deepEqual(explicit.admission,implicit.admission);
  for(const wrongId of [foreign.participantId!,randomUUID()]){
   const denied=await call(wrongId);assert.equal(denied.error!.code,'FORBIDDEN');assert.equal(denied.admission,undefined);
  }
  assert.equal((await s.rpc(s.games[0],{roomGet:{}})).canonicalAdmission!.decision!.playing,true);
  assert.equal((await s.rpc(s.games[1],{roomGet:{}})).canonicalAdmission!.decision!.playing,true);
 }finally{await f.close();}
});
