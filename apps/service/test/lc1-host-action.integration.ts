// Actual canonical SQL/HTTP paths, no Client/native/game composition.
import test from 'node:test';import assert from 'node:assert/strict';import {randomUUID} from 'node:crypto';
import {startLifecycleLc1Fixture} from '../scripts/lifecycle-lc1-fixture.js';
import {livePeer} from '../scripts/g3-live-fixture.js';
test('stale moderation rolls back rejoin prework then applies exact frozen ban; unban/nextRun/exclusion remain scoped',async()=>{
 const x=await startLifecycleLc1Fixture(),sockets:Awaited<ReturnType<typeof livePeer>>[]=[];const db=x.fixture.service.db;let actionCalls=0,forcedStales=0,prework=0;
 try{
  const identity=await db.transaction(async tx=>(await tx.query('SELECT current_database() AS database,current_user AS owner,current_schema() AS schema')).rows[0]);assert.deepEqual(identity,{database:'nt2_g1',owner:'nt2_g1',schema:x.fixture.namespace});
  const peer=await x.bootstrapPeer();const qb=x.request(peer.desktop,1,'peer',peer.observation);qb.reconcile!.association=peer.association;qb.reconcile!.proof=peer.proof;
  const qa=x.request();await x.send(x.desktop,qa);await x.send(peer.desktop,qb);await x.start();
  qa.reconcile!.observation={...x.observation,protected:false,observationRevision:2};qb.reconcile!.observation={...peer.observation,protected:false,observationRevision:2};
  const a=await x.send(x.desktop,qa),b=await x.send(peer.desktop,qb);for(const v of [a,b]){assert.equal(v.canonicalAdmission.decision.playing,true);sockets.push(await livePeer(x.fixture as any,{games:[v.credentials],scopes:[v.canonicalAdmission.scope]} as any,0));}
  const recovery=(x.fixture.service.rooms as any).recovery,retire=recovery.retire.bind(recovery);recovery.retire=async(...args:any[])=>{prework++;return retire(...args);};
  const state=()=>db.transaction(async tx=>({participants:(await tx.query('SELECT id,state,terminal,reason,attachment_id,generation FROM rejoin_participants WHERE run_id=$1 ORDER BY id',[x.room.runId])).rows,worlds:(await tx.query('SELECT world_id,terminal_reason,generation FROM world_associations WHERE run_id=$1 ORDER BY world_id',[x.room.runId])).rows,bans:(await tx.query('SELECT account_id FROM room_bans WHERE room_id=$1 ORDER BY account_id',[x.room.roomId])).rows}));
  const projection=()=>[...x.fixture.service.live!.projection.rooms.values()].flatMap(r=>[...r.members.values()].map(m=>({digest:m.digest,generation:m.generation,lastPose:m.lastPose}))).sort((a,b)=>a.digest.localeCompare(b.digest));
  const original=x.fixture.call.bind(x.fixture);let arm:'ban'|'unban'|'configure'|undefined='ban';
  x.fixture.call=async(...args:Parameters<typeof original>)=>{const body=args[1] as any;const action=body?.roomModerate?.action??(body?.configure?'configure':undefined);if(action)actionCalls++;
   if(arm&&action===arm){arm=undefined;const before=await state(),members=projection(),count=prework;await db.transaction(tx=>tx.query('UPDATE rooms SET revision=revision+1 WHERE id=$1',[x.room.roomId]));const response=await original(...args);assert.equal(JSON.parse(response.body).error?.code,'STALE_REVISION');forcedStales++;assert.deepEqual(projection(),members);assert.deepEqual(await state(),before);assert.ok(sockets.every(s=>s.socket.readyState===1)||action!=='ban');if(action==='ban')assert.ok(prework>count,'Rejoin retirement prework must have run before stale guard');return response;
   }return original(...args);
  };
  await x.moderate('consumer','ban');assert.equal(forcedStales,1);let facts=await state();assert.deepEqual(facts.bans.map(r=>r.account_id),[x.desktop.accountId]);assert.equal(facts.participants.find(r=>r.attachment_id===a.credentials.attachmentId)?.reason,'host_ban');assert.equal(facts.participants.find(r=>r.attachment_id===a.credentials.attachmentId)?.terminal,true);const held=await x.send(x.desktop,{...qa,requestId:randomUUID()});assert.equal(held.world.state,'terminal');assert.equal(held.world.reason,'host_ban');assert.equal((await x.state()).worlds.find(w=>w.world_id===peer.association.worldId)?.terminal_reason,null);
  arm='unban';await x.moderate('consumer','unban');facts=await state();assert.equal(facts.bans.length,0);assert.equal(facts.participants.find(r=>r.attachment_id===a.credentials.attachmentId)?.reason,'host_ban');assert.equal(facts.participants.find(r=>r.attachment_id===a.credentials.attachmentId)?.terminal,true);
  await x.end();const old=x.room.runId;arm='configure';await x.nextRun();assert.notEqual(x.room.runId,old);assert.equal(x.room.state,'staging');
  // New run explicit exclusion still uses exact selected report; Start has no retry.
  const prepare=async(c:any,base:any)=>{await x.rpc(x.host,{roomGet:{}});await x.rpc(c,{roomJoin:{}});const p=await x.prepareWorld(c);const q=x.request(c,1,randomUUID(),{...base,worldToken:p.association.worldId!,gameInstanceId:randomUUID(),observationRevision:1});q.reconcile!.association=p.association;q.reconcile!.proof=p.proof;const value=await x.send(c,q);assert.equal(value.error,undefined);return {...p,q};};
  const ca=await prepare(x.desktop,x.observation),pb=await prepare(peer.desktop,peer.observation);
  await x.start({selectedWorldIds:[pb.association.worldId!],excludedDesktopIds:[x.desktop.sessionId!]});const excluded=await x.send(x.desktop,{...ca.q,requestId:randomUUID()});assert.equal(excluded.world.state,'terminal');assert.equal(excluded.world.reason,'not_selected_by_start');const final=await x.state();assert.equal(final.worlds.find(w=>w.world_id===ca.association.worldId)?.terminal_reason,'not_selected_by_start');assert.equal(final.worlds.find(w=>w.world_id===pb.association.worldId)?.terminal_reason,null);
  assert.equal(forcedStales,3);assert.equal(actionCalls,6);console.log(JSON.stringify({hostActionProof:{schema:x.fixture.namespace,identity,forcedStales,actionCalls,rejoinPreworkObserved:prework>0,rollbackPreserved:true,banUnbanTerminalPreserved:true,nextRun:true,explicitExclusion:true}}));
 }finally{for(const s of sockets)await s.close();await x.close();}
});
