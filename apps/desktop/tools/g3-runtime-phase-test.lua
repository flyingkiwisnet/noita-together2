-- Actual entry callbacks and inspection encoder; engine/native scheduling facts injected.
local root=assert(os.getenv('NT2_G3_MODULES'));local ffi=require('ffi')
local frame,status,ack=1,1,1;local reports={};local values={enabled=true,wait_for_kill_flag_on_death=false}
local native={};local ctx=string.rep('\1',16)
native.nt2_g3_prepare=function(out,c) out[0]=424242;ffi.copy(c,ctx,16);return 1 end
native.nt2_g3_report=function(c,body,size,sequence)
 local text=ffi.string(body,size);reports[#reports+1]={body=text,sequence=tonumber(sequence)}
 return tonumber(sequence)<=ack and 1 or 0
end
native.nt2_g3_status=function() return status end
native.nt2_g3_close=function() return 1 end
for _,n in ipairs({'nt2_g3_seed_accepted','nt2_g3_world_loaded'}) do native[n]=function() return 1 end end
local e={};setmetatable(e,{__index=_G});e._G=e
e.ModMagicNumbersFileAdd=function() end;e.dofile_once=function() end;e.SetWorldSeed=function() end
e.GameGetFrameNum=function() return frame end;e.StatsGetValue=function() return '424242' end
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
local entry=root:gsub('/files/g3$',os.getenv('NT2_G3_PHASE_PRIMARY')=='1' and '/init-g3.lua' or '/init-g3-social.lua')
local chunk=assert(loadfile(entry));setfenv(chunk,e);chunk()
local original=ffi.load;ffi.load=function() return native end;e.OnModPreInit();ffi.load=original
e.OnMagicNumbersAndWorldSeedInitialized();e.OnPlayerSpawned(7)
assert(reports[#reports].body:find('"protected":true',1,true))
status=2;e.OnWorldPreUpdate()
assert(values.enabled==true,'Valid native release restores controls')
local latest=reports[#reports]
print('FIRST_RELEASE_REPORT '..latest.body..' sequence='..latest.sequence)
assert(latest.body:find('"protected":false',1,true),'Playing observation must be queued in the actual release callback, before the300-frame rescan')
ack=latest.sequence;frame=2;e.OnWorldPreUpdate()
assert(values.enabled==true,'Async local observation ACK must not cause another protected transition')
status=0;frame=3;e.OnWorldPreUpdate();assert(values.enabled==false,'Actual native authority loss must hold immediately')
print('PASS actual Lua entry protected-to-playing observation and genuine authority-loss hold')
