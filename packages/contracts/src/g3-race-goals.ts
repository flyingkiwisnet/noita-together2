// Catalogue validation only. Authentication, original receipts and lifecycle ordering
// belong to the future service adapter, not to a client-supplied catalogue or total.
import {RACE_GOAL_SCHEMA,RACE_GOAL_CATALOGUE} from '../g3-race-goals/g3-race-goals.js';
import type {Goal,Observation,Difficulty} from '../g3-race-goals/g3-race-goals.js';
export {RACE_GOAL_SCHEMA,RACE_GOAL_CATALOGUE};
export type {Goal,Observation,Difficulty};
export interface ContentEntry {
 id:string;name:string;available:boolean;difficulty:Difficulty;randomEligible:boolean;
 aliases:readonly string[];prerequisites:readonly string[];detector:string;
}
export interface GoalRegistry {
 revision:string;goalSupport:Record<string,{available:boolean;reason:string}>;
 orbs:readonly number[];bosses:readonly ContentEntry[];biomes:readonly ContentEntry[];
}
export interface RaceGoalEnvironment {worldLayout:'vanilla_default'|'unverified';newGamePlus:boolean}
export const RACE_REGISTRY=RACE_GOAL_CATALOGUE as unknown as GoalRegistry;
export class RaceGoalError extends Error {
 constructor(readonly code:'INVALID_GOAL'|'UNAVAILABLE_CONTENT'|'INVALID_OBSERVATION'|'INVALID_SCOPE'|'LATE_OBSERVATION'|'INVALID_STANDINGS'){super(code);}
}
export function need(v:unknown,code:RaceGoalError['code']='INVALID_GOAL'):asserts v {if(!v)throw new RaceGoalError(code);}
export function exact(v:unknown,keys:readonly string[],code:RaceGoalError['code']='INVALID_GOAL'):asserts v is Record<string,unknown> {
 need(v!==null&&typeof v==='object'&&!Array.isArray(v),code);
 need(Object.keys(v).sort().join(',')===[...keys].sort().join(','),code);
}
export function bounded(v:unknown,min:number,max:number,integer=true):v is number {
 return typeof v==='number'&&Number.isFinite(v)&&(!integer||Number.isSafeInteger(v))&&v>=min&&v<=max;
}
export const stableId=(v:unknown):v is string=>typeof v==='string'&&/^[a-z][a-z0-9_]{0,63}$/.test(v);
type ParameterField={type:'integer';min:number;max:number}|{type:'enum';values:readonly string[]}|{type:'id';registry:'bosses'|'biomes'}|{type:'ids';registry:'bosses'|'biomes';max:number};
export function validateGoal(input:unknown,registry:GoalRegistry=RACE_REGISTRY):Goal {
 exact(input,['instanceId','id','parameters']);need(stableId(input.instanceId)&&typeof input.id==='string');
 const entry=RACE_GOAL_SCHEMA.goals.find(e=>e.id===input.id);need(entry);
 need(registry.goalSupport[entry.id]?.available,'UNAVAILABLE_CONTENT');
 exact(input.parameters,Object.keys(entry.parameters));
 for(const [key,field] of Object.entries(entry.parameters as Record<string,ParameterField>)){
  const value=input.parameters[key];
  if(field.type==='integer')need(bounded(value,field.min,field.max));
  else if(field.type==='enum')need(typeof value==='string'&&(field.values as readonly string[]).includes(value));
  else {
   const entries=registry[field.registry];
   const ids=field.type==='ids'?value:[value];
   need(Array.isArray(ids)&&ids.length>0&&ids.length<=(field.type==='ids'?field.max:1)&&new Set(ids).size===ids.length);
   for(const id of ids){need(stableId(id));need(entries.some(e=>e.id===id&&e.available),'UNAVAILABLE_CONTENT');}
  }
 }
 if(entry.id==='defeat_bosses')need((input.parameters.target as number)<=registry.bosses.filter(e=>e.available).length,'UNAVAILABLE_CONTENT');
 if(entry.id==='collect_orbs')need((input.parameters.target as number)<=registry.orbs.length,'UNAVAILABLE_CONTENT');
 return structuredClone(input) as unknown as Goal;
}
const levels:Difficulty[]=['easy','medium','hard','extreme'];
export function goalDifficulty(goal:Goal,registry:GoalRegistry=RACE_REGISTRY):Difficulty {
 validateGoal(goal,registry);
 if(goal.id==='defeat_named_bosses'){
  const selected=registry.bosses.filter(e=>goal.parameters.bossIds.includes(e.id));
  const hardest=Math.max(...selected.map(e=>levels.indexOf(e.difficulty)));
  // A checklist adds travel/access effort; never infer that any-three equals named-three.
  return levels[Math.min(3,hardest+(selected.length>=3?1:0))]!;
 }
 if(goal.id==='visit_biome')return registry.biomes.find(e=>e.id===goal.parameters.biomeId)!.difficulty;
 const entry=RACE_GOAL_SCHEMA.goals.find(e=>e.id===goal.id)!;
 if('difficulty' in entry)return entry.difficulty;
 const target=goal.id==='visit_parallel_world'?goal.parameters.index:'target' in goal.parameters?goal.parameters.target:0;
 if('difficultyBands' in entry)return entry.difficultyBands.find(([max])=>target<=max)![1];
 throw new RaceGoalError('INVALID_GOAL');
}
export function randomEligible(goal:Goal,registry:GoalRegistry=RACE_REGISTRY):boolean {
 validateGoal(goal,registry);if(goalDifficulty(goal,registry)==='extreme')return false;
 if(goal.id==='classic_ending')return false; // Terminal access/order needs deliberate host review.
 if(goal.id==='defeat_named_bosses')return goal.parameters.bossIds.every(id=>registry.bosses.some(e=>e.id===id&&e.randomEligible));
 if(goal.id==='visit_biome')return registry.biomes.some(e=>e.id===goal.parameters.biomeId&&e.randomEligible);
 if(goal.id==='defeat_bosses'&&registry.bosses.filter(e=>e.available&&e.randomEligible).length<goal.parameters.target)return false;
 const entry=RACE_GOAL_SCHEMA.goals.find(e=>e.id===goal.id)!;
 const target=goal.id==='visit_parallel_world'?goal.parameters.index:'target' in goal.parameters?goal.parameters.target:0;
 return 'randomMax' in entry&&target<=entry.randomMax;
}
function signature(g:Goal){return g.id+':'+Object.entries(g.parameters).map(([k,v])=>k+'='+ (Array.isArray(v)?[...v].sort().join('|'):v)).sort().join(',');}
export function validateGoalSet(input:unknown,registry:GoalRegistry=RACE_REGISTRY,random=false):{goals:Goal[];warnings:string[]} {
 need(Array.isArray(input)&&input.length>=1&&input.length<=RACE_GOAL_SCHEMA.limits.goals);
 const goals=input.map(g=>validateGoal(g,registry));
 need(new Set(goals.map(g=>g.instanceId)).size===goals.length&&new Set(goals.map(signature)).size===goals.length);
 // The verified underground Classic branch explicitly requires fewer than33
 // orbs. Collecting33 first cannot be undone in the same single-life progression.
 need(!(goals.some(g=>g.id==='classic_ending')&&goals.some(g=>g.id==='collect_orbs'&&g.parameters.target>=33)));
 const warnings=new Set<string>();
 for(let i=0;i<goals.length;i++)for(const b of goals.slice(i+1)){
  const a=goals[i]!;
  const bossOverlap=(a.id==='defeat_bosses'||a.id==='defeat_named_bosses')&&(b.id==='defeat_bosses'||b.id==='defeat_named_bosses')&&
   (a.id==='defeat_bosses'||b.id==='defeat_bosses'||a.parameters.bossIds.some(id=>b.parameters.bossIds.includes(id)));
  const nearDuplicate=a.id===b.id&&a.id!=='visit_biome'&&a.id!=='defeat_named_bosses'&&
   (a.id!=='visit_parallel_world'||b.id!=='visit_parallel_world'||a.parameters.direction===b.parameters.direction);
  if(bossOverlap||nearDuplicate){need(!random);warnings.add('overlapping_goals');}
 }
 if(goals.some(g=>g.id==='classic_ending')&&goals.length>1)warnings.add('complete_other_goals_before_classic_ending');
 if(random)need(goals.every(g=>randomEligible(g,registry)));
 return {goals,warnings:[...warnings]};
}
export function validateGoalEnvironment(goals:unknown,environment:RaceGoalEnvironment,registry:GoalRegistry=RACE_REGISTRY){
 exact(environment,['worldLayout','newGamePlus']);
 need((environment.worldLayout==='vanilla_default'||environment.worldLayout==='unverified')&&typeof environment.newGamePlus==='boolean');
 const result=validateGoalSet(goals,registry);
 // No inferred continuity after NG+ clears orb/run facts. PW geometry is only
 // conservatively inferred for the default map; it is not an exact engine boundary.
 need(!environment.newGamePlus,'UNAVAILABLE_CONTENT');
 if(result.goals.some(g=>g.id==='visit_parallel_world'))need(environment.worldLayout==='vanilla_default','UNAVAILABLE_CONTENT');
 return result;
}
export function validateObservation(input:unknown,registry:GoalRegistry=RACE_REGISTRY):Observation {
 need(input!==null&&typeof input==='object'&&!Array.isArray(input),'INVALID_OBSERVATION');
 const o=input as Record<string,unknown>,entry=RACE_GOAL_SCHEMA.observations.find(e=>e.kind===o.kind);
 need(entry,'INVALID_OBSERVATION');exact(o,['kind',...Object.keys(entry.fields)],'INVALID_OBSERVATION');
 const check=(v:unknown)=>need(v,'INVALID_OBSERVATION');
 switch(o.kind){
  case 'orb':check(typeof o.orbId==='number'&&registry.orbs.includes(o.orbId));break;
  case 'boss':check(registry.bosses.some(e=>e.id===o.bossId&&e.available));break;
  case 'biome':check(registry.biomes.some(e=>e.id===o.biomeId&&e.available));break;
  case 'parallel':check((o.direction==='east'||o.direction==='west')&&bounded(o.index,1,RACE_GOAL_SCHEMA.limits.parallelIndex));break;
  case 'max_hp':check(bounded(o.value,0,RACE_GOAL_SCHEMA.limits.hpObservation,false));break;
  case 'gold':check(bounded(o.value,0,RACE_GOAL_SCHEMA.limits.goldObservation));break;
  case 'ending':check(o.ending==='classic'||o.ending==='other');break;
 }
 return structuredClone(o) as Observation;
}
