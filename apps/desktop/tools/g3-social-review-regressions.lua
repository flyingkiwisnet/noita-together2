-- R07/R08 coordinator reproductions, now positive acceptance against actual hook.
local root=assert(os.getenv('NT2_G3_MODULES'))
local validate=dofile(root..'/validation.lua');local module=dofile(root..'/hud_poi_local.lua')
local hud,pois={},{}
local state=module.new({time=function() return 0 end,clear_hud=function() hud={} end,clear_pois=function() pois={} end,
 hud=function(actor,hp,max) hud[actor]={hp=hp,max=max} end,remove_hud=function(actor) hud[actor]=nil end,
 poi=function(sequence) pois[sequence]=true end,remove_poi=function(sequence) pois[sequence]=nil end},validate)
assert(state:bind('scope',1,{'peer'}));assert(state:receive_hud('scope',1,'peer',1,3000,2000,false,2))
assert(hud.peer.hp==3000 and hud.peer.max==2000)
assert(state:begin_snapshot('scope',1,'33'));local first={}
for i=1,32 do first[i]={sequence=tostring(i),actor='peer',text='marker',x=i,y=i} end
assert(state:page('scope',1,'33','32',first))
assert(state:page('scope',1,'34','34',{{sequence='33',actor='peer',text='tail',x=33,y=33},{sequence='34',actor='peer',text='new tail',x=34,y=34}}))
assert(state.playing and hud.peer and pois['33'] and not pois['34'])
print('PASS R07 service-valid overmax HUD renders actual numbers')
print('PASS R08 high33 grows34: prefix33 completes without clearing HUD/playing; marker34 deferred')
