import {readFile} from 'node:fs/promises';
import {Database} from '../g1/database.js';
export class CandidateDatabase extends Database {
 protected readonly namespacePrefix:string='g2_next_';
 override async migrate(){
  if(!this.namespace.startsWith(this.namespacePrefix))throw Error('Successor candidate requires a separate '+this.namespacePrefix+' namespace');
  await super.migrate();const tx=await this.pool.connect();try{await tx.query('BEGIN');await tx.query('SELECT pg_advisory_xact_lock(hashtext($1))',['nt2-g2-next-migrate:'+this.namespace]);if(!(await tx.query('SELECT 1 FROM schema_migrations WHERE version=7')).rowCount)await tx.query(await readFile(new URL('../../../../db/migrations/007_g2_next.sql',import.meta.url),'utf8'));if(!(await tx.query('SELECT 1 FROM schema_migrations WHERE version=8')).rowCount)await tx.query(await readFile(new URL('../../../../db/migrations/008_g2_next_release.sql',import.meta.url),'utf8'));if(!(await tx.query('SELECT 1 FROM schema_migrations WHERE version=9')).rowCount)await tx.query(await readFile(new URL('../../../../db/migrations/009_g2_next_play_entry.sql',import.meta.url),'utf8'));await tx.query('COMMIT');}catch(e){await tx.query('ROLLBACK');throw e;}finally{tx.release();}
  const deletion=await this.pool.connect();try{await deletion.query('BEGIN');await deletion.query('SELECT pg_advisory_xact_lock(hashtext($1))',['nt2-g2-next-migrate:'+this.namespace]);if(!(await deletion.query('SELECT 1 FROM schema_migrations WHERE version=117')).rowCount)await deletion.query(await readFile(new URL('../../../../db/migrations/117_room_deletion.sql',import.meta.url),'utf8'));if(!(await deletion.query('SELECT 1 FROM schema_migrations WHERE version=127')).rowCount)await deletion.query(await readFile(new URL('../../../../db/migrations/127_auth1_accounts.sql',import.meta.url),'utf8'));if(!(await deletion.query('SELECT 1 FROM schema_migrations WHERE version=128')).rowCount)await deletion.query(await readFile(new URL('../../../../db/migrations/128_ux1_lobbies.sql',import.meta.url),'utf8'));if(!(await deletion.query('SELECT 1 FROM schema_migrations WHERE version=131')).rowCount)await deletion.query(await readFile(new URL('../../../../db/migrations/131_d1_lobby_statistics.sql',import.meta.url),'utf8'));if(!(await deletion.query('SELECT 1 FROM schema_migrations WHERE version=133')).rowCount)await deletion.query(await readFile(new URL('../../../../db/migrations/133_auth_reliability.sql',import.meta.url),'utf8'));await deletion.query('COMMIT');}catch(e){await deletion.query('ROLLBACK');throw e;}finally{deletion.release();}
 }
 override async start(){await super.start();await this.transaction(async tx=>{
  await tx.query("UPDATE runs SET state='cancelled',reason='service_restart',ended_at=now() WHERE state='staging'");
  await tx.query("UPDATE rooms SET state='waiting',revision=revision+1 WHERE state='staging'");
  await tx.query("UPDATE preparations SET state='cancelled' WHERE state IN ('issued','consumed','loaded')");
  await tx.query("UPDATE inspected_releases SET revoked_at=now(),reason='service_restart' WHERE revoked_at IS NULL");
  await tx.query("UPDATE account_signin_attempts SET state='cancelled',grant_hash=NULL WHERE state IN ('pending','completed')");
 });}
}
