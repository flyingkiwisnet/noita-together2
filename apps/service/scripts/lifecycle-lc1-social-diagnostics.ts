// Disposable fixture observation only: bounded codes/timings, no request payloads.
import {AsyncLocalStorage} from 'node:async_hooks';
import type {TransactionDiagnostic} from '../src/g1/database.js';
import {decode} from '../../../packages/contracts/src/g3-social.js';
import type {startG3Fixture} from './g3-loot01-fixture.js';
type Service=Awaited<ReturnType<typeof startG3Fixture>>['service'];
type Entry={operation:string;requestId?:string;elapsedMs:number;responseCode?:string;httpStatus?:number;transactions:TransactionDiagnostic[]};
export class SocialDiagnostics{
 private context=new AsyncLocalStorage<Entry>();private requests=new WeakMap<object,{entry:Entry;start:number}>();
 private recent:Entry[]=[];private failures:Entry[]=[];private totals:Record<string,number>={};private second=-1;private secondCount=0;private peak=0;
 snapshot(){return {recent:structuredClone(this.recent),failures:structuredClone(this.failures),totals:{...this.totals},peakRequestsPerSecond:this.peak};}
 install(service:Service){
  const previous=service.db.transactionDiagnostic;
  service.db.transactionDiagnostic=e=>{previous?.(e);const entry=this.context.getStore();if(entry&&entry.transactions.length<8)entry.transactions.push({...e});};
  service.app.addHook('preHandler',(req,_reply,done)=>{
   if(req.url!=='/v3/social/rpc')return done();
   let body:any;try{body=Buffer.isBuffer(req.body)?decode('Request',req.body):req.body;}catch{}
   const operation=body?.publish?(['hud','emote','cosmetic','poi'].find(k=>body.publish[k]!==undefined)??'invalid'):(['snapshot','roster','poiSnapshot','poiOutbox','poiOutcome','dismissPoi'].find(k=>body?.[k]!==undefined)??'invalid');
   const entry:Entry={operation,elapsedMs:0,transactions:[],...(typeof body?.requestId==='string'&&/^[A-Za-z0-9_-]{1,80}$/.test(body.requestId)?{requestId:body.requestId}:{})};
   const start=performance.now(),second=Math.floor(start/1000);if(second!==this.second){this.second=second;this.secondCount=0;}this.peak=Math.max(this.peak,++this.secondCount);this.totals[operation]=(this.totals[operation]??0)+1;
   this.requests.set(req,{entry,start});this.context.run(entry,done);
  });
  service.app.addHook('onSend',async(req,reply,payload)=>{
   const state=this.requests.get(req);if(!state)return payload;
   const {entry,start}=state;entry.elapsedMs=Math.round(performance.now()-start);entry.httpStatus=reply.statusCode;
   try{const body=req.headers['content-type']?.split(';')[0]==='application/x-protobuf'?decode('Response',Buffer.from(payload as Buffer)):JSON.parse(String(payload));const code=(body as any)?.error?.code;if(typeof code==='string'&&/^[A-Z_]{1,64}$/.test(code))entry.responseCode=code;}catch{}
   this.recent.push(entry);if(this.recent.length>64)this.recent.shift();if(entry.responseCode){this.failures.push(entry);if(this.failures.length>16)this.failures.shift();}return payload;
  });
 }
}
