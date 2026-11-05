// Actual routes/TLS/SQL and host Start; paused observations and clock are synthetic.
import test from 'node:test';import assert from 'node:assert/strict';import {randomUUID} from 'node:crypto';
import {startLifecycleLc1Fixture} from '../scripts/lifecycle-lc1-fixture.js';
import {schema as lobby} from '../../../packages/contracts/src/g2-next.js';
type Fixture=Awaited<ReturnType<typeof startLifecycleLc1Fixture>>;
async function run(label:string,fn:(x:Fixture)=>Promise<void>){const x=await startLifecycleLc1Fixture();try{
 // Initial roomJoin and later engine-loaded report are distinct events. Otherwise
 // the frozen clock ties issued_at of a retired unbound and actual loaded prep.
 x.advance(1);await fn(x);console.log(JSON.stringify({presenceRegression:label,schema:x.fixture.namespace,passed:true,synthetic:'controlled service clock and protected loaded observations; no actual Noita pause'}));}finally{await x.close();}}
async function facts(x:Fixture){return x.fixture.service.db.transaction(async tx=>(await tx.query(`SELECT w.generation,w.high_water,w.owner_attempt,w.supersede_after,a.id AS attachment_id,a.presence_until AS game_until,m.presence_until AS desktop_until,m.ready_config_revision,o.revision,o.fingerprint,o.captured_at,o.valid_until FROM world_associations w JOIN attachments a ON a.id=w.attachment_id JOIN memberships m ON m.desktop_id=a.desktop_id JOIN world_observations o ON o.attachment_id=a.id WHERE w.world_id=$1`,[x.association.worldId])).rows[0]);}
async function member(x:Fixture,desktopId=x.desktop.sessionId,actor=x.host){const result=await x.rpc(actor,{roomGet:{}});return result.room.members.find((m:any)=>m.desktopId===desktopId);}

test('desktop presence survives absent observations without reviving game readiness or extending cached presence replay',()=>run('independent-desktop-only',async x=>{
 await x.send(x.desktop,x.request());const original=await facts(x);x.advance(31000);assert.equal((await member(x)).readinessReason,'desktop_offline');
 const body={major:1,revision:lobby.revision,serviceId:x.fixture.service.db.serviceId,serviceEpoch:x.fixture.service.db.epoch,sessionEpoch:x.desktop.sessionEpoch,requestId:randomUUID(),roomId:x.room.roomId,configRevision:x.room.configRevision,presence:{present:true}};
 const send=async()=>JSON.parse((await x.fixture.call('/v1/rpc',body,x.desktop.accessToken)).body);
 assert.equal((await send()).error,undefined);const renewed=await facts(x);assert.ok(+renewed.desktop_until>+original.desktop_until);assert.equal(+renewed.game_until,+original.game_until);assert.equal(renewed.ready_config_revision,original.ready_config_revision);assert.equal((await member(x)).readinessReason,'game_offline');
 x.advance(10000);assert.equal((await send()).error,undefined);assert.equal(+(await facts(x)).desktop_until,+renewed.desktop_until);
 await x.rpc(x.desktop,{presence:{present:true}});assert.ok(+(await facts(x)).desktop_until>+renewed.desktop_until);
 await assert.rejects(x.start(),/NOT_READY/);assert.equal((await x.state()).participants.length,0);assert.equal(x.startDiagnostics().at(-1)?.outcome,'not-committed');
 console.log(JSON.stringify({desktopOnly:{reason:'game_offline',consentPreserved:true,reportRevision:renewed.revision,reportCaptureUnchanged:+renewed.captured_at===+original.captured_at,gameLeaseUnchanged:true,cachedPresenceDidNotRenew:true,hostStartRefused:true}}));
}));

test('identical current observations renew both leases past30s without inventing a new capture or releasing before Start',()=>run('exact-observation-current-owner',async x=>{
 const first=await x.send(x.desktop,x.request());const initial=await facts(x);let value=first;
 for(let i=0;i<4;i++){x.advance(12000);value=await x.send(x.desktop,x.request());assert.equal(value.world.state,'waiting');assert.equal(value.world.mayPlay,false);assert.equal(value.world.generation,1);assert.equal(value.credentials.attachmentId,first.credentials.attachmentId);assert.equal(value.world.ownerRemainingMs,30000);}
 const held=await facts(x);assert.equal(held.revision,initial.revision);assert.equal(held.fingerprint,initial.fingerprint);assert.equal(+held.captured_at,+initial.captured_at);assert.ok(+held.game_until>+initial.game_until);assert.ok(+held.desktop_until>+initial.desktop_until);assert.equal((await member(x)).ready,true);
 const conflict=x.request();conflict.reconcile!.observation!.protected=false;assert.equal((await x.send(x.desktop,conflict)).error.code,'CONFLICT');assert.equal(+(await facts(x)).game_until,+held.game_until);
 await x.start();const playing=await x.send(x.desktop,x.request());assert.equal(playing.world.mayPlay,true);assert.equal(playing.world.generation,1);
 console.log(JSON.stringify({exactObservation:{elapsedInjectedMs:48000,unchangedReportCapture:true,unchangedGeneration:true,bothLeasesRenewed:true,modifiedSameRevisionRefused:true,actualHostStart:true}}));
}));

