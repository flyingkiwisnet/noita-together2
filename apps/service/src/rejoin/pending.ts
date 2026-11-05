import {randomUUID} from 'node:crypto';
import type {Tx} from '../g1/database.js';import type {Actor} from '../g1/auth.js';
import type {Request,WaitingOriginal} from '../../../../packages/contracts/g3-waiting/g3-waiting.js';
import {demand,secret,hash,matches,fingerprint,PolicyError} from '../g1/security.js';
import {operationFingerprint} from '../../../../packages/contracts/src/connection-fingerprint.js';
import {seedPolicy,effectiveLoot,seedMatches} from '../../../../packages/domain/src/seed-policy.js';
import {validateObservation,contentDigest,contentFailure} from '../g2-next/observation.js';
import {WaitingAuthority} from './waiting.js';
import {ENTRY_CURRENT_LINEAGE_LIMIT} from './entry-bounds.js';

// Reservation is durable identity before local consumption. Loaded evidence and
// release decisions still belong to the existing inspection/admission owners.
export class PendingAuthority {
 constructor(readonly waiting:WaitingAuthority){}
 get db(){return this.waiting.db;}get auth(){return this.waiting.auth;}get now(){return this.auth.now();}
 async read(tx:Tx,id:string){
  const key=(await tx.query('SELECT run_id FROM startup_pending WHERE id=$1',[id])).rows[0];if(!key)return;
  await tx.query('SELECT r.id FROM rooms r JOIN runs n ON n.room_id=r.id WHERE n.id=$1 FOR UPDATE OF r',[key.run_id]);
  return (await tx.query(`SELECT z.*,to_jsonb(p) AS p,to_jsonb(n) AS n,to_jsonb(r) AS r,u.disabled,to_jsonb(q) AS q,
   EXISTS(SELECT 1 FROM memberships m WHERE m.desktop_id=z.desktop_id AND m.room_id=r.id AND m.account_id=z.account_id) AS member,
   EXISTS(SELECT 1 FROM room_bans b WHERE b.room_id=r.id AND b.account_id=z.account_id) AS banned,
   EXISTS(SELECT 1 FROM preparations newer JOIN sessions d ON d.id=newer.desktop_id JOIN session_families f ON f.id=d.family_id WHERE newer.run_id=z.run_id AND f.account_id=z.account_id AND f.device_id=z.device_id AND newer.issued_at>p.issued_at AND newer.id<>p.id) AS superseded
   FROM startup_pending z JOIN preparations p ON p.id=z.preparation_id JOIN runs n ON n.id=z.run_id JOIN rooms r ON r.id=n.room_id JOIN accounts u ON u.id=z.account_id JOIN principal_profiles q ON q.principal_id=z.account_id WHERE z.id=$1 FOR UPDATE OF z`,[id])).rows[0];
 }
 own(z:any,a:Actor){demand(z&&a.kind==='desktop'&&z.account_id===a.accountId&&z.device_id===a.deviceId,'FORBIDDEN');}
 reason(z:any){
  if(z.offline)return 'explicit_offline';if(z.p.retired_by)return 'replaced_by_fresh';
  if(z.disabled||z.banned||z.q.kind==='guest'&&(!z.r.allow_guests||+new Date(z.q.guest_expires_at)<=this.now))return 'access_revoked';
  if(z.r.current_run_id!==z.run_id||!['staging','running'].includes(z.r.state)||!['staging','running'].includes(z.n.state))return 'run_unavailable';
  if(z.intent.serviceEpoch!==this.db.epoch||z.n.service_epoch!==this.db.epoch||z.n.epoch!==z.intent.runEpoch)return 'epoch_changed';
  if(z.r.config_revision!==z.intent.configRevision||fingerprint(z.r.config)!==z.intent.configDigest||fingerprint(z.n.config)!==z.intent.configDigest)return 'configuration_changed';
  if(!z.member)return 'membership_removed';if(z.superseded)return 'newer_preparation';
  if(!['issued','consumed','loaded'].includes(z.p.state))return 'preparation_retired';
  if(z.p.state!=='loaded'&&+new Date(z.p.expires_at)<=this.now)return 'preparation_expired';
  if(z.n.state==='staging'&&+new Date(z.n.review_until)<=this.now)return 'review_expired';
 }
 view(z:any){const reason=this.reason(z);return {intent:z.intent,generation:z.generation,preparationState:['issued','consumed'].includes(z.p.state)&&+new Date(z.p.expires_at)<=this.now?'expired':z.p.state,state:reason?'terminal':z.waiting_id?'promoted':'pending',reason:reason??(z.waiting_id?'use_waiting_or_admitted':'fresh_inspection_required'),mayPlay:false,preparationRemainingMs:reason?0:Math.max(0,Math.min(600000,+new Date(z.p.expires_at)-this.now)),reviewRemainingMs:reason?0:Math.max(0,Math.min(3600000,+new Date(z.n.review_until)-this.now)),...(z.bound_seed?{boundActualSeed:z.bound_seed}:{}),...(z.waiting_id?{waitingId:z.waiting_id}:{})};}
 result(z:any,a:Actor,proof=false){return {pending:this.view(z),...(proof&&!this.reason(z)&&z.desktop_id===a.sessionId?{pendingProof:this.auth.security.open<string>(z.proof_ciphertext)}:{})};}
 original(a:Actor,r:Request):WaitingOriginal{return {accountId:a.accountId,deviceId:a.deviceId,originalSessionId:a.sessionId,originalSessionEpoch:a.sessionEpoch,originalServiceEpoch:r.serviceEpoch!,requestId:r.requestId!,kind:r.reserve?'reserve':r.pendingRecover?'pendingRecover':'pendingOffline',fingerprint:operationFingerprint(r)};}
 async record(tx:Tx,z:any,original:WaitingOriginal,effectId:string){
  await tx.query('DELETE FROM startup_pending_operations WHERE pending_id=$1 AND expires_at<=$2',[z.id,new Date(this.now)]);
  demand(+(await tx.query('SELECT count(*) FROM startup_pending_operations WHERE pending_id=$1',[z.id])).rows[0].count<1024,'CAPACITY');
  await tx.query('INSERT INTO startup_pending_operations(account_id,device_id,original_session_id,request_id,original,fingerprint,pending_id,effect_id,effect_generation,created_at,expires_at) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)',[original.accountId,original.deviceId,original.originalSessionId,original.requestId,original,original.fingerprint,z.id,effectId,z.generation,new Date(this.now),new Date(this.now+30*86400000)]);
 }
 async find(tx:Tx,o:WaitingOriginal){return (await tx.query('SELECT * FROM startup_pending_operations WHERE account_id=$1 AND device_id=$2 AND original_session_id=$3 AND request_id=$4',[o.accountId,o.deviceId,o.originalSessionId,o.requestId])).rows[0];}
 async outcome(tx:Tx,a:Actor,o:WaitingOriginal){
  demand(o.accountId===a.accountId&&o.deviceId===a.deviceId,'FORBIDDEN');
  if(o.kind==='gameAttach'){
   const found=(await tx.query("SELECT z.id,p.consumed_operation AS op FROM startup_pending z JOIN preparations p ON p.id=z.preparation_id WHERE z.account_id=$1 AND z.device_id=$2 AND p.consumed_operation->>'sessionId'=$3 AND p.consumed_operation->>'requestId'=$4",[a.accountId,a.deviceId,o.originalSessionId,o.requestId])).rows[0],op=found?.op;
   const exact=!!op&&op.sessionEpoch===o.originalSessionEpoch&&op.serviceEpoch===o.originalServiceEpoch&&op.fingerprint===o.fingerprint&&!o.waitingId&&Number.isFinite(Date.parse(op.createdAt))&&Date.parse(op.createdAt)+30*86400000>this.now;
   const z=exact?await this.read(tx,found.id):undefined;
   return {pendingOutcome:{original:o,originalOutcome:exact?'committed':'unresolved',coverage:exact?'exact_consumed_attach_record':'absent_expired_or_uncovered',...(exact?{pendingId:z.id,effectId:op.attachmentId}:{})},...(z?this.result(z,a):{})};
  }
  const old=await this.find(tx,o),exact=!!old&&operationFingerprint(old.original)===operationFingerprint(o)&&+old.expires_at>this.now;
  const z=exact?await this.read(tx,old.pending_id):undefined;if(z)this.own(z,a);
  return {pendingOutcome:{original:o,originalOutcome:exact?'committed':'unresolved',coverage:exact?'exact_durable_record':'absent_expired_or_uncovered',...(exact?{pendingId:old.pending_id,effectId:old.effect_id}:{})},...(z?this.result(z,a):{})};
 }
 async execute(tx:Tx,a:Actor,r:Request,journal=true):Promise<any>{
  demand(a.kind==='desktop','FORBIDDEN');if(r.pendingOutcome)return this.outcome(tx,a,r.pendingOutcome);
  if(r.pendingStatus){const z=await this.read(tx,r.pendingId!);this.own(z,a);return this.result(z,a,true);}
  const original=this.original(a,r),old=journal?await this.find(tx,original):undefined;
  if(old){
   demand(old.fingerprint===original.fingerprint&&operationFingerprint(old.original)===operationFingerprint(original),'CONFLICT');demand(+old.expires_at>this.now,'OPERATION_UNRESOLVED');
   const z=await this.read(tx,old.pending_id);this.own(z,a);const result=this.result(z,a,!!r.reserve);
   if(r.pendingRecover&&z.waiting_id&&z.generation===old.effect_generation&&!this.reason(z)&&z.desktop_id===a.sessionId){const w=await this.waiting.read(tx,z.waiting_id);if(w&&w.generation===z.waiting_generation&&w.desktop_id===a.sessionId&&!this.waiting.reason(w)&&this.waiting.healthy(w)){const p=w.admitted_id?await this.waiting.rooms.recovery.read(tx,w.admitted_id):undefined;return {...result,...this.waiting.result(w,a,r,true),credentials:await this.auth.mint(tx,a.accountId,a.deviceId,'game',a.familyId,w.g.id,w.g.epoch,w.attachment_id),...(p&&!p.terminal?{participant:this.waiting.rooms.recovery.view(p),resumeProof:this.auth.security.open<string>(p.proof_ciphertext)}:{})};}}
   return {...result,...await this.outcome(tx,a,original)};
  }
  if(r.reserve)return this.reserve(tx,a,r,original);
  let z=await this.read(tx,r.pendingId!);this.own(z,a);demand(z.generation<0x7fffffff,'CAPACITY');const proof=r.pendingRecover?.pendingProof??r.pendingOffline?.pendingProof;
  demand(/^[A-Za-z0-9_-]{43}$/.test(proof??'')&&matches(hash(proof!),z.proof_hash),'FORBIDDEN');demand(r.generation===z.generation,'STALE_EPOCH');
  if(!r.pendingOffline)demand(!this.reason(z),'RETURN_INELIGIBLE');
  if(z.waiting_id){
   const w=await this.waiting.read(tx,z.waiting_id);demand(w&&w.generation===z.waiting_generation,'STALE_EPOCH');
   const forwarded={...r,pendingId:undefined,pendingRecover:undefined,pendingOffline:undefined,waitingId:w.id,generation:w.generation,...(r.pendingOffline?{offline:{waitingProof:this.auth.security.open<string>(w.proof_ciphertext)}}:{recover:{...r.pendingRecover,pendingProof:undefined,waitingProof:this.auth.security.open<string>(w.proof_ciphertext)}})};
   // Remove local-only fields before journaling the canonical waiting operation.
   const result=await this.waiting.execute(tx,a,JSON.parse(JSON.stringify(forwarded)));
   // WaitingAuthority synchronizes linked current owner/generation/offline in
   // the same transaction, whether invoked here or through the direct route.
   z=await this.read(tx,z.id);if(journal)await this.record(tx,z,original,result.waiting.owner.attachmentId);return {...this.result(z,a),...this.waiting.result(await this.waiting.read(tx,w.id),a,r,!r.pendingOffline),...result};
  }
  if(r.pendingOffline){
   demand(!z.p.retired_by,'RETURN_INELIGIBLE');
   // Scoped tombstone, even if the original run has already ended.
   const attachment=(await tx.query('SELECT id FROM attachments WHERE preparation_id=$1',[z.preparation_id])).rows[0];
   if(attachment){const p=(await tx.query('SELECT id FROM rejoin_participants WHERE attachment_id=$1',[attachment.id])).rows[0];if(p)await this.waiting.rooms.recovery.retire(tx,p.id,'explicit_offline','offline');await tx.query('UPDATE g3_contexts SET revoked_at=$2 WHERE attachment_id=$1',[attachment.id,new Date(this.now)]);await tx.query('UPDATE attachments SET revoked_at=$2 WHERE id=$1',[attachment.id,new Date(this.now)]);await tx.query('UPDATE sessions SET revoked_at=$2 WHERE id=(SELECT game_session_id FROM attachments WHERE id=$1)',[attachment.id,new Date(this.now)]);}
   await tx.query("UPDATE preparations SET state='cancelled' WHERE id=$1",[z.preparation_id]);await tx.query('UPDATE startup_pending SET offline=true,generation=generation+1 WHERE id=$1',[z.id]);
   z=await this.read(tx,z.id);if(journal)await this.record(tx,z,original,z.id);return this.result(z,a);
  }
  demand(!this.reason(z),'RETURN_INELIGIBLE');
  const input=r.pendingRecover!,observation=input.observation!,aid=randomUUID();validateObservation({...observation,attachmentId:observation.attachmentId??aid});
  const c=this.waiting.rooms.compatibility;
  demand(input.gameVersion===c.gameVersion&&input.modVersion===c.modVersion&&input.bridgeVersion===c.bridgeVersion&&(z.frozen_config.requiredCapabilities??[]).every((v:string)=>input.capabilities?.includes(v)),'INCOMPATIBLE_GAME');
  demand(observation.worldToken===z.intent.worldToken&&observation.protected&&observation.loaded&&observation.inventoryComplete&&!contentFailure(observation,z.frozen_config,c)&&(!(z.bound_seed??z.intent.actualSeed)||(z.bound_seed??z.intent.actualSeed)===observation.actualSeed),'REPORT_CHANGED');
  const current=(await tx.query('SELECT a.*,to_jsonb(o) AS o FROM attachments a LEFT JOIN world_observations o ON o.attachment_id=a.id WHERE a.preparation_id=$1',[z.preparation_id])).rows[0];
  if(current){demand(observation.attachmentId===current.id,'REPORT_CHANGED');if(current.o)demand(current.o.world_token===z.intent.worldToken&&current.o.content_digest===contentDigest(observation),'REPORT_CHANGED');
   const live=(await tx.query('SELECT 1 FROM attachments a JOIN sessions s ON s.id=a.game_session_id JOIN sessions d ON d.id=a.desktop_id JOIN session_families f ON f.id=s.family_id WHERE a.id=$1 AND a.revoked_at IS NULL AND s.revoked_at IS NULL AND d.revoked_at IS NULL AND f.revoked_at IS NULL AND a.presence_until>$2 AND f.expires_at>$2',[current.id,new Date(this.now)])).rowCount;if(live)throw new PolicyError('OWNER_ACTIVE','OWNER_ACTIVE',Math.max(250,Math.min(30000,+new Date(current.presence_until)-this.now)));
  }else demand(!observation.attachmentId,'REPORT_CHANGED');
  if(z.n.state==='running'){
   const selected=current?(await tx.query('SELECT id FROM rejoin_participants WHERE attachment_id=$1 AND run_id=$2 AND account_id=$3 AND device_id=$4 AND world_token=$5 AND NOT terminal',[current.id,z.run_id,a.accountId,a.deviceId,z.intent.worldToken])).rows[0]:undefined;
   demand(selected&&current.o,'NOT_SELECTED_BY_START');
   const wid=randomUUID(),waitingProof=secret(),op=z.p.consumed_operation;
   const intent={waitingId:wid,serviceId:this.db.serviceId,accountId:a.accountId,deviceId:a.deviceId,preparationId:z.preparation_id,originalAttachmentId:current.id,...(op?{originalAttachRequestId:op.requestId,originalAttachFingerprint:op.fingerprint}:{}),originalDesktopId:z.intent.originalDesktopId,originalDesktopEpoch:z.intent.originalDesktopEpoch,roomId:z.r.id,runId:z.run_id,runEpoch:z.n.epoch,preparationServiceEpoch:this.db.epoch,configRevision:z.intent.configRevision,configDigest:z.intent.configDigest,worldToken:z.intent.worldToken,actualSeed:current.o.body.actualSeed,seedPolicy:z.intent.seedPolicy,randomizeLoot:z.intent.randomizeLoot,originalGameInstanceId:current.game_instance_id,contextId:z.intent.contextId,originalObservationRevision:current.o.revision,contentDigest:current.o.content_digest,pendingReservationId:z.id};
   await tx.query('INSERT INTO waiting_worlds(id,preparation_id,run_id,account_id,device_id,intent,frozen_config,proof_hash,proof_ciphertext,attachment_id,desktop_id,context_id,created_at) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13)',[wid,z.preparation_id,z.run_id,a.accountId,a.deviceId,intent,z.frozen_config,hash(waitingProof),this.auth.security.seal(waitingProof),current.id,z.desktop_id,z.intent.contextId,new Date(this.now)]);
   await tx.query('UPDATE startup_pending SET waiting_id=$2,waiting_generation=1 WHERE id=$1',[z.id,wid]);return this.execute(tx,a,r,journal);
  }
  demand(z.r.state==='staging'&&z.n.state==='staging','NOT_SELECTED_BY_START');
  const elsewhere=(await tx.query('SELECT room_id FROM memberships WHERE desktop_id=$1',[a.sessionId])).rows[0];demand(!elsewhere||elsewhere.room_id===z.r.id,'FORBIDDEN');
  await tx.query('UPDATE startup_pending SET bound_seed=COALESCE(bound_seed,$2) WHERE id=$1',[z.id,observation.actualSeed]);
  if(current){await tx.query('UPDATE attachments SET revoked_at=$2,preparation_id=NULL WHERE id=$1',[current.id,new Date(this.now)]);await tx.query('UPDATE sessions SET revoked_at=$2 WHERE id=$1',[current.game_session_id,new Date(this.now)]);await tx.query('UPDATE g3_contexts SET revoked_at=$2 WHERE attachment_id=$1',[current.id,new Date(this.now)]);}
  await tx.query('DELETE FROM memberships WHERE desktop_id=$1 AND room_id=$2',[z.desktop_id,z.r.id]);
  await tx.query('INSERT INTO memberships(desktop_id,room_id,account_id,presence_until,ready_config_revision) VALUES($1,$2,$3,$4,$5) ON CONFLICT(desktop_id) DO UPDATE SET presence_until=EXCLUDED.presence_until,ready_config_revision=EXCLUDED.ready_config_revision,selected=false',[a.sessionId,z.r.id,a.accountId,new Date(this.now+30000),z.intent.configRevision]);
  const sid=randomUUID(),epoch=randomUUID(),wid=randomUUID(),waitingProof=secret();
  await tx.query("INSERT INTO sessions(id,family_id,kind,epoch,parent_id) VALUES($1,$2,'game',$3,$4)",[sid,a.familyId,epoch,a.sessionId]);
  await tx.query('INSERT INTO attachments(id,run_id,desktop_id,game_session_id,game_instance_id,service_epoch,run_epoch,config_revision,presence_until,resume_until,preparation_id) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$9,$10)',[aid,z.run_id,a.sessionId,sid,observation.gameInstanceId,this.db.epoch,z.n.epoch,z.intent.configRevision,new Date(this.now+30000),z.preparation_id]);
  await tx.query("UPDATE preparations SET desktop_id=$2,desktop_epoch=$3,state='consumed' WHERE id=$1",[z.preparation_id,a.sessionId,a.sessionEpoch]);
  const credentials=await this.auth.mint(tx,a.accountId,a.deviceId,'game',a.familyId,sid,epoch,aid),gameActor=await this.auth.authenticate(tx,credentials.accessToken!);
  await this.waiting.rooms.inspection.observe(tx,gameActor,{serviceId:this.db.serviceId,serviceEpoch:this.db.epoch,sessionEpoch:epoch,roomId:z.r.id,runId:z.run_id,runEpoch:z.n.epoch,configRevision:z.intent.configRevision},{...observation,attachmentId:aid});
  const op=z.p.consumed_operation,intent={waitingId:wid,serviceId:this.db.serviceId,accountId:a.accountId,deviceId:a.deviceId,preparationId:z.preparation_id,originalAttachmentId:current?.id??aid,...(op?{originalAttachRequestId:op.requestId,originalAttachFingerprint:op.fingerprint}:{}),originalDesktopId:z.intent.originalDesktopId,originalDesktopEpoch:z.intent.originalDesktopEpoch,roomId:z.r.id,runId:z.run_id,runEpoch:z.n.epoch,preparationServiceEpoch:this.db.epoch,configRevision:z.intent.configRevision,configDigest:z.intent.configDigest,worldToken:z.intent.worldToken,actualSeed:observation.actualSeed,seedPolicy:z.intent.seedPolicy,randomizeLoot:z.intent.randomizeLoot,originalGameInstanceId:current?.game_instance_id??observation.gameInstanceId,contextId:z.intent.contextId,originalObservationRevision:current?.o?.revision??observation.observationRevision,contentDigest:contentDigest(observation),pendingReservationId:z.id};
  await tx.query('INSERT INTO waiting_worlds(id,preparation_id,run_id,account_id,device_id,intent,frozen_config,proof_hash,proof_ciphertext,attachment_id,desktop_id,context_id,created_at) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13)',[wid,z.preparation_id,z.run_id,a.accountId,a.deviceId,intent,z.frozen_config,hash(waitingProof),this.auth.security.seal(waitingProof),aid,a.sessionId,input.contextId,new Date(this.now)]);
  await tx.query('UPDATE startup_pending SET waiting_id=$2,waiting_generation=1,desktop_id=$3,generation=generation+1 WHERE id=$1',[z.id,wid,a.sessionId]);z=await this.read(tx,z.id);if(journal)await this.record(tx,z,original,aid);
  return {...this.result(z,a),...this.waiting.result(await this.waiting.read(tx,wid),a,r,true),credentials};
 }
 async reserve(tx:Tx,a:Actor,r:Request,original:WaitingOriginal){
  const input=r.reserve!,key=(await tx.query('SELECT run_id FROM preparations WHERE id=$1',[input.preparationId])).rows[0];demand(key,'FORBIDDEN');
  await tx.query('SELECT r.id FROM rooms r JOIN runs n ON n.room_id=r.id WHERE n.id=$1 FOR UPDATE OF r',[key.run_id]);
  const p=(await tx.query('SELECT p.*,to_jsonb(n) AS n,to_jsonb(r) AS r FROM preparations p JOIN runs n ON n.id=p.run_id JOIN rooms r ON r.id=n.room_id WHERE p.id=$1 FOR UPDATE OF p',[input.preparationId])).rows[0];
  demand(p.desktop_id===a.sessionId&&p.desktop_epoch===a.sessionEpoch,'FORBIDDEN');
  const existing=(await tx.query('SELECT id FROM startup_pending WHERE preparation_id=$1',[p.id])).rows[0];
  if(existing){const z=await this.read(tx,existing.id);this.own(z,a);demand(z.intent.worldToken===input.worldToken&&z.intent.contextId===input.contextId,'CONFLICT');demand(!this.reason(z),'RETURN_INELIGIBLE');await this.record(tx,z,original,z.id);return this.result(z,a,true);}
  demand(p.service_epoch===this.db.epoch&&p.n.service_epoch===this.db.epoch&&p.run_epoch===p.n.epoch&&p.r.current_run_id===p.run_id&&p.config_revision===p.r.config_revision&&p.n.config_revision===p.r.config_revision,'STALE_EPOCH');
  demand(p.r.state==='staging'&&p.n.state==='staging'&&+new Date(p.n.review_until)>this.now&&['issued','consumed','loaded'].includes(p.state)&&(p.state==='loaded'||+new Date(p.expires_at)>this.now),'RETURN_INELIGIBLE');
  demand(!!input.worldToken&&!!input.contextId,'INVALID_MESSAGE');
  // A genuinely new issued preparation may supersede unconsumed reservations.
  // Preserve their immutable operation/history rows; never silently replace a
  // consumed world merely because a new desktop issued another preparation.
  const unresolved=(await tx.query("SELECT 1 FROM startup_pending z JOIN preparations prior ON prior.id=z.preparation_id WHERE z.run_id=$1 AND z.account_id=$2 AND z.device_id=$3 AND z.preparation_id<>$4 AND NOT z.offline AND prior.retired_by IS NULL AND (prior.state IN ('consumed','loaded') OR prior.consumed_operation IS NOT NULL OR z.waiting_id IS NOT NULL) LIMIT 1",[p.run_id,a.accountId,a.deviceId,p.id])).rowCount;
  demand(!unresolved,'ORIGINAL_WORLD_UNRESOLVED');
  const id=randomUUID(),proof=secret(),policy=seedPolicy(p.r.config),actualSeed=policy==='individual-random'?undefined:String(p.r.config.seed);
  demand(actualSeed===undefined||/^(0|[1-9][0-9]{0,19})$/.test(actualSeed)&&BigInt(actualSeed)<=18446744073709551615n&&seedMatches(p.r.config,actualSeed),'REPORT_CHANGED');
  const observed=(await tx.query('SELECT o.body FROM attachments a JOIN world_observations o ON o.attachment_id=a.id WHERE a.preparation_id=$1',[p.id])).rows[0]?.body;
  demand(!observed||observed.worldToken===input.worldToken&&(!actualSeed||observed.actualSeed===actualSeed),'REPORT_CHANGED');
  const intent={pendingId:id,preparationId:p.id,serviceId:this.db.serviceId,serviceEpoch:this.db.epoch,roomId:p.r.id,runId:p.run_id,runEpoch:p.run_epoch,configRevision:p.config_revision,configDigest:fingerprint(p.r.config),accountId:a.accountId,deviceId:a.deviceId,originalDesktopId:a.sessionId,originalDesktopEpoch:a.sessionEpoch,worldToken:input.worldToken,contextId:input.contextId,seedPolicy:policy,randomizeLoot:effectiveLoot(p.r.config),actualSeed};
  demand(+(await tx.query('SELECT count(*) FROM startup_pending z JOIN preparations p ON p.id=z.preparation_id WHERE z.run_id=$1 AND p.retired_by IS NULL AND NOT z.offline',[p.run_id])).rows[0].count<ENTRY_CURRENT_LINEAGE_LIMIT,'CAPACITY');
  await tx.query('INSERT INTO startup_pending(id,preparation_id,run_id,account_id,device_id,intent,frozen_config,proof_hash,proof_ciphertext,desktop_id,created_at,bound_seed) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12)',[id,p.id,p.run_id,a.accountId,a.deviceId,JSON.parse(JSON.stringify(intent)),p.r.config,hash(proof),this.auth.security.seal(proof),a.sessionId,new Date(this.now),observed?.protected&&observed.loaded&&observed.inventoryComplete&&!contentFailure(observed,p.r.config,this.waiting.rooms.compatibility)?observed.actualSeed:actualSeed??null]);
  const z=await this.read(tx,id);demand(!this.reason(z),'RETURN_INELIGIBLE');await this.record(tx,z,original,z.id);return this.result(z,a,true);
 }
}
