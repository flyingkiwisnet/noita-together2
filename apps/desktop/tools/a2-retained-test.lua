-- Production capture/native decoder/consumer/renderer. Engine and DLL facts
-- are injected; these checks do not claim actual Noita gameplay evidence.
local repo=assert(os.getenv('NT2_A2_REPO'));local raw=dofile
dofile=function(p)return raw(repo..'/'..p:gsub('^mods/','mod/'))end
local root='mods/noita_together2/files/g3/';local now=0;local images,texts={},{};local checks=0
local function check(v,message)assert(v,message);checks=checks+1 end
local e={GameGetRealWorldTimeSinceStarted=function()return now end,GuiCreate=function()return 1 end,GuiDestroy=function()end,
 GuiStartFrame=function()images={};texts={}end,GuiZSetForNextWidget=function()end,
 GuiImage=function(_,_,_,_,path,_,scale)images[#images+1]={path=path,scale=scale}end,GuiButton=function()return false end,
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
apply(roster('retained-run'));check(label():find('HP unknown',1,true)and bars()==0,'never-received information is unknown with no invented bar')
apply(hud(10,1000,2000,3,'$biome_without_icon'));check(label():find('HP 25.0 / 50.0',1,true)and bars()==2,'initial valid half-health state')
check(label():find('Unlisted Sanctuary',1,true),'named location text must not depend on icon catalogue membership')
for _,at in ipairs({2.1,5.1,60,3600})do now=at;lease=false;adapter:tick(true,99);check(label():find('HP 25.0 / 50.0',1,true)and bars()==2,'connected state persists beyond2/5seconds and long optional gaps');check(not adapter.state.playing,'cached HUD grants no packet authority')end
adapter:tick(false,99);check(label():find('HP 25.0 / 50.0',1,true),'local pause/protection does not manufacture actor departure')
lease=true;apply(string.char(5));check(label():find('HP 25.0 / 50.0',1,true)and not adapter.state.playing and not adapter.state.pending,'optional abort retains information but drops prefix and packet gate')
apply(hud(9,2000,2000,7,'$biome_coalmine'));check(label():find('HP 25.0 / 50.0',1,true),'late lower state revision cannot replace newer value')
apply(hud(11,0,2000,3,'_EMPTY_'));check(label():find('HP 0.0 / 50.0',1,true)and bars()==1,'actual zero health replaces old value and retains red background')
check(label():find('Unknown location',1,true),'observed unnamed location clears previous named location')
apply(hud(12,0,0,0,''));check(label():find('HP 0.0 / 50.0',1,true),'missing observations retain known HP/current-max/Sampo/location')
apply(hud(12,2000,2000,7,'$biome_coalmine'));check(label():find('HP 0.0 / 50.0',1,true),'substituted duplicate revision is ignored')
apply(hud(13,2000,2000,7,'$biome_coalmine'));check(label():find('HP 50.0 / 50.0',1,true)and label():find('Mines',1,true),'newer valid values replace all observed fields')
apply(roster('retained-run',2));check(label():find('HP 50.0 / 50.0',1,true)and label():find('waiting',1,true),'authorized waiting roster retains information')
apply(roster('retained-run',3));check(not adapter.renderer.model.hud.peer and label():find('disconnected',1,true),'canonical disconnected actor is not presented as current HP')
apply(hud(14,1000,2000,3,'$biome_coalmine'));check(not adapter.renderer.model.hud.peer,'disconnected actor cannot accept a retained update')
apply(roster('retained-run',1,string.rep('b',32)));apply(hud(13,2000,2000,7,'$biome_coalmine'));check(label():find('HP 50.0 / 50.0',1,true),'same-world session replacement can recover unchanged canonical state revision')
apply(hud(1,0,2000,3,''));check(label():find('HP 50.0 / 50.0',1,true),'session replacement does not reset world state revision floor')
apply(roster('retained-run',1,nil,true));apply(hud(15,1000,2000,3,''));check(not adapter.renderer.model.hud.peer and not adapter.renderer.peers.peer,'actual roster removal cannot be undone by a late HUD packet')
local old=packet(roster('retained-run')..hud(16,1000,2000,3,''),2);context='d';apply(roster('other-run'));frame=old;adapter:tick(true,99);frame=nil;check(not adapter.renderer.model.hud.peer,'old native scope cannot repopulate a different world')
apply(hud(1,1000,2000,3,'$nt2_starting_mountain'));check(label():find('Гора',1,true),'starting Mountain display uses localization lookup')
terminal=packet(string.char(6));adapter:tick(false,99);terminal=nil;check(adapter.ended and not next(adapter.renderer.model.hud)and not next(adapter.renderer.peers),'terminal control removes retained state while protected')

local capture=dofile(root..'a2_capture.lua');local biome='_EMPTY_';local file='data/biome/mountain_left_entrance.xml'
e.BiomeMapGetName=function(x,y)check(x==200 and y==-80,'biome query uses actual player position');return biome end
e.DebugBiomeMapGetFilename=function(x,y)check(x==200 and y==-80,'starting signal uses actual player position');return file end
check(capture.sample(e,99).biome=='$nt2_starting_mountain','exact observed entrance/tutorial biome permits narrow fallback')
for _,other in ipairs({'data/biome/hills.xml','data/biome/mountain_top.xml','data/biome/mountain_hall.xml','mods/example/unnamed.xml'})do file=other;check(capture.sample(e,99).biome=='_EMPTY_','other unnamed locations do not become Mountain')end
file='data/biome/mountain_left_entrance.xml';biome='$biome_coalmine';check(capture.sample(e,99).biome==biome,'named biome overrides starting fallback')
file='mods/example/unnamed.xml';biome=nil;check(capture.sample(e,99).biome==nil,'unavailable observation stays absent and retains previous location')
local translations=',en,ru,pt-br,es-es,de,fr-fr,it,pl,zh-cn,jp,ko\n';e.ModTextFileGetContent=function()return translations end;e.ModTextFileSetContent=function(_,value)translations=value end
local biomes=dofile(root..'a2_biomes.lua');check(biomes.register(e)and biomes.register(e),'localized label registration is idempotent');local _,n=translations:gsub('nt2_starting_mountain,','');check(n==1 and translations:find('Mountain,Гора',1,true),'one localization row includes supported language columns')
print('PASS retained Client '..checks..' assertions: long connected gaps, zero/unknown, revision/session/world/roster fences, pause/End, named-without-icon and narrow localized starting Mountain; engine/native facts injected')
