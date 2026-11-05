import {AsyncLocalStorage} from 'node:async_hooks';
import {createHash,randomUUID} from 'node:crypto';
import {performance} from 'node:perf_hooks';
import {homedir} from 'node:os';
import {join,resolve,sep,dirname} from 'node:path';
import {mkdir,readdir,writeFile,appendFile,rename,rm,rmdir,lstat,open} from 'node:fs/promises';
import type {Database,TransactionDiagnostic} from '../g1/database.js';

export const DEBUG_SCHEMA='nt2-server-debug-1';
export const DEBUG_LIMITS={eventsBytes:2*1024*1024,firstLossBytes:384*1024,sessions:8,ring:128,queue:256,after:64,critical:32};
export function localDiagnosticBase(){return join(process.env.LOCALAPPDATA??join(homedir(),'.local','share'),'NoitaTogether2');}
export const diagnosticTag=(value:unknown)=>typeof value==='string'&&value.length<=4096?createHash('sha256').update(value).digest('hex').slice(0,16):undefined;
export const diagnosticServiceTag=(serviceId:string,serviceEpoch:string)=>diagnosticTag(serviceId+'\n'+serviceEpoch)!;
export interface DebugOptions {serviceTag?:string;enabled?:boolean;root?:string;settingsPath?:string;environment?:string;production?:boolean;limits?:Partial<typeof DEBUG_LIMITS>}
// Explicit opt-outs win, including an old local logging preference. No source edit
// is needed: remove the opt-outs and set debugDiagnostics=true to re-enable.
export async function diagnosticEnabled(options:DebugOptions={}){
 let file:any;try{file=await boundedJson(options.settingsPath??join(localDiagnosticBase(),'debug-settings.json'),4096);}catch{}
 const env=(options.environment??process.env.NT2_DEBUG_DIAGNOSTICS??'').toLowerCase();
 if(options.enabled===false||file?.debugDiagnostics===false||['0','off','false'].includes(env))return false;
 return options.enabled===true||file?.debugDiagnostics===true||['1','on','true'].includes(env)||!(options.production??process.env.NODE_ENV==='production');
}
const kinds=new Set(['service_start','service_stop','pose_accepted','sample_removed','snapshot_empty','normal_departure','recovery','authenticate_begin','renew_begin','authenticate_result','renew_result','retirement','db_result']);
const numbers=new Set(['sampleAgeMs','sampleGeneration','memberGeneration','memberRemainingMs','poseSequence','lastAcceptedPoseSequence','peerCount','eligiblePeers','poses','authorityRemainingMs','accessRemainingMs','operationId']);
const booleans=new Set(['terminal','normalEnd','eligible','retired','busy','samplePresent','protected','authorized','canonicalTerminal','worldPresent']);
const tags=new Set(['connectionTag','sessionTag','roomTag']);
const values:Record<string,Set<string>>={admissionReason:new Set(['owner_binding_changed','inspection_required','report_invalid','initial_review_expired','lease_expired','entry_required','other']),reason:new Set(['pose_ttl','member_unavailable','terminal','member_removed','generation_changed','eligible_peer_sample_absent','ordinary_departure','peer_sample_returned','shutdown']),outcome:new Set(['accepted','refused','retired-other']),stage:new Set(['admission','db','post_commit','renew_bounds','complete']),code:new Set(['CAPACITY','STALE_EPOCH','EXPIRED','FORBIDDEN','RETIRED','INVALID_MESSAGE','UNAUTHENTICATED','SERVICE_UNAVAILABLE','COMMIT']),phase:new Set(['admission','begin','lock','epoch','work','projection','commit','publish']),category:new Set(['service-admission','transaction','deadline','committed']),commit:new Set(['not-attempted','uncertain','confirmed']),abortCause:new Set(['client-error','transaction-deadline','caller-abort'])};
function clean(input:Record<string,unknown>){const out:Record<string,unknown>={};for(const [key,value]of Object.entries(input)){
 if(numbers.has(key)&&typeof value==='number'&&Number.isFinite(value)&&value>=0)out[key]=Math.min(4294967295,Math.floor(value));
 else if(booleans.has(key)&&typeof value==='boolean')out[key]=value;
 else if(tags.has(key)&&typeof value==='string'&&/^[a-f0-9]{16}$/.test(value))out[key]=value;
 else if(typeof value==='string'&&values[key]?.has(value))out[key]=value;
 else if(key==='sqlState'&&typeof value==='string'&&/^[0-9A-Z]{5}$/.test(value))out[key]=value;
 }return out;}
