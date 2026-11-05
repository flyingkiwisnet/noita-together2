local root='mods/noita_together2/files/'
local json=dofile(root..'g3/bank_json.lua')
local actor=dofile(root..'g3/game_api.lua')
local M={}
function M.new(e,identity,send)
 local self={}
 local function read(key)local raw=e.GlobalsGetValue('nt2_d1_'..key,'');local value=json.decode_saved(raw,{bytes=65536,nodes=8192});return value,raw~=''and value==nil end
 local function write(key,value)e.GlobalsSetValue('nt2_d1_'..key,assert(json.encode_saved(value,{bytes=65536,nodes=8192})))end
 function self:tick(playing,player,view,context,accepted)
  write('authority',{})
  if type(view)~='table'or type(view.world)~='string'or #view.world~=64 then return end
  local q,invalid=read('outgoing');if invalid then self.status='Statistics saved observations unavailable.';return end;if q and q.world~=view.world then self.status='Statistics saved scope differs.';return end
  q=q or{world=view.world,queue={}}
  local done={};for _,id in ipairs(accepted or{})do done[id]=true end
  local rows={};for _,row in ipairs(q.queue)do if not done[row.observation.eventId]then rows[#rows+1]=row end end;q.queue=rows;write('outgoing',q)
  local id,state=actor.resolve_player(e,player)
  if not playing or id~=player or not state or not state.ready or state.transformed then return end
  local tokens=read('tokens');if not tokens or tokens.world~=view.world or tokens.context~=context then tokens={world=view.world,context=context,ids={}}end
  while #tokens.ids<32 do local token=identity();if not token then break end;tokens.ids[#tokens.ids+1]=token end;write('tokens',tokens)
  write('authority',{world=view.world,context=context,player=player,playing=true,frame=e.GameGetFrameNum()})
  for _,row in ipairs(q.queue)do if row.applied then send(assert(json.encode({kind='d1',world=view.world,observation=row.observation})));break end end
 end
 return self
end
return M
