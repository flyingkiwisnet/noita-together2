-- Own only the composer's input lease. Production protection calls before_hold
-- synchronously before it captures original component flags.
local M={}
function M.inherited(e,player)
 local result={};for _,id in ipairs(e.EntityGetComponentIncludingDisabled(player,'VariableStorageComponent')or {})do
  if e.ComponentGetValue2(id,'name')=='nt2_a2_composer_lock'and e.ComponentGetValue2(id,'value_bool')==true then result[#result+1]=id end
 end;return result
end
function M.recover(e,player,playing,lease)
 if not playing or not player or not e.EntityGetIsAlive(player)then return end
 local c=e.EntityGetFirstComponentIncludingDisabled(player,'ControlsComponent');local d=e.EntityGetFirstComponentIncludingDisabled(player,'DamageModelComponent')
 if not c or not d or (e.ComponentGetValue2(d,'hp')or -1)<0 then return end
 for _,id in ipairs(e.EntityGetComponentIncludingDisabled(player,'VariableStorageComponent')or {})do if e.ComponentGetValue2(id,'name')=='nt2_g3_recovery'then return end end
 local inherited=M.inherited(e,player);local owned
 if lease and lease.player==player and lease.controls==c and lease.damage==d then
  for _,id in ipairs(inherited)do if id==lease.marker then owned=id end end
 end
 -- Reclaim orphaned/saved markers only. The live renderer still owns its
 -- active typing lease; restoring it here would enable controls on the next
 -- frame while its bank/composer window remains open.
 for _,id in ipairs(inherited)do if id~=owned then if not owned then e.ComponentSetValue2(c,'enabled',true)end;e.EntityRemoveComponent(player,id)end end
end
function M.new(e)
 local self={lease=nil}
 local function protected(player)
  for _,id in ipairs(e.EntityGetComponentIncludingDisabled(player,'VariableStorageComponent') or {})do
   if e.ComponentGetValue2(id,'name')=='nt2_g3_recovery' then return true end
  end;return false
 end
 function self:open(player,playing)
  if self.lease then return true end
  if not playing or not player or not e.EntityGetIsAlive(player) or protected(player) then return false end
  local c=e.EntityGetFirstComponentIncludingDisabled(player,'ControlsComponent');local d=e.EntityGetFirstComponentIncludingDisabled(player,'DamageModelComponent')
  if not c or not d or e.ComponentGetValue2(c,'enabled')~=true or (e.ComponentGetValue2(d,'hp') or -1)<0 then return false end
  local marker=e.EntityAddComponent2(player,'VariableStorageComponent',{name='nt2_a2_composer_lock',value_bool=true})
  if not marker or marker==0 then return false end
  self.lease={player=player,controls=c,damage=d,marker=marker};e.ComponentSetValue2(c,'enabled',false);return true
 end
 function self:close(playing)
  local l=self.lease;if not l then return end;self.lease=nil
  if not e.EntityGetIsAlive(l.player) then return end
  local owned=false;for _,id in ipairs(e.EntityGetComponentIncludingDisabled(l.player,'VariableStorageComponent') or {})do if id==l.marker and e.ComponentGetValue2(id,'name')=='nt2_a2_composer_lock' then owned=true end end
  if owned then
   local damage=e.EntityGetFirstComponentIncludingDisabled(l.player,'DamageModelComponent')
   local hp=damage and e.ComponentGetValue2(damage,'hp');local dead=type(hp)=='number'and hp<0
   if playing and not protected(l.player) and e.EntityGetFirstComponentIncludingDisabled(l.player,'ControlsComponent')==l.controls and damage==l.damage and hp and hp>=0 then e.ComponentSetValue2(l.controls,'enabled',true)end
   -- An unavailable/replaced damage component is not death. Retain recovery
   -- evidence for the entry's next owned hold/current-subject recovery.
   if playing and damage==l.damage or protected(l.player) or dead then e.EntityRemoveComponent(l.player,l.marker)end
  end
 end
 function self:before_hold(player)if self.lease and self.lease.player==player then self:close(true)end end
 return self
end
return M
