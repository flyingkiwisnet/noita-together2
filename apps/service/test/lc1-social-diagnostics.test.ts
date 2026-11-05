import test from 'node:test';import assert from 'node:assert/strict';import Fastify from 'fastify';
import {SocialDiagnostics} from '../scripts/lifecycle-lc1-social-diagnostics.js';
import type {TransactionDiagnostic} from '../src/g1/database.js';
test('fixture social diagnostics correlate bounded transaction failures without payloads',async()=>{
 const app=Fastify(),db:{transactionDiagnostic?:(e:TransactionDiagnostic)=>void}={};const d=new SocialDiagnostics();let previous=0;db.transactionDiagnostic=()=>{previous++;};
 d.install({app,db} as any);
 app.post('/v3/social/rpc',async req=>{await Promise.resolve();db.transactionDiagnostic!({phase:'work',category:'deadline',code:'SERVICE_UNAVAILABLE',sqlState:'57014',commit:'not-attempted'});return {error:{code:'SERVICE_UNAVAILABLE'}};});
 try{
  for(let i=0;i<70;i++)await app.inject({method:'POST',url:'/v3/social/rpc',payload:{requestId:'probe-'+i,roster:{},secret:'DO_NOT_CAPTURE',text:'DO_NOT_CAPTURE'}});
  const s=d.snapshot();assert.equal(previous,70);assert.equal(s.recent.length,64);assert.equal(s.failures.length,16);assert.equal(s.totals.roster,70);assert.ok(s.peakRequestsPerSecond>0);
  const last=s.recent.at(-1)!;assert.equal(last.requestId,'probe-69');assert.equal(last.responseCode,'SERVICE_UNAVAILABLE');assert.deepEqual(last.transactions,[{phase:'work',category:'deadline',code:'SERVICE_UNAVAILABLE',sqlState:'57014',commit:'not-attempted'}]);
  assert.ok(last.elapsedMs>=0);assert.equal(JSON.stringify(s).includes('DO_NOT_CAPTURE'),false);s.recent[0]!.operation='tampered';assert.equal(d.snapshot().recent[0]!.operation,'roster');
 }finally{await app.close();}
});
