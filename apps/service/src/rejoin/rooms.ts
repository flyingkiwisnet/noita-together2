import {maintainAuthentication} from '../auth1/retention.js';
import {startRace,endRace,purgeRace} from '../g3/race-store.js';
import type {RaceService} from '../g3/race.js';
import {startStatistics,endStatistics,maintainStatistics} from '../g3/stats-store.js';
import type {Tx} from '../g1/database.js';
import type {Actor} from '../g1/auth.js';
import type {Request,Response,GameObservation} from '../../../../packages/contracts/g2-next/g2-next.js';
import {markBankDirty} from '../g3/bank-events.js';
import {touchC1Room} from '../g3/c1.js';
import {G3CandidateRooms,synchronizeGameplay} from '../g3/lifecycle.js';
import {InspectionAuthority} from '../g2-next/inspection.js';
import {RejoinAuthority} from './authority.js';
import {demand} from '../g1/security.js';
import {validate,schema} from '../../../../packages/contracts/src/g2-next.js';
import {operationFingerprint} from '../../../../packages/contracts/src/connection-fingerprint.js';
import {contentFailure} from '../g2-next/observation.js';

class RejoinInspection extends InspectionAuthority {
 constructor(readonly recovery:RejoinAuthority){super(recovery.db,recovery.auth,recovery.compatibility);}
 async capLifecycleLease(tx:Tx,attachmentId?:string){
  await tx.query('UPDATE attachments a SET presence_until=LEAST(a.presence_until,w.supersede_after),resume_until=LEAST(a.resume_until,w.supersede_after) FROM world_associations w WHERE w.attachment_id=a.id AND a.id=$1 AND w.high_water>w.owner_attempt AND w.supersede_after IS NOT NULL',[attachmentId]);
 }
 async participant(tx:Tx,attachmentId?:string){const id=(await tx.query('SELECT id FROM rejoin_participants WHERE attachment_id=$1',[attachmentId])).rows[0]?.id;return id?this.recovery.read(tx,id):undefined;}
 override async attach(tx:Tx,actor:Actor,req:Request):Promise<NonNullable<Response['credentials']>>{demand(false,'LIFECYCLE_REQUIRED');throw Error('LC1 requires reconcile');}
 override async observe(tx:Tx,actor:Actor,req:Request,input:GameObservation){
  const reserved=(await tx.query('SELECT z.id,z.intent,z.bound_seed,z.frozen_config,z.offline FROM startup_pending z JOIN attachments a ON a.preparation_id=z.preparation_id WHERE a.id=$1',[input.attachmentId])).rows[0];
  if(reserved)demand(!reserved.offline&&reserved.intent.worldToken===input.worldToken&&(!(reserved.bound_seed??reserved.intent.actualSeed)||(reserved.bound_seed??reserved.intent.actualSeed)===input.actualSeed),'REPORT_CHANGED');
  const p=await this.participant(tx,input.attachmentId),result=p?await this.recovery.observe(tx,p,actor,input):await super.observe(tx,actor,req,input);
  if(reserved&&input.protected&&input.loaded&&input.inventoryComplete&&!contentFailure(input,reserved.frozen_config,this.compatibility))await tx.query('UPDATE startup_pending SET bound_seed=COALESCE(bound_seed,$2) WHERE id=$1',[reserved.id,input.actualSeed]);await this.capLifecycleLease(tx,input.attachmentId);return result;
 }
 override async pulse(tx:Tx,actor:Actor,req:Request){const p=await this.participant(tx,req.gamePulse?.attachmentId);if(!p)await super.pulse(tx,actor,req);else{const a=await this.recovery.owner(tx,p,actor,p.generation);demand(req.gamePulse?.worldToken===p.world_token&&req.gamePulse?.gameInstanceId===a.game_instance_id,'FORBIDDEN');await this.recovery.lease(tx,p);}await this.capLifecycleLease(tx,req.gamePulse?.attachmentId);}
 override async binding(tx:Tx,report:any,r:any,run:any){const p=await this.participant(tx,report.attachmentId);if(p&&(p.terminal||p.state!=='active'))return;return super.binding(tx,report,r,p?{...run,review_until:new Date(this.now+30000)}:run);}
 override async status(tx:Tx,actor:Actor,req:Request):Promise<Partial<Response>>{
  const id=(await tx.query('SELECT id FROM attachments WHERE game_session_id=$1',[actor.sessionId])).rows[0]?.id,p=await this.participant(tx,id);if(!p)return super.status(tx,actor,req);
  demand(req.serviceEpoch===this.db.epoch&&req.sessionEpoch===actor.sessionEpoch&&req.roomId===p.room_id&&req.runId===p.run_id&&req.runEpoch===p.run_epoch&&req.configRevision===p.config_revision,'STALE_EPOCH');
  if(p.terminal)return {gameStatus:{phase:'revoked',reason:p.reason}};
  const release=await this.recovery.release(tx,p);return release?{gameStatus:{phase:release.phase,reason:''},release}:{gameStatus:{phase:'hold',reason:p.state==='review'?'report_changed':'reconnecting'}};
 }
 override async cancelPreparation(tx:Tx,actor:Actor,req:Request){
  demand(!(await tx.query('SELECT 1 FROM world_associations WHERE preparation_id=$1',[req.preparationCancel?.preparationId])).rowCount,'LIFECYCLE_REQUIRED');
  // Original preparation commands carry no waiting generation. Once ownership
  // was replaced they cannot cancel it; the current waiting proof/generation
  // command (or authoritative host End/cancel) is required instead.
  const waiting=(await tx.query('SELECT generation FROM waiting_worlds WHERE preparation_id=$1',[req.preparationCancel?.preparationId])).rows[0];
  demand(!waiting||waiting.generation===1,'STALE_EPOCH');
  const p=(await tx.query('SELECT p.id,p.generation,a.preparation_id FROM rejoin_participants p JOIN attachments a ON a.id=p.attachment_id WHERE p.desktop_id=$1 AND p.run_id=$2',[actor.sessionId,req.runId])).rows[0];
  if(!p)return super.cancelPreparation(tx,actor,req);demand(p.generation===1&&p.preparation_id===req.preparationCancel?.preparationId,'STALE_EPOCH');const current=await this.recovery.read(tx,p.id);demand(current&&!current.terminal&&current.current_run_id===req.runId&&current.run_state==='running'&&req.roomId===current.room_id&&req.runEpoch===current.run_epoch&&req.configRevision===current.config_revision,'STALE_EPOCH');demand(actor.kind==='desktop'&&req.serviceEpoch===this.db.epoch&&req.sessionEpoch===actor.sessionEpoch,'STALE_EPOCH');await this.recovery.retire(tx,p.id,'explicit_cancel','offline');
 }
}
export class RejoinRooms extends G3CandidateRooms {
 readonly recovery:RejoinAuthority;
 race:RaceService|undefined;
 override async refreshRoster(tx:Tx,r:Record<string,any>){
  // Revocation plus a live replacement on this exact device is definitive;
  // missing presence, another device or another account never retires a row.
  // Retain the row's consent/selected identity for proof-bound world recovery.
  // Only the already-authorized current room is touched, never another lobby.
  const retired=await tx.query(`UPDATE memberships m SET roster_retired=true
   FROM sessions d JOIN session_families f ON f.id=d.family_id
   WHERE m.room_id=$1 AND m.desktop_id=d.id AND NOT m.roster_retired
    AND d.kind='desktop' AND d.revoked_at IS NOT NULL AND f.revoked_at IS NOT NULL
    AND EXISTS(SELECT 1 FROM session_families replacement JOIN sessions s ON s.family_id=replacement.id
     WHERE replacement.account_id=f.account_id AND replacement.device_id=f.device_id
      AND replacement.id<>f.id AND replacement.kind='desktop' AND replacement.revoked_at IS NULL
      AND replacement.expires_at>$2 AND s.kind='desktop' AND s.revoked_at IS NULL)`,[r.id,new Date(this.now)]);
  if(retired.rowCount){await tx.query('UPDATE rooms SET revision=revision+1 WHERE id=$1',[r.id]);r.revision++;await touchC1Room(tx,r.id);}
 }
 protected override async testingStartingLoadoutConfigMutable(tx:Tx,room:Record<string,any>){return await super.testingStartingLoadoutConfigMutable(tx,room)&&!(await tx.query('SELECT 1 FROM world_creation_intents WHERE run_id=$1 LIMIT 1',[room.current_run_id])).rowCount;}
 private housekeepingCursor='';private housekeeping:Promise<void>|undefined;
 constructor(...args:ConstructorParameters<typeof G3CandidateRooms>){super(...args);this.recovery=new RejoinAuthority(this.db,this.principalAuth,this.compatibility);const inspection=new RejoinInspection(this.recovery);inspection.roomBudget=(id,tx)=>this.roomBudget?.(id,tx);Object.defineProperty(this,'inspection',{value:inspection});}
 override async snapshot(tx:Tx,r:any){const room=await super.snapshot(tx,r);const participants=(await tx.query("SELECT p.desktop_id,p.state,p.reason,w.body->>'protected'='true' AS protected FROM rejoin_participants p LEFT JOIN world_observations w ON w.attachment_id=p.attachment_id WHERE p.run_id=$1 AND NOT p.terminal",[r.current_run_id])).rows;return {...room,members:(room.members??[]).map(m=>{const p=participants.find(p=>p.desktop_id===m.desktopId);return !p?m:p.state==='active'?{...m,readiness:p.protected?'ready':'playing',readinessReason:'',ready:!!p.protected}:{...m,readiness:p.state==='review'?'checking_report':'reconnecting',readinessReason:p.reason,ready:false};})};}
 override async reconcile(tx:Tx,r:any){await this.refreshRoster(tx,r);if(r.config.mode==='race'&&this.race){await this.race.deadline(tx,r);if(r.state==='ended')return;}if(r.state==='running'){await this.recovery.sweep(tx,r.current_run_id);return;}return super.reconcile(tx,r);}
 override sweep(){
  if(!this.housekeeping)this.housekeeping=this.sweepBatch().finally(()=>{this.housekeeping=undefined;});return this.housekeeping;
 }
 private async sweepBatch(){
  await this.db.transaction(async tx=>{await maintainAuthentication(tx,this.now);await maintainStatistics(tx,this.now);await purgeRace(tx,this.now);});
  // Discovery holds no room/participant row locks. Each selected room has its
  // own bounded transaction; healthy rooms cannot pin the first page forever.
  const discover=()=>this.db.transaction(async tx=>(await tx.query('SELECT id FROM rooms WHERE deleted_at IS NULL AND current_run_id IS NOT NULL AND ($1::uuid IS NULL OR id>$1) ORDER BY id LIMIT 32',[this.housekeepingCursor||null])).rows);
  let ids=await discover();if(!ids.length&&this.housekeepingCursor){this.housekeepingCursor='';ids=await discover();}if(!ids.length)return;
  for(const row of ids){await this.db.transaction(async tx=>{
   const r=(await tx.query('SELECT * FROM rooms WHERE id=$1 FOR UPDATE SKIP LOCKED',[row.id])).rows[0];if(!r?.current_run_id)return;
   const run=(await tx.query('SELECT * FROM runs WHERE id=$1',[r.current_run_id])).rows[0];
   for(const p of (await tx.query("SELECT * FROM preparations WHERE run_id=$1 AND state IN ('issued','consumed') AND expires_at<=$2 ORDER BY id LIMIT 32 FOR UPDATE",[run.id,new Date(this.now)])).rows)await this.inspection.invalidatePreparation(tx,p,'expired');
   if(r.state==='staging'&&+run.review_until<=this.now){await this.cancel(tx,r,'inspection_expired');return;}
   if(r.state==='running')await this.reconcile(tx,r);
   else if(['ended','cancelled'].includes(run.state))await this.recovery.sweep(tx,run.id);
   await tx.query('DELETE FROM room_chat WHERE room_id=$1 AND sent_at<$2',[r.id,new Date(this.now-15*60000)]);
  });this.housekeepingCursor=row.id;}
 }
 async endForPlayerDeath(tx:Tx,room:Record<string,any>,sourceWorld:string){
  // Persist the actionable reason before cancellation revokes any game session.
  // Previously won/removed worlds are not turned into new death recipients.
  await tx.query('UPDATE world_associations w SET terminal_reason=$2 FROM rejoin_participants p WHERE w.run_id=$1 AND p.run_id=w.run_id AND p.world_token=w.world_id::text AND (NOT p.terminal OR p.world_token=$3)',[room.current_run_id,'player_death_end',sourceWorld]);
  await this.cancel(tx,room,'player_death_end','ended');
  await synchronizeGameplay(tx,this.db.epoch,this.compatibility.capabilities,this.now,{db:this.db,compatibility:this.compatibility,roomId:room.id});
  markBankDirty(tx,room.current_run_id);
 }
 override async cancel(tx:Tx,r:any,reason:string,state:'waiting'|'ended'='waiting'){
  if(r.current_run_id){await endRace(tx,r.current_run_id,this.now,reason);await endStatistics(tx,r.current_run_id,this.now);}
  if(r.current_run_id){await tx.query('UPDATE rejoin_runs SET terminal_reason=$2 WHERE run_id=$1',[r.current_run_id,reason]);for(const p of (await tx.query('SELECT id FROM rejoin_participants WHERE run_id=$1 AND NOT terminal',[r.current_run_id])).rows)await this.recovery.retire(tx,p.id,reason,'ended');}return super.cancel(tx,r,reason,state);
 }
 override async onPlayerRemoved(tx:Tx,r:Record<string,any>,accountId:string,action:string){for(const p of (await tx.query('SELECT p.id FROM rejoin_participants p JOIN rejoin_runs k ON k.run_id=p.run_id WHERE k.room_id=$1 AND p.account_id=$2 AND NOT p.terminal',[r.id,accountId])).rows)await this.recovery.retire(tx,p.id,'host_'+action);}
 override async execute(tx:Tx,req:Request,actor:Actor):Promise<Partial<Response>>{
  // Mark deliberate departure/moderation before the base projection. Transient
  // session loss never uses these transitions. No automatic host promotion.
  if(req.roomLeave){
   validate('Request',req);demand(req.revision===schema.revision&&req.serviceId===this.db.serviceId&&req.serviceEpoch===this.db.epoch&&req.sessionEpoch===actor.sessionEpoch&&actor.kind==='desktop','STALE_EPOCH');
   const r=(await tx.query('SELECT * FROM rooms WHERE id=$1 FOR UPDATE',[req.roomId])).rows[0],rows=(await tx.query('SELECT p.id FROM rejoin_participants p JOIN rejoin_runs k ON k.run_id=p.run_id WHERE p.desktop_id=$1 AND k.room_id=$2',[actor.sessionId,req.roomId])).rows;
   if(rows.length){for(const p of rows)await this.recovery.retire(tx,p.id,'explicit_leave','offline');await tx.query('UPDATE rooms SET revision=revision+1 WHERE id=$1',[r.id]);r.revision++;return {room:await this.snapshot(tx,r)};}
  }
  const statisticsStart=!!req.finalStart&&!(await tx.query('SELECT 1 FROM inspected_releases WHERE run_id=$1',[req.runId])).rowCount;
  const result=await super.execute(tx,req,actor);if(req.finalStart){await this.recovery.capture(tx,req.runId!);if(statisticsStart){await startStatistics(tx,req.runId!);await startRace(tx,req.runId!);}}return result;
 }
}

