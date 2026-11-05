import test from 'node:test';import assert from 'node:assert/strict';
import {mkdtemp,readFile,readdir,rm,writeFile,stat} from 'node:fs/promises';import {tmpdir} from 'node:os';import {join} from 'node:path';
import {ServerDebugDiagnostics,diagnosticEnabled,diagnosticTag,diagnosticServiceTag,DEBUG_LIMITS} from '../src/g3/debug-diagnostics.js';
async function scratch(fn:(root:string)=>Promise<void>){const root=await mkdtemp(join(process.cwd(),'spikes/noita-entry/.build/nt2-server-debug-'));try{await fn(root);}finally{await rm(root,{recursive:true,force:true});}}
const options={serviceTag:diagnosticServiceTag('service','epoch')};
test('debug setting defaults/explicit opt-outs/re-enable/release default and no mod-only preference crossover',()=>scratch(async root=>{
 const settingsPath=join(root,'debug-settings.json');const setting={settingsPath,environment:'',production:false};
 assert.equal(await diagnosticEnabled(setting),true);assert.equal(await diagnosticEnabled({...setting,production:true}),false);
 await writeFile(settingsPath,JSON.stringify({debugDiagnostics:false}));assert.equal(await diagnosticEnabled({...setting,enabled:true,environment:'on'}),false);
 await writeFile(settingsPath,JSON.stringify({debugDiagnostics:true,NT_ENABLE_LOGGER:false}));assert.equal(await diagnosticEnabled(setting),true);
 assert.equal(await diagnosticEnabled({...setting,environment:'off'}),false);assert.equal(await diagnosticEnabled({...setting,enabled:false}),false);
 await writeFile(settingsPath,JSON.stringify({debugDiagnostics:true}));assert.equal(await diagnosticEnabled({...setting,production:true}),true);
}));
test('first meaningful loss persists safe context, distinct pose/event sequences and recovery across restart',()=>scratch(async root=>{
 let now=1000;const d=new ServerDebugDiagnostics('test-runtime',true,()=>now,{root,...options}),connection={};
 d.record('pose_accepted',{sessionTag:diagnosticTag('game'),poseSequence:21,token:'NEVER_COPY',chat:'NEVER_COPY',poses:[{x:7}]},false);
 d.snapshot(connection,['game'],['game'],{connectionTag:diagnosticTag('connection')});now+=2001;
 d.record('sample_removed',{sessionTag:diagnosticTag('game'),reason:'pose_ttl',sampleAgeMs:2001,sampleGeneration:1,memberGeneration:1,lastAcceptedPoseSequence:21,eligible:true});
 d.snapshot(connection,[],['game'],{connectionTag:diagnosticTag('connection')});d.snapshot(connection,['game'],['game'],{connectionTag:diagnosticTag('connection')});
 await d.flush();const first=JSON.parse(await readFile(join(d.folder,'first-loss.json'),'utf8'));
 assert.equal(first.trigger.kind,'snapshot_empty');assert.equal(first.before.at(-1).lastAcceptedPoseSequence,21);assert.equal(first.before[1].poseSequence,21);assert.notEqual(first.before[1].sequence,21);
 assert.equal(first.recovery.kind,'recovery');assert.equal(first.serviceTag,options.serviceTag);assert.equal(JSON.stringify(first).includes('NEVER_COPY'),false);assert.equal(JSON.stringify(first).includes('"x"'),false);
 const trigger=first.trigger.sequence;for(let i=0;i<300;i++)d.record('pose_accepted',{poseSequence:i},false);await d.close();
 const retained=JSON.parse(await readFile(join(d.folder,'first-loss.json'),'utf8'));assert.equal(retained.trigger.sequence,trigger);assert.equal(retained.recovery.kind,'recovery');assert.ok(retained.after.length<=64);
 const next=new ServerDebugDiagnostics('test-runtime',true,()=>now,{root,...options});await next.close();assert.ok(await stat(join(d.folder,'first-loss.json')));
 assert.equal(JSON.parse(await readFile(join(d.folder,'manifest.json'),'utf8')).closed,true);
}));
test('ordinary departure/protection has no unexpected-loss capture, while a surviving peer loss does',()=>scratch(async root=>{
 const d=new ServerDebugDiagnostics('test-runtime',true,()=>1,{root,...options}),c={};d.snapshot(c,['leaving'],['leaving'],{});d.snapshot(c,[],[],{normalEnd:true});await d.flush();
 assert.equal(JSON.parse(await readFile(join(d.folder,'manifest.json'),'utf8')).firstLoss,false);
 d.snapshot(c,['healthy'],['healthy'],{});d.snapshot(c,[],['healthy'],{});await d.close();assert.equal(JSON.parse(await readFile(join(d.folder,'first-loss.json'),'utf8')).trigger.reason,'eligible_peer_sample_absent');
}));
test('bounded queue, disk cap and session retention survive full logs and parallel flush/capture reads',()=>scratch(async root=>{
 const limits={eventsBytes:4096,firstLossBytes:4096,queue:4,sessions:2,ring:8,after:4};
 for(let session=0;session<4;session++){
  const d=new ServerDebugDiagnostics('test-runtime',true,()=>session,{root,...options,limits});
  for(let i=0;i<200;i++)d.record('retirement',{code:'EXPIRED',operationId:i});
  await Promise.all([d.flush(),d.flush()]);await d.close();const manifest=JSON.parse(await readFile(join(d.folder,'manifest.json'),'utf8'));
  assert.ok(manifest.dropped>0);assert.ok((await stat(join(d.folder,'events.jsonl'))).size<=limits.eventsBytes);
 }
 assert.equal((await readdir(join(root,'test-runtime'))).length,2);
 const cap=new ServerDebugDiagnostics('cap-runtime',true,()=>1,{root,...options,limits:{...limits,queue:256}});
 for(let i=0;i<250;i++)cap.record('retirement',{code:'EXPIRED'});await cap.close();assert.equal(cap.status.truncated,true);assert.ok((await stat(join(cap.folder,'events.jsonl'))).size<=4096);
 assert.equal(DEBUG_LIMITS.sessions,8);
}));
test('disabled and unwritable storage cannot throw or alter a transaction result; observer is restored',()=>scratch(async root=>{
 const blocked=join(root,'file');await writeFile(blocked,'owned');
 const off=new ServerDebugDiagnostics('test-runtime',false,()=>{throw Error('should not read clock');},{root:join(blocked,'child'),...options});off.record('snapshot_empty',{});await off.close();assert.equal(off.status.errors,0);
 const d=new ServerDebugDiagnostics('test-runtime',true,()=>1,{root:blocked,...options});
 const original=async()=>17,prior=()=>{};const db:any={transaction:original,transactionDiagnostic:prior};d.bindDatabase(db);
 assert.equal(await db.transaction(),17);d.record('retirement',{code:'EXPIRED',raw:'NEVER_COPY'});await d.close();assert.ok(d.status.errors>=1);assert.equal(db.transaction,original);assert.equal(db.transactionDiagnostic,prior);
}));
test('actual DB observer operation context correlates renewal, commit outcome and retirement without raw messages',()=>scratch(async root=>{
 const d=new ServerDebugDiagnostics('test-runtime',true,()=>1,{root,...options});
 const db:any={transaction:async(fn:()=>Promise<number>)=>{const value=await fn();db.transactionDiagnostic({phase:'lock',category:'deadline',code:'SERVICE_UNAVAILABLE',sqlState:'55P03',commit:'not-attempted',message:'NEVER_COPY'});return value;}};d.bindDatabase(db);
 await d.transaction({connectionTag:diagnosticTag('socket')},async()=>{d.operationEvent('renew_begin',{});await db.transaction(async()=>5);d.operationEvent('renew_result',{stage:'db',outcome:'refused',code:'SERVICE_UNAVAILABLE'});d.operationEvent('retirement',{code:'RETIRED'});});await d.close();
 const rows=(await readFile(join(d.folder,'events.jsonl'),'utf8')).trim().split('\n').map(line=>JSON.parse(line)),related=rows.filter(r=>r.operationId);
 assert.equal(new Set(related.map(r=>r.operationId)).size,1);assert.equal(related.find(r=>r.kind==='db_result').sqlState,'55P03');assert.equal(JSON.stringify(rows).includes('NEVER_COPY'),false);
 assert.equal(diagnosticServiceTag('service','epoch'),diagnosticTag('service\nepoch'));
}));
test('concurrent sessions cannot exceed retention cap or delete an active writer',()=>scratch(async root=>{
 const sinks=Array.from({length:3},()=>new ServerDebugDiagnostics('test-runtime',true,()=>1,{root,...options,limits:{sessions:2}}));
 await Promise.all(sinks.map(d=>d.flush()));assert.equal(sinks.filter(d=>d.status.storageReady).length,2);assert.equal((await readdir(join(root,'test-runtime'))).length,2);
 await Promise.all(sinks.map(d=>d.close()));
 const next=new ServerDebugDiagnostics('test-runtime',true,()=>2,{root,...options,limits:{sessions:2}});await next.close();assert.equal(next.status.storageReady,true);assert.equal((await readdir(join(root,'test-runtime'))).length,2);
}));
test('critical renewal context survives high pose traffic and bounded first-loss context is explicit',()=>scratch(async root=>{
 const d=new ServerDebugDiagnostics('test-runtime',true,()=>1,{root,...options,limits:{firstLossBytes:4096}}),c={};
 d.record('renew_result',{outcome:'accepted',operationId:7});for(let i=0;i<500;i++)d.record('pose_accepted',{poseSequence:i},false);
 d.snapshot(c,['peer'],['peer'],{});d.snapshot(c,[],['peer'],{});d.snapshot(c,['peer'],['peer'],{});await d.close();
 const first=JSON.parse(await readFile(join(d.folder,'first-loss.json'),'utf8'));assert.equal(first.truncatedContext,true);assert.equal(first.trigger.kind,'snapshot_empty');assert.equal(first.recovery.kind,'recovery');assert.ok((await stat(join(d.folder,'first-loss.json'))).size<=4096);
 // The complete critical event is also retained in the capped persistent stream.
 assert.ok((await readFile(join(d.folder,'events.jsonl'),'utf8')).includes('"operationId":7'));
}));
test('routine commits stay in bounded context but a transaction causing sample removal promotes its actual outcome',()=>scratch(async root=>{
 const d=new ServerDebugDiagnostics('test-runtime',true,()=>1,{root,...options});const db:any={transaction:async(fn:()=>Promise<void>)=>{await fn();db.transactionDiagnostic({phase:'commit',category:'committed',code:'COMMIT',commit:'confirmed'});d.operationEvent('sample_removed',{reason:'generation_changed',sampleGeneration:1,memberGeneration:2});}};d.bindDatabase(db);
 await db.transaction(async()=>{});await d.close();const rows=(await readFile(join(d.folder,'events.jsonl'),'utf8')).trim().split('\n').map(s=>JSON.parse(s));
 const removed=rows.find(r=>r.kind==='sample_removed'),commit=rows.find(r=>r.kind==='db_result');assert.equal(removed.operationId,commit.operationId);assert.equal(commit.commit,'confirmed');
}));
test('a stalled writer has one drain across many event-loop turns; records stay bounded and first loss eventually flushes',()=>scratch(async root=>{
 const d=new ServerDebugDiagnostics('test-runtime',true,()=>1,{root,...options}),internal=d as any,write=internal.write.bind(d);
 let release!:()=>void,entered!:()=>void,jobs=0;const held=new Promise<void>(r=>release=r),started=new Promise<void>(r=>entered=r);
 internal.write=async()=>{jobs++;entered();await held;await write();};
 await started;
 for(let turn=0;turn<200;turn++){for(let i=0;i<20;i++)d.record('retirement',{code:'EXPIRED'});await new Promise<void>(r=>setImmediate(r));assert.equal(d.status.pendingWriterJobs,1);assert.equal(d.status.writerScheduled,false);assert.ok(d.status.pendingEvents<=DEBUG_LIMITS.queue);}
 assert.equal(jobs,1,'there is no per-event-loop-turn promise/write backlog');
 const c={};d.snapshot(c,['peer'],['peer'],{});d.snapshot(c,[],['peer'],{});d.snapshot(c,['peer'],['peer'],{});
 let flushed=false;const flushing=d.flush().then(()=>{flushed=true;});await new Promise<void>(r=>setImmediate(r));assert.equal(flushed,false);
 release();await flushing;await d.close();assert.ok(jobs<=4);assert.equal(d.status.pendingWriterJobs,0);assert.equal(d.status.pendingEvents,0);
 const first=JSON.parse(await readFile(join(d.folder,'first-loss.json'),'utf8'));assert.equal(first.trigger.kind,'snapshot_empty');assert.equal(first.recovery.kind,'recovery');assert.ok(d.status.dropped>0);
}));
