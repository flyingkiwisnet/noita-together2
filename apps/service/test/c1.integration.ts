import {schema as rejoinSchema} from '../../../packages/contracts/src/g3-rejoin.js';
import test from 'node:test';import assert from 'node:assert/strict';import {randomUUID} from 'node:crypto';
import {b1Fixture} from './b1-fixture-support.js';import {C1_DEFAULTS} from '../../../packages/domain/src/g3-c1.js';import {schema,encode,decode} from '../../../packages/contracts/src/g3-c1.js';
import {livePeer} from '../scripts/g3-live-fixture.js';
import {startLifecycleLc1Fixture} from '../scripts/lifecycle-lc1-fixture.js';
import {schema as socialSchema} from '../../../packages/contracts/src/g3-social.js';import {transientEventId} from '../../../packages/contracts/src/g3-social-replay.js';
async function fixture(){const f=await b1Fixture({itemTransfers:true}),db=f.x.fixture.service.db;
 const call=async(i:number,payload:any)=>{f.x.advance(250);const c=f.values[i].credentials,request={major:1,revision:schema.revision,serviceId:db.serviceId,serviceEpoch:db.epoch,sessionEpoch:c.sessionEpoch,requestId:randomUUID(),roomId:f.scope(i).roomId,scope:f.scope(i),ownerGeneration:f.values[i].canonicalAdmission.generation,...payload};const response=JSON.parse((await f.x.fixture.call('/v3/c1/rpc',request,c.accessToken)).body);if(!response.error)decode('Response',encode('Response',response));return response;};
 const ok=async(i:number,payload:any)=>{const r=await call(i,payload);assert.equal(r.error,undefined,JSON.stringify(r.error));return r;};
 const host=async(payload:any,credentials=f.x.host)=>{f.x.advance(250);return JSON.parse((await f.x.fixture.call('/v3/c1/rpc',{major:1,revision:schema.revision,serviceId:db.serviceId,serviceEpoch:db.epoch,sessionEpoch:credentials.sessionEpoch,requestId:randomUUID(),roomId:f.scope(0).roomId,...payload},credentials.accessToken)).body);};
 return {f,db,call,ok,host,publish:(payload:any,rulesRevision=1)=>({publish:{eventId:randomUUID(),rulesRevision,payload,senderDisposition:'applied'}})};
}
test('C1 real TLS/auth/SQL acceptance freezes sender-excluded recipients, orders receipts, rejects contradictory originals and isolates worlds',async()=>{
 const x=await fixture();try{const initial=await x.ok(0,{rulesGet:{}});assert.equal(initial.memberCount,2);assert.deepEqual(initial.snapshot.rules,C1_DEFAULTS);
  const original=x.publish({heart:{}}),first=await x.ok(0,original);assert.equal(first.event.rulesRevision,1);assert.deepEqual(first.event.capturedRules,C1_DEFAULTS);assert.equal((await x.ok(0,original)).duplicate,true);assert.equal((await x.call(0,{publish:{...original.publish,payload:{orb:{orbId:0}}}})).error.code,'CONFLICT');assert.deepEqual((await x.ok(0,{deliveries:{}})).deliveries??[],[]);
  const second=await x.ok(0,x.publish({perk:{perkId:'EXTRA_MONEY'}}));const page=await x.ok(1,{deliveries:{}});assert.equal(page.deliveries.length,1);assert.equal(page.deliveries[0].eventId,first.event.eventId);const outOfOrder=await x.call(1,{receipt:{eventId:second.event.eventId,sequence:second.event.sequence,state:'applied'}});assert.equal(outOfOrder.error.code,'CONFLICT');
  const ack={receipt:{eventId:first.event.eventId,sequence:first.event.sequence,state:'applied'}};await x.ok(1,ack);assert.equal((await x.ok(1,ack)).duplicate,true);assert.equal((await x.ok(1,{deliveries:{}})).deliveries[0].eventId,second.event.eventId);
  await x.ok(1,{receipt:{eventId:second.event.eventId,sequence:second.event.sequence,state:'uncertain'}});assert.equal((await x.ok(1,{deliveries:{}})).deliveries[0].state,'uncertain');await x.ok(0,x.publish({heart:{}}));assert.equal((await x.ok(1,{deliveries:{}})).deliveries[0].eventId,second.event.eventId);
  assert.equal((await x.db.transaction(tx=>tx.query('SELECT count(*)::int AS n FROM c1_events'))).rows[0].n,3);assert.equal((await x.call(1,{rulesSet:{expectedRevision:1,rules:C1_DEFAULTS}})).error.code,'FORBIDDEN');
 }finally{await x.f.close();}
});
test('C1 live host rules order with events without seed/inspection changes; disabled unissued work skips, issued work never silently reverses',async()=>{
 const x=await fixture();try{const before=(await x.db.transaction(tx=>tx.query('SELECT config,config_revision,current_run_id FROM rooms WHERE id=$1',[x.f.scope(0).roomId]))).rows[0];
  const first=await x.ok(0,x.publish({heart:{}}));const issued=await x.ok(1,{deliveries:{}});assert.equal(issued.deliveries[0].eventId,first.event.eventId);const unissued=await x.ok(0,x.publish({heart:{}}));
  const update={rulesSet:{expectedRevision:1,rules:{...C1_DEFAULTS,NT_sync_hearts:false,NT_sync_orbs:false,NT_sync_orb_count:10,NT_ban_perks:true}}};const changed=await x.host(update);assert.equal(changed.error,undefined);assert.equal(changed.snapshot.revision,2);assert.equal(changed.snapshot.rules.NT_sync_orb_count,0);assert.ok(BigInt(changed.snapshot.sequence)>BigInt(unissued.event.sequence));assert.equal((await x.host(update)).duplicate,true);
  assert.equal((await x.ok(1,{deliveries:{}})).deliveries[0].eventId,first.event.eventId);await x.ok(1,{receipt:{eventId:first.event.eventId,sequence:first.event.sequence,state:'applied'}});assert.deepEqual((await x.ok(1,{deliveries:{}})).deliveries??[],[]);
  const row=(await x.db.transaction(tx=>tx.query('SELECT state,reason FROM c1_deliveries WHERE event_id=$1',[unissued.event.eventId]))).rows[0];assert.deepEqual(row,{state:'skipped',reason:'sharing_disabled'});await x.host({rulesSet:{expectedRevision:2,rules:C1_DEFAULTS}});assert.deepEqual((await x.ok(1,{deliveries:{}})).deliveries??[],[]);
  assert.equal((await x.call(0,x.publish({perk:{perkId:'../../evil.lua'}}))).error.code,'UNSUPPORTED_CONTENT');assert.equal((await x.call(0,{ownerGeneration:999,publish:x.publish({heart:{}}).publish})).error.code,'STALE_EPOCH');
  const after=(await x.db.transaction(tx=>tx.query('SELECT config,config_revision,current_run_id FROM rooms WHERE id=$1',[x.f.scope(0).roomId]))).rows[0];assert.deepEqual(after,before);
 }finally{await x.f.close();}
});
test('C1 weak respawn keeps selected owner, restores playing under same death identity; later death is new, terminal completion fences callbacks and preserves other players',async()=>{
 const x=await fixture();try{const changed=await x.host({rulesSet:{expectedRevision:1,rules:{...C1_DEFAULTS,NT_death_penalty:'weak_respawn'}}});assert.equal(changed.error,undefined);
  const incoming=await x.ok(1,x.publish({heart:{}},2));const death=await x.ok(0,x.publish({death:{state:'respawning',cause:'polymorph'}},2));assert.deepEqual((await x.ok(0,{deliveries:{}})).deliveries??[],[]);assert.equal(death.event.capturedRules.NT_death_penalty,'weak_respawn');assert.equal(death.lives.find((l:any)=>l.worldToken===x.f.scope(0).worldToken).state,'respawning');const peer=await x.ok(1,{deliveries:{}});assert.equal(peer.deliveries[0].payload.death.state,'respawning');
  const social=async(i:number,payload:any)=>{const c=x.f.values[i].credentials;x.f.x.advance(250);return JSON.parse((await x.f.x.fixture.call('/v3/social/rpc',{major:1,revision:socialSchema.revision,serviceId:x.db.serviceId,serviceEpoch:x.db.epoch,sessionEpoch:c.sessionEpoch,requestId:randomUUID(),scope:x.f.scope(i),...payload},c.accessToken)).body);};
  const roster=await social(1,{roster:{}});assert.equal(roster.error,undefined);assert.equal(roster.players.find((p:any)=>p.worldToken===x.f.scope(0).worldToken).state,'respawning');
  const hud=await social(0,{publish:{eventId:transientEventId(x.f.scope(0),x.f.values[0].credentials.sessionId,'hud',1),sequence:1,hud:{hpKnown:true,hpMilli:0,maxHpMilli:100000,sampoKnown:false}}});assert.equal(hud.error,undefined);assert.equal((await x.db.transaction(tx=>tx.query('SELECT dead FROM g3_social_life WHERE run_id=$1 AND game_session_id=$2',[x.f.scope(0).runId,x.f.values[0].credentials.sessionId]))).rows[0].dead,false);
  await x.host({rulesSet:{expectedRevision:2,rules:{...C1_DEFAULTS,NT_death_penalty:'full_respawn'}}});assert.equal((await x.ok(1,{deliveries:{}})).deliveries[0].capturedRules.NT_death_penalty,'weak_respawn');const done={life:{deathEventId:death.event.eventId,state:'playing'}};await x.ok(0,done);assert.equal((await x.ok(0,done)).duplicate,true);assert.equal((await x.ok(0,{deliveries:{}})).deliveries[0].eventId,incoming.event.eventId);
  const next=await x.ok(0,x.publish({death:{state:'respawning',cause:'normal'}},3));assert.notEqual(next.event.eventId,death.event.eventId);assert.equal((await x.call(0,done)).error.code,'CONFLICT');await x.ok(0,{life:{deathEventId:next.event.eventId,state:'terminal'}});assert.ok((await x.call(0,{life:{deathEventId:next.event.eventId,state:'playing'}})).error);assert.equal((await x.ok(1,{rulesGet:{}})).memberCount,2);assert.equal((await x.db.transaction(tx=>tx.query('SELECT state FROM rooms WHERE id=$1',[x.f.scope(0).roomId]))).rows[0].state,'running');
 }finally{await x.f.close();}
});
test('C1 retained lobby projection counts a real unselected joining principal and advances membership high-water separately from rule revision',async()=>{
 const x=await startLifecycleLc1Fixture({itemTransfers:true});try{const host=async()=>JSON.parse((await x.fixture.call('/v3/c1/rpc',{major:1,revision:schema.revision,serviceId:x.fixture.service.db.serviceId,serviceEpoch:x.fixture.service.db.epoch,sessionEpoch:x.host.sessionEpoch,requestId:randomUUID(),roomId:x.room.roomId,rulesGet:{}},x.host.accessToken)).body);const before=await host();assert.equal(before.error,undefined);assert.equal(before.memberCount,1);await x.bootstrapPeer();const after=await host();assert.equal(after.memberCount,2);assert.equal(after.snapshot.revision,before.snapshot.revision);assert.ok(BigInt(after.sequence)>BigInt(before.sequence));
 }finally{await x.close();}
});
test('C1 authorized protected death/finish observations do not clear gameplay protection or permit ordinary pickup effects',async()=>{
 const x=await fixture();try{await x.host({rulesSet:{expectedRevision:1,rules:{...C1_DEFAULTS,NT_death_penalty:'full_respawn'}}});
  // Explicit synthetic engine observation in the disposable fixture; not Noita save evidence.
  await x.db.transaction(tx=>tx.query("UPDATE world_observations SET body=jsonb_set(body,'{protected}','true'::jsonb) WHERE attachment_id=$1",[x.f.scope(0).attachmentId]));assert.equal((await x.call(0,x.publish({heart:{}},2))).error.code,'FORBIDDEN');
  const death=await x.ok(0,x.publish({death:{state:'respawning',cause:'normal'}},2));await x.ok(0,{life:{deathEventId:death.event.eventId,state:'playing'}});assert.equal((await x.call(0,{deliveries:{}})).error.code,'FORBIDDEN');assert.ok((await x.f.send(0,{goldBank:{}})).error);
  assert.equal((await x.db.transaction(tx=>tx.query("SELECT body->>'protected' AS protected FROM world_observations WHERE attachment_id=$1",[x.f.scope(0).attachmentId]))).rows[0].protected,'true');
 }finally{await x.f.close();}
});
for(const mode of ['weak_respawn','full_respawn'] as const)test('C1 Kick on death includes host, retains controls and fences same-run reentry: '+mode,async()=>{
 const x=await fixture();try{
  await x.host({rulesSet:{expectedRevision:1,rules:{...C1_DEFAULTS,NT_death_penalty:mode,NT_ondeath_kick:true}}});
  // Fixture-only canonical ownership assignment makes the admitted player host.
  const roomId=x.f.scope(0).roomId,accountId=x.f.values[0].credentials.accountId!;
  await x.db.transaction(tx=>tx.query('UPDATE rooms SET host_account_id=$2 WHERE id=$1',[roomId,accountId]));
  const browser=await x.db.transaction(tx=>x.f.x.fixture.service.auth.family(tx,accountId,randomUUID(),'browser'));
  const original=x.publish({death:{state:'respawning',cause:'normal'}},2),death=await x.ok(0,original);
  const stored=(await x.db.pool.query('SELECT host_account_id,state FROM rooms WHERE id=$1',[roomId])).rows[0];assert.equal(stored.host_account_id,accountId);assert.equal(stored.state,'running');
  assert.equal((await x.db.pool.query('SELECT count(*)::int AS n FROM memberships WHERE room_id=$1 AND account_id=$2',[roomId,accountId])).rows[0].n,0);
  const management=await x.host({rulesGet:{}},x.f.x.desktop);assert.equal(management.error,undefined);assert.equal(management.memberCount,1);assert.equal((await x.host({rulesGet:{}},browser)).error,undefined);
  assert.equal((await x.host({rulesSet:{expectedRevision:2,rules:{...C1_DEFAULTS,NT_death_penalty:mode,NT_ondeath_kick:false}}},browser)).error,undefined);
  assert.ok((await x.call(0,{life:{deathEventId:death.event.eventId,state:'playing'}})).error);assert.ok((await x.f.send(0,{goldBank:{}})).error);
  const peer=(await x.ok(1,{deliveries:{}})).deliveries[0];assert.equal(peer.eventId,death.event.eventId);assert.equal(peer.capturedRules.NT_death_penalty,mode);
  const ack={receipt:{eventId:peer.eventId,sequence:peer.sequence,state:'applied'}};await x.ok(1,ack);assert.equal((await x.ok(1,ack)).duplicate,true);assert.deepEqual((await x.ok(1,{deliveries:{}})).deliveries??[],[]);
  assert.equal((await x.db.pool.query('SELECT count(*)::int AS n FROM c1_deliveries WHERE event_id=$1',[peer.eventId])).rows[0].n,1);
  const owner=await x.f.x.rpc(x.f.x.desktop,{roomGet:{}});assert.equal(owner.room.hostAccountId,accountId);assert.equal(owner.room.state,'running');
  await assert.rejects(x.f.x.rpc(x.f.x.desktop,{roomJoin:{}}));
  const continued=await x.f.x.send(x.f.x.desktop,{...x.f.qa,requestId:randomUUID(),reconcile:{...x.f.qa.reconcile,attempt:2,observation:{...x.f.x.observation,protected:true,observationRevision:3}}});assert.equal(continued.world.state,'terminal');
  const otherDesktop=await x.db.transaction(tx=>x.f.x.fixture.service.auth.family(tx,accountId,randomUUID(),'desktop'));assert.equal((await x.f.x.rpc(otherDesktop,{roomGet:{}})).room.hostAccountId,accountId);await assert.rejects(x.f.x.rpc(otherDesktop,{roomJoin:{}}));
  const p=(await x.db.pool.query('SELECT id FROM rejoin_participants WHERE run_id=$1 AND account_id=$2 LIMIT 1',[x.f.scope(0).runId,accountId])).rows[0];const resumed=JSON.parse((await x.f.x.fixture.call('/v3/rejoin/rpc',{major:1,revision:rejoinSchema.revision,serviceId:x.db.serviceId,serviceEpoch:x.db.epoch,sessionEpoch:otherDesktop.sessionEpoch!,requestId:randomUUID(),roomId,runId:x.f.scope(0).runId,participantId:p.id,worldToken:x.f.scope(0).worldToken,configRevision:x.f.scope(0).configRevision,resume:{resumeProof:'a'.repeat(43),gameInstanceId:'other-device'}},otherDesktop.accessToken)).body);assert.equal(resumed.error?.code,'LIFECYCLE_REQUIRED');
  const foreign=await x.f.x.send(otherDesktop,{...x.f.qa,sessionEpoch:otherDesktop.sessionEpoch!,requestId:randomUUID(),reconcile:{...x.f.qa.reconcile,attempt:3}});assert.ok(foreign.error||foreign.world?.state==='terminal');
  await x.ok(1,x.publish({heart:{}},3));assert.ok((await x.f.ok(1,{goldBank:{}})).bank);
  await x.f.x.rpc(x.f.x.desktop,{roomGet:{}});await x.f.x.rpc(x.f.x.desktop,{endRun:{}});const next=await x.f.x.rpc(x.f.x.desktop,{roomJoin:{}});assert.notEqual(next.room.runId,x.f.scope(0).runId);assert.ok(next.preparation);
 }finally{await x.f.close();}
});
test('C1 host death with Kick on death disabled retains participation and normal respawn completion',async()=>{
 const x=await fixture();try{await x.host({rulesSet:{expectedRevision:1,rules:{...C1_DEFAULTS,NT_death_penalty:'full_respawn',NT_ondeath_kick:false}}});await x.db.pool.query('UPDATE rooms SET host_account_id=$2 WHERE id=$1',[x.f.scope(0).roomId,x.f.values[0].credentials.accountId]);const death=await x.ok(0,x.publish({death:{state:'respawning',cause:'normal'}},2));await x.ok(0,{life:{deathEventId:death.event.eventId,state:'playing'}});assert.equal((await x.ok(0,{rulesGet:{}})).memberCount,2);assert.ok((await x.f.ok(0,{goldBank:{}})).bank);
 }finally{await x.f.close();}
});

