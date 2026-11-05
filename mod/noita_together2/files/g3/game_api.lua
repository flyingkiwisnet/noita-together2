-- Fixed local engine adapter; never accepts entity/file paths from network input.
local M={}
local logging_enabled=dofile('mods/noita_together2/files/g3/social_preferences.lua').get(_G,'NT_ENABLE_LOGGER')
function M.diagnostics(playing,d,ghosts)
    if not logging_enabled or d.debug and d.debug.enabled~=true then return end
    print("NT2_G3|event=movement_path|playing="..tostring(playing).."|heartbeat_failure="..d.heartbeat_failure.."|holds="..d.holds.."|recoveries="..d.recoveries.."|hold_reason="..d.hold_reason.."|sampled="..d.sampled.."|accepted="..d.movement_accepted.."|peer_replies="..d.peer_replies.."|nonempty="..d.nonempty_batches.."|ghosts="..ghosts.."|clock_invalid="..d.clock_invalid.."|pose_invalid="..d.pose_invalid.."|player_or_sequence="..d.player_or_sequence)
end
function M.movement()
    return {alive=EntityGetIsAlive,time=GameGetRealWorldTimeSinceStarted,position=function(player)
        local x,y,rotation=EntityGetTransform(player)
        local velocity=EntityGetFirstComponentIncludingDisabled(player,"VelocityComponent")
        local vx,vy=0,0;if velocity then vx,vy=ComponentGetValue2(velocity,"mVelocity") end
        return x,y,rotation or 0,vx<0 and -1 or 1,vx,vy
    end}
end
function M.ghosts()
    local labels=dofile('mods/noita_together2/files/g3/ghost_labels.lua').new(_G)
    return {alive=function(entity)local live=entity~=nil and EntityGetIsAlive(entity);if entity and not live then labels:remove(entity)end;return live end,time=GameGetRealWorldTimeSinceStarted,spawn=function(x,y)
        return EntityLoad("mods/noita_together2/files/g3/ghost.xml",x,y)
    end,name=function(entity,name)return labels:sync(entity,name)end,remove=function(entity)labels:remove(entity);EntityKill(entity)end,opacity=function(entity,alpha)
        for _,id in ipairs(EntityGetComponentIncludingDisabled(entity,'SpriteComponent')or {})do ComponentSetValue2(id,'alpha',alpha)end
        for _,child in ipairs(EntityGetAllChildren(entity)or {})do for _,id in ipairs(EntityGetComponentIncludingDisabled(child,'SpriteComponent')or {})do ComponentSetValue2(id,'alpha',alpha)end end
    end,position=function(entity,x,y,rotation,facing)
        EntitySetTransform(entity,x,y,rotation,facing,1)
    end}
end
-- Current local subject, following legacy player_unit + GameStats.is_player
-- polymorph detection. Never select the first arbitrary transformed NPC.
local function read(fn,...)
 if type(fn)~='function'then return nil end;local ok,a=pcall(fn,...);if ok then return a end
end
function M.transformed(e,id)
 for _,tag in ipairs({'polymorphed','polymorphed_player','polymorphed_cessation'})do if read(e.EntityHasTag,id,tag)==true then return true end end;return false
end
function M.player_state(e,id,trusted)
 if type(id)~='number'or id<=0 or id~=math.floor(id)or read(e.EntityGetIsAlive,id)~=true then return nil end
 if e.EntityGetRootEntity and read(e.EntityGetRootEntity,id)~=id then return nil end
 local stats=read(e.EntityGetFirstComponentIncludingDisabled,id,'GameStatsComponent')
 local local_player=stats and read(e.ComponentGetValue2,stats,'is_player')
 if local_player==false or local_player~=true and read(e.IsPlayer,id)~=true and not trusted then return nil end
 local controls=read(e.EntityGetFirstComponentIncludingDisabled,id,'ControlsComponent')
 local damage=read(e.EntityGetFirstComponentIncludingDisabled,id,'DamageModelComponent')
 if controls==0 then controls=nil end;if damage==0 then damage=nil end
 local hp=damage and read(e.ComponentGetValue2,damage,'hp');local dead=type(hp)=='number'and hp==hp and hp<0
 return {id=id,controls=controls,damage=damage,dead=dead,ready=controls~=nil and damage~=nil and not dead,transformed=M.transformed(e,id)}
