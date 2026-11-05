local entity_id = GetUpdatedEntityID()
local lights = EntityGetComponentIncludingDisabled(entity_id, "LightComponent")
local sprite = EntityGetFirstComponentIncludingDisabled(entity_id, "SpriteComponent")
local particles = EntityGetFirstComponentIncludingDisabled(entity_id, "ParticleEmitterComponent")
local lua = EntityGetFirstComponentIncludingDisabled(entity_id, "LuaComponent")
local times_run = ComponentGetValue2(lua, "mTimesExecuted")
local first_update=times_run==0
local age_frames=0
for _,c in ipairs(EntityGetComponentIncludingDisabled(entity_id,"VariableStorageComponent")or {})do if ComponentGetValue2(c,"name")=="nt2_a3_age_frames"then age_frames=ComponentGetValue2(c,"value_int")or 0 end end
times_run=times_run+math.floor(age_frames/1)


if first_update then
    local x, y = EntityGetTransform(entity_id)
    EntitySetTransform(entity_id, x, y - 20)
end

if times_run > 53 then
    ComponentSetValue2(lights[1], "radius", times_run * 0.3)
    ComponentSetValue2(lights[2], "radius", times_run * 0.1)
    ComponentSetValue2(lights[3], "radius", times_run * 0.1)
end

if times_run>=153 then
 ComponentSetValue2(particles,"is_emitting",times_run<=325)
 local interval=math.max(5,13-2*math.floor((times_run-153)/10))
 ComponentSetValue2(particles,"emission_interval_min_frames",interval)
 ComponentSetValue2(particles,"emission_interval_max_frames",interval)
end
if times_run>153 then
 ComponentSetValue2(lights[4],"radius",(times_run-150)*0.1)
 ComponentSetValue2(lights[5],"radius",(times_run-150)*0.1)
 ComponentSetValue2(sprite,"alpha",math.min(0.1,(times_run-153)*0.003))
end

if times_run > 325 then
    ComponentSetValue2(sprite, "alpha", math.max(0.0, 0.1 - (times_run - 325) * 0.01))
    ComponentSetValue2(particles, "is_emitting", false)
end

--[[if times_run > 68 and times_run < 83 then
    --GamePrint(tostring(times_run))
    ComponentSetValue2(particle_emitter, "is_emitting", true)
    ComponentSetValue2(sprite_emitter, "is_emitting", true)
end

if times_run >= 83 then
    ComponentSetValue2(particle_emitter, "is_emitting", false)
    ComponentSetValue2(sprite_emitter, "is_emitting", false)
end]]