-- Candidate local saved-world association. No secrets or service eligibility here.
local M={}
-- These current facts come only from the optional scoped native display poll.
-- Generic status1, saved metadata and inactivity never establish a host wait.
function M.message(code,fallback)
 if code==1 then return "checking seed and mods; world protected" end
 if code==2 then return "waiting for host Start; world protected" end
 if code==3 then return "reconnecting to this world's run; world protected" end
 if code==4 then return "host Start confirmed; entering play" end
 return fallback=="reconnecting" and "reconnecting to this world's run; world protected" or "checking the current run; world protected"
end
local KEY="nt2_rejoin_local_world_v1"
local function text(value,max) return type(value)=="string" and #value>0 and #value<=max and not value:find("[%z\1-\31]") end
-- Noita writes Globals into XML attributes without escaping. Keep the saved
-- representation ASCII-safe and versioned; mailbox input stays original bytes.
local FORMAT="nt2h1:"
local function bound(max_bytes)
 max_bytes=max_bytes or 1024
 if type(max_bytes)~='number'or max_bytes%1~=0 or max_bytes<1 or max_bytes>131072 then return nil end
 return max_bytes
end
function M.encode_saved(record,max_bytes)
 local max=bound(max_bytes);if not max or not text(record,max) then return nil end
 return FORMAT..tostring(#record)..":"..(record:gsub(".",function(c) return string.format("%02x",string.byte(c)) end))
end
function M.decode_saved(value,max_bytes)
 local max=bound(max_bytes)
 if not max or type(value)~="string" or #value<10 or #value>2*max+14 then return nil end
 local digits,hex=value:match("^nt2h1:(%d+):([0-9a-f]+)$")
 local size=digits and tonumber(digits)
 if not size or size<1 or size>max or tostring(size)~=digits or #hex~=size*2 then return nil end
 local record=hex:gsub("..",function(pair) return string.char(tonumber(pair,16)) end)
 return text(record,max) and record or nil
end
local function loot_value(value)
 return value=="individual:0" or (text(value,64) and value:match("^shared:[01]:%d+$")~=nil)
end
local function seed(value)
 return text(value,20) and value:match("^%d+$") and (#value==1 or value:sub(1,1)~="0") and (#value<20 or value<="18446744073709551615")
end
local function ready() local ok,id=pcall(GameGetWorldStateEntity);return ok and type(id)=="number" and id>0 end
function M.new(adapter)
 local m={state="offline",record=nil,generation=0,pending=nil,chosen=false,hold=nil,reason=nil}
 local function finish(reason)
  if reason=="player_death_end" and adapter.finish then adapter.finish(reason) end
 end
 local function read()
  if not ready() then return nil,"pending" end
  local ok,record,actual,loot,retired,observed=pcall(function()
   return GlobalsGetValue(KEY,""),GlobalsGetValue(KEY.."_seed",""),GlobalsGetValue(KEY.."_loot",""),GlobalsGetValue(KEY.."_offline","0"),tostring(StatsGetValue("world_seed"))
  end)
  if not ok then return nil,"pending" end
  if not seed(observed) then return nil,"pending" end
  if record=="" then
   if actual=="" and loot=="" and (retired=="" or retired=="0") then return nil,"unrelated" end
   return nil,"partial"
  end
  record=M.decode_saved(record);if not record then return nil,"unassociated" end
  if actual=="" or loot=="" then return nil,"partial" end
  if not seed(actual) or not loot_value(loot) then return nil,"unassociated" end
  if not seed(observed) then return nil,"pending" end
  if observed~=actual then return nil,"seed-mismatch" end
  if retired=="1" then return nil,"offline" end
  return {record=record,seed=actual,loot=loot}
 end
 -- A proof-backed waiting marker uses the existing saved mailbox and hold
 -- owner. Persist at a positive loaded hook without changing live authority.
 function m:waiting(record,actual,loot)
  if not ready() or not text(record,1024) or not record:match('"kind"%s*:%s*"nt2%-world"') or not seed(actual) or not loot_value(loot) or tostring(StatsGetValue("world_seed"))~=actual then return false end
  if self.record or self.state=="terminal" or GlobalsGetValue(KEY.."_offline","0")=="1" then return false end
  local saved=read()
  if not (saved and saved.record==record and saved.seed==actual and saved.loot==loot) then
   GlobalsSetValue(KEY.."_seed",actual);GlobalsSetValue(KEY.."_loot",loot);GlobalsSetValue(KEY.."_offline","0");GlobalsSetValue(KEY,assert(M.encode_saved(record)))
   saved=read()
  end
  if not (saved and saved.record==record and saved.seed==actual and saved.loot==loot) then return false end
  -- A fresh world keeps its original native conversation through Start.
  -- Watch only its scoped intent; this marker never grants play or rejoins it.
  self.watching=saved;return true
 end
 function m:associate(record,actual,loot,admitted)
  if not admitted or not ready() or not text(record,1024) or not seed(actual) or not loot_value(loot) or tostring(StatsGetValue("world_seed"))~=actual then return false end
  if self.state=="terminal" or (self.state=="offline" and self.record) or GlobalsGetValue(KEY.."_offline","0")=="1" then return false end
  -- Native/Rust supplies only an actually admitted non-secret association.
  -- Persist only at engine-valid hooks, never init/pre-generation.
  GlobalsSetValue(KEY,assert(M.encode_saved(record)));GlobalsSetValue(KEY.."_seed",actual);GlobalsSetValue(KEY.."_loot",loot);GlobalsSetValue(KEY.."_offline","0")
  local saved=read();if not saved or saved.record~=record or saved.loot~=loot then return false end
  self.watching=nil;self.record=saved;self.state="playing";return true
 end
 function m:loaded()
  local saved,reason=read();if not saved then if reason=="pending" then return nil,reason end;return false,reason end
  -- A saved marker is insufficient: Rust must match its protected association.
  local known,intent,terminal,offline_choice,terminal_reason=adapter.known(saved)
  if known==false then return false,"foreign" end
  if self.record and self.record.record==saved.record and self.state~="offline" then return true end
  self.record=saved;self.state=known==nil and "lookup" or (terminal and "terminal" or "choice");self.generation=self.generation+1
  if terminal then self.reason="Rejoin is unavailable for this participation";finish(terminal_reason) end
  -- Protection owns only its own effect. It never enables another mod's input.
  self.hold=adapter.hold_owned();if offline_choice then self:offline() elseif intent and not terminal then self:rejoin() end;return true
 end
 function m:rejoin()
  if self.state~="choice" and self.state~="reconnecting" then return false end
  if not self.chosen then self.chosen=true;self.pending=adapter.request(self.record,self.generation) end
  self.state="reconnecting";return true
 end
 function m:tick()
  if not self.record and self.watching then
   local known,_,terminal,offline_choice,terminal_reason=adapter.known(self.watching)
   if known==true and (terminal or offline_choice) then
    self.record=self.watching;self.watching=nil;self.hold=adapter.hold_owned();self.state="terminal"
    self.reason="Rejoin is unavailable for this participation"
    finish(terminal_reason)
    if offline_choice then self:offline() end
    return
   end
  end
  if self.state=="terminal" then
   local _,_,_,offline_choice,terminal_reason=adapter.known(self.record)
   finish(terminal_reason)
   if offline_choice then self:offline() end
   return
  end
  if self.state=="lookup" then
   local known,intent,terminal,offline_choice,terminal_reason=adapter.known(self.record)
   if known==nil then return end
   if known==false then
    adapter.release_owned(self.hold,true);self.hold=nil;self.state="offline";self.record=nil;return
   end
   self.state=terminal and "terminal" or "choice"
   if offline_choice then self:offline();return end
   if terminal then self.reason="Rejoin is unavailable for this participation";finish(terminal_reason) elseif intent then self:rejoin() end
  end
  if self.state~="reconnecting" and self.state~="inspecting" then return end
  local result,generation,reason=adapter.poll(self.pending) -- cache poll only; no waiting/I/O
  if generation~=self.generation then return end
  if result=="offline" then self:offline();return end
  if result=="lost" then
   -- Permission/contact loss is not a world close. Keep the current native
   -- request so ordinary heartbeat/auth recovery can reconcile this context.
   self.state="reconnecting";return
  end
  if result=="terminal" then
   self.generation=self.generation+1;self.state="terminal";self.pending=nil;self.chosen=false
   self.reason=text(reason,240) and reason or "Rejoin is unavailable for this participation"
   finish(reason)
   -- Terminal means shared play stopped, not consent to offline play.
   -- Keep the owned hold and saved marker until the user chooses offline.
   return
  end
  if result=="admitted" then self.state="inspecting" end
  if result=="released" and self.state=="inspecting" and adapter.inspected(self.record,generation) then
   adapter.release_owned(self.hold);self.hold=nil;self.state="playing"
  end
 end
 function m:lost()
  if self.state=="playing" then self.hold=adapter.hold_owned();self.state="reconnecting";if not self.pending then self.chosen=false;self:rejoin() end end
 end
 function m:offline()
  if self.state=="offline" then return end
  self.generation=self.generation+1;self.state="offline";self.chosen=false;self.reason=nil
  adapter.retire(self.record);self.pending=nil
  if self.record and ready() then GlobalsSetValue(KEY.."_offline","1") end
  if self.hold then adapter.release_owned(self.hold,true);self.hold=nil end
 end
 function m:draw(gui)
  local compatibility;if adapter.compatibility_status then local ok,value=pcall(adapter.compatibility_status);if ok then compatibility=value end end
  if compatibility then GuiText(gui,10,35,'NT2: '..compatibility.title..'; world protected');GuiText(gui,10,47,compatibility.detail)
  elseif self.state=="choice" then
   GuiText(gui,10,35,"NT2: this saved world can ask to rejoin its previous run")
   if GuiButton(gui,9141,10,49,"Rejoin previous run") then self:rejoin() end
  elseif self.state=="terminal" then
   local result;if adapter.race_status then local ok,value=pcall(adapter.race_status,self.record,self.state,self.reason);if ok then result=value end end
   if result then GuiText(gui,10,35,"NT2: "..result.title.."; world protected");if result.detail~=''then GuiText(gui,10,47,result.detail)end
   else GuiText(gui,10,35,"NT2: "..self.reason.."; world protected")end
  elseif self.state=="lookup" then GuiText(gui,10,35,"NT2: checking saved association; world protected")
  elseif self.state=="inspecting" or self.state=="reconnecting" then
   local code=0;if adapter.display_status then local ok,value=pcall(adapter.display_status);if ok then code=value end end
   if adapter.lobby_status then
    local ok,current=pcall(adapter.lobby_status)
    -- Never ask a recovered, already-started world for another host Start.
    if ok and current and current.code==4 then code=4
    elseif code==2 and (not ok or not current or current.code~=2) then code=0 end
   end
   GuiText(gui,10,35,"NT2: "..M.message(code,self.state))
  end
  if self.state=="lookup" or self.state=="choice" or self.state=="reconnecting" or self.state=="inspecting" or self.state=="terminal" then
   if GuiButton(gui,9142,10,64,"Continue offline") then self:offline() end
  end
 end
 return m
end
return M
