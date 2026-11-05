import {addStatistic} from './stats-store.js';
import {executeItems,isItemRequest} from './items.js';
import {executeBankBatch} from './bank-batch.js';
import {CommittedBankReads,bankRevision} from './bank-events.js';
import type {RoomAdmissions} from './admission.js';
import {randomUUID} from 'node:crypto';
import type {Actor} from '../g1/auth.js';
import type {Tx} from '../g1/database.js';
import {PolicyError,demand,fingerprint} from '../g1/security.js';
import {GOLD_LIMIT,BALANCE_LIMIT,GOLD_CAPABILITY,GOLD_BANK_PAGE,GOLD_NOTICE_MS,sendGoldEnabled,G3_EVENT_CATALOG,receiptTransition} from '../../../../packages/domain/src/g3.js';
import type {Request,Operation,MovementFrame,Response,Scope} from '../../../../packages/contracts/g3/g3.js';
const uuid=(v:unknown)=>typeof v==='string'&&/^[a-f0-9]{8}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{12}$/.test(v);
export function operationDigest(principalId:string,scope:Scope,direction:string,amount:number){return fingerprint({v:1,principalId,scope:{roomId:scope.roomId,runId:scope.runId,runEpoch:scope.runEpoch,configRevision:scope.configRevision,attachmentId:scope.attachmentId,worldToken:scope.worldToken},direction,amount});}
export class G3Authority {
 readonly bankReads=new CommittedBankReads();
 admission?: (tx:Tx,roomId:string)=>Promise<RoomAdmissions>;
 roomBudget?: (roomId:string,tx:Tx)=>void;
 private ghosts=new Map<string,{principalId:string;gameSessionId:string;worldToken:string;frames:MovementFrame[];capturedAt:string;until:number;}>();
 constructor(readonly serviceEpoch:()=>string,readonly now:()=>number=Date.now,readonly onDeath?:(tx:Tx,actor:Actor,scope:Scope)=>Promise<void>){}
 private async access(tx:Tx,actor:Actor,req:Request,allowDeletedHistory=false){const scope=req.scope;demand(scope&&uuid(scope.runId)&&uuid(scope.roomId),'INVALID_MESSAGE');const r=(await tx.query('SELECT r.*,a.display_name FROM g3_runs r JOIN g3_access a ON a.run_id=r.id AND a.principal_id=$2 WHERE r.id=$1 AND r.room_id=$3 FOR UPDATE OF r',[scope.runId,actor.accountId,scope.roomId])).rows[0];demand(r,'NOT_FOUND');this.roomBudget?.(r.room_id,tx);if(this.admission&&!allowDeletedHistory){const room=(await tx.query('SELECT deleted_at FROM rooms WHERE id=$1',[r.room_id])).rows[0];demand(!room?.deleted_at,'ROOM_DELETED');}return r;}
 private async context(tx:Tx,actor:Actor,req:Request,knownRun?:Record<string,any>){const r=knownRun??await this.access(tx,actor,req),s=req.scope!;demand(actor.kind==='game'&&r.state==='running'&&r.epoch===s.runEpoch&&r.config_revision===s.configRevision,'FORBIDDEN');if(this.admission){const a=await this.admission(tx,s.roomId!);demand(a.worlds.some(w=>w.decision.playing&&w.owner.gameSessionId===actor.sessionId&&w.owner.gameSessionEpoch===actor.sessionEpoch&&w.owner.attachmentId===s.attachmentId&&w.scope.worldToken===s.worldToken&&w.scope.runEpoch===s.runEpoch&&w.scope.configRevision===s.configRevision),'STALE_EPOCH');}const c=(await tx.query('SELECT c.* FROM g3_contexts c JOIN sessions d ON d.id=$4 AND d.revoked_at IS NULL WHERE c.attachment_id=$1 AND c.run_id=$2 AND c.game_session_id=$3',[s.attachmentId,s.runId,actor.sessionId,actor.parentId])).rows[0];demand(c&&!c.revoked_at&&c.principal_id===actor.accountId&&c.world_token===s.worldToken&&c.session_epoch===actor.sessionEpoch&&c.service_epoch===this.serviceEpoch()&&+c.presence_until>this.now(),'STALE_EPOCH');return {r,c};}
 private view(o:Record<string,any>,balance:number,version:string,roomId:string,effectAllowed=false):Operation{return {operationId:o.operation_id,principalId:o.principal_id,runId:o.run_id,attachmentId:o.attachment_id,worldToken:o.world_token,direction:o.direction,amount:o.amount,payloadDigest:o.digest,deliveryId:o.delivery_id,state:o.state,balance,balanceVersion:String(version),localOutcome:o.local_outcome,originalScope:{roomId,runId:o.run_id,runEpoch:o.run_epoch,configRevision:o.config_revision,attachmentId:o.attachment_id,worldToken:o.world_token},effectAllowed,savePersistence:'unproven',provenance:'client-reported-local-outcome'};}
 private async balance(tx:Tx,runId:string){await tx.query('INSERT INTO g3_balances(run_id) VALUES($1) ON CONFLICT DO NOTHING',[runId]);return (await tx.query('SELECT amount,version FROM g3_balances WHERE run_id=$1 FOR UPDATE',[runId])).rows[0]!;}
 private async changeBalance(tx:Tx,runId:string,amount:number){const row=(await tx.query('UPDATE g3_balances SET amount=$2,version=version+1 WHERE run_id=$1 AND version<9223372036854775807 RETURNING amount,version',[runId,amount])).rows[0];demand(row,'BANK_FULL');return row;}
 private async goldEnabled(tx:Tx,r:Record<string,any>){
  if(!this.admission)return r.mode==='core'; // Explicit low-level ledger fixtures only.
  const row=(await tx.query('SELECT config FROM rooms WHERE id=$1 AND current_run_id=$2 AND config_revision=$3',[r.room_id,r.id,r.config_revision])).rows[0];return !!row&&sendGoldEnabled(row.config);
 }
 private async effectAllowed(tx:Tx,actor:Actor,req:Request,o:Record<string,any>,r:Record<string,any>){
  const current=req.currentScope??req.scope;
  if(actor.kind!=='game'||!['awaiting_local','reserved'].includes(o.state)||r.state!=='running'||!current||current.roomId!==r.room_id||current.runId!==o.run_id||current.runEpoch!==o.run_epoch||current.configRevision!==o.config_revision||current.worldToken!==o.world_token)return false;
  try{const {c}=await this.context(tx,actor,{...req,scope:current},r);return c.capabilities.includes(GOLD_CAPABILITY)&&await this.goldEnabled(tx,r);}catch(e){if(e instanceof PolicyError&&['FORBIDDEN','STALE_EPOCH','NOT_FOUND'].includes(e.code))return false;throw e;}
 }
 private async bank(tx:Tx,actor:Actor,req:Request,r:Record<string,any>,c:Record<string,any>){
  demand(c.capabilities.includes(GOLD_CAPABILITY),'FORBIDDEN');const p=req.goldBank!,limit=p.limit??GOLD_BANK_PAGE;demand(Number.isInteger(limit)&&limit>=1&&limit<=GOLD_BANK_PAGE,'INVALID_MESSAGE');
  demand(!p.afterOperationId||uuid(p.afterOperationId),'INVALID_MESSAGE');const revision=await bankRevision(tx,r.id),key=JSON.stringify(['gold',this.serviceEpoch(),r.id,r.epoch,r.config_revision,revision]),cached=this.bankReads.get<{balance:Record<string,any>;enabled:boolean}>(key,this.now());let balance:Record<string,any>,enabled:boolean;if(cached){demand((await tx.query('SELECT run_id FROM g3_balances WHERE run_id=$1 FOR UPDATE',[r.id])).rows[0],'STALE_EPOCH');balance=cached.balance;enabled=cached.enabled;}else{balance=await this.balance(tx,r.id);enabled=await this.goldEnabled(tx,r);this.bankReads.put(tx,key,{balance,enabled},this.now());}
  const latest=(await tx.query("SELECT coalesce(max(sequence),0)::text AS sequence FROM g3_outbox WHERE run_id=$1 AND principal_id=$2 AND kind='gold.deposit'",[r.id,actor.accountId])).rows[0].sequence;
  let cursor=latest,noticeReset=false,notices:any[]=[];
  if(p.afterNoticeSequence){const after=this.cursor(p.afterNoticeSequence);if(BigInt(after)>BigInt(latest))noticeReset=true;else{const rows=(await tx.query("SELECT sequence,body FROM g3_outbox WHERE run_id=$1 AND principal_id=$2 AND kind='gold.deposit' AND sequence>$3 AND created_at>$4 ORDER BY sequence LIMIT $5",[r.id,actor.accountId,after,new Date(this.now()-GOLD_NOTICE_MS),limit])).rows;notices=rows.map(v=>({...v.body,sequence:v.sequence}));cursor=rows.at(-1)?.sequence??latest;noticeReset=!rows.length&&BigInt(after)<BigInt(latest);}}
  const rows=(await tx.query("SELECT * FROM g3_operations WHERE run_id=$1 AND principal_id=$2 AND world_token=$3 AND state NOT IN ('completed','cancelled') AND ($4::uuid IS NULL OR operation_id>$4) ORDER BY operation_id LIMIT $5",[r.id,actor.accountId,c.world_token,p.afterOperationId||null,limit+1])).rows,page=rows.slice(0,limit);
  return {bank:{availableBalance:Number(balance.amount),balanceVersion:String(balance.version),sendGoldEnabled:enabled,notices,noticeSequence:cursor,noticeReset,operations:page.map(o=>this.view(o,Number(balance.amount),balance.version,r.room_id,enabled&&['awaiting_local','reserved'].includes(o.state)&&o.run_epoch===r.epoch&&o.config_revision===r.config_revision)),nextOperationId:rows.length>limit?page.at(-1)!.operation_id:''}};
 }
 private async outbox(tx:Tx,o:Operation,kind:string){await tx.query('INSERT INTO g3_outbox(run_id,principal_id,kind,body,created_at) VALUES($1,$2,$3,$4,$5)',[o.runId,o.principalId,kind,o,new Date(this.now())]);}
 private async statistic(tx:Tx,r:Record<string,any>,actor:Actor,kind:string,count:number,provenance:string){await tx.query('INSERT INTO g3_statistics(run_id,principal_id,display_name,version,kind,count,provenance,captured_at,expires_at) VALUES($1,$2,$3,1,$4,$5,$6,$7,$8)',[r.id,actor.accountId,r.display_name,kind,count,provenance,new Date(this.now()),new Date(this.now()+30*86400000)]);await tx.query('UPDATE g3_runs SET statistics_expires_at=greatest(statistics_expires_at,$2),statistics_deleted_at=NULL WHERE id=$1',[r.id,new Date(this.now()+30*86400000)]);}
 async execute(tx:Tx,actor:Actor,req:Request,afterCommit:(()=>void)[]=[]):Promise<Partial<Response>>{
  if(req.bankBatch){await this.access(tx,actor,req);return executeBankBatch(this,tx,actor,req,afterCommit);}
  if(req.statisticsList||req.statisticsAggregate||req.statisticsDelete){const r=await this.access(tx,actor,req,true);if(req.statisticsDelete){demand(r.owner_id===actor.accountId,'FORBIDDEN');await tx.query('DELETE FROM g3_statistics WHERE run_id=$1',[r.id]);await tx.query('UPDATE g3_runs SET statistics_deleted_at=$2,statistics_expires_at=NULL WHERE id=$1',[r.id,new Date(this.now())]);return {};}
   if(req.statisticsAggregate){const rows=(await tx.query('SELECT kind,provenance,sum(count)::text AS count FROM g3_statistics WHERE run_id=$1 AND expires_at>$2 GROUP BY kind,provenance ORDER BY kind,provenance LIMIT 16',[r.id,new Date(this.now())])).rows;return {aggregates:rows};}
   const p=req.statisticsList!,limit=this.limit(p.limit),after=this.cursor(p.afterSequence);const rows=(await tx.query('SELECT * FROM g3_statistics WHERE run_id=$1 AND sequence>$2 AND expires_at>$3 ORDER BY sequence LIMIT $4',[r.id,after,new Date(this.now()),limit])).rows;return {statistics:rows.map(v=>({sequence:v.sequence,version:v.version,kind:v.kind,principalId:v.principal_id,displayName:v.display_name,count:v.count,provenance:v.provenance,capturedAt:v.captured_at.toISOString()})),nextSequence:rows.at(-1)?.sequence??after};
  }
  if(isItemRequest(req))return executeItems({tx,actor,req,now:this.now(),access:this.access.bind(this),context:this.context.bind(this),reads:this.bankReads,serviceEpoch:this.serviceEpoch()});
  if(req.goldStatus||req.outboxList){
   const r=await this.access(tx,actor,req);
   if(req.outboxList){const p=req.outboxList!,rows=(await tx.query('SELECT * FROM g3_outbox WHERE run_id=$1 AND principal_id=$2 AND sequence>$3 ORDER BY sequence LIMIT $4',[r.id,actor.accountId,this.cursor(p.afterSequence),this.limit(p.limit)])).rows;return {outbox:rows.map(v=>({sequence:v.sequence,kind:v.kind,...(v.kind==='game.event'?{event:v.body}:v.kind==='gold.deposit'?{notice:{...v.body,sequence:v.sequence}}:v.kind==='item.deposit'?{itemNotice:{...v.body,sequence:v.sequence}}:v.kind.startsWith('item.')?{itemOperation:{...v.body,effectAllowed:false}}:{operation:{...v.body,effectAllowed:false}})})),nextSequence:rows.at(-1)?.sequence??this.cursor(p.afterSequence)};}
   demand(uuid(req.goldStatus!.operationId),'INVALID_MESSAGE');const o=(await tx.query('SELECT o.*,b.amount AS balance,b.version AS balance_version FROM g3_operations o JOIN g3_balances b ON b.run_id=o.run_id WHERE o.run_id=$1 AND o.principal_id=$2 AND o.operation_id=$3',[r.id,actor.accountId,req.goldStatus!.operationId])).rows[0];demand(o&&o.attachment_id===req.scope!.attachmentId&&o.world_token===req.scope!.worldToken&&o.run_epoch===req.scope!.runEpoch&&o.config_revision===req.scope!.configRevision,'NOT_FOUND');return {operation:this.view(o,Number(o.balance),o.balance_version,r.room_id,await this.effectAllowed(tx,actor,req,o,r))};
  }
  if(req.goldIntent){const r=await this.access(tx,actor,req),p=req.goldIntent;demand(uuid(p.operationId)&&['deposit','withdraw'].includes(p.direction??'')&&Number.isInteger(p.amount)&&p.amount!>=1&&p.amount!<=GOLD_LIMIT,'INVALID_MESSAGE');
   const old=(await tx.query('SELECT o.*,b.amount AS balance,b.version AS balance_version FROM g3_operations o JOIN g3_balances b ON b.run_id=o.run_id WHERE o.run_id=$1 AND o.principal_id=$2 AND o.operation_id=$3',[req.scope!.runId,actor.accountId,p.operationId])).rows[0];if(old){demand(old.digest===operationDigest(actor.accountId,req.scope!,p.direction!,p.amount!),'CONFLICT');return {operation:this.view(old,Number(old.balance),old.balance_version,r.room_id,await this.effectAllowed(tx,actor,req,old,r))};}
  }
  if(req.goldReceipt)return this.receipt(tx,actor,req);
  const {r,c}=await this.context(tx,actor,req);
  if(req.goldBank)return this.bank(tx,actor,req,r,c);
  if(req.movement||req.ghostList){demand(c.capabilities.includes('core.ghost'),'FORBIDDEN');await this.cleanGhosts(tx);if(req.movement){const frames=req.movement.frames??[];demand(frames.length>=1&&frames.length<=4&&c.capabilities.includes('core.ghost'),'INVALID_MESSAGE');const previous=(await tx.query('SELECT * FROM g3_movement_fences WHERE attachment_id=$1 FOR UPDATE',[c.attachment_id])).rows[0];let seq=Number(previous?.last_sequence??0);for(const f of frames){demand(Number.isInteger(f.sequence)&&f.sequence!>seq&&f.sequence!<=0xffffffff&&typeof (f.facingLeft??false)==='boolean','INVALID_MESSAGE');for(const v of [f.xMilli,f.yMilli,f.vxMilli,f.vyMilli])demand(typeof v==='string'&&/^(0|-?[1-9][0-9]{0,9})$/.test(v)&&Math.abs(Number(v))<=2147483647,'INVALID_MESSAGE');seq=f.sequence!;}
    const window=previous&&+previous.window_at+1000>this.now()?+previous.window_at:this.now(),count=previous&&+previous.window_at===window?previous.window_count+1:1;demand(count<=10&&(this.ghosts.has(c.attachment_id)||this.ghosts.size<1024),'CAPACITY');await tx.query('INSERT INTO g3_movement_fences VALUES($1,$2,$3,$4) ON CONFLICT(attachment_id) DO UPDATE SET last_sequence=EXCLUDED.last_sequence,window_at=EXCLUDED.window_at,window_count=EXCLUDED.window_count',[c.attachment_id,seq,new Date(window),count]);const acceptedAt=this.now(),snapshot=frames.map(f=>({...f,facingLeft:f.facingLeft??false}));
    // These are service acceptance times, never per-frame source capture times.
    // Delayed/out-of-order commit callbacks must not revive old display samples.
    afterCommit.push(()=>{const latest=this.ghosts.get(c.attachment_id);if(acceptedAt+2000<=this.now()||latest&&latest.frames.at(-1)!.sequence!>=seq)return;this.ghosts.set(c.attachment_id,{principalId:actor.accountId,gameSessionId:actor.sessionId,worldToken:c.world_token,frames:snapshot,capturedAt:new Date(acceptedAt).toISOString(),until:acceptedAt+2000});});return {};}
   const ids=(await tx.query('SELECT attachment_id FROM g3_contexts WHERE run_id=$1 AND revoked_at IS NULL AND service_epoch=$2 AND presence_until>$3 LIMIT 32',[r.id,this.serviceEpoch(),new Date(this.now())])).rows;return {ghosts:ids.map(v=>this.ghosts.get(v.attachment_id)).filter(v=>v&&v.until>this.now()&&v.gameSessionId!==actor.sessionId).map(v=>({principalId:v!.principalId,gameSessionId:v!.gameSessionId,worldToken:v!.worldToken,frames:v!.frames,capturedAt:v!.capturedAt}))};
  }
  if(req.gameEvent){const p=req.gameEvent,entry=G3_EVENT_CATALOG.find(v=>v.id===p.catalogId);demand(entry&&entry.mode===r.mode&&c.capabilities.includes(entry.capability)&&p.count===1&&(!p.targetGameSessionId||p.targetGameSessionId===actor.sessionId)&&uuid(p.eventId),'FORBIDDEN');const digest=fingerprint(p),old=(await tx.query('SELECT digest FROM g3_events WHERE run_id=$1 AND principal_id=$2 AND event_id=$3',[r.id,actor.accountId,p.eventId])).rows[0];if(old){demand(old.digest===digest,'CONFLICT');return {};}
   demand(+(await tx.query('SELECT count(*) FROM g3_events WHERE run_id=$1',[r.id])).rows[0].count<4096,'CAPACITY');await tx.query('INSERT INTO g3_events VALUES($1,$2,$3,$4)',[r.id,actor.accountId,p.eventId,digest]);if(entry.id==='player.death')await this.onDeath?.(tx,actor,req.scope!);await this.statistic(tx,r,actor,entry.id,1,'validated-client-report');const event={eventId:p.eventId,catalogId:entry.id,principalId:actor.accountId,gameSessionId:actor.sessionId,worldToken:c.world_token,targetGameSessionId:actor.sessionId,count:1,provenance:'validated-client-report'};await tx.query("INSERT INTO g3_outbox(run_id,principal_id,kind,body,created_at) SELECT run_id,principal_id,'game.event',$2,$3 FROM g3_access WHERE run_id=$1",[r.id,event,new Date(this.now())]);return {};
  }
  demand(c.capabilities.includes(GOLD_CAPABILITY)&&await this.goldEnabled(tx,r),'FORBIDDEN');
  const p=req.goldIntent;demand(p&&uuid(p.operationId),'INVALID_MESSAGE');const amountRequested=p.amount!;
  const balance=await this.balance(tx,r.id);let amount=Number(balance.amount),version=String(balance.version);
  // CAPACITY is temporary admission; BANK_FULL is a durable ledger bound.
  // Existing originals were recovered above, before these new-intent checks.
  demand(BigInt(version)<9223372036854775807n,'BANK_FULL');
  demand(+(await tx.query('SELECT count(*) FROM g3_operations WHERE run_id=$1 AND principal_id=$2',[r.id,actor.accountId])).rows[0].count<1000,'BANK_FULL');
  demand(+(await tx.query("SELECT count(*) FROM g3_operations WHERE run_id=$1 AND principal_id=$2 AND state NOT IN ('completed','cancelled')",[r.id,actor.accountId])).rows[0].count<256,'BANK_FULL');
  if(p.direction==='withdraw'){demand(amount>=amountRequested,'INSUFFICIENT_FUNDS');amount-=amountRequested;version=String((await this.changeBalance(tx,r.id,amount)).version);}
  else{const held=(await tx.query("SELECT coalesce(sum(amount),0)::text AS amount FROM g3_operations WHERE run_id=$1 AND direction='deposit' AND state NOT IN ('completed','cancelled')",[r.id])).rows[0].amount;demand(BigInt(amount)+BigInt(held)+BigInt(amountRequested)<=BigInt(BALANCE_LIMIT),'BANK_FULL');}
  const o=(await tx.query('INSERT INTO g3_operations(run_id,principal_id,operation_id,attachment_id,world_token,direction,amount,digest,delivery_id,state,created_at,run_epoch,config_revision) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13) RETURNING *',[r.id,actor.accountId,p.operationId,c.attachment_id,c.world_token,p.direction,amountRequested,operationDigest(actor.accountId,req.scope!,p.direction!,amountRequested),randomUUID(),p.direction==='deposit'?'awaiting_local':'reserved',new Date(this.now()),r.epoch,r.config_revision])).rows[0];
  if(p.direction==='withdraw')await this.ledger(tx,o,-amountRequested,amount);const result=this.view(o,amount,version,r.room_id,true);await this.outbox(tx,result,'gold.intent');return {operation:result};
 }
 private async receipt(tx:Tx,actor:Actor,req:Request):Promise<Partial<Response>>{
  const r=await this.access(tx,actor,req),scope=req.scope!,p=req.goldReceipt!,id=p.operationId;demand(uuid(id),'INVALID_MESSAGE');
  const o=(await tx.query('SELECT * FROM g3_operations WHERE run_id=$1 AND principal_id=$2 AND operation_id=$3 FOR UPDATE',[r.id,actor.accountId,id])).rows[0];
  demand(o&&o.attachment_id===scope.attachmentId&&o.world_token===scope.worldToken&&o.run_epoch===scope.runEpoch&&o.config_revision===scope.configRevision,'NOT_FOUND');
  demand(uuid(p.receiptId)&&p.deliveryId===o.delivery_id&&p.payloadDigest===o.digest&&/^[a-f0-9]{64}$/.test(p.evidenceDigest??''),'INVALID_MESSAGE');
  const digest=fingerprint(p),old=(await tx.query('SELECT fingerprint FROM g3_receipts WHERE run_id=$1 AND principal_id=$2 AND operation_id=$3 AND receipt_id=$4',[r.id,actor.accountId,id,p.receiptId])).rows[0];
  const balance=await this.balance(tx,r.id);let amount=Number(balance.amount),version=String(balance.version);
  if(old){demand(old.fingerprint===digest,'CONFLICT');return {operation:this.view(o,amount,version,r.room_id)};}
  demand(!['completed','cancelled'].includes(o.state),'INVALID_STATE');
  if(actor.kind==='game'){
   const current=req.currentScope??scope;demand(current.roomId===r.room_id&&current.runId===o.run_id&&current.runEpoch===o.run_epoch&&current.configRevision===o.config_revision&&current.worldToken===o.world_token,'FORBIDDEN');
   const {c}=await this.context(tx,actor,{...req,scope:current},r);demand(c.capabilities.includes(GOLD_CAPABILITY)&&await this.goldEnabled(tx,r),'FORBIDDEN');
  }else demand(actor.kind==='desktop'&&p.outcome==='uncertain','FORBIDDEN');
  // A receipt refusal never retires its accepted original or local evidence.
  demand(+(await tx.query('SELECT count(*) FROM g3_receipts WHERE run_id=$1 AND principal_id=$2 AND operation_id=$3',[r.id,actor.accountId,id])).rows[0].count<8,'BANK_FULL');
  const next=receiptTransition(o.direction,o.state,p.outcome??'');demand(next,'INVALID_STATE');
  if(next==='completed'&&o.direction==='deposit'){demand(amount+o.amount<=BALANCE_LIMIT,'BANK_FULL');amount+=o.amount;version=String((await this.changeBalance(tx,r.id,amount)).version);await this.ledger(tx,o,o.amount,amount);}
  await tx.query('INSERT INTO g3_receipts VALUES($1,$2,$3,$4,$5,$6,$7,$8)',[r.id,actor.accountId,id,p.receiptId,digest,p.evidenceDigest,p.outcome,new Date(this.now())]);
  await tx.query('UPDATE g3_operations SET state=$4,local_outcome=$5 WHERE run_id=$1 AND principal_id=$2 AND operation_id=$3',[r.id,actor.accountId,id,next,p.outcome]);o.state=next;o.local_outcome=p.outcome;
  const result=this.view(o,amount,version,r.room_id);await this.outbox(tx,result,'gold.receipt');
  if(next==='completed'){await this.statistic(tx,r,actor,'gold.'+o.direction,o.amount,'service-committed');await addStatistic(tx,r.id,actor.accountId,o.direction==='deposit'?'gold_deposited':'gold_withdrawn',o.amount);}
  if(next==='completed'&&o.direction==='deposit'){
   const notice={principalId:actor.accountId,displayName:r.display_name,amount:o.amount,balance:amount,balanceVersion:version,acceptedAt:new Date(this.now()).toISOString()};
   await tx.query("INSERT INTO g3_outbox(run_id,principal_id,kind,body,created_at) SELECT DISTINCT run_id,principal_id,'gold.deposit',$2::jsonb,$3::timestamptz FROM g3_contexts WHERE run_id=$1 AND revoked_at IS NULL AND presence_until>$3 AND 'core.gold'=ANY(capabilities)",[r.id,notice,new Date(this.now())]);
  }
  return {operation:result};
 }

