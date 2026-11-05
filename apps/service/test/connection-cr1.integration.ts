import test from 'node:test';import assert from 'node:assert/strict';import {randomUUID} from 'node:crypto';import {writeFile,readFile} from 'node:fs/promises';
import {startG3Fixture,provisionG3Fixture} from '../scripts/g3-loot01-fixture.js';
import {readRoomAdmissions} from '../src/g3/admission.js';
import {schema as gameSchema} from '../../../packages/contracts/src/g3.js';
import {schema as socialSchema} from '../../../packages/contracts/src/g3-social.js';
const databaseUrl='postgres://nt2_g1:local-fixture-only@127.0.0.1:54328/nt2_g1';

test('CR1 replayed offline for a terminal old run cannot remove same-desktop membership in its new run',async()=>{
 const f=await startG3Fixture({databaseUrl,rejoin:true,loot01Runtime:true,live:true,tls:true});try{
  const s=await provisionG3Fixture(f);await s.start();await s.playing();
  const base={major:1,revision:'g3-rejoin-1',serviceId:f.service.db.serviceId,serviceEpoch:f.service.db.epoch,sessionEpoch:s.desktops[0]!.sessionEpoch};
  const original=JSON.parse((await f.call('/v3/rejoin/rpc',{...base,requestId:randomUUID(),status:{}},s.desktops[0]!.accessToken)).body);const a=original.participant.association;
  const offline={...base,requestId:randomUUID(),participantId:a.participantId,roomId:a.roomId,runId:a.runId,worldToken:a.worldToken,configRevision:a.configRevision,generation:original.participant.generation,offline:{resumeProof:original.resumeProof}};
  const result=JSON.parse((await f.call('/v3/rejoin/rpc',offline,s.desktops[0]!.accessToken)).body);assert.equal(result.participant.returnEligible,false);
  await s.rpc(s.host,{endRun:{}});const joined=await s.rpc(s.desktops[0],{roomJoin:{}}),instance='cr1-new-run-'+randomUUID();
  const game=(await s.rpc(s.desktops[0],{gameAttach:{preparationId:joined.preparation!.preparationId!,gameInstanceId:instance,gameVersion:f.profile.compatibility.gameVersion,modVersion:f.profile.compatibility.modVersion,bridgeVersion:f.profile.compatibility.bridgeVersion,capabilities:f.profile.compatibility.capabilities}})).credentials!;
  const observation={...s.observations[0]!,attachmentId:game.attachmentId!,gameInstanceId:instance,worldToken:'cr1-new-world-'+randomUUID(),observationRevision:1,protected:true};
  await s.rpc(game,{gameObservation:observation});await s.start();await s.rpc(game,{gameObservation:{...observation,observationRevision:2,protected:false}});
  assert.ok((await s.rpc(game,{roomGet:{}})).release);
  const replay=JSON.parse((await f.call('/v3/rejoin/rpc',offline,s.desktops[0]!.accessToken)).body);assert.equal(replay.participant.returnEligible,false);
  assert.ok((await s.rpc(game,{roomGet:{}})).release);
 }finally{await f.close();}
});

test('CR1 late first unprotected report cannot erase fixed review expiry, entered peer keeps playing',async()=>{
 let now=Date.now();const f=await startG3Fixture({databaseUrl,rejoin:true,loot01Runtime:true,live:true,tls:true,now:()=>now});try{
  const s=await provisionG3Fixture(f,{beforeRpc:()=>{now+=100;}});
  await f.service.db.transaction(tx=>tx.query('UPDATE runs SET review_until=$2 WHERE id=$1',[s.room.runId,new Date(now+10000)]));
  await s.start();await s.rpc(s.games[1],{gameObservation:{...s.observations[1]!,observationRevision:2,protected:false}});
  now+=11000;await s.pulse();
  await assert.rejects(s.rpc(s.games[0],{gameObservation:{...s.observations[0]!,observationRevision:2,protected:false}}),/EXPIRED/);
  assert.equal((await s.rpc(s.games[0],{roomGet:{}})).release,undefined);assert.ok((await s.rpc(s.games[1],{roomGet:{}})).release);

 }finally{await f.close();}
});

