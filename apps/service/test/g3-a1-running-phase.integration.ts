import test from 'node:test';
import assert from 'node:assert/strict';
import {randomUUID} from 'node:crypto';
import {startG3Fixture,provisionG3Fixture} from '../scripts/g3-fixture.js';
import {transientEventId} from '../../../packages/contracts/src/g3-social-replay.js';
import {schema} from '../../../packages/contracts/src/g2-next.js';
import type {Credentials,GameRelease,Request} from '../../../packages/contracts/g2-next/g2-next.js';

// Exact A1 social/TLS service, real HTTP authentication; engine/HUD facts are injected.
// One random owned schema only. No native process, installed service or real Noita.
test('A1 phase churn keeps frozen Start; current peer projections resume without re-Start',async()=>{
 let now=Date.now();
 const f=await startG3Fixture({tls:true,a1Runtime:true,nativeSocial:true,now:()=>now});
 try{
  const s=await provisionG3Fixture(f,{beforeRpc:()=>{now+=100;}});
  // Public lifecycle establishes a five-minute first-entry deadline without DB edits.
  await s.rpc(s.host,{cancelRun:{}});
  for(const d of s.desktops)await s.rpc(d,{ready:{ready:true}});
  await s.rpc(s.host,{prepare:{reviewMinutes:5}});
  for(let i=0;i<2;i++){
   const d=s.desktops[i]!,p=(await s.rpc(d,{prepare:{}})).preparation!;
   const g=(await s.rpc(d,{gameAttach:{preparationId:p.preparationId!,gameInstanceId:s.observations[i]!.gameInstanceId!,gameVersion:f.profile.compatibility.gameVersion,modVersion:f.profile.compatibility.modVersion,bridgeVersion:f.profile.compatibility.bridgeVersion,capabilities:f.profile.compatibility.capabilities}})).credentials!;
   s.games[i]=g;s.observations[i]={...s.observations[i]!,attachmentId:g.attachmentId!};
   await s.rpc(g,{gameObservation:s.observations[i]!});
   s.scopes[i]={...s.scopes[i]!,runId:s.room.runId!,runEpoch:s.room.runEpoch!,attachmentId:g.attachmentId!};
  }
  s.reports.length=0;let cursor='';
  do{const r=await s.rpc(s.host,{inspectionList:{limit:1,afterDesktopId:cursor}});s.reports.push(...r.inspections!);cursor=r.inspectionNextDesktopId??'';}while(cursor);
  const raw=async(c:Credentials,payload:Partial<Request>)=>{
   now+=100;const reply=await f.call('/v1/rpc',{major:1,revision:schema.revision,serviceId:f.service.db.serviceId,serviceEpoch:f.service.db.epoch,sessionEpoch:c.sessionEpoch,requestId:randomUUID(),roomId:s.room.roomId,runId:s.room.runId,runEpoch:s.room.runEpoch,configRevision:s.room.configRevision,expectedRevision:s.room.revision,...payload},c.accessToken);
   assert.equal(reply.status,200);return JSON.parse(reply.body);
  };
  const status=async(i:number)=>s.rpc(s.games[i]!,{roomGet:{}});
  const phase=async(i:number,revision:number,protectedPhase:boolean)=>s.rpc(s.games[i]!,{gameObservation:{...s.observations[i]!,observationRevision:revision,protected:protectedPhase}});
  const call=async(i:number,social:boolean,payload:object,scope=s.scopes[i]!,epoch=f.service.db.epoch)=>{
   now+=100;const c=s.games[i]!;
   const reply=await f.call(social?'/v3/social/rpc':'/v3/rpc',{major:1,revision:social?'g3-social-draft-1':'g3-draft-1',serviceId:f.service.db.serviceId,serviceEpoch:epoch,sessionEpoch:c.sessionEpoch,requestId:randomUUID(),scope,...payload},c.accessToken);
   assert.equal(reply.status,200);return JSON.parse(reply.body);
  };
  const ok=(r:any)=>{assert.equal(r.error,undefined,r.error?.code);return r;};
  const publish=async(i:number,sequence:number)=>{
   ok(await call(i,false,{movement:{frames:[{sequence,xMilli:String(1000+i*1000),yMilli:'2000',vxMilli:'0',vyMilli:'0',facingLeft:false}]}}));
   ok(await call(i,true,{publish:{eventId:transientEventId(s.scopes[i]!,s.games[i]!.sessionId!,'hud',sequence),sequence,hud:{hpMilli:1000,maxHpMilli:2000,sampo:false}}}));
  };
  const peers=async()=>{
   for(let i=0;i<2;i++){
    assert.equal(ok(await call(i,false,{ghostList:{}})).ghosts.length,1,'fresh peer movement must be nonempty');
    const projections=ok(await call(i,true,{snapshot:{}})).projections;
    assert.equal(projections.filter((p:any)=>p.scope.attachmentId===s.games[1-i]!.attachmentId).length,1,'fresh peer injected HUD must be nonempty');
    assert.equal(projections.length,2,'social snapshot includes own current HUD');
   }
  };
  await s.start();
  const frozen=(await status(0)).release!;assert.equal(frozen.phase,'authorized');assert.equal(frozen.observationRevision,1);assert.equal(frozen.enteredPlayingAt,undefined);
  assert.ok((await call(0,false,{ghostList:{}})).error);
  await s.playing();
  const entries:GameRelease[]=[];
  for(let i=0;i<2;i++)entries.push((await status(i)).release!);
  const samePermit=(r:GameRelease,i:number,expected:string)=>{
   assert.equal(r.phase,expected);assert.equal(r.authorizationId,frozen.authorizationId);assert.equal(r.startRequestId,frozen.startRequestId);
   for(const k of ['observationRevision','contentDigest','capturedAt','worldToken','attachmentId','gameSessionId','gameSessionEpoch','enteredPlayingAt'] as const)assert.equal(r[k],entries[i]![k],k);
   assert.equal(s.room.state,'running');assert.equal(s.room.members!.filter(m=>m.selected).length,2);
  };
  for(let sequence=1;sequence<=3;sequence++){for(let i=0;i<2;i++)await publish(i,sequence);await peers();}
  // Identical observation is idempotent, not a new phase/lease/revision.
  await phase(0,2,false);const revision=s.room.revision;await phase(0,2,false);assert.equal(s.room.revision,revision);
  assert.equal((await raw(s.games[0]!,{gameObservation:{...s.observations[0]!,observationRevision:2,protected:true}})).error.code,'CONFLICT');
  assert.equal((await raw(s.games[0]!,{gameObservation:{...s.observations[0]!,observationRevision:1,protected:true}})).error.code,'STALE_REVISION');
  for(let cycle=0;cycle<3;cycle++){
   await phase(0,3+cycle*2,true);samePermit((await status(0)).release!,0,'restoring');
   assert.ok((await call(0,false,{ghostList:{}})).error);assert.ok((await call(0,true,{snapshot:{}})).error);
   assert.equal(ok(await call(1,false,{ghostList:{}})).ghosts.length,0);
   assert.equal(ok(await call(1,true,{snapshot:{}})).projections.filter((p:any)=>p.scope.attachmentId===s.games[0]!.attachmentId).length,0);
   await s.pulse();
   const g=s.games[0]!,renewed=(await s.rpc(s.desktops[0]!,{gameRenew:{attachmentId:g.attachmentId!,gameSessionId:g.sessionId!,gameSessionEpoch:g.sessionEpoch!,gameInstanceId:s.observations[0]!.gameInstanceId!,worldToken:s.observations[0]!.worldToken!}})).credentials!;
   assert.equal(renewed.sessionId,g.sessionId);assert.equal(renewed.sessionEpoch,g.sessionEpoch);assert.equal(renewed.attachmentId,g.attachmentId);s.games[0]=renewed;
   await phase(0,4+cycle*2,false);samePermit((await status(0)).release!,0,'playing');
   for(let i=0;i<2;i++)await publish(i,4+cycle);await peers();
  }
  now+=2100;
  for(let i=0;i<2;i++){
   assert.equal(ok(await call(i,false,{ghostList:{}})).ghosts.length,0);
   assert.equal(ok(await call(i,true,{snapshot:{}})).projections.length,0);
   samePermit((await status(i)).release!,i,'playing');
  }
  for(let i=0;i<2;i++)await publish(i,7);await peers();
  // Keep ordinary short presence leases healthy across the original review cutoff.
  while(now<=Date.parse(frozen.releaseExpiresAt!)){now+=20000;await s.pulse();}
  for(let i=0;i<2;i++)samePermit((await status(i)).release!,i,'playing');
  await phase(0,9,true);samePermit((await status(0)).release!,0,'restoring');
  await phase(0,10,false);samePermit((await status(0)).release!,0,'playing');
  for(let i=0;i<2;i++)await publish(i,8);await peers();
  assert.equal((await raw(s.games[0]!,{gameObservation:{...s.observations[0]!,worldToken:'foreign-world',observationRevision:11,protected:false}})).error.code,'RESTART_REQUIRED');
  assert.equal((await call(0,false,{ghostList:{}},s.scopes[0]!,randomUUID())).error.code,'STALE_EPOCH');
  assert.equal((await call(0,false,{ghostList:{}},{...s.scopes[0]!,worldToken:'foreign-world'})).error.code,'STALE_EPOCH');
  samePermit((await status(0)).release!,0,'playing');
  await s.rpc(s.games[0]!,{gameObservation:{...s.observations[0]!,observationRevision:11,protected:false,actualSeed:'424243'}});
  const revoked=await status(0);assert.equal(revoked.room!.state,'suspended');assert.equal(revoked.gameStatus!.phase,'revoked');assert.equal(revoked.release,undefined);
  assert.ok((await call(1,false,{ghostList:{}})).error);assert.ok((await call(1,true,{snapshot:{}})).error);
  await s.rpc(s.host,{endRun:{}});assert.equal(s.room.state,'ended');
  assert.ok((await call(0,false,{ghostList:{}})).error);assert.ok((await call(0,true,{snapshot:{}})).error);
 }finally{await f.close();}
});
