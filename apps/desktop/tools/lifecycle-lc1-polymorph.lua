-- Optional synthetic engine observations for the real-component composition.
-- Native permission, inspection, scheduler and service outcomes remain real.
local M={}
function M.new(e,state,input,mark)
 local id,poly=99,false
 local original={};for _,key in ipairs({'EntityGetIsAlive','EntityGetFirstComponentIncludingDisabled','EntityGetComponentIncludingDisabled','EntityGetAllChildren','EntityGetParent','EntityGetRootEntity','EntityHasTag','EntityAddTag','EntityRemoveTag','EntityAddComponent2','EntityRemoveComponent','LoadGameEffectEntityTo','EntitySetComponentIsEnabled'})do original[key]=e[key]end
 local function mapped(v)return v==id and 99 or v end
 e.EntityGetWithTag=function(tag)if id~=0 and tag==(poly and 'polymorphed'or 'player_unit')then return {id}end;return {}end
 e.IsPlayer=function(v)return id~=0 and v==id end
 e.EntityGetIsAlive=function(v)if v==99 or v==199 or v==299 or v==399 or v==499 or v==599 then return id~=0 and v==id end;return original.EntityGetIsAlive(v)end
 e.EntityGetFirstComponentIncludingDisabled=function(v,kind)if v==id and kind=='GameStatsComponent'then return 900000 end;return original.EntityGetFirstComponentIncludingDisabled(mapped(v),kind)end
 local get=e.ComponentGetValue2;e.ComponentGetValue2=function(c,k)if c==900000 then return k=='is_player'and true end;return get(c,k)end
 for _,key in ipairs({'EntityGetComponentIncludingDisabled','EntityGetAllChildren','EntityAddComponent2','EntityRemoveComponent','LoadGameEffectEntityTo','EntitySetComponentIsEnabled'})do local f=original[key];e[key]=function(v,...)return f(mapped(v),...)end end
 e.EntityGetParent=function(v)if v==999 and state.effect then return id end;local parent=original.EntityGetParent(v);return parent==99 and id or parent end
 e.EntityGetRootEntity=function(v)if v==id then return id end;local root=original.EntityGetRootEntity(v);return root==99 and id or root end
 e.EntityHasTag=function(v,tag)if v==id then if tag=='player_unit'then return not poly elseif tag=='polymorphed'then return poly elseif tag=='polymorphed_player'or tag=='polymorphed_cessation'then return false elseif tag=='polymorphable_NOT'then return state.poly end;return false end;return original.EntityHasTag(v,tag)end
 e.EntityAddTag=function(v,tag)return original.EntityAddTag(mapped(v),tag)end
 e.EntityRemoveTag=function(v,tag)return original.EntityRemoveTag(mapped(v),tag)end
 local self={}
 function self:current()return id end
 function self:apply()
  local signal=input('a2-form',true);if not signal then return end
  local next_id,kind=signal:match('^(%d+):(%a+)$');next_id=tonumber(next_id)
  assert((next_id==0 and kind=='none')or ((next_id==99 or next_id==199 or next_id==299 or next_id==399 or next_id==499 or next_id==599)and (kind=='poly'or kind=='normal')),'bounded injected local form')
  id=next_id;poly=kind=='poly';state.poly=false;state.effect=false;state.components[3]=nil;state.components[1].enabled=true;state.components[2].wait_for_kill_flag_on_death=false
  mark('a2-form-applied',signal)
 end
 return self
end
return M
