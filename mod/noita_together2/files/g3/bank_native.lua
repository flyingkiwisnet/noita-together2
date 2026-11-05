-- Copy-only bank adapter. No game callback waits on native/network/disk I/O.
local root='mods/noita_together2/files/g3/'
local json=dofile(root..'bank_json.lua')
local M={}
-- Pages only merge within one canonical snapshot revision. A delayed old page
-- cannot put old ranks back into the current standings.
function M.receive_race(self,e,value)
 if value.lobbyOnly then
  local scope,revision,total,offset=value.lobbyScope,value.lobbyRevision,value.lobbyTotal,value.lobbyOffset
  if type(scope)~='string'or #scope>512 or type(revision)~='string'or #revision>20 or not revision:match('^%d+$')or type(total)~='number'or total<0 or total>32 or total%1~=0 or type(offset)~='number'or offset<0 or offset>32 or offset%4~=0 or type(value.lobbyMembers)~='table'or #value.lobbyMembers>4 then return end
  local old=self.race_lobby
  if old and old.world==value.world and old.scope==scope and(#revision<#old.revision or #revision==#old.revision and revision<old.revision)then return end
  if not old or old.world~=value.world or old.scope~=scope or old.revision~=revision then self.race_lobby={world=value.world,scope=scope,revision=revision,parts={},rows={}}end
  local lobby=self.race_lobby
  for i,row in ipairs(value.lobbyMembers)do if type(row.desktop)~='string'or #row.desktop~=36 or row.desktop:find('[^0-9a-f%-]')or type(row.name)~='string'or #row.name>396 or row.name:find('[%z\1-\31]')or not({waiting=true,ready=true,disconnected=true,excluded=true})[row.status]then return end;lobby.parts[offset+i]={name=row.name,status=row.status,desktop=row.desktop,readonly=true}end
  local rows,seen={},{};for i=1,total do local row=lobby.parts[i];if not row or seen[row.desktop]then return end;seen[row.desktop]=true;rows[#rows+1]=row end
  lobby.rows=rows;lobby.complete=true;lobby.at=e.GameGetRealWorldTimeSinceStarted and e.GameGetRealWorldTimeSinceStarted();return
 end
 if value.lobbyAvailable==false or self.race_lobby and self.race_lobby.world~=value.world then self.race_lobby=nil end
 local snapshot=value.snapshot
 if type(snapshot)~='table'or type(snapshot.config)~='table'or snapshot.collectionState and snapshot.collectionState~='available'then
  self.race_parts=nil;self.race_standings=nil;value.standings={};value.standingsComplete=false;value.standingsUpdating=false;value.standingsUnavailable=true;self.race_view=value;return
 end
 if type(snapshot)=='table'and type(snapshot.config)=='table'then
  local scope=value.world..':'..tostring(snapshot.runKey);local revision=tostring(snapshot.revision or'0')
  if self.race_standings and self.race_standings.scope~=scope then self.race_standings=nil end
  local old=self.race_parts
  if old and old.scope==scope and(#revision<#old.revision or #revision==#old.revision and revision<old.revision)then return end
  if not old or old.scope~=scope or old.revision~=revision then self.race_parts={scope=scope,revision=revision,goals={},progress={},players={},standings={}}end
  local parts=self.race_parts;local offset=value.goalOffset or 0
  if parts.elapsed~=snapshot.elapsedMs or parts.elapsed_status~=snapshot.status then
   parts.elapsed=snapshot.elapsedMs;parts.elapsed_status=snapshot.status
   local now=e.GameGetRealWorldTimeSinceStarted and e.GameGetRealWorldTimeSinceStarted();parts.elapsed_at=type(now)=='number'and now==now and now>=0 and now<=1000000000 and math.floor(now*1000)or nil
  end
  value.localElapsedAt=parts.elapsed_at
  for i,goal in ipairs(snapshot.config.goals or{})do parts.goals[offset+i]=goal end
  for _,row in ipairs(snapshot.entrants or{})do if row.rowKey==snapshot.selfRow then for _,p in ipairs(row.progress or{})do parts.progress[p.instanceId]=p end end end
  for _,p in ipairs(value.players or{})do parts.players[p.actor]=p end
  for i,row in ipairs(value.standings or{})do parts.standings[(value.standingsOffset or 0)+i]=row end
  local goals={};for i=1,value.goalTotal or#parts.goals do if parts.goals[i]then goals[#goals+1]=parts.goals[i]end end;snapshot.config.goals=goals
  for _,row in ipairs(snapshot.entrants or{})do row.progress={};for _,goal in ipairs(goals)do local p=parts.progress[goal.instanceId];if p then row.progress[#row.progress+1]=p end end end
  value.players={};for _,p in pairs(parts.players)do value.players[#value.players+1]=p end
  local standings={};value.standingsComplete=true
  for i=1,value.standingsTotal or 0 do if not parts.standings[i]then value.standingsComplete=false;break end;standings[#standings+1]=parts.standings[i]end
  if value.standingsComplete then
   local order={};for i,row in ipairs(standings)do order[row]=i end
   table.sort(standings,function(a,b)
    local ar,br=tonumber(a.rank)or 0,tonumber(b.rank)or 0
    if ar<=0 then ar=math.huge end;if br<=0 then br=math.huge end
    if ar~=br then return ar<br end;return order[a]<order[b]
   end)
   self.race_standings={scope=scope,revision=revision,rows=standings}
  end
  local display=self.race_standings
  value.standingsUpdating=not value.standingsComplete
  value.standings=display and display.rows or{}
  value.standingsRevision=display and display.revision or nil
  value.standingsComplete=display~=nil
 end
 self.race_view=value
end
function M.new(dll,context,ffi,e,validate,renderer,identity)
 local self={player=nil,playing=false,scope=nil,last=nil,pending=nil,interactive=nil,queued=0,near=false,seen={},seen_count=0}
 local supported,a,b=pcall(function()return dll.nt2_g3_bank_view,dll.nt2_g3_bank_publish end);self.available=supported and a~=nil and b~=nil
 local buffer=ffi.new('uint8_t[8192]');local copy=ffi.new('uint8_t[4096]')
 local function nonce()local id=identity();return id and id:gsub('-','')end
 local function uid(rfc)local v=nonce();if v and #v==32 then if rfc==true then v=v:sub(1,12)..'4'..v:sub(14,16)..string.format('%x',8+tonumber(v:sub(17,17),16)%4)..v:sub(18)end;return v:sub(1,8)..'-'..v:sub(9,12)..'-'..v:sub(13,16)..'-'..v:sub(17,20)..'-'..v:sub(21)end end
 -- One bounded foreground intent can wait behind the current immutable copy.
 -- It keeps its ID until copied; automatic chunks cannot eat a physical click.
 local function send(body,priority)
  if priority=='user'then
   if self.interactive then return self.interactive==body end
   if not self.pending and not self.result then self.pending=body else self.interactive=body end;return true
  end
  if self.result or (self.interactive and priority~='result')then return false end
  if self.pending then return self.pending==body end;self.pending=body;return true
 end
 if e.NT2_C1_ENABLED then
  dofile_once('data/scripts/perks/perk.lua')
  dofile_once('mods/noita_together2/files/c1/scripts/ban_perks_events.lua')
  self.c1=dofile(root..'c1_local.lua').new(e,uid,function(body,completion)
   if completion then local wrapped=body:gsub('"kind":"c1"','"kind":"c1_completion"',1);if #wrapped>4096 then return false end;ffi.copy(copy,wrapped,#wrapped);local ok,value=pcall(function()return dll.nt2_g3_bank_publish(context,copy,#wrapped)end);return ok and value==1 end
   return send(body,'result')end,dofile(root..'c1_content.lua').load(e))
  _G.NT2_C1=self.c1
 end
 if e.NT2_C2_ENABLED then
  self.c2=dofile('mods/noita_together2/files/c2/local.lua').new(e,function()return uid(true)end,function(body,protected)
   if protected then if #body>4096 then return false end;ffi.copy(copy,body,#body);local ok,value=pcall(function()return dll.nt2_g3_bank_publish(context,copy,#body)end);return ok and value==1 end
   return send(body,'result')end,function(message)renderer:bank_notice(message)end)
 end
 local d1_ok,d1=pcall(function()return dofile(root..'d1_local.lua').new(e,function()return uid(true)end,function(body)
  if not self.pending and not self.result and not self.interactive then return send(body,'statistics')end;return false
 end)end)
 if d1_ok then self.d1=d1 end
 if e.ModDoesFileExist and e.ModDoesFileExist('mods/noita_together2/files/race/local.lua')then
  self.race=dofile('mods/noita_together2/files/race/local.lua').new(e,function()return uid(true)end,function(body)
   if #body>4096 then return false end;ffi.copy(copy,body,#body)
   local ok,value=pcall(function()return dll.nt2_g3_bank_publish(context,copy,#body)end);return ok and value==1
  end);_G.NT2_RACE=self.race
 end
 local api=dofile(root..'gold_wallet.lua').new(e,validate,function()return self.player,self.playing end,function()return not self.collecting or self:at_outlet()end)
 local current=api.snapshot
 local guard=dofile(root..'gold_local.lua').new(api,{},validate);self.guard=guard
 local ui=dofile(root..'bank_ui.lua').new(function(body)return send(body,'user')end,uid);self.ui=ui
 -- Only matching item profiles map the canonical catalogue dependency. Older
 -- gold profiles keep their original closure and cannot activate item effects.
 local items_ready,items=pcall(function()return dofile(root..'item_bridge.lua').new(e,uid,send,current,function()
  if not self:at_outlet()then return false end;local x,y=e.EntityGetTransform(self.outlet);return true,x,y
 end,function(message)renderer:bank_notice(message)end)end)
 if items_ready then self.items=items;ui.items=items end
 function self:at_outlet()
  local now=current();if not now then return false end
  local x,y=e.EntityGetTransform(now.player)
  for _,entity in ipairs(e.EntityGetWithTag('nt2_b1_outlet')or {})do if e.EntityGetIsAlive(entity)then local ox,oy=e.EntityGetTransform(entity);if math.abs(x-ox)+math.abs(y-oy)<24 then self.outlet=entity;return true end end end
  return false
 end
 function self:wallet()local value=current();return value and value.balance end
 function self:pause()
  if self.items then self.items:pause()end
  self.playing=false;ui.open=false;self.near=false;self.collecting=false
  -- Retiring a prior Lua owner must not write a not-yet-readable world's Globals.
  if e.GameGetWorldStateEntity then local ok,world=pcall(e.GameGetWorldStateEntity);if not ok or type(world)~='number'or world<=0 then return end end
  if e.GlobalsSetValue then e.GlobalsSetValue('nt2_b1_active','0');e.GlobalsSetValue('nt2_items_active','0')end
 end
 function self:tick(playing,player)
  if not self.available then return end
  if self.c1 and e.GameGetWorldStateEntity then local ok,world=pcall(e.GameGetWorldStateEntity);if not ok or type(world)~='number'or world<=0 then self.c1.playing=false;return end end
  self.playing=playing==true;self.player=player
  if self.c1 then
   local bytes={};for i=0,15 do bytes[#bytes+1]=string.format('%02x',tonumber(context[i]))end
   local current_context=table.concat(bytes)
   if self.c1.context and self.c1.context~=current_context then self.c1.ready=false;self.c1.event=nil;self.c1.permit=nil end
   self.c1.context=current_context;self.c1.completion=false
  end
  if self.c2 then local bytes={};for i=0,15 do bytes[#bytes+1]=string.format('%02x',tonumber(context[i]))end;local key=table.concat(bytes);if self.c2.context~=key then self.c2.ready=false;self.c2.last_near=nil end;self.c2.context=key end
  if not self.playing and not self.c1 and not self.c2 then self:pause();return end
  local copied_view,copied_c1,copied_c2=false,false,false;local ok,size=pcall(function()return dll.nt2_g3_bank_view(context,buffer,8192)end)
  if ok and size>0 then
   local body=ffi.string(buffer,size);local view=json.decode(body)
   if view and view.d1 then self.d1_view=view.d1 end
   if view and view.race then M.receive_race(self,e,view.race)end
   if view and view.c1 and view.c1.enabled==true then self.race_view=nil;self.race_parts=nil;e.GlobalsSetValue('nt2_race_active','0')end
   if self.c2 and view and view.c2 then copied_c2=self.c2:receive(view.c2)end
   if self.c1 and view and view.c1 then
    copied_c1=self.c1:receive(view.c1)
    local status=view.c1.status;if type(status)=='string'and status~=''and status~=self.c1_notice then self.c1_notice=status;renderer:bank_notice('Shared player rules: '..status)end
   end
   if view and type(view.run)=='string'and #view.run<=2048 and type(view.queued)=='number'then
    if self.scope~=view.run then self.scope=view.run;guard:bind(self.scope,{});self.pending=nil;self.interactive=nil;self.seen={};self.seen_count=0;ui.pending=nil;ui.amount='1'end
    copied_view=true;self.last=view;ui:receive(view);self.queued=view.queued
    if self.items then self.items:bind(self.scope);self.items:receive(view.items);self.queued=self.queued+(self.items.view.queued or 0)end
    local submitted=view.submission
    if submitted and submitted.status=='accepted'and submitted.direction=='withdraw'and self.take_notice~=submitted.id then
     self.take_notice=submitted.id;renderer:bank_notice('Stay on the left wing altar in the holy mountain to receive your gold')
    end
    for _,n in ipairs(view.notices or {})do if type(n.id)=='string'and type(n.text)=='string'and not self.seen[n.id] then self.seen[n.id]=true;self.seen_count=self.seen_count+1;if self.seen_count>32 then self.seen={};self.seen[n.id]=true;self.seen_count=1 end;renderer:bank_notice(n.text)end end
   end
  end
  -- Invalidate cached policy across native authority loss. While that same
  -- authority is current, mailbox rotation through another optional lane does
  -- not invalidate metadata already copied for this context during recovery.
  if not self.playing and (self.c1 and not copied_c1 or self.c2 and not copied_c2)then
   local current_ok,current_status=pcall(function()return dll.nt2_g3_status(context)end)
   if not current_ok or current_status~=2 then
    if self.c1 and not copied_c1 then self.c1.ready=false end
    if self.c2 and not copied_c2 then self.c2.ready=false end
   end
  end
  local view=self.last;local frame=e.GameGetFrameNum()
  if e.GlobalsSetValue then e.GlobalsSetValue('nt2_b1_active',view and (view.fresh and view.enabled or self.items and self.items.view.enabled)and '1'or '0');e.GlobalsSetValue('nt2_items_active',self.items and self.items.view.enabled and '1'or '0');e.GlobalsSetValue('nt2_b1_queue',tostring(self.queued));e.GlobalsSetValue('nt2_b1_player',tostring(player or 0))end
  if view and view.enabled and type(view.effect)=='table'then
   local op=view.effect;op.scope=self.scope;self.collecting=op.direction=='withdraw'
   if op.stage=='offer'and not self.pending and (not self.collecting or frame%30==0 and self:at_outlet())then
    local attempt=nonce();local receipt=attempt and guard:offer(op,attempt)
    if not receipt and not self.collecting then ui.status='Deposit waiting for an available wallet with enough gold.'end
    if receipt then send('{"kind":"prepare","id":"'..receipt.id..'","digest":"'..receipt.digest..'","attempt":"'..receipt.attempt..'"}')end
   elseif op.stage=='permit'and (not self.collecting or frame%30==0 and self:at_outlet())then
    local receipt=guard:permit(op)
    if receipt then
     if receipt.state=='applied_session'and op.direction=='withdraw'and self.outlet and e.EntityGetIsAlive(self.outlet)then local x,y=e.EntityGetTransform(self.outlet);pcall(e.EntityLoad,'data/entities/particles/poof_pink.xml',x,y)end
     self.result=('{"kind":"result","id":"'..receipt.id..'","digest":"'..receipt.digest..'","attempt":"'..receipt.attempt..'","applied":'..(receipt.state=='applied_session'and 'true'or 'false')..'}')
    end
   end
  end
  local race=self.race and self.race_view~=nil
  if race then
   if self.c1 then self.c1:close()end
   local bytes={};for i=0,15 do bytes[#bytes+1]=string.format('%02x',tonumber(context[i]))end
   self.race_view.mapVerified=e.NT2_RACE_DETECTORS and e.NT2_RACE_DETECTORS.map or false
   self.race:tick(self.playing,player,self.race_view,table.concat(bytes))
  end
  if self.c2 and not race then local permitted=self.playing;if e.nt2_c1_permitted then permitted=e.nt2_c1_permitted()end;self.c2:tick(permitted,player,frame);local status=self.c2.status or self.c2.view and self.c2.view.status;if type(status)=='string'and status~=''and status~=self.c2_notice then self.c2_notice=status;renderer:bank_notice(status)end end
  if self.c1 and not race then
   local permitted=self.playing;if e.nt2_c1_permitted then permitted=e.nt2_c1_permitted()end

   self.c1:tick(permitted,player,e.GameGetFrameNum())
   local status=self.c1.local_status;if status and status~=self.c1_notice then self.c1_notice=status;renderer:bank_notice(status)end
  end
  if self.d1 and not race then
   local bytes={};for i=0,15 do bytes[#bytes+1]=string.format('%02x',tonumber(context[i]))end
   local stats_view=self.d1_view or self.c1 and{world=self.c1.scope}
   local ok=pcall(self.d1.tick,self.d1,self.playing,player,stats_view,table.concat(bytes),self.d1_view and self.d1_view.accepted)
   if not ok then self.d1_status='Statistics capture unavailable.'end
  end
  if not self.playing then
   if self.c1 and self.c1.completion then return end
   if self.pending then ffi.copy(copy,self.pending,#self.pending);local accepted,value=pcall(function()return dll.nt2_g3_bank_publish(context,copy,#self.pending)end);if accepted and value==1 then self.pending=nil end end
   return
  end
  -- An already-mutated result owns its own bounded slot. It cannot be lost
  -- behind a retry of Prepare or an unrelated new UI submission.
  if self.result and (not self.pending or self.pending:find('"kind":"prepare"',1,true))then self.pending=self.result;self.result=nil end
  if self.items then if copied_view then self.items:tick(frame,player)else self.items:observe(frame)end end
  if not self.pending and not self.result and self.interactive then self.pending=self.interactive;self.interactive=nil end
  if self.pending then ffi.copy(copy,self.pending,#self.pending);local accepted,value=pcall(function()return dll.nt2_g3_bank_publish(context,copy,#self.pending)end);if accepted and value==1 then self.pending=nil end end
 end
 return self
end
return M
