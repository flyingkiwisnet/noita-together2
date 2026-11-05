import test from 'node:test';import assert from 'node:assert/strict';import {randomUUID} from 'node:crypto';
import {b1Fixture} from './b1-fixture-support.js';import {startLifecycleLc1Fixture} from '../scripts/lifecycle-lc1-fixture.js';
import {schema as c1} from '../../../packages/contracts/src/g3-c1.js';import {schema as c2} from '../../../packages/contracts/src/g3-c2.js';
import {maintainStatistics} from '../src/g3/stats-store.js';
import {readFileSync} from 'node:fs';import {ITEM_CATALOGUE} from '../../../packages/domain/src/g3-items.js';
const metrics=['hearts','orbs','steveKills','deaths','wins','goldDeposited','goldWithdrawn','itemsDeposited','itemsClaimed'];
function transport(x:any){return async(payload:any,credentials?:any)=>{x.advance(150);return JSON.parse((await x.fixture.call('/v3/stats/rpc',{major:1,revision:'g3-stats-1',serviceId:x.fixture.service.db.serviceId,serviceEpoch:x.fixture.service.db.epoch,requestId:randomUUID(),...(credentials?{sessionEpoch:credentials.sessionEpoch}:{}),...payload},credentials?.accessToken)).body);};}
function typed(x:any){return async(path:string,schema:any,credentials:any,payload:any)=>JSON.parse((await x.fixture.call(path,{major:1,revision:schema.revision,serviceId:x.fixture.service.db.serviceId,serviceEpoch:x.fixture.service.db.epoch,sessionEpoch:credentials.sessionEpoch,requestId:randomUUID(),...payload},credentials.accessToken)).body);}

