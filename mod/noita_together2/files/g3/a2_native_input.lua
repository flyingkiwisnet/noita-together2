-- Copy-only bounded outgoing messages. No pipe/network I/O on the game thread.
local M={}
local function quote(s)return '"'..s:gsub('\\','\\\\'):gsub('"','\\"')..'"'end
local function milli(n)return n<0 and -math.floor(-n*1000+0.5)or math.floor(n*1000+0.5)end
function M.new(dll,context,ffi)
 local buffer=ffi.new('uint8_t[4096]');local nonce=ffi.new('uint8_t[16]')
 local diagnostics={hud_accepted=0,hud_unavailable=0,poi_accepted=0,poi_unavailable=0,dismiss_accepted=0,dismiss_unavailable=0,oversize=0,native_error=0}
 local function count(key)diagnostics[key]=math.min(2147483647,diagnostics[key]+1)end
 local function copy(body,kind)
  if #body>4096 then count('oversize');count(kind..'_unavailable');return false end;ffi.copy(buffer,body,#body)
  local ok,value=pcall(function()return dll.nt2_g3_social_publish(context,buffer,#body)end)
  if not ok then count('native_error')end
  local accepted=ok and tonumber(value)==1;count(kind..(accepted and '_accepted'or '_unavailable'));return accepted
 end
 local self={diagnostics=diagnostics}
 -- One explicit intent may wait behind the DLL's in-flight sample. Its body
 -- and identity stay fixed until copied; no game-thread I/O or growing queue.
 local pending=nil;local copied_body=nil;local ready=true
 function self:set_ready(value)ready=value==true end
 local function explicit(body,kind)
  if #body>4096 then count('oversize');return false end
  if copied_body==body then return true,true end
  if pending then return pending.body==body,false end
  if ready and copy(body,kind)then copied_body=body;return true,true end
  pending={body=body,kind=kind};return true,false
 end
 function self:flush()
  if ready and pending and copy(pending.body,pending.kind)then copied_body=pending.body;pending=nil end
 end
 function self:pause()
  ready=false
  -- A start selected before protection must not begin after returning.
  -- social_local retains its separately identified Stop for reconciliation.
  if pending and pending.kind=='emote'and pending.body:find('"active":true',1,true)then pending=nil end
 end
 function self:close()pending=nil;copied_body=nil;ready=false end
 function self:identity()
  local ok,ready=pcall(function()return dll.nt2_g3_startup_nonce(nonce,16)end)
  if not ok or tonumber(ready)~=1 then return nil end
  return ffi.string(nonce,16):gsub('.',function(c)return string.format('%02x',c:byte())end)
 end
 function self:social(kind,identity,value)
  if type(identity)~='string'or #identity~=32 or identity:find('[^0-9a-f]')then return false end
  if kind=='emote'and value.active==false and pending and pending.kind=='emote'and pending.body:find('"emote":{"emoteId":'..quote(value.emoteId)..',"active":true}',1,true)then
   -- A completed/interrupted selection that was never copied must not start
   -- late. Never replace a copied/in-flight original; it needs its own Stop.
   pending=nil
  end
  local body
  if kind=='emote'then body='"emote":{"emoteId":'..quote(value.emoteId)..',"active":'..tostring(value.active)..'}'
  elseif kind=='cosmetic'then body='"cosmetic":{"robeId":'..quote(value.robeId)..',"hat":'..tostring(value.hat)..',"amulet":'..tostring(value.amulet)..',"gem":'..tostring(value.gem)..'}'
  else return false end
  diagnostics[kind..'_accepted']=diagnostics[kind..'_accepted']or 0;diagnostics[kind..'_unavailable']=diagnostics[kind..'_unavailable']or 0
  return explicit('{"kind":'..quote(kind)..',"id":'..quote(identity)..','..body..'}',kind)
 end
 function self:hud(sample)
  if pending then count('hud_unavailable');return false end
  local known=sample.hp~=nil and sample.max_hp~=nil
  local fields={'"hpKnown":'..tostring(known),'"sampoKnown":'..tostring(sample.sampo~=nil)}
  if known then fields[#fields+1]='"hpMilli":'..milli(sample.hp);fields[#fields+1]='"maxHpMilli":'..milli(sample.max_hp)end
  if sample.sampo~=nil then fields[#fields+1]='"sampo":'..tostring(sample.sampo)end
  if sample.biome then fields[#fields+1]='"biome":'..quote(sample.biome)end
  return copy('{"kind":"hud","hud":{'..table.concat(fields,',')..'}}','hud')
 end
 function self:poi(message,x,y)
  if pending then return nil,'A previous explicit action is waiting; keep the draft and try again'end
  local ok,ready=pcall(function()return dll.nt2_g3_startup_nonce(nonce,16)end)
  if not ok or tonumber(ready)~=1 then return nil,'Random operation identity unavailable'end
  local id=ffi.string(nonce,16):gsub('.',function(c)return string.format('%02x',c:byte())end)
  if explicit('{"kind":"poi","id":'..quote(id)..',"text":'..quote(message)..',"x":'..milli(x)..',"y":'..milli(y)..'}','poi')then return id end
  return nil,'Optional message lane is busy; keep the draft and try again'
 end
 function self:dismiss(sequence)if pending then return false end;return copy('{"kind":"dismiss","sequence":'..quote(sequence)..'}','dismiss')end
 return self
end
return M
