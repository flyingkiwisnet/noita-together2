-- Legacy white pixel-font nameplates, bound only by authenticated roster names.
local M={}
local text=dofile('mods/noita_together2/files/g3/a2_text.lua')
function M.display(name)
 local n=text.length(name);if not n or n==0 or not name:find('[^ ]')then return 'Player',6 end
 local shortened=false;while n>32 do name=text.backspace(name);n=n-1;shortened=true end
 if shortened then name=name..'...';n=n+3 end;return name,n
end
function M.new(engine)
 local e=engine or _G;local labels={}
 local function alive(id)return id and e.EntityGetIsAlive(id)end
 local self={}
 function self:remove(parent)local item=labels[parent];if item and alive(item.entity)then e.EntityKill(item.entity)end;labels[parent]=nil end
 function self:sync(parent,name)
  if not alive(parent)then self:remove(parent);return false end
  if type(e.EntityCreateNew)~='function'or type(e.EntityAddComponent)~='function'then return false end
  local shown,length=M.display(name);local item=labels[parent]
  if not item or not alive(item.entity)then
   local child=e.EntityCreateNew('player_name');e.EntitySetName(child,'player_name')
   e.EntityAddComponent(child,'InheritTransformComponent',{_tags='enabled_in_world',use_root_parent='1',only_position='1'})
   local sprite=e.EntityAddComponent(child,'SpriteComponent',{_tags='enabled_in_world',image_file='data/fonts/font_pixel_white.xml',emissive='1',is_text_sprite='1',offset_x=tostring(length*1.9),offset_y='-6',alpha='1',update_transform='1',update_transform_rotation='0',text=shown,has_special_scale='1',special_scale_x='0.7',special_scale_y='0.7',z_index='-9000'})
   if not sprite then e.EntityKill(child);return false end
   e.EntityAddChild(parent,child);item={entity=child,sprite=sprite,name=shown};labels[parent]=item
  elseif item.name~=shown then e.ComponentSetValue2(item.sprite,'text',shown);e.ComponentSetValue2(item.sprite,'offset_x',length*1.9);item.name=shown end
  -- Position-only inheritance keeps facing/rotation of the body out of text.
  -- Cosmetics own their own children; no body/cape/emote component is replaced.
  return true
 end
 return self
end
return M
