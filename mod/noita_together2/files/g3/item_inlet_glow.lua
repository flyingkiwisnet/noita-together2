-- Legacy10-frame inlet cue; cosmetic only. Capture belongs to the guarded bank.
local entity=GetUpdatedEntityID()
local player=tonumber(GlobalsGetValue('nt2_b1_player','0'))or 0
local active=GlobalsGetValue('nt2_items_active','0')=='1'
local near=false
if active and player>0 and EntityGetIsAlive(player)then local x,y=EntityGetTransform(player);local ix,iy=EntityGetTransform(entity);near=math.abs(x-ix)+math.abs(y-iy)<150 end
local hints=not ModSettingGet or ModSettingGet('noita_together2.NT_HINTS')~=false
for _,c in ipairs(EntityGetComponentIncludingDisabled(entity,'SpriteComponent')or {})do EntitySetComponentIsEnabled(entity,c,near and hints)end
for _,c in ipairs(EntityGetComponentIncludingDisabled(entity,'ParticleEmitterComponent')or {})do ComponentSetValue2(c,'is_emitting',active and near)end
