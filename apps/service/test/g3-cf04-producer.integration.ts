import test from 'node:test';import assert from 'node:assert/strict';
import {randomUUID} from 'node:crypto';import {writeFile} from 'node:fs/promises';
import {startG3Fixture,provisionG3Fixture} from '../scripts/g3-loot01-fixture.js';
import {InspectionAuthority} from '../src/g2-next/inspection.js';

test('CF04 producer characterizes frozen Start, renewable lease, repeated report, entry and recovered ownership',async()=>{
 // Explicit Noita fixture address; never inherit an unrelated database env var.
 const databaseUrl='postgres://nt2_g1:local-fixture-only@127.0.0.1:54328/nt2_g1';
 const address=new URL(databaseUrl);assert.equal(address.hostname,'127.0.0.1');assert.equal(address.port,'54328');assert.equal(address.pathname,'/nt2_g1');
 let now=Date.now();const f=await startG3Fixture({databaseUrl,rejoin:true,loot01Runtime:true,live:true,tls:true,now:()=>now});
 const evidence:any={checkpoint:'CF04-S',database:'127.0.0.1:54328/nt2_g1',schema:f.namespace,engineFacts:'injected; production-service HTTP modules; no native/game execution',checks:[]};
 const diff=(a:any,b:any)=>[...new Set([...Object.keys(a),...Object.keys(b)])].filter(k=>JSON.stringify(a[k])!==JSON.stringify(b[k])).sort();
 const checked=(name:string)=>evidence.checks.push(name);
 try{
  assert.match(f.namespace,/^g3_rejoin_[0-9a-f]{32}$/);
  const s=await provisionG3Fixture(f,{beforeRpc:()=>{now+=300;}});
  const dbRun=await f.service.db.transaction(async tx=>(await tx.query('SELECT review_until,review_minutes FROM runs WHERE id=$1',[s.room.runId])).rows[0]);
  assert.equal(dbRun.review_minutes,30);evidence.defaultReviewMinutes=30;evidence.originalPreparationReviewExpiresAt=dbRun.review_until.toISOString();
  // Compress only this disposable run's pre-Start review window for a deterministic
  // boundary check. Production default and all production code remain unchanged.
  const deadline=now+60000;
  await f.service.db.transaction(tx=>tx.query('UPDATE runs SET review_until=$2 WHERE id=$1',[s.room.runId,new Date(deadline)]));
  evidence.fixtureReviewDeadline=new Date(deadline).toISOString();evidence.fixtureOnlyDeadlineCompression=true;
  await s.start();
  const initial:any=(await s.rpc(s.games[0],{roomGet:{}})).release;assert.ok(initial);assert.equal(initial.phase,'authorized');
  const frozen=await f.service.db.transaction(async tx=>(await tx.query('SELECT x.selected_reports,n.review_until FROM inspected_releases x JOIN runs n ON n.id=x.run_id WHERE x.run_id=$1',[s.room.runId])).rows[0]);
  const selected=frozen.selected_reports.participants.find((p:any)=>p.attachmentId===s.games[0]!.attachmentId);
  assert.equal(initial.capturedAt,selected.capturedAt);assert.equal(initial.observationRevision,1);
  assert.ok(Date.parse(initial.releaseExpiresAt)<deadline);checked('initial generation1 uses renewable lease instead of frozen review deadline');
  const rejoin=async(c:any,p:any,operation:any)=>{
   const a=p?.association,req={major:1,revision:'g3-rejoin-1',serviceId:f.service.db.serviceId,serviceEpoch:f.service.db.epoch,sessionEpoch:c.sessionEpoch,requestId:randomUUID(),...(a?{participantId:a.participantId,roomId:a.roomId,runId:a.runId,worldToken:a.worldToken,configRevision:a.configRevision,generation:p.generation}:{}),...operation};
   const result=JSON.parse((await f.call('/v3/rejoin/rpc',req,c.accessToken)).body);assert.ok(!result.error,result.error?.code);return result;
  };
  const owner=await rejoin(s.desktops[0],undefined,{status:{}});assert.equal(owner.participant.generation,1);
  // Prompt peer enters; delayed owner continues ordinary pulse/roomGet only.
  await s.rpc(s.games[1],{gameObservation:{...s.observations[1]!,observationRevision:2,protected:false}});
  const peer:any=(await s.rpc(s.games[1],{roomGet:{}})).release;assert.equal(peer.phase,'playing');
  const renewed:any[]=[];
  for(let i=0;i<3;i++){now+=5000;await s.pulse();const release:any=(await s.rpc(s.games[0],{roomGet:{}})).release;assert.ok(release);assert.deepEqual(diff(initial,release),['releaseExpiresAt']);renewed.push(release);}
  evidence.initial=initial;evidence.renewed=renewed.at(-1);evidence.pollOnlyChangedFields=diff(initial,renewed.at(-1));checked('three ordinary poll/renewal cycles change only releaseExpiresAt; healthy peer remains playing');
  now+=1000;await s.rpc(s.games[0],{gameObservation:s.observations[0]!});
  const repeated:any=(await s.rpc(s.games[0],{roomGet:{}})).release;
  assert.ok(diff(renewed.at(-1),repeated).includes('capturedAt'));assert.ok(diff(renewed.at(-1),repeated).every(k=>['capturedAt','releaseExpiresAt'].includes(k)));assert.equal(repeated.observationRevision,1);assert.equal(repeated.contentDigest,initial.contentDigest);
  evidence.repeatedIdenticalObservation=repeated;evidence.repeatedChangedFields=diff(renewed.at(-1),repeated);checked('identical revision/body observation rewrites capturedAt and lease without changing report digest');
  const originalSelected=await f.service.db.transaction(async tx=>(await tx.query('SELECT selected_reports FROM inspected_releases WHERE run_id=$1',[s.room.runId])).rows[0]);
  assert.equal(originalSelected.selected_reports.participants.find((p:any)=>p.attachmentId===s.games[0]!.attachmentId).capturedAt,selected.capturedAt);
  while(now<=deadline){now+=5000;await s.pulse();}
  const afterDeadline:any=(await s.rpc(s.games[0],{roomGet:{}})).release;assert.ok(afterDeadline);assert.ok(Date.parse(afterDeadline.releaseExpiresAt)>now);assert.equal(afterDeadline.phase,'authorized');
  const baseStatus=async(plainBinding:boolean)=>f.service.db.transaction(async tx=>{
   const actor=await f.service.auth.authenticate(tx,s.games[0]!.accessToken!);
   const receiver:any=Object.create(f.service.rooms.inspection);if(plainBinding)receiver.binding=InspectionAuthority.prototype.binding;
   const req:any={major:1,revision:initial.contractRevision,serviceId:f.service.db.serviceId,serviceEpoch:f.service.db.epoch,sessionEpoch:s.games[0]!.sessionEpoch,roomId:s.room.roomId,runId:s.room.runId,runEpoch:s.room.runEpoch,configRevision:s.room.configRevision};
   return InspectionAuthority.prototype.status.call(receiver,tx,actor,req);
  });
  const plain=await baseStatus(true),dynamic=await baseStatus(false);
  assert.equal(plain.release,undefined);assert.equal(plain.gameStatus?.phase,'revoked');assert.ok(dynamic.release);assert.equal(Date.parse(dynamic.release!.releaseExpiresAt!),deadline);
  evidence.afterOriginalDeadline=afterDeadline;evidence.basePlainBindingPhase=plain.gameStatus?.phase;evidence.baseDynamicBindingExpiry=dynamic.release!.releaseExpiresAt;
  checked('current rejoin generation1 still offers first entry after original deadline; base binding refuses but rejoin override bypasses bound');
  await s.rpc(s.games[0],{gameObservation:{...s.observations[0]!,observationRevision:2,protected:false}});
  const playing:any=(await s.rpc(s.games[0],{roomGet:{}})).release;assert.equal(playing.phase,'playing');assert.equal(playing.observationRevision,2);assert.equal(playing.contentDigest,initial.contentDigest);
  const entries=await f.service.db.transaction(async tx=>(await tx.query('SELECT entered_playing_at FROM authorization_play_entries WHERE attachment_id=$1',[s.games[0]!.attachmentId])).rows);assert.equal(entries.length,1);
  evidence.playing=playing;evidence.entryRecorded=true;checked('unprotected higher revision changes phase/revision/capturedAt and records entry; original selected report stays frozen');
  for(const ms of [20000,12000]){now+=ms;await s.rpc(s.desktops[1],{presence:{present:true}});await s.rpc(s.games[1],{gamePulse:{attachmentId:s.games[1]!.attachmentId!,gameInstanceId:s.observations[1]!.gameInstanceId!,worldToken:s.observations[1]!.worldToken!}});}assert.equal((await s.rpc(s.games[1],{roomGet:{}})).release!.phase,'playing');
  const status=await rejoin(s.desktops[0],owner.participant,{status:{}});assert.equal(status.participant.state,'reconnecting');
  const gameInstanceId='cf04-recovered-'+randomUUID(),resumed=await rejoin(s.desktops[0],status.participant,{resume:{resumeProof:owner.resumeProof,gameInstanceId,gameVersion:f.profile.compatibility.gameVersion,modVersion:f.profile.compatibility.modVersion,bridgeVersion:f.profile.compatibility.bridgeVersion,capabilities:f.profile.compatibility.capabilities}});
  assert.equal(resumed.participant.generation,2);
  const observed={...s.observations[0]!,attachmentId:resumed.credentials.attachmentId,gameInstanceId,observationRevision:1,protected:true};
  const accepted=await rejoin(resumed.credentials,resumed.participant,{observe:observed});assert.ok(accepted.release);assert.notEqual(accepted.release.authorizationId,initial.authorizationId);assert.notEqual(accepted.release.attachmentId,initial.attachmentId);assert.equal(accepted.release.startRequestId,initial.startRequestId);assert.equal(accepted.release.worldToken,initial.worldToken);
  evidence.recovered=accepted.release;evidence.recoveredGeneration=2;checked('explicit recovered ownership after original deadline has new authorization/session/attachment and generation, same original Start/world');
  const retired=await rejoin(s.desktops[0],accepted.participant,{offline:{resumeProof:owner.resumeProof}});assert.equal(retired.participant.returnEligible,false);const revoked=await rejoin(s.desktops[0],retired.participant,{status:{}});assert.equal(revoked.release,undefined);checked('explicit offline retires the recovered owner and emits no release');
 }finally{await f.close();evidence.fixtureClosed=true;}
 if(process.env.NT2_CF04_EVIDENCE){assert.equal(process.env.NT2_CF04_EVIDENCE,'docs/g3/server-cf04-producer-observations-20261015.json');await writeFile(process.env.NT2_CF04_EVIDENCE,JSON.stringify(evidence,null,2)+'\n');}
});
