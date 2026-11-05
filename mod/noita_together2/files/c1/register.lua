-- Reused Noita Together gameplay, Copyright (c) 2022 Soler91, MIT.
-- Full license notice: ../g3/ui/biomes/LICENSE.
-- Fixed C1 append set, composed after existing loot and mail hooks.
local root='mods/noita_together2/files/c1/'
local hooks={
 {'data/scripts/items/heart.lua','append/heart_append.lua'},
 {'data/scripts/animals/necromancer_shop_death.lua','append/steve_death.lua'},
 {'data/scripts/items/heart_better.lua','append/heart_better_append.lua'},
 {'data/scripts/items/orb_init.lua','append/orb_init.lua'},
 {'data/scripts/items/orb_pickup.lua','append/orb_pickup.lua'},
 {'data/scripts/perks/perk.lua','append/perk.lua'},
 {'data/scripts/perks/perk_pickup.lua','append/perk_pickup.lua'},
 {'data/scripts/magic/fungal_shift.lua','append/fungal_shift.lua'},
 {'data/scripts/biomes/temple_altar.lua','append/checkpoint_mail.lua'},
 {'data/scripts/biomes/boss_arena.lua','append/checkpoint_mail.lua'},
}
for _,hook in ipairs(hooks)do ModLuaFileAppend(hook[1],root..hook[2])end
for _,name in ipairs({'hearty_start','hearty_end'})do
 ModTextFileSetContent('data/scripts/status_effects/'..name..'.lua',ModTextFileGetContent(root..'append/status_effects/'..name..'.lua'))
end
local common='data/translations/common.csv'
ModTextFileSetContent(common,ModTextFileGetContent(common)..'\n'..ModTextFileGetContent(root..'translations.csv'))
return hooks
