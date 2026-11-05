-- Adapted from Noita Together; Copyright (c) 2022 Soler91. MIT notice: loot/LICENSE.
local loot=dofile("mods/noita_together2/files/loot/policy.lua")
local original_SetRandomSeed=SetRandomSeed
SetRandomSeed=function(x,y)
    if loot.enabled() then local deaths=loot.deaths();return original_SetRandomSeed(x+GameGetFrameNum()-deaths,y+deaths) end
    return original_SetRandomSeed(x,y)
end
