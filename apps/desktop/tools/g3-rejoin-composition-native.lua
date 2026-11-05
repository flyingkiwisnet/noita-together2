-- Actual C2 entry + gameplay callbacks, private wrappers/broker and OS pipes.
-- Only engine APIs are injected. Production DLLs and Noita are never executed.
local repo=assert(os.getenv('NT2_C3_REPO'))
local dir=assert(os.getenv('NT2_C3_DIR'))
local ffi=require('ffi');ffi.cdef[[void __stdcall Sleep(unsigned long);unsigned long __stdcall GetTickCount();]]
io.stdout:setvbuf('no')
dofile(repo..'/apps/desktop/tools/mod-protection-stubs.lua')
local state=NT2_FIXTURE
local registrations={}
ModLuaFileAppend=function(base,hook) registrations[#registrations+1]={base,hook};assert(#registrations<=12) end
local savedGlobals={}
local savedMode=os.getenv('NT2_REJOIN_SAVED')=='1'
if savedMode then
 local function read(name) local f=assert(io.open(dir..'/'..name,'rb'));local s=f:read('*a');f:close();return s end
 savedGlobals.nt2_rejoin_local_world_v1=read('xml-restored-association.txt')
 savedGlobals.nt2_rejoin_local_world_v1_seed=read('xml-restored-seed.txt')
 savedGlobals.nt2_rejoin_local_world_v1_loot=read('xml-restored-loot.txt')
 savedGlobals.nt2_rejoin_local_world_v1_offline='0'
 savedGlobals.nt2_seed_loot_policy_v1=savedGlobals.nt2_rejoin_local_world_v1_loot
end
local loot_world_ready=false;local loot_bootstrap='saved';local loot_mod_init=true
GameGetWorldStateEntity=function() return loot_world_ready and 123 or 0 end
GlobalsGetValue=function(k,fallback) assert(loot_world_ready,'WorldState does not exist');return savedGlobals[k] or fallback end
GlobalsSetValue=function(k,v) assert(loot_world_ready,'WorldState does not exist');savedGlobals[k]=v end
ModTextFileSetContent=function(path,value) assert(loot_mod_init and path=='mods/noita_together2/files/loot/run-policy.txt');loot_bootstrap=value end
ModTextFileGetContent=function(path) assert(path=='mods/noita_together2/files/loot/run-policy.txt');return loot_bootstrap end
-- Item-local owned markers must not overwrite the player recovery marker.
local original_add=EntityAddComponent2;local original_components=EntityGetComponentIncludingDisabled
local original_remove=EntityRemoveComponent;local potion_markers={};local marker_id=30
EntityAddComponent2=function(id,kind,values)
 if id~=201 then return original_add(id,kind,values) end
 assert(kind=='VariableStorageComponent');marker_id=marker_id+1;state.components[marker_id]=values;potion_markers[#potion_markers+1]=marker_id;return marker_id
end
EntityGetComponentIncludingDisabled=function(id,kind)
 if id~=201 then return original_components(id,kind) end
 local out={};for _,c in ipairs(potion_markers) do if state.components[c] then out[#out+1]=c end end;return out
end
EntityRemoveComponent=function(id,c) if id==201 then state.components[c]=nil else original_remove(id,c) end end
-- Existing vanilla loadout identities are injected, never created by the entry.
local starter={[200]=true,[201]=true,[202]=true,[203]=true}
state.components[6]={mInitialized=false}
state.components[11]={drinkable=true,is_pickable=true,has_been_picked_by_player=false,inventory_slot=2}
for _,id in ipairs({11,12,13,14,15}) do state.enabled[id]=false end
state.components[12]={};state.components[13]={};state.components[14]={};state.components[15]={}
local entities,next_entity={},1000
local seed,generated,ghosts,held=false,false,0,0
local function mark(name,value) local f=assert(io.open(dir..'/'..name,'wb'));f:write(value or 'fixture');f:close() end
local function exists(name) local f=io.open(dir..'/'..name,'rb');if f then f:close();return true end end
local original_alive=EntityGetIsAlive;local original_kill=EntityKill
EntityGetIsAlive=function(id) return starter[id] or entities[id]~=nil or original_alive(id) end
EntityKill=function(id) if entities[id] then entities[id]=nil else original_kill(id) end end
EntityLoad=function(path,x,y) assert(path=='mods/noita_together2/files/g3/ghost.xml' or path=='mods/noita_together2/files/g3/poi.xml');next_entity=next_entity+1;entities[next_entity]={x=x,y=y};if path:find('/ghost.xml',1,true) then ghosts=ghosts+1;mark('ghost',tostring(x)) end;return next_entity end
EntityGetTransform=function(id) if entities[id] then return entities[id].x,entities[id].y,0 end;return tonumber(os.getenv('NT2_C3_X')),2,0 end
local original_children=EntityGetAllChildren;local original_tag=EntityHasTag
EntityGetAllChildren=function(id) if id==200 then return {201,202,203} end;if id==99 then local result={200};for _,child in ipairs(original_children(id) or {}) do result[#result+1]=child end;return result end;return original_children(id) end
EntityGetName=function(id) return id==200 and 'inventory_quick' or '' end
EntityGetParent=function(id) return id==200 and 99 or starter[id] and 200 or 0 end
EntityGetRootEntity=function(id) return starter[id] and 99 or id end
EntityHasTag=function(id,tag) if starter[id] then return id==201 and (tag=='potion' or tag=='item_pickup' or tag=='item_physics') or (id==202 or id==203) and tag=='wand' end;return original_tag(id,tag) end
GetMaterialInventoryMainMaterial=function(id) return id==201 and 1 or 0 end
EntitySetTransform=function(id,x,y) if entities[id] then entities[id].x=x;entities[id].y=y end end
local original_component=EntityGetFirstComponentIncludingDisabled
EntityGetFirstComponentIncludingDisabled=function(id,kind) if id==201 then return ({ItemComponent=11,MaterialInventoryComponent=12,PhysicsBodyComponent=13,DamageModelComponent=14,PotionComponent=15})[kind] end;if starter[id] or entities[id] or kind=='VelocityComponent' then return nil end;return original_component(id,kind) end
GameGetRealWorldTimeSinceStarted=function() return tonumber(ffi.C.GetTickCount())/1000 end
local policy=os.getenv('NT2_U2_POLICY') or 'shared-random'
local shared=tonumber(os.getenv('NT2_U2_SEED') or '424242')
local actual=policy=='individual-random' and (os.getenv('NT2_U2_ACTUAL') or '777') or tostring(shared)
SetWorldSeed=function(value) assert(not generated,'seed must precede generation');assert(policy~='individual-random' and value==shared);seed=true;mark('seed-before-generation') end
StatsGetValue=function() return os.getenv('NT2_C3_WRONG_SEED')=='1' and '999' or actual end
ModGetActiveModIDs=function() return {'noita_together2'} end
GameIsBetaBuild=function() return nil end
GuiDestroy=function() end;GuiImage=function() end
local raw_dofile=dofile
dofile=function(path) local relative=assert(path:match('^mods/noita_together2/(.+)$'));return raw_dofile(dir..'/verified-files/mods/noita_together2/'..relative) end
local original_load=ffi.load
-- These DLLs were just compiled for this disposable process. Load the private
-- broker before the fixed startup budget, keeping fresh build/loader costs out
-- of the lifecycle transport check. This does not issue any broker operation.
local private_broker_image,private_warm_image
if not savedMode then
private_broker_image=original_load(assert(os.getenv('NT2_C3_DLL_DIR'))..'/nt2_g3_ipc.dll')
-- Warm only DLL/worker creation, without a world, pipe operation or grant.
-- This fixture excludes cold loader scheduling from the unchanged250ms startup.
ffi.cdef[[int nt2_g3_resume(const uint8_t*,const uint8_t*,uint32_t,uint8_t*);]]
private_warm_image=original_load(assert(os.getenv('NT2_C3_DLL_DIR'))..'/nt2_g3_transport.dll')
assert(private_warm_image.nt2_g3_resume(ffi.new('uint8_t[16]'),'0',1,ffi.new('uint8_t[16]'))==0)
ffi.C.Sleep(100)
end
if os.getenv('NT2_REJOIN_MALFORMED_PROBE')=='1' then
 ffi.cdef[[int nt2_g3_saved_lookup(const uint8_t*,uint32_t);]]
 local bad='malformed current association';local deadline=tonumber(ffi.C.GetTickCount())+1500
 while private_warm_image.nt2_g3_saved_lookup(bad,#bad)~=0 do
  assert(tonumber(ffi.C.GetTickCount())<deadline,'malformed metadata did not become ordinary offline');ffi.C.Sleep(10)
 end
 mark('malformed-metadata-refused')
end
local pinned_images={}
local function load_image(path)
 local image=original_load(path)
 if os.getenv('NT2_C3_DIAGNOSTIC_PIN')=='1' then pinned_images[#pinned_images+1]=image end
 return image
end
ffi.load=function(path)
 if path=='mods/noita_together2/files/nt2_g3_transport.dll' then
  local image=load_image(assert(os.getenv('NT2_C3_DLL_DIR'))..'/nt2_g3_transport.dll')
  return setmetatable({}, {__index=function(_,key)
   if key~='nt2_g3_startup_prepare' then return image[key] end
   return function(result,context)
    local status=image.nt2_g3_startup_prepare(result,context)
    print('PRIVATE_STARTUP|status='..tonumber(status)..'|reason='..tonumber(image.nt2_g3_heartbeat_diagnostic(context)))
    return status
   end
  end})
 end
 if path=='mods/noita_together2/files/nt2_g3_social.dll' then return load_image(assert(os.getenv('NT2_C3_DLL_DIR'))..'/nt2_g3_social.dll') end
 return original_load(path)
end
-- Read-only private pipe identity proof, before original engine startup; no grant request.
if os.getenv('NT2_U2_PIPE_NAME') then
 ffi.cdef[[void* __stdcall CreateFileA(const char*,unsigned long,unsigned long,void*,unsigned long,unsigned long,void*);int __stdcall GetNamedPipeServerProcessId(void*,unsigned long*);void* __stdcall OpenProcess(unsigned long,int,unsigned long);int __stdcall QueryFullProcessImageNameA(void*,unsigned long,char*,unsigned long*);int __stdcall CloseHandle(void*);unsigned long __stdcall GetLastError();]]
 local pipe=ffi.C.CreateFileA(os.getenv('NT2_U2_PIPE_NAME'),0xC0000000,0,nil,3,0x40000000,nil)
 local pid=ffi.new('unsigned long[1]');local ok=ffi.C.GetNamedPipeServerProcessId(pipe,pid);local error=ffi.C.GetLastError()
 local image=ffi.new('char[1024]');local count=ffi.new('unsigned long[1]',1024);local process=ffi.C.OpenProcess(0x1000,0,pid[0]);local known=ffi.C.QueryFullProcessImageNameA(process,0,image,count)
 print('PRIVATE_APP_PROOF|ok='..tonumber(ok)..'|error='..tonumber(error)..'|pid='..tonumber(pid[0])..'|known='..tonumber(known)..'|path='..(known~=0 and ffi.string(image,count[0]) or '?'))
 ffi.C.CloseHandle(process);ffi.C.CloseHandle(pipe);ffi.C.Sleep(50)
end
-- Core verified the exact mapped candidate in this owned fixture before launch.
raw_dofile(os.getenv('NT2_C3_ENTRY_PATH') or dir..'/verified-files/mods/noita_together2/init.lua')
assert(#registrations==12);if not savedMode then OnModPreInit();ffi.load=original_load end
ffi.cdef[[void* __stdcall GetModuleHandleA(const char*);unsigned long __stdcall GetModuleFileNameA(void*,char*,unsigned long);]]
for _,name in ipairs({'nt2_g3_transport.dll','nt2_g3_ipc.dll'}) do
 local module=ffi.C.GetModuleHandleA(name);local path=ffi.new('char[2048]');local n=ffi.C.GetModuleFileNameA(module,path,2048)
 print('PRIVATE_MODULE|name='..name..'|loaded='..tostring(module~=nil)..'|path='..ffi.string(path,n))
end
if os.getenv('NT2_C3_DIAGNOSTIC_PIN')=='1' then mark('diagnostic-pinned-images') end
if os.getenv('NT2_C3_GC_PROBE')=='1' then
 assert(os.getenv('NT2_C3_DIAGNOSTIC_PIN')~='1','GC proof must rely on the actual entry')
 collectgarbage('collect');collectgarbage('collect')
 ffi.cdef[[void* __stdcall GetModuleHandleA(const char*);]]
 for _,name in ipairs({'nt2_g3_transport.dll','nt2_g3_social.dll','nt2_g3_ipc.dll'}) do assert(ffi.C.GetModuleHandleA(name)~=nil,'wrapper image collected: '..name) end
 mark('gc-images-retained')
end
if os.getenv('NT2_C3_REFUSE')=='1' then assert(not seed,'Continue/refusal must not replace seed');mark('refused');return end
if savedMode then
 assert(not seed,'saved Continue cannot request seed')
 loot_mod_init=false;generated=true
 -- Actual Continue lifecycle: no synthetic pre-init/startup or broker warmup.
 if OnWorldInitialized then OnWorldInitialized() end
 OnPlayerSpawned(99)
 for i=1,3 do state.frame=i;OnWorldPreUpdate();OnWorldPostUpdate();ffi.C.Sleep(10) end
 assert(not state.components[1].enabled,'Continue readiness gap was not protected')
 loot_world_ready=true;ffi.load=original_load
 mark('continue-no-preinit-cold-broker-late-world')
else
assert(policy=='individual-random' and not seed or policy~='individual-random' and seed,'actual private policy binding refused');mark('generation-policy-bound',policy)
local loot_enabled=policy~='individual-random' and os.getenv('NT2_U2_LOOT_OFF')~='1'
local recorded=policy=='individual-random' and 'individual:0' or ('shared:'..(loot_enabled and '1' or '0')..':'..string.format('%.0f',shared))
assert(loot_bootstrap=='new:'..recorded and savedGlobals.nt2_seed_loot_policy_v1==nil,'actual broker loot policy staged without pre-world Globals')
mark('loot-policy-bound',recorded)
-- Separate generation context loads the exact verified hook and helper.
local function prove_hook_context(world_ready)
 local observed={};local random_calls=0
 local env=setmetatable({}, {__index=_G});env._G=env
 env.dofile=function(path)
  local relative=assert(path:match('^mods/noita_together2/(.+)$'))
  local chunk=assert(loadfile(dir..'/verified-files/mods/noita_together2/'..relative))
  setfenv(chunk,env);return chunk()
 end
 env.spawn_wands=function(x,y) observed={x,y};return 'base-wand-result' end
 env.spawn_potions=env.spawn_wands;env.spawn_heart=env.spawn_wands
 env.SetRandomSeed=function(x,y) assert(x==state.frame and y==69420+7);random_calls=random_calls+1 end
 env.Random=function(min,max) assert(min==0 and max>0);return 1 end
 env.StatsGlobalGetValue=function(key) assert(key=='death_count');return '7' end
 env.dofile('mods/noita_together2/files/loot/biome_scripts.lua')
 assert(env.spawn_wands(100,200)=='base-wand-result')
 assert(observed[1]==(loot_enabled and 99 or 100) and observed[2]==(loot_enabled and 199 or 200))
 assert(random_calls==(loot_enabled and 1 or 0))
 assert(world_ready or savedGlobals.nt2_seed_loot_policy_v1==nil,'early hook touched unavailable Globals')
end
prove_hook_context(false);mark('loot-early-separate-context-bound',recorded)
loot_mod_init=false;loot_world_ready=true
generated=true;OnMagicNumbersAndWorldSeedInitialized();assert(savedGlobals.nt2_seed_loot_policy_v1==recorded);prove_hook_context(true);mark('loot-world-separate-context-bound',recorded);OnPlayerSpawned(99)
end
local base_button=GuiButton
GuiButton=function(gui,id,...)
 if id==9141 and exists('choose-rejoin') then return true end
 return base_button(gui,id,...)
end
local function serialized_save()
 local xml='<WorldState><Globals>'
 for key,value in pairs(savedGlobals) do
  assert(not value:find('["&<>]'),'save attribute is not XML-safe')
  xml=xml..'<E key="'..key..'" value="'..value..'"/>'
 end
 xml=xml..'</Globals></WorldState>';mark('serialized-world.xml',xml);return xml
end
local began=tonumber(ffi.C.GetTickCount());local playing=false
while tonumber(ffi.C.GetTickCount())-began<180000 do
 if exists('continue-in-process') and not exists('continue-in-process-loaded') then
  local xml=serialized_save();local restored={};for key,value in xml:gmatch('<E key="([^"]+)" value="([^"]*)"/>') do restored[key]=value end;savedGlobals=restored
  OnWorldInitialized();OnPlayerSpawned(99);mark('continue-in-process-loaded')
 end
 state.frame=state.frame+1;local before=state.components[1].enabled
 -- CF04: native worker remains live, but this original engine world deliberately
 -- delays consumption across ordinary app pulse/status renewals.
 if os.getenv('NT2_CF04_DELAY_ENTRY')=='1' and exists('delay-first-entry') and not exists('allow-first-entry') then mark('first-entry-paused') else OnWorldPreUpdate();OnWorldPostUpdate() end
 if os.getenv('NT2_C3_CORRECTION')=='1' then
  assert(state.enabled[5] and state.enabled[6] and not state.enabled[7],'preserve original inventory enable flags')
  -- Inject only the eligibility seam; this is not native inventory physics.
  if state.enabled[6] then state.components[6].mInitialized=true;mark('inventory-init-eligible') end
  assert(state.components[11].drinkable==state.components[1].enabled,'drinking follows held/restored original')
 end
 local now=state.components[1].enabled
 if now and not playing then assert(exists('start'),'release before inspected Start');playing=true;mark('playing') end
 if savedGlobals.nt2_rejoin_local_world_v1 then
  mark('saved-association.txt',savedGlobals.nt2_rejoin_local_world_v1)
  mark('saved-seed.txt',savedGlobals.nt2_rejoin_local_world_v1_seed)
  mark('saved-loot.txt',savedGlobals.nt2_rejoin_local_world_v1_loot)
  serialized_save();mark('associated')
 end
 if held>0 and now then mark('resumed-playing') end
 if playing and not exists('wrong-world-refused') then
  local native=original_load(assert(os.getenv('NT2_C3_DLL_DIR'))..'/nt2_g3_transport.dll')
  local wrong=ffi.new('uint8_t[16]');ffi.fill(wrong,16,9)
  assert(native.nt2_g3_move(wrong,1,0,0,0,0,0)==0 and native.nt2_g3_close(wrong)==0)
  mark('wrong-world-refused')
 end
 if before and not now and playing then held=held+1 end
 if not now then mark('protected') end
 if exists('expire') and playing and not now then mark('permission-expired-protected') end
 if exists('end') and not now then
  assert(playing and ghosts>0,'must receive actual remote ghost before End');mark('end-protected','holds='..held..';ghosts='..ghosts);break
 end
 if exists('assert-terminal-end') and playing and not now and savedGlobals.nt2_rejoin_local_world_v1_offline~='1' then mark('terminal-protected') end
 if savedGlobals.nt2_rejoin_local_world_v1_offline=='1' and now then mark('offline-restored') end
 if exists('unaffected-check') then mark('unaffected-snapshot','playing='..tostring(now)..';holds='..held..';ghosts='..ghosts) end
 if exists('stop') then break end
 ffi.C.Sleep(10)
end
mark('result','playing='..tostring(playing)..';ghosts='..ghosts..';holds='..held)
assert(exists('stop') or exists('end-protected'),'composed native fixture timeout')
