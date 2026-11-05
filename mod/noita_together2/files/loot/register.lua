-- Fixed UX027 hook catalogue. No unrelated legacy co-op or hourglass effects.
local prefix="mods/noita_together2/files/loot/"
local mappings={
 {"data/scripts/biomes/coalmine.lua","preset_wands_random.lua"},
 {"data/scripts/biomes/coalmine_alt.lua","preset_wands_random.lua"},
 {"data/scripts/biomes/tower.lua","preset_wands_random.lua"},
 {"data/scripts/biome_scripts.lua","biome_scripts.lua"},
 {"data/scripts/items/generate_shop_item.lua","generate_shop_item.lua"},
 {"data/scripts/gun/procedural/gun_procedural.lua","gun_procedural.lua"},
 {"data/scripts/items/chest_random.lua","chest_random.lua"},
 {"data/scripts/items/chest_random_super.lua","chest_random_super.lua"},
 {"data/scripts/biomes/temple_altar.lua","co_op_mail.lua"},
 {"data/scripts/biomes/boss_arena.lua","co_op_mail.lua"},
 {"data/scripts/perks/perk_reroll.lua","perk_reroll.lua"},
 {"data/scripts/biomes/snowcastle_hourglass_chamber.lua","butcher_randomseed.lua"},
}
for _,mapping in ipairs(mappings) do ModLuaFileAppend(mapping[1],prefix..mapping[2]) end
return mappings
