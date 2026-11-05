-- Record actual renderer/bank draw calls. GUI depth/hover and entity facts are
-- injected; this is ordering/state evidence, not an actual Noita visual test.
local repo=assert(os.getenv('NT2_A3_REPO'));local raw=dofile
dofile=function(path)if path:find('item_catalogue_data.lua',1,true)then return raw(repo..'/packages/contracts/g3-items/catalogue.lua')end;return raw(repo..'/'..path:gsub('^mods/','mod/'))end
local root='mods/noita_together2/files/g3/'
local checks=0;local function check(value,message)assert(value,message);checks=checks+1 end
local frame,hover,click=0,nil,nil;local widgets,last={},nil;local z,next_z=0,nil
local components={[1]={enabled=true},[2]={hp=1}};local markers={};local next_component=2
local e={GUI_OPTION={NoPositionTween=6},GameGetRealWorldTimeSinceStarted=function()return frame/60 end,
 EntityGetIsAlive=function(id)return id==99 end,EntityGetTransform=function()return 0,0 end,
 EntityGetFirstComponentIncludingDisabled=function(_,kind)return ({ControlsComponent=1,DamageModelComponent=2})[kind]end,
 EntityGetComponentIncludingDisabled=function()local result={};for id in pairs(markers)do result[#result+1]=id end;return result end,
 ComponentGetValue2=function(id,key)return assert(components[id])[key]end,
 ComponentSetValue2=function(id,key,value)assert(components[id])[key]=value end,
 EntityAddComponent2=function(_,kind,value)assert(kind=='VariableStorageComponent');next_component=next_component+1;components[next_component]=value;markers[next_component]=true;return next_component end,
 EntityRemoveComponent=function(_,id)markers[id]=nil;components[id]=nil end,
 GuiCreate=function()return 1 end,GuiDestroy=function()end,GuiOptionsAdd=function()end,
 GuiStartFrame=function()widgets={};last=nil;z=0;next_z=nil end,
 GuiZSet=function(_,value)z=value end,GuiZSetForNextWidget=function(_,value)next_z=value end,
 GuiGetScreenDimensions=function()return 640,360 end}
local function record(kind,id,x,y,label)
 local widget={kind=kind,id=id,x=x,y=y,label=label,z=next_z or z};next_z=nil;widgets[#widgets+1]=widget;last=widget;return widget
end
e.GuiImageButton=function(_,id,x,y,_,path)
 local label=path:find('/bank.png',1,true)and 'Gold bank'or path
 record('button',id,x,y,label);return click==label
end
e.GuiButton=function(_,id,x,y,label)record('button',id,x,y,label);return click==label end
e.GuiText=function(_,x,y,label)record('text',nil,x,y,label)end
e.GuiTextInput=function(_,id,x,y,value)record('input',id,x,y,value);return value end
e.GuiImage=function(_,id,x,y,path)record('image',id,x,y,path)end
e.GuiImageNinePiece=function(_,id,x,y,_,_,_,path)record('background',id,x,y,path)end
e.GuiTooltip=function(_,label)
 if last and hover==label then
  record('tooltip',nil,last.x,last.y,label)
  -- Deliberately exercise both shared and next-widget state changes around
  -- tooltips. The engine's tooltip internals are not asserted by this test.
  z=frame%2==0 and 300 or -300;next_z=z+1
 end
end
local sent={};local ui=dofile(root..'bank_ui.lua').new(function(body)sent[#sent+1]=body;return true end,function()return '00000000-0000-4000-8000-000000000001'end)
ui:receive({fresh=true,enabled=true,balance=100,queued=4,status='Ready.'});ui.amount='7'
local bank={ui=ui,wallet=function()return 200 end,pause=function()ui.open=false end}
local renderer=dofile(root..'a2_renderer.lua').new(e,dofile(root..'validation.lua'));renderer:set_bank(bank)
local function draw(over,pressed,pending)
 frame=frame+1;hover=over;click=pressed;renderer:tick(true,99)
 local background,content,ids,labels=nil,{}, {},{}
 for _,w in ipairs(widgets)do
  if w.kind=='background'then background=w
  elseif w.x>=200 and w.y>=130 then content[#content+1]=w;if w.id then ids[#ids+1]=w.id end;labels[w.label]=w end
 end
 check(background~=nil,'bank background exists frame '..frame)
 local background_z=background.z
 for _,w in ipairs(content)do
  check(background_z>w.z,'opaque bank background must stay behind '..w.kind..' '..w.label..' frame '..frame..' hover '..tostring(over)..' (background '..background_z..', content '..w.z..')')
  check(w.z==9,'bank foreground stable across hover frames')
 end
 check(background_z==10,'legacy background layer retained')
 check(labels['BANK   Gold: 100']and labels['Wallet: 200   Queued: 4']and labels.Amount and labels.TAKE and labels.DEPOSIT and labels.Close,'every bank label and action drawn')
 check(#ids==(pending and 3 or 4),'input omitted only while original pending')
 check(background.id==107 and ids[1]==(pending and 109 or 108)and ids[#ids]==111,'widget IDs unchanged by depth/tooltip scopes')
 e.GuiText(1,0,300,'after bank')
 check(last.z==0,'bank restores shared HUD default depth after tooltip')
 check(components[1].enabled==false and renderer.lock.lease~=nil,'rendering preserves actual bank input lease')
end
-- Open through the real shared toolbar callback, then move on/off it and all
-- dialog buttons for multiple consecutive frames without clicking them.
draw(nil,'Gold bank',false)
for _,over in ipairs({'none','Gold bank','none','TAKE','none','DEPOSIT','none','Close','none','Gold bank','TAKE','none'})do draw(over,nil,false)end
check(ui.amount=='7'and ui.pending==nil and #sent==0,'hover never submits, resets amount or closes panel')
draw('DEPOSIT','DEPOSIT',false)
check(#sent==1 and ui.pending~=nil and ui.amount=='7','real deposit button still enqueues one original without clearing amount')
for _,over in ipairs({'none','Gold bank','TAKE','none','DEPOSIT','Close'})do draw(over,nil,true)end
check(#sent==1 and ui.pending~=nil,'pending redraw/hover never repeats a request')
-- Close through the actual shared button. Depth restoration also applies on
-- this frame even though the click closes the window and its owned lease.
frame=frame+1;hover='Close';click='Close';renderer:tick(true,99);e.GuiText(1,0,300,'after close')
check(last.z==0 and not ui.open and not renderer.model.bank_open,'close frame restores depth and closes the bank')
check(components[1].enabled and not renderer.lock.lease and next(markers)==nil,'close restores input and removes only its own marker')
renderer:close(false)
-- Item rows refresh asynchronously; each cell keeps its own three IDs even
-- when empty. Pending gold input must not shift the entire item grid either.
local json=dofile(root..'bank_json.lua');local catalogue=dofile(root..'item_catalogue.lua')
local items={view={enabled=true,fresh=true},catalogue=catalogue,bank={items={},page=0,pageCount=1},status='Ready.'}
function items:query(q)self.bank.query=q;return true end
function items:claim()error('Refresh/hover must not claim')end
function items:inspect()return true end
ui.items=items;ui.amount='7'
local stable={};local slot_art='mods/noita_together2/files/g3/ui/item_bank/slot.png'
for pass=1,8 do
 frame=frame+1;hover=pass%2==0 and 'TAKE'or nil;click=nil;e.GuiStartFrame(1)
 ui.pending=pass%3==0 and 'pending-original'or nil
 ui.view.refresh_status=pass%2==0 and 'Refreshing bank...'or nil;items.view.refresh_status=pass%2==0 and 'Refreshing item bank...'or nil
 items.bank.items={}
 if pass%2==0 then for cell=1,40 do items.bank.items[cell]={itemId='row-'..pass..'-'..cell,kind='spell',iconId='LIGHT_BULLET',usesRemaining='-1',state='available',claimAllowed=true}end end
 local widget=100;local function id()widget=widget+1;return widget end
 local function button(x,y,label)local result=e.GuiButton(1,id(),x,y,label);e.GuiTooltip(1,label,'');return result end
 ui:draw(e,1,id,button,200,640,360)
 local panel,seen,slots,icons=nil,{},0,0
 for _,w in ipairs(widgets)do
  if w.id then check(not seen[w.id],'Unique GUI ID per frame');seen[w.id]=true end
  if w.kind=='background'then panel=w;check(w.z==10,'Panel stays behind slots')end
  local key
  if w.label==slot_art and w.kind=='image'then
   slots=slots+1;key='slot-'..slots;check(w.z==9,'Every empty/populated slot is in front of panel')
  elseif w.kind=='image'and w.label==catalogue.spells.LIGHT_BULLET.sprite then
   icons=icons+1;key='icon-'..icons;check(w.z==7,'Item image is strictly in front of slot button8')
  elseif w.kind=='button'and w.label==slot_art then
   key='claim-'..w.x..':'..w.y;check(w.z==8,'Item action is in front of its slot')
  elseif w.kind=='button'and (w.label=='TAKE'or w.label=='DEPOSIT'or w.label=='Close'or w.label:find('back',1,true)or w.label:find('forward',1,true))then key=w.label end
  if key then if stable[key]then check(stable[key]==w.id,'Stable widget ID across refresh/pending: '..key)else stable[key]=w.id end end
 end
 check(panel and slots==40 and icons==(pass%2==0 and 40 or 0),'All forty slots persist across empty/populated refreshes')
 check(last.z==9,'Close keeps foreground depth after item presenter')
 e.GuiText(1,0,300,'after items');check(last.z==0,'Items restore default depth')
 check(ui.amount=='7','Pending refresh retains gold amount')
end
print('B1_BANK_RENDER_CHECKS '..checks..'; actual bank/shared renderer, '..frame..' recorded frames; GUI/engine facts injected')
