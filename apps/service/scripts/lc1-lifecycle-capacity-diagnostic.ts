// Fixture-only, request-correlated guard evidence. No payloads or credentials.
import {AsyncLocalStorage} from 'node:async_hooks';
import {WorkGate} from '../src/g1/work.js';
import type {TransactionDiagnostic} from '../src/g1/database.js';
import {decode,schema as lifecycleSchema} from '../../../packages/contracts/src/g3-lifecycle.js';
import {decode as socialDecode,schema as socialSchema} from '../../../packages/contracts/src/g3-social.js';
import {decode as gameplayDecode,schema as gameplaySchema} from '../../../packages/contracts/src/g3.js';
import {decode as connectionDecode,schema as connectionSchema} from '../../../packages/contracts/src/g3-connection.js';
import {decode as waitingDecode,schema as waitingSchema} from '../../../packages/contracts/src/g3-waiting.js';
import {decode as lobbyDecode,schema as lobbySchema} from '../../../packages/contracts/src/g2-next.js';
import {decode as rejoinDecode,schema as rejoinSchema} from '../../../packages/contracts/src/g3-rejoin.js';
import {SocialAuthority} from '../src/g3/social.js';
const routes:Record<string,{decode:typeof decode;schema:typeof lifecycleSchema}>={
 '/v3/lifecycle/rpc':{decode,schema:lifecycleSchema},'/v3/social/rpc':{decode:socialDecode,schema:socialSchema},'/v3/rpc':{decode:gameplayDecode,schema:gameplaySchema},'/v3/connection/rpc':{decode:connectionDecode,schema:connectionSchema},'/v3/waiting/rpc':{decode:waitingDecode,schema:waitingSchema},'/v1/rpc':{decode:lobbyDecode,schema:lobbySchema},'/v3/rejoin/rpc':{decode:rejoinDecode,schema:rejoinSchema},
};
const operation=(path:string,q:any)=>path==='/v3/social/rpc'&&q?.publish?(['hud','emote','cosmetic','poi'].find(k=>q.publish[k]!==undefined)??'unknown'):(routes[path]?.schema.messages.Request?.oneof??[]).find(k=>q?.[k]!==undefined)??'unknown';
type Entry={operation:string;route?:string;requestId?:string;eventId?:string;sequence?:number;selection?:'start'|'stop';elapsedMs:number;httpStatus?:number;responseCode?:string;correlated:boolean;guard?:string;workQueryCalls?:number;workQueryMs?:number;transactionMs?:number;gates:{limit:number;stats:WorkGate['stats']}[];transactions:TransactionDiagnostic[]};
export class LifecycleCapacityDiagnostics{
 private context=new AsyncLocalStorage<Entry>();private requests=new WeakMap<object,{entry:Entry;start:number}>();private recent:Entry[]=[];private failures:Entry[]=[];private total=0;private firstSocialCapacity:Entry|undefined;
 private active=new Map<Entry,number>();private totals=new Map<string,{route:string;operation:string;requests:number;failures:number;elapsedMs:number;maxMs:number;workQueryCalls:number;workQueryMs:number;transactionMs:number}>();
 private saturations:any[]=[];private firstSaturation:any;
 snapshot(){return {revision:2,total:this.total,recent:structuredClone(this.recent),failures:structuredClone(this.failures),active:[...this.active].map(([entry,start])=>({...structuredClone(entry),elapsedMs:Math.round(performance.now()-start)})),operations:structuredClone([...this.totals.values()]),saturations:structuredClone(this.saturations),firstSaturation:this.firstSaturation?structuredClone(this.firstSaturation):null,...(this.firstSocialCapacity?{firstSocialCapacity:structuredClone(this.firstSocialCapacity)}:{})};}
 trafficGuard(guard:string){const entry=this.context.getStore();if(entry)entry.guard=guard;}
 install(service:any){
  const context=this.context,enter=WorkGate.prototype.enter;
  const observed=function(this:WorkGate){try{return enter.call(this);}catch(e){const entry=context.getStore();if(entry&&(e as any)?.code==='CAPACITY'&&entry.gates.length<4)entry.gates.push({limit:this.limit,stats:{...this.stats}});throw e;}};
  WorkGate.prototype.enter=observed;
  service.app.addHook('preClose',async()=>{if(WorkGate.prototype.enter===observed)WorkGate.prototype.enter=enter;});
  // Observe the existing private method before execute normalizes policy errors.
  // Preserve the original rejection and expose only a fixed category, no stack.
  const social=SocialAuthority.prototype as any,apply=social.apply;
  const observedSocial=async function(this:SocialAuthority,...args:any[]){try{return await apply.apply(this,args);}catch(e){const entry=context.getStore();if(entry&&(e as any)?.code==='CAPACITY'){
   const stack=e instanceof Error?e.stack??'':'';
   entry.guard=stack.includes('checkSocialBudget')?'social-kind-rate':stack.includes('readBudget')?'social-read-budget':stack.includes('authorizeSocial')?'social-recipient-capacity':'social-domain';
  }throw e;}};
  social.apply=observedSocial;
  service.app.addHook('preClose',async()=>{if(social.apply===observedSocial)social.apply=apply;});
  const previous=service.db.transactionDiagnostic;
  service.db.transactionDiagnostic=(e:TransactionDiagnostic)=>{previous?.(e);const entry=context.getStore();if(entry&&entry.transactions.length<8)entry.transactions.push({...e});if(e.phase==='admission'&&e.code==='CAPACITY'){
   // HTTP activity is not proof that every request owns a database permit.
   // Capture it at rejection time, including WSS/background admission failures.
   const sample={...(entry?{route:entry.route,operation:entry.operation}:{}),scheduler:service.db.workStats,activeHttp:[...this.active].slice(0,16).map(([v,start])=>({route:v.route,operation:v.operation,elapsedMs:Math.round(performance.now()-start),workQueryCalls:v.workQueryCalls??0,workQueryMs:v.workQueryMs??0})),activeHttpTotal:this.active.size};if(!this.firstSaturation)this.firstSaturation=structuredClone(sample);this.saturations.push(sample);if(this.saturations.length>8)this.saturations.shift();
  }};
  const transaction=service.db.transaction;
  if(typeof transaction==='function'){
   const observedTransaction=async function(this:any,fn:any,...args:any[]){const entry=context.getStore();if(!entry)return transaction.call(this,fn,...args);const start=performance.now();
    try{return await transaction.call(this,async(tx:any)=>{const query=tx.query,observedQuery=async(...q:any[])=>{const began=performance.now();entry.workQueryCalls=(entry.workQueryCalls??0)+1;try{return await query.apply(tx,q);}finally{entry.workQueryMs=(entry.workQueryMs??0)+Math.round(performance.now()-began);}};tx.query=observedQuery;try{return await fn(tx);}finally{if(tx.query===observedQuery)tx.query=query;}},...args);}finally{entry.transactionMs=(entry.transactionMs??0)+Math.round(performance.now()-start);}
   };service.db.transaction=observedTransaction;service.app.addHook('preClose',async()=>{if(service.db.transaction===observedTransaction)service.db.transaction=transaction;});
  }
  service.app.addHook('onRequest',(req:any,_reply:any,done:()=>void)=>{
   if(!routes[req.url])return done();
   const entry:Entry={route:req.url,operation:'unknown',elapsedMs:0,correlated:false,gates:[],transactions:[]},start=performance.now();this.total++;
   this.requests.set(req,{entry,start});if(this.active.size<64)this.active.set(entry,start);context.run(entry,done);
  });
  service.app.addHook('preHandler',async(req:any)=>{const state=this.requests.get(req);if(!state)return;try{const q=Buffer.isBuffer(req.body)?routes[req.url]!.decode('Request',req.body):req.body;state.entry.operation=operation(req.url,q);}catch{}});
  service.app.addHook('onResponse',async(req:any)=>{const state=this.requests.get(req);if(state)this.active.delete(state.entry);});
  service.app.addHook('onSend',async(req:any,reply:any,payload:any)=>{
   const state=this.requests.get(req);if(!state)return payload;const {entry,start}=state;entry.elapsedMs=Math.round(performance.now()-start);entry.httpStatus=reply.statusCode;
   const isSocial=req.url==='/v3/social/rpc';
   try{const binary=req.headers['content-type']?.split(';')[0]==='application/x-protobuf',codec=routes[req.url]!.decode,q=binary?codec('Request',req.body):req.body;
    entry.operation=operation(req.url,q);if(typeof q?.requestId==='string'&&/^[A-Za-z0-9_-]{1,128}$/.test(q.requestId))entry.requestId=q.requestId;
    if(isSocial&&q?.publish){if(typeof q.publish.eventId==='string'&&/^[a-f0-9-]{36}$/i.test(q.publish.eventId))entry.eventId=q.publish.eventId;if(Number.isInteger(q.publish.sequence)&&q.publish.sequence>=1&&q.publish.sequence<=0xffffffff)entry.sequence=q.publish.sequence;if(q.publish.emote)entry.selection=q.publish.emote.active===true?'start':'stop';}
    // Shared HTTP quotas return JSON even for a protobuf RPC request.
    const responseBinary=binary&&String(reply.getHeader('content-type')??'').startsWith('application/x-protobuf');
    const result=responseBinary?codec('Response',Buffer.from(payload)):JSON.parse(String(payload));entry.correlated=!!entry.requestId&&result.requestId===entry.requestId;
    if(typeof result.error?.code==='string'&&/^[A-Z_]{1,64}$/.test(result.error.code))entry.responseCode=result.error.code;
   }catch{/* Diagnostic parsing cannot affect the original response. */}
   if(entry.responseCode==='CAPACITY')entry.guard=entry.httpStatus===429?entry.guard??'http-quota':entry.gates.some(g=>g.limit===7)?'database-work-7':entry.transactions.some(t=>t.code==='CAPACITY'&&t.phase==='admission')?'database-'+(entry.transactions.find(t=>t.code==='CAPACITY'&&t.phase==='admission')!.workClass??'unknown')+'-admission':entry.gates.some(g=>g.limit===8)?(isSocial?'social-route-work-8':'lifecycle-route-work-8'):entry.guard??(entry.transactions.some(t=>t.code==='CAPACITY'&&t.phase==='work')?(isSocial?'social-domain':'lifecycle-domain'):'unattributed');
   if(isSocial&&entry.responseCode==='CAPACITY'&&!this.firstSocialCapacity)this.firstSocialCapacity=structuredClone(entry);
   const key=entry.route+':'+entry.operation;if(this.totals.has(key)||this.totals.size<128){const total=this.totals.get(key)??{route:entry.route!,operation:entry.operation,requests:0,failures:0,elapsedMs:0,maxMs:0,workQueryCalls:0,workQueryMs:0,transactionMs:0};total.requests++;if(entry.responseCode)total.failures++;total.elapsedMs+=entry.elapsedMs;total.maxMs=Math.max(total.maxMs,entry.elapsedMs);total.workQueryCalls+=entry.workQueryCalls??0;total.workQueryMs+=entry.workQueryMs??0;total.transactionMs+=entry.transactionMs??0;this.totals.set(key,total);}this.active.delete(entry);
   this.recent.push(entry);if(this.recent.length>32)this.recent.shift();if(entry.responseCode){this.failures.push(entry);if(this.failures.length>16)this.failures.shift();}return payload;
  });
 }
}
