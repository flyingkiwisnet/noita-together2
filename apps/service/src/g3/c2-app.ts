import {HOURGLASS_CATALOGUE} from '../../../../packages/domain/src/g3-hourglass.js';
import {rememberTrafficBody} from '../g2-next/traffic.js';
import type {FastifyInstance} from 'fastify';
import type {Database} from '../g1/database.js';import type {Auth} from '../g1/auth.js';
import {PolicyError,demand} from '../g1/security.js';import {WorkGate} from '../g1/work.js';
import {schema,validate,decode,encode} from '../../../../packages/contracts/src/g3-c2.js';
import type {Request,Response} from '../../../../packages/contracts/g3-c2/g3-c2.js';
import {C2_DEFAULTS,C2_SKIP_REASONS} from '../../../../packages/domain/src/g3-c2.js';
import {C2Authority} from './c2.js';
export async function registerC2Routes(app:FastifyInstance,db:Database,auth:Auth,authority:C2Authority){
 // Gameplay work must leave the existing reserved database slots for presence and renewal.
 const gate=new WorkGate(8);
 app.get('/v3/c2/info',async()=>({implemented:true,requiredCapabilities:['core.world-rules','core.hourglass'],major:1,revision:schema.revision,serviceId:db.serviceId,serviceEpoch:db.epoch,endpoint:'/v3/c2/rpc',wake:'c2Changed',defaults:C2_DEFAULTS,skipReasons:C2_SKIP_REASONS,provenance:'validated-client-report',encounterIdentity:'run+engine-progression; own-world immutable arenaToken; service encounterId',limits:{payloadBytes:32768,deliveryPage:1,cleanupPage:8,pendingPerWorld:256},effectAuthority:false}));
 app.get('/v3/c2/schema',async()=>schema);app.get('/v3/c2/catalogue',async()=>HOURGLASS_CATALOGUE);
 app.post('/v3/c2/rpc',{bodyLimit:schema.limits.bytes,preValidation:async req=>{if(req.headers['content-type']?.split(';')[0]==='application/x-protobuf')rememberTrafficBody(req,decode('Request',req.body as Buffer));}},async(req,reply)=>{
  let input:Request={},result:Response,release:(()=>void)|undefined;const binary=req.headers['content-type']?.split(';')[0]==='application/x-protobuf',base=()=>({major:1,revision:schema.revision,serviceId:db.serviceId,serviceEpoch:db.epoch,requestId:input.requestId??''});
  const abort=new AbortController(),deadline=setTimeout(()=>abort.abort(),3000),closed=()=>{if(!reply.raw.writableFinished)abort.abort();};req.raw.once('aborted',closed);reply.raw.once('close',closed);
  try{input=(binary?decode('Request',req.body as Buffer):validate('Request',req.body)) as Request;demand(input.major===1&&input.revision===schema.revision,'INVALID_MESSAGE');demand(input.serviceId===db.serviceId&&input.serviceEpoch===db.epoch,'STALE_EPOCH');demand(/^[A-Za-z0-9_-]{1,80}$/.test(input.requestId??''),'INVALID_MESSAGE');const token=req.headers.authorization?.match(/^Bearer ([^ ]{1,4096})$/)?.[1];demand(token,'UNAUTHENTICATED');release=gate.enter();result=await db.transaction(async tx=>{const actor=await auth.authenticate(tx,token);demand(actor.sessionEpoch===input.sessionEpoch,'STALE_EPOCH');return {...base(),...await authority.execute(tx,actor,input),...(!input.rulesGet&&!input.rulesSet&&!input.sourceOutcome&&input.scope?{scope:input.scope,ownerGeneration:input.ownerGeneration}:{})};},true,abort.signal,'optional');}
  catch(e){result={...base(),error:{code:e instanceof PolicyError?e.code:'INVALID_MESSAGE'}};}
  finally{clearTimeout(deadline);req.raw.removeListener('aborted',closed);reply.raw.removeListener('close',closed);release?.();}
  let bytes:Uint8Array;try{validate('Response',result);demand(Buffer.byteLength(JSON.stringify(result))<=schema.limits.bytes,'CAPACITY');bytes=encode('Response',result);}catch{result={...base(),error:{code:'CAPACITY'}};bytes=encode('Response',result);}reply.header('cache-control','no-store');return binary?reply.type('application/x-protobuf').send(Buffer.from(bytes)):result;
 });
 let pending:Promise<unknown>|undefined;const timer=setInterval(()=>{if(!pending)pending=db.transaction(tx=>authority.maintain(tx)).catch(()=>{}).finally(()=>{pending=undefined;});},60000);timer.unref();app.addHook('preClose',async()=>{clearInterval(timer);await pending;});
}
