import {readFile} from 'node:fs/promises';
import {SocialCandidateDatabase} from '../g3/social-database.js';
export class RejoinDatabase extends SocialCandidateDatabase {
 readonly rejoinEnabled=true;
 override async migrate(){
  if(!this.namespace.startsWith('g3_rejoin_'))throw Error('Rejoin uses only the explicitly selected new g3_rejoin_* namespace');
  await super.migrate();const tx=await this.pool.connect();try{await tx.query('BEGIN');await tx.query('SELECT pg_advisory_xact_lock(hashtext($1))',['nt2-rejoin-migrate:'+this.namespace]);for(const [version,file]of [[110,'110_g3_rejoin.sql'],[111,'111_g3_rejoin_host_identity.sql'],[112,'112_g3_connection_outcomes.sql'],[113,'113_g3_waiting_worlds.sql'],[114,'114_g3_startup_pending.sql'],[115,'115_g3_startup_entry.sql'],[116,'116_g3_world_lifecycle.sql'],[129,'129_ux1_rejoin_reports.sql']] as const)if(!(await tx.query('SELECT 1 FROM schema_migrations WHERE version=$1',[version])).rowCount)await tx.query(await readFile(new URL('../../../../db/migrations/'+file,import.meta.url),'utf8'));await tx.query('COMMIT');}catch(e){await tx.query('ROLLBACK');throw e;}finally{tx.release();}
 }
 override async start(){
  await super.start();await this.transaction(async tx=>{
   // Base startup fences every old session/attachment. Restore only durable,
   // nonterminal runs, with fresh epochs and no active reports or game owners.
   await tx.query(`UPDATE runs n SET state='running',service_epoch=$1,ended_at=NULL,reason='recovery_waiting' FROM rejoin_runs k WHERE k.run_id=n.id AND k.terminal_reason IS NULL AND n.state='suspended' AND n.reason='service_restart'`,[this.epoch]);
   await tx.query("UPDATE rooms r SET state='running' FROM runs n JOIN rejoin_runs k ON k.run_id=n.id WHERE r.current_run_id=n.id AND n.state='running' AND k.terminal_reason IS NULL");
   await tx.query("UPDATE rejoin_participants p SET state='reconnecting',reason='service_restart',pending_report=NULL FROM runs n WHERE n.id=p.run_id AND n.state='running' AND NOT p.terminal");
   await tx.query("UPDATE inspected_releases x SET revoked_at=NULL,reason=NULL,selected_reports=jsonb_build_object('participants','[]'::jsonb,'excludedDesktopIds','[]'::jsonb) FROM runs n JOIN rejoin_runs k ON k.run_id=n.id WHERE x.run_id=n.id AND n.state='running' AND k.terminal_reason IS NULL");
   await tx.query("UPDATE g3_runs g SET state='running',epoch=n.epoch FROM runs n JOIN rejoin_runs k ON k.run_id=n.id WHERE g.id=n.id AND n.state='running' AND k.terminal_reason IS NULL");
  });
 }
}

