import test from 'node:test';import assert from 'node:assert/strict';import Fastify from 'fastify';
import {LifecycleCapacityDiagnostics} from '../scripts/lc1-lifecycle-capacity-diagnostic.js';import {WorkGate} from '../src/g1/work.js';import type {TransactionDiagnostic} from '../src/g1/database.js';
import {SocialAuthority} from '../src/g3/social.js';import {randomUUID} from 'node:crypto';
import {encode,decode,schema} from '../../../packages/contracts/src/g3-social.js';import {transientEventId} from '../../../packages/contracts/src/g3-social-replay.js';

test('existing diagnostic attributes overlapping gameplay and main-RPC work without SQL or payload capture',async()=>{
 const app=Fastify(),d=new LifecycleCapacityDiagnostics();let started=0,release!:()=>void;const hold=new Promise<void>(r=>{release=r;});
 const db:any={transaction:async(fn:any)=>{const tx={query:async()=>({rows:[]})},query=tx.query;const result=await fn(tx);assert.equal(tx.query,query);db.transactionDiagnostic?.({phase:'commit',category:'committed',code:'COMMIT',commit:'confirmed'});return result;}};const original=db.transaction;d.install({app,db});
 for(const path of ['/v3/rpc','/v1/rpc'])app.post(path,async(req:any)=>db.transaction(async(tx:any)=>{await tx.query('DO_NOT_CAPTURE_SQL',['DO_NOT_CAPTURE_SECRET']);started++;await hold;return{requestId:req.body.requestId};}));
 try{await app.ready();const requests=[app.inject({method:'POST',url:'/v3/rpc',payload:{requestId:'bank',itemBank:{search:'DO_NOT_CAPTURE_SECRET'}}}),app.inject({method:'POST',url:'/v1/rpc',payload:{requestId:'lane',gamePulse:{token:'DO_NOT_CAPTURE_SECRET'}}})];const complete=Promise.all(requests);
  for(let i=0;i<100&&started<2;i++)await new Promise<void>(r=>setImmediate(r));assert.equal(started,2);assert.deepEqual(d.snapshot().active.map(e=>e.operation).sort(),['gamePulse','itemBank']);db.transactionDiagnostic({phase:'admission',category:'service-admission',code:'CAPACITY',commit:'not-attempted'});assert.equal(d.snapshot().firstSaturation.activeHttpTotal,2);assert.deepEqual(d.snapshot().firstSaturation.activeHttp.map((e:any)=>e.operation).sort(),['gamePulse','itemBank']);release();await complete;
  const result=d.snapshot();assert.equal(result.active.length,0);assert.equal(result.operations.length,2);assert.ok(result.operations.every(e=>e.requests===1&&e.workQueryCalls===1&&e.transactionMs>=0));assert.ok(result.recent.every(e=>e.correlated&&e.transactions[0]!.code==='COMMIT'));assert.equal(JSON.stringify(result).includes('DO_NOT_CAPTURE'),false);
 }finally{release();await app.close();assert.equal(db.transaction,original);}
});
test('fixture distinguishes real route8/DB7 guards, correlated domain and HTTP quota without leaking payloads',async()=>{
 const app=Fastify(),db:{transactionDiagnostic?:(e:TransactionDiagnostic)=>void}={},d=new LifecycleCapacityDiagnostics(),original=WorkGate.prototype.enter;let previous=0;db.transactionDiagnostic=()=>previous++;d.install({app,db});
 const route=new WorkGate(8),database=new WorkGate(7),releaseRoute=Array.from({length:8},()=>route.enter()),releaseDb=Array.from({length:7},()=>database.enter());
 app.addHook('preHandler',async(req,reply)=>{if((req.body as any)?.requestId==='quota')await reply.code(429).send({error:{code:'CAPACITY'}});});
 app.post('/v3/lifecycle/rpc',async req=>{const id=(req.body as any).requestId;await Promise.resolve();try{
  if(id==='route')route.enter();else if(id==='database'){try{database.enter();}catch(e){db.transactionDiagnostic!({phase:'admission',category:'service-admission',code:'CAPACITY',commit:'not-attempted'});throw e;}}
  else db.transactionDiagnostic!({phase:'work',category:'service-admission',code:'CAPACITY',commit:'not-attempted'});
 }catch{/* Return the original route-shaped correlated refusal. */}return {requestId:id,error:{code:'CAPACITY'}};});
 try{
  for(const requestId of ['route','database','domain','quota'])await app.inject({method:'POST',url:'/v3/lifecycle/rpc',payload:{requestId,reconcile:{privateBody:'DO_NOT_CAPTURE'},credentials:'DO_NOT_CAPTURE'}});
  const rows=d.snapshot().failures;assert.deepEqual(rows.map(r=>r.guard),['lifecycle-route-work-8','database-work-7','lifecycle-domain','http-quota']);assert.ok(rows.slice(0,3).every(r=>r.correlated));assert.equal(rows[3]!.correlated,false);assert.equal(rows[1]!.gates[0]!.stats.active,7);assert.equal(rows[1]!.transactions[0]!.phase,'admission');assert.equal(previous,2);
  for(let i=0;i<40;i++)await app.inject({method:'POST',url:'/v3/lifecycle/rpc',payload:{requestId:'domain',reconcile:{}}});
  const s=d.snapshot();assert.equal(s.recent.length,32);assert.equal(s.failures.length,16);assert.equal(JSON.stringify(s).includes('DO_NOT_CAPTURE'),false);s.recent[0]!.operation='mutated';assert.notEqual(d.snapshot().recent[0]!.operation,'mutated');
 }finally{releaseRoute.forEach(f=>f());releaseDb.forEach(f=>f());await app.close();assert.equal(WorkGate.prototype.enter,original);}
});

