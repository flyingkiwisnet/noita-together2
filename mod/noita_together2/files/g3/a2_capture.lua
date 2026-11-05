-- Real game-thread observations. Missing/polymorphed/dead facts stay unknown.
local M={}
local biomes=dofile("mods/noita_together2/files/g3/a2_biomes.lua")
local function call(fn,...) if type(fn)~='function' then return nil end;local ok,a,b=pcall(fn,...);if ok then return a,b end end
local function finite(n,limit) return type(n)=='number' and n==n and math.abs(n)<=limit end
function M.sample(e,player)
 local result={}
 if not player or call(e.EntityGetIsAlive,player)~=true then return result end
 -- Form location is independent; never report its HP/inventory as ordinary HP.
 local transformed=call(e.EntityHasTag,player,'polymorphed') or call(e.EntityHasTag,player,'polymorphed_player') or call(e.EntityHasTag,player,'polymorphed_cessation')
 local damage=call(e.EntityGetFirstComponentIncludingDisabled,player,'DamageModelComponent')
 if not transformed and damage and damage~=0 then
  local hp=call(e.ComponentGetValue2,damage,'hp');local maximum=call(e.ComponentGetValue2,damage,'max_hp')
  if finite(hp,2147483.647) and hp>=0 then result.hp=hp end -- Wire uses milli-Noita HP; display multiplies by25.
  if finite(maximum,2147483.647) and maximum>0 then result.max_hp=maximum end
 end
 local x,y=call(e.EntityGetTransform,player)
 if finite(x,2147483.647) and finite(y,2147483.647) then result.x=x;result.y=y;result.depth=y end
 -- No coordinates samples the camera, which may be detached from the player.
 local biome=result.x and result.y and call(e.BiomeMapGetName,result.x,result.y)
 if type(biome)=='string' and #biome>0 and #biome<=128 and biome~='_EMPTY_' and not biome:find('[%z\1-\31\127]')then result.biome=biome
 elseif result.x and result.y and (biome==nil or biome==''or biome=='_EMPTY_')then
  if biomes.starting(e,result.x,result.y)then result.biome='$nt2_starting_mountain'
  elseif type(biome)=='string'then result.biome='_EMPTY_'end -- Observed unnamed, unlike an unavailable query.
 end
 if transformed then return result end
 -- Inspect actual inventory children; the legacy mod's private NT_got_sampo
 -- flag is not a source of truth for this implementation.
 local roots=call(e.EntityGetAllChildren,player);if type(roots)~='table' then return result end;local queue={};local seen={};local truncated=#roots>128;for i=1,math.min(#roots,128)do queue[#queue+1]=roots[i]end
 local at=1;result.sampo=false
 while at<=#queue and at<=128 do
  local id=queue[at];at=at+1
  if not seen[id] then seen[id]=true
   if call(e.EntityHasTag,id,'sampo') or call(e.EntityHasTag,id,'this_is_sampo') then result.sampo=true;break end
   local file=call(e.EntityGetFilename,id)
   if file=='data/entities/items/pickup/sampo.xml' then result.sampo=true;break end
   -- Noita returns nil for a leaf. An API exception is unavailable, not absence.
   local ok,children=pcall(e.EntityGetAllChildren,id);if not ok or children~=nil and type(children)~='table'then truncated=true;children={}end
   for _,child in ipairs(children or {})do if #queue<128 then queue[#queue+1]=child else truncated=true;break end end
  end
 end
 if result.sampo~=true and (truncated or at<=#queue)then result.sampo=nil end -- A truncated inventory is not proof of absence.
 return result
end
function M.publisher(e,send)
 local self={next=0,last=nil,last_sent=-math.huge,diagnostics={samples=0,known=0,unknown=0,biome=0,accepted=0,unavailable=0}}
 local function count(key)self.diagnostics[key]=math.min(2147483647,self.diagnostics[key]+1)end
 function self:tick(player,playing)
  local now=call(e.GameGetRealWorldTimeSinceStarted);if not finite(now,1000000000) then return end
  if not playing then return end
  if now<self.next then return end;self.next=now+0.5
  local value=M.sample(e,player)
  count('samples');count(value.hp~=nil and value.max_hp~=nil and 'known'or 'unknown')
  if value.biome~=nil then count('biome')end
  local key=table.concat({tostring(value.hp),tostring(value.max_hp),tostring(value.sampo),tostring(value.biome)},'|')
  if key~=self.last or now-self.last_sent>=1 then if send(value)==true then count('accepted');self.last=key;self.last_sent=now else count('unavailable')end end
 end
 return self
end
return M
