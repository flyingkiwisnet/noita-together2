-- Owned local/ghost presentation over fixed legacy assets. No authority here.
local M={};local catalogue=dofile('mods/noita_together2/files/g3/social_catalogue.lua')
local journal=dofile('mods/noita_together2/files/g3/social_restore.lua');M.recover=journal.recover
local palette={red={0xff9e2812,0xffc83d35},orange={0xffc06f00,0xfff57d00},yellow={0xffc1a515,0xfff3da00},green={0xff0f8339,0xff2f9e3c},blue={0xff1a4fac,0xff3579c9}}
local cosmetic={hat={tag='player_hat2',file='player_hat2',emote='emotes_gfx_player_hat2',z=0.58},amulet={tag='player_amulet',file='player_amulet',emote='emotes_gfx_player_amulet',z=0.59},gem={tag='player_amulet_gem',file='player_amulet_gem',emote='emotes_gfx_player_amulet_gem',z=0.58}}
local function call(e,key,...)local fn=e[key];if type(fn)~='function'then return nil end;local ok,a,b=pcall(fn,...);if ok then return a,b end;return nil end
local function valid_id(id)return type(id)=='number'and id>0 and id<math.huge and id%1==0 end
local function component_value(e,id,key)if valid_id(id)then return call(e,'ComponentGetValue2',id,key)end;return nil end
local function alive(e,id)return valid_id(id)and call(e,'EntityGetIsAlive',id)==true end
local function first(e,id,kind,tag)
 if not valid_id(id)then return nil end
 local c;if tag==nil then c=call(e,'EntityGetFirstComponentIncludingDisabled',id,kind)elseif type(tag)=='string'then c=call(e,'EntityGetFirstComponentIncludingDisabled',id,kind,tag)end
 return valid_id(c)and c or nil
