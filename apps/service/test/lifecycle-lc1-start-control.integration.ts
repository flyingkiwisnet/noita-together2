import test from 'node:test';import assert from 'node:assert/strict';
import {startLifecycleLc1Fixture} from '../scripts/lifecycle-lc1-fixture.js';
type Fixture=Awaited<ReturnType<typeof startLifecycleLc1Fixture>>;
const latest=(x:Fixture)=>x.startDiagnostics().at(-1)!;
async function run(label:string,body:(x:Fixture)=>Promise<void>){const x=await startLifecycleLc1Fixture();try{await body(x);console.log(JSON.stringify({startControl:label,schema:x.fixture.namespace,diagnostics:x.startDiagnostics()}));}finally{await x.close();}}
async function ready(x:Fixture){const value=await x.send(x.desktop,x.request());assert.equal(value.world?.state,'waiting');}

test('Start refreshes room revision after paged inspection while preserving exact selected and excluded worlds',()=>run('revision-after-inspection',async x=>{
 const peer=await x.bootstrapPeer();await ready(x);const q=x.request(peer.desktop,1,'peer-revision',peer.observation);q.reconcile!.association=peer.association;q.reconcile!.proof=peer.proof;assert.equal((await x.send(peer.desktop,q)).world.state,'waiting');
 const original=x.fixture.call;let changed=false,submissions=0;
 x.fixture.call=async(...args)=>{const response=await original(...args),input=args[1] as any;
  if(input?.finalStart)submissions++;
  if(input?.inspectionList&&!JSON.parse(response.body).inspectionNextDesktopId&&!changed){changed=true;q.requestId+='x';q.reconcile!.observation={...peer.observation,observationRevision:2};assert.equal((await x.send(peer.desktop,q)).world.state,'waiting');}
  return response;
 };
 const result=await x.start({selectedWorldIds:[x.association.worldId!],excludedDesktopIds:[peer.desktop.sessionId!]});assert.ok(changed);assert.equal(submissions,1);assert.equal(result.room.state,'running');assert.equal(latest(x).outcome,'committed');
 const release=(await x.fixture.service.db.transaction(tx=>tx.query('SELECT selected_reports FROM inspected_releases WHERE run_id=$1',[x.room.runId]))).rows[0].selected_reports;assert.deepEqual(release.participants.map((p:any)=>p.worldToken),[x.association.worldId]);assert.deepEqual(release.excludedDesktopIds,[peer.desktop.sessionId]);
}));

test('selected inspection changed during pagination still rejects once without re-review or automatic resubmission',()=>run('selected-report-changed',async x=>{
 await ready(x);const original=x.fixture.call;let changed=false,submissions=0;
 x.fixture.call=async(...args)=>{const response=await original(...args),input=args[1] as any;if(input?.finalStart)submissions++;
  if(input?.inspectionList&&!JSON.parse(response.body).inspectionNextDesktopId&&!changed){changed=true;const q=x.request();q.reconcile!.observation={...x.observation,observationRevision:2};assert.equal((await x.send(x.desktop,q)).world.state,'waiting');}return response;
 };
 await assert.rejects(x.start(),/REPORT_CHANGED/);assert.equal(submissions,1);assert.equal(latest(x).outcome,'not-committed');assert.equal((await x.state()).participants.length,0);
}));

test('room roster changes during inspection require a new review before any Start submission',()=>run('reviewed-roster-changed',async x=>{
 await ready(x);const original=x.fixture.call;let changed=false,submissions=0;
 x.fixture.call=async(...args)=>{const response=await original(...args),input=args[1] as any;if(input?.finalStart)submissions++;
  if(input?.inspectionList&&!JSON.parse(response.body).inspectionNextDesktopId&&!changed){changed=true;await x.bootstrapPeer();}return response;
 };
 await assert.rejects(x.start(),/FIXTURE_START_INTENT_CHANGED/);assert.equal(submissions,0);assert.equal(latest(x).failedStage,'roomRecheck');assert.equal(latest(x).outcome,'not-submitted');
}));

