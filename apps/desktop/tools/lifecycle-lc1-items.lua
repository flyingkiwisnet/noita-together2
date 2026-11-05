-- Synthetic engine facts for the existing ordinary-scheduler composition.
-- Production capture, escrow, constructors, mailbox/native, app journal and
-- authenticated service make all transfer decisions. No loaded-save proof.
local M={}
function M.new(e,state,input,mark,cell_hit)
 local root='mods/noita_together2/files/g3/'
 local json=dofile(root..'bank_json.lua');local catalogue=dofile(root..'item_catalogue.lua');local codec=dofile(root..'item_descriptor.lua').new(catalogue)
 local api=dofile(root..'item_game_api.lua').new(e);local base={}
 for _,key in ipairs({'EntityGetFirstComponentIncludingDisabled','EntityGetComponentIncludingDisabled','EntityGetIsAlive','EntityGetTransform','EntityGetWithTag','EntityHasTag','EntityGetAllChildren','EntityGetParent','EntityGetRootEntity','EntityGetFilename','EntityGetName','ComponentGetValue2','ComponentSetValue2','ComponentObjectGetValue2','ComponentObjectSetValue2','EntityAddComponent2','EntityRemoveComponent','EntityLoad','EntityKill','EntityAddChild','EntitySetComponentsWithTagEnabled','EntityRefreshSprite','EntityGetInRadiusWithTag','CellFactory_GetName','CellFactory_GetType','GameGetPotionColorUint','RemoveMaterialInventoryMaterial','AddMaterialInventoryMaterial','CreateItemActionEntity'})do base[key]=e[key]end
 local rows,parts,owners={}, {}, {};local serial,part=800010,810000;local inlet=800001;local inlet_x,inlet_y;local creating_original=false;local kills,created,originals=0,0,0;local removals={};local removal_history={}
 local function new_part(id,kind,value)part=part+1;parts[part]=value;owners[part]={id=id,kind=kind};rows[id].parts[kind]=rows[id].parts[kind]or {};table.insert(rows[id].parts[kind],part);return part end
 local function spawn(path,x,y,kind)
  serial=serial+1;local id=serial;rows[id]={x=x,y=y,path=path,tags=kind=='wand'and {wand=true}or kind=='spell'and {card_action=true}or {item_physics=true},parts={},children={},original=creating_original}
  new_part(id,'ItemComponent',{is_pickable=true,uses_remaining=-1,inventory_slot={0,0},permanently_attached=false,always_use_item_name_in_ui=false,item_name=kind=='powder'and '$item_powder_stash_3'or '',ui_sprite=''})
  new_part(id,'SimplePhysicsComponent',{})
  if kind=='wand'then new_part(id,'AbilityComponent',{gun_config={},gunaction_config={}});new_part(id,'SpriteComponent',{});new_part(id,'HotspotComponent',{})
  elseif kind=='potion'or kind=='powder'then new_part(id,'MaterialInventoryComponent',{count_per_material_type={}})end
  created=created+1;return id
 end
 e.EntityGetIsAlive=function(id)return id==inlet or rows[id]~=nil or base.EntityGetIsAlive(id)end
 e.EntityGetTransform=function(id)if id==inlet then if not inlet_x then local x,y=base.EntityGetTransform(99);inlet_x=x+100;inlet_y=y end;return inlet_x,inlet_y end;local r=rows[id];if r then return r.x,r.y,0 end;return base.EntityGetTransform(id)end
 e.EntityGetFirstComponentIncludingDisabled=function(...)local id,kind,tag=...;assert(select('#',...)==2 or type(tag)=='string','Optional string cannot be explicit nil');local r=rows[id];if r then return r.parts[kind]and r.parts[kind][1]end;return base.EntityGetFirstComponentIncludingDisabled(...)end
 e.EntityGetComponentIncludingDisabled=function(id,kind)local r=rows[id];if r then return r.parts[kind]or {}end;return base.EntityGetComponentIncludingDisabled(id,kind)end
 e.ComponentGetValue2=function(c,key)if parts[c]then local v=parts[c][key];if type(v)=='table'and key~='count_per_material_type'then return unpack(v)end;return v end;return base.ComponentGetValue2(c,key)end
 e.ComponentSetValue2=function(c,key,...)if parts[c]then local value={...};if #value==1 then parts[c][key]=value[1]else parts[c][key]=value end;return end;return base.ComponentSetValue2(c,key,...)end
 e.ComponentObjectGetValue2=function(c,o,k)if parts[c]then return parts[c][o][k]end;return base.ComponentObjectGetValue2(c,o,k)end
 e.ComponentObjectSetValue2=function(c,o,k,v)if parts[c]then parts[c][o][k]=v;return end;return base.ComponentObjectSetValue2(c,o,k,v)end
 e.EntityAddComponent2=function(id,kind,value)if rows[id]then return new_part(id,kind,value)end;return base.EntityAddComponent2(id,kind,value)end
 e.EntityRemoveComponent=function(id,c)if owners[c]then local kind=owners[c].kind;local list=rows[id]and rows[id].parts[kind]or {};for i=#list,1,-1 do if list[i]==c then table.remove(list,i)end end;parts[c]=nil;owners[c]=nil;return end;return base.EntityRemoveComponent(id,c)end
 e.EntityGetWithTag=function(tag)if tag=='nt2_item_inlet'then return {inlet}end;return base.EntityGetWithTag(tag)end
 e.EntityGetInRadiusWithTag=function(x,y,radius,tag)local result={};for id,r in pairs(rows)do if r.tags[tag]and (r.x-x)^2+(r.y-y)^2<radius^2 then result[#result+1]=id end end;return result end
 e.EntityHasTag=function(id,tag)if rows[id]then return rows[id].tags[tag]==true end;return base.EntityHasTag(id,tag)end
 e.EntityGetAllChildren=function(id)if rows[id]then return rows[id].children end;return base.EntityGetAllChildren(id)end
 e.EntityGetParent=function(id)if rows[id]then return rows[id].parent or 0 end;return base.EntityGetParent(id)end
 e.EntityGetRootEntity=function(id)if rows[id]then local p=rows[id].parent;return p and e.EntityGetRootEntity(p)or id end;return base.EntityGetRootEntity(id)end
 e.EntityGetFilename=function(id)if rows[id]then return rows[id].path end;return base.EntityGetFilename and base.EntityGetFilename(id)or ''end
 e.EntityGetName=function(id)if rows[id]then return ''end;return base.EntityGetName(id)end
 e.EntityLoad=function(path,x,y)
  local kind=path:find('/item_wand.xml',1,true)and 'wand'or path:find('/item_potion.xml',1,true)and 'potion'or path:find('/item_powder.xml',1,true)and 'powder'
  if not kind then for _,o in pairs(catalogue.objects)do if o.available and o.path==path then kind='object';break end end end
  if kind then if not creating_original and input('items-spawn-fail',false)then return 0 end;return spawn(path,x,y,kind)end
  return base.EntityLoad(path,x,y)
 end
 e.EntityKill=function(id)if rows[id]then
  kills=kills+1
  if rows[id].original then
   assert(not removals[id],'Original removal must not be requested twice')
   local delay=(input('items-removal-stuck',false)or input('items-removal-stuck-once',true))and 100000 or 3
   removals[id]=state.frame+delay;removal_history[#removal_history+1]={entity=id,requestedFrame=state.frame,dueFrame=removals[id]}
  else rows[id]=nil end
  return
 end;return base.EntityKill(id)end
 e.EntityAddChild=function(parent,child)if rows[parent]and rows[child]then rows[child].parent=parent;table.insert(rows[parent].children,child);return end;return base.EntityAddChild(parent,child)end
 e.EntitySetComponentsWithTagEnabled=function(id,tag,value)if rows[id]then return end;if base.EntitySetComponentsWithTagEnabled then return base.EntitySetComponentsWithTagEnabled(id,tag,value)end end
 e.EntityRefreshSprite=function(id,c)if rows[id]then return end;if base.EntityRefreshSprite then return base.EntityRefreshSprite(id,c)end end
 local materials={};for name in pairs(catalogue.materials)do materials[#materials+1]=name end;table.sort(materials);local indexes={};for i,name in ipairs(materials)do indexes[name]=i-1 end
 e.CellFactory_GetName=function(index)return materials[index+1]end;e.CellFactory_GetType=function(name)return indexes[name]end
 e.GameGetPotionColorUint=function(id)if rows[id]then return 4294967295 end;return base.GameGetPotionColorUint and base.GameGetPotionColorUint(id)or 0 end
 e.RemoveMaterialInventoryMaterial=function(id)if rows[id]then parts[e.EntityGetFirstComponentIncludingDisabled(id,'MaterialInventoryComponent')].count_per_material_type={};return end;return base.RemoveMaterialInventoryMaterial(id,'')end
 e.AddMaterialInventoryMaterial=function(id,name,amount)if rows[id]then parts[e.EntityGetFirstComponentIncludingDisabled(id,'MaterialInventoryComponent')].count_per_material_type[indexes[name]+1]=amount;return end;return base.AddMaterialInventoryMaterial(id,name,amount)end
 e.CreateItemActionEntity=function(action,x,y)if not creating_original and input('items-spawn-fail',false)then return 0 end;local id=spawn('action',x or 0,y or 0,'spell');new_part(id,'ItemActionComponent',{action_id=action});return id end
 local constructor=dofile(root..'item_construct.lua').new(e,codec)
 local self={};local source_block
 local hit=assert(cell_hit,'Physical GUI cell adapter required')
 function self:hover(x,y)local value=input('items-cell-hover',false);return value and hit(assert(json.decode(value)),x,y,'')or false end
 function self:button(x,y,label)
  local value=input('items-cell-click',false);local plan=value and assert(json.decode(value))
  if plan and hit(plan,x,y,label)then input('items-cell-click',true);mark('items-cell-clicked',value);return true end
  return false
 end
 function self:snapshot(bank)
  -- Explicit, owned injected source condition; never a production engine hook.
  if source_block and input('items-source-release',true)then if rows[source_block.entity]then rows[source_block.entity].parent=nil end;mark('items-source-released',source_block.id);source_block=nil end
  local block=input('items-source-block',false)
  if block and not source_block and bank and bank.items then
   local plan=assert(json.decode(block));local records={}
   for _,record in pairs(bank.items.guard.records)do if record.snapshot then records[#records+1]=record end end
   table.sort(records,function(a,b)return a.snapshot.entity<b.snapshot.entity end)
   local record=records[assert(plan.ordinal)];if record and record.state==plan.stage and rows[record.snapshot.entity]then
    input('items-source-block',true);rows[record.snapshot.entity].parent=99;source_block={id=record.id,entity=record.snapshot.entity,stage=record.state};mark('items-blocked.json',assert(json.encode(source_block)))
   end
  end
  for id,due in pairs(removals)do if state.frame>=due then rows[id]=nil;removals[id]=nil;for _,event in ipairs(removal_history)do if event.entity==id then event.observedFrame=state.frame end end end end
  local late=input('items-late-result',true);if late and bank and bank.items then local result=assert(json.decode(late));assert(result.kind=='item_result');bank.items.result=result end
  local incoming=input('items-spawn',true);local burst=input('items-spawn-burst',true);local batch=burst and assert(json.decode(burst,{bytes=131072,nodes=32768,depth=16}))or {};assert(#batch<=3,'Small owned burst only');if incoming then batch[#batch+1]=assert(json.decode(incoming,{bytes=131072,nodes=32768,depth=16}))end
  for _,item in ipairs(batch)do local x,y=e.EntityGetTransform(inlet);creating_original=true;local job=assert(constructor:start(item,x+1,y));local result=job:step(true);while result=='constructing'do result=job:step(true)end;assert(result=='applied_session');creating_original=false;originals=originals+1;mark('items-original',tostring(job.entity))end
  if bank and bank.items then
   local alive_originals,received={},{}
   for id,r in pairs(rows)do if not r.parent and parts[e.EntityGetFirstComponentIncludingDisabled(id,'ItemComponent')].is_pickable then local ok,item=pcall(codec.capture,codec,api,e,id);if ok then if r.original then alive_originals[#alive_originals+1]=item else received[#received+1]=item end end end end
   mark('items-view.json',assert(json.encode({frame=state.frame,originals=originals,kills=kills,created=created,aliveOriginals=json.array(alive_originals),received=json.array(received),bank=bank.items.bank,loading=bank.items.loading,detail=bank.items.detail,preview=bank.items.preview,view=bank.items.view,uploads=#bank.items.uploads,status=bank.items.guard.status,removalHistory=json.array(removal_history),effectOutcomes=json.array(bank.items.guard.outcomes or {}),engineFactsInjected=true,savePersistenceProven=false},{bytes=262144,nodes=65536,depth=20})))
  end
 end
 return self
end
return M
