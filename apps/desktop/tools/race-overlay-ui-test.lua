-- Actual renderer/player-list/panel; GUI widgets and canonical inputs are injected.
local repo=assert(os.getenv('NT2_A2_REPO'));local raw=dofile
dofile=function(p)return raw(repo..'/'..p:gsub('^mods/','mod/'))end
local checks=0;local function check(v,s)assert(v,s);checks=checks+1 end
local draw,containers,click={}, {},nil;local tips={};local alpha,option=1,nil;local width,height=427,240;local globals={}
local e={GUI_OPTION={NoPositionTween=6,Layout_NextSameLine=14,DrawSemiTransparent=26},
 GameGetRealWorldTimeSinceStarted=function()return 2 end,GlobalsGetValue=function(k,d)return globals[k]or d end,GlobalsSetValue=function(k,v)globals[k]=v end,
 EntityGetIsAlive=function()return true end,EntityGetTransform=function()return 0,0 end,
 GuiCreate=function()return 1 end,GuiDestroy=function()end,GuiGetScreenDimensions=function()return width,height end,
 GuiStartFrame=function()draw={};containers={};tips={};alpha=1;option=nil end,GuiOptionsAdd=function()end,
 GuiColorSetForNextWidget=function(_,r,g,b,a)alpha=a end,GuiOptionsAddForNextWidget=function(_,v)option=v end,
 GuiGetTextDimensions=function(_,s)return #s*3,9 end,GuiImage=function()end,GuiTooltip=function(_,title,body)tips[title]=body end,
 GuiLayoutBeginVertical=function()end,GuiLayoutEnd=function()end,GuiLayoutAddVerticalSpacing=function()end}
local function widget(label,x,y)
 draw[#draw+1]={label=label,x=x,y=y,alpha=alpha};alpha=1;option=nil
 if label==click then click=nil;return true end;return false
end
e.GuiText=function(_,x,y,s)widget(s,x,y)end;e.GuiButton=function(_,_,x,y,s)return widget(s,x,y)end
e.GuiImageButton=function(_,_,x,y,_,p)return widget(p:match('player_list.png$')and'Players'or p,x,y)end
e.GuiBeginScrollContainer=function(_,widget_id,x,y,w,h,focus,mx,my)containers[#containers+1]={id=widget_id,x=x,y=y,w=w,h=h,alpha=alpha,option=option,focus=focus,mx=mx,my=my};alpha=1;option=nil end
e.GuiEndScrollContainer=function()end
local root='mods/noita_together2/files/g3/';local r=dofile(root..'a2_renderer.lua').new(e,dofile(root..'validation.lua'))
local function find(s)for _,w in ipairs(draw)do if w.label==s then return w end end end
local function tick(label,playing)click=label;r:tick(playing~=false,99)end
r:roster('run','one',{{actor='one',name='Alex',status=1,ghost=''},{actor='two',name='Sam',status=3,ghost=''}})
tick();check(find('Players').x==140 and find('Players').y==0,'Core keeps existing top-row Players icon')
tick('Players');check(r.model.show_players and find(' Alex')and find(' Sam'),'Core existing roster/list still works')
local race={world=string.rep('a',64),status='racing',players={{actor='one',count=2,status='racing'},{actor='two',count=1,status='dead'}},snapshot={selfRow='row-one',status='racing',elapsedMs='1000',config={goals={{id='reach_max_hp',instanceId='hp',target=500}},durationMs=0},entrants={{rowKey='row-one',status='racing',progress={}}}},standings={},standingsComplete=true}
r:set_bank({ui={open=false},race_view=race,pause=function()end});r.model.show_players=false
tick();check(find('Show players')and find('Show players').y==36 and find('Hide goals'),'Race has obvious independent player toggle below toolbar')
check(#containers==1 and containers[1].option==26 and containers[1].alpha==0.22,'goals use documented DrawSemiTransparent on container, not false background argument')
check(containers[1].focus==false and containers[1].mx==1 and containers[1].my==1,'scroll arguments remain gamepad focus and margins')
for _,w in ipairs(draw)do check(w.alpha==1,'text/buttons remain opaque')end
tick('Show players');check(r.model.show_players and r.model.show_race~=false and find(' Alex [2]')and find(' Sam [1]'),'Race list populated from existing roster + canonical actor counts')
check(#containers==2 and containers[1].x>=115 and containers[2].x==5,'goals coexist beside legacy player list')
check(tips[' Alex']:find('Race racing',1,true)and tips[' Sam']:find('Race dead',1,true),'existing player list exposes canonical Race status without fake HP')
check(tips[' Alex']:find('HP unknown',1,true),'Race roster without HUD retains honest unknown HP')
local list_id=containers[2].id;local goals_id=containers[1].id
tick('Hide goals');check(r.model.show_race==false and r.model.show_players and#containers==1 and find(' Alex [2]'),'hiding goals does not hide player list')
check(containers[1].id==list_id and containers[1].id~=goals_id,'player container retains identity instead of inheriting goals scroll cache')
tick('Show goals',false);check(r.model.show_race and r.model.show_players and#containers==2,'held Race keeps independently selectable goals/list')
width=240;tick();check(containers[1].x>=115 and containers[1].x+containers[1].w<=width-5,'narrow Race goals leave clearance for player list')
tick('Hide players');check(not r.model.show_players and r.model.show_race and#containers==1,'hiding players leaves goals visible')
r:close(true);tick('Show players',false);check(find(' Alex [2]')and find(' Sam [1]'),'terminal Race preserves last known roster instead of dropping list')
r:roster('run','one',{});tick(nil,false);check(find('Player roster unavailable'),'missing canonical roster is explicit, names/counts never fabricated')
r:set_bank(nil);tick();check(find('Players')and not find('Show players'),'Core control restored after Race view leaves')
print('PASS Race overlay/player list '..checks..' assertions; actual renderer modules, injected GUI/roster facts, no engine pixels or gameplay authority')
