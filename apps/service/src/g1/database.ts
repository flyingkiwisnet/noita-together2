import pg from 'pg';
import {readFile} from 'node:fs/promises';
import {randomUUID} from 'node:crypto';
import {PolicyError} from './security.js';
import {WORK_LIMITS,DatabaseWorkScheduler,type DatabaseWorkClass,unavailable,waitFor} from './work.js';
import {publishCommitEffects,discardCommitEffects} from './commit-hooks.js';
export type Tx=pg.PoolClient;
// PostgreSQL 16 has no transaction_timeout. Negotiate the extra server-side
// bound after startup, before pg-pool lends a new connection to any caller.
// The JS deadline and statement/lock/idle bounds apply on both versions.
export async function configureTransactionTimeout(client:pg.ClientBase,transactionMs:number){
 await client.query("SELECT pg_catalog.set_config(name,$1,false) FROM pg_catalog.pg_settings WHERE name='transaction_timeout'",[String(transactionMs)]);
}
export interface DatabaseBudget {maxWork:number;acquireMs:number;statementMs:number;lockMs:number;transactionMs:number}
export type TransactionDiagnostic={phase:string;category:'service-admission'|'transaction'|'deadline'|'committed';code:string;workClass?:DatabaseWorkClass;queueMs?:number;sqlState?:string;abortCause?:'client-error'|'transaction-deadline'|'caller-abort';commit:'not-attempted'|'uncertain'|'confirmed'};
export class Database {
 // Internal observer only. Never includes SQL, parameters, messages or credentials.
 transactionDiagnostic?:((event:TransactionDiagnostic)=>void)|undefined;
 private diagnose(event:TransactionDiagnostic){try{this.transactionDiagnostic?.(event);}catch{/* Observability cannot change transaction semantics. */}}
 // Optional bounded committed projection observer; durable results retain ownership.
 transactionProjection?:{prepare:(tx:Tx)=>Promise<void>;uncertain:()=>void};
 readonly pool:pg.Pool; epoch=''; private leader:Tx|undefined; private healthy=false;
 readonly budget:DatabaseBudget;private gate:DatabaseWorkScheduler;private closing=false;private closingPromise:Promise<void>|undefined;
 private active=new Set<{abort:()=>void;done:Promise<unknown>}>();
 constructor(url:string,readonly serviceId:string,readonly namespace='public',budget:Partial<DatabaseBudget>={},readonly g2=false){
  if(!/^[a-z][a-z0-9_]{0,62}$/.test(namespace))throw Error('Invalid database namespace');
  this.budget={maxWork:WORK_LIMITS.databaseWork,acquireMs:WORK_LIMITS.acquireMs,statementMs:WORK_LIMITS.statementMs,lockMs:WORK_LIMITS.lockMs,transactionMs:WORK_LIMITS.transactionMs,...budget};
  if(this.budget.maxWork>7||Object.values(this.budget).some(n=>!Number.isInteger(n)||n<1||n>10000))throw Error('Invalid database work/deadline budget');
  this.gate=new DatabaseWorkScheduler(this.budget.maxWork);
  const {statementMs,lockMs,transactionMs}=this.budget;
  this.pool=new pg.Pool({connectionString:url,max:8,connectionTimeoutMillis:this.budget.acquireMs,query_timeout:statementMs+250,options:`-c search_path=${namespace},pg_catalog -c statement_timeout=${statementMs} -c lock_timeout=${lockMs} -c idle_in_transaction_session_timeout=${transactionMs} -c client_connection_check_interval=100`,onConnect:client=>configureTransactionTimeout(client,transactionMs)});
  this.pool.on('error',()=>{/* Idle worker failures are discarded by the pool; leader errors have their own handler. */});
 }
 get ready(){return this.healthy;}
 get workStats(){return {...this.gate.stats,poolWaiting:this.pool.waitingCount,poolTotal:this.pool.totalCount,poolIdle:this.pool.idleCount};}
 async migrate(){
  const tx=await this.pool.connect();try{
   await tx.query('BEGIN');await tx.query('SELECT pg_advisory_xact_lock(hashtext($1))',['nt2-g1-migrate:'+this.namespace]);
   await tx.query(`CREATE SCHEMA IF NOT EXISTS "${this.namespace}"`);
   const exists=await tx.query("SELECT to_regclass('schema_migrations') AS table_name");
   const versions=exists.rows[0].table_name?new Set((await tx.query('SELECT version FROM schema_migrations')).rows.map(r=>r.version)):new Set();
   for(const [version,file] of [[1,'001_g1_identity.sql'],[2,'002_g1_rooms.sql'],[3,'003_g1_frozen_exclusions.sql'],[4,'004_g1_admission.sql'],[130,'130_lobby_session_retirement.sql']] as const){if(!versions.has(version))await tx.query(await readFile(new URL('../../../../db/migrations/'+file,import.meta.url),'utf8'));}
   if(this.g2&&!versions.has(5))await tx.query(await readFile(new URL('../../../../db/migrations/005_g2_lobby.sql',import.meta.url),'utf8'));
   if(this.g2&&!versions.has(6))await tx.query(await readFile(new URL('../../../../db/migrations/006_g2_chat_retention.sql',import.meta.url),'utf8'));
   await tx.query('COMMIT');
  }catch(error){await tx.query('ROLLBACK');throw error;}finally{tx.release();}
 }
 async start(){
  this.leader=await this.pool.connect();
  this.leader.on('error',()=>{this.healthy=false;});
  const locked=await this.leader.query('SELECT pg_try_advisory_lock(hashtext($1)) AS held',['nt2-g1-service:'+this.namespace]);
  if(!locked.rows[0].held){this.leader.release();this.leader=undefined;throw Error('Another service owns this database');}
  this.healthy=true;
  this.epoch=randomUUID();
  await this.transaction(async tx=>{
   const identity=await tx.query('SELECT * FROM service_identity');
   if(identity.rowCount&&identity.rows[0].service_id!==this.serviceId)throw Error('Configured serviceId differs from durable database identity');
   await tx.query('INSERT INTO service_identity(service_id,epoch) VALUES($1,$2) ON CONFLICT(singleton) DO UPDATE SET epoch=EXCLUDED.epoch',[this.serviceId,this.epoch]);
   await tx.query("UPDATE sessions SET revoked_at=now() WHERE kind='game' AND revoked_at IS NULL");
   await tx.query('UPDATE attachments SET revoked_at=now() WHERE revoked_at IS NULL');
   await tx.query('UPDATE memberships SET presence_until=NULL,ready_config_revision=NULL,selected=false WHERE presence_until IS NOT NULL OR ready_config_revision IS NOT NULL OR selected IS DISTINCT FROM false');
   await tx.query("UPDATE runs SET state='cancelled',ended_at=now(),reason='service_restart' WHERE state IN ('preparing','loading')");
   await tx.query("UPDATE rooms SET state='waiting',revision=revision+1 WHERE state IN ('preparing','loading')");
   await tx.query("UPDATE runs SET state='suspended',epoch=gen_random_uuid(),reason='service_restart' WHERE state IN ('running','suspended')");
   await tx.query("UPDATE rooms SET state='suspended',revision=revision+1 WHERE state='running'");
  },false);
 }
 async transaction<T>(fn:(tx:Tx)=>Promise<T>,checkEpoch=true,callerSignal?:AbortSignal,workClass:DatabaseWorkClass='optional'):Promise<T>{
  if(!this.healthy||this.closing||callerSignal?.aborted){this.diagnose({phase:'admission',category:callerSignal?.aborted?'deadline':'service-admission',code:'SERVICE_UNAVAILABLE',commit:'not-attempted'});throw unavailable();}
  let releasePermit:(()=>void)|undefined;
  const controller=new AbortController();const signal=callerSignal?AbortSignal.any([callerSignal,controller.signal]):controller.signal;
  let deadlineExpired=false,clientFailed=false;
  const deadline=setTimeout(()=>{deadlineExpired=true;controller.abort();},this.budget.transactionMs);deadline.unref();
  let tx:Tx|undefined,released=false,commitUncertain=false,commitConfirmed=false,phase='admission',queueMs=0;const admittedAt=performance.now();const clientError=()=>{clientFailed=true;controller.abort();};
  const checkDeadline=()=>{if(performance.now()-admittedAt>=this.budget.transactionMs){deadlineExpired=true;controller.abort();}if(signal.aborted||!this.healthy||this.closing)throw unavailable();};
  const destroy=()=>{if(tx&&!released){released=true;tx.release(true);}};
  signal.addEventListener('abort',destroy,{once:true});
  let acquire:Promise<Tx>|undefined;
  let finish!:()=>void;const done=new Promise<void>(resolve=>{finish=resolve;});const running={abort:()=>controller.abort(),done};this.active.add(running);
  try{
   releasePermit=await this.gate.enter(workClass,signal);queueMs=Math.round(performance.now()-admittedAt);checkDeadline();
   phase='acquire';acquire=this.pool.connect().then(client=>{
    // Capture ownership before resolving, including an abort/acquire race.
    tx=client;tx.on('error',clientError);if(signal.aborted)destroy();return client;
   });
   tx=await waitFor(acquire,signal);if(signal.aborted){destroy();throw unavailable();}
   // Client errors must not become unhandled events when a deadline destroys it.
   return await waitFor((async()=>{
    phase='begin';await tx!.query('BEGIN');phase='lock';await tx!.query('SELECT pg_advisory_xact_lock(hashtext($1))',['nt2-g1-mutation:'+this.namespace]);
    if(!this.healthy||signal.aborted)throw unavailable();
    phase='epoch';if(checkEpoch){const identity=(await tx!.query('SELECT epoch FROM service_identity')).rows[0];if(identity?.epoch!==this.epoch)throw unavailable();}
    checkDeadline();phase='work';const result=await fn(tx!);checkDeadline();phase='projection';await this.transactionProjection?.prepare(tx!);checkDeadline();phase='commit';commitUncertain=true;const committed=await tx!.query('COMMIT');commitUncertain=false;if(committed.command!=='COMMIT')throw unavailable();commitConfirmed=true;this.diagnose({phase,category:'committed',code:'COMMIT',commit:'confirmed',workClass,queueMs});phase='publish';publishCommitEffects(tx!);return result;
   })(),signal);
  }catch(e){
   const sqlState=!(e instanceof PolicyError)&&/^[0-9A-Z]{5}$/.test(String((e as {code?:unknown})?.code??''))?String((e as {code:string}).code):undefined;
   this.diagnose({phase,category:clientFailed?'transaction':signal.aborted||sqlState==='57014'||sqlState==='55P03'?'deadline':e instanceof PolicyError?'service-admission':'transaction',code:e instanceof PolicyError&&/^[A-Z_]{1,64}$/.test(e.code)?e.code:'SERVICE_UNAVAILABLE',workClass,queueMs:releasePermit?queueMs:Math.round(performance.now()-admittedAt),...(sqlState?{sqlState}:{}),...(clientFailed?{abortCause:'client-error' as const}:deadlineExpired?{abortCause:'transaction-deadline' as const}:callerSignal?.aborted?{abortCause:'caller-abort' as const}:{}),commit:commitConfirmed?'confirmed':commitUncertain?'uncertain':'not-attempted'});
   if(tx&&!released){try{await tx.query('ROLLBACK');}catch{destroy();}}
   if(commitUncertain)this.transactionProjection?.uncertain();if(e instanceof PolicyError)throw e;
   // Database/network diagnostics are not part of the public wire response.
   throw unavailable();
  }finally{
   clearTimeout(deadline);signal.removeEventListener('abort',destroy);
   // Keep the permit until a cancelled acquisition has left pg-pool's queue.
   if(!tx&&acquire)await acquire.catch(()=>undefined);
   if(tx)discardCommitEffects(tx);if(tx&&!released){tx.removeListener('error',clientError);released=true;tx.release();}releasePermit?.();this.active.delete(running);finish();
  }
 }
 async close(){
  if(this.closingPromise)return this.closingPromise;this.closing=true;this.healthy=false;this.gate.close();
  this.closingPromise=(async()=>{for(const work of this.active)work.abort();await Promise.allSettled([...this.active].map(work=>work.done));if(this.leader){this.leader.release(true);this.leader=undefined;}await this.pool.end();})();return this.closingPromise;
 }
}
