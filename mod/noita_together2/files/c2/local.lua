-- C2 local owner: current scoped facts and save-aware issue/receipt boundaries.
local root='mods/noita_together2/files/'
local json=dofile(root..'g3/bank_json.lua')
local store=dofile(root..'c2/storage.lua')
local resolver=dofile(root..'g3/game_api.lua')
local text=dofile(root..'c2/hourglass.lua').text
local world=dofile(root..'c2/world.lua')
local catalogue={ambrosia='magic_liquid_protection_all',berserk='magic_liquid_berserk',charm='magic_liquid_charm',confusion='material_confusion',speed='magic_liquid_movement_faster',levitation='magic_liquid_faster_levitation',speed_and_levitation='magic_liquid_faster_levitation_and_movement'}
local M={}
function M.new(e,identity,send,notice)
 local self={ready=false,playing=false,pages={}}
 local function persist()store.write(e,'state',self.saved)end
 local arena=world.new(e,persist)
 local handlers=e.NT2_C2_HOURGLASS_REGISTRY or dofile(root..'c2/hourglass.lua').new(e);e.NT2_C2_HOURGLASS_REGISTRY=handlers
 local function emit(key,value,protected)
  local encoded=json.encode(value);if not encoded then return false end
  return send('{"kind":"'..(protected and 'c2_completion'or 'c2')..'","'..key..'":'..encoded..'}',protected)
 end
 function self:receive(view)
  if type(view)~='table'or type(view.world)~='string'or #view.world~=64 then return false end
  if self.scope~=view.world then
   self.scope=view.world;self.ready=false;self.pages={}
   local saved,invalid=store.read(e,'state');if invalid then self.corrupt=true;self.status='C2 saved ownership data is corrupt; reconciliation is protected.'end;self.saved=saved and saved.world==self.scope and saved or{world=self.scope,notices={},steve=false}
   local outgoing=store.read(e,'outgoing');if outgoing and outgoing.world==self.scope then for _,v in ipairs(outgoing.queue or{})do v.senderDisposition='uncertain'end;store.write(e,'outgoing',outgoing)end
   store.write(e,'grant',{});store.write(e,'tokens',{world=self.scope,ids={}})
  end
  self.view=view;self.ready=view.initialized==true;self.snapshot=view.snapshot
  local snapshot=self.snapshot
  if type(snapshot)=='table'then
   local encounter=snapshot.encounter
   if encounter then
    local stamp=tostring(snapshot.sequence)..':'..encounter.encounterId..':'..tostring(view.participantsTotal)
    if self.page_stamp~=stamp then self.pages={};self.page_stamp=stamp end
    self.pages[(view.participantsPage or 0)+1]=encounter.participants or{}
    local all={};local complete=true
    for i=1,view.participantsPages or 1 do if not self.pages[i]then complete=false;break end;for _,v in ipairs(self.pages[i])do all[#all+1]=v end end
    self.roster=complete and #all==view.participantsTotal and all or nil
    self.readiness=nil
    for _,page in pairs(self.pages)do for _,member in ipairs(page)do if member.principalId==view.principal and member.worldToken==view.worldToken then self.readiness=member.readinessEpoch end end end
   else self.roster=nil;self.readiness=nil end
  end
  local outgoing=store.read(e,'outgoing')
  if outgoing and outgoing.world==self.scope then
   local accepted={};for _,id in ipairs(view.localAccepted or{})do accepted[id]=true end
   local queue={};for _,v in ipairs(outgoing.queue or{})do if not accepted[v.eventId..':0']then queue[#queue+1]=v end end;outgoing.queue=queue;store.write(e,'outgoing',outgoing)
  end
  return true
 end
 function self:steve(player,event)
  if self.snapshot.rules.rules.NT_sync_steve~=true then return 'skipped','sharing_disabled'end
  if e.GlobalsGetValue('TEMPLE_SPAWN_GUARDIAN','0')=='1'then return 'skipped','already_angered'end
  self.saved.steve=true;persist();e.GlobalsSetValue('TEMPLE_SPAWN_GUARDIAN','1')
  local x,y=e.EntityGetTransform(player);local exit=e.EntityGetClosestWithTag(x,y,'workshop_exit')
  if exit and exit~=0 and e.EntityGetIsAlive(exit)then local gx,gy=e.EntityGetTransform(exit);e.EntityLoad('data/entities/misc/spawn_necromancer_shop.xml',gx-120,gy+30)end
  e.GamePrintImportant(text(e,'$noitatogether_angered_gods_title',event.sourceName),text(e,'$noitatogether_angered_gods_subtitle'))
  e.GamePlaySound('data/audio/Desktop/event_cues.snd','event_cues/sampo_pick/create',x,y);e.GameScreenshake(150)
  return 'applied','effect_applied'
 end
 function self:delivery(player)
  local event=self.view.event;if type(event)~='table'then return end
  local old,invalid=store.read(e,'receipt');if invalid then self.status='C2 saved effect receipt is corrupt; effect remains protected.';return end
  local same=old and old.world==self.scope and old.eventId==event.eventId and old.sequence==event.sequence
  if same and old.state=='applying'and (old.permit~=self.view.permit or self.issued~=event.eventId..':'..event.sequence)then old.state='uncertain';old.reason='ambiguous_restore';store.write(e,'receipt',old)end
  if same and old.state~='applying'then emit('receipt',{eventId=event.eventId,sequence=event.sequence,state=old.state,reason=old.reason});return end
  if not player then return end
  if not self.view.permit then emit('apply',{eventId=event.eventId,sequence=event.sequence});return end
  if same then return end
  self.issued=event.eventId..':'..event.sequence
  local row={world=self.scope,eventId=event.eventId,sequence=event.sequence,state='applying',permit=self.view.permit};store.write(e,'receipt',row)
  local state,reason='skipped','unsupported_content'
  local ok,err=pcall(function()
   if event.recipientProgression~=self.saved.progression then state,reason='skipped','effect_not_applicable'
   elseif event.payload.steve then state,reason=self:steve(player,event)
   elseif event.payload.pickup or event.payload.completed then state,reason='applied','fact_reconciled'
   elseif event.payload.hourglass then local h=event.payload.hourglass;if catalogue[h.effectId]==h.materialId and h.catalogueRevision=='nt2-hourglass-builtins-1'then state,reason=handlers:apply(h.effectId,player,event.sourceName,{eventId=event.eventId,sequence=event.sequence})end end
  end)
  if not ok then state,reason='uncertain','engine_callback_failed';self.status='Shared effect could not be confirmed; it will not replay.';if e.print then e.print('NT2_C2|event=effect_failed|id='..event.eventId..'|reason='..tostring(err):gsub('[\r\n]',' '):sub(1,240))end end
  row.state=state;row.reason=reason;store.write(e,'receipt',row)
  emit('receipt',{eventId=row.eventId,sequence=row.sequence,state=row.state,reason=row.reason})
 end
 function self:tick(playing,player,frame)
  self.playing=playing==true;store.write(e,'mode',{managed=true});store.write(e,'grant',{})
  if self.corrupt then return end
  local restored,invalid=store.read(e,'state');if invalid then self.corrupt=true;self.status='C2 saved ownership data is corrupt; reconciliation is protected.';return end;if restored and restored.world==self.scope then self.saved=restored end
  -- The shared completion slot has no C2 consumer until scoped C2 metadata
  -- arrives. An unbound progression probe can otherwise block Race originals.
  if not self.view or not self.scope then return end
  local progression=store.progression(e);if not progression then self.status='C2 waiting for validated world progression.';return end
  if not self.view or self.view.progression~=progression then emit('state',{progression=progression},not playing);return end
  if not self.ready or type(self.snapshot)~='table'or not self.snapshot.rules then return end;self.status=nil
  if self.saved.progression~=progression then self.saved={world=self.scope,progression=progression,notices={},steve=e.GlobalsGetValue('TEMPLE_SPAWN_GUARDIAN','0')=='1'};persist();self.last_near=nil end
  local association=e.GlobalsGetValue('nt2_rejoin_local_world_v1','');if association~=''and self.saved.association~=association then self.saved.association=association;persist()end
  local id,actor=resolver.resolve_player(e,player);local valid=playing and id and actor and actor.ready and not actor.transformed
  local encounter=self.snapshot.encounter
  if not valid then self.last_near=nil;return end
  local pool=store.read(e,'tokens')or{};if pool.world~=self.scope or pool.context~=self.context then pool={world=self.scope,context=self.context,ids={}}end
  while #pool.ids<32 do local token=identity();if not token then break end;pool.ids[#pool.ids+1]=token end;store.write(e,'tokens',pool)
  local grant={playing=true,frame=frame,player=id,world=self.scope,context=self.context,progression=progression,rulesRevision=self.snapshot.rules.revision,hourglass=catalogue}
  if arena:bind(self.saved,identity,self.context)then
   if not self.view.bound or self.view.bound.arenaToken~=self.saved.arenaToken or self.view.bound.progression~=progression then emit('arena_bind',{progression=progression,arenaToken=self.saved.arenaToken},not playing)
   elseif encounter and encounter.progression==progression then
    grant.arenaToken=self.saved.arenaToken;grant.encounterId=encounter.encounterId;grant.stage=encounter.stage;arena:scope(self.saved,encounter,self.context)
    if encounter.stage=='locked'and(frame%60==0 or self.last_near==nil)then local sampo=arena:owned(self.saved,'disabled');if sampo then local x,y=e.EntityGetTransform(id);local sx,sy=e.EntityGetTransform(sampo);local near=world.near(x,y,sx,sy);if self.last_readiness~=self.readiness then self.last_readiness=self.readiness;self.last_near=nil end;if self.readiness and self.last_near~=near and emit('near',{encounterId=encounter.encounterId,arenaToken=self.saved.arenaToken,near=near,readinessEpoch=self.readiness})then self.last_near=near end end end
    if encounter.stage=='unlocked'or encounter.stage=='battle'then arena:unlock(self.saved)end
    if encounter.stage=='battle'then arena:battle(self.saved,id)end
   end
  end
  store.write(e,'grant',grant)
  for _,anchor in ipairs(e.EntityGetWithTag('nt2_c2_hourglass_group')or{})do
   local row=store.component(e,anchor,'nt2_c2_hourglass_group')
   if row and row.progression==progression and(not row.world or row.world==self.scope)and not row.claimed then
    row.world=self.scope;row.context=self.context;row.activationId=row.activationId or identity();store.mark(e,anchor,'nt2_c2_hourglass_group',row)
    if row.activationId then for _,checker in ipairs(e.EntityGetWithTag('hourglass_entity')or{})do local own=store.component(e,checker,'nt2_c2_hourglass');if own and own.anchor==anchor then local c=e.EntityGetFirstComponentIncludingDisabled(checker,'MaterialAreaCheckerComponent');if c then e.EntitySetComponentIsEnabled(checker,c,true)end end end end
   end
  end
  local angered=e.GlobalsGetValue('TEMPLE_SPAWN_GUARDIAN','0')=='1'
  if angered and not self.saved.steve then self.saved.steve=true;persist();if self.snapshot.rules.rules.NT_sync_steve then local original=dofile(root..'c2/hook_api.lua').capture({steve={progression=progression}});if original then dofile(root..'c2/hook_api.lua').submit(original)end end end
  local outgoing=store.read(e,'outgoing');if outgoing and outgoing.world==self.scope and outgoing.queue and outgoing.queue[1]then emit('publish',outgoing.queue[1])end
  self:delivery(id)
  local seen=self.saved.notices or{};self.saved.notices=seen
  for _,done in ipairs(self.snapshot.completions or{})do if done.progression==progression and not seen[done.eventId]then seen[done.eventId]=true;persist();if done.principalId~=self.view.principal then e.GamePrint(text(e,'$noitatogether_player_won',done.displayName))end end end
  if self.roster then
   local waiting={}
   for _,member in ipairs(self.roster)do
    if not member.near and encounter.stage=='locked'or not member.pickedUp and encounter.stage=='unlocked'then waiting[#waiting+1]=member.displayName..' ('..member.status..')'end
    local key=encounter.encounterId..':'..member.principalId
    if member.pickedUp and not seen[key]then seen[key]=true;persist();if member.principalId~=self.view.principal then local prefix=e.GameHasFlagRun('NT_got_sampo')and '$noitatogether_player_got_mcguffin'or '$noitatogether_player_got_mcguffin_waiting';if member.orbCount>=13 then prefix=prefix..'_orbs'end;e.GamePrint(text(e,prefix,member.displayName,e.GameTextGet('$item_mcguffin_'..world.name_index(member.orbCount)),tostring(member.orbCount)))end end
   end
   local text=#waiting>0 and 'Waiting for '..table.concat(waiting,', ')or'';local disabled=arena:owned(self.saved,'disabled');if disabled then local info=e.EntityGetFirstComponentIncludingDisabled(disabled,'UIInfoComponent');if info then e.ComponentSetValue2(info,'name',text~=''and text or 'All players are nearby')end end;if text~=self.waiting then self.waiting=text;if text~=''and notice then notice(text)end end
  end
 end
 return self
end
return M
