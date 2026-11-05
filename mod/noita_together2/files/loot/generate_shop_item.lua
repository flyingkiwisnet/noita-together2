-- Adapted from Noita Together; Copyright (c) 2022 Soler91. MIT notice: loot/LICENSE.
local loot=dofile("mods/noita_together2/files/loot/policy.lua")
local original_generate_shop_item=generate_shop_item
function generate_shop_item(x,y,...)
    if loot.enabled() then local rx,ry=loot.offset(5,5);return original_generate_shop_item(x-rx,y-ry,...) end
    return original_generate_shop_item(x,y,...)
end