end
local function children(e,id)return valid_id(id)and call(e,'EntityGetAllChildren',id)or {}end
local function named(e,id,name)for _,child in ipairs(children(e,id))do if alive(e,child)and call(e,'EntityGetName',child)==name then return child end end end
local function body(e,id,local_player)return first(e,id,'SpriteComponent',local_player and 'lukki_disable'or nil)end
function M.new(e)
 local self={active={},colours={},particles={}};local owned={};local serial=0
 local function claim(id)
  if not alive(e,id)then return nil end
  serial=serial+1;local tag='nt2_a3_owned_'..tostring(self):gsub('[^%w]','')..'_'..serial;call(e,'EntityAddTag',id,tag);owned[id]=tag;return id
 end
 local function member(entity,component,kind)
  if not valid_id(component)or not alive(e,entity)then return false end
  for _,c in ipairs(call(e,'EntityGetComponentIncludingDisabled',entity,kind)or {})do if c==component then return true end end;return false
 end
 local function refresh(entity,component,key)
  if key=='image_file'and member(entity,component,'SpriteComponent')then call(e,'EntityRefreshSprite',entity,component)end
 end
 local function write(lease,entity,c,kind,key,value)
  if not c or not member(entity,c,kind)then return false end
  local old=component_value(e,c,key);if old==nil then return false end
  for _,s in ipairs(lease.fields)do if s.component==c and s.key==key then
   if old==s.ours and (not lease.persist or journal.update(e,s,value))then if old~=value then call(e,'ComponentSetValue2',c,key,value);refresh(entity,c,key)end;s.ours=value;return true end;return false
  end end
  local s={entity=entity,component=c,kind=kind,key=key,original=old,ours=value};if lease.persist and not journal.record(e,s)then return end
  lease.fields[#lease.fields+1]=s;if old~=value then call(e,'ComponentSetValue2',c,key,value);refresh(entity,c,key)end;return true
 end
 local function restore(lease)
  for i=#lease.fields,1,-1 do local s=lease.fields[i]
   if member(s.entity,s.component,s.kind)and component_value(e,s.component,s.key)==s.ours then call(e,'ComponentSetValue2',s.component,s.key,s.original);refresh(s.entity,s.component,s.key)end
   if lease.persist then journal.finish(e,s)end
  end
 end
 local function own_component(lease,id,kind,values)
  local c=call(e,'EntityAddComponent2',id,kind,values);if valid_id(c)then lease.components[#lease.components+1]={entity=id,component=c,kind=kind};if kind=='SpriteComponent'then refresh(id,c,'image_file')end;return c end
 end
 local function cleanup_components(lease)
  for _,s in ipairs(lease.components)do if member(s.entity,s.component,s.kind)then call(e,'EntityRemoveComponent',s.entity,s.component)end end
 end
 local function variant(robe)return catalogue.root..'emotes_gfx_player'..(robe=='purple'and ''or '_'..robe)..'.xml'end
 local function owned_at(id,parent)
  local tag=owned[id];return tag and alive(e,id)and call(e,'EntityHasTag',id,tag)==true and (not parent or call(e,'EntityGetParent',id)==parent)
 end
 local function remove_owned(id,parent)
  if not valid_id(id)then return end
  if owned_at(id,parent)then call(e,'EntityKill',id)end;owned[id]=nil
 end
 function self:stop(entity)
  local a=self.active[entity];if not a then return end;self.active[entity]=nil
  restore(a)
  if a.cape and alive(e,a.cape)and call(e,'EntityGetParent',a.cape)==a.holder and alive(e,a.cape_parent)then call(e,'EntityRemoveFromParent',a.cape);call(e,'EntityAddChild',a.cape_parent,a.cape)end
  cleanup_components(a);remove_owned(a.effect);remove_owned(a.holder,entity)
 end
 function self:clear_colour(entity)
  self:stop(entity);local c=self.colours[entity];if c then restore(c);cleanup_components(c);remove_owned(c.owned_cape,entity);self.colours[entity]=nil end
  for _,id in ipairs(self.particles[entity]or {})do remove_owned(id,entity)end;self.particles[entity]=nil
 end
 function self:colour(entity,robe,flags,local_player,animate)
  if not alive(e,entity)or not catalogue.robe(robe)then return false end
  local c=self.colours[entity];local active=self.active[entity];local cape_parent=active and active.cape== (c and c.cape)and active.holder or entity
  local cape_current=c and alive(e,c.cape)and call(e,'EntityGetParent',c.cape)==cape_parent and (not c.owned_cape or owned_at(c.owned_cape,cape_parent))
  if c and c.robe==robe and c.hat==flags.hat and c.amulet==flags.amulet and c.gem==flags.gem and (local_player or cape_current)then return true end
  local previous=c and c.robe or 'purple';local owned_cape=c and c.owned_cape
  if owned_cape and not owned_at(owned_cape,cape_parent)then owned[owned_cape]=nil;owned_cape=nil end
  -- Colour flags may change during a dance: update its variant without a restart.
  if c then restore(c);cleanup_components(c)end
  c={fields={},components={},persist=local_player,robe=robe,hat=flags.hat,amulet=flags.amulet,gem=flags.gem,owned_cape=owned_cape};self.colours[entity]=c
  local b=body(e,entity,local_player)
  if robe~='purple'then write(c,entity,b,'SpriteComponent','image_file',catalogue.root..'player_'..robe..'.xml')end
  local arm=named(e,entity,'arm_r');if arm and robe~='purple'then write(c,arm,first(e,arm,'SpriteComponent','with_item'),'SpriteComponent','image_file',catalogue.root..'player_arm_'..robe..'.xml')end
  local cape=named(e,entity,'cape')or active and active.cape
  if not alive(e,cape)then cape=nil end
  if not cape and not local_player then
   local x,y=call(e,'EntityGetTransform',entity);cape=call(e,'EntityLoad',catalogue.root..'remote_cape.xml',x,y)
   if alive(e,cape)then claim(cape);call(e,'EntityAddChild',entity,cape);c.owned_cape=cape end
  end
  if not alive(e,cape)then cape=nil end
  c.cape=cape;if cape then local v=first(e,cape,'VerletPhysicsComponent');if v then c.cape_base={component_value(e,v,'cloth_color'),component_value(e,v,'cloth_color_edge')}end end
  if cape and palette[robe]then local v=first(e,cape,'VerletPhysicsComponent');write(c,cape,v,'VerletPhysicsComponent','cloth_color',palette[robe][1]);write(c,cape,v,'VerletPhysicsComponent','cloth_color_edge',palette[robe][2])end
  if not local_player then for key,spec in pairs(cosmetic)do if flags[key]then own_component(c,entity,'SpriteComponent',{_tags='nt2_a3_cosmetic,'..spec.tag,image_file='data/enemies_gfx/'..spec.file..'.xml',z_index=spec.z,alpha=1,visible=true,rect_animation='stand',next_rect_animation='stand'})end end end
  if active then
   if active.sprite and member(active.holder,active.sprite,'SpriteComponent')and component_value(e,active.sprite,'image_file')~=variant(robe)then call(e,'ComponentSetValue2',active.sprite,'image_file',variant(robe));refresh(active.holder,active.sprite,'image_file')end
   -- A destroyed owned cape can be recreated while a dance remains active.
   -- Only reparent from this actor; foreign ownership changes are preserved.
   if cape and catalogue.entries[active.id].cape and alive(e,active.holder)and call(e,'EntityGetParent',cape)==entity then active.cape=cape;active.cape_parent=entity;call(e,'EntityRemoveFromParent',cape);call(e,'EntityAddChild',active.holder,cape)end
   self:overlays(entity,active,flags,local_player)
   -- New ordinary cosmetic sprites stay hidden until the current dance ends.
   for _,s in ipairs(c.components)do if s.kind=='SpriteComponent'then write(active,s.entity,s.component,'SpriteComponent','visible',false)end end
  end
  if animate and previous~=robe then
   local x,y=call(e,'EntityGetTransform',entity);local p=call(e,'EntityLoad',catalogue.root..'effects/color_swap_particles.xml',x,y)
   if alive(e,p)then claim(p);call(e,'EntityAddTag',p,previous);call(e,'EntityAddChild',entity,p);local list=self.particles[entity]or {};self.particles[entity]=list;if #list>=8 then remove_owned(table.remove(list,1),entity)end;list[#list+1]=p end
  end
  return true
 end
 function self:opacity(entity,age_frames)
  local b=body(e,entity,false);local alpha=tonumber(component_value(e,b,'alpha'))or 1;alpha=math.max(0,math.min(1,alpha))
  local c=self.colours[entity];if c then
   for _,s in ipairs(c.components)do if s.kind=='SpriteComponent'and member(s.entity,s.component,s.kind)then call(e,'ComponentSetValue2',s.component,'alpha',alpha)end end
   if c.cape and c.cape_base then local v=first(e,c.cape,'VerletPhysicsComponent');for index,key in ipairs({'cloth_color','cloth_color_edge'})do local base=palette[c.robe]and palette[c.robe][index]or c.cape_base[index];if type(base)=='number'then local a=math.floor(base/16777216)%256;write(c,c.cape,v,'VerletPhysicsComponent',key,base%16777216+math.floor(a*alpha)*16777216)end end end
  end
  local a=self.active[entity];if not a then return end;a.alpha=alpha
  for _,s in ipairs(a.components)do if s.entity==a.holder and s.kind=='SpriteComponent'and member(s.entity,s.component,s.kind)then call(e,'ComponentSetValue2',s.component,'alpha',alpha)end end
  if alpha==0 and a.effect then remove_owned(a.effect);a.effect=nil
  elseif alpha>0 and not a.effect and catalogue.entries[a.id].effect then
   local x,y=call(e,'EntityGetTransform',entity);a.effect=claim(call(e,'EntityLoad',catalogue.entries[a.id].effect,x,y));if alive(e,a.effect)then call(e,'EntityAddComponent2',a.effect,'VariableStorageComponent',{name='nt2_a3_age_frames',value_int=age_frames or 0})end
  end
 end
 function self:compatible(entity)
  local b=body(e,entity,true);if not journal.supported(e)or not b or not alive(e,entity)or call(e,'ComponentGetIsEnabled',b)==false then return false end
  if component_value(e,b,'visible')~=false then return true end
  local a=self.active[entity];if a then for _,s in ipairs(a.fields)do if s.entity==entity and s.component==b and s.key=='visible'and s.original==true and s.ours==false then return true end end end
  return false
 end
 function self:overlays(entity,a,flags,local_player)
  a.overlays=a.overlays or {};for key,cos in pairs(cosmetic)do
   local component=first(e,entity,'SpriteComponent',cos.tag);local unlocked=flags[key]==true
   if local_player then unlocked=call(e,'HasFlagPersistent',key=='hat'and 'secret_hat'or key=='amulet'and 'secret_amulet'or 'secret_amulet_gem')==true end
   local overlay=a.overlays[key]
   if not unlocked and overlay then if member(a.holder,overlay,'SpriteComponent')then call(e,'EntityRemoveComponent',a.holder,overlay)end;a.overlays[key]=nil end
   if unlocked and component and call(e,'ComponentGetIsEnabled',component)~=false and (overlay or component_value(e,component,'visible')~=false)then
    write(a,entity,component,'SpriteComponent','visible',false);write(a,entity,component,'SpriteComponent','rect_animation','stand')
    if not overlay then a.overlays[key]=own_component(a,a.holder,'SpriteComponent',{image_file=catalogue.root..cos.emote..'.xml',alpha=a.alpha,z_index=cos.z,rect_animation=a.id,next_rect_animation=a.id})end
   end
  end
 end
 function self:start(entity,id,local_player,age_frames)
  local spec=catalogue.entries[id];if not spec or not alive(e,entity)then return false end
  self:stop(entity)
  local b=body(e,entity,local_player);if not b then return false end
  if local_player then if not self:compatible(entity)then return false end;local controls=first(e,entity,'ControlsComponent');local damage=first(e,entity,'DamageModelComponent');local hp=damage and component_value(e,damage,'hp')
   if not controls or component_value(e,controls,'enabled')~=true or not damage or type(hp)~='number'or hp<0 then return false end
   for _,v in ipairs(call(e,'EntityGetComponentIncludingDisabled',entity,'VariableStorageComponent')or {})do if component_value(e,v,'name')=='nt2_g3_recovery'then return false end end
  end
  local x,y=call(e,'EntityGetTransform',entity);local holder=call(e,'EntityLoad',catalogue.root..(local_player and 'holder_local.xml'or 'holder.xml'),x,y);if not alive(e,holder)then return false end
  claim(holder);call(e,'EntityAddChild',entity,holder)
  local a={id=id,fields={},components={},persist=local_player,holder=holder,alpha=component_value(e,b,'alpha')or 1};self.active[entity]=a
  local robe=(self.colours[entity]or {}).robe or 'purple'
  a.sprite=own_component(a,holder,'SpriteComponent',{_tags='nt2_a3_emote',image_file=variant(robe),alpha=a.alpha,z_index=0.6,rect_animation=id,next_rect_animation=id})
  if not a.sprite then self:stop(entity);return false end
  if not write(a,entity,b,'SpriteComponent','visible',false)then self:stop(entity);return false end;write(a,entity,b,'SpriteComponent','rect_animation','stand')
  local arm=named(e,entity,'arm_r');if arm then write(a,arm,first(e,arm,'SpriteComponent','with_item'),'SpriteComponent','visible',false)end
  self:overlays(entity,a,self.colours[entity]or {},local_player)
  local inventory=first(e,entity,'Inventory2Component');local item=inventory and component_value(e,inventory,'mActualActiveItem')or named(e,entity,'held_item')
  if alive(e,item)then write(a,item,first(e,item,'SpriteComponent'),'SpriteComponent','visible',false)end
  if local_player then local motion=first(e,entity,'CharacterPlatformingComponent');write(a,entity,motion,'CharacterPlatformingComponent','run_velocity',0);write(a,entity,motion,'CharacterPlatformingComponent','mouse_look',false);a.motion=motion end
  if spec.effect and (local_player or a.alpha>0)then a.effect=claim(call(e,'EntityLoad',spec.effect,x,y));if alive(e,a.effect)then call(e,'EntityAddComponent2',a.effect,'VariableStorageComponent',{name='nt2_a3_age_frames',value_int=age_frames or 0})end end
  local cape=named(e,entity,'cape');if cape and spec.cape then a.cape=cape;a.cape_parent=call(e,'EntityGetParent',cape);call(e,'EntityRemoveFromParent',cape);call(e,'EntityAddChild',holder,cape)end
  return true
 end
 function self:aim(entity)
  local a=self.active[entity];if not a or not a.motion then return end
  local c=first(e,entity,'ControlsComponent');local x,y;if c then x,y=component_value(e,c,'mMouseDelta')end;if not x then return end
  if x>1 or (y or 0)>1 then for _,s in ipairs(a.fields)do if s.key=='mouse_look'and s.original==true then write(a,entity,a.motion,'CharacterPlatformingComponent','mouse_look',true)end end end
 end
 function self:clear()local ids={};for id in pairs(self.active)do ids[id]=true end;for id in pairs(self.colours)do ids[id]=true end;for id in pairs(ids)do self:clear_colour(id)end end
 return self
end
return M
