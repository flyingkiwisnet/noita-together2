-- Actual entry bootstrap callbacks; private DLL/pipe, injected engine facts only.
io.stdout:setvbuf('no')
local ffi=require('ffi');local root=assert(os.getenv('NT2_G3_MODULES'))
ffi.cdef[[void __stdcall Sleep(unsigned long);]]
local native=ffi.load(assert(os.getenv('NT2_INITIAL_DLL')))
local seed;local values={enabled=true,wait_for_kill_flag_on_death=false}
local outcomes={prepare=false,accepted=false,loaded=false}
local e={};setmetatable(e,{__index=_G});e._G=e
e.print=function(text)
 print(text)
 if text:find('event=prepare|',1,true) then outcomes.prepare=text:find('detail=1',1,true)~=nil end
 if text:find('event=seed_accepted|',1,true) then outcomes.accepted=text:find('detail=true;',1,true)~=nil end
 if text:find('event=world_loaded|',1,true) then outcomes.loaded=text:find('detail=true;',1,true)~=nil end
end
e.ModMagicNumbersFileAdd=function() end;e.dofile_once=function() end
e.SetWorldSeed=function(value) seed=value end;e.GameGetFrameNum=function() return 1 end
e.StatsGetValue=function() return os.getenv('NT2_INITIAL_WRONG_SEED')=='1' and '424243' or tostring(seed or 0) end
e.ModGetActiveModIDs=function() return {'noita_together2'} end;e.GameIsBetaBuild=function() return false end
e.EntityGetIsAlive=function() return true end;e.EntityGetTransform=function() return 0,0 end
e.EntityGetFirstComponentIncludingDisabled=function(_,kind) if kind=='ControlsComponent' then return 1 elseif kind=='DamageModelComponent' then return 2 elseif kind=='GameEffectComponent' then return 3 end end
e.ComponentGetValue2=function(_,key) return values[key] end;e.ComponentSetValue2=function(_,key,value) values[key]=value end
e.EntityGetComponentIncludingDisabled=function() return {} end;e.EntityHasTag=function() return true end
e.EntityAddComponent2=function() return 4 end;e.LoadGameEffectEntityTo=function() return 5 end
for _,n in ipairs({'EntityRemoveComponent','EntityKill','EntityRemoveTag','EntityAddTag','EntitySetTransform','EntitySetComponentIsEnabled'}) do e[n]=function() end end
e.dofile=function(path)
 if path:find('/native_owner.lua',1,true) or path:find('/social_native.lua',1,true) then return {new=function() return {tick=function() end,close=function() end} end} end
 local target=path:find('/inspection.lua',1,true) and root:gsub('/files/g3$','/files/inspection.lua') or root..'/'..assert(path:match('/files/g3/(.+)$'))
 local chunk=assert(loadfile(target));setfenv(chunk,e);return chunk()
end
local entry=root:gsub('/files/g3$',os.getenv('NT2_INITIAL_PRIMARY')=='1' and '/init-g3.lua' or '/init-g3-social.lua')
local chunk=assert(loadfile(entry));setfenv(chunk,e);chunk()
local original=ffi.load;ffi.load=function() return native end;e.OnModPreInit();ffi.load=original
-- Wait for the owned test's generation invalidation after op1, if requested.
local gate=os.getenv('NT2_INITIAL_GATE')
if gate then local f=assert(io.open(gate..'/prepared','w'));f:write('callback complete');f:close()
 local C=ffi.C
 for i=1,200 do local ready=io.open(gate..'/continue','r');if ready then ready:close();break end;C.Sleep(5) end
end
local spacing=tonumber(os.getenv('NT2_INITIAL_SPACING')) or 0
ffi.C.Sleep(spacing);e.OnMagicNumbersAndWorldSeedInitialized();ffi.C.Sleep(spacing);e.OnPlayerSpawned(7)
-- Calling pre-init again is idempotent; it must never mint another nonce/world.
e.OnModPreInit()
print('INITIAL_CALLBACK_RESULT prepare='..tostring(outcomes.prepare)..';accepted='..tostring(outcomes.accepted)..';loaded='..tostring(outcomes.loaded)..';held='..tostring(not values.enabled))
if os.getenv('NT2_INITIAL_HEARTBEAT')=='1' then
 if os.getenv('NT2_INITIAL_HEARTBEAT_GUARD')=='1' then
  local f=assert(io.open(assert(os.getenv('NT2_INITIAL_FINAL_GATE'))..'/heartbeat-ready','w'));f:write('loaded callback finished');f:close()
  for i=1,600 do local ready=io.open(os.getenv('NT2_INITIAL_FINAL_GATE')..'/heartbeat-continue','r');if ready then ready:close();break end;ffi.C.Sleep(5) end
 end
 local function find(fn,seen)
  seen=seen or {};if seen[fn] then return end;seen[fn]=true
  for i=1,60 do local name,value=debug.getupvalue(fn,i);if not name then break end
   if name=='context' then return value elseif type(value)=='function' then local c=find(value,seen);if c then return c end end
  end
 end
 ffi.C.Sleep(2250)
 local context=assert(find(e.OnWorldPreUpdate))
 print('HEARTBEAT_RETAINED status='..tostring(native.nt2_g3_status(context)))
 print('HEARTBEAT_DIAGNOSTIC reason='..tostring(native.nt2_g3_heartbeat_diagnostic(context)))
end
local final=assert(os.getenv('NT2_INITIAL_FINAL_GATE'))
local done=assert(io.open(final..'/done','w'));done:write('callbacks complete');done:close()
for i=1,1000 do local ready=io.open(final..'/finish','r');if ready then ready:close();break end;ffi.C.Sleep(5) end
-- Exit without invoking another prepare. No retry/new world on an ambiguous reply.