for(const binary of [false,true])test('private social '+(binary?'protobuf':'JSON')+' capacity evidence separates actual kind/read policy guards from route/DB/quota saturation',async()=>{
 const app=Fastify(),db:{transactionDiagnostic?:(e:TransactionDiagnostic)=>void}={},d=new LifecycleCapacityDiagnostics(),enter=WorkGate.prototype.enter,apply=(SocialAuthority.prototype as any).apply;d.install({app,db});
 app.addContentTypeParser('application/x-protobuf',{parseAs:'buffer'},(_req,body,done)=>done(null,body));
 const route=new WorkGate(8),database=new WorkGate(7),releases=[...Array.from({length:8},()=>route.enter()),...Array.from({length:7},()=>database.enter())];
 const scope={roomId:randomUUID(),runId:randomUUID(),runEpoch:randomUUID(),configRevision:1,attachmentId:randomUUID(),worldToken:'DO_NOT_CAPTURE'},sessionId=randomUUID();
 const sender={principalId:randomUUID(),gameSessionId:sessionId,...scope,mode:'core',live:true,displayName:'DO_NOT_CAPTURE',sessionEpoch:randomUUID(),capabilities:['core.emotes']};
 const authority=new SocialAuthority(()=>randomUUID(),['core.emotes'],()=>1000);(authority as any).context=async()=>({sender,live:[sender]});
 const parse=(body:any)=>Buffer.isBuffer(body)?decode('Request',body):body;
 app.addHook('preHandler',async(req,reply)=>{if(parse(req.body).requestId==='quota'){d.trafficGuard('traffic-class-social');await reply.code(429).send({error:{code:'CAPACITY'}});}});
 app.post('/v3/social/rpc',async(req,reply)=>{const q=parse(req.body) as any;try{
  if(q.requestId==='route')route.enter();else if(q.requestId==='database'){try{database.enter();}catch(e){db.transactionDiagnostic!({phase:'admission',category:'service-admission',code:'CAPACITY',commit:'not-attempted'});throw e;}}
  else await authority.execute({query:async(sql:string)=>({rows:sql.includes('g3_social_fences')?[{last_sequence:0,window_at:new Date(1000),window_count:q.requestId==='kind'?4:10}]:[]})} as any,{} as any,q);
 }catch{const result={requestId:q.requestId,error:{code:'CAPACITY'}};return binary?reply.type('application/x-protobuf').send(Buffer.from(encode('Response',result))):result;}throw Error('Expected existing guard rejection');});
 try{
  const eventId=transientEventId(scope,sessionId,'emote',1);
  for(const requestId of ['route','database','kind','read','quota']){const q={major:1,revision:schema.revision,requestId,scope,...(requestId==='read'?{snapshot:{}}:{publish:{eventId,sequence:1,emote:{emoteId:'wave',active:true}}})};await app.inject({method:'POST',url:'/v3/social/rpc',headers:{'content-type':binary?'application/x-protobuf':'application/json'},payload:binary?Buffer.from(encode('Request',q)):q});}
  const rows=d.snapshot().failures;assert.deepEqual(rows.map(r=>r.guard),['social-route-work-8','database-work-7','social-kind-rate','social-read-budget','traffic-class-social']);assert.ok(rows.slice(0,4).every(r=>r.correlated));assert.equal(rows[2]!.eventId,eventId);assert.equal(rows[2]!.sequence,1);assert.equal(rows[2]!.selection,'start');assert.equal(rows[2]!.operation,'emote');assert.equal(rows[3]!.operation,'snapshot');assert.equal(d.snapshot().firstSocialCapacity!.guard,'social-route-work-8');assert.equal(JSON.stringify(d.snapshot()).includes('DO_NOT_CAPTURE'),false);
 }finally{releases.forEach(r=>r());await app.close();assert.equal(WorkGate.prototype.enter,enter);assert.equal((SocialAuthority.prototype as any).apply,apply);}
});
