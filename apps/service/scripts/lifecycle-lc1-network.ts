// Disposable application-transport propagation only; never canonical authority.
import {performance} from 'node:perf_hooks';
export type NetworkTarget='consumer'|'peer';
export type NetworkLane='httpIn'|'httpOut'|'wssIn'|'wssOut';
export type NetworkProfile='rtt80'|'rtt200'|'rtt400'|'c1Fixed250';
export const NETWORK_LIMITS={profiles:2,configurations:64,durationMinMs:1000,durationMaxMs:600000,defaultDurationMs:120000,flows:128,frames:256,bytes:4194304,wssFrames:64,wssBytes:65536,httpFrames:1,httpBytes:262144,maxResidenceMs:4000,samples:64,completedProfiles:8,transitions:16} as const;
export const NETWORK_PROFILES={rtt80:{rttMs:80,oneWayMs:40,jitterMs:8},rtt200:{rttMs:200,oneWayMs:100,jitterMs:20},rtt400:{rttMs:400,oneWayMs:200,jitterMs:40},c1Fixed250:{rttMs:500,oneWayMs:250,jitterMs:0}} as const;
type Clock={now:()=>number;set:(fn:()=>void,ms:number)=>any;clear:(timer:any)=>void};
type Timing={classificationMs:number;rawQueueMs:number};
type Stats={scheduled:number;delivered:number;cancelled:number;overflow:number;late:number;maxQueuedFrames:number;maxQueuedBytes:number;actualDelaySumMs:number;actualDelayMaxMs:number;timerLatenessMaxMs:number;classificationSumMs:number;rawQueueSumMs:number;preScheduleSumMs:number;propagationWaitSumMs:number;samples:{plannedMs:number;actualMs:number;clampedMs:number;classificationMs:number;rawQueueMs:number;preScheduleMs:number;remainingPropagationWaitMs:number;timerLatenessMs:number}[]};
type Profile={name:NetworkProfile;seed:number;expires:number;generation:number;timer:any};
type Entry={arrival:number;ready:number;timing:Timing;due:number;planned:number;bytes:number;deliver:()=>void;cancel:()=>void;resolve:(ok:boolean)=>void;stats:Stats;generation:number};
type Flow={key:object;target:NetworkTarget;lane:NetworkLane;queue:Entry[];bytes:number;lastDue:number;timer:any};
const empty=():Stats=>({scheduled:0,delivered:0,cancelled:0,overflow:0,late:0,maxQueuedFrames:0,maxQueuedBytes:0,actualDelaySumMs:0,actualDelayMaxMs:0,timerLatenessMaxMs:0,classificationSumMs:0,rawQueueSumMs:0,preScheduleSumMs:0,propagationWaitSumMs:0,samples:[]});
const offsets:Record<NetworkLane,number>={httpIn:0,httpOut:1,wssIn:2,wssOut:3};
export class LifecycleNetwork {
 private profiles=new Map<NetworkTarget,Profile>();private flows=new Set<Flow>();private stats=new Map<string,Stats>();private counters=new Map<string,number>();
 private dueHistory=new WeakMap<object,Partial<Record<NetworkLane,number>>>();
 private generations={consumer:0,peer:0};private configurations=0;private pendingFrames=0;private pendingBytes=0;private closed=false;
 private stateRevision=0;private completed:any[]=[];private transitions:any[]=[];private highWater={flows:0,frames:0,bytes:0,timers:0};
 readonly clock:Clock;
 onReset:(target:NetworkTarget,reason:string)=>void=()=>{};
 constructor(clock?:Clock){this.clock=clock??{now:()=>performance.now(),set:(fn,ms)=>{const t=setTimeout(fn,ms);t.unref();return t;},clear:t=>clearTimeout(t)};}
 get enabled(){return this.profiles.size>0;}
 active(target:NetworkTarget){return this.profiles.has(target);}
 generation(target:NetworkTarget){return this.generations[target];}
 private stat(target:NetworkTarget,lane:NetworkLane){const key=target+':'+lane;let value=this.stats.get(key);if(!value){value=empty();this.stats.set(key,value);}return value;}
 configure(target:NetworkTarget,name:NetworkProfile,seed=0,durationMs:number=NETWORK_LIMITS.defaultDurationMs){
  if(this.closed||!['consumer','peer'].includes(target)||typeof name!=='string'||!Object.hasOwn(NETWORK_PROFILES,name)||!Number.isInteger(seed)||seed<0||seed>65535||!Number.isInteger(durationMs)||durationMs<NETWORK_LIMITS.durationMinMs||durationMs>NETWORK_LIMITS.durationMaxMs||this.configurations>=NETWORK_LIMITS.configurations)throw Error('FIXTURE_NETWORK_ARGUMENT');
  if(this.profiles.has(target))this.reset(target,'profile-replaced');
  this.configurations++;const generation=++this.generations[target];for(const lane of Object.keys(offsets) as NetworkLane[]){this.stats.set(target+':'+lane,empty());this.counters.set(target+':'+lane,0);}
  const expires=this.clock.now()+durationMs,timer=this.clock.set(()=>{if(this.profiles.get(target)?.generation===generation)this.reset(target,'profile-expired');},durationMs);
  this.profiles.set(target,{name,seed,expires,generation,timer});this.transition(target,'profile-enabled');this.observeHighWater();return this.status();
 }
 reset(target?:NetworkTarget,reason='profile-reset'){
  if(target!==undefined&&!['consumer','peer'].includes(target))throw Error('FIXTURE_NETWORK_ARGUMENT');
  for(const role of target?[target]:['consumer','peer'] as NetworkTarget[]){const p=this.profiles.get(role);if(p)this.clock.clear(p.timer);this.profiles.delete(role);this.generations[role]++;this.cancel(role,undefined);this.transition(role,reason);if(p){this.completed.push({target:role,profile:p.name,seed:p.seed,generation:p.generation,reason,stateRevision:this.stateRevision,telemetry:this.summaries(role)});if(this.completed.length>NETWORK_LIMITS.completedProfiles)this.completed.shift();}this.onReset(role,reason);}
  return this.status();
 }
 cancel(target:NetworkTarget,kind?:'http'|'wss'){
  for(const flow of [...this.flows])if(flow.target===target&&(!kind||flow.lane.startsWith(kind)))this.drop(flow);
 }
 cancelKey(key:object){this.dueHistory.delete(key);for(const flow of [...this.flows])if(flow.key===key)this.drop(flow);}
 transition(target:NetworkTarget,reason:string){this.stateRevision++;this.transitions.push({target,reason,stateRevision:this.stateRevision});if(this.transitions.length>NETWORK_LIMITS.transitions)this.transitions.shift();}
 private summaries(target:NetworkTarget){return [...this.stats].filter(([key])=>key.startsWith(target+':')).map(([key,s])=>{const sorted=s.samples.map(v=>v.actualMs).sort((a,b)=>a-b);return {lane:key.split(':')[1],scheduled:s.scheduled,delivered:s.delivered,cancelled:s.cancelled,overflow:s.overflow,late:s.late,maxQueuedFrames:s.maxQueuedFrames,maxQueuedBytes:s.maxQueuedBytes,meanActualMs:s.delivered?s.actualDelaySumMs/s.delivered:0,meanClassificationMs:s.delivered?s.classificationSumMs/s.delivered:0,meanRawQueueMs:s.delivered?s.rawQueueSumMs/s.delivered:0,meanRemainingPropagationWaitMs:s.delivered?s.propagationWaitSumMs/s.delivered:0,maxActualMs:s.actualDelayMaxMs,recentP95ActualMs:sorted[Math.floor((sorted.length-1)*.95)]??0};});}
 private observeHighWater(){this.highWater.flows=Math.max(this.highWater.flows,this.flows.size);this.highWater.frames=Math.max(this.highWater.frames,this.pendingFrames);this.highWater.bytes=Math.max(this.highWater.bytes,this.pendingBytes);this.highWater.timers=Math.max(this.highWater.timers,[...this.flows].filter(f=>f.timer!==undefined).length+this.profiles.size);}
 private drop(flow:Flow){
  if(flow.timer!==undefined)this.clock.clear(flow.timer);flow.timer=undefined;this.flows.delete(flow);
  const entries=flow.queue.splice(0);flow.bytes=0;
  for(const e of entries){this.pendingFrames--;this.pendingBytes-=e.bytes;e.stats.cancelled++;try{e.cancel();}finally{e.resolve(false);}}
 }
 enqueue(target:NetworkTarget,lane:NetworkLane,key:object,arrival:number,bytes:number,deliver:()=>void,cancel:()=>void,timing:Timing={classificationMs:0,rawQueueMs:0}){
  const profile=this.profiles.get(target),now=this.clock.now();
  if(!profile){deliver();return {accepted:true,done:Promise.resolve(true)};}
  if(now>=profile.expires){this.reset(target,'profile-expired');cancel();return {accepted:false,done:Promise.resolve(false)};}
  const stats=this.stat(target,lane),existing=[...this.flows].find(f=>f.key===key&&f.lane===lane),wss=lane.startsWith('wss'),maxFrames=wss?NETWORK_LIMITS.wssFrames:NETWORK_LIMITS.httpFrames,maxBytes=wss?NETWORK_LIMITS.wssBytes:NETWORK_LIMITS.httpBytes;
  if(this.closed||!Number.isFinite(arrival)||arrival>now+1||now-arrival>NETWORK_LIMITS.maxResidenceMs||!Number.isSafeInteger(bytes)||bytes<0||bytes>maxBytes||this.pendingFrames>=NETWORK_LIMITS.frames||this.pendingBytes+bytes>NETWORK_LIMITS.bytes||(!existing&&this.flows.size>=NETWORK_LIMITS.flows)||(existing&&(existing.target!==target||existing.queue.length>=maxFrames||existing.bytes+bytes>maxBytes))){stats.overflow++;cancel();return {accepted:false,done:Promise.resolve(false)};}
  const count=this.counters.get(target+':'+lane)??0;this.counters.set(target+':'+lane,count+1);
  const p=NETWORK_PROFILES[profile.name],jitter=[-1,1,0,-.5,.5][(profile.seed+offsets[lane]+count)%5]!,planned=p.oneWayMs+jitter*p.jitterMs;
  const flow=existing??{key,target,lane,queue:[],bytes:0,lastDue:0,timer:undefined};this.flows.add(flow);
  const history=this.dueHistory.get(key)??{},due=Math.max(arrival+planned,flow.lastDue,history[lane]??0);flow.lastDue=due;history[lane]=due;this.dueHistory.set(key,history);
  const done=new Promise<boolean>(resolve=>flow.queue.push({arrival,ready:now,timing:{...timing},due,planned,bytes,deliver,cancel,resolve,stats,generation:profile.generation}));
  flow.bytes+=bytes;this.pendingFrames++;this.pendingBytes+=bytes;stats.scheduled++;stats.maxQueuedFrames=Math.max(stats.maxQueuedFrames,flow.queue.length);stats.maxQueuedBytes=Math.max(stats.maxQueuedBytes,flow.bytes);this.arm(flow);this.observeHighWater();
  return {accepted:true,done};
 }
 private arm(flow:Flow){if(flow.timer!==undefined||!flow.queue.length)return;flow.timer=this.clock.set(()=>{flow.timer=undefined;this.flush(flow);},Math.max(0,flow.queue[0]!.due-this.clock.now()));}
 private flush(flow:Flow){
  while(flow.queue.length&&flow.queue[0]!.due<=this.clock.now()){
   const e=flow.queue.shift()!,now=this.clock.now();flow.bytes-=e.bytes;this.pendingFrames--;this.pendingBytes-=e.bytes;
   const profile=this.profiles.get(flow.target);
   if(!profile||profile.generation!==e.generation||now>=profile.expires||now-e.arrival>NETWORK_LIMITS.maxResidenceMs){e.stats.cancelled++;if(now-e.arrival>NETWORK_LIMITS.maxResidenceMs)e.stats.late++;try{e.cancel();}finally{e.resolve(false);}}
   else{try{e.deliver();const actual=now-e.arrival,remaining=Math.max(0,e.due-e.ready),lateness=now-Math.max(e.due,e.ready);e.stats.delivered++;e.stats.actualDelaySumMs+=actual;e.stats.actualDelayMaxMs=Math.max(e.stats.actualDelayMaxMs,actual);e.stats.classificationSumMs+=e.timing.classificationMs;e.stats.rawQueueSumMs+=e.timing.rawQueueMs;e.stats.preScheduleSumMs+=e.ready-e.arrival;e.stats.propagationWaitSumMs+=remaining;e.stats.timerLatenessMaxMs=Math.max(e.stats.timerLatenessMaxMs,lateness);e.stats.samples.push({plannedMs:e.planned,actualMs:actual,clampedMs:e.due-e.arrival,...e.timing,preScheduleMs:e.ready-e.arrival,remainingPropagationWaitMs:remaining,timerLatenessMs:lateness});if(e.stats.samples.length>NETWORK_LIMITS.samples)e.stats.samples.shift();e.resolve(true);}catch{e.stats.cancelled++;e.cancel();e.resolve(false);}}
  }
  if(flow.queue.length)this.arm(flow);else this.flows.delete(flow);
 }
 status(){return {revision:1,stateRevision:this.stateRevision,semantics:'minimum-application-arrival-to-release-includes-classification',limits:NETWORK_LIMITS,definitions:NETWORK_PROFILES,configurations:this.configurations,profiles:[...this.profiles].map(([target,p])=>({target,profile:p.name,seed:p.seed,generation:p.generation,remainingMs:Math.max(0,Math.ceil(p.expires-this.clock.now()))})),pending:{flows:this.flows.size,frames:this.pendingFrames,bytes:this.pendingBytes,timers:[...this.flows].filter(f=>f.timer!==undefined).length+this.profiles.size},highWater:{...this.highWater},completed:this.completed.map(v=>structuredClone(v)),transitions:this.transitions.map(v=>({...v})),telemetry:[...this.stats].map(([key,s])=>({target:key.split(':')[0],lane:key.split(':')[1],...s,samples:s.samples.map(v=>({...v})),meanActualMs:s.delivered?s.actualDelaySumMs/s.delivered:0,meanClassificationMs:s.delivered?s.classificationSumMs/s.delivered:0,meanRawQueueMs:s.delivered?s.rawQueueSumMs/s.delivered:0,meanRemainingPropagationWaitMs:s.delivered?s.propagationWaitSumMs/s.delivered:0}))};}
 close(){if(this.closed)return;this.closed=true;this.reset(undefined,'fixture-close');}
}
