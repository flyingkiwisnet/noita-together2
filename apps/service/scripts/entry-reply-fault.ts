// Fixture-only post-commit transport fault. Never imported by production routes.
import {randomUUID} from 'node:crypto';
import type {FastifyInstance} from 'fastify';
import {decode} from '../../../packages/contracts/src/g3-waiting.js';
import {operationFingerprint} from '../../../packages/contracts/src/connection-fingerprint.js';
export const ENTRY_REPLY_FAULT_LIMITS={arms:8,events:32,maxDelayMs:10000} as const;
export type EntryReplyFaultTarget={pendingId:string;worldToken:string;requestId?:string};
export type EntryReplyFaultPlan=EntryReplyFaultTarget&({mode:'drop'}|{mode:'delay';delayMs:number});
export type EntryReplyCommit=EntryReplyFaultTarget&{requestId:string;accountId:string;deviceId:string;sessionEpoch:string;serviceEpoch:string;runId:string;fingerprint:string};
export type EntryReplyFaultEvent={sequence:number;kind:string;armId:string;pendingId:string;worldToken:string;requestId?:string;reason?:string;delayMs?:number};
type Arm={id:string;plan:EntryReplyFaultPlan;phase:'checking'|'armed'|'confirming'|'drop-ready'|'delaying';timer?:ReturnType<typeof setTimeout>;resume?:()=>void};
const uuid=(s:unknown)=>typeof s==='string'&&/^[a-f0-9]{8}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{12}$/i.test(s);
const text=(s:unknown,max:number)=>typeof s==='string'&&s.length>0&&Buffer.byteLength(s)<=max&&!/\p{Cc}/u.test(s);
export class EntryReplyFault {
 private armState:Arm|undefined;private closed=false;private count=0;private sequence=0;private events:EntryReplyFaultEvent[]=[];
 private event(a:Arm,kind:string,fields:Pick<EntryReplyFaultEvent,'requestId'|'reason'|'delayMs'>={}){this.events.push({sequence:++this.sequence,kind,armId:a.id,pendingId:a.plan.pendingId,worldToken:a.plan.worldToken,...fields});if(this.events.length>ENTRY_REPLY_FAULT_LIMITS.events)this.events.shift();}
 async arm(plan:EntryReplyFaultPlan,isFuture:(target:EntryReplyFaultTarget)=>Promise<boolean>){
  if(!uuid(plan.pendingId)||!text(plan.worldToken,128)||plan.requestId!==undefined&&!/^[A-Za-z0-9_-]{1,128}$/.test(plan.requestId)||!['drop','delay'].includes(plan.mode)||plan.mode==='delay'&&(!Number.isInteger(plan.delayMs)||plan.delayMs<1||plan.delayMs>ENTRY_REPLY_FAULT_LIMITS.maxDelayMs)||Object.keys(plan).some(k=>!['pendingId','worldToken','requestId','mode','delayMs'].includes(k)))throw Error('Invalid bounded entry reply fault');
  if(this.closed||this.armState||this.count>=ENTRY_REPLY_FAULT_LIMITS.arms)throw Error('Entry reply fault closed, busy or capacity reached');
  const a:Arm={id:randomUUID(),plan:structuredClone(plan),phase:'checking'};this.armState=a;
  let future=false;try{future=await isFuture({pendingId:plan.pendingId,worldToken:plan.worldToken,...(plan.requestId?{requestId:plan.requestId}:{})});}catch{if(this.armState===a)this.armState=undefined;throw Error('Entry reply future-scope verification failed');}
  if(this.closed||this.armState!==a)throw Error('Entry reply fault was released during arming');
  if(!future){this.armState=undefined;throw Error('Entry reply fault requires an unused future identity');}
  this.count++;a.phase='armed';this.event(a,'armed',plan.mode==='delay'?{delayMs:plan.delayMs}:{});return {armId:a.id,pendingId:plan.pendingId,worldToken:plan.worldToken,mode:plan.mode};
 }
 snapshot(){const a=this.armState;return {closed:this.closed,armedCount:this.count,...(a?{active:{armId:a.id,pendingId:a.plan.pendingId,worldToken:a.plan.worldToken,mode:a.plan.mode,phase:a.phase}}:{}),events:structuredClone(this.events)};}
 matches(commit:EntryReplyCommit){const a=this.armState;return !!a&&!this.closed&&a.phase==='armed'&&a.plan.pendingId===commit.pendingId&&a.plan.worldToken===commit.worldToken&&(a.plan.requestId===undefined||a.plan.requestId===commit.requestId);}
 release(armId:string,reason='caller'){const a=this.armState;if(!a||a.id!==armId)return false;this.armState=undefined;if(a.timer)clearTimeout(a.timer);this.event(a,a.phase==='delaying'?'released':'disarmed',{reason:['caller','fixture-close','fixture-restart','delay-elapsed','transport-closed'].includes(reason)?reason:'caller'});a.resume?.();return true;}
 releaseAll(reason:'fixture-close'|'fixture-restart'){const a=this.armState;if(a)this.release(a.id,reason);}
 close(){this.closed=true;this.releaseAll('fixture-close');}
 async afterCanonicalSuccess(commit:EntryReplyCommit,confirm:(value:EntryReplyCommit)=>Promise<boolean>):Promise<{action:'pass'}|{action:'drop';armId:string}>{
  const a=this.armState;if(!a||!this.matches(commit))return {action:'pass'};
  a.phase='confirming';let confirmed=false;try{confirmed=await confirm(commit);}catch{/* Never relabel refusal/failed verification as lost commit. */}
  if(this.armState!==a||this.closed)return {action:'pass'};
  if(!confirmed){a.phase='armed';this.event(a,'commit-unconfirmed',{requestId:commit.requestId});return {action:'pass'};}
  this.event(a,'committed',{requestId:commit.requestId});
  if(a.plan.mode==='drop'){a.phase='drop-ready';return {action:'drop',armId:a.id};}
  a.phase='delaying';this.event(a,'delayed',{requestId:commit.requestId,delayMs:a.plan.delayMs});
  await new Promise<void>(resolve=>{a.resume=resolve;a.timer=setTimeout(()=>this.release(a.id,'delay-elapsed'),a.plan.mode==='delay'?a.plan.delayMs:0);});return {action:'pass'};
 }
 applyDrop(armId:string,destroy:()=>void){const a=this.armState;if(!a||a.id!==armId||a.phase!=='drop-ready')return false;let applied=false;try{destroy();this.event(a,'dropped');applied=true;}catch{this.event(a,'drop-failed');}finally{this.armState=undefined;}return applied;}
}
// Extract only nonsecret operation identity. No request/response object is stored
// in the controller or exported through diagnostics.
export function entryReplyCommit(path:string,body:unknown,payload:unknown,statusCode:number):EntryReplyCommit|undefined{
 if(path.split('?')[0]!=='/v3/waiting/rpc'||statusCode!==200||typeof payload==='string'&&Buffer.byteLength(payload)>65536||Buffer.isBuffer(payload)&&payload.length>65536)return;
 try{const input:any=Buffer.isBuffer(body)?decode('Request',body):body,result:any=Buffer.isBuffer(payload)?decode('Response',payload):typeof payload==='string'?JSON.parse(payload):undefined;
  const value=input?.entryActivate,id=value?.identity,scope=value?.scope;
  if(!value||!id||!scope||result?.error||result?.entry?.state!=='activated'||result.requestId!==input.requestId||result.entry.identity?.pendingId!==id.pendingId||result.entry.identity?.worldToken!==id.worldToken||!uuid(id.pendingId)||!text(id.worldToken,128)||!text(input.requestId,128)||!uuid(scope.accountId)||!text(scope.deviceId,128)||!uuid(input.sessionEpoch)||!uuid(input.serviceEpoch)||!uuid(scope.runId))return;
  return {pendingId:id.pendingId,worldToken:id.worldToken,requestId:input.requestId,accountId:scope.accountId,deviceId:scope.deviceId,sessionEpoch:input.sessionEpoch,serviceEpoch:input.serviceEpoch,runId:scope.runId,fingerprint:operationFingerprint(input)};
 }catch{return;}
}
export function installEntryReplyFault(app:FastifyInstance,control:EntryReplyFault,confirm:(commit:EntryReplyCommit)=>Promise<boolean>){
 app.addHook('onSend',async(req,reply,payload)=>{const commit=entryReplyCommit(req.url,req.body,payload,reply.statusCode);if(!commit||!control.matches(commit))return payload;
  const active=control.snapshot().active,closed=()=>{if(active)control.release(active.armId,'transport-closed');};reply.raw.once('close',closed);
  try{const decision=await control.afterCanonicalSuccess(commit,confirm);if(decision.action==='drop'){reply.raw.removeListener('close',closed);if(control.applyDrop(decision.armId,()=>{reply.hijack();reply.raw.destroy();}))return null;}return payload;}
  finally{reply.raw.removeListener('close',closed);}
 });
}
