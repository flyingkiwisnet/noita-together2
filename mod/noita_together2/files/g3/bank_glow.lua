-- Cosmetic outlet cue, every10frames. Queued gold alone keeps glow stable.
local entity=GetUpdatedEntityID()
local player=tonumber(GlobalsGetValue('nt2_b1_player','0'))or 0
local queued=tonumber(GlobalsGetValue('nt2_b1_queue','0'))or 0
local active=GlobalsGetValue('nt2_b1_active','0')=='1'
local near=false
if active and queued>0 and player>0 and EntityGetIsAlive(player)then
 local x,y=EntityGetTransform(player);local ox,oy=EntityGetTransform(entity)
 near=math.abs(x-ox)+math.abs(y-oy)<150
end
local hints=not ModSettingGet or ModSettingGet('noita_together2.NT_HINTS')~=false
for _,c in ipairs(EntityGetComponentIncludingDisabled(entity,'SpriteComponent')or {})do EntitySetComponentIsEnabled(entity,c,near and hints)end
for _,c in ipairs(EntityGetComponentIncludingDisabled(entity,'ParticleEmitterComponent')or {})do ComponentSetValue2(c,'is_emitting',active and queued>0)end
