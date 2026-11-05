-- Source prototype only. Not selected by init.lua and no native ABI frozen.
-- This layer stores intent/parks a saved waiting world; it never authorizes play,
-- consumes a grant, changes seed/inventory or releases player controls.
local codec=dofile("mods/noita_together2/files/rejoin/local_world.lua")
local M={};local key="nt2_waiting_intent_v1"
local function seed(value) return type(value)=="string" and #value<=20 and value:match("^%d+$") and (#value==1 or value:sub(1,1)~="0") and (#value<20 or value<="18446744073709551615") end
local function ready() local ok,id=pcall(GameGetWorldStateEntity);return ok and type(id)=="number" and id>0 end
local function actual() local ok,value=pcall(StatsGetValue,"world_seed");return ok and seed(tostring(value)) and tostring(value) or nil end
local function valid_loot(loot,value) return loot=="individual:0" or (tonumber(value)<=4294967295 and (loot=="shared:0:"..value or loot=="shared:1:"..value and value~="0")) end
function M.new(adapter)
 local m={state="unresolved",spawned=false,owner=nil,record=nil,reason="Startup intent is unresolved; world remains protected"}
 function m:world_initialized() self.spawned=false;self.owner=nil;self.record=nil;self.state="unresolved" end
 function m:spawn() self.spawned=true;self.owner=adapter.current_owner();self:tick() end
 function m:tick()
  if self.state=="offline" or self.state=="superseded" or not ready() then return end
  local value=actual();if not value then return end
  local ok,saved,saved_seed,loot,offline=pcall(function() return GlobalsGetValue(key,""),GlobalsGetValue(key.."_seed",""),GlobalsGetValue(key.."_loot",""),GlobalsGetValue(key.."_offline","0") end)
  if not ok then return end
  if saved~="" then
   local record=codec.decode_saved(saved)
   if not record or saved_seed~=value or not valid_loot(loot,value) then self.reason="Saved waiting intent differs; world remains protected";return end
   if self.state=="saved-waiting" and record==self.record then
    if adapter.current_owner()~=self.owner then self.state="superseded" end
    return
   end
   -- The protected Rust record/current issuer must validate exact marker/seed/
   -- loot and original scope. A flag/hash or successful lookup never means play.
   local disposition=adapter.park(record,value,loot)
   if disposition=="await-canonical-waiting" then self.record=record;self.state=offline=="1" and "offline-intent" or "parked";self.reason="Saved waiting world: canonical recovery/Start required" end
   return
  end
  if not self.spawned or not self.owner then return end
  if adapter.current_owner()~=self.owner then self.state="superseded";return end
  -- Cached typed snapshot only; no server/discovery/filesystem wait in a frame.
  local snapshot=adapter.snapshot(self.owner,value)
  if not snapshot then return end
  if snapshot.kind~="fresh-waiting" or snapshot.owner~=self.owner or adapter.current_owner()~=self.owner or snapshot.seed~=value or not seed(snapshot.seed) or not valid_loot(snapshot.loot,value) then self.reason="Waiting snapshot scope differs; world remains protected";return end
  local encoded=codec.encode_saved(snapshot.record);if not encoded then return end
  -- Commit the complete non-secret record LAST; partial fields cannot confer
  -- authority. The Rust protected record is durable before snapshot publication.
  GlobalsSetValue(key.."_seed",snapshot.seed);GlobalsSetValue(key.."_loot",snapshot.loot);GlobalsSetValue(key.."_offline","0");GlobalsSetValue(key,encoded)
  self.record=snapshot.record;self.state="saved-waiting";self.reason="Waiting-world intent saved; host Start still required"
 end
 function m:offline()
  if self.state=="offline" then return end
  if adapter.choose_offline(self.record,self.owner)==true then
   if self.record and ready() then GlobalsSetValue(key.."_offline","1") end
   self.state="offline";self.reason=nil
  end
 end
 return m
end
return M
