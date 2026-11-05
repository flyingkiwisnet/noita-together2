// Isolated provisional domain. Context/state MUST be service-derived and committed
// transactionally by a future service adapter; this module exposes no HTTP authority.
import {createRequire} from 'node:module';
import {createHash} from 'node:crypto';
const require=createRequire(import.meta.url);
export const HOURGLASS_CATALOGUE=require('../../contracts/g3-hourglass/catalogue.json') as {
 revision:string;capability:string;entries:readonly HourglassEntry[];
};
export interface HourglassEntry {id:string;materialId:string;localHandlerId:string;durationFrames:number|null;effect:string}
export class HourglassError extends Error {constructor(readonly code:'INVALID_MESSAGE'|'FORBIDDEN'|'CONFLICT'|'CAPACITY'|'NOT_FOUND'){super(code);}}
function demand(value:unknown,code:HourglassError['code']='INVALID_MESSAGE'):asserts value {if(!value)throw new HourglassError(code);}
const uuid=(v:unknown):v is string=>typeof v==='string'&&/^[0-9a-f]{8}-[0-9a-f]{4}-[1-8][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/.test(v);
const id=(v:unknown):v is string=>typeof v==='string'&&/^[a-z][a-z0-9_.:-]{0,63}$/.test(v);
const text=(v:unknown):v is string=>typeof v==='string'&&Buffer.byteLength(v)>0&&Buffer.byteLength(v)<=80&&!/[\u0000-\u001f\u007f-\u009f\ud800-\udfff]/u.test(v);
export interface HourglassActor {
 principalId:string;gameSessionId:string;attachmentId:string;worldToken:string;name:string;
 roomId:string;runId:string;runEpoch:string;configRevision:number;mode:string;live:boolean;
 capabilities:readonly string[];catalogueRevision:string;
}
export type HourglassIntent={activationId:string;effectId:string;materialId:string};
export interface HourglassManifest {revision:string;entries:readonly HourglassEntry[];replace:boolean;disabledIds:readonly string[]}
// Local trusted registration only. The owning service reviews/pins each manifest and
// its installed handler catalogue before negotiation. Never deserialize this from RPC.
export class HourglassRegistry {
 private entries=new Map(HOURGLASS_CATALOGUE.entries.map(e=>[e.id,structuredClone(e)]));
 revision=HOURGLASS_CATALOGUE.revision;
 get(effectId:string){const entry=this.entries.get(effectId);return entry?structuredClone(entry):undefined;}
 registerTrusted(manifest:HourglassManifest){
  demand(id(manifest.revision)&&manifest.revision!==this.revision&&typeof manifest.replace==='boolean'&&Array.isArray(manifest.entries)&&Array.isArray(manifest.disabledIds)&&manifest.entries.length+manifest.disabledIds.length<=32);
  const next=new Map(this.entries),seen=new Set<string>();
  for(const entry of manifest.entries){
   demand(Object.keys(entry).sort().join(',')==='durationFrames,effect,id,localHandlerId,materialId'&&id(entry.id)&&id(entry.materialId)&&id(entry.localHandlerId)&&text(entry.effect)&& (entry.durationFrames===null||Number.isInteger(entry.durationFrames)&&entry.durationFrames>=1&&entry.durationFrames<=7200)&&!seen.has(entry.id));
   demand(manifest.replace||!next.has(entry.id),'CONFLICT');seen.add(entry.id);next.set(entry.id,structuredClone(entry));
  }
  for(const effectId of manifest.disabledIds){demand(id(effectId)&&!seen.has(effectId));demand(manifest.replace&&next.has(effectId),'CONFLICT');seen.add(effectId);next.delete(effectId);}
  demand(next.size<=64,'CAPACITY');this.entries=next;this.revision=manifest.revision;
 }
}
function identity(actor:HourglassActor){
 demand([actor.principalId,actor.gameSessionId,actor.attachmentId,actor.roomId,actor.runId,actor.runEpoch].every(uuid)&&typeof actor.worldToken==='string'&&Buffer.byteLength(actor.worldToken)>0&&Buffer.byteLength(actor.worldToken)<=128&&text(actor.name)&&Number.isInteger(actor.configRevision)&&actor.configRevision>=1&&actor.configRevision<=0xffffffff,'FORBIDDEN');
}
function active(actor:HourglassActor,registry:HourglassRegistry){identity(actor);demand(actor.live&&actor.mode==='core'&&actor.capabilities.includes('core.hourglass')&&actor.catalogueRevision===registry.revision,'FORBIDDEN');}
function intent(value:unknown):HourglassIntent {
 demand(value&&typeof value==='object'&&!Array.isArray(value));const p=value as Record<string,unknown>;
 demand(Object.keys(p).sort().join(',')==='activationId,effectId,materialId'&&uuid(p.activationId)&&id(p.effectId)&&id(p.materialId));return p as HourglassIntent;
}
function sameRun(a:HourglassActor,b:HourglassActor){return a.roomId===b.roomId&&a.runId===b.runId&&a.runEpoch===b.runEpoch&&a.configRevision===b.configRevision;}
export function planHourglass(sender:HourglassActor,value:unknown,roster:readonly HourglassActor[],registry=new HourglassRegistry()){
 active(sender,registry);const p=intent(value),entry=registry.get(p.effectId);demand(entry&&entry.materialId===p.materialId,'FORBIDDEN');
 demand(roster.length<=32,'CAPACITY');const sessions=new Set<string>();
 const recipients=roster.filter(r=>{
  if(!sameRun(sender,r)||r.principalId===sender.principalId||!r.live||r.mode!=='core'||!r.capabilities.includes('core.hourglass')||r.catalogueRevision!==registry.revision)return false;
  identity(r);demand(!sessions.has(r.gameSessionId),'CONFLICT');sessions.add(r.gameSessionId);return true;
 });
 return {activationId:p.activationId,effectId:entry.id,materialId:entry.materialId,catalogueRevision:registry.revision,source:{principalId:sender.principalId,name:sender.name,gameSessionId:sender.gameSessionId,attachmentId:sender.attachmentId,worldToken:sender.worldToken},scope:{roomId:sender.roomId,runId:sender.runId,runEpoch:sender.runEpoch,configRevision:sender.configRevision},provenance:'validated-client-report' as const,recipients:recipients.map(r=>({principalId:r.principalId,gameSessionId:r.gameSessionId,attachmentId:r.attachmentId,worldToken:r.worldToken}))};
}
type Plan=ReturnType<typeof planHourglass>;
export type HourglassDelivery={deliveryId:string;recipient:Plan['recipients'][number];outcome:'pending'|'applied_reported'|'not_applied_reported'|'unknown';receiptId?:string};
export type HourglassRecord={digest:string;plan:Plan;deliveries:HourglassDelivery[]};
export interface HourglassState {records:HourglassRecord[]}
// Bounded serializable transaction seam. Caller-owned state is NOT a persisted DB.
// Original activations/recipient IDs are retained; capacity never evicts uncertainty.
export class HourglassLedger {
 constructor(private readonly state:HourglassState,private readonly registry=new HourglassRegistry()){}
 activate(sender:HourglassActor,value:unknown,roster:readonly HourglassActor[]){
  active(sender,this.registry);const p=intent(value);
  // Status wire identifies one original by run + activationId. Never permit a
  // second source/context to create an ambiguous original with that same key.
  const prior=this.state.records.find(r=>r.plan.activationId===p.activationId&&r.plan.scope.runId===sender.runId);
  const digest=createHash('sha256').update(JSON.stringify([p.activationId,p.effectId,p.materialId])).digest('hex');
  if(prior){demand(prior.digest===digest&&sameRun(sender,{...sender,...prior.plan.scope})&&prior.plan.source.principalId===sender.principalId&&prior.plan.source.gameSessionId===sender.gameSessionId&&prior.plan.source.attachmentId===sender.attachmentId&&prior.plan.source.worldToken===sender.worldToken,'CONFLICT');return structuredClone(prior);}
  demand(this.state.records.length<256,'CAPACITY');const plan=planHourglass(sender,p,roster,this.registry);
  const deliveries=plan.recipients.map(recipient=>({deliveryId:createHash('sha256').update(JSON.stringify([plan.scope,plan.source.principalId,plan.source.worldToken,p.activationId,recipient.gameSessionId,recipient.worldToken])).digest('hex'),recipient,outcome:'pending' as const}));
  const record={digest,plan,deliveries};this.state.records.push(record);return structuredClone(record);
 }
 status(actor:HourglassActor,activationId:string){
  identity(actor);demand(uuid(activationId));
  const record=this.state.records.find(r=>r.plan.activationId===activationId&&sameRun(actor,{...actor,...r.plan.scope})&&(r.plan.source.principalId===actor.principalId||r.deliveries.some(d=>d.recipient.principalId===actor.principalId)));
  demand(record,'NOT_FOUND');const copy=structuredClone(record);
  if(copy.plan.source.principalId!==actor.principalId){copy.deliveries=copy.deliveries.filter(d=>d.recipient.principalId===actor.principalId);copy.plan.recipients=copy.plan.recipients.filter(r=>r.principalId===actor.principalId);}
  return copy;
 }
 pending(actor:HourglassActor,afterSequence=0,throughSequence=this.state.records.length){
  active(actor,this.registry);
  demand(Number.isInteger(afterSequence)&&Number.isInteger(throughSequence)&&afterSequence>=0&&afterSequence<=throughSequence&&throughSequence<=this.state.records.length);
  return this.state.records.flatMap((record,index)=>index+1>afterSequence&&index+1<=throughSequence&&record.plan.catalogueRevision===actor.catalogueRevision&&sameRun(actor,{...actor,...record.plan.scope})?record.deliveries.filter(d=>d.outcome==='pending'&&d.recipient.principalId===actor.principalId&&d.recipient.gameSessionId===actor.gameSessionId&&d.recipient.attachmentId===actor.attachmentId&&d.recipient.worldToken===actor.worldToken).map(d=>({sequence:index+1,plan:{...structuredClone(record.plan),recipients:[structuredClone(d.recipient)]},delivery:structuredClone(d)})):[]).slice(0,32);
 }
 pendingPage(actor:HourglassActor,afterSequence=0,throughSequence=this.state.records.length,limit=32){
  demand(Number.isInteger(limit)&&limit>=1&&limit<=32);
  const deliveries=this.pending(actor,afterSequence,throughSequence).slice(0,limit);
  return {deliveries,nextSequence:deliveries.length===limit?deliveries.at(-1)!.sequence:throughSequence,highSequence:throughSequence};
 }
 report(actor:HourglassActor,deliveryId:string,receiptId:string,outcome:Exclude<HourglassDelivery['outcome'],'pending'>){
  active(actor,this.registry);demand(/^[0-9a-f]{64}$/.test(deliveryId)&&uuid(receiptId)&&['applied_reported','not_applied_reported','unknown'].includes(outcome));
  const record=this.state.records.find(r=>sameRun(actor,{...actor,...r.plan.scope})&&r.deliveries.some(d=>d.deliveryId===deliveryId));const delivery=record?.deliveries.find(d=>d.deliveryId===deliveryId);
  demand(record?.plan.catalogueRevision===actor.catalogueRevision&&delivery&&delivery.recipient.principalId===actor.principalId&&delivery.recipient.gameSessionId===actor.gameSessionId&&delivery.recipient.attachmentId===actor.attachmentId&&delivery.recipient.worldToken===actor.worldToken,'NOT_FOUND');
  if(delivery.receiptId){demand(delivery.receiptId===receiptId&&delivery.outcome===outcome,'CONFLICT');return structuredClone(delivery);}
  delivery.receiptId=receiptId;delivery.outcome=outcome;return structuredClone(delivery);
 }
}