test('explicit reviewed Start converges after a definite metadata-only report rejection using a new original',()=>run('reviewed-start-converges',async x=>{
 await ready(x);const original=x.fixture.call,submitted:any[]=[];let changed=false;
 x.fixture.call=async(...args)=>{const response=await original(...args),input=args[1] as any;if(input?.finalStart)submitted.push(structuredClone(input));
  if(input?.inspectionList&&!JSON.parse(response.body).inspectionNextDesktopId&&!changed){changed=true;const q=x.request();q.reconcile!.observation={...x.observation,observationRevision:2};await x.send(x.desktop,q);}return response;
 };
 await x.startReviewed();const d=x.startDiagnostics();assert.equal(submitted.length,2);assert.notEqual(submitted[0].requestId,submitted[1].requestId);assert.equal(d[0]!.outcome,'not-committed');assert.equal(d[1]!.outcome,'committed');assert.equal(d[1]!.review!.previousOperationId,d[0]!.operationId);assert.ok(d[1]!.review!.inspectionChanges[0]!.fields.includes('observationRevision'));assert.ok(d[1]!.review!.inspectionChanges[0]!.fields.every(k=>['observationRevision','capturedAt','presenceUntil','validUntil'].includes(k)));
 assert.equal(submitted[0].finalStart.participants[0].observationRevision,1);assert.equal(submitted[1].finalStart.participants[0].observationRevision,2);assert.equal(submitted[0].finalStart.participants[0].contentDigest,submitted[1].finalStart.participants[0].contentDigest);
}));

test('explicit re-review surfaces meaningful report content change before a new Start original',()=>run('reviewed-start-content-change',async x=>{
 const waiting=await x.send(x.desktop,x.request());const original=x.fixture.call;let changed=false,submissions=0;
 x.fixture.call=async(...args)=>{const response=await original(...args),input=args[1] as any;if(input?.finalStart)submissions++;
  if(input?.inspectionList&&!JSON.parse(response.body).inspectionNextDesktopId&&!changed){changed=true;const attachment=(await x.fixture.service.db.transaction(tx=>tx.query('SELECT id FROM attachments WHERE game_session_id=$1',[waiting.credentials.sessionId]))).rows[0];await x.rpc(waiting.credentials,{gameObservation:{...x.observation,attachmentId:attachment.id,observationRevision:2,activeMods:['noita_together2','changed-mod']}});}return response;
 };
 await assert.rejects(x.startReviewed(),/FIXTURE_START_INSPECTION_CHANGED/);assert.equal(submissions,1);assert.equal(latest(x).outcome,'not-submitted');assert.ok(latest(x).review!.inspectionChanges[0]!.fields.includes('contentDigest'));assert.equal((await x.state()).participants.length,0);
}));

test('explicit re-review never expands the original host roster/selection after a rejection',()=>run('reviewed-start-roster-change',async x=>{
 await ready(x);const original=x.fixture.call;let changed=false,submissions=0;
 x.fixture.call=async(...args)=>{const response=await original(...args),input=args[1] as any;
  if(input?.inspectionList&&!JSON.parse(response.body).inspectionNextDesktopId&&!changed){changed=true;const q=x.request();q.reconcile!.observation={...x.observation,observationRevision:2};await x.send(x.desktop,q);}
  if(input?.finalStart){submissions++;assert.equal(JSON.parse(response.body).error.code,'REPORT_CHANGED');await x.bootstrapPeer();}return response;
 };
 await assert.rejects(x.startReviewed(),/FIXTURE_START_INTENT_CHANGED/);assert.equal(submissions,1);assert.equal(latest(x).outcome,'not-submitted');
}));

for(const committed of [false,true])test('explicit reviewed Start never resubmits '+(committed?'committed':'unknown')+' transport outcome',()=>run('reviewed-start-'+(committed?'committed':'unknown'),async x=>{
 await ready(x);const original=x.fixture.call;let submissions=0;
 x.fixture.call=async(...args)=>{if((args[1] as any)?.finalStart){submissions++;if(committed)await original(...args);throw Error('ECONNRESET');}return original(...args);};
 await assert.rejects(x.startReviewed(),/ECONNRESET/);assert.equal(submissions,1);assert.equal(x.startDiagnostics().length,1);assert.equal(latest(x).outcome,committed?'committed':'unknown');if(!committed){await assert.rejects(x.startReviewed(),/FIXTURE_START_UNCERTAIN/);assert.equal(submissions,1);}
}));

