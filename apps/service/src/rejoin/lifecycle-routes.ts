import type {FastifyInstance} from 'fastify';
import type {RejoinRooms} from './rooms.js';
import {WorldLifecycle} from './lifecycle.js';
import {WorkGate} from '../g1/work.js';
import {PolicyError,demand} from '../g1/security.js';
import {credentialValidity} from '../g3/connection.js';
import {schema,validate,decode,encode} from '../../../../packages/contracts/src/g3-lifecycle.js';
import type {Request} from '../../../../packages/contracts/g3-lifecycle/g3-lifecycle.js';
export async function registerLifecycleRoutes(app:FastifyInstance,rooms:RejoinRooms){
 const authority=new WorldLifecycle(rooms),gate=new WorkGate(8),{db,principalAuth:auth}=rooms;
 app.get('/v3/lifecycle/schema',async()=>schema);
 app.get('/v3/lifecycle/info',async()=>({revision:schema.revision,endpoint:'/v3/lifecycle/rpc',serviceId:db.serviceId,serviceEpoch:db.epoch}));
 app.post('/v3/lifecycle/rpc',{bodyLimit:schema.limits.bytes},async(req,reply)=>{
  let input:Request={},result:any,release:(()=>void)|undefined;
  const binary=req.headers['content-type']?.split(';')[0]==='application/x-protobuf';
  const base=()=>({major:1,revision:schema.revision,serviceId:db.serviceId,serviceEpoch:db.epoch,requestId:input.requestId??''});
  const controller=new AbortController(),timer=setTimeout(()=>controller.abort(),3000),closed=()=>{if(!reply.raw.writableFinished)controller.abort();};req.raw.once('aborted',closed);reply.raw.once('close',closed);
  try{
   input=(binary?decode('Request',req.body as Buffer):validate('Request',req.body)) as Request;
   demand(input.major===schema.major&&input.revision===schema.revision,'VERSION_MISMATCH');demand(input.serviceId===db.serviceId,'SERVICE_MISMATCH');demand(input.serviceEpoch===db.epoch,'STALE_EPOCH');demand(/^[A-Za-z0-9_-]{1,128}$/.test(input.requestId??''),'INVALID_MESSAGE');
   const token=req.headers.authorization?.match(/^Bearer ([^ ]{1,4096})$/)?.[1];demand(token,'UNAUTHENTICATED');release=gate.enter();
   result=await db.transaction(async tx=>{const sample=await credentialValidity(tx,auth,token,input.requestId!);demand(input.sessionEpoch===sample.actor.sessionEpoch,'STALE_EPOCH');const value=await authority.execute(tx,sample.actor,input);return {...base(),...value,...(!value.credentialValidity?{credentialValidity:sample.value}:{})};},true,controller.signal,'control');
  }catch(e){result={...base(),error:{code:e instanceof PolicyError?e.code:'INVALID_MESSAGE',retryAfterMs:e instanceof PolicyError?Math.min(30000,Math.max(0,e.retryAfterMs)):0}};}
  finally{clearTimeout(timer);req.raw.removeListener('aborted',closed);reply.raw.removeListener('close',closed);release?.();}
  result=JSON.parse(JSON.stringify(result));validate('Response',result);reply.header('cache-control','no-store');return binary?reply.type('application/x-protobuf').send(Buffer.from(encode('Response',result))):result;
 });
}