test('CR1 production admission holds incomplete same-digest across HTTP/WSS/social and keeps peer, immutable release/idempotence',async()=>{
 let now=Date.now();const f=await startG3Fixture({databaseUrl,rejoin:true,loot01Runtime:true,live:true,tls:true,now:()=>now});
 try{
  const s=await provisionG3Fixture(f,{beforeRpc:()=>{now+=100;}});await s.start();
  const first=(await s.rpc(s.games[0],{roomGet:{}})).release!;now+=500;await s.rpc(s.games[0],{gameObservation:s.observations[0]!});
  const retry=(await s.rpc(s.games[0],{roomGet:{}})).release!;assert.equal(retry.capturedAt,first.capturedAt);assert.equal(retry.authorizedAt,first.authorizedAt);
  const protectedAdmission=(await f.service.db.transaction(tx=>readRoomAdmissions(tx,f.service.db,f.profile.compatibility,now,s.room.roomId!))).worlds.find(w=>w.owner.attachmentId===s.games[0]!.attachmentId)!;
  await s.playing();const playing=(await s.rpc(s.games[0],{roomGet:{}})).release!;assert.equal(playing.capturedAt,first.capturedAt);assert.equal(playing.observationRevision,first.observationRevision);
  const measurements:{queries:number;elapsedMs:number;worlds:number}[]=[];
  const read=async()=>{const started=performance.now();let counted=0;const admission=await f.service.db.transaction(async tx=>{let queries=0;const measured={query:(...args:any[])=>{queries++;return (tx.query as any).apply(tx,args);}};const result=await readRoomAdmissions(measured as any,f.service.db,f.profile.compatibility,now,s.room.roomId!);counted=queries;return result;});measurements.push({queries:counted,elapsedMs:performance.now()-started,worlds:admission.worlds.length});return admission;};
  const before=await read();assert.equal(before.worlds.filter(w=>w.decision.playing).length,2);
  f.service.live!.projection.watch(s.room.roomId!);
  await s.rpc(s.games[0],{gameObservation:{...s.observations[0]!,observationRevision:3,protected:false,inventoryComplete:false,collectorFailure:'collection_failed'}});
  const held=await read(),me=held.worlds.find(w=>w.owner.attachmentId===s.games[0]!.attachmentId)!;assert.equal(me.decision.authorized,false);assert.equal(held.worlds.find(w=>w.owner.attachmentId===s.games[1]!.attachmentId)!.decision.playing,true);
  assert.equal(f.service.live!.projection.member(s.room.roomId!,s.games[0]!.sessionId!),undefined);assert.ok(f.service.live!.projection.member(s.room.roomId!,s.games[1]!.sessionId!));
  const call=async(path:string,revision:string,payload:any)=>JSON.parse((await f.call(path,{major:1,revision,serviceId:f.service.db.serviceId,serviceEpoch:f.service.db.epoch,sessionEpoch:s.games[0]!.sessionEpoch,requestId:randomUUID(),scope:s.scopes[0],...payload},s.games[0]!.accessToken)).body);
  assert.ok((await call('/v3/rpc',gameSchema.revision,{ghostList:{}})).error);
  assert.ok((await call('/v3/social/rpc',socialSchema.revision,{snapshot:{}})).error);
  assert.equal((await s.rpc(s.games[0],{roomGet:{}})).release,undefined);
  await s.rpc(s.games[0],{gameObservation:{...s.observations[0]!,observationRevision:4,protected:true}});
  await s.rpc(s.games[0],{gameObservation:{...s.observations[0]!,observationRevision:5,protected:false}});
  const restored=(await read()).worlds.find(w=>w.owner.attachmentId===s.games[0]!.attachmentId)!;assert.equal(restored.decision.playing,true);assert.equal(restored.immutable.selectedInspection!.capturedAt,first.capturedAt);assert.equal(restored.immutable.authorizationId,first.authorizationId);
  await s.rpc(s.games[0],{gameObservation:{...s.observations[0]!,observationRevision:6,protected:true,activeMods:['noita_together2','advisory-mod']}});
  const participant=JSON.parse((await f.call('/v3/rejoin/rpc',{major:1,revision:'g3-rejoin-1',serviceId:f.service.db.serviceId,serviceEpoch:f.service.db.epoch,sessionEpoch:s.desktops[0]!.sessionEpoch,requestId:randomUUID(),status:{}},s.desktops[0]!.accessToken)).body).participant;
  const a=participant.association,review=JSON.parse((await f.call('/v3/rejoin/rpc',{major:1,revision:'g3-rejoin-1',serviceId:f.service.db.serviceId,serviceEpoch:f.service.db.epoch,sessionEpoch:s.host.sessionEpoch,requestId:randomUUID(),roomId:a.roomId,runId:a.runId,participantId:a.participantId,worldToken:a.worldToken,configRevision:a.configRevision,generation:participant.generation,review:{participantId:a.participantId,observationRevision:6,contentDigest:participant.pendingContentDigest}},s.host.accessToken)).body);assert.ok(!review.error,review.error?.code);
  const reviewed=(await read()).worlds.find(w=>w.owner.attachmentId===s.games[0]!.attachmentId)!;assert.notEqual(reviewed.immutable.authorizationId,first.authorizationId);assert.equal(reviewed.originalStart.requestId,restored.originalStart.requestId);assert.equal(reviewed.originalStart.selectedInspection!.capturedAt,restored.originalStart.selectedInspection!.capturedAt);
  const fixtures={pairs:{identicalObservationRetry:{before:first,after:retry},currentReportAfterStart:{before:protectedAdmission,after:before.worlds.find(w=>w.owner.attachmentId===s.games[0]!.attachmentId)},protectedToPlaying:{before:protectedAdmission,after:before.worlds.find(w=>w.owner.attachmentId===s.games[0]!.attachmentId)}},revision:'world-admission-cr1-candidate-1',scope:'actual disposable production service; injected engine facts; no tokens/proofs',initial:before.worlds.find(w=>w.owner.attachmentId===s.games[0]!.attachmentId),held:me,restored,reviewed,queryMeasurement:{roomReadQueries:measurements,reason:'room/current run/release, current owner rows, rejoin participant rows; remaining global synchronization/projection work not optimized'}};
  await writeFile('docs/g3/server-connection-cr1-admission-fixtures.json',JSON.stringify(fixtures,null,2)+'\n');
 }finally{await f.close();}
});

