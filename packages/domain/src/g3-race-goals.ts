// Pure, immutable evaluation of already authenticated, eligible original facts.
// This is not a gameplay RPC or an admission controller. The future service must
// resolve accepted-original retries before current-state/deadline checks, as D1 does.
import {RACE_GOAL_SCHEMA,RACE_REGISTRY,validateGoalSet,validateGoalEnvironment,validateObservation,need,exact,bounded} from '../../contracts/src/g3-race-goals.js';
import type {Goal,GoalRegistry,Observation,RaceGoalEnvironment} from '../../contracts/src/g3-race-goals.js';
export interface RaceGoalScope {roomId:string;runId:string;runEpoch:string;configRevision:number;participantId:string;worldToken:string}
export interface GoalCredit {id:string;firstAtMs:number}
export interface GoalProgress {
 instanceId:string;value:number;currentValue:number;target:number;complete:boolean;completedAtMs:number|null;credits:GoalCredit[];
}
export interface GoalEvaluation {
 registryRevision:string;scope:RaceGoalScope;goals:Goal[];progress:GoalProgress[];
 lastAcceptedElapsedMs:number;durationMs:number|null;environment:RaceGoalEnvironment;
}
const uuid=(v:unknown)=>typeof v==='string'&&/^[0-9a-f]{8}-[0-9a-f]{4}-[1-8][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/.test(v);
function validateScope(v:RaceGoalScope){
 exact(v,['roomId','runId','runEpoch','configRevision','participantId','worldToken'],'INVALID_SCOPE');
 need([v.roomId,v.runId,v.runEpoch,v.participantId].every(uuid)&&bounded(v.configRevision,1,0xffffffff)&&
  typeof v.worldToken==='string'&&v.worldToken.length>=1&&Buffer.byteLength(v.worldToken)<=128&&!/[\u0000-\u001f]/.test(v.worldToken),'INVALID_SCOPE');
}
export function createGoalEvaluation(scope:RaceGoalScope,goals:unknown,durationMs:number|null=null,registry:GoalRegistry=RACE_REGISTRY,environment:RaceGoalEnvironment={worldLayout:'vanilla_default',newGamePlus:false}):GoalEvaluation {
 validateScope(scope);need(durationMs===null||bounded(durationMs,1,RACE_GOAL_SCHEMA.limits.durationMs));
 const checked=validateGoalEnvironment(goals,environment,registry).goals;
 return {registryRevision:registry.revision,scope:structuredClone(scope),goals:checked,durationMs,environment:structuredClone(environment),lastAcceptedElapsedMs:0,
  progress:checked.map(g=>({instanceId:g.instanceId,value:0,currentValue:0,target:'target' in g.parameters?g.parameters.target:
   g.id==='defeat_named_bosses'?g.parameters.bossIds.length:g.id==='visit_parallel_world'?g.parameters.index:1,
   complete:false,completedAtMs:null,credits:[]}))};
}
function credit(p:GoalProgress,id:string,elapsedMs:number){if(!p.credits.some(c=>c.id===id))p.credits.push({id,firstAtMs:elapsedMs});}
export function acceptGoalObservation(state:GoalEvaluation,scope:RaceGoalScope,input:unknown,acceptedElapsedMs:number,registry:GoalRegistry=RACE_REGISTRY):GoalEvaluation {
 validateScope(scope);need(state.registryRevision===registry.revision,'INVALID_SCOPE');
 need((Object.keys(state.scope) as (keyof RaceGoalScope)[]).every(k=>state.scope[k]===scope[k]),'INVALID_SCOPE');
 need(bounded(acceptedElapsedMs,state.lastAcceptedElapsedMs,RACE_GOAL_SCHEMA.limits.elapsedMs),'LATE_OBSERVATION');
 need(state.durationMs===null||acceptedElapsedMs<state.durationMs,'LATE_OBSERVATION');
 const observation=validateObservation(input,registry),next=structuredClone(state);
 for(let i=0;i<next.goals.length;i++)evaluate(next.goals[i]!,next.progress[i]!,observation,acceptedElapsedMs);
 next.lastAcceptedElapsedMs=acceptedElapsedMs;return next;
}
function evaluate(g:Goal,p:GoalProgress,o:Observation,elapsedMs:number){
 switch(g.id){
  case 'collect_orbs':if(o.kind==='orb'){credit(p,String(o.orbId),elapsedMs);p.value=p.currentValue=p.credits.length;}break;
  case 'defeat_bosses':if(o.kind==='boss'){credit(p,o.bossId,elapsedMs);p.value=p.currentValue=p.credits.length;}break;
  case 'defeat_named_bosses':if(o.kind==='boss'&&g.parameters.bossIds.includes(o.bossId)){credit(p,o.bossId,elapsedMs);p.value=p.currentValue=p.credits.length;}break;
  case 'visit_biome':if(o.kind==='biome'&&o.biomeId===g.parameters.biomeId){credit(p,o.biomeId,elapsedMs);p.value=p.currentValue=1;}break;
  case 'visit_parallel_world':if(o.kind==='parallel'&&o.direction===g.parameters.direction){p.value=Math.max(p.value,o.index);p.currentValue=o.index;}break;
  case 'reach_max_hp':if(o.kind==='max_hp'){p.currentValue=o.value;p.value=Math.max(p.value,o.value);}break;
  case 'accumulate_gold':if(o.kind==='gold'){p.currentValue=o.value;p.value=Math.max(p.value,o.value);}break;
  case 'classic_ending':if(o.kind==='ending'&&o.ending==='classic'){credit(p,'classic',elapsedMs);p.value=p.currentValue=1;}break;
 }
 if(!p.complete&&p.value>=p.target){p.complete=true;p.completedAtMs=elapsedMs;}
}
export interface EntrantGoalCompletion {instanceId:string;elapsedMs:number}
export interface RaceRankingInput {
 participantId:string;status:'finished'|'racing'|'dead'|'dnf';finishOrder:number|null;completions:EntrantGoalCompletion[];
}
export interface RaceRankedEntry extends RaceRankingInput {rank:number;completedGoals:number;finalCountAtMs:number|null}
// Server acceptance order is an immutable ordinal for full finishers, even when
// timestamps have equal resolution. Unfinished entrants have no ordinal tie-break.
// Caller finalizes outcomes/deadline transactionally; no local clock is accepted here.
export function rankRaceEntrants(goals:readonly Goal[],input:readonly RaceRankingInput[],registry:GoalRegistry=RACE_REGISTRY,durationMs:number|null=null):RaceRankedEntry[] {
 validateGoalSet(goals,registry);need(Array.isArray(input as unknown)&&input.length<=RACE_GOAL_SCHEMA.limits.entrants,'INVALID_STANDINGS');
 need(durationMs===null||bounded(durationMs,1,RACE_GOAL_SCHEMA.limits.durationMs),'INVALID_STANDINGS');
 const participants=new Set<string>(),orders=new Set<number>(),instances=new Set(goals.map(g=>g.instanceId));
 const entries=input.map(p=>{
  exact(p,['participantId','status','finishOrder','completions'],'INVALID_STANDINGS');
  need(uuid(p.participantId)&&!participants.has(p.participantId)&&['finished','racing','dead','dnf'].includes(p.status),'INVALID_STANDINGS');participants.add(p.participantId);
  need(Array.isArray(p.completions)&&p.completions.length<=goals.length,'INVALID_STANDINGS');const seen=new Set<string>();
  for(const c of p.completions){exact(c,['instanceId','elapsedMs'],'INVALID_STANDINGS');need(instances.has(c.instanceId)&&!seen.has(c.instanceId)&&bounded(c.elapsedMs,0,RACE_GOAL_SCHEMA.limits.elapsedMs),'INVALID_STANDINGS');seen.add(c.instanceId);}
  need(durationMs===null||p.completions.every(c=>c.elapsedMs<durationMs),'INVALID_STANDINGS');
  if(p.status==='finished'){need(p.completions.length===goals.length&&bounded(p.finishOrder,1,input.length)&&!orders.has(p.finishOrder),'INVALID_STANDINGS');orders.add(p.finishOrder);}
  else need(p.finishOrder===null&&p.completions.length<goals.length,'INVALID_STANDINGS');
  return {...structuredClone(p),rank:0,completedGoals:p.completions.length,finalCountAtMs:p.completions.length?Math.max(...p.completions.map(c=>c.elapsedMs)):null};
 });
 need([...orders].sort((a,b)=>a-b).every((n,i)=>n===i+1),'INVALID_STANDINGS');
 const compare=(a:RaceRankedEntry,b:RaceRankedEntry)=>{
  if(a.finishOrder!==null&&b.finishOrder!==null)return a.finishOrder-b.finishOrder;
  if(a.finishOrder!==null)return -1;if(b.finishOrder!==null)return 1;
  return b.completedGoals-a.completedGoals||(a.finalCountAtMs??0)-(b.finalCountAtMs??0);
 };
 // Stable input order is display order only: the comparator returns zero for
 // exact count/time ties and for every zero-goal pair, irrespective of identity.
 entries.sort(compare);for(let i=0;i<entries.length;i++)entries[i]!.rank=i>0&&compare(entries[i-1]!,entries[i]!)===0?entries[i-1]!.rank:i+1;
 return entries;
}
