local api=dofile('mods/noita_together2/files/race/hook_api.lua')
function death(...)api.player_death(GetUpdatedEntityID(),false)end
-- Documented LuaComponent Message_DamageReceived callback. No waits or effects.
function damage_received(damage,message,responsible,is_fatal,...)
 if is_fatal==true then api.player_death(GetUpdatedEntityID(),true)end
end
