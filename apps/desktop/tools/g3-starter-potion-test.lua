-- Diagnosis fixture only: real entry/vanilla Lua, injected engine/native facts.
-- Does not model C++ inventory registration, physics, or reproduce real Noita.
local repo=assert(os.getenv('NT2_POTION_REPO'))
local sources=assert(os.getenv('NT2_POTION_SOURCES'))
local entry_path=os.getenv('NT2_POTION_ENTRY') or repo..'/mod/noita_together2/init-g3-c2.lua'
local diagnostic=os.getenv('NT2_POTION_DIAGNOSTIC')=='1'
local ffi=require('ffi');local original_load=ffi.load
local cases=0
local function scenario(startup,finish,original_inventory)
 dofile(repo..'/apps/desktop/tools/mod-protection-stubs.lua')
 local s=NT2_FIXTURE
 s.enabled[5]=original_inventory[1];s.enabled[6]=original_inventory[2];s.enabled[7]=original_inventory[3]
 s.components[11]={drinkable=true};s.components[12]={drinkable=false}
 local e=setmetatable({},{__index=_G});e._G=e
 local playing=false;local seed_calls,close_calls,materials=0,0,0
 local trace_lines={};e.print=function(line) if line:find('NT2_POTION|',1,true)==1 then trace_lines[#trace_lines+1]=line;assert(#line<=4096) end end
 local ids={[201]=true,[202]=true,[203]=true,[204]=true}
 local inventory={201,202,203,204}
 local get_component=EntityGetFirstComponentIncludingDisabled
 e.EntityGetFirstComponentIncludingDisabled=function(id,kind)
  if id==201 and kind=='ItemComponent' then return 11 end
  if id==204 and kind=='ItemComponent' then return 12 end
  if ids[id] or id==200 then return nil end
  return get_component(id,kind)
 end
 e.EntityGetIsAlive=function(id) return ids[id] or id==200 or EntityGetIsAlive(id) end
 e.EntityGetAllChildren=function(id) if id==200 then return inventory end;if id==99 then return s.effect and {200,999} or {200} end;return {} end
 e.EntityGetName=function(id) if os.getenv('NT2_POTION_DIAGNOSTIC_FAULT')=='1' then error('injected observation getter failure') end;return id==200 and 'inventory_quick' or '' end
 e.EntityHasTag=function(id,tag) if ids[id] then return (id==201 or id==204) and tag=='potion' end;return EntityHasTag(id,tag) end
 e.EntityKill=function(id) assert(id==999,'entry must not kill a starting inventory item');EntityKill(id) end
 e.EntityLoad=function() error('entry must not spawn replacement items') end
 e.GameGetAllInventoryItems=function() return inventory end
 e.EntityGetParent=function(id) return id==200 and 99 or ids[id] and 200 or 0 end
 e.EntityGetRootEntity=function() return 99 end
 e.GetMaterialInventoryMainMaterial=function() return 1 end
 s.components[6]={mInitialized=false}
 e.GameKillInventoryItem=function() error('entry must not remove starting items') end
 e.GameDestroyInventoryItems=e.GameKillInventoryItem
 e.SetWorldSeed=function(v) assert(v==424242);seed_calls=seed_calls+1 end
 e.StatsGetValue=function() return '424242' end
 e.ModGetActiveModIDs=function() return {'noita_together2'} end
 e.GameIsBetaBuild=function() return nil end
 e.SetRandomSeed=function() end
 e.StatsGlobalGetValue=function() return '0' end
 e.GameGetDateAndTimeLocal=function() return 2026,10,10 end
 e.Random=function() return 100 end
 e.GlobalsGetValue=function(_,fallback) return fallback end
 e.AddMaterialInventoryMaterial=function(id,material,amount) assert(id==201 and material=='water' and amount==1000);materials=materials+1 end
 e.EntityGetComponent=function() return nil end
 e.dofile_once=function(path)
  if path=='data/scripts/items/init_potion.lua' then local f=assert(loadfile(sources..'/init_potion.lua'));setfenv(f,e);return f() end
 end
 e.dofile=function(path)
  if path:find('/native_owner.lua',1,true) or path:find('/social_native.lua',1,true) then return {new=function() return {tick=function() end,close=function() end} end} end
  if path:find('mods/noita_together2/files/g3/',1,true)==1 then return {} end
  assert(path=='mods/noita_together2/files/inspection.lua')
  local f=assert(loadfile(repo..'/mod/noita_together2/files/inspection.lua'));setfenv(f,e);return f()
 end
 local native={}
 native.nt2_g3_startup_prepare=function(out,c) if startup==1 then out[0]=424242;ffi.fill(c,16,1) end;return startup end
 native.nt2_g3_prepare=function(out,c) out[0]=424242;ffi.fill(c,16,1);return 1 end
 native.nt2_g3_seed_accepted=function() return 1 end
 native.nt2_g3_world_loaded=function() return 1 end
 native.nt2_g3_report=function() return 1 end
 native.nt2_g3_status=function() return playing and 2 or 1 end
 native.nt2_g3_close=function() close_calls=close_calls+1;return 1 end
 for _,name in ipairs({'nt2_g3_heartbeat_diagnostic','nt2_g3_move','nt2_g3_peers','nt2_g3_social_lease','nt2_g3_social_next','nt2_g3_social_terminal'}) do native[name]=function() return 0 end end
 ffi.load=function() return native end
 local entry=assert(loadfile(entry_path));setfenv(entry,e);entry();e.OnModPreInit();ffi.load=original_load
 e.OnMagicNumbersAndWorldSeedInitialized();e.OnPlayerSpawned(99)
 -- Exercise the shipped potion Lua after spawn protection. This proves only
 -- Lua initialization independence, not the engine's inventory/physics order.
 local potion=assert(loadfile(sources..'/potion_starting.lua'));setfenv(potion,e);potion();e.init(201)
 assert(materials==1 and #inventory==4 and ids[201] and s.components[11].drinkable,'unchanged vanilla Lua fills existing potion')
 local function restored()
  assert(s.components[1].enabled and not s.components[2].wait_for_kill_flag_on_death and not s.poly and not s.effect and not s.components[3])
  for i,id in ipairs({5,6,7}) do assert(s.enabled[id]==original_inventory[i],'restore original component state') end
  assert(#inventory==4 and ids[201] and ids[202] and ids[203] and ids[204] and materials==1)
  assert(s.components[11].drinkable and not s.components[12].drinkable,'item originals unchanged')
 end
 if startup~=1 then assert(seed_calls==0);restored();if diagnostic then assert(#trace_lines==2 and trace_lines[1]:find('phase=spawn_before',1,true) and trace_lines[2]:find('phase=unbound_after_recovery',1,true)) end;cases=cases+1;return end
 local function waiting()
  assert(not s.components[1].enabled and s.components[2].wait_for_kill_flag_on_death and s.poly and s.effect and s.components[3])
  assert(not s.enabled[5] and not s.enabled[6] and not s.enabled[7])
  assert(#inventory==4 and ids[201] and materials==1)
 end
 waiting();s.frame=1;e.OnWorldPreUpdate();waiting()
 if diagnostic then e.OnWorldPostUpdate();s.frame=61;e.OnWorldPostUpdate();assert(#trace_lines==4 and trace_lines[3]:find('phase=first_post_update',1,true) and trace_lines[4]:find('phase=waiting_after_60_frames',1,true)) end
 if finish=='start-end-offline' then
  playing=true;s.frame=2;e.OnWorldPreUpdate();restored()
  playing=false;s.frame=3;e.OnWorldPreUpdate();waiting()
  s.button=true;e.OnWorldPostUpdate();restored();assert(close_calls==1)
  playing=true;s.frame=4;e.OnWorldPreUpdate();restored()
 elseif finish=='waiting-offline' then
  s.button=true;e.OnWorldPostUpdate();restored();assert(close_calls==1)
 elseif finish=='saved-continue' then
  -- Reset the script's locals as after reload, retain only serialized engine
  -- components/owned marker. Continue native refusal must restore the hold.
  local saved_seed_calls=seed_calls;startup=-2
  local continued=assert(loadfile(entry_path));setfenv(continued,e);continued();ffi.load=function() return native end
  e.OnModPreInit();ffi.load=original_load;e.OnPlayerSpawned(99);restored();assert(seed_calls==saved_seed_calls)
 else error('unknown case') end
 if diagnostic then assert(#trace_lines<=8);for i=1,100 do e.OnWorldPostUpdate() end;assert(#trace_lines<=8,'no repeated snapshots') end
 cases=cases+1
end
scenario(1,'start-end-offline',{true,true,true})
scenario(1,'start-end-offline',{false,false,false})
scenario(1,'waiting-offline',{true,true,false})
scenario(1,'saved-continue',{true,true,true})
scenario(-2,'none',{true,true,true})
ffi.load=original_load
print('PASS '..cases..' diagnosis scenarios: current startup/waiting/Start/End/offline/saved Continue component restoration; existing loadout identity retained; shipped vanilla potion Lua fills material. Engine/native facts injected; C++ inventory registration and real-game potion disappearance NOT reproduced.')
