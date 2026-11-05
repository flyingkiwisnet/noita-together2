import test from 'node:test';import assert from 'node:assert/strict';import {randomUUID} from 'node:crypto';
import {startLifecycleLc1Fixture} from '../scripts/lifecycle-lc1-fixture.js';
import {encode,decode} from '../../../packages/contracts/src/g3-race.js';
import {purgeRace} from '../src/g3/race-store.js';
import {schema as socialSchema} from '../../../packages/contracts/src/g3-social.js';
import {schema as lobbySchema} from '../../../packages/contracts/src/g2-next.js';
import {transientEventId} from '../../../packages/contracts/src/g3-social-replay.js';
const allGoals=[{instanceId:'hp',id:'reach_max_hp',target:500},{instanceId:'gold',id:'accumulate_gold',target:10000},{instanceId:'orbs',id:'collect_orbs',target:1},{instanceId:'bosses',id:'defeat_bosses',target:1},{instanceId:'named',id:'defeat_named_bosses',bossIds:['pyramid_boss']},{instanceId:'biome',id:'visit_biome',biomeId:'coal_pits'},{instanceId:'parallel',id:'visit_parallel_world',direction:'east',index:1},{instanceId:'ending',id:'classic_ending'}];
async function fixture(durationMs=0,goals=allGoals){
 const x=await startLifecycleLc1Fixture({race:true,b1Gold:true,raceConfiguration:{registryRevision:'race-goals-1',durationMs,goals}});
 try{
  const peer=await x.bootstrapPeer(),qa=x.request(),qb=x.request(peer.desktop,1,'race-peer',peer.observation);qb.reconcile!.association=peer.association;qb.reconcile!.proof=peer.proof;
  let a=await x.send(x.desktop,qa),b=await x.send(peer.desktop,qb);assert.equal(a.error,undefined,JSON.stringify(a.error));assert.equal(b.error,undefined,JSON.stringify(b.error));await x.start();
  qa.reconcile!.observation={...x.observation,protected:false,observationRevision:2};qb.reconcile!.observation={...peer.observation,protected:false,observationRevision:2};a=await x.send(x.desktop,qa);b=await x.send(peer.desktop,qb);
  assert.equal(a.error,undefined,JSON.stringify(a.error));assert.equal(b.error,undefined,JSON.stringify(b.error));assert.equal(a.canonicalAdmission.decision.playing,true,JSON.stringify(a.canonicalAdmission.decision));
  const values=[a,b],actors=[x.desktop,peer.desktop];
  const scope=(i:number)=>Object.fromEntries(['roomId','runId','runEpoch','configRevision','attachmentId','worldToken'].map(k=>[k,values[i].canonicalAdmission.scope[k]]));
  const call=async(payload:any,actor:any=actors[0])=>{x.advance(200);const request={major:1,revision:'g3-race-1',serviceId:x.fixture.service.db.serviceId,serviceEpoch:x.fixture.service.db.epoch,requestId:randomUUID(),...(actor?{sessionEpoch:actor.sessionEpoch}:{}),...payload};const r=JSON.parse((await x.fixture.call('/rpc/g3/race',request,actor?.accessToken)).body);decode('Response',encode('Response',r));return r;};
  const commit=async(i:number,p:any)=>call({scope:scope(i),ownerGeneration:values[i].canonicalAdmission.generation,commit:p},actors[i]);
  const snapshot=async(actor:any=actors[0])=>call({read:{locator:x.room.statsLocator,limit:8}},actor);
  return {x,values,actors,scope,call,commit,snapshot,close:x.close};
 }catch(e){await x.close();throw e;}
}
test('Race actual Start, eight goal kinds, originals, ordered terminal and individual finish',async()=>{
 const f=await fixture();try{
  const initial=await f.snapshot();assert.equal(initial.error,undefined,JSON.stringify(initial));assert.equal(initial.snapshot.entrants.length,2);assert.ok(initial.snapshot.entrants.every((e:any)=>e.completedGoals===0));
  const first={eventId:randomUUID(),sequence:1,terminal:'none',observations:[{kind:'orb',orbId:0},{kind:'boss',bossId:'pyramid_boss'},{kind:'biome',biomeId:'coal_pits'},{kind:'parallel',direction:'east',index:3},{kind:'max_hp',value:'500'},{kind:'gold',value:'10000'}]};
  const one=await f.commit(0,first);assert.equal(one.error,undefined,JSON.stringify(one));assert.equal(one.receipt.completedGoals,7);assert.equal(one.receipt.status,'racing');
  f.x.advance(50);assert.deepEqual((await f.commit(0,first)).receipt,one.receipt);
  assert.equal((await f.commit(0,{...first,observations:[]})).error.code,'CONFLICT');
  assert.equal((await f.commit(0,{eventId:randomUUID(),sequence:3,terminal:'none',observations:[]})).error.code,'SEQUENCE_GAP');
  const last={eventId:randomUUID(),sequence:2,terminal:'dead',observations:[{kind:'ending',ending:'classic'}]};
  const finished=await f.commit(0,last);assert.equal(finished.error,undefined,JSON.stringify(finished));assert.equal(finished.receipt.status,'finished');assert.equal(finished.receipt.finishOrder,1);
  assert.equal((await f.snapshot()).snapshot.status,'running');{const retried=await f.commit(0,last);assert.equal(retried.error,undefined,JSON.stringify(retried));assert.deepEqual(retried.receipt,finished.receipt);}
  assert.equal((await f.commit(0,{eventId:randomUUID(),sequence:3,terminal:'none',observations:[]})).error.code,'FORBIDDEN');
  const dead=await f.commit(1,{eventId:randomUUID(),sequence:1,terminal:'dead',observations:[{kind:'gold',value:'10000'}]});assert.equal(dead.error,undefined,JSON.stringify(dead));assert.equal(dead.receipt.status,'dead');
  const ended=(await f.snapshot()).snapshot;assert.equal(ended.status,'ended');assert.equal(ended.endReason,'race_all_terminal');assert.deepEqual(ended.entrants.map((e:any)=>e.completedGoals).sort((a:number,b:number)=>a-b),[1,8]);
  assert.deepEqual((await f.commit(0,last)).receipt,finished.receipt);
 }finally{await f.close();}
});
test('Race HP100 and HP200 originals produce bounded binary receipts and exact replay',async()=>{
 const f=await fixture();try{
  const binary=async(commit:any)=>{
   const request={major:1,revision:'g3-race-1',serviceId:f.x.fixture.service.db.serviceId,serviceEpoch:f.x.fixture.service.db.epoch,requestId:randomUUID(),sessionEpoch:f.actors[0]!.sessionEpoch,scope:f.scope(0),ownerGeneration:f.values[0].canonicalAdmission.generation,commit};
   const response=await f.x.fixture.service.app.inject({method:'POST',url:'/rpc/g3/race',headers:{host:new URL(f.x.fixture.origin).host,authorization:`Bearer ${f.actors[0]!.accessToken}`,'content-type':'application/x-protobuf'},payload:Buffer.from(encode('Request',request))});
   assert.match(response.headers['content-type'] as string,/application\/x-protobuf/,response.body);return decode('Response',response.rawPayload) as any;
  };
  for(const [index,value]of ['100','200'].entries()){
   const original={eventId:randomUUID(),sequence:index+1,terminal:'none',observations:[{kind:'max_hp',value}]};
   const accepted=await binary(original);assert.equal(accepted.error,undefined,JSON.stringify(accepted));assert.equal(accepted.receipt.sequence,index+1);assert.equal(accepted.receipt.status,'racing');assert.deepEqual((await binary(original)).receipt,accepted.receipt);
  }
  const snapshot=await f.snapshot();assert.equal(snapshot.error,undefined,JSON.stringify(snapshot));assert.equal(snapshot.snapshot.entrants.find((v:any)=>v.rowKey===snapshot.snapshot.selfRow).progress.find((v:any)=>v.instanceId==='hp').currentValue,'200');
  assert.deepEqual(f.x.faults.raceControl.status().responses.map(v=>[v.operation,v.sequence,v.code]),[['commit',1,'receipt'],['commit',1,'receipt'],['commit',2,'receipt'],['commit',2,'receipt'],['read',undefined,'snapshot']]);
 }finally{await f.close();}
});
test('Race private/public revocation, current device fencing, strict deadline and retained receipt',async()=>{
 const f=await fixture(60000);try{
  assert.equal((await f.snapshot(null)).error.code,'NOT_FOUND');
  await f.x.rpc(f.x.host,{roomGet:{}});await f.x.rpc(f.x.host,{roomPolicy:{publicStats:true}});
  const pub=await f.snapshot(null);assert.equal(pub.snapshot.collectionState,'available');assert.equal(JSON.stringify(pub).includes(f.x.desktop.accountId!),false);assert.equal(JSON.stringify(pub).includes(f.scope(0).worldToken),false);
  await f.x.rpc(f.x.host,{roomPolicy:{publicStats:false}});assert.equal((await f.snapshot(null)).error.code,'NOT_FOUND');
  const event={eventId:randomUUID(),sequence:1,terminal:'none',observations:[{kind:'gold',value:'10000'}]};const original=await f.commit(0,event);assert.equal(original.error,undefined,JSON.stringify(original));
  f.x.advance(60001);const late=await f.commit(1,{eventId:randomUUID(),sequence:1,terminal:'none',observations:[]});assert.equal(late.error.code,'FORBIDDEN');
  const ended=(await f.snapshot()).snapshot;assert.equal(ended.status,'ended');assert.equal(ended.endReason,'race_deadline');assert.deepEqual((await f.commit(0,event)).receipt,original.receipt);
 }finally{await f.close();}
});
test('Race one account per lobby leaves concurrent host controls and current other account intact',async()=>{
 const f=await fixture();try{
  const stranger=await f.x.fixture.service.db.transaction(tx=>f.x.fixture.service.auth.family(tx,f.x.desktop.accountId!,randomUUID(),'desktop'));
  await assert.rejects(f.x.rpc(stranger,{roomJoin:{}}),/ACCOUNT_ALREADY_JOINED/);
  const count=await f.x.fixture.service.db.transaction(async tx=>(await tx.query('SELECT count(*) FROM memberships WHERE room_id=$1 AND NOT roster_retired',[f.x.room.roomId])).rows[0].count);assert.equal(Number(count),2);
  await f.x.rpc(f.x.host,{roomGet:{}});await f.x.rpc(f.x.host,{roomPolicy:{publicStats:true}});assert.equal((await f.snapshot(null)).snapshot.entrants.length,2);
  const forged={eventId:randomUUID(),sequence:1,terminal:'none',observations:[{kind:'gold',value:'10000'}]};
  assert.equal((await f.call({scope:f.scope(0),ownerGeneration:f.values[0].canonicalAdmission.generation,commit:forged},f.actors[1])).error.code,'FORBIDDEN');
  assert.equal((await f.call({scope:f.scope(0),ownerGeneration:999,commit:forged})).error.code,'STALE_EPOCH');
  assert.equal((await f.commit(0,{...forged,observations:[{kind:'boss',bossId:'unverified'}]})).error.code,'INVALID_MESSAGE');
  assert.equal((await f.snapshot()).snapshot.entrants.reduce((n:number,e:any)=>n+e.completedGoals,0),0);
 }finally{await f.close();}
});
test('Race retained-current-owner desktop drains terminal after game lease/actor loss',async()=>{
 const f=await fixture();try{
  await f.x.fixture.loseParticipant(f.values[0].canonicalAdmission.participantId);
  const event={eventId:randomUUID(),sequence:1,terminal:'dead',observations:[{kind:'max_hp',value:'500'},{kind:'gold',value:'10000'}]};
  const received=await f.commit(0,event);assert.equal(received.error,undefined,JSON.stringify(received));assert.equal(received.receipt.status,'dead');assert.equal(received.receipt.completedGoals,2);
  assert.deepEqual((await f.commit(0,event)).receipt,received.receipt);
  assert.equal((await f.commit(0,{...event,eventId:randomUUID(),sequence:2})).error.code,'FORBIDDEN');
 }finally{await f.close();}
});
test('Race revision-fenced paging, host End, expired and deleted results',async()=>{
 const f=await fixture();try{
  const first=await f.call({read:{locator:f.x.room.statsLocator,limit:1}});assert.ok(first.snapshot.nextRow);
  await f.commit(0,{eventId:randomUUID(),sequence:1,terminal:'none',observations:[{kind:'gold',value:'10000'}]});
  assert.equal((await f.call({read:{locator:f.x.room.statsLocator,afterRow:first.snapshot.nextRow,limit:1,runKey:first.snapshot.runKey,snapshotRevision:first.snapshot.revision}})).error.code,'STALE_REVISION');
  await f.x.end();const ended=(await f.snapshot()).snapshot;assert.equal(ended.status,'ended');assert.ok(ended.entrants.every((e:any)=>e.status==='dnf'));
  await f.x.fixture.service.db.transaction(async tx=>{await tx.query('UPDATE race_runs SET expires_at=$2 WHERE run_id=$1',[f.x.room.runId,new Date(f.x.fixture.service.auth.now()-1)]);await purgeRace(tx,f.x.fixture.service.auth.now());});
  const expired=(await f.snapshot()).snapshot;assert.equal(expired.collectionState,'expired');assert.equal(expired.entrants,undefined);
  await f.x.rpc(f.x.host,{roomGet:{}});await f.x.rpc(f.x.host,{roomDelete:{}});assert.equal((await f.snapshot()).error.code,'NOT_FOUND');
 }finally{await f.close();}
});
test('Race service restart preserves official Start, deadline and accepted-original recovery',async()=>{
 const f=await fixture(60000);try{
  const event={eventId:randomUUID(),sequence:1,terminal:'none',observations:[{kind:'orb',orbId:0}]},original=await f.commit(0,event);assert.equal(original.error,undefined,JSON.stringify(original));
  await f.x.rpc(f.x.host,{roomGet:{}});await f.x.rpc(f.x.host,{roomPolicy:{publicStats:true}});const before=(await f.snapshot(null)).snapshot;
  await f.x.fixture.restart();const after=(await f.snapshot(null)).snapshot;assert.equal(after.startedAt,before.startedAt);assert.equal(after.deadlineAt,before.deadlineAt);assert.equal(after.runKey,before.runKey);
  f.actors[0]=await f.x.signIn();assert.deepEqual((await f.commit(0,event)).receipt,original.receipt);
  const pending={eventId:randomUUID(),sequence:2,terminal:'dead',observations:[{kind:'max_hp',value:'500'}]};
  const terminal=await f.commit(0,pending);assert.equal(terminal.error,undefined,JSON.stringify(terminal));assert.equal(terminal.receipt.status,'dead');assert.equal(terminal.receipt.completedGoals,2);
  f.x.advance(60001);const closed=(await f.snapshot(null)).snapshot;assert.equal(closed.status,'ended');assert.equal(closed.endReason,'race_deadline');assert.equal(closed.endedAt,before.deadlineAt);
  assert.deepEqual((await f.commit(0,event)).receipt,original.receipt);
 }finally{await f.close();}
});
test('Race authenticated social roster maps opaque counts by world identity despite equal names',async()=>{
 const f=await fixture();try{
  await f.x.fixture.service.db.transaction(async tx=>{await tx.query('UPDATE principal_profiles SET display_name=$1 WHERE principal_id=ANY($2::uuid[])',['Equal name',f.actors.map(a=>a.accountId)]);await tx.query('UPDATE race_entrants SET display_name=$2 WHERE run_id=$1',[f.x.room.runId,'Equal name']);});
  await f.commit(0,{eventId:randomUUID(),sequence:1,terminal:'none',observations:[{kind:'gold',value:'10000'}]});
  const c=f.values[0].credentials;
  const roster=JSON.parse((await f.x.fixture.call('/v3/social/rpc',{major:1,revision:socialSchema.revision,serviceId:f.x.fixture.service.db.serviceId,serviceEpoch:f.x.fixture.service.db.epoch,sessionEpoch:c.sessionEpoch,requestId:randomUUID(),scope:f.scope(0),roster:{limit:32}},c.accessToken)).body);
  assert.equal(roster.error,undefined,JSON.stringify(roster));assert.equal(roster.players.length,2);assert.equal(new Set(roster.players.map((p:any)=>p.raceRowKey)).size,2);
  assert.equal(roster.players.find((p:any)=>p.worldToken===f.scope(0).worldToken).completedGoals,1);assert.equal(roster.players.find((p:any)=>p.worldToken===f.scope(1).worldToken).completedGoals,0);
  assert.ok(roster.players.every((p:any)=>p.displayName==='Equal name'&&p.raceStatus==='racing'));
 }finally{await f.close();}
});
test('Race social HUD, emote, cosmetic and POI publish to an admitted peer without granting goal progress',async()=>{
 const f=await fixture();try{
  const call=async(i:number,payload:any,scope=f.scope(i))=>{const c=f.values[i].credentials;return JSON.parse((await f.x.fixture.call('/v3/social/rpc',{major:1,revision:socialSchema.revision,serviceId:f.x.fixture.service.db.serviceId,serviceEpoch:f.x.fixture.service.db.epoch,sessionEpoch:c.sessionEpoch,requestId:randomUUID(),scope,...payload},c.accessToken)).body);};
  const payloads={hud:{hpMilli:100000,maxHpMilli:100000,hpKnown:true,sampo:false,sampoKnown:true},emote:{emoteId:'wave',active:true},cosmetic:{robeId:'red',hat:false,amulet:false,gem:false},poi:{text:'Race marker',xMilli:'1000',yMilli:'2000'}};
  for(const [kind,payload]of Object.entries(payloads)){
   const eventId=kind==='poi'?randomUUID():transientEventId(f.scope(0),f.values[0].credentials.sessionId,kind as 'hud'|'emote'|'cosmetic',1);
   const accepted=await call(0,{publish:{eventId,sequence:1,[kind]:payload}});assert.equal(accepted.error,undefined,JSON.stringify(accepted));assert.equal(accepted.acceptedEventId,eventId);
  }
  const peer=await call(1,{snapshot:{limit:32}});assert.equal(peer.error,undefined,JSON.stringify(peer));assert.ok(peer.projections.some((v:any)=>v.hud?.hpMilli===100000));assert.ok(peer.projections.some((v:any)=>v.emote?.emoteId==='wave'));assert.ok(peer.projections.some((v:any)=>v.cosmetic?.robeId==='red'));
  const poi=await call(1,{poiSnapshot:{limit:32}});assert.equal(poi.error,undefined,JSON.stringify(poi));assert.equal(poi.markers[0].poi.text,'Race marker');
  assert.equal((await call(0,{snapshot:{limit:32}},{...f.scope(0),worldToken:randomUUID()})).error.code,'STALE_EPOCH');
  assert.ok((await f.snapshot()).snapshot.entrants.every((v:any)=>v.completedGoals===0));
 }finally{await f.close();}
});
test('Race simultaneous finishes retain distinct ordinals; deadline exact count/time and zero ties share rank',async()=>{
 const finished=await fixture(0,[{instanceId:'hp',id:'reach_max_hp',target:500}]);try{
  const results=await Promise.all([0,1].map(i=>finished.commit(i,{eventId:randomUUID(),sequence:1,terminal:'dead',observations:[{kind:'max_hp',value:'500'}]})));
  for(const value of results)assert.equal(value.error,undefined,JSON.stringify(value));
  assert.deepEqual(results.map(r=>r.receipt.finishOrder).sort(),[1,2]);assert.equal(results[0].receipt.acceptedAtMs,results[1].receipt.acceptedAtMs);
  const view=(await finished.snapshot()).snapshot;assert.equal(view.status,'ended');assert.deepEqual(view.entrants.map((e:any)=>e.rank).sort(),[1,2]);
 }finally{await finished.close();}
 const tied=await fixture(60000);try{
  const receipts=await Promise.all([0,1].map(i=>tied.commit(i,{eventId:randomUUID(),sequence:1,terminal:'none',observations:[{kind:'gold',value:'10000'}]})));
  for(const v of receipts)assert.equal(v.error,undefined,JSON.stringify(v));assert.equal(receipts[0].receipt.acceptedAtMs,receipts[1].receipt.acceptedAtMs);
  tied.x.advance(60001);const view=(await tied.snapshot()).snapshot;
  assert.deepEqual(view.entrants.map((e:any)=>e.rank),[1,1]);assert.ok(view.entrants.every((e:any)=>e.finalCountAtMs===receipts[0].receipt.acceptedAtMs&&e.completedGoals===1&&e.status==='dnf'));
 }finally{await tied.close();}
 const zero=await fixture(60000);try{
  zero.x.advance(60001);
  // Ordinary service timer, no RPC or direct-tick loop closes the idle race.
  await new Promise(resolve=>setTimeout(resolve,1250));
  const record=await zero.x.fixture.service.db.transaction(async tx=>(await tx.query('SELECT ended_at,deadline_at,end_reason FROM race_runs WHERE run_id=$1',[zero.x.room.runId])).rows[0]);
  assert.equal(record.end_reason,'race_deadline');assert.equal(+record.ended_at,+record.deadline_at);
  const view=(await zero.snapshot()).snapshot;assert.deepEqual(view.entrants.map((e:any)=>e.rank),[1,1]);assert.ok(view.entrants.every((e:any)=>e.completedGoals===0&&e.finalCountAtMs===''));
 }finally{await zero.close();}
});
test('Race failed selected detector inspection cannot Start or freeze a competition',async()=>{
 const x=await startLifecycleLc1Fixture({race:true,b1Gold:true});try{
  const q=x.request();q.reconcile!.observation={...x.observation,inventoryComplete:false,collectorFailure:'collection_failed'};
  const result=await x.send(x.desktop,q);assert.ok(!result.error||['REPORT_CHANGED','NOT_READY'].includes(result.error.code),JSON.stringify(result));
  await assert.rejects(x.start(),/NOT_READY|REPORT_CHANGED|REPORT_REQUIRED|ROSTER_MISMATCH/);
  const count=await x.fixture.service.db.transaction(async tx=>(await tx.query('SELECT count(*) FROM race_runs')).rows[0].count);assert.equal(Number(count),0);
 }finally{await x.close();}
});
test('Race rejects cooperative bank publication and leaves for a separate normal Core lobby',async()=>{
 const f=await fixture();try{
  const game=f.values[0].credentials;
  const gameplay=async(scope:any,credential:any,payload:any)=>JSON.parse((await f.x.fixture.call('/v3/rpc',{major:1,revision:'g3-draft-4',serviceId:f.x.fixture.service.db.serviceId,serviceEpoch:f.x.fixture.service.db.epoch,sessionEpoch:credential.sessionEpoch,requestId:randomUUID(),scope,...payload},credential.accessToken)).body);
  const denied=await gameplay(f.scope(0),game,{goldIntent:{operationId:randomUUID(),direction:'deposit',amount:1}});assert.equal(denied.error.code,'FORBIDDEN');
  await f.x.end();const raceRoomId=f.x.room.roomId,raceLocator=f.x.room.statsLocator,{race,...base}=f.x.room.config;
  await f.x.rpc(f.x.desktop,{roomLeave:{}});await f.x.rpc(f.actors[1]!,{roomLeave:{}});
  await f.x.createCoreLobby({...base,mode:'core',requiredCapabilities:base.requiredCapabilities.filter((c:string)=>c!=='race.goals.v1'),settings:[{key:'NT_send_gold',value:'true'}]});assert.notEqual(f.x.room.roomId,raceRoomId);
  await f.x.rpc(f.x.desktop,{roomJoin:{}});const fresh=await f.x.prepareWorld();
  const q=f.x.request(f.x.desktop,1,'core-after-race',{...f.x.observation,worldToken:fresh.association.worldId!,gameInstanceId:'core-after-race',observationRevision:1,protected:true});q.reconcile!.association=fresh.association;q.reconcile!.proof=fresh.proof;
  assert.equal((await f.x.send(f.x.desktop,q)).error,undefined);await f.x.start();q.reconcile!.observation={...q.reconcile!.observation!,observationRevision:2,protected:false};const playing=await f.x.send(f.x.desktop,q);assert.equal(playing.error,undefined,JSON.stringify(playing));assert.equal(playing.canonicalAdmission.decision.playing,true);
  const scope=Object.fromEntries(['roomId','runId','runEpoch','configRevision','attachmentId','worldToken'].map(k=>[k,playing.canonicalAdmission.scope[k]]));
  const allowed=await gameplay(scope,playing.credentials,{goldIntent:{operationId:randomUUID(),direction:'deposit',amount:1}});assert.equal(allowed.error,undefined,JSON.stringify(allowed));assert.equal(allowed.operation.state,'awaiting_local');
  const prior=(await f.call({read:{locator:raceLocator,limit:8}})).snapshot;assert.equal(prior.lobbyMode,'race');assert.equal(prior.status,'ended');assert.deepEqual(prior.config,race);
 }finally{await f.close();}
});
test('Separate Core fixture reads canonical automatic Race closure instead of cached running state',async()=>{
 const f=await fixture();try{
  const oldRoom=structuredClone(f.x.room),{race,...base}=oldRoom.config,core={...base,mode:'core',requiredCapabilities:base.requiredCapabilities.filter((c:string)=>c!=='race.goals.v1')};
  await assert.rejects(f.x.createCoreLobby(core),/FIXTURE_CORE_LOBBY/);assert.equal(f.x.room.roomId,oldRoom.roomId);
  for(const i of [0,1])assert.equal((await f.commit(i,{eventId:randomUUID(),sequence:1,observations:[],terminal:'dead'})).receipt.status,'dead');
  const direct=async(actor:any,payload:any,r:any)=>{
   const value=JSON.parse((await f.x.fixture.call('/v1/rpc',{major:1,revision:lobbySchema.revision,serviceId:f.x.fixture.service.db.serviceId,serviceEpoch:f.x.fixture.service.db.epoch,sessionEpoch:actor.sessionEpoch,requestId:randomUUID(),roomId:r.roomId,runId:r.runId,runEpoch:r.runEpoch,configRevision:r.configRevision,expectedRevision:r.revision,...payload},actor.accessToken)).body);assert.equal(value.error,undefined,JSON.stringify(value));return value;
  };
  assert.equal((await direct(f.x.host,{roomGet:{}},oldRoom)).room.state,'ended');
  const remaining=await f.x.fixture.service.db.transaction(async tx=>(await tx.query('SELECT count(*) FROM memberships WHERE room_id=$1',[oldRoom.roomId])).rows[0].count);assert.equal(Number(remaining),0);
  assert.equal(f.x.room.state,'running');
  const created=await f.x.createCoreLobby(core);assert.equal(created.previousRoomId,oldRoom.roomId);assert.notEqual(created.roomId,oldRoom.roomId);assert.equal(f.x.room.config.mode,'core');
  const old=(await direct(f.x.host,{roomGet:{}},oldRoom)).room;assert.equal(old.config.mode,'race');assert.equal(old.state,'ended');
 }finally{await f.close();}
});
test('One-account concurrent Join creates one entry; repeat Join and browser control reuse it',async()=>{
 const f=await fixture();try{
  await f.x.end();const {race,...base}=f.x.room.config;
  await f.x.rpc(f.x.host,{roomCreate:{name:'Concurrent entry test',visibility:'public',config:{...base,mode:'core',requiredCapabilities:base.requiredCapabilities.filter((c:string)=>c!=='race.goals.v1')}}});
  const actors=await f.x.fixture.service.db.transaction(async tx=>[await f.x.fixture.service.auth.family(tx,f.x.desktop.accountId!,randomUUID(),'desktop'),await f.x.fixture.service.auth.family(tx,f.x.desktop.accountId!,randomUUID(),'desktop')]);
  const results=await Promise.allSettled(actors.map(a=>f.x.rpc(a,{roomJoin:{}})));assert.equal(results.filter(r=>r.status==='fulfilled').length,1);assert.equal(results.filter(r=>r.status==='rejected').length,1);
  const failed=results.find(r=>r.status==='rejected') as PromiseRejectedResult;assert.match(String(failed.reason),/ACCOUNT_ALREADY_JOINED/);
  const index=results.findIndex(r=>r.status==='fulfilled'),first=(results[index] as PromiseFulfilledResult<any>).value;const repeated=await f.x.rpc(actors[index]!,{roomJoin:{}});assert.equal(repeated.preparation.preparationId,first.preparation.preparationId);
  const count=await f.x.fixture.service.db.transaction(async tx=>(await tx.query('SELECT count(*) FROM memberships WHERE room_id=$1 AND account_id=$2 AND NOT roster_retired',[f.x.room.roomId,f.x.desktop.accountId])).rows[0].count);assert.equal(Number(count),1);
  assert.equal((await f.x.rpc(f.x.host,{roomGet:{}})).room.members.length,1);
 }finally{await f.close();}
});