 private async cleanGhosts(tx:Tx){
  for(const [id,pose] of this.ghosts)if(pose.until<=this.now())this.ghosts.delete(id);
  if(!this.ghosts.size)return;
  const live=(await tx.query("SELECT c.attachment_id FROM g3_contexts c JOIN g3_runs r ON r.id=c.run_id JOIN sessions s ON s.id=c.game_session_id JOIN sessions d ON d.id=s.parent_id JOIN session_families f ON f.id=s.family_id WHERE c.attachment_id=ANY($1::uuid[]) AND c.revoked_at IS NULL AND r.state='running' AND c.service_epoch=$2 AND c.presence_until>$3 AND s.revoked_at IS NULL AND d.revoked_at IS NULL AND f.revoked_at IS NULL AND f.expires_at>$3",[[...this.ghosts.keys()],this.serviceEpoch(),new Date(this.now())])).rows;
  const ids=new Set(live.map(v=>v.attachment_id));for(const id of this.ghosts.keys())if(!ids.has(id))this.ghosts.delete(id);
 }
 private async ledger(tx:Tx,o:Record<string,any>,delta:number,balance:number){await tx.query('INSERT INTO g3_ledger(run_id,principal_id,operation_id,delta,balance,created_at) VALUES($1,$2,$3,$4,$5,$6)',[o.run_id,o.principal_id,o.operation_id,delta,balance,new Date(this.now())]);}
 private limit(n:number|undefined){demand(n===undefined||Number.isInteger(n)&&n>=1&&n<=32,'INVALID_MESSAGE');return n??32;}
 private cursor(s:string|undefined){const v=s||'0';demand(/^(0|[1-9][0-9]{0,18})$/.test(v)&&BigInt(v)<=9223372036854775807n,'INVALID_MESSAGE');return v;}
 async retainStatistics(tx:Tx){return tx.query('DELETE FROM g3_statistics WHERE sequence IN (SELECT sequence FROM g3_statistics WHERE expires_at<=$1 ORDER BY expires_at,sequence LIMIT 1000)',[new Date(this.now())]);}
 async maintain(tx:Tx){
  await this.cleanGhosts(tx);
  // Expired presence can be renewed in the same world: keep its fence. Only
  // irreversible revocation/terminal runs permit deletion of historical fences.
  await tx.query("DELETE FROM g3_movement_fences WHERE attachment_id IN (SELECT c.attachment_id FROM g3_contexts c JOIN g3_runs r ON r.id=c.run_id WHERE c.revoked_at IS NOT NULL OR r.state='ended' ORDER BY c.attachment_id LIMIT 1000)");
  return this.retainStatistics(tx);
 }
}

