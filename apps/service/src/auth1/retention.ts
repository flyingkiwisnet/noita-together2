import type {Tx} from '../g1/database.js';
export const AUTH_RETENTION={batch:256,handoffMs:3600000,routineMs:900000} as const;
export async function maintainAuthentication(tx:Tx,now:number){
 const n=new Date(now),handoff=new Date(now-AUTH_RETENTION.handoffMs),routine=new Date(now-AUTH_RETENTION.routineMs),limit=AUTH_RETENTION.batch;
 // Expired handoff records get a full additional hour beyond their recovery
 // window. They cannot issue a current grant; flow FKs are removed first.
 await tx.query('DELETE FROM auth1_flows WHERE id IN (SELECT id FROM auth1_flows WHERE expires_at<$1 ORDER BY expires_at,id LIMIT $2 FOR UPDATE SKIP LOCKED)',[handoff,limit]);
 await tx.query('DELETE FROM account_signin_attempts WHERE id IN (SELECT a.id FROM account_signin_attempts a WHERE expires_at<$1 AND NOT EXISTS(SELECT 1 FROM auth1_flows f WHERE f.attempt_id=a.id) ORDER BY expires_at,id LIMIT $2 FOR UPDATE SKIP LOCKED)',[handoff,limit]);
 await tx.query('DELETE FROM auth_requests WHERE (scope,request_id) IN (SELECT scope,request_id FROM auth_requests WHERE recovery_until<$1 ORDER BY recovery_until LIMIT $2 FOR UPDATE SKIP LOCKED)',[handoff,limit]);
 // Keep the latest expired access token per still-live session: abandoned
 // browser reclamation needs its expiry watermark. All older tokens may go.
 await tx.query(`DELETE FROM access_tokens WHERE id IN (SELECT t.id FROM access_tokens t JOIN sessions s ON s.id=t.session_id JOIN session_families f ON f.id=s.family_id WHERE t.expires_at<$1 AND
  (s.revoked_at IS NOT NULL OR f.revoked_at IS NOT NULL OR f.expires_at<=$1 OR EXISTS(SELECT 1 FROM access_tokens newer WHERE newer.session_id=t.session_id AND (newer.expires_at,newer.id)>(t.expires_at,t.id))) ORDER BY t.expires_at,t.id LIMIT $2 FOR UPDATE OF t SKIP LOCKED)`,[n,limit]);
 // A spent refresh hash remains a replay fence for its whole valid lifetime.
 // Only its expired recovery ciphertext is discarded early, never that fence.
 await tx.query('UPDATE refresh_tokens SET recovery_ciphertext=NULL WHERE hash IN (SELECT hash FROM refresh_tokens WHERE spent_at IS NOT NULL AND recovery_until<$1 AND recovery_ciphertext IS NOT NULL ORDER BY recovery_until LIMIT $2 FOR UPDATE SKIP LOCKED)',[n,limit]);
 await tx.query('DELETE FROM refresh_tokens WHERE hash IN (SELECT hash FROM refresh_tokens WHERE expires_at<$1 AND (recovery_until IS NULL OR recovery_until<$1) ORDER BY expires_at,hash LIMIT $2 FOR UPDATE SKIP LOCKED)',[n,limit]);
 // Presence/pulse outcomes have a 30s recovery window; retain their compact
 // fences for 15min. Reapplying such an old observation still requires current
 // session/run/config admission; it is never a Start or transfer authority.
 await tx.query("DELETE FROM command_results WHERE (scope,request_id) IN (SELECT scope,request_id FROM command_results WHERE operation='routine' AND created_at<$1 ORDER BY created_at,scope,request_id LIMIT $2 FOR UPDATE SKIP LOCKED)",[routine,limit]);
 // Rejoin does not call the old base room sweep. Preserve its chat retention
 // policy here too; expired chat replay fences remain for live sessions.
 await tx.query('DELETE FROM room_chat WHERE (room_id,sequence) IN (SELECT room_id,sequence FROM room_chat WHERE sent_at<$1 ORDER BY sent_at LIMIT $2 FOR UPDATE SKIP LOCKED)',[routine,limit]);
 await tx.query("UPDATE command_results SET result='{\"expired\":true}'::jsonb WHERE (scope,request_id) IN (SELECT scope,request_id FROM command_results WHERE operation='chat' AND created_at<$1 AND NOT result ? 'expired' ORDER BY created_at LIMIT $2 FOR UPDATE SKIP LOCKED)",[routine,limit]);
 await tx.query("DELETE FROM command_results WHERE (scope,request_id) IN (SELECT c.scope,c.request_id FROM command_results c JOIN sessions s ON s.id=c.scope JOIN session_families f ON f.id=s.family_id WHERE c.operation='chat' AND c.created_at<$1 AND (s.revoked_at IS NOT NULL OR f.revoked_at IS NOT NULL OR f.expires_at<=$2) ORDER BY c.created_at LIMIT $3 FOR UPDATE OF c SKIP LOCKED)",[routine,n,limit]);
 await tx.query('DELETE FROM auth1_web_sessions WHERE handle_hash IN (SELECT handle_hash FROM auth1_web_sessions WHERE expires_at<$1 ORDER BY expires_at LIMIT $2 FOR UPDATE SKIP LOCKED)',[n,limit]);
 await tx.query('DELETE FROM auth1_admission_sources WHERE source_hash IN (SELECT source_hash FROM auth1_admission_sources WHERE until_at<$1 ORDER BY until_at LIMIT $2 FOR UPDATE SKIP LOCKED)',[handoff,limit]);
 await tx.query('DELETE FROM auth1_login_limits WHERE bucket IN (SELECT bucket FROM auth1_login_limits WHERE until_at<$1 ORDER BY until_at LIMIT $2 FOR UPDATE SKIP LOCKED)',[handoff,limit]);
 // Foundation/Start/transfer/world receipts, session identities and world
 // associations are deliberately untouched. Their durable replay fences differ.
}
