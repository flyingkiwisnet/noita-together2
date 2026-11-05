import {PolicyError} from './security.js';
export const WORK_LIMITS={requestWork:16,databaseWork:7,acquireMs:500,statementMs:1000,lockMs:500,transactionMs:2500,requestMs:3000,retryMs:250} as const;
export const unavailable=()=>new PolicyError('SERVICE_UNAVAILABLE','SERVICE_UNAVAILABLE',WORK_LIMITS.retryMs);
export const capacity=()=>new PolicyError('CAPACITY','CAPACITY',WORK_LIMITS.retryMs);
// No application wait queue: excess callers receive a retryable error immediately.
export class WorkGate {
 private active=0; private peak=0;private rejected=0;
 constructor(readonly limit:number){if(!Number.isInteger(limit)||limit<1||limit>16)throw Error('Invalid finite work budget');}
 enter(){if(this.active>=this.limit){this.rejected++;throw capacity();}this.active++;this.peak=Math.max(this.peak,this.active);let released=false;return ()=>{if(!released){released=true;this.active--;}};}
 get stats(){return {active:this.active,peak:this.peak,rejected:this.rejected,limit:this.limit};}
}

export type DatabaseWorkClass='control'|'optional';
type ScheduledWork={kind:DatabaseWorkClass;at:number;dispatchedAt?:number;state:'queued'|'executing'|'done';signal:AbortSignal;abort:()=>void;resolve:(release:()=>void)=>void;reject:(error:Error)=>void};
// The database already serializes on one advisory lock. Keep its finite wait
// before pool acquisition, with control reservation and bounded FIFO fairness.
export class DatabaseWorkScheduler {
 private queue:ScheduledWork[]=[];private executing:ScheduledWork|undefined;private closed=false;private count=0;private optional=0;private peak=0;private rejected=0;private consecutiveControl=0;
 readonly reservedControl:number;
 constructor(readonly limit:number){if(!Number.isInteger(limit)||limit<1||limit>7)throw Error('Invalid database scheduler budget');this.reservedControl=Math.min(2,Math.max(0,limit-1));}
 enter(kind:DatabaseWorkClass,signal:AbortSignal):Promise<()=>void>{
  if(this.closed||signal.aborted)return Promise.reject(unavailable());
  if(this.count>=this.limit||kind==='optional'&&this.optional>=this.limit-this.reservedControl){this.rejected++;return Promise.reject(capacity());}
  this.count++;if(kind==='optional')this.optional++;this.peak=Math.max(this.peak,this.count);
  return new Promise((resolve,reject)=>{
   const entry:ScheduledWork={kind,at:performance.now(),state:'queued',signal,resolve,reject,abort:()=>{if(entry.state!=='queued')return;this.queue=this.queue.filter(v=>v!==entry);this.finish(entry);reject(unavailable());this.dispatch();}};
   this.queue.push(entry);signal.addEventListener('abort',entry.abort,{once:true});if(signal.aborted)entry.abort();else this.dispatch();
  });
 }
 private finish(entry:ScheduledWork){if(entry.state==='done')return;entry.signal.removeEventListener('abort',entry.abort);entry.state='done';this.count--;if(entry.kind==='optional')this.optional--;}
 private dispatch(){
  if(this.closed||this.executing||!this.queue.length)return;
  const control=this.queue.findIndex(v=>v.kind==='control'),optional=this.queue.findIndex(v=>v.kind==='optional'),index=control>=0&&(optional<0||this.consecutiveControl<3)?control:optional;
  const entry=this.queue.splice(index,1)[0]!;entry.signal.removeEventListener('abort',entry.abort);
  if(entry.signal.aborted){this.finish(entry);entry.reject(unavailable());this.dispatch();return;}
  entry.state='executing';entry.dispatchedAt=performance.now();this.executing=entry;this.consecutiveControl=entry.kind==='control'?Math.min(3,this.consecutiveControl+1):0;
  let released=false;entry.resolve(()=>{if(released)return;released=true;this.finish(entry);if(this.executing===entry)this.executing=undefined;this.dispatch();});
 }
 close(){if(this.closed)return;this.closed=true;for(const entry of this.queue.splice(0)){this.finish(entry);entry.reject(unavailable());}}
 get stats(){const now=performance.now(),age=(kind:DatabaseWorkClass)=>Math.max(0,...this.queue.filter(v=>v.kind===kind).map(v=>Math.floor(now-v.at)));return{active:this.count,peak:this.peak,rejected:this.rejected,limit:this.limit,reservedControl:this.reservedControl,optionalLimit:this.limit-this.reservedControl,optionalActive:this.optional,queued:this.queue.length,queuedControl:this.queue.filter(v=>v.kind==='control').length,queuedOptional:this.queue.filter(v=>v.kind==='optional').length,oldestControlMs:age('control'),oldestOptionalMs:age('optional'),executing:this.executing?.kind??'none',executingAgeMs:this.executing?Math.max(0,Math.floor(now-this.executing.dispatchedAt!)):0};}
}
export function waitFor<T>(promise:Promise<T>,signal:AbortSignal):Promise<T>{
 if(signal.aborted){void promise.catch(()=>{});return Promise.reject(unavailable());}
 return new Promise((resolve,reject)=>{
  const abort=()=>{cleanup();reject(unavailable());};const cleanup=()=>signal.removeEventListener('abort',abort);
  signal.addEventListener('abort',abort,{once:true});promise.then(value=>{cleanup();resolve(value);},error=>{cleanup();reject(error);});
 });
}
