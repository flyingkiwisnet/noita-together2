-- Production presenter/bridge/guard; geometry and engine facts are injected.
local repo=assert(os.getenv('NT2_ITEM_REPO'));local raw=dofile
dofile=function(path)if path:find('item_catalogue_data.lua',1,true)then return raw(repo..'/packages/contracts/g3-items/catalogue.lua')end;return raw(repo..'/'..path:gsub('^mods/','mod/'))end
local root='mods/noita_together2/files/g3/';local json=dofile(root..'bank_json.lua');local chunks=dofile(root..'item_chunks.lua');local checks=0
local function check(v,why)assert(v,why);checks=checks+1 end
local function equal(a,b,why)check(a==b,why or (tostring(a)..' ~= '..tostring(b)))end
local catalogue=dofile(root..'item_catalogue.lua');local presenter=dofile(root..'item_presenter.lua');local frame=0;local serial=0
local function identity()serial=serial+1;return string.format('00000000-0000-4000-8000-%012d',serial)end
local mouse,pressed,option,last,z,claimed,inspected,widgets
local slot=root..'ui/item_bank/slot.png';local file=assert(io.open(repo..'/mod/noita_together2/files/g3/ui/item_bank/slot.png','rb'));local header=file:read(24);file:close()
local function integer(at)local a,b,c,d=header:byte(at,at+3);return ((a*256+b)*256+c)*256+d end
local sw,sh=integer(17),integer(21);equal(sw,20);equal(sh,20)
local e={GUI_OPTION={NonInteractive=99},GameGetFrameNum=function()return frame end,GuiTextInput=function(_,_,_,_,value)return value end,GuiButton=function()return false end,GuiGetImageDimensions=function()return 20,20 end}
e.GuiOptionsAddForNextWidget=function(_,v)option=v end
local function record(kind,id,x,y,text)local v={kind=kind,id=id,x=x,y=y,z=z,option=option,text=text};option=nil;last=v;widgets[#widgets+1]=v;return v end
e.GuiImageButton=function(_,id,x,y,_,path)local w=record(path==slot and 'slot'or 'control',id,x,y,path);w.over=path==slot and mouse.x>=x and mouse.x<x+sw and mouse.y>=y and mouse.y<y+sh;return pressed and w.over end
e.GuiGetPreviousWidgetInfo=function()return false,false,last and last.over==true end
e.GuiImage=function(_,id,x,y,path)local w=record('image',id,x,y,path);equal(w.option,99,'Decorative image cannot intercept the slot');equal(w.z,path==slot and 9 or 7,'Image/button layers are strictly separate')end
e.GuiText=function(_,x,y,text)local w=record('text',nil,x,y,text);if text=='5'then equal(w.option,99,'Uses overlay cannot intercept the slot');equal(w.z,6,'Uses remain above icon7 and button8')end end
e.GuiTooltip=function()equal(last.kind,'slot','Tooltip is anchored to full-slot button');equal(last.z,8);equal(z,5,'Tooltip stays above uses6, item7 and button8')end
local function layer(value)z=value end
local wandSprite=next(catalogue.sprites)
for _,kind in ipairs({'wand','spell','container'})do
 local row={itemId=identity(),revision='1',kind=kind,iconId=kind=='wand'and wandSprite or kind=='spell'and 'LIGHT_BULLET'or 'potion',usesRemaining='5',claimAllowed=true,state='available'}
 local items={catalogue=catalogue,view={enabled=true,fresh=false,refresh_status='Refreshing item bank...'},bank={items={row},page=0,pageCount=1},loading=true,query=function(self,q)self.bank.query=q;return true end,claim=function(_,r)claimed[#claimed+1]=r.itemId;return true end,inspect=function()inspected=inspected+1;return true end}
 local stable
 for _,point in ipairs({{0.25,0.25},{10,10},{19.75,19.75},{19.75,0.25},{0.25,19.75}})do
  mouse={x=point[1],y=55+point[2]};pressed=true;option=nil;last=nil;widgets={};claimed={};inspected=0;row.claimAllowed=true;row.state='available';local n=100
  presenter.draw(e,1,function()n=n+1;return n end,items,0,0,layer)
  equal(#claimed,1,kind..' center/edge claims exactly one matching row');equal(claimed[1],row.itemId);equal(inspected,0,'Click never starts a competing hover read');check(not row.claimAllowed);equal(row.state,'claimed')
  for _,w in ipairs(widgets)do if w.kind=='slot'then equal(w.x,0);equal(w.y,55);if stable then equal(w.id,stable,'Stable slot ID through refresh')else stable=w.id end end end
 end
 mouse={x=20.25,y=65};pressed=true;widgets={};claimed={};row.claimAllowed=true;row.state='available';local n=100;presenter.draw(e,1,function()n=n+1;return n end,items,0,0,layer);equal(#claimed,0,'Gap never claims adjacent or earlier row')
 mouse={x=10,y=65};pressed=true;widgets={};claimed={};items.view.enabled=false;n=100;presenter.draw(e,1,function()n=n+1;return n end,items,0,0,layer);equal(#claimed,0,'Disabled rows remain inert')
end
-- Hover churn is coalesced before transport; claims keep original identity.
local sent={};local bridge=dofile(root..'item_bridge.lua').new(e,identity,function(body,priority)sent[#sent+1]={value=assert(json.decode(body)),priority=priority};return true end,function()return {}end,function()return false end,function()end)
bridge:bind('hover');bridge.view={enabled=true};bridge.next_scan=100000
local a,b,c=identity(),identity(),identity();bridge:inspect(a);bridge:inspect(b);bridge:inspect(c);frame=1;bridge:tick(frame,99)
equal(#sent,1);equal(sent[1].value.kind,'item_inspect');equal(sent[1].value.item_id,c)
for f=2,50 do frame=f;bridge:inspect(a);bridge:tick(frame,99)end;equal(#sent,1,'One outstanding detail read despite per-frame hover changes')
local claim={itemId=identity(),revision='2',claimAllowed=true,state='available'};check(bridge:claim(claim));equal(sent[2].value.kind,'item_claim');equal(sent[2].priority,'user');local original=sent[2].value.id
local fresh={itemId=claim.itemId,revision='3',claimAllowed=true,state='available'};bridge:receive({enabled=true,chunk=assert(chunks.sender(identity(),'bank',assert(json.encode({query={},revision='3',items={fresh}})))):chunk()})
check(not bridge.bank.items[1].claimAllowed,'Refresh cannot reopen a pending claim');check(not bridge:claim(fresh));equal(sent[2].value.id,original);equal(#sent,2)
bridge:receive({enabled=true,chunk=assert(chunks.sender(identity(),'detail',assert(json.encode({summary={itemId=c},item={}})))):chunk()});frame=51;bridge:tick(frame,99);frame=52;bridge:tick(frame,99)
local reads=0;for _,s in ipairs(sent)do if s.value.kind=='item_inspect'then reads=reads+1 end end;equal(reads,2,'Only the latest queued hover follows the completed read')
bridge:cancel_inspect();bridge:inspect(c);equal(bridge.detail.summary.itemId,c,'Immutable descriptor cache answers an earlier hover');check(bridge.inspect_wanted==nil)
bridge:bind('other');check(next(bridge.details)==nil and bridge.inspect_inflight==nil,'Scope change removes hover cache and pending reads')
-- Diagnose exclusions without accepting anything outside the established rule.
local row={x=15,y=15,physics=2,parent=0};local logs={};local minted=0
local engine={EntityGetIsAlive=function()return true end,EntityGetParent=function(id)return id==10 and row.parent or 0 end,EntityGetRootEntity=function(id)return id==10 and row.parent~=0 and row.parent or id end,EntityGetTransform=function(id)if id==10 then return row.x,row.y end;return 0,0 end,EntityHasTag=function(id,tag)return id==10 and tag=='wand'end,EntityGetFirstComponentIncludingDisabled=function(id,kind)if id==10 then if kind=='SimplePhysicsComponent'then return row.physics elseif kind=='ItemComponent'then return 3 elseif kind=='AbilityComponent'then return 4 end end end,ComponentGetValue2=function(_,key)if key=='ui_sprite'then return 'data/items_gfx/wands/not_registered.xml'end end,EntityGetInRadiusWithTag=function(_,_,r,tag)return tag=='wand'and row.x^2+row.y^2<r^2 and {10}or {}end,GameGetFrameNum=function()return 1 end,print=function(s)logs[#logs+1]=s end}
local api=dofile(root..'item_game_api.lua').new(engine);local guard=dofile(root..'item_local.lua').new(engine,{capture=function()error('Unsupported wand sprite PRIVATE_DATA')end},api,function()minted=minted+1;return identity()end);guard:bind('inlet')
for _=1,1000 do guard:capture(1)end;equal(#logs,7,'Repeated exclusion produces bounded logarithmic diagnostics');check(logs[1]:find('reason=outside_inlet',1,true))
row.x=30;row.y=0;guard:capture(1);check(logs[#logs]:find('nearby_outside_radius',1,true),'Near wands outside query radius are observation only')
row.x=1;row.physics=nil;guard:capture(1);check(logs[#logs]:find('missing_physics',1,true))
row.physics=2;row.parent=99;guard:capture(1);check(logs[#logs]:find('not_root',1,true))
row.parent=0;guard:capture(1);check(logs[#logs]:find('sprite_ui_base_xml',1,true));check(not table.concat(logs):find('PRIVATE_DATA',1,true),'Capture exception text never enters diagnostic output');equal(minted,0,'All excluded candidates retain originals and mint no operation')
-- Batched observations must log threshold crossings even when totals skip powers of two.
local multi={};row.x=30;row.y=0
engine.print=function(s)multi[#multi+1]=s end
engine.EntityGetTransform=function(id)if id>=10 and id<=12 then return row.x,row.y end;return 0,0 end
engine.EntityGetInRadiusWithTag=function(_,_,r,tag)return tag=='wand'and r==48 and {10,11,12}or {}end
local multiple=dofile(root..'item_local.lua').new(engine,{capture=function()error('No capture outside accepted radius')end},api,function()error('No original outside accepted radius')end);multiple:bind('multiple')
multiple:capture(1);equal(#multi,1,'First batch of three excluded wands is recorded');check(multi[1]:find('count=3|',1,true))
for _=1,1000 do multiple:capture(1)end;equal(#multi,6,'Batches of three cross only bounded diagnostic thresholds');check(multi[#multi]:find('count=66|',1,true));check(#multi<=128)
print('ITEM_TARGET_INLET_CHECKS|'..checks..'|engineFactsInjected=true|actualNoita=false')