test('CR1 real delayed preparation cancel after proof-bound owner replacement is refused, peer continues',async()=>{
 const f=await startG3Fixture({databaseUrl,rejoin:true,loot01Runtime:true,live:true,tls:true});try{
  const s=await provisionG3Fixture(f);await s.start();await s.playing();
  const old=(await f.service.db.transaction(tx=>tx.query('SELECT preparation_id FROM attachments WHERE id=$1',[s.games[0]!.attachmentId]))).rows[0].preparation_id;
  const rpc=async(c:any,p:any,payload:any)=>{const a=p?.association;const result=JSON.parse((await f.call('/v3/rejoin/rpc',{major:1,revision:'g3-rejoin-1',serviceId:f.service.db.serviceId,serviceEpoch:f.service.db.epoch,sessionEpoch:c.sessionEpoch,requestId:randomUUID(),...(a?{roomId:a.roomId,runId:a.runId,participantId:a.participantId,worldToken:a.worldToken,configRevision:a.configRevision,generation:p.generation}:{}),...payload},c.accessToken)).body);assert.ok(!result.error,result.error?.code);return result;};
  const prior=(await f.service.db.transaction(tx=>readRoomAdmissions(tx,f.service.db,f.profile.compatibility,Date.now(),s.room.roomId!))).worlds.find(w=>w.owner.attachmentId===s.games[0]!.attachmentId)!;
  const original=await rpc(s.desktops[0],undefined,{status:{}});await f.loseParticipant(original.participant.association.participantId);
  const instance='cr1-return-'+randomUUID(),returned=await rpc(s.desktops[0],original.participant,{resume:{resumeProof:original.resumeProof,gameInstanceId:instance,gameVersion:f.profile.compatibility.gameVersion,modVersion:f.profile.compatibility.modVersion,bridgeVersion:f.profile.compatibility.bridgeVersion,capabilities:f.profile.compatibility.capabilities}});
  await rpc(returned.credentials,returned.participant,{observe:{...s.observations[0]!,attachmentId:returned.credentials.attachmentId,gameInstanceId:instance,observationRevision:1,protected:true}});
  await assert.rejects(s.rpc(s.desktops[0],{preparationCancel:{preparationId:old}}),/STALE_EPOCH/);
  const status=await rpc(s.desktops[0],returned.participant,{status:{}});assert.equal(status.participant.generation,2);assert.equal(status.participant.returnEligible,true);assert.ok((await s.rpc(s.games[1],{roomGet:{}})).release);
  const resumedAdmission=(await f.service.db.transaction(tx=>readRoomAdmissions(tx,f.service.db,f.profile.compatibility,Date.now(),s.room.roomId!))).worlds.find(w=>w.owner.attachmentId===returned.credentials.attachmentId)!;
  assert.equal(prior.scope.worldToken,resumedAdmission.scope.worldToken);assert.equal(prior.participantId,resumedAdmission.participantId);assert.equal(resumedAdmission.generation,2);
  const path='docs/g3/server-connection-cr1-admission-fixtures.json',fixtures=JSON.parse(await readFile(path,'utf8'));fixtures.pairs.resumedOwner={before:prior,after:resumedAdmission};await writeFile(path,JSON.stringify(fixtures,null,2)+'\n');
 }finally{await f.close();}
});