test('C1 retained membership and frozen deliveries survive a short lease gap; new-world and foreign scope cannot inherit work',async()=>{
 const x=await fixture();try{const saved=(await x.db.transaction(tx=>tx.query('SELECT presence_until FROM attachments WHERE id=$1',[x.f.scope(1).attachmentId]))).rows[0];
  await x.db.transaction(tx=>tx.query('UPDATE attachments SET presence_until=$2 WHERE id=$1',[x.f.scope(1).attachmentId,new Date(0)]));const event=await x.ok(0,x.publish({heart:{}}));assert.equal(event.memberCount,2);assert.equal((await x.db.transaction(tx=>tx.query('SELECT count(*)::int AS n FROM c1_deliveries WHERE event_id=$1',[event.event.eventId]))).rows[0].n,1);
  assert.equal((await x.call(1,{deliveries:{}})).error.code,'FORBIDDEN');await x.db.transaction(tx=>tx.query('UPDATE attachments SET presence_until=$2 WHERE id=$1',[x.f.scope(1).attachmentId,saved.presence_until]));const returned=await x.ok(1,{deliveries:{}});assert.equal(returned.deliveries[0].eventId,event.event.eventId);
  assert.equal((await x.call(1,{scope:{...x.f.scope(1),worldToken:randomUUID()},deliveries:{}})).error.code,'STALE_EPOCH');assert.equal((await x.call(1,{roomId:randomUUID(),deliveries:{}})).error.code,'NOT_FOUND');
 }finally{await x.f.close();}
});
test('C1 real WSS initial/committed wake, bidirectional poses and bank operations share the accepted connection',async()=>{
 const x=await fixture(),peers:any[]=[];try{const composition:any={games:x.f.values.map((v:any)=>v.credentials),scopes:[x.f.scope(0),x.f.scope(1)]};const a=await livePeer(x.f.x.fixture as any,composition,0),b=await livePeer(x.f.x.fixture as any,composition,1);peers.push(a,b);await a.wait(v=>v.c1Changed?.sequence==='0');
  const event=await x.ok(0,x.publish({heart:{}}));await b.wait(v=>v.c1Changed?.sequence===event.event.sequence);a.send({pose:{sequence:1,xMilli:'25',yMilli:'0',vxMilli:'1',vyMilli:'0'}});b.send({pose:{sequence:1,xMilli:'-25',yMilli:'0',vxMilli:'-1',vyMilli:'0'}});await a.wait(v=>!!v.snapshot?.peers?.some(p=>p.poses?.some(p=>p.sequence===1)));await b.wait(v=>!!v.snapshot?.peers?.some(p=>p.poses?.some(p=>p.sequence===1)));
  assert.ok((await x.f.ok(0,{goldBank:{}})).bank);const current=await x.host({rulesSet:{expectedRevision:1,rules:{...C1_DEFAULTS,NT_alt_heartache:true}}});await a.wait(v=>v.c1Changed?.sequence===current.snapshot.sequence);assert.equal((await x.ok(1,{deliveries:{}})).deliveries[0].eventId,event.event.eventId);
 }finally{for(const peer of peers)await peer.close();await x.f.close();}
});
test('C1 winning reports preserve lobby membership and never invoke kick; end-mode ordinary death sends no weak penalty meaning',async()=>{
 const x=await fixture();try{await x.host({rulesSet:{expectedRevision:1,rules:{...C1_DEFAULTS,NT_ondeath_kick:true}}});const won=await x.ok(1,x.publish({death:{state:'won',cause:'normal'}},2));assert.equal(won.event.payload.death.state,'won');assert.equal((await x.ok(0,{rulesGet:{}})).memberCount,2);const delivery=(await x.ok(0,{deliveries:{}})).deliveries[0];assert.equal(delivery.capturedRules.NT_death_penalty,'end');assert.equal(delivery.payload.death.state,'won');assert.equal((await x.db.transaction(tx=>tx.query('SELECT state FROM rooms WHERE id=$1',[x.f.scope(0).roomId]))).rows[0].state,'running');
 }finally{await x.f.close();}
});
test('C1 two actual lobbies on one service keep rules, worlds and frozen recipients separate',async()=>{
 const x=await fixture();try{const first=await x.ok(0,x.publish({heart:{}})),second=(await x.f.x.rpc(x.f.x.host,{roomCreate:{name:'Separate C1 lobby',visibility:'public',allowGuests:false,config:structuredClone(x.f.x.room.config)}})).room;assert.notEqual(second.roomId,x.f.scope(0).roomId);
  const changed=await x.host({roomId:second.roomId,rulesSet:{expectedRevision:1,rules:{...C1_DEFAULTS,NT_sync_hearts:false}}});assert.equal(changed.error,undefined);assert.equal(changed.memberCount,0);assert.equal((await x.ok(0,{rulesGet:{}})).snapshot.rules.NT_sync_hearts,true);
  assert.equal((await x.call(0,{roomId:second.roomId,deliveries:{}})).error.code,'FORBIDDEN');assert.equal((await x.ok(1,{deliveries:{}})).deliveries[0].eventId,first.event.eventId);assert.equal((await x.db.transaction(tx=>tx.query('SELECT count(*)::int AS n FROM c1_events WHERE room_id=$1',[second.roomId]))).rows[0].n,0);
 }finally{await x.f.close();}
});

test('C1 host win with Kick on death enabled retains lobby membership and controls',async()=>{const x=await fixture();try{await x.host({rulesSet:{expectedRevision:1,rules:{...C1_DEFAULTS,NT_ondeath_kick:true}}});await x.db.pool.query('UPDATE rooms SET host_account_id=$2 WHERE id=$1',[x.f.scope(0).roomId,x.f.values[0].credentials.accountId]);const won=await x.ok(0,x.publish({death:{state:'won',cause:'normal'}},2));assert.equal(won.event.payload.death.state,'won');assert.equal((await x.host({rulesGet:{}},x.f.x.desktop)).memberCount,2);assert.equal((await x.db.pool.query('SELECT count(*)::int AS n FROM memberships WHERE room_id=$1 AND account_id=$2',[x.f.scope(0).roomId,x.f.values[0].credentials.accountId])).rows[0].n,1);assert.equal((await x.ok(1,{deliveries:{}})).deliveries[0].payload.death.state,'won');assert.ok((await x.f.ok(1,{goldBank:{}})).bank);}finally{await x.f.close();}});
