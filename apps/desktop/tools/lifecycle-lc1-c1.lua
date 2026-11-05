-- Injected Noita facts only. Actual C1 hooks, native mailbox, app worker and
-- service handle authority/originals/delivery. Not actual game/save evidence.
local M={}
function M.new(e,state,forms,input,mark,files)
 local restricted=assert(loadfile(assert(os.getenv('NT2_C3_REPO'))..'/apps/desktop/tools/c1-restricted-context.lua'))()
 local function hook_context()
  return restricted(e,function(path)local f=assert(io.open(files..'/'..path,'rb'));local body=f:read('*a');f:close();return body end)
 end
 local json=e.dofile('mods/noita_together2/files/g3/bank_json.lua')
 local base={};for _,k in ipairs({'EntityGetIsAlive','EntityGetTransform','EntityGetFirstComponentIncludingDisabled','EntityGetComponentIncludingDisabled','EntityGetWithTag','EntityHasTag','EntityAddComponent2','EntityRemoveComponent','ComponentGetValue2','ComponentSetValue2','EntityKill','EntityLoad','ModTextFileGetContent','ModTextFileSetContent'})do base[k]=e[k]end
 local rows,parts={},{};local player_variables={};local next_id=910000;local mutations=0;local actions=0;local text={};local last_id;local perks={};local orbs={};local conversions=0;local death_count=0;local checkpoint_x,checkpoint_y
 local function current()return forms and forms:current()or 99 end
 state.components[2].hp=2;state.components[2].max_hp=4;state.components[2].max_hp_cap=0
 local function part(id,kind,v)next_id=next_id+1;parts[next_id]=v;rows[id].parts[kind]=rows[id].parts[kind]or{};table.insert(rows[id].parts[kind],next_id);return next_id end
 local function spawn(x,y)next_id=next_id+1;local id=next_id;rows[id]={x=x,y=y,parts={},tags={},born=state.frame};return id end
 e.ModDoesFileExist=function(path)local f=io.open(files..'/'..path,'rb');if f then f:close();return true end;return false end
 e.ModTextFileGetContent=function(path)if path=='mods/noita_together2/files/loot/run-policy.txt'then return base.ModTextFileGetContent(path)end;if text[path]then return text[path]end;if path=='data/translations/common.csv'then return ''end;local f=assert(io.open(files..'/'..path,'rb'));local s=f:read('*a');f:close();return s end
 e.ModTextFileSetContent=function(path,v)if path=='mods/noita_together2/files/loot/run-policy.txt'then return base.ModTextFileSetContent(path,v)end;text[path]=v end
 e.EntityGetIsAlive=function(id)return rows[id]~=nil or base.EntityGetIsAlive(id)end
 e.EntityGetTransform=function(id)if rows[id]then return rows[id].x,rows[id].y end;return base.EntityGetTransform(id)end
 e.EntityGetFirstComponentIncludingDisabled=function(id,kind,...)if rows[id]then return(rows[id].parts[kind]or{})[1]end;if id==current()and kind=='GameStatsComponent'then return 919999 end;return base.EntityGetFirstComponentIncludingDisabled(id,kind,...)end
 e.EntityGetComponentIncludingDisabled=function(id,kind)if rows[id]then return rows[id].parts[kind]or{}end;if id==current()and kind=='DamageModelComponent'then return{2}end;local out=base.EntityGetComponentIncludingDisabled(id,kind)or{};if id==current()and kind=='VariableStorageComponent'then for c in pairs(player_variables)do out[#out+1]=c end end;return out end;e.EntityGetComponent=e.EntityGetComponentIncludingDisabled
 e.EntityGetWithTag=function(tag)if tag=='player_unit'then if input('c1-missing-actor',false)then return{}end;return{current()}end;return base.EntityGetWithTag and base.EntityGetWithTag(tag)or{}end
 e.EntityHasTag=function(id,tag)if rows[id]then return rows[id].tags[tag]or false end;if not forms and id==current()then if tag=='player_unit'then return true end;if tag=='polymorphed'or tag=='polymorphed_player'or tag=='polymorphed_cessation'then return false end end;return base.EntityHasTag(id,tag)end
 e.EntityAddComponent2=function(id,kind,v)if rows[id]then return part(id,kind,v)end;if id==current()and kind=='VariableStorageComponent'and (v.name=='nt2_c1_wait'or v.name:match('^nt2_race_'))then next_id=next_id+1;parts[next_id]=v;player_variables[next_id]=true;return next_id end;return base.EntityAddComponent2(id,kind,v)end;e.EntityAddComponent=e.EntityAddComponent2
 local remove_component=e.EntityRemoveComponent;e.EntityRemoveComponent=function(id,c)if player_variables[c]then player_variables[c]=nil;parts[c]=nil;return end;return remove_component(id,c)end
 e.ComponentGetValue2=function(id,key)if id==919999 then return key=='is_player'end;if parts[id]then return parts[id][key]end;return base.ComponentGetValue2(id,key)end;e.ComponentGetValue=e.ComponentGetValue2
 -- ControlsComponent is active even while its enabled input field is held.
 -- The older shared stub only modelled inventory component activation.
 local component_enabled=e.ComponentGetIsEnabled;e.ComponentGetIsEnabled=function(id)if id==1 then return true end;return component_enabled(id)end
 e.ComponentSetValue2=function(id,key,v,...)if parts[id]then parts[id][key]=v;return end;if id==2 and(key=='hp'or key=='max_hp')then mutations=mutations+1;if key=='max_hp'and input('c1-partial-fail',false)then error('Injected failure after HP mutation')end end;return base.ComponentSetValue2(id,key,v,...)end;e.ComponentSetValue=e.ComponentSetValue2
 e.EntityKill=function(id)if rows[id]then rows[id]=nil;return end;return base.EntityKill(id)end
 e.EntityLoad=function(path,x,y)if path=='data/entities/particles/poof_pink.xml'then return base.EntityLoad(path,x,y)end;if path:find('/c1/',1,true)or path:find('data/entities/particles/',1,true)then local id=spawn(x,y);if path:find('death_cam',1,true)then rows[id].tags.nt2_c1_death_camera=true end;if path:find('forced_orb',1,true)then part(id,'OrbComponent',{});part(id,'ItemComponent',{})end;return id end;return base.EntityLoad(path,x,y)end
 e.GameTriggerMusicCue=function()end;e.GamePrintImportant=e.GamePrintImportant or function()end;e.GameTextGet=function(key)return key end;e.print_error=function()end
 e.GameTriggerGameOver=function()e.game_overs=(e.game_overs or 0)+1;mark('c1-gameover-'..e.game_overs..'.json',assert(json.encode({marker=e.GlobalsGetValue('nt2_c1_finished',''),frame=e.GameGetFrameNum()})))end
 e.GameGetCameraPos=function()return e.EntityGetTransform(current())end;e.GameSetCameraFree=function()end;e.GameSetCameraPos=function()end
 e.GameGetOrbCountThisRun=function()local count=0;for _ in pairs(orbs)do count=count+1 end;return count end;e.GameGetOrbCollectedThisRun=function(id)return orbs[id]or false end
 e.perk_list={};local catalogue=e.dofile('mods/noita_together2/files/g3/c1_catalogue_data.lua');for id in pairs(catalogue.perks)do e.perk_list[#e.perk_list+1]={id=id}end
 e.get_perk_with_id=function(list,id)for _,v in ipairs(list)do if v.id==id then return v end end end
 e.perk_spawn=function(x,y,id)local entity=spawn(x,y);part(entity,'VariableStorageComponent',{name='perk_id',value_string=id});return entity end
 e.perk_pickup=function(entity,player)for _,c in ipairs(rows[entity].parts.VariableStorageComponent or{})do if parts[c].name=='perk_id'then local id=parts[c].value_string;perks[id]=(perks[id]or 0)+1 end end;e.EntityKill(entity)end
 e.dofile('mods/noita_together2/files/c1/scripts/ban_perks.lua')
 -- Base-game content/engine calls are injected; production handlers still own decisions.
 e.HandleBannedPerksEvent=function()assert(not e.GameHasFlagRun('NT_ban_perks'),'Live ban is covered by the real handler focused test');return true end
 local first=e.EntityGetFirstComponentIncludingDisabled;e.EntityGetFirstComponentIncludingDisabled=function(id,kind,...)if id==e.GameGetWorldStateEntity()and id>0 and kind=='WorldStateComponent'then return 919997 end;return first(id,kind,...)end;e.EntityGetFirstComponent=e.EntityGetFirstComponentIncludingDisabled
 parts[919997]={EVERYTHING_TO_GOLD=false}
 local types={water=1,blood=2,oil=3};e.CellFactory_GetType=e.CellFactory_GetType or function(name)return types[name]or -1 end;e.CellFactory_GetName=e.CellFactory_GetName or function(id)for name,n in pairs(types)do if n==id then return name end end end
 e.CellFactory_GetUIName=e.CellFactory_GetUIName or function(id)return tostring(id)end;e.GameTextGetTranslatedOrNot=e.GameTextGetTranslatedOrNot or function(v)return v end;e.SetRandomSeed=function()end;e.rand=function()return 0 end
 e.ConvertMaterialEverywhere=function()conversions=conversions+1 end;e.GameCreateParticle=function()end;e.GameTriggerMusicFadeOutAndDequeueAll=function()end;e.GameTriggerMusicEvent=function()end
 local add_child=e.EntityAddChild;e.EntityAddChild=function(parent,child)if rows[child]then return end;if add_child then return add_child(parent,child)end end;e.EntityCreateNew=function(name)return spawn(0,0)end;local entity_name=e.EntityGetName;e.EntityGetName=function(id)if rows[id]then return ''end;return entity_name(id)end
 local transform=e.EntitySetTransform;e.EntitySetTransform=function(id,x,y)if rows[id]then rows[id].x=x;rows[id].y=y elseif id==current()then checkpoint_x=x;checkpoint_y=y;transform(id,x,y)else transform(id,x,y)end end

 e.MagicNumbersGetValue=function(key)return key:find('_X')and '227'or '-85'end
 local flags={};local has=e.GameHasFlagRun
 e.GameHasFlagRun=function(key)if flags[key]~=nil then return flags[key]end;return has and has(key)or false end
 e.GameAddFlagRun=function(key)flags[key]=true end;e.GameRemoveFlagRun=function(key)flags[key]=false end
 e.GamePrint=e.GamePrint or function()end
 local function physical(kind,id)
  local h=hook_context();assert(h.require==nil and h.NT2_C1==nil)
  local loaded,api=pcall(h.dofile,'mods/noita_together2/files/g3/c1_hook_api.lua')
  local facts={loaded=loaded,canPickup=loaded and api:can_pickup(current())or false,kind=kind,requireAvailable=h.require~=nil,ffiAvailable=h.ffi~=nil,frame=e.GameGetFrameNum()}
  mark('c1-hook-context.json',assert(json.encode(facts)))
  h.item_pickup=function(entity,player)
   if kind=='perk'then e.perk_pickup(entity,player)
   elseif kind=='orb'then local c=e.EntityGetFirstComponentIncludingDisabled(entity,'OrbComponent');orbs[e.ComponentGetValue2(c,'orb_id')]=true;e.EntityKill(entity)
   elseif kind=='heart'or kind=='large_heart'then
    assert(not e.GameHasFlagRun('NT_sync_hearts'),'Sharing-on must invoke production shared heart')
    local max=state.components[2].max_hp+(kind=='large_heart'and 2 or 1);state.components[2].max_hp=max;if kind=='large_heart'then state.components[2].hp=max end;e.EntityKill(entity)
   else error('Unsupported injected vanilla pickup')end
  end
  h.dofile('mods/noita_together2/files/c1/append/'..({heart='heart_append.lua',large_heart='heart_better_append.lua',perk='perk_pickup.lua',orb='orb_pickup.lua'})[kind])
  h.item_pickup(id,current(),'Injected '..kind)
 end
 e.SessionNumbersGetValue=e.SessionNumbersGetValue or function(key)assert(key=='NEW_GAME_PLUS_COUNT');return '0'end
 local self={}
 function self:engine(parent)return setmetatable({LoadGameEffectEntityTo=function(player,path)assert(path=='data/entities/misc/effect_protection_all.xml');local id=spawn(0,0);part(id,'GameEffectComponent',{frames=0});return id end},{__index=parent})end
 function self:tick(bank)
  if not bank or not bank.c1 then return end
  local owner=bank.c1
  for id,row in pairs(rows)do local orb=(row.parts.OrbComponent or{})[1];local item=(row.parts.ItemComponent or{})[1];if orb and item and parts[item].auto_pickup and state.frame>=row.born+11 then orbs[parts[orb].orb_id]=true;e.EntityKill(id)end end
  if owner.ready and not owner.playing and not owner.completion and mutations==0 then mark('c1-protected-bootstrap.json',assert(json.encode({frame=state.frame,controls=state.components[1].enabled,mutations=mutations,queued=#owner.queue,metadataOnly=true})))end
  if owner.completion and not owner.playing then mark('c1-protected-completion.json',assert(json.encode({frame=state.frame,controls=state.components[1].enabled,hp=tostring(state.components[2].hp),max=tostring(state.components[2].max_hp),mutations=mutations,world=owner.scope,eventAbsent=owner.event==nil,permitAbsent=owner.permit==nil})))end
  local plan=input('c1-action',true)
  if plan then
   local data=assert(json.decode(plan));assert(data.kind=='death'or data.kind=='win_death'or owner.ready and owner.playing,'C1 pickup action requires actual current gameplay');actions=actions+1
   if data.kind=='heart'or data.kind=='large_heart'then
    local id=spawn(e.EntityGetTransform(current()));last_id=id
    physical(data.kind,id)
   elseif data.kind=='perk'then physical('perk',e.perk_spawn(0,0,data.id))
   elseif data.kind=='orb'then local id=spawn(0,0);part(id,'OrbComponent',{orb_id=data.id});physical('orb',id)
   elseif data.kind=='steve'then
    local id=spawn(0,0);local h=hook_context();h.GetUpdatedEntityID=function()return id end;h.death=function()e.GlobalsSetValue('STEVARI_DEATHS',tostring(tonumber(e.GlobalsGetValue('STEVARI_DEATHS','0'))+1));e.EntityKill(id)end;h.dofile('mods/noita_together2/files/c1/append/steve_death.lua');h.death(0,'Injected actual death callback',current(),true)
   elseif data.kind=='fungal'then
    local h=hook_context();h.materials_from={{materials={'water','blood'},name_material='water'}};h.materials_to={{material='oil'}}
    h.random_create=function()return{}end;h.pick_random_from_table_weighted=function(_,values)return values[1]end;h.get_held_item_material=function()return 0 end
    h.EntityRemoveIngestionStatusEffect=function()end;h.log_messages={'Injected local fungal'};h.random_from_array=function(values)return values[1]end
    h.dofile('mods/noita_together2/files/c1/append/fungal_shift.lua');h.fungal_shift(current(),0,0,false)
   elseif data.kind=='checkpoint'then
    local h=hook_context();local checkpoint=spawn(50,-20);h.GetUpdatedEntityID=function()return checkpoint end
    h.dofile('mods/noita_together2/files/c1/scripts/checkpoint.lua');h.collision_trigger(current());e.EntityKill(checkpoint)
   elseif data.kind=='death'or data.kind=='win_death'then if data.kind=='win_death'then flags.ending_game_completed=true end;state.components[2].hp=-1;death_count=death_count+1
   elseif data.kind=='repeat_delivery'then owner:tick(owner.playing,current(),state.frame)
   elseif data.kind=='hp'then state.components[2].hp=tonumber(data.hp);state.components[2].max_hp=tonumber(data.max)
   else error('Unknown fixed C1 fixture action')end
  end
  mark('c1-view.json',assert(json.encode({ready=owner.ready,playing=owner.playing,world=owner.scope,rules=owner.rules,count=owner.count,revision=owner.revision,hp=tostring(state.components[2].hp),max=tostring(state.components[2].max_hp),mutations=mutations,actions=actions,event=owner.event,permit=owner.permit,status=owner.status,queued=#owner.queue,engineFactsInjected=true,actualNoita=false,perks=perks,orbCount=e.GameGetOrbCountThisRun(),orbZero=orbs[0]==true,conversions=conversions,deaths=death_count,checkpointX=checkpoint_x and tostring(checkpoint_x)or '',checkpointY=checkpoint_y and tostring(checkpoint_y)or '',terminal=owner.terminal_state==true,gameOvers=e.game_overs or 0,life=owner.life,localStatus=owner.local_status})))
 end
 return self
end
return M
