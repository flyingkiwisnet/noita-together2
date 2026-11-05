// Reviewed base-game gun abilities outside the original procedural sprite namespace.
// Keep these exact entity/ability/Sprite image relationships explicit: unrelated
// components (including particle emitters) also have sprite_file attributes.
export const WAND_SPRITE_SOURCES = [
  ['data/entities/items/starting_wand.xml','data/items_gfx/handgun.xml','data/items_gfx/handgun.png'],
  ['data/entities/items/starting_bomb_wand.xml','data/items_gfx/bomb_wand.xml','data/items_gfx/bomb_wand.png'],
  ['data/entities/items/flute.xml','data/items_gfx/flute.xml','data/items_gfx/flute.png'],
  ['data/entities/items/kantele.xml','data/items_gfx/kantele.xml','data/items_gfx/kantele.png'],
  ['data/entities/items/wand_level_01.xml','data/items_gfx/machinegun.xml','data/items_gfx/machinegun.png'],
  ['data/entities/items/wands/experimental/experimental_wand_1.xml','data/entities/items/wands/experimental/experimental_wand_1_sprite.xml','data/entities/items/wands/experimental/experimental_wand_1.png'],
  ['data/entities/items/wands/experimental/experimental_wand_2.xml','data/entities/items/wands/experimental/experimental_wand_2_sprite.xml','data/entities/items/wands/experimental/experimental_wand_2.png'],
].map(([entity,abilitySprite,image])=>Object.freeze({entity,abilitySprite,image}));
const uncomment=text=>text.replace(/<!--[\s\S]*?-->/g,'');
const attributes=tag=>Object.fromEntries([...tag.matchAll(/([\w_]+)\s*=\s*"([^"]*)"/g)].map(m=>[m[1],m[2]]));
export function reviewedWandSprites(read,has){
  const paths=new Set();
  for(const {entity,abilitySprite,image} of WAND_SPRITE_SOURCES){
    const xml=uncomment(read(entity));
    const abilities=[...xml.matchAll(/<AbilityComponent\b[^>]*>/g)].map(m=>attributes(m[0]));
    if(!/<ItemComponent\b/.test(xml)||!abilities.some(a=>a.use_gun_script==='1'&&a.sprite_file===abilitySprite))throw Error('Reviewed wand components changed: '+entity);
    if(!has(abilitySprite)||!has(image))throw Error('Reviewed wand asset missing: '+abilitySprite);
    const sprite=uncomment(read(abilitySprite)).match(/^\s*<Sprite\b[^>]*>/)?.[0];
    if(!sprite||attributes(sprite).filename!==image)throw Error('Reviewed wand Sprite image changed: '+abilitySprite);
    read(image); // Record exact local PNG bytes in provenance; never redistribute them.
    paths.add(abilitySprite);paths.add(image);
  }
  return {paths:[...paths],sources:WAND_SPRITE_SOURCES};
}