test('D1 actual service initializes zero rows, isolates public access and reconciles observations with committed accounting',async()=>{
 const f=await b1Fixture({worldRules:true,itemTransfers:true}),x=f.x,q=transport(x),rpc=typed(x);try{
  const discovered=await q({discover:{limit:32}});assert.equal(discovered.error,undefined);const locator=discovered.lobbies[0].locator;
  assert.equal(discovered.lobbies[0].publicStats,false);for(const id of [x.desktop.accountId,x.desktop.sessionId,x.room.roomId,x.room.runId])assert.equal(JSON.stringify(discovered).includes(id),false);
  const stranger=await x.fixture.service.db.transaction(async tx=>{const id=await x.fixture.service.auth.mockRegistered(tx,'website',randomUUID(),'Unrelated viewer');return x.fixture.service.auth.family(tx,id,randomUUID(),'desktop');});
  assert.equal((await q({read:{locator}})).error.code,'NOT_FOUND');
  assert.equal((await q({read:{locator}},stranger)).error.code,'NOT_FOUND');
  let summary=(await q({read:{locator}},x.host)).summary;assert.equal(summary.collectionState,'available');assert.equal(summary.rows.length,2);for(const row of summary.rows)for(const k of metrics)assert.equal(row[k],'0');
  await x.rpc(x.host,{roomGet:{}});const config=structuredClone(x.room.config),configRevision=x.room.configRevision;
  await x.rpc(x.host,{roomPolicy:{publicStats:true}});assert.equal(x.room.publicStats,true);assert.equal(x.room.configRevision,configRevision);assert.deepEqual(x.room.config,config);
  assert.ok((await q({read:{locator}},stranger)).summary);const viewers=await Promise.all(Array.from({length:4},()=>q({read:{locator}})));assert.ok(viewers.every(v=>v.summary?.rows.length===2));
  await assert.rejects(x.rpc(stranger,{roomPolicy:{publicStats:false}}),/FORBIDDEN/);
  assert.equal((await x.rpc(f.values[0].credentials,{roomGet:{}})).canonicalAdmission.decision.playing,true);
  const scope=f.scope(0),credentials=f.values[0].credentials;
  const observe=async(eventId:string,kind:string,extra:any={})=>{const r=await q({scope,ownerGeneration:f.values[0].world.generation,observe:{eventId,kind,progression:0,...extra}},credentials);return r;};
  const heart=randomUUID();assert.equal((await observe(heart,'heart')).accepted,true);assert.equal((await observe(heart,'heart')).duplicate,true);assert.equal((await observe(heart,'steve')).error.code,'CONFLICT');
  const rules=await rpc('/v3/c1/rpc',c1,x.host,{roomId:scope.roomId,rulesGet:{}});
  const publication=await rpc('/v3/c1/rpc',c1,credentials,{scope,ownerGeneration:f.values[0].world.generation,publish:{eventId:heart,rulesRevision:rules.snapshot.revision,payload:{heart:{}},senderDisposition:'applied'}});assert.equal(publication.error,undefined);
  const off=await rpc('/v3/c1/rpc',c1,x.host,{roomId:scope.roomId,rulesSet:{expectedRevision:rules.snapshot.revision,rules:{...rules.snapshot.rules,NT_sync_hearts:false,NT_sync_orbs:false}}});assert.equal(off.error,undefined);
  // Sharing-off observations use the independent path and create no deliveries.
  await observe(randomUUID(),'heart');await observe(randomUUID(),'orb',{orbId:'3'});assert.equal((await observe(randomUUID(),'orb',{orbId:'3'})).duplicate,true);
  await observe(randomUUID(),'orb',{orbId:'131'});await observe(randomUUID(),'steve');await observe(randomUUID(),'steve');
  const gold=(await f.ok(0,{goldIntent:{operationId:randomUUID(),direction:'deposit',amount:75}})).operation,receipt=f.receipt(gold,'escrowed');await f.ok(0,receipt);await f.ok(0,receipt);
  const withdrawal=(await f.ok(1,{goldIntent:{operationId:randomUUID(),direction:'withdraw',amount:25}})).operation;await f.ok(1,f.receipt(withdrawal,'applied'));
  const examples=JSON.parse(readFileSync(new URL('../../../docs/g3/server-item-transfer-examples.json',import.meta.url),'utf8'));
  const itemReceipt=(o:any,outcome:string)=>({itemReceipt:{operationId:o.operationId,deliveryId:o.deliveryId,payloadDigest:o.payloadDigest,receiptId:randomUUID(),outcome,evidenceDigest:'b'.repeat(64)}});
  const bank=async(i:number,p:any)=>{x.advance(250);return f.ok(i,p);};
  for(const item of [examples.descriptors.spell,examples.descriptors.wand,examples.descriptors.container,{catalogueRevision:ITEM_CATALOGUE.revision,object:{objectId:ITEM_CATALOGUE.objects.find(o=>o.available)!.id}}]){
   const deposit=(await bank(0,{itemDeposit:{operationId:randomUUID(),item}})).itemOperation,proof=itemReceipt(deposit,'escrowed');await bank(0,proof);await bank(0,proof);
   const detail=(await bank(1,{itemInspect:{itemId:deposit.itemId}})).itemDetail;
   const claim=(await bank(1,{itemClaim:{operationId:randomUUID(),itemId:deposit.itemId,expectedRevision:detail.summary.revision}})).itemOperation,applied=itemReceipt(claim,'applied');await bank(1,applied);await bank(1,applied);
  }
  summary=(await q({read:{locator}})).summary;const own=summary.rows.find((r:any)=>r.goldDeposited==='75'),peer=summary.rows.find((r:any)=>r.goldWithdrawn==='25');assert.equal(own.hearts,'2');assert.equal(own.orbs,'2');assert.equal(own.steveKills,'2');assert.equal(peer.hearts,'0');
  assert.equal(own.itemsDeposited,'4');assert.equal(peer.itemsClaimed,'4');
  for(const id of [x.desktop.accountId,x.desktop.sessionId,x.room.roomId,x.room.runId,heart])assert.equal(JSON.stringify(summary).includes(id),false);
  assert.equal((await q({scope,ownerGeneration:999,observe:{eventId:randomUUID(),kind:'heart'}},credentials)).error.code,'STALE_EPOCH');
  assert.equal((await q({scope:{...scope,worldToken:randomUUID()},ownerGeneration:f.values[0].world.generation,observe:{eventId:randomUUID(),kind:'heart'}},credentials)).error.code,'STALE_EPOCH');
  assert.equal((await q({observe:{eventId:randomUUID(),kind:'heart'}})).error.code,'UNAUTHENTICATED');
  const page=(await q({read:{locator,limit:1}})).summary;assert.equal(page.rows.length,1);assert.ok(page.nextRow);assert.equal((await q({read:{locator,afterRow:randomUUID(),runKey:page.runKey,summaryRevision:page.summaryRevision}})).error.code,'INVALID_MESSAGE');
  await observe(randomUUID(),'heart');assert.equal((await q({read:{locator,afterRow:page.nextRow,runKey:page.runKey,summaryRevision:page.summaryRevision}})).error.code,'STALE_REVISION');
  await x.rpc(x.host,{roomGet:{}});assert.equal((await x.rpc(x.host,{roomPolicy:{publicStats:false}})).room.publicStats,false);assert.equal((await q({read:{locator}})).error.code,'NOT_FOUND');assert.ok((await q({read:{locator}},x.desktop)).summary);
  console.log(JSON.stringify({d1AccessAndCounters:true,zeroRows:2,sharingOffIndependent:true,bankOriginalReceipts:true,publicPolicyDoesNotPause:true,actualNoita:false}));
 }finally{await f.close();}
});

