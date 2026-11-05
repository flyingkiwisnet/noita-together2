import test from 'node:test';import assert from 'node:assert/strict';import {randomUUID} from 'node:crypto';import {writeFile} from 'node:fs/promises';
import {startG3Fixture,provisionG3Fixture} from '../scripts/g3-loot01-fixture.js';
import {schema,validate,encode,decode} from '../../../packages/contracts/src/g3-waiting.js';
import {operationFingerprint} from '../../../packages/contracts/src/connection-fingerprint.js';
import {contentDigest} from '../src/g2-next/observation.js';
import {schema as gameSchema} from '../../../packages/contracts/src/g3.js';

async function setup(focus=0){
 let now=Date.now();const f=await startG3Fixture({rejoin:true,loot01Runtime:true,live:true,tls:true,databaseUrl:'postgres://nt2_g1:local-fixture-only@127.0.0.1:54328/nt2_g1',now:()=>now});
 console.log(JSON.stringify({fixtureSchema:f.namespace,scope:'owned waiting candidate only'}));
 try{
  const attaches:any[]=[],original=f.call;f.call=async(path,input,token,headers)=>{now+=150;if((input as any)?.gameAttach)attaches.push(structuredClone(input));return original(path,input,token,headers);};
  const s=await provisionG3Fixture(f,{beforeRpc:()=>{now+=100;}}),attach=attaches[focus],observation=s.observations[focus]!;
  const prep=(await f.service.db.transaction(tx=>tx.query('SELECT preparation_id FROM attachments WHERE id=$1',[s.games[focus]!.attachmentId]))).rows[0].preparation_id;
  const payload={preparationId:prep,attachmentId:s.games[focus]!.attachmentId,originalAttachRequestId:attach.requestId,originalAttachFingerprint:operationFingerprint(attach),worldToken:observation.worldToken,gameInstanceId:observation.gameInstanceId,observationRevision:observation.observationRevision,contentDigest:contentDigest(observation),contextId:'fixture-original-context'};
  const request=(c:any,p:any)=>({major:1,revision:schema.revision,serviceId:f.service.db.serviceId,serviceEpoch:f.service.db.epoch,sessionEpoch:c.sessionEpoch,requestId:randomUUID(),...p});
  const call=async(c:any,p:any,binary=false)=>{const req=request(c,p);if(!binary){const value=JSON.parse((await f.call('/v3/waiting/rpc',req,c.accessToken)).body);validate('Response',value);return {req,value};}const result=await f.service.app.inject({method:'POST',url:'/v3/waiting/rpc',headers:{host:new URL(f.origin).host,'content-type':'application/x-protobuf',authorization:'Bearer '+c.accessToken},payload:Buffer.from(encode('Request',req))});return {req,value:decode('Response',result.rawPayload) as any};};
  const recovery=(proof:string,attachmentId=s.games[focus]!.attachmentId)=>({waitingProof:proof,gameVersion:f.profile.compatibility.gameVersion,modVersion:f.profile.compatibility.modVersion,bridgeVersion:f.profile.compatibility.bridgeVersion,capabilities:f.profile.compatibility.capabilities,contextId:'fixture-return-context',observation:{...observation,attachmentId,gameInstanceId:'fixture-return-'+randomUUID(),observationRevision:1,protected:true}});
  return {f,s,payload,request,call,recovery,advance:(ms:number)=>{now+=ms;},now:()=>now};
 }catch(e){await f.close();throw e;}
}
test('Waiting enrollment requires real consumed operation and complete protected report; duplicate/outcome and wrong scopes',async()=>{
 const x=await setup();const {f,s,call,payload}=x;try{
  assert.equal((await call(s.desktops[1],{enroll:payload})).value.error.code,'FORBIDDEN');
  assert.equal((await call(s.desktops[0],{enroll:{...payload,worldToken:'foreign-world'}})).value.error.code,'REPORT_CHANGED');
  assert.equal((await call(s.desktops[0],{enroll:{...payload,originalAttachFingerprint:'0'.repeat(64)}})).value.error.code,'ORIGINAL_OPERATION_MISMATCH');
  assert.equal((await call(s.desktops[0],{enroll:{...payload,preparationId:randomUUID()}})).value.error.code,'STALE_EPOCH');
  await s.rpc(s.games[0],{gameObservation:{...s.observations[0]!,observationRevision:2,inventoryComplete:false,collectorFailure:'collection_failed'}});
  assert.equal((await call(s.desktops[0],{enroll:{...payload,observationRevision:2}})).value.error.code,'REPORT_CHANGED');
  await s.rpc(s.games[0],{gameObservation:{...s.observations[0]!,observationRevision:3}});payload.observationRevision=3;
  const first=await call(s.desktops[0],{enroll:payload},true);assert.equal(first.value.error,undefined);const w=first.value.waiting;assert.equal(w.mayPlay,false);assert.equal(w.intent.originalStartId,undefined);assert.equal(first.value.canonicalAdmission,undefined);assert.match(first.value.waitingProof,/^[A-Za-z0-9_-]{43}$/);
  const retry=JSON.parse((await f.call('/v3/waiting/rpc',first.req,s.desktops[0]!.accessToken)).body);assert.equal(retry.waitingProof,first.value.waitingProof);assert.equal(retry.waiting.intent.waitingId,w.intent.waitingId);assert.equal(retry.waiting.sample.sampleKey,w.sample.sampleKey);
  const original={accountId:s.desktops[0]!.accountId,deviceId:s.desktops[0]!.deviceId,originalSessionId:s.desktops[0]!.sessionId,originalSessionEpoch:s.desktops[0]!.sessionEpoch,originalServiceEpoch:f.service.db.epoch,requestId:first.req.requestId,kind:'enroll',fingerprint:operationFingerprint(first.req)};
  const known=(await call(s.desktops[0],{outcome:original})).value;assert.equal(known.outcome.originalOutcome,'committed');assert.equal(known.waitingProof,undefined);assert.equal(known.credentials,undefined);assert.equal(known.waiting.sample.originRequestId,known.requestId);
  assert.equal((await call(s.desktops[0],{outcome:{...original,requestId:randomUUID()}})).value.outcome.originalOutcome,'unresolved');
  const device=await f.service.db.transaction(tx=>f.service.auth.family(tx,s.desktops[0]!.accountId!,'wrong-device','desktop'));assert.equal((await call(device,{waitingId:w.intent.waitingId,status:{}})).value.error.code,'FORBIDDEN');
  await f.service.db.transaction(tx=>tx.query('UPDATE waiting_operations SET expires_at=$2 WHERE waiting_id=$1',[w.intent.waitingId,new Date(x.now()-1000)]));assert.equal((await call(s.desktops[0],{outcome:original})).value.outcome.originalOutcome,'unresolved');
  await writeFile('docs/g3/server-startup-repair-retained-waiting-enrollment-fixture.json',JSON.stringify({engineFacts:'synthetic, not save persistence',waiting:w,outcome:known.outcome},null,2)+'\n');
 }finally{await f.close();console.log(JSON.stringify({cleanedSchema:f.namespace}));}
});
test('Saved waiting recovery fences old generation, preserves consumed world, stays held until real Start via same canonical admission',async()=>{
 const x=await setup();const {f,s,call,payload}=x;try{
  const enrolled=(await call(s.desktops[0],{enroll:payload})).value,w=enrolled.waiting,id=w.intent.waitingId,proof=enrolled.waitingProof;
  const body=x.recovery(proof),scope={waitingId:id,generation:1};
  assert.equal((await call(s.desktops[0],{...scope,recover:body})).value.error.code,'OWNER_ACTIVE');
  for(let i=0;i<3;i++){x.advance(11000);await s.rpc(s.desktops[1],{presence:{present:true}});await s.rpc(s.games[1],{gamePulse:{attachmentId:s.games[1]!.attachmentId!,gameInstanceId:s.observations[1]!.gameInstanceId!,worldToken:s.observations[1]!.worldToken!}});}
  assert.equal((await call(s.desktops[0],{...scope,recover:{...body,observation:{...body.observation,protected:false}}})).value.error.code,'REPORT_CHANGED');
  assert.equal((await call(s.desktops[0],{...scope,recover:{...body,observation:{...body.observation,actualSeed:'1'}}})).value.error.code,'REPORT_CHANGED');
  const result=await call(s.desktops[0],{...scope,recover:body});assert.equal(result.value.error,undefined);const current=result.value.waiting,game=result.value.credentials;
  assert.equal(current.generation,2);assert.equal(current.mayPlay,false);assert.equal(current.currentReport.protected,true);assert.deepEqual(current.intent,w.intent);assert.equal(result.value.canonicalAdmission,undefined);
  const duplicate=JSON.parse((await f.call('/v3/waiting/rpc',result.req,s.desktops[0]!.accessToken)).body);assert.equal(duplicate.waiting.generation,2);assert.equal(duplicate.credentials.attachmentId,game.attachmentId);assert.equal(duplicate.waiting.sample.originRequestId,result.req.requestId);
  await assert.rejects(s.rpc(s.games[0],{roomGet:{}}),/UNAUTHENTICATED|RESTART_REQUIRED/);
  assert.equal((await call(s.desktops[0],{waitingId:id,generation:1,offline:{waitingProof:proof}})).value.error.code,'STALE_EPOCH');
  await assert.rejects(s.rpc(s.desktops[0],{preparationCancel:{preparationId:payload.preparationId!}}),/STALE_EPOCH/);
  const observed={...body.observation,attachmentId:game.attachmentId};s.games[0]=game;s.observations[0]=observed;Object.assign(s.scopes[0]!,{attachmentId:game.attachmentId});
  const hold=await s.rpc(game,{roomGet:{}});assert.equal(hold.release,undefined);assert.equal(hold.gameStatus!.phase,'hold');
  assert.equal(hold.room!.members!.find(m=>m.desktopId===s.desktops[0]!.sessionId)!.readiness,'ready');
  const blocked=JSON.parse((await f.call('/v3/rpc',{major:1,revision:gameSchema.revision,serviceId:f.service.db.serviceId,serviceEpoch:f.service.db.epoch,sessionEpoch:game.sessionEpoch,requestId:randomUUID(),scope:s.scopes[0],ghostList:{}},game.accessToken)).body);assert.ok(blocked.error,'No gameplay before actual Start');
  assert.equal((await f.service.db.transaction(tx=>tx.query('SELECT count(*) FROM preparations WHERE id=$1',[payload.preparationId]))).rows[0].count,'1');
  await s.start();const admitted=await s.rpc(game,{roomGet:{}});assert.equal(admitted.canonicalAdmission!.generation,1);assert.ok(admitted.canonicalAdmission!.originalStart!.requestId);assert.equal(admitted.canonicalAdmission!.scope!.worldToken,payload.worldToken);assert.equal(admitted.canonicalAdmission!.decision!.authorized,true);
  await s.rpc(game,{gameObservation:{...observed,observationRevision:2,protected:false}});assert.equal((await s.rpc(game,{roomGet:{}})).canonicalAdmission!.decision!.playing,true);
  assert.equal((await call(s.desktops[0],{waitingId:id,status:{}})).value.waiting.state,'started');assert.equal((await call(s.desktops[0],{waitingId:id,generation:2,recover:x.recovery(proof,game.attachmentId)})).value.error.code,'OWNER_ACTIVE');
  await writeFile('docs/g3/server-startup-repair-retained-waiting-recovery-fixture.json',JSON.stringify({engineFacts:'synthetic, not save persistence',before:w,after:current,actualStartAdmission:admitted.canonicalAdmission},null,2)+'\n');
 }finally{await f.close();console.log(JSON.stringify({cleanedSchema:f.namespace}));}
});
test('Waiting lost recovery reply reconciles under new auth without obsolete credentials; ban revokes return but not peer',async()=>{
 const x=await setup(1);const {f,s,call,payload}=x;const target=s.desktops[1]!;try{
  const enrolled=await call(target,{enroll:payload}),w=enrolled.value.waiting,proof=enrolled.value.waitingProof,id=w.intent.waitingId;
  const makeDesktop=()=>f.service.db.transaction(tx=>f.service.auth.family(tx,target!.accountId!,target!.deviceId!,'desktop'));
  const desktop=await makeDesktop();
  const status=(await call(desktop,{waitingId:id,status:{}})).value;assert.equal(status.waiting.state,'recoverable');assert.equal(status.waitingProof,undefined);assert.equal(status.credentials,undefined);
  const enrollmentOriginal={accountId:target.accountId,deviceId:target.deviceId,originalSessionId:target.sessionId,originalSessionEpoch:enrolled.req.sessionEpoch,originalServiceEpoch:enrolled.req.serviceEpoch,requestId:enrolled.req.requestId,kind:'enroll',fingerprint:operationFingerprint(enrolled.req)};
  const lostProof=(await call(desktop,{outcome:enrollmentOriginal})).value;assert.equal(lostProof.outcome.originalOutcome,'committed');assert.equal(lostProof.waitingProof,undefined);assert.equal(lostProof.credentials,undefined);
  assert.equal((await call(desktop,{waitingId:id,generation:1,recover:x.recovery('')})).value.error.code,'FORBIDDEN','committed enrollment is not possession of its lost proof');
  const recovered=await call(desktop,{waitingId:id,generation:1,recover:x.recovery(proof)});assert.equal(recovered.value.error,undefined);assert.equal(recovered.value.waiting.generation,2);
  const original={accountId:desktop.accountId,deviceId:desktop.deviceId,originalSessionId:desktop.sessionId,originalSessionEpoch:desktop.sessionEpoch,originalServiceEpoch:f.service.db.epoch,requestId:recovered.req.requestId,kind:'recover',fingerprint:operationFingerprint(recovered.req),waitingId:id};
  const newAuth=await makeDesktop(),known=(await call(newAuth,{outcome:original})).value;
  assert.equal(known.outcome.originalOutcome,'committed');assert.equal(known.outcome.effectId,recovered.value.credentials.attachmentId);assert.equal(known.credentials,undefined);assert.equal(known.waitingProof,undefined);assert.equal(known.waiting.generation,2);
  assert.equal((await call(desktop,{waitingId:id,status:{}})).value.error.code,'UNAUTHENTICATED');
  const again=(await call(newAuth,{waitingId:id,generation:2,recover:x.recovery(proof,recovered.value.credentials.attachmentId)})).value;assert.equal(again.error,undefined);assert.equal(again.waiting.generation,3);assert.deepEqual(again.waiting.intent,w.intent);
  await s.rpc(s.host,{roomGet:{}});await s.rpc(s.host,{roomModerate:{action:'ban',accountId:target!.accountId!}});
  const terminal=(await call(newAuth,{waitingId:id,status:{}})).value;assert.equal(terminal.waiting.state,'terminal');assert.equal(terminal.waiting.reason,'access_revoked');
  assert.equal((await call(newAuth,{waitingId:id,generation:3,recover:x.recovery(proof,again.credentials.attachmentId)})).value.error.code,'RETURN_INELIGIBLE');
  assert.equal((await call(newAuth,{outcome:original})).value.outcome.originalOutcome,'committed');assert.equal((await call(newAuth,{outcome:{...original,requestId:randomUUID()}})).value.outcome.originalOutcome,'unresolved');
  assert.equal((await s.rpc(s.games[0],{roomGet:{}})).gameStatus!.phase,'hold');
  await writeFile('docs/g3/server-startup-repair-retained-waiting-outcome-fixture.json',JSON.stringify({engineFacts:'synthetic',outcome:known.outcome,current:known.waiting,terminal:terminal.waiting},null,2)+'\n');
 }finally{await f.close();console.log(JSON.stringify({cleanedSchema:f.namespace}));}
});
test('Waiting cancel/end are terminal and old offline replay cannot remove a new waiting-run membership',async()=>{
 const x=await setup();const {f,s,call,payload}=x;try{
  const enrolled=(await call(s.desktops[0],{enroll:payload})).value,id=enrolled.waiting.intent.waitingId,proof=enrolled.waitingProof;
  await s.rpc(s.host,{roomGet:{}});await s.rpc(s.host,{cancelRun:{}});
  assert.equal((await call(s.desktops[0],{waitingId:id,status:{}})).value.waiting.state,'terminal');
  assert.equal((await call(s.desktops[0],{waitingId:id,generation:1,recover:x.recovery(proof)})).value.error.code,'RETURN_INELIGIBLE');
  const offline=await call(s.desktops[0],{waitingId:id,generation:1,offline:{waitingProof:proof}});assert.equal(offline.value.error,undefined);assert.equal(offline.value.waiting.state,'terminal');
  await s.rpc(s.host,{roomGet:{}});await s.rpc(s.desktops[0],{roomJoin:{}});await s.rpc(s.desktops[0],{ready:{ready:true}});await s.rpc(s.desktops[0],{prepare:{}});
  const newRun=s.room.runId;assert.notEqual(newRun,enrolled.waiting.intent.runId);
  const replay=JSON.parse((await f.call('/v3/waiting/rpc',offline.req,s.desktops[0]!.accessToken)).body);assert.equal(replay.error,undefined);assert.equal(replay.outcome.originalOutcome,'committed');
  const otherOffline=(await call(s.desktops[0],{waitingId:id,generation:1,offline:{waitingProof:proof}})).value;assert.equal(otherOffline.error,undefined);
  assert.equal((await f.service.db.transaction(tx=>tx.query('SELECT room_id FROM memberships WHERE desktop_id=$1',[s.desktops[0]!.sessionId]))).rows[0].room_id,s.room.roomId);
  // Existing host End remains authoritative for the newly opened run.
  await s.rpc(s.host,{roomGet:{}});await s.rpc(s.host,{endRun:{}});assert.equal(s.room.state,'ended');
 }finally{await f.close();console.log(JSON.stringify({cleanedSchema:f.namespace}));}
});
test('Waiting eligibility does not cross the fixed review deadline or a server epoch restart',async()=>{
 const x=await setup();const {f,s,call,payload}=x;try{
  const enrolled=await call(s.desktops[0],{enroll:payload}),id=enrolled.value.waiting.intent.waitingId;
  await f.service.db.transaction(tx=>tx.query('UPDATE runs SET review_until=$2 WHERE id=$1',[s.room.runId,new Date(x.now()+1000)]));x.advance(2000);
  const expired=(await call(s.desktops[0],{waitingId:id,status:{}})).value;assert.equal(expired.waiting.state,'terminal');assert.equal(expired.waitingProof,undefined);
  assert.equal((await call(s.desktops[0],{waitingId:id,generation:1,recover:x.recovery(enrolled.value.waitingProof)})).value.error.code,'RETURN_INELIGIBLE');
  await f.restart();
  const auth=await f.service.db.transaction(tx=>f.service.auth.family(tx,s.desktops[0]!.accountId!,s.desktops[0]!.deviceId!,'desktop'));
  const terminal=(await call(auth,{waitingId:id,status:{}})).value;assert.equal(terminal.waiting.state,'terminal');assert.equal(terminal.waitingProof,undefined);assert.equal(terminal.credentials,undefined);
  const original={accountId:s.desktops[0]!.accountId,deviceId:s.desktops[0]!.deviceId,originalSessionId:s.desktops[0]!.sessionId,originalSessionEpoch:enrolled.req.sessionEpoch,originalServiceEpoch:enrolled.req.serviceEpoch,requestId:enrolled.req.requestId,kind:'enroll',fingerprint:operationFingerprint(enrolled.req)};
  const known=(await call(auth,{outcome:original})).value;assert.equal(known.outcome.originalOutcome,'committed');assert.equal(known.outcome.currentDisposition,'terminal');assert.equal(known.credentials,undefined);
 }finally{await f.close();console.log(JSON.stringify({cleanedSchema:f.namespace}));}
});
test('Actual host Start while waiting world is absent does not bridge excluded waiting intent into admitted recovery',async()=>{
 const x=await setup();const {f,s,call,payload}=x;try{
  const enrolled=(await call(s.desktops[0],{enroll:payload})).value,id=enrolled.waiting.intent.waitingId;
  for(let i=0;i<3;i++){x.advance(11000);await s.rpc(s.desktops[1],{presence:{present:true}});await s.rpc(s.games[1],{gamePulse:{attachmentId:s.games[1]!.attachmentId!,gameInstanceId:s.observations[1]!.gameInstanceId!,worldToken:s.observations[1]!.worldToken!}});}
  await s.rpc(s.host,{roomGet:{}});const firstPage=await s.rpc(s.host,{inspectionList:{limit:1,afterDesktopId:''}}),reports=[...firstPage.inspections!];
  // Read the complete bounded roster by its normal cursor, selecting only the
  // still-present peer. The saved waiting owner is explicitly excluded.
  let cursor=firstPage.inspectionNextDesktopId??'';
  while(cursor){const page=await s.rpc(s.host,{inspectionList:{limit:1,afterDesktopId:cursor}});reports.push(...page.inspections!);cursor=page.inspectionNextDesktopId??'';}
  const peer=reports.find(r=>r.desktopId===s.desktops[1]!.sessionId)!;
  await s.rpc(s.host,{finalStart:{participants:[{desktopId:peer.desktopId!,attachmentId:peer.attachmentId!,worldToken:peer.worldToken!,observationRevision:peer.observationRevision!,contentDigest:peer.contentDigest!}],excludedDesktopIds:[s.desktops[0]!.sessionId!]}});
  const status=(await call(s.desktops[0],{waitingId:id,status:{}})).value;assert.equal(status.waiting.state,'started');assert.equal(status.waiting.reason,'not_selected_by_start');assert.equal(status.waiting.admittedParticipantId,undefined);assert.equal(status.waiting.mayPlay,false);assert.notEqual(status.canonicalAdmission?.decision?.authorized,true);
  const refused=(await call(s.desktops[0],{waitingId:id,generation:1,recover:x.recovery(enrolled.waitingProof)})).value;assert.equal(refused.error.code,'NOT_SELECTED_BY_START');assert.equal(refused.credentials,undefined);
  await writeFile('docs/g3/server-startup-repair-retained-waiting-start-absent-fixture.json',JSON.stringify({engineFacts:'synthetic',waiting:status.waiting,refusal:refused.error},null,2)+'\n');
 }finally{await f.close();console.log(JSON.stringify({cleanedSchema:f.namespace}));}
});
test('A waiting proof cannot impersonate admitted resume after Start and desktop replacement',async()=>{
 const x=await setup(1);const {f,s,call,payload}=x;try{
  const enrolled=(await call(s.desktops[1],{enroll:payload})).value;await s.start();
  const current=await f.service.db.transaction(tx=>f.service.auth.family(tx,s.desktops[1]!.accountId!,s.desktops[1]!.deviceId!,'desktop'));
  const status=(await call(current,{waitingId:enrolled.waiting.intent.waitingId,status:{}})).value;
  assert.equal(status.waiting.state,'started');assert.ok(status.waiting.admittedParticipantId);assert.equal(status.waiting.mayPlay,false);assert.equal(status.canonicalAdmission,undefined);assert.equal(status.waitingProof,undefined);assert.equal(status.credentials,undefined);
  const denied=(await call(current,{waitingId:enrolled.waiting.intent.waitingId,generation:1,recover:x.recovery(enrolled.waitingProof)})).value;assert.equal(denied.error.code,'OWNER_ACTIVE');
  const base={major:1,revision:'g3-rejoin-1',serviceId:f.service.db.serviceId,serviceEpoch:f.service.db.epoch,sessionEpoch:current.sessionEpoch,requestId:randomUUID(),participantId:status.waiting.admittedParticipantId};
  const admitted=JSON.parse((await f.call('/v3/rejoin/rpc',{...base,status:{}},current.accessToken)).body);assert.ok(admitted.participant.association.originalStartId);assert.equal(admitted.resumeProof,undefined);
  const association=admitted.participant.association;
  const forged=JSON.parse((await f.call('/v3/rejoin/rpc',{...base,requestId:randomUUID(),roomId:association.roomId,runId:association.runId,worldToken:association.worldToken,configRevision:association.configRevision,generation:admitted.participant.generation,resume:{resumeProof:enrolled.waitingProof,gameInstanceId:'no-bridge',gameVersion:f.profile.compatibility.gameVersion,modVersion:f.profile.compatibility.modVersion,bridgeVersion:f.profile.compatibility.bridgeVersion,capabilities:f.profile.compatibility.capabilities}},current.accessToken)).body);
  assert.equal(forged.error.code,'FORBIDDEN');assert.equal(forged.credentials,undefined);
 }finally{await f.close();console.log(JSON.stringify({cleanedSchema:f.namespace}));}
});
