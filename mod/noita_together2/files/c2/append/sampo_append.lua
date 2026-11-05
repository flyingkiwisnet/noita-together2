-- Legacy chest/non-boss activation; canonical battle starts Kolmi separately.
local store=dofile('mods/noita_together2/files/c2/storage.lua');local api=dofile('mods/noita_together2/files/c2/hook_api.lua');local original=item_pickup
function item_pickup(item,player,name)
 if GlobalsGetValue('nt2_race_active','0')=='1'then return original(item,player,name)end
 if not api.managed()then
  if store.component(_G,item,'nt2_c2_arena')then return dofile('mods/noita_together2/files/c2/world.lua').offline(_G,player,item)end
  return original(item,player,name)
 end
 if store.component(_G,item,'nt2_c2_original')then return end
 local a=api.current(player);local own=store.component(_G,item,'nt2_c2_arena')
 if not a or not own or own.world~=a.world or own.progression~=a.progression or own.arenaToken~=a.arenaToken or own.encounterId~=a.encounterId or(a.stage~='unlocked'and a.stage~='battle')then return end
 local orbs=GameGetOrbCountThisRun();if type(orbs)~='number'or orbs%1~=0 or orbs<0 or orbs>4294967295 then return end
 local captured=api.capture({pickup={encounterId=a.encounterId,arenaToken=a.arenaToken,orbCount=orbs}},nil,item);if not captured then return end
 local saved=store.read(_G,'state');if saved and saved.world==own.world and saved.arenaToken==own.arenaToken then dofile('mods/noita_together2/files/c2/world.lua').new(_G,function()store.write(_G,'state',saved)end):pickup(saved,item)end
 api.submit(captured)
end
