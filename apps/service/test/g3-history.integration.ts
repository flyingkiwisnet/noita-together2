import test from 'node:test';import assert from 'node:assert/strict';import {randomUUID} from 'node:crypto';
import {startG3Fixture,provisionG3Fixture} from '../scripts/g3-fixture.js';
import {encode,decode} from '../../../packages/contracts/src/g3-history.js';
// @ts-expect-error Browser module exercised through an injected real HTTPS transport.
import {StatisticsClient} from '../../web/g3/client.js';
import {G3Authority} from '../src/g3/authority.js';
import type {Credentials} from '../../../packages/contracts/g2-next/g2-next.js';
test('one-issuer historical statistics discovery pages frozen authorized runs, preserves privacy and retention across restart',async()=>{
 let now=Date.now();const f=await startG3Fixture({tls:true,now:()=>now});try{
  let state!:Awaited<ReturnType<typeof provisionG3Fixture>>;const scopes:typeof state.scopes=[];
  for(let i=0;i<3;i++){
   now+=1000;state=await provisionG3Fixture(f,{roomName:i===0?'<svg onload=alert(1)> 雪':'History '+i});await state.start();await state.playing();scopes.push(state.scopes[0]!);
   const game=state.games[0]!;const r=JSON.parse((await f.call('/v3/rpc',{major:1,revision:'g3-draft-1',serviceId:f.service.db.serviceId,serviceEpoch:f.service.db.epoch,sessionEpoch:game.sessionEpoch,requestId:randomUUID(),scope:scopes.at(-1),gameEvent:{eventId:randomUUID(),catalogId:'player.death',count:1}},game.accessToken)).body);assert.equal(r.error,undefined);
   await state.rpc(state.host,{endRun:{}});
  }
  const envelope=(c:Credentials,discover={})=>({major:1,revision:'g3-history-draft-1',serviceId:f.service.db.serviceId,serviceEpoch:f.service.db.epoch,sessionEpoch:c.sessionEpoch,requestId:randomUUID(),discover});
  const call=async(c:Credentials,discover={})=>{now+=100;const r=await f.call('/v3/history/rpc',envelope(c,discover),c.accessToken);assert.equal(r.status,200,r.body);return JSON.parse(r.body);};
  const browser:any={credentials:state.host,info:{serviceId:f.service.db.serviceId},generation:1,now:()=>now,uuid:randomUUID,exclusive:(fn:()=>unknown)=>fn()};
  const transport=async(path:string,body?:unknown,token?:string)=>JSON.parse((await f.call(path,body,token)).body);
  const client=new StatisticsClient(browser,transport),first=await client.discover('0',undefined,1);assert.equal(first.runs.length,1);assert.match(first.runs[0].roomName,/<svg/);assert.equal(first.runs[0].collectionState,'available');assert.equal(first.runs[0].canDelete,true);
  const duplicate=await call(state.host,{afterSequence:'0',throughSequence:first.highSequence,limit:1});assert.deepEqual(duplicate.runs,first.runs);
  now+=1000;state=await provisionG3Fixture(f,{roomName:'New after cursor'});await state.start();await state.playing();assert.equal((await call(state.games[0]!)).error.code,'FORBIDDEN');await state.rpc(state.host,{endRun:{}});browser.credentials=state.host;
  const second=await client.discover(first.nextSequence,first.highSequence,1),third=await client.discover(second.nextSequence,first.highSequence,1);assert.equal(third.nextSequence,first.highSequence);assert.equal(new Set([...first.runs,...second.runs,...third.runs].map(r=>r.scope.runId)).size,3);
  assert.deepEqual((await client.discover(third.nextSequence,first.highSequence,1)).runs,[]);
  const fresh=await client.discover();assert.equal(fresh.runs.length,4);assert.equal(fresh.runs.at(-1).collectionState,'empty');
  assert.equal((await call(state.host,{limit:33})).error.code,'INVALID_MESSAGE');assert.equal((await call(state.host,{afterSequence:'-1'})).error.code,'INVALID_MESSAGE');assert.equal((await call(state.host,{throughSequence:'9223372036854775807'})).error.code,'INVALID_MESSAGE');
  const binary=await f.service.app.inject({method:'POST',url:'/v3/history/rpc',headers:{host:new URL(f.origin).host,authorization:'Bearer '+state.desktops[1]!.accessToken,'content-type':'application/x-protobuf'},payload:Buffer.from(encode('Request',envelope(state.desktops[1]!,{limit:2})))});assert.equal(binary.statusCode,200,binary.body);const parsed:any=decode('Response',binary.rawPayload);assert.equal(parsed.runs.length,2);assert.equal(parsed.runs[0].canDelete??false,false);
  const stranger=JSON.parse((await f.call('/v1/mock/browser-login',{subject:'twitch-local-alex',deviceId:'history-outsider',requestId:randomUUID()})).body).credentials;assert.ok(stranger);
  assert.deepEqual((await call(stranger)).runs,[]);
  // Disposable SQL history shells exercise the32 boundary without claiming33 games were played.
  const privateScopes:typeof state.scopes=[];await f.service.db.transaction(async tx=>{for(let i=0;i<33;i++){const id=randomUUID(),roomId=randomUUID(),epoch=randomUUID();privateScopes.push({runId:id,roomId,runEpoch:epoch,configRevision:1});await tx.query("INSERT INTO g3_runs(id,room_id,epoch,config_revision,owner_id,state,mode,room_name,created_at) VALUES($1,$2,$3,1,$4,'ended','core','Private history shell',$5)",[id,roomId,epoch,state.host.accountId,new Date(now)]);await tx.query('INSERT INTO g3_access(run_id,principal_id,display_name) VALUES($1,$2,$3)',[id,state.host.accountId,'Alex']);}});
  const boundary=await call(state.host,{limit:32});assert.equal(boundary.runs.length,32);assert.ok(BigInt(boundary.nextSequence)<BigInt(boundary.highSequence));const tail=await call(state.host,{afterSequence:boundary.nextSequence,throughSequence:boundary.highSequence,limit:32});assert.equal(tail.runs.length,5);assert.equal(tail.nextSequence,tail.highSequence);assert.equal(new Set([...boundary.runs,...tail.runs].map(r=>r.scope.runId)).size,37);assert.equal((await call(state.desktops[1]!)).runs.length,4);
  const detail=async(c:Credentials,scope:unknown,payload:unknown)=>JSON.parse((await f.call('/v3/rpc',{major:1,revision:'g3-draft-1',serviceId:f.service.db.serviceId,serviceEpoch:f.service.db.epoch,sessionEpoch:c.sessionEpoch,requestId:randomUUID(),scope,...payload as object},c.accessToken)).body);
  assert.equal((await detail(stranger,scopes[0],{statisticsList:{}})).error.code,'NOT_FOUND');assert.equal((await detail(state.host,{...scopes[0],runId:randomUUID()},{statisticsList:{}})).error.code,'NOT_FOUND');
  assert.equal((await detail(state.desktops[1]!,privateScopes[0],{statisticsList:{}})).error.code,'NOT_FOUND');assert.deepEqual((await detail(state.host,privateScopes[0],{statisticsList:{}})).statistics,[]);
  assert.equal((await f.call('/v3/history/rpc',{...envelope(state.host),principalId:state.desktops[1]!.accountId},state.host.accessToken)).status,400);
  assert.equal((await detail(state.desktops[1]!,scopes[0],{statisticsDelete:{}})).error.code,'FORBIDDEN');assert.equal((await detail(state.host,scopes[0],{statisticsAggregate:{}})).aggregates[0].count,'1');
  assert.equal((await detail(state.host,scopes[0],{statisticsDelete:{}})).error,undefined);assert.equal((await call(state.host)).runs[0].collectionState,'deleted');assert.deepEqual((await detail(state.host,scopes[0],{statisticsList:{}})).statistics,[]);
  await f.restart();assert.equal((await call(state.host)).runs.length,32);
  now+=31*86400000;
  const renewed=JSON.parse((await f.call('/v1/mock/browser-login',{subject:'website-local-alex',deviceId:'history-return',requestId:randomUUID()})).body).credentials;assert.ok(renewed);
  const expired=await call(renewed);assert.equal(expired.runs[0].collectionState,'deleted');assert.equal(expired.runs[1].collectionState,'expired');assert.equal(expired.runs[3].collectionState,'empty');
  assert.deepEqual((await detail(renewed,scopes[1],{statisticsList:{}})).statistics,[]);
  await f.service.db.transaction(tx=>new G3Authority(()=>f.service.db.epoch,()=>now).maintain(tx)); // The production minute maintenance callback, without a real-time wait.
  assert.equal((await f.service.db.transaction(tx=>tx.query('SELECT count(*)::int AS count FROM g3_statistics'))).rows[0].count,0);
  await f.restart();assert.deepEqual((await call(renewed)).runs,expired.runs);
 }finally{await f.close();}
});
