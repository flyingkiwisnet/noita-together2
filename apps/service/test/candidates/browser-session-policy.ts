// ISOLATED CANDIDATE ONLY. Not imported by production or existing fixture sources.
import type {startG3Fixture} from '../../scripts/g3-loot01-fixture.js';
export const BROWSER_ABANDONED_GRACE_MS=30*60*1000;
type Service=Awaited<ReturnType<typeof startG3Fixture>>['service'];
export async function reclaimAbandonedBrowserCandidate(service:Service,tx:import('../../src/g1/database.js').Tx,accountId:string,now:number){
 // The lock-taking statement may wait behind refresh while retaining its old
 // snapshot of access_tokens. Always recheck in a NEW statement after the lock.
 const candidates=(await tx.query(`SELECT f.id FROM session_families f WHERE f.account_id=$1 AND f.kind='browser' AND f.revoked_at IS NULL AND f.expires_at>$2
  AND (SELECT max(t.expires_at) FROM sessions s JOIN access_tokens t ON t.session_id=s.id WHERE s.family_id=f.id)<=$3
  ORDER BY (SELECT max(t.expires_at) FROM sessions s JOIN access_tokens t ON t.session_id=s.id WHERE s.family_id=f.id),f.id LIMIT 8 FOR UPDATE OF f`,[accountId,new Date(now),new Date(now-BROWSER_ABANDONED_GRACE_MS)])).rows;
 for(const row of candidates){
  const eligible=(await tx.query(`SELECT f.id FROM session_families f WHERE f.id=$1 AND f.account_id=$2 AND f.kind='browser' AND f.revoked_at IS NULL AND f.expires_at>$3
   AND (SELECT max(t.expires_at) FROM sessions s JOIN access_tokens t ON t.session_id=s.id WHERE s.family_id=f.id)<=$4`,[row.id,accountId,new Date(now),new Date(now-BROWSER_ABANDONED_GRACE_MS)])).rowCount;
  if(eligible){await service.auth.revoke(tx,row.id);return row.id as string;}
 }
 return null;
}
export function installBrowserSessionCandidate(service:Service){
 const original=service.auth.family.bind(service.auth);
 service.auth.family=async(tx,accountId,deviceId,kind)=>{
  if(kind==='browser'){
   await tx.query('SELECT id FROM accounts WHERE id=$1 FOR UPDATE',[accountId]);
   const now=service.auth.now(),live=+(await tx.query("SELECT count(*) FROM session_families WHERE account_id=$1 AND kind='browser' AND revoked_at IS NULL AND expires_at>$2",[accountId,new Date(now)])).rows[0].count;
   if(live>=8)await reclaimAbandonedBrowserCandidate(service,tx,accountId,now);
  }
  return original(tx,accountId,deviceId,kind);
 };
}