test('healthy peer stays ready during consumer observation gap, protected resume replaces expired attachment and real Start selects both',()=>run('paused-gap-resume-peer',async x=>{
 const peer=await x.bootstrapPeer();x.advance(1);const q=x.request(peer.desktop,1,'peer',peer.observation);q.reconcile!.association=peer.association;q.reconcile!.proof=peer.proof;
 const a=await x.send(x.desktop,x.request());assert.equal((await x.send(peer.desktop,q)).world.state,'waiting');const initial=await facts(x);
 for(let i=0;i<4;i++){x.advance(10000);await x.rpc(x.desktop,{presence:{present:true}});assert.equal((await x.send(peer.desktop,{...q,requestId:randomUUID()})).world.generation,1);}
 assert.equal((await member(x)).readinessReason,'game_offline');assert.equal((await member(x,peer.desktop.sessionId)).ready,true);await assert.rejects(x.start(),/NOT_READY/);assert.equal((await x.state()).participants.length,0);
 const resumed=x.request(x.desktop,1,'lc1-connection-1',{...x.observation,observationRevision:2});const b=await x.send(x.desktop,resumed);assert.equal(b.world.state,'waiting');assert.equal(b.world.mayPlay,false);assert.equal(b.world.generation,2);assert.notEqual(b.credentials.attachmentId,a.credentials.attachmentId);assert.equal(b.world.association.worldId,x.association.worldId);
 await assert.rejects(x.rpc(a.credentials,{roomGet:{}}),/UNAUTHENTICATED/);assert.ok(+(await facts(x)).captured_at>+initial.captured_at);assert.equal((await member(x)).ready,true);
 await x.send(peer.desktop,{...q,requestId:randomUUID()});await x.start();assert.equal((await x.state()).participants.length,2);assert.equal((await x.send(x.desktop,{...resumed,requestId:randomUUID()})).world.mayPlay,true);assert.equal((await x.send(peer.desktop,{...q,requestId:randomUUID()})).world.mayPlay,true);
}));

test('independent desktop presence and existing gamePulse keep inspected paused owner includable without resending any observation',()=>run('three-minute-presence-and-game-health',async x=>{
 const first=await x.send(x.desktop,x.request());const capture=await facts(x);const pulse={gamePulse:{attachmentId:first.credentials.attachmentId,gameInstanceId:x.observation.gameInstanceId,worldToken:x.association.worldId}};
 for(let i=0;i<18;i++){x.advance(10000);await x.rpc(x.desktop,{presence:{present:true}});await x.rpc(first.credentials,pulse);}
 const held=await facts(x);assert.equal(+held.captured_at,+capture.captured_at);assert.equal(held.revision,capture.revision);assert.equal(held.fingerprint,capture.fingerprint);assert.equal((await member(x)).ready,true);
 await x.start();const paused=await x.rpc(first.credentials,pulse);assert.equal(paused.canonicalAdmission.decision.authorized,true);assert.equal(paused.canonicalAdmission.decision.playing,false);
 const validated=await x.send(x.desktop,x.request(x.desktop,1,'lc1-connection-1',{...x.observation,observationRevision:2}));assert.equal(validated.world.mayPlay,true);assert.equal(validated.canonicalAdmission.decision.playing,false);
 const entered=await x.send(x.desktop,x.request(x.desktop,1,'lc1-connection-1',{...x.observation,observationRevision:3,protected:false}));assert.equal(entered.canonicalAdmission.decision.playing,true);assert.equal(entered.world.generation,1);assert.equal(entered.credentials.attachmentId,first.credentials.attachmentId);
 console.log(JSON.stringify({pausedHealth:{injectedWaitMs:180000,noObservationResentDuringWait:true,originalCaptureAndRevisionUnchanged:true,desktopPresenceAndGamePulseSeparate:true,actualStartWhileHeld:true,currentProtectedReportValidatedOnResume:true,entryOnlyAfterUnprotectedReport:true}}));
}));

