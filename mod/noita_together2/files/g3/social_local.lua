-- Per-actor A3 presentation, original/revision fences and local cancellation.
local M={};local root='mods/noita_together2/files/g3/'
local catalogue=dofile(root..'social_catalogue.lua');local preferences=dofile(root..'social_preferences.lua');local controls=dofile(root..'social_controls.lua');local menu_module=dofile(root..'social_menu.lua');local visual_module=dofile(root..'social_visual.lua')
local function read(e,key,...)local f=e[key];if type(f)~='function'then return nil end;local ok,a,b=pcall(f,...);if ok then return a,b end end
local function finite(v)return type(v)=='number'and v==v and v>=0 and v<math.huge end
function M.catalogue()return catalogue end
function M.new(e,publish,renderer)
 local self={run=nil,local_actor=nil,peers={},states={},versions={},rendered={},player=nil,local_emote=nil,pending_stop=nil,robe='purple',confirmed_robe='purple',cosmetic_sent=nil,enabled=true,frame=nil,need_cosmetic=true,social_ready=false}
 local visual=visual_module.new(e);self.visual=visual
 local function notice(s)renderer.model.feedback=s;if e.GamePrint then pcall(e.GamePrint,'NT2: '..s)end end
 local function flags()return {hat=read(e,'HasFlagPersistent','secret_hat')==true,amulet=read(e,'HasFlagPersistent','secret_amulet')==true,gem=read(e,'HasFlagPersistent','secret_amulet_gem')==true}end
 local function cosmetic_key(robe,f)return robe..tostring(f.hat)..tostring(f.amulet)..tostring(f.gem)end
 local function flush_stop()
  if not self.pending_stop then return true end;local stop=self.pending_stop;local id,reason,retry,copied=publish('emote',{emoteId=stop.emoteId,active=false},stop.identity)
  stop.identity=id or retry or stop.identity;if id and copied~=false then self.pending_stop=nil;return true end;return false
 end
 function self:cancel(send_stop)
  local a=self.local_emote;if a then if send_stop then self.pending_stop={emoteId=a.id,identity=nil}end;visual:stop(a.player);self.local_emote=nil end
 end
 local menu=menu_module.new(e,controls.new(e),{notice=notice,feedback=function(s)renderer.model.feedback=s end,before_open=function()renderer.model.composer=false;renderer.model.bank_open=false;if renderer.bank then renderer.bank.ui.open=false end;renderer.model.show_players=false;renderer.lock:close(true)end,
  select=function(id)
   if not self.social_ready then notice('Social controls are busy; try again');return false end
   local c=read(e,'EntityGetFirstComponentIncludingDisabled',self.player,'ControlsComponent');local d=read(e,'EntityGetFirstComponentIncludingDisabled',self.player,'DamageModelComponent');local hp=d and read(e,'ComponentGetValue2',d,'hp')
   if not c or read(e,'ComponentGetValue2',c,'enabled')~=true or not finite(hp)or not visual:compatible(self.player)then notice('Current player input or animation is unavailable for emotes');return false end
   if not flush_stop()then notice('Finishing the previous emote; try again');return false end
   local identity,reason=publish('emote',{emoteId=id,active=true})
   if not identity then notice(reason or 'Social controls are busy; try again');return false end
   self:cancel(false);self.local_emote={id=id,identity=identity,selected=self.frame,player=self.player};return true
  end,
  robe=function(robe)
   if not self.social_ready then notice('Colour change is busy; try again');return false end
   if self.pending_cosmetic then notice('Previous colour change is awaiting its result; try again');return false end
   if not catalogue.robe(robe)then return false end
   local f=flags();f.robeId=robe;local identity,reason=publish('cosmetic',f)
   if not identity then notice(reason or 'Colour change unavailable; try again');return false end
   self.pending_cosmetic={identity=identity,previous=self.confirmed_robe,desired=robe,flags=f,manual=true}
   self.robe=robe;self.cosmetic_sent=cosmetic_key(robe,f);self.need_cosmetic=false
   visual:colour(self.player,robe,f,true,true);return true
  end});self.menu=menu
 function self:roster(run,local_actor,players)
  if self.run and self.run~=run then self:close();self.robe='purple' end
  self.run=run;self.local_actor=local_actor;local next={}
  for _,p in ipairs(players)do next[p.actor]=p end
  for actor,old in pairs(self.peers)do
   local p=next[actor];if not p or p.status~=1 or p.ghost~=old.ghost then
    local entity=self.rendered[actor];if entity then visual:clear_colour(entity.entity);self.rendered[actor]=nil end
    local s=self.states[actor];if s and s.emote then s.emote.blocked=true end
    if not p then self.states[actor]=nil;self.versions[actor]=nil end
   end
  end;self.peers=next
 end
 local function accept(c,kind,now)
  if type(c)~='table'or type(c.actor)~='string'or type(c.ghost)~='string'or #c.ghost~=32 or c.ghost:find('[^0-9a-f]')or type(c.event)~='string'or #c.event~=36 or not finite(c.sequence)or c.sequence<1 or c.sequence%1~=0 or not finite(now)then return false end
  if kind=='emote'then if not catalogue.entries[c.id]or c.duration~=catalogue.entries[c.id].frames or type(c.active)~='boolean'or not finite(c.age_ms)or c.age_ms>4294967295 then return false end
  elseif not catalogue.robe(c.robe)or type(c.hat)~='boolean'or type(c.amulet)~='boolean'or type(c.gem)~='boolean'then return false end
  local peer=self.peers[c.actor];if not peer or peer.status~=1 or peer.ghost~=c.ghost or not finite(c.revision)or c.revision<1 or c.revision%1~=0 then return false end
  local state=self.states[c.actor]or {};self.states[c.actor]=state;local versions=self.versions[c.actor]or {};self.versions[c.actor]=versions;local old=versions[kind]
  local signature=kind=='emote'and c.event..':'..c.sequence..':'..c.id..':'..c.duration or c.event..':'..c.sequence..':'..c.robe..':'..tostring(c.hat)..tostring(c.amulet)..tostring(c.gem)
  if old and c.revision<old.revision then return false end
  if old and c.revision==old.revision and signature~=old.signature then return false end
  if old and state[kind]and state[kind].event==c.event and signature~=old.signature then return false end
  if kind=='emote'then
   if not catalogue.entries[c.id]or c.duration~=catalogue.entries[c.id].frames or not finite(c.age_ms)then return false end
   local prior=state.emote;local blocked=prior and prior.event==c.event and (prior.blocked or not prior.active)
   local age=c.age_ms;if prior and prior.event==c.event then age=math.max(age,prior.age_ms+math.max(0,now-prior.at)*1000)end
   c={id=c.id,event=c.event,sequence=c.sequence,revision=c.revision,active=c.active and not blocked,blocked=blocked,ghost=c.ghost,age_ms=age,at=now,duration=c.duration}
  else
   if not catalogue.robe(c.robe)then return false end
   c={robe=c.robe,hat=c.hat,amulet=c.amulet,gem=c.gem,event=c.event,sequence=c.sequence,revision=c.revision,ghost=c.ghost,animate=old~=nil and state.cosmetic and state.cosmetic.ghost==c.ghost and state.cosmetic.robe~=c.robe}
  end
  versions[kind]={revision=c.revision,signature=signature};state[kind]=c;return true
 end
 function self:emote(c,now)return accept(c,'emote',now)end
 function self:cosmetic(c,now)return accept(c,'cosmetic',now)end
 function self:outcome(id,status,reason,event)
  if self.local_emote and self.local_emote.identity==id then
   if status==2 then self:cancel(true);notice(reason~=''and reason or 'Emote refused')elseif status==1 then self.local_emote.event=event end
  elseif self.pending_cosmetic and self.pending_cosmetic.identity==id and status~=0 then
   local pending=self.pending_cosmetic;self.robe=status==1 and pending.desired or self.confirmed_robe
   if status==1 then self.confirmed_robe=self.robe;if e.GlobalsSetValue then e.GlobalsSetValue('nt2_a3_robe',self.robe)end end
   local f=flags();self.cosmetic_sent=cosmetic_key(self.robe,pending.flags);if self.enabled then visual:colour(self.player,self.robe,f,true,false)end
   if status==2 then notice(reason~=''and reason or 'Colour selection refused')end
   self.pending_cosmetic=nil
  end
 end
 local function interrupt(player)
  local c=read(e,'EntityGetFirstComponentIncludingDisabled',player,'ControlsComponent');if not c or read(e,'ComponentGetValue2',c,'enabled')~=true then return true end
  for _,key in ipairs({'mButtonDownLeft','mButtonDownRight','mButtonDownDown','mButtonDownUp','mButtonDownFire'})do if read(e,'ComponentGetValue2',c,key)==true then return true end end;return false
 end
 function self:tick(playing,player,social_ready)
  self.social_ready=playing==true and social_ready==true
  local frame=read(e,'GameGetFrameNum');local now=read(e,'GameGetRealWorldTimeSinceStarted');if not finite(frame)or not finite(now)then self:pause();return end;self.frame=frame
  if player~=self.player then self:cancel(true);if self.player then visual:clear_colour(self.player)end;self.player=player;self.need_cosmetic=true
   local robe=read(e,'GlobalsGetValue','nt2_a3_robe','purple');self.confirmed_robe=catalogue.robe(robe)and robe or 'purple';self.robe=self.pending_cosmetic and self.pending_cosmetic.desired or self.confirmed_robe
  end
  local enabled=preferences.get(e,'NT_SHOW_EMOTES')
  if enabled~=self.enabled then self:cancel(true);visual:clear();self.rendered={};self.enabled=enabled;if not enabled then notice('Emote display disabled')end end
  if not playing then self:cancel(true);visual:clear();self.rendered={};menu:tick(player,false,enabled,preferences.get(e,'NT_EMOTE_BINDING'),true);return end
  if social_ready then flush_stop()end
  local f=flags();local signature=cosmetic_key(self.robe,f)
  if social_ready and not self.pending_cosmetic and (self.need_cosmetic or self.cosmetic_sent~=signature)then f.robeId=self.robe;local id=publish('cosmetic',f);if id then self.pending_cosmetic={identity=id,previous=self.confirmed_robe,desired=self.robe,flags=f,manual=false};self.cosmetic_sent=signature;self.need_cosmetic=false end end
  if enabled and player then visual:colour(player,self.robe,f,true,false)end
  -- A short copy-lease gap cannot latch the held menu closed. Gameplay owns
  -- presentation eligibility; each click still needs current copy permission.
  local down=menu:tick(player,playing,enabled,preferences.get(e,'NT_EMOTE_BINDING'),(renderer.model.composer or renderer.model.bank_open) or self.pending_stop~=nil)
  local a=self.local_emote;if a then
   local elapsed=frame-a.selected;local spec=catalogue.entries[a.id]
   local controls_component=read(e,'EntityGetFirstComponentIncludingDisabled',player,'ControlsComponent')
   local damage=read(e,'EntityGetFirstComponentIncludingDisabled',player,'DamageModelComponent');local hp=damage and read(e,'ComponentGetValue2',damage,'hp')
   local foreign=not controls_component or read(e,'ComponentGetValue2',controls_component,'enabled')~=true or not finite(hp)
   if not enabled or elapsed<0 or foreign or spec.frames>0 and elapsed>=spec.frames+2 or elapsed>15 and not down and interrupt(player)then self:cancel(true)
   elseif elapsed>=1 then if not a.started then a.started=visual:start(player,a.id,true,0);if not a.started then self:cancel(true)end end;visual:aim(player)end
   local own=(self.states[self.local_actor]or {}).emote
   if self.local_emote and own and a.event==own.event and not own.active then self:cancel(false)end
  end
  local ghosts=e.a3_ghosts and e.a3_ghosts()or {}
  for actor,p in pairs(self.peers)do if actor~=self.local_actor then
   local state=self.states[actor]or {};local entity=ghosts[p.ghost];local old=self.rendered[actor]
   if old and (old.entity~=entity or not enabled or p.status~=1)then visual:clear_colour(old.entity);self.rendered[actor]=nil;old=nil end
   if enabled and entity and p.status==1 then
    if not old then old={entity=entity};self.rendered[actor]=old end
    local c=state.cosmetic;if c then visual:colour(entity,c.robe,c,false,c.animate and old.cosmetic~=nil and old.cosmetic~=c.revision);old.cosmetic=c.revision;c.animate=false end
    local v=state.emote;if v then
     local age=v.age_ms+math.max(0,now-v.at)*1000;local frames=math.floor(age*60/1000);local active=v.active and not v.blocked and (v.duration==0 or frames<v.duration+2)
     if active and old.emote~=v.event then if visual:start(entity,v.id,false,frames)then old.emote=v.event end
     elseif not active then visual:stop(entity);old.emote=nil;v.active=false end
    end
    visual:opacity(entity,v and math.floor((v.age_ms+math.max(0,now-v.at)*1000)*60/1000)or 0)
   end
  end end
 end
 function self:before_hold(player)self.social_ready=false;self:cancel(true);menu:close(player)end
 function self:pause()self.social_ready=false;self:cancel(true);visual:clear();self.rendered={};menu:close(self.player)end
 function self:close()self:pause();self.pending_stop=nil;self.pending_cosmetic=nil;local robe=read(e,'GlobalsGetValue','nt2_a3_robe','purple');self.confirmed_robe=catalogue.robe(robe)and robe or 'purple';self.robe=self.confirmed_robe;self.states={};self.versions={};self.peers={};self.local_actor=nil;self.run=nil;self.need_cosmetic=true;self.cosmetic_sent=nil end
 return self
end
return M
