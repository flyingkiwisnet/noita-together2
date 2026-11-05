// Disposable TLS fixture faults only. No public/production control endpoint.
type Target='consumer'|'peer';
export class LifecycleD1Controls {
 private accounts=new Map<Target,string>();private drops=new Set<Target>();private delays=new Map<Target,number>();
 private pending=new Set<()=>void>();private commands=0;private dropped=0;private delayed=0;
 bind(target:Target,accountId:string){this.accounts.set(target,accountId);}
 private target(value:unknown):Target{if(value!=='consumer'&&value!=='peer'||!this.accounts.has(value))throw Error('FIXTURE_D1_TARGET');return value;}
 dropReply(target:unknown){const t=this.target(target);if(this.commands++>=32||this.drops.has(t))throw Error('FIXTURE_D1_BOUNDS');this.drops.add(t);return this.status();}
 delay(target:unknown,ms:unknown){const t=this.target(target);if(this.commands++>=32||!Number.isInteger(ms)||Number(ms)<0||Number(ms)>10000)throw Error('FIXTURE_D1_BOUNDS');if(ms===0)this.delays.delete(t);else this.delays.set(t,Number(ms));return this.status();}
 status(){return {revision:'d1-fixture-1',dropArmed:[...this.drops],delayArmed:[...this.delays].map(([target,durationMs])=>({target,durationMs})),dropped:this.dropped,delayed:this.delayed,pendingDelays:this.pending.size};}
 release(){this.drops.clear();this.delays.clear();for(const done of [...this.pending])done();}
 install(service:any){
  const targets=new WeakMap<object,Target>();
  service.app.addHook('preHandler',async(req:any)=>{
   if(req.url!=='/v3/stats/rpc'||!req.body?.observe)return;
   const token=req.headers.authorization?.match(/^Bearer ([^ ]{1,4096})$/)?.[1];if(!token)return;let claims;try{claims=await service.auth.security.verify(token,service.auth.now());}catch{return;}
   const target=[...this.accounts].find(([,id])=>id===claims.sub)?.[0];if(!target)return;targets.set(req,target);
   const ms=this.delays.get(target);if(!ms)return;this.delays.delete(target);this.delayed++;
   await new Promise<void>(resolve=>{const done=()=>{clearTimeout(timer);this.pending.delete(done);resolve();},timer=setTimeout(done,ms);this.pending.add(done);});
  });
  service.app.addHook('onSend',async(req:any,reply:any,payload:any)=>{
   const target=targets.get(req);if(!target||!this.drops.has(target))return payload;
   let body;try{body=JSON.parse(String(payload));}catch{return payload;}if(body.error||body.accepted!==true)return payload;
   this.drops.delete(target);this.dropped++;reply.hijack();reply.raw.destroy();return null;
  });
 }
}
