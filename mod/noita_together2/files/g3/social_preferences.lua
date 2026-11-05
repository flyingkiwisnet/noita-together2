-- Local Noita settings, never server statistics consent or a play grant.
local M={prefix='noita_together2.',defaults={NT_HINTS=true,NT_NO_STAT_PROGRESS=false,NT_FOLLOW_DEFAULT=false,NT_GHOST_OPACITY=1,NT_ENABLE_LOGGER=true,NT_SHOW_WUOTE_FRIDGE=false,NT_SHOW_EMOTES=true,NT_EMOTE_BINDING='key_code,20,mouse_code,joystick_code'}}
function M.get(e,key)
 local default=M.defaults[key];if default==nil then return nil end
 local ok,v=pcall(function()return e.ModSettingGet and e.ModSettingGet(M.prefix..key)end)
 if not ok or type(v)~=type(default)then return default end
 if key=='NT_GHOST_OPACITY'then return v==v and math.max(0,math.min(1,v))or default end
 return v
end
function M.set(e,key,v)
 if type(v)~=type(M.defaults[key])then return false end
 if key=='NT_GHOST_OPACITY'and (v~=v or v<0 or v>1)then return false end
 if not e.ModSettingSet then return false end
 e.ModSettingSet(M.prefix..key,v);if e.ModSettingSetNextValue then e.ModSettingSetNextValue(M.prefix..key,v,false)end;return true
end
function M.progress(e)
 if not (e.GameHasFlagRun and e.GameAddFlagRun and e.GameRemoveFlagRun)then return end
 for _,flag in ipairs({'no_progress_flags_perk','no_progress_flags_animal','no_progress_flags_action'})do
  local marker='nt2_a3_owned_'..flag
  if M.get(e,'NT_NO_STAT_PROGRESS')then
   if not e.GameHasFlagRun(flag)then e.GameAddFlagRun(marker);e.GameAddFlagRun(flag)end
  elseif e.GameHasFlagRun(marker)then e.GameRemoveFlagRun(flag);e.GameRemoveFlagRun(marker)end
 end
end
return M