end
function M.ready(e,id)return (M.player_state(e,id,true)or {}).ready==true end
function M.resolve_player(e,known)
 if type(e.EntityGetWithTag)~='function'then local state=M.player_state(e,known,true);return state and known or nil,state end
 local found,seen={},{};local count=0
 for _,tag in ipairs({'player_unit','polymorphed','polymorphed_player','polymorphed_cessation'})do
  local values=read(e.EntityGetWithTag,tag);if values~=nil and type(values)~='table'then return nil end
  for _,id in ipairs(values or {})do
   count=count+1;if count>1024 then return nil end
   if not seen[id]then seen[id]=true;local state=M.player_state(e,id,false);if state then found[#found+1]=state end end
  end
 end
 if #found~=1 then return nil end
 return found[1].id,found[1]
end
-- Fixed numeric/boolean report only. No filesystem calls, raw errors, identity,
-- player text or arbitrary engine objects cross this optional copy boundary.
function M.debug(bridge,context,ffi)
 local keys={'peer_copies','peer_unavailable','peer_denied','social_frames','hud_samples','hud_copied','hud_received','hud_refused','poi_copied','poi_refused','social_errors','movement_errors'}
 local self={revision=1,frame=0,time_ms=0,roster=0,poses=0,recent={},before_loss={},after_loss={},truncated=false}
 for _,key in ipairs(keys)do self[key]=0 end
 local seen=false;local previous=0;local last=nil;local last_frame=-60;local dirty=true
 local function number(value,max)
  if type(value)~='number'or value~=value or value<0 then return 0 end
  return math.min(max or 2147483647,math.floor(value))
 end
 local function event(kind,playing,copied)
  return {frame=self.frame,time_ms=self.time_ms,kind=kind,playing=playing==true,copied=math.max(-1,math.min(864,tonumber(copied)or -1)),roster=self.roster,poses=self.poses,canonical_roster=self.canonical_roster}
 end
 local function remember(e)
  if self.first_loss and e~=self.first_loss and #self.after_loss<4 then self.after_loss[#self.after_loss+1]=e end
  if #self.recent==8 then table.remove(self.recent,1);self.truncated=true end
  self.recent[#self.recent+1]=e;dirty=true
 end
 local function encode_event(e)
  return '{"frame":'..e.frame..',"time_ms":'..e.time_ms..',"kind":'..e.kind..',"playing":'..tostring(e.playing)..',"copied":'..e.copied..',"roster":'..e.roster..',"poses":'..e.poses..',"canonical_roster":'..(e.canonical_roster or 'null')..'}'
 end
 function self:error(kind)
  local key=kind==7 and 'social_errors'or 'movement_errors';self[key]=number(self[key]+1)
  if self.enabled then remember(event(kind==7 and 7 or 6,self.playing,-1))end
 end
 function self:canonical(count)self.canonical_roster=number(count,32)end
 function self:social(d,c,i)
  self.social_frames=number(d.frames);self.hud_samples=number(c.samples);self.hud_copied=number(i.hud_accepted);self.hud_received=number(d.received)
  self.hud_refused=number(i.hud_unavailable);self.poi_copied=number(i.poi_accepted);self.poi_refused=number(i.poi_unavailable)
  local errors=number(i.native_error)+number(d.invalid)+number(d.log_errors);self.social_errors=math.max(self.social_errors,number(errors))
 end
 function self:observe(frame,playing,copied,roster,poses,ghosts)
  self.frame=number(frame);local ok,now=pcall(GameGetRealWorldTimeSinceStarted);self.time_ms=ok and type(now)=="number" and number(now*1000,9007199254740991)or 0
  self.playing=playing==true;self.roster=number(roster,32);self.poses=number(poses,4)
  local mode=false;if bridge.nt2_g3_debug_mode then local good,value=pcall(bridge.nt2_g3_debug_mode,context);mode=good and value==1 end
  local enabled=mode and logging_enabled;local changed=self.enabled~=enabled;self.enabled=enabled
  if enabled then
   if copied==864 then self.peer_copies=number(self.peer_copies+1)elseif copied==0 then self.peer_unavailable=number(self.peer_unavailable+1)else self.peer_denied=number(self.peer_denied+1)end
   local count=number(ghosts,32);local kind=0
   if seen and previous>0 and count==0 then
    if not playing then kind=4 elseif self.canonical_roster==0 then kind=3 else kind=1 end
   elseif seen and previous==0 and count>0 then kind=2 end
   if kind~=0 or self.frame-last_frame>=60 then local e=event(kind,playing,copied);if kind==1 and not self.first_loss then for index=math.max(1,#self.recent-3),#self.recent do self.before_loss[#self.before_loss+1]=self.recent[index]end;self.first_loss=e end;if kind==2 and self.first_loss and not self.recovery then self.recovery=e end;remember(e) end
   seen=seen or count>0;previous=count
  end
  if not changed and (not enabled or not dirty and self.frame-last_frame<60)then return end
  if not changed and self.frame-last_frame<60 and not (self.first_loss and not self.loss_sent)then return end
  local parts={'"revision":1','"enabled":'..tostring(enabled),'"logger":'..tostring(logging_enabled),'"frame":'..self.frame,'"time_ms":'..self.time_ms,'"roster":'..(enabled and self.roster or 0),'"poses":'..(enabled and self.poses or 0),'"truncated":'..tostring(enabled and self.truncated or false)}
  for _,key in ipairs(keys)do parts[#parts+1]='"'..key..'":'..(enabled and self[key]or 0)end
  parts[#parts+1]='"canonical_roster":'..(enabled and self.canonical_roster or 'null')
  parts[#parts+1]='"first_loss":'..(enabled and self.first_loss and encode_event(self.first_loss)or 'null')
  local events={};if enabled then for _,e in ipairs(self.recent)do events[#events+1]=encode_event(e)end end
  parts[#parts+1]='"recent":['..table.concat(events,',')..']'
  for _,key in ipairs({'before_loss','after_loss'})do local context={};if enabled then for _,e in ipairs(self[key])do context[#context+1]=encode_event(e)end end;parts[#parts+1]='"'..key..'":['..table.concat(context,',')..']'end
  parts[#parts+1]='"recovery":'..(enabled and self.recovery and encode_event(self.recovery)or 'null')
  local bytes='{'..table.concat(parts,',')..'}'
  if bridge.nt2_g3_debug_publish and #bytes<=4096 then local good,result=pcall(bridge.nt2_g3_debug_publish,context,bytes,#bytes);if good and result==1 then last_frame=self.frame;dirty=false;self.loss_sent=enabled and self.first_loss~=nil end end
 end
 return self
end
return M

