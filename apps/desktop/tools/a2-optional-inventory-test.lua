-- Actual fixture-only engine view + production capture/publisher/renderer.
-- Engine facts are injected. No native/service/Noita claim.
local repo=assert(os.getenv('NT2_A2_REPO'));local raw=dofile
dofile=function(path)return raw(repo..'/'..path:gsub('^mods/','mod/'))end
local root='mods/noita_together2/files/g3/';local checks=0
local function check(v,message)assert(v,message);checks=checks+1 end
local unavailable=false;local has_sampo=true;local hp=4;local now=0;local calls=0
local children=function(id)calls=calls+1;return id==99 and {101}or {}end
local e={EntityGetAllChildren=children,EntityGetIsAlive=function(id)return id==99 end,
 EntityGetFirstComponentIncludingDisabled=function()return 2 end,
 EntityHasTag=function(id,tag)return has_sampo and id==101 and tag=='this_is_sampo'end,
 ComponentGetValue2=function(_,key)if key=='hp'then return hp elseif key=='max_hp'then return 8 end end,
 EntityGetTransform=function()return 10,20 end,BiomeMapGetName=function()return '$biome_coalmine'end,
 GameGetRealWorldTimeSinceStarted=function()return now end}
local proxy=raw(repo..'/apps/desktop/tools/lifecycle-lc1-optional-inventory.lua').capture_engine(e,function()return unavailable end)
local r=dofile(root..'a2_renderer.lua').new(e,dofile(root..'validation.lua'));r:roster('run','me',{{actor='me',name='Me',status=1}})
local samples={};local capture=dofile(root..'a2_capture.lua');local publisher=capture.publisher(proxy,function(v)
 samples[#samples+1]=v;r:receive_info({actor='me',revision=#samples,hp=v.hp and v.hp*1000,max_hp=v.max_hp and v.max_hp*1000,sampo=v.sampo,biome=v.biome});return true
end)
publisher:tick(99,true);check(samples[1].sampo==true and r.model.hud.me.sampo==true,'valid Sampo observed through actual capture')
unavailable=true;hp=0;now=1;publisher:tick(99,true)
check(samples[2].sampo==nil,'optional unavailable observation emits no invented Sampo absence')
check(samples[2].hp==0 and samples[2].max_hp==8 and samples[2].biome=='$biome_coalmine','other optional observations still progress, including valid zero HP')
check(r.model.hud.me.sampo==true and r.model.hud.me.hp==0,'missing Sampo retains known true while new zero HP replaces')
check(e.EntityGetAllChildren==children,'global gameplay/protection engine API is unchanged')
local before=calls;check(e.EntityGetAllChildren(99)[1]==101 and e.EntityGetAllChildren(101)[1]==nil,'ordinary inventory traversal stays available while optional fault is active')
check(calls==before+2,'ordinary callers reach original engine API')
check(proxy.EntityGetAllChildren(99)==nil,'fault remains confined to capture engine view')
unavailable=false;has_sampo=false;now=2;publisher:tick(99,true)
check(samples[3].sampo==false and r.model.hud.me.sampo==false,'new valid absence replaces retained Sampo true')
check(e.EntityGetAllChildren(99)[1]==101,'global API stays intact after optional recovery')
print('PASS optional inventory isolation '..checks..' assertions; production capture/publisher/renderer; engine facts injected')