type Row=Record<string,unknown>;
// Export the same primitive/enum allowlist used by the writer, so collectors do
// not have to infer privacy validation from field names or accept arbitrary JSON.
export function serverDebugSchema(){
 const integer=(maximum=4294967295)=>({type:'integer',minimum:0,maximum});
 const tag={type:'string',pattern:'^[a-f0-9]{16}$'},runtime={type:'string',pattern:'^[a-z0-9][a-z0-9._-]{0,95}$'},uuid={type:'string',pattern:'^[a-f0-9]{8}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{12}$'};
 const utc={type:'string',pattern:'^\\d{4}-\\d{2}-\\d{2}T\\d{2}:\\d{2}:\\d{2}\\.\\d{3}Z$'};
 const identity={schema:{const:DEBUG_SCHEMA},runtimeId:runtime,serviceTag:{anyOf:[tag,{const:'unclassified'}]},serverSession:uuid};
 const eventProperties:Record<string,unknown>={...identity,sequence:integer(Number.MAX_SAFE_INTEGER),utc,monoMs:integer(Number.MAX_SAFE_INTEGER),serviceTimeMs:integer(Number.MAX_SAFE_INTEGER),kind:{enum:[...kinds]}};
 for(const key of numbers)eventProperties[key]=integer();for(const key of booleans)eventProperties[key]={type:'boolean'};for(const key of tags)eventProperties[key]=tag;for(const [key,allowed]of Object.entries(values))eventProperties[key]={enum:[...allowed]};
 eventProperties.sqlState={type:'string',pattern:'^[0-9A-Z]{5}$'};
 const event={type:'object',additionalProperties:false,required:[...Object.keys(identity),'sequence','utc','monoMs','serviceTimeMs','kind'],properties:eventProperties};
 const limits={type:'object',additionalProperties:false,required:Object.keys(DEBUG_LIMITS),properties:Object.fromEntries(Object.entries(DEBUG_LIMITS).map(([key,max])=>[key,{type:'integer',minimum:1,maximum:max}]))};
 const manifestProperties={...identity,component:{const:'server'},pid:integer(),startedAt:utc,updatedAt:utc,closed:{type:'boolean'},debugEnabled:{const:true},firstLoss:{type:'boolean'},files:{type:'object',additionalProperties:false,required:['events','firstLoss'],properties:{events:{const:'events.jsonl'},firstLoss:{enum:['first-loss.json',null]}}},eventsBytes:integer(DEBUG_LIMITS.eventsBytes),dropped:integer(Number.MAX_SAFE_INTEGER),truncated:{type:'boolean'},errors:integer(Number.MAX_SAFE_INTEGER),limits};
 const manifest={type:'object',additionalProperties:false,required:Object.keys(manifestProperties),properties:manifestProperties};
 const firstLoss={type:'object',additionalProperties:false,required:[...Object.keys(identity),'trigger','before','after','truncatedContext'],properties:{...identity,trigger:{allOf:[event,{properties:{kind:{const:'snapshot_empty'}}}]},before:{type:'array',maxItems:DEBUG_LIMITS.ring,items:event},after:{type:'array',maxItems:DEBUG_LIMITS.after,items:event},recovery:{allOf:[event,{properties:{kind:{const:'recovery'}}}]},truncatedContext:{type:'boolean'}}};
 return {$schema:'https://json-schema.org/draft/2020-12/schema',$id:'nt2-server-debug-1',description:'Safe server export validation; poses is a numeric count. sequence is event order; poseSequence/lastAcceptedPoseSequence are pose counters.',$defs:{event,manifest,firstLoss},oneOf:[{$ref:'#/$defs/manifest'},{$ref:'#/$defs/firstLoss'},{$ref:'#/$defs/event'}]};
}
type Context={connectionTag?:string;sessionTag?:string;roomTag?:string;operationId:number;transactionResult?:TransactionDiagnostic;transactionPersisted?:boolean};
const context=new AsyncLocalStorage<Context>();
async function boundedJson(path:string,maximum:number){
 const stat=await lstat(path);if(!stat.isFile()||stat.isSymbolicLink()||stat.size>maximum)throw Error('Invalid diagnostic metadata');
 const file=await open(path,'r');try{const buffer=Buffer.alloc(maximum+1),result=await file.read(buffer,0,buffer.length,0);if(result.bytesRead>maximum)throw Error('Diagnostic metadata grew');return JSON.parse(buffer.subarray(0,result.bytesRead).toString());}finally{await file.close();}
}
async function retentionLock(root:string){
 const path=join(root,'.retention.lock'),nonce=randomUUID();
 for(let attempt=0;attempt<5;attempt++){
  try{const file=await open(path,'wx',0o600);try{await file.writeFile(JSON.stringify({pid:process.pid,nonce}));}finally{await file.close();}return async()=>{try{if((await boundedJson(path,256)).nonce===nonce)await rm(path);}catch{}};}catch(e){if((e as NodeJS.ErrnoException).code!=='EEXIST')throw e;}
  try{const old=await boundedJson(path,256);if(Number.isInteger(old.pid)&&old.pid>0){let alive=true;try{process.kill(old.pid,0);}catch{alive=false;}if(!alive)await rm(path);}}catch{}
  await new Promise(r=>setTimeout(r,20));
 }
 throw Error('Diagnostic retention busy');
}
export class ServerDebugDiagnostics {
 readonly sessionId=randomUUID();readonly startedAt=new Date().toISOString();readonly began=performance.now();
 readonly serviceTag:string;readonly folder:string;readonly limits:typeof DEBUG_LIMITS;
 private lowPriority=new WeakSet<Row>();private ring:Row[]=[];private critical:Row[]=[];private queue:string[]=[];private sequence=0;private operation=0;private first:undefined|{trigger:Row;before:Row[];after:Row[];recovery?:Row};
 private firstDirty=false;private scheduled=false;private dirty=false;private draining:Promise<void>|undefined;private ready:Promise<void>;
 private bytes=0;private dropped=0;private errors=0;private truncated=false;private failed=false;private closed=false;private storageReady=false;private unbind:()=>void=()=>{};
 private snapshots=new WeakMap<object,{visible:Set<string>;lost:boolean}>();
 constructor(readonly runtimeId:string,readonly enabled:boolean,readonly now:()=>number,options:DebugOptions={}){
  this.serviceTag=/^[a-f0-9]{16}$/.test(options.serviceTag??'')?options.serviceTag!:'unclassified';
  this.limits={...DEBUG_LIMITS};for(const key of Object.keys(DEBUG_LIMITS) as (keyof typeof DEBUG_LIMITS)[]){const value=options.limits?.[key];if(value!==undefined&&Number.isInteger(value)&&value>0)this.limits[key]=Math.min(value,DEBUG_LIMITS[key]);}
  const safeId=/^[a-z0-9][a-z0-9._-]{0,95}$/.test(runtimeId)?runtimeId:'unclassified';
  this.runtimeId=safeId;
  this.folder=join(options.root??join(localDiagnosticBase(),'diagnostics','server'),safeId,this.sessionId);
  this.ready=enabled?this.initialize().catch(()=>{this.failed=true;this.errors++;}):Promise.resolve();
  if(enabled)this.record('service_start',{});
 }
 private async initialize(){
  const root=resolve(this.folder,'../..');await mkdir(root,{recursive:true});
  if((await lstat(root)).isSymbolicLink())throw Error('Diagnostic root link');
  const unlock=await retentionLock(root);try{
  const sessions:{path:string;at:number;active:boolean}[]=[];
  for(const runtime of (await readdir(root,{withFileTypes:true})).slice(0,64))if(runtime.isDirectory()&&!runtime.isSymbolicLink()&&/^[a-z0-9][a-z0-9._-]{0,95}$/.test(runtime.name)){
   for(const entry of (await readdir(join(root,runtime.name),{withFileTypes:true})).slice(0,64))if(entry.isDirectory()&&!entry.isSymbolicLink()&&/^[a-f0-9-]{36}$/.test(entry.name)){
    const path=join(root,runtime.name,entry.name);let manifest:any;try{manifest=await boundedJson(join(path,'manifest.json'),16384);}catch{}
    let active=false;if(manifest?.closed===false&&Number.isInteger(manifest.pid)&&manifest.pid>0){try{process.kill(manifest.pid,0);active=true;}catch{}}
    sessions.push({path,at:Date.parse(manifest?.startedAt??'')||0,active});
   }
  }
  sessions.sort((a,b)=>a.at-b.at);let count=sessions.length;
  for(const old of sessions){if(count<this.limits.sessions)break;if(old.active)continue;const target=resolve(old.path);if(!target.startsWith(root+sep))throw Error('Diagnostic retention outside root');await rm(target,{recursive:true,force:true});try{await rmdir(dirname(target));}catch{}count--;}
  if(count>=this.limits.sessions)throw Error('All retained diagnostic sessions active');
  await mkdir(this.folder,{recursive:true});this.storageReady=true;await this.manifest();
  }finally{await unlock();}
 }
 get status(){return {pendingWriterJobs:this.draining?1:0,writerScheduled:this.scheduled,pendingEvents:this.queue.length,enabled:this.enabled,storageReady:this.storageReady&&!this.failed,errors:this.errors,dropped:this.dropped,truncated:this.truncated,sessionId:this.sessionId,runtimeId:this.runtimeId,serviceTag:this.serviceTag};}
 record(kind:string,input:Record<string,unknown>,persist=true){
  if(!this.enabled||this.closed||!kinds.has(kind))return;
  try{
   const row:Row={schema:DEBUG_SCHEMA,runtimeId:this.runtimeId,serviceTag:this.serviceTag,serverSession:this.sessionId,sequence:++this.sequence,utc:new Date().toISOString(),monoMs:Math.max(0,Math.floor(performance.now()-this.began)),serviceTimeMs:Math.max(0,Math.floor(this.now())),kind,...clean(input)};
   if(!persist)this.lowPriority.add(row);
   this.ring.push(row);if(this.ring.length>this.limits.ring)this.ring.shift();
   if(persist){this.critical.push(row);if(this.critical.length>this.limits.critical)this.critical.shift();}
   if(kind==='snapshot_empty'&&!this.first){const prior=this.critical.filter(r=>r.sequence!==row.sequence),recent=this.ring.slice(-(Math.max(1,this.limits.ring-prior.length)));const unique=new Map([...prior,...recent].filter(r=>r.sequence!==row.sequence).map(r=>[r.sequence,r]));this.first={trigger:row,before:[...unique.values()].sort((a,b)=>Number(a.sequence)-Number(b.sequence)).slice(-this.limits.ring),after:[]};this.firstDirty=true;}
   else if(this.first){if(this.first.after.length<this.limits.after){this.first.after.push(row);this.firstDirty=true;}else if(persist){const pose=this.first.after.findIndex(r=>this.lowPriority.has(r));if(pose>=0){this.first.after.splice(pose,1);this.first.after.push(row);this.firstDirty=true;}}}
   if(kind==='recovery'&&this.first&&!this.first.recovery&&row.connectionTag===this.first.trigger.connectionTag){this.first.recovery=row;this.firstDirty=true;}
   if(persist){if(this.queue.length<this.limits.queue)this.queue.push(JSON.stringify(row)+'\n');else this.dropped++;}
   if(persist||this.firstDirty)this.schedule();
  }catch{this.errors++;}
 }
 private schedule(){
  if(this.failed)return;this.dirty=true;if(this.scheduled||this.draining)return;this.scheduled=true;
  setImmediate(()=>{this.scheduled=false;if(!this.failed)this.draining=this.drain();});
 }
 private async drain(){
  try{while(!this.failed&&(this.dirty||this.queue.length||this.firstDirty)){this.dirty=false;await this.write();}}
  catch{this.failed=true;this.errors++;}
  finally{if(this.failed){this.queue=[];this.firstDirty=false;this.dirty=false;}this.draining=undefined;}
 }
 private async write(){
  await this.ready;if(this.failed)return;
  const rows=this.queue.splice(0);let chunk='';for(const row of rows){const size=Buffer.byteLength(row);if(this.bytes+Buffer.byteLength(chunk)+size>this.limits.eventsBytes){this.truncated=true;this.dropped++;continue;}chunk+=row;}
  if(chunk){await appendFile(join(this.folder,'events.jsonl'),chunk,{mode:0o600});this.bytes+=Buffer.byteLength(chunk);}
  if(this.firstDirty&&this.first){this.firstDirty=false;const payload={schema:DEBUG_SCHEMA,runtimeId:this.runtimeId,serviceTag:this.serviceTag,serverSession:this.sessionId,...this.first,before:[...this.first.before],after:[...this.first.after],truncatedContext:false};let text=JSON.stringify(payload)+'\n';while(Buffer.byteLength(text)>this.limits.firstLossBytes&&(payload.before.length||payload.after.length)){if(payload.before.length)payload.before.shift();else payload.after.pop();payload.truncatedContext=true;text=JSON.stringify(payload)+'\n';}if(Buffer.byteLength(text)<=this.limits.firstLossBytes)await this.atomic('first-loss.json',text);else this.truncated=true;}
  await this.manifest();
 }
 private async atomic(name:string,text:string){const path=join(this.folder,name);await writeFile(path+'.tmp',text,{mode:0o600});await rename(path+'.tmp',path);}
 private async manifest(){await this.atomic('manifest.json',JSON.stringify({schema:DEBUG_SCHEMA,component:'server',runtimeId:this.runtimeId,serviceTag:this.serviceTag,serverSession:this.sessionId,pid:process.pid,startedAt:this.startedAt,updatedAt:new Date().toISOString(),closed:this.closed,debugEnabled:this.enabled,firstLoss:!!this.first,files:{events:'events.jsonl',firstLoss:this.first?'first-loss.json':null},eventsBytes:this.bytes,dropped:this.dropped,truncated:this.truncated,errors:this.errors,limits:this.limits})+'\n');}
 async flush(){if(!this.enabled)return;await this.ready;if(this.failed)return;if(this.dirty||this.queue.length||this.firstDirty)this.schedule();await new Promise<void>(r=>setImmediate(r));while(this.draining||this.scheduled){if(this.draining)await this.draining;else await new Promise<void>(r=>setImmediate(r));}}
 async close(){if(this.closed)return;this.record('service_stop',{reason:'shutdown'});this.closed=true;this.unbind();await this.flush();}
 bindDatabase(db:Database){
  if(!this.enabled)return;const prior=db.transactionDiagnostic,original=db.transaction;
  const observer=(event:TransactionDiagnostic)=>{try{prior?.(event);}finally{const scope=context.getStore();if(scope){const persist=!!scope.connectionTag||event.category!=='committed';scope.transactionResult=event;scope.transactionPersisted=persist;this.record('db_result',{...scope,...event},persist);}}};
  const wrapped=((...args:Parameters<Database['transaction']>)=>context.getStore()?original.apply(db,args):this.transaction({},()=>original.apply(db,args))) as Database['transaction'];
  db.transactionDiagnostic=observer;db.transaction=wrapped;
  this.unbind=()=>{if(db.transactionDiagnostic===observer)db.transactionDiagnostic=prior;if(db.transaction===wrapped)db.transaction=original;};
 }
 async transaction<T>(data:Record<string,unknown>,fn:()=>Promise<T>):Promise<T>{
  if(!this.enabled)return fn();return context.run({...clean(data),operationId:++this.operation} as Context,fn);
 }
 operationEvent(kind:string,input:Record<string,unknown>){const scope=context.getStore();if(scope&&['sample_removed','retirement'].includes(kind)&&scope.transactionResult&&!scope.transactionPersisted){scope.transactionPersisted=true;this.record('db_result',{...scope,...scope.transactionResult});}this.record(kind,{...scope,...input});}
 snapshot(connection:object,wireSessions:string[],eligibleSessions:string[],input:Record<string,unknown>){
  if(!this.enabled)return;try{
   const prior=this.snapshots.get(connection),visible=new Set(wireSessions.map(diagnosticTag).filter((v):v is string=>!!v));
   if(visible.size){if(prior?.lost)this.record('recovery',{...input,reason:'peer_sample_returned',peerCount:visible.size});this.snapshots.set(connection,{visible,lost:false});return;}
   if(!prior?.visible.size||prior.lost)return;
   const eligible=new Set(eligibleSessions.map(diagnosticTag));const unexpected=[...prior.visible].some(tag=>eligible.has(tag));
   this.record(unexpected?'snapshot_empty':'normal_departure',{...input,reason:unexpected?'eligible_peer_sample_absent':'ordinary_departure',peerCount:0,eligiblePeers:eligible.size});
   this.snapshots.set(connection,{visible:unexpected?prior.visible:new Set(),lost:unexpected});
  }catch{this.errors++;}
 }
}
