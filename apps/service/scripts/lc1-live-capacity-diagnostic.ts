// Test-only bounded guard attribution. No identifiers, payloads or credentials.
import {performance} from 'node:perf_hooks';
import {AsyncLocalStorage} from 'node:async_hooks';
import {LifecycleCapacityDiagnostics} from './lc1-lifecycle-capacity-diagnostic.js';
import {traceLiveRoster} from './lc1-live-roster-diagnostic.js';
export function traceLiveCapacity(service:any){
 const http=service.app&&service.db?new LifecycleCapacityDiagnostics():undefined;http?.install(service);
 const roster=service.live?.projection&&typeof service.live.frame==='function'?traceLiveRoster(service.live,service.auth.now):undefined;service.app?.addHook('preClose',async()=>roster?.close());
 const events:any[]=[],began=performance.now(),windows=new WeakMap<object,{wall:number;rate:number}>(),counts:Record<string,number>={};let guard:any,firstRetire:any;
 // HTTP's observer has no request context during WSS authentication. Keep its
 // existing database admission result with this exact authentication attempt.
 const authentication=new AsyncLocalStorage<{connection:object;guard?:any}>();
 if(service.db&&typeof service.live?.authenticate==='function'){
  const live=service.live,authenticate=live.authenticate.bind(live),previous=service.db.transactionDiagnostic;
  live.authenticate=(c:any,...args:any[])=>authentication.run({connection:c},()=>authenticate(c,...args));
  service.db.transactionDiagnostic=(event:any)=>{previous?.(event);const state=authentication.getStore();if(state&&event.phase==='admission'&&event.code==='CAPACITY'){
   state.guard={guard:'database-work',database:service.db.workStats,phase:event.phase,commit:event.commit};
  }};
 }
 const push=(event:any)=>{const value={elapsedMs:Math.floor(performance.now()-began),...event};events.push(value);if(event.guard)counts[event.guard]=(counts[event.guard]??0)+1;if(event.retire&&!firstRetire)firstRetire=value;if(events.length>64)events.shift();};
 const traffic=service.traffic,take=traffic.take.bind(traffic);
 traffic.take=(key:string,limit:number)=>{try{const result=take(key,limit),c=traffic.counters.get(key);if(c&&!windows.has(c))windows.set(c,{wall:performance.now(),rate:service.auth.now()});return result;}catch(e){const c=traffic.counters.get(key),first=c?windows.get(c):undefined;guard={guard:!c?'traffic-key-capacity':key.startsWith('class:')?'traffic-class-'+key.split(':').at(-1):key.startsWith('live-control:')?'live-control':key.startsWith('room:')?'room-traffic':key.startsWith('session:')?'session-traffic':key.startsWith('principal:')?'principal-traffic':'traffic-key-capacity',limit,count:c?.count,windowRemainingMs:c?Math.max(0,c.until-service.auth.now()):0,...(first?{windowWallElapsedMs:Math.round(performance.now()-first.wall),windowRateElapsedMs:service.auth.now()-first.rate}:{})};http?.trafficGuard(guard.guard);push(guard);throw e;}};
 const pose=traffic.takePose.bind(traffic);traffic.takePose=(key:string)=>{try{return pose(key);}catch(e){const c=traffic.counters.get(key);guard={guard:c?'traffic-pose-burst':'traffic-key-capacity',credit:c?.credit,refillRemainingMs:c?Math.max(0,c.until-service.auth.now()):0};push(guard);throw e;}};
 const live=service.live,enter=live.admission.enter.bind(live.admission),retire=live.retire.bind(live);
 const connect=live.connect.bind(live);live.connect=(socket:any)=>{if(live.connections.size>=4096)push({guard:'connection-capacity'});connect(socket);socket.prependListener('message',()=>{guard=undefined;});};
 live.admission.enter=()=>{try{return enter();}catch(e){guard={guard:'live-auth-admission'};push(guard);throw e;}};
 live.retire=(c:any,code:string,...args:any[])=>{if(!c.retired&&code==='CAPACITY'){const state=authentication.getStore(),cause=state&&state.connection===c&&state.guard?state.guard:guard,stack=new Error().stack??'',session=c.binding?.scope.sessionId??c.initial?.scope.sessionId,duplicate=!!session&&live.sessions.has(session)&&live.sessions.get(session)!==c;push({retire:'CAPACITY',...cause,guard:cause?.guard??(stack.includes('drain')?'outbound-bytes':stack.includes('tick')?'slow-consumer':stack.includes('authenticate')?(duplicate?'duplicate-session':'auth-capacity-unattributed'):c.control.length>=8?'control-queue':c.busy?'busy-auth':'capacity-unattributed'),busy:!!c.busy,controlFrames:c.control.length,bufferedBytes:c.socket.bufferedAmount,stalledMs:c.stalledAt?Date.now()-c.stalledAt:0,admission:live.admission.stats});guard=undefined;}return retire(c,code,...args);};
 return {snapshot:()=>structuredClone(events),status:()=>({revision:1,events:structuredClone(events),counts:{...counts},firstRetire:firstRetire?structuredClone(firstRetire):null,...(roster?{roster:roster.status()}:{}),...(http?{http:http.snapshot()}:{})})};
}
