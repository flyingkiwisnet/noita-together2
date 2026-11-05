import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {RACE_GOAL_SCHEMA,RACE_REGISTRY,validateGoal,validateGoalSet,validateGoalEnvironment,validateObservation,goalDifficulty,randomEligible} from '../src/g3-race-goals.js';
import type {GoalRegistry,Goal} from '../src/g3-race-goals.js';
import {createGoalEvaluation,acceptGoalObservation,rankRaceEntrants} from '../../domain/src/g3-race-goals.js';
import type {RaceGoalScope,RaceRankingInput} from '../../domain/src/g3-race-goals.js';
const fixture=JSON.parse(readFileSync(new URL('../g3-race-goals/fixture-inputs.json',import.meta.url),'utf8'));
const registry=fixture.registry as GoalRegistry;
const scope:RaceGoalScope={roomId:'20000000-0000-4000-8000-000000000001',runId:'20000000-0000-4000-8000-000000000002',runEpoch:'20000000-0000-4000-8000-000000000003',participantId:'20000000-0000-4000-8000-000000000004',worldToken:'fixture-owned-world',configRevision:1};
for(const f of fixture.evaluations)test('Race fixture: '+f.name,()=>{
 let state=createGoalEvaluation(scope,[f.goal],null,registry);const original=structuredClone(state);
 for(const [i,event] of f.events.entries())state=acceptGoalObservation(state,scope,event,(i+1)*100,registry);
 assert.deepEqual(state.progress[0],{instanceId:f.goal.instanceId,...f.expected,credits:state.progress[0]!.credits});
 assert.equal(new Set(state.progress[0]!.credits.map(c=>c.id)).size,state.progress[0]!.credits.length);
 assert.deepEqual(original.progress[0]!.credits,[]);assert.equal(original.progress[0]!.complete,false);
});
test('Race strict configuration bounds and known distinct IDs',()=>{
 for(const [id,min,max] of [['collect_orbs',1,33],['reach_max_hp',500,25000],['accumulate_gold',10000,1000000]] as const){
  for(const target of [min,max])validateGoal({instanceId:'goal',id,parameters:{target}},registry);
  for(const target of [min-1,max+1,NaN,Infinity,1.5,'500'])assert.throws(()=>validateGoal({instanceId:'goal',id,parameters:{target}},registry));
 }
 const named=(bossIds:unknown)=>({instanceId:'named',id:'defeat_named_bosses',parameters:{bossIds}});
 for(const ids of [[],['boss_a','boss_a'],['unknown'],['unverified'],new Array(17).fill('boss_a')])assert.throws(()=>validateGoal(named(ids),registry));
 validateGoal(named(['boss_a']),registry);
 assert.throws(()=>validateGoal({instanceId:'x',id:'visit_biome',parameters:{biomeId:'unknown'}},registry));
 assert.throws(()=>validateGoal({instanceId:'x',id:'defeat_bosses',parameters:{target:5}},registry));
 for(const parameters of [{direction:'north',index:1},{direction:'east',index:0},{direction:'east',index:11}])assert.throws(()=>validateGoal({instanceId:'pw',id:'visit_parallel_world',parameters},registry));
 for(const value of [{instanceId:'x',id:'classic_ending',parameters:{wrong:true}},{instanceId:'x',id:'unknown',parameters:{}},{instanceId:'x',id:'classic_ending',parameters:{},winner:true},null])assert.throws(()=>validateGoal(value,registry));
 assert.throws(()=>validateGoalSet([],registry));assert.throws(()=>validateGoalSet(new Array(17).fill(fixture.ranking.goals[0]),registry));
});
test('Race named boss order does not change meaning; overlaps explicit and not random',()=>{
 const named:Goal={instanceId:'named',id:'defeat_named_bosses',parameters:{bossIds:['boss_a','boss_b']}};
 const reordered:Goal={...named,instanceId:'other',parameters:{bossIds:['boss_b','boss_a']}};
 assert.throws(()=>validateGoalSet([named,reordered],registry));
 const generic:Goal={instanceId:'generic',id:'defeat_bosses',parameters:{target:1}};
 assert.deepEqual(validateGoalSet([named,generic],registry).warnings,['overlapping_goals']);
 assert.throws(()=>validateGoalSet([named,generic],registry,true));
 let state=createGoalEvaluation(scope,[named,generic],null,registry);
 state=acceptGoalObservation(state,scope,{kind:'boss',bossId:'boss_a'},100,registry);
 state=acceptGoalObservation(state,scope,{kind:'boss',bossId:'boss_a'},200,registry);
 assert.deepEqual(state.progress.map(p=>[p.value,p.complete,p.completedAtMs]),[[1,false,null],[1,true,100]]);
 assert.deepEqual(state.progress[0]!.credits,[{id:'boss_a',firstAtMs:100}]);
});
test('Race malformed observations cannot inflate progress or submit totals',()=>{
 for(const o of [{kind:'orb',orbId:999},{kind:'orb',orbId:0,total:33},{kind:'boss',bossId:'unknown'},{kind:'boss',bossId:'unverified'},{kind:'biome',biomeId:'unknown'},{kind:'gold',value:Infinity},{kind:'gold',value:1.5},{kind:'gold',value:-1},{kind:'gold',value:2147483648},{kind:'max_hp',value:NaN},{kind:'parallel',direction:'east',index:0},{kind:'ending',ending:'arbitrary'},null])assert.throws(()=>validateObservation(o,registry));
 validateObservation({kind:'max_hp',value:499.5},registry);
});
test('Race difficulty depends on configured threshold; extreme/terminal excluded from random',()=>{
 const goal=(id:string,target:number)=>validateGoal({instanceId:'g',id,parameters:{target}},registry);
 assert.equal(goalDifficulty(goal('reach_max_hp',500),registry),'easy');assert.equal(goalDifficulty(goal('reach_max_hp',25000),registry),'extreme');
 assert.equal(randomEligible(goal('collect_orbs',33),registry),false);assert.equal(randomEligible(goal('collect_orbs',11),registry),true);
 assert.equal(randomEligible(goal('reach_max_hp',25000),registry),false);
 assert.equal(randomEligible({instanceId:'end',id:'classic_ending',parameters:{}},registry),false);
 assert.equal(goalDifficulty({instanceId:'named',id:'defeat_named_bosses',parameters:{bossIds:['boss_a','boss_b','boss_c']}},registry),'extreme');
 const hp=goal('reach_max_hp',500);assert.throws(()=>validateGoalSet([hp,{...hp,instanceId:'hp2',parameters:{target:1000}}],registry,true));
 assert.ok(validateGoalSet([hp,{...hp,instanceId:'hp2',parameters:{target:1000}}],registry).warnings.includes('overlapping_goals'));
});
test('Race own-world scope, registry revision, accepted timing and strict deadline',()=>{
 const state=createGoalEvaluation(scope,[fixture.ranking.goals[0]],1000,registry);
 for(const patch of [{participantId:'20000000-0000-4000-8000-000000000009'},{runId:'20000000-0000-4000-8000-000000000009'},{runEpoch:'20000000-0000-4000-8000-000000000009'},{worldToken:'other'},{configRevision:2}])assert.throws(()=>acceptGoalObservation(state,{...scope,...patch},{kind:'max_hp',value:500},100,registry));
 assert.throws(()=>acceptGoalObservation(state,scope,{kind:'max_hp',value:500},100,{...registry,revision:'other'}));
 const before=acceptGoalObservation(state,scope,{kind:'max_hp',value:499},999,registry);
 assert.throws(()=>acceptGoalObservation(before,scope,{kind:'max_hp',value:500},1000,registry));
 assert.throws(()=>acceptGoalObservation(before,scope,{kind:'max_hp',value:500},1001,registry));
 assert.throws(()=>acceptGoalObservation(before,scope,{kind:'max_hp',value:500},998,registry));
 assert.equal(acceptGoalObservation(before,scope,{kind:'max_hp',value:500},999,registry).progress[0]!.completedAtMs,999);
 assert.throws(()=>createGoalEvaluation(scope,[fixture.ranking.goals[0]],0,registry));
 assert.throws(()=>createGoalEvaluation(scope,[fixture.ranking.goals[0]],RACE_GOAL_SCHEMA.limits.durationMs+1,registry));
});
test('Race deadline fixture: full order, count, time to final count, exact/zero ties',()=>{
 const f=fixture.ranking,result=rankRaceEntrants(f.goals,f.entrants,registry);
 assert.deepEqual(result.map(p=>p.participantId.slice(-3)),f.expectedSuffixOrder);
 assert.deepEqual(result.map(p=>p.rank),f.expectedRanks);assert.deepEqual(result.map(p=>p.finalCountAtMs),f.expectedCountTimes);
 // Count-2 time500 wins over time600 even though its sum990 is greater than601.
 // Full finishers with equal timestamp retain their distinct original order.
 const nobodyFinished=rankRaceEntrants(f.goals,f.entrants.slice(2),registry);
 assert.deepEqual(nobodyFinished.slice(0,2).map(p=>p.rank),[1,1]);
 const reversed=rankRaceEntrants(f.goals,[...f.entrants.slice(2)].reverse(),registry);
 assert.equal(reversed.find(p=>p.participantId.endsWith('007'))!.rank,reversed.find(p=>p.participantId.endsWith('008'))!.rank);
 assert.equal(result[2]!.status,'dead');assert.equal(result[4]!.status,'dnf');
 assert.deepEqual(rankRaceEntrants(f.goals,f.entrants,registry,2000),result);
 assert.throws(()=>rankRaceEntrants(f.goals,f.entrants,registry,1000));
});
test('Race standings reject duplicate/unknown goals, accounts, ordinals and invented completion',()=>{
 const f=fixture.ranking;
 const bad:((a:RaceRankingInput[])=>void)[]=[a=>{a[1]!.participantId=a[0]!.participantId},a=>{a[1]!.finishOrder=1},a=>{a[0]!.finishOrder=4},a=>{a[2]!.completions.push(a[2]!.completions[0]!)},a=>{a[2]!.completions[0]!.instanceId='unknown'},a=>{a[2]!.completions[0]!.elapsedMs=-1},a=>{a[0]!.completions.pop()},a=>{a[2]!.finishOrder=3},a=>{a[2]!.completions.push({instanceId:'orbs',elapsedMs:700})}];
 for(const mutate of bad){const a=structuredClone(f.entrants);mutate(a);assert.throws(()=>rankRaceEntrants(f.goals,a,registry));}
});
test('Race production catalogue never accepts the synthetic fixture IDs',()=>{
 assert.throws(()=>validateObservation({kind:'boss',bossId:'boss_a'}));
 assert.throws(()=>validateGoal({instanceId:'x',id:'visit_biome',parameters:{biomeId:'test_biome'}}));
 for(const entry of [...RACE_REGISTRY.bosses,...RACE_REGISTRY.biomes])if(entry.available)assert.ok(entry.detector&&entry.name&&entry.prerequisites);
 assert.equal(RACE_REGISTRY.revision,RACE_GOAL_SCHEMA.revision);
});
test('Race source-backed catalogue prerequisites and impossible ending combination',()=>{
 const ending:Goal={instanceId:'ending',id:'classic_ending',parameters:{}};
 const orbs:Goal={instanceId:'orbs',id:'collect_orbs',parameters:{target:33}};
 validateGoal(orbs);assert.throws(()=>validateGoalSet([ending,orbs]));
 assert.ok(validateGoalSet([ending,{...orbs,parameters:{target:11}}]).warnings.includes('complete_other_goals_before_classic_ending'));
 assert.throws(()=>validateObservation({kind:'orb',orbId:13}));
 const pw:Goal={instanceId:'east',id:'visit_parallel_world',parameters:{direction:'east',index:1}};
 validateGoalEnvironment([pw],{worldLayout:'vanilla_default',newGamePlus:false});
 assert.throws(()=>validateGoalEnvironment([pw],{worldLayout:'unverified',newGamePlus:false}));
 assert.throws(()=>validateGoalEnvironment([orbs],{worldLayout:'vanilla_default',newGamePlus:true}));
 assert.throws(()=>createGoalEvaluation(scope,[pw],null,RACE_REGISTRY,{worldLayout:'unverified',newGamePlus:false}));
 const named=validateGoal({instanceId:'named',id:'defeat_named_bosses',parameters:{bossIds:['kolmisilma','pyramid_boss']}});
 let state=createGoalEvaluation(scope,[named]);
 state=acceptGoalObservation(state,scope,{kind:'boss',bossId:'kolmisilma'},10);
 state=acceptGoalObservation(state,scope,{kind:'boss',bossId:'high_alchemist'},20);
 state=acceptGoalObservation(state,scope,{kind:'boss',bossId:'pyramid_boss'},30);
 assert.equal(state.progress[0]!.completedAtMs,30);assert.equal(state.progress[0]!.value,2);
 const unavailable={...RACE_REGISTRY,goalSupport:{...RACE_REGISTRY.goalSupport,collect_orbs:{available:false,reason:'diagnostic unavailable'}}};
 assert.throws(()=>validateGoal(orbs,unavailable));
 assert.throws(()=>validateGoal(orbs,{...RACE_REGISTRY,orbs:[0]}));
});
test.skip('Historical private detector-review receipt is not included in public source export',()=>{});
