import {operationFingerprint} from '../../../../packages/contracts/src/connection-fingerprint.js';
import type {Tx} from '../g1/database.js';
import type {Actor} from '../g1/auth.js';
import {demand,fingerprint} from '../g1/security.js';
import type {OriginalOperation} from '../../../../packages/contracts/g3-connection/g3-connection.js';
export const OUTCOME_RETENTION_MS=30*86400000,OUTCOME_RECORD_LIMIT=1024;
export function originalOperation(actor:Actor,p:any,req:any):OriginalOperation{
 return {requestId:req.requestId,kind:req.resume?'resume':'offline',fingerprint:operationFingerprint(req),originalSessionId:actor.sessionId,originalSessionEpoch:actor.sessionEpoch,originalOwnerGeneration:req.generation,participantId:p.id,roomId:p.room_id,runId:p.run_id,worldToken:p.world_token,configRevision:p.config_revision,originalServiceEpoch:req.serviceEpoch,accountId:actor.accountId,deviceId:actor.deviceId};
}
// Must run under the same participant/room lock and transaction as the mutation.
// Bounds apply before a new mutation can commit, never silently evict live evidence.
export async function reserveOutcome(tx:Tx,original:OriginalOperation,now:number){
 await tx.query('DELETE FROM connection_outcomes WHERE participant_id=$1 AND expires_at<=$2',[original.participantId,new Date(now)]);
 const old=(await tx.query('SELECT original FROM connection_outcomes WHERE participant_id=$1 AND request_id=$2 AND kind=$3',[original.participantId,original.requestId,original.kind])).rows[0];
 if(old){demand(fingerprint(old.original)===fingerprint(original),'CONFLICT');return;}
 demand(+(await tx.query('SELECT count(*) FROM connection_outcomes WHERE participant_id=$1',[original.participantId])).rows[0].count<OUTCOME_RECORD_LIMIT,'CAPACITY');
}
export async function recordOutcome(tx:Tx,original:OriginalOperation,effectId:string,now:number){
 await tx.query('INSERT INTO connection_outcomes(participant_id,request_id,kind,original,fingerprint,effect_id,created_at,expires_at) VALUES($1,$2,$3,$4,$5,$6,$7,$8) ON CONFLICT DO NOTHING',[original.participantId,original.requestId,original.kind,original,original.fingerprint,effectId,new Date(now),new Date(now+OUTCOME_RETENTION_MS)]);
}
export async function readOutcome(tx:Tx,actor:Actor,original:OriginalOperation,now:number){
 demand(original.accountId===actor.accountId&&original.deviceId===actor.deviceId&&actor.kind==='desktop','FORBIDDEN');
 demand(!!original.kind&&/^[a-f0-9]{64}$/.test(original.fingerprint??'')&&/^[A-Za-z0-9_-]{1,128}$/.test(original.requestId??'')&&original.originalOwnerGeneration!>0&&original.configRevision!>0,'INVALID_MESSAGE');
 const p=(await tx.query(`SELECT p.*,k.room_id,k.config_revision,k.terminal_reason,n.state AS run_state,r.current_run_id,r.allow_guests,u.disabled,q.kind,q.guest_expires_at,
 EXISTS(SELECT 1 FROM room_bans b WHERE b.room_id=r.id AND b.account_id=p.account_id) AS banned
 FROM rejoin_participants p JOIN rejoin_runs k ON k.run_id=p.run_id JOIN runs n ON n.id=p.run_id JOIN rooms r ON r.id=k.room_id JOIN accounts u ON u.id=p.account_id JOIN principal_profiles q ON q.principal_id=p.account_id WHERE p.id=$1`,[original.participantId])).rows[0];
 demand(p&&p.account_id===actor.accountId&&p.device_id===actor.deviceId&&p.run_id===original.runId&&p.room_id===original.roomId&&p.world_token===original.worldToken&&p.config_revision===original.configRevision,'FORBIDDEN');
 const row=(await tx.query('SELECT * FROM connection_outcomes WHERE participant_id=$1 AND request_id=$2 AND kind=$3',[p.id,original.requestId,original.kind])).rows[0];
 const exact=row&&fingerprint(row.original)===fingerprint(original)&&row.fingerprint===original.fingerprint&&+row.expires_at>now;
 const terminal=p.terminal||p.terminal_reason||p.current_run_id!==p.run_id||p.run_state!=='running'||p.disabled||p.banned||p.kind==='guest'&&(!p.allow_guests||+p.guest_expires_at<=now);
 return {original,originalOutcome:exact?'committed':'unresolved',coverage:exact?'exact_durable_record':['resume','offline'].includes(original.kind!)?'absent_expired_or_uncovered':'kind_not_covered',currentDisposition:terminal?'terminal':p.state,currentReason:p.terminal_reason??p.reason,currentGeneration:p.generation,...(exact?{effectId:row.effect_id}:{}),retryAfterMs:0};
}
