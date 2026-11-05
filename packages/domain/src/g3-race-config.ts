import type {Config,RaceConfig,RaceGoal} from '../../contracts/g2-next/g2-next.js';
import type {Goal,Observation} from '../../contracts/src/g3-race-goals.js';
import {RACE_REGISTRY,validateGoalEnvironment,validateObservation,need,exact} from '../../contracts/src/g3-race-goals.js';
import type {Observation as WireObservation} from '../../contracts/g3-race/g3-race.js';
export const RACE_CAPABILITY='race.goals.v1';
export function raceCompatibility<T extends {modes:string[];capabilities:string[]}>(value:T):T {
 return {...value,modes:[...new Set([...value.modes,'race'])],capabilities:[...new Set([...value.capabilities,RACE_CAPABILITY])]};
}
export function pureRaceGoal(g:RaceGoal):Goal {
 const p=g.id==='classic_ending'?{}:g.id==='defeat_named_bosses'?{bossIds:g.bossIds}:g.id==='visit_biome'?{biomeId:g.biomeId}:g.id==='visit_parallel_world'?{direction:g.direction,index:g.index}:{target:g.target};
 exact(g,['instanceId','id',...Object.keys(p)]);
 return {instanceId:g.instanceId,id:g.id,parameters:p} as Goal;
}
export function validateRaceConfig(config:Config):RaceConfig|undefined {
 if(config.mode!=='race'){need(config.race===undefined);return;}
 const r=config.race;need(r);exact(r,['registryRevision','goals','durationMs']);
 need(r.registryRevision===RACE_REGISTRY.revision&&typeof r.durationMs==='number'&&Number.isInteger(r.durationMs)&&r.durationMs>=0&&r.durationMs<=604800000);
 need(config.requiredCapabilities?.includes(RACE_CAPABILITY)&&config.seedPolicy!=='individual-random');
 need(!(config.settings??[]).some(s=>(s.key?.startsWith('NT_sync_')||['NT_send_gold','NT_send_wands','NT_send_flasks','NT_send_items','NT_team_perks','NT_alt_heartache','NT_testing_mode'].includes(s.key??''))&&s.value==='true'));
 need(Array.isArray(r.goals));validateGoalEnvironment(r.goals.map(pureRaceGoal),{worldLayout:'vanilla_default',newGamePlus:false});return r;
}
export function pureRaceObservation(o:WireObservation):Observation {
 const result=o.kind==='max_hp'||o.kind==='gold'?{...o,value:Number(o.value)}:o;
 if(o.kind==='max_hp'||o.kind==='gold')need(typeof o.value==='string'&&/^(0|[1-9][0-9]{0,9})(\.[0-9]{1,20})?$/.test(o.value),'INVALID_OBSERVATION');
 return validateObservation(result);
}