test('D1 one-account entry rejects second desktop, equal names stay distinct and fresh Start gets complete zeroes',async()=>{
 const x=await startLifecycleLc1Fixture({itemTransfers:true}),q=transport(x);try{
  const other=await x.fixture.service.db.transaction(tx=>x.fixture.service.auth.family(tx,x.desktop.accountId!,randomUUID(),'desktop'));
  await assert.rejects(x.rpc(other,{roomJoin:{}}),/ACCOUNT_ALREADY_JOINED/);
  const peer=await x.bootstrapPeer(),peerRequest=x.request(peer.desktop,1,'equal-name-peer',peer.observation);peerRequest.reconcile!.association=peer.association;peerRequest.reconcile!.proof=peer.proof;
  await x.fixture.service.db.pool.query('UPDATE principal_profiles SET display_name=$1 WHERE principal_id=ANY($2::uuid[])',['Equal <name>',[x.desktop.accountId,peer.desktop.accountId]]);
  await x.send(x.desktop,x.request());await x.send(peer.desktop,peerRequest);await x.start();
  const ownRequest=x.request();ownRequest.reconcile!.observation={...x.observation,protected:false,observationRevision:2};peerRequest.reconcile!.observation={...peer.observation,protected:false,observationRevision:2};
  const values=[await x.send(x.desktop,ownRequest),await x.send(peer.desktop,peerRequest)];
  const locator=(await q({discover:{}})).lobbies[0].locator,eventId=randomUUID();for(const v of values){const s=v.canonicalAdmission.scope,scope=Object.fromEntries(['roomId','runId','runEpoch','configRevision','attachmentId','worldToken'].map(k=>[k,s[k]]));assert.equal((await q({scope,ownerGeneration:v.world.generation,observe:{eventId,kind:'heart'}},v.credentials)).accepted,true);}
  const before=(await q({read:{locator}},x.host)).summary;assert.equal(before.rows.length,2);assert.deepEqual(before.rows.map((r:any)=>r.hearts).sort(),['1','1']);assert.equal(new Set(before.rows.map((r:any)=>r.rowKey)).size,2);assert.ok(before.rows.every((r:any)=>r.displayName==='Equal <name>'));
  await x.end();await x.nextRun();await x.rpc(x.desktop,{roomJoin:{}});const fresh=await x.prepareWorld(),freshRequest=x.request(x.desktop,1,'fresh-zero-run',{...x.observation,worldToken:fresh.association.worldId!});freshRequest.reconcile!.association=fresh.association;freshRequest.reconcile!.proof=fresh.proof;assert.equal((await x.send(x.desktop,freshRequest)).world.state,'waiting');await x.start();const after=(await q({read:{locator}},x.host)).summary;assert.notEqual(after.runKey,before.runKey);assert.ok(after.rows.length);for(const row of after.rows)for(const k of metrics)assert.equal(row[k],'0');
  console.log(JSON.stringify({d1SingleAccountEntry:true,sameEventIdsWorldScoped:true,equalNamesDistinct:true,zeroEventPlayers:true,freshStartZeroes:true,actualNoita:false}));
 }finally{await x.close();}
});

