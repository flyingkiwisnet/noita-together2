-- Game-thread consumer seam. Only a trusted Rust projection adapter may call it.
-- Fixed local render callbacks; literal Unicode text, no entity/file paths or code.
local M={}
local function decimal(v)
 return type(v)=="string" and #v>0 and #v<=20 and not v:find("[^0-9]") and (#v==1 or v:sub(1,1)~="0")
  and (#v<20 or v<="18446744073709551615")
end
local function less(a,b) return #a<#b or (#a==#b and a<b) end
local function coordinate(v) return type(v)=="number" and v==v and v>=-2147483.648 and v<=2147483.647 end
function M.new(api,validate)
 local self={scope=nil,generation=nil,playing=false,roster={},marker_roster={},hud={},sequences={},markers={},dismissed={},dismiss_count=0,pending=nil}
 function self:admit_marker_actor(scope,generation,actor)
  if not self.playing or scope~=self.scope or generation~=self.generation or not validate.id(actor) then return false end
  local count=0;for _ in pairs(self.marker_roster) do count=count+1 end;if not self.marker_roster[actor] and count>=256 then return false end
  self.marker_roster[actor]=true;return true -- Trusted private Marker only, no HUD/emote authority.
 end
 local function clear_visible() api.clear_hud();api.clear_pois();self.hud={} end
 function self:bind(scope,generation,roster)
  if type(scope)~="string" or #scope==0 or #scope>512 or not validate.integer(generation,1,4294967295) or not validate.array(roster,32) then return false end
  local seen={};for _,actor in ipairs(roster) do if not validate.id(actor) or seen[actor] then return false end;seen[actor]=true end
  if self.scope==scope then
   if self.generation~=generation then self:pause();return false end
   for actor in pairs(self.roster) do if not seen[actor] then self.hud[actor]=nil;api.remove_hud(actor) end end
   self.roster=seen -- Trusted current roster; historical admission remains separate.
   self.playing=true;return true
  end
  clear_visible();self.scope=scope;self.generation=generation;self.roster=seen;self.marker_roster={};self.sequences={};self.markers={};self.dismissed={};self.dismiss_count=0;self.pending=nil;self.playing=true;return true
 end
 -- A missing optional copy lease stops application, but does not invalidate
 -- already validated immutable POI pages. Invalid/control transitions discard.
 -- A2 may also retain current display facts until their original deadline;
 -- this never keeps the consumer's playing gate or permits another packet.
 function self:pause(keep_prefix,keep_current_info) api.clear_hud(keep_current_info);self.hud={};self.playing=false;if not keep_prefix then self.pending=nil end end
 function self:receive_hud(scope,generation,actor,sequence,hp,maxhp,sampo)
  local now=api.time()
  if not self.playing or scope~=self.scope or generation~=self.generation or not self.roster[actor]
   or not validate.integer(sequence,1,4294967295) or (self.sequences[actor] and sequence<=self.sequences[actor])
   or not validate.integer(hp,0,2147483647) or not validate.integer(maxhp,1,2147483647) or type(sampo)~="boolean" then return false end
  self.sequences[actor]=sequence;self.hud[actor]={hp=hp,maxhp=maxhp,sampo=sampo};api.hud(actor,hp,maxhp,sampo);return true
 end
 function self:tick()end -- Retained information has no display-age timer.
 function self:remove_live(scope,generation,actor)
  if not self.playing or scope~=self.scope or generation~=self.generation or not self.roster[actor] then return false end
  self.hud[actor]=nil;api.remove_hud(actor);return true -- Keep replay fences and historical markers.
 end
 function self:begin_snapshot(scope,generation,high)
  if not self.playing or scope~=self.scope or generation~=self.generation or not decimal(high) then self:pause();return false end
  self.pending={high=high,live_high=high,after="0",markers={},count=0,pages=0};return true
 end
 function self:page(scope,generation,high,next_sequence,markers)
  local pending=self.pending
  local function refuse() self:pause();return false end
  if not self.playing or scope~=self.scope or generation~=self.generation or not pending or not decimal(high) or less(high,pending.live_high)
   or not decimal(next_sequence) or less(next_sequence,pending.after) or not validate.array(markers,32) then return refuse() end
  if pending.pages>=32 then return refuse() end
  pending.pages=pending.pages+1;pending.live_high=high
  local previous=pending.after
  for _,marker in ipairs(markers) do
   if type(marker)~="table" or not decimal(marker.sequence) or not less(previous,marker.sequence) or less(high,marker.sequence)
    or not (self.roster[marker.actor] or self.marker_roster[marker.actor]) or type(marker.text)~="string" or #marker.text>396 or marker.text:find("%z")
    or not coordinate(marker.x) or not coordinate(marker.y)
    then return refuse() end
   previous=marker.sequence
   if not less(pending.high,marker.sequence) then
    if pending.count>=256 then return refuse() end
    pending.count=pending.count+1
    if not self.dismissed[marker.sequence] then pending.markers[marker.sequence]=marker end
   end
  end
  if less(next_sequence,previous) or less(high,next_sequence) or (next_sequence==pending.after and less(next_sequence,pending.high)) then return refuse() end
  pending.after=next_sequence
  if not less(next_sequence,pending.high) then
   local old=self.markers;self.markers=pending.markers;self.pending=nil
   for sequence in pairs(old)do if not self.markers[sequence]then api.remove_poi(sequence)end end
   -- Reassert canonical metadata even if the local renderer discarded its
   -- view. The fixed adapter retains existing entities by sequence.
   for sequence,marker in pairs(self.markers) do api.poi(sequence,marker.actor,marker.text,marker.x,marker.y,marker.name)end
  end
  return true
 end
 function self:dismiss(sequence)
  -- A local preference over a previously committed marker needs its bound
  -- scope, not a fresh optional copy lease. The renderer gates actual pickup
  -- handling on gameplay; remote page application still requires playing.
  if not self.scope or not decimal(sequence) then return false end
  if self.dismissed[sequence] then return true end
  if not self.markers[sequence] or self.dismiss_count>=256 then return false end
  self.dismissed[sequence]=true;self.dismiss_count=self.dismiss_count+1;self.markers[sequence]=nil
  if self.pending then self.pending.markers[sequence]=nil end
  api.remove_poi(sequence);return true -- Remember BEFORE any caller sends an acknowledgement.
 end
 function self:close() self:pause();api.clear_pois();self.scope=nil;self.generation=nil;self.roster={};self.marker_roster={};self.sequences={};self.markers={};self.dismissed={};self.dismiss_count=0 end
 return self
end
return M
