// Private LC1 fixture controls only. No production route or synthetic success.
import {randomUUID} from 'node:crypto';
import {SocialDiagnostics} from './lifecycle-lc1-social-diagnostics.js';
import {decode,validate} from '../../../packages/contracts/src/g3-social.js';
import {fingerprint,PolicyError} from '../src/g1/security.js';
import type {startG3Fixture} from './g3-loot01-fixture.js';
import {SocialRatePrecondition} from './lc1-social-rate-precondition.js';
type Service=Awaited<ReturnType<typeof startG3Fixture>>['service'];
type Target='consumer'|'peer';
type Operation='publishPoi'|'publishEmote'|'snapshot'|'roster'|'poiSnapshot'|'poiOutbox'|'poiOutcome';
export type SocialFaultPlan={target:Target;worldId:string;operation:Operation;eventId?:string;stopOnly?:boolean;phase:'before'|'reply';effect:'delay'|'drop';delayMs?:number};
type Arm={id:string;plan:SocialFaultPlan;matched:boolean;cancelled:boolean;matchedEventId?:string;release?:()=>void};
const uuid=(v:unknown)=>typeof v==='string'&&/^[a-f0-9]{8}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{12}$/i.test(v);
const fail=()=>{throw new PolicyError('FIXTURE_SOCIAL_ARGUMENT');};
export class LifecycleSocialControls{
 private diagnostics=new SocialDiagnostics();
 private service!:Service;private owner!:(target:Target,actor:{accountId:string;deviceId:string},roomId:string)=>boolean;
 private active:Arm|undefined;private arms=0;private events:any[]=[];private matched=new WeakMap<object,Arm>();private closed=false;private timers=0;
 private seededIds=new Set<string>();private seededRuns=new Set<string>();
 private rate:SocialRatePrecondition|undefined;
 armRate(value:unknown){if(this.closed||!this.rate)fail();return this.rate!.arm(value);}
 private event(a:Arm,kind:string,extra:any={}){this.events.push({armId:a.id,kind,...a.plan,...extra});if(this.events.length>64)this.events.shift();}
 arm(value:unknown){
  if(!value||typeof value!=='object'||Array.isArray(value))fail();const p=value as SocialFaultPlan;
  if(Object.keys(p).some(k=>!['target','worldId','operation','eventId','stopOnly','phase','effect','delayMs'].includes(k))||!['consumer','peer'].includes(p.target)||!uuid(p.worldId)||!['publishPoi','publishEmote','snapshot','roster','poiSnapshot','poiOutbox','poiOutcome'].includes(p.operation)||!['before','reply'].includes(p.phase)||p.stopOnly!==undefined&&(p.operation!=='publishEmote'||p.stopOnly!==true))fail();
  if(p.operation==='publishPoi'){if(p.phase!=='reply'||p.effect!=='drop'||p.delayMs!==undefined||!uuid(p.eventId))fail();}
  else if(p.operation==='publishEmote'){if(p.phase!=='reply'||p.effect!=='drop'||p.delayMs!==undefined||(p.eventId!==undefined?!uuid(p.eventId):p.stopOnly!==true))fail();}
  else{if(p.effect!=='delay'||!Number.isInteger(p.delayMs)||p.delayMs!<1||p.delayMs!>10000||(p.operation==='poiOutcome'?!uuid(p.eventId):p.eventId!==undefined))fail();}
  if(this.closed||this.active||this.arms>=32)throw Error('FIXTURE_SOCIAL_BUSY');
  const a:Arm={id:randomUUID(),plan:structuredClone(p),matched:false,cancelled:false};this.arms++;this.active=a;this.event(a,'armed');return {armId:a.id};
 }
 status(){return {revision:1,armsUsed:this.arms,armLimit:32,timers:this.timers,active:this.active?{armId:this.active.id,...this.active.plan,matched:this.active.matched,...(this.active.matchedEventId?{matchedEventId:this.active.matchedEventId}:{})}:null,events:structuredClone(this.events),requests:this.diagnostics.snapshot(),...(this.rate?{ratePrecondition:this.rate.status()}:{})};}
 release(){const a=this.active;if(a){a.cancelled=true;this.event(a,'released');a.release?.();this.active=undefined;}this.rate?.release();return {released:true};}
 close(){this.closed=true;this.release();this.rate?.close();}
 private finish(a:Arm){if(this.active===a)this.active=undefined;}
 private refusedReply(a:Arm){if(a.plan.operation==='publishEmote'){a.matched=false;delete a.matchedEventId;}else this.finish(a);}
 private async delay(a:Arm,reply:any){
  this.event(a,a.plan.phase+'-delay');this.timers++;
  try{await new Promise<void>(resolve=>{const done=()=>{clearTimeout(timer);reply.raw.removeListener('close',done);if(a.release===done)delete a.release;resolve();};const timer=setTimeout(done,a.plan.delayMs);a.release=done;reply.raw.once('close',done);});}
  finally{this.timers--;}
 }
 install(service:Service,owner:typeof this.owner){
  this.service=service;this.owner=owner;service.app.addHook('preClose',async()=>this.close());
  this.rate=new SocialRatePrecondition(service,owner);this.rate.install();
  this.diagnostics.install(service);
  service.app.addHook('preHandler',async(req,reply)=>{
   const a=this.active;if(req.url!=='/v3/social/rpc'||!a||a.matched||a.cancelled)return;
   let input:any;try{input=Buffer.isBuffer(req.body)?decode('Request',req.body):validate('Request',req.body);}catch{return;}
   const p=a.plan,operation=input.publish?.poi?'publishPoi':input.publish?.emote?'publishEmote':(['snapshot','roster','poiSnapshot','poiOutbox','poiOutcome'] as const).find(k=>input[k]!==undefined);
   if(operation!==p.operation||input.scope?.worldToken!==p.worldId||(p.eventId&&(input.publish?.eventId??input.poiOutcome?.eventId)!==p.eventId))return;
   if(operation==='publishEmote'&&(!uuid(input.publish.eventId)||p.stopOnly&&(input.publish.emote.active??false)!==false))return;
   const token=req.headers.authorization?.match(/^Bearer ([^ ]{1,4096})$/)?.[1];if(!token)return;
   let verified=false;try{verified=await service.db.transaction(async tx=>{const actor=await service.auth.authenticate(tx,token);
    if(actor.kind!=='game'||input.serviceId!==service.db.serviceId||input.serviceEpoch!==service.db.epoch||input.sessionEpoch!==actor.sessionEpoch||!owner(p.target,actor,input.scope.roomId))return false;
    return !!(await tx.query('SELECT 1 FROM world_associations w JOIN attachments a ON a.id=w.attachment_id JOIN runs n ON n.id=w.run_id WHERE w.world_id=$1 AND w.account_id=$2 AND w.device_id=$3 AND a.game_session_id=$4 AND a.id=$5 AND n.id=$6 AND n.epoch=$7 AND n.config_revision=$8 AND w.terminal_reason IS NULL',[p.worldId,actor.accountId,actor.deviceId,actor.sessionId,input.scope.attachmentId,input.scope.runId,input.scope.runEpoch,input.scope.configRevision])).rowCount;
   },true,AbortSignal.timeout(2500));}catch{return;}
   if(!verified||this.active!==a||a.cancelled||a.matched)return;a.matched=true;if(operation==='publishEmote')a.matchedEventId=input.publish.eventId;this.matched.set(req,a);this.event(a,'matched',{requestId:input.requestId,...(a.matchedEventId?{eventId:a.matchedEventId}:{})});
   if(p.phase==='before'){await this.delay(a,reply);this.event(a,'before-released');this.finish(a);}
  });
  service.app.addHook('onSend',async(req,reply,payload)=>{
   const a=this.matched.get(req);if(!a||a.cancelled||a.plan.phase!=='reply')return payload;
   this.matched.delete(req);
   let result:any;try{result=req.headers['content-type']?.split(';')[0]==='application/x-protobuf'?decode('Response',Buffer.from(payload as Buffer)):validate('Response',JSON.parse(String(payload)));}catch{this.event(a,'unreadable-reply');this.refusedReply(a);return payload;}
   if(result.error){this.event(a,'handler-refused');this.refusedReply(a);return payload;}
   if(a.plan.operation==='publishPoi'||a.plan.operation==='publishEmote'){
    const eventId=a.plan.operation==='publishEmote'?a.matchedEventId:a.plan.eventId;
    if(result.acceptedEventId!==eventId){this.event(a,'unmatched-reply');this.refusedReply(a);return payload;}
    this.event(a,'committed-reply-drop',{eventId,duplicate:result.duplicate??false});this.finish(a);reply.hijack();reply.raw.destroy();return null;
   }
   await this.delay(a,reply);this.event(a,'reply-released');this.finish(a);return payload;
  });
 }
 async seed(target:Target,eventId:string,total:number,roomId:string){
  if(!['consumer','peer'].includes(target)||!uuid(eventId)||!Number.isInteger(total)||total<1||total>256||this.seededIds.has(eventId))fail();
  const service=this.service;if(!/^g3_rejoin_[a-f0-9]{32}$/.test(service.db.namespace)||this.closed)fail();
  const result=await service.db.transaction(async tx=>{
   const identity=(await tx.query('SELECT current_database() AS database,current_user AS owner,current_schema() AS schema')).rows[0];if(identity.database!=='nt2_g1'||identity.owner!=='nt2_g1'||identity.schema!==service.db.namespace)fail();
   const r=(await tx.query('SELECT current_run_id,state FROM rooms WHERE id=$1',[roomId])).rows[0];if(r?.state!=='running')fail();const runId=r.current_run_id;
   if(!this.seededRuns.has(runId)&&this.seededRuns.size>=32)fail();
   const source=(await tx.query('SELECT p.* FROM g3_social_pois p WHERE run_id=$1 AND event_id=$2',[runId,eventId])).rows[0];if(!source||!Number.isInteger(source.body.operationSequence))fail();
   const w=(await tx.query('SELECT account_id,device_id FROM world_associations WHERE world_id=$1 AND run_id=$2',[source.body.scope.worldToken,runId])).rows[0];if(!w||!this.owner(target,{accountId:w.account_id,deviceId:w.device_id},roomId)||w.account_id!==source.body.principalId)fail();
   const rows=(await tx.query('SELECT sequence,body FROM g3_social_pois WHERE run_id=$1 ORDER BY sequence',[runId])).rows;if(rows.length>total||source.body.operationSequence+total>0xffffffff)fail();
   const needed=total-rows.length,positions=rows.map(p=>({x:Number(p.body.poi.xMilli),y:Number(p.body.poi.yMilli)})),sourceX=Number(source.body.poi.xMilli),y=Number(source.body.poi.yMilli),direction=sourceX>=0?-1:1,createdAt=new Date(service.auth.now());let first:string|undefined,last:string|undefined;const ids:string[]=[];
   for(let slot=1;ids.length<needed&&slot<=1024;slot++){
    const x=sourceX+direction*slot*100000;if(Math.abs(x)>2147483647||positions.some(p=>Math.hypot(x-p.x,y-p.y)<50000))continue;
    const id=randomUUID(),sequence=source.body.operationSequence+ids.length+1,poi={...source.body.poi,xMilli:String(x)},body={...source.body,eventId:id,operationSequence:sequence,poi,capturedAt:createdAt.toISOString()},intent={catalogId:'world.poi',text:poi.text,xMilli:x,yMilli:y};
    const digest=fingerprint({scope:body.scope,publish:{eventId:id,sequence,intent}});
    await tx.query('INSERT INTO g3_social_ids VALUES($1,$2,$3,$4,$5)',[runId,source.game_session_id,id,digest,createdAt]);
    const row=(await tx.query('INSERT INTO g3_social_pois(run_id,game_session_id,event_id,body,recipient_sessions,created_at) VALUES($1,$2,$3,$4,$5,$6) RETURNING sequence',[runId,source.game_session_id,id,body,source.recipient_sessions,createdAt])).rows[0];first??=String(row.sequence);last=String(row.sequence);ids.push(id);positions.push({x,y});
   }
   if(ids.length!==needed)throw new PolicyError('FIXTURE_SOCIAL_SEED_BOUNDS');return {runId,sourceEventId:eventId,alreadyPresent:rows.length,seededNow:ids.length,total,firstSeedSequence:first??null,lastSeedSequence:last??null,ids};
  });
  this.seededRuns.add(result.runId);for(const id of result.ids)this.seededIds.add(id);const {ids:_,...receipt}=result;return {...receipt,label:'Existing retained originals plus explicit fixture SQL seeds; seededNow is not an actual publish count'};
 }
}
