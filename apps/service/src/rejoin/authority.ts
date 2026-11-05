import {touchC1Room} from '../g3/c1.js';
import {originalOperation,reserveOutcome,recordOutcome} from './outcomes.js';
import {readRoomAdmissions} from '../g3/admission.js';
import {randomUUID} from 'node:crypto';
import type {Tx,Database} from '../g1/database.js';
import type {Actor} from '../g1/auth.js';
import type {PrincipalAuth} from '../g2-next/auth.js';
import {demand,hash,matches,secret,fingerprint} from '../g1/security.js';
import {validateObservation,contentDigest,contentFailure} from '../g2-next/observation.js';
import {schema as lobbySchema} from '../../../../packages/contracts/src/g2-next.js';
import {schema,validate} from '../../../../packages/contracts/src/g3-rejoin.js';
import type {Request,Response,GameObservation} from '../../../../packages/contracts/g3-rejoin/g3-rejoin.js';
import {REJOIN_LEASE_MS} from '../../../../packages/domain/src/rejoin.js';

// One database transaction owns admission, request recovery and generation fencing.
export class RejoinAuthority {
 constructor(readonly db:Database,readonly auth:PrincipalAuth,readonly compatibility:any){}
 get now(){return this.auth.now();}
 async capture(tx:Tx,runId:string){
  const run=(await tx.query('SELECT n.*,r.current_run_id FROM runs n JOIN rooms r ON r.id=n.room_id WHERE n.id=$1',[runId])).rows[0];
  const x=(await tx.query('SELECT * FROM inspected_releases WHERE run_id=$1',[runId])).rows[0];
  if(!run||run.state!=='running'||!x||x.revoked_at)return;
  await tx.query('INSERT INTO rejoin_runs(run_id,room_id,original_start_id,frozen_config,config_revision,created_at) VALUES($1,$2,$3,$4,$5,$6) ON CONFLICT DO NOTHING',[run.id,run.room_id,x.authorization_id,run.config,run.config_revision,new Date(this.now)]);
  for(const report of x.selected_reports.participants){
   const a=(await tx.query('SELECT a.*,f.account_id,f.device_id FROM attachments a JOIN sessions s ON s.id=a.game_session_id JOIN session_families f ON f.id=s.family_id WHERE a.id=$1',[report.attachmentId])).rows[0];
   const originalObservation=(await tx.query('SELECT body FROM world_observations WHERE attachment_id=$1 AND revision=$2 AND content_digest=$3',[report.attachmentId,report.observationRevision,report.contentDigest])).rows[0]?.body;
   const proof=secret();
   await tx.query("INSERT INTO rejoin_participants(id,run_id,account_id,device_id,world_token,proof_hash,proof_ciphertext,original_report,accepted_digest,desktop_id,attachment_id,state,authorization_id,authorized_at,created_at,original_observation) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,'active',$12,$13,$13,$14) ON CONFLICT(run_id,world_token) DO NOTHING",[randomUUID(),runId,a.account_id,a.device_id,report.worldToken,hash(proof),this.auth.security.seal(proof),report,report.contentDigest,a.desktop_id,a.id,x.authorization_uuid,new Date(this.now),originalObservation??null]);
  }
 }
 async retire(tx:Tx,id:string,reason:string,state='removed'){
  const current=await this.read(tx,id);if(!current||current.terminal)return;
  const p=(await tx.query('UPDATE rejoin_participants SET terminal=true,state=$2,reason=$3 WHERE id=$1 RETURNING *',[id,state,reason])).rows[0];if(!p)return;
  await this.fence(tx,p);
  const raceWithdrawal=await tx.query("UPDATE race_entrants SET status='dnf' WHERE run_id=$1 AND account_id=$2 AND world_token=$3 AND status='racing' RETURNING run_id",[p.run_id,p.account_id,p.world_token]);
  if(raceWithdrawal.rowCount)await tx.query('UPDATE race_runs SET revision=revision+1 WHERE run_id=$1',[p.run_id]);
  await tx.query('DELETE FROM memberships WHERE desktop_id=$1 AND room_id=(SELECT room_id FROM rejoin_runs WHERE run_id=$2) AND EXISTS(SELECT 1 FROM rooms WHERE id=memberships.room_id AND current_run_id=$2)',[p.desktop_id,p.run_id]);
  const room=(await tx.query('SELECT room_id FROM runs WHERE id=$1',[p.run_id])).rows[0];if(room)await touchC1Room(tx,room.room_id);
 }
 async fence(tx:Tx,p:any){
  await tx.query('UPDATE attachments SET revoked_at=$2 WHERE id=$1 AND revoked_at IS NULL',[p.attachment_id,new Date(this.now)]);
  await tx.query('UPDATE sessions SET revoked_at=$2 WHERE id=(SELECT game_session_id FROM attachments WHERE id=$1) AND revoked_at IS NULL',[p.attachment_id,new Date(this.now)]);
  await tx.query('UPDATE g3_contexts SET revoked_at=$2 WHERE attachment_id=$1 AND revoked_at IS NULL',[p.attachment_id,new Date(this.now)]);
 }
 async healthy(tx:Tx,p:any){
  return !!(await tx.query(`SELECT 1 FROM attachments a JOIN sessions s ON s.id=a.game_session_id JOIN sessions d ON d.id=a.desktop_id JOIN session_families f ON f.id=s.family_id JOIN memberships m ON m.desktop_id=d.id
   WHERE a.id=$1 AND a.revoked_at IS NULL AND s.revoked_at IS NULL AND d.revoked_at IS NULL AND f.revoked_at IS NULL AND f.expires_at>$2 AND a.presence_until>$2 AND m.presence_until>$2 AND a.service_epoch=$3`,[p.attachment_id,new Date(this.now),this.db.epoch])).rowCount;
 }
 async lockRoom(tx:Tx,runId:string){return (await tx.query('SELECT r.id FROM rooms r JOIN runs n ON n.room_id=r.id WHERE n.id=$1 FOR UPDATE OF r',[runId])).rows[0];}
 // A run has at most32 originally selected participants. No global first page,
 // and room-before-participant order matches both requests and reconciliation.
 async sweep(tx:Tx,runId:string,participantId?:string){
  await this.lockRoom(tx,runId);
  const ids=(await tx.query('SELECT id FROM rejoin_participants WHERE run_id=$1 AND NOT terminal AND ($2::uuid IS NULL OR id=$2) ORDER BY id LIMIT 33',[runId,participantId??null])).rows;
  demand(ids.length<=32,'CAPACITY');
  for(const row of ids){const p=await this.read(tx,row.id);if(!p)continue;
   const reason=this.accessFailure(p);
   if(reason){await this.retire(tx,p.id,reason,p.terminal_reason||['ended','cancelled'].includes(p.run_state)?'ended':'removed');continue;}
   if(['active','inspecting','review'].includes(p.state)&&!await this.healthy(tx,p)){await this.fence(tx,p);await tx.query("UPDATE rejoin_participants SET state='reconnecting',reason='connection_lost',pending_report=NULL WHERE id=$1",[p.id]);}
  }
 }
 async read(tx:Tx,id:string){
  const key=(await tx.query('SELECT run_id FROM rejoin_participants WHERE id=$1',[id])).rows[0];if(!key)return;
  await this.lockRoom(tx,key.run_id);
  return (await tx.query(`SELECT p.*,k.room_id,k.original_start_id,k.frozen_config,k.config_revision,k.terminal_reason,a.game_session_id AS owner_session_id,a.presence_until AS owner_until,n.epoch AS run_epoch,n.state AS run_state,r.current_run_id,r.host_account_id,r.allow_guests,r.config AS current_config,r.config_revision AS current_revision,u.disabled AS account_disabled,q.kind AS principal_kind,q.guest_expires_at,
   EXISTS(SELECT 1 FROM room_bans b WHERE b.room_id=r.id AND b.account_id=p.account_id) AS banned
   FROM rejoin_participants p JOIN rejoin_runs k ON k.run_id=p.run_id JOIN runs n ON n.id=p.run_id JOIN rooms r ON r.id=k.room_id JOIN attachments a ON a.id=p.attachment_id JOIN accounts u ON u.id=p.account_id JOIN principal_profiles q ON q.principal_id=p.account_id WHERE p.id=$1 FOR UPDATE OF p`,[id])).rows[0];
 }
 private accessFailure(p:any):string|undefined{
  if(p.terminal||p.terminal_reason||p.current_run_id!==p.run_id||p.run_state!=='running')return p.terminal_reason??'run_unavailable';
  if(p.config_revision!==p.current_revision||fingerprint(p.frozen_config)!==fingerprint(p.current_config))return 'configuration_changed';
  if(p.account_disabled||p.banned||p.principal_kind==='guest'&&(!p.allow_guests||+p.guest_expires_at<=this.now))return 'access_revoked';
 }
 view(p:any,report?:'original'|'pending'):any{return {...(report==='original'&&p.original_observation?{originalReport:p.original_observation}:{}),...(report==='pending'&&p.pending_report?{pendingReport:p.pending_report}:{}),association:{serviceId:this.db.serviceId,roomId:p.room_id,runId:p.run_id,participantId:p.id,worldToken:p.world_token,accountId:p.account_id,originalStartId:p.original_start_id,configRevision:p.config_revision,actualSeed:p.original_report.actualSeed,seedPolicy:p.original_report.seedPolicy,randomizeLoot:p.original_report.randomizeLoot},state:p.state,reason:p.reason,generation:p.generation,serviceEpoch:this.db.epoch,runEpoch:p.run_epoch,desktopId:p.desktop_id,attachmentId:p.attachment_id,gameSessionId:p.owner_session_id,leaseExpiresAt:p.owner_until.toISOString(),returnEligible:!p.terminal&&!p.terminal_reason,reviewRequired:p.state==='review',...(p.pending_report?{pendingObservationRevision:p.pending_report.observationRevision,pendingContentDigest:contentDigest(p.pending_report)}:{})};}
 async eligible(tx:Tx,p:any){
  const current=p?await this.read(tx,p.id):undefined;
  demand(current&&!this.accessFailure(current),'RETURN_INELIGIBLE');
 }
 async owner(tx:Tx,p:any,actor:Actor,generation:number|undefined){
  await this.eligible(tx,p);demand(actor.kind==='game'&&generation===p.generation,'STALE_EPOCH');
  const a=(await tx.query('SELECT * FROM attachments WHERE id=$1 AND game_session_id=$2',[p.attachment_id,actor.sessionId])).rows[0];
  demand(a&&!a.revoked_at&&a.desktop_id===actor.parentId&&a.service_epoch===this.db.epoch&&a.run_epoch===p.run_epoch&&+a.presence_until>this.now,'STALE_EPOCH');return a;
 }
 async lease(tx:Tx,p:any){await tx.query('UPDATE attachments SET presence_until=$2,resume_until=$2 WHERE id=$1',[p.attachment_id,new Date(this.now+REJOIN_LEASE_MS)]);await tx.query('UPDATE attachments a SET presence_until=LEAST(a.presence_until,w.supersede_after),resume_until=LEAST(a.resume_until,w.supersede_after) FROM world_associations w WHERE w.attachment_id=a.id AND a.id=$1 AND w.high_water>w.owner_attempt AND w.supersede_after IS NOT NULL',[p.attachment_id]);}
 async authorize(tx:Tx,p:any,input:GameObservation){
  const a=(await tx.query('SELECT a.*,s.epoch AS game_epoch,d.epoch AS desktop_epoch FROM attachments a JOIN sessions s ON s.id=a.game_session_id JOIN sessions d ON d.id=a.desktop_id WHERE a.id=$1',[p.attachment_id])).rows[0];
  const report={...p.original_report,desktopId:a.desktop_id,attachmentId:a.id,worldToken:p.world_token,contentDigest:contentDigest(input),observationRevision:input.observationRevision,gameSessionId:a.game_session_id,gameSessionEpoch:a.game_epoch,desktopSessionEpoch:a.desktop_epoch,gameInstanceId:a.game_instance_id,capturedAt:new Date(this.now).toISOString()};
  const x=(await tx.query('SELECT * FROM inspected_releases WHERE run_id=$1 FOR UPDATE',[p.run_id])).rows[0];
  const previous=x.selected_reports.participants.find((r:any)=>r.attachmentId===a.id);
  if(previous?.contentDigest===contentDigest(input)){
   await tx.query("UPDATE rejoin_participants SET state='active',reason='',pending_report=NULL WHERE id=$1",[p.id]);p.state='active';p.reason='';p.pending_report=null;return;
  }
  // A changed inspected admission gets a new identity; the original Start and
  // original_report stay immutable. A resumed owner already has its fresh ID.
  if(previous)p.authorization_id=randomUUID();
  const others=x.selected_reports.participants.filter((r:any)=>r.worldToken!==p.world_token);
  await tx.query('UPDATE inspected_releases SET selected_reports=$2 WHERE run_id=$1',[p.run_id,{...x.selected_reports,participants:[...others,report]}]);
  await tx.query("UPDATE rejoin_participants SET state='active',reason='',accepted_digest=$2,pending_report=NULL,authorized_at=$3,authorization_id=$4 WHERE id=$1",[p.id,contentDigest(input),new Date(this.now),p.authorization_id]);p.state='active';p.accepted_digest=contentDigest(input);p.pending_report=null;p.authorized_at=new Date(this.now);
 }
 async observe(tx:Tx,p:any,actor:Actor,input:GameObservation){
  const a=await this.owner(tx,p,actor,p.generation);validateObservation(input);
  demand(input.attachmentId===a.id&&input.gameInstanceId===a.game_instance_id&&input.worldToken===p.world_token,'FORBIDDEN');
  const old=(await tx.query('SELECT * FROM world_observations WHERE attachment_id=$1',[a.id])).rows[0];
  if(old){demand(input.observationRevision!>=old.revision,'STALE_REVISION');if(input.observationRevision===old.revision){demand(fingerprint(input)===old.fingerprint,'CONFLICT');await this.lease(tx,p);return {rows:[old]};}}
  if(input.actualSeed!==p.original_report.actualSeed||contentFailure(input,p.frozen_config,this.compatibility)){
   await tx.query("UPDATE rejoin_participants SET state='inspecting',reason='inspection_incompatible',pending_report=NULL WHERE id=$1",[p.id]);await this.storeObservation(tx,p,input);await tx.query('UPDATE g3_contexts SET presence_until=$2 WHERE attachment_id=$1',[a.id,new Date(this.now)]);p.state='inspecting';p.reason='inspection_incompatible';return {rows:[{body:input,content_digest:contentDigest(input)}]};
  }
  if(p.state!=='active')demand(input.protected&&input.loaded&&input.inventoryComplete,'NOT_READY');
  if(!input.protected&&p.generation===1&&p.state==='active'){
   // An unprotected report cannot itself manufacture first-entry evidence after
   // the original review window. Already-entered owners keep renewable leases.
   const before=await readRoomAdmissions(tx,this.db,this.compatibility,this.now,p.room_id),owner=before.worlds.find(w=>w.participantId===p.id);
   demand(owner?.decision.authorized,owner?.decision.reason==='initial_review_expired'?'EXPIRED':'NOT_READY');
  }
  const digest=contentDigest(input),changed=digest!==p.accepted_digest;
  // Unprotected changed reports immediately hold this owner, never peers.
  if(changed){await tx.query("UPDATE rejoin_participants SET state='review',reason='report_changed',pending_report=$2 WHERE id=$1",[p.id,input]);await tx.query('UPDATE g3_contexts SET presence_until=$2 WHERE attachment_id=$1',[a.id,new Date(this.now)]);p.state='review';p.reason='report_changed';p.pending_report=input;}
  await this.storeObservation(tx,p,input);
  await tx.query('UPDATE attachments SET seed_applied=true,loaded=true WHERE id=$1',[a.id]);await this.lease(tx,p);
  if(!changed&&p.state!=='active')await this.authorize(tx,p,input);
  if(p.state==='active'&&!input.protected){const x=(await tx.query('SELECT authorization_uuid FROM inspected_releases WHERE run_id=$1',[p.run_id])).rows[0];await tx.query('INSERT INTO authorization_play_entries(authorization_uuid,attachment_id,entered_playing_at) VALUES($1,$2,$3) ON CONFLICT DO NOTHING',[x.authorization_uuid,a.id,new Date(this.now)]);}
  return {rows:[{body:input,content_digest:digest}]};
 }
 private async storeObservation(tx:Tx,p:any,input:GameObservation){await tx.query('INSERT INTO world_observations(attachment_id,world_token,revision,body,fingerprint,content_digest,captured_at,valid_until) VALUES($1,$2,$3,$4,$5,$6,$7,$8) ON CONFLICT(attachment_id) DO UPDATE SET revision=EXCLUDED.revision,body=EXCLUDED.body,fingerprint=EXCLUDED.fingerprint,content_digest=EXCLUDED.content_digest,captured_at=EXCLUDED.captured_at,valid_until=EXCLUDED.valid_until',[p.attachment_id,p.world_token,input.observationRevision,input,fingerprint(input),contentDigest(input),new Date(this.now),new Date(this.now+REJOIN_LEASE_MS)]);}
 async release(tx:Tx,p:any){
  p=await this.read(tx,p.id);if(!p||this.accessFailure(p))return;
  const admission=await readRoomAdmissions(tx,this.db,this.compatibility,this.now,p.room_id),w=admission.worlds.find(w=>w.participantId===p.id&&w.owner.attachmentId===p.attachment_id);
  if(!w?.decision.authorized)return;
  const selected=w.immutable.selectedInspection;
  if(!selected)return;
  return {authorizationId:w.immutable.authorizationId,startRequestId:w.immutable.startRequestId,contractRevision:lobbySchema.revision,serviceId:this.db.serviceId,serviceEpoch:this.db.epoch,roomId:p.room_id,runId:p.run_id,runEpoch:p.run_epoch,configRevision:p.config_revision,desktopId:w.owner.desktopId,desktopSessionEpoch:w.owner.desktopSessionEpoch,gameSessionId:w.owner.gameSessionId,gameSessionEpoch:w.owner.gameSessionEpoch,attachmentId:w.owner.attachmentId,gameInstanceId:w.owner.gameInstanceId,worldToken:p.world_token,observationRevision:selected.observationRevision,contentDigest:selected.contentDigest,actualSeed:p.original_report.actualSeed,seedPolicy:p.original_report.seedPolicy,randomizeLoot:p.original_report.randomizeLoot,capturedAt:selected.capturedAt,authorizedAt:w.immutable.authorizedAt,releaseExpiresAt:new Date(w.until).toISOString(),phase:w.decision.playing?'playing':'authorized'};

 }
 async execute(tx:Tx,actor:Actor,req:Request):Promise<Partial<Response>>{
  validate('Request',req);demand(req.major===schema.major&&req.revision===schema.revision,'VERSION_MISMATCH');demand(req.serviceId===this.db.serviceId,'SERVICE_MISMATCH');demand(req.serviceEpoch===this.db.epoch&&req.sessionEpoch===actor.sessionEpoch,'STALE_EPOCH');demand(/^[A-Za-z0-9_-]{1,128}$/.test(req.requestId??''),'INVALID_MESSAGE');
  for(const id of [req.participantId,req.roomId,req.runId])if(id!==undefined)demand(/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i.test(id),'INVALID_MESSAGE');
  demand(req.observationReport===undefined||!!req.status&&['original','pending'].includes(req.observationReport),'INVALID_MESSAGE');
  if(req.list){const room=(await tx.query('SELECT host_account_id,current_run_id FROM rooms WHERE id=$1 FOR UPDATE',[req.roomId])).rows[0];demand(actor.kind!=='game'&&room?.host_account_id===actor.accountId,'FORBIDDEN');await this.sweep(tx,room.current_run_id);const ids=(await tx.query('SELECT id FROM rejoin_participants WHERE run_id=$1 ORDER BY id LIMIT 32',[room.current_run_id])).rows;const participants=[];for(const row of ids)participants.push(this.view(await this.read(tx,row.id)));return {participants};}
  let p=req.participantId?await this.read(tx,req.participantId):(await tx.query('SELECT id FROM rejoin_participants WHERE desktop_id=$1 AND NOT terminal ORDER BY created_at DESC LIMIT 1',[actor.sessionId])).rows[0];if(p&&!req.participantId)p=await this.read(tx,p.id);demand(p,'NOT_FOUND');
  const isHost=actor.kind!=='game'&&actor.accountId===p.host_account_id;
  demand(actor.accountId===p.account_id||isHost,'NOT_FOUND');
  if(req.observationReport){demand(req.roomId===p.room_id&&req.runId===p.run_id&&req.worldToken===p.world_token&&req.configRevision===p.config_revision,'FORBIDDEN');demand(req.generation===p.generation,'STALE_EPOCH');}
  await this.sweep(tx,p.run_id,p.id);p=await this.read(tx,p.id);
  if(req.status){return {participant:this.view(p,req.observationReport as 'original'|'pending'|undefined),...(actor.kind==='desktop'&&p.desktop_id===actor.sessionId&&!p.terminal?{resumeProof:this.auth.security.open<string>(p.proof_ciphertext)}:{})};}
  demand(req.roomId===p.room_id&&req.runId===p.run_id&&req.worldToken===p.world_token&&req.configRevision===p.config_revision,'FORBIDDEN');
  if(req.review){demand(isHost&&(await this.auth.principal(tx,actor.accountId,true)).kind==='registered','FORBIDDEN');await this.eligible(tx,p);demand(req.generation===p.generation,'STALE_EPOCH');demand(p.state==='review'&&p.pending_report?.protected&&req.review.participantId===p.id&&req.review.observationRevision===p.pending_report.observationRevision&&req.review.contentDigest===contentDigest(p.pending_report),'REPORT_CHANGED');demand(await this.healthy(tx,p),'NOT_READY');await this.authorize(tx,p,p.pending_report);return {participant:this.view(p)};}
  demand(actor.accountId===p.account_id,'FORBIDDEN');
  if(req.offline){demand(actor.kind==='desktop'&&actor.deviceId===p.device_id&&matches(hash(req.offline.resumeProof??''),p.proof_hash),'FORBIDDEN');demand(req.generation===p.generation,'STALE_EPOCH');const original=originalOperation(actor,p,req);await reserveOutcome(tx,original,this.now);await this.retire(tx,p.id,'explicit_offline','offline');await recordOutcome(tx,original,p.id,this.now);return {participant:this.view(await this.read(tx,p.id))};}
  await this.eligible(tx,p);
  if(req.resume){
   const input=req.resume;demand(actor.kind==='desktop'&&actor.deviceId===p.device_id&&/^[A-Za-z0-9_-]{43}$/.test(input.resumeProof??'')&&matches(hash(input.resumeProof!),p.proof_hash),'FORBIDDEN');
   const fp=fingerprint(req),old=(await tx.query('SELECT * FROM rejoin_requests WHERE participant_id=$1 AND account_id=$2 AND request_id=$3',[p.id,actor.accountId,req.requestId])).rows[0];
   if(old){demand(old.fingerprint===fp,'CONFLICT');demand(old.generation===p.generation&&old.desktop_id===actor.sessionId&&old.service_epoch===this.db.epoch&&!p.terminal&&['active','inspecting','review'].includes(p.state)&&await this.healthy(tx,p),'STALE_EPOCH');const a=(await tx.query('SELECT a.*,s.epoch FROM attachments a JOIN sessions s ON s.id=a.game_session_id WHERE a.id=$1',[p.attachment_id])).rows[0];return {participant:this.view(p),credentials:await this.auth.mint(tx,actor.accountId,actor.deviceId,'game',actor.familyId,a.game_session_id,a.epoch,a.id)};}
   const original=originalOperation(actor,p,req);await reserveOutcome(tx,original,this.now);demand(!await this.healthy(tx,p),'OWNER_ACTIVE');demand(input.gameInstanceId&&input.gameVersion===this.compatibility.gameVersion&&input.modVersion===this.compatibility.modVersion&&input.bridgeVersion===this.compatibility.bridgeVersion&&p.frozen_config.requiredCapabilities.every((c:string)=>input.capabilities?.includes(c)),'INCOMPATIBLE_GAME');
   demand(p.generation<0x7fffffff,'CAPACITY');const elsewhere=(await tx.query('SELECT room_id FROM memberships WHERE desktop_id=$1',[actor.sessionId])).rows[0];demand(!elsewhere||elsewhere.room_id===p.room_id,'FORBIDDEN');
   await this.fence(tx,p);await tx.query('DELETE FROM memberships WHERE desktop_id=$1 AND room_id=$2',[p.desktop_id,p.room_id]);
   await tx.query('INSERT INTO memberships(desktop_id,room_id,account_id,presence_until,ready_config_revision,selected) VALUES($1,$2,$3,$4,$5,true) ON CONFLICT(desktop_id) DO UPDATE SET presence_until=EXCLUDED.presence_until,ready_config_revision=EXCLUDED.ready_config_revision,selected=true,roster_retired=false',[actor.sessionId,p.room_id,actor.accountId,new Date(this.now+REJOIN_LEASE_MS),p.config_revision]);
   const sid=randomUUID(),epoch=randomUUID(),aid=randomUUID(),authorization=randomUUID();await tx.query("INSERT INTO sessions(id,family_id,kind,epoch,parent_id) VALUES($1,$2,'game',$3,$4)",[sid,actor.familyId,epoch,actor.sessionId]);
   await tx.query('INSERT INTO attachments(id,run_id,desktop_id,game_session_id,game_instance_id,service_epoch,run_epoch,config_revision,presence_until,resume_until) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$9)',[aid,p.run_id,actor.sessionId,sid,input.gameInstanceId,this.db.epoch,p.run_epoch,p.config_revision,new Date(this.now+REJOIN_LEASE_MS)]);
   await tx.query("UPDATE rejoin_participants SET desktop_id=$2,attachment_id=$3,generation=generation+1,state='inspecting',reason='fresh_inspection_required',authorization_id=$4,pending_report=NULL WHERE id=$1",[p.id,actor.sessionId,aid,authorization]);p=await this.read(tx,p.id);
   const credentials=await this.auth.mint(tx,actor.accountId,actor.deviceId,'game',actor.familyId,sid,epoch,aid),result={participant:this.view(p),credentials};
   await tx.query('INSERT INTO rejoin_requests(participant_id,account_id,request_id,fingerprint,generation,desktop_id,service_epoch,result_ciphertext) VALUES($1,$2,$3,$4,$5,$6,$7,$8)',[p.id,actor.accountId,req.requestId,fp,p.generation,actor.sessionId,this.db.epoch,this.auth.security.seal(result)]);await recordOutcome(tx,original,aid,this.now);return result;
  }
  await this.owner(tx,p,actor,req.generation);
  if(req.observe)await this.observe(tx,p,actor,req.observe);
  if(req.pulse)await this.lease(tx,p);
  p=await this.read(tx,p.id);const release=await this.release(tx,p);return {participant:this.view(p),...(release?{release}:{})};
 }
}
