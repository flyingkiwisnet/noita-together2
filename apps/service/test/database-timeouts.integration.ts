import test from 'node:test';import assert from 'node:assert/strict';import pg from 'pg';import {randomUUID} from 'node:crypto';import {setTimeout as delay} from 'node:timers/promises';import {Database} from '../src/g1/database.js';

test('negotiated database timeouts preserve native bounds and JS rollback without transaction_timeout',async()=>{
 const url='postgresql://nt2_g1:local-fixture-only@127.0.0.1:54328/nt2_g1',namespace='g3_timeouts_'+randomUUID().replaceAll('-','');
 const admin=new pg.Client({connectionString:url,connectionTimeoutMillis:3000,query_timeout:3000});await admin.connect();
 const db=new Database(url,'timeout-compatibility',namespace,{statementMs:150,lockMs:100,transactionMs:650});let owns=false;let blocker:pg.Client|undefined;
 try{
  assert.equal((await admin.query('SELECT to_regnamespace($1) AS schema',[namespace])).rows[0].schema,null);await admin.query('CREATE SCHEMA "'+namespace+'"');owns=true;
  await db.migrate();await db.start();await db.pool.query('CREATE TABLE timeout_probe(id integer PRIMARY KEY)');
  const settings=(await db.pool.query("SELECT current_setting('server_version_num') AS version,current_setting('transaction_timeout',true) AS transaction,current_setting('statement_timeout') AS statement,current_setting('lock_timeout') AS lock,current_setting('idle_in_transaction_session_timeout') AS idle")).rows[0];
  assert.equal(settings.statement,'150ms');assert.equal(settings.lock,'100ms');assert.equal(settings.idle,'650ms');assert.equal(settings.transaction,Number(settings.version)>=170000?'650ms':null);
  await assert.rejects(db.transaction(async tx=>{await tx.query('INSERT INTO timeout_probe VALUES(1)');await tx.query('SELECT pg_sleep(2)');}),/SERVICE_UNAVAILABLE/);
  // Disable the PG17-only safety net and extend idle timeout, so only the
  // existing JS whole-transaction deadline can prevent the late commit.
  await assert.rejects(db.transaction(async tx=>{
   await tx.query("SELECT pg_catalog.set_config(name,'0',false) FROM pg_catalog.pg_settings WHERE name='transaction_timeout'");await tx.query("SET idle_in_transaction_session_timeout='5s'");
   await tx.query('INSERT INTO timeout_probe VALUES(2)');await delay(850);await tx.query('INSERT INTO timeout_probe VALUES(3)');
  }),/SERVICE_UNAVAILABLE/);
  await delay(300);assert.equal(Number((await db.pool.query('SELECT count(*) FROM timeout_probe')).rows[0].count),0);assert.equal(db.workStats.active,0);assert.equal(db.workStats.poolWaiting,0);
  await db.pool.query('INSERT INTO timeout_probe VALUES(4)');blocker=new pg.Client({connectionString:url,connectionTimeoutMillis:3000,query_timeout:3000});await blocker.connect();await blocker.query('BEGIN');await blocker.query('SELECT id FROM "'+namespace+'".timeout_probe WHERE id=4 FOR UPDATE');
  await assert.rejects(db.transaction(tx=>tx.query('DELETE FROM timeout_probe WHERE id=4')),/SERVICE_UNAVAILABLE/);await blocker.query('ROLLBACK');await blocker.end();blocker=undefined;
  await db.transaction(tx=>tx.query('SELECT 1'));assert.equal(db.workStats.active,0);
  console.log(JSON.stringify({databaseTimeoutCompatibility:{serverVersion:Number(settings.version),nativeTransactionTimeout:settings.transaction,statementBound:true,lockBound:true,idleBound:true,jsDeadlineWithoutNativeTimeout:true,rolledBack:true,recovered:true}}));
 }finally{
  if(blocker){await blocker.query('ROLLBACK').catch(()=>{});await blocker.end();}await db.close();
  try{if(owns){await admin.query('DROP SCHEMA "'+namespace+'" CASCADE');assert.equal((await admin.query('SELECT to_regnamespace($1) AS schema',[namespace])).rows[0].schema,null);console.log(JSON.stringify({verifiedAbsentSchema:namespace}));}}finally{await admin.end();}
 }
});
