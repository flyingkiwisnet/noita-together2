-- Production capture/native decoder/consumer/renderer. Engine and DLL facts
-- are injected; these checks do not claim actual Noita gameplay evidence.
local repo=assert(os.getenv('NT2_A2_REPO'));local raw=dofile
dofile=function(p)return raw(repo..'/'..p:gsub('^mods/','mod/'))end
local root='mods/noita_together2/files/g3/';local now=0;local images,texts={},{};local checks=0
local function check(v,message)assert(v,message);checks=checks+1 end
local e={GameGetRealWorldTimeSinceStarted=function()return now end,GuiCreate=function()return 1 end,GuiDestroy=function()end,
 GuiStartFrame=function()images={};texts={}end,GuiZSetForNextWidget=function()end,
 GuiImage=function(_,_,_,_,path,_,scale)images[#images+1]={path=path,scale=scale}end,GuiButton=function(_,_,_,_,label)texts[#texts+1]=label;return false end,
 GuiText=function(_,_,_,s)texts[#texts+1]=s end,EntityGetIsAlive=function(id)return id==99 end,EntityGetTransform=function()return 200,-80 end,
 EntityHasTag=function()return false end,EntityGetAllChildren=function()return {}end,
 GameTextGetTranslatedOrNot=function(key)return ({['$biome_without_icon']='Unlisted Sanctuary',['$biome_coalmine']='Mines',['$nt2_starting_mountain']='Гора'})[key]or key end}
raw(repo..'/apps/desktop/tools/a2-layout-test-api.lua')(e)
e.GuiTooltip=function(_,title,body)texts[#texts+1]=title..'\n'..body end
local frame;local lease=true;local context='c';local generation=1;local terminal;local native_context={};local pointer={};pointer.__add=function(_,offset)return {offset=offset}end
local ffi={new=function()return setmetatable({},pointer)end,copy=function()end,string=function(p,n)if p==native_context then return string.rep(context,16)end;if p.offset then return string.rep(p.offset==40 and 'o'or context,n)end;return terminal or frame end}
local dll={nt2_g3_social_publish=function()return 1 end,nt2_g3_social_next=function()return frame and #frame or 0 end,
 nt2_g3_social_terminal=function()return terminal and #terminal or 0 end,
 nt2_g3_social_lease=function(_,h)if not lease then return 0 end;for i=0,63 do h[i]=0 end;h[24]=generation;h[28]=75;return 64 end}
local adapter=dofile(root..'social_native.lua').new(dll,native_context,ffi,e,dofile(root..'validation.lua'),dofile(root..'a2_renderer.lua'),dofile(root..'hud_poi_local.lua'),dofile(root..'social_dispatch.lua'))
adapter.renderer.model.show_players=true
local function le(n,size)local out={};for _=1,size do out[#out+1]=string.char(n%256);n=math.floor(n/256)end;return table.concat(out)end
local function literal(s)return le(#s,2)..s end
local sequence=0
local function packet(body,count,scope)sequence=sequence+1;return 'NSF1\1\0\0\0'..string.rep(scope or context,16)..le(generation,4)..le(sequence,4)..le(#body,4)..le(count or 1,2)..'\0\0'..body end
local function apply(body,count)frame=packet(body,count);adapter:tick(true,99);frame=nil end
local function roster(run,status,ghost,empty)return string.char(10)..literal(run)..literal('peer')..string.char(empty and 0 or 1)..(empty and ''or literal('peer')..literal('Peer')..string.char(status or 1)..literal(ghost or string.rep('a',32)))end
local function hud(revision,hp,max,flags,biome)return string.char(11)..literal('peer')..le(revision,4)..le(hp or 0,4)..le(max or 0,4)..string.char(flags or 0)..literal(biome or '')end
local function label()return table.concat(texts,'|')end
local function bars()local n=0;for _,im in ipairs(images)do if im.path:find('hpbar_',1,true)then n=n+1 end end;return n end

-- Bytes use the same tag10/length/state/ghost layout as g3_a2_frame::encode.
local local_actor='11111111-1111-4111-8111-111111111111';local remote_actor='22222222-2222-4222-8222-222222222222'
local race={world=string.rep('a',64),status='racing',players={{actor=local_actor,count=2,status='racing'},{actor=remote_actor,count=1,status='racing'}},snapshot={selfRow='local-row',status='racing',config={goals={}},entrants={{rowKey='local-row',progress={}}}},standings={},standingsComplete=true}
adapter.bank.race_view=race;adapter.renderer:set_bank(adapter.bank)
local body=string.char(10)..literal('race-run')..literal(local_actor)..string.char(2)
 ..literal(local_actor)..literal('Local')..string.char(1)..literal(string.rep('a',32))
 ..literal(remote_actor)..literal('Remote')..string.char(1)..literal(string.rep('b',32))
apply(body)
check(adapter.renderer.model.peers[local_actor]and adapter.renderer.model.peers[remote_actor],'actual frame decode/dispatch retains both canonical actors')
check(label():find(' Local [2]',1,true)and label():find(' Remote [1]',1,true),'Race frame reaches production player-list drawing with local/remote counts: '..label())
adapter:tick(false,99);check(label():find(' Local [2]',1,true)and label():find(' Remote [1]',1,true),'protected pause retains the previously authenticated roster')
adapter.renderer.model.show_race=false;adapter:tick(true,99);check(label():find(' Remote [1]',1,true),'hiding goals preserves populated native-dispatched Race list')
adapter.renderer.model.show_players=false;adapter.renderer.model.show_race=true;adapter:tick(true,99);check(not label():find(' Remote [1]',1,true),'independent player visibility does not alter stored canonical roster')
check(adapter.renderer.model.peers[local_actor]and adapter.renderer.model.peers[remote_actor],'hiding list never removes actors')
print('PASS Race actual Lua social frame/dispatch/list '..checks..' assertions; synthetic native bytes/lease and roster facts, no actual Noita/service')
