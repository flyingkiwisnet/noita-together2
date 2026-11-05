import {c1Statistic} from './stats-store.js';
// Exact original-outcome recovery with normal current desktop authentication.
// This path records source self-reports only; it never restores world authority.
import type {Tx} from '../g1/database.js';import type {Actor} from '../g1/auth.js';
import {demand,fingerprint} from '../g1/security.js';
import {validate} from '../../../../packages/contracts/src/g3-c1.js';
import type {Publish,Request,SourceOriginal,SourceDisposition,SourceOutcome} from '../../../../packages/contracts/g3-c1/g3-c1.js';
import {normalizeC1Payload,C1_SKIP_REASONS} from '../../../../packages/domain/src/g3-c1.js';
const uuid=(v:unknown)=>typeof v==='string'&&/^[a-f0-9]{8}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{12}$/.test(v);
export function c1PublishDigest(principalId:string,original:SourceOriginal){const p=original.publish!;return fingerprint({principalId,runId:original.runId,roomId:original.roomId,worldToken:original.worldToken,eventId:p.eventId,rulesRevision:p.rulesRevision,payload:normalizeC1Payload(p.payload!),senderDisposition:p.senderDisposition});}
export function c1SourceReceipt(input:SourceDisposition){validate('SourceDisposition',input);demand(['applied','skipped','uncertain'].includes(input.state??''),'INVALID_MESSAGE');const reason=input.reason??'';demand(input.state==='skipped'?C1_SKIP_REASONS.includes(reason as any):reason==='','INVALID_MESSAGE');return {state:input.state!,reason};}
export async function recordC1SourceReceipt(tx:Tx,event:any,input:SourceDisposition){const receipt=c1SourceReceipt(input);const duplicate=event.source_state!=='pending';if(duplicate)demand(event.source_state===receipt.state&&(event.source_reason??'')===receipt.reason,'CONFLICT');else await tx.query('UPDATE c1_events SET source_state=$3,source_reason=$4 WHERE run_id=$1 AND event_id=$2',[event.run_id,event.event_id,receipt.state,receipt.reason]);const result={...event,source_state:receipt.state,source_reason:receipt.reason,duplicate};await c1Statistic(tx,result);return result;}
function sourceOriginal(value:SourceOriginal|undefined):SourceOriginal{
 demand(value,'INVALID_MESSAGE');validate('SourceOriginal',value);const v=value!,p=v.publish;demand(uuid(v.roomId)&&uuid(v.runId)&&typeof v.worldToken==='string'&&Buffer.byteLength(v.worldToken)>0&&Number.isInteger(v.configRevision)&&v.configRevision!>=1&&p&&uuid(p.eventId)&&Number.isInteger(p.rulesRevision)&&p.rulesRevision!>=1&&['pending','applied','skipped','uncertain'].includes(p.senderDisposition??'')&&p.payload,'INVALID_MESSAGE');
 const payload=normalizeC1Payload(p!.payload!);demand(payload.death,'FORBIDDEN');return {roomId:v.roomId!,runId:v.runId!,worldToken:v.worldToken!,configRevision:v.configRevision!,publish:{eventId:p!.eventId!,rulesRevision:p!.rulesRevision!,payload,senderDisposition:p!.senderDisposition!}};
}
export async function readC1SourceOutcome(tx:Tx,actor:Actor,input:Request):Promise<{sourceOutcome:SourceOutcome}>{
 // The caller already passed the ordinary token/session/family/expiry checks.
 demand(actor.kind==='desktop'&&input.sourceOutcome&&input.scope===undefined&&input.ownerGeneration===undefined,'FORBIDDEN');
 const original=sourceOriginal(input.sourceOutcome!.original);demand(input.roomId===undefined||input.roomId===original.roomId,'FORBIDDEN');const receipt=input.sourceOutcome!.receipt;if(receipt)c1SourceReceipt(receipt);
 const base={original,effectAuthority:false,selfReported:true,savedEffectProof:false};
 const event=(await tx.query('SELECT * FROM c1_events WHERE run_id=$1 AND event_id=$2 FOR UPDATE',[original.runId,original.publish!.eventId])).rows[0];
 if(!event)return {sourceOutcome:{...base,outcome:'unresolved',coverage:'absent_expired_or_uncovered'}};
 demand(event.source_principal===actor.accountId&&event.room_id===original.roomId&&event.source_world===original.worldToken,'FORBIDDEN');
 if(!event.source_device||!Number(event.source_config_revision))return {sourceOutcome:{...base,outcome:'unresolved',coverage:'absent_expired_or_uncovered'}};
 demand(event.source_device===actor.deviceId&&Number(event.source_config_revision)===original.configRevision,'FORBIDDEN');demand(event.digest===c1PublishDigest(actor.accountId,original),'CONFLICT');
 const current=receipt?await recordC1SourceReceipt(tx,event,receipt):event;
 return {sourceOutcome:{...base,outcome:'committed',coverage:'exact_durable_record',sourceDisposition:current.source_state,sourceReason:current.source_reason??'',acceptedSequence:String(event.sequence),rulesRevision:Number(event.rules_revision),duplicate:current.duplicate??false}};
}
