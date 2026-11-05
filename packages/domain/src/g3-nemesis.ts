// Pure caller-owned transaction seam. Context, offers and NP balance MUST come
// from the service transaction; this module has no RPC or earning grant.
import {createRequire} from 'node:module';import {createHash} from 'node:crypto';
const require=createRequire(import.meta.url);
export const NEMESIS_CATALOGUE=require('../../contracts/g3-nemesis/catalogue-v2.json') as {revision:string;capability:string;entries:readonly {id:string;depthTierWeights:readonly number[];durationFrames:number|null;domainPurchaseImplemented:boolean;localHandlerId:string;effectKind:'timed'|'one-shot'|'game-effect'}[]};
export class NemesisError extends Error {constructor(readonly code:'INVALID_MESSAGE'|'FORBIDDEN'|'CONFLICT'|'CAPACITY'|'NOT_FOUND'|'INSUFFICIENT_POINTS'){super(code);}}
function demand(value:unknown,code:NemesisError['code']='INVALID_MESSAGE'):asserts value{if(!value)throw new NemesisError(code);}
const uuid=(value:unknown):value is string=>typeof value==='string'&&/^[0-9a-f]{8}-[0-9a-f]{4}-[1-8][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/.test(value);
const hash=(value:unknown)=>createHash('sha256').update(JSON.stringify(value,(_key,v)=>v&&typeof v==='object'&&!Array.isArray(v)?Object.fromEntries(Object.keys(v).sort().map(k=>[k,v[k]])):v)).digest('hex');
export interface NemesisActor {principalId:string;gameSessionId:string;attachmentId:string;worldToken:string;name:string;roomId:string;runId:string;runEpoch:string;configRevision:number;mode:string;live:boolean;abilitiesEnabled:boolean;capabilities:readonly string[];catalogueRevision:string}
type Context=Pick<NemesisActor,'principalId'|'gameSessionId'|'attachmentId'|'worldToken'|'roomId'|'runId'|'runEpoch'|'configRevision'>;
function context(a:Context):Context{return {principalId:a.principalId,gameSessionId:a.gameSessionId,attachmentId:a.attachmentId,worldToken:a.worldToken,roomId:a.roomId,runId:a.runId,runEpoch:a.runEpoch,configRevision:a.configRevision};}
export {context as nemesisContext};
function identity(a:NemesisActor){demand([a.principalId,a.gameSessionId,a.attachmentId,a.roomId,a.runId,a.runEpoch].every(uuid)&&typeof a.worldToken==='string'&&Buffer.byteLength(a.worldToken)>0&&Buffer.byteLength(a.worldToken)<=80&&typeof a.name==='string'&&Buffer.byteLength(a.name)>0&&Buffer.byteLength(a.name)<=80&&!/[\u0000-\u001f\u007f-\u009f\ud800-\udfff]/u.test(a.name)&&Number.isInteger(a.configRevision)&&a.configRevision>=1&&a.configRevision<=0xffffffff,'FORBIDDEN');}
function active(a:NemesisActor){identity(a);demand(a.live&&a.mode==='nemesis'&&a.abilitiesEnabled&&a.capabilities.includes(NEMESIS_CATALOGUE.capability)&&a.catalogueRevision===NEMESIS_CATALOGUE.revision,'FORBIDDEN');}
export {active as validateNemesisActiveActor};
function sameRun(a:Context,b:Context){return a.roomId===b.roomId&&a.runId===b.runId&&a.runEpoch===b.runEpoch&&a.configRevision===b.configRevision;}
const tiers:Readonly<Record<number,number>>={2:1,5:2,9:3,12:4,16:5,20:6,25:6};
export function nemesisOfferPrice(level:number){demand(Number.isInteger(level)&&tiers[level]!==undefined);return 10*Math.floor(level**1.5)+15;}
export function nemesisOperationDigest(sender:NemesisActor,input:{operationId:string;offerId:string}){return hash([1,NEMESIS_CATALOGUE.revision,context(sender),input]);}
export interface NemesisOfferGroup {groupId:string;source:Context;level:number;offers:{offerId:string;abilityId:string}[];consumedBy?:string}
export interface NemesisPurchase {operationId:string;offerId:string;groupId:string;abilityId:string;cost:number;pointsAfter:number;source:Context&{name:string};digest:string;durationFrames:number|null;localHandlerId:string;effectKind:'timed'|'one-shot'|'game-effect';catalogueRevision:string;recipients:{context:Context;deliveryId:string;outcome:'pending'|'applied_reported'|'not_applied_reported'|'unknown';receiptId?:string}[];provenance:'validated-client-report'}
export interface NemesisState {groups:NemesisOfferGroup[];balances:{runId:string;principalId:string;points:number}[];purchases:NemesisPurchase[]}
export class NemesisLedger {
 constructor(private readonly state:NemesisState){}
 // TRUSTED local server offer selection only. A future adapter must establish
 // weighted without-replacement tier provenance and shop proximity/group identity.
 offerTrusted(sender:NemesisActor,value:{groupId:string;level:number;offers:{offerId:string;abilityId:string}[]}):NemesisOfferGroup{
  active(sender);demand(Object.keys(value).sort().join(',')==='groupId,level,offers'&&uuid(value.groupId)&&Array.isArray(value.offers)&&value.offers.length>=1&&value.offers.length<=3);nemesisOfferPrice(value.level);
  const seen=new Set<string>(),abilities=new Set<string>();for(const o of value.offers){const entry=NEMESIS_CATALOGUE.entries.find(e=>e.id===o.abilityId);demand(Object.keys(o).sort().join(',')==='abilityId,offerId'&&uuid(o.offerId)&&entry&&entry.depthTierWeights[tiers[value.level]!-1]!>0&&!seen.has(o.offerId)&&!abilities.has(o.abilityId));seen.add(o.offerId);abilities.add(o.abilityId);}
  const original=this.state.groups.find(g=>g.source.runId===sender.runId&&g.groupId===value.groupId),group={...structuredClone(value),source:context(sender)};
  if(original){demand(hash({...original,consumedBy:undefined})===hash(group),'CONFLICT');return structuredClone(original);}
  demand(this.state.groups.length<256,'CAPACITY');demand(!this.state.groups.some(g=>g.source.runId===sender.runId&&g.offers.some(o=>seen.has(o.offerId))),'CONFLICT');this.state.groups.push(group);return structuredClone(group);
 }
 purchase(sender:NemesisActor,value:unknown,roster:readonly NemesisActor[]){
  active(sender);demand(value&&typeof value==='object'&&!Array.isArray(value));const input=value as {operationId:string;offerId:string};demand(Object.keys(input).sort().join(',')==='offerId,operationId'&&uuid(input.operationId)&&uuid(input.offerId));
  const digest=nemesisOperationDigest(sender,input),original=this.state.purchases.find(p=>p.source.runId===sender.runId&&p.operationId===input.operationId);
  if(original){demand(original.digest===digest,'CONFLICT');return structuredClone(original);}
  const group=this.state.groups.find(g=>g.source.runId===sender.runId&&g.offers.some(o=>o.offerId===input.offerId));demand(group,'NOT_FOUND');demand(hash(group.source)===hash(context(sender)),'FORBIDDEN');demand(!group.consumedBy,'CONFLICT');
  const offer=group.offers.find(o=>o.offerId===input.offerId)!,entry=NEMESIS_CATALOGUE.entries.find(e=>e.id===offer.abilityId)!;demand(entry.domainPurchaseImplemented,'FORBIDDEN');
  demand(this.state.purchases.length<256&&roster.length<=32,'CAPACITY');const cost=nemesisOfferPrice(group.level),balance=this.state.balances.find(b=>b.runId===sender.runId&&b.principalId===sender.principalId);demand(balance&&Number.isSafeInteger(balance.points)&&balance.points>=0&&balance.points<=0xffffffff,'FORBIDDEN');demand(balance.points>=cost,'INSUFFICIENT_POINTS');
  const sessions=new Set<string>();const recipients=roster.filter(a=>{if(!sameRun(context(sender),context(a))||a.principalId===sender.principalId||!a.live||a.mode!=='nemesis'||!a.abilitiesEnabled||!a.capabilities.includes(NEMESIS_CATALOGUE.capability)||a.catalogueRevision!==NEMESIS_CATALOGUE.revision)return false;identity(a);demand(!sessions.has(a.gameSessionId),'CONFLICT');sessions.add(a.gameSessionId);return true;}).map(a=>({context:context(a),deliveryId:hash([digest,context(a)]),outcome:'pending' as const}));
  const purchase:NemesisPurchase={operationId:input.operationId,offerId:input.offerId,groupId:group.groupId,abilityId:entry.id,cost,pointsAfter:balance.points-cost,source:{...context(sender),name:sender.name},digest,durationFrames:entry.durationFrames,localHandlerId:entry.localHandlerId,effectKind:entry.effectKind,catalogueRevision:NEMESIS_CATALOGUE.revision,recipients,provenance:'validated-client-report'};
  balance.points-=cost;group.consumedBy=input.operationId;this.state.purchases.push(purchase);return structuredClone(purchase);
 }
 report(actor:NemesisActor,deliveryId:string,receiptId:string,outcome:string){
  active(actor);demand(/^[0-9a-f]{64}$/.test(deliveryId)&&uuid(receiptId)&&['applied_reported','not_applied_reported','unknown'].includes(outcome));
  const original=this.state.purchases.find(p=>sameRun(context(actor),p.source)&&p.catalogueRevision===NEMESIS_CATALOGUE.revision&&p.recipients.some(r=>r.deliveryId===deliveryId&&hash(r.context)===hash(context(actor))));demand(original,'NOT_FOUND');const delivery=original.recipients.find(r=>r.deliveryId===deliveryId)!;
  if(delivery.outcome!=='pending'){demand(delivery.receiptId===receiptId&&delivery.outcome===outcome,'CONFLICT');return structuredClone(original);}
  delivery.outcome=outcome as 'applied_reported'|'not_applied_reported'|'unknown';delivery.receiptId=receiptId;return structuredClone(original);
 }
 status(actor:NemesisActor,operationId:string){identity(actor);demand(uuid(operationId));const original=this.state.purchases.find(p=>p.operationId===operationId&&sameRun(context(actor),p.source));demand(original,'NOT_FOUND');const source=hash(context(actor))===hash(context(original.source)),own=original.recipients.filter(r=>hash(r.context)===hash(context(actor)));demand(source||own.length>0,'NOT_FOUND');const copy=structuredClone(original);if(!source){copy.recipients=own;const {pointsAfter:_pointsAfter,...visible}=copy;return visible;}return copy;}
}
