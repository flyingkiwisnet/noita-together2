import test from 'node:test';import assert from 'node:assert/strict';import {randomUUID} from 'node:crypto';import {b1Fixture} from './b1-fixture-support.js';import {C1_DEFAULTS} from '../../../packages/domain/src/g3-c1.js';import {schema} from '../../../packages/contracts/src/g3-c1.js';
for(const scenario of ['end-paused','end-disconnected','weak-terminal','weak-life-terminal','full-terminal-captured'] as const)test('Shared death/end persists actionable lifecycle outcome and isolates a fresh run: '+scenario,async()=>{
 const f=await b1Fixture({itemTransfers:true}),db=f.x.fixture.service.db;
 const c1=async(c:any,body:any)=>{f.x.advance(50);return JSON.parse((await f.x.fixture.call('/v3/c1/rpc',{major:1,revision:schema.revision,serviceId:db.serviceId,serviceEpoch:db.epoch,sessionEpoch:c.sessionEpoch,requestId:randomUUID(),...body},c.accessToken)).body);};
 const host=(body:any)=>c1(f.x.host,{roomId:f.scope(0).roomId,...body});
 const game=(i:number,body:any)=>c1(f.values[i].credentials,{roomId:f.scope(i).roomId,scope:f.scope(i),ownerGeneration:f.values[i].canonicalAdmission.generation,...body});
 const ok=(v:any)=>{assert.equal(v.error,undefined,JSON.stringify(v.error));return v;};
 try{
  const runId=f.scope(0).runId,roomId=f.scope(0).roomId;ok(await host({rulesGet:{}}));let captured=1;
  if(scenario.startsWith('weak')||scenario==='full-terminal-captured'){ok(await host({rulesSet:{expectedRevision:1,rules:{...C1_DEFAULTS,NT_death_penalty:scenario==='full-terminal-captured'?'full_respawn':'weak_respawn',NT_ondeath_kick:false}}}));captured=2;}
  // Change the current rule after capture. Original rules must decide End.
  ok(await host({rulesSet:{expectedRevision:captured,rules:{...C1_DEFAULTS,NT_death_penalty:scenario==='full-terminal-captured'?'end':'full_respawn',NT_ondeath_kick:false}}}));
  if(scenario==='end-paused')await db.pool.query("UPDATE world_observations SET body=jsonb_set(body,'{protected}','true'::jsonb) WHERE attachment_id=$1",[f.scope(1).attachmentId]);
  if(scenario==='end-disconnected')await db.pool.query('UPDATE attachments SET presence_until=$2 WHERE id=$1',[f.scope(1).attachmentId,new Date(0)]);
  const original={eventId:randomUUID(),rulesRevision:captured,payload:{death:{state:scenario==='weak-life-terminal'?'respawning':'terminal',cause:scenario.startsWith('weak')?'low_life':'normal'}},senderDisposition:'applied'};
  if(scenario==='weak-life-terminal')original.payload.death.cause='normal';
  const accepted=ok(await game(0,{publish:original}));assert.equal(accepted.event.capturedRules.NT_ondeath_kick,false);
  if(scenario==='full-terminal-captured'){assert.equal((await db.pool.query('SELECT state FROM runs WHERE id=$1',[runId])).rows[0].state,'running');assert.ok((await f.ok(1,{goldBank:{}})).bank);return;}
  if(scenario==='weak-life-terminal'){assert.equal((await db.pool.query('SELECT state FROM runs WHERE id=$1',[runId])).rows[0].state,'running');ok(await game(0,{life:{deathEventId:original.eventId,state:'terminal'}}));}
  const states=(await db.pool.query('SELECT n.state,n.reason,r.state AS room_state,r.host_account_id,k.terminal_reason FROM runs n JOIN rooms r ON r.id=n.room_id JOIN rejoin_runs k ON k.run_id=n.id WHERE n.id=$1',[runId])).rows[0];assert.equal(states.state,'ended');assert.equal(states.room_state,'ended');assert.equal(states.reason,'player_death_end');assert.equal(states.terminal_reason,'player_death_end');assert.equal(states.host_account_id,f.x.host.accountId);
  assert.equal((await db.pool.query('SELECT count(*)::int AS n FROM rejoin_participants WHERE run_id=$1 AND NOT terminal',[runId])).rows[0].n,0);
  assert.equal((await db.pool.query('SELECT count(*)::int AS n FROM world_associations WHERE run_id=$1 AND terminal_reason=$2',[runId,'player_death_end'])).rows[0].n,2);
  assert.equal((await db.pool.query("SELECT state FROM g3_runs WHERE id=$1",[runId])).rows[0].state,'ended');
  // Paused/disconnected owners obtain the reason using current DESKTOP auth;
  // game credentials are deliberately revoked, not needed for terminal read.
  for(const [desktop,q]of [[f.x.desktop,f.qa],[f.peer.desktop,f.qb]] as const){const result=await f.x.send(desktop,{...q,requestId:randomUUID(),reconcile:{...q.reconcile,attempt:2}});assert.equal(result.world.state,'terminal');assert.equal(result.world.reason,'player_death_end');assert.equal(result.world.mayPlay,false);}
  const source={roomId,runId,worldToken:f.scope(0).worldToken,configRevision:f.scope(0).configRevision,publish:original};
  // Discarded publish reply is recovered by exact original, never republished.
  for(let i=0;i<2;i++){const outcome=ok(await c1(f.x.desktop,{sourceOutcome:{original:source}}));assert.equal(outcome.sourceOutcome.outcome,'committed');assert.equal(outcome.sourceOutcome.acceptedSequence,accepted.event.sequence);}
  assert.equal((await db.pool.query('SELECT count(*)::int AS n FROM c1_events WHERE run_id=$1 AND event_id=$2',[runId,original.eventId])).rows[0].n,1);assert.equal((await db.pool.query('SELECT count(*)::int AS n FROM c1_deliveries WHERE run_id=$1 AND event_id=$2',[runId,original.eventId])).rows[0].n,1);
  assert.ok((await game(0,{publish:original})).error);assert.ok((await f.send(1,{goldBank:{}})).error);
  ok(await host({rulesSet:{expectedRevision:captured+1,rules:{...C1_DEFAULTS,NT_death_penalty:'full_respawn'}}}));
  // Deliberate Join/prepare/Start makes a distinct world and run. No auto-run.
  await f.x.rpc(f.x.host,{roomGet:{}});const joined=await f.x.rpc(f.x.desktop,{roomJoin:{}});assert.notEqual(joined.room.runId,runId);assert.equal(joined.room.state,'staging');
  const fresh=await f.x.prepareWorld(),q=f.x.request(f.x.desktop,1,'fresh-after-end',{...f.x.observation,worldToken:fresh.association.worldId!,gameInstanceId:'fresh-after-shared-end',observationRevision:1,protected:true});q.reconcile!.association=fresh.association;q.reconcile!.proof=fresh.proof;ok(await f.x.send(f.x.desktop,q));await f.x.start();
  q.reconcile!.observation={...q.reconcile!.observation!,protected:false,observationRevision:2};const playing=ok(await f.x.send(f.x.desktop,q));assert.equal(playing.canonicalAdmission.decision.playing,true);const freshRun=playing.canonicalAdmission.scope.runId;
  // Late old terminal or old publish cannot end or authorize the fresh run.
  const late=await f.x.send(f.peer.desktop,{...f.qb,requestId:randomUUID(),reconcile:{...f.qb.reconcile,attempt:3}});assert.equal(late.world.reason,'player_death_end');assert.equal(late.world.association.runId,runId);assert.ok((await game(0,{publish:original})).error);assert.equal((await db.pool.query('SELECT state FROM runs WHERE id=$1',[freshRun])).rows[0].state,'running');
  console.log(JSON.stringify({sharedDeathEnd:scenario,capturedRule:true,terminalDesktopReason:true,oldRunEnded:true,ownerPreserved:true,lostOriginalReadOnly:true,freshExplicitStart:true,lateOldRunIsolated:true,actualNoita:false}));
 }finally{await f.close();}
});
