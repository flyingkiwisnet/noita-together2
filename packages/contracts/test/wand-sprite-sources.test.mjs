import test from 'node:test';import assert from 'node:assert/strict';
import {reviewedWandSprites,WAND_SPRITE_SOURCES} from '../scripts/wand-sprite-sources.mjs';
function metadata(){const data=new Map();for(const s of WAND_SPRITE_SOURCES){data.set(s.entity,`<Entity><ItemComponent/><AbilityComponent use_gun_script="1" sprite_file="${s.abilitySprite}"/><ParticleEmitterComponent sprite_file="data/particles/ray.xml"/></Entity>`);data.set(s.abilitySprite,`<Sprite filename="${s.image}"/>`);data.set(s.image,'local PNG bytes');}return data;}
const run=data=>reviewedWandSprites(p=>{assert.ok(data.has(p),p);return data.get(p);},p=>data.has(p));
test('Fixed reviewed gun abilities include their exact XML/PNG closure, never unrelated component paths',()=>{
 const result=run(metadata());assert.equal(result.paths.length,14);assert.equal(new Set(result.paths).size,14);assert.equal(result.sources.length,7);assert.ok(!result.paths.includes('data/particles/ray.xml'));
});
test('Missing components, commented abilities, arbitrary replacement paths and missing/changed dependencies fail closed',()=>{
 const s=WAND_SPRITE_SOURCES[0];
 for(const xml of [`<Entity><AbilityComponent use_gun_script="1" sprite_file="${s.abilitySprite}"/></Entity>`,`<Entity><ItemComponent/><!-- <AbilityComponent use_gun_script="1" sprite_file="${s.abilitySprite}"/> --></Entity>`,`<Entity><ItemComponent/><ParticleEmitterComponent use_gun_script="1" sprite_file="${s.abilitySprite}"/></Entity>`,`<Entity><ItemComponent/><AbilityComponent use_gun_script="0" sprite_file="${s.abilitySprite}"/></Entity>`,`<Entity><ItemComponent/><AbilityComponent use_gun_script="1" sprite_file="../../evil.xml"/></Entity>`]){const data=metadata();data.set(s.entity,xml);assert.throws(()=>run(data),/components changed/);}
 for(const p of [s.abilitySprite,s.image]){const data=metadata();data.delete(p);assert.throws(()=>run(data),/asset missing/);}
 for(const image of ['../../evil.png','data/particles/ray.xml']){const data=metadata();data.set(s.abilitySprite,`<Sprite filename="${image}"/>`);assert.throws(()=>run(data),/image changed/);}
});
