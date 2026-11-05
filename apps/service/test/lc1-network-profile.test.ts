import test from 'node:test';import assert from 'node:assert/strict';
import {LifecycleNetwork,NETWORK_LIMITS as limits} from '../scripts/lifecycle-lc1-network.js';
class Clock{time=0;next=0;tasks=new Map<number,{at:number;fn:()=>void}>();now=()=>this.time;set=(fn:()=>void,ms:number)=>{const id=++this.next;this.tasks.set(id,{at:this.time+ms,fn});return id;};clear=(id:number)=>{this.tasks.delete(id);};advance(ms:number){const target=this.time+ms;for(;;){const first=[...this.tasks].filter(([,v])=>v.at<=target).sort((a,b)=>a[1].at-b[1].at||a[0]-b[0])[0];if(!first)break;this.time=Math.max(this.time,first[1].at);this.tasks.delete(first[0]);first[1].fn();}this.time=target;}}
test('disabled profile delivers immediately and allocates no delay timers',async()=>{const c=new Clock(),n=new LifecycleNetwork(c);let count=0;const r=n.enqueue('consumer','wssOut',{},0,12,()=>count++,()=>assert.fail());assert.equal(count,1);assert.equal(await r.done,true);assert.equal(c.tasks.size,0);assert.deepEqual(n.status().pending,{flows:0,frames:0,bytes:0,timers:0});n.close();});
test('arrival-based jitter preserves burst FIFO and pipelines10Hz through400ms RTT',async()=>{
 const c=new Clock(),n=new LifecycleNetwork(c),key={},out:{i:number;at:number}[]=[],done:Promise<boolean>[]=[];n.configure('consumer','rtt400',0);
 for(let i=0;i<20;i++){done.push(n.enqueue('consumer','wssOut',key,c.now(),100,()=>out.push({i,at:c.now()}),()=>assert.fail()).done);c.advance(100);}c.advance(400);
 assert.ok((await Promise.all(done)).every(Boolean));assert.deepEqual(out.map(v=>v.i),Array.from({length:20},(_,i)=>i));assert.ok(out.at(-1)!.at<=2140);assert.ok(out.at(-1)!.at-out[0]!.at<2100);assert.ok(n.status().highWater.frames<=3);assert.ok(n.status().highWater.timers<=2);
 const burst:Promise<boolean>[]=[],order:number[]=[];for(let i=0;i<5;i++)burst.push(n.enqueue('consumer','wssIn',key,c.now(),10,()=>order.push(i),()=>assert.fail()).done);c.advance(500);await Promise.all(burst);assert.deepEqual(order,[0,1,2,3,4]);assert.ok(n.status().telemetry.every(s=>s.samples.every(v=>v.clampedMs>=v.plannedMs)));n.close();assert.equal(c.tasks.size,0);
});
test('deterministic seed and three bounded RTT definitions produce repeatable delays',async()=>{
 const values:any[]=[];for(const name of ['rtt80','rtt200','rtt400'] as const){const repetitions:number[][]=[];for(let repeat=0;repeat<2;repeat++){const c=new Clock(),n=new LifecycleNetwork(c),times:number[]=[];n.configure('consumer',name,13);for(let i=0;i<5;i++){const r=n.enqueue('consumer','httpOut',{},c.now(),20,()=>times.push(c.now()),()=>assert.fail());c.advance(500);assert.equal(await r.done,true);}repetitions.push(n.status().telemetry.find(s=>s.lane==='httpOut')!.samples.map(s=>s.plannedMs));n.close();}assert.deepEqual(repetitions[0],repetitions[1]);values.push(repetitions[0]);}assert.ok(values[0][0]<values[1][0]&&values[1][0]<values[2][0]);
});
test('target reset and expiry cancel once, retain bounded completed summaries, isolate peer',async()=>{
 const c=new Clock(),n=new LifecycleNetwork(c),events:string[]=[];n.configure('consumer','rtt400',0,1000);n.configure('peer','rtt80');
 const a=n.enqueue('consumer','wssIn',{},c.now(),10,()=>events.push('bad'),()=>events.push('cancel-consumer')),b=n.enqueue('peer','wssIn',{},c.now(),10,()=>events.push('peer'),()=>assert.fail());n.reset('consumer');assert.equal(await a.done,false);c.advance(100);assert.equal(await b.done,true);assert.deepEqual(events,['cancel-consumer','peer']);assert.equal(n.status().profiles[0]!.target,'peer');assert.equal(n.status().completed[0]!.reason,'profile-reset');
 n.configure('consumer','rtt400',0,1000);c.advance(900);const e=n.enqueue('consumer','httpOut',{},c.now(),10,()=>events.push('bad'),()=>events.push('cancel-expired'));c.advance(500);assert.equal(await e.done,false);assert.equal(events.filter(v=>v==='cancel-expired').length,1);assert.ok(n.status().transitions.some(v=>v.reason==='profile-expired'));n.close();assert.equal(c.tasks.size,0);
});
test('finite per-flow count/bytes, global flow/count/bytes and profile configuration bounds',async()=>{
 for(const kind of ['flow-count','flow-bytes','global-flows','global-frames','global-bytes','http-bytes'] as const){const c=new Clock(),n=new LifecycleNetwork(c);n.configure('consumer','rtt400');const key={};let rejected=0;const add=(key:object,bytes=1,lane:'wssIn'|'httpIn'='wssIn')=>n.enqueue('consumer',lane,key,c.now(),bytes,()=>{},()=>rejected++);
  if(kind==='flow-count'){for(let i=0;i<limits.wssFrames;i++)assert.equal(add(key).accepted,true);assert.equal(add(key).accepted,false);}
  if(kind==='flow-bytes'){assert.equal(add(key,32768).accepted,true);assert.equal(add(key,32768).accepted,true);assert.equal(add(key,1).accepted,false);}
  if(kind==='global-flows'){for(let i=0;i<limits.flows;i++)assert.equal(add({}).accepted,true);assert.equal(add({}).accepted,false);}
  if(kind==='global-frames'){for(let i=0;i<4;i++){const f={};for(let j=0;j<64;j++)assert.equal(add(f).accepted,true);}assert.equal(add({}).accepted,false);}
  if(kind==='global-bytes'){for(let i=0;i<64;i++){const f={};assert.equal(add(f,32768).accepted,true);assert.equal(add(f,32768).accepted,true);}assert.equal(add({}).accepted,false);}
  if(kind==='http-bytes')assert.equal(add({},limits.httpBytes+1,'httpIn').accepted,false);
  assert.equal(rejected,1);assert.ok(n.status().pending.frames<=limits.frames&&n.status().pending.bytes<=limits.bytes&&n.status().pending.flows<=limits.flows);n.close();assert.equal(c.tasks.size,0);assert.equal(n.status().pending.frames,0);
 }
 const c=new Clock(),n=new LifecycleNetwork(c);for(let i=0;i<limits.configurations;i++)n.configure('consumer','rtt80');assert.throws(()=>n.configure('consumer','rtt80'),/FIXTURE_NETWORK_ARGUMENT/);assert.equal(n.status().completed.length,limits.completedProfiles);assert.equal(n.status().transitions.length,limits.transitions);n.close();assert.equal(c.tasks.size,0);
});
test('invalid profile parameters and overdue dispatch never silently deliver',async()=>{
 const c=new Clock(),n=new LifecycleNetwork(c);for(const args of [['unknown','rtt80'],['consumer','toString'],['consumer','rtt80',-1],['consumer','rtt80',65536],['consumer','rtt80',0,999],['consumer','rtt80',0,600001]])assert.throws(()=>(n.configure as any)(...args),/FIXTURE_NETWORK_ARGUMENT/);
 n.configure('consumer','rtt400');let cancelled=0;const r=n.enqueue('consumer','wssOut',{},0,10,()=>assert.fail(),()=>cancelled++);c.time=5000;c.advance(0);assert.equal(await r.done,false);assert.equal(cancelled,1);assert.equal(n.status().telemetry.find(s=>s.lane==='wssOut')!.late,1);n.close();
});
