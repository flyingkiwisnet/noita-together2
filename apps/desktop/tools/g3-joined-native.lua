-- Two real x86 workers/pipes. Engine observations/APIs/profile are simulated.
local ffi=require('ffi')
io.stdout:setvbuf('no')
ffi.cdef[[void __stdcall Sleep(unsigned long);
int __stdcall SetEnvironmentVariableA(const char*,const char*);
int nt2_g3_prepare(uint32_t*,uint8_t*);int nt2_g3_seed_accepted(const uint8_t*,uint32_t);
int nt2_g3_world_loaded(const uint8_t*);int nt2_g3_status(const uint8_t*);
int nt2_g3_heartbeat_diagnostic(const uint8_t*);
int nt2_g3_report(const uint8_t*,const uint8_t*,uint32_t,uint64_t);
int nt2_g3_move(const uint8_t*,uint32_t,int32_t,int32_t,int32_t,int32_t,uint32_t);
int nt2_g3_peers(const uint8_t*,uint8_t*,uint32_t);int nt2_g3_close(const uint8_t*);]]
local dll=ffi.load(assert(os.getenv('NT2_G3_JOINED_DLL')))
local dir=assert(os.getenv('NT2_G3_JOINED_DIR'))
local function mark(name,value) local f=assert(io.open(dir..'/'..name,'wb'));f:write(value or 'fixture');f:close() end
local function exists(name) local f=io.open(dir..'/'..name,'rb');if f then f:close();return true end end
local function wait(fn) for i=1,1200 do if fn() then return end;ffi.C.Sleep(10) end;error('joined native fixture timeout') end
local ctx=ffi.new('uint8_t[16]');local seed=ffi.new('uint32_t[1]')
assert(dll.nt2_g3_prepare(seed,ctx)==1 and seed[0]==424242)
assert(dll.nt2_g3_seed_accepted(ctx,seed[0])==1 and dll.nt2_g3_world_loaded(ctx)==1)
assert(dll.nt2_g3_move(ctx,1,1000,2000,0,0,0)==0)
local function report(held,revision)
 local body='{"actualSeed":"424242","mods":["noita_together2"],"complete":true,"error":null,"beta":null,"loaded":true,"protected":'..tostring(held)..',"modVersion":"g3-simulated","bridgeVersion":"g3-simulated"}'
 if os.getenv('NT2_G3_PRIMARY_SOURCE')=='1' then body=body:gsub('"modVersion":"g3%-simulated"','"modVersion":"nt2-g3-dev-1"'):gsub('"bridgeVersion":"g3%-simulated"','"bridgeVersion":"g3-transport-1"') end
 if os.getenv('NT2_G3_SOCIAL_SOURCE')=='1' then body=body:gsub('"modVersion":"g3%-simulated"','"modVersion":"nt2-g3-social-dev-1"'):gsub('"bridgeVersion":"g3%-simulated"','"bridgeVersion":"g3-social-transport-1"') end
 if os.getenv('NT2_G3_SOCIAL_NATIVE')=='1' then assert(dll.nt2_g3_report(ctx,ffi.cast('const uint8_t*',body),#body,revision)==0) end
 wait(function() return dll.nt2_g3_report(ctx,ffi.cast('const uint8_t*',body),#body,revision)==1 end)
 if os.getenv('NT2_G3_SOCIAL_NATIVE')=='1' then local changed=body..' ';assert(dll.nt2_g3_report(ctx,ffi.cast('const uint8_t*',changed),#changed,revision)==0) end
end
local phase
if os.getenv('NT2_G3_RUNTIME_STABILITY_TEST')=='1' then
 phase=dofile(assert(os.getenv('NT2_G3_RUNTIME_HARNESS')))(dll,ctx,assert(os.getenv('NT2_G3_MODULES')))
 wait(function() phase:tick();return phase:acknowledged(false) end)
else report(true,1) end
mark('protected');wait(function() return exists('start') end)
if phase then wait(function() phase:tick();return phase:acknowledged(true) end) else report(false,2) end
mark('playing');wait(function() return exists('pump') end)
local wrong=ffi.new('uint8_t[16]');ffi.fill(wrong,16,9)
assert(dll.nt2_g3_move(wrong,1,0,0,0,0,0)==0 and dll.nt2_g3_close(wrong)==0)
local root=assert(os.getenv('NT2_G3_MODULES'))
local function load(name) return dofile(root..'/'..name..'.lua') end
local social
if os.getenv('NT2_G3_SOCIAL_NATIVE')=='1' then
 ffi.cdef[[unsigned long __stdcall GetTickCount();int nt2_g3_social_terminal(const uint8_t*,uint8_t*,uint32_t);int nt2_g3_social_lease(const uint8_t*,uint8_t*,uint32_t);int nt2_g3_social_next(const uint8_t*,uint8_t*,uint32_t);]]
 local owned={};local created=0
 local engine={GameGetRealWorldTimeSinceStarted=function() return tonumber(ffi.C.GetTickCount())/1000 end,
  EntityLoad=function(path,x,y) assert(path=='mods/noita_together2/files/g3/poi.xml');created=created+1;owned[created]={};return created end,
  EntityGetIsAlive=function(id) return owned[id]~=nil end,EntityKill=function(id) owned[id]=nil end,
  EntityGetFirstComponentIncludingDisabled=function(id,kind) assert(kind=='ItemComponent');return id end,ComponentSetValue2=function(id,key,value) owned[id][key]=value end,
  GuiCreate=function() return 1 end,GuiDestroy=function() end,GuiStartFrame=function() end,GuiButton=function() return false end,GuiText=function() end,GuiImage=function() end}
 social=load('social_native').new(dll,ctx,ffi,engine,load('validation'),load('social_game_api'),load('hud_poi_local'),load('social_dispatch'))
 social.fixture_owned=owned
 social.renderer.show_players=true;social.renderer.show_pois=true
end
local entities={};local count=0;local removed=0;local timer=0
local api={movement=function() return {alive=function() return true end,time=function() return timer end,
 position=function() return tonumber(os.getenv('NT2_G3_JOINED_X')),2,0,1,0,0 end} end,
 ghosts=function() return {time=function() return timer end,alive=function(id) return entities[id]~=nil end,
 spawn=function(x,y) count=count+1;entities[count]={x=x,y=y};return count end,
 remove=function(id) entities[id]=nil;removed=removed+1 end,position=function(id,x,y) assert(entities[id]);entities[id]={x=x,y=y} end} end}
local owner=load('native_owner').new(dll,ctx,ffi,api,load('validation'),load('movement'),load('ghosts'))
local reply=ffi.new('uint8_t[864]');local seen=false
local raced,race_off=false,false
for frame=0,(os.getenv('NT2_G3_MOVEMENT_GRANT_TEST')=='1' and 6000 or 1800) do
 if not raced and exists('heartbeat-race') then assert(ffi.C.SetEnvironmentVariableA('NT2_HEARTBEAT_CONNECT_RACE','1')~=0);raced=true end
 if raced and not race_off and exists('heartbeat-race-off') then assert(ffi.C.SetEnvironmentVariableA('NT2_HEARTBEAT_CONNECT_RACE',nil)~=0);race_off=true end
 timer=frame/100;if phase then phase:tick();mark('runtime-phase-result','holds='..phase.holds..';ticks='..phase.ticks) end;owner:tick(1,frame,not phase or phase:playing())
 if phase then local active=0;for _ in pairs(entities) do active=active+1 end;mark('runtime-ghost-result','active='..active..';removed='..removed) end
 if social then
  social:tick(not phase or phase:playing())
  for _,hud in pairs(social.renderer.hud) do if hud.hp==5000 and not exists('social-native-refresh') then mark('social-native-refresh','second fresh canonical HUD sequence through actual stream') end end
  local marker_count=0;for _ in pairs(social.state.markers) do marker_count=marker_count+1 end
  local entity_count=0;for _ in pairs(social.renderer.entities) do entity_count=entity_count+1 end
  if marker_count==256 and entity_count==256 and not exists('social-native-max') then mark('social-native-max','synthetic maximum Unicode256-marker capture drained through real pipe/worker/game thread') end
  if not next(social.renderer.hud) and not next(social.renderer.peers) and exists('social-native-max') then
   for sequence,poi in pairs(social.renderer.pois) do
    local entity=social.renderer.entities[sequence]
    if poi.text=='literal 雪 <img onerror=literal>' and entity and social.fixture_owned[entity].item_name==poi.name and not exists('social-native-historical') then mark('social-native-historical','canonical expired-latest removal and retained private Marker.name without live peer') end
   end
  end
  if next(social.renderer.hud) and next(social.renderer.pois) and not exists('social-native') then
   local _,hud=next(social.renderer.hud);local _,poi=next(social.renderer.pois)
   assert(hud.hp==3000 and hud.max==2000 and poi.text=='literal 雪 <img onerror=literal>' and poi.name)
   local f=assert(io.open(dir..'/social-native-frame.bin','wb'));f:write(assert(social.last_frame));f:close()
   mark('social-native','actual Core/pipe/worker/lease/NSF1/game-thread HUD and private-name POI')
  end
 end
 if not seen and count>0 then
  assert(dll.nt2_g3_peers(ctx,reply,864)==864 and reply[6]>0)
  local f=assert(io.open(dir..'/nonempty-reply.bin','wb'));f:write(ffi.string(reply,864));f:close()
  seen=true;mark('ghost',tostring(entities[count].x))
  assert(dll.nt2_g3_move(ctx,1,0,0,0,0,0)==0) -- Existing sequence cannot replay.
 end
 if exists('hold') then
  owner:tick(1,frame,false);if social then social:tick(false);assert(not next(social.renderer.hud) and not next(social.renderer.pois)) end;assert(next(entities)==nil);mark('cleared')
  wait(function() if social then social:tick(false) end;return exists('end') end)
  wait(function() if social then social:tick(false) end;return dll.nt2_g3_status(ctx)~=2 end)
  if social and exists('social-native') then wait(function() social:tick(false);return social.ended end) end
  assert(dll.nt2_g3_move(ctx,4294967295,0,0,0,0,0)==0)
  owner:tick(1,frame+1,true);assert(next(entities)==nil) -- Old reply cannot revive after End.
  if social then social:tick(true);assert(not next(social.renderer.hud) and not next(social.renderer.pois));social:close() end
  if social and exists('social-native') then assert(social.ended,'Actual native terminal must be observed');mark('social-native-terminal','control-only End clears private state without gameplay authority') end
  if phase then mark('runtime-phase-result','holds='..phase.holds..';ticks='..phase.ticks) end
  owner:close();assert(dll.nt2_g3_close(ctx)==1);mark('closed');return
 end
 ffi.C.Sleep(10)
end
error('joined gameplay deadline')
