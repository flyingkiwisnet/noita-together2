-- Production renderer calls; Noita GUI/input/viewport facts are injected.
-- Tests requested coordinates and frame options, not actual engine pixels/tweens.
local repo=assert(os.getenv('NT2_A2_REPO'));local raw=dofile
dofile=function(path)return raw(repo..'/'..path:gsub('^mods/','mod/'))end
local root='mods/noita_together2/files/g3/';local checks=0;local case=os.getenv('NT2_A2_LAYOUT_CASE')or 'clearance'
local function check(v,message)assert(v,message);checks=checks+1 end
local height=360;local draw,options={},{};local created,destroyed=0,0;local click
local icons={keyboard='World message',player_list='Players',chat='Hide chat',hide_chat='Show chat',player_ghosts='Hide ghosts',hide_player_ghosts='Show ghosts'}
local function record(gui,id,x,y,label)
 draw[#draw+1]={gui=gui,id=id,x=x,y=y,label=label,no_tween=options[gui]==6}
 if click and click.label==label then check(click.x==x and click.y==y,'click target remains at fixed requested screen position');click=nil;return true end;return false
end
local e={GameGetRealWorldTimeSinceStarted=function()return 20 end,GUI_OPTION={NoPositionTween=6},
 GuiCreate=function()created=created+1;return created end,GuiDestroy=function()destroyed=destroyed+1 end,
 GuiStartFrame=function(gui)draw={};options[gui]=nil end,GuiOptionsAdd=function(gui,value)options[gui]=value end,
 GuiGetScreenDimensions=function()return 640,height end,GuiText=function(_,x,y,label)draw[#draw+1]={x=x,y=y,label=label}end,
 GuiButton=record,GuiImage=function()end,GuiTooltip=function()end,
 GuiImageButton=function(gui,id,x,y,_,path)local name=path:match('/([^/]+)%.png$');return record(gui,id,x,y,icons[name])end,
 EntityGetIsAlive=function()return true end,EntityGetTransform=function()return 50,100 end}
raw(repo..'/apps/desktop/tools/a2-layout-test-api.lua')(e)
e.GuiBeginScrollContainer=function(gui,id,x,y,width,height,focus,mx,my)record(gui,id,x,y,'legacy scroll');local w=draw[#draw];w.width=width;w.height=height;w.focus=focus;w.mx=mx;w.my=my end
local path=os.getenv('NT2_A2_RENDERER_FILE');local module=path and raw(path)or dofile(root..'a2_renderer.lua');local r=module.new(e,dofile(root..'validation.lua'))
local peers={};for i=1,32 do peers[i]={actor='p'..i,name=string.format('Player%02d',i),status=1,ghost=''}end;r:roster('run','p1',peers)
local function widget(label)for _,w in ipairs(draw)do if w.label==label then return w end end end
local function tick(playing)
 local chat_label=r.model.show_chat and 'Hide chat'or 'Show chat';local ghost_label=r.model.show_ghosts and 'Hide ghosts'or 'Show ghosts';local opacity_label=string.format('Ghost opacity %d%%',r.model.opacity*100)
 r:tick(playing,99)
 local w=assert(widget('Players'));if case=='reset'then check(w.no_tween,'NoPositionTween is applied after every GuiStartFrame, including recreated contexts')else check(w.x==140 and w.y==0,'Players uses the legacy top-row anchor')end
 for _,spec in ipairs({{'World message',79},{ghost_label,100},{chat_label,120},{'Players',140},{opacity_label,160}})do local v=assert(widget(spec[1]),'Missing widget '..spec[1]);check(v.x==spec[2]and v.y==0 and v.no_tween,'all toolbar controls keep exact legacy coordinates and immediate positioning')end
end
tick(true);local first=widget('Players');click={label='Players',x=first.x,y=first.y};tick(true);check(r.model.show_players,'real player-list toggle stays clickable')
r.model.feedback='Message placed'
for _,value in ipairs({360,240,180,540})do height=value;tick(true)
 local n=0;for _,w in ipairs(draw)do if w.label and w.label:match('^ Player%d%d$')then n=n+1;check(w.x==0 and w.y==0,'legacy rows use local scroll layout coordinates')end end
 check(n==32,'all bounded roster rows share a scroll container independent of viewport pagination')
 local panel=assert(widget('legacy scroll'));check(panel.x==5 and panel.y==50 and panel.width==100 and panel.height==150 and panel.focus==false and panel.mx==1 and panel.my==1,'exact legacy compact scroll panel')
 check(not widget('Previous')and not widget('Next')and r.offset==nil,'paginated replacement is removed')
end
local before=r.gui;tick(false);check(r.gui==before and not r.playing,'ordinary protection/pause reuses GUI and does not change play eligibility')
local hide=widget('Hide chat');click={label='Hide chat',x=hide.x,y=hide.y};tick(false);check(not r.model.show_chat,'local visibility control stays clickable while held')
r:close();check(destroyed==1 and not r.gui,'context close destroys GUI')
tick(false);check(r.gui~=before and created==2 and widget('Show chat').no_tween,'recreated GUI uses same immediate anchor/options')
local show=widget('Show chat');click={label='Show chat',x=show.x,y=show.y};tick(false);check(r.model.show_chat,'recreated context visibility control stays clickable')
local opacity=widget('Ghost opacity 100%');click={label=opacity.label,x=opacity.x,y=opacity.y};tick(false);check(r.model.opacity==0.75,'opacity remains clickable at160,0 while held')
tick(true);check(r.playing,'renderer reflects caller permission; no independent authority timer')
r:close(true);check(destroyed==2 and not next(r.peers),'terminal closes final context and player panel')
print('PASS toolbar layout '..case..' '..checks..' assertions; exact legacy top row, compact legacy scroll panel, pause/context reset and click calls; injected engine facts')
