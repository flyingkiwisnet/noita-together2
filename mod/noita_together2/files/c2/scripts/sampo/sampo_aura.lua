-- Reused Noita Together gameplay, Copyright (c) 2022 Soler91, MIT.
-- Full license notice: ../../../g3/ui/biomes/LICENSE.
local store=dofile('mods/noita_together2/files/c2/storage.lua')
local api=dofile('mods/noita_together2/files/c2/hook_api.lua')
local id=GetUpdatedEntityID();local row=store.component(_G,id,'nt2_c2_arena');local a=api.current()
if not a or not row or row.world~=a.world or row.progression~=a.progression or row.encounterId~=a.encounterId or row.arenaToken~=a.arenaToken or(a.stage~='unlocked'and a.stage~='battle')then return end
local script=EntityGetFirstComponentIncludingDisabled(id,'LuaComponent','nt_aura_script')
local emitter=EntityGetFirstComponentIncludingDisabled(id,'ParticleEmitterComponent');if not emitter then return end
if ComponentGetValue2(emitter,'attractor_force')>0 then ComponentSetValue2(emitter,'attractor_force',-0.4);if script then EntitySetComponentIsEnabled(id,script,false)end;return end
local radius=ComponentGetValue2(emitter,'area_circle_radius');local x,y=EntityGetTransform(id)
if radius==190 and not row.sound then row.sound=true;store.mark(_G,id,'nt2_c2_arena',row);ComponentSetValue2(emitter,'emitted_material_name','spark_blue');ComponentSetValue2(emitter,'lifetime_max',5.0);GamePlaySound('data/audio/Desktop/projectiles.snd','player_projectiles/crumbling_earth/create',x,y)end
if radius>0 then
 ComponentSetValue2(emitter,'velocity_always_away_from_center',ComponentGetValue2(emitter,'velocity_always_away_from_center')*1.1)
 ComponentSetValue2(emitter,'area_circle_radius',radius-6,radius+4);return
end
if row.issued then return end
local disabled
for _,other in ipairs(EntityGetWithTag('nt2_c2_owned')or{})do local own=store.component(_G,other,'nt2_c2_arena');if own and own.world==row.world and own.group==row.group and own.progression==row.progression and own.role=='disabled'and EntityGetIsAlive(other)then if disabled then return end;disabled=other end end
if not disabled then return end
row.issued=true;store.mark(_G,id,'nt2_c2_arena',row)
local sampo=EntityLoad('data/entities/animals/boss_centipede/sampo.xml',x,y);if not sampo or sampo==0 then return end
EntityAddTag(sampo,'nt2_c2_owned');local own={};for k,v in pairs(row)do own[k]=v end;own.role='sampo';own.issued=nil;store.mark(_G,sampo,'nt2_c2_arena',own)
local saved=store.read(_G,'state');if saved and saved.world==row.world and saved.group==row.group then saved.realSampo=true;store.write(_G,'state',saved)end
GameAddFlagRun('NT_unlocked_sampo');GamePlaySound('data/audio/Desktop/explosion.bank','explosions/explosion_magic',x,y);EntityKill(disabled);EntityKill(id)
