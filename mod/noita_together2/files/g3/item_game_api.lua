-- Adapted from Noita Together spell_eater.lua and utils.lua GetWandStats/
-- GetWandSpells (MIT, Soler91, 2022). These are local ENGINE FACTS, not a
-- network schema/authority. The canonical catalogue adapter validates them.
local M={}
local function finite(v)return type(v)=='number'and v==v and v>-math.huge and v<math.huge end
local function integer(v)return finite(v)and v%1==0 end
local function read(e,key,...)
 local f=e[key];if type(f)~='function'then return nil,'Engine API unavailable: '..key end
 local ok,a,b=pcall(f,...);if not ok then return nil,'Engine read failed: '..key end;return a,b
end
-- Noita distinguishes an omitted optional string from an explicit nil.
local function component(e,id,kind,tag)
 local c;if tag==nil then c=read(e,'EntityGetFirstComponentIncludingDisabled',id,kind)else c=read(e,'EntityGetFirstComponentIncludingDisabled',id,kind,tag)end
 return integer(c)and c>0 and c or nil
end
local function number(e,c,key)local v=read(e,'ComponentGetValue2',c,key);if not finite(v)then error('Unavailable item field: '..key)end;return v end
local function object_number(e,c,object,key)local v=read(e,'ComponentObjectGetValue2',c,object,key);if not finite(v)then error('Unavailable wand field: '..key)end;return v end
local function flag(e,c,key)local v=read(e,'ComponentGetValue2',c,key);if type(v)~='boolean'then error('Unavailable item flag: '..key)end;return v end
local function string_field(e,c,key)local v=read(e,'ComponentGetValue2',c,key);if type(v)~='string'then error('Unavailable item text: '..key)end;return v end
function M.new(e,limits)
 limits=limits or {};local self={max_cards=limits.cards or 1280,max_deck=limits.deck or 1024,max_permanent=limits.permanent or 256,max_materials=limits.materials or 1024,max_candidates=limits.candidates or 128}
 function self:spell(entity)
  local item=assert(component(e,entity,'ItemComponent'),'Item component unavailable');local action=assert(component(e,entity,'ItemActionComponent'),'Spell action unavailable')
  local uses=number(e,item,'uses_remaining');assert(integer(uses)and uses>=-1,'Invalid spell uses')
  return {actionId=string_field(e,action,'action_id'),usesRemaining=uses}
 end
 function self:wand_sprite(entity)
  local item=assert(component(e,entity,'ItemComponent'),'Wand item unavailable')
  local ui=string_field(e,item,'ui_sprite');if ui~=''then return ui,'ui'end
  local ability=assert(component(e,entity,'AbilityComponent'),'Wand ability unavailable')
  return string_field(e,ability,'sprite_file'),'ability'
 end
 function self:sprite_diagnostic(entity)
  local ok,path,source=pcall(self.wand_sprite,self,entity)
  if not ok then return 'sprite_source_unavailable'end
  local category='other'
  if not path:find('..',1,true)and path:match('^data/[A-Za-z_0-9/.-]+%.xml$')then category='base_xml'
  elseif not path:find('..',1,true)and path:match('^data/[A-Za-z_0-9/.-]+%.png$')then category='base_png'
  elseif path:match('^mods/')then category='mod_path'end
  return 'sprite_'..source..'_'..category
 end
 function self:wand(entity)
  local ability=assert(component(e,entity,'AbilityComponent'),'Wand ability unavailable');local item=assert(component(e,entity,'ItemComponent'),'Wand item unavailable');local sprite=assert(component(e,entity,'SpriteComponent'),'Wand sprite unavailable');local hotspot=assert(component(e,entity,'HotspotComponent','shoot_pos'),'Wand tip unavailable')
  local stats={named=flag(e,item,'always_use_item_name_in_ui'),uiName=string_field(e,item,'item_name')}
  if not stats.named then stats.uiName=string_field(e,ability,'ui_name')end
  stats.sprite=self:wand_sprite(entity)
  for _,key in ipairs({'mana_max','mana_charge_speed'})do stats[key=='mana_max'and 'manaMax'or 'manaChargeSpeed']=number(e,ability,key)end
  for _,pair in ipairs({{'reload_time','reloadTime'},{'actions_per_round','actionsPerRound'},{'deck_capacity','deckCapacity'}})do stats[pair[2]]=object_number(e,ability,'gun_config',pair[1])end
  local shuffle=read(e,'ComponentObjectGetValue2',ability,'gun_config','shuffle_deck_when_empty');assert(type(shuffle)=='boolean','Wand shuffle unavailable');stats.shuffleDeckWhenEmpty=shuffle
  for _,pair in ipairs({{'spread_degrees','spreadDegrees'},{'speed_multiplier','speedMultiplier'},{'fire_rate_wait','fireRateWait'}})do stats[pair[2]]=object_number(e,ability,'gunaction_config',pair[1])end
  stats.gripX=number(e,sprite,'offset_x');stats.gripY=number(e,sprite,'offset_y');stats.tipX,stats.tipY=read(e,'ComponentGetValue2',hotspot,'offset');assert(finite(stats.tipX)and finite(stats.tipY),'Wand tip geometry unavailable')
  local children=read(e,'EntityGetAllChildren',entity)or {};assert(type(children)=='table'and #children<=self.max_cards+16,'Wand child bound exceeded')
  local deck,permanent,slots={}, {}, {}
  for _,child in ipairs(children)do if read(e,'EntityHasTag',child,'card_action')==true then
   assert(read(e,'EntityGetParent',child)==entity,'Wand card parent changed');local card=assert(component(e,child,'ItemComponent'),'Wand card unavailable');local spell=self:spell(child)
   local x,y=read(e,'ComponentGetValue2',card,'inventory_slot');y=y or 0;assert(integer(x)and y==0 and x>=0,'Wand card slot unavailable')
   local attached=flag(e,card,'permanently_attached');local key=(attached and 'p:'or 'd:')..x..':'..y;assert(not slots[key],'Duplicate wand card slot');slots[key]=true
   spell.slotX=x;spell.slotY=y;local list=attached and permanent or deck;list[#list+1]=spell;assert(#deck<=self.max_deck and #permanent<=self.max_permanent,'Wand card bound exceeded')
  end end
  local function ordered(a,b)return a.slotY==b.slotY and a.slotX<b.slotX or a.slotY<b.slotY end;table.sort(deck,ordered);table.sort(permanent,ordered)
  stats.deckCapacity=stats.deckCapacity-#permanent -- Construction adds these once.
  return {stats=stats,deck=deck,permanent=permanent}
 end
 function self:container(entity)
  local inventory=assert(component(e,entity,'MaterialInventoryComponent'),'Container inventory unavailable');local item=assert(component(e,entity,'ItemComponent'),'Container item unavailable')
  local counts=read(e,'ComponentGetValue2',inventory,'count_per_material_type');assert(type(counts)=='table','Container contents unavailable');local contents,seen={},{}
  for local_index,amount in pairs(counts)do
   assert(integer(local_index)and local_index>=1 and finite(amount)and amount>=0,'Invalid container quantity')
   if amount>0 then local name=read(e,'CellFactory_GetName',local_index-1);assert(type(name)=='string'and #name>0 and not seen[name],'Unknown or duplicate material');assert(read(e,'CellFactory_GetType',name)==local_index-1,'Material identity does not round trip');seen[name]=true;contents[#contents+1]={material=name,amount=amount};assert(#contents<=self.max_materials,'Material bound exceeded')end
  end
  table.sort(contents,function(a,b)return a.material<b.material end)
  local name=string_field(e,item,'item_name');local filename=read(e,'EntityGetFilename',entity)or ''
  local powder=name=='$item_powder_stash_3'or filename=='data/entities/items/pickup/powder_stash.xml'
  local colour=read(e,'GameGetPotionColorUint',entity);assert(integer(colour)and colour>=0 and colour<=4294967295,'Container colour unavailable')
  return {kind=powder and 'powder'or 'potion',contents=contents,colour=colour}
 end
 function self:dropped(entity,inlet)
  if read(e,'EntityGetIsAlive',entity)~=true or read(e,'EntityGetIsAlive',inlet)~=true then return nil,'not_alive' end
  if read(e,'EntityGetParent',entity)~=0 or read(e,'EntityGetRootEntity',entity)~=entity then return nil,'not_root' end
  local physics=component(e,entity,'SimplePhysicsComponent')or component(e,entity,'PhysicsBodyComponent')or component(e,entity,'PhysicsBody2Component');if not physics then return nil,'missing_physics' end
  local x,y=read(e,'EntityGetTransform',entity);local ix,iy=read(e,'EntityGetTransform',inlet)
  if not finite(x)or not finite(y)or not finite(ix)or not finite(iy)then return nil,'geometry_unavailable' end
  if math.abs(x-ix)+math.abs(y-iy)>=24 then return nil,'outside_inlet' end
  local item=component(e,entity,'ItemComponent');if not item then return nil,'missing_item' end
  local components={};for _,kind in ipairs({'AbilityComponent','ItemActionComponent','SpriteComponent','HotspotComponent','MaterialInventoryComponent'})do components[kind]=component(e,entity,kind,kind=='HotspotComponent'and 'shoot_pos'or nil)or 0 end
  return {entity=entity,inlet=inlet,item=item,physics=physics,x=x,y=y,components=components}
 end
 function self:revalidate(snapshot)
  local now=self:dropped(snapshot.entity,snapshot.inlet)
  if not now or now.item~=snapshot.item or now.physics~=snapshot.physics then return nil end
  for kind,id in pairs(snapshot.components or {})do if now.components[kind]~=id then return nil end end
  return now
 end
 function self:candidates(inlet)
  local x,y=read(e,'EntityGetTransform',inlet);if not finite(x)or not finite(y)then return {}end
  local found,seen={},{}
  for _,tag in ipairs({'card_action','wand','item_physics','tablet','moon_energy'})do
   local list=read(e,'EntityGetInRadiusWithTag',x,y,24,tag)or {};if type(list)~='table'or #list>self.max_candidates then return nil,'Inlet is busy; originals kept'end
   for _,entity in ipairs(list)do if not seen[entity]then seen[entity]=true;found[#found+1]=entity;if #found>self.max_candidates then return nil,'Inlet is busy; originals kept'end end end
  end
  -- Observation only: nearby wands outside the original radius are counted,
  -- never added to the capture candidates or granted acceptance.
  local outside=0;local nearby=read(e,'EntityGetInRadiusWithTag',x,y,48,'wand')
  if type(nearby)=='table'and #nearby<=self.max_candidates then for _,entity in ipairs(nearby)do
   local wx,wy=read(e,'EntityGetTransform',entity)
   if finite(wx)and finite(wy)and (wx-x)^2+(wy-y)^2>=24^2 then outside=outside+1 end
  end end
  table.sort(found);return found,nil,outside
 end
 return self
end
return M
