-- Observer-only injected facts. Every setter is a trap; real engine unexecuted.
local repo=assert(os.getenv('NT2_POTION_REPO'))
local function observer(environment)
 environment._G=environment;setmetatable(environment,{__index={type=type,pcall=pcall,math=math,string=string,ipairs=ipairs,table=table}})
 local file=assert(io.open(repo..'/apps/desktop/tools/g3-potion-diagnostic-observer.lua','rb'));local source=file:read('*a');file:close()
 local f=assert(loadstring(source..'\nreturn potion_trace'));setfenv(f,environment);return f()
end
local logs,calls,frame,detached,dead={},0,0,false,false
local components={ControlsComponent=1,Inventory2Component=2,InventoryGuiComponent=3,ItemPickUpperComponent=4,ItemComponent=5,MaterialInventoryComponent=6,PhysicsBodyComponent=7,PhysicsBody2Component=8,DamageModelComponent=9,PotionComponent=10}
local e={print=function(line) logs[#logs+1]=line end}
local function getter(fn) return function(...) calls=calls+1;return fn(...) end end
e.GameGetFrameNum=getter(function() return frame end)
e.EntityGetFirstComponentIncludingDisabled=getter(function(_,kind) return components[kind] end)
e.ComponentGetIsEnabled=getter(function() return true end)
e.ComponentGetValue2=getter(function(_,field) if field=='inventory_slot' then return 2,0 end;return false end)
e.EntityGetAllChildren=getter(function(id) if id==99 then return {200} end;local children={};for i=1,40 do children[i]=200+i end;return children end)
e.EntityGetName=getter(function() return 'inventory_quick' end)
e.EntityHasTag=getter(function(id,tag) return id>=201 and tag=='potion' end)
e.EntityGetParent=getter(function() return detached and 0 or 200 end)
e.EntityGetRootEntity=getter(function() return detached and 201 or 99 end)
e.EntityGetIsAlive=getter(function() return not dead end)
e.EntityGetTransform=getter(function() return 17.5,-85 end)
e.GetMaterialInventoryMainMaterial=getter(function() return 42 end)
for _,name in ipairs({'ComponentSetValue2','EntitySetComponentIsEnabled','EntityAddComponent2','EntityRemoveComponent','EntityKill','EntityLoad','SetWorldSeed','EntitySetTransform','EntityAddTag','EntityRemoveTag','AddMaterialInventoryMaterial','GamePickUpInventoryItem'}) do e[name]=function() error('observer mutation: '..name) end end
local trace=observer(e)
trace:sample('spawn_before',99)
assert(logs[1]:find('child_count=40',1,true) and logs[1]:find('tracked=[201,1,200,99,17.50,-85.00',1,true),'capture actual existing tracked facts')
local at=calls;trace:sample('spawn_before',99);assert(calls==at,'duplicate snapshot does no engine work')
detached=true;trace:sample('spawn_after_protect',99);assert(logs[2]:find('tracked=[201,1,0,201',1,true),'retain tracked detached potion')
dead=true;frame=1;trace:frame(true);assert(logs[3]:find('tracked=[201,0;202,0;203,0;204,0]',1,true),'retain dead IDs')
frame=61;trace:frame(true)
for _,phase in ipairs({'start_restored','reprotected','offline_restored','unbound_after_recovery'}) do trace:sample(phase,99) end
local at=calls;trace:sample('extra',99);trace:frame(true);assert(calls==at and #logs==8,'global snapshot count bound')
for _,line in ipairs(logs) do assert(#line<=4096);local n=tonumber(line:match('|calls=(%d+)'));assert(n and n<=512,'getter call budget') end
-- Missing APIs, thrown getters and failed print never escape or mutate authority.
local failed=observer({print=function() error('print unavailable') end,GameGetFrameNum=function() error('getter unavailable') end})
failed:sample('spawn_before',99);failed:frame(true)
local unknown={};local malformed=observer({print=function(line) unknown[#unknown+1]=line end,GameGetFrameNum=function() return 'secret\ntext' end,EntityGetFirstComponentIncludingDisabled=function() return 'secret' end})
malformed:sample('spawn_before',99);malformed:frame(true);assert(#unknown==2)
for _,line in ipairs(unknown) do assert(not line:find('secret',1,true) and not line:find('\n',1,true)) end
print('PASS observer bounded count/length/getters, tracked attached/detached/dead identity, duplicate/no-frame logging, missing/throwing APIs/print and no arbitrary-string output; engine facts injected')
