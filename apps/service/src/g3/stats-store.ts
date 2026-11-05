import type {Tx} from '../g1/database.js';
import {demand,fingerprint} from '../g1/security.js';
import {C1_CATALOGUE} from '../../../../packages/domain/src/g3-c1.js';
export const STAT_METRICS={heart:'hearts',orb:'orbs',steve:'steve_kills',death:'deaths',win:'wins'} as const;
export const STAT_COLUMNS=['hearts','orbs','steve_kills','deaths','wins','gold_deposited','gold_withdrawn','items_deposited','items_claimed'] as const;
function observationOriginal(input:{kind:string;progression?:number;orbId?:string}){
 const kind=input.kind as keyof typeof STAT_METRICS;demand(Object.hasOwn(STAT_METRICS,kind),'INVALID_MESSAGE');
 const progression=input.progression??0;demand(Number.isInteger(progression)&&progression>=0&&progression<=0xffffffff,'INVALID_MESSAGE');
 if(kind==='orb'){demand(/^(0|[1-9][0-9]{0,19})$/.test(input.orbId??''),'INVALID_MESSAGE');demand(C1_CATALOGUE.orbs.some(o=>String(o.id)===input.orbId),'UNSUPPORTED_CONTENT');}else demand(input.orbId===undefined,'INVALID_MESSAGE');
 return {kind,...(['orb','win','steve'].includes(kind)?{progression}:{}),...(kind==='orb'?{orbId:input.orbId}:{})};
}
export async function knownStatisticReceipt(tx:Tx,source:{runId:string;worldToken:string;principalId:string;deviceId:string},input:{eventId:string;kind:string;progression?:number;orbId?:string}){
 const original=observationOriginal(input),old=(await tx.query('SELECT * FROM d1_events WHERE run_id=$1 AND world_token=$2 AND event_id=$3',[source.runId,source.worldToken,input.eventId])).rows[0];
 if(!old)return;
 demand(old.principal_id===source.principalId&&old.device_id===source.deviceId,'FORBIDDEN');demand(fingerprint(old.original)===fingerprint(original),'CONFLICT');return {accepted:true,duplicate:true};
}
export async function startStatistics(tx:Tx,runId:string){
 const start=(await tx.query(`INSERT INTO d1_runs(run_id,room_id,started_at)
  SELECT n.id,n.room_id,x.authorized_at FROM runs n JOIN inspected_releases x ON x.run_id=n.id
  WHERE n.id=$1 AND n.state='running' AND x.revoked_at IS NULL ON CONFLICT DO NOTHING RETURNING *`,[runId])).rows[0];
 if(!start)return;
 await tx.query(`INSERT INTO d1_players(run_id,principal_id,display_name)
  SELECT DISTINCT $1::uuid,f.account_id,p.display_name FROM inspected_releases x
  CROSS JOIN LATERAL jsonb_array_elements(x.selected_reports->'participants') selected
  JOIN attachments a ON a.id=(selected->>'attachmentId')::uuid
  JOIN sessions s ON s.id=a.game_session_id JOIN session_families f ON f.id=s.family_id
  JOIN principal_profiles p ON p.principal_id=f.account_id WHERE x.run_id=$1 ON CONFLICT DO NOTHING`,[runId]);
}
export async function endStatistics(tx:Tx,runId:string,now:number){await tx.query('UPDATE d1_runs SET ended_at=$2,expires_at=$3,revision=revision+1 WHERE run_id=$1 AND ended_at IS NULL',[runId,new Date(now),new Date(now+30*86400000)]);}
export async function addStatistic(tx:Tx,runId:string,accountId:string,column:string,amount:number|string){
 demand(STAT_COLUMNS.includes(column as any),'INVALID_MESSAGE');
 if(!(await tx.query("SELECT to_regclass('d1_runs') AS present")).rows[0].present)return;
 const active=(await tx.query('SELECT run_id FROM d1_runs WHERE run_id=$1 AND ended_at IS NULL',[runId])).rowCount;if(!active)return;
 await tx.query(`INSERT INTO d1_players(run_id,principal_id,display_name) SELECT $1,p.principal_id,p.display_name FROM principal_profiles p WHERE p.principal_id=$2 AND (SELECT count(*) FROM d1_players WHERE run_id=$1)<32 ON CONFLICT DO NOTHING`,[runId,accountId]);
 const result=await tx.query(`UPDATE d1_players SET ${column}=${column}+$3::bigint WHERE run_id=$1 AND principal_id=$2 AND ${column}<=9223372036854775807-$3::bigint`,[runId,accountId,String(amount)]);
 if(result.rowCount)await tx.query('UPDATE d1_runs SET revision=revision+1 WHERE run_id=$1',[runId]);
 else await tx.query('UPDATE d1_runs SET complete=false,revision=revision+1 WHERE run_id=$1',[runId]);
}
export async function recordGameplayStatistic(tx:Tx,source:{runId:string;worldToken:string;principalId:string;deviceId:string},input:{eventId:string;kind:string;progression?:number;orbId?:string}){
 const original=observationOriginal(input),kind=original.kind,progression=input.progression??0,known=await knownStatisticReceipt(tx,source,input);if(known)return known;
 const run=(await tx.query('SELECT * FROM d1_runs WHERE run_id=$1 FOR UPDATE',[source.runId])).rows[0];
 // Legacy runs lack complete zero initialization. Never invent missing history.
 if(!run)return {accepted:false,duplicate:false};
 demand(!run.ended_at,'FORBIDDEN');
 if(run.event_count>=65536){await tx.query('UPDATE d1_runs SET complete=false,revision=revision+1 WHERE run_id=$1 AND complete',[source.runId]);return {accepted:false,duplicate:false};}
 const key=kind==='win'?String(progression):kind==='orb'?progression+':'+input.orbId:input.eventId;
 const seen=!!(await tx.query('SELECT 1 FROM d1_events WHERE run_id=$1 AND world_token=$2 AND kind=$3 AND event_key=$4 AND counted',[source.runId,source.worldToken,kind,key])).rowCount;
 await tx.query('INSERT INTO d1_events(run_id,world_token,event_id,principal_id,device_id,kind,event_key,original,counted) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9)',[source.runId,source.worldToken,input.eventId,source.principalId,source.deviceId,kind,key,original,!seen]);
 await tx.query('UPDATE d1_runs SET event_count=event_count+1 WHERE run_id=$1',[source.runId]);
 if(!seen)await addStatistic(tx,source.runId,source.principalId,STAT_METRICS[kind],1);
 return {accepted:true,duplicate:seen};
}
export async function validateObservationSource(tx:Tx,source:{runId:string;worldToken:string;principalId:string;deviceId:string},input:{eventId:string;kind:string;progression?:number;orbId?:string}){
 const c1=(await tx.query('SELECT * FROM c1_events WHERE run_id=$1 AND source_world=$2 AND event_id=$3',[source.runId,source.worldToken,input.eventId])).rows[0];
 if(c1){const p=c1.body.payload,kind=p.heart?'heart':p.orb?'orb':p.death?(p.death.state==='won'?'win':'death'):'other';demand(c1.source_principal===source.principalId&&(!c1.source_device||c1.source_device===source.deviceId)&&kind===input.kind,'CONFLICT');if(p.orb)demand(String(p.orb.orbId)===input.orbId,'CONFLICT');if(kind==='win')demand((p.death.progression??0)===(input.progression??0),'CONFLICT');}
 const c2=(await tx.query('SELECT * FROM c2_events WHERE run_id=$1 AND source_world=$2 AND event_id=$3',[source.runId,source.worldToken,input.eventId])).rows[0];
 if(c2){const p=c2.body.payload;demand(c2.source_principal===source.principalId&&(!c2.source_device||c2.source_device===source.deviceId)&&!!p.completed&&input.kind==='win'&&p.completed.progression===(input.progression??0),'CONFLICT');}
}
export async function recordTerminalStatistic(tx:Tx,source:{runId:string;worldToken:string;principalId:string;deviceId:string},input:{eventId:string;kind:'death'|'win';progression?:number}){
 const old=(await tx.query('SELECT * FROM d1_events WHERE run_id=$1 AND world_token=$2 AND event_id=$3',[source.runId,source.worldToken,input.eventId])).rows[0];
 const original=input.kind==='win'?{kind:'win',progression:input.progression??0}:{kind:'death'};
 if(old&&(old.principal_id!==source.principalId||old.device_id!==source.deviceId||fingerprint(old.original)!==fingerprint(original))){
  // Optional observations never become a veto over a canonical life transition.
  // Keep the conflicting receipt pinned and label its data incomplete.
  await tx.query('UPDATE d1_runs SET complete=false,revision=revision+1 WHERE run_id=$1 AND complete',[source.runId]);return {accepted:false,duplicate:false};
 }
 return recordGameplayStatistic(tx,source,input);
}
export async function c1Statistic(tx:Tx,event:any){
 // Pickups use the actual-local-observation path whether sharing is on or off.
 // Publication/recipient/source-effect receipts are never another collector.
 // Terminal originals must be counted here before their authority is revoked.
 const p=event.body.payload;if(!p.death)return;const kind=p.death.state==='won'?'win':'death';
 if(!(await tx.query('SELECT 1 FROM d1_runs WHERE run_id=$1 AND ended_at IS NULL',[event.run_id])).rowCount)return;
 return recordTerminalStatistic(tx,{runId:event.run_id,worldToken:event.source_world,principalId:event.source_principal,deviceId:event.source_device},{eventId:event.event_id,kind,progression:p.death.progression??0});
}
export async function maintainStatistics(tx:Tx,now:number){
 const runs=(await tx.query('SELECT run_id,room_id FROM d1_runs WHERE expires_at<=$1 AND purged_at IS NULL ORDER BY expires_at LIMIT 1 FOR UPDATE',[new Date(now)])).rows;
 for(const r of runs){await tx.query('DELETE FROM d1_events WHERE (run_id,world_token,event_id) IN (SELECT run_id,world_token,event_id FROM d1_events WHERE run_id=$1 LIMIT 512)',[r.run_id]);if((await tx.query('SELECT 1 FROM d1_events WHERE run_id=$1 LIMIT 1',[r.run_id])).rowCount)continue;await tx.query('DELETE FROM d1_players WHERE run_id=$1',[r.run_id]);await tx.query('UPDATE d1_runs SET purged_at=$2 WHERE run_id=$1',[r.run_id,new Date(now)]);if((await tx.query('SELECT 1 FROM rooms WHERE id=$1 AND deleted_at IS NOT NULL',[r.room_id])).rowCount)await tx.query('DELETE FROM d1_runs WHERE run_id=$1',[r.run_id]);}
}
