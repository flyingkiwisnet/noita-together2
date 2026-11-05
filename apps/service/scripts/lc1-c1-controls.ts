import {decode as decodeC2,validate as validateC2} from '../../../packages/contracts/src/g3-c2.js';
import {c2FixtureAudit} from './lc1-c2-audit.js';
// Private disposable fixture controls only. Never registered as service endpoints.
import {randomUUID} from 'node:crypto';
import {decode,validate,schema} from '../../../packages/contracts/src/g3-c1.js';
import type {Rules,Request} from '../../../packages/contracts/g3-c1/g3-c1.js';
import type {Credentials} from '../../../packages/contracts/g2-next/g2-next.js';
import {C1_SKIP_REASONS} from '../../../packages/domain/src/g3-c1.js';
const uuid=(s:unknown)=>typeof s==='string'&&/^[a-f0-9]{8}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{12}$/.test(s);
type Target='consumer'|'peer';type Owner={principalId:string;deviceId:string;roomId:string};
type Drop={target:Target;eventId?:string};type Arm=Drop&{armId:string;roomId:string;runId:string;owner:Owner};
export const C1_FIXTURE_LIMITS={ruleCommands:32,audits:128,events:64,dropArms:16,dropHistory:32} as const;
export function matchesC1PublishReply(arm:Arm,input:any,result:any){
 return !!input?.publish&&!result?.error&&!result?.duplicate&&input.scope?.roomId===arm.roomId&&input.scope?.runId===arm.runId&&result.scope?.roomId===arm.roomId&&result.scope?.runId===arm.runId&&result.requestId===input.requestId&&uuid(input.publish.eventId)&&result.event?.eventId===input.publish.eventId&&(!arm.eventId||arm.eventId===input.publish.eventId)&&result.event.sourcePrincipalId===arm.owner.principalId&&result.event.sourceWorldToken===input.scope.worldToken;
}
export function validateFixtureC1Rules(input:{expectedRevision?:number;rules?:Rules}){
 if(!input||Object.keys(input).some(k=>!['expectedRevision','rules'].includes(k)))throw Error('FIXTURE_C1_RULES');
 if(input.rules===undefined&&input.expectedRevision!==undefined)throw Error('FIXTURE_C1_RULES');
 if(input.rules!==undefined){validate('Rules',input.rules);if(!Number.isInteger(input.expectedRevision)||input.expectedRevision!<1||input.expectedRevision!>0xffffffff)throw Error('FIXTURE_C1_RULES');}
}
export async function fixtureC1Rules(input:{expectedRevision?:number;rules?:Rules},send:(payload:any)=>Promise<any>){
 validateFixtureC1Rules(input);
 const sanitize=(value:any)=>{validate('Response',value);if(value.error)throw Error('FIXTURE_C1_'+value.error.code);if(!value.snapshot||typeof value.sequence!=='string')throw Error('FIXTURE_C1_RESPONSE');return {snapshot:structuredClone(value.snapshot),sequence:value.sequence};};
 const before=sanitize(await send({rulesGet:{}}));if(input.rules===undefined)return before;
 // Preserve unrelated C1 keys. Production still authorizes the host and CAS.
 return sanitize(await send({rulesSet:{expectedRevision:input.expectedRevision,rules:{...before.snapshot.rules,...input.rules}}}));
}
export class LifecycleC1Controls {
 constructor(readonly family:'c1'|'c2'='c1'){}
 private codec(buffer:unknown,name:string){return this.family==='c1'?(Buffer.isBuffer(buffer)?decode(name,buffer):validate(name,buffer)):(Buffer.isBuffer(buffer)?decodeC2(name,buffer):validateC2(name,buffer));}
 private service:any;private active:Arm|undefined;private matched=new WeakMap<object,Arm>();private owners=new Map<Target,Owner>();private arms=0;private audits=0;private history:any[]=[];private activated=false;private closed=false;
 get enabled(){return this.activated&&!this.closed;}
 activate(runtime:any){if(runtime?.fixtureOnly!==true||runtime.fixtureNative?.private!==true||runtime[this.family+'Fixture']?.revision!==(this.family==='c1'?'c1-player-private-1':'c2-world-private-1')||runtime[this.family+'Fixture'].installed!==false)throw Error('FIXTURE_C1_SELECTOR');this.activated=true;}
 private require(){if(!this.enabled)throw Error('FIXTURE_C1_DISABLED');}
 bind(target:Target,credentials:Credentials,roomId:string){this.require();if(!['consumer','peer'].includes(target)||credentials.kind!=='desktop'||!uuid(credentials.accountId)||!uuid(roomId)||typeof credentials.deviceId!=='string'||!credentials.deviceId.length||credentials.deviceId.length>128)throw Error('FIXTURE_C1_OWNER');if([...this.owners].some(([role,o])=>role!==target&&o.principalId===credentials.accountId&&o.deviceId===credentials.deviceId))throw Error('FIXTURE_C1_OWNER');const old=this.owners.get(target);if(old&&(old.principalId!==credentials.accountId||old.deviceId!==credentials.deviceId))throw Error('FIXTURE_C1_OWNER');this.owners.set(target,{principalId:credentials.accountId!,deviceId:credentials.deviceId,roomId});if(this.active?.target===target&&this.active.roomId!==roomId)this.release();}
 arm(value:Drop,runId:string,roomId:string){this.require();if(!value||Object.keys(value).some(k=>!['target','eventId'].includes(k))||!['consumer','peer'].includes(value.target)||value.eventId!==undefined&&!uuid(value.eventId)||!uuid(runId)||!uuid(roomId)||this.active||this.arms>=C1_FIXTURE_LIMITS.dropArms)throw Error('FIXTURE_C1_DROP');const owner=this.owners.get(value.target);if(!owner||owner.roomId!==roomId)throw Error('FIXTURE_C1_OWNER');this.active={...value,armId:randomUUID(),runId,roomId,owner:{...owner}};this.arms++;return this.status();}
 release(){this.active=undefined;}
 status(){const a=this.active;return {armsUsed:this.arms,armLimit:C1_FIXTURE_LIMITS.dropArms,armed:a?{armId:a.armId,target:a.target,runId:a.runId,...(a.eventId?{eventId:a.eventId}:{})}:null,events:structuredClone(this.history)};}
 private role(principalId:string,deviceId:string,roomId:string){return [...this.owners].find(([,o])=>o.roomId===roomId&&o.principalId===principalId&&o.deviceId===deviceId)?.[0];}
 async audit(runId:string,roomId:string,eventId?:string){this.require();if(!uuid(runId)||!uuid(roomId)||eventId!==undefined&&!uuid(eventId)||this.audits++>=C1_FIXTURE_LIMITS.audits)throw Error('FIXTURE_C1_AUDIT');const a=this.owners.get('consumer'),b=this.owners.get('peer');if(a?.roomId!==roomId)throw Error('FIXTURE_C1_OWNER');
  if(this.family==='c2')return c2FixtureAudit(this.service,[...this.owners].map(([target,o])=>({target,...o})),runId,roomId,eventId,this.status());
  return this.service.db.transaction(async(tx:any)=>{
   const rows=(await tx.query(`SELECT e.event_id,e.source_principal,e.source_world,e.sequence,e.rules_revision,e.source_state,w.device_id,(SELECT key FROM jsonb_object_keys(e.body->'payload') AS key LIMIT 1) AS kind
    FROM c1_events e JOIN world_associations w ON w.run_id=e.run_id AND w.world_id::text=e.source_world AND w.account_id=e.source_principal
    WHERE e.run_id=$1 AND e.room_id=$2 AND ((e.source_principal=$3 AND w.device_id=$4) OR (e.source_principal=$5 AND w.device_id=$6)) AND ($7::uuid IS NULL OR e.event_id=$7) ORDER BY e.sequence DESC LIMIT 65`,[runId,roomId,a.principalId,a.deviceId,b?.roomId===roomId?b.principalId:null,b?.roomId===roomId?b.deviceId:null,eventId??null])).rows;
   const visible=rows.slice(0,C1_FIXTURE_LIMITS.events),ids=visible.map((r:any)=>r.event_id);
   const deliveries=ids.length?(await tx.query(`SELECT d.event_id,d.recipient_principal,d.recipient_world,d.state,d.reason,d.issued,w.device_id FROM c1_deliveries d JOIN world_associations w ON w.run_id=d.run_id AND w.world_id::text=d.recipient_world AND w.account_id=d.recipient_principal WHERE d.run_id=$1 AND d.event_id=ANY($2::uuid[]) ORDER BY d.sequence,d.recipient_world LIMIT 129`,[runId,ids])).rows:[];
   const lives=(await tx.query(`SELECT l.principal_id,l.world_token,l.state,l.death_event,l.rules_revision,w.device_id FROM c1_life l JOIN runs n ON n.id=l.run_id JOIN world_associations w ON w.run_id=l.run_id AND w.world_id::text=l.world_token AND w.account_id=l.principal_id WHERE l.run_id=$1 AND n.room_id=$2 ORDER BY l.world_token LIMIT 65`,[runId,roomId])).rows;
   return {runId,roomId,events:visible.reverse().map((r:any)=>({eventId:r.event_id,source:this.role(r.source_principal,r.device_id,roomId),sourceWorldToken:r.source_world,sequence:String(r.sequence),rulesRevision:Number(r.rules_revision),sourceDisposition:r.source_state,kind:['heart','orb','perk','fungal','death'].includes(r.kind)?r.kind:'other',recipients:deliveries.filter((v:any)=>v.event_id===r.event_id&&this.role(v.recipient_principal,v.device_id,roomId)).map((v:any)=>({target:this.role(v.recipient_principal,v.device_id,roomId),worldToken:v.recipient_world,state:v.state,reason:v.reason===''||C1_SKIP_REASONS.includes(v.reason)?v.reason:'other',issued:!!v.issued}))})),lives:lives.filter((r:any)=>this.role(r.principal_id,r.device_id,roomId)).slice(0,64).map((r:any)=>({target:this.role(r.principal_id,r.device_id,roomId),worldToken:r.world_token,state:r.state,deathEventId:r.death_event,rulesRevision:Number(r.rules_revision)})),truncated:rows.length>64||deliveries.length>128||lives.length>64,drop:this.status()};
  },true,AbortSignal.timeout(2500));
 }
 install(service:any){this.service=service;service.app.addHook('preClose',async()=>{this.closed=true;this.release();});
 service.app.addHook('preHandler',async(req:any)=>{
  const arm=this.active;if(!this.enabled||!arm||req.url!=='/v3/'+this.family+'/rpc')return;
  try{const input=this.codec(req.body,'Request') as Request;if(!input.publish||input.scope?.roomId!==arm.roomId||input.scope?.runId!==arm.runId||arm.eventId&&input.publish.eventId!==arm.eventId)return;
   const token=req.headers.authorization?.match(/^Bearer ([^ ]{1,4096})$/)?.[1];if(!token)return;
   const actor=await service.db.transaction((tx:any)=>service.auth.authenticate(tx,token),true,AbortSignal.timeout(2500),'control');
   if(this.active===arm&&actor.kind==='game'&&actor.accountId===arm.owner.principalId&&actor.deviceId===arm.owner.deviceId)this.matched.set(req,arm);
  }catch{/* An invalid request still follows the unchanged production authorization path. */}
 });
 service.app.addHook('onSend',async(req:any,reply:any,payload:any)=>{
  const arm=this.active;if(!this.enabled||!arm||req.url!=='/v3/'+this.family+'/rpc'||reply.statusCode!==200)return payload;
  let input:any,result:any;try{input=this.codec(req.body,'Request');result=this.codec(Buffer.isBuffer(payload)?payload:JSON.parse(String(payload)),'Response');}catch{return payload;}
  if(this.matched.get(req)!==arm||!matchesC1PublishReply(arm,input,result))return payload;this.active=undefined;this.history.push({armId:arm.armId,target:arm.target,eventId:input.publish.eventId,sequence:result.event.sequence,phase:'reply',effect:'drop',committed:true,duplicate:false});if(this.history.length>C1_FIXTURE_LIMITS.dropHistory)this.history.shift();reply.hijack();reply.raw.destroy();return null;
 });}
}
