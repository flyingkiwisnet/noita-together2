-- Legacy Manhattan75 proximity particles, fixed local effect only.
local entity=GetUpdatedEntityID();local x,y=EntityGetTransform(entity)
local player=(EntityGetWithTag('player_unit')or {})[1];local distance=math.huge
if player then local px,py=EntityGetTransform(player);distance=math.abs(x-px)+math.abs(y-py)end
local emitter=EntityGetFirstComponentIncludingDisabled(entity,'ParticleEmitterComponent')
if emitter then
 local active=ComponentGetValue2(emitter,'is_emitting')
 if not active and distance<75 then ComponentSetValue2(emitter,'is_emitting',true)
 elseif active and distance>75 then ComponentSetValue2(emitter,'is_emitting',false)end
end
