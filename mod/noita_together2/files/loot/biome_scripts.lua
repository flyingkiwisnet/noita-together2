-- Adapted from Noita Together; Copyright (c) 2022 Soler91. MIT notice: loot/LICENSE.
local loot=dofile("mods/noita_together2/files/loot/policy.lua")
local original_spawn_wands=spawn_wands
function spawn_wands(x,y,...)
    if loot.enabled() then local rx,ry=loot.offset(5,5);return original_spawn_wands(x-rx,y-ry,...) end
    return original_spawn_wands(x,y,...)
end
local original_spawn_potions=spawn_potions
function spawn_potions(x,y,...)
    if loot.enabled() then local rx,ry=loot.offset(3,5);return original_spawn_potions(x-rx,y-ry,...) end
    return original_spawn_potions(x,y,...)
end
local original_spawn_heart=spawn_heart
function spawn_heart(x,y,...)
    if loot.enabled() then local rx,ry=loot.offset(5,5);return original_spawn_heart(x-rx,y-ry,...) end
    return original_spawn_heart(x,y,...)
end
