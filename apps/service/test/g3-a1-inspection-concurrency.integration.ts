import test from 'node:test';
import assert from 'node:assert/strict';
import {randomUUID} from 'node:crypto';
import {startG3Fixture,provisionG3Fixture} from '../scripts/g3-fixture.js';
import type {Credentials,Request,Response} from '../../../packages/contracts/g2-next/g2-next.js';

test('A1 inspection: two-desktop revision races reject attach without cancelling preparation; polls omit it; reports/renewal preserve exact world scope',async()=>{
 let now=Date.now();const f=await startG3Fixture({a1Runtime:true,nativeSocial:true,tls:true,now:()=>now});
 try{
  const s=await provisionG3Fixture(f,{beforeRpc:()=>{now+=1000;}});let room=s.room;
  const envelope=(c:Credentials,payload:Partial<Request>):Request=>({major:1,revision:'g2-draft-5',serviceId:f.service.db.serviceId,serviceEpoch:f.service.db.epoch,requestId:randomUUID(),sessionEpoch:c.sessionEpoch!,roomId:room.roomId!,runId:room.runId??'',runEpoch:room.runEpoch??'',configRevision:room.configRevision!,expectedRevision:room.revision!,...payload});
  const send=async(c:Credentials,request:Request)=>{now+=250;const response=await f.call('/v1/rpc',request,c.accessToken);assert.equal(response.status,200);const value=JSON.parse(response.body) as Response;if(value.room)room=value.room;return value;};
  const ok=async(c:Credentials,payload:Partial<Request>)=>{const r=await send(c,envelope(c,payload));assert.equal(r.error,undefined,JSON.stringify(r.error));return r;};
  const [a,b]=s.desktops as [Credentials,Credentials];
  await ok(s.host,{cancelRun:{}});await ok(a,{ready:{ready:true}});await ok(b,{ready:{ready:true}});await ok(s.host,{prepare:{}});
  const pa=(await ok(a,{prepare:{}})).preparation!;assert.equal(pa.state,'issued');
  const stalePrepare=envelope(b,{prepare:{}});await ok(a,{ready:{ready:true}});
  assert.equal((await send(b,stalePrepare)).error?.code,'STALE_REVISION');
  const pb=(await ok(b,{prepare:{}})).preparation!;assert.equal(pb.state,'issued');
  const revision=room.revision;const poll=await ok(a,{roomGet:{}});assert.equal(poll.preparation,undefined);assert.equal(room.revision,revision);
  const heartbeat=await ok(b,{presence:{present:true},expectedRevision:0});assert.equal(heartbeat.preparation,undefined);assert.equal(room.revision,revision);
  assert.equal((await ok(a,{prepare:{}})).preparation!.preparationId,pa.preparationId);
  const attachInput={preparationId:pa.preparationId!,gameInstanceId:'a1-concurrent-game-a',gameVersion:f.profile.compatibility.gameVersion,modVersion:f.profile.compatibility.modVersion,bridgeVersion:f.profile.compatibility.bridgeVersion,capabilities:f.profile.compatibility.capabilities};
  const staleAttach=envelope(a,{gameAttach:attachInput});await ok(b,{ready:{ready:true}});
  assert.equal((await send(a,staleAttach)).error?.code,'STALE_REVISION');assert.equal((await send(a,staleAttach)).error?.code,'STALE_REVISION');
  const recoveredPreparation=(await ok(a,{prepare:{}})).preparation!;assert.equal(recoveredPreparation.preparationId,pa.preparationId);assert.equal(recoveredPreparation.state,'issued');assert.equal(recoveredPreparation.expiresAt,pa.expiresAt);
  const incompatible=envelope(a,{gameAttach:{...attachInput,modVersion:'nt2-g3-dev-1',bridgeVersion:'g3-transport-1',capabilities:['core.seed','core.ghost']}});
  assert.equal((await send(a,incompatible)).error?.code,'INCOMPATIBLE_GAME');assert.equal((await send(a,incompatible)).error?.code,'INCOMPATIBLE_GAME');
  assert.equal((await send(a,envelope(a,{gameAttach:{...attachInput,capabilities:['core.seed','core.ghost']}}))).error?.code,'INCOMPATIBLE_GAME');
  const stillIssued=(await ok(a,{prepare:{}})).preparation!;assert.equal(stillIssued.state,'issued');assert.equal(stillIssued.preparationId,pa.preparationId);
  // Policy rejections rolled back; this ID has no successful result/fingerprint yet.
  const attachRequest={...incompatible,gameAttach:attachInput};const attached=await send(a,attachRequest);assert.equal(attached.error,undefined);const ga=attached.credentials!;
  const replay=await send(a,attachRequest);assert.equal(replay.credentials!.accessToken,ga.accessToken);assert.equal(replay.credentials!.attachmentId,ga.attachmentId);
  assert.equal((await send(a,{...attachRequest,gameAttach:{...attachInput,gameInstanceId:'changed'}})).error?.code,'CONFLICT');
  const gb=(await ok(b,{gameAttach:{...attachInput,preparationId:pb.preparationId!,gameInstanceId:'a1-concurrent-game-b'}})).credentials!;
  const reportA={...s.observations[0]!,attachmentId:ga.attachmentId!,gameInstanceId:attachInput.gameInstanceId,worldToken:'a1-concurrent-world-a',observationRevision:1};
  const reportB={...s.observations[1]!,attachmentId:gb.attachmentId!,gameInstanceId:'a1-concurrent-game-b',worldToken:'a1-concurrent-world-b',observationRevision:1};
  const renew={attachmentId:ga.attachmentId!,gameInstanceId:attachInput.gameInstanceId,worldToken:reportA.worldToken,gameSessionId:ga.sessionId!,gameSessionEpoch:ga.sessionEpoch!};
  assert.equal((await ok(ga,{roomGet:{},expectedRevision:0})).gameStatus!.phase,'hold');
  assert.equal((await ok(s.host,{inspectionList:{limit:1},expectedRevision:0})).inspections!.length,0);
  await ok(a,{presence:{present:true}});await ok(b,{presence:{present:true}});
  assert.equal((await ok(a,{roomGet:{}})).room!.members!.filter(m=>m.present&&m.ready&&m.attached).length,2);
  assert.equal((await send(a,envelope(a,{gameRenew:renew,expectedRevision:0}))).error?.code,'RESTART_REQUIRED');
  assert.equal((await send(ga,envelope(ga,{gamePulse:{attachmentId:ga.attachmentId!,gameInstanceId:attachInput.gameInstanceId,worldToken:reportA.worldToken},expectedRevision:0}))).error?.code,'RESTART_REQUIRED');
  const reports=await Promise.all([send(ga,envelope(ga,{gameObservation:reportA,expectedRevision:0})),send(gb,envelope(gb,{gameObservation:reportB,expectedRevision:0}))]);for(const r of reports)assert.equal(r.error,undefined);
  let listed=await ok(s.host,{inspectionList:{limit:1},expectedRevision:0});assert.equal(listed.inspections!.length,1);assert.ok(listed.inspectionNextDesktopId);const second=await ok(s.host,{inspectionList:{limit:1,afterDesktopId:listed.inspectionNextDesktopId!},expectedRevision:0});assert.equal(second.inspections!.length,1);
  const beforeDuplicate=room.revision;await ok(ga,{gameObservation:reportA,expectedRevision:0});assert.equal(room.revision,beforeDuplicate);
  assert.equal((await send(ga,envelope(ga,{gameObservation:{...reportA,actualSeed:'424243'},expectedRevision:0}))).error?.code,'CONFLICT');
  assert.equal((await send(ga,envelope(ga,{gameObservation:{...reportA,worldToken:'foreign-world',observationRevision:2},expectedRevision:0}))).error?.code,'RESTART_REQUIRED');
  const renewed=(await ok(a,{gameRenew:renew,expectedRevision:0})).credentials!;assert.equal(renewed.sessionId,ga.sessionId);assert.equal(renewed.sessionEpoch,ga.sessionEpoch);assert.equal(renewed.attachmentId,ga.attachmentId);
  await ok(ga,{gamePulse:{attachmentId:ga.attachmentId!,gameInstanceId:attachInput.gameInstanceId,worldToken:reportA.worldToken},expectedRevision:0});
  await ok(ga,{gameObservation:{...reportA,observationRevision:2},expectedRevision:0});
  assert.equal((await send(ga,envelope(ga,{gameObservation:reportA,expectedRevision:0}))).error?.code,'STALE_REVISION');
  const mismatch=await send(ga,envelope(ga,{gameObservation:{...reportA,observationRevision:3},configRevision:room.configRevision!+1}));assert.equal(mismatch.error?.code,'STALE_REVISION');
  assert.equal((await ok(a,{prepare:{}})).preparation!.state,'loaded');
  await ok(s.host,{cancelRun:{}});assert.equal((await send(ga,envelope(ga,{gameObservation:{...reportA,observationRevision:3}}))).error?.code,'UNAUTHENTICATED');
 }finally{await f.close();}
});
