-- Callback-side capture only: XML-safe Globals, no native, disk or network I/O.
local root='mods/noita_together2/files/'
local store=dofile(root..'race/storage.lua')
local actor=dofile(root..'g3/game_api.lua')
local sample=dofile(root..'race/sample.lua')
local catalogue=dofile(root..'race/catalogue.lua')
local progression=dofile(root..'c2/storage.lua').progression
local M={}
local function authority(terminal,forms)
 local a=store.read(_G,'authority');local frame=GameGetFrameNum()
 if not store.active(_G)or not a or not a.playing or type(a.frame)~='number'or frame<a.frame or frame>a.frame+1 or progression(_G)~=0 then return nil end
 local id,state=actor.resolve_player(_G,a.player)
 if id~=a.player or not state or state.transformed and not forms or state.dead and not terminal then return nil end
 if GlobalsGetValue('nt2_rejoin_local_world_v1_offline','0')=='1'then return nil end
 for _,c in ipairs(EntityGetComponentIncludingDisabled(id,'VariableStorageComponent')or{})do if ComponentGetValue2(c,'name')=='nt2_g3_recovery'then return nil end end
 return a
end
local function state(a)
 local q,invalid=store.read(_G,'outgoing')
 if invalid or not q or q.world~=a.world or q.closed or #q.queue>=64 then return nil end
 if q.terminalFrame and q.terminalFrame~=GameGetFrameNum()then return nil end
 return q
end
local function key(o)
 if o.kind=='orb'then return 'orb:'..o.orbId end
 if o.kind=='boss'then return 'boss:'..o.bossId end
 if o.kind=='biome'then return 'biome:'..o.biomeId end
 if o.kind=='parallel'then return 'parallel:'..o.direction end
 return o.kind
end
local function add(q,o,final)
 local k=key(o);local value=o.kind=='max_hp'or o.kind=='gold';local numeric=value and tonumber(o.value)or o.kind=='parallel'and o.index
 local prior=q.known[k]
 if prior and(not numeric or numeric<=prior)then return false end
 -- Coalesce high-water facts within this frame, retaining discrete facts.
 for i,old in ipairs(q.staged)do if key(old)==k then q.staged[i]=o;q.known[k]=numeric or true;return true end end
 if #q.staged>=(final and 32 or 29)then store.drop(_G);return false end
 q.staged[#q.staged+1]=o;q.known[k]=numeric or true;return true
end
function M.observe(observations)
 local a=authority(true,true);if not a then return end;local q=state(a);if not q or #q.queue>=63 then return end
 for _,o in ipairs(observations)do add(q,o)end;store.write(_G,'outgoing',q)
end
function M.orb_begin(entity,player)
 local a=authority(false,true);if not a or player~=a.player or not EntityGetIsAlive(entity)or EntityHasTag(entity,'fake_orb')then return nil end
 local q=state(a);if not q or #q.queue>=63 then return nil end
 local c=EntityGetFirstComponentIncludingDisabled(entity,'OrbComponent');local id=c and ComponentGetValue2(c,'orb_id')
 if type(id)~='number'or id%1~=0 or id<0 then return nil end
 if not catalogue.orbs[tostring(id)]then return nil end
 if not store.marker(_G,entity,'nt2_race_orb',tostring(id))then return nil end
 return {kind='orb',orbId=id}
end
function M.orb_commit(o)if o then M.observe({o})end end
function M.boss(entity)
 local id=catalogue.bosses[EntityGetFilename(entity)]
 if id then M.observe({{kind='boss',bossId=id}})end
end
function M.terminal(kind,classic)
 local a=authority(true,true);if not a then return end;local q=state(a);if not q then return end
 -- Reserve final numeric facts even when the ordinary observation batch is full.
 for _,o in ipairs(sample.facts(_G,a.player,true))do add(q,o,true)end
 if kind=='ending'then
  local ending=classic and'classic'or'other'
  for _,o in ipairs(q.staged)do if o.kind=='ending'and o.ending=='classic'then ending='classic'end end
  add(q,{kind='ending',ending=ending},true)
  -- Ending can upgrade the candidate earlier in this same engine frame.
  for _,o in ipairs(q.staged)do if o.kind=='ending'then o.ending=ending end end
 end
 if kind=='ending'or q.terminal~='ending'then q.terminal=kind end
 q.terminalFrame=GameGetFrameNum();store.write(_G,'outgoing',q)
end
function M.player_death(entity,fatal)
 local a=authority(true,true);if not a or entity~=a.player then return end
 local _,subject=actor.resolve_player(_G,a.player)
 -- Polymorph expiry can retire the form without killing the player. Only its
 -- documented fatal damage message is terminal; normal bodies use real death.
 if not subject or subject.transformed and not fatal then return end
 M.terminal('dead',false)
end
-- Optional capture failure must never interrupt the original gameplay callback.
for name,fn in pairs(M)do M[name]=function(...)local ok,result=pcall(fn,...);if ok then return result end;pcall(store.drop,_G)end end
return M
