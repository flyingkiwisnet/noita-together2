import type {FastifyInstance} from 'fastify';import type {RejoinRooms} from './rooms.js';
import {WaitingAuthority} from './waiting.js';import {WorkGate} from '../g1/work.js';import {PolicyError,demand,fingerprint} from '../g1/security.js';
import {connectionReply,credentialValidity} from '../g3/connection.js';
import {schema,decode,encode,validate} from '../../../../packages/contracts/src/g3-waiting.js';
import type {Request,Response} from '../../../../packages/contracts/g3-waiting/g3-waiting.js';
import {PendingAuthority} from './pending.js';
import {EntryAuthority} from './entry.js';
export async function registerWaitingRoutes(app:FastifyInstance,rooms:RejoinRooms){
 const authority=new WaitingAuthority(rooms),pending=new PendingAuthority(authority),entry=new EntryAuthority(pending),gate=new WorkGate(8),{db,principalAuth:auth}=rooms;
 app.get('/v3/waiting/schema',async()=>schema);
 app.get('/v3/waiting/info',async()=>({revision:schema.revision,serviceId:db.serviceId,serviceEpoch:db.epoch,candidate:true,enrollment:'after consumed preparation protected observation acknowledgement, before quit'}));
 app.post('/v3/waiting/rpc',{bodyLimit:schema.limits.bytes},async(req,reply)=>{
  let input:Request={},result:Response,release:(()=>void)|undefined;
  const binary=req.headers['content-type']?.split(';')[0]==='application/x-protobuf',base=()=>({major:1,revision:schema.revision,serviceId:db.serviceId,serviceEpoch:db.epoch,requestId:input.requestId??''});
  const controller=new AbortController(),timer=setTimeout(()=>controller.abort(),3000),closed=()=>{if(!reply.raw.writableFinished)controller.abort();};req.raw.once('aborted',closed);reply.raw.once('close',closed);
  try{
   input=(binary?decode('Request',req.body as Buffer):validate('Request',req.body)) as Request;
   demand(input.major===1&&input.revision===schema.revision,'VERSION_MISMATCH');demand(input.serviceId===db.serviceId,'SERVICE_MISMATCH');demand(input.serviceEpoch===db.epoch,'STALE_EPOCH');
   demand(/^[A-Za-z0-9_-]{1,128}$/.test(input.requestId??''),'INVALID_MESSAGE');
   for(const id of [input.waitingId,input.pendingId,input.reserve?.preparationId,input.enroll?.preparationId,input.enroll?.attachmentId,input.outcome?.originalSessionId,input.outcome?.originalSessionEpoch,input.outcome?.originalServiceEpoch,input.outcome?.waitingId,input.pendingOutcome?.originalSessionId,input.pendingOutcome?.originalSessionEpoch,input.pendingOutcome?.originalServiceEpoch])if(id!==undefined)demand(/^[a-f0-9]{8}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{12}$/i.test(id),'INVALID_MESSAGE');
   if(input.enroll)demand(!!input.enroll.contextId&&!!input.enroll.gameInstanceId&&/^[a-f0-9]{64}$/.test(input.enroll.originalAttachFingerprint??'')&&/^[a-f0-9]{64}$/.test(input.enroll.contentDigest??''),'INVALID_MESSAGE');
   if(input.recover)demand(!!input.recover.contextId&&!!input.recover.observation?.gameInstanceId,'INVALID_MESSAGE');
   if(input.outcome)demand(/^[a-f0-9]{64}$/.test(input.outcome.fingerprint??'')&&/^[A-Za-z0-9_-]{1,128}$/.test(input.outcome.requestId??''),'INVALID_MESSAGE');
   const token=req.headers.authorization?.match(/^Bearer ([^ ]{1,4096})$/)?.[1];demand(token,'UNAUTHENTICATED');release=gate.enter();
   result=await db.transaction(async tx=>{
    const sample=await credentialValidity(tx,auth,token,input.requestId!);demand(input.sessionEpoch===sample.actor.sessionEpoch,'STALE_EPOCH');
    const value=await (input.entryPreview||input.entryActivate||input.entryCancel||input.entryStatus||input.entryOutcome?entry:input.reserve||input.pendingRecover||input.pendingStatus||input.pendingOffline||input.pendingOutcome?pending:authority).execute(tx,sample.actor,input);
    if(value.pending)value.pending.sample={originRequestId:input.requestId,channel:'pending-http',credentialId:sample.actor.credentialId,sampleKey:fingerprint([db.serviceId,db.epoch,value.pending.intent.pendingId,value.pending.generation,sample.actor.sessionId,sample.actor.sessionEpoch,sample.actor.credentialId,input.requestId,'pending-http'])};
    return {...base(),...await connectionReply(tx,db,auth,rooms.compatibility,{requestId:input.requestId,roomId:value.waiting?.intent?.roomId,expectedAttachmentId:value.waiting?.owner?.attachmentId},value,token)};
   },true,controller.signal,'control');
  }catch(e){result={...base(),error:{code:e instanceof PolicyError?e.code:'INVALID_MESSAGE',retryAfterMs:e instanceof PolicyError?Math.max(0,Math.min(60000,e.retryAfterMs)):0}};}
  finally{clearTimeout(timer);req.raw.removeListener('aborted',closed);reply.raw.removeListener('close',closed);release?.();}
  result=JSON.parse(JSON.stringify(result));validate('Response',result);reply.header('cache-control','no-store');return binary?reply.type('application/x-protobuf').send(Buffer.from(encode('Response',result))):result;
 });
}

