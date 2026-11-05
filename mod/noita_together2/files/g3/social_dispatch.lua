-- UNAPPROVED framing seam: no I/O, paths, credentials or native authority creation.
-- Native integration must supply both authority and an already-owned batch sink.
local M={}
local function integer(n,lo,hi) return type(n)=="number" and n==n and n%1==0 and n>=lo and n<=hi end
local function literal(s)
 local i=1
 while i<=#s do
  local a=s:byte(i);local n,minimum
  if a<128 then if a<32 or a==127 then return false end;n=1;minimum=0
  elseif a>=194 and a<=223 then n=2;minimum=128
  elseif a>=224 and a<=239 then n=3;minimum=2048
  elseif a>=240 and a<=244 then n=4;minimum=65536 else return false end
  local cp=a%(2^(8-n-1));if n==1 then cp=a end
  for j=1,n-1 do local b=s:byte(i+j);if not b or b<128 or b>191 then return false end;cp=cp*64+b-128 end
  if cp<minimum or cp>1114111 or (cp>=55296 and cp<=57343) or (cp>=128 and cp<=159) then return false end
  i=i+n
 end
 return true
end
local function decimal(s) return #s>0 and #s<=20 and not s:find("[^0-9]") and (#s==1 or s:sub(1,1)~="0") and (#s<20 or s<="18446744073709551615") end
function M.decode(frame)
 if type(frame)~="string" or #frame<40 or #frame>32768 or frame:sub(1,5)~="NSF1\1" or frame:sub(6,8)~="\0\0\0" or frame:sub(39,40)~="\0\0" then return nil end
 local at=9
 local function bytes(n) if at+n-1>#frame then error("short frame") end;local v=frame:sub(at,at+n-1);at=at+n;return v end
 local function u(n) local v=0;local s=bytes(n);for i=n,1,-1 do v=v*256+s:byte(i) end;return v end
 local function str(max) local n=u(2);if n>max then error("string bound") end;local s=bytes(n);if not literal(s) then error("UTF8 literal") end;return s end
 local function dec() local s=str(20);if not decimal(s) then error("cursor") end;return s end
 local function signed() local v=u(4);if v>=2147483648 then v=v-4294967296 end;return v/1000 end
 local ok,result=pcall(function()
  local context=bytes(16);local generation=u(4);local sequence=u(4);local length=u(4);local count=u(2);bytes(2)
  if context==string.rep("\0",16) or generation==0 or sequence==0 or length~=#frame-40 or count>32 then error("header") end
  local commands={}
  for _=1,count do
   local tag=u(1);local c={tag=tag}
   if tag==1 then c.actor=str(128);c.name=str(396);c.sequence=u(4);c.hp=u(4);c.max_hp=u(4);local sampo=u(1)
    if #c.actor==0 or c.sequence==0 or c.hp>2147483647 or c.max_hp==0 or c.max_hp>2147483647 or sampo>1 then error("HUD bounds") end;c.sampo=sampo==1
   elseif tag==2 then c.actor=str(128);if #c.actor==0 then error("actor") end
   elseif tag==3 then c.high=dec()
   elseif tag==4 then c.high=dec();c.next=dec();local n=u(1);if n>32 then error("page capacity") end;c.markers={}
    for j=1,n do c.markers[j]={sequence=dec(),actor=str(128),name=str(396),text=str(396),x=signed(),y=signed()};if #c.markers[j].actor==0 then error("actor") end end
   elseif tag==7 then local n=u(1);if n>32 then error("roster capacity") end;c.actors={};local seen={};for j=1,n do local actor=str(128);if #actor==0 or seen[actor] then error("roster identity") end;seen[actor]=true;c.actors[j]=actor end
   elseif tag==8 or tag==9 then
    c.actor=str(128);c.ghost=str(32);c.revision=u(4);c.event=str(36);c.sequence=u(4)
    if #c.actor==0 or #c.ghost~=32 or c.ghost:find('[^0-9a-f]')or c.revision==0 or c.sequence==0 or #c.event~=36 or c.event:find('[^0-9a-f%-]')then error('presentation identity')end
    if tag==8 then c.id=u(1);local active=u(1);c.duration=u(2);c.age_ms=u(4);if c.id>=20 or active>1 then error('emote bounds')end;c.active=active==1
    else c.robe=u(1);local bits=u(1);if c.robe>=6 or bits>7 then error('cosmetic bounds')end;c.hat=bits%2==1;c.amulet=math.floor(bits/2)%2==1;c.gem=math.floor(bits/4)%2==1 end
   elseif tag==10 then
    c.run=str(512);c.local_actor=str(128);local n=u(1);if n>32 then error('player capacity')end;c.players={};local seen={}
    for j=1,n do local p={actor=str(128),name=str(396),status=u(1),ghost=str(32),order=j};if #p.actor==0 or seen[p.actor] or p.status<1 or p.status>4 or #p.ghost~=0 and (#p.ghost~=32 or p.ghost:find('[^0-9a-f]')) then error('player identity')end;seen[p.actor]=true;c.players[j]=p end
   elseif tag==11 then
    c.actor=str(128);c.revision=u(4);local hp,max=u(4),u(4);local flags=u(1);c.biome=str(128)
    if flags>7 or c.revision==0 or hp>2147483647 or max>2147483647 or flags%2==1 and max==0 then error('player facts')end
    if flags%2==1 then c.hp=hp;c.max_hp=max end;if math.floor(flags/2)%2==1 then c.sampo=math.floor(flags/4)%2==1 end
   elseif tag==12 then c.id=str(160);c.name=str(396);c.text=str(4096)
   elseif tag==13 then c.id=str(32);c.status=u(1);c.reason=str(396);c.draft=str(396);if c.status>2 then error('operation outcome')end
   elseif tag==14 then local n=u(2);if n>256 then error('dismissals')end;c.dismissed={};for j=1,n do c.dismissed[j]=dec()end
   elseif tag==15 then c.id=str(32);c.status=u(1);c.reason=str(396);c.event=str(36);if #c.id~=32 or c.id:find('[^0-9a-f]')or c.status>2 then error('social outcome')end
   elseif tag~=5 and tag~=6 then error("command") end
   commands[#commands+1]=c
  end
  if at~=#frame+1 then error("tail") end
  return {context=context,generation=generation,sequence=sequence,commands=commands}
 end)
 if ok then return result end
end
function M.new(authority,apply,pause)
 local self={context=nil,generation=nil,owner=nil,sequence=0,last_frame=nil}
 function self:receive(frame)
  local ok,a=pcall(authority)
  if not ok or type(a)~="table" or type(a.context)~="string" or #a.context~=16 or not integer(a.generation,1,4294967295) or type(a.owner)~="string" or #a.owner==0
   or type(a.now)~="number" or a.now~=a.now or a.now<0 or a.now==math.huge or type(a.until_time)~="number" or a.until_time~=a.until_time or a.until_time==math.huge then pause('authority');return false,'authority' end
  if a.context~=self.context or a.generation~=self.generation or a.owner~=self.owner then pause('context');self.context=a.context;self.generation=a.generation;self.owner=a.owner;self.sequence=0;self.last_frame=nil end
  if not a.playing or a.until_time<=a.now then pause('authority');return false,'authority' end
  local batch=M.decode(frame)
  if not batch or batch.context~=a.context or batch.generation~=a.generation then return false,'invalid' end
  -- Broker ACK is asynchronous. Ignore an already consumed frame without
  -- reapplying it or discarding a different, in-progress POI snapshot.
  if batch.sequence<self.sequence or batch.sequence==self.sequence and frame==self.last_frame then return false,'replay' end
  if batch.sequence==self.sequence then return false,'invalid' end
  -- Consume before callbacks. Failure cannot replay a partly applied batch.
  self.sequence=batch.sequence;self.last_frame=frame
  local applied,value=pcall(apply,batch.commands,a.now)
  if not applied or value~=true then pause('invalid');return false,'invalid' end
  return true
 end
 function self:close() pause('context');self.context=nil;self.generation=nil;self.owner=nil;self.sequence=0;self.last_frame=nil end
 return self
end
return M
