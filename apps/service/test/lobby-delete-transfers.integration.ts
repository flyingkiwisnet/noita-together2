// Actual authenticated TLS/SQL/LC1 routes. Wallet/engine outcomes are injected.
import test from 'node:test';import assert from 'node:assert/strict';import {randomUUID} from 'node:crypto';
import {b1Fixture} from './b1-fixture-support.js';import {migrateGameplay} from '../src/g3/database.js';import {PolicyError} from '../src/g1/security.js';
type Fixture=Awaited<ReturnType<typeof b1Fixture>>;
async function transfers(f:Fixture,amount=100){const scope=structuredClone(f.scope(0)),original={goldIntent:{operationId:randomUUID(),direction:'deposit',amount}};
 const completed=(await f.ok(0,original)).operation,receipt=f.receipt(completed,'escrowed');await f.ok(0,receipt);
 const uncertain=(await f.ok(0,{goldIntent:{operationId:randomUUID(),direction:'deposit',amount:3}})).operation;await f.ok(0,f.receipt(uncertain,'uncertain'));
 const queued=(await f.ok(0,{goldIntent:{operationId:randomUUID(),direction:'withdraw',amount:10}})).operation;
 return {scope,original,completed,receipt,uncertain,queued};}
async function enter(f:Fixture,freshPreparation=false){const {x}=f;await x.rpc(x.desktop,{roomJoin:{}});const q=x.request();if(freshPreparation){const p=await x.prepareWorld();q.reconcile!.association=p.association;q.reconcile!.proof=p.proof;q.reconcile!.observation!.worldToken=p.association.worldId!;}
 const waiting=await x.send(x.desktop,q);assert.equal(waiting.error,undefined);await x.start({selectedWorldIds:[q.reconcile!.association!.worldId!],excludedDesktopIds:[]});q.reconcile!.observation!.protected=false;q.reconcile!.observation!.observationRevision=2;f.values[0]=await x.send(x.desktop,q);assert.equal(f.values[0].world.mayPlay,true);}
async function counts(f:Fixture,roomId:string){return f.x.fixture.service.db.transaction(async tx=>{const row=(await tx.query(`SELECT
 (SELECT count(*)::int FROM g3_operations WHERE run_id IN (SELECT id FROM g3_runs WHERE room_id=$1)) AS operations,
 (SELECT count(*)::int FROM g3_receipts WHERE run_id IN (SELECT id FROM g3_runs WHERE room_id=$1)) AS receipts,
 (SELECT count(*)::int FROM g3_ledger WHERE run_id IN (SELECT id FROM g3_runs WHERE room_id=$1)) AS ledger,
 (SELECT count(*)::int FROM g3_balances WHERE run_id IN (SELECT id FROM g3_runs WHERE room_id=$1)) AS balances,
 (SELECT count(*)::int FROM g3_outbox WHERE run_id IN (SELECT id FROM g3_runs WHERE room_id=$1) AND kind LIKE 'gold.%') AS outbox,
 (SELECT count(*)::int FROM g3_statistics WHERE run_id IN (SELECT id FROM g3_runs WHERE room_id=$1)) AS statistics,
 (SELECT count(*)::int FROM g3_runs WHERE room_id=$1) AS runs,
 (SELECT count(*)::int FROM g3_access WHERE run_id IN (SELECT id FROM g3_runs WHERE room_id=$1)) AS access`,[roomId])).rows[0];return row;});}
