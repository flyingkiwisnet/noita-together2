import test from 'node:test';import assert from 'node:assert/strict';import {randomUUID} from 'node:crypto';
import {startG3Fixture,provisionG3Fixture} from '../scripts/g3-loot01-fixture.js';
import {schema} from '../../../packages/contracts/src/g3-rejoin.js';
// @ts-expect-error Production browser controller has no separate Node type layer.
import {HostClient} from '../../web/candidate/client.js';
async function setup(seedPolicy='shared-random',playerCount=2){
 let now=Date.now();const f=await startG3Fixture({rejoin:true,loot01Runtime:true,live:true,tls:true,now:()=>now});
 try{const s=await provisionG3Fixture(f,{seedPolicy,playerCount,beforeRpc:()=>{now+=300;}});await s.start();await s.playing();
  const call=async(c:any,part:any,operation:any,extra:any={})=>{const a=part?.association;const req={major:1,revision:schema.revision,serviceId:f.service.db.serviceId,serviceEpoch:f.service.db.epoch,sessionEpoch:c.sessionEpoch,requestId:randomUUID(),...(a?{participantId:a.participantId,roomId:a.roomId,runId:a.runId,worldToken:a.worldToken,configRevision:a.configRevision,generation:part.generation}:{}),...operation,...extra};const r=JSON.parse((await f.call('/v3/rejoin/rpc',req,c.accessToken)).body);return {r,req};};
  const associations=[];for(const d of s.desktops){const {r}=await call(d,undefined,{status:{}});assert.ok(!r.error,JSON.stringify(r));assert.equal(r.resumeProof.length,43);associations.push(r);}
  const resumeInput=(proof:string,i=0)=>({resumeProof:proof,gameInstanceId:'return-'+i+'-'+randomUUID(),gameVersion:f.profile.compatibility.gameVersion,modVersion:f.profile.compatibility.modVersion,bridgeVersion:f.profile.compatibility.bridgeVersion,capabilities:f.profile.compatibility.capabilities});
  const observed=(r:any,i=0,patch:any={})=>({...s.observations[i],attachmentId:r.credentials.attachmentId,gameInstanceId:r.req.resume.gameInstanceId,observationRevision:1,protected:true,...patch});
  return {f,s,call,associations,resumeInput,observed,advance:(ms:number)=>{now+=ms;}};
 }catch(e){await f.close();throw e;}
}
test('RJ1 outage >5 minutes holds only affected player; fresh inspection, generations and lost replies retain original world',async()=>{
 const h=await setup();const {f,s,call,associations,resumeInput,observed}=h;try{
  const original=associations[0],p=original.participant;
  f.service.live!.projection.watch(s.room.roomId!);
  assert.equal((await call(s.desktops[0],p,{resume:resumeInput(original.resumeProof)})).r.error.code,'OWNER_ACTIVE');
  assert.equal((await call(s.desktops[0],p,{resume:resumeInput(original.resumeProof)},{worldToken:'foreign-world'})).r.error.code,'FORBIDDEN');
  assert.equal((await call(s.desktops[0],p,{resume:resumeInput(original.resumeProof)},{configRevision:99})).r.error.code,'FORBIDDEN');
  for(let i=0;i<17;i++){h.advance(20000);await s.rpc(s.desktops[1],{presence:{present:true}});await s.rpc(s.games[1],{gamePulse:{attachmentId:s.games[1]!.attachmentId!,gameInstanceId:s.observations[1]!.gameInstanceId!,worldToken:s.observations[1]!.worldToken!}});}
  assert.equal(s.room.state,'running');assert.ok((await s.rpc(s.games[1],{roomGet:{}})).release);const status=(await call(s.desktops[0],p,{status:{}})).r;assert.equal(status.participant.state,'reconnecting');assert.equal(status.participant.returnEligible,true);
  assert.ok(f.service.live!.projection.member(s.room.roomId!,s.games[1]!.sessionId!));assert.equal(f.service.live!.projection.member(s.room.roomId!,s.games[0]!.sessionId!),undefined);
  const input=resumeInput(original.resumeProof),returned=await call(s.desktops[0],p,{resume:input});assert.ok(!returned.r.error,JSON.stringify(returned.r));assert.equal(returned.r.participant.generation,2);assert.notEqual(returned.r.credentials.sessionId,s.games[0]!.sessionId);assert.equal(returned.r.release,undefined);assert.equal(returned.r.participant.association.worldToken,p.association.worldToken);
  const replay=await call(s.desktops[0],p,{resume:input},{requestId:returned.req.requestId});assert.equal(replay.r.credentials.sessionId,returned.r.credentials.sessionId);assert.equal(replay.r.participant.generation,2);
  assert.equal((await call(s.desktops[0],p,{resume:{...input,gameInstanceId:'changed-request'}},{requestId:returned.req.requestId})).r.error.code,'CONFLICT');
  assert.equal((await call(s.desktops[0],p,{resume:input})).r.error.code,'OWNER_ACTIVE');
  assert.equal((await call(s.games[0],p,{pulse:{}})).r.error.code,'UNAUTHENTICATED');
  const r={...returned.r,req:returned.req},bad=await call(r.credentials,r.participant,{observe:observed(r,0,{actualSeed:'1'})});assert.equal(bad.r.participant.reason,'inspection_incompatible');assert.equal(bad.r.release,undefined);
  const accepted=await call(r.credentials,r.participant,{observe:observed(r,0,{observationRevision:2})});assert.ok(accepted.r.release,JSON.stringify(accepted.r));assert.equal(accepted.r.release.actualSeed,s.observations[0]!.actualSeed);assert.notEqual(accepted.r.release.authorizationId,p.association.originalStartId);
  assert.equal((await call(r.credentials,r.participant,{pulse:{}},{generation:1})).r.error.code,'STALE_EPOCH');
  assert.equal((await call(s.desktops[1],p,{resume:resumeInput(original.resumeProof)})).r.error.code,'NOT_FOUND');
  assert.equal((await call(s.desktops[0],r.participant,{resume:resumeInput('x'.repeat(43))})).r.error.code,'FORBIDDEN');
  assert.ok((await s.rpc(s.games[1],{roomGet:{}})).release);
  await call(r.credentials,r.participant,{observe:observed(r,0,{observationRevision:3,protected:false})});assert.ok(f.service.live!.projection.member(s.room.roomId!,r.credentials.sessionId));
  const held=await call(r.credentials,r.participant,{observe:observed(r,0,{observationRevision:4,actualSeed:'1'})});assert.equal(held.r.release,undefined);assert.equal(f.service.live!.projection.member(s.room.roomId!,r.credentials.sessionId),undefined);assert.ok(f.service.live!.projection.member(s.room.roomId!,s.games[1]!.sessionId!));
 }finally{await f.close();}
});
test('RJ1 same issuer restart retains eligible individual world, fences epochs and leaves operation journal immutable',async()=>{
 const h=await setup('individual-random');const {f,s,call,associations,resumeInput,observed}=h;try{
  const original=associations[0],p=original.participant,priorEpoch=f.service.db.epoch;
  const operationId=randomUUID(),deliveryId=randomUUID();await f.service.db.transaction(async tx=>{
   await tx.query("INSERT INTO g3_operations(run_id,principal_id,operation_id,attachment_id,world_token,direction,amount,digest,delivery_id,state,created_at) VALUES($1,$2,$3,$4,$5,'withdraw',1,$6,$7,'uncertain',$8)",[s.room.runId,s.desktops[0]!.accountId,operationId,s.games[0]!.attachmentId,s.observations[0]!.worldToken,'a'.repeat(64),deliveryId,new Date()]);
   await tx.query('INSERT INTO g3_ledger(run_id,principal_id,operation_id,delta,balance,created_at) VALUES($1,$2,$3,-1,0,$4)',[s.room.runId,s.desktops[0]!.accountId,operationId,new Date()]);
   await tx.query("INSERT INTO g3_outbox(run_id,principal_id,kind,body,created_at) VALUES($1,$2,'uncertain',$3,$4)",[s.room.runId,s.desktops[0]!.accountId,{operationId,deliveryId},new Date()]);
  });
  const journalBefore=await f.service.db.transaction(async tx=>({ledger:(await tx.query('SELECT * FROM g3_ledger ORDER BY sequence')).rows,outbox:(await tx.query('SELECT * FROM g3_outbox ORDER BY sequence')).rows,operations:(await tx.query('SELECT * FROM g3_operations ORDER BY operation_id')).rows}));
  await f.restart();assert.notEqual(f.service.db.epoch,priorEpoch);
  await assert.rejects(s.start(),/STALE_EPOCH|CONFLICT/);
  const stale=await call(s.desktops[0],p,{status:{}},{serviceEpoch:priorEpoch});assert.equal(stale.r.error.code,'STALE_EPOCH');
  const status=(await call(s.desktops[0],p,{status:{}})).r;assert.equal(status.participant.returnEligible,true);assert.equal(status.participant.state,'reconnecting');
  const returned=await call(s.desktops[0],status.participant,{resume:resumeInput(original.resumeProof)});assert.ok(!returned.r.error,JSON.stringify(returned.r));assert.notEqual(returned.r.participant.runEpoch,p.runEpoch);
  const r={...returned.r,req:returned.req},accepted=await call(r.credentials,r.participant,{observe:observed(r)});assert.ok(accepted.r.release,JSON.stringify(accepted.r));assert.equal(accepted.r.release.actualSeed,s.observations[0]!.actualSeed);assert.equal(accepted.r.release.seedPolicy,'individual-random');
  const after=await f.service.db.transaction(async tx=>({ledger:(await tx.query('SELECT * FROM g3_ledger ORDER BY sequence')).rows,outbox:(await tx.query('SELECT * FROM g3_outbox ORDER BY sequence')).rows,operations:(await tx.query('SELECT * FROM g3_operations ORDER BY operation_id')).rows}));assert.deepEqual(after,journalBefore);
  const a=await f.service.db.transaction(async tx=>f.service.auth.family(tx,s.desktops[0]!.accountId!,s.desktops[0]!.deviceId!,'desktop'));
  assert.equal((await call(a,r.participant,{resume:resumeInput(original.resumeProof)})).r.error.code,'OWNER_ACTIVE');h.advance(31000);
  const foreign=await f.service.db.transaction(async tx=>f.service.auth.family(tx,s.desktops[0]!.accountId!,'different-device','desktop'));assert.equal((await call(foreign,r.participant,{resume:resumeInput(original.resumeProof)})).r.error.code,'FORBIDDEN');
  const resumed=await call(a,r.participant,{resume:resumeInput(original.resumeProof)});assert.ok(!resumed.r.error,JSON.stringify(resumed.r));assert.equal(resumed.r.participant.generation,3);assert.equal((await call(r.credentials,r.participant,{pulse:{}})).r.error.code,'UNAUTHENTICATED');
  await s.rpc(s.host,{roomGet:{}});await s.rpc(s.host,{endRun:{}});await f.restart();const ended=(await call(a,resumed.r.participant,{status:{}})).r;assert.equal(ended.participant.returnEligible,false);assert.equal((await call(a,ended.participant,{resume:resumeInput(original.resumeProof)})).r.error.code,'RETURN_INELIGIBLE');
 }finally{await f.close();}
});
test('RJ1 changed advisory inspection requires exact host review; offline, End, kick, ban and guest policy are terminal',async()=>{
 const h=await setup('shared-random',3);const {f,s,call,associations,resumeInput,observed}=h;try{
  const original=associations[0],p=original.participant;h.advance(31000);await f.service.rooms.sweep();
  const returned=await call(s.desktops[0],p,{resume:resumeInput(original.resumeProof)});const r={...returned.r,req:returned.req};assert.ok(r.credentials,JSON.stringify(r));
  const changed=await call(r.credentials,r.participant,{observe:observed(r,0,{activeMods:['noita_together2','advisory-difference']})});assert.equal(changed.r.participant.state,'review');assert.equal(changed.r.release,undefined);
  const q=changed.r.participant;assert.equal((await call(s.host,q,{review:{participantId:q.association.participantId,observationRevision:1,contentDigest:'0'.repeat(64)}})).r.error.code,'REPORT_CHANGED');
  const listed=(await call(s.host,undefined,{list:{}},{roomId:s.room.roomId})).r;assert.equal(listed.participants.length,3);assert.ok(!JSON.stringify(listed).includes(original.resumeProof));assert.equal((await call(s.desktops[0],undefined,{list:{}},{roomId:s.room.roomId})).r.error.code,'FORBIDDEN');
  const browser=new HostClient(async(path:string,body?:any,token?:string)=>JSON.parse((await f.call(path,body,token)).body),randomUUID);browser.credentials=s.host;browser.info={serviceId:f.service.db.serviceId,serviceEpoch:f.service.db.epoch};assert.equal((await browser.rejoin({roomId:s.room.roomId,list:{}})).participants.length,3);
  const reviewed=await call(s.host,q,{review:{participantId:q.association.participantId,observationRevision:q.pendingObservationRevision,contentDigest:q.pendingContentDigest}});assert.equal(reviewed.r.participant.state,'active');assert.ok((await call(r.credentials,reviewed.r.participant,{pulse:{}})).r.release);
  const offline=await call(s.desktops[0],reviewed.r.participant,{offline:{resumeProof:original.resumeProof}});assert.equal(offline.r.participant.returnEligible,false);assert.equal((await call(s.desktops[0],offline.r.participant,{resume:resumeInput(original.resumeProof)})).r.error.code,'RETURN_INELIGIBLE');
  await s.rpc(s.host,{roomModerate:{accountId:s.desktops[1]!.accountId!,action:'ban'}});assert.equal(s.room.state,'running');const removed=(await call(s.desktops[1],associations[1].participant,{status:{}})).r;assert.equal(removed.participant.returnEligible,false);
  await f.service.db.transaction(tx=>tx.query('UPDATE rooms SET allow_guests=false WHERE id=$1',[s.room.roomId]));await f.service.rooms.sweep();assert.equal((await call(s.desktops[2],associations[2].participant,{status:{}})).r.participant.returnEligible,false);
  await s.rpc(s.host,{endRun:{}});assert.equal((await call(s.desktops[1],associations[1].participant,{resume:resumeInput(associations[1].resumeProof,1)})).r.error.code,'RETURN_INELIGIBLE');
  await f.restart();assert.equal((await call(s.desktops[1],associations[1].participant,{status:{}})).r.participant.returnEligible,false);
 }finally{await f.close();}
});
test('RJ1 kick, disabled access and explicit Leave retire only the affected participant; delayed offline cannot retire a newer owner',async()=>{
 const h=await setup('shared-fixed',3);const {f,s,call,associations,resumeInput}=h;try{
  await s.rpc(s.host,{roomModerate:{accountId:s.desktops[1]!.accountId!,action:'kick'}});assert.equal(s.room.state,'running');assert.ok((await s.rpc(s.games[0],{roomGet:{}})).release);assert.equal((await call(s.desktops[1],associations[1].participant,{status:{}})).r.participant.returnEligible,false);
  await f.service.db.transaction(tx=>tx.query('UPDATE accounts SET disabled=true WHERE id=$1',[s.desktops[2]!.accountId]));await f.service.rooms.sweep();const retired=await f.service.db.transaction(tx=>tx.query('SELECT terminal FROM rejoin_participants WHERE id=$1',[associations[2].participant.association.participantId]));assert.equal(retired.rows[0].terminal,true);
  const a=associations[0];h.advance(31000);const returned=await call(s.desktops[0],a.participant,{resume:resumeInput(a.resumeProof)});assert.equal(returned.r.participant.generation,2);assert.equal((await call(s.desktops[0],a.participant,{offline:{resumeProof:a.resumeProof}})).r.error.code,'STALE_EPOCH');
  await s.rpc(s.desktops[0],{roomLeave:{}});assert.equal(s.room.state,'running');assert.equal((await call(s.desktops[0],returned.r.participant,{resume:resumeInput(a.resumeProof)})).r.error.code,'RETURN_INELIGIBLE');
 }finally{await f.close();}
});
