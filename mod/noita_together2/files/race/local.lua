local root='mods/noita_together2/files/'
local store=dofile(root..'race/storage.lua')
local json=dofile(root..'g3/bank_json.lua')
local sample=dofile(root..'race/sample.lua')
local actor=dofile(root..'g3/game_api.lua')
local progression=dofile(root..'c2/storage.lua').progression
local M={}
function M.new(e,identity,send)
 local self={ready=false};local last_sample=-30
 function self:tick(playing,player,view,context,flush)
  store.write(e,'authority',{});self.ready=false
  if type(view)~='table'or type(view.world)~='string'or #view.world~=64 then return end
  store.write(e,'mode',{race=true})
  self.status=nil;e.GlobalsSetValue('nt2_race_detector_failure','')
  e.GlobalsSetValue('nt2_race_active','1');self.context=context
  self.player=player;self.view=view
  local q,invalid=store.read(e,'outgoing')
  if invalid or q and q.world~=view.world then self.status='Race saved observations unavailable or from another world.';return end
  q=q or{world=view.world,nextSequence=1,queue={},staged={},known={},terminal='none'}
  local accepted=view.acceptedSequence or 0;local retained={};local originals={}
  for _,id in ipairs(view.accepted or{})do originals[id]=true end
  for _,row in ipairs(q.queue)do if not originals[row.eventId]then retained[#retained+1]=row end end;q.queue=retained
  q.nextSequence=math.max(q.nextSequence,accepted+1,(view.reservedSequence or 0)+1)
  self.ready=view.initialized==true
  local detectors=e.NT2_RACE_DETECTORS or{}
  if view.needsClassic and not detectors.classic or view.needsMap and not detectors.map then self.ready=false;self.status='Race detector unavailable for this game source/map.'end
  for _,goal in ipairs(view.snapshot and view.snapshot.config and view.snapshot.config.goals or{})do
   local detectors=e.NT2_RACE_DETECTORS or{}
   if goal.id=='classic_ending'and not detectors.classic or goal.id=='visit_parallel_world'and not detectors.map then self.ready=false;self.status='Race detector unavailable for this game source/map.'end
  end
  if progression(e)~=0 then self.ready=false;self.status='Race NG+ tracking is unavailable.'end
  if not self.ready and self.status then e.GlobalsSetValue('nt2_race_detector_failure',self.status)end
  -- Seal only on a later tick so all callbacks in the terminal frame can join.
  local frame=e.GameGetFrameNum()
  if not q.closed and(#q.staged>0 or q.terminal~='none')and(not q.terminalFrame or flush or frame>q.terminalFrame)and #q.queue<64 then
   local event=identity()
   if event then
    q.queue[#q.queue+1]={eventId=event,sequence=q.nextSequence,observations=q.staged,terminal=q.terminal}
    q.nextSequence=q.nextSequence+1;q.staged={};q.closed=q.terminal~='none'
   end
  end
  store.write(e,'outgoing',q)
  -- Local-copy acknowledgements never remove originals; only accepted sequence does.
  if q.queue[1]then
   q.queue[1].observations=json.array(q.queue[1].observations)
   local body='{"kind":"race_completion","world":'..assert(json.encode(view.world))..',"context":'..assert(json.encode(context))..',"commit":'..assert(json.encode(q.queue[1]))..'}'
   send(body,true)
  end
  local id,state=actor.resolve_player(e,player)
  if id==player and state and store.marker(e,player,'nt2_race_death','1')then
   e.EntityAddComponent2(player,'LuaComponent',{script_death=root..'race/player_death.lua',script_damage_received=root..'race/player_death.lua',execute_every_n_frame=-1})
  end
  if not playing or not self.ready or q.closed or q.terminalFrame or view.status~='racing'or id~=player or not state or not state.ready then return end
  store.write(e,'authority',{world=view.world,context=context,player=player,playing=true,frame=frame})
  self.ready=true
  if frame-last_sample>=30 then
   last_sample=frame;local api=e.dofile(root..'race/hook_api.lua')
   api.observe(sample.facts(e,player,false));api.observe(sample.locations(e,player,view.mapVerified==true))
  end
 end
 function self:post_update()
  local q=store.read(e,'outgoing')
  if q and q.terminalFrame and not q.closed and self.view then self:tick(false,self.player,self.view,self.context,true)end
 end
 function self:close()store.write(e,'authority',{});self.ready=false end
 return self
end
return M
