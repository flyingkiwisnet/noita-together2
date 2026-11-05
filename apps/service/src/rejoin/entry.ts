import {randomUUID} from 'node:crypto';
import type {Tx} from '../g1/database.js';import type {Actor} from '../g1/auth.js';
import type {Request,EntryScope,EntryIdentity,EntryOriginal} from '../../../../packages/contracts/g3-waiting/g3-waiting.js';
import {demand,hash,matches,fingerprint,PolicyError} from '../g1/security.js';
import {operationFingerprint} from '../../../../packages/contracts/src/connection-fingerprint.js';
import {seedPolicy,effectiveLoot} from '../../../../packages/domain/src/seed-policy.js';
import {remaining} from '../g3/connection.js';import {validateObservation,contentFailure} from '../g2-next/observation.js';
import {PendingAuthority} from './pending.js';
import {ENTRY_CURRENT_LINEAGE_LIMIT,ENTRY_RETIRED_LINEAGE_LIMIT,ENTRY_OPERATION_LIMIT,ENTRY_OUTCOME_RETENTION_MS} from './entry-bounds.js';
const uuid=(s:unknown)=>typeof s==='string'&&/^[a-f0-9]{8}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{12}$/i.test(s);
export class EntryAuthority {
 constructor(readonly pending:PendingAuthority){}
 get waiting(){return this.pending.waiting;}get rooms(){return this.waiting.rooms;}get db(){return this.waiting.db;}get auth(){return this.waiting.auth;}get now(){return this.auth.now();}
 async room(tx:Tx,id:string,lock:'UPDATE'|'SHARE'='UPDATE'){demand(uuid(id),'INVALID_MESSAGE');const r=(await tx.query(`SELECT * FROM rooms WHERE id=$1 FOR ${lock}`,[id])).rows[0];demand(r,'NOT_FOUND');return r;}
 async lineage(tx:Tx,r:any,a:Actor){
  const rows=(await tx.query(`SELECT p.*,z.id AS pending_id,z.generation AS pending_generation,w.id AS waiting_id,w.generation AS waiting_generation,
   g.id AS attachment_id,g.game_session_id,s.epoch AS game_epoch,COALESCE(w.intent->>'worldToken',z.intent->>'worldToken',o.world_token) AS world_token
   FROM preparations p JOIN sessions d ON d.id=p.desktop_id JOIN session_families f ON f.id=d.family_id
   LEFT JOIN startup_pending z ON z.preparation_id=p.id LEFT JOIN waiting_worlds w ON w.preparation_id=p.id
   LEFT JOIN attachments g ON g.preparation_id=p.id LEFT JOIN sessions s ON s.id=g.game_session_id LEFT JOIN world_observations o ON o.attachment_id=g.id
   WHERE p.run_id=$1 AND f.account_id=$2 AND f.device_id=$3 AND p.retired_by IS NULL AND p.state IN ('issued','consumed','loaded') ORDER BY p.issued_at DESC,p.id DESC LIMIT 2`,[r.current_run_id,a.accountId,a.deviceId])).rows;
  demand(rows.length<=1,'ENTRY_BASE_AMBIGUOUS');const p=rows[0];
  const baseFingerprint=fingerprint([this.db.serviceId,this.db.epoch,r.current_run_id,r.config_revision,p?.id??null,p?.desktop_id??null,p?.desktop_epoch??null,p?.pending_id??null,p?.pending_generation??null,p?.waiting_id??null,p?.waiting_generation??null,p?.attachment_id??null,p?.game_session_id??null,p?.game_epoch??null]);
  return {p,baseFingerprint};
 }
 async eligible(tx:Tx,r:any,n:any,a:Actor,p:any){
  demand(r.state==='staging'&&n?.state==='staging'&&r.current_run_id===n.id,'ENTRY_NOT_STAGING');
  demand(n.service_epoch===this.db.epoch&&n.config_revision===r.config_revision&&fingerprint(n.config)===fingerprint(r.config),'STALE_EPOCH');demand(+new Date(n.review_until)>this.now,'EXPIRED');
  const principal=await this.auth.principal(tx,a.accountId,true);demand(principal.kind!=='guest'||r.allow_guests,'FORBIDDEN');
  demand(!(await tx.query('SELECT 1 FROM room_bans WHERE room_id=$1 AND account_id=$2',[r.id,a.accountId])).rowCount,'FORBIDDEN');
  const m=(await tx.query('SELECT * FROM memberships WHERE room_id=$1 AND desktop_id=$2 AND account_id=$3',[r.id,p?.desktop_id??a.sessionId,a.accountId])).rows[0];demand(m&&m.ready_config_revision===r.config_revision,'FORBIDDEN');
  if(p){demand(p.service_epoch===this.db.epoch&&p.run_epoch===n.epoch&&p.config_revision===r.config_revision,'STALE_EPOCH');demand(p.state==='loaded'||+new Date(p.expires_at)>this.now,'EXPIRED');}
 }
 scope(r:any,n:any,a:Actor,base:any):EntryScope{return {serviceId:this.db.serviceId,serviceEpoch:this.db.epoch,accountId:a.accountId,deviceId:a.deviceId,roomId:r.id,runId:n.id,runEpoch:n.epoch,configRevision:r.config_revision,configDigest:fingerprint(r.config),...(base.p?{basePreparationId:base.p.id}:{}),baseFingerprint:base.baseFingerprint,seedPolicy:seedPolicy(r.config),randomizeLoot:effectiveLoot(r.config),...(seedPolicy(r.config)==='individual-random'?{}:{actualSeed:String(r.config.seed)})};}
 async preview(tx:Tx,a:Actor,req:Request){const r=await this.room(tx,req.entryPreview!.roomId!,'SHARE'),n=(await tx.query('SELECT * FROM runs WHERE id=$1',[r.current_run_id])).rows[0],base=await this.lineage(tx,r,a);await this.eligible(tx,r,n,a,base.p);if(req.entryPreview!.basePreparationId)demand(base.p?.id===req.entryPreview!.basePreparationId,'STALE_EPOCH');
  return {entryPreview:{scope:this.scope(r,n,a,base),sample:{originRequestId:req.requestId,channel:'entry-http',credentialId:a.credentialId,sampleKey:fingerprint([this.db.serviceId,this.db.epoch,base.baseFingerprint,a.sessionId,a.sessionEpoch,a.credentialId,req.requestId,'entry-http'])},reviewRemainingMs:remaining(+new Date(n.review_until),this.now,3600000),mayPlay:false}};
 }
 target(a:Actor,scope:EntryScope,id:EntryIdentity,proof?:string){
  demand(scope.serviceId===this.db.serviceId&&scope.accountId===a.accountId&&scope.deviceId===a.deviceId,'FORBIDDEN');
  for(const value of [scope.roomId,scope.runId,scope.runEpoch,scope.serviceEpoch,id.preparationId,id.pendingId,...(scope.basePreparationId?[scope.basePreparationId]:[])])demand(uuid(value),'INVALID_MESSAGE');
  demand(id.preparationId!==id.pendingId&&id.preparationId!==scope.basePreparationId&&!!id.worldToken&&!!id.contextId&&/^[a-f0-9]{64}$/.test(id.proofCommitment??'')&&/^[a-f0-9]{64}$/.test(scope.baseFingerprint??'')&&/^[a-f0-9]{64}$/.test(scope.configDigest??''),'INVALID_MESSAGE');
  if(proof!==undefined)demand(/^[A-Za-z0-9_-]{43}$/.test(proof)&&matches(hash(proof),id.proofCommitment!),'FORBIDDEN');
 }
 async find(tx:Tx,a:Actor,scope:EntryScope,id:EntryIdentity){
  const rows=(await tx.query('SELECT * FROM startup_entry_effects WHERE pending_id=$1 OR preparation_id=$2 OR (run_id=$3 AND world_token=$4)',[id.pendingId,id.preparationId,scope.runId,id.worldToken])).rows;
  if(!rows.length)return;demand(rows.length===1,'CONFLICT');const e=rows[0];demand(e.account_id===a.accountId&&e.device_id===a.deviceId,'FORBIDDEN');demand(operationFingerprint(e.scope)===operationFingerprint(scope)&&operationFingerprint(e.identity)===operationFingerprint(id),'CONFLICT');return e;
 }
 view(e:any,scope:EntryScope,id:EntryIdentity){return {scope,identity:id,state:e?.state??'unresolved',reason:e?e.state==='cancelled'?'explicit_offline':'use_current_pending':'no_effect_record',mayPlay:false,...(e?.attachment_id?{attachmentId:e.attachment_id}:{})};}
 async result(tx:Tx,a:Actor,e:any,req:Request,secrets=false){
  const z=e.state==='activated'?await this.pending.read(tx,e.pending_id):undefined;let value:any={entry:this.view(e,e.scope,e.identity),...(z?this.pending.result(z,a):{})};
  if(z?.waiting_id){const w=await this.waiting.read(tx,z.waiting_id);value={...value,...this.waiting.result(w,a,req,false)};
   if(secrets&&z.generation===e.pending_generation&&w.generation===e.waiting_generation&&w.desktop_id===a.sessionId&&!this.waiting.reason(w)&&this.waiting.healthy(w)){
    const participant=w.admitted_id?await this.rooms.recovery.read(tx,w.admitted_id):undefined;
    value={...value,...this.waiting.result(w,a,req,true),credentials:await this.auth.mint(tx,a.accountId,a.deviceId,'game',a.familyId,w.g.id,w.g.epoch,w.attachment_id),...(participant&&!participant.terminal?{participant:this.rooms.recovery.view(participant),resumeProof:this.auth.security.open<string>(participant.proof_ciphertext)}:{})};
   }
  }return value;
 }
 original(a:Actor,r:Request,scope:EntryScope,id:EntryIdentity):EntryOriginal{return {accountId:a.accountId,deviceId:a.deviceId,originalSessionId:a.sessionId,originalSessionEpoch:a.sessionEpoch,originalServiceEpoch:r.serviceEpoch!,requestId:r.requestId!,kind:r.entryActivate?'entryActivate':'entryCancel',fingerprint:operationFingerprint(r),runId:scope.runId!,pendingId:id.pendingId!};}
 async operation(tx:Tx,o:EntryOriginal){return (await tx.query('SELECT * FROM startup_entry_operations WHERE account_id=$1 AND device_id=$2 AND original_session_id=$3 AND request_id=$4',[o.accountId,o.deviceId,o.originalSessionId,o.requestId])).rows[0];}
 async outcome(tx:Tx,a:Actor,o:EntryOriginal){demand(o.accountId===a.accountId&&o.deviceId===a.deviceId,'FORBIDDEN');const old=await this.operation(tx,o),exact=!!old&&operationFingerprint(old.original)===operationFingerprint(o)&&+old.expires_at>this.now;return {entryOutcome:{original:o,originalOutcome:exact?'committed':'unresolved',coverage:exact?'exact_durable_record':'absent_expired_or_uncovered',...(exact?{effectId:old.effect_id}:{})}};}
 async record(tx:Tx,o:EntryOriginal,e:any){
  await tx.query('DELETE FROM startup_entry_operations WHERE account_id=$1 AND device_id=$2 AND run_id=$3 AND expires_at<=$4',[o.accountId,o.deviceId,o.runId,new Date(this.now)]);
  demand(+(await tx.query('SELECT count(*) FROM startup_entry_operations WHERE account_id=$1 AND device_id=$2 AND run_id=$3',[o.accountId,o.deviceId,o.runId])).rows[0].count<ENTRY_OPERATION_LIMIT,'CAPACITY');
  await tx.query('INSERT INTO startup_entry_operations(account_id,device_id,run_id,original_session_id,request_id,kind,original,fingerprint,pending_id,effect_id,created_at,expires_at) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12)',[o.accountId,o.deviceId,o.runId,o.originalSessionId,o.requestId,o.kind,o,o.fingerprint,e.pending_id,e.attachment_id??e.pending_id,new Date(this.now),new Date(this.now+ENTRY_OUTCOME_RETENTION_MS)]);
 }
 async insert(tx:Tx,a:Actor,scope:EntryScope,id:EntryIdentity,state:string){
  demand(!(await tx.query('SELECT 1 FROM preparations WHERE id=$1 UNION ALL SELECT 1 FROM startup_pending WHERE id=$2 OR (run_id=$3 AND intent->>\'worldToken\'=$4)',[id.preparationId,id.pendingId,scope.runId,id.worldToken])).rowCount,'CONFLICT');
  return (await tx.query('INSERT INTO startup_entry_effects(pending_id,preparation_id,run_id,account_id,device_id,world_token,scope,identity,proof_hash,state,created_at) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11) RETURNING *',[id.pendingId,id.preparationId,scope.runId,a.accountId,a.deviceId,id.worldToken,scope,id,id.proofCommitment,state,new Date(this.now)])).rows[0];
 }
 async execute(tx:Tx,a:Actor,req:Request):Promise<any>{
  demand(a.kind==='desktop','FORBIDDEN');if(req.entryPreview)return this.preview(tx,a,req);if(req.entryOutcome)return this.outcome(tx,a,req.entryOutcome);
  const input=req.entryActivate??req.entryCancel??req.entryStatus!,scope=input.scope!,id=input.identity!;this.target(a,scope,id,req.entryActivate?.recoveryProof??req.entryCancel?.recoveryProof);
  const r=await this.room(tx,scope.roomId!),n=(await tx.query('SELECT * FROM runs WHERE id=$1 AND room_id=$2',[scope.runId,scope.roomId])).rows[0];demand(n,'FORBIDDEN');let e=await this.find(tx,a,scope,id);
  if(req.entryStatus)return e?this.result(tx,a,e,req):{entry:this.view(undefined,scope,id)};
  const original=this.original(a,req,scope,id),old=await this.operation(tx,original);
  if(old){demand(operationFingerprint(old.original)===operationFingerprint(original),'CONFLICT');demand(+old.expires_at>this.now,'OPERATION_UNRESOLVED');demand(e&&old.pending_id===e.pending_id,'CONFLICT');return {...await this.result(tx,a,e,req,!!req.entryActivate),...await this.outcome(tx,a,original)};}
  if(req.entryCancel){
   demand(req.entryCancel.worldOffline===true,'INVALID_MESSAGE');
   if(!e){const own=scope.basePreparationId?(await tx.query('SELECT 1 FROM preparations p JOIN sessions d ON d.id=p.desktop_id JOIN session_families f ON f.id=d.family_id WHERE p.id=$1 AND p.run_id=$2 AND f.account_id=$3 AND f.device_id=$4',[scope.basePreparationId,scope.runId,a.accountId,a.deviceId])).rowCount:(await tx.query('SELECT 1 FROM memberships WHERE room_id=$1 AND desktop_id=$2 AND account_id=$3',[scope.roomId,a.sessionId,a.accountId])).rowCount;demand(!!own,'FORBIDDEN');e=await this.insert(tx,a,scope,id,'cancelled');}
   else if(e.state==='activated'){
    const z=await this.pending.read(tx,e.pending_id);this.pending.own(z,a);
    if(!z.p.retired_by){demand(z.waiting_id,'RETURN_INELIGIBLE');await this.waiting.offline(tx,await this.waiting.read(tx,z.waiting_id));}
    e=(await tx.query("UPDATE startup_entry_effects SET state='cancelled' WHERE pending_id=$1 RETURNING *",[e.pending_id])).rows[0];
   }
   await this.record(tx,original,e);return this.result(tx,a,e,req);
  }
  if(e){demand(e.state==='activated','ENTRY_CANCELLED');await this.record(tx,original,e);return this.result(tx,a,e,req);}
  const base=await this.lineage(tx,r,a);await this.eligible(tx,r,n,a,base.p);
  demand(operationFingerprint(this.scope(r,n,a,base))===operationFingerprint(scope),'STALE_ENTRY_BASE');
  demand(id.worldToken!==base.p?.world_token,'CONFLICT');if(base.p)demand(!(await tx.query('SELECT 1 FROM startup_pending WHERE preparation_id=$1 AND proof_hash=$2 UNION ALL SELECT 1 FROM waiting_worlds WHERE preparation_id=$1 AND proof_hash=$2',[base.p.id,id.proofCommitment])).rowCount,'CONFLICT');
  if(base.p?.waiting_id){const w=await this.waiting.read(tx,base.p.waiting_id);demand(!this.waiting.reason(w),'RETURN_INELIGIBLE');if(this.waiting.healthy(w))throw new PolicyError('OWNER_ACTIVE','OWNER_ACTIVE',Math.max(250,remaining(this.waiting.until(w),this.now,30000)));}
  else if(base.p?.attachment_id){const active=(await tx.query('SELECT a.presence_until FROM attachments a JOIN sessions g ON g.id=a.game_session_id JOIN sessions d ON d.id=a.desktop_id JOIN session_families f ON f.id=g.family_id JOIN memberships m ON m.desktop_id=d.id AND m.room_id=$2 WHERE a.id=$1 AND a.revoked_at IS NULL AND g.revoked_at IS NULL AND d.revoked_at IS NULL AND f.revoked_at IS NULL AND a.presence_until>$3 AND m.presence_until>$3 AND f.expires_at>$3',[base.p.attachment_id,r.id,new Date(this.now)])).rows[0];if(active)throw new PolicyError('OWNER_ACTIVE','OWNER_ACTIVE',Math.max(250,remaining(+new Date(active.presence_until),this.now,30000)));}
  const inputActivate=req.entryActivate!,observation=inputActivate.observation!,aid=randomUUID();demand(observation.attachmentId===undefined&&observation.worldToken===id.worldToken&&observation.protected&&observation.loaded&&observation.inventoryComplete,'REPORT_CHANGED');validateObservation({...observation,attachmentId:aid});demand(!contentFailure(observation,r.config,this.rooms.compatibility),'REPORT_CHANGED');
  demand(+(await tx.query('SELECT count(*) FROM preparations WHERE run_id=$1 AND retired_by IS NOT NULL',[n.id])).rows[0].count+(base.p?1:0)<=ENTRY_RETIRED_LINEAGE_LIMIT,'CAPACITY');
  e=await this.insert(tx,a,scope,id,'activated');
  if(base.p){
   await tx.query("UPDATE preparations SET state='cancelled',retired_by=$2 WHERE id=$1",[base.p.id,id.pendingId]);
   await tx.query('UPDATE waiting_worlds SET retired_by=$2 WHERE preparation_id=$1',[base.p.id,id.pendingId]);
   await tx.query('UPDATE attachments SET revoked_at=$2 WHERE preparation_id=$1 AND revoked_at IS NULL',[base.p.id,new Date(this.now)]);
   await tx.query('UPDATE sessions SET revoked_at=$2 WHERE id IN (SELECT game_session_id FROM attachments WHERE preparation_id=$1)',[base.p.id,new Date(this.now)]);
   await tx.query('UPDATE g3_contexts SET revoked_at=$2 WHERE attachment_id IN (SELECT id FROM attachments WHERE preparation_id=$1)',[base.p.id,new Date(this.now)]);
  }
  const elsewhere=(await tx.query('SELECT room_id FROM memberships WHERE desktop_id=$1',[a.sessionId])).rows[0];demand(!elsewhere||elsewhere.room_id===r.id,'FORBIDDEN');
  if(base.p)await tx.query('DELETE FROM memberships WHERE desktop_id=$1 AND room_id=$2',[base.p.desktop_id,r.id]);
  await tx.query('INSERT INTO memberships(desktop_id,room_id,account_id,presence_until,ready_config_revision) VALUES($1,$2,$3,$4,$5) ON CONFLICT(desktop_id) DO UPDATE SET presence_until=EXCLUDED.presence_until,ready_config_revision=EXCLUDED.ready_config_revision,selected=false',[a.sessionId,r.id,a.accountId,new Date(this.now+30000),r.config_revision]);
  demand(+(await tx.query('SELECT count(*) FROM waiting_worlds WHERE run_id=$1 AND retired_by IS NULL AND NOT offline',[n.id])).rows[0].count<ENTRY_CURRENT_LINEAGE_LIMIT,'CAPACITY');
  await tx.query("INSERT INTO preparations(id,run_id,desktop_id,desktop_epoch,service_epoch,run_epoch,config_revision,state,issued_at,expires_at) VALUES($1,$2,$3,$4,$5,$6,$7,'issued',$8,$9)",[id.preparationId,n.id,a.sessionId,a.sessionEpoch,this.db.epoch,n.epoch,r.config_revision,new Date(this.now),new Date(Math.min(this.now+600000,+new Date(n.review_until)))]);
  const intent={pendingId:id.pendingId,preparationId:id.preparationId,serviceId:this.db.serviceId,serviceEpoch:this.db.epoch,roomId:r.id,runId:n.id,runEpoch:n.epoch,configRevision:r.config_revision,configDigest:scope.configDigest,accountId:a.accountId,deviceId:a.deviceId,originalDesktopId:a.sessionId,originalDesktopEpoch:a.sessionEpoch,worldToken:id.worldToken,contextId:id.contextId,seedPolicy:scope.seedPolicy,randomizeLoot:scope.randomizeLoot,...(scope.actualSeed?{actualSeed:scope.actualSeed}:{})};
  await tx.query('INSERT INTO startup_pending(id,preparation_id,run_id,account_id,device_id,intent,frozen_config,proof_hash,proof_ciphertext,desktop_id,created_at) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)',[id.pendingId,id.preparationId,n.id,a.accountId,a.deviceId,intent,r.config,id.proofCommitment,this.auth.security.seal(inputActivate.recoveryProof),a.sessionId,new Date(this.now)]);
  const value=await this.pending.execute(tx,a,JSON.parse(JSON.stringify({major:req.major,revision:req.revision,serviceId:req.serviceId,serviceEpoch:req.serviceEpoch,sessionEpoch:req.sessionEpoch,requestId:req.requestId,pendingId:id.pendingId,generation:1,pendingRecover:{pendingProof:inputActivate.recoveryProof,contextId:id.contextId,gameVersion:inputActivate.gameVersion,modVersion:inputActivate.modVersion,bridgeVersion:inputActivate.bridgeVersion,capabilities:inputActivate.capabilities,observation}})),false);
  e=(await tx.query('UPDATE startup_entry_effects SET attachment_id=$2,pending_generation=$3,waiting_generation=$4 WHERE pending_id=$1 RETURNING *',[id.pendingId,value.credentials.attachmentId,value.pending.generation,value.waiting.generation])).rows[0];
  await this.record(tx,original,e);return {...value,entry:this.view(e,scope,id)};
 }
}
