import test from 'node:test';import assert from 'node:assert/strict';
import {randomUUID} from 'node:crypto';
import {writeFile,mkdir,access} from 'node:fs/promises';
import {resolve,dirname,sep} from 'node:path';
import {fileURLToPath} from 'node:url';
import {startLifecycleLc1Fixture} from '../scripts/lifecycle-lc1-fixture.js';
const evidence:any[]=[];
const evidenceRoot=fileURLToPath(new URL('../../../spikes/noita-entry/.build/',import.meta.url)),evidencePath=resolve(process.env.NT2_LC1_EVIDENCE_OUTPUT??resolve(evidenceRoot,'lc1-service-evidence-'+randomUUID()+'.json'));
if(!evidencePath.startsWith(resolve(evidenceRoot)+sep)||!evidencePath.endsWith('.json'))throw Error('LC1 evidence requires a fresh isolated .build JSON path');
try{await access(evidencePath);throw Error('LC1 evidence path already exists; use a fresh owned output');}catch(e){if((e as NodeJS.ErrnoException).code!=='ENOENT')throw e;}
await mkdir(dirname(evidencePath),{recursive:true});
async function run(name:string,body:(x:Awaited<ReturnType<typeof startLifecycleLc1Fixture>>)=>Promise<void>){const x=await startLifecycleLc1Fixture();try{await body(x);evidence.push({name,result:'PASS',schema:x.fixture.namespace,faults:x.faults.snapshot()});}finally{await x.close();await writeFile(evidencePath,JSON.stringify({engineFacts:'synthetic protected loaded inspection; actual TLS/SQL/routes/hostStart',checks:evidence},null,2)+'\n');}}
for(const fault of ['never-sent','before-refused','creation-rollback'] as const)test('LC1 initial '+fault+' has no association effect and reconnects with current auth/context',()=>run(fault,async x=>{
 const old=x.request();
 if(fault!=='never-sent'){x.faults.arm({worldId:x.association.worldId!,attempt:1,phase:fault==='creation-rollback'?'rollback':'before',effect:'refuse'});const failed=await x.send(x.desktop,old);assert.equal(failed.error.code,fault==='creation-rollback'?'FIXTURE_ROLLBACK':'FIXTURE_REFUSED');}
 assert.equal((await x.state()).worlds.length,0);
 const current=await x.signIn();const next=x.request(current,2,'new-process',{...x.observation,gameInstanceId:'new-process'});const success=await x.send(current,next);assert.equal(success.error,undefined);assert.equal(success.world.state,'waiting');assert.equal(success.world.generation,1);
 assert.equal((await x.send(current,{...old,sessionEpoch:current.sessionEpoch!})).error.code,'STALE_ATTEMPT');
}));
test('LC1 stage B preserves A through Continue; loaded B replaces A and fences every delayed A',()=>run('world-bound replacement',async x=>{
 const a=await x.send(x.desktop,x.request());const b=await x.prepareWorld();const b2=await x.prepareWorld(x.desktop,b.association.worldId!,b.proof);assert.deepEqual(b2.association,b.association);
 assert.equal((await x.state()).worlds.length,1);assert.equal((await x.state()).worlds[0].terminal_reason,null);
 x.advance(31000);const current=await x.signIn();const continued=await x.send(current,x.request(current,2,'continue-A',{...x.observation,gameInstanceId:'continue-A'}));assert.equal(continued.world.state,'waiting');assert.equal(continued.world.association.worldId,x.association.worldId);
 const bRequest=x.request(current,1,'fresh-B',{...x.observation,worldToken:b.association.worldId!,gameInstanceId:'fresh-B'});bRequest.reconcile!.association=b.association;bRequest.reconcile!.proof=b.proof;
 assert.equal((await x.send(current,bRequest)).world.reason,'OWNER_ACTIVE');x.advance(31000);const loaded=await x.send(current,{...bRequest,requestId:randomUUID()});assert.equal(loaded.error,undefined);assert.equal(loaded.world.state,'waiting');assert.notEqual(loaded.credentials.attachmentId,a.credentials.attachmentId);
 const old=await x.send(current,x.request(current,3,'late-A',{...x.observation,gameInstanceId:'late-A'}));assert.equal(old.world.reason,'replaced_by_new_world');assert.equal(old.world.mayPlay,false);
 await x.start();const running=await x.send(current,{...bRequest,requestId:randomUUID()});assert.equal(running.world.mayPlay,true);await assert.rejects(x.prepareWorld(current),/ENTRY_NOT_STAGING/);
}));
test('LC1 old never-committed A creation cannot supersede loaded newer B',()=>run('creation base CAS',async x=>{
 const stagedB=await x.prepareWorld();const b=x.request();b.reconcile!.association=stagedB.association;b.reconcile!.proof=stagedB.proof;b.reconcile!.observation!.worldToken=stagedB.association.worldId!;
 assert.equal((await x.send(x.desktop,b)).world.state,'waiting');assert.equal((await x.send(x.desktop,x.request())).error.code,'STALE_CREATION');
}));
test('LC1 creation lineage survives B offline; old empty-base A stays fenced and new C may stage',()=>run('creation ABA tombstone',async x=>{
 const b=await x.prepareWorld();const request=x.request();request.reconcile!.association=b.association;request.reconcile!.proof=b.proof;request.reconcile!.observation!.worldToken=b.association.worldId!;
 assert.equal((await x.send(x.desktop,request)).world.state,'waiting');
 const off=structuredClone(request);off.requestId=randomUUID();off.reconcile!.attempt=2;off.reconcile!.action='offline';delete off.reconcile!.observation;assert.equal((await x.send(x.desktop,off)).world.reason,'explicit_offline');
 assert.equal((await x.send(x.desktop,x.request())).error.code,'STALE_CREATION');
 const c=await x.prepareWorld();const fresh=x.request();fresh.reconcile!.association=c.association;fresh.reconcile!.proof=c.proof;fresh.reconcile!.observation!.worldToken=c.association.worldId!;assert.equal((await x.send(x.desktop,fresh)).world.state,'waiting');
}));
test('LC1 delayed old success reply carries no current ownership after supersession',()=>run('delayed reply fenced',async x=>{
 x.faults.arm({worldId:x.association.worldId!,attempt:1,phase:'reply',effect:'delay',delayMs:5000});const old=x.send(x.desktop,x.request());
 for(let i=0;i<60&&!x.faults.snapshot().some(e=>e.kind==='committed-reply');i++)await new Promise(r=>setTimeout(r,10));assert.ok(x.faults.snapshot().some(e=>e.kind==='committed-reply'));
 x.advance(31000);const next=await x.send(x.desktop,x.request(x.desktop,2,'new-generation',{...x.observation,gameInstanceId:'new-generation'}));assert.equal(next.world.generation,2);x.faults.release();const stale=await old;assert.equal(stale.world.generation,1);await assert.rejects(x.rpc(stale.credentials,{roomGet:{}}),/UNAUTHENTICATED/);assert.equal((await x.state()).worlds[0].generation,2);
}));
test('LC1 loaded world survives original preparation and access credential expiry',()=>run('expiry independence',async x=>{
 assert.equal((await x.send(x.desktop,x.request())).world.state,'waiting');x.advance(920000);assert.equal((await x.send(x.desktop,x.request())).error.code,'UNAUTHENTICATED');
 const current=await x.signIn();const resumed=await x.send(current,x.request(current,2,'after-expiry',{...x.observation,gameInstanceId:'after-expiry'}));assert.equal(resumed.error,undefined);assert.equal(resumed.world.state,'waiting');assert.equal(resumed.world.association.worldId,x.association.worldId);
}));
test('LC1 fresh waiting, exact retry, staging Continue and selected running Continue keep one world',()=>run('stable staging and running',async x=>{
 const initial=await x.send(x.desktop,x.request());assert.equal(initial.error,undefined);assert.equal(initial.world.state,'waiting');assert.equal(initial.world.mayPlay,false);assert.equal(initial.world.generation,1);
 const retry=await x.send(x.desktop,x.request());assert.equal(retry.credentials.attachmentId,initial.credentials.attachmentId);assert.equal(retry.world.generation,1);
 for(let i=0;i<2;i++){x.advance(20000);const health=await x.send(x.desktop,x.request());assert.equal(health.world.generation,1);assert.equal(health.world.ownerRemainingMs,30000);}
 const next=x.request(x.desktop,2,'context-2',{...x.observation,gameInstanceId:'game-2'});const wait=await x.send(x.desktop,next);assert.equal(wait.world.reason,'OWNER_ACTIVE');assert.equal(wait.world.attempt,2);assert.equal(wait.credentials,undefined);
 x.advance(31000);const restored=await x.send(x.desktop,{...next,requestId:randomUUID()});assert.equal(restored.world.state,'waiting');assert.equal(restored.world.generation,2);assert.notEqual(restored.credentials.attachmentId,initial.credentials.attachmentId);
 await x.start();const selected=await x.send(x.desktop,{...next,requestId:randomUUID()});assert.equal(selected.world.mayPlay,true);assert.equal(selected.world.state,'playing');const original=selected.canonicalAdmission.originalStart;
 x.advance(31000);const current=await x.signIn();const resumed=await x.send(current,x.request(current,3,'cold-context',{...x.observation,gameInstanceId:'cold-game'}));assert.equal(resumed.world.mayPlay,true);assert.equal(resumed.world.generation,3);assert.deepEqual(resumed.canonicalAdmission.originalStart,original);assert.equal((await x.state()).worlds.length,1);
}));
test('LC1 never-sent and rejected/rolled-back attempts reconcile after current auth/context changes',()=>run('noncommits and credential turnover',async x=>{
 const unsent=x.request();const invalid=x.request();invalid.reconcile!.observation!.actualSeed='1';const rejected=await x.send(x.desktop,invalid);assert.equal(rejected.world.state,'review');assert.equal(rejected.credentials,undefined);
 // A genuinely rolled-back original leaves no row. Its proof/local identity
 // remains usable with a later sequence and current authentication.
 const target=x.request(x.desktop,2,'rolled-back',{...x.observation,gameInstanceId:'rollback-game'});x.faults.arm({worldId:x.association.worldId!,attempt:2,phase:'rollback',effect:'refuse'});assert.equal((await x.send(x.desktop,target)).error.code,'FIXTURE_ROLLBACK');
 const current=await x.signIn();const success=await x.send(current,x.request(current,3,'current',{...x.observation,gameInstanceId:'current-game'}));assert.equal(success.error,undefined);assert.equal(success.world.state,'waiting');assert.equal(success.world.attempt,3);
 const delayed=await x.send(current,{...unsent,sessionEpoch:current.sessionEpoch!});assert.ok(['STALE_ATTEMPT','CONFLICT'].includes(delayed.error.code));assert.equal((await x.state()).worlds[0].high_water,3);
}));
test('LC1 committed lost reply and delayed old requests cannot reclaim a newer owner',()=>run('lost reply and delayed precommit',async x=>{
 x.faults.arm({worldId:x.association.worldId!,attempt:1,phase:'reply',effect:'drop'});await assert.rejects(x.send(x.desktop,x.request()));assert.equal((await x.state()).worlds[0].generation,1);
 x.advance(31000);const current=await x.signIn();const newRequest=x.request(current,3,'new',{...x.observation,gameInstanceId:'new'});
 x.faults.arm({worldId:x.association.worldId!,attempt:2,phase:'before',effect:'delay',delayMs:2000});const old=x.send(current,x.request(current,2,'old',{...x.observation,gameInstanceId:'old'}));await new Promise(r=>setTimeout(r,75));
 const success=await x.send(current,newRequest);assert.equal(success.world.state,'waiting');x.faults.release();assert.equal((await old).error.code,'STALE_ATTEMPT');assert.equal((await x.state()).worlds[0].owner_attempt,3);
}));
test('LC1 OWNER_ACTIVE commits supersession, duplicates do not extend wait or restore old authority',()=>run('owner lease fence',async x=>{
 const first=await x.send(x.desktop,x.request());const second=x.request(x.desktop,2,'second',{...x.observation,gameInstanceId:'second'});const wait=await x.send(x.desktop,second);assert.equal(wait.world.reason,'OWNER_ACTIVE');assert.equal((await x.state()).worlds[0].high_water,2);
 assert.equal((await x.send(x.desktop,x.request())).error.code,'STALE_ATTEMPT');
 x.advance(15000);await x.rpc(x.desktop,{presence:{present:true}});await x.rpc(first.credentials,{gamePulse:{attachmentId:first.credentials.attachmentId,gameInstanceId:x.observation.gameInstanceId,worldToken:x.association.worldId}});const next=await x.send(x.desktop,{...second,requestId:randomUUID()});assert.ok(next.world.retryAfterMs<wait.world.retryAfterMs);
 x.advance(16000);const recovered=await x.send(x.desktop,{...second,requestId:randomUUID()});assert.equal(recovered.world.state,'waiting');assert.equal(recovered.world.generation,2);
}));
test('LC1 wrong proof/account/report and offline remain scoped; running unknown world cannot inherit Start',()=>run('identity selection and terminal',async x=>{
 const first=await x.send(x.desktop,x.request());const bad=x.request(x.desktop,2,'bad');bad.reconcile!.proof='x'.repeat(43);assert.equal((await x.send(x.desktop,bad)).error.code,'FORBIDDEN');
 const other=await x.signIn(1);assert.equal((await x.send(other,x.request(other,2,'other'))).error.code,'FORBIDDEN');
 await x.start();const unknown=x.request(x.desktop,2,'unknown');unknown.reconcile!.association={...x.association,worldId:randomUUID()};unknown.reconcile!.observation!.worldToken=unknown.reconcile!.association.worldId!;assert.ok(['FORBIDDEN','ENTRY_NOT_STAGING'].includes((await x.send(x.desktop,unknown)).error.code));
 const off=x.request(x.desktop,2,'offline');off.reconcile!.action='offline';delete off.reconcile!.observation;assert.equal((await x.send(x.desktop,off)).world.state,'terminal');
 const later=await x.send(x.desktop,x.request(x.desktop,3,'later'));assert.equal(later.world.reason,'explicit_offline');assert.equal(later.credentials,undefined);assert.equal(later.world.mayPlay,false);assert.equal(first.world.association.worldId,x.association.worldId);
}));



