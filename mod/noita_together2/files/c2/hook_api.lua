local store=dofile('mods/noita_together2/files/c2/storage.lua')
local actor=dofile('mods/noita_together2/files/g3/game_api.lua')
local M={}
function M.managed()
 if GlobalsGetValue('nt2_race_active','0')=='1'then return false end
 if GlobalsGetValue('nt2_rejoin_local_world_v1_offline','0')=='1'then return false end
 local mode=store.read(_G,'mode');return mode and mode.managed==true or GlobalsGetValue('nt2_rejoin_local_world_v1','')~=''
end
function M.current(player)
 if GlobalsGetValue('nt2_race_active','0')=='1'then return nil end
 if GlobalsGetValue('nt2_rejoin_local_world_v1_offline','0')=='1'then return nil end
 local grant=store.read(_G,'grant');local frame=GameGetFrameNum()
 if not grant or not grant.playing or type(grant.frame)~='number'or frame<grant.frame or frame>grant.frame+1 or grant.progression~=store.progression(_G)then return nil end
 local id,state=actor.resolve_player(_G,player or grant.player)
 if id~=grant.player or not state or not state.ready or state.transformed then return nil end
 if ComponentGetValue2(state.controls,'enabled')==false or ComponentGetIsEnabled and not ComponentGetIsEnabled(state.controls)then return nil end
 for _,c in ipairs(EntityGetComponentIncludingDisabled(id,'VariableStorageComponent')or{})do if ComponentGetValue2(c,'name')=='nt2_g3_recovery'then return nil end end
 return grant
end
function M.capture(payload,event_id,entity)
 local a=M.current();if not a then return nil end
 local saved,invalid=store.read(_G,'outgoing');if invalid then return nil end;saved=saved or{world=a.world,queue={}}
 if saved.world~=a.world or #saved.queue>=32 then return nil end
 if entity then local old=store.component(_G,entity,'nt2_c2_original');if old then return nil,'duplicate'end end
 if not event_id then
  local pool=store.read(_G,'tokens');if not pool or pool.world~=a.world or pool.context~=a.context then return nil end
  event_id=table.remove(pool.ids,1);if not event_id then return nil end;store.write(_G,'tokens',pool)
 end
 for _,v in ipairs(saved.queue)do if v.eventId==event_id then return nil,'duplicate'end end
 local original={progression=a.progression,eventId=event_id,rulesRevision=a.rulesRevision,payload=payload,senderDisposition='uncertain'}
 if entity then store.mark(_G,entity,'nt2_c2_original',{world=a.world,eventId=event_id})end
 saved.queue[#saved.queue+1]=original;store.write(_G,'outgoing',saved);return original
end
function M.submit(original)
 local a=M.current();local saved=store.read(_G,'outgoing');if not a or not saved or saved.world~=a.world then return end
 for _,v in ipairs(saved.queue)do if v.eventId==original.eventId then v.senderDisposition='applied'end end;store.write(_G,'outgoing',saved)
end
return M
