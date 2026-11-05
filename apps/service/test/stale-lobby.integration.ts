import test from 'node:test';
import assert from 'node:assert/strict';
import {randomUUID} from 'node:crypto';
import {startLifecycleLc1Fixture} from '../scripts/lifecycle-lc1-fixture.js';

test('same-device sign-in retires existing stale roster rows without deleting saved-world consent',async()=>{
 const x=await startLifecycleLc1Fixture();try{
  const db=x.fixture.service.db,old=x.desktop,roomId=x.room.roomId;
  await x.rpc(x.host,{roomPolicy:{capacity:1}});
  // The real fixture sign-in uses production RejoinAuth, not a substituted auth.
  await x.signIn();
  const raw=(await db.pool.query('SELECT * FROM memberships WHERE desktop_id=$1',[old.sessionId])).rows[0];
  assert.equal(raw.roster_retired,false); // existing row from before cleanup
  const firstPreview=await x.rpc(x.desktop,{roomPreview:{}});
  assert.equal(firstPreview.admission.playerCount,0);assert.equal(firstPreview.room.revision,firstPreview.admission.revision);
  const view=await x.rpc(x.host,{roomGet:{}});
  assert.equal(view.room.members.length,0);
  const retained=(await db.pool.query('SELECT * FROM memberships WHERE desktop_id=$1',[old.sessionId])).rows[0];
  assert.equal(retained.roster_retired,true);assert.equal(retained.ready_config_revision,raw.ready_config_revision);
  const preview=await x.rpc(x.desktop,{roomPreview:{}});assert.equal(preview.admission.playerCount,0);assert.equal(preview.admission.joinAllowed,true);
  // Recovery without a new Join must still honor the existing saved-world proof.
  const recovered=await x.send(x.desktop,x.request(x.desktop,2,'cold-pre-start'));
  assert.equal(recovered.error,undefined);assert.equal(recovered.world.state,'waiting');
  assert.equal((await x.rpc(x.host,{roomGet:{}})).room.members.length,1);
  await assert.rejects(x.rpc(old,{presence:{present:true}}),/UNAUTHENTICATED/);
  for(let i=0;i<3;i++){
   const previous=x.desktop;await x.signIn();
   assert.equal((await x.rpc(x.desktop,{roomPreview:{}})).admission.playerCount,0);
   const joined=await x.rpc(x.desktop,{roomJoin:{}});assert.equal(joined.room.members.length,1);
   assert.equal(joined.room.members[0].desktopId,x.desktop.sessionId);
   await assert.rejects(x.rpc(previous,{roomJoin:{}}),/UNAUTHENTICATED/);
  }
  const fresh=await x.prepareWorld(),q=x.request(x.desktop,1,'fresh-staging',{...x.observation,worldToken:fresh.association.worldId!});
  q.reconcile!.association=fresh.association;q.reconcile!.proof=fresh.proof;
  assert.equal((await x.send(x.desktop,q)).world.state,'waiting');
  await x.rpc(x.host,{roomGet:{}});
  const reports=await x.rpc(x.host,{inspectionList:{limit:1}}),report=reports.inspections[0];
  await assert.rejects(x.rpc(x.host,{finalStart:{participants:[{desktopId:report.desktopId,attachmentId:report.attachmentId,worldToken:report.worldToken,observationRevision:report.observationRevision,contentDigest:report.contentDigest}],excludedDesktopIds:[old.sessionId]}}),/ROSTER_MISMATCH/);
  await x.start();assert.equal(x.room.state,'running');
  const start=(await db.pool.query('SELECT roster,excluded_roster FROM runs WHERE id=$1',[x.room.runId])).rows[0];
  assert.deepEqual(start.roster,[x.desktop.sessionId]);assert.deepEqual(start.excluded_roster,[]);
  console.log(JSON.stringify({case:'same-device-existing-stale',repeatedLoginJoin:3,capacity:1,preStartProofRecovery:true,lateOldRequestsRejected:true,actualNoita:false}));
 }finally{await x.close();}
});

test('distinct devices, ordinary absence and another lobby are preserved',async()=>{
 const x=await startLifecycleLc1Fixture();try{
  const db=x.fixture.service.db,old=x.desktop;
  const other=await db.transaction(tx=>x.fixture.service.auth.family(tx,old.accountId!,randomUUID(),'desktop'));
  await x.rpc(other,{roomJoin:{}});assert.equal(x.room.members.length,2);
  await x.rpc(old,{presence:{present:false}});
  x.advance(31000);await x.rpc(x.host,{roomGet:{}});
  assert.equal(x.room.members.length,2);assert.equal((await db.pool.query('SELECT count(*) FROM memberships WHERE roster_retired')).rows[0].count,'0');
  await x.rpc(other,{presence:{present:true}});
  const roomId=x.room.roomId,oldRevision=x.room.revision,config=x.room.config;
  const created=await x.rpc(x.host,{roomCreate:{name:'Separate lobby',config}}),second=created.room;
  const sameDevice=await x.signIn();
  // A read of a different room cannot retire/change the original room.
  await x.rpc(x.host,{roomGet:{}});
  assert.equal((await db.pool.query('SELECT revision FROM rooms WHERE id=$1',[roomId])).rows[0].revision,oldRevision);
  assert.equal((await db.pool.query('SELECT roster_retired FROM memberships WHERE desktop_id=$1',[old.sessionId])).rows[0].roster_retired,false);
  await x.rpc(sameDevice,{roomJoin:{}});assert.equal(x.room.roomId,second.roomId);
  // Explicitly reading the original room retires only the revoked device row.
  const original=await x.rpc(x.host,{roomId,roomGet:{}});
  assert.deepEqual(original.room.members.map((m:any)=>m.desktopId),[other.sessionId]);
  assert.equal(original.room.state,'staging');
  console.log(JSON.stringify({case:'device-and-room-isolation',sameAccountDifferentDevice:true,ordinaryTimeoutRetained:true,otherLobbyUnchangedUntilRead:true,actualNoita:false}));
 }finally{await x.close();}
});