test('explicit reviewed Start bounds continued metadata churn to three definitely rejected originals',()=>run('reviewed-start-bounded',async x=>{
 await ready(x);const original=x.fixture.call;let revision=1;const ids:string[]=[];
 x.fixture.call=async(...args)=>{const response=await original(...args),input=args[1] as any;if(input?.finalStart)ids.push(input.requestId);
  if(input?.inspectionList&&!JSON.parse(response.body).inspectionNextDesktopId){const q=x.request();q.reconcile!.observation={...x.observation,observationRevision:++revision};await x.send(x.desktop,q);}return response;
 };
 await assert.rejects(x.startReviewed(),/FIXTURE_START_REVIEW_EXHAUSTED/);assert.equal(ids.length,3);assert.equal(new Set(ids).size,3);assert.ok(x.startDiagnostics().every(d=>d.outcome==='not-committed'));assert.equal(latest(x).review!.attempt,3);assert.equal((await x.state()).participants.length,0);
}));

test('explicit reviewed Start refuses a noncorrelated rejection even when its original did not commit',()=>run('reviewed-start-wrong-correlation',async x=>{
 await ready(x);const original=x.fixture.call;let changed=false,submissions=0;
 x.fixture.call=async(...args)=>{const response=await original(...args),input=args[1] as any;
  if(input?.inspectionList&&!JSON.parse(response.body).inspectionNextDesktopId&&!changed){changed=true;const q=x.request();q.reconcile!.observation={...x.observation,observationRevision:2};await x.send(x.desktop,q);}
  if(input?.finalStart){submissions++;const body=JSON.parse(response.body);assert.equal(body.error.code,'REPORT_CHANGED');return {...response,body:JSON.stringify({...body,requestId:'unrelated-response'})};}return response;
 };
 await assert.rejects(x.startReviewed(),/REPORT_CHANGED/);assert.equal(submissions,1);assert.equal(x.startDiagnostics().length,1);assert.equal(latest(x).outcome,'not-committed');
}));

test('Start renews expired host after real two-peer auth/loaded inspection with bounded optional stall',()=>run('expired-host-two-peer',async x=>{
 const peer=await x.bootstrapPeer();x.advance(901000);
 const consumer=(await x.rpc(undefined,{refresh:{secret:x.desktop.refreshSecret,deviceId:x.desktop.deviceId}})).credentials;
 const sam=(await x.rpc(undefined,{refresh:{secret:peer.desktop.refreshSecret,deviceId:peer.desktop.deviceId}})).credentials;
 assert.equal((await x.send(consumer,x.request(consumer))).world.state,'waiting');
 const q=x.request(sam,1,'peer',peer.observation);q.reconcile!.association=peer.association;q.reconcile!.proof=peer.proof;
 assert.equal((await x.send(sam,q)).world.state,'waiting');x.faults.optionalStall(50);
 const result=await x.start(undefined,async()=>{});assert.equal(result.participants.length,2);
 const d=latest(x);assert.equal(d.outcome,'committed');assert.equal(d.failedStage,undefined);assert.ok(d.stages.find(s=>s.stage==='host-renewal')?.requestId);assert.ok(d.stages.filter(s=>s.stage==='inspectionList').length>=2);
 assert.equal(d.stages.find(s=>s.stage==='finalStart')?.requestId,d.startRequestId);
 for(const secret of [consumer.accessToken,sam.accessToken,x.host.refreshSecret,x.proof,peer.proof])assert.ok(!JSON.stringify(d).includes(secret));
}));

test('optional inspection stall hits existing HTTP deadline before Start submission',()=>run('optional-inspection-deadline',async x=>{
 await ready(x);x.faults.optionalStall(6000);await assert.rejects(x.start(),/Fixture deadline exceeded/);
 const d=latest(x);assert.equal(d.failedStage,'inspectionList');assert.equal(d.category,'deadline');assert.equal(d.outcome,'not-submitted');assert.equal(d.startRequestId,undefined);
 x.faults.optionalStall(0);await x.start();assert.equal(latest(x).outcome,'committed');
}));

test('actual SQL failure in finalStart projection rolls back and retains exact transaction category',()=>run('finalStart-sql-rollback',async x=>{
 await ready(x);const previous=x.fixture.service.db.transactionProjection!;
 x.fixture.service.db.transactionProjection={...previous,prepare:async tx=>{await previous.prepare(tx);if(latest(x).stages.at(-1)?.stage==='finalStart')await tx.query('SELECT 1/0');}};
 try{await assert.rejects(x.start(),/SERVICE_UNAVAILABLE/);}finally{x.fixture.service.db.transactionProjection=previous;}
 const d=latest(x);assert.equal(d.failedStage,'finalStart');assert.equal(d.category,'transaction');assert.equal(d.outcome,'not-committed');assert.ok(d.stages.find(s=>s.stage==='finalStart')?.transactions.some(t=>t.sqlState==='22012'&&t.phase==='projection'&&t.commit==='not-attempted'));
 assert.equal((await x.state()).participants.length,0);
}));

