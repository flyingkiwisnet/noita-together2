import {ITEM_CATALOGUE} from '../../../../packages/domain/src/g3-items.js';
import {bankRevision,markBankDirty} from './bank-events.js';
import {rememberTrafficBody} from '../g2-next/traffic.js';
import Fastify,{type FastifyInstance} from 'fastify';
import {G3Database} from './database.js';
import type {Database,Tx} from '../g1/database.js';
import {G3Authority} from './authority.js';
import {Auth} from '../g1/auth.js';
import {Security,PolicyError,demand} from '../g1/security.js';
import {WorkGate} from '../g1/work.js';
import type {Keys} from '../g1/security.js';
import {schema,parseJson,decode,encode,validate} from '../../../../packages/contracts/src/g3.js';
import type {Request,Response} from '../../../../packages/contracts/g3/g3.js';

export async function registerG3Routes(app:FastifyInstance,options:{db:Database;auth:Auth;authority:G3Authority;beforeExecute?:(tx:Tx,input:Request)=>Promise<void>;sweep?:()=>Promise<void>}){
 const {db,auth,authority}=options,gate=new WorkGate(16);
 app.get('/v3/schema',async()=>schema);
 app.get('/v3/items/catalogue',async()=>ITEM_CATALOGUE);
 app.get('/v3/info',async()=>({revision:schema.revision,serviceId:db.serviceId,serviceEpoch:db.epoch,contractStatus:'provisional-local-delivery-unproven'}));
 app.post('/v3/rpc',{bodyLimit:schema.limits.bytes,preValidation:async req=>{if(req.headers['content-type']?.split(';')[0]==='application/x-protobuf')rememberTrafficBody(req,decode('Request',req.body as Buffer));}},async(req,reply)=>{
  let input:Request={},result:Response;
  const base=()=>({major:1,revision:schema.revision,serviceId:db.serviceId,serviceEpoch:db.epoch,requestId:input.requestId??''});
  const binary=req.headers['content-type']?.split(';')[0]==='application/x-protobuf';let release:(()=>void)|undefined;
  try{
   input=(binary?decode('Request',req.body as Buffer):validate('Request',req.body)) as Request;
   demand(input.major===1&&input.revision===schema.revision,'INVALID_MESSAGE');
   demand(input.serviceId===db.serviceId&&input.serviceEpoch===db.epoch,'STALE_EPOCH');
   demand(input.scope&&/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i.test(input.scope.roomId??'')&&/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i.test(input.scope.runId??''),'INVALID_MESSAGE');
   demand(/^[A-Za-z0-9_-]{1,80}$/.test(input.requestId??''),'INVALID_MESSAGE');
   const token=req.headers.authorization?.match(/^Bearer ([^ ]{1,4096})$/)?.[1];demand(token,'UNAUTHENTICATED');
   release=gate.enter();await options.sweep?.();const effects:(()=>void)[]=[];
   result=await db.transaction(async tx=>{
    const actor=await auth.authenticate(tx,token);demand(actor.sessionEpoch===input.sessionEpoch,'STALE_EPOCH');
    await options.beforeExecute?.(tx,input);const value=await authority.execute(tx,actor,input,effects);
    const bankCommand=!!(input.goldBank||input.itemBank||input.goldIntent||input.goldStatus||input.goldReceipt||input.itemDeposit||input.itemClaim||input.itemStatus||input.itemReceipt||input.bankBatch);
    if(bankCommand){value.bankRevision=await bankRevision(tx,input.scope!.runId!);const mutates=!!(input.goldIntent||input.goldReceipt||input.itemDeposit||input.itemClaim||input.itemReceipt)||!!input.bankBatch?.entries?.some((entry,i)=>!value.bankBatch?.entries?.[i]?.error&&(entry.goldIntent||entry.goldReceipt||entry.itemDeposit||entry.itemClaim||entry.itemReceipt));if(mutates)markBankDirty(tx,input.scope!.runId!);}
    return {...base(),...value};
   });
   for(const effect of effects)effect();
  }catch(e){result={...base(),error:{code:e instanceof PolicyError?e.code:'INVALID_MESSAGE'}};}finally{release?.();}
  validate('Response',result);demand(Buffer.byteLength(JSON.stringify(result))<=schema.limits.bytes,'CAPACITY');reply.header('cache-control','no-store');
  return binary?reply.type('application/x-protobuf').send(Buffer.from(encode('Response',result))):result;
 });
 let retention:Promise<unknown>|undefined;
 const timer=setInterval(()=>{if(!retention)retention=db.transaction(tx=>authority.maintain(tx)).catch(()=>{}).finally(()=>{retention=undefined;});},60000);
 timer.unref();app.addHook('preClose',async()=>{clearInterval(timer);await retention;});
}

// Low-level domain fixtures only; application integration uses createG3CandidateApp.
export async function createG3App(options:{databaseUrl:string;namespace:string;serviceId:string;keys:Keys;mockEnabled:boolean;now?:()=>number}){
 demand(options.mockEnabled&&process.env.NODE_ENV!=='production'&&options.namespace.startsWith('g3_'),'FORBIDDEN');
 const db=new G3Database(options.databaseUrl,options.serviceId,options.namespace),security=new Security(options.keys,options.serviceId),auth=new Auth(db,security,options.now,true),authority=new G3Authority(()=>db.epoch,options.now);
 try{await db.migrate();await db.start();}catch(e){await db.close();throw e;}
 const app=Fastify({logger:false,bodyLimit:schema.limits.bytes,requestTimeout:5000});
 app.removeContentTypeParser('application/json');
 app.addContentTypeParser('application/json',{parseAs:'buffer'},(_req,body,done)=>{try{done(null,parseJson('Request',new TextDecoder('utf-8',{fatal:true}).decode(body as Buffer)));}catch(e){done(e as Error);}});
 app.addContentTypeParser('application/x-protobuf',{parseAs:'buffer'},(_req,body,done)=>done(null,body));
 app.setErrorHandler((_e,_req,reply)=>void reply.code(400).send({error:{code:'INVALID_MESSAGE'}}));
 app.addHook('onRequest',async(req,reply)=>{if(req.url.includes('?')||(req.headers.origin&&req.headers.origin!==options.keys.issuer))await reply.code(403).send({error:{code:'FORBIDDEN'}});});
 await registerG3Routes(app,{db,auth,authority});
 app.addHook('preClose',async()=>db.close());return {app,db,auth,authority};
}
