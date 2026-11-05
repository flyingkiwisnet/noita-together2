import {RACE_GOAL_SCHEMA,RACE_GOAL_CATALOGUE} from '../../../../packages/contracts/src/g3-race-goals.js';
import type {FastifyInstance,FastifyRequest} from 'fastify';
import {WorkGate} from '../g1/work.js';
import {demand,PolicyError} from '../g1/security.js';
import {schema,validate,encode,decode} from '../../../../packages/contracts/src/g3-race.js';
import type {Request,Response} from '../../../../packages/contracts/g3-race/g3-race.js';
import type {RaceService} from './race.js';
export async function registerRaceRoutes(app:FastifyInstance,race:RaceService,webToken?:((req:FastifyRequest)=>Promise<string|undefined>)|undefined){
 const gate=new WorkGate(8),{db,auth}=race;
 app.get('/v3/race/catalogue',async(_req,reply)=>reply.header('cache-control','no-store').send({schema:RACE_GOAL_SCHEMA,content:RACE_GOAL_CATALOGUE}));
 app.get('/v3/race/schema',async(_req,reply)=>reply.header('cache-control','no-store').send(schema));
 app.post('/rpc/g3/race',{bodyLimit:schema.limits.bytes},async(req,reply)=>{
  let input:Request={},result:Response,release:(()=>void)|undefined;
  const binary=req.headers['content-type']?.split(';')[0]==='application/x-protobuf';
  const base=()=>({major:1,revision:schema.revision,serviceId:db.serviceId,serviceEpoch:db.epoch,requestId:input.requestId??''});
  const controller=new AbortController(),timer=setTimeout(()=>controller.abort(),5000);
  const abort=()=>{if(!reply.raw.writableFinished)controller.abort();};req.raw.once('aborted',abort);reply.raw.once('close',abort);
  try{
   input=(binary?decode('Request',req.body as Buffer):validate('Request',req.body)) as Request;
   demand(input.major===1&&input.revision===schema.revision,'VERSION_MISMATCH');
   demand(input.serviceId===db.serviceId&&input.serviceEpoch===db.epoch,'STALE_EPOCH');demand(/^[A-Za-z0-9_-]{1,80}$/.test(input.requestId??''),'INVALID_MESSAGE');
   for(const id of [input.scope?.roomId,input.read?.locator])if(id!==undefined)demand(/^[a-f0-9]{8}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{12}$/.test(id),'INVALID_MESSAGE');
   release=gate.enter();
   // Clock-driven closure is committed separately from a rejected late event.
   // It grants no player authority and is identical to the timer's housekeeping.
   if(input.commit)await db.transaction(async tx=>{const room=(await tx.query('SELECT * FROM rooms WHERE id=$1::uuid AND deleted_at IS NULL FOR UPDATE',[input.scope?.roomId??null])).rows[0];if(room&&room.config.mode==='race')await race.deadline(tx,room);},false,controller.signal);
   const token=req.headers.authorization?.match(/^Bearer ([^ ]{1,4096})$/)?.[1]??await webToken?.(req);
   result=await db.transaction(async tx=>{
    const actor=token?await auth.authenticate(tx,token):undefined;
    if(input.commit)demand(actor&&actor.sessionEpoch===input.sessionEpoch,'UNAUTHENTICATED');
    if(actor&&input.sessionEpoch)demand(actor.sessionEpoch===input.sessionEpoch,'STALE_EPOCH');
    // A successful read can commit deadline closure with its snapshot. Unlike a
    // rejected late original, it needs no separate transaction/projection pass.
    if(input.read){const room=(await tx.query('SELECT * FROM rooms WHERE stats_locator=$1::uuid AND deleted_at IS NULL FOR UPDATE',[input.read.locator??null])).rows[0];if(room&&room.config.mode==='race')await race.deadline(tx,room);}
    return {...base(),...await race.execute(tx,input,actor)};
   },false,controller.signal);
  }catch(e){result={...base(),error:{code:e instanceof PolicyError?e.code:'INVALID_MESSAGE'}};}
  finally{clearTimeout(timer);req.raw.removeListener('aborted',abort);reply.raw.removeListener('close',abort);release?.();}
  reply.header('cache-control','no-store');
  try{validate('Response',result);demand(Buffer.byteLength(JSON.stringify(result))<=schema.limits.bytes,'CAPACITY');return binary?reply.type('application/x-protobuf').send(Buffer.from(encode('Response',result))):result;}catch{return {...base(),error:{code:'CAPACITY'}};}
 });
}
