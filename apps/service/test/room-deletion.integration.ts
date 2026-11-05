// Actual isolated TLS/SQL service; explicit SQL quota/history seeds, no Noita claim.
import test from 'node:test';import assert from 'node:assert/strict';import {randomUUID} from 'node:crypto';import pg from 'pg';
import {startG3Fixture} from '../scripts/g3-loot01-fixture.js';import {schema} from '../../../packages/contracts/src/g2-next.js';
test('inactive lobby deletion retains history, fences stale access, pages lists and frees only active quota',async()=>{
 let now=Date.now();
 const f=await startG3Fixture({databaseUrl:'postgres://nt2_g1:local-fixture-only@127.0.0.1:54328/nt2_g1',b1Gold:true,rejoin:true,loot01Runtime:true,tls:true,live:true,now:()=>now});
 const db=f.service.db;
 try{
  assert.match(f.namespace,/^g3_rejoin_[a-f0-9]{32}$/);assert.deepEqual(await db.transaction(async tx=>(await tx.query('SELECT current_database() AS db,current_user AS owner,current_schema() AS schema')).rows[0]),{db:'nt2_g1',owner:'nt2_g1',schema:f.namespace});
  const host=await db.transaction(async tx=>{const id=await f.service.auth.mockRegistered(tx,'website','delete-host','Delete host');return f.service.auth.family(tx,id,'delete-host-browser','browser');});
  const other=await db.transaction(async tx=>{const id=await f.service.auth.mockRegistered(tx,'website','delete-other','Other');return f.service.auth.family(tx,id,'delete-other-browser','browser');});
  const desktop=await db.transaction(tx=>f.service.auth.family(tx,host.accountId!,'delete-host-desktop','desktop'));
  const envelope=(c:any,payload:any,r?:any,requestId=randomUUID())=>({major:1,revision:schema.revision,serviceId:db.serviceId,serviceEpoch:db.epoch,sessionEpoch:c.sessionEpoch,requestId,...(r?{roomId:r.roomId,expectedRevision:r.revision,configRevision:r.configRevision,runId:r.runId,runEpoch:r.runEpoch}:{}),...payload});
  const send=async(c:any,q:any)=>JSON.parse((await f.call('/v1/rpc',q,c.accessToken)).body);
  const call=async(c:any,p:any,r?:any)=>{now+=1100;return send(c,envelope(c,p,r));};
  const creation={roomCreate:{name:'Delete target',visibility:'public',config:{seed:1,mode:'core',requiredCapabilities:['core.seed']}}};
  now+=1100;const createRequest=envelope(host,creation),created=await send(host,createRequest);assert.equal(created.error,undefined);let room=created.room;assert.equal(room.state,'staging');
  assert.equal((await call(other,{roomDelete:{}},room)).error.code,'FORBIDDEN');assert.equal((await call(desktop,{roomDelete:{}},room)).error.code,'FORBIDDEN');assert.equal((await call(host,{roomDelete:{}},room)).error.code,'INVALID_STATE');
  const joined=await call(desktop,{roomJoin:{}},room);assert.equal(joined.error,undefined);assert.ok(joined.preparation);room=joined.room;
  await db.transaction(tx=>tx.query("UPDATE rooms SET state='waiting' WHERE id=$1",[room.roomId]));assert.equal((await call(host,{roomDelete:{}},room)).error.code,'INVALID_STATE','issued real preparation blocks deletion even with an inactive room state');
  const gameId=randomUUID();await db.transaction(async tx=>{await tx.query("UPDATE preparations SET state='cancelled' WHERE run_id=$1",[room.runId]);const family=(await tx.query('SELECT family_id FROM sessions WHERE id=$1',[desktop.sessionId])).rows[0].family_id;await tx.query("INSERT INTO sessions(id,family_id,kind,epoch,parent_id) VALUES($1,$2,'game',$3,$4)",[gameId,family,randomUUID(),desktop.sessionId]);await tx.query('INSERT INTO attachments(id,run_id,desktop_id,game_session_id,game_instance_id,service_epoch,run_epoch,config_revision,presence_until,resume_until) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)',[randomUUID(),room.runId,desktop.sessionId,gameId,'synthetic-delete-guard',db.epoch,room.runEpoch,room.configRevision,new Date(now+30000),new Date(now+60000)]);});
  assert.equal((await call(host,{roomDelete:{}},room)).error.code,'INVALID_STATE','unrevoked attachment blocks deletion');
  await db.transaction(async tx=>{await tx.query('UPDATE attachments SET revoked_at=$2 WHERE game_session_id=$1',[gameId,new Date(now)]);await tx.query('UPDATE sessions SET revoked_at=$2 WHERE id=$1',[gameId,new Date(now)]);});
  // Exact state/prepare guard probes use labelled owned SQL state, not simulated Noita observations.
  for(const state of ['running','suspended','preparing','loading']){await db.transaction(tx=>tx.query('UPDATE rooms SET state=$2 WHERE id=$1',[room.roomId,state]));assert.equal((await call(host,{roomDelete:{}},room)).error.code,'INVALID_STATE');}
  await db.transaction(tx=>tx.query("UPDATE rooms SET state='staging' WHERE id=$1",[room.roomId]));room=(await call(host,{cancelRun:{}},room)).room;assert.equal(room.state,'waiting');
  const oldId=room.roomId,runId=room.runId;
  await db.transaction(async tx=>{await tx.query("INSERT INTO g3_runs(id,room_id,epoch,config_revision,owner_id,state,mode,room_name,statistics_expires_at) VALUES($1,$2,$3,1,$4,'ended','core','Retained history',$5)",[runId,oldId,room.runEpoch,host.accountId,new Date(now+86400000)]);await tx.query('INSERT INTO g3_access(run_id,principal_id,display_name) VALUES($1,$2,$3)',[runId,host.accountId,'Delete host']);await tx.query("INSERT INTO g3_statistics(run_id,principal_id,display_name,version,kind,count,provenance,captured_at,expires_at) VALUES($1,$2,'Delete host',1,'player.death',1,'validated-client-report',$3,$4)",[runId,host.accountId,new Date(now),new Date(now+86400000)]);});
  assert.equal((await call(host,{roomDelete:{}},{...room,revision:room.revision-1})).error.code,'STALE_REVISION');
  now+=1100;const deletion=envelope(host,{roomDelete:{}},room),deleted=await send(host,deletion);assert.equal(deleted.error,undefined);assert.equal(deleted.deletedRoomId,oldId);
  assert.deepEqual(await call(host,{roomDelete:{}},room).then(v=>v.deletedRoomId),oldId);assert.equal((await send(host,deletion)).deletedRoomId,oldId);
  assert.equal((await call(other,{roomDelete:{}},room)).error.code,'FORBIDDEN');assert.equal((await send(host,{...deletion,expectedRevision:999})).error.code,'CONFLICT');
  assert.equal(await db.transaction(async tx=>(await tx.query('SELECT count(*)::int AS n FROM memberships WHERE room_id=$1',[oldId])).rows[0].n),0);
  for(const [who,p]of [[host,{roomGet:{}}],[host,{roomPreview:{}}],[host,{configure:creation.roomCreate.config}],[desktop,{roomJoin:{}}],[desktop,{prepare:{}}]])assert.equal((await call(who,p,room)).error.code,'NOT_FOUND');
  assert.equal((await send(host,createRequest)).error.code,'NOT_FOUND','stale create cannot resurrect a deleted room');
  const history=JSON.parse((await f.call('/v3/history/rpc',{major:1,revision:'g3-history-draft-1',serviceId:db.serviceId,serviceEpoch:db.epoch,sessionEpoch:host.sessionEpoch,requestId:randomUUID(),discover:{}},host.accessToken)).body);assert.equal(history.error,undefined);assert.equal(history.runs[0].scope.runId,runId);assert.equal(history.runs[0].collectionState,'available');
  assert.equal(await db.transaction(async tx=>(await tx.query('SELECT count(*)::int AS n FROM g3_statistics WHERE run_id=$1',[runId])).rows[0].n),1);
  // Seed32inactive rows to exercise exact list paging and concurrent quota commands without generating32worlds.
  await db.transaction(async tx=>{for(let i=0;i<32;i++)await tx.query("INSERT INTO rooms(id,name,host_account_id,state,config,visibility) VALUES($1,$2,$3,'waiting',$4,'public')",[randomUUID(),'Quota '+i,host.accountId,creation.roomCreate.config]);});
  assert.equal((await call(host,creation)).error.code,'CAPACITY');
  let after='',ids:string[]=[];do{const result=await call(host,{roomList:{limit:7,afterRoomId:after}});assert.equal(result.error,undefined);ids.push(...result.rooms.map((r:any)=>r.roomId));after=result.nextRoomId??'';}while(after);assert.equal(ids.length,32);assert.equal(new Set(ids).size,32);assert.equal(ids.includes(oldId),false);
  const quota=(await call(host,{roomGet:{}},{...room,roomId:ids[0]})).room;
  now+=1100;const race=await Promise.all([send(host,envelope(host,creation)),send(host,envelope(host,{roomDelete:{}},quota)),send(host,envelope(host,creation))]);assert.equal(race[1].deletedRoomId,quota.roomId);const admitted=race.filter(r=>r.room).length;assert.ok(admitted<=1);assert.equal(race.filter(r=>r.error?.code==='CAPACITY').length,2-admitted);if(!admitted)assert.equal((await call(host,creation)).error,undefined);
  assert.equal(await db.transaction(async tx=>(await tx.query('SELECT count(*)::int AS n FROM rooms WHERE host_account_id=$1 AND deleted_at IS NULL',[host.accountId])).rows[0].n),32);
 }finally{const namespace=f.namespace;await f.close();const client=new pg.Client({connectionString:'postgres://nt2_g1:local-fixture-only@127.0.0.1:54328/nt2_g1'});try{await client.connect();assert.equal((await client.query('SELECT 1 FROM pg_namespace WHERE nspname=$1',[namespace])).rowCount,0);console.log(JSON.stringify({roomDeletionSchemaAbsent:namespace}));}finally{await client.end();}}
});
