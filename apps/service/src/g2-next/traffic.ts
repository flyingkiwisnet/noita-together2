import {isIP} from 'node:net';
import type {FastifyInstance} from 'fastify';
import type {Security} from '../g1/security.js';
import {PolicyError} from '../g1/security.js';
export const TRAFFIC_LIMITS={keys:32768,ipKeys:1024,ipAbuse:4096,anonymousLogin:20,anonymousRead:120,
 session:96,principal:256,room:2048,health:16,read:24,social:24,pose:10,reliable:12,lifecycle:8} as const;
// Same sustained pose rate, finite allowance for arrival compression. Millitoken
// credits keep refill arithmetic integral; reconnect/reauth retain the session key.
export const POSE_RATE_POLICY={perSecond:TRAFFIC_LIMITS.pose,burst:3} as const;
export type CommandClass='health'|'read'|'social'|'pose'|'reliable'|'lifecycle';
const decodedBodies=new WeakMap<object,unknown>();
export function rememberTrafficBody(request:object,body:unknown){decodedBodies.set(request,body);}
export function commandClass(path:string,body:any):CommandClass{
 if(path==='/v3/stats/rpc')return body?.discover||body?.read?'read':'reliable';
 if(path==='/v3/rejoin/rpc'&&(body?.list||body?.status))return 'read';
 if(path==='/v3/c2/rpc')return body?.rulesGet||body?.state||body?.deliveries?'read':body?.publish||body?.receipt||body?.near||body?.sourceOutcome?'reliable':'lifecycle';
 if(path==='/v3/c1/rpc')return body?.rulesGet||body?.deliveries?'read':body?.publish||body?.receipt||body?.life||body?.sourceOutcome?'reliable':'lifecycle';
 if(path==='/v3/waiting/rpc'&&(body?.status||body?.pendingStatus||body?.entryPreview||body?.entryStatus))return 'read';
 if(path==='/v3/waiting/rpc'&&(body?.outcome||body?.pendingOutcome||body?.entryOutcome))return 'reliable';
 if(path==='/v3/connection/rpc'&&body?.admission)return 'health';
 if(path==='/v3/connection/rpc'&&body?.outcome)return 'reliable';
 if(body?.movement)return 'pose';
 if(body?.gamePulse||body?.gameRenew||body?.gameObservation||body?.presence||body?.refresh)return 'health';
 if(path.includes('/social/'))return 'social';
 if(body?.goldBank||body?.itemBank||body?.itemInspect||body?.inventoryInspect)return 'read';
 if(body?.itemDeposit||body?.itemClaim||body?.itemStatus||body?.itemReceipt||body?.inventoryPublish)return 'reliable';
 if(body?.goldIntent||body?.goldReceipt||body?.goldStatus||body?.outboxList||body?.chatSend||body?.bankBatch)return 'reliable';
 if(body&&Object.keys(body).some(k=>/List$|Aggregate$/.test(k)||['roomGet','roomPreview','roomPlayerDetails'].includes(k)))return 'read';
 return 'lifecycle';
}
// Saturation refuses new keys; untrusted fields cannot allocate unbounded state.
export class TrafficBudgets {
 private counters=new Map<string,{count:number;until:number;credit?:number;at?:number}>();
 constructor(readonly now:()=>number=Date.now,readonly maxKeys:number=TRAFFIC_LIMITS.keys){}
 get size(){return this.counters.size;}
 take(key:string,limit:number){
  if(key.length>512)throw new PolicyError('CAPACITY');
  const now=this.now();let c=this.counters.get(key);
  if(!c&&this.counters.size>=this.maxKeys){for(const [k,v]of this.counters)if(v.until<=now)this.counters.delete(k);if(this.counters.size>=this.maxKeys)throw new PolicyError('CAPACITY');}
  if(!c||c.until<=now){c={count:0,until:now+1000};this.counters.set(key,c);}
  if(++c.count>limit)throw new PolicyError('CAPACITY');
 }
 private takePose(key:string){
  if(key.length>512)throw new PolicyError('CAPACITY');
  const now=Math.floor(this.now()),cap=POSE_RATE_POLICY.burst*1000;let c=this.counters.get(key);
  if(!c&&this.counters.size>=this.maxKeys){for(const [k,v]of this.counters)if(v.until<=now)this.counters.delete(k);if(this.counters.size>=this.maxKeys)throw new PolicyError('CAPACITY');}
  if(!c||c.credit===undefined||c.at===undefined){c={count:0,until:now,credit:cap,at:now};this.counters.set(key,c);}
  const at=Math.max(now,c.at!),elapsed=Math.min(cap/POSE_RATE_POLICY.perSecond,at-c.at!);
  c.credit=Math.min(cap,c.credit!+elapsed*POSE_RATE_POLICY.perSecond);c.at=at;c.count++;
  if(c.credit<1000){c.until=at+Math.ceil((cap-c.credit)/POSE_RATE_POLICY.perSecond);throw new PolicyError('CAPACITY');}
  c.credit-=1000;c.until=at+Math.ceil((cap-c.credit)/POSE_RATE_POLICY.perSecond);
 }
 private chargedRooms=new WeakMap<object,Set<string>>();
 // Only canonical access checks or a verified live binding may call this seam.
 authorizedRoom(room:string,context?:object){
  const charged=context?this.chargedRooms.get(context):undefined;if(charged?.has(room))return;
  this.take('room:'+room,TRAFFIC_LIMITS.room);
  if(context){const rooms=charged??new Set<string>();rooms.add(room);this.chargedRooms.set(context,rooms);}
 }
 authenticated(actor:{sessionId:string;principalId:string},kind:CommandClass){
  this.take('session:'+actor.sessionId,TRAFFIC_LIMITS.session);
  this.take('principal:'+actor.principalId,TRAFFIC_LIMITS.principal);
  const key='class:'+actor.sessionId+':'+kind;
  if(kind==='pose')this.takePose(key);else this.take(key,TRAFFIC_LIMITS[kind]);
 }
}
export function explicitTrustedProxies(value:unknown):false|string[]{
 if(value===undefined)return false;
 if(!Array.isArray(value)||value.length>16)throw Error('Invalid explicitly trusted proxies');
 for(const s of value){if(typeof s!=='string'||s.length>64)throw Error('Invalid explicitly trusted proxy');const [ip,prefix,...extra]=s.split('/');const version=isIP(ip!);if(!version||extra.length||prefix!==undefined&&(!/^\d+$/.test(prefix)||+prefix<1||+prefix>(version===4?32:128)))throw Error('Invalid explicitly trusted proxy');}
 return [...value];
}
export function installTrafficBudgets(app:FastifyInstance,security:Security,now:()=>number){
 const traffic=new TrafficBudgets(now),ips=new TrafficBudgets(now,TRAFFIC_LIMITS.ipKeys);
 app.addHook('onRequest',async(req,reply)=>{try{ips.take('ip:'+req.ip,TRAFFIC_LIMITS.ipAbuse);}catch{await reply.code(429).send({error:{code:'CAPACITY'}});}});
 app.addHook('preHandler',async(req,reply)=>{
  const body=(decodedBodies.get(req)??req.body) as any,token=req.headers.authorization?.match(/^Bearer ([^ ]{1,4096})$/)?.[1];
  try{
   if(token){
    const claims=await security.verify(token,now());
    if(claims.purpose!=='access'||typeof claims.sid!=='string'||typeof claims.sub!=='string'||typeof claims.sessionEpoch!=='string')throw new PolicyError('UNAUTHENTICATED');
    const count=body?.bankBatch?.entries?.length??1;if(!Number.isInteger(count)||count<1||count>4)throw new PolicyError('INVALID_MESSAGE');
    for(let i=0;i<count;i++)traffic.authenticated({sessionId:claims.sid,principalId:claims.sub},commandClass(req.url,body));
   }else{
    const login=req.url.startsWith('/auth/login/')||req.url.includes('/signin/')||req.url.includes('browser-login')||body?.authBegin||body?.authExchange||body?.guestIssue||body?.refresh;
    traffic.take('anon:'+req.ip+':'+(login?'login':'read'),login?TRAFFIC_LIMITS.anonymousLogin:TRAFFIC_LIMITS.anonymousRead);
   }
  }catch(e){
   if(e instanceof PolicyError&&e.code==='UNAUTHENTICATED'){
    try{traffic.take('anon:'+req.ip+':login',TRAFFIC_LIMITS.anonymousLogin);}catch{await reply.code(429).send({error:{code:'CAPACITY'}});} // Existing route retains canonical authentication/error behavior.
   }else await reply.code(429).send({error:{code:'CAPACITY'}});
  }
 });
 return traffic;
}
