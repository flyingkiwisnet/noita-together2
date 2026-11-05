-- Actual C2 entry + gameplay callbacks, private wrappers/broker and OS pipes.
-- Only engine APIs are injected. Production DLLs and Noita are never executed.
local repo=assert(os.getenv('NT2_C3_REPO'))
local dir=assert(os.getenv('NT2_C3_DIR'))
local ffi=require('ffi');ffi.cdef[[void __stdcall Sleep(unsigned long);unsigned long __stdcall GetTickCount();]]
io.stdout:setvbuf('no')
dofile(repo..'/apps/desktop/tools/mod-protection-stubs.lua')
local state=NT2_FIXTURE
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
SetWorldSeed=function(value) assert(not generated,'seed must precede generation');assert(value==424242);seed=true;mark('seed-before-generation') end
StatsGetValue=function() return os.getenv('NT2_C3_WRONG_SEED')=='1' and '999' or '424242' end
ModGetActiveModIDs=function() return {'noita_together2'} end
GameIsBetaBuild=function() return nil end
GuiDestroy=function() end;GuiImage=function() end
local raw_dofile=dofile
dofile=function(path) local relative=assert(path:match('^mods/noita_together2/(.+)$'));return raw_dofile(repo..'/mod/noita_together2/'..relative) end
local original_load=ffi.load
local pinned_images={}
local function load_image(path)
 local image=original_load(path)
 if os.getenv('NT2_C3_DIAGNOSTIC_PIN')=='1' then pinned_images[#pinned_images+1]=image end
 return image
end
ffi.load=function(path)
 if path=='mods/noita_together2/files/nt2_g3_transport.dll' then return load_image(assert(os.getenv('NT2_C3_DLL_DIR'))..'/nt2_g3_transport.dll') end
 if path=='mods/noita_together2/files/nt2_g3_social.dll' then return load_image(assert(os.getenv('NT2_C3_DLL_DIR'))..'/nt2_g3_social.dll') end
 return original_load(path)
end
-- Core verified the exact mapped candidate in this owned fixture before launch.
raw_dofile(os.getenv('NT2_C3_ENTRY_PATH') or dir..'/verified-files/mods/noita_together2/init.lua')
OnModPreInit();ffi.load=original_load
if os.getenv('NT2_C3_DIAGNOSTIC_PIN')=='1' then mark('diagnostic-pinned-images') end
if os.getenv('NT2_C3_GC_PROBE')=='1' then
 assert(os.getenv('NT2_C3_DIAGNOSTIC_PIN')~='1','GC proof must rely on the actual entry')
 collectgarbage('collect');collectgarbage('collect')
 ffi.cdef[[void* __stdcall GetModuleHandleA(const char*);]]
 for _,name in ipairs({'nt2_g3_transport.dll','nt2_g3_social.dll','nt2_g3_ipc.dll'}) do assert(ffi.C.GetModuleHandleA(name)~=nil,'wrapper image collected: '..name) end
 mark('gc-images-retained')
end
if os.getenv('NT2_C3_REFUSE')=='1' then assert(not seed,'Continue/refusal must not replace seed');mark('refused');return end
assert(seed,'actual private broker preparation refused')
generated=true;OnMagicNumbersAndWorldSeedInitialized();OnPlayerSpawned(99)
local began=tonumber(ffi.C.GetTickCount());local playing=false
while tonumber(ffi.C.GetTickCount())-began<90000 do
 state.frame=state.frame+1;local before=state.components[1].enabled
 OnWorldPreUpdate();OnWorldPostUpdate()
 if os.getenv('NT2_C3_CORRECTION')=='1' then
  assert(state.enabled[5] and state.enabled[6] and not state.enabled[7],'preserve original inventory enable flags')
  -- Inject only the eligibility seam; this is not native inventory physics.
  if state.enabled[6] then state.components[6].mInitialized=true;mark('inventory-init-eligible') end
  assert(state.components[11].drinkable==state.components[1].enabled,'drinking follows held/restored original')
 end
 local now=state.components[1].enabled
 if now and not playing then assert(exists('start'),'release before inspected Start');playing=true;mark('playing') end
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
 if exists('stop') then break end
 ffi.C.Sleep(10)
end
mark('result','playing='..tostring(playing)..';ghosts='..ghosts..';holds='..held)
assert(exists('stop') or exists('end-protected'),'composed native fixture timeout')
