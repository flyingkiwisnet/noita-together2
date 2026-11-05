import {randomUUID} from 'node:crypto';
import type {Tx} from '../g1/database.js';import type {Actor} from '../g1/auth.js';
import type {RejoinRooms} from './rooms.js';
import {demand,hash,matches,secret,fingerprint,PolicyError} from '../g1/security.js';
import {operationFingerprint} from '../../../../packages/contracts/src/connection-fingerprint.js';
import {seedPolicy,effectiveLoot} from '../../../../packages/domain/src/seed-policy.js';
import {contentDigest,contentFailure,validateObservation} from '../g2-next/observation.js';
import {remaining} from '../g3/connection.js';
import type {Request,WaitingOriginal} from '../../../../packages/contracts/g3-waiting/g3-waiting.js';
import {ENTRY_CURRENT_LINEAGE_LIMIT} from './entry-bounds.js';
import {schema as rejoinSchema} from '../../../../packages/contracts/src/g3-rejoin.js';

const RETENTION=30*86400000;
export class WaitingAuthority {
 constructor(readonly rooms:RejoinRooms){}
 get db(){return this.rooms.db;}get auth(){return this.rooms.principalAuth;}get now(){return this.auth.now();}
 async read(tx:Tx,id:string){
  const key=(await tx.query('SELECT run_id FROM waiting_worlds WHERE id=$1',[id])).rows[0];if(!key)return;
  await tx.query('SELECT r.id FROM rooms r JOIN runs n ON n.room_id=r.id WHERE n.id=$1 FOR UPDATE OF r',[key.run_id]);
  return (await tx.query(`SELECT w.*,to_jsonb(a) AS a,to_jsonb(g) AS g,to_jsonb(d) AS d,to_jsonb(f) AS f,to_jsonb(p) AS p,to_jsonb(n) AS n,to_jsonb(r) AS r,to_jsonb(o) AS o,to_jsonb(m) AS m,to_jsonb(q) AS q,u.disabled,
   EXISTS(SELECT 1 FROM room_bans b WHERE b.room_id=r.id AND b.account_id=w.account_id) AS banned,
   EXISTS(SELECT 1 FROM preparations newer JOIN sessions nd ON nd.id=newer.desktop_id JOIN session_families nf ON nf.id=nd.family_id WHERE newer.run_id=w.run_id AND nf.account_id=w.account_id AND nf.device_id=w.device_id AND newer.issued_at>p.issued_at AND newer.id<>p.id) AS superseded,
   (SELECT id FROM rejoin_participants WHERE attachment_id=a.id) AS admitted_id
   FROM waiting_worlds w JOIN preparations p ON p.id=w.preparation_id JOIN runs n ON n.id=w.run_id JOIN rooms r ON r.id=n.room_id
   JOIN attachments a ON a.id=w.attachment_id JOIN sessions g ON g.id=a.game_session_id JOIN sessions d ON d.id=w.desktop_id JOIN session_families f ON f.id=g.family_id
   JOIN accounts u ON u.id=w.account_id JOIN principal_profiles q ON q.principal_id=w.account_id
   LEFT JOIN world_observations o ON o.attachment_id=a.id LEFT JOIN memberships m ON m.desktop_id=w.desktop_id AND m.room_id=r.id WHERE w.id=$1 FOR UPDATE OF w`,[id])).rows[0];
 }
 reason(w:any){
  if(w.offline)return 'explicit_offline';if(w.retired_by||w.p.retired_by)return 'replaced_by_fresh';
  if(w.disabled||w.banned||w.q.kind==='guest'&&(!w.r.allow_guests||+new Date(w.q.guest_expires_at)<=this.now))return 'access_revoked';
  if(w.r.current_run_id!==w.run_id||!['staging','running'].includes(w.r.state)||!['staging','running'].includes(w.n.state))return 'run_unavailable';
  if(w.intent.preparationServiceEpoch!==this.db.epoch||w.n.service_epoch!==this.db.epoch||w.intent.runEpoch!==w.n.epoch)return 'epoch_changed';
  if(w.r.config_revision!==w.intent.configRevision||w.n.config_revision!==w.intent.configRevision||fingerprint(w.r.config)!==w.intent.configDigest||fingerprint(w.n.config)!==w.intent.configDigest)return 'configuration_changed';
  if(!w.m||w.m.account_id!==w.account_id)return 'membership_removed';
  if(w.superseded)return 'newer_preparation';
  if(w.p.state!=='loaded')return 'preparation_retired';
  if(w.n.state==='staging'&&+new Date(w.n.review_until)<=this.now)return 'review_expired';
 }
 until(w:any){return Math.min(+new Date(w.a.presence_until),w.m?.presence_until?+new Date(w.m.presence_until):0,+new Date(w.f.expires_at),w.q.guest_expires_at?+new Date(w.q.guest_expires_at):Infinity);}
 healthy(w:any){return !w.a.revoked_at&&!w.g.revoked_at&&!w.d.revoked_at&&!w.f.revoked_at&&this.until(w)>this.now;}
 own(w:any,actor:Actor){demand(w&&actor.kind==='desktop'&&w.account_id===actor.accountId&&w.device_id===actor.deviceId,'FORBIDDEN');}
 view(w:any,actor:Actor,requestId:string){
  const reason=this.reason(w),started=!reason&&w.n.state==='running',valid=!!w.o?.body&&!contentFailure(w.o.body,w.frozen_config,this.rooms.compatibility);
  return {intent:w.intent,generation:w.generation,currentContextId:w.context_id,
   owner:{familyId:w.f.id,desktopId:w.desktop_id,desktopSessionEpoch:w.d.epoch,gameSessionId:w.g.id,gameSessionEpoch:w.g.epoch,attachmentId:w.attachment_id,gameInstanceId:w.a.game_instance_id},
   state:reason?'terminal':started?'started':this.healthy(w)?'waiting':'recoverable',reason:reason??(started?w.admitted_id?'use_initial_admission':'not_selected_by_start':this.healthy(w)?'awaiting_host_start':'owner_disconnected'),mayPlay:false,
   reviewRemainingMs:remaining(+new Date(w.n.review_until),this.now,3600000),ownerRemainingMs:reason?0:remaining(this.until(w),this.now,30000),
   sample:{originRequestId:requestId,channel:'waiting-http',credentialId:actor.credentialId??'',sampleKey:fingerprint([this.db.serviceId,this.db.epoch,w.id,w.generation,w.attachment_id,actor.sessionId,actor.sessionEpoch,actor.credentialId,requestId,'waiting-http'])},
   currentReport:{...(w.o?{revision:w.o.revision,contentDigest:w.o.content_digest,capturedAt:new Date(w.o.captured_at).toISOString()}:{}),complete:!!w.o?.body?.inventoryComplete,loaded:!!w.o?.body?.loaded,protected:!!w.o?.body?.protected,contentValid:valid},...(started&&w.admitted_id?{admittedParticipantId:w.admitted_id}:{})};
 }
 result(w:any,actor:Actor,req:Request,proof=false){return {waiting:this.view(w,actor,req.requestId!),...(proof&&!this.reason(w)&&w.n.state==='staging'&&w.desktop_id===actor.sessionId?{waitingProof:this.auth.security.open<string>(w.proof_ciphertext)}:{})};}
 original(actor:Actor,req:Request):WaitingOriginal{return {accountId:actor.accountId,deviceId:actor.deviceId,originalSessionId:actor.sessionId,originalSessionEpoch:actor.sessionEpoch,originalServiceEpoch:req.serviceEpoch!,requestId:req.requestId!,kind:req.enroll?'enroll':req.recover?'recover':'offline',fingerprint:operationFingerprint(req),...(req.waitingId?{waitingId:req.waitingId}:{})};}
 async operation(tx:Tx,original:WaitingOriginal){return (await tx.query('SELECT * FROM waiting_operations WHERE account_id=$1 AND device_id=$2 AND original_session_id=$3 AND request_id=$4',[original.accountId,original.deviceId,original.originalSessionId,original.requestId])).rows[0];}
 async record(tx:Tx,original:WaitingOriginal,w:any){
  await tx.query('DELETE FROM waiting_operations WHERE waiting_id=$1 AND expires_at<=$2',[w.id,new Date(this.now)]);
  demand(+(await tx.query('SELECT count(*) FROM waiting_operations WHERE waiting_id=$1',[w.id])).rows[0].count<1024,'CAPACITY');
  await tx.query('INSERT INTO waiting_operations(account_id,device_id,original_session_id,request_id,kind,original,fingerprint,waiting_id,effect_id,effect_generation,created_at,expires_at) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12)',[original.accountId,original.deviceId,original.originalSessionId,original.requestId,original.kind,original,original.fingerprint,w.id,w.attachment_id,w.generation,new Date(this.now),new Date(this.now+RETENTION)]);
 }
 async outcome(tx:Tx,actor:Actor,original:WaitingOriginal,requestId:string){
  demand(actor.kind==='desktop'&&original.accountId===actor.accountId&&original.deviceId===actor.deviceId,'FORBIDDEN');
  const row=await this.operation(tx,original),exact=!!row&&operationFingerprint(row.original)===operationFingerprint(original)&&+row.expires_at>this.now,w=exact?await this.read(tx,row.waiting_id):undefined;
  if(w)this.own(w,actor);
  return {outcome:{original,originalOutcome:exact?'committed':'unresolved',coverage:exact?'exact_durable_record':'absent_expired_or_uncovered',...(exact?{waitingId:row.waiting_id,effectId:row.effect_id}:{}),currentDisposition:w?this.view(w,actor,requestId).state:'unknown',retryAfterMs:0},...(w?this.result(w,actor,{requestId},false):{})};
 }
 async fence(tx:Tx,w:any){
  await tx.query('UPDATE attachments SET revoked_at=$2 WHERE id=$1 AND revoked_at IS NULL',[w.attachment_id,new Date(this.now)]);
  await tx.query('UPDATE sessions SET revoked_at=$2 WHERE id=$1 AND revoked_at IS NULL',[w.g.id,new Date(this.now)]);
  await tx.query('UPDATE g3_contexts SET revoked_at=$2 WHERE attachment_id=$1 AND revoked_at IS NULL',[w.attachment_id,new Date(this.now)]);
 }
 async syncPending(tx:Tx,w:any){
  // The room transaction already serializes both authority paths. A direct
  // waiting transition also invalidates pending commands for its prior owner;
  // exact operation rows and original intent remain immutable.
  const rows=(await tx.query('SELECT id,generation FROM startup_pending WHERE waiting_id=$1 AND (waiting_generation IS DISTINCT FROM $2 OR desktop_id<>$3 OR offline IS DISTINCT FROM $4) FOR UPDATE',[w.id,w.generation,w.desktop_id,w.offline])).rows;
  for(const z of rows){demand(z.generation<0x7fffffff,'CAPACITY');await tx.query('UPDATE startup_pending SET waiting_generation=$2,desktop_id=$3,offline=$4,generation=generation+1 WHERE id=$1',[z.id,w.generation,w.desktop_id,w.offline]);}
 }
 async admittedRecovery(tx:Tx,w:any,actor:Actor,req:Request){
  demand(w.generation<0x7fffffff,'CAPACITY');
  const id=(await tx.query('SELECT id FROM rejoin_participants WHERE run_id=$1 AND account_id=$2 AND device_id=$3 AND world_token=$4',[w.run_id,w.account_id,w.device_id,w.intent.worldToken])).rows[0]?.id;
  demand(id,'NOT_SELECTED_BY_START');const p=await this.rooms.recovery.read(tx,id);
  demand(p&&!p.terminal&&p.attachment_id===w.attachment_id,'RETURN_INELIGIBLE');
  const input=req.recover!,observation=input.observation!;validateObservation(observation);
  demand(observation.attachmentId===w.attachment_id&&observation.worldToken===w.intent.worldToken&&observation.protected&&observation.loaded&&observation.inventoryComplete&&!contentFailure(observation,w.frozen_config,this.rooms.compatibility)&&contentDigest(observation)===w.intent.contentDigest,'REPORT_CHANGED');
  if(await this.rooms.recovery.healthy(tx,p))throw new PolicyError('OWNER_ACTIVE','OWNER_ACTIVE',Math.max(250,remaining(+new Date(p.owner_until),this.now,30000)));
  const resumed=await this.rooms.recovery.execute(tx,actor,{major:1,revision:rejoinSchema.revision,serviceId:this.db.serviceId,serviceEpoch:this.db.epoch,sessionEpoch:actor.sessionEpoch,requestId:req.requestId!,roomId:p.room_id,runId:p.run_id,worldToken:p.world_token,participantId:p.id,configRevision:p.config_revision,generation:p.generation,resume:{resumeProof:this.auth.security.open<string>(p.proof_ciphertext),gameInstanceId:observation.gameInstanceId!,gameVersion:input.gameVersion!,modVersion:input.modVersion!,bridgeVersion:input.bridgeVersion!,capabilities:input.capabilities??[]}});
  const credentials=resumed.credentials!,gameActor=await this.auth.authenticate(tx,credentials.accessToken!);
  await tx.query('UPDATE attachments SET preparation_id=NULL WHERE id=$1',[w.attachment_id]);
  await tx.query('UPDATE attachments SET preparation_id=$2 WHERE id=$1',[credentials.attachmentId,w.preparation_id]);
  await tx.query('UPDATE preparations SET desktop_id=$2,desktop_epoch=$3 WHERE id=$1',[w.preparation_id,actor.sessionId,actor.sessionEpoch]);
  await tx.query('UPDATE waiting_worlds SET attachment_id=$2,desktop_id=$3,generation=generation+1,context_id=$4 WHERE id=$1',[w.id,credentials.attachmentId,actor.sessionId,input.contextId]);
  const current=await this.rooms.recovery.read(tx,p.id);await this.rooms.recovery.observe(tx,current,gameActor,{...observation,attachmentId:credentials.attachmentId!});
  const replaced=await this.read(tx,w.id);await this.syncPending(tx,replaced);await this.record(tx,this.original(actor,req),replaced);
  return {...this.result(replaced,actor,req),credentials,participant:this.rooms.recovery.view(await this.rooms.recovery.read(tx,p.id)),resumeProof:this.auth.security.open<string>(p.proof_ciphertext)};
 }
 async offline(tx:Tx,w:any){
  demand(!w.retired_by&&!w.p.retired_by,'RETURN_INELIGIBLE');
  if(w.admitted_id)await this.rooms.recovery.retire(tx,w.admitted_id,'explicit_offline','offline');await this.fence(tx,w);await tx.query('UPDATE waiting_worlds SET offline=true WHERE id=$1',[w.id]);
  await tx.query("UPDATE preparations SET state='cancelled' WHERE id=$1",[w.preparation_id]);
  await tx.query('DELETE FROM memberships WHERE desktop_id=$1 AND room_id=$2 AND EXISTS(SELECT 1 FROM rooms WHERE id=$2 AND current_run_id=$3)',[w.desktop_id,w.r.id,w.run_id]);
  const retired=await this.read(tx,w.id);await this.syncPending(tx,retired);return retired;
 }
 async execute(tx:Tx,actor:Actor,req:Request):Promise<any>{
  demand(actor.kind==='desktop','FORBIDDEN');
  if(req.outcome)return this.outcome(tx,actor,req.outcome,req.requestId!);
  if(req.status){const w=req.waitingId?await this.read(tx,req.waitingId):undefined;this.own(w,actor);return this.result(w,actor,req,true);}
  const original=this.original(actor,req),old=await this.operation(tx,original);
  if(old){
   demand(old.fingerprint===original.fingerprint&&operationFingerprint(old.original)===operationFingerprint(original),'CONFLICT');demand(+old.expires_at>this.now,'OPERATION_UNRESOLVED');
   const w=await this.read(tx,old.waiting_id);this.own(w,actor);const value=this.result(w,actor,req,!!req.enroll);
   if(req.recover&&!this.reason(w)&&w.generation===old.effect_generation&&w.desktop_id===actor.sessionId&&this.healthy(w)){const p=w.admitted_id?await this.rooms.recovery.read(tx,w.admitted_id):undefined;return {...value,credentials:await this.auth.mint(tx,w.account_id,w.device_id,'game',actor.familyId,w.g.id,w.g.epoch,w.attachment_id),...(p&&!p.terminal?{participant:this.rooms.recovery.view(p),resumeProof:this.auth.security.open<string>(p.proof_ciphertext)}:{})};}
   return {...value,...await this.outcome(tx,actor,original,req.requestId!)};
  }
  if(req.enroll)return this.enroll(tx,actor,req,original);
  const w=req.waitingId?await this.read(tx,req.waitingId):undefined;this.own(w,actor);
  const proof=req.recover?.waitingProof??req.offline?.waitingProof;
  demand(/^[A-Za-z0-9_-]{43}$/.test(proof??'')&&matches(hash(proof!),w.proof_hash),'FORBIDDEN');demand(req.generation===w.generation,'STALE_EPOCH');
  if(req.offline){
   // Old-world offline cannot remove membership in another current run.
   const retired=await this.offline(tx,w);await this.record(tx,original,retired);return this.result(retired,actor,req);
  }
  demand(!this.reason(w),'RETURN_INELIGIBLE');
  if(w.n.state==='running')return this.admittedRecovery(tx,w,actor,req);
  demand(w.n.state==='staging'&&w.r.state==='staging','WAITING_STARTED');
  if(this.healthy(w))throw new PolicyError('OWNER_ACTIVE','OWNER_ACTIVE',Math.max(250,remaining(this.until(w),this.now,30000)));
  const input=req.recover!,observation=input.observation!;validateObservation(observation);
  demand(input.gameVersion===this.rooms.compatibility.gameVersion&&input.modVersion===this.rooms.compatibility.modVersion&&input.bridgeVersion===this.rooms.compatibility.bridgeVersion&&(w.frozen_config.requiredCapabilities??[]).every((v:string)=>input.capabilities?.includes(v)),'INCOMPATIBLE_GAME');
  demand(observation.attachmentId===w.attachment_id&&observation.worldToken===w.intent.worldToken&&observation.protected&&observation.loaded&&observation.inventoryComplete&&!contentFailure(observation,w.frozen_config,this.rooms.compatibility)&&contentDigest(observation)===w.intent.contentDigest,'REPORT_CHANGED');
  demand(w.generation<0x7fffffff,'CAPACITY');
  const elsewhere=(await tx.query('SELECT room_id FROM memberships WHERE desktop_id=$1',[actor.sessionId])).rows[0];demand(!elsewhere||elsewhere.room_id===w.r.id,'FORBIDDEN');
  await this.fence(tx,w);
  await tx.query('UPDATE attachments SET preparation_id=NULL WHERE id=$1',[w.attachment_id]);
  await tx.query('DELETE FROM memberships WHERE desktop_id=$1 AND room_id=$2',[w.desktop_id,w.r.id]);
  await tx.query('INSERT INTO memberships(desktop_id,room_id,account_id,presence_until,ready_config_revision) VALUES($1,$2,$3,$4,$5) ON CONFLICT(desktop_id) DO UPDATE SET presence_until=EXCLUDED.presence_until,ready_config_revision=EXCLUDED.ready_config_revision,selected=false',[actor.sessionId,w.r.id,actor.accountId,new Date(this.now+30000),w.intent.configRevision]);
  const sid=randomUUID(),epoch=randomUUID(),aid=randomUUID();
  await tx.query("INSERT INTO sessions(id,family_id,kind,epoch,parent_id) VALUES($1,$2,'game',$3,$4)",[sid,actor.familyId,epoch,actor.sessionId]);
  await tx.query('INSERT INTO attachments(id,run_id,desktop_id,game_session_id,game_instance_id,service_epoch,run_epoch,config_revision,presence_until,resume_until,preparation_id) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$9,$10)',[aid,w.run_id,actor.sessionId,sid,observation.gameInstanceId,this.db.epoch,w.n.epoch,w.intent.configRevision,new Date(this.now+30000),w.preparation_id]);
  await tx.query('UPDATE preparations SET desktop_id=$2,desktop_epoch=$3 WHERE id=$1',[w.preparation_id,actor.sessionId,actor.sessionEpoch]);
  await tx.query('UPDATE waiting_worlds SET attachment_id=$2,desktop_id=$3,generation=generation+1,context_id=$4 WHERE id=$1',[w.id,aid,actor.sessionId,input.contextId]);
  const credentials=await this.auth.mint(tx,actor.accountId,actor.deviceId,'game',actor.familyId,sid,epoch,aid),gameActor=await this.auth.authenticate(tx,credentials.accessToken!);
  await this.rooms.inspection.observe(tx,gameActor,{serviceId:this.db.serviceId,serviceEpoch:this.db.epoch,sessionEpoch:epoch,roomId:w.r.id,runId:w.run_id,runEpoch:w.n.epoch,configRevision:w.intent.configRevision},{...observation,attachmentId:aid});
  await tx.query('UPDATE rooms SET revision=revision+1 WHERE id=$1',[w.r.id]);
  const replaced=await this.read(tx,w.id);await this.syncPending(tx,replaced);await this.record(tx,original,replaced);return {...this.result(replaced,actor,req),credentials};
 }
 async enroll(tx:Tx,actor:Actor,req:Request,original:WaitingOriginal){
  const input=req.enroll!,key=(await tx.query('SELECT run_id FROM attachments WHERE id=$1',[input.attachmentId])).rows[0];demand(key,'FORBIDDEN');
  await tx.query('SELECT r.id FROM rooms r JOIN runs n ON n.room_id=r.id WHERE n.id=$1 FOR UPDATE OF r',[key.run_id]);
  const a=(await tx.query(`SELECT a.*,to_jsonb(p) AS p,to_jsonb(n) AS n,to_jsonb(r) AS r,to_jsonb(o) AS o,to_jsonb(g) AS g,to_jsonb(d) AS d,to_jsonb(f) AS f,to_jsonb(m) AS m
   FROM attachments a JOIN preparations p ON p.id=a.preparation_id JOIN runs n ON n.id=a.run_id JOIN rooms r ON r.id=n.room_id
   JOIN sessions g ON g.id=a.game_session_id JOIN sessions d ON d.id=a.desktop_id JOIN session_families f ON f.id=g.family_id
   LEFT JOIN world_observations o ON o.attachment_id=a.id LEFT JOIN memberships m ON m.desktop_id=a.desktop_id AND m.room_id=r.id WHERE a.id=$1 FOR UPDATE OF p`,[input.attachmentId])).rows[0];
  demand(a&&a.desktop_id===actor.sessionId&&a.f.account_id===actor.accountId&&a.f.device_id===actor.deviceId&&a.f.id===actor.familyId&&a.g.parent_id===actor.sessionId&&!a.revoked_at&&!a.g.revoked_at&&!a.d.revoked_at&&!a.f.revoked_at,'FORBIDDEN');
  demand(a.p.id===input.preparationId&&a.p.run_id===a.run_id&&a.p.desktop_id===a.desktop_id&&a.p.service_epoch===this.db.epoch&&a.service_epoch===this.db.epoch&&a.n.service_epoch===this.db.epoch&&a.p.run_epoch===a.n.epoch&&a.run_epoch===a.n.epoch&&a.r.current_run_id===a.run_id,'STALE_EPOCH');
  demand(a.r.state==='staging'&&a.n.state==='staging'&&a.p.state==='loaded'&&+new Date(a.n.review_until)>this.now,'WAITING_UNAVAILABLE');
  demand(a.m&&a.m.account_id===actor.accountId&&a.m.ready_config_revision===a.r.config_revision&&+new Date(a.m.presence_until)>this.now&&+new Date(a.presence_until)>this.now,'NOT_READY');
  demand(a.config_revision===a.r.config_revision&&a.p.config_revision===a.r.config_revision&&a.n.config_revision===a.r.config_revision&&fingerprint(a.n.config)===fingerprint(a.r.config),'STALE_REVISION');
  const principal=await this.auth.principal(tx,actor.accountId,true);demand(principal.kind!=='guest'||a.r.allow_guests,'FORBIDDEN');demand(!(await tx.query('SELECT 1 FROM room_bans WHERE room_id=$1 AND account_id=$2',[a.r.id,actor.accountId])).rowCount,'FORBIDDEN');
  const op=a.p.consumed_operation;demand(op&&op.requestId===input.originalAttachRequestId&&op.fingerprint===input.originalAttachFingerprint&&op.sessionId===actor.sessionId&&op.sessionEpoch===a.p.desktop_epoch&&op.serviceEpoch===this.db.epoch,'ORIGINAL_OPERATION_MISMATCH');
  demand(a.o&&a.o.world_token===input.worldToken&&a.game_instance_id===input.gameInstanceId&&a.o.revision===input.observationRevision&&a.o.content_digest===input.contentDigest&&a.o.body.protected&&a.o.body.loaded&&a.o.body.inventoryComplete&&!contentFailure(a.o.body,a.r.config,this.rooms.compatibility),'REPORT_CHANGED');
  const existing=(await tx.query('SELECT id,intent FROM waiting_worlds WHERE preparation_id=$1',[a.p.id])).rows[0];
  if(existing){const w=await this.read(tx,existing.id);this.own(w,actor);demand(w.intent.contextId===input.contextId&&w.intent.worldToken===input.worldToken&&w.intent.originalAttachmentId===input.attachmentId,'CONFLICT');await this.record(tx,original,w);return this.result(w,actor,req,true);}
  demand(+(await tx.query('SELECT count(*) FROM waiting_worlds WHERE run_id=$1 AND retired_by IS NULL AND NOT offline',[a.run_id])).rows[0].count<ENTRY_CURRENT_LINEAGE_LIMIT,'CAPACITY');
  const id=randomUUID(),proof=secret(),intent={waitingId:id,serviceId:this.db.serviceId,accountId:actor.accountId,deviceId:actor.deviceId,preparationId:a.p.id,originalAttachmentId:a.id,originalAttachRequestId:op.requestId,originalAttachFingerprint:op.fingerprint,originalDesktopId:op.sessionId,originalDesktopEpoch:op.sessionEpoch,roomId:a.r.id,runId:a.run_id,runEpoch:a.n.epoch,preparationServiceEpoch:this.db.epoch,configRevision:a.r.config_revision,configDigest:fingerprint(a.r.config),worldToken:input.worldToken,actualSeed:a.o.body.actualSeed,seedPolicy:seedPolicy(a.r.config),randomizeLoot:effectiveLoot(a.r.config),originalGameInstanceId:a.game_instance_id,contextId:input.contextId,originalObservationRevision:a.o.revision,contentDigest:a.o.content_digest};
  await tx.query('INSERT INTO waiting_worlds(id,preparation_id,run_id,account_id,device_id,intent,frozen_config,proof_hash,proof_ciphertext,attachment_id,desktop_id,context_id,created_at) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13)',[id,a.p.id,a.run_id,actor.accountId,actor.deviceId,intent,a.r.config,hash(proof),this.auth.security.seal(proof),a.id,actor.sessionId,input.contextId,new Date(this.now)]);
  const pending=(await tx.query('SELECT id,intent FROM startup_pending WHERE preparation_id=$1',[a.p.id])).rows[0];
  if(pending){demand(pending.intent.worldToken===input.worldToken,'REPORT_CHANGED');await tx.query('UPDATE startup_pending SET waiting_id=$2,waiting_generation=1 WHERE id=$1',[pending.id,id]);}
  const w=await this.read(tx,id);await this.record(tx,original,w);return this.result(w,actor,req,true);
 }
}
