-- Test-only Noita engine facts. Production C2 Lua/native/app/service remain real.
-- Owned rows/components are injected and saved as XML-safe fixture Globals.
local M={}
function M.new(e,state,forms,input,mark,files)
 local json=e.dofile('mods/noita_together2/files/g3/bank_json.lua')
 local restricted=assert(loadfile(assert(os.getenv('NT2_C3_REPO'))..'/apps/desktop/tools/c1-restricted-context.lua'))()
 local function hook()return restricted(e,function(path)local f=assert(io.open(files..'/'..path,'rb'));local b=f:read('*a');f:close();return b end)end
 local base={};for _,k in ipairs({'EntityGetIsAlive','EntityGetTransform','EntitySetTransform','EntityGetFirstComponentIncludingDisabled','EntityGetComponentIncludingDisabled','EntityGetWithTag','EntityHasTag','EntityAddTag','EntityAddComponent2','EntityRemoveComponent','ComponentGetValue2','ComponentSetValue2','EntityKill','EntityLoad','EntityGetAllChildren','EntityGetParent','EntitySetComponentIsEnabled','ComponentGetIsEnabled','ComponentObjectGetValue2','ComponentObjectSetValue2','LoadGameEffectEntityTo'})do base[k]=e[k]end
 local rows,parts={},{};local next_id=930001;local stomach=930000;parts[stomach]={ingestion_size=0};local progression=0;local px,py,goalx,goaly,release_position;local arena_x=tonumber(os.getenv('NT2_C3_X'))or 0;local restored=false;local updated;local effects={};local rocks=0;local music=0;local guards=0;local notices=0;local groups=0;local freeze_at
 local function current()return forms and forms:current()or 99 end
 local function part(id,kind,data)next_id=next_id+1;parts[next_id]=data;rows[id].parts[kind]=rows[id].parts[kind]or{};table.insert(rows[id].parts[kind],next_id);return next_id end
 local function spawn(x,y)next_id=next_id+1;rows[next_id]={x=x,y=y,parts={},tags={},parent=0};return next_id end
 local function pack(value)
  if type(value)=='number'and value%1~=0 then return{nt2FixtureFloat=string.format('%.17g',value)}end
  if type(value)~='table'then return value end;local out={};for key,v in pairs(value)do out[key]=pack(v)end;return out
 end
 local function unpack_saved(value)
  if type(value)~='table'then return value end
  if value.nt2FixtureFloat then return assert(tonumber(value.nt2FixtureFloat))end
  local out={};for key,v in pairs(value)do out[key]=unpack_saved(v)end;return out
 end
 local function save()
  local entities={};for id,row in pairs(rows)do local p={};for _,ids in pairs(row.parts)do for _,c in ipairs(ids)do p[#p+1]={id=c,data=parts[c]}end end;entities[#entities+1]={id=id,row=row,components=p}end
  e.GlobalsSetValue('nt2_c2_fixture_engine',assert(json.encode_saved(pack({entities=entities,nextId=next_id,progression=progression,effects=effects,rocks=rocks,music=music,guards=guards,stomach=parts[stomach].ingestion_size}),{bytes=131072,nodes=16384})))
 end
 local function restore()
  if restored then return end;restored=true
  local saved=json.decode_saved(e.GlobalsGetValue('nt2_c2_fixture_engine',''),{bytes=131072,nodes=16384})
  if saved then saved=unpack_saved(saved);parts[stomach]={ingestion_size=saved.stomach or 0};next_id=saved.nextId;progression=saved.progression;effects=saved.effects;rocks=saved.rocks;music=saved.music;guards=saved.guards;for _,v in ipairs(saved.entities)do rows[v.id]=v.row;for _,c in ipairs(v.components)do parts[c.id]=c.data end end end
 end
 e.SessionNumbersGetValue=function(key)assert(key=='NEW_GAME_PLUS_COUNT');return tostring(progression)end
 e.EntityGetIsAlive=function(id)return rows[id]~=nil or base.EntityGetIsAlive(id)end
 e.EntityGetTransform=function(id)if id==current()and px then return px,py end;if rows[id]then return rows[id].x,rows[id].y end;return base.EntityGetTransform(id)end
 e.EntitySetTransform=function(id,x,y)if id==current()and px then px,py=x,y;return end;if rows[id]then rows[id].x=x;rows[id].y=y else base.EntitySetTransform(id,x,y)end end
 e.EntityGetComponentIncludingDisabled=function(id,kind)if id==current()and kind=='IngestionComponent'then return{stomach}end;if rows[id]then return rows[id].parts[kind]or{}end;return base.EntityGetComponentIncludingDisabled(id,kind)end;e.EntityGetComponent=e.EntityGetComponentIncludingDisabled
 e.EntityGetFirstComponentIncludingDisabled=function(...)local id,kind,tag=...;if id==current()and kind=='IngestionComponent'then return stomach end;if rows[id]then for _,c in ipairs(rows[id].parts[kind]or{})do if not tag or parts[c]._tags==tag then return c end end;return nil end;return base.EntityGetFirstComponentIncludingDisabled(...)or nil end;e.EntityGetFirstComponent=e.EntityGetFirstComponentIncludingDisabled
 e.EntityGetWithTag=function(tag)local out=base.EntityGetWithTag(tag)or{};for id,row in pairs(rows)do if row.tags[tag]then out[#out+1]=id end end;return out end
 e.EntityHasTag=function(id,tag)if rows[id]then return rows[id].tags[tag]or false end;return base.EntityHasTag(id,tag)end
 e.EntityAddTag=function(id,tag)if rows[id]then rows[id].tags[tag]=true else base.EntityAddTag(id,tag)end end
 e.EntityAddComponent2=function(id,kind,data)if rows[id]then return part(id,kind,data)end;return base.EntityAddComponent2(id,kind,data)end;e.EntityAddComponent=e.EntityAddComponent2
 e.EntityRemoveComponent=function(id,c)if rows[id]then for _,ids in pairs(rows[id].parts)do for i=#ids,1,-1 do if ids[i]==c then table.remove(ids,i)end end end;parts[c]=nil else base.EntityRemoveComponent(id,c)end end
 e.ComponentGetValue2=function(c,key)if parts[c]then return parts[c][key]end;return base.ComponentGetValue2(c,key)end;e.ComponentGetValue=e.ComponentGetValue2
 e.ComponentSetValue2=function(c,key,value,...)if parts[c]then parts[c][key]=value else base.ComponentSetValue2(c,key,value,...)end end;e.ComponentSetValue=e.ComponentSetValue2
 e.EntityGetParent=function(id)if rows[id]then return rows[id].parent end;return base.EntityGetParent(id)end
 e.EntityAddChild=function(parent,id)assert(rows[id]);rows[id].parent=parent end
 e.EntityGetAllChildren=function(id)local out=base.EntityGetAllChildren and base.EntityGetAllChildren(id)or{};for child,row in pairs(rows)do if row.parent==id then out[#out+1]=child end end;return out end
 e.EntityKill=function(id)if rows[id]then rows[id]=nil else base.EntityKill(id)end end
 e.ComponentGetIsEnabled=function(c)if parts[c]then return parts[c].enabled~=false end;return base.ComponentGetIsEnabled(c)end
 e.EntitySetComponentIsEnabled=function(id,c,value)if parts[c]then parts[c].enabled=value else base.EntitySetComponentIsEnabled(id,c,value)end end
 e.ComponentHasTag=function(c,tag)return parts[c]and parts[c]._tags==tag end
 e.EntitySetComponentsWithTagEnabled=function()end;e.PhysicsSetStatic=function()end
 e.EntityCreateNew=function(name)local id=spawn(0,0);rows[id].name=name;return id end
 local set_name=e.EntitySetName;e.EntitySetName=function(id,name)if rows[id]then rows[id].name=name elseif set_name then set_name(id,name)end end
 e.EntityGetClosestWithTag=function(x,y,tag)local best,d;for _,id in ipairs(e.EntityGetWithTag(tag))do local a,b=e.EntityGetTransform(id);local n=(a-x)^2+(b-y)^2;if not d or n<d then best,d=id,n end end;return best or 0 end
 e.EntityGetInRadiusWithTag=function(x,y,r,tag)local out={};for _,id in ipairs(e.EntityGetWithTag(tag))do local a,b=e.EntityGetTransform(id);if(a-x)^2+(b-y)^2<=r^2 then out[#out+1]=id end end;return out end
 e.EntityLoad=function(path,x,y)
  if path:find('/c2/',1,true)or path:find('boss_centipede/',1,true)or path:find('regeneration_field_long',1,true)or path:find('spawn_necromancer_shop',1,true)then
   local id=spawn(x,y);rows[id].path=path
   if path:find('sampo_aura.xml',1,true)then part(id,'LuaComponent',{_tags='nt_aura_script',enabled=true});part(id,'ParticleEmitterComponent',{attractor_force=-.4,area_circle_radius=190,velocity_always_away_from_center=1})end
   if path:find('hourglass_effect_trigger',1,true)then rows[id].tags.hourglass_entity=true;part(id,'MaterialAreaCheckerComponent',{})end
   if path:find('loose_lava',1,true)then rocks=rocks+1 end
   if path:find('sampo.xml',1,true)and not path:find('disabled_sampo',1,true)then rows[id].tags.sampo=true;rows[id].tags.this_is_sampo=true end
   if path:find('spawn_necromancer_shop',1,true)then guards=guards+1 end
   return id
  end
  return base.EntityLoad(path,x,y)
 end
 e.LoadGameEffectEntityTo=function(player,path)if path~='data/entities/misc/effect_damage_multiplier.xml'and path~='data/entities/misc/effect_movement_faster_2x.xml'and path~='data/entities/misc/effect_faster_levitation.xml'and path~='mods/noita_together2/files/c2/effects/hourglass_hastium.xml'then return base.LoadGameEffectEntityTo(player,path)end;local id=spawn(0,0);rows[id].path=path;rows[id].parent=player;part(id,'GameEffectComponent',{frames=path:find('hastium',1,true)and 7200 or 0});if path:find('hastium',1,true)then part(id,'GameEffectComponent',{frames=7200})end;effects[path]=(effects[path]or 0)+1;return id end
 local emit_load=e.EntityLoad;e.EntityLoad=function(path,x,y)local id=emit_load(path,x,y);if path:find('weak_ambrosia',1,true)or path:find('regeneration_field_long',1,true)then effects[path]=(effects[path]or 0)+1 end;return id end
 e.GameTriggerMusicFadeOutAndDequeueAll=function()music=music+1 end;e.GameTriggerMusicEvent=function()end;e.GamePlaySound=function()end;e.GameScreenshake=function()end
 e.EntityIngestMaterial=function(_,_,count)effects.confusion=(effects.confusion or 0)+1;parts[stomach].ingestion_size=parts[stomach].ingestion_size+count end
 local text=e.GamePrintImportant;e.GamePrintImportant=function(...)notices=notices+1;if text then text(...)end end
 e.CellFactory_GetType=function(name)local names={'water','blood','oil','magic_liquid_protection_all','magic_liquid_berserk','magic_liquid_charm','material_confusion','magic_liquid_movement_faster','magic_liquid_faster_levitation','magic_liquid_faster_levitation_and_movement','fungi'};for i,v in ipairs(names)do if name==v then return i end end;return -1 end
 local self={};local last_bank
 function self:tick(bank)
  if bank then last_bank=bank end;bank=bank or last_bank or{c2={}}
  if not bank.c2 then return end;restore();local owner=bank.c2
  local plan=input('c2-action',true)
  if plan then local action=assert(json.decode(plan));local opted_out=e.GlobalsGetValue('nt2_rejoin_local_world_v1_offline','0')=='1';assert(opted_out and action.kind=='pickup'or not opted_out and owner.playing and owner.ready,'C2 action requires current authority or an explicit offline pickup')
   if action.kind=='arena'then local h=hook();h.spawn_items=function()error('Managed C2 must own arena')end;h.dofile('mods/noita_together2/files/c2/append/boss_arena.lua');h.spawn_items(arena_x,0)
   elseif action.kind=='distance'then if not px then px,py=base.EntityGetTransform(current())end;goalx,goaly=arena_x+tonumber(action.x),80;release_position=false
   elseif action.kind=='position_free'then if not px then px,py=base.EntityGetTransform(current())end;goalx,goaly=base.EntityGetTransform(current());release_position=true
   elseif action.kind=='pickup'then for _,id in ipairs(e.EntityGetWithTag('nt2_c2_owned'))do if rows[id]and rows[id].tags.sampo then local h=hook();h.item_pickup=function()error('C2 must defer base boss start')end;h.dofile('mods/noita_together2/files/c2/append/sampo_append.lua');rows[id].parent=current();h.item_pickup(id,current(),'sampo');break end end
   elseif action.kind=='steve'then e.GlobalsSetValue('TEMPLE_SPAWN_GUARDIAN','1')
   elseif action.kind=='hourglass'then local h=hook();h.spawn_hourglass_blood=function()end;h.dofile('mods/noita_together2/files/c2/append/snowcastle_cavern.lua');h.spawn_hourglass_blood(0,0);groups=groups+1
   elseif action.kind=='trigger'then local store=e.dofile('mods/noita_together2/files/c2/storage.lua');for _,id in ipairs(e.EntityGetWithTag('hourglass_entity'))do local own=store.component(e,id,'nt2_c2_hourglass');if own and own.effectId==action.effect then local h=hook();h.GetUpdatedEntityID=function()return id end;h.dofile('mods/noita_together2/files/c2/scripts/hourglass_events.lua');h.material_area_checker_success()end end;if input('c2-freeze-after-capture',true)then freeze_at=state.frame+1 end
   else error('Unknown C2 fixture action')end
  end
  if goalx then
   if release_position then goalx,goaly=base.EntityGetTransform(current())end
   local dx,dy=goalx-px,goaly-py
   px=px+math.max(-16,math.min(16,dx));py=py+math.max(-16,math.min(16,dy))
   if math.abs(dx)<=16 and math.abs(dy)<=16 then goalx,goaly=nil,nil;if release_position then px,py=nil,nil;release_position=false end end
  end
  if state.frame%2==0 then for _,id in ipairs(e.EntityGetWithTag('nt2_c2_owned'))do if rows[id]and rows[id].path and rows[id].path:find('sampo_aura.xml',1,true)then local c=e.EntityGetFirstComponentIncludingDisabled(id,'LuaComponent','nt_aura_script');if c and e.ComponentGetIsEnabled(c)then local h=hook();h.GetUpdatedEntityID=function()return id end;h.dofile('mods/noita_together2/files/c2/scripts/sampo/sampo_aura.lua')end end end end
  save()
  if freeze_at and state.frame>=freeze_at then mark('pause-game-thread','C2 saved original before engine ACK copy');freeze_at=nil end
  local storage=e.dofile('mods/noita_together2/files/c2/storage.lua');local outgoing=storage.read(e,'outgoing');local receipt=storage.read(e,'receipt')
  local sampos=0;for _,id in ipairs(e.EntityGetWithTag('nt2_c2_owned'))do if rows[id]and rows[id].tags.sampo then sampos=sampos+1 end end
  -- The observer has its own JSON instance; preserve empty array shape when
  -- re-encoding a snapshot decoded by the production module's instance.
  if owner.snapshot and type(owner.snapshot.completions)=='table'then json.array(owner.snapshot.completions)end
  mark('c2-view.json',assert(json.encode({offline=e.GlobalsGetValue('nt2_rejoin_local_world_v1_offline','0')=='1',ready=owner.ready,playing=owner.playing,world=owner.scope,progression=progression,moving=goalx~=nil,snapshot=owner.snapshot,waiting=owner.waiting,status=owner.status,appStatus=owner.view and owner.view.status,queued=outgoing and #outgoing.queue or 0,effectReceipt=receipt,event=owner.view and owner.view.event,permit=owner.view and owner.view.permit,sampos=sampos,rocks=rocks,music=music,effects=effects,guards=guards,stomach=parts[stomach].ingestion_size,steveAngered=e.GlobalsGetValue('TEMPLE_SPAWN_GUARDIAN','0')=='1',notices=notices,hourglassGroups=groups,engineFactsInjected=true,actualNoita=false})))
 end
 return self
end
return M
