-- UNAPPROVED native candidate: callbacks copy only DLL-owned buffers, never wait on I/O.
local M={}
function M.new(dll,context,ffi,engine,validate,renderer_module,hook_module,dispatch_module)
 local renderer=renderer_module.new(engine,validate);local state=hook_module.new(renderer.api,validate)
 local input=dofile('mods/noita_together2/files/g3/a2_native_input.lua').new(dll,context,ffi)
 local capture=dofile('mods/noita_together2/files/g3/a2_capture.lua').publisher(engine,function(value)return input:hud(value)end)
 local catalogue=dofile('mods/noita_together2/files/g3/social_catalogue.lua')
 local a3=dofile('mods/noita_together2/files/g3/social_local.lua').new(engine,function(kind,value,id)
  id=id or input:identity();if id then local accepted,copied=input:social(kind,id,value);if accepted then return id,nil,id,copied end end
  return nil,'Social controls are busy; try again',id
 end,renderer)
 local bank=dofile('mods/noita_together2/files/g3/bank_native.lua').new(dll,context,ffi,engine,validate,renderer,function()return input:identity()end);if bank.available then renderer:set_bank(bank)end
 local logging=dofile('mods/noita_together2/files/g3/social_preferences.lua').get(engine,'NT_ENABLE_LOGGER')
 local diagnostic={frames=0,received=0,known=0,biome=0,applied=0,ignored=0,expired=0,chats_received=0,chats_admitted=0,chats_visible=0,replays=0,invalid=0,invalid_pauses=0,authority_pauses=0,context_pauses=0,no_copy_lease=0,last_ttl_ms=0,last_apply_ms=0,logs=0,log_errors=0}
 local function count(key)diagnostic[key]=math.min(2147483647,diagnostic[key]+1)end
 local last_log=nil
 local function report(playing)
  local debug=engine.nt2_debug and engine.nt2_debug();if debug then debug:social(diagnostic,capture.diagnostics,input.diagnostics);if not debug.enabled then return end end
  if not logging or type(engine.print)~='function'then return end
  local now=engine.GameGetRealWorldTimeSinceStarted();if type(now)~='number'or now~=now or math.abs(now)>1000000000 or last_log and now-last_log<5 then return end;last_log=now
  local fresh,known=0,0;for _,h in pairs(renderer.model.hud or {})do fresh=fresh+1;if h.hp~=nil and h.max~=nil then known=known+1 end end
  local c=capture.diagnostics;local i=input.diagnostics
  local text='NT2_A2|v=1|playing='..(playing and '1'or '0')..'|samples='..c.samples..'|capture_known='..c.known..'|capture_unknown='..c.unknown..'|hud_copied='..i.hud_accepted..'|hud_unavailable='..i.hud_unavailable..'|poi_copied='..i.poi_accepted..'|poi_unavailable='..i.poi_unavailable..'|dismiss_unavailable='..i.dismiss_unavailable..'|native_errors='..i.native_error..'|frames='..diagnostic.frames..'|received='..diagnostic.received..'|received_known='..diagnostic.known..'|applied='..diagnostic.applied..'|ignored='..diagnostic.ignored..'|expired='..diagnostic.expired..'|last_ttl_ms='..diagnostic.last_ttl_ms..'|fresh_rows='..fresh..'|known_rows='..known..'|no_copy_lease='..diagnostic.no_copy_lease..'|replays='..diagnostic.replays..'|invalid='..diagnostic.invalid..'|authority_pauses='..diagnostic.authority_pauses..'|context_pauses='..diagnostic.context_pauses
  text=text..'|capture_biome='..c.biome..'|received_biome='..diagnostic.biome..'|last_apply_ms='..diagnostic.last_apply_ms..'|chats_received='..diagnostic.chats_received..'|chats_admitted='..diagnostic.chats_admitted..'|chats_visible='..diagnostic.chats_visible..'|show_chat='..(renderer.model.show_chat and '1'or '0')
  local n=renderer.notice_diagnostics or {};text=text..'|notice_attempted='..(n.attempted or 0)..'|notice_succeeded='..(n.succeeded or 0)..'|notice_failed='..(n.failed or 0)..'|notice_unavailable='..(n.unavailable or 0)..'|notice_dropped='..(n.dropped or 0)..'|notice_pending='..(n.pending or 0)
  count('logs');if not pcall(engine.print,text)then count('log_errors')end
 end
 local pending_dismissals={};local run=nil;local frame_started=nil
 local function remember_dismissal(sequence)
  if not state:dismiss(sequence)then return false end;pending_dismissals[sequence]=true
  if engine.GlobalsSetValue then local ids={};for id in pairs(state.dismissed)do ids[#ids+1]=id end;table.sort(ids);engine.GlobalsSetValue('nt2_a2_dismissed',table.concat(ids,','))end
  return true
 end
 local header=ffi.new('uint8_t[64]');local buffer=ffi.new('uint8_t[32768]');local roster={};local owner=nil;local scope=nil;local generation=nil
 local function uint(at) local n=0;for i=at+3,at,-1 do n=n*256+tonumber(header[i]) end;return n end
 local function authority()
  if dll.nt2_g3_social_lease(context,header,64)~=64 then return nil end
  local bytes=ffi.string(header+8,16);local gen=uint(24);local key=ffi.string(header+40,8)
  if owner~=key or generation~=gen or scope~=bytes then input:close();a3:close();state:close();renderer:close();roster={};run=nil;pending_dismissals={};owner=key;generation=gen;scope=bytes end
  local now=engine.GameGetRealWorldTimeSinceStarted();return {context=bytes,generation=gen,owner=key,playing=true,now=now,until_time=now+uint(28)/1000}
 end
 local dispatch=dispatch_module.new(authority,function(commands,now)
  for _,c in ipairs(commands) do
   if c.tag==7 then roster=c.actors;assert(state:bind(scope,generation,roster));local seen={};for _,actor in ipairs(roster) do seen[actor]=true end;for actor in pairs(renderer.peers) do if not seen[actor] then renderer.peers[actor]=nil end end
   elseif c.tag==1 then assert(state:bind(scope,generation,roster));assert(renderer:set_peer(c.actor,c.name,nil));assert(state:receive_hud(scope,generation,c.actor,c.sequence,c.hp,c.max_hp,c.sampo))
   elseif c.tag==2 then renderer.api.remove_hud(c.actor);renderer.peers[c.actor]=nil
   elseif c.tag==3 then assert(state:bind(scope,generation,roster));assert(state:begin_snapshot(scope,generation,c.high))
   elseif c.tag==4 then assert(state:bind(scope,generation,roster));for _,marker in ipairs(c.markers) do assert(state:admit_marker_actor(scope,generation,marker.actor)) end;assert(state:page(scope,generation,c.high,c.next,c.markers))
   -- Optional transport abort discards the incomplete marker prefix and stops
   -- packet application. Already authenticated information keeps only its
   -- server-owned lifecycle, exactly as for a missing optional copy lease.
   elseif c.tag==5 then state:pause(false,true)
   elseif c.tag==6 then state:close();renderer:close()
   elseif c.tag==8 then c.id=catalogue.ids[c.id+1];a3:emote(c,now)
   elseif c.tag==9 then c.robe=catalogue.robes[c.robe+1];a3:cosmetic(c,now)
   elseif c.tag==10 then
    local previous=engine.GlobalsGetValue and engine.GlobalsGetValue('nt2_a2_run','')or ''
    if run and run~=c.run then state:close();pending_dismissals={} end
    local debug=engine.nt2_debug and engine.nt2_debug();if debug then local count=0;for _,p in ipairs(c.players)do if p.actor~=c.local_actor and (p.status==nil or p.status==1 or p.status=='playing')then count=count+1 end end;debug:canonical(count)end
    renderer:roster(c.run,c.local_actor,c.players);a3:roster(c.run,c.local_actor,c.players);roster={};for _,p in ipairs(c.players)do roster[#roster+1]=p.actor end;assert(state:bind(scope,generation,roster))
    if run~=c.run then
     run=c.run;if previous==run and engine.GlobalsGetValue then for id in engine.GlobalsGetValue('nt2_a2_dismissed',''):gmatch('[^,]+')do if not state.dismissed[id]and state.dismiss_count<256 then state.dismissed[id]=true;state.dismiss_count=state.dismiss_count+1;pending_dismissals[id]=true end end
     elseif engine.GlobalsSetValue then engine.GlobalsSetValue('nt2_a2_dismissed','')end
    end
   elseif c.tag==11 then count('received');if c.biome and c.biome~=''then count('biome')end;if c.hp~=nil and c.max_hp~=nil then count('known')end;if renderer:receive_info(c)then count('applied')else count('ignored')end
   elseif c.tag==12 then count('chats_received');if renderer.model:chat(c.id,c.name,c.text)then count('chats_admitted')end
   elseif c.tag==13 then
    if c.status==0 and not renderer.model.pending then renderer.model.pending=c.id;renderer.model.draft=c.draft end
    renderer.model:outcome(c.id,c.status==1 and 'accepted'or c.status==2 and 'refused'or 'uncertain',c.reason)
   elseif c.tag==14 then for _,id in ipairs(c.dismissed)do if not state.dismissed[id]and state.dismiss_count<256 then state.dismissed[id]=true;state.dismiss_count=state.dismiss_count+1;state.markers[id]=nil;if state.pending then state.pending.markers[id]=nil end;renderer.api.remove_poi(id)end end
   elseif c.tag==15 then a3:outcome(c.id,c.status,c.reason,c.event)
   else error('unsupported native social command') end
  end;return true
 end,function(reason)count(reason=='authority'and 'authority_pauses'or reason=='context'and 'context_pauses'or 'invalid_pauses');state:pause(reason=='authority',reason~='context') end)
 renderer:set_dismiss(remember_dismissal)
 renderer:set_submit(function(message,x,y)return input:poi(message,x,y)end)
 local self={bank=bank,renderer=renderer,a3=a3,state=state,diagnostics=diagnostic,capture_diagnostics=capture.diagnostics,input_diagnostics=input.diagnostics}
 function self:tick(playing,player,observe_current)
  if self.ended then if bank.race then bank:tick(false,player);renderer:tick(false,player)end;return end
  local ended=dll.nt2_g3_social_terminal(context,buffer,32768)
  if ended>0 then local batch=dispatch_module.decode(ffi.string(buffer,ended));if batch and batch.context==ffi.string(context,16) and (not generation or batch.generation==generation) and #batch.commands==1 and batch.commands[1].tag==6 then self:close(true);self.ended=true;if bank.race then bank:tick(false,player);renderer:tick(false,player)end;return end end
  bank:tick(playing,player)
  if not playing then input:pause();a3:tick(false,player,false);state:pause(true,true);renderer:tick(false,player);capture:tick(player,observe_current==true);report(false);return end
  -- Playing is supplied by the independently validated gameplay owner. A short
  -- output-copy lease gap may defer a click; only the native publisher can copy
  -- it under current owner/status/health, and the service still validates it.
  local permission=authority();input:set_ready(permission~=nil);input:flush()
  if not permission then count('no_copy_lease');state:pause(true,true);renderer:tick(true,player);a3:tick(true,player,true);capture:tick(player,true);report(true);return end
  frame_started=engine.GameGetRealWorldTimeSinceStarted()
  local size=dll.nt2_g3_social_next(context,buffer,32768);if size>0 then count('frames');self.last_frame=ffi.string(buffer,size);local accepted,reason=dispatch:receive(self.last_frame);if not accepted then count(reason=='replay'and 'replays'or 'invalid');if reason~='replay'then state:pause(reason=='authority',reason~='context')end end end
  frame_started=nil
  for id in pairs(pending_dismissals)do if input:dismiss(id)then pending_dismissals[id]=nil end;break end
  state:tick();renderer:tick(true,player);a3:tick(true,player,true)
  -- Explicit menu input and owned Stop get the first chance at the single
  -- copy slot. A due automatic HUD refresh must not monopolize every ACK gap.
  capture:tick(player,true);report(true)
 end
 function self:before_protect(player)bank:pause();input:pause();a3:before_hold(player);renderer:before_hold(player)end
 function self:post_update()if bank.race then bank.race:post_update()end;if renderer.post_update then renderer:post_update();diagnostic.chats_visible=renderer.notice_diagnostics.chat_succeeded end end
 function self:close(terminal) if bank.c1 then bank.c1:close()end;bank:pause();input:close();a3:close();dispatch:close();state:close();renderer:close(terminal);roster={};pending_dismissals={} end
 return self
end
return M
