-- Actual C2 callback code; native/engine facts injected. No Noita execution.
local repo=assert(os.getenv('NT2_C2_REPO'));local ffi=require('ffi');local original_load=ffi.load
local function scenario(startup,actual)
 dofile(repo..'/apps/desktop/tools/mod-protection-stubs.lua')
 local e={};setmetatable(e,{__index=_G});e._G=e
 local seeded,generated,seed_calls,loaded_calls,report_calls,closed,playing=false,false,0,0,0,0,false
 e.SetWorldSeed=function(seed) assert(not generated,'seed must precede generation');assert(seed==424242);seeded=true end
 e.StatsGetValue=function() return tostring(actual) end;e.ModGetActiveModIDs=function() return {'noita_together2'} end;e.GameIsBetaBuild=function() return nil end
 e.dofile=function(path)
  if path:find('/native_owner.lua',1,true) or path:find('/social_native.lua',1,true) then return {new=function() return {tick=function() end,close=function() end} end} end
  local fn=assert(loadfile(repo..'/mod/noita_together2/files/inspection.lua'));setfenv(fn,e);return fn()
 end
 local native={}
 native.nt2_g3_startup_prepare=function(out,c) if startup==1 then out[0]=424242;ffi.fill(c,16,1) end;return startup end
 native.nt2_g3_prepare=function(out,c) assert(startup==1);out[0]=424242;ffi.fill(c,16,1);return 1 end
 native.nt2_g3_seed_accepted=function(_,seed) assert(seed==424242);seed_calls=seed_calls+1;return seed_calls>=3 and 1 or 0 end
 native.nt2_g3_world_loaded=function() assert(seed_calls>=3);loaded_calls=loaded_calls+1;return loaded_calls>=3 and 1 or 0 end
 native.nt2_g3_report=function(_,body,size,sequence) assert(loaded_calls>=3 or not playing);report_calls=report_calls+1;return report_calls>=3 and 1 or 0 end
 native.nt2_g3_status=function() return playing and 2 or 1 end
 native.nt2_g3_close=function() closed=closed+1;return 1 end
 for _,name in ipairs({'nt2_g3_heartbeat_diagnostic','nt2_g3_move','nt2_g3_peers','nt2_g3_social_lease','nt2_g3_social_next','nt2_g3_social_terminal'}) do native[name]=function() return 0 end end
 local images={};ffi.load=function(path) assert(path=='mods/noita_together2/files/nt2_g3_social.dll' or path=='mods/noita_together2/files/nt2_g3_transport.dll');images[path]=true;return native end
 local entry=assert(loadfile(repo..'/mod/noita_together2/init-g3-c2.lua'));setfenv(entry,e);entry();e.OnModPreInit();ffi.load=original_load
 assert(seeded==(startup==1));assert(images['mods/noita_together2/files/nt2_g3_social.dll'] and images['mods/noita_together2/files/nt2_g3_transport.dll']);generated=true;e.OnMagicNumbersAndWorldSeedInitialized();e.OnPlayerSpawned(99)
 for i=1,12 do NT2_FIXTURE.frame=i;e.OnWorldPreUpdate() end
 if startup~=1 then assert(seed_calls==0 and loaded_calls==0 and not seeded,'Continue/refusal must preserve existing seed');return end
 if actual~=424242 then assert(seed_calls==0 and loaded_calls==0 and not NT2_FIXTURE.components[1].enabled,'wrong seed remains protected');return end
 assert(seed_calls==3 and loaded_calls==3,'pending controls must be polled to completion')
 assert(not NT2_FIXTURE.components[1].enabled,'inspection without verified Start cannot release')
 playing=true;e.OnWorldPreUpdate();assert(NT2_FIXTURE.components[1].enabled,'verified playing and completed inspection release')
 playing=false;e.OnWorldPreUpdate();assert(not NT2_FIXTURE.components[1].enabled,'loss of authority protects');assert(closed==0)
end
scenario(1,424242);scenario(1,999);scenario(-2,777);scenario(0,777)
ffi.load=original_load
print('PASS C2 actual Lua startup-before-generation, pending seed/loaded polling, inspection, wrong seed, Continue/refusal and health protection; engine/native facts injected')
