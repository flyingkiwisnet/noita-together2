// Shared browser/desktop presentation helpers. No transport or gameplay authority.
export const RACE_DEFAULT={registryRevision:'race-goals-1',durationMs:0,goals:[{instanceId:'classic',id:'classic_ending'}]};
function fail(message){throw Object.assign(new Error(message),{code:'INVALID_MESSAGE',settingsMessage:message});}
const exact=(v,keys)=>v&&typeof v==='object'&&!Array.isArray(v)&&Object.keys(v).sort().join(',')===[...keys].sort().join(',');
export function validateRaceDraft(race,catalogue){
 if(!exact(race,['registryRevision','durationMs','goals'])||race.registryRevision!==catalogue.content.revision||!Number.isInteger(race.durationMs)||race.durationMs<0||race.durationMs>604800000||!Array.isArray(race.goals)||race.goals.length<1||race.goals.length>16)fail('Choose 1–16 Race goals and a duration from 0 to 10,080 minutes.');
 const ids=new Set(),meanings=new Set();
 for(const g of race.goals){
  if(!g||typeof g!=='object'||Array.isArray(g))fail('Choose a supported goal.');
  const definition=catalogue.schema.goals.find(e=>e.id===g.id);if(!definition||!catalogue.content.goalSupport[g.id]?.available||!exact(g,['instanceId','id',...Object.keys(definition.parameters)])||!/^[a-z][a-z0-9_]{0,63}$/.test(g.instanceId)||ids.has(g.instanceId))fail('Each goal needs a distinct identity and a supported type.');ids.add(g.instanceId);
  for(const [key,rule]of Object.entries(definition.parameters)){
   const v=g[key];
   if(rule.type==='integer'){if(!Number.isSafeInteger(v)||v<rule.min||v>rule.max)fail('Choose '+key+' from '+rule.min+' to '+rule.max+'.');}
   else if(rule.type==='enum'){if(!rule.values.includes(v))fail('Choose a supported '+key+'.');}
   else {const selected=rule.type==='ids'?v:[v];if(!Array.isArray(selected)||!selected.length||selected.length>(rule.max??1)||new Set(selected).size!==selected.length||selected.some(id=>!catalogue.content[rule.registry].some(e=>e.id===id&&e.available)))fail('Choose distinct supported '+rule.registry+'.');}
  }
  if(g.id==='defeat_bosses'&&g.target>catalogue.content.bosses.filter(b=>b.available).length)fail('Not enough supported boss types for that target.');
  const meaning=g.id+JSON.stringify(Object.fromEntries(Object.entries(g).filter(([k])=>k!=='instanceId').map(([k,v])=>[k,Array.isArray(v)?[...v].sort():v]).sort(([a],[b])=>a.localeCompare(b))));if(meanings.has(meaning))fail('Remove duplicate goals.');meanings.add(meaning);
 }
 if(race.goals.some(g=>g.id==='classic_ending')&&race.goals.some(g=>g.id==='collect_orbs'&&g.target===33))fail('Classic Ending cannot be combined with Collect 33 Orbs in this run.');
 return structuredClone(race);
}
export function applyRaceDraft(config,json,catalogue){
 if(config.mode!=='race'){const {race,...core}=config;return {...core,requiredCapabilities:core.requiredCapabilities.filter(c=>c!=='race.goals.v1')};}
 let race;try{race=JSON.parse(json??JSON.stringify(RACE_DEFAULT));}catch{fail('Review the selected Race goals.');}
 if(catalogue)race=validateRaceDraft(race,catalogue);
 if(!race.goals?.length||race.goals.length>16||!Number.isInteger(race.durationMs)||race.durationMs<0||race.durationMs>604800000||config.seedPolicy==='individual-random')fail('Race needs a shared seed and valid selected goals.');
 return {...config,race,requiredCapabilities:[...new Set([...config.requiredCapabilities,'race.goals.v1'])],settings:(config.settings??[]).map(s=>s.key!=='NT_sync_orb_count'&&(s.key.startsWith('NT_sync_')||s.key.startsWith('NT_send_')||['NT_team_perks','NT_alt_heartache','NT_testing_mode'].includes(s.key))?{...s,value:'false'}:s)};
}
export function raceGoalLabel(goal,catalogue){
 const name=(kind,id)=>catalogue?.content?.[kind]?.find(e=>e.id===id)?.name??id.replaceAll('_',' ');
 switch(goal.id){
  case 'classic_ending':return 'Classic Ending';
  case 'collect_orbs':return 'Collect '+goal.target+' Orbs';
  case 'defeat_bosses':return 'Defeat '+goal.target+' distinct bosses';
  case 'defeat_named_bosses':return 'Defeat '+goal.bossIds.map(id=>name('bosses',id)).join(', ');
  case 'visit_biome':return 'Visit '+name('biomes',goal.biomeId);
  case 'visit_parallel_world':return 'Visit '+goal.direction+' Parallel World '+goal.index;
  case 'reach_max_hp':return 'Reach '+goal.target.toLocaleString()+' maximum HP';
  case 'accumulate_gold':return 'Hold '+goal.target.toLocaleString()+' gold';
  default:return 'Unavailable goal';
 }
}
