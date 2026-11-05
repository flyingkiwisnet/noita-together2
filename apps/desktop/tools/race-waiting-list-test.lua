-- Existing room projection -> private Race pages -> actual bank merge/list renderer.
local repo=assert(os.getenv('NT2_A2_REPO'));local raw=dofile
dofile=function(p)return raw(repo..'/'..p:gsub('^mods/','mod/'))end
local root='mods/noita_together2/files/g3/';local checks=0;local function check(v,s)assert(v,s);checks=checks+1 end
local now=1;local labels={};local clicks=0;local previews=0
local e={GameGetRealWorldTimeSinceStarted=function()return now end,GuiCreate=function()return 1 end,GuiDestroy=function()end,GUI_OPTION={NoPositionTween=6,DrawSemiTransparent=26},
 GuiStartFrame=function()labels={}end,GuiOptionsAdd=function()end,GuiColorSetForNextWidget=function()end,GuiGetTextDimensions=function(_,s)return #s*3,9 end,GuiTooltip=function()end,
 GuiGetScreenDimensions=function()return 427,240 end,GuiText=function(_,_,_,s)labels[#labels+1]=s end,GuiImage=function()end,GuiButton=function(_,_,_,_,s)labels[#labels+1]=s;return s:match('^ ')~=nil,true end,
 EntityGetIsAlive=function()return true end,EntityGetTransform=function()return 0,0 end,GlobalsGetValue=function(_,d)return d end}
raw(repo..'/apps/desktop/tools/a2-layout-test-api.lua')(e)
local bank={ui={open=false},pause=function()end,items={player_row=function()previews=previews+1 end}};local receive=dofile(root..'bank_native.lua').receive_race
local renderer=dofile(root..'a2_renderer.lua').new(e,dofile(root..'validation.lua'));renderer:set_bank(bank);renderer.model.show_players=true
local function page(revision,members,total,offset,scope)return {world='world',lobbyOnly=true,lobbyScope=scope or'room/run/epoch/1',lobbyRevision=tostring(revision),lobbyMembers=members,lobbyTotal=total,lobbyOffset=offset or 0}end
local localrow={desktop='11111111-1111-4111-8111-111111111111',name='Local waiting',status='waiting'};local remote={desktop='22222222-2222-4222-8222-222222222222',name='Remote offline',status='disconnected'}
receive(bank,e,{world='world',lobbyAvailable=true,snapshot={config={goals={}},status='waiting',entrants={}},standings={},standingsComplete=true})
receive(bank,e,page(1,{localrow,remote},2));renderer:tick(false,99)
local function shown(s)return table.concat(labels,'|'):find(s,1,true)~=nil end
check(shown(' Local waiting')and shown(' Remote offline'),'both authenticated room members render before Start without gameplay roster')
check(not next(renderer.model.peers),'display members never become gameplay actors')
check(previews==0 and not next(renderer.model.follow),'readonly rows cannot target follow or wand requests')
check(bank.race_lobby.rows[1].hud==nil and bank.race_lobby.rows[1].actor==nil,'waiting HP/world actor remain unknown')
receive(bank,e,page(2,{localrow},1));renderer:tick(false,99);check(shown(' Local waiting')and not shown(' Remote offline'),'new complete room revision removes departed membership')
receive(bank,e,page(1,{localrow,remote},2));renderer:tick(false,99);check(not shown(' Remote offline'),'late old room revision cannot restore removed row')
receive(bank,e,page(3,{remote},2,0,'new-room/run/epoch/1'));renderer:tick(false,99);check(not shown(' Local waiting'),'new scope never displays preceding room while pages incomplete')
receive(bank,e,page(3,{localrow,remote},2,0,'new-room/run/epoch/1'));now=5;renderer:tick(false,99);check(shown('Player roster unavailable'),'expired local display receipt is unavailable, not fresh')
now=1;receive(bank,e,page(4,{localrow,remote},2,0));renderer:roster('live', 'actor-local',{{actor='actor-local',name='Actual Local',status=1,ghost=''}});receive(bank,e,{world='world',lobbyAvailable=false,snapshot={config={goals={}},status='racing',entrants={}}});renderer:tick(true,99)
check(shown(' Actual Local')and not shown(' Local waiting'),'after Start existing gameplay roster takes over without name/account matching')
print('PASS Race protected lobby list '..checks..' assertions; actual merge/renderer, synthetic authenticated room/native pages; no gameplay identity/grant')