test('D1 C1/C2 completion reports reconcile by world/progression; later genuine progression counts again',async()=>{
 const f=await b1Fixture({worldRules:true,itemTransfers:true}),x=f.x,q=transport(x),rpc=typed(x);try{
  const scope=f.scope(0),credentials=f.values[0].credentials,ownerGeneration=f.values[0].world.generation,locator=(await q({discover:{}})).lobbies[0].locator;
  const state=async(progression:number)=>rpc('/v3/c2/rpc',c2,credentials,{scope,ownerGeneration,state:{progression}});
  const initial=await state(0);assert.equal(initial.error,undefined);const rulesRevision=initial.snapshot.rules.revision;
  const complete=async(progression:number)=>rpc('/v3/c2/rpc',c2,credentials,{scope,ownerGeneration,publish:{eventId:randomUUID(),rulesRevision,progression,payload:{completed:{progression}},senderDisposition:'applied'}});
  const win=async(progression:number)=>q({scope,ownerGeneration,observe:{eventId:randomUUID(),kind:'win',progression}},credentials);
  assert.equal((await win(0)).accepted,true);assert.equal((await complete(0)).error,undefined);assert.equal((await win(0)).duplicate,true);
  await state(1);assert.equal((await win(1)).accepted,true);assert.equal((await complete(1)).error,undefined);
  const rules=await rpc('/v3/c1/rpc',c1,x.host,{roomId:scope.roomId,rulesGet:{}}),eventId=randomUUID();
  const won=await rpc('/v3/c1/rpc',c1,credentials,{scope,ownerGeneration,publish:{eventId,rulesRevision:rules.snapshot.revision,payload:{death:{state:'won',cause:'normal',progression:1}},senderDisposition:'applied'}});assert.equal(won.error,undefined);
  const rows=(await q({read:{locator}},x.host)).summary.rows;assert.equal(rows.reduce((a:number,r:any)=>a+Number(r.wins),0),2);assert.equal(rows.reduce((a:number,r:any)=>a+Number(r.deaths),0),0);
  console.log(JSON.stringify({d1C1C2WinsDedup:true,laterProgressionCounts:true,winNotDeath:true,actualNoita:false}));
 }finally{await f.close();}
});

