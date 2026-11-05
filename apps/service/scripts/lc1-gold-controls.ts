// Private disposable fixture only: drop/delay one successful committed reply.
// No game facts, outcomes, ledger values or scope authority are fabricated.
import {decode} from '../../../packages/contracts/src/g3.js';
import {G3Authority} from '../src/g3/authority.js';
type Plan={target:'consumer'|'peer';operationId:string;kind:'goldIntent'|'goldReceipt'|'itemDeposit'|'itemClaim'|'itemReceipt';effect?:'drop'|'delay';delayMs?:number};
export function matchesGoldControl(plan:Pick<Plan,'kind'|'operationId'>,input:any,result:any,owner:{principalId:string;roomId:string}){
 if(input?.scope?.roomId!==owner.roomId||result?.error)return false;
 let returned=result;
 if(input.bankBatch){
  const index=input.bankBatch.entries?.findIndex((e:any)=>e?.[plan.kind]?.operationId===plan.operationId)??-1;
  if(index<0||input.bankBatch.entries[index].scope?.roomId!==owner.roomId)return false;
  returned=result.bankBatch?.entries?.[index];
  if(returned?.index!==index||returned?.operationId!==plan.operationId||returned.error)return false;
 }else if(input?.[plan.kind]?.operationId!==plan.operationId)return false;
 const operation=returned?.itemOperation??returned?.operation;
 return operation?.operationId===plan.operationId&&operation.principalId===owner.principalId;
}
export class LifecycleGoldControls{
 private owners=new Map<string,{principalId:string;roomId:string}>();private active:Plan|undefined;private count=0;private events:any[]=[];private claims:any[]=[];private unblock:(()=>void)|undefined;
 private inFlightDelayedOp:(Pick<Plan,'target'|'kind'|'operationId'>&{delayMs:number})|undefined;
 bind(target:'consumer'|'peer',principalId:string,roomId:string){this.owners.set(target,{principalId,roomId});}
 arm(plan:Plan){if(this.active||this.unblock||this.count>=16||!['consumer','peer'].includes(plan?.target)||!this.owners.has(plan.target)||!['goldIntent','goldReceipt','itemDeposit','itemClaim','itemReceipt'].includes(plan.kind)||!/^[a-f0-9]{8}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{12}$/.test(plan.operationId)||!['drop','delay'].includes(plan.effect??'drop')||plan.delayMs!==undefined&&(!Number.isInteger(plan.delayMs)||plan.delayMs<1||plan.delayMs>10000||plan.effect!=='delay')||Object.keys(plan).some(k=>!['target','operationId','kind','effect','delayMs'].includes(k)))throw Error('FIXTURE_GOLD_PLAN');this.active={...plan};this.count++;return this.status();}
 release(){this.active=undefined;this.unblock?.();}
 status(){return {armed:this.active?{...this.active}:null,inFlightDelayedOp:this.inFlightDelayedOp?{...this.inFlightDelayedOp}:null,events:structuredClone(this.events),claims:structuredClone(this.claims)};}
 install(service:any){
  // Observe the row already selected under the existing claim lock. No added
  // query, transaction, payload capture or change to claim authority/result.
  const controls=this,prototype=G3Authority.prototype as any,execute=prototype.execute;
  const observed=async function(this:G3Authority,tx:any,actor:any,input:any,...args:any[]){
   const p=input?.itemClaim,target=[...controls.owners].find(([,owner])=>owner.principalId===actor.accountId&&owner.roomId===input.scope?.roomId)?.[0],uuid=(v:any)=>typeof v==='string'&&/^[a-f0-9-]{36}$/.test(v);
   if(!p||!target||!uuid(p.operationId)||!uuid(p.itemId))return execute.call(this,tx,actor,input,...args);
   const event:any={target,operationId:p.operationId,itemId:p.itemId,expectedRevision:/^[0-9]{1,19}$/.test(p.expectedRevision??'')?p.expectedRevision:'invalid',phase:'authority',commit:'unobserved'},query=tx.query;
   const observedQuery=async(...q:any[])=>{const result=await query.apply(tx,q);if(q[0]==='SELECT * FROM g3_bank_items WHERE run_id=$1 AND item_id=$2 FOR UPDATE'){
    const row=result.rows[0];event.rowFound=!!row;if(row){event.actualState=['awaiting_local','available','claimed','delivered','cancelled','uncertain'].includes(row.state)?row.state:'other';event.actualRevision=/^[0-9]{1,19}$/.test(String(row.revision))?String(row.revision):'invalid';}
   }return result;};tx.query=observedQuery;
   try{const result=await execute.call(this,tx,actor,input,...args);event.outcome='accepted';event.operationState=result.itemOperation?.state;return result;}
   catch(error){const code=(error as any)?.code;event.outcome=typeof code==='string'&&/^[A-Z_]{1,64}$/.test(code)?code:'OTHER';throw error;}
   finally{if(tx.query===observedQuery)tx.query=query;controls.claims.push(event);if(controls.claims.length>32)controls.claims.shift();}
  };prototype.execute=observed;
  service.app.addHook('preClose',async()=>{if(prototype.execute===observed)prototype.execute=execute;});
  service.app.addHook('onSend',async(req:any,reply:any,payload:any)=>{
  const plan=this.active;if(!plan||req.url!=='/v3/rpc'||reply.statusCode!==200)return payload;
  try{const input=Buffer.isBuffer(req.body)?decode('Request',req.body):req.body,result=Buffer.isBuffer(payload)?decode('Response',payload):JSON.parse(String(payload));const owner=this.owners.get(plan.target)!;
   if(!matchesGoldControl(plan,input,result,owner))return payload;
   this.active=undefined;const effect=plan.effect??'drop';this.events.push({target:plan.target,kind:plan.kind,operationId:plan.operationId,phase:'reply',effect,committed:true});if(this.events.length>32)this.events.shift();
   if(effect==='delay'){this.inFlightDelayedOp={target:plan.target,kind:plan.kind,operationId:plan.operationId,delayMs:plan.delayMs??5000};await new Promise<void>(resolve=>{const done=()=>{clearTimeout(timer);this.unblock=undefined;this.inFlightDelayedOp=undefined;resolve();},timer=setTimeout(done,plan.delayMs??5000);this.unblock=done;});return payload;}
   reply.hijack();reply.raw.destroy();return null;
  }catch{return payload;}
 });}
}

