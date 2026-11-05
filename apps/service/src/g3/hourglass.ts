import type {Actor} from '../g1/auth.js';import type {Tx} from '../g1/database.js';
import {demand,fingerprint,PolicyError} from '../g1/security.js';
import {HourglassLedger,HourglassError,HOURGLASS_CATALOGUE,type HourglassActor,type HourglassRecord} from '../../../../packages/domain/src/g3-hourglass.js';
import type {Request,Response,Scope,Delivery} from '../../../../packages/contracts/g3-hourglass-v2/g3-hourglass-v2.js';
import {hourglassRoster} from './hourglass-context.js';
import {loadHourglass,storeHourglass} from './hourglass-storage.js';
const uuid=(v:unknown):v is string=>typeof v==='string'&&/^[a-f0-9]{8}-[a-f0-9]{4}-[1-8][a-f0-9]{3}-[89ab][a-f0-9]{3}-[a-f0-9]{12}$/.test(v);
const scopeOf=(a:HourglassActor):Scope=>({roomId:a.roomId,runId:a.runId,runEpoch:a.runEpoch,configRevision:a.configRevision,attachmentId:a.attachmentId,worldToken:a.worldToken});
const activation=(r:HourglassRecord)=>({activationId:r.plan.activationId,effectId:r.plan.effectId,materialId:r.plan.materialId,recipientCount:r.deliveries.length,provenance:r.plan.provenance});
function wire(record:HourglassRecord,delivery:HourglassRecord['deliveries'][number],sequence:number):Delivery {return {sequence,deliveryId:delivery.deliveryId,activationId:record.plan.activationId,effectId:record.plan.effectId,materialId:record.plan.materialId,catalogueRevision:record.plan.catalogueRevision,source:record.plan.source,scope:{...record.plan.scope,attachmentId:delivery.recipient.attachmentId,worldToken:delivery.recipient.worldToken},recipientGameSessionId:delivery.recipient.gameSessionId,recipientAttachmentId:delivery.recipient.attachmentId,recipientWorldToken:delivery.recipient.worldToken,outcome:delivery.outcome,provenance:record.plan.provenance};}
export class HourglassAuthority {
 constructor(readonly epoch:()=>string,readonly capabilities:readonly string[],readonly now:()=>number=Date.now){}
 async maintain(_tx:Tx){} // Retain all original identities/uncertainty; no tombstone expiry/remint.
 private async budget(tx:Tx,a:HourglassActor){
  const old=(await tx.query('SELECT * FROM g3_hourglass_fences WHERE run_id=$1 AND session_id=$2',[a.runId,a.gameSessionId])).rows[0],now=this.now();
  const window=old&&+old.window_at+1000>now?+old.window_at:now,count=old&&window===+old.window_at?old.count+1:1;demand(count<=10&&window<=now,'CAPACITY');
  await tx.query('INSERT INTO g3_hourglass_fences VALUES($1,$2,$3,$4) ON CONFLICT(run_id,session_id) DO UPDATE SET window_at=EXCLUDED.window_at,count=EXCLUDED.count',[a.runId,a.gameSessionId,new Date(window),count]);
 }
 async execute(tx:Tx,actor:Actor,input:Request):Promise<Partial<Response>>{
  try{return await this.apply(tx,actor,input);}catch(e){if(e instanceof HourglassError)throw new PolicyError(e.code);throw e;}
 }
 private async apply(tx:Tx,actor:Actor,input:Request):Promise<Partial<Response>>{
  const scope=input.scope;demand(scope&&[scope.runId,scope.roomId,scope.runEpoch,scope.attachmentId].every(uuid)&&typeof scope.worldToken==='string'&&scope.worldToken.length>0&&Number.isInteger(scope.configRevision),'INVALID_MESSAGE');
  demand(input.catalogueRevision===HOURGLASS_CATALOGUE.revision,'FORBIDDEN');
  // Durable historical membership and frozen scope do not depend on a current room.
  const access=(await tx.query('SELECT g.* FROM g3_runs g JOIN g3_access a ON a.run_id=g.id WHERE g.id=$1 AND a.principal_id=$2',[scope.runId,actor.accountId])).rows[0];
  demand(access&&access.room_id===scope.roomId&&access.epoch===scope.runEpoch&&access.config_revision===scope.configRevision,'FORBIDDEN');
  const readOne=async(id:string)=>(await tx.query('SELECT * FROM g3_hourglass_activations WHERE run_id=$1 AND activation_id=$2 FOR UPDATE',[scope.runId,id])).rows[0];
  if(input.status){
   demand(uuid(input.status.activationId),'INVALID_MESSAGE');const row=await readOne(input.status!.activationId!);demand(row&&!row.c2_body,'NOT_FOUND');const original=loadHourglass(row.record),ledger=new HourglassLedger({records:[original]});
   const source=original.plan.source.principalId===actor.accountId?original.plan.source:original.plan.recipients.find(r=>r.principalId===actor.accountId);demand(source&&source.attachmentId===scope.attachmentId&&source.worldToken===scope.worldToken,'NOT_FOUND');
   const historical:HourglassActor={...original.plan.scope,...source,name:original.plan.source.name,mode:'core',live:false,capabilities:[],catalogueRevision:original.plan.catalogueRevision};
   const record=ledger.status(historical,input.status.activationId!);
   const after=input.status.afterRecipientKey??'',limit=input.status.limit??8;
   demand(Number.isInteger(limit)&&limit>=1&&limit<=8,'INVALID_MESSAGE');
   const visible=[...record.deliveries].sort((a,b)=>a.recipient.gameSessionId.localeCompare(b.recipient.gameSessionId));
   demand(after===''||uuid(after)&&visible.some(d=>d.recipient.gameSessionId===after),'INVALID_MESSAGE');
   const page=visible.filter(d=>d.recipient.gameSessionId>after).slice(0,limit);
   return {activation:activation(record),original:{activationId:record.plan.activationId,digest:record.digest,effectId:record.plan.effectId,materialId:record.plan.materialId,catalogueRevision:record.plan.catalogueRevision,source:record.plan.source,scope:{...record.plan.scope,attachmentId:record.plan.source.attachmentId,worldToken:record.plan.source.worldToken},recipientCount:record.deliveries.length,provenance:record.plan.provenance},recipientStatuses:page.map(d=>({deliveryId:d.deliveryId,gameSessionId:d.recipient.gameSessionId,attachmentId:d.recipient.attachmentId,worldToken:d.recipient.worldToken,outcome:d.outcome,...(d.receiptId?{receiptId:d.receiptId}:{})})),nextRecipientKey:page.at(-1)?.recipient.gameSessionId??after,highRecipientKey:visible.at(-1)?.recipient.gameSessionId??''};
  }
  demand(actor.kind==='game','FORBIDDEN');const roster=await hourglassRoster(tx,scope.runId!,this.epoch(),this.capabilities,HOURGLASS_CATALOGUE.revision,this.now());
  const sender=roster.find(a=>a.principalId===actor.accountId&&a.gameSessionId===actor.sessionId&&a.sessionEpoch===actor.sessionEpoch);demand(sender&&fingerprint(scopeOf(sender))===fingerprint(scope),'STALE_EPOCH');
  if(input.activate){
   demand(uuid(input.activate.activationId),'INVALID_MESSAGE');await tx.query('SELECT id FROM g3_runs WHERE id=$1 FOR UPDATE',[scope.runId]);const row=await readOne(input.activate.activationId!);demand(!row?.c2_body,'CONFLICT');const state={records:row?[loadHourglass(row.record)]:[]},ledger=new HourglassLedger(state),record=ledger.activate(sender,input.activate,roster);
   if(!row){await this.budget(tx,sender);const sequence=Number((await tx.query('SELECT coalesce(max(sequence),0)+1 AS n FROM g3_hourglass_activations WHERE run_id=$1',[scope.runId])).rows[0].n);demand(sequence<=0xffffffff,'CAPACITY');await tx.query('INSERT INTO g3_hourglass_activations(run_id,activation_id,sequence,record) VALUES($1,$2,$3,$4)',[sender.runId,record.plan.activationId,sequence,storeHourglass(record)]);}
   return {activation:activation(record)};
  }
  if(input.pending){
   const limit=input.pending.limit??8,high=Number((await tx.query('SELECT coalesce(max(sequence),0) AS n FROM g3_hourglass_activations WHERE run_id=$1',[scope.runId])).rows[0].n),after=input.pending.afterSequence??0,through=input.pending.throughSequence??high;
   demand(Number.isInteger(limit)&&limit>=1&&limit<=8&&after<=through&&through<=high,'INVALID_MESSAGE');await this.budget(tx,sender);
   const rows=(await tx.query('SELECT * FROM g3_hourglass_activations WHERE run_id=$1 AND sequence>$2 AND sequence<=$3 AND c2_body IS NULL AND record @> $4::jsonb ORDER BY sequence LIMIT $5',[scope.runId,after,through,JSON.stringify({deliveries:[{outcome:'pending',recipient:{gameSessionId:sender.gameSessionId}}]}),limit])).rows,deliveries=[];
   for(const row of rows){const ledger=new HourglassLedger({records:[loadHourglass(row.record)]});for(const d of ledger.pending(sender,0,1))deliveries.push(wire({plan:d.plan,deliveries:[d.delivery],digest:''},d.delivery,Number(row.sequence)));}
   return {deliveries,nextSequence:rows.length===limit?Number(rows.at(-1)!.sequence):through,highSequence:through};
  }
  const receipt=input.receipt;demand(receipt,'INVALID_MESSAGE');
  const row=(await tx.query('SELECT * FROM g3_hourglass_activations WHERE run_id=$1 AND c2_body IS NULL AND record @> $2::jsonb LIMIT 1 FOR UPDATE',[scope.runId,JSON.stringify({deliveries:[{deliveryId:receipt.deliveryId}]})])).rows[0];demand(row,'NOT_FOUND');const state={records:[loadHourglass(row.record)]},ledger=new HourglassLedger(state);
  ledger.report(sender,receipt.deliveryId!,receipt.receiptId!,receipt.outcome as 'unknown'|'applied_reported'|'not_applied_reported');
  const record=state.records.find(r=>r.deliveries.some(d=>d.deliveryId===receipt.deliveryId))!,old=row;
  if(fingerprint(record)!==fingerprint(loadHourglass(old.record))){await this.budget(tx,sender);await tx.query('UPDATE g3_hourglass_activations SET record=$3 WHERE run_id=$1 AND activation_id=$2',[sender.runId,record.plan.activationId,storeHourglass(record,!!old.record.worldTokenEncoding)]);}
  return {activation:activation(record),deliveries:record.deliveries.filter(d=>d.recipient.principalId===actor.accountId).map(d=>wire(record,d,Number(old.sequence)))};
 }
}
