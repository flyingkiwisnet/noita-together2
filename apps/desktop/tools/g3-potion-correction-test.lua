-- Diagnosis fixture only: real entry/vanilla Lua, injected engine/native facts.
-- Does not model C++ inventory registration, physics, or reproduce real Noita.
local repo=assert(os.getenv('NT2_POTION_REPO'))
local sources=assert(os.getenv('NT2_POTION_SOURCES'))
local entry_path=os.getenv('NT2_POTION_ENTRY') or repo..'/mod/noita_together2/init-g3-c2-potion-correction.lua'
local diagnostic=os.getenv('NT2_POTION_DIAGNOSTIC')=='1'
local seed_loot=entry_path:find('init-g3-seed-loot.lua',1,true)~=nil
local ffi=require('ffi');local original_load=ffi.load
local cases=0
local function scenario(startup,finish,original_inventory,preexisting,marker_failure)
 dofile(repo..'/apps/desktop/tools/mod-protection-stubs.lua')
 local s=NT2_FIXTURE
 if preexisting then s.components[1].enabled=false;s.poly=true;s.components[2].wait_for_kill_flag_on_death=true end
 s.enabled[5]=original_inventory[1];s.enabled[6]=original_inventory[2];s.enabled[7]=original_inventory[3]
 s.components[11]={drinkable=true};s.components[12]={drinkable=false}
 local e=setmetatable({},{__index=_G});e._G=e
 s.enabled[11]=false;s.enabled[12]=true
 s.components[29]={name='foreign_item_marker',value_string='0'}
 local markers={[201]={29}};local next_marker=30
 local marker_attempts=0;local failure_mode=marker_failure
 e.EntityAddComponent2=function(id,kind,values)
  if id==99 then return EntityAddComponent2(id,kind,values) end
  marker_attempts=marker_attempts+1
  if failure_mode=='zero' then return 0 end
  if failure_mode=='nil' then return nil end
  if failure_mode=='throw' then error('injected potion marker failure') end
  assert(id==201 or id==204);next_marker=next_marker+1;s.components[next_marker]=values;markers[id]=markers[id] or {};table.insert(markers[id],next_marker);return next_marker
 end
 e.EntityGetComponentIncludingDisabled=function(id,kind)
  if id==99 then return EntityGetComponentIncludingDisabled(id,kind) end
  local result={};for _,v in ipairs(markers[id] or {}) do if s.components[v] then result[#result+1]=v end end;return result
 end
 e.EntityRemoveComponent=function(id,c) s.components[c]=nil end
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
 if seed_loot then
  local globals={};local ready=false;local bootstrap='saved'
  e.GlobalsGetValue=function(key,fallback) assert(ready,'WorldState does not exist');return globals[key] or fallback end
  e.GlobalsSetValue=function(key,value) assert(ready,'WorldState does not exist');globals[key]=value end;e.ModLuaFileAppend=function() end
  e.GameGetWorldStateEntity=function() return ready and 123 or 0 end
  e.ModTextFileSetContent=function(_,value) bootstrap=value end;e.ModTextFileGetContent=function() return bootstrap end
  e.loot_world_ready=function() ready=true end
 end
 e.AddMaterialInventoryMaterial=function(id,material,amount) assert(id==201 and material=='water' and amount==1000);materials=materials+1 end
 e.EntityGetComponent=function() return nil end
 e.dofile_once=function(path)
  if path=='data/scripts/items/init_potion.lua' then local f=assert(loadfile(sources..'/init_potion.lua'));setfenv(f,e);return f() end
 end
 e.dofile=function(path)
  if seed_loot and path:find('/files/rejoin/local_world.lua',1,true) then local f=assert(loadfile(repo..'/mod/noita_together2/files/rejoin/local_world.lua'));setfenv(f,e);return f() end
  if seed_loot and path:find('/files/loot/',1,true) then local f=assert(loadfile(repo..'/mod/noita_together2/files/loot/'..path:match('[^/]+$')));setfenv(f,e);return f() end
  if path:find('/native_owner.lua',1,true) or path:find('/social_native.lua',1,true) then return {new=function() return {tick=function() end,close=function() end} end} end
  if path:find('mods/noita_together2/files/g3/',1,true)==1 then return {} end
  assert(path=='mods/noita_together2/files/inspection.lua')
  local f=assert(loadfile(repo..'/mod/noita_together2/files/inspection.lua'));setfenv(f,e);return f()
 end
 local native={}
 native.nt2_g3_startup_prepare=function(out,c) if startup==1 then out[0]=424242;ffi.fill(c,16,1) end;return startup end
 native.nt2_g3_prepare=function(out,c) out[0]=424242;ffi.fill(c,16,1);return 1 end
 native.nt2_g3_seed_accepted=function() return 1 end
 if seed_loot then
  native.nt2_g3_world_policy=function() return 5 end
  native.nt2_g3_seed_known=function(_,value,size) assert(value=='424242' and size==6);return 1 end
 end
 native.nt2_g3_world_loaded=function() return 1 end
 native.nt2_g3_report=function() return 1 end
 native.nt2_g3_status=function() return playing and 2 or 1 end
 native.nt2_g3_close=function() close_calls=close_calls+1;return 1 end
 for _,name in ipairs({'association','saved_lookup','offline_choice','resume_decision','resume'}) do native['nt2_g3_'..name]=function() return 0 end end
 for _,name in ipairs({'nt2_g3_heartbeat_diagnostic','nt2_g3_move','nt2_g3_peers','nt2_g3_social_lease','nt2_g3_social_next','nt2_g3_social_terminal'}) do native[name]=function() return 0 end end
 ffi.load=function() return native end
 local entry=assert(loadfile(entry_path));setfenv(entry,e);entry();e.OnModPreInit();ffi.load=original_load
 if seed_loot then e.loot_world_ready() end
 e.OnMagicNumbersAndWorldSeedInitialized();e.OnPlayerSpawned(99)
 -- Exercise the shipped potion Lua after spawn protection. This proves only
 -- Lua initialization independence, not the engine's inventory/physics order.
 local potion=assert(loadfile(sources..'/potion_starting.lua'));setfenv(potion,e);potion();e.init(201)
 assert(materials==1 and #inventory==4 and ids[201] and (startup~=1 or marker_failure or not s.components[11].drinkable),'unchanged vanilla Lua fills existing potion; owned drinking gate')
 local function restored()
  assert(s.components[1].enabled==not preexisting and s.components[2].wait_for_kill_flag_on_death==not not preexisting and s.poly==not not preexisting and not s.effect and not s.components[3])
  for i,id in ipairs({5,6,7}) do assert(s.enabled[id]==original_inventory[i],'restore original component state') end
  assert(#inventory==4 and ids[201] and ids[202] and ids[203] and ids[204] and materials==1)
  assert(s.components[11].drinkable and not s.components[12].drinkable,'item originals unchanged')
  assert(not s.enabled[11] and s.enabled[12] and s.components[29],'preexisting item enable state and foreign marker unchanged')
 end
 if startup~=1 then
  assert(seed_calls==0)
  if seed_loot then assert(not s.components[1].enabled,'unconfirmed startup stays held');s.button=true;e.OnWorldPostUpdate();s.button=false end
  restored();if diagnostic then assert(#trace_lines==(seed_loot and 4 or 2) and trace_lines[1]:find('phase=spawn_before',1,true) and trace_lines[2]:find('phase=unbound_after_recovery',1,true)) end;cases=cases+1;return
 end
 local function waiting()
  assert(not s.components[1].enabled and s.components[2].wait_for_kill_flag_on_death and s.poly and s.effect and s.components[3])
  for i,id in ipairs({5,6,7}) do assert(s.enabled[id]==original_inventory[i],'inventory must keep original enable state during waiting') end
  assert(not s.components[11].drinkable and not s.components[12].drinkable,'no potion drinking during hold')
  assert(#inventory==4 and ids[201] and materials==1)
 end
 if marker_failure then
  assert(marker_attempts==1 and s.components[11].drinkable,'failed spawn marker must not change flag')
  s.frame=1;e.OnWorldPreUpdate()
  assert(marker_attempts==2 and s.components[11].drinkable,'next held frame must retry; no cached unready original')
  assert(not s.components[1].enabled and s.components[2].wait_for_kill_flag_on_death,'failure retains controls protection')
  for i,id in ipairs({5,6,7}) do assert(s.enabled[id]==original_inventory[i]) end
  if finish=='retry-saved' or finish=='retry-offline' then
   failure_mode=nil;s.frame=2;e.OnWorldPreUpdate()
   assert(marker_attempts==3 and not s.components[11].drinkable,'successful retry gates flag')
   local recovery=false
   for _,m in ipairs(markers[201]) do if s.components[m] and s.components[m].name=='nt2_g3_drinkable_recovery_v1' then recovery=s.components[m].value_string=='1' end end
   assert(recovery,'changed flag has valid original in saved marker')
   for frame=3,30 do s.frame=frame;e.OnWorldPreUpdate() end
   assert(marker_attempts==3,'successful original is cached once')
  end
  if finish=='failure-saved' or finish=='retry-saved' then
   local original_seed_calls=seed_calls;startup=-2
   local continued=assert(loadfile(entry_path));setfenv(continued,e);continued();ffi.load=function() return native end
   e.OnModPreInit();ffi.load=original_load;e.OnPlayerSpawned(99)
   if seed_loot then assert(not s.components[1].enabled);s.button=true;e.OnWorldPostUpdate();s.button=false end
   restored();assert(seed_calls==original_seed_calls)
  else
   s.button=true;e.OnWorldPostUpdate();restored()
  end
  for _,m in ipairs(markers[201]) do assert(not s.components[m] or s.components[m].name=='foreign_item_marker','owned marker removed after recovery') end
  cases=cases+1;return
 end
 waiting()
 -- Engine lifecycle seam: invoke one initialization callback only if its
 -- Inventory2 component is enabled. This does NOT simulate C++ physics.
 local initialized_calls=0
 for frame=1,3000 do
  s.frame=frame;e.OnWorldPreUpdate()
  if s.enabled[6] and not s.components[6].mInitialized then s.components[6].mInitialized=true;initialized_calls=initialized_calls+1 end
  waiting()
 end
 assert(initialized_calls==(original_inventory[2] and 1 or 0),'normal inventory initialization must be reachable, disabled originals stay disabled')
 s.frame=1;e.OnWorldPreUpdate();waiting()
 if diagnostic then e.OnWorldPostUpdate();s.frame=3061;e.OnWorldPostUpdate();assert(#trace_lines==4 and trace_lines[3]:find('phase=first_post_update',1,true) and trace_lines[4]:find('phase=waiting_after_60_frames',1,true)) end
 if finish=='start-end-offline' then
  playing=true;s.frame=2;e.OnWorldPreUpdate();restored()
  playing=false;s.frame=3;e.OnWorldPreUpdate();waiting()
  s.button=true;e.OnWorldPostUpdate();restored();assert(close_calls==1)
  playing=true;s.frame=4;e.OnWorldPreUpdate();restored()
 elseif finish=='waiting-offline' then
  s.button=true;e.OnWorldPostUpdate();restored();assert(close_calls==1)
 elseif finish=='saved-continue' or finish=='legacy-saved-continue' then
  if finish=='legacy-saved-continue' then
   -- A held save made by the accepted old entry must still restore its
   -- original inventory flags, even though the candidate never disables them.
   s.components[3].value_string='101110'
   s.enabled[5]=false;s.enabled[6]=false;s.enabled[7]=false
  end
  -- Reset the script's locals as after reload, retain only serialized engine
  -- components/owned marker. Continue native refusal must restore the hold.
  local saved_seed_calls=seed_calls;startup=-2
  local continued=assert(loadfile(entry_path));setfenv(continued,e);continued();ffi.load=function() return native end
  e.OnModPreInit();ffi.load=original_load;e.OnPlayerSpawned(99)
  if seed_loot then assert(not s.components[1].enabled);s.button=true;e.OnWorldPostUpdate();s.button=false end
  restored();assert(seed_calls==saved_seed_calls)
 else error('unknown case') end
 if diagnostic then assert(#trace_lines<=8);for i=1,100 do e.OnWorldPostUpdate() end;assert(#trace_lines<=8,'no repeated snapshots') end
 cases=cases+1
end
scenario(1,'start-end-offline',{true,true,true})
scenario(1,'start-end-offline',{false,false,false})
scenario(1,'waiting-offline',{true,true,false})
scenario(1,'saved-continue',{true,true,true})
scenario(-2,'none',{true,true,true})
scenario(1,'start-end-offline',{true,true,true},true)
scenario(1,'saved-continue',{false,false,false},true)
scenario(1,'legacy-saved-continue',{true,true,false})
for _,failure in ipairs({'zero','nil','throw'}) do
 for _,finish in ipairs({'failure-saved','failure-offline','retry-saved','retry-offline'}) do
  scenario(1,finish,{true,true,false},false,failure)
 end
end
ffi.load=original_load
print('PASS '..cases..' correction scenarios with3000-frame waiting/init callback seam: current startup/waiting/Start/End/offline/saved Continue component restoration; existing loadout identity retained; shipped vanilla potion Lua fills material. Engine/native facts injected; C++ inventory registration and real-game potion disappearance NOT reproduced.')
