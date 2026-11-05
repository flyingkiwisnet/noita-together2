import test from 'node:test';import assert from 'node:assert/strict';import {randomUUID} from 'node:crypto';import {writeFile} from 'node:fs/promises';
import {startG3Fixture,provisionG3Fixture} from '../scripts/g3-loot01-fixture.js';
import {schema,validate,encode,decode} from '../../../packages/contracts/src/g3-waiting.js';
import {operationFingerprint} from '../../../packages/contracts/src/connection-fingerprint.js';
import {contentDigest} from '../src/g2-next/observation.js';
import {schema as lobbySchema} from '../../../packages/contracts/src/g2-next.js';
async function setup(individual=false){
 let now=Date.now();const f=await startG3Fixture({rejoin:true,loot01Runtime:true,live:true,tls:true,databaseUrl:'postgres://nt2_g1:local-fixture-only@127.0.0.1:54328/nt2_g1',now:()=>now});
 console.log(JSON.stringify({fixtureSchema:f.namespace,scope:'startup repair only'}));
 try{
  const s=await provisionG3Fixture(f,{consumerPlayer:1,...(individual?{seedPolicy:'individual-random'}:{}),beforeRpc:()=>{now+=100;}}),desktop=s.desktops[1]!,prepared=(await s.rpc(desktop,{roomGet:{}})).preparation!,worldToken='repair-world-'+randomUUID();
  const request=(c:any,p:any)=>({major:1,revision:schema.revision,serviceId:f.service.db.serviceId,serviceEpoch:f.service.db.epoch,sessionEpoch:c.sessionEpoch,requestId:randomUUID(),...p});
  const exact=async(c:any,r:any,binary=false)=>{const out=binary?decode('Response',(await f.service.app.inject({method:'POST',url:'/v3/waiting/rpc',headers:{host:new URL(f.origin).host,'content-type':'application/x-protobuf',authorization:'Bearer '+c.accessToken},payload:Buffer.from(encode('Request',r))})).rawPayload):JSON.parse((await f.call('/v3/waiting/rpc',r,c.accessToken)).body);validate('Response',out);return out as any;};
  const call=async(c:any,p:any,binary=false)=>{now+=50;const req=request(c,p);return {req,value:await exact(c,req,binary)};};
  const actualSeed=individual?'18446744073709551615':String(s.room.config!.seed),reserve=await call(desktop,{reserve:{preparationId:prepared.preparationId,worldToken,contextId:'app-persisted-attempt-context'}},true);assert.equal(reserve.value.error,undefined);
  let game:any,attach:any;
  const observation:any={...s.observations[0],attachmentId:undefined,worldToken,gameInstanceId:'repair-original-game',actualSeed};
  const attachWorld=async(observe=false)=>{const original=f.call;f.call=async(path,input,token,headers)=>{if((input as any)?.gameAttach)attach=structuredClone(input);return original(path,input,token,headers);};try{game=(await s.rpc(desktop,{gameAttach:{preparationId:prepared.preparationId!,gameInstanceId:observation.gameInstanceId,gameVersion:f.profile.compatibility.gameVersion,modVersion:f.profile.compatibility.modVersion,bridgeVersion:f.profile.compatibility.bridgeVersion,capabilities:f.profile.compatibility.capabilities}})).credentials!;observation.attachmentId=game.attachmentId;if(observe)await s.rpc(game,{gameObservation:observation});}finally{f.call=original;}return game;};
  const enroll=()=>call(desktop,{enroll:{preparationId:prepared.preparationId,attachmentId:game.attachmentId,originalAttachRequestId:attach.requestId,originalAttachFingerprint:operationFingerprint(attach),worldToken,gameInstanceId:observation.gameInstanceId,observationRevision:observation.observationRevision,contentDigest:contentDigest(observation),contextId:'app-persisted-attempt-context'}});
  const fresh=()=>f.service.db.transaction(tx=>f.service.auth.family(tx,desktop.accountId!,desktop.deviceId!,'desktop'));
  const recover=(proof=reserve.value.pendingProof)=>({pendingProof:proof,contextId:'fresh-native-contact-context',gameVersion:f.profile.compatibility.gameVersion,modVersion:f.profile.compatibility.modVersion,bridgeVersion:f.profile.compatibility.bridgeVersion,capabilities:f.profile.compatibility.capabilities,observation:JSON.parse(JSON.stringify({...observation,gameInstanceId:'repair-return-game',observationRevision:1,protected:true}))});
  const original=(c:any,r:any,kind:string)=>({accountId:c.accountId,deviceId:c.deviceId,originalSessionId:c.sessionId,originalSessionEpoch:r.sessionEpoch,originalServiceEpoch:r.serviceEpoch,requestId:r.requestId,kind,fingerprint:operationFingerprint(r)});
  return {f,s,desktop,prepared,observation,reserve,request,call,exact,attachWorld,enroll,fresh,recover,original,advance:(ms:number)=>{now+=ms;},now:()=>now};
 }catch(e){await f.close();console.log(JSON.stringify({cleanedSchema:f.namespace}));throw e;}
}
async function close(x:Awaited<ReturnType<typeof setup>>){await x.f.close();console.log(JSON.stringify({cleanedSchema:x.f.namespace}));}
for(const started of [false,true])for(const origin of ['enroll','pending'])test('Linked pending owner follows direct waiting recovery: started='+started+', origin='+origin,async()=>{
 const x=await setup();try{await x.attachWorld(true);let current=x.desktop,waiting:any,waitingProof:string,pending:any=x.reserve.value.pending;
  if(origin==='enroll'){const enrolled=(await x.enroll()).value;waiting=enrolled.waiting;waitingProof=enrolled.waitingProof;}
  else{current=await x.fresh();const promoted=(await x.call(current,{pendingId:pending.intent.pendingId,generation:pending.generation,pendingRecover:x.recover()})).value;assert.equal(promoted.error,undefined);waiting=promoted.waiting;waitingProof=promoted.waitingProof;pending=promoted.pending;}
  if(started)await x.s.start();
  const beforePending=(await x.call(current,{pendingId:pending.intent.pendingId,pendingStatus:{}})).value.pending;
  const expireOwner=async()=>{for(let tick=0;tick<3;tick++){x.advance(11000);await x.s.rpc(x.s.desktops[0],{presence:{present:true}});await x.s.rpc(x.s.games[0],{gamePulse:{attachmentId:x.s.games[0]!.attachmentId!,gameInstanceId:x.s.observations[0]!.gameInstanceId!,worldToken:x.s.observations[0]!.worldToken!}});}};
  const fresh=await x.fresh();await expireOwner();
  const waitingRecovery=await x.call(fresh,{waitingId:waiting.intent.waitingId,generation:waiting.generation,recover:{...x.recover(),pendingProof:undefined,waitingProof,observation:{...x.recover().observation,attachmentId:waiting.owner.attachmentId}}});assert.equal(waitingRecovery.value.error,undefined);
  const after=(await x.call(fresh,{pendingId:pending.intent.pendingId,pendingStatus:{}})).value.pending;assert.equal(after.state,'promoted');assert.equal(after.generation,beforePending.generation+1);assert.deepEqual(after.intent,beforePending.intent);
  const duplicate=await x.exact(fresh,waitingRecovery.req);assert.equal(duplicate.credentials.attachmentId,waitingRecovery.value.credentials.attachmentId);assert.equal((await x.call(fresh,{pendingId:pending.intent.pendingId,pendingStatus:{}})).value.pending.generation,after.generation);
  assert.equal((await x.call(fresh,{pendingId:pending.intent.pendingId,generation:beforePending.generation,pendingOffline:{pendingProof:x.reserve.value.pendingProof}})).value.error.code,'STALE_EPOCH');
  if(origin==='enroll'){
   const offline=await x.call(fresh,{pendingId:pending.intent.pendingId,generation:after.generation,pendingOffline:{pendingProof:x.reserve.value.pendingProof}});assert.equal(offline.value.error,undefined);assert.equal(offline.value.pending.reason,'explicit_offline');assert.equal((await x.exact(fresh,offline.req)).pendingOutcome.originalOutcome,'committed');const old=await x.exact(fresh,waitingRecovery.req);assert.equal(old.credentials,undefined);assert.equal(old.outcome.originalOutcome,'committed');
   const peer=await x.s.rpc(x.s.games[0],{roomGet:{}});if(started)assert.equal(peer.canonicalAdmission!.decision!.authorized,true);else assert.equal(peer.gameStatus!.phase,'hold');
   await writeFile('docs/g3/server-startup-repair-linked-'+started+'-'+origin+'-fixture.json',JSON.stringify({engineFacts:'synthetic',before:beforePending,afterDirect:after,offline:offline.value.pending},null,2)+'\n');return;
  }
  const next=await x.fresh();await expireOwner();const pendingRecovery=await x.call(next,{pendingId:pending.intent.pendingId,generation:after.generation,pendingRecover:{...x.recover(),observation:{...x.recover().observation,attachmentId:waitingRecovery.value.credentials.attachmentId}}});assert.equal(pendingRecovery.value.error,undefined);assert.equal(pendingRecovery.value.pending.generation,after.generation+1);assert.equal(pendingRecovery.value.waiting.mayPlay,false);
  if(started){assert.equal(pendingRecovery.value.waitingProof,undefined);assert.ok(pendingRecovery.value.resumeProof);assert.ok(pendingRecovery.value.participant.association.originalStartId);}else{assert.ok(pendingRecovery.value.waitingProof);assert.equal(pendingRecovery.value.resumeProof,undefined);}
  const currentWaiting=(await x.call(next,{waitingId:waiting.intent.waitingId,status:{}})).value.waiting;assert.equal(currentWaiting.owner.attachmentId,pendingRecovery.value.credentials.attachmentId);
  const offline=await x.call(next,{pendingId:pending.intent.pendingId,generation:pendingRecovery.value.pending.generation,pendingOffline:{pendingProof:x.reserve.value.pendingProof}});assert.equal(offline.value.error,undefined);assert.equal(offline.value.pending.state,'terminal');assert.equal(offline.value.pending.reason,'explicit_offline');assert.equal((await x.exact(next,offline.req)).pendingOutcome.originalOutcome,'committed');
  const oldReply=await x.exact(next,pendingRecovery.req);assert.equal(oldReply.credentials,undefined);assert.equal(oldReply.resumeProof,undefined);assert.equal(oldReply.pendingOutcome.originalOutcome,'committed');
  const peer=await x.s.rpc(x.s.games[0],{roomGet:{}});if(started)assert.equal(peer.canonicalAdmission!.decision!.authorized,true);else assert.equal(peer.gameStatus!.phase,'hold');
  await writeFile('docs/g3/server-startup-repair-linked-'+started+'-'+origin+'-fixture.json',JSON.stringify({engineFacts:'synthetic',before:beforePending,afterDirect:after,afterPending:pendingRecovery.value.pending,currentWaiting,participant:pendingRecovery.value.participant,offline:offline.value.pending},null,2)+'\n');
 }finally{await close(x);}
});
test('Pending status and original reserve retry expose current issued, consumed and loaded preparation state',async()=>{
 const x=await setup();try{const id=x.reserve.value.pending.intent.pendingId;
  assert.equal(x.reserve.value.pending.preparationState,'issued');await x.attachWorld();
  const consumed=(await x.call(x.desktop,{pendingId:id,pendingStatus:{}})).value;assert.equal(consumed.pending.preparationState,'consumed');assert.equal(consumed.pendingProof,x.reserve.value.pendingProof);
  assert.equal((await x.exact(x.desktop,x.reserve.req,true)).pending.preparationState,'consumed');
  const issuedSnapshot=x.reserve.value.pending;assert.equal(issuedSnapshot.preparationState,'issued');assert.notEqual(consumed.pending.preparationState,issuedSnapshot.preparationState);
  // A recovery acknowledges a complete protected inspection and makes loaded
  // visible even when the original reserve operation is retried afterwards.
  const current=await x.fresh(),loaded=(await x.call(current,{pendingId:id,generation:1,pendingRecover:x.recover()})).value;assert.equal(loaded.error,undefined);assert.equal(loaded.pending.preparationState,'loaded');
  assert.equal((await x.call(current,{pendingId:id,pendingStatus:{}})).value.pending.preparationState,'loaded');
  await writeFile('docs/g3/server-startup-repair-preparation-state-fixture.json',JSON.stringify({engineFacts:'synthetic',issued:issuedSnapshot,consumed:consumed.pending,loaded:loaded.pending},null,2)+'\n');
 }finally{await close(x);}
});
test('Opaque lost attach and repeated recovery replies reconcile only saved originals and public outcome/status',async()=>{
 const x=await setup();try{const {f,s,desktop}=x,pendingId=x.reserve.value.pending.intent.pendingId;
  const savedAttach={major:1,revision:lobbySchema.revision,serviceId:f.service.db.serviceId,serviceEpoch:f.service.db.epoch,sessionEpoch:desktop.sessionEpoch,requestId:randomUUID(),roomId:s.room.roomId,runId:s.room.runId,runEpoch:s.room.runEpoch,configRevision:s.room.configRevision,expectedRevision:s.room.revision,gameAttach:{preparationId:x.prepared.preparationId,gameInstanceId:'opaque-original-instance',gameVersion:f.profile.compatibility.gameVersion,modVersion:f.profile.compatibility.modVersion,bridgeVersion:f.profile.compatibility.bridgeVersion,capabilities:f.profile.compatibility.capabilities}};
  // Intentionally discard the entire reply without parsing or retaining any
  // credential/attachment. Nothing below reads consumed_operation or SQL.
  await f.call('/v1/rpc',savedAttach,desktop.accessToken).then(()=>undefined);
  let current=await x.fresh();const originalAttach=x.original(desktop,savedAttach,'gameAttach');
  const attachOutcome=(await x.call(current,{pendingOutcome:originalAttach})).value;assert.equal(attachOutcome.pendingOutcome.originalOutcome,'committed');assert.ok(attachOutcome.pendingOutcome.effectId);assert.equal(attachOutcome.credentials,undefined);
  const originalAttachment=attachOutcome.pendingOutcome.effectId;let ownerAttachment=originalAttachment,publicState=(await x.call(current,{pendingId,pendingStatus:{}})).value.pending;
  assert.equal(publicState.preparationState,'consumed');const snapshots:any[]=[];
  for(let attempt=0;attempt<2;attempt++){
   const savedRecovery=x.request(current,{pendingId,generation:publicState.generation,pendingRecover:{...x.recover(),observation:{...x.recover().observation,attachmentId:ownerAttachment,gameInstanceId:attempt===0?savedAttach.gameAttach.gameInstanceId:'opaque-return-instance'}}});
   await f.call('/v3/waiting/rpc',savedRecovery,current.accessToken).then(()=>undefined);
   const outcome=(await x.call(current,{pendingOutcome:x.original(current,savedRecovery,'pendingRecover')})).value;assert.equal(outcome.pendingOutcome.originalOutcome,'committed');assert.equal(outcome.credentials,undefined);assert.equal(outcome.waitingProof,undefined);
   publicState=(await x.call(current,{pendingId,pendingStatus:{}})).value.pending;assert.equal(publicState.preparationState,'loaded');assert.ok(publicState.waitingId);
   const status=(await x.call(current,{waitingId:publicState.waitingId,status:{}})).value;ownerAttachment=status.waiting.owner.attachmentId;
   assert.equal(ownerAttachment,outcome.pendingOutcome.effectId);assert.notEqual(ownerAttachment,originalAttachment);assert.equal(status.waiting.mayPlay,false);
   const historical=(await x.call(current,{pendingOutcome:originalAttach})).value;assert.equal(historical.pendingOutcome.effectId,originalAttachment);assert.notEqual(historical.pendingOutcome.effectId,ownerAttachment);
   snapshots.push({pending:publicState,waiting:status.waiting,outcome:outcome.pendingOutcome,historicalAttach:historical.pendingOutcome});
   if(attempt===0)current=await x.fresh();
  }
  assert.notEqual(snapshots[0].waiting.owner.attachmentId,snapshots[1].waiting.owner.attachmentId);
  await writeFile('docs/g3/server-startup-repair-opaque-lost-reply-fixture.json',JSON.stringify({engineFacts:'synthetic',originalAttach:attachOutcome.pendingOutcome,snapshots},null,2)+'\n');
 }finally{await close(x);}
});
test('Superseded unconsumed reservation permits a distinct issued replacement without rewriting history or cancelling its world',async()=>{
 const x=await setup();try{const first=x.reserve.value.pending,firstProof=x.reserve.value.pendingProof,current=await x.fresh();
  await x.s.rpc(current,{roomJoin:{}});const next=(await x.s.rpc(current,{roomGet:{}})).preparation!;assert.notEqual(next.preparationId,x.prepared.preparationId);assert.equal(next.state,'issued');
  const nextWorld='new-world-'+randomUUID(),second=await x.call(current,{reserve:{preparationId:next.preparationId,worldToken:nextWorld,contextId:'replacement-attempt'}});assert.equal(second.value.error,undefined);assert.equal(second.value.pending.preparationState,'issued');assert.notEqual(second.value.pending.intent.pendingId,first.intent.pendingId);
  const oldStatus=(await x.call(current,{pendingId:first.intent.pendingId,pendingStatus:{}})).value;assert.equal(oldStatus.pending.state,'terminal');assert.equal(oldStatus.pendingProof,undefined);assert.deepEqual(oldStatus.pending.intent,first.intent);
  const oldOriginal=x.original(x.desktop,x.reserve.req,'reserve'),oldOutcome=(await x.call(current,{pendingOutcome:oldOriginal})).value;assert.equal(oldOutcome.pendingOutcome.originalOutcome,'committed');assert.equal(oldOutcome.pendingOutcome.pendingId,first.intent.pendingId);
  assert.equal((await x.call(current,{pendingId:first.intent.pendingId,generation:1,pendingRecover:x.recover(firstProof)})).value.error.code,'RETURN_INELIGIBLE');
  const oldOffline=await x.call(current,{pendingId:first.intent.pendingId,generation:1,pendingOffline:{pendingProof:firstProof}});assert.equal(oldOffline.value.error,undefined);
  assert.equal((await x.call(current,{pendingId:second.value.pending.intent.pendingId,pendingStatus:{}})).value.pending.preparationState,'issued');
  const recovered=(await x.call(current,{pendingId:second.value.pending.intent.pendingId,generation:1,pendingRecover:{...x.recover(second.value.pendingProof),observation:{...x.recover().observation,worldToken:nextWorld}}})).value;assert.equal(recovered.error,undefined);assert.equal(recovered.waiting.intent.preparationId,next.preparationId);assert.equal(recovered.waiting.intent.worldToken,nextWorld);
  await x.exact(current,oldOffline.req);const healthy=(await x.call(current,{waitingId:recovered.waiting.intent.waitingId,status:{}})).value;assert.equal(healthy.waiting.currentReport.loaded,true);assert.equal(healthy.waiting.state,'waiting');
  await writeFile('docs/g3/server-startup-repair-replacement-fixture.json',JSON.stringify({engineFacts:'synthetic',first,retired:oldStatus.pending,oldOutcome:oldOutcome.pendingOutcome,replacement:second.value.pending,recovered:recovered.pending},null,2)+'\n');
 }finally{await close(x);}
});
test('A newer issued preparation cannot silently replace a previously consumed reserved world',async()=>{
 const x=await setup();try{await x.attachWorld();const current=await x.fresh();await x.s.rpc(current,{roomJoin:{}});const next=(await x.s.rpc(current,{roomGet:{}})).preparation!;
  const refused=(await x.call(current,{reserve:{preparationId:next.preparationId,worldToken:'do-not-replace-consumed',contextId:'new-attempt'}})).value;assert.equal(refused.error.code,'ORIGINAL_WORLD_UNRESOLVED');assert.equal(refused.pendingProof,undefined);
  assert.equal((await x.call(current,{pendingOutcome:x.original(x.desktop,x.reserve.req,'reserve')})).value.pendingOutcome.originalOutcome,'committed');
 }finally{await close(x);}
});
test('Pending reservation precedes attach: exact outcomes, false-only hold, original expiry and protected individual-world recovery',async()=>{
 const x=await setup(true);try{const {f,s,desktop,call,reserve}=x,id=reserve.value.pending.intent.pendingId,proof=reserve.value.pendingProof;
  assert.equal(reserve.value.pending.mayPlay,false);assert.equal(reserve.value.pending.intent.actualSeed,undefined);assert.equal(reserve.value.pending.boundActualSeed,undefined);assert.equal(reserve.value.pending.sample.originRequestId,reserve.req.requestId);assert.equal(reserve.value.pending.sample.credentialId,reserve.value.credentialValidity.credentialId);
  assert.equal((await x.exact(desktop,reserve.req)).pendingProof,proof);
  assert.equal((await call(desktop,{reserve:{...reserve.req.reserve,worldToken:'other'}})).value.error.code,'CONFLICT');
  assert.equal((await call(s.desktops[0],{pendingId:id,pendingStatus:{}})).value.error.code,'FORBIDDEN');
  const known=(await call(desktop,{pendingOutcome:x.original(desktop,reserve.req,'reserve')})).value;assert.equal(known.pendingOutcome.originalOutcome,'committed');assert.equal(known.pendingProof,undefined);assert.equal(known.credentials,undefined);
  assert.equal((await call(desktop,{pendingOutcome:{...x.original(desktop,reserve.req,'reserve'),requestId:randomUUID()}})).value.pendingOutcome.originalOutcome,'unresolved');
  x.advance(1000);const status=(await call(desktop,{pendingId:id,pendingStatus:{}})).value;assert.ok(status.pending.preparationRemainingMs<reserve.value.pending.preparationRemainingMs);
  const current=await x.fresh();assert.equal((await call(current,{pendingId:id,pendingStatus:{}})).value.pendingProof,undefined);
  const req={pendingId:id,generation:1,pendingRecover:x.recover()};assert.equal(req.pendingRecover.observation.attachmentId,undefined);
  assert.equal((await call(current,{...req,pendingRecover:{...req.pendingRecover,pendingProof:'x'.repeat(43)}})).value.error.code,'FORBIDDEN');
  assert.equal((await call(current,{...req,pendingRecover:{...req.pendingRecover,observation:{...req.pendingRecover.observation,protected:false}}})).value.error.code,'REPORT_CHANGED');
  const recovered=await call(current,req,true);assert.equal(recovered.value.error,undefined);assert.equal(recovered.value.waiting.mayPlay,false);assert.equal(recovered.value.pending.generation,2);assert.equal(recovered.value.canonicalAdmission,undefined);assert.ok(recovered.value.waitingProof);assert.equal(recovered.value.waiting.intent.originalAttachRequestId,undefined);assert.equal(recovered.value.waiting.intent.pendingReservationId,id);assert.equal(recovered.value.pending.boundActualSeed,'18446744073709551615');const boundObs={...req.pendingRecover.observation,attachmentId:recovered.value.credentials.attachmentId,observationRevision:2};await assert.rejects(x.s.rpc(recovered.value.credentials,{gameObservation:{...boundObs,actualSeed:'1'}}),/REPORT_CHANGED/);assert.equal((await x.s.rpc(recovered.value.credentials,{gameObservation:boundObs})).error,undefined);
  const retry=await x.exact(current,recovered.req);assert.equal(retry.credentials.attachmentId,recovered.value.credentials.attachmentId);
  const changed=await x.fresh(),outcome=(await call(changed,{pendingOutcome:x.original(current,recovered.req,'pendingRecover')})).value;assert.equal(outcome.pendingOutcome.originalOutcome,'committed');assert.equal(outcome.credentials,undefined);assert.equal(outcome.waitingProof,undefined);assert.equal(outcome.pendingProof,undefined);
  assert.equal((await f.service.db.transaction(tx=>tx.query('SELECT count(*) FROM preparations WHERE id=$1',[x.prepared.preparationId]))).rows[0].count,'1');
  await writeFile('docs/g3/server-startup-repair-pending-fixture.json',JSON.stringify({engineFacts:'synthetic; no engine save evidence',reserve:{...reserve.value,pendingProof:undefined},recovered:{...recovered.value,credentials:undefined,waitingProof:undefined},outcome},null,2)+'\n');
 }finally{await close(x);}
});
for(const cut of ['attach_reply_lost','inspection_acknowledged','enrollment_reply_lost'])test('Pending recovery preserves consumed original across '+cut,async()=>{
 const x=await setup();try{await x.attachWorld(cut!=='attach_reply_lost');if(cut==='enrollment_reply_lost')assert.equal((await x.enroll()).value.error,undefined);
  const before=(await x.f.service.db.transaction(tx=>tx.query('SELECT consumed_operation FROM preparations WHERE id=$1',[x.prepared.preparationId]))).rows[0].consumed_operation;
  const current=await x.fresh(),result=(await x.call(current,{pendingId:x.reserve.value.pending.intent.pendingId,generation:1,pendingRecover:x.recover()})).value;assert.equal(result.error,undefined);assert.equal(result.waiting.mayPlay,false);assert.equal(result.waiting.intent.worldToken,x.observation.worldToken);assert.equal(result.waiting.intent.originalAttachRequestId,before.requestId);
  assert.deepEqual((await x.f.service.db.transaction(tx=>tx.query('SELECT consumed_operation FROM preparations WHERE id=$1',[x.prepared.preparationId]))).rows[0].consumed_operation,before);const originalAttach={accountId:x.desktop.accountId,deviceId:x.desktop.deviceId,originalSessionId:before.sessionId,originalSessionEpoch:before.sessionEpoch,originalServiceEpoch:before.serviceEpoch,requestId:before.requestId,kind:'gameAttach',fingerprint:before.fingerprint};const attachOutcome=(await x.call(current,{pendingOutcome:originalAttach})).value;assert.equal(attachOutcome.pendingOutcome.originalOutcome,'committed');assert.equal(attachOutcome.pendingOutcome.effectId,before.attachmentId);assert.equal(attachOutcome.credentials,undefined);assert.equal((await x.call(current,{pendingOutcome:{...originalAttach,fingerprint:'0'.repeat(64)}})).value.pendingOutcome.originalOutcome,'unresolved');
  await assert.rejects(x.s.rpc(x.desktop,{roomGet:{}}),/UNAUTHENTICATED/);
 }finally{await close(x);}
});
for(const enrolled of [false,true])test('Originally selected world returns after real Start through existing admission; enrolled='+enrolled,async()=>{
 const x=await setup();try{await x.attachWorld(true);let waiting:any;if(enrolled)waiting=(await x.enroll()).value;await x.s.start();
  const p=(await x.f.service.db.transaction(tx=>tx.query('SELECT id,original_report FROM rejoin_participants WHERE account_id=$1',[x.desktop.accountId]))).rows[0];assert.ok(p);
  const current=await x.fresh();const scope={pendingId:x.reserve.value.pending.intent.pendingId,generation:1,pendingRecover:x.recover()};assert.equal((await x.call(current,scope)).value.error.code,'OWNER_ACTIVE');for(let tick=0;tick<3;tick++){x.advance(11000);await x.s.rpc(x.s.desktops[0],{presence:{present:true}});await x.s.rpc(x.s.games[0],{gamePulse:{attachmentId:x.s.games[0]!.attachmentId!,gameInstanceId:x.s.observations[0]!.gameInstanceId!,worldToken:x.s.observations[0]!.worldToken!}});}const result=await x.call(current,{pendingId:x.reserve.value.pending.intent.pendingId,generation:1,pendingRecover:x.recover()});assert.equal(result.value.error,undefined);assert.equal(result.value.participant.association.participantId,p.id);assert.equal(result.value.participant.generation,2);assert.ok(result.value.participant.association.originalStartId);assert.ok(result.value.resumeProof);assert.equal(result.value.canonicalAdmission.decision.authorized,true);assert.equal(result.value.waiting.mayPlay,false);
  const repeated=await x.exact(current,result.req);assert.equal(repeated.credentials.attachmentId,result.value.credentials.attachmentId);
  const original=(await x.f.service.db.transaction(tx=>tx.query('SELECT original_report FROM rejoin_participants WHERE id=$1',[p.id]))).rows[0].original_report;assert.deepEqual(original,p.original_report);
  await x.s.rpc(result.value.credentials,{gameObservation:{...x.recover().observation,attachmentId:result.value.credentials.attachmentId,observationRevision:2,protected:false}});assert.equal((await x.s.rpc(result.value.credentials,{roomGet:{}})).canonicalAdmission!.decision!.playing,true);
  assert.equal((await x.s.rpc(x.s.games[0],{roomGet:{}})).canonicalAdmission!.decision!.authorized,true);
  await writeFile('docs/g3/server-startup-repair-selected-'+enrolled+'-fixture.json',JSON.stringify({engineFacts:'synthetic',pending:result.value.pending,waiting:result.value.waiting,participant:result.value.participant,canonicalAdmission:result.value.canonicalAdmission},null,2)+'\n');
 }finally{await close(x);}
});
test('Excluded world remains excluded after actual Start; explicit offline prevents pending resurrection',async()=>{
 const x=await setup();try{await x.attachWorld(true);const enrolled=(await x.enroll()).value;await x.s.rpc(x.s.host,{roomGet:{}});const peer=x.s.reports[0]!;
  await x.s.rpc(x.s.host,{finalStart:{participants:[{desktopId:peer.desktopId!,attachmentId:peer.attachmentId!,worldToken:peer.worldToken!,observationRevision:peer.observationRevision!,contentDigest:peer.contentDigest!}],excludedDesktopIds:[x.desktop.sessionId!]}});
  const current=await x.fresh();assert.equal((await x.call(current,{pendingId:x.reserve.value.pending.intent.pendingId,generation:1,pendingRecover:x.recover()})).value.error.code,'NOT_SELECTED_BY_START');
  assert.equal((await x.call(current,{waitingId:enrolled.waiting.intent.waitingId,generation:1,recover:{...x.recover(),pendingProof:undefined,waitingProof:enrolled.waitingProof}})).value.error.code,'NOT_SELECTED_BY_START');
  const offline=(await x.call(current,{pendingId:x.reserve.value.pending.intent.pendingId,generation:1,pendingOffline:{pendingProof:x.reserve.value.pendingProof}})).value;assert.equal(offline.error,undefined);assert.equal(offline.pending.state,'terminal');
  assert.equal((await x.call(current,{pendingId:offline.pending.intent.pendingId,generation:offline.pending.generation,pendingRecover:x.recover()})).value.error.code,'RETURN_INELIGIBLE');
  assert.equal((await x.f.service.db.transaction(tx=>tx.query('SELECT count(*) FROM rejoin_participants WHERE account_id=$1',[x.desktop.accountId]))).rows[0].count,'0');
  await writeFile('docs/g3/server-startup-repair-excluded-fixture.json',JSON.stringify({engineFacts:'synthetic',pending:offline.pending,waiting:offline.waiting},null,2)+'\n');
 }finally{await close(x);}
});
test('Pending expiry, cancellation, wrong device and current epoch remain terminal; exact outcome expiry stays unresolved',async()=>{
 const x=await setup();try{const id=x.reserve.value.pending.intent.pendingId,device=await x.f.service.db.transaction(tx=>x.f.service.auth.family(tx,x.desktop.accountId!,'other-device','desktop'));
  assert.equal((await x.call(device,{pendingId:id,pendingStatus:{}})).value.error.code,'FORBIDDEN');
  await x.f.service.db.transaction(tx=>tx.query('UPDATE startup_pending_operations SET expires_at=$2 WHERE pending_id=$1',[id,new Date(x.now()-1)]));assert.equal((await x.call(x.desktop,{pendingOutcome:x.original(x.desktop,x.reserve.req,'reserve')})).value.pendingOutcome.originalOutcome,'unresolved');
  x.advance(601000);const expired=(await x.call(x.desktop,{pendingId:id,pendingStatus:{}})).value;assert.equal(expired.pending.state,'terminal');assert.equal(expired.pending.preparationRemainingMs,0);assert.equal(expired.pendingProof,undefined);
  assert.equal((await x.call(x.desktop,{pendingId:id,generation:1,pendingRecover:x.recover()})).value.error.code,'RETURN_INELIGIBLE');
  await x.f.restart();const current=await x.fresh();assert.equal((await x.call(current,{pendingId:id,pendingStatus:{}})).value.pending.state,'terminal');
 }finally{await close(x);}
});
for(const cause of ['cancel','leave','ban','configuration','newer_preparation','offline'])test('Pending terminal fence: '+cause,async()=>{
 const x=await setup();try{const id=x.reserve.value.pending.intent.pendingId;
  if(cause==='cancel')await x.s.rpc(x.s.host,{cancelRun:{}});
  if(cause==='leave')await x.s.rpc(x.desktop,{roomLeave:{}});
  if(cause==='ban')await x.s.rpc(x.s.host,{roomModerate:{accountId:x.desktop.accountId!,action:'ban'}});
  if(cause==='configuration')await x.f.service.db.transaction(tx=>tx.query('UPDATE rooms SET config_revision=config_revision+1 WHERE id=$1',[x.s.room.roomId]));
  if(cause==='newer_preparation'){const current=await x.fresh();await x.s.rpc(current,{roomJoin:{}});await x.s.rpc(current,{roomGet:{}});}
  if(cause==='offline'){const offline=await x.call(x.desktop,{pendingId:id,generation:1,pendingOffline:{pendingProof:x.reserve.value.pendingProof}});assert.equal(offline.value.error,undefined);assert.equal((await x.exact(x.desktop,offline.req)).pending.state,'terminal');}
  const current=await x.fresh(),status=(await x.call(current,{pendingId:id,pendingStatus:{}})).value;assert.equal(status.pending.state,'terminal');assert.equal(status.pendingProof,undefined);
  assert.equal((await x.call(current,{pendingId:id,generation:status.pending.generation,pendingRecover:x.recover()})).value.error.code,'RETURN_INELIGIBLE');
 }finally{await close(x);}
});
