import type {FastifyInstance,FastifyRequest} from 'fastify';
import type {Database,Tx} from '../g1/database.js';
import type {Auth,Actor} from '../g1/auth.js';
import {demand,fingerprint,PolicyError} from '../g1/security.js';
import {WorkGate} from '../g1/work.js';
import {schema,validate,decode,encode} from '../../../../packages/contracts/src/g3-stats.js';
import type {Request,Response,Summary} from '../../../../packages/contracts/g3-stats/g3-stats.js';
import {readRoomAdmissions} from './admission.js';
import {recordGameplayStatistic,knownStatisticReceipt,validateObservationSource,STAT_COLUMNS} from './stats-store.js';
const uuid=(v:unknown):v is string=>typeof v==='string'&&/^[a-f0-9]{8}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{12}$/.test(v);
const camel=(s:string)=>s.replace(/_([a-z])/g,(_,c:string)=>c.toUpperCase());
export class LobbyStatistics {
 constructor(readonly db:Database,readonly auth:Auth,readonly compatibility:any){}
 async execute(tx:Tx,input:Request,actor?:Actor):Promise<Partial<Response>>{
  if(input.observe){
   demand(actor,'UNAUTHENTICATED');const a=actor!,s=input.scope,p=input.observe;
   demand(s&&uuid(s.roomId)&&uuid(s.runId)&&uuid(s.attachmentId)&&s.worldToken&&uuid(p.eventId),'INVALID_MESSAGE');
   if(a.kind==='desktop'){
    // Recover a previously accepted receipt across owner/epoch/End changes;
    // account/device/world/original and canonical run/config stay pinned.
    const known=await knownStatisticReceipt(tx,{runId:s!.runId!,worldToken:s!.worldToken!,principalId:a.accountId,deviceId:a.deviceId},{eventId:p.eventId!,kind:p.kind!,...(p.progression!==undefined?{progression:p.progression}:{}),...(p.orbId!==undefined?{orbId:p.orbId}:{})});
    if(known){demand((await tx.query('SELECT 1 FROM d1_runs d JOIN runs n ON n.id=d.run_id WHERE d.run_id=$1 AND d.room_id=$2 AND n.config_revision=$3',[s!.runId,s!.roomId,s!.configRevision])).rowCount,'FORBIDDEN');return known;}
    // Narrow exact-original recovery after a terminal C1 transition revoked its
    // game credential. This never accepts a fresh gameplay observation.
    const e=(await tx.query('SELECT * FROM c1_events WHERE run_id=$1 AND event_id=$2',[s!.runId,p.eventId])).rows[0];
    demand(e&&e.source_principal===a.accountId&&e.source_device===a.deviceId&&e.source_world===s!.worldToken&&e.room_id===s!.roomId&&Number(e.source_config_revision)===s!.configRevision,'FORBIDDEN');
    demand(e.body.payload.death&&(p.kind==='death'&&e.body.payload.death.state!=='won'||p.kind==='win'&&e.body.payload.death.state==='won'),'FORBIDDEN');
    demand((p.kind!=='win'||(p.progression??0)===(e.body.payload.death.progression??0))&&p.orbId===undefined,'CONFLICT');
    return {accepted:false,duplicate:false};
   }
   demand(a.kind==='game','FORBIDDEN');
   const admission=await readRoomAdmissions(tx,this.db,this.compatibility,this.auth.now(),s!.roomId!);
   const w=admission.worlds.find(w=>w.owner.gameSessionId===a.sessionId);
   demand(w&&w.mode==='core'&&w.generation===input.ownerGeneration,'STALE_EPOCH');
   demand(fingerprint(s)===fingerprint({roomId:w!.scope.roomId,runId:w!.scope.runId,runEpoch:w!.scope.runEpoch,configRevision:w!.scope.configRevision,attachmentId:w!.scope.attachmentId,worldToken:w!.scope.worldToken}),'STALE_EPOCH');
   const source={runId:s!.runId!,worldToken:s!.worldToken!,principalId:a.accountId,deviceId:a.deviceId},observation={eventId:p.eventId!,kind:p.kind!,...(p.progression!==undefined?{progression:p.progression}:{}),...(p.orbId!==undefined?{orbId:p.orbId}:{})};
   const known=await knownStatisticReceipt(tx,source,observation);if(known)return known;
   const entered=(p.kind==='death'||p.kind==='win')&&w!.decision.authorized&&!!(await tx.query('SELECT 1 FROM authorization_play_entries WHERE attachment_id=$1',[s!.attachmentId])).rowCount;
   demand(w!.decision.playing||entered,'FORBIDDEN');
   if(p.kind==='orb'||p.kind==='win'){const progression=Number((await tx.query('SELECT progression FROM c2_world_progress WHERE run_id=$1 AND principal_id=$2 AND world_token=$3',[s!.runId,a.accountId,s!.worldToken])).rows[0]?.progression??0);demand(p.progression===progression,'CONFLICT');}
   await validateObservationSource(tx,source,observation);return recordGameplayStatistic(tx,source,observation);
  }
  demand(!actor||actor.kind!=='game','FORBIDDEN');
  if(input.discover){
   const p=input.discover,limit=p.limit??32;demand(limit>=1&&limit<=32&&(!p.afterLocator||uuid(p.afterLocator)),'INVALID_MESSAGE');
   const rows=(await tx.query(`SELECT r.stats_locator,r.name,r.config->>'mode' AS mode,r.capacity,r.state,r.public_stats,
    (SELECT count(*) FROM memberships m WHERE m.room_id=r.id AND NOT m.roster_retired) AS occupancy
    FROM rooms r WHERE r.deleted_at IS NULL AND r.visibility='public' AND ($1::uuid IS NULL OR r.stats_locator>$1) ORDER BY r.stats_locator LIMIT $2`,[p.afterLocator||null,limit+1])).rows;
   return {lobbies:rows.slice(0,limit).map(r=>({locator:r.stats_locator,name:r.name,mode:r.mode,capacity:r.capacity,state:r.state,publicStats:r.public_stats,occupancy:Number(r.occupancy)})),nextLocator:rows.length>limit?rows[limit-1].stats_locator:''};
  }
  const p=input.read!;demand(p&&uuid(p.locator)&&(p.limit??32)>=1&&(p.limit??32)<=32&&(!p.afterRow||uuid(p.afterRow)),'INVALID_MESSAGE');
  const r=(await tx.query('SELECT * FROM rooms WHERE stats_locator=$1 AND deleted_at IS NULL',[p.locator])).rows[0];demand(r,'NOT_FOUND');
  const n=(await tx.query('SELECT * FROM d1_runs WHERE room_id=$1 ORDER BY started_at DESC,run_id DESC LIMIT 1',[r.id])).rows[0];
  const publicRead=r.visibility==='public'&&r.public_stats;
  const privateRead=actor&&!!(r.host_account_id===actor.accountId||(await tx.query('SELECT 1 FROM g3_access WHERE run_id=$1 AND principal_id=$2 LIMIT 1',[n?.run_id??null,actor.accountId])).rowCount||(await tx.query('SELECT 1 FROM memberships WHERE room_id=$1 AND account_id=$2',[r.id,actor.accountId])).rowCount);
  demand(publicRead||privateRead,'NOT_FOUND');
  if(actor&&r.host_account_id!==actor.accountId)demand(!(await tx.query('SELECT 1 FROM room_bans WHERE room_id=$1 AND account_id=$2',[r.id,actor.accountId])).rowCount,'NOT_FOUND');

  const base:Summary={locator:r.stats_locator,lobbyName:r.name,retentionDays:30,gameReported:true};
  if(!n){const legacy=!!(await tx.query('SELECT 1 FROM runs n JOIN inspected_releases x ON x.run_id=n.id WHERE n.room_id=$1 LIMIT 1',[r.id])).rowCount;return {summary:{...base,collectionState:legacy?'unavailable':'none'}};}
  const expired=n.expires_at&&+n.expires_at<=this.auth.now();
  const summary:Summary={...base,runKey:n.run_key,summaryRevision:String(n.revision),collectionState:expired?'expired':n.complete?'available':'partial',runStatus:n.ended_at?'finished':'live',startedAt:n.started_at.toISOString(),endedAt:n.ended_at?.toISOString()??'',elapsedSeconds:String(Math.max(0,Math.floor(((n.ended_at?+n.ended_at:this.auth.now())-n.started_at)/1000)))};
  if(expired)return {summary};
  let after:any;
  if(p.afterRow){demand(p.runKey===n.run_key&&p.summaryRevision===String(n.revision),'STALE_REVISION');after=(await tx.query('SELECT lower(display_name) AS name,row_key FROM d1_players WHERE run_id=$1 AND row_key=$2',[n.run_id,p.afterRow])).rows[0];demand(after,'INVALID_MESSAGE');}
  const rows=(await tx.query('SELECT * FROM d1_players WHERE run_id=$1 AND ($2::text IS NULL OR (lower(display_name),row_key)>($2,$3::uuid)) ORDER BY lower(display_name),row_key LIMIT $4',[n.run_id,after?.name??null,after?.row_key??null,(p.limit??32)+1])).rows;
  summary.rows=rows.slice(0,p.limit??32).map(row=>({rowKey:row.row_key,displayName:row.display_name,...Object.fromEntries(STAT_COLUMNS.map(c=>[camel(c),String(row[c])]))}));summary.nextRow=rows.length>(p.limit??32)?summary.rows.at(-1)!.rowKey!:'';
  return {summary};
 }
}
export async function registerStatisticsRoutes(app:FastifyInstance,stats:LobbyStatistics,webToken?:((req:FastifyRequest)=>Promise<string|undefined>)|undefined){
 const gate=new WorkGate(8),{db,auth}=stats;
 app.get('/v3/stats/schema',async(_req,reply)=>reply.header('cache-control','no-store').send(schema));
 app.post('/v3/stats/rpc',{bodyLimit:schema.limits.bytes},async(req,reply)=>{
  let input:Request={},release:(()=>void)|undefined,result:Response;const binary=req.headers['content-type']?.split(';')[0]==='application/x-protobuf';
  const base=()=>({major:1,revision:schema.revision,serviceId:db.serviceId,serviceEpoch:db.epoch,requestId:input.requestId??''});
  const controller=new AbortController(),timer=setTimeout(()=>controller.abort(),3000),abort=()=>{if(!reply.raw.writableFinished)controller.abort();};req.raw.once('aborted',abort);reply.raw.once('close',abort);
  try{
   input=(binary?decode('Request',req.body as Buffer):validate('Request',req.body)) as Request;
   demand(input.major===1&&input.revision===schema.revision,'VERSION_MISMATCH');demand(input.serviceId===db.serviceId&&input.serviceEpoch===db.epoch,'STALE_EPOCH');demand(/^[A-Za-z0-9_-]{1,80}$/.test(input.requestId??''),'INVALID_MESSAGE');
   release=gate.enter();const token=req.headers.authorization?.match(/^Bearer ([^ ]{1,4096})$/)?.[1]??await webToken?.(req);
   result=await db.transaction(async tx=>{const actor=token?await auth.authenticate(tx,token):undefined;if(input.observe)demand(actor,'UNAUTHENTICATED');if(actor&&(input.observe||input.sessionEpoch))demand(actor.sessionEpoch===input.sessionEpoch,'STALE_EPOCH');return {...base(),...await stats.execute(tx,input,actor)};},false,controller.signal);
  }catch(e){result={...base(),error:{code:e instanceof PolicyError?e.code:'INVALID_MESSAGE'}};}
  finally{clearTimeout(timer);req.raw.removeListener('aborted',abort);reply.raw.removeListener('close',abort);release?.();}
  reply.header('cache-control','no-store');try{validate('Response',result);demand(Buffer.byteLength(JSON.stringify(result))<=32768,'CAPACITY');return binary?reply.type('application/x-protobuf').send(Buffer.from(encode('Response',result))):result;}catch{return {...base(),error:{code:'CAPACITY'}};}
 });
}
