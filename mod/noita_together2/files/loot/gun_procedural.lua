-- Adapted from Noita Together; Copyright (c) 2022 Soler91. MIT notice: loot/LICENSE.
local loot=dofile("mods/noita_together2/files/loot/policy.lua")
local original_generate_gun=generate_gun
function generate_gun(...)
    if loot.enabled() then return loot.frame_rng(original_generate_gun,...) end
    return original_generate_gun(...)
end
