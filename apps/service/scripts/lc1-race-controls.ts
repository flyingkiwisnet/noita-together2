// Disposable TLS fixture faults only. No public/production control endpoint.
import {decode} from '../../../packages/contracts/src/g3-race.js';
type Target='consumer'|'peer';
export class LifecycleRaceControls {
 private accounts=new Map<Target,string>();private drops=new Set<Target>();private delays=new Map<Target,number>();
 private pending=new Set<()=>void>();private commands=0;private dropped=0;private delayed=0;
 private responses:{target:Target;operation:'commit'|'read';sequence?:number;code:string}[]=[];
 bind(target:Target,accountId:string){this.accounts.set(target,accountId);}
 private target(value:unknown):Target{if(value!=='consumer'&&value!=='peer'||!this.accounts.has(value))throw Error('FIXTURE_RACE_TARGET');return value;}
 dropReply(target:unknown){const t=this.target(target);if(this.commands++>=32||this.drops.has(t))throw Error('FIXTURE_RACE_BOUNDS');this.drops.add(t);return this.status();}
 delay(target:unknown,ms:unknown){const t=this.target(target);if(this.commands++>=32||!Number.isInteger(ms)||Number(ms)<0||Number(ms)>10000)throw Error('FIXTURE_RACE_BOUNDS');if(ms===0)this.delays.delete(t);else this.delays.set(t,Number(ms));return this.status();}
 status(){return {revision:'race-fixture-1',dropArmed:[...this.drops],delayArmed:[...this.delays].map(([target,durationMs])=>({target,durationMs})),dropped:this.dropped,delayed:this.delayed,pendingDelays:this.pending.size,responses:this.responses.map(v=>({...v}))};}
 release(){this.drops.clear();this.delays.clear();for(const done of [...this.pending])done();}
 install(service:any){
  const targets=new WeakMap<object,Target>(),inputs=new WeakMap<object,any>();
  service.app.addHook('preHandler',async(req:any)=>{
   if(req.url!=='/rpc/g3/race')return;
   let input;try{input=Buffer.isBuffer(req.body)?decode('Request',req.body):req.body;}catch{return;}inputs.set(req,input);
   const token=req.headers.authorization?.match(/^Bearer ([^ ]{1,4096})$/)?.[1];if(!token)return;let claims;try{claims=await service.auth.security.verify(token,service.auth.now());}catch{return;}
   const target=[...this.accounts].find(([,id])=>id===claims.sub)?.[0];if(!target)return;targets.set(req,target);
   const ms=input?.commit?this.delays.get(target):undefined;if(!ms)return;this.delays.delete(target);this.delayed++;
   await new Promise<void>(resolve=>{const done=()=>{clearTimeout(timer);this.pending.delete(done);resolve();},timer=setTimeout(done,ms);this.pending.add(done);});
  });
  service.app.addHook('onSend',async(req:any,reply:any,payload:any)=>{
   const target=targets.get(req);if(!target)return payload;
   let body;try{body=reply.getHeader('content-type')?.toString().startsWith('application/x-protobuf')?decode('Response',payload):JSON.parse(String(payload));}catch{return payload;}
   const input=inputs.get(req);this.responses.push({target,operation:input?.commit?'commit':'read',...(Number.isSafeInteger(input?.commit?.sequence)?{sequence:input.commit.sequence}:{}),code:body.error?.code??(body.receipt?'receipt':body.snapshot?'snapshot':'unknown')});if(this.responses.length>32)this.responses.shift();
   if(!this.drops.has(target)||body.error||!body.receipt)return payload;
   this.drops.delete(target);this.dropped++;reply.hijack();reply.raw.destroy();return null;
  });
 }
}
