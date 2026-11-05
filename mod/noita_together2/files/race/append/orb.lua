local api=dofile('mods/noita_together2/files/race/hook_api.lua')
local original=item_pickup
function item_pickup(entity,player,...)
 local observation=api.orb_begin(entity,player)
 local result=original(entity,player,...)
 api.orb_commit(observation)
 return result
end