test('D1 terminal death counts before shared End, exact original recovery and frozen elapsed survive late requests',async()=>{
 const f=await b1Fixture({worldRules:true,itemTransfers:true}),x=f.x,q=transport(x),rpc=typed(x);try{
  await x.rpc(x.host,{roomGet:{}});await x.rpc(x.host,{roomPolicy:{publicStats:true}});const locator=(await q({discover:{}})).lobbies[0].locator;
  const rules=await rpc('/v3/c1/rpc',c1,x.host,{roomId:x.room.roomId,rulesGet:{}});const changed=await rpc('/v3/c1/rpc',c1,x.host,{roomId:x.room.roomId,rulesSet:{expectedRevision:rules.snapshot.revision,rules:{...rules.snapshot.rules,NT_death_penalty:'end',NT_ondeath_kick:false}}});assert.equal(changed.error,undefined);
  const eventId=randomUUID(),scope=f.scope(0),generation=f.values[0].world.generation;
  const pickupId=randomUUID();assert.equal((await q({scope,ownerGeneration:generation,observe:{eventId:pickupId,kind:'heart',progression:0}},f.values[0].credentials)).accepted,true);
  const death=await rpc('/v3/c1/rpc',c1,f.values[0].credentials,{scope,ownerGeneration:generation,publish:{eventId,rulesRevision:changed.snapshot.revision,payload:{death:{state:'terminal',cause:'normal'}},senderDisposition:'pending'}});assert.equal(death.error,undefined);
  let summary=(await q({read:{locator}})).summary;assert.equal(summary.runStatus,'finished');assert.deepEqual(summary.rows.map((r:any)=>r.deaths).sort(),['0','1']);const elapsed=summary.elapsedSeconds;
  const original={scope,ownerGeneration:generation,observe:{eventId,kind:'death',progression:0}};
  const recovered=await q(original,x.desktop);assert.equal(recovered.accepted,true);assert.equal(recovered.duplicate,true);
  const pickup={scope,ownerGeneration:generation,observe:{eventId:pickupId,kind:'heart',progression:0}};
  assert.equal((await q(pickup,x.desktop)).duplicate,true);
  assert.equal((await q(pickup,f.peer.desktop)).error.code,'FORBIDDEN');
  assert.equal((await q({...pickup,scope:{...scope,configRevision:scope.configRevision!+1}},x.desktop)).error.code,'FORBIDDEN');
  assert.equal((await q({...pickup,observe:{...pickup.observe,eventId:randomUUID()}},x.desktop)).error.code,'FORBIDDEN');
  assert.equal((await q({...original,observe:{...original.observe,eventId:randomUUID()}},x.desktop)).error.code,'FORBIDDEN');
  x.advance(60000);summary=(await q({read:{locator}})).summary;assert.equal(summary.elapsedSeconds,elapsed);assert.equal(summary.rows.reduce((n:number,r:any)=>n+Number(r.deaths),0),1);assert.equal(summary.rows.reduce((n:number,r:any)=>n+Number(r.wins),0),0);
  await x.nextRun();const current=(await q({read:{locator}})).summary;assert.equal(current.runKey,summary.runKey,'pre-Start staging still shows latest finished run');
  console.log(JSON.stringify({d1TerminalOrdering:true,oneSourceDeath:true,forcedPeerEndNotDeath:true,exactTerminalOriginalOnly:true,frozenElapsed:true,actualNoita:false}));
 }finally{await f.close();}
});

test('D1 persistent aggregate survives service restart, active runs outlive retention and expired/deleted views never invent zeroes',async()=>{
 const f=await b1Fixture(),x=f.x,q=transport(x);try{
  await x.rpc(x.host,{roomGet:{}});await x.rpc(x.host,{roomPolicy:{publicStats:true}});const locator=(await q({discover:{}})).lobbies[0].locator;
  const observed=await q({scope:f.scope(0),ownerGeneration:f.values[0].world.generation,observe:{eventId:randomUUID(),kind:'heart',progression:0}},f.values[0].credentials);assert.equal(observed.accepted,true);
  const before=(await q({read:{locator}})).summary;await x.fixture.restart();assert.equal((await q({read:{locator}})).summary.rows.reduce((n:number,r:any)=>n+Number(r.hearts),0),1);
  for(let h=0;h<31*24;h++)x.advance(3600000);let current=(await q({read:{locator}})).summary;assert.equal(current.collectionState,'available');assert.equal(current.runKey,before.runKey);
  const host=await x.fixture.service.db.transaction((tx:any)=>x.fixture.service.auth.family(tx,x.host.accountId!,randomUUID(),'browser'));await x.rpc(host,{roomGet:{}});await x.rpc(host,{endRun:{}});
  for(let h=0;h<30*24;h++)x.advance(3600000);x.advance(1);current=(await q({read:{locator}})).summary;assert.equal(current.collectionState,'expired');assert.equal(current.rows,undefined);
  await x.fixture.service.db.transaction((tx:any)=>maintainStatistics(tx,x.fixture.service.auth.now()));assert.equal((await x.fixture.service.db.pool.query('SELECT count(*) FROM d1_players')).rows[0].count,'0');
  const freshHost=await x.fixture.service.db.transaction((tx:any)=>x.fixture.service.auth.family(tx,x.host.accountId!,randomUUID(),'browser'));await x.rpc(freshHost,{roomGet:{}});await x.rpc(freshHost,{roomDelete:{}});assert.equal((await q({read:{locator}})).error.code,'NOT_FOUND');assert.equal((await q({discover:{}})).lobbies.length,0);
  console.log(JSON.stringify({d1RestartPersistence:true,activeBeyond30Days:true,endBasedExpiry:true,deletedDenied:true,actualNoita:false}));
 }finally{await f.close();}
});

