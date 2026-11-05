-- Adapted from Noita Together; Copyright (c) 2022 Soler91. MIT notice: loot/LICENSE.
local loot=dofile("mods/noita_together2/files/loot/policy.lua")
local original_spawn_all_shopitems=spawn_all_shopitems
function spawn_all_shopitems(x,y,...)
    if loot.enabled() then local rx,ry=loot.offset(3,5);return original_spawn_all_shopitems(x-rx,y-ry,...) end
    return original_spawn_all_shopitems(x,y,...)
end
local original_spawn_all_perks=spawn_all_perks
function spawn_all_perks(...)
    if loot.enabled() then return loot.frame_rng(original_spawn_all_perks,...) end
    return original_spawn_all_perks(...)
end
