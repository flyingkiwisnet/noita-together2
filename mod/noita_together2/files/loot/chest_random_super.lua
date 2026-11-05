-- Adapted from Noita Together; Copyright (c) 2022 Soler91. MIT notice: loot/LICENSE.
local loot=dofile("mods/noita_together2/files/loot/policy.lua")
local original_drop_random_reward=drop_random_reward
function drop_random_reward(x,y,entity_id,rand_x,rand_y,set_rnd)
    if not loot.enabled() then return original_drop_random_reward(x,y,entity_id,rand_x,rand_y,set_rnd) end
    SetRandomSeed(GameGetFrameNum(),x+y+entity_id)
    -- We already seeded. Forward false explicitly so base cannot reset to x/y.
    return original_drop_random_reward(x,y,entity_id,rand_x,rand_y,false)
end