test('D1 real committed reply loss retries once and delayed optional stats never holds bank work',async()=>{
 const f=await b1Fixture(),x=f.x,q=transport(x);try{
  const eventId=randomUUID(),original={scope:f.scope(0),ownerGeneration:f.values[0].world.generation,observe:{eventId,kind:'heart',progression:0}};
  x.faults.d1Control.dropReply('consumer');await assert.rejects(q(original,f.values[0].credentials));
  assert.equal(x.faults.d1Control.status().dropped,1);assert.equal((await q(original,f.values[0].credentials)).duplicate,true);
  x.faults.d1Control.delay('consumer',2000);let settled=false;const delayed=q({...original,observe:{...original.observe,eventId:randomUUID()}},f.values[0].credentials).finally(()=>{settled=true;});
  for(let i=0;i<100&&!x.faults.d1Control.status().pendingDelays;i++)await new Promise(r=>setTimeout(r,5));
  assert.equal(x.faults.d1Control.status().pendingDelays,1);await f.ok(1,{goldBank:{}});assert.equal(settled,false);assert.equal((await delayed).accepted,true);
  const locator=(await q({discover:{}})).lobbies[0].locator,summary=(await q({read:{locator}},x.host)).summary;assert.equal(summary.rows.reduce((a:number,r:any)=>a+Number(r.hearts),0),2);
  // Synthetic owned-schema boundary; do not generate65k gameplay mutations.
  await x.fixture.service.db.pool.query('UPDATE d1_runs SET event_count=65536 WHERE run_id=$1',[x.room.runId]);
  assert.equal((await q({...original,observe:{...original.observe,eventId:randomUUID()}},f.values[0].credentials)).accepted,false);
  assert.equal((await q(original,f.values[0].credentials)).duplicate,true);await f.ok(1,{goldBank:{}});
  assert.equal((await q({read:{locator}},x.host)).summary.collectionState,'partial');
  console.log(JSON.stringify({d1CommittedReplyLoss:true,stableRetryOneCount:true,optionalDelayBankIndependent:true,actualNoita:false}));
 }finally{await f.close();}
});

