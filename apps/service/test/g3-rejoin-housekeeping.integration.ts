import test from 'node:test';import assert from 'node:assert/strict';
import {startG3Fixture,provisionG3Fixture} from '../scripts/g3-loot01-fixture.js';
import {RejoinRooms} from '../src/rejoin/rooms.js';
import {schema} from '../../../packages/contracts/src/g3-rejoin.js';
import pg from 'pg';

test('RJ1 scoped housekeeping passes528 healthy earlier rows, isolates locks and advances periodic room batches',async()=>{
 const f=await startG3Fixture({rejoin:true,loot01Runtime:true,live:true,tls:true});let blocker:pg.Client|undefined;
 try{
  const s=await provisionG3Fixture(f);await s.start();await s.playing();const db=f.service.db,rooms=f.service.rooms as RejoinRooms;
  const participant=(await db.pool.query('SELECT * FROM rejoin_participants WHERE account_id=$1',[s.desktops[1]!.accountId])).rows[0];
  // Synthetic database geometry only:33 healthy retained rosters of16 rows.
  // Reuse private fixture identity/attachment facts; never admit/load531 games.
  const id=(n:number)=>'00000000-0000-4000-8000-'+n.toString().padStart(12,'0');
  const targetRoom='ffffffff-ffff-4fff-8fff-fffffffffff1',targetRun='ffffffff-ffff-4fff-8fff-fffffffffff2',targetId='ffffffff-ffff-4fff-8fff-fffffffffff3';
  const create=async(tx:any,roomId:string,runId:string,firstId:number,count:number)=>{
   await tx.query(`INSERT INTO rooms(id,name,host_account_id,state,revision,config_revision,config,visibility,capacity,allow_guests) SELECT $1,'Scoped housekeeping fixture',host_account_id,'running',1,config_revision,config,visibility,32,allow_guests FROM rooms WHERE id=$2`,[roomId,s.room.roomId]);
   await tx.query(`INSERT INTO runs(id,room_id,epoch,service_epoch,config_revision,config,roster,excluded_roster,state,preparation_until,review_until,review_minutes,created_at) SELECT $1,$2,epoch,service_epoch,config_revision,config,roster,excluded_roster,'running',preparation_until,review_until,review_minutes,created_at FROM runs WHERE id=$3`,[runId,roomId,s.room.runId]);
   await tx.query('UPDATE rooms SET current_run_id=$2 WHERE id=$1',[roomId,runId]);
   await tx.query('INSERT INTO rejoin_runs(run_id,room_id,original_start_id,frozen_config,config_revision,created_at) SELECT $1,$2,original_start_id,frozen_config,config_revision,created_at FROM rejoin_runs WHERE run_id=$3',[runId,roomId,s.room.runId]);
   await tx.query(`INSERT INTO rejoin_participants(id,run_id,account_id,device_id,world_token,proof_hash,proof_ciphertext,original_report,accepted_digest,generation,desktop_id,attachment_id,state,reason,terminal,authorization_id,authorized_at,created_at)
    SELECT ('00000000-0000-4000-8000-'||lpad((n+$3)::text,12,'0'))::uuid,$1,account_id,'geometry-device-'||n,'geometry-world-'||n,proof_hash,proof_ciphertext,original_report,accepted_digest,generation,desktop_id,attachment_id,'active','',false,authorization_id,authorized_at,created_at FROM rejoin_participants CROSS JOIN generate_series(1,$4) n WHERE id=$2`,[runId,participant.id,firstId,count]);
  };
  await db.transaction(async tx=>{for(let n=0;n<33;n++)await create(tx,id(10000+n),id(20000+n),n*16,16);await create(tx,targetRoom,targetRun,100000,1);await tx.query('UPDATE rejoin_participants SET id=$2 WHERE run_id=$1',[targetRun,targetId]);await tx.query('INSERT INTO room_bans(room_id,account_id) VALUES($1,$2)',[targetRoom,participant.account_id]);});
  assert.ok(+(await db.pool.query('SELECT count(*) FROM rejoin_participants WHERE NOT terminal')).rows[0].count>512);
  blocker=new pg.Client({connectionString:db.pool.options.connectionString,options:'-c search_path='+db.namespace+',pg_catalog -c transaction_timeout=20000 -c idle_in_transaction_session_timeout=20000 -c statement_timeout=5000'});blocker.on('error',()=>{});await blocker.connect();await blocker.query('BEGIN');await blocker.query('SELECT id FROM rooms WHERE id=$1 FOR UPDATE',[id(10000)]);await blocker.query('SELECT id FROM rejoin_participants WHERE id=$1 FOR UPDATE',[id(1)]);
  // An unrelated room and first-page participant stay locked. Targeted cleanup
  // must still complete; the old global scan would wait on this participant.
  await db.transaction(tx=>rooms.recovery.sweep(tx,targetRun));
  assert.equal((await db.pool.query('SELECT terminal FROM rejoin_participants WHERE id=$1',[targetId])).rows[0].terminal,true);
  assert.equal((await db.pool.query('SELECT terminal FROM rejoin_participants WHERE id=$1',[id(1)])).rows[0].terminal,false);
  assert.equal((await blocker.query('SELECT 1 AS held')).rows[0].held,1);
  // Release guards are current even when no periodic cleanup has seen a new ban.
  const own=(await db.pool.query('SELECT id FROM rejoin_participants WHERE run_id=$1 AND account_id=$2',[s.room.runId,s.desktops[0]!.accountId])).rows[0];
  const p=await db.transaction(tx=>rooms.recovery.read(tx,own.id));assert.ok(await db.transaction(tx=>rooms.recovery.release(tx,p)));
  await db.transaction(tx=>tx.query('INSERT INTO room_bans(room_id,account_id) VALUES($1,$2)',[s.room.roomId,s.desktops[0]!.accountId]));
  assert.equal(await db.transaction(tx=>rooms.recovery.release(tx,p)),undefined);assert.equal((await db.pool.query('SELECT terminal FROM rejoin_participants WHERE id=$1',[own.id])).rows[0].terminal,false);
  // Reconcile a different room without asking a global request sweep to run.
  const desktop=s.desktops[0]!,reply=await f.call('/v3/rejoin/rpc',{major:1,revision:schema.revision,serviceId:db.serviceId,serviceEpoch:db.epoch,sessionEpoch:desktop.sessionEpoch,requestId:'scoped-status',status:{}},desktop.accessToken);
  assert.equal(JSON.parse(reply.body).error,undefined);
  // Reset just the synthetic target and prove the global periodic cursor can
  // reach its final room despite earlier healthy rows and a busy first room.
  await db.transaction(tx=>tx.query("UPDATE rejoin_participants SET terminal=false,state='reconnecting',reason='' WHERE id=$1",[targetId]));
  for(let n=0;n<3;n++)await rooms.sweep();
  assert.equal((await db.pool.query('SELECT terminal FROM rejoin_participants WHERE id=$1',[targetId])).rows[0].terminal,true);
  assert.equal((await db.pool.query('SELECT terminal FROM rejoin_participants WHERE id=$1',[id(1)])).rows[0].terminal,false);
 }finally{try{if(blocker){try{await blocker.query('ROLLBACK');}catch{}await blocker.end();}}finally{await f.close();}}
});
