-- One per-item inlet/outlet effect guard. Original identity is attached to the
-- world entity; app journal authorization must arrive before any engine effect.
local root='mods/noita_together2/files/g3/'
local json=dofile(root..'bank_json.lua')
local M={MAX_RECEIPTS=256,MAX_ACTIVE=4,REMOVAL_FRAMES=120}
function M.new(e,codec,api,identity)
 local self={scope=nil,records={},entities={},count=0,status='',outcomes={},trace_count=0}
 local construct=dofile(root..'item_construct.lua').new(e,codec)
 local inlet_counts={};local inlet_prints=0
 local function inlet_reason(kind,reason,amount)
  local key=kind..':'..reason;local before=inlet_counts[key]or 0;local count=math.min(2147483647,before+(amount or 1));inlet_counts[key]=count
  local crossed=false;for _,threshold in ipairs({1,2,4,8,16,32,64})do if before<threshold and count>=threshold then crossed=true;break end end
  -- Fixed categories only, logarithmic sampling, bounded for this guard.
  if e.print and inlet_prints<128 and crossed then
   inlet_prints=inlet_prints+1;local frame=e.GameGetFrameNum and e.GameGetFrameNum()or 0
   pcall(e.print,'NT2_ITEM_INLET|kind='..kind..'|reason='..reason..'|count='..count..'|frame='..tostring(frame))
  end
 end
 local function inlet_kind(entity)
  for _,pair in ipairs({{'wand','wand'},{'card_action','spell'},{'item_physics','container'},{'tablet','object'},{'moon_energy','object'}})do local ok,tag=pcall(e.EntityHasTag,entity,pair[1]);if ok and tag then return pair[2]end end
  return 'unknown'
 end
 local function marker(entity)
  for _,c in ipairs(e.EntityGetComponentIncludingDisabled(entity,'VariableStorageComponent')or {})do if e.ComponentGetValue2(c,'name')=='nt2_item_original'then return c,e.ComponentGetValue2(c,'value_string')end end
 end
 local function stored(id)return e.GlobalsGetValue and e.GlobalsGetValue('nt2_item_'..id,'')or ''end
 local function persist(r,state)
  r.state=state
  if not e.GlobalsSetValue then return false end
  local value=self.scope..':'..r.digest..':'..state;e.GlobalsSetValue('nt2_item_'..r.id,value)
  return stored(r.id)==value
 end
 local function trace(r,state,reason,frame)
  local event={id=r.id,attempt=r.attempt or string.rep('0',32),state=state,reason=reason,frame=frame or 0}
  self.outcomes[#self.outcomes+1]=event;if #self.outcomes>32 then table.remove(self.outcomes,1)end
  -- Fixed reasons and locally generated IDs only; bounded logger evidence,
  -- never a network authority or loaded-save completion proof.
  if e.print and self.trace_count<256 then self.trace_count=self.trace_count+1;pcall(e.print,'NT2_ITEM_EFFECT|id='..r.id..'|attempt='..event.attempt..'|state='..state..'|reason='..reason..'|frame='..tostring(frame or 0))end
 end
 local function outcome(r,state,reason,frame)r.state=state;trace(r,state,reason,frame)end
 local function waiting(r,reason,frame)
  if r.wait_reason~=reason then r.wait_reason=reason;trace(r,'waiting',reason,frame)end
 end
 local function resumed(r,frame)if r.wait_reason then r.wait_reason=nil;trace(r,'resumed','source_revalidated',frame)end end
 local function same_original(r)
  if not api:revalidate(r.snapshot)then return false end
  local c,value=marker(r.snapshot.entity);if c~=r.marker or value~=self.scope..':'..r.id then return false end
  local ok,item=pcall(codec.capture,codec,api,e,r.snapshot.entity);return ok and json.encode(item)==r.body
 end
 function self:observe_removals(frame,op,authorized,only_id)
  for _,r in pairs(self.records)do if r.state=='removing'and (not only_id or r.id==only_id)then
   local selected=op;if op and not op.id then selected=nil;for _,candidate in ipairs(op)do if candidate.id==r.id then selected=candidate;break end end end
   local exact=authorized==true and selected and selected.stage=='permit'and selected.id==r.id and selected.attempt==r.attempt and selected.digest==r.digest and selected.direction==r.direction
   if not exact then outcome(r,'uncertain','authority_lost',frame)
   elseif type(frame)~='number'or frame<r.removal_frame or frame-r.removal_frame>M.REMOVAL_FRAMES then outcome(r,'uncertain','removal_timeout',frame)
   elseif stored(r.id)~=self.scope..':'..r.digest..':uncertain'then outcome(r,'uncertain','evidence_changed',frame)
   else
    local ok,alive=pcall(e.EntityGetIsAlive,r.snapshot.entity)
    if not ok or type(alive)~='boolean'then outcome(r,'uncertain','observation_failed',frame)
    elseif not alive then
     if persist(r,'applied_session')then outcome(r,'applied_session','removal_observed',frame)else outcome(r,'uncertain','completion_not_persisted',frame)end
    else local valid,unchanged=pcall(same_original,r);if not valid or not unchanged then outcome(r,'uncertain','original_changed',frame)end end
   end
  end end
 end
 function self:bind(scope)if self.scope~=scope then
  -- A terminal positive deleted-bank reset releases only metadata on originals
  -- which this local guard has definitely never mutated. It is not a refund.
  if scope=='deleted'then for id,r in pairs(self.records)do if r.snapshot and (r.state=='captured'or r.state=='prepared'or r.state=='refused')and e.EntityGetIsAlive(r.snapshot.entity)then
   local c,value=marker(r.snapshot.entity);if c==r.marker and value==self.scope..':'..id then e.EntityRemoveComponent(r.snapshot.entity,c)end
  end end end
  self.scope=scope;self.records={};self.entities={};self.count=0
 end end
 function self:active_count()
  local count=0;for _,r in pairs(self.records)do if r.state=='captured'or r.state=='prepared'or r.state=='removing'or r.state=='constructing'then count=count+1 end end;return count
 end
 function self:capture(inlet)
  for id,r in pairs(self.records)do if r.state=='refused'and not api:dropped(r.snapshot.entity,r.snapshot.inlet)then
   if e.EntityGetIsAlive(r.snapshot.entity)then local c,value=marker(r.snapshot.entity);if c==r.marker and value==self.scope..':'..id then e.EntityRemoveComponent(r.snapshot.entity,c)end end
   self.records[id]=nil;self.entities[r.snapshot.entity]=nil;self.count=self.count-1
  end end
  if self.count>=M.MAX_RECEIPTS then inlet_reason('unknown','retained_limit');return nil end
  if self:active_count()>=M.MAX_ACTIVE then inlet_reason('unknown','active_limit');return nil end
  local candidates,reason,outside=api:candidates(inlet);if outside and outside>0 then inlet_reason('wand','nearby_outside_radius',outside)end;if not candidates then self.status=reason;inlet_reason('unknown','candidate_limit');return nil end
  for _,entity in ipairs(candidates)do local snapshot,excluded=api:dropped(entity,inlet);local kind=inlet_kind(entity)
   if not snapshot then inlet_reason(kind,excluded or 'geometry_unavailable')end
   if snapshot and not self.entities[entity]then
    local ok,item=pcall(codec.capture,codec,api,e,entity)
    if ok then local body=json.encode(item);if body then
     local c,value=marker(entity);local prefix=self.scope..':'
     if value and value:sub(1,#prefix)~=prefix then local old=value:match(':([^:]+)$');if old and e.GlobalsGetValue and e.GlobalsGetValue('nt2_item_refused_'..old,'')==value then e.EntityRemoveComponent(entity,c);c=nil;value=nil end end
     if value and value:sub(1,#prefix)~=prefix then inlet_reason(kind,'foreign_original')end
     if not value or value:sub(1,#prefix)==prefix then
      local id=value and value:sub(#prefix+1)or identity()
      if id and not self.records[id]then
       if not c then c=e.EntityAddComponent2(entity,'VariableStorageComponent',{name='nt2_item_original',value_string=prefix..id})end
       if c and e.ComponentGetValue2(c,'value_string')==prefix..id then
        local record={id=id,item=item,body=body,snapshot=snapshot,marker=c,state='captured'};self.records[id]=record;self.entities[entity]=id;self.count=self.count+1
        return record
       else inlet_reason(kind,'marker_unavailable')end
      end
     end
    else inlet_reason(kind,'descriptor_size')end else
     local text=tostring(item);local reason=text:find('Unsupported wand sprite',1,true)and 'sprite_unmapped'or text:find('Wand card bound',1,true)and 'wand_cards_bound'or text:find('Wand child bound',1,true)and 'wand_cards_bound'or text:find('unavailable',1,true)and 'engine_field_unavailable'or 'capture_rejected'
     if reason=='sprite_unmapped'and api.sprite_diagnostic then reason=api:sprite_diagnostic(entity)end
     inlet_reason(kind,reason);self.status='Item capture unavailable; original retained.'
    end
   end
  end
 end
 function self:refused(id)
  local r=self.records[id];if not r or r.state~='captured'then return end
  -- Frozen run policy/capacity refusal keeps the usable original and prevents
  -- a new operation every30frames for that same rejected dropped entity.
  r.state='refused';if e.GlobalsSetValue then e.GlobalsSetValue('nt2_item_refused_'..id,self.scope..':'..id)end
 end
 function self:offer(op,item,attempt,at_outlet)
  if not self.scope or not op or type(op.id)~='string'or type(op.digest)~='string'or #op.digest~=64 or op.digest:find('[^a-f0-9]')or not codec:resolve(item)then return nil end
  local r=self.records[op.id]
  if r and r.state=='prepared'then if r.digest==op.digest and r.direction==op.direction then return r end;return nil end
  if stored(op.id)~=''then self.status='Earlier item effect unresolved; it will not replay.';return nil end
  if op.direction=='deposit'then
   if not r or r.state~='captured'then return nil end;if not api:revalidate(r.snapshot)then waiting(r,'source_unavailable',e.GameGetFrameNum and e.GameGetFrameNum());return nil end
   local ok,current=pcall(codec.capture,codec,api,e,r.snapshot.entity)
   if not ok or json.encode(current)~=r.body or json.encode(item)~=r.body then waiting(r,'descriptor_changed',e.GameGetFrameNum and e.GameGetFrameNum());self.status='Dropped item changed; original retained.';return nil end
  elseif op.direction=='claim'then
   if not at_outlet or self.count>=M.MAX_RECEIPTS then return nil end
   r={id=op.id,item=item};self.records[op.id]=r;self.count=self.count+1
  else return nil end
  r.digest=op.digest;r.direction=op.direction;r.attempt=attempt;r.state='prepared';return r
 end
 function self:permit(op,at_outlet,x,y,authorized,frame)
  local r=op and self.records[op.id]
  if not r or r.digest~=op.digest or r.attempt~=op.attempt or r.direction~=op.direction or authorized~=true then return nil end
  if r.state=='removing'then return nil end
  if r.state=='applied_session'or r.state=='uncertain'then return r end
  if r.direction=='claim'and not at_outlet then return nil end
  if r.state=='prepared'then
   if r.direction=='deposit'then
    if not api:revalidate(r.snapshot)then waiting(r,'source_unavailable',frame);return nil end
    local ok,current=pcall(codec.capture,codec,api,e,r.snapshot.entity);local c,value=marker(r.snapshot.entity)
    if not ok or json.encode(current)~=r.body then waiting(r,'descriptor_changed',frame);return nil end
    if c~=r.marker or value~=self.scope..':'..r.id then waiting(r,'identity_changed',frame);return nil end
    resumed(r,frame)
    -- Persist uncertainty BEFORE calling EntityKill, including thrown callbacks.
    if not persist(r,'uncertain')then return nil end
    local killed=pcall(e.EntityKill,r.snapshot.entity)
    frame=frame or (e.GameGetFrameNum and e.GameGetFrameNum())or 0
    if not killed then outcome(r,'uncertain','kill_threw',frame);return r end
    -- Only this live attempt can observe a deferred removal. Durable state
    -- stays uncertain until absence is observed; cold guards never resume it.
    r.removal_frame=frame;outcome(r,'removing','kill_returned',frame)
    local live={id=r.id,digest=r.digest,attempt=r.attempt,direction=r.direction,stage='permit'}
    self:observe_removals(frame,live,true,r.id)
    if r.state~='removing'then return r end
    return nil
   end
   local job,reason=construct:start(r.item,x,y);if not job then self.status=reason;return nil end
   if not persist(r,'uncertain')then return nil end;r.job=job;r.state='constructing'
  end
  if r.state=='constructing'then local state=r.job:step(authorized);if state=='applied_session'or state=='uncertain'then persist(r,state);if state=='applied_session'then pcall(e.EntityLoad,'data/entities/particles/poof_pink.xml',x,y)end;return r end end
 end
 return self
end
return M