test('private saved-world consent remains discoverable without auto-admitting a new desktop',async()=>{
 const x=await startLifecycleLc1Fixture();try{
  const old=x.desktop;
  assert.equal((await x.send(old,x.request())).world.state,'waiting');
  await x.rpc(x.host,{roomGet:{}});
  await x.rpc(x.host,{roomAccess:{visibility:'private',admittedAccountIds:[]}});
  await x.signIn();await x.rpc(x.host,{roomGet:{}});
  assert.equal(x.room.members.length,0);
  // Historical consent still permits discovery/read, but never a fresh Join.
  const list=await x.rpc(x.desktop,{roomList:{limit:32}});assert.equal(list.rooms.some((r:any)=>r.roomId===x.room.roomId),true);
  await x.rpc(x.desktop,{roomGet:{}});
  await assert.rejects(x.rpc(x.desktop,{roomJoin:{}}),/NOT_FOUND/);
  const q=x.request(x.desktop,2,'private-cold',{...x.observation,observationRevision:2});
  const recovered=await x.send(x.desktop,q);assert.equal(recovered.error,undefined);assert.equal(recovered.world.state,'waiting');
  const room=(await x.rpc(x.host,{roomGet:{}})).room;assert.equal(room.members.length,1);assert.equal(room.members[0].desktopId,x.desktop.sessionId);
  console.log(JSON.stringify({case:'private-cold-recovery',historicalConsentDiscoverable:true,newDesktopNotAutoAdmitted:true,proofBoundRecovery:true,actualNoita:false}));
 }finally{await x.close();}
});

test('running frozen participant and excluded world survive roster retirement with paused and cold recovery',async()=>{
 const x=await startLifecycleLc1Fixture();try{
  const peer=await x.bootstrapPeer(),old=x.desktop;
  assert.equal((await x.send(old,x.request())).error,undefined);
  const pq=x.request(peer.desktop,1,'excluded-peer',peer.observation);pq.reconcile!.association=peer.association;pq.reconcile!.proof=peer.proof;
  assert.equal((await x.send(peer.desktop,pq)).error,undefined);
  await x.start({selectedWorldIds:[x.association.worldId!],excludedDesktopIds:[peer.desktop.sessionId!]});
  const db=x.fixture.service.db,run=x.room.runId;
  const frozen=async()=>(await db.pool.query('SELECT roster,excluded_roster FROM runs WHERE id=$1',[run])).rows[0];
  const before=await frozen();
  // Healthy running owner is deliberately retained by RejoinAuth.
  await x.signIn();await x.rpc(x.host,{roomGet:{}});
  assert.equal(x.room.members.some((m:any)=>m.desktopId===old.sessionId),true);
  assert.equal((await db.pool.query('SELECT revoked_at FROM sessions WHERE id=$1',[old.sessionId])).rows[0].revoked_at,null);
  // A pause/outage alone never removes it, and the retained owner can restore.
  x.advance(31000);await x.rpc(x.host,{roomGet:{}});
  assert.equal(x.room.members.some((m:any)=>m.desktopId===old.sessionId),true);
  const paused=await x.send(old,x.request(old,2,'paused-owner',{...x.observation,observationRevision:2}));
  assert.equal(paused.error,undefined);assert.equal(paused.world.state,'playing');
  x.advance(31000);await x.signIn();await x.rpc(x.host,{roomGet:{}});
  assert.equal(x.room.members.some((m:any)=>m.desktopId===old.sessionId),false);
  assert.deepEqual(await frozen(),before);
  const p=(await db.pool.query('SELECT * FROM rejoin_participants WHERE run_id=$1',[run])).rows[0];assert.equal(p.terminal,false);
  const cold=await x.send(x.desktop,x.request(x.desktop,3,'cold-running',{...x.observation,observationRevision:3}));
  assert.equal(cold.error,undefined);assert.equal(cold.world.state,'playing');
  await x.rpc(x.host,{roomGet:{}});assert.equal(x.room.state,'running');
  assert.equal(x.room.members.filter((m:any)=>m.accountId===old.accountId).length,1);
  const after=(await db.pool.query('SELECT * FROM rejoin_participants WHERE id=$1',[p.id])).rows[0];
  assert.equal(after.id,p.id);assert.equal(after.world_token,p.world_token);assert.equal(after.original_start_id,p.original_start_id);assert.equal(after.terminal,false);
  assert.deepEqual(await frozen(),before);
  await assert.rejects(x.rpc(old,{presence:{present:true}}),/UNAUTHENTICATED/);
  const excluded=await x.send(peer.desktop,{...pq,requestId:randomUUID(),reconcile:{...pq.reconcile!,attempt:2,connectionId:'late-excluded'}});
  assert.equal(excluded.error,undefined);assert.equal(excluded.world.state,'terminal');assert.equal(excluded.world.reason,'not_selected_by_start');
  console.log(JSON.stringify({case:'running-recovery',healthyOwnerRetained:true,pausedOwnerRestored:true,coldSameDeviceRestored:true,frozenRosterPreserved:true,excludedNotAdmitted:true,actualNoita:false}));
 }finally{await x.close();}
});
