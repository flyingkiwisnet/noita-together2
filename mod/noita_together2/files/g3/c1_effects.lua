-- Closed C1 content handlers. Authority, delivery order and durable originals
-- belong to the existing app/native scheduler; this adapter owns engine effects.
local root='mods/noita_together2/files/'
local parity=dofile(root..'g3/c1_gameplay.lua')
local M={}
function M.new(e,content)
 local self={orb_moves={}}
 local function damage(player)return e.EntityGetComponentIncludingDisabled(player,'DamageModelComponent')or{}end
 local function notice(key,name)e.GamePrint(e.GameTextGet(key,name))end
 function self:heart(player,count,name)
  for _,c in ipairs(damage(player))do
   local hp,max=parity.heart(e.ComponentGetValue2(c,'hp'),e.ComponentGetValue2(c,'max_hp'),e.ComponentGetValue2(c,'max_hp_cap'),count,tonumber(e.GlobalsGetValue('HEARTS_MORE_EXTRA_HP_MULTIPLIER','1')),false,true)
   e.ComponentSetValue2(c,'hp',hp);e.ComponentSetValue2(c,'max_hp',max)
  end
  e.GameTriggerMusicCue('item');notice('$noitatogether_player_got_heart',name)
  if count==0 then e.GamePrint(e.GameTextGet('$noitatogether_heart_blocked_playercount'))end
  return 'applied','effect_applied'
 end
 function self:orb(player,payload,rules,frame,name)
  local allowed,why=parity.orb_allowed(rules,e.GameGetOrbCountThisRun(),e.GameGetOrbCollectedThisRun(payload.orbId))
  local definition=content.orbs[payload.orbId];local label=e.GameTextGet(definition.nameKey,payload.orbId);if definition.worldNameKey~=''then label=e.GameTextGet(definition.worldNameKey,label)end
  if not allowed then if why=='orb_owned'then e.GamePrint(e.GameTextGet('$noitatogether_player_got_orb_had',name,label))end;return 'skipped',why end
  local x,y=e.EntityGetTransform(player)
  local orb=e.EntityLoad(root..'c1/entities/forced_orb.xml',x,y)
  assert(orb and orb~=0,'Fixed orb unavailable')
  local c=assert(e.EntityGetFirstComponentIncludingDisabled(orb,'OrbComponent'))
  e.ComponentSetValue2(c,'orb_id',payload.orbId)
  c=assert(e.EntityGetFirstComponentIncludingDisabled(orb,'ItemComponent'))
  e.ComponentSetValue2(c,'auto_pickup',true)
  self.orb_moves[#self.orb_moves+1]={entity=orb,frame=frame+10}
  e.GamePrint(e.GameTextGet('$noitatogether_player_got_orb',name,label))
  return 'applied','effect_applied'
 end
 function self:perk(player,payload,rules,name)
  if rules.NT_ban_perks and payload.perkId=='GLASS_CANNON'then return 'skipped','perk_banned'end
  if not parity.share_perk(rules,payload.perkId,false)then return 'skipped','sharing_disabled'end
  if not content.perks[payload.perkId]or not e.get_perk_with_id(e.perk_list,payload.perkId)then return 'skipped','unsupported_content'end
  local x,y=e.EntityGetTransform(player)
  local perk=assert(e.perk_spawn(x,y-8,payload.perkId),'Fixed perk unavailable')
  e.EntityAddComponent2(perk,'VariableStorageComponent',{name='NT_DONT_SHARE',value_string=''})
  e.perk_pickup(perk,player,nil,true,false)
  e.GamePrintImportant(e.GameTextGet('$noitatogether_teamperk_received_title',name),'$noitatogether_teamperk_received_subtitle')
  return 'applied','effect_applied'
 end
 function self:penalty(player,name)
  for _,c in ipairs(damage(player))do
   local hp,max,terminal=parity.penalty(e.ComponentGetValue2(c,'hp'),e.ComponentGetValue2(c,'max_hp'),false)
   if terminal then
    e.GamePrintImportant('$noitatogether_run_end_lowhp_title','$noitatogether_run_end_lowhp_subtitle')
    -- Local terminal callback must fence recursive penalty publication.
    self.terminal(player,'low_life');return 'applied','effect_applied'
   end
   e.ComponentSetValue2(c,'max_hp',max);e.ComponentSetValue2(c,'hp',hp)
  end
  e.GamePrintImportant(e.GameTextGet('$noitatogether_player_died_penalty_title',name),'$noitatogether_player_died_penalty_subtitle')
  return 'applied','effect_applied'
 end
 function self:tick(frame,player,permitted)
  if not permitted or not player then return end
  for i=#self.orb_moves,1,-1 do local move=self.orb_moves[i]
   if frame>=move.frame then
    if e.EntityGetIsAlive(move.entity)then local x,y=e.EntityGetTransform(player);e.EntitySetTransform(move.entity,x,y)end
    table.remove(self.orb_moves,i)
   end
  end
 end
 function self:apply(player,event,rules,count,frame,original_rules)
  local p=event.payload;local name=event.sourceName
  if p.heart then if not rules.NT_sync_hearts then return 'skipped','sharing_disabled'end;return self:heart(player,count,name)
  elseif p.orb then if not content.orbs[p.orb.orbId]then return 'skipped','unsupported_content'end;return self:orb(player,p.orb,rules,frame,name)
  elseif p.perk then return self:perk(player,p.perk,rules,name)
  elseif p.fungal then
   if not rules.NT_sync_shift then return 'skipped','sharing_disabled'end
   local f=p.fungal
   for _,material in ipairs(f.fromMaterials)do if not content.materials[material]or e.CellFactory_GetType(material)<0 then return 'skipped','unsupported_content'end end
   if not content.materials[f.toMaterial]or e.CellFactory_GetType(f.toMaterial)<0 or f.displayMaterial and not content.materials[f.displayMaterial]then return 'skipped','unsupported_content'end
   return content.fungal(e,player,{materials=f.fromMaterials,name_material=f.displayMaterial},{material=f.toMaterial})
  elseif p.death then
   if p.death.state=='won'then e.GamePrintImportant(e.GameTextGet('$noitatogether_player_won',name),'');return 'skipped','effect_not_applicable'end
   if original_rules.NT_death_penalty=='end'or original_rules.NT_death_penalty=='weak_respawn'and p.death.state=='terminal'then self.terminal(player,'player_death_end',true);return 'applied','effect_applied'end
   if p.death.state=='respawning'and original_rules.NT_death_penalty=='weak_respawn'then return self:penalty(player,name)end
   notice('$noitatogether_player_died',name);return 'skipped','effect_not_applicable'
  end
  return 'skipped','unsupported_content'
 end
 return self
end
return M
