import test from 'node:test';
import assert from 'node:assert/strict';
import {randomUUID} from 'node:crypto';
import {writeFile} from 'node:fs/promises';
import {startG3Fixture,provisionG3Fixture} from '../scripts/g3-loot01-fixture.js';
import {schema as connectionSchema} from '../../../packages/contracts/src/g3-connection.js';
import {schema as waitingSchema} from '../../../packages/contracts/src/g3-waiting.js';

test('SE1 staging member health and passive preview succeed; gameplay admission requires actual Start',async()=>{
 let now=Date.now();
 const f=await startG3Fixture({rejoin:true,loot01Runtime:true,live:true,tls:true,databaseUrl:'postgres://nt2_g1:local-fixture-only@127.0.0.1:54328/nt2_g1',now:()=>now});
 const evidence:any={gate:'G3A reconnect/rejoin before A2',engineFacts:'synthetic protected loaded report',schema:f.namespace,observations:[]};
 try{
  const s=await provisionG3Fixture(f,{consumerPlayer:1,beforeRpc:()=>{now+=120;}});
  // Public fixture provisions two identities; the unused peer leaves before the
  // reproduction, leaving exactly one authenticated staging member/attachment.
  await s.rpc(s.desktops[1],{roomLeave:{}});
  const desktop=s.desktops[0]!;let game=s.games[0]!;
  const room=await s.rpc(desktop,{roomGet:{}});assert.equal(room.room!.state,'staging');assert.equal(room.room!.members!.length,1);
  const call=async(endpoint:string,revision:string,credential:any,payload:any)=>{
   now+=120;const request={major:1,revision,serviceId:f.service.db.serviceId,serviceEpoch:f.service.db.epoch,sessionEpoch:credential.sessionEpoch,requestId:randomUUID(),...payload};
   const response=await f.call(endpoint,request,credential.accessToken),value=JSON.parse(response.body);
   evidence.observations.push({endpoint,operation:Object.keys(payload)[0],httpStatus:response.status,error:value.error?.code??null,hasAdmission:!!value.admission,hasCanonicalAdmission:!!value.canonicalAdmission,mayPlay:value.entryPreview?.mayPlay});
   return value;
  };
  const admission=()=>call('/v3/connection/rpc',connectionSchema.revision,game,{admission:{roomId:s.room.roomId}});
  const preview=()=>call('/v3/waiting/rpc',waitingSchema.revision,desktop,{entryPreview:{roomId:s.room.roomId}});
  for(let tick=0;tick<4;tick++){
   const report=await s.rpc(game,{gameObservation:{...s.observations[0]!,observationRevision:tick+2}});
   assert.equal(report.canonicalAdmission,undefined);
   evidence.observations.push({endpoint:'/v1/rpc',operation:'gameObservation',error:null});
   const pulse=await s.rpc(game,{gamePulse:{attachmentId:game.attachmentId!,gameInstanceId:s.observations[0]!.gameInstanceId!,worldToken:s.observations[0]!.worldToken!}});
   assert.equal(pulse.canonicalAdmission,undefined);
   evidence.observations.push({endpoint:'/v1/rpc',operation:'gamePulse',error:null});
   const current=await s.rpc(game,{roomGet:{}});assert.equal(current.gameStatus!.phase,'hold');assert.equal(current.room!.members![0]!.ready,true);
   evidence.observations.push({endpoint:'/v1/rpc',operation:'roomGet',error:null,phase:current.gameStatus!.phase,ready:current.room!.members![0]!.ready});
   const renewed=await s.rpc(desktop,{gameRenew:{attachmentId:game.attachmentId!,gameInstanceId:s.observations[0]!.gameInstanceId!,worldToken:s.observations[0]!.worldToken!,gameSessionId:game.sessionId!,gameSessionEpoch:game.sessionEpoch!}});
   assert.equal(renewed.credentials!.sessionId,game.sessionId);game=renewed.credentials!;
   evidence.observations.push({endpoint:'/v1/rpc',operation:'gameRenew',error:null,sameSession:true});
   const p=await preview();assert.equal(p.error,undefined);assert.equal(p.entryPreview.mayPlay,false);assert.ok(p.entryPreview.scope.basePreparationId);
   const a=await admission();assert.equal(a.error.code,'FORBIDDEN');assert.equal(a.admission,undefined);
  }
  const persisted=await f.service.db.transaction(async tx=>({
   membership:(await tx.query('SELECT ready_config_revision,selected FROM memberships WHERE room_id=$1',[s.room.roomId])).rows,
   owner:(await tx.query('SELECT a.revoked_at,a.loaded,p.state FROM attachments a JOIN preparations p ON p.id=a.preparation_id WHERE a.id=$1',[game.attachmentId])).rows[0],
   releases:+(await tx.query('SELECT count(*) FROM inspected_releases WHERE run_id=$1',[s.room.runId])).rows[0].count,
   entryEffects:+(await tx.query('SELECT count(*) FROM startup_entry_effects WHERE run_id=$1',[s.room.runId])).rows[0].count
  }));
  assert.equal(persisted.membership.length,1);assert.equal(persisted.membership[0].selected,false);assert.equal(persisted.owner.revoked_at,null);assert.equal(persisted.owner.state,'loaded');assert.equal(persisted.releases,0);assert.equal(persisted.entryEffects,0);evidence.stagingState=persisted;
  await s.start();const authorized=await admission();assert.equal(authorized.error,undefined);assert.equal(authorized.admission.decision.authorized,true);assert.ok(authorized.admission.originalStart.requestId);
  const afterStart=await preview();assert.equal(afterStart.error.code,'ENTRY_NOT_STAGING');
  evidence.finding='Same healthy owner: report/pulse/renew/entryPreview succeed; admission returns FORBIDDEN solely because no canonical Start exists. After actual Start the same admission read succeeds. Preview is staging-only and creates no entry effect.';
 }finally{
  await f.close();evidence.cleanedSchema=f.namespace;evidence.cleanup='fixture verified absent';
  await writeFile('docs/g3/server-se1-staging-health-reproduction.json',JSON.stringify(evidence,null,2)+'\n');
 }
});
