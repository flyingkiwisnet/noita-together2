import {readFile} from 'node:fs/promises';
import {G3CandidateDatabase} from './lifecycle.js';
export class HourglassCandidateDatabase extends G3CandidateDatabase {
 override async migrate(){await super.migrate();const tx=await this.pool.connect();try{
  await tx.query('BEGIN');await tx.query('SELECT pg_advisory_xact_lock(hashtext($1))',['nt2-g3-hourglass-migrate:'+this.namespace]);
  if(!(await tx.query('SELECT 1 FROM schema_migrations WHERE version=106')).rowCount)await tx.query(await readFile(new URL('../../../../db/migrations/106_g3_hourglass.sql',import.meta.url),'utf8'));
  if(!(await tx.query('SELECT 1 FROM schema_migrations WHERE version=107')).rowCount)await tx.query(await readFile(new URL('../../../../db/migrations/107_g3_hourglass_storage.sql',import.meta.url),'utf8'));
  await tx.query('COMMIT');
 }catch(e){await tx.query('ROLLBACK');throw e;}finally{tx.release();}}
}