test('D1 no run and legacy missing collection stay distinct; Start replay cannot retroactively invent zero rows',async()=>{
 const x=await startLifecycleLc1Fixture(),q=transport(x);try{
  const locator=(await q({discover:{}})).lobbies[0].locator;assert.equal((await q({read:{locator}},x.host)).summary.collectionState,'none');
  let original:any;const call=x.fixture.call;x.fixture.call=((...args:any[])=>{if(args[0]==='/v1/rpc'){const body=typeof args[1]==='string'?JSON.parse(args[1]):args[1];if(body?.finalStart)original=structuredClone(body);}return (call as any)(...args);}) as any;
  await x.send(x.desktop,x.request());await x.start();assert.ok(original);
  await x.fixture.service.db.pool.query('DELETE FROM d1_runs WHERE run_id=$1',[x.room.runId]);
  assert.equal((await q({read:{locator}},x.host)).summary.collectionState,'unavailable');
  const replay=JSON.parse((await call('/v1/rpc',original,x.host.accessToken)).body);assert.equal(replay.error,undefined);
  const unavailable=(await q({read:{locator}},x.host)).summary;assert.equal(unavailable.collectionState,'unavailable');assert.equal(unavailable.rows,undefined);
  console.log(JSON.stringify({d1NoRunDistinct:true,legacyUnavailable:true,replayDoesNotInventZeroes:true,actualNoita:false}));
 }finally{await x.close();}
});

test('D1 observed orb identity survives delayed C1 receipt and shared recipient/anger effects are not collectors',async()=>{
 const f=await b1Fixture({worldRules:true,itemTransfers:true}),x=f.x,q=transport(x),rpc=typed(x);try{
  const scope=f.scope(0),credentials=f.values[0].credentials,ownerGeneration=f.values[0].world.generation;
  const c1Rules=await rpc('/v3/c1/rpc',c1,x.host,{roomId:scope.roomId,rulesGet:{}});
  const worldState=async(progression:number)=>{x.advance(150);return rpc('/v3/c2/rpc',c2,credentials,{scope,ownerGeneration,state:{progression}});};
  await worldState(0);const eventId=randomUUID();x.advance(150);
  const pending=await rpc('/v3/c1/rpc',c1,credentials,{scope,ownerGeneration,publish:{eventId,rulesRevision:c1Rules.snapshot.revision,payload:{orb:{orbId:3}},senderDisposition:'pending'}});assert.equal(pending.error,undefined);
  assert.equal((await q({scope,ownerGeneration,observe:{eventId,kind:'steve',progression:0}},credentials)).error.code,'CONFLICT');
  assert.equal((await q({scope,ownerGeneration,observe:{eventId,kind:'orb',orbId:'3',progression:0}},credentials)).accepted,true);
  const next=await worldState(1);x.advance(150);
  assert.equal((await q({scope,ownerGeneration,observe:{eventId,kind:'orb',orbId:'3',progression:0}},credentials)).duplicate,true,'accepted original survives progression advancement');
  const applied=await rpc('/v3/c1/rpc',c1,credentials,{scope,ownerGeneration,receipt:{eventId,sequence:'0',state:'applied'}});assert.equal(applied.error,undefined);
  assert.equal((await q({scope,ownerGeneration,observe:{eventId:randomUUID(),kind:'orb',orbId:'3',progression:1}},credentials)).accepted,true);
  assert.equal((await q({scope,ownerGeneration,observe:{eventId:randomUUID(),kind:'orb',orbId:'999',progression:1}},credentials)).error.code,'UNSUPPORTED_CONTENT');
  x.advance(150);const received=await rpc('/v3/c1/rpc',c1,f.values[1].credentials,{scope:f.scope(1),ownerGeneration:f.values[1].world.generation,receipt:{eventId,sequence:pending.event.sequence,state:'applied'}});assert.equal(received.error,undefined);
  const angerId=randomUUID();x.advance(150);const anger=await rpc('/v3/c2/rpc',c2,credentials,{scope,ownerGeneration,publish:{eventId:angerId,rulesRevision:next.snapshot.rules.revision,progression:1,payload:{steve:{progression:1}},senderDisposition:'applied'}});assert.equal(anger.error,undefined);
  assert.equal((await q({scope,ownerGeneration,observe:{eventId:angerId,kind:'steve',progression:1}},credentials)).error.code,'CONFLICT');
  const locator=(await q({discover:{}})).lobbies[0].locator,rows=(await q({read:{locator}},x.host)).summary.rows;assert.deepEqual(rows.map((r:any)=>r.orbs).sort(),['0','2']);assert.ok(rows.every((r:any)=>r.steveKills==='0'));
  console.log(JSON.stringify({d1ObservedPickupProgression:true,sharedRecipientNotCollector:true,angerNotSteveKill:true,unknownOrbRefused:true,actualNoita:false}));
 }finally{await f.close();}
});

