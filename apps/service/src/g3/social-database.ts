import {readFile} from 'node:fs/promises';
import {G3CandidateDatabase} from './lifecycle.js';
export class SocialCandidateDatabase extends G3CandidateDatabase {
 override async migrate(){await super.migrate();const tx=await this.pool.connect();try{
  await tx.query('BEGIN');await tx.query('SELECT pg_advisory_xact_lock(hashtext($1))',['nt2-g3-social-migrate:'+this.namespace]);
  if(!(await tx.query('SELECT 1 FROM schema_migrations WHERE version=103')).rowCount)await tx.query(await readFile(new URL('../../../../db/migrations/103_g3_social.sql',import.meta.url),'utf8'));
  if(!(await tx.query('SELECT 1 FROM schema_migrations WHERE version=104')).rowCount)await tx.query(await readFile(new URL('../../../../db/migrations/104_g3_social_replay.sql',import.meta.url),'utf8'));
  if(!(await tx.query('SELECT 1 FROM schema_migrations WHERE version=118')).rowCount)await tx.query(await readFile(new URL('../../../../db/migrations/118_retained_player_information.sql',import.meta.url),'utf8'));
  if(!(await tx.query('SELECT 1 FROM schema_migrations WHERE version=119')).rowCount)await tx.query(await readFile(new URL('../../../../db/migrations/119_social_presentation_state.sql',import.meta.url),'utf8'));
  await tx.query('COMMIT');
 }catch(e){await tx.query('ROLLBACK');throw e;}finally{tx.release();}}
 override async start(){await super.start();await this.transaction(async tx=>{await tx.query('DELETE FROM g3_social_latest');await tx.query("UPDATE g3_social_presentation SET body=jsonb_set(body,'{emote,active}','false'::jsonb),revision=revision+1 WHERE kind='emote' AND body->'emote'->>'active'='true' AND revision<4294967295");});}
}
