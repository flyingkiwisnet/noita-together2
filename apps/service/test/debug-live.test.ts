import test from 'node:test';import assert from 'node:assert/strict';import {randomUUID} from 'node:crypto';
import {mkdtemp,readFile,readdir,rm} from 'node:fs/promises';import {join} from 'node:path';
import {LiveService} from '../src/g3/live-service.js';import {ServerDebugDiagnostics,diagnosticServiceTag} from '../src/g3/debug-diagnostics.js';import {PolicyError} from '../src/g1/security.js';
async function run(fn:(f:any)=>Promise<void>,enabled=true){const root=await mkdtemp(join(process.cwd(),'spikes/noita-entry/.build/nt2-debug-live-'));let now=100000;
 const db:any={ready:true,serviceId:'test-service',epoch:randomUUID(),transaction:async()=>{db.transactionDiagnostic?.({phase:'lock',category:'deadline',code:'SERVICE_UNAVAILABLE',sqlState:'55P03',commit:'not-attempted'});throw new PolicyError('SERVICE_UNAVAILABLE');}};
 const diagnostic=new ServerDebugDiagnostics('test-runtime',enabled,()=>now,{root,serviceTag:diagnosticServiceTag(db.serviceId,db.epoch)}),live:any=new LiveService(db,{now:()=>now} as any,{} as any,{} as any,diagnostic);clearInterval(live.timer);
 const roomId=randomUUID(),sessionId=randomUUID(),scope={principalId:randomUUID(),sessionId,sessionEpoch:randomUUID(),roomId,runId:randomUUID(),runEpoch:randomUUID(),configRevision:1,attachmentId:randomUUID(),worldToken:randomUUID()};
 const member={scope,digest:'test',generation:1,until:now+30000,parentId:randomUUID(),familyId:randomUUID(),lastPose:21},room={id:roomId,nonce:{},version:1,members:new Map([[sessionId,member]]),poseFences:new Map(),terminal:false,normalEnd:false};live.projection.rooms.set(roomId,room);
 const c:any={binding:{scope:{...scope,sessionId:randomUUID()},serviceId:db.serviceId,serviceEpoch:db.epoch,connectionId:randomUUID(),clientGeneration:1,authorityGeneration:1,authRevision:1,accessUntil:now+900000,authorityUntil:now+30000},serverSequence:0,retired:false,busy:false,control:[],socket:{readyState:1,bufferedAmount:0,terminate(){}},snapshot:undefined};
 const seed=()=>live.samples.set(sessionId,{scope,generation:1,acceptedAt:now,poses:[{sequence:21,xMilli:'SECRET_POSITION',yMilli:'0',vxMilli:'0',vyMilli:'0'}],poseTimes:[now]});
 const frame=(visible:boolean)=>live.frame(c,{snapshot:{peers:visible?[{principalId:scope.principalId,sessionId,attachmentId:scope.attachmentId,worldToken:scope.worldToken,poses:live.samples.get(sessionId)?.poses??[]}]:[],acceptedAt:new Date(now).toISOString()}});
 try{await fn({db,live,diagnostic,room,scope,member,c,seed,frame,root,advance:(n:number)=>{now+=n;},events:async()=>{await diagnostic.flush();return (await readFile(join(diagnostic.folder,'events.jsonl'),'utf8')).trim().split('\n').map(s=>JSON.parse(s));}});}finally{live.close();await diagnostic.close();await rm(root,{recursive:true,force:true});}}
test('production TTL deletion records exact age/generation/last accepted pose and first empty wire boundary, then recovery',()=>run(async f=>{
 f.seed();f.frame(true);f.advance(2001);f.live.tick();f.frame(false);assert.equal(f.live.samples.size,0);
 let rows=await f.events();const removed=rows.find((r:any)=>r.kind==='sample_removed');assert.equal(removed.reason,'pose_ttl');assert.equal(removed.sampleAgeMs,2001);assert.equal(removed.sampleGeneration,1);assert.equal(removed.memberGeneration,1);assert.equal(removed.lastAcceptedPoseSequence,21);assert.equal(removed.eligible,true);
 f.seed();f.frame(true);await f.diagnostic.flush();const first=JSON.parse(await readFile(join(f.diagnostic.folder,'first-loss.json'),'utf8'));assert.equal(first.trigger.kind,'snapshot_empty');assert.equal(first.recovery.kind,'recovery');assert.equal(JSON.stringify(first).includes('SECRET_POSITION'),false);
}));
test('production generation deletion and ordinary protected/End departures are distinguished',()=>run(async f=>{
 f.seed();f.member.generation=2;f.live.changed(f.room);assert.equal((await f.events()).find((r:any)=>r.kind==='sample_removed').reason,'generation_changed');
 f.member.generation=1;f.seed();f.frame(true);f.live.projection.observeWorlds(f.room,[{sessionId:f.scope.sessionId,protected:true,authorized:true,terminal:false,reason:''}]);f.room.members.clear();f.live.changed(f.room);f.frame(false);
 const rows=await f.events(),protectedRemoval=rows.filter((r:any)=>r.kind==='sample_removed').at(-1);assert.equal(protectedRemoval.protected,true);assert.equal(protectedRemoval.reason,'member_removed');assert.equal(rows.some((r:any)=>r.kind==='snapshot_empty'),false);
 f.seed();f.room.terminal=true;f.room.normalEnd=true;f.live.changed(f.room);assert.equal((await f.events()).filter((r:any)=>r.kind==='sample_removed').at(-1).normalEnd,true);
}));
test('actual renewal failure records correlated DB lock outcome and exact retirement code without token contents',()=>run(async f=>{
 await f.live.authenticate(f.c,{serviceId:f.db.serviceId,serviceEpoch:f.db.epoch,reauthenticate:{accessToken:'DO_NOT_LOG_TOKEN'}},true);
 const rows=await f.events(),result=rows.find((r:any)=>r.kind==='renew_result'),db=rows.find((r:any)=>r.kind==='db_result'),retire=rows.find((r:any)=>r.kind==='retirement');
 assert.equal(result.stage,'db');assert.equal(result.code,'SERVICE_UNAVAILABLE');assert.equal(db.sqlState,'55P03');assert.equal(db.commit,'not-attempted');assert.equal(result.operationId,db.operationId);assert.equal(retire.operationId,result.operationId);assert.equal(retire.code,'SERVICE_UNAVAILABLE');assert.equal(JSON.stringify(rows).includes('DO_NOT_LOG_TOKEN'),false);assert.equal(f.c.retired,true);
}));
test('disabled diagnostics preserve real deletion and renewal retirement with no disk output',()=>run(async f=>{
 f.seed();f.advance(2001);f.live.tick();assert.equal(f.live.samples.size,0);await f.live.authenticate(f.c,{serviceId:f.db.serviceId,serviceEpoch:f.db.epoch,reauthenticate:{accessToken:'off-token'}},true);assert.equal(f.c.retired,true);assert.deepEqual(await readdir(f.root),[]);
},false));