test('D1 conflicting optional receipt cannot veto canonical terminal gameplay',async()=>{
 const f=await b1Fixture({itemTransfers:true}),x=f.x,q=transport(x),rpc=typed(x);try{
  const scope=f.scope(0),credentials=f.values[0].credentials,ownerGeneration=f.values[0].world.generation,eventId=randomUUID();
  // Deliberately contradictory synthetic reports exercise failure isolation.
  assert.equal((await q({scope,ownerGeneration,observe:{eventId,kind:'heart',progression:0}},credentials)).accepted,true);
  const current=await rpc('/v3/c1/rpc',c1,x.host,{roomId:scope.roomId,rulesGet:{}}),rules=await rpc('/v3/c1/rpc',c1,x.host,{roomId:scope.roomId,rulesSet:{expectedRevision:current.snapshot.revision,rules:{...current.snapshot.rules,NT_death_penalty:'end',NT_ondeath_kick:false}}});
  const death=await rpc('/v3/c1/rpc',c1,credentials,{scope,ownerGeneration,publish:{eventId,rulesRevision:rules.snapshot.revision,payload:{death:{state:'terminal',cause:'normal'}},senderDisposition:'pending'}});assert.equal(death.error,undefined);
  const locator=(await q({discover:{}})).lobbies[0].locator,summary=(await q({read:{locator}},x.host)).summary;assert.equal(summary.runStatus,'finished');assert.equal(summary.collectionState,'partial');
  assert.equal((await q({scope,ownerGeneration,observe:{eventId,kind:'death'}},x.desktop)).error.code,'CONFLICT');
  console.log(JSON.stringify({d1OptionalConflictNoGameplayVeto:true,canonicalEndContinues:true,partialDataExplicit:true,actualNoita:false}));
 }finally{await f.close();}
});

test('D1 respawn/life terminal uses one death original, while a later actual death counts again',async()=>{
 const f=await b1Fixture({itemTransfers:true}),x=f.x,q=transport(x),rpc=typed(x);try{
  const scope=f.scope(0),credentials=f.values[0].credentials,ownerGeneration=f.values[0].world.generation;
  const current=await rpc('/v3/c1/rpc',c1,x.host,{roomId:scope.roomId,rulesGet:{}}),rules=await rpc('/v3/c1/rpc',c1,x.host,{roomId:scope.roomId,rulesSet:{expectedRevision:current.snapshot.revision,rules:{...current.snapshot.rules,NT_death_penalty:'weak_respawn',NT_ondeath_kick:false}}});
  for(let i=0;i<2;i++){const eventId=randomUUID();x.advance(200);const death=await rpc('/v3/c1/rpc',c1,credentials,{scope,ownerGeneration,publish:{eventId,rulesRevision:rules.snapshot.revision,payload:{death:{state:'respawning',cause:'normal'}},senderDisposition:'applied'}});assert.equal(death.error,undefined);x.advance(200);const life=await rpc('/v3/c1/rpc',c1,credentials,{scope,ownerGeneration,life:{deathEventId:eventId,state:i?'terminal':'playing'}});assert.equal(life.error,undefined);}
  const locator=(await q({discover:{}})).lobbies[0].locator,summary=(await q({read:{locator}},x.host)).summary;assert.equal(summary.runStatus,'finished');assert.deepEqual(summary.rows.map((r:any)=>r.deaths).sort(),['0','2']);
  console.log(JSON.stringify({d1RespawnDeathOriginal:true,lifeTerminalNotDouble:true,laterDeathCounts:true,actualNoita:false}));
 }finally{await f.close();}
});
