import {PrincipalAuth} from '../g2-next/auth.js';
import type {Tx} from '../g1/database.js';
import {randomUUID} from 'node:crypto';
import {demand} from '../g1/security.js';
export class RejoinAuth extends PrincipalAuth {
 override async family(tx:Tx,accountId:string,deviceId:string,kind:'browser'|'desktop'){
  if(kind==='browser')return super.family(tx,accountId,deviceId,kind);
  await tx.query('SELECT id FROM accounts WHERE id=$1 FOR UPDATE',[accountId]);await tx.query('INSERT INTO devices(account_id,device_id) VALUES($1,$2) ON CONFLICT DO NOTHING',[accountId,deviceId]);
  const old=(await tx.query("SELECT id FROM session_families WHERE account_id=$1 AND device_id=$2 AND kind='desktop' AND revoked_at IS NULL FOR UPDATE",[accountId,deviceId])).rows;
  for(const f of old){const active=(await tx.query(`SELECT 1 FROM rejoin_participants p JOIN attachments a ON a.id=p.attachment_id JOIN sessions s ON s.id=a.game_session_id JOIN sessions d ON d.id=a.desktop_id JOIN memberships m ON m.desktop_id=d.id WHERE s.family_id=$1 AND NOT p.terminal AND a.revoked_at IS NULL AND s.revoked_at IS NULL AND d.revoked_at IS NULL AND a.presence_until>$2 AND m.presence_until>$2`,[f.id,new Date(this.now())])).rowCount;if(!active)await this.revoke(tx,f.id);}
  const count=+(await tx.query("SELECT count(*) FROM session_families WHERE account_id=$1 AND kind='desktop' AND revoked_at IS NULL AND expires_at>$2",[accountId,new Date(this.now())])).rows[0].count;demand(count<8,'CAPACITY');
  const id=randomUUID(),sid=randomUUID(),epoch=randomUUID();await tx.query("INSERT INTO session_families(id,account_id,device_id,kind,expires_at) VALUES($1,$2,$3,'desktop',$4)",[id,accountId,deviceId,new Date(this.now()+30*86400000)]);await tx.query("INSERT INTO sessions(id,family_id,kind,epoch) VALUES($1,$2,'desktop',$3)",[sid,id,epoch]);return this.mint(tx,accountId,deviceId,kind,id,sid,epoch);
 }
 override async revoke(tx:Tx,familyId:string){
  // Reauthentication may supersede a desktop family. Fence only its games;
  // account/participant eligibility and all other active owners remain intact.
  await tx.query('UPDATE session_families SET revoked_at=$2 WHERE id=$1',[familyId,new Date(this.now())]);
  await tx.query('UPDATE sessions SET revoked_at=$2 WHERE family_id=$1 AND revoked_at IS NULL',[familyId,new Date(this.now())]);
  await tx.query('UPDATE attachments SET revoked_at=$2 WHERE game_session_id IN (SELECT id FROM sessions WHERE family_id=$1)',[familyId,new Date(this.now())]);
  await tx.query('UPDATE g3_contexts SET revoked_at=$2 WHERE game_session_id IN (SELECT id FROM sessions WHERE family_id=$1)',[familyId,new Date(this.now())]);
  await tx.query('UPDATE memberships SET presence_until=NULL WHERE desktop_id IN (SELECT id FROM sessions WHERE family_id=$1)',[familyId]);
  await tx.query("UPDATE rejoin_participants SET state='reconnecting',reason='session_replaced' WHERE NOT terminal AND desktop_id IN (SELECT id FROM sessions WHERE family_id=$1)",[familyId]);
 }
}
