-- Adapted legacy factory, composed after current mail/checkpoint appends.
local store=dofile('mods/noita_together2/files/c2/storage.lua')
local api=dofile('mods/noita_together2/files/c2/hook_api.lua');local original=spawn_items
function spawn_items(x,y)
 if GlobalsGetValue('nt2_race_active','0')=='1'then return original(x,y)end
 local progression=store.progression(_G);local old,invalid=store.read(_G,'arena_created');if invalid then return end
 if old and old.progression==progression then return end
 if not api.managed()then return original(x,y)end
 local anchor=EntityCreateNew('NT2 arena generation');EntityAddTag(anchor,'nt2_c2_arena_group');EntitySetTransform(anchor,x,y)
 local group={group=anchor,progression=progression};store.mark(_G,anchor,'nt2_c2_arena',group);store.write(_G,'arena_created',group)
 local function spawn(path,role,px,py)
  local id=EntityLoad(path,px,py);if not id or id==0 then return end
  EntityAddTag(id,'nt2_c2_owned');store.mark(_G,id,'nt2_c2_arena',{group=anchor,progression=progression,role=role})
 end
 spawn('data/entities/animals/boss_centipede/boss_centipede.xml','boss',x,y)
 if not GameHasFlagRun('ending_game_completed')then
  spawn('mods/noita_together2/files/c2/entities/sampo/disabled_sampo.xml','disabled',x,y+80)
  spawn('mods/noita_together2/files/c2/entities/sampo/sampo_aura.xml','aura',x,y+80)
 end
 spawn('data/entities/animals/boss_centipede/reference_point.xml','reference',x,y)
end
