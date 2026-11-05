-- LC1 actual entry/gameplay adapters + mapped production wrapper/broker source
-- with isolated selectors. Engine APIs/save restoration are injected; no Noita.
local repo=assert(os.getenv('NT2_C3_REPO'))
local globals_xml=dofile(repo..'/apps/desktop/tools/noita-globals-xml.lua')
local dir=assert(os.getenv('NT2_C3_DIR'))
local files=os.getenv('NT2_LC1_FILES') or dir..'/verified-files'
local ffi=require('ffi');ffi.cdef[[void __stdcall Sleep(unsigned long);unsigned long __stdcall GetTickCount();]]
local cadence=dofile(repo..'/apps/desktop/tools/lifecycle-lc1-clock.lua').new()
local transport_metrics=os.getenv('NT2_LOCAL_WS_METRICS')=='1'
local measure_image,callback_max,callback_samples,measure_at=nil,0,{},0
if transport_metrics then ffi.cdef[[typedef struct {uint32_t version,size;void* first[7];int (*metrics)(uint32_t*,uint32_t);} NT2LocalMeasure;const NT2LocalMeasure* nt2_g3_private_api(void);]] end

io.stdout:setvbuf('no')
dofile(repo..'/apps/desktop/tools/mod-protection-stubs.lua')
local state=NT2_FIXTURE
local permission_observer=dofile(repo..'/apps/desktop/tools/lifecycle-lc1-permission-observer.lua').new()
local permission_edges={}
local registrations,registry={},{}
ModLuaFileAppend=function(base,hook) if base:match('/startup/scope_') then registry[base]=registry[base] or {};table.insert(registry[base],hook)else registrations[#registrations+1]={base,hook};assert(#registrations<=(os.getenv('NT2_RACE_FIXTURE')=='1'and 38 or os.getenv('NT2_C2_WORLD_FIXTURE')=='1'and 27 or os.getenv('NT2_C1_PLAYER_FIXTURE')=='1'and 24 or 14))end end
ModLuaFileGetAppends=function(base)return registry[base] or {}end
local savedGlobals={}
local savedMode=os.getenv('NT2_REJOIN_SAVED')=='1'
if savedMode then
 local function read(name) local f=assert(io.open(dir..'/'..name,'rb'));local s=f:read('*a');f:close();return s end
 savedGlobals.nt2_rejoin_local_world_v1=read('xml-restored-association.txt')
 savedGlobals.nt2_rejoin_local_world_v1_seed=read('xml-restored-seed.txt')
 savedGlobals.nt2_rejoin_local_world_v1_loot=read('xml-restored-loot.txt')
 local offline_file=io.open(dir..'/xml-restored-offline.txt','rb');savedGlobals.nt2_rejoin_local_world_v1_offline=offline_file and offline_file:read('*a') or '0';if offline_file then offline_file:close()end
 savedGlobals.nt2_seed_loot_policy_v1=savedGlobals.nt2_rejoin_local_world_v1_loot
 if os.getenv('NT2_A3_FIXTURE')=='1'then
  local f=assert(io.open(dir..'/xml-restored-a3-world.xml','rb'));local xml=f:read('*a');f:close()
  local restored=assert(globals_xml.restore(xml),'Invalid raw Noita Globals XML')
  for _,key in ipairs({'nt2_rejoin_local_world_v1','nt2_rejoin_local_world_v1_seed','nt2_rejoin_local_world_v1_loot'})do assert(restored[key]==savedGlobals[key],'Serialized association field missing/different: '..key)end
  savedGlobals=restored
 end
end
local marker_case=os.getenv('NT2_LC1_MARKER_CASE')
if marker_case=='' then marker_case=nil end
local restored_complete={};for key,value in pairs(savedGlobals)do restored_complete[key]=value end
if marker_case=='missing-seed' or marker_case=='delayed' then savedGlobals.nt2_rejoin_local_world_v1_seed='' end
if marker_case=='missing-loot' or marker_case=='delayed' then savedGlobals.nt2_rejoin_local_world_v1_loot='' end
if marker_case=='malformed' then savedGlobals.nt2_rejoin_local_world_v1='nt2h1:4:zzzzzzzz' end
if marker_case=='missing-marker' then savedGlobals.nt2_rejoin_local_world_v1='' end
if marker_case=='wrong-world' then
 local codec=dofile(files..'/mods/noita_together2/files/rejoin/local_world.lua')
 local record=assert(codec.decode_saved(savedGlobals.nt2_rejoin_local_world_v1))
 record=record:gsub('("local_world"%s*:%s*")[%x%-]+',function(prefix)return prefix..'11111111-1111-4111-8111-111111111111'end)
 savedGlobals.nt2_rejoin_local_world_v1=assert(codec.encode_saved(record))
end
if marker_case=='unrelated' then savedGlobals={} end
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
local ghost_motion_updates=0
local seed,generated,ghosts,held,seed_calls=false,false,0,0,0
local hud_frames,hud_content_frames=0,0
local last_playing_frame,last_playing_tick=0,0
local native_reason=0
local native_status,native_status_frame,native_context,native_context_bytes=0,0,'',nil
local native_lookup,native_lookup_frame,native_lookup_world,native_lookup_context=0,0,'',''
local function mark(name,value) local f=assert(io.open(dir..'/'..name,'wb'));f:write(value or 'fixture');f:close() end
local function exists(name) local f=io.open(dir..'/'..name,'rb');if f then f:close();return true end end
local function input(name,remove)local f=io.open(dir..'/'..name,'rb');if not f then return nil end;local value=f:read('*a');f:close();if remove then assert(os.remove(dir..'/'..name))end;return value end
if os.getenv('NT2_A3_FIXTURE')=='1'then
 local signals=dofile(repo..'/apps/desktop/tools/lifecycle-lc1-files.lua').new(dir)
 mark=function(name,value)return signals:mark(name,value)end;exists=function(name)return signals:exists(name)end;input=function(name,remove)return signals:input(name,remove)end
end
-- A2 fixtures drive the real local UI and inject only engine observations.
local a2_variables={};local a2_serial=50000;local add_a2=EntityAddComponent2;local list_a2=EntityGetComponentIncludingDisabled;local remove_a2=EntityRemoveComponent
EntityAddComponent2=function(id,kind,values)if id==99 and kind=='VariableStorageComponent'and values.name=='nt2_a2_composer_lock'then a2_serial=a2_serial+1;state.components[a2_serial]=values;a2_variables[a2_serial]=true;return a2_serial end;return add_a2(id,kind,values)end
EntityGetComponentIncludingDisabled=function(id,kind)local out=list_a2(id,kind)or {};if id==99 and kind=='VariableStorageComponent'then for c in pairs(a2_variables)do out[#out+1]=c end end;return out end
EntityRemoveComponent=function(id,c)if a2_variables[c]then a2_variables[c]=nil;state.components[c]=nil else remove_a2(id,c)end end
state.components[2].hp=4;state.components[2].max_hp=8
local get_a2=ComponentGetValue2
ComponentGetValue2=function(id,key)if id==2 and (key=='hp'or key=='max_hp')then if exists('a2-hp-unavailable')then return nil end;return tonumber(input(key=='hp'and 'a2-hp'or 'a2-max-hp',false))or get_a2(id,key)end;return get_a2(id,key)end
BiomeMapGetName=function()if exists('a2-biome-unavailable')then return nil end;return input('a2-biome',false)or '$biome_coalmine'end
DebugBiomeMapGetFilename=function()if exists('a2-biome-unavailable')then return nil end;return input('a2-biome-file',false)or 'data/biome/coalmine.xml'end
GameTextGetTranslatedOrNot=function(key)return ({['$biome_coalmine']='Mines',['$biome_snowcave']='Snowy Depths',['$nt2_starting_mountain']='Mountain',['$biome_retained_custom']='Unlisted Sanctuary'})[key]or key end
GamePrint=function(value)local f=assert(io.open(dir..'/a2-notices.txt','ab'));f:write(value,'\n');f:close()end
local a2_texts,a2_names={},{};local a2_sampo,a2_radar=0,0;local a2_fill,a2_bar,a2_min,a2_max=0,0,1,0;local function press(label)if input('a2-click',false)==label then input('a2-click',true);mark('a2-clicked',label);return true end;return false end
GuiImageButton=function(_,_,_,_,_,path)local icon=path:match('/([^/]+)%.png$');return press(({keyboard='World message',player_list='Players',chat='Hide chat',hide_chat='Show chat',player_ghosts='Hide ghosts',hide_player_ghosts='Show ghosts'})[icon]or '')end
GuiTextInput=function(_,_,_,_,value)return input('a2-text',true)or value end
local original_alive=EntityGetIsAlive;local original_kill=EntityKill
EntityGetIsAlive=function(id) return starter[id] or entities[id]~=nil or original_alive(id) end
EntityKill=function(id) if entities[id] then entities[id]=nil else original_kill(id) end end
EntityLoad=function(path,x,y) assert(path=='mods/noita_together2/files/g3/ghost.xml' or path=='mods/noita_together2/files/g3/poi.xml');next_entity=next_entity+1;entities[next_entity]={x=x,y=y,ghost=path:find('/ghost.xml',1,true)~=nil};if path:find('/ghost.xml',1,true) then ghosts=ghosts+1;mark('ghost',tostring(x))else state.components[next_entity+100000]={};entities[next_entity].item=next_entity+100000 end;return next_entity end
EntityGetTransform=function(id) if entities[id] then return entities[id].x,entities[id].y,0 end;return (tonumber(input('a2-x',false))or tonumber(os.getenv('NT2_C3_X')))+(exists('move-source') and math.sin(state.frame/50)*20 or 0),tonumber(input('a2-y',false))or 2,0 end
local original_children=EntityGetAllChildren;local original_tag=EntityHasTag
EntityGetAllChildren=function(id) if id==200 then return {201,202,203} end;if id==99 then local result={200};for _,child in ipairs(original_children(id) or {}) do result[#result+1]=child end;return result end;return original_children(id) end
EntityGetName=function(id) return id==200 and 'inventory_quick' or '' end
EntityGetParent=function(id) return id==999 and state.effect and 99 or id==200 and 99 or starter[id] and 200 or 0 end
EntityGetRootEntity=function(id) return starter[id] and 99 or id end
EntityHasTag=function(id,tag) if starter[id] then return id==203 and tag=='this_is_sampo'and exists('a2-sampo')or id==201 and (tag=='potion' or tag=='item_pickup' or tag=='item_physics') or (id==202 or id==203) and tag=='wand' end;return original_tag(id,tag) end
GetMaterialInventoryMainMaterial=function(id) return id==201 and 1 or 0 end
EntitySetTransform=function(id,x,y)
 if not entities[id]then return end
 local step=math.abs(entities[id].x-x);entities[id].x=x;entities[id].y=y
 if step<=0.001 then return end
 local expected=tonumber(input('race-expected-parallel-teleport',false))
 if os.getenv('NT2_RACE_FIXTURE')=='1'and expected and step>=256 and math.abs(step-expected)<256 then
  mark('race-ghost-teleport.json',string.format('{"injectedEngine":true,"step":%.3f,"expectedWidth":%d}',step,expected))
 else
  assert(step<128,'unbounded synthetic ghost step')
  if entities[id].ghost then ghost_motion_updates=ghost_motion_updates+1 end
  mark('ghost-motion',tostring(step))
 end
end
local original_component=EntityGetFirstComponentIncludingDisabled
EntityGetFirstComponentIncludingDisabled=function(id,kind) if entities[id]then return kind=='ItemComponent'and entities[id].item end;if id==201 then return ({ItemComponent=11,MaterialInventoryComponent=12,PhysicsBodyComponent=13,DamageModelComponent=14,PotionComponent=15})[kind] end;if starter[id] or kind=='VelocityComponent' then return nil end;return original_component(id,kind) end
GameGetRealWorldTimeSinceStarted=function() return tonumber(ffi.C.GetTickCount())/1000 end
local polymorph=nil
if os.getenv('NT2_POLYMORPH_FIXTURE')=='1'then polymorph=dofile(repo..'/apps/desktop/tools/lifecycle-lc1-polymorph.lua').new(_G,state,input,mark)end
local policy=os.getenv('NT2_U2_POLICY') or 'shared-random'
local shared=tonumber(os.getenv('NT2_U2_SEED') or '424242')
local actual=policy=='individual-random' and (os.getenv('NT2_U2_ACTUAL') or '777') or tostring(shared)
SetWorldSeed=function(value) assert(not generated,'seed must precede generation');assert(policy~='individual-random' and value==shared);seed=true;seed_calls=seed_calls+1;mark('seed-before-generation') end
StatsGetValue=function() return (os.getenv('NT2_C3_WRONG_SEED')=='1' or exists('wrong-seed')) and '999' or actual end
ModGetActiveModIDs=function() if exists('wrong-mods')then return {'noita_together2','lc1_injected_unapproved'}end;return {'noita_together2'} end
GameIsBetaBuild=function() return nil end
GuiDestroy=function() end;GuiImage=function(_,_,_,_,path,_,scale)if path:find("/hpbar_full.png",1,true)then a2_fill=a2_fill+1;a2_min=math.min(a2_min,scale);a2_max=math.max(a2_max,scale)elseif path:find("/hpbar_empty.png",1,true)then a2_bar=a2_bar+1 end;if path:find('/sampo.png',1,true)then a2_sampo=a2_sampo+1 end;if path:find('/radar.png',1,true)then a2_radar=a2_radar+1 end end
GuiText=function(_,_,_,text)if type(text)=='string'then a2_texts[#a2_texts+1]=text;if text:match('^HP ')then hud_content_frames=hud_content_frames+1 end end end
-- Record the production player-list tooltip as well as inline GUI text.
GuiTooltip=function(_,_,body)if type(body)=='string'then a2_texts[#a2_texts+1]=body;if body:match('^HP ')then hud_content_frames=hud_content_frames+1 end end end
local a3_fixture=nil
if os.getenv('NT2_A3_FIXTURE')=='1'then a3_fixture=dofile(repo..'/apps/desktop/tools/lifecycle-lc1-a3.lua').new(_G,state,polymorph,entities,input,mark)end
local race_fixture=nil
local b1_fixture=nil
if os.getenv('NT2_B1_FIXTURE')=='1'then b1_fixture=dofile(repo..'/apps/desktop/tools/lifecycle-lc1-b1.lua').new(_G,state,polymorph,input,mark)end
local item_fixture=nil
if os.getenv('NT2_ITEM_FIXTURE')=='1'then item_fixture=dofile(repo..'/apps/desktop/tools/lifecycle-lc1-items.lua').new(_G,state,input,mark,assert(b1_fixture).item_cell_hit)end
local raw_dofile=dofile
local optional_inventory=raw_dofile(repo..'/apps/desktop/tools/lifecycle-lc1-optional-inventory.lua')
local function json_text(value)return '"'..value:gsub('[%z\1-\31\\"]',function(c)if c=='"'then return '\\"'elseif c=='\\'then return '\\\\'else return string.format('\\u%04x',c:byte())end end)..'"'end
local c1_fixture=nil;local c2_fixture=nil
local a2_social=nil;local a2_pages={gaps=0,retained=0,discarded=0,commits=0,rejected=0,replays=0}
local a2_hud={samples=0,known=0,copied=0,received=0,receivedKnown=0,applied=0,cleared=0,clearedVisible=0,copyGaps=0,freshRepeat=0,liveRows=0,retainedVisible=0,endControls=0}
dofile=function(path)
 local relative=assert(path:match('^mods/noita_together2/(.+)$'));local result=raw_dofile(files..'/mods/noita_together2/'..relative)
 -- Read-only fixture counters over actual adapters. No returned data or
 -- validation outcome is replaced; no bodies/credentials are recorded.
 if relative=='files/g3/a2_capture.lua' then local create=result.publisher;result.publisher=function(e,send)local capture_engine=optional_inventory.capture_engine(e,function()return exists('a2-inventory-unavailable')end);return create(capture_engine,function(value)a2_hud.samples=a2_hud.samples+1;if value.hp~=nil and value.max_hp~=nil then a2_hud.known=a2_hud.known+1 end;if exists('a2-no-publish')then return false end;local ok=send(value);if ok then a2_hud.copied=a2_hud.copied+1 end;return ok end)end
 elseif relative=='files/g3/bank_native.lua'and os.getenv('NT2_RACE_FIXTURE')=='1'then
  local create=result.new;result.new=function(dll,context,ffi,...)
   local copied,refused,last=0,0,nil
   local proxy=setmetatable({nt2_g3_bank_publish=function(c,body,n)
    local value=dll.nt2_g3_bank_publish(c,body,n)
    if n>=26 and ffi.string(body,26)=='{"kind":"race_completion",'then
     if value==1 then copied=copied+1 else refused=refused+1 end
     if value~=last or(copied+refused)%30==0 then mark('race-copy.json',string.format('{"copied":%d,"refused":%d,"lastReturn":%d,"bytes":%d}',copied,refused,value,n));last=value end
    end
    return value
   end},{__index=function(_,key)return dll[key]end})
   return create(proxy,context,ffi,...)
  end
 elseif relative=='files/g3/c1_local.lua'and c1_fixture then local create=result.new;result.new=function(e,...)return create(c1_fixture:engine(e),...)end
 elseif relative=='files/g3/social_native.lua' then local create=result.new;result.new=function(...)
  local v=create(...);a2_social=v;local native_tick=v.tick;v.tick=function(s,...)local ended=s.ended;local result=native_tick(s,...);if c1_fixture then c1_fixture:tick(s.bank)end;if c2_fixture then c2_fixture:tick(s.bank)end;if race_fixture then race_fixture:tick(s.bank)end;if not ended and s.ended then a2_hud.endControls=a2_hud.endControls+1 end;return result end;local pause=v.state.pause;v.state.pause=function(s,keep,...)local prefix=s.pending;local r=pause(s,keep,...);if prefix then a2_pages.gaps=a2_pages.gaps+1;if s.pending==prefix then a2_pages.retained=a2_pages.retained+1 else a2_pages.discarded=a2_pages.discarded+1 end end;return r end
  if a3_fixture then local submit=v.renderer.model.submit;v.renderer.model.submit=function(s,...)local result=submit(s,...);a3_fixture:observe_poi_submit(s,result);return result end end
  local observe_pause=v.state.pause;v.state.pause=function(s,keep,...)if keep then a2_hud.copyGaps=a2_hud.copyGaps+1 end;return observe_pause(s,keep,...)end
  local clear=v.renderer.api.clear_hud;v.renderer.api.clear_hud=function(...)a2_hud.cleared=a2_hud.cleared+1;local had=next(v.renderer.model.hud);local r=clear(...);if had then if next(v.renderer.model.hud)then a2_hud.retainedVisible=a2_hud.retainedVisible+1 else a2_hud.clearedVisible=a2_hud.clearedVisible+1 end end;return r end
  local receive=v.renderer.receive_info;v.renderer.receive_info=function(s,c)a2_hud.received=a2_hud.received+1;if c.hp~=nil and c.max_hp~=nil then a2_hud.receivedKnown=a2_hud.receivedKnown+1 end;local old=s.model.hud_versions[c.actor];if old and old.revision==c.revision and not s.model.hud[c.actor]then a2_hud.freshRepeat=a2_hud.freshRepeat+1 end;local ok=receive(s,c);if ok then a2_hud.applied=a2_hud.applied+1 end;return ok end
  local page=v.state.page;v.state.page=function(s,...)local prefix=s.pending;local r=page(s,...);if not r then a2_pages.rejected=a2_pages.rejected+1 elseif prefix and not s.pending then a2_pages.commits=a2_pages.commits+1 end;return r end
  return v
 end
 elseif relative=='files/g3/social_dispatch.lua' then local create=result.new;result.new=function(...)
  local v=create(...);local receive=v.receive;v.receive=function(s,...)local r,reason=receive(s,...);if reason=='replay'then a2_pages.replays=a2_pages.replays+1 end;return r,reason end;return v
 end end
 return result
end
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
local observed_moves,last_move_x,last_move_subject=0,0,0
local function load_image(path)
 local image=original_load(path)
 if os.getenv('NT2_C3_DIAGNOSTIC_PIN')=='1' then pinned_images[#pinned_images+1]=image end
 return image
end
ffi.load=function(path)
 if path=='mods/noita_together2/files/nt2_g3_social.dll'and a3_fixture then
  local image=load_image(assert(os.getenv('NT2_C3_DLL_DIR'))..'/nt2_g3_social.dll')
  return setmetatable({}, {__index=function(_,key)
   if key=='nt2_g3_social_lease'then return function(context,header,size)local result=image.nt2_g3_social_lease(context,header,size);a3_fixture:observe_social_lease(tonumber(result),header);return result end end
   return image[key]
  end})
 end
 if path=='mods/noita_together2/files/nt2_g3_transport.dll' then
  local image=load_image(assert(os.getenv('NT2_C3_DLL_DIR'))..'/nt2_g3_transport.dll')
  if transport_metrics then measure_image=image end
  return setmetatable({}, {__index=function(_,key)
   if key=='nt2_g3_peers'and a3_fixture then return function(context,reply,size)local result=image.nt2_g3_peers(context,reply,size);a3_fixture:observe_peers(tonumber(result),reply);return result end end
   if key=='nt2_g3_social_publish'and a3_fixture then return function(context,buffer,size)local body=ffi.string(buffer,size);local blocked=input('a3-copy-block',false)and(body:find('"kind":"emote"',1,true)or body:find('"kind":"poi"',1,true));if blocked then mark('a3-copy-blocked','explicit native copy refusal injected; original bytes not copied');a3_fixture:observe_social_copy(0,body);return 0 end;local result=image.nt2_g3_social_publish(context,buffer,size);a3_fixture:observe_social_copy(tonumber(result),body);return result end end
   if key=='nt2_g3_bank_publish'and b1_fixture then return function(context,buffer,size)local body=ffi.string(buffer,size);mark('b1-last-input.json',body);local kind=body:match('"kind":"([^"]+)"');if kind=='submit'then mark('b1-last-submit.json',body)end;if kind=='item_chunk'and body:find('"channel":"deposit"',1,true)then mark('items-last-deposit.json',body)end;if kind=='item_claim'then mark('items-last-claim.json',body)end;local blocked=input('b1-copy-block-kind',false);if (blocked and blocked==kind and not kind:match('^item_'))or b1_fixture:block_item_copy(body)then mark('b1-copy-blocked.json',body);return 0 end;return image.nt2_g3_bank_publish(context,buffer,size)end end
   if key=='nt2_g3_move'then return function(...)if exists('debug-pause-movement')then mark('debug-movement-paused','labelled injected source suppression; native heartbeat and ownership unchanged');return 0 end;local args={...};local result=image.nt2_g3_move(...);if tonumber(result)==1 then observed_moves=observed_moves+1;last_move_x=tonumber(args[3]);last_move_subject=polymorph and polymorph:current()or 99 end;return result end end
   if key=='nt2_g3_saved_lookup' then return function(record,size)
    local value=tonumber(image.nt2_g3_saved_lookup(record,size))
    local body=type(record)=='string' and record or ffi.string(record,size)
    native_lookup=value;native_lookup_frame=state.frame;native_lookup_world=body:match('"local_world"%s*:%s*"([%x%-]+)"') or '';native_lookup_context=native_context
    return value -- Observe only the production caller's actual scoped poll.
   end end
   if key=='nt2_g3_status' then return function(context)
    local value=tonumber(image.nt2_g3_status(context));native_status=value;native_status_frame=state.frame;native_reason=tonumber(image.nt2_g3_heartbeat_diagnostic(context))
    if context~=nil then local bytes=ffi.string(context,16);if bytes~=native_context_bytes then native_context_bytes=bytes;native_context=bytes:gsub('.',function(c)return string.format('%02x',string.byte(c))end)end end
    return value -- Observe the actual cached wrapper result without changing it.
   end end
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
if os.getenv('NT2_C1_PLAYER_FIXTURE')=='1'then c1_fixture=raw_dofile(repo..'/apps/desktop/tools/lifecycle-lc1-c1.lua').new(_G,state,polymorph,input,mark,files)end
if os.getenv('NT2_C2_WORLD_FIXTURE')=='1'then c2_fixture=raw_dofile(repo..'/apps/desktop/tools/lifecycle-lc1-c2.lua').new(_G,state,polymorph,input,mark,files)end
if os.getenv('NT2_RACE_FIXTURE')=='1'then race_fixture=raw_dofile(repo..'/apps/desktop/tools/lifecycle-lc1-race.lua').new(_G,state,polymorph,input,mark,files)end
raw_dofile(os.getenv('NT2_C3_ENTRY_PATH') or files..'/mods/noita_together2/init.lua')
assert(#registrations==(os.getenv('NT2_RACE_FIXTURE')=='1'and 38 or os.getenv('NT2_C2_WORLD_FIXTURE')=='1'and 27 or os.getenv('NT2_C1_PLAYER_FIXTURE')=='1'and 24 or 14))
-- Explicit engine setting scope, not callback timing or missing-marker inference.
local scope=raw_dofile(files..'/mods/noita_together2/files/startup/settings_scope.lua')
assert(scope.produce(savedMode and 'restart' or 'new')=='recorded');OnModPreInit();if not savedMode then ffi.load=original_load end
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
raw_dofile(repo..'/apps/desktop/tools/a2-layout-test-api.lua')(_G)
local loot_checks=raw_dofile(repo..'/apps/desktop/tools/lifecycle-lc1-loot-harness.lua')
if savedMode then
 assert(not seed,'saved Continue cannot request seed')
 loot_mod_init=false;generated=true
 -- Restored scope was consumed by production OnModPreInit above. Exercise
 -- callbacks before readable WorldState, without a fresh bundle or broker warmup.
 if OnWorldInitialized then OnWorldInitialized() end
 OnPlayerSpawned(polymorph and polymorph:current()or 99)
 for i=1,3 do state.frame=i;OnWorldPreUpdate();OnWorldPostUpdate();ffi.C.Sleep(10) end
 assert(not state.components[1].enabled,'Continue readiness gap was not protected')
 loot_world_ready=true;ffi.load=original_load
 loot_checks.saved(files,_G,savedGlobals.nt2_seed_loot_policy_v1 or '')
 assert(seed_calls==0,'Continue reapplied a fresh seed')
 mark('continue-restored-scope-cold-broker-late-world')
else
assert(policy=='individual-random' and not seed or policy~='individual-random' and seed,'actual private policy binding refused');mark('generation-policy-bound',policy)
local loot_enabled=policy~='individual-random' and os.getenv('NT2_U2_LOOT_OFF')~='1'
local recorded=policy=='individual-random' and 'individual:0' or ('shared:'..(loot_enabled and '1' or '0')..':'..string.format('%.0f',shared))
assert(loot_bootstrap=='entry-new:'..recorded and savedGlobals.nt2_seed_loot_policy_v1==nil,'actual broker loot policy must be staged for the consumed entry without pre-world Globals')
mark('loot-policy-bound',recorded)
loot_checks.fresh(files,_G,recorded,loot_enabled);assert(savedGlobals.nt2_seed_loot_policy_v1==nil,'early hook touched unavailable Globals');mark('loot-early-separate-context-bound',recorded)
loot_mod_init=false;loot_world_ready=true
generated=true;OnMagicNumbersAndWorldSeedInitialized();assert(savedGlobals.nt2_seed_loot_policy_v1==recorded);loot_checks.fresh(files,_G,recorded,loot_enabled);mark('loot-world-separate-context-bound',recorded);OnPlayerSpawned(polymorph and polymorph:current()or 99)
end
local item_image_button=GuiImageButton;local item_widget_info=GuiGetPreviousWidgetInfo;local item_hover
GuiImageButton=function(gui,id,x,y,text,path)
 if item_fixture and path:match('/item_bank/slot.png$')then item_hover=item_fixture:hover(x,y);return item_fixture:button(x,y,'')end
 item_hover=nil;return item_image_button(gui,id,x,y,text,path)
end
GuiGetPreviousWidgetInfo=function(...)if item_hover~=nil then return false,false,item_hover end;if item_widget_info then return item_widget_info(...)end;return false,false,false end
local base_button=GuiButton
GuiButton=function(gui,id,...)
 if id==9141 and exists('choose-rejoin') then return true end
 return base_button(gui,id,...)
end
local function serialized_save(output)
 local xml=globals_xml.serialize(savedGlobals)
 assert(globals_xml.restore(xml),'Raw Globals broke Noita XML serialization')
 mark(output or 'serialized-world.xml',xml);return xml
end
local began=tonumber(ffi.C.GetTickCount());local playing=false;local last_saved_dismissal=nil
GuiButton=function(_,id,x,y,label)if item_fixture and item_fixture:button(x,y,label)then return true end;if label=='Players' then hud_frames=hud_frames+1 end;if label and label:sub(1,1)==' 'then a2_names[#a2_names+1]=label end;if id==9141 and exists('choose-rejoin')then return true end;if id==9142 and exists('choose-offline')then return true end;return press(label)end
while tonumber(ffi.C.GetTickCount())-began<900000 do
 local frame_started=cadence.time()
 if polymorph then polymorph:apply()end
 if marker_case=='delayed' and exists('restore-fields') then
  for key,value in pairs(restored_complete)do savedGlobals[key]=value end
  marker_case='completed-delay';mark('fields-restored')
 end
 if exists('continue-in-process') and not exists('continue-in-process-loaded') then
  local xml=serialized_save();savedGlobals=assert(globals_xml.restore(xml))
  local before_seed_calls=seed_calls;local restored_loot=savedGlobals.nt2_seed_loot_policy_v1 or ''
  registry={};loot_mod_init=true;loot_world_ready=false;assert(scope.produce('restart')=='recorded');OnModPreInit();assert(loot_bootstrap=='saved','same-process Continue did not reset bootstrap')
  loot_mod_init=false;OnWorldInitialized();OnPlayerSpawned(polymorph and polymorph:current()or 99);loot_world_ready=true;loot_checks.saved(files,_G,restored_loot);assert(seed_calls==before_seed_calls,'same-process Continue reapplied seed');mark('continue-in-process-loaded')
 end
 state.frame=state.frame+1;a2_texts={};a2_names={};a2_sampo=0;a2_radar=0;a2_fill=0;a2_bar=0;a2_min=1;a2_max=0
 if input('a2-dismiss',true)then for id,entity in pairs(entities)do if not entity.ghost then entities[id]=nil;break end end end
 -- CF04: native worker remains live, but this original engine world deliberately
 -- delays consumption across ordinary app pulse/status renewals.
 local callback_started=cadence.time()
 if exists('pause-game-thread') then mark('game-thread-paused')
 elseif os.getenv('NT2_CF04_DELAY_ENTRY')=='1' and exists('delay-first-entry') and not exists('allow-first-entry') then mark('first-entry-paused') else OnWorldPreUpdate();OnWorldPostUpdate() end
 if transport_metrics then
  local elapsed=math.max(0,math.floor((cadence.time()-callback_started)*1000000));callback_max=math.max(callback_max,elapsed);if #callback_samples==256 then table.remove(callback_samples,1)end;callback_samples[#callback_samples+1]=elapsed
  if cadence.time()-measure_at>=1 and measure_image then
   measure_at=cadence.time();local values=ffi.new('uint32_t[24]');local api=measure_image.nt2_g3_private_api();local count=api.metrics(values,24)==1 and 24 or 9;if count==9 then assert(api.metrics(values,9)==1)end
   local fields={};for i=0,count-1 do fields[#fields+1]=tostring(tonumber(values[i]))end;local sorted={};for i,value in ipairs(callback_samples)do sorted[i]=value end;table.sort(sorted);local p95=sorted[math.max(1,math.ceil(#sorted*.95))]or 0
   mark('local-transport-metrics.json',string.format('{"tick_ms":%d,"native":[%s],"callback_p95_us":%d,"callback_max_us":%d,"callback_samples":%d,"lua_heap_kib":%d}',math.floor(cadence.time()*1000),table.concat(fields,','),p95,callback_max,#sorted,math.ceil(collectgarbage('count'))))
  end
 end
 if os.getenv('NT2_C3_CORRECTION')=='1' then
  assert(state.enabled[5] and state.enabled[6] and not state.enabled[7],'preserve original inventory enable flags')
  -- Inject only the eligibility seam; this is not native inventory physics.
  if state.enabled[6] then state.components[6].mInitialized=true;mark('inventory-init-eligible') end
  assert(state.components[11].drinkable==(state.components[1].enabled or next(a2_variables)~=nil),'drinking follows protection; composer owns only its Controls lease; actual='..tostring(state.components[11].drinkable)..';controls='..tostring(state.components[1].enabled)..';composer='..tostring(next(a2_variables)~=nil))
 end
 if c2_fixture and savedGlobals.nt2_rejoin_local_world_v1_offline=='1'then c2_fixture:tick()end
 local now=state.components[1].enabled
 local current_player=polymorph and polymorph:current()or 99
 local renderer=a2_social and a2_social.renderer;local lease=renderer and renderer.lock.lease
 local lock_marker=lease and state.components[lease.marker]
 local lease_owned=lease~=nil and lease.player==current_player
  and lease.controls==EntityGetFirstComponentIncludingDisabled(current_player,'ControlsComponent')
  and lease.damage==EntityGetFirstComponentIncludingDisabled(current_player,'DamageModelComponent')
  and lock_marker~=nil and lock_marker.name=='nt2_a2_composer_lock'and lock_marker.value_bool==true
  and (renderer.model.composer==true or renderer.model.bank_open==true)
 local recovery_marker=state.components[3]
 local protection_present=recovery_marker~=nil and recovery_marker.name=='nt2_g3_recovery'
  or state.effect==true and state.poly==true and state.components[2].wait_for_kill_flag_on_death==true
 -- Acknowledge the control flag from the SAME sample that decides whether to
 -- retain a loss receipt. The driver must see a later frame before injecting
 -- an expected fault or resuming ordinary unexpected-loss monitoring.
 local capture_loss=exists('capture-first-loss')==true
 local permission=permission_observer:sample({frame=state.frame,controls=now,lease_owned=lease_owned,protected=protection_present,native_status=native_status,native_status_frame=native_status_frame},playing)
 held=permission.holds
 if permission.changed then
  if #permission_edges>=32 then table.remove(permission_edges,1)end
  permission_edges[#permission_edges+1]=string.format('{"frame":%d,"tick_ms":%d,"controls_enabled":%s,"input_lease_owned":%s,"input_locked":%s,"native_status":%d,"native_status_frame":%d,"native_protection":%s,"permitted":%s,"holds":%d,"input_locks":%d}',state.frame,tonumber(ffi.C.GetTickCount()),tostring(now),tostring(lease_owned),tostring(permission.typing),native_status,native_status_frame,tostring(protection_present),tostring(permission.permitted),held,permission.input_locks)
  mark('permission-edges.jsonl',table.concat(permission_edges,'\n')..'\n')
 end
 if marker_case and marker_case~='unrelated' and marker_case~='completed-delay' then assert(not now,'partial/malformed NT2 metadata released');mark('partial-protected') end
 if marker_case=='unrelated' and now then mark('unrelated-offline') end
 if now and not playing then assert(exists('start') or marker_case=='unrelated' or savedGlobals.nt2_rejoin_local_world_v1_offline=='1','release before inspected Start');playing=true;mark('playing') end
 if savedGlobals.nt2_rejoin_local_world_v1 then
  mark('saved-association.txt',savedGlobals.nt2_rejoin_local_world_v1)
  mark('saved-seed.txt',savedGlobals.nt2_rejoin_local_world_v1_seed)
  mark('saved-loot.txt',savedGlobals.nt2_rejoin_local_world_v1_loot)
  mark('saved-offline.txt',savedGlobals.nt2_rejoin_local_world_v1_offline or '0')
  serialized_save();mark('associated')
 end
 if held>0 and now then mark('resumed-playing') end
 if playing and not exists('wrong-world-refused') then
  local native=original_load(assert(os.getenv('NT2_C3_DLL_DIR'))..'/nt2_g3_transport.dll')
  local wrong=ffi.new('uint8_t[16]');ffi.fill(wrong,16,9)
  assert(native.nt2_g3_move(wrong,1,0,0,0,0,0)==0 and native.nt2_g3_close(wrong)==0)
  mark('wrong-world-refused')
 end
 if exists('retired-context.hex') and not exists('retired-context-refused') then
  local input=assert(io.open(dir..'/retired-context.hex','rb'));local hex=input:read('*a');input:close();assert(#hex==32 and not hex:find('[^0-9a-f]'))
  local native=original_load(assert(os.getenv('NT2_C3_DLL_DIR'))..'/nt2_g3_transport.dll');local old=ffi.new('uint8_t[16]');for i=0,15 do old[i]=tonumber(hex:sub(i*2+1,i*2+2),16) end
  assert(native.nt2_g3_move(old,2147483647,0,0,0,0,0)==0 and native.nt2_g3_status(old)==0 and native.nt2_g3_close(old)==0,'retired actual context was reused');mark('retired-context-refused')
 end
 if permission.lost then
  if capture_loss and not exists('first-unexpected-hold.json') then
   mark('first-unexpected-hold.tmp',string.format('{"frame":%d,"tick_ms":%d,"last_playing_frame":%d,"last_playing_tick_ms":%d,"holds":%d,"elapsed_ms":%d,"native_status":%d,"native_status_frame":%d,"native_reason":%d,"protected":true}',state.frame,tonumber(ffi.C.GetTickCount()),last_playing_frame,last_playing_tick,held,tonumber(ffi.C.GetTickCount())-began,native_status,native_status_frame,native_reason))
   assert(os.rename(dir..'/first-unexpected-hold.tmp',dir..'/first-unexpected-hold.json'))
  end
 end
 if permission.permitted then last_playing_frame=state.frame;last_playing_tick=tonumber(ffi.C.GetTickCount()) end
 if not now then mark('protected') end
 if exists('expire') and playing and not now then mark('permission-expired-protected') end
 if exists('end') and not now then
  assert(playing and ghosts>0,'must receive actual remote ghost before End');mark('end-protected','holds='..held..';ghosts='..ghosts);break
 end
 if exists('assert-terminal-end') and playing and not now and savedGlobals.nt2_rejoin_local_world_v1_offline~='1' then mark('terminal-protected') end
 if savedGlobals.nt2_rejoin_local_world_v1_offline=='1' and now then mark('offline-restored') end
 if exists('unaffected-check') then mark('unaffected-snapshot','playing='..tostring(permission.permitted)..';holds='..held..';ghosts='..ghosts..';complete=1') end
 if exists('foreground-monitor') then
  if a3_fixture then a3_fixture:snapshot(a2_social and a2_social.a3)end
  if b1_fixture then b1_fixture:snapshot(a2_social and a2_social.bank)end
  if item_fixture then item_fixture:snapshot(a2_social and a2_social.bank)end
  if polymorph then mark('a2-subject.json',string.format('{"id":%d,"moves":%d,"movementSubject":%d,"x":%d,"controls":%s,"wait":%s}',polymorph:current(),observed_moves,last_move_subject,last_move_x,tostring(now),tostring(state.components[2].wait_for_kill_flag_on_death)))end
  local rows={};local total=0;for _,entity in pairs(entities)do if not entity.ghost then local item=state.components[entity.item];total=total+1;rows[#rows+1]=(item.item_name or '')..'\t'..(item.ui_description or '')end end
  local pending=a2_social and a2_social.state.pending
  local saved_dismissal=savedGlobals.nt2_a2_dismissed or '';local saved_count=0;for _ in saved_dismissal:gmatch('[^,]+')do saved_count=saved_count+1 end
  if saved_dismissal~=last_saved_dismissal then serialized_save('a2-dismissal-save.xml');last_saved_dismissal=saved_dismissal end
  local dismissal_scope=a2_social and savedGlobals.nt2_a2_run==a2_social.renderer.model.run or false
  a2_hud.liveRows=0;if a2_social then for _,h in pairs(a2_social.renderer.model.hud)do a2_hud.liveRows=a2_hud.liveRows+1 end end;local hud_fields={};for k,v in pairs(a2_hud)do hud_fields[#hud_fields+1]='"'..k..'":'..v end;table.sort(hud_fields)
  mark('a2-markers.txt',table.concat(rows,'\n'));mark('a2-ui.txt',table.concat(a2_texts,'\n'));mark('a2-players.txt',table.concat(a2_names,'\n'));mark('a2-view.json',string.format('{"markers":%d,"composerLock":%s,"sampoIcons":%d,"radarIcons":%d,"poiPendingPages":%d,"poiPendingRows":%d,"poiPrefixGaps":%d,"poiPrefixRetained":%d,"poiPrefixDiscarded":%d,"poiCommits":%d,"poiPageRejected":%d,"poiFrameReplays":%d,"poiDismissed":%d,"poiSavedDismissed":%d,"poiDismissalScopeMatches":%s,"hpFills":%d,"hpBars":%d,"hpFillMin":%.3f,"hpFillMax":%.3f,"ui":%s,"hud":{%s}}',total,tostring(next(a2_variables)~=nil),a2_sampo,a2_radar,pending and pending.pages or 0,pending and pending.count or 0,a2_pages.gaps,a2_pages.retained,a2_pages.discarded,a2_pages.commits,a2_pages.rejected,a2_pages.replays,a2_social and a2_social.state.dismiss_count or 0,saved_count,tostring(dismissal_scope),a2_fill,a2_bar,a2_min,a2_max,json_text(table.concat(a2_texts,'\n')),table.concat(hud_fields,',')))
  local live=0;for _,entity in pairs(entities)do if entity.ghost then live=live+1 end end
  -- Read the production-created owned marker and its actual protection effects;
  -- the historical playing-to-held transition counter cannot identify a hold.
  local recovery=state.components[3];local flags=recovery and recovery.value_string or ''
  local owned_marker=recovery and recovery.name=='nt2_g3_recovery' and type(flags)=='string' and flags:match('^[01][01][01]%-%-%-$')~=nil or false
  local owned_effect=state.effect==true and state.components[4].frames==-1
  local owned_hold=owned_marker and owned_effect and state.poly==true and state.components[2].wait_for_kill_flag_on_death==true and not now
  local activity=string.format('{"frame":%d,"seed_calls":%d,"tick_ms":%d,"protected":%s,"holds":%d,"ghosts":%d,"hud_button_frames":%d,"hud_content_frames":%d,"native_status":%d,"native_status_frame":%d,"native_context":"%s","ghost_motion_updates":%d,"native_lookup":%d,"native_lookup_frame":%d,"native_lookup_world":"%s","native_lookup_context":"%s","native_owned_hold":%s,"native_hold_marker":%s,"native_hold_effect":%s}',state.frame,seed_calls,math.floor(cadence.time()*1000),tostring(not permission.permitted),held,live,hud_frames,hud_content_frames,native_status,native_status_frame,native_context,ghost_motion_updates,native_lookup,native_lookup_frame,native_lookup_world,native_lookup_context,tostring(owned_hold),tostring(owned_marker),tostring(owned_effect))
  mark('activity.json',activity:sub(1,-2)..',"input_locked":'..tostring(permission.typing)..',"input_locks":'..permission.input_locks..',"controls_enabled":'..tostring(now)..',"loss_monitor_armed":'..tostring(capture_loss)..',"offline_marker":'..tostring(savedGlobals.nt2_rejoin_local_world_v1_offline=='1')..'}')
 end
 if exists('stop') then if os.getenv('NT2_C1_PLAYER_FIXTURE')=='1'then serialized_save()end;break end
 local frame_ms=10;if exists('frame-ms') then local input=assert(io.open(dir..'/frame-ms','rb'));frame_ms=tonumber(input:read('*a'));input:close();assert(frame_ms==17 or frame_ms==33,'bounded foreground cadence only') end
 if frame_ms==10 then ffi.C.Sleep(frame_ms) else cadence.wait(frame_started,frame_ms) end
end
cadence.close()
mark('result','playing='..tostring(playing)..';ghosts='..ghosts..';holds='..held)
assert(exists('stop') or exists('end-protected'),'composed native fixture timeout')
