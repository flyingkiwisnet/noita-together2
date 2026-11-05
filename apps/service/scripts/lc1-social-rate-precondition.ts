// Explicit synthetic policy setup in the private fixture only. Both requests
// still run the real authenticated authority and normal transaction outcomes.
import {randomUUID} from 'node:crypto';import {SocialAuthority} from '../src/g3/social.js';import {fingerprint,PolicyError} from '../src/g1/security.js';import {afterCommit} from '../src/g1/commit-hooks.js';
import {transientEventId} from '../../../packages/contracts/src/g3-social-replay.js';
type Plan={target:'consumer'|'peer';worldId:string;activeEventId:string};
type Arm={id:string;plan:Plan;stage:'start'|'stop';binding?:string;stopEventId?:string};
const uuid=(v:unknown):v is string=>typeof v==='string'&&/^[a-f0-9]{8}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{12}$/i.test(v);
export class SocialRatePrecondition{
 private active:Arm|undefined;private used=0;private events:any[]=[];private restore:(()=>void)|undefined;private closed=false;
 constructor(private service:any,private owner:(target:Plan['target'],actor:any,roomId:string)=>boolean){}
 private event(a:Arm,kind:string,extra:any={}){this.events.push({armId:a.id,kind,synthetic:true,...a.plan,...extra});if(this.events.length>32)this.events.shift();}
 arm(value:unknown){const p=value as Plan;if(!p||typeof p!=='object'||Array.isArray(p)||Object.keys(p).sort().join()!=='activeEventId,target,worldId'||!['consumer','peer'].includes(p.target)||!uuid(p.worldId)||!uuid(p.activeEventId)||!/^g3_rejoin_[a-f0-9]{32}$/.test(this.service.db.namespace))throw Error('FIXTURE_SOCIAL_RATE_ARGUMENT');if(this.closed||this.active||this.used>=8)throw Error('FIXTURE_SOCIAL_RATE_BUSY');this.active={id:randomUUID(),plan:structuredClone(p),stage:'start'};this.used++;this.event(this.active,'armed');return {armId:this.active.id,synthetic:true};}
 status(){return {armsUsed:this.used,armLimit:8,active:this.active?{armId:this.active.id,...this.active.plan,stage:this.active.stage}:null,events:structuredClone(this.events)};}
 release(){if(this.active)this.event(this.active,'released');this.active=undefined;}
 close(){this.closed=true;this.release();this.restore?.();}
 install(){const controller=this,prototype=SocialAuthority.prototype,execute=prototype.execute;const observed=async function(this:SocialAuthority,tx:any,actor:any,input:any){const a=controller.active,p=input?.publish;
  if(!a||input.scope?.worldToken!==a.plan.worldId||!p?.emote||(p.emote.active===true)!==(a.stage==='start')||input.serviceId!==controller.service.db.serviceId||input.serviceEpoch!==controller.service.db.epoch||actor.kind!=='game'||actor.sessionEpoch!==input.sessionEpoch||!controller.owner(a.plan.target,actor,input.scope.roomId))return execute.call(this,tx,actor,input);
  if(!uuid(p.eventId)||!Number.isInteger(p.sequence)||p.sequence<1||p.eventId!==transientEventId(input.scope,actor.sessionId,'emote',p.sequence))return execute.call(this,tx,actor,input);
  const binding=fingerprint({scope:input.scope,accountId:actor.accountId,deviceId:actor.deviceId,sessionId:actor.sessionId,sessionEpoch:actor.sessionEpoch});
  if(a.binding&&a.binding!==binding||a.stopEventId&&a.stopEventId!==p.eventId){controller.event(a,'scope-or-original-changed');controller.active=undefined;return execute.call(this,tx,actor,input);}
  await (this as any).context(tx,actor,input); // Real current playing/capability scope.
  const identity=(await tx.query('SELECT current_database() AS database,current_user AS owner,current_schema() AS schema')).rows[0];if(identity.database!=='nt2_g1'||identity.owner!=='nt2_g1'||identity.schema!==controller.service.db.namespace)throw Error('FIXTURE_SOCIAL_RATE_SCOPE');
  const current=(await tx.query("SELECT body FROM g3_social_presentation WHERE run_id=$1 AND world_token=$2 AND principal_id=$3 AND kind='emote' FOR UPDATE",[input.scope.runId,a.plan.worldId,actor.accountId])).rows[0];
  const fence=(await tx.query("SELECT last_sequence,window_at,window_count FROM g3_social_fences WHERE run_id=$1 AND game_session_id=$2 AND kind='emote' FOR UPDATE",[input.scope.runId,actor.sessionId])).rows[0];
  if(controller.active!==a)return execute.call(this,tx,actor,input);
  if(current?.body.eventId!==a.plan.activeEventId||current.body.gameSessionId!==actor.sessionId||current.body.emote.active!==true||!fence){controller.event(a,'prior-active-original-changed');controller.active=undefined;return execute.call(this,tx,actor,input);}
  if(p.sequence<=Number(fence.last_sequence))return execute.call(this,tx,actor,input);
  const stage=a.stage;a.binding=binding;
  await tx.query("UPDATE g3_social_fences SET window_at=$3,window_count=4 WHERE run_id=$1 AND game_session_id=$2 AND kind='emote'",[input.scope.runId,actor.sessionId,new Date(controller.service.auth.now())]);
  if(controller.active!==a){await tx.query("UPDATE g3_social_fences SET window_at=$3,window_count=$4 WHERE run_id=$1 AND game_session_id=$2 AND kind='emote'",[input.scope.runId,actor.sessionId,fence.window_at,fence.window_count]);return execute.call(this,tx,actor,input);}
  controller.event(a,'synthetic-window-prefill',{stage,eventId:p.eventId,sequence:p.sequence,count:4});
  try{const result=await execute.call(this,tx,actor,input);if(stage==='start')throw new PolicyError('INVALID_MESSAGE','FIXTURE_SOCIAL_RATE_WINDOW_MISSED');
   if(result.acceptedEventId!==p.eventId)throw new PolicyError('INVALID_MESSAGE','FIXTURE_SOCIAL_RATE_STOP_UNMATCHED');a.stopEventId=p.eventId;
   afterCommit(tx,()=>{controller.event(a,'stop-committed',{eventId:p.eventId,sequence:p.sequence});if(controller.active===a)controller.active=undefined;},()=>{controller.event(a,'stop-commit-uncertain');if(controller.active===a)controller.active=undefined;});return result;
  }catch(e){if(stage==='start'&&e instanceof PolicyError&&e.code==='CAPACITY'){if(controller.active===a)a.stage='stop';controller.event(a,'start-refused-real-policy',{eventId:p.eventId,sequence:p.sequence,setupRollsBack:true});}else if(stage==='start'){controller.event(a,'setup-failed');if(controller.active===a)controller.active=undefined;}throw e;}
 };prototype.execute=observed;this.restore=()=>{if(prototype.execute===observed)prototype.execute=execute;};}
}
