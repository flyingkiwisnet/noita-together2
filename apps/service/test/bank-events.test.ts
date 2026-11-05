// In-memory transaction/socket seams; SQL and actual component evidence are separate.
import test from 'node:test';import assert from 'node:assert/strict';import {randomUUID} from 'node:crypto';
import {afterCommit,publishCommitEffects,discardCommitEffects,commitEffectsCheckpoint} from '../src/g1/commit-hooks.js';
import {markBankDirty,prepareBankEvents,CommittedBankReads} from '../src/g3/bank-events.js';
import {executeBankBatch} from '../src/g3/bank-batch.js';import {PolicyError} from '../src/g1/security.js';
import {LiveService} from '../src/g3/live-service.js';import {liveScopeDigest,LIVE_POLICY} from '../../../packages/domain/src/g3-live.js';
import {decode,encode,schema as liveSchema} from '../../../packages/contracts/src/g3-live.js';
import {schema as gameplay,validate} from '../../../packages/contracts/src/g3.js';
test('commit-only versions and cache entries; discard and savepoint rollback clear reused-client bookkeeping',async()=>{
 let queries=0;const tx:any={query:async(_sql:string,args:any[])=>{queries++;return{rows:args[0].map((id:string)=>({run_id:id,revision:'5'}))};}},events:any[]=[];
 const a=randomUUID(),b=randomUUID();markBankDirty(tx,a);await prepareBankEvents(tx,v=>events.push(v));assert.equal(events.length,0);discardCommitEffects(tx);
 await prepareBankEvents(tx,v=>events.push(v));assert.equal(queries,1,'rollback bookkeeping cannot leak into pool-client reuse');
 markBankDirty(tx,a);const rollback=commitEffectsCheckpoint(tx);markBankDirty(tx,b);afterCommit(tx,()=>events.push('bad'),()=>{});rollback();await prepareBankEvents(tx,v=>events.push(v));publishCommitEffects(tx);assert.deepEqual(events,[[{runId:a,revision:'5'}]]);
 const cache=new CommittedBankReads();cache.put(tx,'epoch/run/rev1/query',{rows:[{itemId:a}]},100);assert.equal(cache.get('epoch/run/rev1/query',100),undefined);discardCommitEffects(tx);assert.equal(cache.get('epoch/run/rev1/query',100),undefined);
 cache.put(tx,'epoch/run/rev1/query',{rows:[{itemId:a}]},100);publishCommitEffects(tx);assert.deepEqual(cache.get('epoch/run/rev1/query',100),{rows:[{itemId:a}]});assert.equal(cache.get('epoch/run/rev2/query',100),undefined);assert.equal(cache.get('epoch/run/rev1/other-query',100),undefined);assert.equal(cache.get('epoch/run/rev1/query',30100),undefined);
});
test('batch savepoints preserve siblings and remove commit effects/local callbacks on refusal and response overflow',async()=>{
 const ids=Array.from({length:4},()=>randomUUID()),published:string[]=[],effects:(()=>void)[]=[],sql:string[]=[],scope={roomId:randomUUID(),runId:randomUUID(),runEpoch:randomUUID(),configRevision:1,attachmentId:randomUUID(),worldToken:randomUUID()};
 const tx:any={query:async(s:string,args?:any[])=>{sql.push(s);return{rows:s.startsWith('SELECT run_id,revision')?args![0].map((id:string)=>({run_id:id,revision:'1'})):[]};}};
 const authority:any={execute:async(_tx:any,_actor:any,q:any)=>{const id=q.itemStatus.operationId;markBankDirty(tx,id);afterCommit(tx,()=>published.push(id),()=>{});effects.push(()=>published.push('local:'+id));if(id===ids[3])throw new PolicyError('CLAIM_LOST');return{itemOperation:{operationId:id,...(id===ids[0]||id===ids[1]?{item:{object:{objectId:'x'.repeat(131072)}}}:{})}};}};
 const result=await executeBankBatch(authority,tx,{}as any,{scope,bankBatch:{entries:ids.map(operationId=>({scope,itemStatus:{operationId}}))}},effects);
 assert.deepEqual(result.bankBatch!.entries!.map(e=>e.error?.code??'ok'),['ok','BATCH_TOO_LARGE','ok','CLAIM_LOST']);assert.deepEqual(result.bankBatch!.entries!.map(e=>e.operationId),ids);assert.equal(sql.filter(s=>s==='ROLLBACK TO SAVEPOINT bank_entry').length,2);
 const notifications:any[]=[];await prepareBankEvents(tx,v=>notifications.push(v));publishCommitEffects(tx);for(const f of effects)f();assert.deepEqual(published,[ids[0],ids[2],'local:'+ids[0],'local:'+ids[2]]);assert.deepEqual(notifications[0].map((e:any)=>e.runId),[ids[0],ids[2]]);
});
test('canonical batch limits and bank invalidation payload round-trip',()=>{
 const q={major:1,revision:gameplay.revision,bankBatch:{entries:[{scope:{runId:randomUUID()},itemStatus:{operationId:randomUUID()}}]}};validate('Request',q);assert.throws(()=>validate('Request',{...q,bankBatch:{entries:Array(5).fill(q.bankBatch.entries[0])}}));
 const frame={major:1,revision:liveSchema.revision,serviceId:'test',serviceEpoch:randomUUID(),connectionId:randomUUID(),clientGeneration:1,authorityGeneration:1,sequence:1,scope:{principalId:randomUUID(),sessionId:randomUUID(),sessionEpoch:randomUUID(),roomId:randomUUID(),runId:randomUUID(),runEpoch:randomUUID(),configRevision:1,attachmentId:randomUUID(),worldToken:randomUUID()},bankChanged:{revision:'9223372036854775807'},ackClientSequence:1,authorityRemainingMs:30000,accessRemainingMs:30000,credentialId:randomUUID()};assert.deepEqual(decode('ServerFrame',encode('ServerFrame',frame)),frame);
});
test('120 subscribed service connections coalesce independently of poses/control, with zero idle bank SQL and one slow peer isolated',async()=>{
 let now=Date.now();const originalNow=Date.now;Date.now=()=>now;let sql=0;
 const db:any={ready:true,serviceId:'fanout',epoch:randomUUID()},auth:any={now:()=>now},rooms:any={compatibility:{}},service:any=new LiveService(db,auth,rooms,{}as any);clearInterval(service.timer);
 const delivered:any[][]=Array.from({length:120},()=>[]),runs=Array.from({length:4},()=>randomUUID());let releaseSlow:undefined|(()=>void);
 try{
  for(let room=0;room<4;room++){
   const roomId=randomUUID(),projected:any={id:roomId,nonce:{},version:1,members:new Map(),poseFences:new Map(),terminal:false,normalEnd:false};service.projection.rooms.set(roomId,projected);
   for(let j=0;j<30;j++){const i=room*30+j,scope={principalId:randomUUID(),sessionId:randomUUID(),sessionEpoch:randomUUID(),roomId,runId:runs[room],runEpoch:randomUUID(),configRevision:1,attachmentId:randomUUID(),worldToken:randomUUID()};
    const socket:any={readyState:1,bufferedAmount:0,send:(bytes:Buffer,_options:any,callback:()=>void)=>{delivered[i]!.push(decode('ServerFrame',bytes));if(i===0)releaseSlow=callback;else callback?.();},close:()=>{},terminate:()=>{}};
    const c:any={socket,retired:false,busy:false,deadline:now+30000,serverSequence:0,control:[],sending:false,stalledAt:0,binding:{serviceId:db.serviceId,serviceEpoch:db.epoch,connectionId:randomUUID(),clientGeneration:1,authorityGeneration:1,scope,lastSequence:1,lastPoseSequence:0,authRevision:1,accessUntil:now+30000,authorityUntil:now+30000,tokenId:randomUUID(),familyId:randomUUID()}};
    projected.members.set(scope.sessionId,{scope,digest:liveScopeDigest(scope),generation:1,until:now+30000,lastPose:0});service.connections.add(c);
   }
  }
  db.pool={query:()=>{sql++;throw Error('idle bank SQL');}};
  for(let r=1;r<=50;r++)service.bankChanged(runs.map(runId=>({runId,revision:String(r)})));
  service.tick();await new Promise<void>(r=>setImmediate(r));assert.equal(delivered.flat().filter(f=>f.bankChanged).length,0);assert.ok([...service.connections].every((c:any)=>c.control.length===0&&c.bank));
  now+=200;service.tick();await new Promise<void>(r=>setImmediate(r));assert.equal(delivered.slice(1).flat().filter(f=>f.bankChanged?.revision==='50').length,119);assert.equal(delivered[0]!.filter(f=>f.bankChanged).length,0);assert.equal(sql,0);
  for(const c of service.connections)if(!c.sending)service.send(c,{pong:{}});await new Promise<void>(r=>setImmediate(r));assert.equal(delivered.slice(1).flat().filter(f=>f.pong).length,119);
  service.bankChanged(runs.map(runId=>({runId,revision:'49'})));now+=200;service.tick();await new Promise<void>(r=>setImmediate(r));assert.equal(delivered.slice(1).flat().filter(f=>f.bankChanged).length,119,'older invalidations ignored');
  releaseSlow?.();assert.equal(delivered[0]!.filter(f=>f.bankChanged?.revision==='50').length,1);assert.ok([...service.connections].every((c:any)=>!c.retired));assert.equal(sql,0);
 }finally{service.connections.clear();service.close();Date.now=originalNow;}
});
