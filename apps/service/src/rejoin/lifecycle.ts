import {randomUUID} from 'node:crypto';
import type {Tx} from '../g1/database.js';
import type {Actor} from '../g1/auth.js';
import {demand,hash,matches,fingerprint} from '../g1/security.js';
import {validateObservation,contentFailure,contentDigest} from '../g2-next/observation.js';
import {remaining,connectionReply} from '../g3/connection.js';
import type {RejoinRooms} from './rooms.js';
import type {Request,WorldAssociation} from '../../../../packages/contracts/g3-lifecycle/g3-lifecycle.js';

const uuid=(v:unknown)=>typeof v==='string'&&/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i.test(v);
export class WorldLifecycle {
 constructor(readonly rooms:RejoinRooms){}
 get db(){return this.rooms.db;}get auth(){return this.rooms.principalAuth;}get now(){return this.auth.now();}
 async fence(tx:Tx,w:any){if(w.attachment_id)await this.rooms.recovery.fence(tx,w);}
 async owner(tx:Tx,w:any){return w.attachment_id?(await tx.query(`SELECT a.*,s.epoch AS game_epoch,s.revoked_at AS game_revoked,d.revoked_at AS desktop_revoked,f.revoked_at AS family_revoked,f.expires_at,m.presence_until AS desktop_until FROM attachments a JOIN sessions s ON s.id=a.game_session_id JOIN sessions d ON d.id=a.desktop_id JOIN session_families f ON f.id=s.family_id LEFT JOIN memberships m ON m.desktop_id=d.id AND m.room_id=$2 WHERE a.id=$1`,[w.attachment_id,w.association.roomId])).rows[0]:undefined;}
 healthy(a:any){return !!a&&!a.revoked_at&&!a.game_revoked&&!a.desktop_revoked&&!a.family_revoked&&a.service_epoch===this.db.epoch&&Math.min(+a.presence_until,+a.desktop_until,+a.expires_at)>this.now;}
 async terminal(tx:Tx,w:any,reason:string){await this.fence(tx,w);await tx.query('UPDATE world_associations SET terminal_reason=$2 WHERE world_id=$1',[w.world_id,reason]);w.terminal_reason=reason;return w;}
 state(w:any,n:any,state:string,reason:string,retry=0){return {association:w.association,state,reason,attempt:w.high_water,generation:w.generation,mayPlay:false,retryAfterMs:retry,ownerRemainingMs:retry,reviewRemainingMs:remaining(+n.review_until,this.now,3600000),runEpoch:n.epoch,connectionId:w.connection_id};}
 async prepare(tx:Tx,actor:Actor,req:Request){
  const input=req.prepare!;demand(uuid(input.worldId)&&uuid(input.roomId)&&/^[A-Za-z0-9_-]{43}$/.test(input.proof??'')&&/^[a-f0-9]{64}$/.test(input.configDigest??''),'INVALID_MESSAGE');
  const r=(await tx.query('SELECT * FROM rooms WHERE id=$1 FOR UPDATE',[input.roomId])).rows[0];demand(r,'NOT_FOUND');
  const n=(await tx.query('SELECT * FROM runs WHERE id=$1',[r.current_run_id])).rows[0];demand(r.state==='staging'&&n?.state==='staging','ENTRY_NOT_STAGING');
  demand(n.service_epoch===this.db.epoch&&r.config_revision===input.configRevision&&fingerprint(r.config)===input.configDigest,'STALE_EPOCH');demand(+n.review_until>this.now,'EXPIRED');
  let intent=(await tx.query('SELECT * FROM world_creation_intents WHERE world_id=$1',[input.worldId])).rows[0];
  if(intent){demand(intent.account_id===actor.accountId&&intent.device_id===actor.deviceId&&matches(hash(input.proof!),intent.proof_hash),'FORBIDDEN');demand(intent.run_id===n.id&&intent.association.configDigest===input.configDigest,'CONFLICT');}
  else{
   const m=(await tx.query('SELECT * FROM memberships WHERE room_id=$1 AND desktop_id=$2 AND account_id=$3',[r.id,actor.sessionId,actor.accountId])).rows[0];demand(m?.ready_config_revision===r.config_revision&&+m.presence_until>this.now,'FORBIDDEN');
   const principal=await this.auth.principal(tx,actor.accountId,true);demand(principal.kind!=='guest'||r.allow_guests,'FORBIDDEN');demand(!(await tx.query('SELECT 1 FROM room_bans WHERE room_id=$1 AND account_id=$2',[r.id,actor.accountId])).rowCount,'FORBIDDEN');
   demand(+(await tx.query('SELECT count(*) FROM world_creation_intents WHERE run_id=$1 AND account_id=$2 AND device_id=$3',[n.id,actor.accountId,actor.deviceId])).rows[0].count<1024,'CAPACITY');
   await tx.query('INSERT INTO world_lineages(run_id,account_id,device_id) VALUES($1,$2,$3) ON CONFLICT DO NOTHING',[n.id,actor.accountId,actor.deviceId]);
   const lineage=(await tx.query('SELECT * FROM world_lineages WHERE run_id=$1 AND account_id=$2 AND device_id=$3',[n.id,actor.accountId,actor.deviceId])).rows[0];
   const association={worldId:input.worldId,roomId:r.id,runId:n.id,preparationId:randomUUID(),configRevision:r.config_revision,configDigest:input.configDigest};
   intent=(await tx.query('INSERT INTO world_creation_intents(world_id,run_id,account_id,device_id,association,proof_hash,desktop_id,base_world_id,base_revision,created_at) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10) RETURNING *',[input.worldId,n.id,actor.accountId,actor.deviceId,association,hash(input.proof!),actor.sessionId,lineage.current_world_id,lineage.revision,new Date(this.now)])).rows[0];
  }
  return {prepared:{association:intent.association,config:r.config,runEpoch:n.epoch,reviewRemainingMs:remaining(+n.review_until,this.now,3600000)}};
 }
 async execute(tx:Tx,actor:Actor,req:Request):Promise<any>{
  demand(actor.kind==='desktop','FORBIDDEN');if(req.prepare)return this.prepare(tx,actor,req);const input=req.reconcile!,scope=input.association!;
  for(const id of [scope.worldId,scope.roomId,scope.runId,scope.preparationId])demand(uuid(id),'INVALID_MESSAGE');
  demand(/^[a-f0-9]{64}$/.test(scope.configDigest??'')&&/^[A-Za-z0-9_-]{43}$/.test(input.proof??'')&&input.attempt!>0&&!!input.connectionId,'INVALID_MESSAGE');
  const r=(await tx.query('SELECT * FROM rooms WHERE id=$1 FOR UPDATE',[scope.roomId])).rows[0];demand(r,'NOT_FOUND');
  const n=(await tx.query('SELECT * FROM runs WHERE id=$1 AND room_id=$2',[scope.runId,r.id])).rows[0];demand(n,'NOT_FOUND');
  let w=(await tx.query('SELECT * FROM world_associations WHERE world_id=$1 FOR UPDATE',[scope.worldId])).rows[0];
  let p=(await tx.query('SELECT p.*,f.account_id,f.device_id FROM preparations p JOIN sessions d ON d.id=p.desktop_id JOIN session_families f ON f.id=d.family_id WHERE p.id=$1',[scope.preparationId])).rows[0];
  const intent=(await tx.query('SELECT * FROM world_creation_intents WHERE world_id=$1',[scope.worldId])).rows[0];
  const observation=input.observation;
  const key=fingerprint([scope,actor.sessionId,actor.sessionEpoch,this.db.epoch,input.connectionId,input.action,input.gameVersion,input.modVersion,input.bridgeVersion,input.capabilities,observation?.gameInstanceId,observation?contentDigest(observation):null]);
  if(!w){
   demand(intent&&intent.run_id===n.id&&intent.account_id===actor.accountId&&intent.device_id===actor.deviceId&&matches(hash(input.proof!),intent.proof_hash),'FORBIDDEN');demand(fingerprint(intent.association)===fingerprint(scope),'CONFLICT');
   demand(r.current_run_id===n.id&&r.state==='staging'&&n.state==='staging','ENTRY_NOT_STAGING');
   demand(+n.review_until>this.now,'EXPIRED');demand(n.service_epoch===this.db.epoch&&r.config_revision===scope.configRevision&&fingerprint(r.config)===scope.configDigest,'STALE_EPOCH');
   const consent=(await tx.query('SELECT m.* FROM memberships m WHERE m.room_id=$1 AND m.desktop_id=COALESCE((SELECT a.desktop_id FROM world_lineages l JOIN world_associations a ON a.world_id=l.current_world_id WHERE l.run_id=$4 AND l.account_id=$3 AND l.device_id=$5),$2::uuid) AND m.account_id=$3',[r.id,intent.desktop_id,actor.accountId,n.id,actor.deviceId])).rows[0];demand(consent?.ready_config_revision===r.config_revision,'FORBIDDEN');
   demand(+(await tx.query('SELECT count(*) FROM world_associations WHERE run_id=$1 AND account_id=$2 AND device_id=$3',[n.id,actor.accountId,actor.deviceId])).rows[0].count<1024,'CAPACITY');
   w=(await tx.query('INSERT INTO world_associations(world_id,run_id,preparation_id,account_id,device_id,association,proof_hash,high_water,attempt_key,desktop_id,connection_id,created_at) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12) RETURNING *',[scope.worldId,n.id,scope.preparationId,actor.accountId,actor.deviceId,scope,hash(input.proof!),input.attempt,key,intent.desktop_id,input.connectionId,new Date(this.now)])).rows[0];
  }else{
   demand(w.account_id===actor.accountId&&w.device_id===actor.deviceId&&matches(hash(input.proof!),w.proof_hash),'FORBIDDEN');
   demand(fingerprint(w.association)===fingerprint(scope),'CONFLICT');
   demand(input.attempt!>=w.high_water,'STALE_ATTEMPT');
   if(input.attempt===w.high_water)demand(w.attempt_key===key,'CONFLICT');
   else{
    // This write commits even on OWNER_ACTIVE/review/terminal replies. The
    // old sequence can never execute later, including an old never-sent request.
    const old=await this.owner(tx,w),until=this.healthy(old)?Math.min(+old.presence_until,+old.desktop_until,+old.expires_at,this.now+30000):this.now;
    w=(await tx.query('UPDATE world_associations SET high_water=$2,attempt_key=$3,connection_id=$4,supersede_after=COALESCE(supersede_after,$5) WHERE world_id=$1 RETURNING *',[w.world_id,input.attempt,key,input.connectionId,new Date(until)])).rows[0];
   }
  }
  const q=await this.auth.principal(tx,actor.accountId,true),banned=(await tx.query('SELECT 1 FROM room_bans WHERE room_id=$1 AND account_id=$2',[r.id,actor.accountId])).rowCount;
  let reason=w.terminal_reason??(r.current_run_id!==n.id||!['staging','running'].includes(n.state)||!['staging','running'].includes(r.state)?'run_ended':r.config_revision!==scope.configRevision||n.config_revision!==scope.configRevision||fingerprint(r.config)!==scope.configDigest?'configuration_changed':banned?'host_ban':q.kind==='guest'&&!r.allow_guests?'access_revoked':p?.state==='cancelled'||p?.retired_by?'preparation_retired':undefined);
  let participant=(await tx.query('SELECT id FROM rejoin_participants WHERE run_id=$1 AND world_token=$2 AND account_id=$3 AND device_id=$4',[n.id,w.world_id,actor.accountId,actor.deviceId])).rows[0];
  participant=participant?await this.rooms.recovery.read(tx,participant.id):undefined;
  if(n.state==='running'&&!participant)reason??='not_selected_by_start';
  if(participant?.terminal||participant?.terminal_reason)reason??=participant.reason||participant.terminal_reason;
  const member=(await tx.query('SELECT * FROM memberships WHERE room_id=$1 AND desktop_id=CASE WHEN $3=0 THEN COALESCE((SELECT a.desktop_id FROM world_lineages l JOIN world_associations a ON a.world_id=l.current_world_id WHERE l.run_id=$4 AND l.account_id=$5 AND l.device_id=$6),$2::uuid) ELSE $2::uuid END',[r.id,w.desktop_id,w.generation,n.id,actor.accountId,actor.deviceId])).rows[0];
  if(!member||member.account_id!==actor.accountId)reason??='membership_removed';
  else if(member.ready_config_revision!==scope.configRevision)reason??='consent_withdrawn';
  if(input.action==='offline'&&reason!=='player_death_end')reason='explicit_offline';
  if(reason){await this.terminal(tx,w,reason);if(participant&&!participant.terminal)await this.rooms.recovery.retire(tx,participant.id,reason,'offline');return {world:this.state(w,n,'terminal',reason)};}
  if(n.state==='staging'&&+n.review_until<=this.now)return {world:this.state(await this.terminal(tx,w,'review_expired'),n,'terminal','review_expired')};
  demand(input.gameVersion===this.rooms.compatibility.gameVersion&&input.modVersion===this.rooms.compatibility.modVersion&&input.bridgeVersion===this.rooms.compatibility.bridgeVersion&&(r.config.requiredCapabilities??[]).every((c:string)=>input.capabilities?.includes(c)),'INCOMPATIBLE_GAME');
  demand(observation&&observation.attachmentId===undefined&&observation.worldToken===w.world_id&&observation.loaded&&observation.inventoryComplete,'REPORT_CHANGED');
  validateObservation({...observation,attachmentId:randomUUID()});
  const failure=contentFailure(observation,r.config,this.rooms.compatibility);
  if(failure||w.actual_seed&&w.actual_seed!==observation.actualSeed){await this.fence(tx,w);return {world:this.state(w,n,'review',failure??'seed_changed')};}
  let a=await this.owner(tx,w);
  let base:any;
  if(w.generation===0){
   const current=(await tx.query('SELECT * FROM world_associations WHERE run_id=$1 AND account_id=$2 AND device_id=$3 AND terminal_reason IS NULL AND attachment_id IS NOT NULL',[n.id,actor.accountId,actor.deviceId])).rows;demand(current.length<=1,'CONFLICT');base=current[0];
   const lineage=(await tx.query('SELECT * FROM world_lineages WHERE run_id=$1 AND account_id=$2 AND device_id=$3',[n.id,actor.accountId,actor.deviceId])).rows[0];
   demand(lineage&&lineage.revision===intent?.base_revision,'STALE_CREATION');
   if(base){a=await this.owner(tx,base);if(!w.supersede_after){w.supersede_after=new Date(this.healthy(a)?Math.min(+a.presence_until,+a.desktop_until,+a.expires_at,this.now+30000):this.now);await tx.query('UPDATE world_associations SET supersede_after=$2 WHERE world_id=$1',[w.world_id,w.supersede_after]);}}
  }
  const same=w.owner_attempt===input.attempt&&w.desktop_id===actor.sessionId&&a?.game_instance_id===observation.gameInstanceId&&this.healthy(a);
  if(!same&&this.healthy(a)&&remaining(+w.supersede_after,this.now,30000)>0){
   const wait=remaining(+w.supersede_after,this.now,30000);
   await tx.query('UPDATE attachments SET presence_until=LEAST(presence_until,$2),resume_until=LEAST(resume_until,$2) WHERE id=$1',[a.id,w.supersede_after]);
   return {world:this.state(w,n,'retry','OWNER_ACTIVE',wait)};
  }
  if(!same){
   demand(observation.protected,'NOT_READY');demand(w.generation<2147483647,'CAPACITY');
   const elsewhere=(await tx.query('SELECT room_id FROM memberships WHERE desktop_id=$1',[actor.sessionId])).rows[0];demand(!elsewhere||elsewhere.room_id===r.id,'FORBIDDEN');
   await this.fence(tx,w);
   if(base){await this.terminal(tx,base,'replaced_by_new_world');await tx.query("UPDATE preparations SET state='cancelled' WHERE id=$1",[base.preparation_id]);await tx.query('DELETE FROM memberships WHERE desktop_id=$1 AND room_id=$2',[base.desktop_id,r.id]);}
   if(w.generation===0){const lineage=(await tx.query('SELECT revision FROM world_lineages WHERE run_id=$1 AND account_id=$2 AND device_id=$3',[n.id,actor.accountId,actor.deviceId])).rows[0];demand(lineage.revision<2147483647,'CAPACITY');await tx.query('UPDATE world_lineages SET revision=revision+1,current_world_id=$4 WHERE run_id=$1 AND account_id=$2 AND device_id=$3',[n.id,actor.accountId,actor.deviceId,w.world_id]);}
   if(w.attachment_id)await tx.query('UPDATE attachments SET preparation_id=NULL WHERE id=$1',[w.attachment_id]);
   await tx.query('DELETE FROM memberships WHERE room_id=$1 AND desktop_id=$2',[r.id,w.desktop_id]);
   await tx.query('INSERT INTO memberships(desktop_id,room_id,account_id,presence_until,ready_config_revision,selected) VALUES($1,$2,$3,$4,$5,$6) ON CONFLICT(desktop_id) DO UPDATE SET presence_until=EXCLUDED.presence_until,ready_config_revision=EXCLUDED.ready_config_revision,selected=EXCLUDED.selected,roster_retired=false',[actor.sessionId,r.id,actor.accountId,new Date(this.now+30000),scope.configRevision,!!participant]);
   // Only one current lineage per account/device/run. A distinct New Game must
   // use a new preparation; it never adopts a selected participant.
   const conflicts=(await tx.query("SELECT p.id FROM preparations p WHERE p.run_id=$1 AND p.desktop_id=$2 AND p.id<>$3 AND p.state IN ('issued','consumed','loaded')",[n.id,actor.sessionId,w.preparation_id])).rows;
   // Only actual loaded B retires unbound early roomJoin preparations. Staging
   // a creation intent above never touches these facts or saved/current A.
   for(const old of conflicts){demand(!(await tx.query('SELECT 1 FROM world_associations WHERE preparation_id=$1 AND terminal_reason IS NULL AND world_id<>$2',[old.id,w.world_id])).rowCount,'CONFLICT');await this.rooms.inspection.invalidatePreparation(tx,old,'cancelled');}
   if(!p){p=(await tx.query("INSERT INTO preparations(id,run_id,desktop_id,desktop_epoch,service_epoch,run_epoch,config_revision,state,issued_at,expires_at) VALUES($1,$2,$3,$4,$5,$6,$7,'loaded',$8,$9) RETURNING *",[w.preparation_id,n.id,actor.sessionId,actor.sessionEpoch,this.db.epoch,n.epoch,scope.configRevision,new Date(this.now),new Date(this.now+600000)])).rows[0];}
   const sid=randomUUID(),epoch=randomUUID(),aid=randomUUID();await tx.query("INSERT INTO sessions(id,family_id,kind,epoch,parent_id) VALUES($1,$2,'game',$3,$4)",[sid,actor.familyId,epoch,actor.sessionId]);
   await tx.query('INSERT INTO attachments(id,run_id,desktop_id,game_session_id,game_instance_id,service_epoch,run_epoch,config_revision,presence_until,resume_until,preparation_id) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$9,$10)',[aid,n.id,actor.sessionId,sid,observation.gameInstanceId,this.db.epoch,n.epoch,scope.configRevision,new Date(this.now+30000),p.id]);
   await tx.query("UPDATE preparations SET desktop_id=$2,desktop_epoch=$3,service_epoch=$4,run_epoch=$5,state='loaded' WHERE id=$1",[p.id,actor.sessionId,actor.sessionEpoch,this.db.epoch,n.epoch]);
   w=(await tx.query('UPDATE world_associations SET attachment_id=$2,desktop_id=$3,owner_attempt=$4,generation=generation+1,actual_seed=COALESCE(actual_seed,$5),supersede_after=NULL WHERE world_id=$1 RETURNING *',[w.world_id,aid,actor.sessionId,input.attempt,observation.actualSeed])).rows[0];
   if(participant){demand(participant.generation<2147483647,'CAPACITY');await tx.query("UPDATE rejoin_participants SET desktop_id=$2,attachment_id=$3,generation=generation+1,state='inspecting',reason='fresh_inspection_required',authorization_id=$4,pending_report=NULL WHERE id=$1",[participant.id,actor.sessionId,aid,randomUUID()]);}
   a=await this.owner(tx,w);
  }
  const credentials=await this.auth.mint(tx,actor.accountId,actor.deviceId,'game',actor.familyId,a.game_session_id,a.game_epoch,a.id),gameActor=await this.auth.authenticate(tx,credentials.accessToken!);
  const current={...observation,attachmentId:a.id};
  await this.rooms.inspection.observe(tx,gameActor,{serviceId:this.db.serviceId,serviceEpoch:this.db.epoch,sessionEpoch:a.game_epoch,roomId:r.id,runId:n.id,runEpoch:n.epoch,configRevision:scope.configRevision!},current);
  await tx.query('UPDATE memberships SET presence_until=$2 WHERE desktop_id=$1',[actor.sessionId,new Date(this.now+30000)]);
  // An exact observation retry may take the inspection's immutable fast path;
  // reconcile still renews this current connection through ordinary health.
  await this.rooms.inspection.pulse(tx,gameActor,{serviceId:this.db.serviceId,serviceEpoch:this.db.epoch,sessionEpoch:a.game_epoch,roomId:r.id,runId:n.id,runEpoch:n.epoch,configRevision:scope.configRevision!,gamePulse:{attachmentId:a.id,gameInstanceId:observation.gameInstanceId!,worldToken:w.world_id}});
  const value=await connectionReply(tx,this.db,this.auth,this.rooms.compatibility,{roomId:r.id,requestId:req.requestId,expectedAttachmentId:a.id},{credentials},credentials.accessToken);
  const state=this.state(w,n,n.state==='staging'?'waiting':value.canonicalAdmission?.decision.authorized?'playing':'review',n.state==='staging'?'awaiting_host_start':value.canonicalAdmission?.decision.reason??'inspection_required');
  state.ownerRemainingMs=remaining(+(await this.owner(tx,w)).presence_until,this.now,30000);
  return {...value,world:{...state,mayPlay:!!value.canonicalAdmission?.decision.authorized,...(participant?{participantId:participant.id}:{}),...(value.canonicalAdmission?.currentReport?{currentReport:value.canonicalAdmission.currentReport}:{})}};
 }
}

