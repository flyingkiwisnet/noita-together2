import test from 'node:test';
import assert from 'node:assert/strict';
import {randomUUID} from 'node:crypto';
import {startG3Fixture,provisionG3Fixture} from '../scripts/g3-fixture.js';
import {schema} from '../../../packages/contracts/src/g2-next.js';
import type {Credentials,Request} from '../../../packages/contracts/g2-next/g2-next.js';

// Canonical HTTP counterpart, not execution of the GUI/native production scheduler.
// Logical time injects normal 2s control polls and explicit lost/delayed work.
test('A1 canonical timing separates phase restoration, empty poses and presence loss',async()=>{
 let now=Date.now();
 const f=await startG3Fixture({tls:true,a1Runtime:true,a1RuntimeFix:true,nativeSocial:true,now:()=>now});
 try{
  const s=await provisionG3Fixture(f,{beforeRpc:()=>{now+=100;}});
  const envelope=(c:Credentials,payload:Partial<Request>):Request=>({major:1,revision:schema.revision,serviceId:f.service.db.serviceId,serviceEpoch:f.service.db.epoch,sessionEpoch:c.sessionEpoch!,requestId:randomUUID(),roomId:s.room.roomId!,runId:s.room.runId!,runEpoch:s.room.runEpoch!,configRevision:s.room.configRevision!,expectedRevision:s.room.revision!,...payload});
  const send=async(c:Credentials,input:Request)=>{
   now+=100;const r=await f.call('/v1/rpc',input,c.accessToken);assert.equal(r.status,200);return JSON.parse(r.body);
  };
  const phaseRequest=(i:number,sequence:number,protectedPhase:boolean)=>envelope(s.games[i]!,{gameObservation:{...s.observations[i]!,observationRevision:sequence,protected:protectedPhase}});
  const ok=(r:any)=>{assert.equal(r.error,undefined,r.error?.code);return r;};
  const status=async(i:number)=>ok(await send(s.games[i]!,envelope(s.games[i]!,{roomGet:{}})));
  const pulseRequest=(i:number)=>envelope(s.games[i]!,{gamePulse:{attachmentId:s.games[i]!.attachmentId!,gameInstanceId:s.observations[i]!.gameInstanceId!,worldToken:s.observations[i]!.worldToken!}});
  const presenceUntil=async(i:number)=>f.service.db.transaction(async tx=>+(await tx.query('SELECT presence_until FROM attachments WHERE id=$1',[s.games[i]!.attachmentId])).rows[0].presence_until);
  const lane=async(i:number,social:boolean,payload:object)=>{
   now+=100;const c=s.games[i]!;
   const r=await f.call(social?'/v3/social/rpc':'/v3/rpc',{major:1,revision:social?'g3-social-draft-1':'g3-draft-1',serviceId:f.service.db.serviceId,serviceEpoch:f.service.db.epoch,sessionEpoch:c.sessionEpoch,requestId:randomUUID(),scope:s.scopes[i]!,...payload},c.accessToken);
   assert.equal(r.status,200);return JSON.parse(r.body);
  };
  const publish=(i:number,sequence:number)=>lane(i,false,{movement:{frames:[{sequence,xMilli:String(1000+i*1000),yMilli:'2000',vxMilli:'0',vyMilli:'0',facingLeft:false}]}});
  await s.start();
  const frozen=(await status(0)).release;assert.equal(frozen.phase,'authorized');
  assert.equal((await lane(0,false,{ghostList:{}})).error.code,'STALE_EPOCH');
  // A local unprotected report2 superseded before HTTP delivery cannot enter playing.
  ok(await send(s.games[0]!,phaseRequest(0,3,true)));
  const missedEntry=(await status(0)).release;
  assert.equal(missedEntry.phase,'authorized');assert.equal(missedEntry.enteredPlayingAt,undefined);
  assert.equal((await lane(0,false,{ghostList:{}})).error.code,'STALE_EPOCH');
  // An envelope captured before peer observation revision changes is still a valid read.
  const staleRead=envelope(s.desktops[0]!,{roomGet:{}});
  for(let i=0;i<2;i++)ok(await send(s.games[i]!,phaseRequest(i,i===0?4:2,false)));
  assert.ok(ok(await send(s.desktops[0]!,staleRead)).room.revision>staleRead.expectedRevision!);
  const entered=(await status(0)).release.enteredPlayingAt;assert.ok(entered);
  assert.equal(ok(await lane(0,false,{ghostList:{}})).ghosts.length,0,'live peer alone supplies no pose');
  ok(await publish(1,1));assert.equal(ok(await lane(0,false,{ghostList:{}})).ghosts.length,1);
  // Late report3 after report4 is refused, without rolling the canonical phase backwards.
  const delayed=phaseRequest(0,3,true);ok(await send(s.games[0]!,phaseRequest(0,4,false)));
  assert.equal((await send(s.games[0]!,delayed)).error.code,'STALE_REVISION');
  assert.equal((await status(0)).release.phase,'playing');
  for(let seq=5;seq<=10;seq++){
   now+=2000;
   // Production order: desktop presence/read, observation, game pulse/read.
   for(let i=0;i<2;i++){
    await s.rpc(s.desktops[i]!,{presence:{present:true}});
    ok(await send(s.desktops[i]!,envelope(s.desktops[i]!,{roomGet:{}})));
    if(i===0)ok(await send(s.games[i]!,phaseRequest(i,seq,seq%2===1)));
    ok(await send(s.games[i]!,pulseRequest(i)));
   }
   const release=(await status(0)).release;
   assert.equal(release.phase,seq%2===1?'restoring':'playing');
   assert.equal(release.authorizationId,frozen.authorizationId);
   assert.equal(release.observationRevision,1);
   assert.equal(release.enteredPlayingAt,entered);
   if(seq%2===1){
    assert.equal((await lane(0,false,{ghostList:{}})).error.code,'STALE_EPOCH');
    assert.equal(ok(await lane(1,false,{ghostList:{}})).ghosts.length,0);
   }else{
    ok(await publish(0,seq));assert.equal(ok(await lane(1,false,{ghostList:{}})).ghosts.length,1);
   }
  }
  // Repeated same observation and exact pulse retry recover, but do not refresh presence.
  const fixedPulse=pulseRequest(0);ok(await send(s.games[0]!,fixedPulse));
  const until=await presenceUntil(0);now+=1000;
  ok(await send(s.games[0]!,fixedPulse));
  ok(await send(s.games[0]!,phaseRequest(0,10,false)));
  assert.equal(await presenceUntil(0),until);
  assert.equal((await send(s.games[0]!,phaseRequest(0,10,true))).error.code,'CONFLICT');
  const g=s.games[0]!;
  const renewed=ok(await send(s.desktops[0]!,envelope(s.desktops[0]!,{gameRenew:{attachmentId:g.attachmentId!,gameSessionId:g.sessionId!,gameSessionEpoch:g.sessionEpoch!,gameInstanceId:s.observations[0]!.gameInstanceId!,worldToken:s.observations[0]!.worldToken!}}))).credentials;
  assert.equal(renewed.sessionId,g.sessionId);assert.equal(await presenceUntil(0),until);
  now+=2100;assert.equal(ok(await lane(1,false,{ghostList:{}})).ghosts.length,0);
  assert.equal((await status(0)).release.phase,'playing','pose TTL does not revoke permit');
  // Withhold desktop0 work, retain fresh game pulses: deliberately beyond normal cadence.
  for(let tick=0;tick<18;tick++){
   now+=2000;await s.rpc(s.desktops[1]!,{presence:{present:true}});
   for(let i=0;i<2;i++)ok(await send(s.games[i]!,pulseRequest(i)));
  }
  const absent=await status(0);assert.equal(absent.release.phase,'playing');
  assert.equal(absent.room.members.find((m:any)=>m.desktopId===s.desktops[0]!.sessionId).present,false);
  assert.equal((await lane(0,true,{snapshot:{}})).error.code,'STALE_EPOCH');
  ok(await publish(0,11));assert.equal(ok(await lane(1,false,{ghostList:{}})).ghosts.length,1);
  await s.rpc(s.desktops[0]!,{presence:{present:true}});
  ok(await lane(0,true,{snapshot:{}}));assert.equal((await status(0)).release.authorizationId,frozen.authorizationId);
  // Late pulse after expiry cannot resurrect a selected frozen authorization.
  const latePulse=pulseRequest(0);now=await presenceUntil(0);
  assert.equal((await send(s.games[0]!,latePulse)).error.code,'INVALID_STATE');
  const expired=await status(0);assert.equal(expired.room.state,'suspended');
  assert.equal(expired.gameStatus.reason,'selected_context_lost');assert.equal(expired.release,undefined);
  assert.equal((await lane(1,false,{ghostList:{}})).error.code,'FORBIDDEN');
 }finally{await f.close();}
});
