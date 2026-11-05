-- Adapted from legacy spell_spitter.lua. Construction uses only a previously
-- validated fixed local catalogue plan. Each job owns only its new entities;
-- per-card work is bounded and no network/disk work occurs on the game thread.
local M={CARDS_PER_FRAME=8}
local root='mods/noita_together2/files/g3/'
local function positive(id)return type(id)=='number'and id>0 and id%1==0 end
local function component(e,id,kind,tag)
 local c;if tag==nil then c=e.EntityGetFirstComponentIncludingDisabled(id,kind)else c=e.EntityGetFirstComponentIncludingDisabled(id,kind,tag)end
 assert(positive(c),'New item component unavailable');return c
end
local function write(e,c,key,...)
 local result=e.ComponentSetValue2(c,key,...);assert(result~=false,'Item component write refused')
end
function M.new(e,codec)
 local self={}
 function self:start(item,x,y)
  local plan,error=codec:resolve(item);if not plan then return nil,error end
  if type(x)~='number'or type(y)~='number'or x~=x or y~=y or math.abs(x)>2147483 or math.abs(y)>2147483 then return nil,'Outlet coordinates unavailable'end
  local job={plan=plan,state='ready',entity=nil,owned={},at=1,cards={},x=x,y=y}
  local function own(id)assert(positive(id)and e.EntityGetIsAlive(id),'Item creation unavailable');job.owned[#job.owned+1]=id;return id end
  local function protect(id)
   local c=component(e,id,'ItemComponent');local pickable=e.ComponentGetValue2(c,'is_pickable');assert(type(pickable)=='boolean','New item pickup flag unavailable');write(e,c,'is_pickable',false);assert(e.ComponentGetValue2(c,'is_pickable')==false,'New item pickup lock failed');job.item=c;job.pickable=pickable
  end
  local function release()
   assert(e.EntityGetIsAlive(job.entity)and e.EntityGetFirstComponentIncludingDisabled(job.entity,'ItemComponent')==job.item,'New item owner changed')
   write(e,job.item,'is_pickable',job.pickable);assert(e.ComponentGetValue2(job.item,'is_pickable')==job.pickable,'New item pickup release failed');job.state='applied_session'
  end
  local function card(wand,entry,permanent)
   local id=own(e.CreateItemActionEntity(entry.actionId));local c=component(e,id,'ItemComponent')
   write(e,c,'uses_remaining',entry.uses);write(e,c,'permanently_attached',permanent);write(e,c,'inventory_slot',entry.slot,0)
   assert(e.ComponentGetValue2(c,'uses_remaining')==entry.uses and e.ComponentGetValue2(c,'permanently_attached')==permanent,'Wand card fidelity failed')
   local sx,sy=e.ComponentGetValue2(c,'inventory_slot');assert(sx==entry.slot and sy==0,'Wand slot write failed')
   e.EntityAddChild(wand,id);assert(e.EntityGetParent(id)==wand,'Wand card attachment failed');e.EntitySetComponentsWithTagEnabled(id,'enabled_in_world',false)
  end
  local function wand()
   job.entity=own(e.EntityLoad(root..'item_wand.xml',x,y));protect(job.entity)
   local ability=component(e,job.entity,'AbilityComponent');local sprite=component(e,job.entity,'SpriteComponent');local hotspot=component(e,job.entity,'HotspotComponent','shoot_pos');local s=plan.stats
   local path=assert(plan.sprite.path,'Fixed wand sprite absent')
   write(e,ability,'sprite_file',path);write(e,sprite,'image_file',path);write(e,sprite,'offset_x',s.gripX);write(e,sprite,'offset_y',s.gripY);write(e,hotspot,'offset',s.tipX,s.tipY)
   if e.EntityRefreshSprite then e.EntityRefreshSprite(job.entity,sprite)end
   write(e,ability,'ui_name',plan.name);write(e,job.item,'always_use_item_name_in_ui',plan.named);write(e,job.item,'item_name',plan.name)
   write(e,ability,'mana_max',s.manaMax);write(e,ability,'mana',s.manaMax);write(e,ability,'mana_charge_speed',s.manaChargeSpeed)
   local override=e.GlobalsGetValue and e.GlobalsGetValue('PERK_NO_MORE_SHUFFLE_WANDS','0')=='1';job.shuffle=not override and plan.shuffle
   for _,value in ipairs({{'gun_config','reload_time',s.reloadTime},{'gun_config','actions_per_round',s.actionsPerRound},{'gun_config','deck_capacity',s.deckCapacity+#plan.permanent},{'gun_config','shuffle_deck_when_empty',job.shuffle},{'gunaction_config','spread_degrees',s.spreadDegrees},{'gunaction_config','speed_multiplier',s.speedMultiplier},{'gunaction_config','fire_rate_wait',s.fireRateWait}})do
    assert(e.ComponentObjectSetValue2(ability,value[1],value[2],value[3])~=false,'Wand field write failed');assert(e.ComponentObjectGetValue2(ability,value[1],value[2])==value[3],'Wand field readback failed')
   end
   for _,entry in ipairs(plan.permanent)do job.cards[#job.cards+1]={entry=entry,permanent=true}end
   for _,entry in ipairs(plan.deck)do job.cards[#job.cards+1]={entry=entry,permanent=false}end
   job.state='constructing'
  end
  local function container()
   job.entity=own(e.EntityLoad(root..(plan.kind=='powder'and 'item_powder.xml'or 'item_potion.xml'),x,y));protect(job.entity)
   local inventory=component(e,job.entity,'MaterialInventoryComponent');e.RemoveMaterialInventoryMaterial(job.entity,'')
   local vector,expected,fractional={}, {}, false
   for _,part in ipairs(plan.contents)do local index=e.CellFactory_GetType(part.name);assert(type(index)=='number'and index>=0 and index%1==0 and e.CellFactory_GetName(index)==part.name,'Material identity changed');vector[index+1]=part.amount;expected[part.name]=part.amount;if part.amount%1~=0 then fractional=true end end
   if fractional then
    -- MATERIAL_VEC_DOUBLES can contain fractional quantities. Never pass those
    -- through AddMaterialInventoryMaterial's int argument and silently floor.
    write(e,inventory,'count_per_material_type',vector)
   else for _,part in ipairs(plan.contents)do e.AddMaterialInventoryMaterial(job.entity,part.name,part.amount)end end
   local actual=e.ComponentGetValue2(inventory,'count_per_material_type');assert(type(actual)=='table','Material readback unavailable');local count=0
   for index,amount in pairs(actual)do if amount~=0 then local name=e.CellFactory_GetName(index-1);assert(expected[name]==amount,'Container mixture changed');expected[name]=nil;count=count+1 end end
   assert(next(expected)==nil and count==#plan.contents,'Container contents incomplete');release()
  end
  function job:step(authorized)
   if self.state=='applied_session'or self.state=='uncertain'then return self.state end
   if authorized~=true then return 'deferred'end
   local ok=pcall(function()
    if self.state=='ready'then
     if plan.kind=='wand'then wand()
     elseif plan.kind=='spell'then self.entity=own(e.CreateItemActionEntity(plan.actionId,x,y));protect(self.entity);write(e,self.item,'uses_remaining',plan.uses);assert(e.ComponentGetValue2(self.item,'uses_remaining')==plan.uses,'Spell uses changed');release()
     elseif plan.kind=='potion'or plan.kind=='powder'then container()
     else self.entity=own(e.EntityLoad(assert(plan.object.path,'Fixed object path absent'),x,y));protect(self.entity);release()end
    end
    if self.state=='constructing'then
     assert(e.EntityGetIsAlive(self.entity),'Constructing wand disappeared')
     for _=1,M.CARDS_PER_FRAME do local next=self.cards[self.at];if not next then release();break end;card(self.entity,next.entry,next.permanent);self.at=self.at+1 end
     if self.at>#self.cards and self.state=='constructing'then release()end
    end
   end)
   if not ok then
    -- Only entities created by THIS job are touched. An ambiguous effect is
    -- never requeued/refunded because cleanup was attempted or succeeded.
    self.state='uncertain';for i=#self.owned,1,-1 do local id=self.owned[i];pcall(function()if e.EntityGetIsAlive(id)then e.EntityKill(id)end end)end
    self.reason='Item construction unresolved; query the original transfer.'
   end
   return self.state
  end
  return job
 end
 return self
end
return M