test('lost finalStart reply is correlated to durable commit without resending Start',()=>run('committed-reply-loss',async x=>{
 await ready(x);const original=x.fixture.call;let submitted=0;
 x.fixture.call=async(...args)=>{const response=await original(...args);if((args[1] as any)?.finalStart){submitted++;throw Error('ECONNRESET');}return response;};
 await assert.rejects(x.start(),/ECONNRESET/);assert.equal(submitted,1);assert.equal(latest(x).outcome,'committed');assert.equal(latest(x).failedStage,'finalStart');assert.equal(latest(x).category,'transport-cut');assert.equal((await x.state()).participants.length,1);
}));

test('postcommit state SQL failure preserves committed Start and the failing stage',()=>run('postcommit-state',async x=>{
 await ready(x);const previous=x.fixture.service.db.transactionProjection!;
 x.fixture.service.db.transactionProjection={...previous,prepare:async tx=>{await previous.prepare(tx);if(latest(x).stages.at(-1)?.stage==='state')await tx.query('SELECT 1/0');}};
 try{await assert.rejects(x.start(),/SERVICE_UNAVAILABLE/);}finally{x.fixture.service.db.transactionProjection=previous;}
 assert.equal(latest(x).failedStage,'state');assert.equal(latest(x).category,'postcommit-control');assert.equal(latest(x).outcome,'committed');assert.ok(latest(x).stages.find(s=>s.stage==='state')?.transactions.some(t=>t.sqlState==='22012'));
}));

test('service policy rejection remains distinct from database failure',()=>run('host-auth-policy',async x=>{
 await ready(x);await x.fixture.service.db.transaction(tx=>tx.query('UPDATE sessions SET revoked_at=now() WHERE id=$1',[x.host.sessionId]));
 await assert.rejects(x.start(),/UNAUTHENTICATED/);assert.equal(latest(x).failedStage,'roomGet');assert.equal(latest(x).category,'service-admission');assert.equal(latest(x).outcome,'not-submitted');
}));

test('descriptor failure after Start is explicitly postcommit with durable matched outcome',()=>run('postcommit-descriptor',async x=>{
 await ready(x);await assert.rejects(x.start(undefined,async()=>{throw Error('EACCES');}),/EACCES/);
 assert.equal(latest(x).failedStage,'descriptor');assert.equal(latest(x).category,'postcommit-control');assert.equal(latest(x).outcome,'committed');assert.equal((await x.state()).participants.length,1);
}));

test('actual scoped service lock timeout and injected HTTP cut remain distinct',()=>run('lock-timeout-and-transport-cut',async x=>{
 await ready(x);const blocker=await x.fixture.service.db.pool.connect();
 try{await blocker.query('BEGIN');await blocker.query('SELECT pg_advisory_xact_lock(hashtext($1))',['nt2-g1-mutation:'+x.fixture.namespace]);await assert.rejects(x.start(),/SERVICE_UNAVAILABLE/);
  assert.equal(latest(x).failedStage,'roomGet');assert.equal(latest(x).category,'deadline');assert.ok(latest(x).stages.find(s=>s.stage==='roomGet')?.transactions.some(t=>t.sqlState==='55P03'&&t.phase==='lock'));
 }finally{await blocker.query('ROLLBACK');blocker.release();}
 await x.transport('http',true);await assert.rejects(x.start(),/SERVICE_UNAVAILABLE/);assert.equal(latest(x).category,'transport-cut');assert.equal(latest(x).outcome,'not-submitted');await x.transport('http',false);
}));

test('unresolved submitted request is unknown and fixture refuses a blind second Start',()=>run('unknown-no-retry',async x=>{
 await ready(x);const original=x.fixture.call;let submissions=0;
 x.fixture.call=async(...args)=>{if((args[1] as any)?.finalStart){submissions++;throw Error('ECONNRESET');}return original(...args);};
 await assert.rejects(x.start(),/ECONNRESET/);assert.equal(latest(x).outcome,'unknown');await assert.rejects(x.start(),/FIXTURE_START_UNCERTAIN/);assert.equal(submissions,1);
}));
