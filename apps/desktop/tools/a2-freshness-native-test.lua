-- Real wrappers/broker and production capture/decoder/consumer/renderer.
-- Engine facts and service responses are injected; no actual Noita is launched.
local ffi=require('ffi');ffi.cdef[[
void __stdcall Sleep(unsigned long);unsigned long __stdcall GetTickCount(void);
int nt2_g3_resume(const uint8_t*,const uint8_t*,uint32_t,uint8_t*);
int nt2_g3_world_loaded(const uint8_t*);int nt2_g3_report(const uint8_t*,const uint8_t*,uint32_t,uint64_t);
int nt2_g3_social_lease(const uint8_t*,uint8_t*,uint32_t);int nt2_g3_social_next(const uint8_t*,uint8_t*,uint32_t);
int nt2_g3_social_terminal(const uint8_t*,uint8_t*,uint32_t);int nt2_g3_social_publish(const uint8_t*,const uint8_t*,uint32_t);
]]
local repo=assert(os.getenv('NT2_A2_REPO'));local own=assert(os.getenv('NT2_A2_NATIVE_DIR'));local dir=assert(os.getenv('NT2_A2_FIXTURE_DIR'));local raw=dofile
dofile=function(p)return raw(repo..'/'..p:gsub('^mods/','mod/'))end
local root='mods/noita_together2/files/g3/';local primary=ffi.load(own..'/nt2_g3_transport.dll');local social=ffi.load(own..'/nt2_g3_social.dll')
local began=tonumber(ffi.C.GetTickCount());local function now()return (tonumber(ffi.C.GetTickCount())-began)/1000 end
local e={GameGetRealWorldTimeSinceStarted=now,GuiCreate=function()return 1 end,GuiDestroy=function()end,GuiStartFrame=function()end,
 GuiText=function()end,GuiButton=function()return false end,GuiImage=function()end,GuiZSetForNextWidget=function()end,
 EntityGetIsAlive=function(id)return id==99 end,EntityHasTag=function()return false end,EntityGetTransform=function()return 500,1200 end,
 EntityGetFirstComponentIncludingDisabled=function()return 2 end,ComponentGetValue2=function(_,key)return key=='hp'and 4 or key=='max_hp'and 8 end,
 EntityGetAllChildren=function()return {}end,BiomeMapGetName=function(x,y)assert(x==500 and y==1200);return '$biome_coalmine'end,
 GameTextGetTranslatedOrNot=function()return 'Mines'end}
local function mark(name)local f=assert(io.open(dir..'/'..name,'wb'));f:write('fixture');f:close()end
local a=ffi.new('uint8_t[16]');ffi.fill(a,16,1);local c=ffi.new('uint8_t[16]');local peer=ffi.new('uint8_t[16]');local joined=false;local adapter
local first_sequence;local blocked=false;local preserved=false;local calls=0
local module=dofile(root..'social_native.lua')
local dispatch=dofile(root..'social_dispatch.lua');local observed_dispatch={decode=dispatch.decode,new=function(authority,apply,pause)return dispatch.new(authority,function(...)
 local ok,value=pcall(apply,...);if not ok then print('FIXTURE_APPLY_ERROR '..tostring(value));error(value)end;return value
end,pause)end}
while now()<9 do
 if primary.nt2_g3_resume(a,'424242',6,c)==1 then
  if not joined and social.nt2_g3_resume(a,'424242',6,peer)==1 then
   adapter=module.new(social,c,ffi,e,dofile(root..'validation.lua'),dofile(root..'a2_renderer.lua'),dofile(root..'hud_poi_local.lua'),observed_dispatch);joined=true
  end
  primary.nt2_g3_world_loaded(c);primary.nt2_g3_report(c,'{}',2,1)
  if adapter then
   adapter:tick(true,99);calls=calls+1
   local actor=adapter.renderer.local_actor;local h=actor and adapter.renderer.model.hud[actor]
   if h and not blocked then
    first_sequence=h.revision;mark('block-ready');blocked=true;ffi.C.Sleep(300)
   end
   local batch=adapter.last_frame and dofile(root..'social_dispatch.lua').decode(adapter.last_frame)
   if blocked and not preserved and batch and #batch.commands==1 and batch.commands[1].tag==5 then
    assert(h and h.revision==first_sequence,'optional Pause discarded retained HUD')
    assert(not adapter.state.playing and not adapter.state.pending,'Pause must revoke application gate and discard historical prefix')
    preserved=true;mark('pause-preserved')
   end
   if preserved and h and h.revision>first_sequence then
    assert(adapter.capture_diagnostics.accepted>=2,'actual independent Lua producer did not progress')
    mark('fresh-returned');print('PASS actual broker/pipe/capture/read/decode/render; bounded300ms game-thread stall, retained canonical revision, fresh replacement; callbacks '..calls);return
   end
  end
 end
 ffi.C.Sleep(5)
end
local d=adapter and adapter.diagnostics or {};local hs=0;if adapter then for _ in pairs(adapter.renderer.model.hud)do hs=hs+1 end end
error('bounded scheduling fixture timeout; blocked='..tostring(blocked)..'; preserved='..tostring(preserved)..'; frames='..tostring(d.frames)..'; invalid='..tostring(d.invalid)..'; applied='..tostring(d.applied)..'; received='..tostring(d.received)..'; local='..tostring(adapter and adapter.renderer.local_actor~=nil)..'; hud_rows='..hs)
