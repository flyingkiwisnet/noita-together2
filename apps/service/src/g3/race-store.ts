import type {Tx} from '../g1/database.js';
import {demand} from '../g1/security.js';
import {createGoalEvaluation} from '../../../../packages/domain/src/g3-race-goals.js';
import {validateRaceConfig,pureRaceGoal} from '../../../../packages/domain/src/g3-race-config.js';
export async function startRace(tx:Tx,runId:string){
 const n=(await tx.query('SELECT n.*,x.authorized_at,x.selected_reports FROM runs n JOIN inspected_releases x ON x.run_id=n.id WHERE n.id=$1 AND n.state=\'running\' AND x.revoked_at IS NULL',[runId])).rows[0];
 if(!n||n.config.mode!=='race')return;
 const c=validateRaceConfig(n.config)!;
 const inserted=await tx.query('INSERT INTO race_runs(run_id,room_id,config,registry_revision,started_at,deadline_at) VALUES($1,$2,$3,$4,$5,$6) ON CONFLICT DO NOTHING RETURNING run_id',[runId,n.room_id,c,c.registryRevision,n.authorized_at,c.durationMs?new Date(+new Date(n.authorized_at)+c.durationMs):null]);
 if(!inserted.rowCount)return;
 const accounts=new Set<string>();
 for(const report of n.selected_reports.participants){
  const a=(await tx.query('SELECT f.account_id,f.device_id,p.display_name FROM attachments a JOIN sessions s ON s.id=a.game_session_id JOIN session_families f ON f.id=s.family_id JOIN principal_profiles p ON p.principal_id=f.account_id WHERE a.id=$1',[report.attachmentId])).rows[0];
  demand(a&&!accounts.has(a.account_id),'ROSTER_MISMATCH');accounts.add(a.account_id);
  const evaluation=createGoalEvaluation({roomId:n.room_id,runId,runEpoch:n.epoch,configRevision:n.config_revision,participantId:a.account_id,worldToken:report.worldToken},c.goals!.map(pureRaceGoal),c.durationMs||null);
  await tx.query('INSERT INTO race_entrants(run_id,account_id,world_token,device_id,display_name,evaluation) VALUES($1,$2,$3,$4,$5,$6)',[runId,a.account_id,report.worldToken,a.device_id,a.display_name,evaluation]);
 }
 demand(accounts.size>0&&accounts.size<=32,'ROSTER_MISMATCH');
}
export async function endRace(tx:Tx,runId:string,now:number,reason:string){
 const changed=await tx.query('UPDATE race_runs SET ended_at=$2,end_reason=$3,expires_at=$4,revision=revision+1 WHERE run_id=$1 AND ended_at IS NULL RETURNING run_id',[runId,new Date(now),reason,new Date(now+30*86400000)]);
 if(changed.rowCount)await tx.query("UPDATE race_entrants SET status='dnf' WHERE run_id=$1 AND status='racing'",[runId]);
}
export async function purgeRace(tx:Tx,now:number){
 const rows=(await tx.query('SELECT run_id FROM race_runs WHERE expires_at<=$1 AND purged_at IS NULL ORDER BY expires_at LIMIT 8 FOR UPDATE',[new Date(now)])).rows;
 for(const r of rows){await tx.query('UPDATE race_entrants SET evaluation=\'{}\'::jsonb,display_name=\'\',finish_order=NULL WHERE run_id=$1',[r.run_id]);await tx.query('UPDATE race_runs SET purged_at=$2,config=\'{}\'::jsonb WHERE run_id=$1',[r.run_id,new Date(now)]);}
 // Minimal immutable receipt hashes/results remain operational originals, not
 // presentation history, exactly as existing gameplay replay records do.
}
