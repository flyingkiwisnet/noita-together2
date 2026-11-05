import Fastify,{type FastifyRequest,type FastifyReply} from 'fastify';
import type {Request,Response} from '../../../../packages/contracts/g1/g1.js';
import * as g2Codec from '../../../../packages/contracts/src/g2.js';
import {registerWebsite} from '../g2/web.js';
import {schema as g1Schema,validate as g1Validate,encode as g1Encode,decode as g1Decode,parseJson as g1ParseJson,InvalidMessage} from '../../../../packages/contracts/src/g1.js';
import {Database,type DatabaseBudget} from './database.js';
import {Auth} from './auth.js';
import {Rooms,type Compatibility} from './rooms.js';
import {Security,PolicyError,demand,fingerprint,type Keys} from './security.js';
import {WORK_LIMITS,WorkGate,unavailable,waitFor} from './work.js';
export interface Options {databaseUrl:string;serviceId:string;keys:Keys;namespace?:string;now?:()=>number;mockEnabled:boolean;providerAvailable?:()=>boolean;compatibility:Compatibility;databaseBudget?:Partial<DatabaseBudget>;requestBudget?:number;requestDeadlineMs?:number;protocolG2?:boolean;website?:boolean;profile?:import('../../../../packages/contracts/g2/g2.js').CompatibilityProfile;tls?:{pfx:Buffer;passphrase:string};canonicalOrigin?:string}
export async function createG1App(options:Options){
 if(options.canonicalOrigin&&(!options.tls||options.canonicalOrigin!==options.keys.issuer))throw Error('Canonical HTTPS origin must match issuer');
 const {schema,validate,encode,decode,parseJson}=options.protocolG2?g2Codec:{schema:g1Schema,validate:g1Validate,encode:g1Encode,decode:g1Decode,parseJson:g1ParseJson};
 const requestGate=new WorkGate(options.requestBudget??WORK_LIMITS.requestWork);const requestMs=options.requestDeadlineMs??WORK_LIMITS.requestMs;
 if(!Number.isInteger(requestMs)||requestMs<1||requestMs>10000)throw Error('Invalid request deadline');
 const db=new Database(options.databaseUrl,options.serviceId,options.namespace,options.databaseBudget,options.protocolG2??false);const security=new Security(options.keys,options.serviceId);const auth=new Auth(db,security,options.now,options.mockEnabled,options.providerAvailable);const rooms=new Rooms(db,auth,options.compatibility,options.protocolG2??false);
 try{await db.migrate();await db.start();}catch(e){await db.close();throw e;}
 const app=Fastify({logger:false,bodyLimit:schema.limits.bytes,requestTimeout:5000,...(options.tls?{https:{...options.tls,minVersion:'TLSv1.2' as const}}:{})});
 async function work<T>(req:FastifyRequest,reply:FastifyReply,fn:(signal:AbortSignal)=>Promise<T>):Promise<T>{
  const release=requestGate.enter(),controller=new AbortController();const abort=()=>controller.abort();const closed=()=>{if(!reply.raw.writableFinished)abort();};
  const deadline=setTimeout(abort,requestMs);deadline.unref();req.raw.once('aborted',abort);reply.raw.once('close',closed);
  try{if(req.raw.aborted||reply.raw.destroyed)throw unavailable();return await fn(controller.signal);}
  finally{clearTimeout(deadline);req.raw.removeListener('aborted',abort);reply.raw.removeListener('close',closed);release();}
 }
 app.removeContentTypeParser('application/json');
 app.addContentTypeParser('application/json',{parseAs:'buffer'},(req,body,done)=>{try{const text=new TextDecoder('utf-8',{fatal:true}).decode(body as Buffer);done(null,parseJson(req.url.split('?')[0]==='/v1/mock/browser-login'?'BrowserLogin':'Request',text));}catch(e){done(e as Error);}});
 app.setErrorHandler((error,_req,reply)=>{const status=error&&typeof error==='object'&&'statusCode' in error&&typeof error.statusCode==='number'?error.statusCode:400;void reply.code(status).send({error:{code:status===413?'CAPACITY':'INVALID_MESSAGE'}});});
 app.addContentTypeParser('application/x-protobuf',{parseAs:'buffer'},(_req,body,done)=>done(null,body));
 const rates=new Map<string,{n:number;until:number}>();
 app.addHook('onRequest',async(req,reply)=>{
  const now=auth.now();for(const [key,value] of rates)if(value.until<=now)rates.delete(key);
  if(req.url.includes('?')||(req.headers.origin&&req.headers.origin!==options.keys.issuer)||(options.canonicalOrigin&&req.headers.host!==new URL(options.canonicalOrigin).host)){await reply.code(403).send({error:{code:'FORBIDDEN'}});return;}
  if(rates.size>=256&&!rates.has(req.ip)){await reply.code(429).send({error:{code:'CAPACITY',retryAfterMs:1000}});return;}
  const r=rates.get(req.ip)??{n:0,until:now+1000};rates.set(req.ip,r);if(++r.n>60)await reply.code(429).send({error:{code:'CAPACITY',retryAfterMs:1000}});
 });
 app.get('/v1/info',async(_req,reply)=>{if(!db.ready)return reply.code(503).send({error:{code:'SERVICE_UNAVAILABLE',retryAfterMs:WORK_LIMITS.retryMs}});return validate('Info',{major:schema.major,revision:schema.revision,serviceId:db.serviceId,serviceEpoch:db.epoch,capabilities:options.compatibility.capabilities,compatibility:options.compatibility,limits:{...schema.limits,requestWork:requestGate.limit,databaseWork:db.budget.maxWork,acquireMs:db.budget.acquireMs,statementMs:db.budget.statementMs,lockMs:db.budget.lockMs,transactionMs:db.budget.transactionMs,requestMs},mockProvider:options.mockEnabled,contractStatus:options.protocolG2?'g2-contract-accepted-engine-pending':'g1-windows-mock-accepted',...(options.protocolG2?{profile:options.profile??{id:'g2-simulated-1',evidenceStatus:'simulated-game-facts'}}:{})});});
 app.post('/v1/mock/browser-login',async(req,reply)=>{
  try{return await work(req,reply,async signal=>{const value=validate('BrowserLogin',req.body) as {subject?:string;deviceId?:string;requestId?:string};return {credentials:await auth.mockBrowserLogin(value.subject??'',value.deviceId??'',value.requestId??'',signal)};});}
  catch(e){if(e instanceof PolicyError||(e instanceof InvalidMessage||e instanceof g2Codec.InvalidMessage))return reply.code(e instanceof PolicyError&&e.code==='CAPACITY'?429:e instanceof PolicyError&&e.code==='SERVICE_UNAVAILABLE'?503:400).send({error:{code:e instanceof PolicyError?e.code:'INVALID_MESSAGE',retryAfterMs:e instanceof PolicyError?e.retryAfterMs:0}});throw e;}
 });
 app.post('/v1/rpc',async(req,reply)=>{
  const binary=req.headers['content-type']?.split(';')[0]==='application/x-protobuf';
  let input:Request={};const base=():Response=>({major:schema.major,revision:schema.revision,serviceId:db.serviceId,serviceEpoch:db.epoch,requestId:input.requestId??''});let result:Response;
  try{
   input=(binary?decode('Request',req.body as Buffer):validate('Request',req.body)) as Request;
   demand(input.major===schema.major&&input.revision===schema.revision,'VERSION_MISMATCH');demand(input.serviceId===db.serviceId,'SERVICE_MISMATCH');demand(input.requestId&&/^[a-zA-Z0-9_-]{1,128}$/.test(input.requestId),'INVALID_MESSAGE');
   const publicCommand=!!(input.pairBegin||input.pairPoll||input.pairExchange||input.pairCancel||input.refresh);
   const token=req.headers.authorization?.match(/^Bearer ([^ ]{1,4096})$/)?.[1];
   demand(publicCommand||token,'UNAUTHENTICATED');
   result=await work(req,reply,async signal=>{
   // Single-flight expiration commits independently of a rejected/stale command.
   // Pairing/refresh have their own expiry checks and do not wait for room cleanup.
   if(!publicCommand)await waitFor(rooms.sweep(),signal);
   return await db.transaction(async tx=>{
    const actor=publicCommand?undefined:await auth.authenticate(tx,token!,input);
    if(input.pairBegin)return {...base(),pairing:await auth.begin(tx,input)};
    if(input.pairApprove||input.pairPoll||input.pairExchange||input.pairCancel)return {...base(),...await auth.pairing(tx,input,actor)};
    if(input.refresh)return {...base(),...await auth.refresh(tx,input)};
    if(input.logout){
     demand(actor,'UNAUTHENTICATED');demand(actor.kind!=='game','FORBIDDEN');
     const cached=(await tx.query('SELECT * FROM command_results WHERE scope=$1 AND request_id=$2',[actor.sessionId,input.requestId])).rows[0];
     if(cached){demand(cached.fingerprint===fingerprint(input),'CONFLICT');return base();}
     demand(input.serviceEpoch===db.epoch&&input.sessionEpoch===actor.sessionEpoch,'STALE_EPOCH');
     await tx.query('INSERT INTO command_results(scope,request_id,fingerprint,result,created_at) VALUES($1,$2,$3,$4,$5)',[actor.sessionId,input.requestId,fingerprint(input),{sealed:security.seal({})},new Date(auth.now())]);
     await auth.revoke(tx,actor.familyId);return base();
    }
    demand(actor,'UNAUTHENTICATED');return {...base(),...await rooms.execute(tx,input,actor)};
   },true,signal);
   });
  }catch(e){
   const code=e instanceof PolicyError?e.code:(e instanceof InvalidMessage||e instanceof g2Codec.InvalidMessage)?'INVALID_MESSAGE':'INTERNAL';
   // No credentials, database queries, provider subjects or arbitrary exception text in responses/logs.
   if(code==='INTERNAL')app.log.error({code},'G1 operation failed');
   result={...base(),error:{code,message:code,retryAfterMs:e instanceof PolicyError?e.retryAfterMs:0}};
  }
  try{validate('Response',result);if(Buffer.byteLength(JSON.stringify(result))>schema.limits.bytes)throw new PolicyError('CAPACITY');}catch{result={...base(),error:{code:'CAPACITY',message:'CAPACITY'}};}
  if(binary){reply.type('application/x-protobuf');return Buffer.from(encode('Response',result));}return result;
 });
 if(options.website)await registerWebsite(app);
 const timer=setInterval(()=>{void rooms.sweep().catch(()=>app.log.error({code:'INTERNAL'},'G1 expiry sweep failed'));},1000);timer.unref();
 app.addHook('preClose',async()=>{clearInterval(timer);await db.close();});
 return {app,db,auth,rooms,requestGate};
}
