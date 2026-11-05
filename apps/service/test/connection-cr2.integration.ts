import test from 'node:test';import assert from 'node:assert/strict';import {randomUUID} from 'node:crypto';import {writeFile} from 'node:fs/promises';
import {startG3Fixture,provisionG3Fixture} from '../scripts/g3-loot01-fixture.js';
import {livePeer} from '../scripts/g3-live-fixture.js';
import {schema,encode,decode} from '../../../packages/contracts/src/g3-connection.js';
import {schema as rejoinSchema} from '../../../packages/contracts/src/g3-rejoin.js';
import {schema as lobbySchema} from '../../../packages/contracts/src/g2-next.js';
import {operationFingerprint as fingerprint} from '../../../packages/contracts/src/connection-fingerprint.js';
import {OUTCOME_RETENTION_MS} from '../src/rejoin/outcomes.js';
import {connectionReply} from '../src/g3/connection.js';
const databaseUrl='postgres://nt2_g1:local-fixture-only@127.0.0.1:54328/nt2_g1';
test('CR2 issued guest credential sample is shortened by current guest/family expiry and cannot outlive them',async()=>{
 let now=Date.now();const f=await startG3Fixture({databaseUrl,rejoin:true,loot01Runtime:true,live:true,tls:true,now:()=>now});try{
  const base={major:1,revision:lobbySchema.revision,serviceId:f.service.db.serviceId,serviceEpoch:f.service.db.epoch};
  const issueId=randomUUID(),issued=JSON.parse((await f.call('/v1/rpc',{...base,requestId:issueId,guestIssue:{deviceId:'cr2-guest-fixture',displayName:'Guest fixture',issuanceProof:'a'.repeat(43)}})).body);assert.ok(issued.credentials,JSON.stringify(issued));assert.equal(issued.credentialValidity.originRequestId,issueId);assert.equal(issued.credentialValidity.sessionId,issued.credentials.sessionId);
  const claims=await f.service.auth.security.verify(issued.credentials.accessToken,now);assert.equal(issued.credentialValidity.credentialId,claims.jti);
  const read=async()=>JSON.parse((await f.call('/v1/rpc',{...base,sessionEpoch:issued.credentials.sessionEpoch,requestId:randomUUID(),roomList:{}},issued.credentials.accessToken)).body);
  await f.service.db.transaction(tx=>tx.query('UPDATE principal_profiles SET guest_expires_at=$2 WHERE principal_id=$1',[issued.credentials.accountId,new Date(now+1500)]));assert.ok((await read()).credentialValidity.accessRemainingMs<=1500);
  await f.service.db.transaction(tx=>tx.query('UPDATE session_families SET expires_at=$2 WHERE id=(SELECT family_id FROM sessions WHERE id=$1)',[issued.credentials.sessionId,new Date(now+500)]));assert.ok((await read()).credentialValidity.accessRemainingMs<=500);
  now+=1000;assert.equal((await read()).error.code,'UNAUTHENTICATED');
 }finally{await f.close();}
});
test('CR2 actual initial/rejoin replies share immutable/current admission and exact request/token samples',async()=>{
 let now=Date.now();const f=await startG3Fixture({databaseUrl,rejoin:true,loot01Runtime:true,live:true,tls:true,now:()=>now});
 try{
  const s=await provisionG3Fixture(f,{beforeRpc:()=>{now+=10;}});await s.start();
  const a=(await s.rpc(s.games[0],{roomGet:{}})).canonicalAdmission!;assert.equal(a.decision!.authorized,true);assert.equal(a.decision!.playing,false);
  await s.playing();const after=await s.rpc(s.games[0],{roomGet:{}}),b=after.canonicalAdmission!;assert.equal(b.decision!.playing,true);assert.equal(b.immutable!.selectedInspection!.observationRevision,a.immutable!.selectedInspection!.observationRevision);assert.ok(b.currentReport!.revision!>a.currentReport!.revision!);
  const req={major:1,revision:schema.revision,serviceId:f.service.db.serviceId,serviceEpoch:f.service.db.epoch,sessionEpoch:s.games[0]!.sessionEpoch,requestId:randomUUID(),admission:{roomId:s.room.roomId!}};
  const first=JSON.parse((await f.call('/v3/connection/rpc',req,s.games[0]!.accessToken)).body);assert.ok(first.admission,JSON.stringify(first));
  now+=100;const retry=JSON.parse((await f.call('/v3/connection/rpc',req,s.games[0]!.accessToken)).body);assert.equal(first.admission.sample.sampleKey,retry.admission.sample.sampleKey);assert.ok(retry.credentialValidity.accessRemainingMs<first.credentialValidity.accessRemainingMs);
  const binary=await f.service.app.inject({method:'POST',url:'/v3/connection/rpc',headers:{host:new URL(f.origin).host,'content-type':'application/x-protobuf',authorization:'Bearer '+s.games[0]!.accessToken},payload:Buffer.from(encode('Request',req))});assert.equal((decode('Response',binary.rawPayload) as any).admission.sample.sampleKey,first.admission.sample.sampleKey);
  const old=s.desktops[0]!,refreshId=randomUUID();const refreshed=await s.rpc(old,{requestId:refreshId,refresh:{secret:old.refreshSecret!,deviceId:old.deviceId!}});assert.equal(refreshed.credentialValidity!.sessionId,refreshed.credentials!.sessionId);assert.equal(refreshed.credentialValidity!.sessionEpoch,refreshed.credentials!.sessionEpoch);assert.equal(refreshed.credentialValidity!.originRequestId,refreshId);
  Object.assign(old,refreshed.credentials);const renew=await s.rpc(old,{gameRenew:{attachmentId:s.games[0]!.attachmentId!,gameSessionId:s.games[0]!.sessionId!,gameSessionEpoch:s.games[0]!.sessionEpoch!,gameInstanceId:s.observations[0]!.gameInstanceId!,worldToken:s.scopes[0]!.worldToken!}});assert.equal(renew.credentialValidity!.sessionId,renew.credentials!.sessionId);assert.equal(renew.credentialValidity!.credentialId,(await f.service.auth.security.verify(renew.credentials!.accessToken!,now)).jti);
  let queries=0;const started=performance.now();await f.service.db.transaction(tx=>connectionReply({query:(...args:any[])=>{queries++;return (tx.query as any).apply(tx,args);}} as any,f.service.db,f.service.auth,f.profile.compatibility,{requestId:'measured-read',roomId:s.room.roomId},{},renew.credentials!.accessToken));const elapsedMs=performance.now()-started;
  const claims=await f.service.auth.security.verify(renew.credentials!.accessToken!,now);await f.service.db.transaction(tx=>tx.query('UPDATE access_tokens SET expires_at=$2 WHERE id=$1',[claims.jti,new Date(now+5000)]));Object.assign(s.games[0]!,renew.credentials);
  const tokenBound=await s.rpc(s.games[0],{roomGet:{}});assert.ok(tokenBound.credentialValidity!.accessRemainingMs!<=5000);
  await f.service.db.transaction(tx=>tx.query('UPDATE session_families SET expires_at=$2 WHERE id=(SELECT family_id FROM sessions WHERE id=$1)',[s.games[0]!.sessionId,new Date(now+3000)]));const familyBound=await s.rpc(s.games[0],{roomGet:{}});assert.ok(familyBound.credentialValidity!.accessRemainingMs!<=3000);assert.ok(familyBound.canonicalAdmission!.decision!.remainingMs!<=3000);
  await writeFile('docs/g3/server-connection-cr2-admission-fixtures.json',JSON.stringify({scope:'disposable production service, injected engine facts; no credentials/proofs',initial:a,postStart:b,duplicate:[first.admission,retry.admission],credentialSamples:[refreshed.credentialValidity,renew.credentialValidity,tokenBound.credentialValidity,familyBound.credentialValidity],measurement:{queries,elapsedMs,worlds:2,coverage:'connectionReply read only; excludes transaction BEGIN/namespace lock/epoch/COMMIT and transactionProjection reads',database:f.service.db.workStats}},null,2)+'\n');
 }finally{await f.close();}
});
test('CR2 lost resume/offline outcome after changed authentication is exact durable evidence, never old credentials',async()=>{
 const f=await startG3Fixture({databaseUrl,rejoin:true,loot01Runtime:true,live:true,tls:true});try{
  const s=await provisionG3Fixture(f);await s.start();await s.playing();
  const rpc=async(c:any,p:any,payload:any)=>{const a=p?.association;const request={major:1,revision:rejoinSchema.revision,serviceId:f.service.db.serviceId,serviceEpoch:f.service.db.epoch,sessionEpoch:c.sessionEpoch,requestId:randomUUID(),...(a?{roomId:a.roomId,runId:a.runId,participantId:a.participantId,worldToken:a.worldToken,configRevision:a.configRevision,generation:p.generation}:{}),...payload};const result=JSON.parse((await f.call('/v3/rejoin/rpc',request,c.accessToken)).body);assert.ok(!result.error,JSON.stringify(result));return {request,result};};
  const status=(await rpc(s.desktops[0],undefined,{status:{}})).result,prior=status.participant,a=prior.association;
  await f.loseParticipant(a.participantId);
  const resumed=await rpc(s.desktops[0],prior,{resume:{resumeProof:status.resumeProof,gameInstanceId:'cr2-return-'+randomUUID(),gameVersion:f.profile.compatibility.gameVersion,modVersion:f.profile.compatibility.modVersion,bridgeVersion:f.profile.compatibility.bridgeVersion,capabilities:f.profile.compatibility.capabilities}});
  assert.equal(resumed.result.canonicalAdmission.generation,2);assert.equal(resumed.result.canonicalAdmission.decision.authorized,false);
  const observed=await rpc(resumed.result.credentials,resumed.result.participant,{observe:{...s.observations[0]!,attachmentId:resumed.result.credentials.attachmentId,gameInstanceId:resumed.request.resume!.gameInstanceId,observationRevision:1,protected:true}});assert.equal(observed.result.canonicalAdmission.decision.authorized,true);assert.equal(observed.result.canonicalAdmission.originalStart.selectedInspection.attachmentId,s.games[0]!.attachmentId);assert.equal(observed.result.canonicalAdmission.immutable.selectedInspection.attachmentId,resumed.result.credentials.attachmentId);
  const original={requestId:resumed.request.requestId,kind:'resume',fingerprint:fingerprint(resumed.request),originalSessionId:s.desktops[0]!.sessionId,originalSessionEpoch:s.desktops[0]!.sessionEpoch,originalOwnerGeneration:prior.generation,participantId:a.participantId,roomId:a.roomId,runId:a.runId,worldToken:a.worldToken,configRevision:a.configRevision,originalServiceEpoch:f.service.db.epoch,accountId:s.desktops[0]!.accountId,deviceId:s.desktops[0]!.deviceId};
  const current=await f.service.db.transaction(tx=>f.service.auth.family(tx,original.accountId!,original.deviceId!,'desktop'));
  const read=async(o:any,c:any=current)=>JSON.parse((await f.call('/v3/connection/rpc',{major:1,revision:schema.revision,serviceId:f.service.db.serviceId,serviceEpoch:f.service.db.epoch,sessionEpoch:c.sessionEpoch,requestId:randomUUID(),outcome:o},c.accessToken)).body);
  const committed=await read(original);assert.equal(committed.outcome.originalOutcome,'committed');assert.equal(committed.outcome.effectId,resumed.result.credentials.attachmentId);assert.equal(committed.credentials,undefined);assert.equal(committed.outcome.currentGeneration,2);
  assert.equal((await read({...original,kind:'goldIntent'})).outcome.coverage,'kind_not_covered');assert.equal((await read({...original,fingerprint:'0'.repeat(64)})).outcome.originalOutcome,'unresolved');assert.equal((await read({...original,requestId:randomUUID()})).outcome.originalOutcome,'unresolved');assert.equal((await read({...original,deviceId:'wrong'})).error.code,'FORBIDDEN');
  // Exact committed knowledge cannot bypass current ownership or healthy lease.
  await assert.rejects(rpc(s.desktops[0],resumed.result.participant,{resume:{...resumed.request.resume,gameInstanceId:'overlap'}}),/OWNER_ACTIVE/);
  await f.service.db.transaction(tx=>tx.query("INSERT INTO connection_outcomes(participant_id,request_id,kind,original,fingerprint,effect_id,created_at,expires_at) SELECT $1,'fixture-cap-'||i,'offline','{}'::jsonb,$2,'fixture',now(),now()+interval '1 day' FROM generate_series(1,1023) i",[a.participantId,'0'.repeat(64)]));
  await assert.rejects(rpc(current,resumed.result.participant,{offline:{resumeProof:status.resumeProof}}),/CAPACITY/);assert.equal((await read(original)).outcome.currentGeneration,2);
  await f.service.db.transaction(tx=>tx.query("DELETE FROM connection_outcomes WHERE participant_id=$1 AND request_id LIKE 'fixture-cap-%'",[a.participantId]));
  const offline=await rpc(current,resumed.result.participant,{offline:{resumeProof:status.resumeProof}}),offOriginal={...original,requestId:offline.request.requestId,kind:'offline',fingerprint:fingerprint(offline.request),originalSessionId:current.sessionId,originalSessionEpoch:current.sessionEpoch,originalOwnerGeneration:2};
  const off=await read(offOriginal);assert.equal(off.outcome.originalOutcome,'committed');assert.equal(off.outcome.currentDisposition,'terminal');assert.equal((await read({...original,requestId:randomUUID()})).outcome.originalOutcome,'unresolved');
  await f.service.db.transaction(tx=>tx.query('UPDATE connection_outcomes SET expires_at=$2 WHERE participant_id=$1',[a.participantId,new Date(Date.now()-OUTCOME_RETENTION_MS)]));assert.equal((await read(original)).outcome.originalOutcome,'unresolved');
  assert.ok((await s.rpc(s.games[1],{roomGet:{}})).canonicalAdmission!.decision!.playing);
  await writeFile('docs/g3/server-connection-cr2-outcome-fixtures.json',JSON.stringify({resumedAdmission:observed.result.canonicalAdmission,scope:'disposable actual service; no credentials/proofs',resume:committed,offline:off},null,2)+'\n');
 }finally{await f.close();}
});
test('CR2 real WSS echoes auth/ping/pose ack and computes relative ages at drain; Ping/Pong and channel denial preserve HTTP',async()=>{
 let now=Date.now();const f=await startG3Fixture({databaseUrl,rejoin:true,loot01Runtime:true,live:true,tls:true,now:()=>now});const peers:any[]=[];try{
  const s=await provisionG3Fixture(f);await s.start();await s.playing();const a=await livePeer(f,s,0),b=await livePeer(f,s,1);peers.push(a,b);
  const welcome=await a.wait(v=>!!v.welcome);assert.equal(welcome.ackClientSequence,1);assert.ok(welcome.authorityRemainingMs!<=30000);assert.ok(welcome.accessRemainingMs!<=900000);
  let pong=false;a.socket.once('pong',()=>{pong=true;});a.socket.ping();await new Promise(r=>setTimeout(r,40));assert.equal(pong,true);
  a.send({ping:{}});assert.equal((await a.wait(v=>!!v.pong)).ackClientSequence,2);
  a.send({pose:{sequence:1,xMilli:'1',yMilli:'0',vxMilli:'0',vyMilli:'0'}});const snapshot=await b.wait(v=>!!v.snapshot?.peers?.length);assert.ok(snapshot.snapshot!.peers![0]!.poseAgesMsAtSend![0]!<=2000);assert.equal((await a.wait(v=>v.ackClientSequence===3)).ackClientSequence,3);
  now+=1100;const renewed=(await s.rpc(s.desktops[0],{gameRenew:{attachmentId:s.games[0]!.attachmentId!,gameSessionId:s.games[0]!.sessionId!,gameSessionEpoch:s.games[0]!.sessionEpoch!,gameInstanceId:s.observations[0]!.gameInstanceId!,worldToken:s.scopes[0]!.worldToken!}})).credentials!;a.send({reauthenticate:{accessToken:renewed.accessToken!,authRevision:2}});const reauth=await a.wait(v=>v.welcome?.authRevision===2);assert.equal(reauth.ackClientSequence,4);assert.notEqual(reauth.credentialId,welcome.credentialId);
  now+=2100;const gone=await b.wait(v=>!!v.snapshot&&!v.snapshot.peers?.length);assert.ok(gone.snapshot);
  const closed=new Promise<string>(resolve=>{a.socket.once('close',(_code,reason)=>resolve(reason.toString()));});a.send({pose:{sequence:1,xMilli:'1',yMilli:'0',vxMilli:'0',vyMilli:'0'}});assert.equal(await closed,'STALE_EPOCH');Object.assign(s.games[0]!,renewed);assert.ok((await s.rpc(s.games[0],{roomGet:{}})).canonicalAdmission!.decision!.playing);assert.equal(b.socket.readyState,1);
 }finally{for(const p of peers)await p.close();await f.close();}
});
