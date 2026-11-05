-- Scoped C1 game owner. Closed canonical payloads, fixed content handlers and
-- existing actor resolver; no separate admission or connection controller.
local root='mods/noita_together2/files/'
local json=dofile(root..'g3/bank_json.lua')
local parity=dofile(root..'g3/c1_gameplay.lua')
local resolver=dofile(root..'g3/game_api.lua')
local M={}
-- Also used by the scoped saved-world terminal mailbox, without a play grant.
function M.finish_run(e,player,scope,reason)
 local association=e.GlobalsGetValue('nt2_rejoin_local_world_v1','')
 local saved=json.decode_saved(e.GlobalsGetValue('nt2_c1_finished',''))
 if saved and ((scope and saved.world==scope)or(association~=''and saved.association==association))then return false end
 local marker={world=scope,association=association,reason=reason,state='applying'}
 -- Persist before mutation; an ambiguous saved effect is never replayed.
 e.GlobalsSetValue('nt2_c1_finished',json.encode_saved(marker))
 e.GlobalsSetValue('nt2_c1_life','');e.GlobalsSetValue('nt2_c1_tokens','');e.GlobalsSetValue('nt2_c1_authority','')
 local d=player and e.EntityGetFirstComponentIncludingDisabled(player,'DamageModelComponent')
 if d then e.ComponentSetValue2(d,'hp',0);e.ComponentSetValue2(d,'kill_now',true)end
 e.GameTriggerGameOver()
 marker.state='applied';e.GlobalsSetValue('nt2_c1_finished',json.encode_saved(marker));return true
end
local function owns_camera(e,entity,id)
 if not entity or not e.EntityGetIsAlive(entity)or not e.EntityHasTag(entity,'nt2_c1_death_camera')then return false end
 for _,c in ipairs(e.EntityGetComponentIncludingDisabled(entity,'VariableStorageComponent')or{})do if e.ComponentGetValue2(c,'name')=='nt2_c1_camera_owner'and e.ComponentGetValue2(c,'value_string')==id then return true end end
 return false
end
function M.recover(e,player)
 local vars=e.EntityGetComponentIncludingDisabled(player,'VariableStorageComponent')or{}
 for _,c in ipairs(vars)do if e.ComponentGetValue2(c,'name')=='nt2_g3_recovery'then return false end end
 local damage=e.EntityGetFirstComponentIncludingDisabled(player,'DamageModelComponent')
 for _,c in ipairs(vars)do if e.ComponentGetValue2(c,'name')=='nt2_c1_wait' then
  if damage then e.ComponentSetValue2(damage,'wait_for_kill_flag_on_death',e.ComponentGetValue2(c,'value_bool')==true)end
  e.EntityRemoveComponent(player,c)
 end end
 local readable,camera=pcall(e.GlobalsGetValue,'nt2_c1_camera','');if not readable then return false end
 if camera~=''then
  for _,id in ipairs(e.EntityGetWithTag('nt2_c1_death_camera')or{})do if owns_camera(e,id,camera)then e.EntityKill(id)end end
  e.GameSetCameraFree(false);e.GlobalsSetValue('nt2_c1_camera','')
 end
 return true
