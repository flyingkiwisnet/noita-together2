-- Player list/radar/chat/composer over the existing fixed HUD and POI adapter.
local M={}
local root='mods/noita_together2/files/g3/'
local text=dofile(root..'a2_text.lua');local models=dofile(root..'a2_ui_model.lua');local biomes=dofile(root..'a2_biomes.lua');local locks=dofile(root..'a2_input_lock.lua')
local preferences=dofile(root..'social_preferences.lua')
local player_list=dofile(root..'a2_player_list.lua')
function M.new(engine,validate)
 local e=engine or _G
 -- Delivery queue, not chat history: at most32 notices, four per post-update,
 -- and at most two callback opportunities for an unavailable/throwing printer.
 local notices={};local notice_stats={pending=0,attempted=0,succeeded=0,failed=0,unavailable=0,dropped=0,chat_succeeded=0}
 local function count(key,n)notice_stats[key]=math.min(2147483647,notice_stats[key]+(n or 1))end
 local function notice(s,kind)if kind~='chat'and kind~='bank'and not preferences.get(e,'NT_HINTS')then return true end;if #notices>=32 then return false end;notices[#notices+1]={text=s,kind=kind,tries=0};notice_stats.pending=#notices;return true end
 local self={gui=nil,hud={},peers={},pois={},entities={},playing=false,dismiss=nil,submit=nil,player=nil,caps=false}
 local model=models.new(text,notice);self.model=model;local lock=locks.new(e);self.lock=lock;self.notice_diagnostics=notice_stats
 function self:post_update()
  for _=1,4 do local item=notices[1];if not item then break end
   local done=item.kind=='chat'and not model.show_chat
   if not done then
    item.tries=item.tries+1
    if type(e.GamePrint)~='function'then count('unavailable')
    else count('attempted');local ok=pcall(e.GamePrint,' '..item.text);if ok then count('succeeded');if item.kind=='chat'then count('chat_succeeded')end;done=true else count('failed')end end
    if not done and item.tries>=2 then count('dropped');done=true end
   end
   if done then table.remove(notices,1)else break end
  end;notice_stats.pending=#notices
 end
 local preferences_loaded=false
 local function visibility()if e.a2_ghost_visibility then e.a2_ghost_visibility(not model.show_ghosts,model.opacity)end end
 local function save_preferences()preferences.set(e,'NT_GHOST_OPACITY',model.opacity);if e.GlobalsSetValue then e.GlobalsSetValue('nt2_a2_hide_chat',model.show_chat and '0'or '1');e.GlobalsSetValue('nt2_a2_hide_ghosts',model.show_ghosts and '0'or '1');e.GlobalsSetValue('nt2_a2_ghost_opacity',tostring(model.opacity))end end
 local function remove(sequence)local entity=self.entities[sequence];if entity then if e.EntityGetIsAlive(entity)then e.EntityKill(entity)end;self.entities[sequence]=nil end end
 local function literal(s,max)return type(s)=='string' and #s<=max and not s:find('[%z\1-\31\127]')end
 local function persist_follow()
  if not e.GlobalsSetValue or not model.run then return end
  local ids={};for id in pairs(model.follow)do ids[#ids+1]=id end;table.sort(ids)
  local choices={};for actor in pairs(self.follow_seen or {})do choices[#choices+1]=actor..':'..(model.follow[actor]and '1'or '0')end;table.sort(choices)
  e.GlobalsSetValue('nt2_a2_run',model.run);e.GlobalsSetValue('nt2_a2_follow',table.concat(ids,','));e.GlobalsSetValue('nt2_a3_follow_choices',table.concat(choices,','))
 end
 function self:roster(run,local_actor,players)
  local changed=run~=model.run;model:roster(run,players);self.peers=model.peers;self.local_actor=local_actor
  if e.a2_ghost_names then local names={};for _,peer in pairs(model.peers)do if peer.actor~=local_actor and type(peer.ghost)=='string'then names[peer.ghost]=peer.name end end;e.a2_ghost_names(names)end
  if changed then self.follow_seen={};self.follow_choices={};if e.GlobalsGetValue and e.GlobalsGetValue('nt2_a2_run','')==run then for entry in e.GlobalsGetValue('nt2_a3_follow_choices',''):gmatch('[^,]+')do local actor,value=entry:match('^(.-):([01])$');if actor then self.follow_choices[actor]=value=='1'end end end end
  self.follow_seen=self.follow_seen or {};self.follow_choices=self.follow_choices or {}
  if changed and e.GlobalsGetValue and e.GlobalsGetValue('nt2_a2_run','')==run then for id in e.GlobalsGetValue('nt2_a2_follow',''):gmatch('[^,]+')do if model.peers[id]then model.follow[id]=true end end end
  for _,p in ipairs(players)do if not self.follow_seen[p.actor]then self.follow_seen[p.actor]=true;local choice=self.follow_choices[p.actor];if choice~=nil then model.follow[p.actor]=choice or nil elseif p.actor~=local_actor and preferences.get(e,'NT_FOLLOW_DEFAULT')then model.follow[p.actor]=true end end end
  for actor in pairs(self.follow_seen)do if not self.peers[actor]then self.follow_seen[actor]=nil;self.follow_choices[actor]=nil end end
  persist_follow()
 end
 function self:set_peer(actor,name,depth)if not validate.id(actor)or not literal(name,396)then return false end;self.peers[actor]={actor=actor,name=name,depth=depth,status='playing'};model.peers[actor]=self.peers[actor];return true end
 self.api={time=function()return e.GameGetRealWorldTimeSinceStarted()end,
  clear_hud=function(keep_current)
   self.hud={}
   if not keep_current then model.hud={};return end
   -- Optional transport gates do not expire server-owned player state.
   for actor in pairs(model.hud)do if not self.peers[actor]then model.hud[actor]=nil end end
  end,remove_hud=function(actor)self.hud[actor]=nil;model.hud[actor]=nil end,
  hud=function(actor,hp,max,sampo)self.hud[actor]={hp=hp,max=max,sampo=sampo};model.hud[actor]=self.hud[actor]end,
  clear_pois=function()for sequence in pairs(self.entities)do remove(sequence)end;self.pois={}end,
  remove_poi=function(sequence)remove(sequence);self.pois[sequence]=nil end,
  poi=function(sequence,actor,message,x,y,name)if not literal(message,396)or name and not literal(name,396)then return false end;self.pois[sequence]={actor=actor,text=message,x=x,y=y,name=name};return true end}
 function self:receive_info(c)
  local peer=self.peers[c.actor];if not peer or peer.status==3 or peer.status=='disconnected'then return false end
  local revision=c.revision;local old=model.hud_versions[c.actor]
  if old and revision<old.revision then return false end
  local raw={hp=c.hp,max=c.max_hp,sampo=c.sampo,biome=c.biome or ''}
  if old and revision==old.revision then
   local v=old.raw;if v.hp~=raw.hp or v.max~=raw.max or v.sampo~=raw.sampo or v.biome~=raw.biome then return false end
   if not model.hud[c.actor]then model.hud[c.actor]=old.value;return true end;return false
  end
  local prior=old and old.value or {}
  local value={hp=raw.hp~=nil and raw.hp or prior.hp,max=raw.max~=nil and raw.max or prior.max,sampo=raw.sampo,biome=raw.biome~=''and raw.biome or prior.biome,revision=revision}
  if raw.sampo==nil then value.sampo=prior.sampo end
  model.hud_versions[c.actor]={revision=revision,raw=raw,value=value};model.hud[c.actor]=value;return true
 end
 function self:set_dismiss(callback)self.dismiss=callback end
 function self:bank_notice(message)return notice(message,'bank')end
 function self:set_bank(bank)self.bank=bank end
 function self:set_submit(callback)self.submit=callback end
 function self:before_hold(player)lock:before_hold(player);model.composer=false;model.bank_open=false;if self.bank then self.bank:pause()end end
 function self:close(terminal)
  count('dropped',#notices);notices={};notice_stats.pending=0
  lock:close(self.playing);self.api.clear_pois();self.api.clear_hud();self.playing=false;model.composer=false;model.bank_open=false;if self.bank then self.bank:pause()end
  if terminal and self.bank and self.bank.race_view then model.hud={}
  elseif terminal then model:roster(nil,{});self.peers=model.peers;if e.a2_ghost_names then e.a2_ghost_names({})end;if e.GlobalsSetValue then e.GlobalsSetValue('nt2_a2_run','');e.GlobalsSetValue('nt2_a2_follow','')end end
  if self.gui then e.GuiDestroy(self.gui);self.gui=nil end
 end
 local function translated(key)
  if not literal(key,128)or key==''or key=='_EMPTY_'then return 'Unknown location'end
  if e.GameTextGetTranslatedOrNot then local ok,v=pcall(e.GameTextGetTranslatedOrNot,key);if ok and literal(v,396)and v~=''and (v~=key or key:sub(1,1)~='$')then return v end end
  return key:sub(1,1)~='$'and key or 'Unknown location'
 end
 function self:tick(playing,player)
  if self.player~=player then lock:close(playing and self.playing);model.composer=false;model.bank_open=false;if self.bank then self.bank.ui.open=false end end
  self.playing=playing;self.player=player
  if self.bank and not self.bank.ui.open then model.bank_open=false end
  if not preferences_loaded then preferences_loaded=true;if e.GlobalsGetValue then model.show_chat=e.GlobalsGetValue('nt2_a2_hide_chat','0')~='1';model.show_ghosts=e.GlobalsGetValue('nt2_a2_hide_ghosts','0')~='1';local alpha=tonumber(e.GlobalsGetValue('nt2_a2_ghost_opacity','1'));if alpha and alpha==alpha then model.opacity=math.max(0,math.min(1,alpha))end end;visibility()end
  if e.ModSettingGet then local alpha=preferences.get(e,'NT_GHOST_OPACITY');if model.opacity~=alpha then model.opacity=alpha;visibility()end end
  if not playing then lock:close(false);model.composer=false;model.bank_open=false;if self.bank then self.bank.ui.open=false end end
  local now=self.api.time();if not self.gui then self.gui=e.GuiCreate()end;local gui=self.gui;e.GuiStartFrame(gui)
  -- Recreated GUI contexts must use these fixed screen coordinates immediately.
  -- Noita clears frame options in GuiStartFrame, so apply this on every draw.
  if e.GuiOptionsAdd and e.GUI_OPTION and e.GUI_OPTION.NoPositionTween then e.GuiOptionsAdd(gui,e.GUI_OPTION.NoPositionTween)end
  local toolbar_y=0
  local widget=100
  local function id()widget=widget+1;return widget end
  -- Stateful scroll widgets must not inherit another panel's ID/cache when toggled.
  local race_widget,players_widget=10000,11000
  local function race_id()race_widget=race_widget+1;return race_widget end
  local function players_id()players_widget=players_widget+1;return players_widget end
  local function button(x,y,label,icon)
   local clicked;if e.GuiImageButton and icon then clicked=e.GuiImageButton(gui,id(),x,y,'',root..'ui/'..icon..'.png')else clicked=e.GuiButton(gui,id(),x,y,label)end
   if e.GuiTooltip then e.GuiTooltip(gui,label,'')end;return clicked
  end
  if button(79,toolbar_y,'World message','keyboard')and not model.bank_open and playing and not model.pending then
   if model.composer then model.composer=false;lock:close(playing)else if lock:open(self.player,playing)then model.composer=true;model.show_players=false else model.feedback='Controls are unavailable' end end
  end
  if button(100,toolbar_y,model.show_ghosts and 'Hide ghosts' or 'Show ghosts',model.show_ghosts and 'player_ghosts' or 'hide_player_ghosts')then model.show_ghosts=not model.show_ghosts;visibility();save_preferences()end
  if button(120,toolbar_y,model.show_chat and 'Hide chat' or 'Show chat',model.show_chat and 'chat' or 'hide_chat')then model.show_chat=not model.show_chat;save_preferences()end
  local race=self.bank and self.bank.race_view
  if button(race and 5 or 140,race and 36 or toolbar_y,race and(model.show_players and'Hide players'or'Show players')or'Players',not race and'player_list'or nil)then model.show_players=not model.show_players;if model.show_players then model.composer=false;model.bank_open=false;if self.bank then self.bank.ui.open=false end;lock:close(playing)end end
  if self.bank and button(160,toolbar_y,'Gold bank',preferences.get(e,'NT_SHOW_WUOTE_FRIDGE')and 'fridge'or 'bank')then
   if model.bank_open then model.bank_open=false;self.bank.ui.open=false;lock:close(playing)
   elseif playing then model.composer=false;lock:close(playing);if lock:open(self.player,playing)then model.bank_open=true;self.bank.ui.open=true;model.show_players=false end end
  end
  if button(self.bank and 180 or 160,toolbar_y,string.format('Ghost opacity %d%%',model.opacity*100))then model.opacity=model.opacity<=0 and 1 or math.max(0,model.opacity-0.25);visibility();save_preferences()end
  local positions=e.a2_positions and e.a2_positions()or {};local px,py
  if self.player and e.EntityGetIsAlive(self.player)then px,py=e.EntityGetTransform(self.player)end
  local width,height=640,360;if e.GuiGetScreenDimensions then width,height=e.GuiGetScreenDimensions(gui)end
  if self.bank and self.bank.race_view then
   if button(width<400 and 5 or 300,toolbar_y+(width<400 and 14 or 0),model.show_race==false and'Show goals'or'Hide goals')then model.show_race=model.show_race==false end
   if model.show_race~=false and not model.composer and not model.bank_open then dofile('mods/noita_together2/files/race/panel.lua').draw(e,gui,race_id,self.bank.race_view,width,height,model.show_players and 110 or 0)end
  end
  local rows=model:rows()
  if race and not playing and self.bank.race_view.lobbyAvailable then
   local lobby=self.bank.race_lobby;local clock=e.GameGetRealWorldTimeSinceStarted and e.GameGetRealWorldTimeSinceStarted()
   rows=lobby and lobby.complete and type(clock)=='number'and type(lobby.at)=='number'and clock>=lobby.at and clock-lobby.at<=3 and lobby.rows or{}
  end
  if self.bank and self.bank.race_view then
   local counts={};for _,p in ipairs(self.bank.race_view.players or{})do counts[p.actor]=p end
   for _,row in ipairs(rows)do local p=counts[row.actor];if p then row.race_count=p.count;row.race_status=p.status;row.race_connection_stale=self.bank.race_view.status~='racing'end end
  end
  if model.show_players then
   if race and #rows==0 then e.GuiText(gui,5,50,'Player roster unavailable')end
   player_list.draw(e,gui,race and players_id or id,rows,function(row)
    local peer=self.peers[row.actor];local pose=peer and positions[peer.ghost]
    return row.hud,row.actor==self.local_actor and py or pose and pose.y
   end,function(actor)model:toggle_follow(actor);persist_follow()end,translated,self.bank and self.bank.items and function(row,right,hovered)
    return self.bank.items:player_row(e,gui,id,row,right,hovered)
   end or nil)
  end
  if px and py then for actor in pairs(model.follow)do local p=self.peers[actor];local pos=p and positions[p.ghost];if pos then local x,y,d=models.radar(pos.x,pos.y,px,py,width,height);if x then
   if e.GuiImage then e.GuiImage(gui,id(),x,y,root..'ui/radar.png',1,1,1)end
   local name=p.name;local label=' '..name..' '..d..'m';local tx=x
   if e.GuiGetTextDimensions then local w=e.GuiGetTextDimensions(gui,label);while w>width-10 and #name>0 do name=text.backspace(name);label=' '..name..'... '..d..'m';w=e.GuiGetTextDimensions(gui,label)end;tx=math.max(5,math.min(width-w-5,x))end
   e.GuiText(gui,tx,math.min(height-12,y+12),label)
  end end end end
  if model.composer and playing then
   local x,y=width/2-90,height/2-90
   if e.GuiTextInput and not model.pending then local value=e.GuiTextInput(gui,id(),x,y,model.draft,150,396);model:edit(value)end
   local keys={'1234567890','qwertyuiop','asdfghjkl','zxcvbnm'};for row,keyrow in ipairs(keys)do for col=1,#keyrow do local c=keyrow:sub(col,col);if self.caps then c=c:upper()end;if button(x+(col-1)*16,y+row*13,c)then model:edit(model.draft..c)end end end
   if button(x,y+65,'CAPS')then self.caps=not self.caps end;if button(x+45,y+65,'Space')then model:edit(model.draft..' ')end;if button(x+90,y+65,'Backspace')then model:edit(text.backspace(model.draft))end
   e.GuiText(gui,x,y+80,string.format('%d/99 - %d/396 bytes',text.length(model.draft)or 0,#model.draft))
   local enter=e.InputIsKeyJustDown and e.InputIsKeyJustDown(40);local escape=e.InputIsKeyJustDown and e.InputIsKeyJustDown(41)
   if (button(x,y+95,'Place message')or enter)and self.submit and px and py then model:submit(self.submit,px,py,playing)end
   if button(x+85,y+95,model.pending and 'Close pending'or 'Cancel')or escape then if model.pending then model.composer=false;lock:close(playing)elseif model:cancel()then lock:close(playing)end end
  end
  if model.feedback~=''then e.GuiText(gui,5,height-20,' '..model.feedback)end
  if self.bank and model.bank_open and playing then self.bank.ui:draw(e,gui,id,button,self.bank:wallet(),width,height);model.bank_open=self.bank.ui.open end
  if not model.composer and not model.bank_open and lock.lease then lock:close(playing)end
  if playing then
   local budget=8;for sequence,poi in pairs(self.pois)do local entity=self.entities[sequence]
    if entity and not e.EntityGetIsAlive(entity)then if self.dismiss and self.dismiss(sequence)then self.entities[sequence]=nil;self.pois[sequence]=nil end
    elseif not entity and budget>0 then local entity=e.EntityLoad(root..'poi.xml',poi.x,poi.y);if entity and entity~=0 then self.entities[sequence]=entity;local item=e.EntityGetFirstComponentIncludingDisabled(entity,'ItemComponent');if item then e.ComponentSetValue2(item,'item_name',poi.actor==self.local_actor and 'My message'or ' '..(poi.name or 'NT2 player'));e.ComponentSetValue2(item,'ui_description',' '..poi.text)end end;budget=budget-1 end
   end
  end
 end
 return self
end
return M
