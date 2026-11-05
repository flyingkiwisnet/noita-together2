local api=dofile('mods/noita_together2/files/race/hook_api.lua')
local original=death
function death(...)
 -- This append executes only on the ten verified final-death scripts.
 api.boss(GetUpdatedEntityID())
 if original then return original(...)end
end
