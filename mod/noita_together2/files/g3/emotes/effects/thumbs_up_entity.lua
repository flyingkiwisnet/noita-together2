local entity_id = GetUpdatedEntityID()
local x, y = EntityGetTransform(entity_id)
local sprite = EntityGetFirstComponentIncludingDisabled(entity_id, "SpriteComponent")
local lua = EntityGetFirstComponentIncludingDisabled(entity_id, "LuaComponent")
local times_run = ComponentGetValue2(lua, "mTimesExecuted")
local first_update=times_run==0
local age_frames=0
for _,c in ipairs(EntityGetComponentIncludingDisabled(entity_id,"VariableStorageComponent")or {})do if ComponentGetValue2(c,"name")=="nt2_a3_age_frames"then age_frames=ComponentGetValue2(c,"value_int")or 0 end end
times_run=times_run+math.floor(age_frames/3)


if first_update then
    x = x - 10
    y = y - 28
end

local alpha=math.min(0.7,math.max(0,(times_run-15)*0.1))
if times_run>=50 then alpha=math.max(0,0.7-(times_run-49)*0.1)end
ComponentSetValue2(sprite,"alpha",alpha)

EntitySetTransform(entity_id, x, y - 0.25)