import type {FastifyInstance} from 'fastify';
import type {Database} from '../g1/database.js';
import type {PrincipalAuth} from '../g2-next/auth.js';
import type {AdmissionCompatibility} from './admission.js';
import {connectionReply} from './connection.js';
import {readOutcome} from '../rejoin/outcomes.js';
import {WorkGate} from '../g1/work.js';
import {PolicyError,demand} from '../g1/security.js';
import {decode,encode,validate,schema} from '../../../../packages/contracts/src/g3-connection.js';
import type {Request,Response} from '../../../../packages/contracts/g3-connection/g3-connection.js';
export async function registerConnectionRoutes(app:FastifyInstance,db:Database,auth:PrincipalAuth,compat:AdmissionCompatibility){
 const gate=new WorkGate(8);
 app.get('/v3/connection/schema',async()=>schema);
 app.get('/v3/connection/info',async()=>({revision:schema.revision,development:true,serviceId:db.serviceId,serviceEpoch:db.epoch,endpoint:'/v3/connection/rpc'}));
 app.post('/v3/connection/rpc',{bodyLimit:schema.limits.bytes},async(req,reply)=>{
  let input:Request={},result:Response,release:(()=>void)|undefined;
  const binary=req.headers['content-type']?.split(';')[0]==='application/x-protobuf';
  const base=()=>({major:1,revision:schema.revision,serviceId:db.serviceId,serviceEpoch:db.epoch,requestId:input.requestId??''});
  const controller=new AbortController(),timer=setTimeout(()=>controller.abort(),3000),closed=()=>{if(!reply.raw.writableFinished)controller.abort();};req.raw.once('aborted',closed);reply.raw.once('close',closed);
  try{
   input=(binary?decode('Request',req.body as Buffer):validate('Request',req.body)) as Request;
   demand(input.major===1&&input.revision===schema.revision,'VERSION_MISMATCH');demand(input.serviceId===db.serviceId,'SERVICE_MISMATCH');demand(input.serviceEpoch===db.epoch,'STALE_EPOCH');demand(/^[A-Za-z0-9_-]{1,128}$/.test(input.requestId??''),'INVALID_MESSAGE');
   for(const id of [input.admission?.roomId,input.admission?.participantId,input.outcome?.participantId,input.outcome?.roomId,input.outcome?.runId,input.outcome?.originalSessionId,input.outcome?.originalSessionEpoch,input.outcome?.originalServiceEpoch])if(id!==undefined)demand(/^[0-9a-f-]{36}$/i.test(id),'INVALID_MESSAGE');
   const token=req.headers.authorization?.match(/^Bearer ([^ ]{1,4096})$/)?.[1];demand(token,'UNAUTHENTICATED');release=gate.enter();
   result=await db.transaction(async tx=>{
    const actor=await auth.authenticate(tx,token);demand(input.sessionEpoch===actor.sessionEpoch,'STALE_EPOCH');
    if(input.outcome){demand((db as any).rejoinEnabled,'FORBIDDEN');return {...base(),outcome:await readOutcome(tx,actor,input.outcome,auth.now())};}
    const value=await connectionReply(tx,db,auth,compat,{requestId:input.requestId,roomId:input.admission!.roomId},{},token);
    demand(value.canonicalAdmission&&(!input.admission?.participantId||value.canonicalAdmission.participantId===input.admission.participantId),'FORBIDDEN');return {...base(),admission:value.canonicalAdmission,credentialValidity:value.credentialValidity};
   },true,controller.signal,input.admission?'control':'optional');
  }catch(e){result={...base(),error:{code:e instanceof PolicyError?e.code:'INVALID_MESSAGE',retryAfterMs:e instanceof PolicyError?Math.max(0,Math.min(60000,e.retryAfterMs)):0}};}
  finally{clearTimeout(timer);req.raw.removeListener('aborted',closed);reply.raw.removeListener('close',closed);release?.();}
  validate('Response',result);reply.header('cache-control','no-store');return binary?reply.type('application/x-protobuf').send(Buffer.from(encode('Response',result))):result;
 });
}

