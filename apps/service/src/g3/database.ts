import {readFile} from 'node:fs/promises';
import {Database} from '../g1/database.js';
export class G3Database extends Database {
 override async migrate(){
  if(!this.namespace.startsWith('g3_'))throw Error('G3 requires an isolated g3_ namespace');await super.migrate();await migrateGameplay(this);
 }
 override async start(){await super.start();await this.transaction(tx=>tx.query('UPDATE g3_contexts SET revoked_at=$1 WHERE revoked_at IS NULL',[new Date()]));}
}
export async function migrateGameplay(db:Database){
 const tx=await db.pool.connect();try{await tx.query('BEGIN');await tx.query('SELECT pg_advisory_xact_lock(hashtext($1))',['nt2-g3-migrate:'+db.namespace]);
  for(const [version,file] of [[101,'101_g3_gameplay.sql'],[102,'102_g3_movement_fences.sql'],[105,'105_g3_history.sql'],[106,'106_g3_hourglass.sql'],[107,'107_g3_hourglass_storage.sql'],[120,'120_b1_gold_bank.sql'],[121,'121_deleted_lobby_transfers.sql'],[122,'122_item_transfers.sql'],[123,'123_bank_events.sql'],[124,'124_c1_player_rules.sql'],[125,'125_c1_source_outcomes.sql'],[126,'126_c2_world_rules.sql'],[132,'132_race_first_release.sql']] as const)
   if(!(await tx.query('SELECT 1 FROM schema_migrations WHERE version=$1',[version])).rowCount)await tx.query(await readFile(new URL('../../../../db/migrations/'+file,import.meta.url),'utf8'));
  await tx.query('COMMIT');
 }catch(e){await tx.query('ROLLBACK');throw e;}finally{tx.release();}
}