test('Delete lobby removes all run transfers, preserves other lobby/stats, and proves terminal status after cold sign-in',async()=>{
 const f=await b1Fixture();try{
  const first=await transfers(f),deletedRoom=first.scope.roomId!,initial=await counts(f,deletedRoom);
  await assert.rejects(f.x.deleteLobby(),/INVALID_STATE/);assert.deepEqual(await counts(f,deletedRoom),initial);
  await f.x.end();const endedStatus=await f.raw(f.x.desktop,f.body(0,{goldStatus:{operationId:first.queued.operationId}},first.scope,f.x.desktop));assert.equal(endedStatus.operation.state,'reserved');assert.equal(endedStatus.operation.effectAllowed,false);assert.deepEqual(await counts(f,deletedRoom),initial);
  const absent=await f.raw(f.x.desktop,f.body(0,{goldStatus:{operationId:randomUUID()}},first.scope,f.x.desktop));assert.equal(absent.error.code,'NOT_FOUND');assert.deepEqual(await counts(f,deletedRoom),initial);
  await f.x.nextRun();await enter(f,true);const second=await transfers(f,50);await f.x.end();
  const current=await f.x.signIn();assert.equal(f.x.desktop.accessToken,current.accessToken);
  // Emulate Client refresh-secret rotation unknown to the fixture, then token
  // expiry. New-lobby setup must not reuse that stale token/refresh secret.
  await f.x.rpc(undefined,{refresh:{secret:current.refreshSecret,deviceId:current.deviceId}});f.x.advance(920001);
  await f.x.createLobby();assert.notEqual(f.x.desktop.accessToken,current.accessToken);assert.equal(f.x.desktop.accountId,current.accountId);await enter(f);const other=await transfers(f,30),otherBefore=await counts(f,other.scope.roomId!);
  const before=await counts(f,deletedRoom);assert.equal(before.runs,2);assert.equal(before.operations,6);assert.equal(before.statistics,2);
  const probeBefore=await f.x.lobbyTransferSummary(deletedRoom);assert.equal(probeBefore.operations,6);assert.equal(probeBefore.ledgerEntries,4);assert.equal(probeBefore.statistics,2);assert.equal(probeBefore.deleted,false);
  assert.deepEqual(await f.x.forbiddenLobbyDelete(deletedRoom),{accepted:false,rejection:'FORBIDDEN'});assert.deepEqual(await f.x.lobbyTransferSummary(deletedRoom),probeBefore);
  await assert.rejects(f.x.lobbyTransferSummary(randomUUID()),/FIXTURE_ROOM/);
  const foreignDesktop=await f.x.signIn(1);await assert.rejects(f.x.rpc(foreignDesktop,{roomId:deletedRoom,roomDelete:{}}),/FORBIDDEN/);assert.deepEqual(await counts(f,deletedRoom),before);
  await assert.rejects(f.x.fixture.service.db.transaction(tx=>tx.query('DELETE FROM g3_ledger WHERE run_id=$1',[other.scope.runId])));
  await assert.rejects(f.x.fixture.service.db.transaction(tx=>tx.query('SELECT g3_delete_room_transfers($1)',[other.scope.roomId])));
  const deleted=await f.x.deleteLobby(deletedRoom);assert.equal(deleted.deletedRoomId,deletedRoom);
  const after=await counts(f,deletedRoom);assert.deepEqual(after,{...before,operations:0,receipts:0,ledger:0,balances:0,outbox:0});assert.deepEqual(await counts(f,other.scope.roomId!),otherBefore);
  const probeAfter=await f.x.lobbyTransferSummary(deletedRoom);assert.equal(probeAfter.deleted,true);assert.equal(probeAfter.operations,0);assert.equal(probeAfter.ledgerEntries,0);assert.equal(probeAfter.ledgerDelta,'0');assert.equal(probeAfter.balanceTotal,'0');assert.equal(probeAfter.goldOutbox,0);assert.equal(probeAfter.statistics,probeBefore.statistics);assert.equal(probeAfter.statisticsTotal,probeBefore.statisticsTotal);
  assert.equal((await f.ok(0,{goldStatus:{operationId:other.queued.operationId}})).operation.effectAllowed,true);
  assert.equal((await f.x.deleteLobby(deletedRoom)).deletedRoomId,deletedRoom);
  const cold=await f.x.signIn();
  for(const original of [first,second]){
   for(const payload of [{goldStatus:{operationId:original.completed.operationId}},original.original,original.receipt]){
    const response=await f.raw(cold,f.body(0,payload,original.scope,cold));assert.equal(response.error.code,'ROOM_DELETED');assert.ok(response.requestId);
   }
  }
  const missing=await f.raw(cold,f.body(0,{goldStatus:{operationId:first.completed.operationId}},{...first.scope,roomId:randomUUID()},cold));assert.equal(missing.error.code,'NOT_FOUND');
  const statistics=await f.raw(cold,f.body(0,{statisticsList:{limit:32}},first.scope,cold));assert.equal(statistics.error,undefined);assert.equal(statistics.statistics.length,1);assert.equal(statistics.statistics[0].kind,'gold.deposit');assert.equal(statistics.statistics[0].count,100);
  const foreign=await f.raw(foreignDesktop,f.body(0,{goldStatus:{operationId:other.completed.operationId}},other.scope,foreignDesktop));assert.equal(foreign.error.code,'NOT_FOUND');
  assert.deepEqual(await counts(f,deletedRoom),after);assert.deepEqual(await counts(f,other.scope.roomId!),otherBefore);
 }finally{await f.close();}
});
test('Deletion cleanup rolls back with its transaction; migration cleans legacy already-deleted rows and retains history',async()=>{
 const f=await b1Fixture();try{
  const old=await transfers(f),roomId=old.scope.roomId!,before=await counts(f,roomId);await f.x.end();
  const db=f.x.fixture.service.db,projection=db.transactionProjection;let fail=true;
  db.transactionProjection={uncertain:()=>projection?.uncertain(),prepare:async tx=>{await projection?.prepare(tx);if(fail&&(await tx.query('SELECT 1 FROM rooms WHERE id=$1 AND deleted_at IS NOT NULL',[roomId])).rowCount){fail=false;throw new PolicyError('FIXTURE_DELETE_ROLLBACK');}}};
  await assert.rejects(f.x.deleteLobby(roomId),/FIXTURE_DELETE_ROLLBACK/);if(projection)db.transactionProjection=projection;else delete db.transactionProjection;
  assert.deepEqual(await counts(f,roomId),before);assert.equal((await db.transaction(tx=>tx.query('SELECT deleted_at FROM rooms WHERE id=$1',[roomId]))).rows[0].deleted_at,null);
  // Owned disposable schema only: restore pre121 audit rules and legacy soft
  // deletion, then exercise the actual migration/backfill, not a fake cleanup.
  await db.transaction(async tx=>{await tx.query('DROP TABLE g3_item_inventory,g3_item_receipts,g3_item_operations,g3_bank_items; DROP FUNCTION g3_item_original_guard(); ALTER TABLE g3_runs DROP COLUMN item_revision; DROP TRIGGER g3_ledger_immutable ON g3_ledger; DROP TRIGGER g3_receipts_immutable ON g3_receipts; DROP FUNCTION g3_gold_audit_guard(); DROP FUNCTION g3_delete_room_transfers(uuid); DROP INDEX g3_runs_room');await tx.query('CREATE TRIGGER g3_ledger_immutable BEFORE UPDATE OR DELETE ON g3_ledger FOR EACH ROW EXECUTE FUNCTION g3_immutable_audit(); CREATE TRIGGER g3_receipts_immutable BEFORE UPDATE OR DELETE ON g3_receipts FOR EACH ROW EXECUTE FUNCTION g3_immutable_audit()');await tx.query('DELETE FROM schema_migrations WHERE version IN (121,122)');await tx.query('UPDATE rooms SET deleted_at=now() WHERE id=$1',[roomId]);await tx.query('DELETE FROM memberships WHERE room_id=$1',[roomId]);});
  await migrateGameplay(db);const after=await counts(f,roomId);assert.deepEqual(after,{...before,operations:0,receipts:0,ledger:0,balances:0,outbox:0});
  const cold=await f.x.signIn(),response=await f.raw(cold,f.body(0,{goldStatus:{operationId:old.queued.operationId}},old.scope,cold));assert.equal(response.error.code,'ROOM_DELETED');
  assert.equal((await f.x.deleteLobby(roomId)).deletedRoomId,roomId);assert.deepEqual(await counts(f,roomId),after);
  // The existing run history/statistics remain discoverable to the same account.
  const history=(await f.x.fixture.call('/v3/history/rpc',{major:1,revision:'g3-history-draft-1',serviceId:db.serviceId,serviceEpoch:db.epoch,sessionEpoch:cold.sessionEpoch,requestId:randomUUID(),discover:{}},cold.accessToken));
  const value=JSON.parse(history.body);assert.equal(value.error,undefined);assert.equal(value.runs.some((r:any)=>r.scope.runId===old.scope.runId),true);
 }finally{await f.close();}
});

