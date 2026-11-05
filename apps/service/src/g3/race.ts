import type {Database,Tx} from '../g1/database.js';
import type {Actor,Auth} from '../g1/auth.js';
import type {RejoinRooms} from '../rejoin/rooms.js';
import {demand,fingerprint,PolicyError} from '../g1/security.js';
import {RaceGoalError} from '../../../../packages/contracts/src/g3-race-goals.js';
import {acceptGoalObservation,rankRaceEntrants,type GoalEvaluation} from '../../../../packages/domain/src/g3-race-goals.js';
import {pureRaceGoal,pureRaceObservation,RACE_CAPABILITY} from '../../../../packages/domain/src/g3-race-config.js';
import type {Request,Response,Snapshot,Entrant,Commit,Receipt} from '../../../../packages/contracts/g3-race/g3-race.js';
import {readRoomAdmissions} from './admission.js';
import {endRace} from './race-store.js';
const uuid=(v:unknown)=>typeof v==='string'&&/^[a-f0-9]{8}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{12}$/.test(v);
export class RaceService {
 constructor(readonly db:Database,readonly auth:Auth,readonly compatibility:any,readonly rooms:RejoinRooms){}
 async close(tx:Tx,room:any,reason:string,at=this.auth.now()){
  await endRace(tx,room.current_run_id,at,reason);
  if(room.state==='running'){await this.rooms.cancel(tx,room,reason,'ended');await tx.query('UPDATE inspected_releases SET revoked_at=$2,reason=$3 WHERE run_id=$1 AND revoked_at IS NULL',[room.current_run_id,new Date(at),reason]);}
 }
 async deadline(tx:Tx,room:any){
  const n=(await tx.query('SELECT * FROM race_runs WHERE run_id=$1 FOR UPDATE',[room.current_run_id])).rows[0];
  if(n&&!n.ended_at){
   if(n.deadline_at&&+n.deadline_at<=this.auth.now())await this.close(tx,room,'race_deadline',+n.deadline_at);
   else if(!(await tx.query("SELECT 1 FROM race_entrants WHERE run_id=$1 AND status='racing'",[n.run_id])).rowCount)await this.close(tx,room,'race_all_terminal');
  }
 }
 async execute(tx:Tx,input:Request,actor?:Actor):Promise<Partial<Response>> {
  try{return input.commit?await this.commit(tx,input,actor):await this.read(tx,input,actor);}
  catch(e){if(e instanceof RaceGoalError)throw new PolicyError(e.code==='LATE_OBSERVATION'?'FORBIDDEN':'INVALID_MESSAGE');throw e;}
 }
 private async commit(tx:Tx,input:Request,actor?:Actor):Promise<Partial<Response>> {
  const s=input.scope,p:Commit={...input.commit!,observations:input.commit?.observations??[]};demand(actor&&['game','desktop'].includes(actor.kind),'UNAUTHENTICATED');
  demand(s&&uuid(s.roomId)&&uuid(s.runId)&&uuid(s.attachmentId)&&uuid(p.eventId)&&Number.isInteger(p.sequence)&&p.sequence!>0&&['none','dead','ending'].includes(p.terminal??'')&&Array.isArray(p.observations)&&p.observations.length<=32,'INVALID_MESSAGE');
  const room=(await tx.query('SELECT * FROM rooms WHERE id=$1 FOR UPDATE',[s!.roomId])).rows[0];demand(room&&!room.deleted_at,'NOT_FOUND');
  const run=(await tx.query('SELECT d.*,n.config_revision FROM race_runs d JOIN runs n ON n.id=d.run_id WHERE d.run_id=$1 AND d.room_id=$2 FOR UPDATE OF d',[s!.runId,s!.roomId])).rows[0];demand(run&&run.config_revision===s!.configRevision,'FORBIDDEN');
  const entrant=(await tx.query('SELECT * FROM race_entrants WHERE run_id=$1 AND account_id=$2 FOR UPDATE',[s!.runId,actor!.accountId])).rows[0];
  demand(entrant&&entrant.world_token===s!.worldToken&&entrant.device_id===actor!.deviceId,'FORBIDDEN');
  const hash=fingerprint(p),old=(await tx.query('SELECT * FROM race_receipts WHERE run_id=$1 AND world_token=$2 AND event_id=$3',[s!.runId,s!.worldToken,p.eventId])).rows[0];
  if(old){demand(old.original_hash===hash,'CONFLICT');return {receipt:old.result};}
  // Committed canonical deadline closure is performed in the request preflight
  // and timer housekeeping. This check cannot backdate a delayed new original.
  demand(!run.ended_at&&room.current_run_id===s!.runId&&room.state==='running'&&entrant.status==='racing'&&(!run.deadline_at||+run.deadline_at>this.auth.now()),'FORBIDDEN');
  demand(p.sequence===Number(entrant.last_sequence)+1,'SEQUENCE_GAP');demand(run.event_count<65536,'CAPACITY');
  demand((await tx.query('SELECT 1 FROM race_receipts WHERE run_id=$1 AND world_token=$2 AND sequence=$3',[s!.runId,s!.worldToken,p.sequence])).rowCount===0,'CONFLICT');
  const admission=await readRoomAdmissions(tx,this.db,this.compatibility,this.auth.now(),room.id);
  const w=admission.worlds.find(w=>w.scope.principalId===actor!.accountId&&w.scope.worldToken===s!.worldToken&&w.owner.attachmentId===s!.attachmentId);
  demand(w&&w.mode==='race'&&w.capabilities.includes(RACE_CAPABILITY)&&w.generation===input.ownerGeneration,'STALE_EPOCH');
  demand(!w!.decision.terminal,'FORBIDDEN');
  const currentScope={roomId:w!.scope.roomId,runId:w!.scope.runId,runEpoch:w!.scope.runEpoch,configRevision:w!.scope.configRevision,attachmentId:w!.scope.attachmentId,worldToken:w!.scope.worldToken};
  if(actor!.kind==='game'){demand(fingerprint(s)===fingerprint(currentScope),'STALE_EPOCH');demand(w!.decision.playing&&w!.owner.gameSessionId===actor!.sessionId,'FORBIDDEN');}
  else {
   // A retained desktop journal does not need a live actor. It needs the exact
   // current owner generation, admitted world and previous entered-play record;
   // explicit offline/removal/successor fencing cannot use this path.
   demand(['roomId','runId','configRevision','attachmentId','worldToken'].every(k=>(s as any)[k]===(currentScope as any)[k]),'STALE_EPOCH');
   const retained=(await tx.query(`SELECT 1 FROM rejoin_participants p JOIN attachments a ON a.id=p.attachment_id
    JOIN runs n ON n.id=p.run_id JOIN sessions d ON d.id=p.desktop_id JOIN session_families old_family ON old_family.id=d.family_id
    WHERE p.run_id=$1 AND p.account_id=$2 AND p.world_token=$3 AND p.generation=$5 AND NOT p.terminal
     AND p.device_id=$6 AND a.id=$7 AND a.run_epoch=$8 AND n.state='running'
     AND (d.id=$4 AND d.revoked_at IS NULL OR d.revoked_at IS NOT NULL AND old_family.revoked_at IS NOT NULL)
     AND NOT EXISTS(SELECT 1 FROM world_associations w WHERE w.attachment_id=a.id AND w.high_water>w.owner_attempt)
     AND EXISTS(SELECT 1 FROM authorization_play_entries e WHERE e.attachment_id=a.id)`,[s!.runId,actor!.accountId,s!.worldToken,actor!.sessionId,input.ownerGeneration,actor!.deviceId,s!.attachmentId,s!.runEpoch])).rowCount;
   demand(retained,'FORBIDDEN');
  }
  const elapsed=Math.max(0,this.auth.now()-Number(new Date(run.started_at)));
  demand(!run.deadline_at||elapsed<Number(run.deadline_at)-Number(run.started_at),'FORBIDDEN');
  let evaluation=entrant.evaluation as GoalEvaluation;
  for(const o of p.observations!)evaluation=acceptGoalObservation(evaluation,evaluation.scope,pureRaceObservation(o),elapsed);
  const finished=evaluation.progress.every(v=>v.complete),ending=p.terminal==='ending'||p.observations!.some(o=>o.kind==='ending');
  const status=finished?'finished':p.terminal==='dead'?'dead':ending?'dnf':'racing';
  const finishOrder=finished?Number((await tx.query('SELECT count(*) FROM race_entrants WHERE run_id=$1 AND finish_order IS NOT NULL',[s!.runId])).rows[0].count)+1:null;
  await tx.query('UPDATE race_entrants SET evaluation=$3,last_sequence=$4,status=$5,finish_order=$6 WHERE run_id=$1 AND account_id=$2',[s!.runId,actor!.accountId,evaluation,p.sequence,status,finishOrder]);
  const revision=String((await tx.query('UPDATE race_runs SET revision=revision+1,event_count=event_count+1 WHERE run_id=$1 RETURNING revision',[s!.runId])).rows[0].revision);
  const receipt:Receipt={eventId:p.eventId!,sequence:p.sequence,acceptedAtMs:String(elapsed),revision,status,completedGoals:evaluation.progress.filter(v=>v.complete).length,finishOrder:finishOrder??0};
  await tx.query('INSERT INTO race_receipts(run_id,world_token,event_id,sequence,original_hash,result) VALUES($1,$2,$3,$4,$5,$6)',[s!.runId,s!.worldToken,p.eventId,p.sequence,hash,receipt]);
  if(status!=='racing'){
   const participant=(await tx.query('SELECT id FROM rejoin_participants WHERE run_id=$1 AND account_id=$2 AND world_token=$3',[s!.runId,actor!.accountId,s!.worldToken])).rows[0];
   if(participant)await this.rooms.recovery.retire(tx,participant.id,'race_'+status,'ended');
   await tx.query('UPDATE rooms SET revision=revision+1 WHERE id=$1',[room.id]);
   if(!(await tx.query("SELECT 1 FROM race_entrants WHERE run_id=$1 AND status='racing'",[s!.runId])).rowCount)await this.close(tx,room,'race_all_terminal');
  }
  return {receipt};
 }
 private async read(tx:Tx,input:Request,actor?:Actor):Promise<Partial<Response>> {
  const p=input.read!;demand(p&&uuid(p.locator)&&(p.limit??8)>=1&&(p.limit??8)<=8&&(!p.afterRow||uuid(p.afterRow)),'INVALID_MESSAGE');
  const room=(await tx.query('SELECT * FROM rooms WHERE stats_locator=$1 AND deleted_at IS NULL',[p.locator])).rows[0];demand(room,'NOT_FOUND');
  const run=(await tx.query('SELECT * FROM race_runs WHERE room_id=$1 ORDER BY started_at DESC,run_id DESC LIMIT 1',[room.id])).rows[0];
  const member=actor&&!!(room.host_account_id===actor.accountId||(await tx.query('SELECT 1 FROM memberships WHERE room_id=$1 AND account_id=$2 UNION ALL SELECT 1 FROM race_entrants WHERE run_id=$3 AND account_id=$2',[room.id,actor.accountId,run?.run_id??null])).rowCount);
  demand(room.visibility==='public'&&room.public_stats||member,'NOT_FOUND');
  if(actor&&room.host_account_id!==actor.accountId)demand(!(await tx.query('SELECT 1 FROM room_bans WHERE room_id=$1 AND account_id=$2',[room.id,actor.accountId])).rowCount,'NOT_FOUND');
  const snapshot:Snapshot={locator:p.locator!,lobbyName:room.name,lobbyMode:room.config.mode,collectionState:'none'};
  if(!run)return {snapshot};
  Object.assign(snapshot,{runKey:run.run_key,revision:String(run.revision),status:run.ended_at?'ended':'running',registryRevision:run.registry_revision,startedAt:run.started_at.toISOString(),deadlineAt:run.deadline_at?.toISOString()??'',endedAt:run.ended_at?.toISOString()??'',elapsedMs:String(Math.max(0,Number(run.ended_at??new Date(this.auth.now()))-Number(run.started_at))),endReason:run.end_reason,collectionState:run.expires_at&&+run.expires_at<=this.auth.now()?'expired':'available'});
  if(snapshot.collectionState==='expired')return {snapshot};snapshot.config=run.config;
  const entries=(await tx.query('SELECT * FROM race_entrants WHERE run_id=$1 ORDER BY row_key',[run.run_id])).rows;
  const ranking=rankRaceEntrants(run.config.goals.map(pureRaceGoal),entries.map(e=>({participantId:e.account_id,status:e.status,finishOrder:e.finish_order,completions:(e.evaluation as GoalEvaluation).progress.filter(v=>v.complete).map(v=>({instanceId:v.instanceId,elapsedMs:v.completedAtMs!}))})),undefined,run.config.durationMs||null);
  snapshot.selfRow=entries.find(e=>e.account_id===actor?.accountId)?.row_key??'';
  if(p.afterRow)demand(p.runKey===run.run_key&&p.snapshotRevision===String(run.revision)&&entries.some(e=>e.row_key===p.afterRow),'STALE_REVISION');
  const page=entries.filter(e=>(!p.self||e.account_id===actor?.accountId)&&(!p.afterRow||e.row_key>p.afterRow)).slice(0,(p.limit??8)+1);
  snapshot.entrants=page.slice(0,p.limit??8).map(e=>{
   const rank=ranking.find(v=>v.participantId===e.account_id)!;
   return {rowKey:e.row_key,displayName:e.display_name,status:e.status,acceptedSequence:Number(e.last_sequence),completedGoals:rank.completedGoals,rank:rank.rank,finishOrder:e.finish_order??0,finalCountAtMs:rank.finalCountAtMs===null?'':String(rank.finalCountAtMs),progress:(e.evaluation as GoalEvaluation).progress.map(v=>({instanceId:v.instanceId,value:String(v.value),currentValue:String(v.currentValue),target:String(v.target),complete:v.complete,completedAtMs:v.completedAtMs===null?'':String(v.completedAtMs),credits:v.credits.map(c=>({id:c.id,firstAtMs:String(c.firstAtMs)}))}))} satisfies Entrant;
  });snapshot.nextRow=page.length>(p.limit??8)?page[(p.limit??8)-1].row_key:'';
  return {snapshot};
 }
}
