import test from 'node:test';import assert from 'node:assert/strict';
import {C1RuleDiagnostic,runC1RuleControl} from '../scripts/lc1-c1-rule-control.js';
import {fixtureC1ReadRefreshCapacity,FixtureRpcRejection} from '../scripts/lc1-host-action.js';
import {C1_DEFAULTS} from '../../../packages/domain/src/g3-c1.js';
const read=JSON.stringify({requestId:'frozen',roomId:'room',sessionEpoch:'session',rulesGet:{}});
const refresh=JSON.stringify({requestId:'frozen',refresh:{secret:'private-refresh-secret',deviceId:'device'}});
const capacity=()=>new FixtureRpcRejection('CAPACITY',200,true);
const response=(rules:any=C1_DEFAULTS)=>({snapshot:{revision:4,sequence:'4',rules},sequence:'4'});
test('C1 successful host refresh/read/write reports stages and retains exact CAS and unrelated rules',async()=>{
 const diagnostic=new C1RuleDiagnostic(),calls:any[]=[],deadlines:number[]=[];
 const result=await runC1RuleControl({expectedRevision:4,rules:{NT_sync_hearts:false}},{before:()=>{},ensureHost:async d=>{deadlines.push(d);},send:async(p,d)=>{calls.push(p);deadlines.push(d);return response(p.rulesSet?.rules??{...C1_DEFAULTS,NT_ban_perks:true});}},diagnostic,()=>100);
 assert.deepEqual(deadlines,[8100,8100,8100]);assert.equal(calls.length,2);assert.equal(calls[1].rulesSet.expectedRevision,4);assert.equal(calls[1].rulesSet.rules.NT_ban_perks,true);assert.equal(result.snapshot.rules.NT_sync_hearts,false);assert.deepEqual(diagnostic.snapshot().events.filter(v=>v.outcome==='accepted').map(v=>v.stage),['arguments','host-refresh','rules-read','rules-write']);
});
for(const [kind,bytes] of [['read',read],['refresh',refresh]] as const){
 test(`C1 ${kind} confirmed capacity retries exact immutable bytes/token with bounded attempts`,async()=>{let now=0;const seen:any[]=[];const timing={now:()=>now,wait:async(ms:number)=>{now+=ms;}};const result=await fixtureC1ReadRefreshCapacity(kind,bytes,'private-token',8000,async(body,token,timeout)=>{seen.push({body,token,timeout});if(seen.length<3)throw capacity();return 'ok';},()=>{},timing);assert.equal(result,'ok');assert.equal(seen.length,3);assert.equal(now,500);assert.ok(seen.every(v=>v.body===bytes&&v.token==='private-token'&&v.timeout===5000));
 let calls=0;now=0;await assert.rejects(fixtureC1ReadRefreshCapacity(kind,bytes,'token',8000,async()=>{calls++;throw capacity();},()=>{},timing),/CAPACITY/);assert.equal(calls,4);assert.equal(now,750);
 calls=0;now=0;await assert.rejects(fixtureC1ReadRefreshCapacity(kind,bytes,'token',400,async(_b,_t,timeout)=>{assert.equal(timeout,400);calls++;now=200;throw capacity();},()=>{},timing),/CAPACITY/);assert.equal(calls,1);assert.equal(now,200);
 });
 test(`C1 ${kind} refuses uncorrelated, non-200, unknown and non-capacity failure replay`,async()=>{for(const error of [new FixtureRpcRejection('CAPACITY',200,false),new FixtureRpcRejection('CAPACITY',429,true),new FixtureRpcRejection('UNAUTHENTICATED',200,true),Error('private-secret transport unknown commit')]){let calls=0;await assert.rejects(fixtureC1ReadRefreshCapacity(kind,bytes,'token',8000,async()=>{calls++;throw error;},()=>{},{now:()=>0,wait:async()=>assert.fail()}),e=>e===error);assert.equal(calls,1);}});
}
test('C1 retry selector rejects mutations and mixed commands before sending',async()=>{for(const [kind,body] of [['read',{rulesSet:{expectedRevision:4,rules:{}}}],['read',{rulesGet:{},rulesSet:{expectedRevision:4,rules:{}}}],['refresh',{roomGet:{}}],['refresh',{refresh:{secret:'s',deviceId:'d'},roomGet:{}}]] as const){let calls=0;await assert.rejects(fixtureC1ReadRefreshCapacity(kind,JSON.stringify(body),'token',8000,async()=>{calls++;},()=>{},{now:()=>0,wait:async()=>{}}) as any);assert.equal(calls,0);}});
test('C1 rule mutations are single attempt even for confirmed capacity or unknown commit; diagnostics exclude raw secrets',async()=>{
 for(const error of [capacity(),new FixtureRpcRejection('CAPACITY',200,false),Error('private-token private-refresh-secret raw-body')]){let reads=0,writes=0;const diagnostic=new C1RuleDiagnostic();await assert.rejects(runC1RuleControl({expectedRevision:4,rules:{NT_sync_hearts:false}},{before:()=>{},ensureHost:async()=>{},send:async p=>{if(p.rulesGet){reads++;return response();}writes++;assert.equal(p.rulesSet.expectedRevision,4);throw error;}},diagnostic,()=>0),e=>e===error);assert.equal(reads,1);assert.equal(writes,1);const safe=JSON.stringify(diagnostic.snapshot());assert.ok(!safe.includes('private-'));assert.equal(diagnostic.snapshot().stage,'rules-write');assert.equal(diagnostic.snapshot().events.at(-1).code,error instanceof FixtureRpcRejection?'CAPACITY':'UNKNOWN_FAILURE');}
});
test('C1 shares one total deadline across refresh/read/write and diagnoses invalid inputs',async()=>{
 let now=0,calls=0;const d=new C1RuleDiagnostic();await assert.rejects(runC1RuleControl({rules:{}},{before:()=>{},ensureHost:async()=>{assert.fail();},send:async()=>{}},d,()=>now));assert.equal(d.snapshot().stage,'arguments');
 await assert.rejects(runC1RuleControl({expectedRevision:4,rules:{}},{before:()=>{},ensureHost:async()=>{now=7999;},send:async p=>{calls++;assert.ok(p.rulesGet);now=8000;return response();}},d,()=>now),/FIXTURE_C1_DEADLINE/);assert.equal(calls,1);assert.equal(d.snapshot().stage,'rules-read');
});
test('C1 malformed successful reply still records a safe failure at the read stage',async()=>{
 const d=new C1RuleDiagnostic();await assert.rejects(runC1RuleControl({},{before:()=>{},ensureHost:async()=>{},send:async()=>({sequence:'4'})},d,()=>0));assert.equal(d.snapshot().stage,'rules-read');assert.equal(d.snapshot().events.at(-1).outcome,'failed');assert.equal(d.snapshot().events.at(-1).code,'FIXTURE_C1_RESPONSE');
});

