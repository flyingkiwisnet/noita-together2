-- Legacy geometry/effects reconciled from monotonic scoped service facts.
local store=dofile('mods/noita_together2/files/c2/storage.lua');local M={}
function M.near(px,py,sx,sy)return(sx-px)^2+(sy-py)^2<200^2 end
function M.name_index(orbs)if orbs<0 then return 0 elseif orbs>33 then return 33 elseif orbs>13 and orbs<33 then return 13 end;return orbs end
function M.new(e,persist)
 local self={}
 function self:owned(saved,role)
  local found={};for _,id in ipairs(e.EntityGetWithTag('nt2_c2_owned')or{})do local row=store.component(e,id,'nt2_c2_arena')
   if e.EntityGetIsAlive(id)and row and saved.arenaToken and row.arenaToken==saved.arenaToken and row.world==saved.world and row.progression==saved.progression and row.role==role then found[#found+1]=id end
  end;return #found==1 and found[1]or nil
 end
 function self:bind(saved,identity,context)
  if saved.group and e.EntityGetIsAlive(saved.group)then local row=store.component(e,saved.group,'nt2_c2_arena');if row and row.world==saved.world and row.arenaToken==saved.arenaToken and row.progression==saved.progression then return true end end
  local groups={};for _,id in ipairs(e.EntityGetWithTag('nt2_c2_arena_group')or{})do local row=store.component(e,id,'nt2_c2_arena');if row and row.progression==saved.progression and(not row.world or row.world==saved.world)and(not saved.arenaToken or row.arenaToken==saved.arenaToken)then groups[#groups+1]=id end end
  if #groups~=1 then return false end
  local prior=store.component(e,groups[1],'nt2_c2_arena');local old_group=prior.group
  saved.group=groups[1];saved.arenaToken=saved.arenaToken or identity();if not saved.arenaToken then return false end;persist()
  for _,id in ipairs(e.EntityGetWithTag('nt2_c2_owned')or{})do local row=store.component(e,id,'nt2_c2_arena');if row and(row.group==old_group or row.arenaToken==saved.arenaToken)and(not row.world or row.world==saved.world)and row.progression==saved.progression then row.group=saved.group;row.world=saved.world;row.context=context;row.arenaToken=saved.arenaToken;store.mark(e,id,'nt2_c2_arena',row)end end
  local row=store.component(e,saved.group,'nt2_c2_arena');row.world=saved.world;row.group=saved.group;row.arenaToken=saved.arenaToken;store.mark(e,saved.group,'nt2_c2_arena',row);return true
 end
 function self:scope(saved,encounter,context)
  for _,id in ipairs(e.EntityGetWithTag('nt2_c2_owned')or{})do local row=store.component(e,id,'nt2_c2_arena');if row and row.group==saved.group and row.world==saved.world then row.encounterId=encounter.encounterId;row.arenaToken=saved.arenaToken;row.context=context;store.mark(e,id,'nt2_c2_arena',row)end end
 end
 function self:unlock(saved)
  if saved.realSampo then return true end
  local disabled=self:owned(saved,'disabled');local aura=self:owned(saved,'aura');if not disabled or not aura then return false end
  for _,c in ipairs(e.EntityGetComponentIncludingDisabled(aura,'LuaComponent')or{})do if e.ComponentHasTag(c,'nt_aura_script')then e.EntitySetComponentIsEnabled(aura,c,true)end end;return true
 end
 function self:pickup(saved,item)
  if saved.localPickup then return true end
  saved.localPickup=true;persist()
  local x,y=e.EntityGetTransform(item);e.EntityLoad('data/entities/particles/image_emitters/chest_effect.xml',x,y)
  local sampo=self:owned(saved,'sampo');if sampo then e.EntitySetComponentsWithTagEnabled(sampo,'disabled_at_start',true);e.EntitySetComponentsWithTagEnabled(sampo,'enabled_at_start',false);e.PhysicsSetStatic(sampo,false)end
  e.GameAddFlagRun('NT_got_sampo');return true
 end
 function self:battle(saved,player)
  local step=saved.battle or{};saved.battle=step;if step.done then return true end
  local boss=self:owned(saved,'boss');local reference=self:owned(saved,'reference');if not boss or not reference then return false end
  local x,y=e.EntityGetTransform(player)
  if not step.music then step.music=true;persist();e.GameTriggerMusicFadeOutAndDequeueAll(10.0);e.GamePlaySound('data/audio/Desktop/event_cues.bank','event_cues/sampo_pick/create',x,y);e.GameTriggerMusicEvent('music/boss_arena/battle',false,x,y)end
  e.EntitySetComponentsWithTagEnabled(boss,'disabled_at_start',true);e.EntitySetComponentsWithTagEnabled(boss,'enabled_at_start',false);e.PhysicsSetStatic(boss,false);e.EntityAddTag(boss,'boss_centipede_active')
  for _,child in ipairs(e.EntityGetAllChildren(boss)or{})do if e.EntityHasTag(child,'protection')then e.EntityKill(child)end end
  x,y=e.EntityGetTransform(reference);step.rocks=step.rocks or{}
  for i,row in ipairs({{'loose_lavaceiling',-235,-73},{'loose_lavaceiling',264,-50},{'loose_lavabridge',-235,282},{'loose_lavabridge',257,262}})do
   if not step.rocks[i]then step.rocks[i]='issuing';persist();local rock=e.EntityLoad('data/entities/animals/boss_centipede/'..row[1]..'.xml',x+row[2],y+row[3]);if not rock or rock==0 then return false end;step.rocks[i]='done';persist()
   elseif step.rocks[i]~='done'then return false end
  end
  e.GameAddFlagRun('NT_final_boss_active');e.GlobalsSetValue('FINAL_BOSS_ACTIVE','1');step.done=true;persist();return true
 end
 return self
end
-- Explicit saved opt-out only. Connection/terminal holds never enter here.
-- No service effects, grants or originals are resumed by this local handoff.
function M.offline(e,player,item)
 if e.GlobalsGetValue('nt2_rejoin_local_world_v1_offline','0')~='1'then return false end
 store.write(e,'grant',{})
 local saved=store.read(e,'state');local association=e.GlobalsGetValue('nt2_rejoin_local_world_v1','')
 if not saved or not saved.arenaToken or saved.progression~=store.progression(e)or not saved.association or saved.association~=association or not dofile('mods/noita_together2/files/rejoin/local_world.lua').decode_saved(association)then return false end
 local id,actor=dofile('mods/noita_together2/files/g3/game_api.lua').resolve_player(e,player)
 if not id or not actor or not actor.ready or actor.transformed or e.ComponentGetValue2(actor.controls,'enabled')==false then return false end
 for _,c in ipairs(e.EntityGetComponentIncludingDisabled(id,'VariableStorageComponent')or{})do if e.ComponentGetValue2(c,'name')=='nt2_g3_recovery'then return false end end
 local function persist()store.write(e,'state',saved)end
 local arena=M.new(e,persist)
 if not arena:bind(saved,function()return nil end,'offline')then return false end
 local sampo=arena:owned(saved,'sampo')
 if not sampo and not saved.realSampo and not saved.localPickup then
  local disabled=arena:owned(saved,'disabled');local aura=arena:owned(saved,'aura');local row=aura and store.component(e,aura,'nt2_c2_arena')
  -- A prior issuing boundary without a surviving marked entity is uncertain.
  if disabled and row and not row.issued and not saved.offlineSpawn then
   saved.offlineSpawn='issuing';persist()
   local x,y=e.EntityGetTransform(disabled);sampo=e.EntityLoad('data/entities/animals/boss_centipede/sampo.xml',x,y)
   if not sampo or sampo==0 then return false end
   local own={world=saved.world,progression=saved.progression,group=saved.group,arenaToken=saved.arenaToken,role='sampo'}
   e.EntityAddTag(sampo,'nt2_c2_owned');store.mark(e,sampo,'nt2_c2_arena',own)
  end
 end
 if sampo then saved.realSampo=true;saved.offlineSpawn='done';persist()end
 if saved.realSampo or saved.localPickup then
  for _,role in ipairs({'disabled','aura'})do local entity=arena:owned(saved,role);if entity then e.EntityKill(entity)end end
 end
 if item then
  if item~=sampo then return false end
  arena:pickup(saved,item)
 end
 if saved.localPickup then arena:battle(saved,id)end
 return true
end
return M
