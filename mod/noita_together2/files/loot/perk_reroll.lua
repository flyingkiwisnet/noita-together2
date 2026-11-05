-- Adapted from Noita Together; Copyright (c) 2022 Soler91. MIT notice: loot/LICENSE.
local loot=dofile("mods/noita_together2/files/loot/policy.lua")
local original_item_pickup=item_pickup
function item_pickup(...)
    if loot.enabled() then return loot.frame_rng(original_item_pickup,...) end
    return original_item_pickup(...)
end
