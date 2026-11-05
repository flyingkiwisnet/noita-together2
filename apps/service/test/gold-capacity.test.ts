// Policy checks with labelled in-memory SQL/authority seams; not SQL or engine evidence.
import test from 'node:test';
import assert from 'node:assert/strict';
import {randomUUID} from 'node:crypto';
import {G3Authority,operationDigest} from '../src/g3/authority.js';
import {fingerprint} from '../src/g1/security.js';
import {DatabaseWorkScheduler} from '../src/g1/work.js';

function fixture(options:{total?:number;pending?:number;receipts?:number;amount?:number;held?:number;version?:string;changeFails?:boolean}={}){
 const scope={roomId:randomUUID(),runId:randomUUID(),runEpoch:randomUUID(),configRevision:1,attachmentId:randomUUID(),worldToken:randomUUID()};
 const actor:any={kind:'game',accountId:randomUUID(),sessionId:randomUUID()},intent={operationId:randomUUID(),direction:'deposit',amount:1};
 const r={id:scope.runId,room_id:scope.roomId,epoch:scope.runEpoch,config_revision:1,state:'running',mode:'core'};
 const o:any={run_id:r.id,principal_id:actor.accountId,operation_id:intent.operationId,attachment_id:scope.attachmentId,world_token:scope.worldToken,run_epoch:r.epoch,config_revision:1,direction:'deposit',amount:1,state:'awaiting_local',delivery_id:randomUUID(),digest:operationDigest(actor.accountId,scope,'deposit',1)};
 const proof={operationId:o.operation_id,receiptId:randomUUID(),deliveryId:o.delivery_id,payloadDigest:o.digest,outcome:'escrowed',evidenceDigest:'a'.repeat(64)};
 const calls:string[]=[];let existing=false,duplicateReceipt=false;
 const tx:any={query:async(sql:string)=>{calls.push(sql);
  let rows:any[]=[];
  if(sql.startsWith('SELECT o.*'))rows=existing?[{...o,balance:options.amount??0,balance_version:options.version??'0'}]:[];
  else if(sql.startsWith('SELECT * FROM g3_operations'))rows=[o];
  else if(sql.startsWith('SELECT fingerprint FROM g3_receipts'))rows=duplicateReceipt?[{fingerprint:fingerprint(proof)}]:[];
  else if(sql.startsWith('SELECT amount,version'))rows=[{amount:options.amount??0,version:options.version??'0'}];
  else if(sql.startsWith('SELECT count(*)'))rows=[{count:sql.includes('g3_receipts')?options.receipts??0:sql.includes('state NOT IN')?options.pending??0:options.total??0}];
  else if(sql.startsWith('SELECT coalesce(sum'))rows=[{amount:options.held??0}];
  else if(sql.startsWith('UPDATE g3_balances'))rows=options.changeFails?[]:[{amount:1,version:'1'}];
  return {rows};
 }};
 const authority=new G3Authority(()=>randomUUID(),()=>1000);
 (authority as any).access=async()=>r;
 (authority as any).context=async()=>({r,c:{capabilities:['core.gold'],attachment_id:scope.attachmentId,world_token:scope.worldToken}});
 return {authority,tx,scope,actor,o,intent,proof,calls,recover:()=>{existing=true;},duplicate:()=>{duplicateReceipt=true;}};
}
const bankFull=(e:any)=>e.code==='BANK_FULL';
test('gold durable intent bounds are BANK_FULL before an original or balance mutation',async()=>{
 for(const options of [{total:1000},{pending:256},{amount:4294967295},{held:4294967295},{version:'9223372036854775807'}]){
  const f=fixture(options);
  await assert.rejects(f.authority.execute(f.tx,f.actor,{scope:f.scope,goldIntent:f.intent}),bankFull);
  assert.equal(f.calls.some(s=>/^(INSERT INTO g3_operations|UPDATE g3_balances|INSERT INTO g3_ledger)/.test(s)),false);
 }
});
test('accepted originals and duplicate receipts recover before durable quota checks',async()=>{
 const f=fixture({total:1000,pending:256,receipts:8,version:'9223372036854775807'});f.recover();
 const original=await f.authority.execute(f.tx,f.actor,{scope:f.scope,goldIntent:f.intent});
 assert.equal(original.operation?.deliveryId,f.o.delivery_id);assert.equal(original.operation?.payloadDigest,f.o.digest);
 f.duplicate();const receipt=await f.authority.execute(f.tx,f.actor,{scope:f.scope,goldReceipt:f.proof});
 assert.equal(receipt.operation?.operationId,f.o.operation_id);assert.equal(f.calls.some(s=>s.startsWith('SELECT count(*)')),false);
});
test('receipt quota, balance and version refusal retain the accepted original without ledger or receipt mutation',async()=>{
 for(const options of [{receipts:8},{amount:4294967295},{version:'9223372036854775807',changeFails:true}]){
  const f=fixture(options);await assert.rejects(f.authority.execute(f.tx,f.actor,{scope:f.scope,goldReceipt:f.proof}),bankFull);
  assert.equal(f.o.state,'awaiting_local');
  assert.equal(f.calls.some(s=>/^(INSERT INTO g3_receipts|INSERT INTO g3_ledger|UPDATE g3_operations)/.test(s)),false);
 }
});
test('temporary optional admission remains CAPACITY and admission succeeds after release',async()=>{
 const scheduler=new DatabaseWorkScheduler(1),signal=new AbortController().signal,release=await scheduler.enter('optional',signal);
 await assert.rejects(scheduler.enter('optional',signal),(e:any)=>e.code==='CAPACITY');
 release();const retry=await scheduler.enter('optional',signal);retry();assert.equal(scheduler.stats.active,0);
});
