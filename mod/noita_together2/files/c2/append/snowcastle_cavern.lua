-- Legacy competing seven checkers, composed with existing loot appends.
dofile('mods/noita_together2/files/c2/scripts/hourglass_events.lua')
local store=dofile('mods/noita_together2/files/c2/storage.lua')
local api=dofile('mods/noita_together2/files/c2/hook_api.lua')
local original=spawn_hourglass_blood
function spawn_hourglass_blood(x,y)
 if api.managed()then
  local anchor=EntityCreateNew('NT2 hourglass activation');EntityAddTag(anchor,'nt2_c2_hourglass_group');EntitySetTransform(anchor,x,y+2)
  store.mark(_G,anchor,'nt2_c2_hourglass_group',{progression=store.progression(_G),claimed=false})
  for effect,row in pairs(hourglassEventHandlers)do if type(row.handler)=='function'then CreateHourglassEntity(x,y+2,effect,row.material,anchor)end end
 end
 return original(x,y)
end
