-- Reused Noita Together gameplay, Copyright (c) 2022 Soler91, MIT.
-- Full license notice: ../g3/ui/biomes/LICENSE.
local root='mods/noita_together2/files/c2/'
local hooks={{'data/scripts/biomes/boss_arena.lua','append/boss_arena.lua'},{'data/entities/animals/boss_centipede/sampo_pickup.lua','append/sampo_append.lua'},{'data/scripts/biomes/snowcastle_cavern.lua','append/snowcastle_cavern.lua'}}
for _,hook in ipairs(hooks)do ModLuaFileAppend(hook[1],root..hook[2])end
dofile(root..'hourglass.lua').install_translations(_G)
return hooks
