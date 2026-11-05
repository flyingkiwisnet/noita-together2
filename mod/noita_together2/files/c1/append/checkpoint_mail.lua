-- Legacy co_op_mail checkpoint only. Existing loot/mail placement stays owned.
local original=spawn_hp
function spawn_hp(x,y)
 original(x,y)
 local checkpoint=EntityCreateNew('nt2_c1_checkpoint')
 EntityAddComponent2(checkpoint,'CollisionTriggerComponent',{width=20,height=100,radius=64,destroy_this_entity_when_triggered=false,required_tag='player_unit'})
 EntityAddComponent2(checkpoint,'LuaComponent',{script_collision_trigger_hit='mods/noita_together2/files/c1/scripts/checkpoint.lua',execute_every_n_frame=1})
 EntitySetTransform(checkpoint,x,y)
end
