-- Actual player-list module + shared renderer/model/lock. GUI layout, clipping,
-- scrolling, hover and component facts are injected; not Noita pixel evidence.
local repo=assert(os.getenv('NT2_A2_REPO'));local raw=dofile
dofile=function(p)return raw(repo..'/'..p:gsub('^mods/','mod/'))end
local root='mods/noita_together2/files/g3/';local checks=0
local function check(v,s)assert(v,s);checks=checks+1 end
local frame,press,hover,scroll=0,nil,nil,0;local widgets,panels,spacings,tooltips={}, {}, {},{}
local z,next_z,same,inside,row=0,nil,false,false,1;local last;local globals={}
local components={[1]={enabled=true},[2]={hp=1}};local markers={};local component_id=2
local e={GUI_OPTION={NoPositionTween=6,Layout_NextSameLine=1},
 GameGetRealWorldTimeSinceStarted=function()return frame/60 end,
 GlobalsGetValue=function(k,d)return globals[k]or d end,GlobalsSetValue=function(k,v)globals[k]=v end,
 EntityGetIsAlive=function(id)return id==99 end,EntityGetTransform=function()return 0,-80 end,
 EntityGetFirstComponentIncludingDisabled=function(_,kind)return ({ControlsComponent=1,DamageModelComponent=2})[kind]end,
 EntityGetComponentIncludingDisabled=function()local ids={};for id in pairs(markers)do ids[#ids+1]=id end;return ids end,
 ComponentGetValue2=function(id,key)return components[id][key]end,ComponentSetValue2=function(id,key,v)components[id][key]=v end,
 EntityAddComponent2=function(_,_,v)component_id=component_id+1;components[component_id]=v;markers[component_id]=true;return component_id end,
 EntityRemoveComponent=function(_,id)markers[id]=nil;components[id]=nil end,
 a2_positions=function()return {ga={x=0,y=1234}}end,
 GameTextGetTranslatedOrNot=function(key)return ({['$biome_coalmine']='Mines',['$biome_without_icon']='Unlisted Sanctuary',['$nt2_starting_mountain']='Mountain'})[key]or key end,
 GuiCreate=function()return 1 end,GuiDestroy=function()end,GuiGetScreenDimensions=function()return 640,360 end,
 GuiStartFrame=function()widgets={};panels={};spacings={};tooltips={};z=0;next_z=nil;same=false;inside=false;row=1 end,
 GuiOptionsAdd=function(_,value)check(value==6,'shared toolbar keeps immediate positioning')end,
 GuiOptionsAddForNextWidget=function(_,value)check(value==1,'legacy next-same-line option');same=true end,
 GuiZSet=function(_,value)z=value end,GuiZSetForNextWidget=function(_,value)next_z=value end}
local function widget(kind,id,x,y,label,scale)
 local w={kind=kind,id=id,x=x,y=y,label=label,scale=scale,z=next_z or z,same=same,inside=inside,row=row,visible=not inside or row>scroll and row<=scroll+8}
 widgets[#widgets+1]=w;last=w;next_z=nil;same=false;return w
end
e.GuiBeginScrollContainer=function(_,id,x,y,w,h,focus,mx,my)
 check(not inside,'one flat scroll scope');local p=widget('scroll',id,x,y,'scroll');p.w=w;p.h=h;p.focus=focus;p.mx=mx;p.my=my;panels[#panels+1]=p;inside=true
end
e.GuiLayoutBeginVertical=function(_,x,y)check(inside and x==0 and y==0,'legacy vertical layout begins at local0,0')end
e.GuiLayoutEnd=function()check(inside,'layout closes inside scroll scope')end
e.GuiEndScrollContainer=function()check(inside,'scroll scope paired');inside=false end
e.GuiLayoutAddVerticalSpacing=function(_,value)check(value==2,'legacy row spacing2');spacings[#spacings+1]=value;row=row+1 end
e.GuiImage=function(_,id,x,y,path,_,scale)widget('image',id,x,y,path,scale)end
e.GuiImageNinePiece=function(_,id,x,y,_,_,_,path)widget('background',id,x,y,path)end
e.GuiText=function(_,x,y,text)widget('text',nil,x,y,text)end
e.GuiTextInput=function(_,id,x,y,value)widget('input',id,x,y,value);return value end
e.GuiButton=function(_,id,x,y,label)local w=widget('button',id,x,y,label);return press==label and w.visible,press=='right:'..label and w.visible end
local icons={keyboard='World message',player_list='Players',bank='Gold bank',fridge='Gold bank'}
e.GuiImageButton=function(_,id,x,y,_,path)local label=icons[path:match('/([^/]+)%.png$')]or path;widget('button',id,x,y,label);return press==label end
e.GuiTooltip=function(_,title,body)
 local target=last;tooltips[#tooltips+1]={title=title,body=body,z=next_z or z,row=target.row,inside=target.inside,hovered=hover==title and target.visible}
 if hover==title and target.visible then z=frame%2==0 and 300 or -300;next_z=z+1 end
end
local module=dofile(root..'a2_renderer.lua');local r=module.new(e,dofile(root..'validation.lua'))
local bank_ui=dofile(root..'bank_ui.lua').new(function()error('layout must not submit bank operation')end,function()return 'id'end)
r:set_bank({ui=bank_ui,wallet=function()return 100 end,pause=function()bank_ui.open=false end})
local peers={{actor='z',name='alice',status=1,ghost='',order=1},{actor='b',name='Bob',status=2,ghost='',order=2},{actor='a',name='Alice',status=1,ghost='ga',order=3}}
r:roster('list-run','z',peers)
r:receive_info({actor='a',revision=1,hp=1000,max_hp=2000,sampo=true,biome='$biome_coalmine'})
r:receive_info({actor='z',revision=1,hp=0,max_hp=4000,sampo=false,biome='$nt2_starting_mountain'})
local function tick(clicked,over)frame=frame+1;press=clicked;hover=over;r:tick(true,99);press=nil end
local function rows()local result={};for _,w in ipairs(widgets)do if w.inside and w.kind=='button'then result[#result+1]=w end end;return result end
local function images(index,suffix)local result={};for _,w in ipairs(widgets)do if w.inside and w.kind=='image'and w.row==index and w.label:sub(-#suffix)==suffix then result[#result+1]=w end end;return result end
local function tip(name)for _,t in ipairs(tooltips)do if t.inside and t.title==' '..name then return t end end;error('missing tooltip '..name)end
local function layout()
 check(#panels==1,'one scroll panel, no pagination');local p=panels[1]
 check(p.x==5 and p.y==50 and p.w==100 and p.h==150 and p.focus==false and p.mx==1 and p.my==1 and p.z==10,'exact legacy scroll geometry/options/depth')
 for _,w in ipairs(widgets)do
  check(w.label~='Previous'and w.label~='Next','tall paginated controls removed')
  if w.inside then
   if w.kind=='button'then check(w.x==0 and w.y==0 and w.z==10,'name uses legacy local0,0/depth10')
   elseif w.kind=='image'then check(w.z==(w.label:find('hpbar_empty',1,true)and 10 or 9),'explicit icon/bar depths independent of hover')end
  end
 end
 for _,t in ipairs(tooltips)do if t.inside then check(t.z==8,'name tooltip explicit foreground layer')end end
 e.GuiText(1,1,350,'following HUD');check(last.z==0,'list restores shared and next-widget depth')
end
tick();check(#panels==0,'player list initially closed')
for _,over in ipairs({'none','Players',' Alice','none',' alice',' Bob','none','Players'})do
 tick(frame==1 and 'Players'or nil,over);layout();local names=rows()
 check(#names==3 and names[1].label==' Alice'and names[2].label==' alice'and names[3].label==' Bob','case-insensitive alphabetical order with stable actor tie, independent of wire order')
 check(#spacings==3,'one compact spacing call per roster row')
 local green,red=images(1,'hpbar_full.png'),images(1,'hpbar_empty.png');check(#green==1 and green[1].scale==0.5 and green[1].same and #red==1 and red[1].scale==1,'legacy half-health fill over red background')
 check(#images(2,'hpbar_full.png')==0 and #images(2,'hpbar_empty.png')==1,'zero HP keeps only red background')
 check(#images(3,'hpbar_empty.png')==0,'never-known maximum has no invented bar')
 local sampo,biome=images(1,'sampo.png'),images(1,'biomes/coalmine.png')
 check(#sampo==1 and sampo[1].x==88 and sampo[1].y==0 and sampo[1].same and #biome==1 and biome[1].x==80 and biome[1].y==0 and biome[1].same,'legacy Sampo/biome icon anchors and same-line layout')
 check(tip('Alice').body:find('HP 25.0 / 50.0\nLocation: Mines\nDepth: 123m',1,true),'name-hover HP/location/depth tooltip and milli-HP/world-unit conversion')
 check(tip('alice').body:find('HP 0.0 / 100.0',1,true)and tip('alice').body:find('Depth: -8m',1,true),'known zero and local negative depth displayed safely')
 check(tip('Bob').body:find('HP unknown\nLocation: Unknown location\nDepth: unknown',1,true)and tip('Bob').body:find('waiting',1,true),'unknown fields and authoritative roster status preserved')
end
tick('right: Alice',' Alice');check(not r.model.follow.a,'deferred right-click inventory preview neither invents a preview nor toggles follow')
tick(' Alice',' Alice');check(r.model.follow.a and globals.nt2_a2_follow=='a','actual name click reaches persisted current follow adapter')
tick(' Alice',' Alice');check(not r.model.follow.a,'second name click toggles follow off')
tick(' Alice',' Alice');r:roster('list-run','z',peers);check(r.model.follow.a,'same-run roster refresh retains follow')
local restored=module.new(e,dofile(root..'validation.lua'));restored:roster('list-run','z',peers);check(restored.model.follow.a,'same-run cold renderer restores follow choices')
tick('World message');check(r.model.composer and not r.model.show_players and not components[1].enabled,'composer closes list and owns input')
tick('Players');check(r.model.show_players and not r.model.composer and components[1].enabled,'list closes composer and restores its owned input')
tick('Gold bank');check(r.model.bank_open and not r.model.show_players and not components[1].enabled and #panels==0,'bank closes list and owns input')
tick('Players');check(r.model.show_players and not r.model.bank_open and not bank_ui.open and components[1].enabled,'list closes bank and preserves input ownership')
local special={{actor='bad',name='Bad',status=1,ghost=''},{actor='full',name='Full',status=1,ghost=''},{actor='unnamed',name='Unnamed',status=1,ghost=''}}
r:roster('special','bad',special);r.model.show_players=true
r.model.hud.bad={hp=1000,max=0,biome='_EMPTY_'};r.model.hud.full={hp=3000,max=1000,biome='$biome_without_icon'};r.model.hud.unnamed={hp=0/0,max=math.huge,biome='Mountain'}
tick(nil,' Full');layout();check(tip('Bad').body:find('HP unknown',1,true)and #images(1,'hpbar_empty.png')==0,'zero/invalid max stays unknown without division')
check(images(2,'hpbar_full.png')[1].scale==1 and tip('Full').body:find('Unlisted Sanctuary',1,true),'over-max fill clamps while named unlisted biome remains readable')
check(tip('Unnamed').body:find('HP unknown',1,true)and tip('Unnamed').body:find('Unknown location',1,true)==nil,'nonfinite health is unknown without inventing a Mountain icon')
for _,w in ipairs(widgets)do check(not w.label:find('/biomes/',1,true),'unknown/unsupported/literal-Mountain locations never select a biome sprite')end
local biomes=dofile(root..'a2_biomes.lua');local n=0;for key,asset in pairs(biomes.sprites)do n=n+1;check(asset:match('^[a-z_]+%.png$')~=nil,'fixed local sprite names only');local f=assert(io.open(repo..'/mod/noita_together2/files/g3/ui/biomes/'..asset,'rb'));f:close()end
check(n==35 and biomes.icon('$nt2_starting_mountain')=='mountain.png'and not biomes.icon('Mountain')and not biomes.icon('_EMPTY_')and not biomes.icon('../unknown'),'all35 legacy icons with starting-only Mountain mapping')
local many={};for i=32,1,-1 do many[#many+1]={actor=string.format('p%02d',i),name=string.format('Player%02d',i),status=1,ghost='',order=33-i}end
r:roster('full-roster','p01',many);r.model.show_players=true;scroll=0;tick();layout();check(#rows()==32 and #spacings==32 and rows()[32].label==' Player32','all32 current bounded roster rows emitted into one scroll container')
check(not rows()[32].visible,'injected clipping hides last row at top')
scroll=24;tick(' Player32',' Player32');layout();check(rows()[32].visible and r.model.follow.p32,'injected scroll reaches/clicks final row without pagination')
local gui=r.gui;r:tick(false,99);check(r.gui==gui and #panels==1 and r.model.follow.p32,'protection retains roster/follow and GUI without a play grant')
r:roster('full-roster','p01',{});check(not r.model.follow.p32,'authoritative roster removal clears follow')
r:close(true);check(not next(r.model.hud)and not next(r.peers)and components[1].enabled,'terminal closes retained rows and leaves no input lease')
print('PASS legacy player list '..checks..' assertions; actual renderer/model/lock, recorded GUI/layout/hover/scroll facts; no actual Noita visual proof')
