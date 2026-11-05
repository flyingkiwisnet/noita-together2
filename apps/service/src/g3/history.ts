import type {Tx,Database} from '../g1/database.js';
import type {Auth,Actor} from '../g1/auth.js';
import type {FastifyInstance} from 'fastify';
import {demand,PolicyError} from '../g1/security.js';
import {WorkGate} from '../g1/work.js';
import {schema,validate,encode,decode} from '../../../../packages/contracts/src/g3-history.js';
import type {Request,Response,Run} from '../../../../packages/contracts/g3-history/g3-history.js';
import {createHash} from 'node:crypto';
import {readFile} from 'node:fs/promises';
const cursor=(s:string|undefined)=>{const v=s??'0';demand(/^(0|[1-9][0-9]{0,18})$/.test(v)&&BigInt(v)<=9223372036854775807n,'INVALID_MESSAGE');return v;};
export async function discoverRuns(tx:Tx,actor:Actor,input:Request,now:number):Promise<Partial<Response>>{
 demand(actor.kind==='browser'||actor.kind==='desktop','FORBIDDEN');const p=input.discover;demand(p,'INVALID_MESSAGE');
 const after=cursor(p.afterSequence),limit=p.limit??32;demand(Number.isInteger(limit)&&limit>=1&&limit<=32,'INVALID_MESSAGE');
 const max=(await tx.query('SELECT discovery_sequence::text AS high FROM g3_access WHERE principal_id=$1 ORDER BY discovery_sequence DESC LIMIT 1',[actor.accountId])).rows[0]?.high??'0';
 const high=p.throughSequence===undefined?max:cursor(p.throughSequence);demand(BigInt(high)<=BigInt(max),'INVALID_MESSAGE');
 const rows=(await tx.query(`SELECT r.* FROM g3_runs r JOIN g3_access a ON a.run_id=r.id AND a.principal_id=$1
  WHERE a.discovery_sequence>$2 AND a.discovery_sequence<=$3 ORDER BY a.discovery_sequence LIMIT $4`,[actor.accountId,after,high,limit+1])).rows;
 const more=rows.length>limit,runs:Run[]=rows.slice(0,limit).map(r=>({sequence:r.discovery_sequence,scope:{roomId:r.room_id,runId:r.id,runEpoch:r.epoch,configRevision:r.config_revision},roomName:r.room_name,createdAt:r.created_at.toISOString(),state:r.state,mode:r.mode,canDelete:r.owner_id===actor.accountId,retentionDays:30,collectionState:r.statistics_deleted_at?'deleted':r.statistics_expires_at?(+r.statistics_expires_at>now?'available':'expired'):'empty',latestExpiresAt:r.statistics_expires_at?.toISOString()??'',deletedAt:r.statistics_deleted_at?.toISOString()??''}));
 return {runs,nextSequence:more?runs.at(-1)!.sequence!:(BigInt(after)>BigInt(high)?after:high),highSequence:high};
}
export async function registerHistoryRoutes(app:FastifyInstance,{db,auth}:{db:Database;auth:Auth}){
 const gate=new WorkGate(8),schemaSha256=createHash('sha256').update(await readFile(new URL('../../../../packages/contracts/g3-history/schema.json',import.meta.url))).digest('hex');
 app.get('/v3/history/schema',async()=>schema);app.get('/v3/history/info',async()=>({revision:schema.revision,schemaSha256,serviceId:db.serviceId,serviceEpoch:db.epoch,retentionDays:30}));
 app.post('/v3/history/rpc',{bodyLimit:schema.limits.bytes},async(req,reply)=>{
  let input:Request={},result:Response,release:(()=>void)|undefined;
  const binary=req.headers['content-type']?.split(';')[0]==='application/x-protobuf';
  const base=()=>({major:1,revision:schema.revision,serviceId:db.serviceId,serviceEpoch:db.epoch,requestId:input.requestId??''});
  const abort=new AbortController(),deadline=setTimeout(()=>abort.abort(),3000),closed=()=>{if(!reply.raw.writableFinished)abort.abort();};req.raw.once('aborted',closed);reply.raw.once('close',closed);
  try{
   input=(binary?decode('Request',req.body as Buffer):validate('Request',req.body)) as Request;
   demand(input.major===1&&input.revision===schema.revision,'VERSION_MISMATCH');demand(input.serviceId===db.serviceId&&input.serviceEpoch===db.epoch,'STALE_EPOCH');demand(/^[A-Za-z0-9_-]{1,80}$/.test(input.requestId??''),'INVALID_MESSAGE');
   const token=req.headers.authorization?.match(/^Bearer ([^ ]{1,4096})$/)?.[1];demand(token,'UNAUTHENTICATED');release=gate.enter();
   result=await db.transaction(async tx=>{const actor=await auth.authenticate(tx,token);demand(actor.sessionEpoch===input.sessionEpoch,'STALE_EPOCH');return {...base(),...await discoverRuns(tx,actor,input,auth.now())};},false,abort.signal);
  }catch(e){result={...base(),error:{code:e instanceof PolicyError?e.code:'INVALID_MESSAGE'}};}
  finally{clearTimeout(deadline);req.raw.removeListener('aborted',closed);reply.raw.removeListener('close',closed);release?.();}
  let bytes:Uint8Array;try{validate('Response',result);demand(Buffer.byteLength(JSON.stringify(result))<=schema.limits.bytes,'CAPACITY');bytes=encode('Response',result);}catch{result={...base(),error:{code:'CAPACITY'}};bytes=encode('Response',result);}
  reply.header('cache-control','no-store');return binary?reply.type('application/x-protobuf').send(Buffer.from(bytes)):result;
 });
}
