-- Statistics observations only. No gameplay effect, native I/O or arbitrary count.
local root='mods/noita_together2/files/'
local json=dofile(root..'g3/bank_json.lua')
local actor=dofile(root..'g3/game_api.lua')
local store=dofile(root..'c2/storage.lua')
local M={}
local function read(key)local raw=GlobalsGetValue('nt2_d1_'..key,'');local value=json.decode_saved(raw,{bytes=65536,nodes=8192});return value,raw~=''and value==nil end
local function write(key,value)GlobalsSetValue('nt2_d1_'..key,assert(json.encode_saved(value,{bytes=65536,nodes=8192})))end
local function dropped()
 local n=tonumber(GlobalsGetValue('nt2_d1_dropped','0'))or 0;GlobalsSetValue('nt2_d1_dropped',tostring(math.min(2147483647,n+1)))
end
local function authority()
 if GlobalsGetValue('nt2_race_active','0')=='1'then return nil end
 local a=read('authority');local frame=GameGetFrameNum()
 if not a or not a.playing or type(a.frame)~='number'or frame<a.frame or frame>a.frame+1 then return nil end
 local id,state=actor.resolve_player(_G,a.player)
 if id~=a.player or not state or not state.ready or state.transformed then return nil end
 local composing=false
 for _,c in ipairs(EntityGetComponentIncludingDisabled(id,'VariableStorageComponent')or{})do local name=ComponentGetValue2(c,'name');if name=='nt2_g3_recovery'then return nil end;if name=='nt2_a2_composer_lock'and ComponentGetValue2(c,'value_bool')==true then composing=true end end
 -- Typing in the live bank/composer is still admitted play. Native protection
 -- always wins over that existing input lease.
 if ComponentGetValue2(state.controls,'enabled')==false and not composing then return nil end
 return a
end
function M.begin(kind,player,entity,event_id,orb_id)
 if kind~='heart'and kind~='orb'and kind~='steve'then return nil end
 local a=authority();if not a or player~=a.player then return nil end
 local progression=store.progression(_G);if progression==nil then return nil end
 if entity then
  if EntityHasTag(entity,'fake_orb')then return nil end
  for _,c in ipairs(EntityGetComponentIncludingDisabled(entity,'VariableStorageComponent')or{})do if ComponentGetValue2(c,'name')=='nt2_d1_original'then return nil end end
 end
 if kind=='orb'then
  if type(orb_id)=='number'and orb_id%1==0 then orb_id=string.format('%.0f',orb_id)end
  if type(orb_id)~='string'or not orb_id:match('^%d+$')or #orb_id>20 then return nil end
 end
 local q,invalid=read('outgoing');if invalid then dropped();return nil end;q=q or{world=a.world,queue={}}
 if q.world~=a.world or #q.queue>=64 then dropped();return nil end
 if not event_id then
  local tokens=read('tokens');if not tokens or tokens.world~=a.world or tokens.context~=a.context then return nil end
  event_id=table.remove(tokens.ids,1);write('tokens',tokens)
 end
 if type(event_id)~='string'or #event_id~=36 then dropped();return nil end
 local original={eventId=event_id,kind=kind,progression=progression,orbId=kind=='orb'and orb_id or nil}
 if entity then local marker=EntityAddComponent2(entity,'VariableStorageComponent',{name='nt2_d1_original',value_string=event_id});if not marker or marker==0 then dropped();return nil end end
 q.queue[#q.queue+1]={observation=original,applied=false};write('outgoing',q)
 return original
end
function M.commit(original)
 if not original then return end
 local q=read('outgoing');if not q then return end
 for _,row in ipairs(q.queue)do if row.observation.eventId==original.eventId then row.applied=true end end
 write('outgoing',q)
end
function M.steve(entity,responsible)
 local a=authority();if not a or not responsible or responsible==0 then return end
 local root=EntityGetRootEntity and EntityGetRootEntity(responsible)or responsible
 if root~=a.player then
  local c=EntityGetFirstComponentIncludingDisabled(responsible,'ProjectileComponent')
  local shooter=c and ComponentGetValue2(c,'mWhoShot')
  if not shooter or (EntityGetRootEntity and EntityGetRootEntity(shooter)or shooter)~=a.player then return end
 end
 M.commit(M.begin('steve',a.player,entity))
end
-- Hooks never stop vanilla pickup/death handling if optional capture fails.
for key,fn in pairs(M)do M[key]=function(...)local ok,a=pcall(fn,...);if ok then return a end;pcall(dropped);return nil end end
return M