end
function M.new(e,identity,send,content)
 local effects=dofile(root..'g3/c1_effects.lua').new(e,content)
 local self={scope=nil,ready=false,playing=false,queue={},receipts={},rules=nil,revision=0,count=0,last_death=-31}
 local trace_counts={}
 local function trace(id,sequence,state,reason,frame)
  local key=state..':'..reason;trace_counts[key]=math.min(2147483647,(trace_counts[key]or 0)+1)
  if e.print then e.print('NT2_C1|world='..self.scope..'|id='..id..'|sequence='..sequence..'|state='..state..'|reason='..reason..'|count='..trace_counts[key]..'|frame='..frame)end
 end
 local function persist_queue()e.GlobalsSetValue('nt2_c1_outgoing',json.encode_saved({world=self.scope,context=self.context,queue=self.queue}))end
 local function own_death(original,state,mode,poly)
  e.GlobalsSetValue('nt2_c1_own_death',json.encode_saved({world=self.scope,id=original.eventId,state=state,mode=mode,poly=poly==true,original=original}))
 end
 local function replace_original(original)
  for i,v in ipairs(self.queue)do if v.eventId==original.eventId then self.queue[i]=original;break end end
  persist_queue()
 end
 local function mark(id,sequence,state)e.GlobalsSetValue('nt2_c1_effect_receipt',json.encode_saved({world=self.scope,id=id,sequence=sequence,state=state}))end
 local function marker(id,sequence)
  local saved=json.decode_saved(e.GlobalsGetValue('nt2_c1_effect_receipt',''))
  if not saved or saved.world~=self.scope then return ''end
  if #saved.sequence>#sequence or #saved.sequence==#sequence and saved.sequence>=sequence then return saved.state end
  return ''
 end
 local function checkpoint()
  local saved=json.decode_saved(e.GlobalsGetValue('nt2_c1_checkpoint',''))
  if saved and saved.world==self.scope then return tonumber(saved.x),tonumber(saved.y) end
  return tonumber(e.MagicNumbersGetValue('DESIGN_PLAYER_START_POS_X')),tonumber(e.MagicNumbersGetValue('DESIGN_PLAYER_START_POS_Y'))
 end
 function self:roster_count()return self.count end
 function self:can_pickup(player)
  local id,state=resolver.resolve_player(e,self.player)
  return self.ready and not self.disabled and not self.terminal_state and self.playing and id==player and state and state.ready
 end
 function self:owns_death(player)
  local state=resolver.player_state(e,player,false)
  -- An existing wait lease still owns a genuine death while policy is being
  -- refreshed. This preserves connection protection; it grants no mutation.
  return not self.disabled and not self.terminal_state and not self.own_uncertain and state and state.dead and state.damage and state.controls
   and(self.ready or self.wait and self.wait.player==player and self.wait.damage==state.damage)
 end
 function self:capture(kind,payload,entity,completion)
  if self.terminal_state or not self.ready or not(self.playing or completion and self.completion and kind=='death')then return nil end
  if #self.queue>=32 then self.status='C1 capture capacity; sharing unavailable.';return nil end
  if entity then
   for _,c in ipairs(e.EntityGetComponentIncludingDisabled(entity,'VariableStorageComponent')or{})do
    if e.ComponentGetValue2(c,'name')=='nt2_c1_original'then return nil end
   end
  end
  local id=identity();if not id then self.status='C1 identity unavailable.';return nil end
  if kind=='death'and payload.state=='won'and e.NT2_C2_ENABLED then payload.progression=dofile(root..'c2/storage.lua').progression(e)end
  local original={eventId=id,rulesRevision=self.revision,payload={[kind]=payload},senderDisposition='applied'}
  if entity then e.EntityAddComponent2(entity,'VariableStorageComponent',{name='nt2_c1_original',value_string=id})end
  -- Persist before local gameplay, so a mid-hook error is explicitly uncertain.
  original.senderDisposition='uncertain';self.queue[#self.queue+1]=original;persist_queue()
  trace(id,'0','captured','local_original',e.GameGetFrameNum())
  return original
 end
 function self:submit(original)
  original.senderDisposition='applied';replace_original(original)
  trace(original.eventId,'0','applied','local_effect',e.GameGetFrameNum())
 end
 function self:emit_fungal(from,to)
  local original=self:capture('fungal',{fromMaterials=from.materials,toMaterial=to.material,displayMaterial=from.name_material})
  if original then self:submit(original)end
 end
 function self:checkpoint(player,x,y)
  if not self:can_pickup(player)or self.rules.NT_death_penalty=='end'then return false end
  e.GlobalsSetValue('nt2_c1_checkpoint',json.encode_saved({world=self.scope,x=tostring(x),y=tostring(y)}));return true
 end
 -- Legacy utils.lua FinishRun: guard first, kill the actor and trigger game over.
 -- The canonical service owns shared termination; remote completion never emits
 -- another death/RunOver or applies a second respawn penalty.
 function self:finish_run(player,reason)
  self.terminal_state=true;self.playing=false;self.death=nil;self.life=nil;self:close_camera()
  return M.finish_run(e,player,self.scope,reason)
 end
 function self:terminal(player,cause,original)
  if self.terminal_state then return end
  if not original then original=self:capture('death',{state='terminal',cause=cause})end
  if original then self:submit(original)end
  if original and original.payload.death.state=='won'then
   self.terminal_state=true;self.death=nil;self.life=nil;e.GlobalsSetValue('nt2_c1_life','')
   local d=e.EntityGetFirstComponentIncludingDisabled(player,'DamageModelComponent');if d then e.ComponentSetValue2(d,'kill_now',true)end;return
  end
  self:finish_run(player,cause)
 end
 function self:close_camera()
  if not self.camera then return end
  if e.GlobalsGetValue('nt2_c1_camera','')==self.camera.id then
   e.GameSetCameraFree(false);e.GlobalsSetValue('nt2_c1_camera','')
  end
  if owns_camera(e,self.camera.entity,self.camera.id)then e.EntityKill(self.camera.entity)end
  self.camera=nil
 end
 function self:close()
  self.playing=false;self.ready=false
  -- Entry reset can retire a retained owner before the restored WorldState is
  -- readable. Drop local authority now; saved leases are recovered on spawn.
  if e.GameGetWorldStateEntity then
   local ok,world=pcall(e.GameGetWorldStateEntity)
   if not ok or type(world)~='number'or world<=0 then self.camera=nil;self.wait=nil;return end
  end
  e.GlobalsSetValue('nt2_c1_authority','');e.GlobalsSetValue('nt2_c1_tokens','');self:close_camera()
  local l=self.wait
  if l and e.EntityGetIsAlive(l.player)and e.EntityGetFirstComponentIncludingDisabled(l.player,'DamageModelComponent')==l.damage then
   local held=false;for _,c in ipairs(e.EntityGetComponentIncludingDisabled(l.player,'VariableStorageComponent')or{})do if e.ComponentGetValue2(c,'name')=='nt2_g3_recovery'then held=true end end
   if not held then e.ComponentSetValue2(l.damage,'wait_for_kill_flag_on_death',l.old);if l.marker then e.EntityRemoveComponent(l.player,l.marker)end end
  end
 end
 effects.terminal=function(player,cause,remote)if remote then self:finish_run(player,cause)else self:terminal(player,cause)end end
 function self:receive(view)
  if type(view)=='table'and view.enabled==false then self.ready=true;self.disabled=true;return true end
  if type(view)~='table'or type(view.world)~='string'or #view.world~=64 then return false end
  if self.scope~=view.world then
   e.GlobalsSetValue('nt2_c1_tokens','')
   self:close_camera();self.wait=nil;effects.orb_moves={}
   self.scope=view.world;self.ready=false;self.rules=nil;self.revision=0;self.receipts={};self.death=nil;self.terminal_state=false;self.queue={};self.own_uncertain=nil;self.local_status=nil;self.life=nil;self.last_death=-31
   local saved=json.decode_saved(e.GlobalsGetValue('nt2_c1_outgoing',''),{bytes=131072,nodes=16384})
   if saved and saved.world==self.scope and type(saved.queue)=='table'and #saved.queue<=32 then
    self.queue=saved.queue;for _,v in ipairs(self.queue)do v.senderDisposition="uncertain"end
    -- tick and physical hooks share this mailbox. Persist the restore boundary
    -- before either can read an older applied self-report back into memory.
    persist_queue()
   end
   local life=json.decode_saved(e.GlobalsGetValue('nt2_c1_life',''));self.life=life and life.world==self.scope and life.life or nil
   local own=json.decode_saved(e.GlobalsGetValue('nt2_c1_own_death',''))
   if own and own.world==self.scope then
    if own.state=='waiting' then
     local original=own.original;for _,v in ipairs(self.queue)do if v.eventId==own.id then original=v;break end end
     if original then self.death={original=original,mode=own.mode,poly=own.poly,frame=e.GameGetFrameNum(),player=nil,deferred_camera=true}end
    else self.own_uncertain=own.id;self.local_status='Own respawn save outcome is uncertain; it will not repeat.' end
   end
  end
  local snapshot=view.snapshot
  if snapshot and type(snapshot.revision)=='number'and snapshot.revision>self.revision then
   local old_ban=self.rules and self.rules.NT_ban_perks
   self.rules=snapshot.rules;self.revision=snapshot.revision
   -- Snapshot contains C1 keys only. Never reset seed/loot or unrelated flags.
   for _,key in ipairs({'NT_sync_hearts','NT_sync_orbs','NT_sync_shift','NT_team_perks','NT_sync_perks','NT_alt_heartache','NT_ban_perks','NT_sync_orbs_no_curse','NT_ondeath_kick'})do
    if self.rules[key]then e.GameAddFlagRun(key)else e.GameRemoveFlagRun(key)end
   end
   for _,mode in ipairs({'end','weak_respawn','full_respawn'})do local key='NT_death_penalty_'..mode;if self.rules.NT_death_penalty==mode then e.GameAddFlagRun(key)else e.GameRemoveFlagRun(key)end end
   if self.rules.NT_ban_perks and not old_ban then self.ban_pending=self.revision elseif not self.rules.NT_ban_perks then self.ban_pending=nil end
  end
  self.completion=view.completion==true;self.disabled=false;self.ready=view.initialized==true and self.rules~=nil
  self.local_accepted={};for _,key in ipairs(view.localAccepted or{})do self.local_accepted[key]=true end
  self.count=view.memberCount or 0;self.event=view.event;self.permit=view.permit;self.status=view.status
  return true
 end
 function self:death_tick(frame,player,state)
  if self.terminal_state or not self.rules then return end
  if not self.death then
   if not state or not state.dead or frame<=self.last_death+30 then return end
   local won=e.GameHasFlagRun('ending_game_completed');local mode=self.rules.NT_death_penalty
   local cause=state.transformed and 'polymorph'or 'normal'
   if won or mode=='end'then
    local original=self:capture('death',{state=won and 'won'or 'terminal',cause=cause})
    self:terminal(player,cause,original);return
   end
   local original=self:capture('death',{state='respawning',cause=cause})
   if not original then return end
   self.last_death=frame;self.death={original=original,mode=mode,frame=frame,poly=state.transformed,player=player};own_death(original,'waiting',mode,state.transformed)
   self:close_camera()
   local x,y=e.GameGetCameraPos();self.camera={id=original.eventId,x=x,y=y,until_frame=frame+599}
   self.camera.entity=e.EntityLoad(root..'c1/entities/death_cam.xml',x,y)
   e.EntityAddComponent2(self.camera.entity,'VariableStorageComponent',{name='nt2_c1_camera_owner',value_string=original.eventId})
   e.GlobalsSetValue('nt2_c1_camera',original.eventId);e.GameSetCameraFree(true)
   if state.transformed then
    for _,child in ipairs(e.EntityGetAllChildren(player)or{})do for _,c in ipairs(e.EntityGetComponentIncludingDisabled(child,'GameEffectComponent')or{})do e.ComponentSetValue2(c,'frames',1)end end
   end
  end
  local death=self.death
  if death.deferred_camera then
   self:close_camera();local x,y=e.GameGetCameraPos();self.camera={id=death.original.eventId,x=x,y=y,until_frame=frame+599};self.camera.entity=e.EntityLoad(root..'c1/entities/death_cam.xml',x,y);e.EntityAddComponent2(self.camera.entity,'VariableStorageComponent',{name='nt2_c1_camera_owner',value_string=death.original.eventId});e.GlobalsSetValue('nt2_c1_camera',death.original.eventId);e.GameSetCameraFree(true);death.deferred_camera=false
  end
  if death.poly and(frame<death.frame+2 or not state or state.transformed)then return end
  if not state or not state.damage or not state.controls then return end
  local damage=state.damage;local max=e.ComponentGetValue2(damage,'max_hp')
  if death.mode=='weak_respawn'then
   local _,maximum,terminal=parity.penalty(e.ComponentGetValue2(damage,'hp'),max,true)
   if terminal then
    death.original.payload.death.state='terminal';death.original.payload.death.cause='low_life'
    replace_original(death.original);own_death(death.original,'terminal',death.mode,death.poly);self:terminal(player,'low_life',death.original);return
   end
   max=maximum
  end
  replace_original(death.original);own_death(death.original,'applying',death.mode,death.poly)
  if death.mode=='weak_respawn' then e.ComponentSetValue2(damage,'max_hp',max)end
  e.ComponentSetValue2(damage,'hp',max)
  local effect=e.LoadGameEffectEntityTo(player,'data/entities/misc/effect_protection_all.xml')
  local c=assert(e.EntityGetFirstComponentIncludingDisabled(effect,'GameEffectComponent'))
  e.ComponentSetValue2(c,'frames',60*40)
  e.EntityAddComponent2(effect,'UIIconComponent',{icon_sprite_file='data/ui_gfx/status_indicators/protection_all.png',name=e.GameTextGet('$noitatogether_spawncamp_buff_name'),description=e.GameTextGet('$noitatogether_spawncamp_buff_desc'),display_above_head=true,display_in_hud=true,is_perk=false})
  local x,y=checkpoint();e.EntitySetTransform(player,x,y)
  own_death(death.original,'applied_session',death.mode,death.poly);self:submit(death.original);self.life={deathEventId=death.original.eventId,state='playing'};e.GlobalsSetValue('nt2_c1_life',json.encode_saved({world=self.scope,life=self.life}));self.death=nil
 end
 function self:completion_tick(player,frame)
  if not self.completion or not self.ready or self.disabled or self.own_uncertain then return end
  local id,state=resolver.resolve_player(e,player)
  if not self.terminal_state and not self.own_uncertain and state and state.dead and state.damage then
   local death=self.death
   if not death and frame>self.last_death+30 then
    local won=e.GameHasFlagRun('ending_game_completed');local mode=self.rules.NT_death_penalty;local cause=state.transformed and 'polymorph'or 'normal'
    local original=self:capture('death',{state=won and 'won'or mode=='end'and 'terminal'or 'respawning',cause=cause},nil,true)
    if original then self.last_death=frame;death={original=original,mode=mode,poly=state.transformed,frame=frame,player=id,deferred_camera=true};self.death=death end
   end
   if death and not death.published and(not death.poly or not state.transformed)then
    if death.mode=='weak_respawn'and e.ComponentGetValue2(state.damage,'max_hp')*.8<1 then death.original.payload.death.state='terminal';death.original.payload.death.cause='low_life'end
    death.published=true;self:submit(death.original);own_death(death.original,'waiting',death.mode,death.poly)
    if death.original.payload.death.state~='respawning'then own_death(death.original,'terminal',death.mode,death.poly);self:terminal(id,death.original.payload.death.cause,death.original)end
   end
  end
  if self.death and id and state then self:death_tick(frame,id,state)end -- Own respawn retains every connection/Controls hold.
  while self.queue[1]and self.local_accepted and self.local_accepted[self.queue[1].eventId..':0']do table.remove(self.queue,1);persist_queue()end
  for _,original in ipairs(self.queue)do if original.payload.death and(not self.death or self.death.original.eventId~=original.eventId or self.death.published)then send(json.encode({kind='c1',publish=original}),true);break end end
  if self.life and self.local_accepted and self.local_accepted[self.life.deathEventId..':life']then self.life=nil;e.GlobalsSetValue('nt2_c1_life','')end
  if #self.queue==0 and self.life then send(json.encode({kind='c1',life=self.life}),true)end
  for key,result in pairs(self.receipts)do local event,sequence=key:match('^([^:]+):(%d+)$');if event and not(self.local_accepted and self.local_accepted[key])then send(json.encode({kind='c1',receipt={eventId=event,sequence=sequence,state=result.state,reason=result.reason}}),true)end end
 end
 function self:tick(playing,player,frame)
  if e.GameGetWorldStateEntity then local ok,world=pcall(e.GameGetWorldStateEntity);if not ok or type(world)~='number'or world<=0 then self.playing=false;return end end
  if not self.recovered and player then self.recovered=M.recover(e,player)end
  self.playing=playing==true and not self.terminal_state;self.player=player
  local saved=json.decode_saved(e.GlobalsGetValue('nt2_c1_outgoing',''),{bytes=131072,nodes=16384});if saved and saved.world==self.scope then
   if saved.context==self.context then self.queue=saved.queue else persist_queue()end
  end
  -- Item/status/biome Lua states cannot use require/FFI. Reserve a bounded pool
  -- of immutable native-generated IDs here; hooks consume only Globals tokens.
  local active=self.playing and self.ready and not self.disabled
  if active and self.context and #self.context==32 then
   local tokens=json.decode_saved(e.GlobalsGetValue('nt2_c1_tokens',''))
   if not tokens or tokens.world~=self.scope or tokens.context~=self.context then tokens={world=self.scope,context=self.context,ids={}}end
   for i=#tokens.ids+1,32 do local id=identity();if not id then break end;tokens.ids[#tokens.ids+1]=id end
   e.GlobalsSetValue('nt2_c1_tokens',json.encode_saved(tokens))
  else e.GlobalsSetValue('nt2_c1_tokens','')end
  e.GlobalsSetValue('nt2_c1_authority',json.encode_saved({world=self.scope or '',revision=self.revision,count=self.count,frame=frame,player=player or 0,playing=active,context=self.context or ''}))
  if not self.playing or not self.ready or self.disabled then self:close_camera();self:completion_tick(player,frame);return end
  local id,state=resolver.resolve_player(e,player)
  if state and state.damage and(not self.wait or self.wait.damage~=state.damage)then
   local old=e.ComponentGetValue2(state.damage,'wait_for_kill_flag_on_death')==true
   local marker
   for _,c in ipairs(e.EntityGetComponentIncludingDisabled(id,'VariableStorageComponent')or{})do if e.ComponentGetValue2(c,'name')=='nt2_c1_wait'then marker=c;old=e.ComponentGetValue2(c,'value_bool')==true;break end end
   marker=marker or e.EntityAddComponent2(id,'VariableStorageComponent',{name='nt2_c1_wait',value_bool=old})
   self.wait={player=id,damage=state.damage,old=old,marker=marker}
  end
  if state and state.damage then e.ComponentSetValue2(state.damage,'wait_for_kill_flag_on_death',true)end
  if self.own_uncertain and state and not state.dead then self.own_uncertain=nil;self.local_status=nil end
  if id and not self.own_uncertain then self:death_tick(frame,id,state)end
  if self.camera then
   if frame>=self.camera.until_frame or self.terminal_state then self:close_camera()
   elseif id and state and not state.transformed then local x,y=checkpoint();e.EntitySetTransform(id,x,y);e.GameSetCameraPos(self.camera.x,self.camera.y)end
  end
  while self.queue[1]and self.local_accepted and self.local_accepted[self.queue[1].eventId..':0']do table.remove(self.queue,1);persist_queue()end
  if self.queue[1]and(not self.death or self.queue[1].eventId~=self.death.original.eventId)then send(json.encode({kind='c1',publish=self.queue[1]}))end
  if self.life and self.local_accepted and self.local_accepted[self.life.deathEventId..':life']then self.life=nil;e.GlobalsSetValue('nt2_c1_life','')end
  if #self.queue==0 and self.life then send(json.encode({kind='c1',life=self.life}))end
  if self.terminal_state or self.own_uncertain then return end
  if self.ban_pending and state and state.ready and not state.transformed then
   if e.HandleBannedPerksEvent()then self.ban_pending=nil end
  end
  if not state or not state.ready then return end
  effects:tick(frame,id,true)
  local event=self.event;if not event or not event.eventId then return end
  local key=event.eventId..':'..event.sequence;local result=self.receipts[key]
  if not result then
   if marker(event.eventId,event.sequence)~=''then result={state='uncertain',reason='ambiguous_restore'}
   elseif not self.permit then send(json.encode({kind='c1',apply={eventId=event.eventId,sequence=event.sequence}}));return
   else
    mark(event.eventId,event.sequence,'applying')
    local ok,status,reason=pcall(effects.apply,effects,id,event,self.rules,self.count,frame,event.capturedRules)
    result={state=ok and status or 'uncertain',reason=ok and reason or 'effect_failed'}
    mark(event.eventId,event.sequence,result.state)
   end
   self.receipts={[key]=result}
   trace(event.eventId,event.sequence,result.state,result.reason,frame)
  end
  if self.local_accepted and self.local_accepted[key]then return end
  send(json.encode({kind='c1',receipt={eventId=event.eventId,sequence=event.sequence,state=result.state,reason=result.reason}}))
 end
 return self
end
return M
