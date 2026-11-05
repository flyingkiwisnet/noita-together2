-- Preserve the engine's actual Stevari death handler, including guardian upgrade.
local original=death
local ok,stats=pcall(dofile,'mods/noita_together2/files/g3/d1_hook_api.lua')
function death(damage_type,damage_message,responsible,drop_items)
 if ok then stats.steve(GetUpdatedEntityID(),responsible)end
 return original(damage_type,damage_message,responsible,drop_items)
end
