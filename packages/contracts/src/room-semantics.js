export const ROOM_SEMANTICS_REVISION='g3-room-semantics-1';
const definitions={'core-world-rules-hourglass-items-gold-social-controls-seed-loot':['core.seed','core.ghost','core.hud','core.poi','core.seed-policy.v1','core.randomize-loot.v1','core.emotes','core.cosmetics','core.gold','core.items','core.world-rules','core.hourglass'],'core-items-gold-social-controls-seed-loot':['core.seed','core.ghost','core.hud','core.poi','core.seed-policy.v1','core.randomize-loot.v1','core.emotes','core.cosmetics','core.gold','core.items'],'core-gold-social-controls-seed-loot':['core.seed','core.ghost','core.hud','core.poi','core.seed-policy.v1','core.randomize-loot.v1','core.emotes','core.cosmetics','core.gold'],'core-seed':['core.seed'],'core-ghost':['core.seed','core.ghost'],'core-social':['core.seed','core.ghost','core.hud','core.poi'],'core-social-seed-loot':['core.seed','core.ghost','core.hud','core.poi','core.seed-policy.v1','core.randomize-loot.v1'],'core-social-controls-seed-loot':['core.seed','core.ghost','core.hud','core.poi','core.seed-policy.v1','core.randomize-loot.v1','core.emotes','core.cosmetics']};
const exact=(a,b)=>Array.isArray(a)&&a.length===b.length&&new Set(a).size===a.length&&b.every(x=>a.includes(x));
export function validateRoomSemantics(value,compatibility){
 const refuse=()=>{throw Error('INCOMPATIBLE_GAME');};
 if(!value||typeof value!=='object'||Array.isArray(value)||Object.keys(value).sort().join()!=='kind,requiredCapabilities,revision'||value.revision!==ROOM_SEMANTICS_REVISION||!Object.hasOwn(definitions,value.kind))refuse();
 const required=definitions[value.kind],supported=compatibility?.capabilities;
 if(!exact(value.requiredCapabilities,required)||!Array.isArray(supported)||supported.length>32||new Set(supported).size!==supported.length||supported.some(x=>typeof x!=='string')||!required.every(x=>supported.includes(x)))refuse();
 // Native ghost/social semantics require the exact supported closure, not extra grants.
 if(value.kind!=='core-seed'&&!exact(supported,required))refuse();
 return [...required];
}
export function declareRoomSemantics(compatibility){
 const kind=exact(compatibility.capabilities,definitions['core-world-rules-hourglass-items-gold-social-controls-seed-loot'])?'core-world-rules-hourglass-items-gold-social-controls-seed-loot':exact(compatibility.capabilities,definitions['core-items-gold-social-controls-seed-loot'])?'core-items-gold-social-controls-seed-loot':exact(compatibility.capabilities,definitions['core-gold-social-controls-seed-loot'])?'core-gold-social-controls-seed-loot':exact(compatibility.capabilities,definitions['core-social-controls-seed-loot'])?'core-social-controls-seed-loot':exact(compatibility.capabilities,definitions['core-social-seed-loot'])?'core-social-seed-loot':exact(compatibility.capabilities,definitions['core-social'])?'core-social':exact(compatibility.capabilities,definitions['core-ghost'])?'core-ghost':'core-seed';
 const value={revision:ROOM_SEMANTICS_REVISION,kind,requiredCapabilities:[...definitions[kind]]};validateRoomSemantics(value,compatibility);return value;
}
