local entity_id = GetUpdatedEntityID()
local x, y = EntityGetTransform(entity_id)
local particle_emitter = EntityGetFirstComponentIncludingDisabled(entity_id, "ParticleEmitterComponent")
local sprite_emitter = EntityGetFirstComponentIncludingDisabled(entity_id, "SpriteParticleEmitterComponent")
local lua = EntityGetFirstComponentIncludingDisabled(entity_id, "LuaComponent")
local times_run = ComponentGetValue2(lua, "mTimesExecuted")
local first_update=times_run==0
local age_frames=0
for _,c in ipairs(EntityGetComponentIncludingDisabled(entity_id,"VariableStorageComponent")or {})do if ComponentGetValue2(c,"name")=="nt2_a3_age_frames"then age_frames=ComponentGetValue2(c,"value_int")or 0 end end
times_run=times_run+math.floor(age_frames/3)


if times_run >= 18 then
    ComponentSetValue2(particle_emitter, "is_emitting", true)
    ComponentSetValue2(sprite_emitter, "is_emitting", true)
end

EntitySetTransform(entity_id, x, y - 1)