test('expired desktop and game plus auth turnover recover same staging world and actual Start without old credential',()=>run('both-expired-auth-turnover',async x=>{
 const initial=await x.send(x.desktop,x.request());x.advance(901000);await assert.rejects(x.rpc(x.desktop,{presence:{present:true}}),/UNAUTHENTICATED/);
 const refreshed=(await x.rpc(undefined,{refresh:{secret:x.desktop.refreshSecret,deviceId:x.desktop.deviceId}})).credentials;
 await x.rpc(refreshed,{presence:{present:true}});assert.equal((await member(x,x.desktop.sessionId,refreshed)).readinessReason,'game_offline');
 const q=x.request(refreshed,1,'lc1-connection-1',{...x.observation,observationRevision:2});const recovered=await x.send(refreshed,q);assert.equal(recovered.world.state,'waiting');assert.equal(recovered.world.generation,2);assert.equal(recovered.world.association.worldId,x.association.worldId);assert.notEqual(recovered.credentials.attachmentId,initial.credentials.attachmentId);await x.start();assert.equal((await x.send(refreshed,{...q,requestId:randomUUID()})).world.mayPlay,true);
}));

test('desktop presence and old game pulse cannot extend superseded owner cutoff or resurrect ended world',()=>run('stale-owner-and-terminal-fences',async x=>{
 const old=await x.send(x.desktop,x.request());x.advance(10000);const next=x.request(x.desktop,2,'new-context',{...x.observation,gameInstanceId:'new-context',observationRevision:2});const held=await x.send(x.desktop,next);assert.equal(held.world.reason,'OWNER_ACTIVE');const cutoff=+(await facts(x)).supersede_after;
 x.advance(10000);await x.rpc(x.desktop,{presence:{present:true}});await x.rpc(old.credentials,{gamePulse:{attachmentId:old.credentials.attachmentId,gameInstanceId:x.observation.gameInstanceId,worldToken:x.association.worldId}});assert.ok(+(await facts(x)).game_until<=cutoff);assert.equal((await x.send(x.desktop,x.request())).error.code,'STALE_ATTEMPT');
 x.advance(11000);const current=await x.send(x.desktop,{...next,requestId:randomUUID()});assert.equal(current.world.generation,2);await assert.rejects(x.rpc(old.credentials,{roomGet:{}}),/UNAUTHENTICATED/);
 await x.rpc(x.desktop,{presence:{present:true}});assert.equal((await facts(x)).owner_attempt,2);await x.start();await x.end();await assert.rejects(x.rpc(x.desktop,{presence:{present:true}}),/FORBIDDEN/);const ended=await x.send(x.desktop,{...next,requestId:randomUUID()});assert.equal(ended.world.state,'terminal');assert.equal(ended.world.mayPlay,false);
}));

test('three-minute unchanged inspection wait remains includable for Start during simulated pause and enters only after resumed report',()=>run('three-minute-start-during-pause',async x=>{
 const initial=await x.send(x.desktop,x.request());const captured=await facts(x);
 for(let i=0;i<18;i++){x.advance(10000);await x.rpc(x.desktop,{presence:{present:true}});const held=await x.send(x.desktop,x.request());assert.equal(held.world.state,'waiting');assert.equal(held.world.mayPlay,false);assert.equal(held.world.generation,1);}
 const waiting=await facts(x);assert.equal(+waiting.captured_at,+captured.captured_at);assert.equal(waiting.revision,captured.revision);assert.equal(waiting.fingerprint,captured.fingerprint);assert.equal((await member(x)).ready,true);
 await x.start();const authorized=await x.send(x.desktop,x.request());assert.equal(authorized.world.mayPlay,true);assert.equal(authorized.canonicalAdmission.decision.authorized,true);assert.equal(authorized.canonicalAdmission.decision.playing,false);assert.equal(authorized.credentials.attachmentId,initial.credentials.attachmentId);
 x.advance(10000);const stillPaused=await x.send(x.desktop,x.request());assert.equal(stillPaused.canonicalAdmission.decision.playing,false);
 const resume=x.request(x.desktop,1,'lc1-connection-1',{...x.observation,observationRevision:2,protected:false});const entered=await x.send(x.desktop,resume);assert.equal(entered.world.mayPlay,true);assert.equal(entered.canonicalAdmission.decision.playing,true);assert.equal(entered.world.generation,1);assert.equal(entered.credentials.attachmentId,initial.credentials.attachmentId);assert.equal((await x.state()).participants.length,1);
 console.log(JSON.stringify({pausedStart:{injectedWaitMs:180000,inspectionCaptureUnchanged:true,hostStartDuringNoNewObservation:true,protectedUntilResumedReport:true,sameGenerationAndAttachment:true,automaticSameWorldEntry:true,secondHostStart:false,enginePauseIsSynthetic:true}}));
}));
