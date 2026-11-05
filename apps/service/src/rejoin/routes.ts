import {connectionReply} from '../g3/connection.js';
import type {FastifyInstance} from 'fastify';
import {schema,decode,encode,validate} from '../../../../packages/contracts/src/g3-rejoin.js';
import type {Request,Response} from '../../../../packages/contracts/g3-rejoin/g3-rejoin.js';
import {PolicyError,demand} from '../g1/security.js';
import {WorkGate} from '../g1/work.js';
import {RejoinAuthority} from './authority.js';
import {REJOIN_RETENTION,REJOIN_LEASE_MS} from '../../../../packages/domain/src/rejoin.js';
import {synchronizeGameplay} from '../g3/lifecycle.js';
export async function registerRejoinRoutes(app:FastifyInstance,authority:RejoinAuthority){
 const gate=new WorkGate(8),{db,auth}=authority;
 app.get('/v3/rejoin/schema',async()=>schema);
 app.get('/v3/rejoin/info',async()=>({revision:schema.revision,serviceId:db.serviceId,serviceEpoch:db.epoch,leaseMs:REJOIN_LEASE_MS,retention:REJOIN_RETENTION,endpoint:'/v3/rejoin/rpc',contractStatus:'source-candidate-engine-evidence-pending'}));
 app.post('/v3/rejoin/rpc',{bodyLimit:schema.limits.bytes},async(req,reply)=>{
  let input:Request={},result:Response,release:(()=>void)|undefined;
  const binary=req.headers['content-type']?.split(';')[0]==='application/x-protobuf',base=()=>({major:1,revision:schema.revision,serviceId:db.serviceId,serviceEpoch:db.epoch,requestId:input.requestId??''});
  const abort=new AbortController(),deadline=setTimeout(()=>abort.abort(),3000),closed=()=>{if(!reply.raw.writableFinished)abort.abort();};req.raw.once('aborted',closed);reply.raw.once('close',closed);
  try{input=(binary?decode('Request',req.body as Buffer):validate('Request',req.body)) as Request;const token=req.headers.authorization?.match(/^Bearer ([^ ]{1,4096})$/)?.[1];demand(token,'UNAUTHENTICATED');release=gate.enter();
   result=await db.transaction(async tx=>{const actor=await auth.authenticate(tx,token);demand(!input.resume&&!input.offline,'LIFECYCLE_REQUIRED');const value=await authority.execute(tx,actor,input);await synchronizeGameplay(tx,db.epoch,authority.compatibility.capabilities,auth.now(),{db,compatibility:authority.compatibility,roomId:value.participant?.association?.roomId??input.roomId});return {...base(),...await connectionReply(tx,db,auth,authority.compatibility,input,value,token)};},true,abort.signal,'control');
  }catch(e){result={...base(),error:{code:e instanceof PolicyError?e.code:'INVALID_MESSAGE',retryAfterMs:e instanceof PolicyError?Math.max(0,Math.min(60000,e.retryAfterMs)):0}};}finally{clearTimeout(deadline);req.raw.removeListener('aborted',closed);reply.raw.removeListener('close',closed);release?.();}
  try{validate('Response',result);return binary?reply.type('application/x-protobuf').header('cache-control','no-store').send(Buffer.from(encode('Response',result))):reply.header('cache-control','no-store').send(result);}catch{return reply.header('cache-control','no-store').send({...base(),error:{code:'CAPACITY'}});}
 });
}


