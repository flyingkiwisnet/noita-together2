import test from 'node:test';import assert from 'node:assert/strict';import {randomUUID} from 'node:crypto';
import {startConnectionCr2Fixture} from '../scripts/connection-cr2-fixture.js';
import {validate,parseJson,encode,decode} from '../../../packages/contracts/src/g2-next.js';
import type {BrowserLoginResponse} from '../../../packages/contracts/g2-next/g2-next.js';

test('CR2-R2 actual browser login and cached retry strictly decode with exact issued-token validity',async()=>{
 let now=Date.now();const {fixture:f}=await startConnectionCr2Fixture({now:()=>now});try{
  const request={subject:'twitch-local-alex',deviceId:'cr2-browser-r2-'+randomUUID(),requestId:randomUUID()};
  const login=async(input:typeof request)=>{
   const result=await f.call('/v1/mock/browser-login',input);assert.equal(result.status,200);
   const value=parseJson('BrowserLoginResponse',result.body) as BrowserLoginResponse;
   validate('BrowserLoginResponse',value);
   const decoded=decode('BrowserLoginResponse',encode('BrowserLoginResponse',value)) as BrowserLoginResponse;
   assert.equal(decoded.credentials!.accessToken,value.credentials!.accessToken);
   assert.deepEqual(decoded.credentialValidity,value.credentialValidity);
   const claims=await f.service.auth.security.verify(value.credentials!.accessToken!,now),sample=value.credentialValidity!;
   assert.equal(sample.credentialId,claims.jti);assert.equal(sample.principalId,value.credentials!.accountId);
   assert.equal(sample.sessionId,value.credentials!.sessionId);assert.equal(sample.sessionEpoch,value.credentials!.sessionEpoch);
   assert.equal(sample.originRequestId,input.requestId);assert.ok(sample.accessRemainingMs!>0&&sample.accessRemainingMs!<=900000);
   return value;
  };
  const first=await login(request);now+=1000;const cached=await login(request);
  assert.deepEqual(cached.credentials,first.credentials);
  assert.equal(cached.credentialValidity!.sampleKey,first.credentialValidity!.sampleKey);
  assert.equal(cached.credentialValidity!.accessRemainingMs,first.credentialValidity!.accessRemainingMs!-1000);
  const fresh=await login({...request,requestId:randomUUID()});
  assert.notEqual(fresh.credentialValidity!.credentialId,first.credentialValidity!.credentialId);
  assert.notEqual(fresh.credentialValidity!.sampleKey,first.credentialValidity!.sampleKey);
  assert.throws(()=>validate('BrowserLoginResponse',{...first,undeclaredValidity:{}}));
  assert.throws(()=>validate('BrowserLoginResponse',{...first,credentialValidity:{...first.credentialValidity,accessRemainingMs:900001}}));
 }finally{await f.close();}
});
