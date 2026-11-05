local nt2_ok,NT2_C1=pcall(dofile,'mods/noita_together2/files/g3/c1_hook_api.lua')
if not nt2_ok then NT2_C1=nil end
-- Legacy collision geometry; current local actor/permission owns the checkpoint.
function collision_trigger(player)
 if not NT2_C1 then return end
 local x,y=EntityGetTransform(GetUpdatedEntityID())
 NT2_C1:checkpoint(player,x,y)
end
