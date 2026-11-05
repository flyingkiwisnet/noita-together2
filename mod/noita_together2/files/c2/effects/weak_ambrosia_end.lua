-- Undo only this effect's confirmed multipliers on its original actor/component.
local json=dofile('mods/noita_together2/files/g3/bank_json.lua')
local entity=GetUpdatedEntityID();local player=EntityGetParent(entity)
for _,c in ipairs(EntityGetComponentIncludingDisabled(entity,'VariableStorageComponent')or{})do if ComponentGetValue2(c,'name')=='nt2_c2_ambrosia'then
 local state=json.decode_saved(ComponentGetValue2(c,'value_string'))
 local damage=player and player~=0 and EntityGetIsAlive(player)and EntityGetFirstComponentIncludingDisabled(player,'DamageModelComponent')
 if state and damage and(state.guard and type(ComponentHasTag)=='function'and ComponentHasTag(damage,state.guard)or not state.guard and state.player==player and state.damage==damage)then
  for name,status in pairs(state.fields)do if status=='applied'then
   local value=ComponentObjectGetValue2(damage,'damage_multipliers',name)
   state.fields[name]='removed';ComponentSetValue2(c,'value_string',assert(json.encode_saved(state)))
   if type(value)=='number'then ComponentObjectSetValue2(damage,'damage_multipliers',name,value*2)end
  end end
  if state.guard and type(ComponentRemoveTag)=='function'then ComponentRemoveTag(damage,state.guard)end
 end
end end
