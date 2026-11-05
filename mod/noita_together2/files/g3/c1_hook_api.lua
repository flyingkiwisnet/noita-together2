-- Noita item/status/biome Lua contexts do not share init.lua globals. Use the
-- same world's fixed Globals mailbox, as legacy NT stringstore did, with a
-- main-context grant. Native calls and ID allocation stay in the main context.
local root='mods/noita_together2/files/g3/'
local json=dofile(root..'bank_json.lua')
local resolver=dofile(root..'game_api.lua')
local self={}
local function authority()
 if GlobalsGetValue('nt2_race_active','0')=='1'then return nil end
 local a=json.decode_saved(GlobalsGetValue('nt2_c1_authority',''))
 if not a or not a.playing or type(a.frame)~='number'or type(a.world)~='string'or #a.world~=64 or type(a.context)~='string'or #a.context~=32 or GameGetFrameNum()<a.frame or GameGetFrameNum()>a.frame+1 then return nil end
 local id,state=resolver.resolve_player(_G,a.player)
 if id~=a.player or not state or not state.ready then return nil end
 if ComponentGetIsEnabled and not ComponentGetIsEnabled(state.controls)then return nil end
 if ComponentGetValue2(state.controls,'enabled')==false then return nil end
 for _,c in ipairs(EntityGetComponentIncludingDisabled(id,'VariableStorageComponent')or{})do if ComponentGetValue2(c,'name')=='nt2_g3_recovery'then return nil end end
 local tokens=json.decode_saved(GlobalsGetValue('nt2_c1_tokens',''))
 if not tokens or tokens.world~=a.world or tokens.context~=a.context then return nil end
 return a,tokens
end
local function trace(a,id,state,reason)
 local key='nt2_c1_count_'..state;local count=math.min(2147483647,tonumber(GlobalsGetValue(key,'0'))+1);GlobalsSetValue(key,tostring(count))
 print('NT2_C1|world='..a.world..'|id='..id..'|sequence=0|state='..state..'|reason='..reason..'|count='..count..'|frame='..GameGetFrameNum())
end
function self:duplicate(entity)
 if not entity or not EntityGetIsAlive(entity)then return false end
 for _,c in ipairs(EntityGetComponentIncludingDisabled(entity,'VariableStorageComponent')or{})do if ComponentGetValue2(c,'name')=='nt2_c1_original'then
  if ComponentGetValue2(c,'value_bool')~=true then ComponentSetValue2(c,'value_bool',true);GamePrint('NT2: this shared pickup was already recorded; it will not repeat.')end
  return true
 end end
 return false
end
function self:can_pickup(player)
 local a=authority();if not a then return false end
 local id,state=resolver.resolve_player(_G,player)
 return id==player and id==a.player and state and state.ready
end
function self:roster_count()local a=authority();return a and a.count or 0 end
function self:capture(kind,payload,entity)
 local a,tokens=authority();if not a then return nil end
 if entity then for _,c in ipairs(EntityGetComponentIncludingDisabled(entity,'VariableStorageComponent')or{})do if ComponentGetValue2(c,'name')=='nt2_c1_original'then return nil,'duplicate' end end end
 local catalogue=dofile(root..'c1_catalogue_data.lua')
 local supported=kind=='heart'or kind=='death'
 if kind=='perk'then supported=catalogue.perks[payload.perkId]~=nil
 elseif kind=='orb'then supported=catalogue.orbs[tostring(payload.orbId)]~=nil
 elseif kind=='fungal'then
  supported=type(payload.fromMaterials)=='table'and #payload.fromMaterials>0 and #payload.fromMaterials<=64 and catalogue.materials[payload.toMaterial]==true and(not payload.displayMaterial or catalogue.materials[payload.displayMaterial]==true)
  for _,name in ipairs(payload.fromMaterials or{})do if not catalogue.materials[name]then supported=false end end
 end
 if not supported then GamePrint('NT2: unsupported shared player content.');return nil end
 local saved=json.decode_saved(GlobalsGetValue('nt2_c1_outgoing',''),{bytes=131072,nodes=16384})
 if not saved or saved.world~=a.world or saved.context~=a.context then saved={world=a.world,context=a.context,queue={}}end
 if #saved.queue>=32 then GamePrint('NT2: C1 sharing queue full.');return nil end
 local id=table.remove(tokens.ids,1);if type(id)~='string'or #id~=36 then return nil end
 -- Burn before any gameplay/queue operation. Even a failed hook cannot reuse it.
 GlobalsSetValue('nt2_c1_tokens',json.encode_saved(tokens))
 local original={eventId=id,rulesRevision=a.revision,payload={[kind]=payload},senderDisposition='uncertain'}
 if entity then EntityAddComponent2(entity,'VariableStorageComponent',{name='nt2_c1_original',value_string=id})end
 saved.queue[#saved.queue+1]=original;GlobalsSetValue('nt2_c1_outgoing',json.encode_saved(saved))
 trace(a,id,'captured','local_original');return original
end
function self:submit(original)
 local a=authority();if not a then return end
 local saved=json.decode_saved(GlobalsGetValue('nt2_c1_outgoing',''),{bytes=131072,nodes=16384})
 if not saved or saved.world~=a.world or saved.context~=a.context then return end
 for _,v in ipairs(saved.queue)do if v.eventId==original.eventId then v.senderDisposition='applied' end end
 GlobalsSetValue('nt2_c1_outgoing',json.encode_saved(saved));trace(a,original.eventId,'applied','local_effect')
end
function self:checkpoint(player,x,y)
 local a=authority();if not a or not self:can_pickup(player)or GameHasFlagRun('NT_death_penalty_end')then return false end
 GlobalsSetValue('nt2_c1_checkpoint',json.encode_saved({world=a.world,x=tostring(x),y=tostring(y)}));return true
end
-- Sharing is optional to the base callback. Missing/unavailable engine helpers
-- must never throw through item_pickup, fungal_shift or collision_trigger.
for name,fn in pairs(self)do self[name]=function(...)
 local ok,a,b=pcall(fn,...);if ok then return a,b end
 return nil
end end
return self
