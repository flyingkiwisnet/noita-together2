-- Legacy thirteen0.5 multipliers, with per-effect ownership and saved substeps.
local json=dofile('mods/noita_together2/files/g3/bank_json.lua')
local entity=GetUpdatedEntityID();local player=EntityGetParent(entity)
if not player or player==0 or player==entity or not EntityGetIsAlive(player)then return end
local damage=EntityGetFirstComponentIncludingDisabled(player,'DamageModelComponent');if not damage then return end
local marker
for _,c in ipairs(EntityGetComponentIncludingDisabled(entity,'VariableStorageComponent')or{})do if ComponentGetValue2(c,'name')=='nt2_c2_ambrosia'then marker=c end end
local state=marker and json.decode_saved(ComponentGetValue2(marker,'value_string'))or nil
if marker then return end -- duplicate, corrupt marker or Continue cannot multiply twice
local guard
for _,c in ipairs(EntityGetComponentIncludingDisabled(entity,'VariableStorageComponent')or{})do if ComponentGetValue2(c,'name')=='nt2_c2_ambrosia_key'then local key=ComponentGetValue2(c,'value_string');if type(key)=='string'and key:match('^[a-f0-9%-]+$')and #key==36 then guard='nt2_c2_ambrosia_'..key end end end
if guard then if type(ComponentAddTag)~='function'then return end;ComponentAddTag(damage,guard)end
state={player=player,damage=damage,guard=guard,fields={}}
marker=EntityAddComponent2(entity,'VariableStorageComponent',{name='nt2_c2_ambrosia',value_string=assert(json.encode_saved(state))})
for _,name in ipairs({'melee','projectile','explosion','electricity','fire','drill','slice','ice','physics_hit','radioactive','poison','overeating','curse'})do
 local value=ComponentObjectGetValue2(damage,'damage_multipliers',name)
 if type(value)=='number'then
  state.fields[name]='applying';ComponentSetValue2(marker,'value_string',assert(json.encode_saved(state)))
  ComponentObjectSetValue2(damage,'damage_multipliers',name,value*0.5)
  state.fields[name]='applied';ComponentSetValue2(marker,'value_string',assert(json.encode_saved(state)))
 end
end
