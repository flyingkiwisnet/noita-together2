import type {FastifyInstance} from 'fastify';
import type {Database} from '../g1/database.js';
import type {Auth} from '../g1/auth.js';
import {demand,PolicyError} from '../g1/security.js';
import {WorkGate} from '../g1/work.js';
import {NemesisAuthority} from './nemesis.js';

import {schema,decode,encode,validate} from '../../../../packages/contracts/src/g3-nemesis.js';
import type {Request,Response} from '../../../../packages/contracts/g3-nemesis/g3-nemesis.js';
import {createHash} from 'node:crypto';
import {readFile} from 'node:fs/promises';
export async function registerNemesisRoutes(app:FastifyInstance,{db,auth,authority,sweep}:{db:Database;auth:Auth;authority:NemesisAuthority;sweep:()=>Promise<void>}){
 const gate=new WorkGate(8);
 const schemaSha256=createHash('sha256').update(await readFile(new URL('../../../../packages/contracts/g3-nemesis/schema.json',import.meta.url))).digest('hex');
 app.get('/v3/nemesis/schema',async()=>schema);
 app.get('/v3/nemesis/info',async()=>({major:1,revision:schema.revision,schemaSha256,serviceId:db.serviceId,serviceEpoch:db.epoch,catalogueRevision:'nt2-nemesis-builtins-2',catalogueSha256:createHash('sha256').update(await readFile(new URL('../../../../packages/contracts/g3-nemesis/catalogue-v2.json',import.meta.url))).digest('hex'),purchaseCapacity:256,earningEnabled:false,offeringEnabled:false,fixtureFundingOnly:true,pageLimit:8,capabilities:authority.capabilities.filter(c=>c==='nemesis.abilities'),contractStatus:'simulated-candidate-engine-pending'}));
 app.post('/v3/nemesis/rpc',{bodyLimit:schema.limits.bytes},async(req,reply)=>{
  let input:Request={},result:Response,release:(()=>void)|undefined;
  const binary=req.headers['content-type']?.split(';')[0]==='application/x-protobuf',base=()=>({major:1,revision:schema.revision,serviceId:db.serviceId,serviceEpoch:db.epoch,requestId:input.requestId??''});
  const abort=new AbortController(),deadline=setTimeout(()=>abort.abort(),3000),closed=()=>{if(!reply.raw.writableFinished)abort.abort();};
  req.raw.once('aborted',closed);reply.raw.once('close',closed);
  try{
   input=(binary?decode('Request',req.body as Buffer):validate('Request',req.body)) as Request;
   demand(input.major===1&&input.revision===schema.revision,'INVALID_MESSAGE');
   demand(input.serviceId===db.serviceId&&input.serviceEpoch===db.epoch,'STALE_EPOCH');
   demand(/^[A-Za-z0-9_-]{1,80}$/.test(input.requestId??''),'INVALID_MESSAGE');
   const token=req.headers.authorization?.match(/^Bearer ([^ ]{1,4096})$/)?.[1];demand(token,'UNAUTHENTICATED');
   release=gate.enter();await sweep();
   result=await db.transaction(async tx=>{const actor=await auth.authenticate(tx,token);demand(actor.sessionEpoch===input.sessionEpoch,'STALE_EPOCH');await authority.maintain(tx);return {...base(),...await authority.execute(tx,actor,input)};},true,abort.signal);
  }catch(e){result={...base(),error:{code:e instanceof PolicyError?e.code:'INVALID_MESSAGE'}};}
  finally{clearTimeout(deadline);req.raw.removeListener('aborted',closed);reply.raw.removeListener('close',closed);release?.();}
  let encoded:Uint8Array;
  try{validate('Response',result);demand(Buffer.byteLength(JSON.stringify(result))<=schema.limits.bytes,'CAPACITY');encoded=encode('Response',result);}
  catch{result={...base(),error:{code:'CAPACITY'}};encoded=encode('Response',result);}
  reply.header('cache-control','no-store');return binary?reply.type('application/x-protobuf').send(Buffer.from(encoded)):result;
 });
 let pending:Promise<unknown>|undefined;
 const timer=setInterval(()=>{if(!pending)pending=db.transaction(tx=>authority.maintain(tx)).catch(()=>{}).finally(()=>{pending=undefined;});},60000);timer.unref();
 app.addHook('preClose',async()=>{clearInterval(timer);await pending;});
}
