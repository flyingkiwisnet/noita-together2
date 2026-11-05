-- Item/inventory copies share the existing native bank mailbox and lifecycle.
local root='mods/noita_together2/files/g3/'
local json=dofile(root..'bank_json.lua');local chunks=dofile(root..'item_chunks.lua')
local M={}
local function older(a,b)a=tostring(a or '0');b=tostring(b or '0');return #a<#b or #a==#b and a<b end
function M.new(e,identity,send,eligible,outlet,notice)
 local catalogue=dofile(root..'item_catalogue.lua');local codec=dofile(root..'item_descriptor.lua').new(catalogue)
 local api=dofile(root..'item_game_api.lua').new(e);local guard=dofile(root..'item_local.lua').new(e,codec,api,identity)
 local self={catalogue=catalogue,codec=codec,scope=nil,view={},uploads={},receiver=chunks.receiver(),complete={},order={},effects={},bank=nil,detail=nil,preview=nil,players={},guard=guard,inventory_revision=0,claimed={},claim_ids={}}
 local function request(value)local body=json.encode(value,{bytes=4096,nodes=512,depth=12});return body and send(body,value.kind=='item_claim'and 'user'or value.kind=='item_result'and 'result'or nil)==true end
 function self:upload(id,channel,body)
  if #self.uploads>=4 then return false end;local sender=chunks.sender(id,channel,body);if not sender then return false end;self.uploads[#self.uploads+1]=sender;return true
 end
 function self:query(query)
  if not request({kind='item_query',query=query})then return false end
  local key=json.encode(query);self.requested_query=key;self.bank=self.banks and self.banks[key]or nil;return true
 end
 -- Hover is display-only: coalesce it before the shared mailbox, with one
 -- outstanding read and a bounded cache of immutable item descriptors.
 function self:inspect(id)
  self.inspect_wanted=id
  if self.details and self.details[id]then self.detail=self.details[id];self.inspect_wanted=nil end
  return true
 end
 function self:cancel_inspect()self.inspect_wanted=nil end
 function self:inspect_player(actor,slot)return request({kind='inventory_inspect',actor=actor,slot=slot or 0})end
 function self:player_row(e,gui,id,row,right,hovered)
  if not row.actor then return end
  if right then
   local close=self.preview_mode and self.preview_selected==row.actor
   self.preview_mode=not close;self.preview_selected=not close and row.actor or nil;self.preview=nil;self.preview_actor=nil;self.preview_request=nil;self.preview_slot=0
  end
  if not self.preview_mode or self.preview_selected~=row.actor then return end
  local function feedback(message)if e.GuiTooltip then e.GuiTooltip(gui,' '..row.name..' Wands',message)end;return true end
  local peer;for _,p in ipairs(self.players)do if p.actor==row.actor then peer=p end end
  if not peer or type(peer.session)~='string'or #peer.session~=36 then self.preview=nil;return feedback('Wand preview unavailable until current inventory information is received after Start.')end
  local frame=e.GameGetFrameNum();local key=peer.actor..':'..tostring(peer.session)..':'..tostring(peer.revision)..':'..tostring(self.preview_slot or 0)
  if self.preview_request~=key or frame-(self.preview_frame or -120)>120 then
   if self:inspect_player(peer.session,self.preview_slot or 0)then self.preview_request=key;self.preview_frame=frame;self.preview_actor=peer.actor end
  end
  local view=self.preview;if view and view.gameSessionId==peer.session then
   local lines={};for _,wand in ipairs(view.wands or {})do if wand.wand then local item={catalogueRevision=catalogue.revision,wand=wand.wand};lines=dofile(root..'item_presenter.lua').lines(e,codec,item);table.insert(lines,1,'Wand slot '..tostring(wand.slot or 0))else lines={wand.unavailableReason or 'Unavailable wand'}end end
   if #lines==0 then lines={view.state=='empty'and 'No wands'or view.state=='removed'and 'Player left'or 'Wands not observed'}end
   if e.GuiZSet then e.GuiZSet(gui,8)end;if e.GuiZSetForNextWidget then e.GuiZSetForNextWidget(gui,8)end
   if e.GuiTooltip then e.GuiTooltip(gui,' '..row.name..' Wands',table.concat(lines,'\n'))end
   -- Preview mode keeps slot navigation on the hovered row.
   if view.nextSlot and view.nextSlot~=''then if e.GuiButton(gui,id(),70,0,'Next wand')then self.preview_slot=tonumber(view.nextSlot)or 0;self.preview_request=nil end
   elseif (self.preview_slot or 0)>0 then if e.GuiButton(gui,id(),70,0,'First wand')then self.preview_slot=0;self.preview_request=nil end end
   return true
  end
  return feedback(self.preview_actor==peer.actor and 'Loading wands...'or 'Wand preview request waiting for connection.')
 end
 function self:claim(row)
  if not self.view.enabled or row.claimAllowed~=true or self.claim_ids[row.itemId]then self.status='Item unavailable or disabled.';return false end
  local id=identity();if not id then return false end
  if request({kind='item_claim',id=id,item_id=row.itemId,revision=row.revision})then self.status='Claim requested.';self.claimed[row.itemId]=row.revision;self.claim_ids[row.itemId]=id;return true end;return false
 end
 function self:bind(scope)
  if self.scope==scope then return end;self.scope=scope;guard:bind(scope);self.uploads={};self.receiver:reset();self.complete={};self.order={};self.effects={};self.bank=nil;self.banks={};self.bank_keys={};self.requested_query=nil;self.detail=nil;self.preview=nil;self.preview_mode=false;self.preview_selected=nil;self.preview_request=nil;self.players={};self.inventory_body=nil;self.inventory_revision=0;self.inspect_wanted=nil;self.inspect_inflight=nil;self.inspect_due=nil;self.details={};self.detail_order={};self.next_scan=nil;self.query_needed=true;self.claimed={};self.claim_ids={};self.ack=nil;self.ack_sent=nil;self.result=nil;self.reported={}
 end
 function self:receive(view)
  self.view=view or {};local copied=self.view.copied;local upload=self.uploads[1]
  if upload and copied and upload:copied(copied)then
   table.remove(self.uploads,1);if upload:chunk()then self.uploads[#self.uploads+1]=upload end
  end
  local chunk=self.view.chunk
  if type(chunk)=='table'then
   local body,state=self.receiver:accept(chunk)
   if state=='complete'or state=='replay'or state=='pending'then
    local copied_key=chunk.channel..':'..chunk.id..':'..chunk.offset
    if self.ack_sent~=copied_key then self.ack={kind='item_copied',id=chunk.id,channel=chunk.channel,offset=chunk.offset};self.ack_key=copied_key end
    local key=chunk.channel..':'..chunk.id
    if body and not self.complete[key]then
     local value=json.decode(body,{bytes=131072,nodes=32768,depth=16})
     if value then
      self.complete[key]=true;self.order[#self.order+1]={key=key,id=chunk.id,channel=chunk.channel};if #self.order>4 then local old=table.remove(self.order,1);self.complete[old.key]=nil;self.receiver:forget(old.id,old.channel)end
      if chunk.channel=='effect_item'and codec:resolve(value)then self.effects[chunk.id]=value
      elseif chunk.channel=='bank'and type(value.query)=='table'then
       local query_key=json.encode(value.query);local old=self.banks[query_key]
       if not old or not older(value.revision,old.revision)then
       for _,row in ipairs(value.items or {})do local revision=self.claimed[row.itemId];if revision then row.claimAllowed=false end end
       if not old then self.bank_keys[#self.bank_keys+1]=query_key;if #self.bank_keys>8 then self.banks[table.remove(self.bank_keys,1)]=nil end end
       self.banks[query_key]=value
       if not self.requested_query or self.requested_query==query_key then self.bank=value;self.bank_changed=true end
       for _,n in ipairs(value.notices or {})do notice((n.displayName or 'Player')..' deposited '..tostring(n.count or 1)..' '..tostring(n.kind or 'item'))end
       end
      elseif chunk.channel=='detail'then
       local id=value.summary and value.summary.itemId
       if id then
        self.details=self.details or {};self.detail_order=self.detail_order or {}
        if not self.details[id]then self.detail_order[#self.detail_order+1]=id;if #self.detail_order>8 then self.details[table.remove(self.detail_order,1)]=nil end end
        self.details[id]=value;if self.inspect_inflight==id then self.inspect_inflight=nil end
        if self.inspect_wanted==id or self.inspect_id==id then self.detail=value;self.inspect_wanted=nil end
       end
      elseif chunk.channel=='preview'then self.preview=value
      elseif chunk.channel=='players'then self.players=value;local found=false;for _,p in ipairs(value)do if self.preview_actor==p.actor then found=true end end;if not found then self.preview=nil;self.preview_actor=nil end end
     end
    end
   end
  end
  local submissions={};for _,value in ipairs(self.view.submissions or {})do submissions[#submissions+1]=value end;if self.view.submission then submissions[#submissions+1]=self.view.submission end
  for _,submitted in ipairs(submissions)do
  if submitted and (submitted.status=='accepted'or submitted.status=='refused'or submitted.status=='unresolved')then for _,id in pairs(self.claim_ids)do if id==submitted.id then self.status=nil end end end
  if submitted and submitted.status=='refused'then
   guard:refused(submitted.id)
   for item,id in pairs(self.claim_ids)do if id==submitted.id then self.claimed[item]=nil;self.claim_ids[item]=nil;self.query_needed=true end end
  end
  end
 end
 local function inventory(player)
  local wands={};local held='-1';local active=e.EntityGetFirstComponentIncludingDisabled(player,'Inventory2Component');active=active and e.ComponentGetValue2(active,'mActualActiveItem')
  for _,child in ipairs(e.EntityGetAllChildren(player)or {})do if e.EntityGetName(child)=='inventory_quick'then
   for _,entity in ipairs(e.EntityGetAllChildren(child)or {})do if e.EntityHasTag(entity,'wand')then
    local item=e.EntityGetFirstComponentIncludingDisabled(entity,'ItemComponent');local slot,y;if item then slot,y=e.ComponentGetValue2(item,'inventory_slot')end
    if type(slot)=='number'and slot>=0 and slot<8 and (y==nil or y==0)then
     local ok,descriptor=pcall(codec.capture,codec,api,e,entity);wands[#wands+1]=ok and {slot=slot,wand=descriptor.wand}or {slot=slot,unavailableReason='unsupported-content'};if active==entity then held=tostring(slot)end
    end
   end end
  end end
  table.sort(wands,function(a,b)return a.slot<b.slot end);return {wands=json.array(wands),heldSlot=held}
 end
 function self:pause()guard:observe_removals(nil,nil,false)end
 -- A short native output-copy lease gap does not revoke the independently
 -- validated Playing owner. Observe ONLY an already-issued removal; starting
 -- effects or publishing results still requires a freshly copied bank view.
 function self:observe(frame)
  guard:observe_removals(frame,self.view.effects or self.view.effect,eligible()~=nil and self.view.enabled==true)
 end
 function self:tick(frame,player)
  local view=self.view;local ready=eligible()~=nil and view.enabled==true
  self:observe(frame)
  -- Results and exact output-copy ACKs precede new queries/capture uploads.
  local sent=false
  if self.result and request(self.result)then self.reported[self.result.id]=self.result.digest..':'..self.result.attempt;self.result=nil;sent=true end
  if not sent and self.ack and request(self.ack)then self.ack_sent=self.ack_key;self.ack=nil;sent=true end
  if not ready then return end
  local ghosts=e.a3_ghosts and e.a3_ghosts()or {};self.held=self.held or {};local visible={}
  for _,p in ipairs(self.players)do local entity=ghosts[p.ghost];local sprite=catalogue.sprites[p.sprite]
   if entity and sprite and e.EntityGetIsAlive(entity)then visible[entity]=true;local old=self.held[entity]
    if not old or old.id~=p.sprite then
     if old then pcall(e.EntityRemoveComponent,entity,old.component)end
     local component=e.EntityAddComponent2(entity,'SpriteComponent',{_tags='nt2_item_held',image_file=sprite.path,offset_x=-5,offset_y=0,z_index=0.5,alpha=0.8})
     if component then self.held[entity]={id=p.sprite,component=component}end
    end
   end
  end
  for entity,held in pairs(self.held)do if not visible[entity]then if e.EntityGetIsAlive(entity)then pcall(e.EntityRemoveComponent,entity,held.component)end;self.held[entity]=nil end end
  local ops=view.effects or (view.effect and {view.effect}or {});local near,x,y=outlet()
  -- All issued permits remain observable while one blocked source yields to
  -- siblings. At most four are published; one new handshake/result per frame.
  for offset=1,math.min(#ops,4)do local index=((self.effect_cursor or 0)+offset-1)%#ops+1;local op=ops[index]
   if self.effects[op.id]then
    if op.stage=='offer'and not sent and (op.direction~='claim'or near)then
     local attempt=identity();attempt=attempt and attempt:gsub('-','');local r=attempt and guard:offer(op,self.effects[op.id],attempt,near)
     if r then sent=request({kind='item_prepare',id=r.id,digest=r.digest,attempt=r.attempt});if sent then self.effect_cursor=index;break end end
    elseif op.stage=='permit'and not self.result and self.reported[op.id]~=op.digest..':'..tostring(op.attempt)then
     local r=guard:permit(op,near,x,y,ready,frame)
     if r then self.result={kind='item_result',id=r.id,digest=r.digest,attempt=r.attempt,applied=r.state=='applied_session'};self.effect_cursor=index;break end
    end
   end
  end
  if not self.next_scan or frame>=self.next_scan then
   self.next_scan=frame+30 -- First eligible callback after due; no missed modulo/catch-up.
   -- At most four live originals, including copies already accepted by the app.
   -- Round-robin inlets; no frame catch-up and no unbounded HTTP fan-out.
   local inlets=e.EntityGetWithTag('nt2_item_inlet')or {};local empty=0;local attempts=0
   while #inlets>0 and #self.uploads<4 and guard:active_count()<4 and empty<#inlets and attempts<4 do
    attempts=attempts+1
    self.inlet_cursor=(self.inlet_cursor or 0)%#inlets+1
    local r=guard:capture(inlets[self.inlet_cursor])
    if r then
     if self:upload(r.id,'deposit',r.body)then empty=0 else guard:refused(r.id);empty=empty+1 end
    else empty=empty+1 end
   end
   local ok,value=pcall(inventory,player);local body=ok and json.encode(value)
   if ok and not body then for _,slot in ipairs(value.wands)do slot.wand=nil;slot.unavailableReason='unsupported-content'end;body=json.encode(value);self.status='Wand inventory exceeds the supported snapshot size.'end
   if body and body~=self.inventory_body and self.inventory_revision<4294967295 then
    value.revision=self.inventory_revision+1;local full=json.encode(value);if full and self:upload('inventory-'..value.revision,'inventory',full)then self.inventory_revision=value.revision;self.inventory_body=body end
   end
  end
  local upload=self.uploads[1];if upload and not sent then sent=request(upload:chunk())end
  if self.inspect_inflight and frame-(self.inspect_since or frame)>=300 then self.inspect_inflight=nil end
  if not sent and not self.result and not self.inspect_inflight and self.inspect_wanted and (not self.inspect_due or frame>=self.inspect_due)then
   local id=self.inspect_wanted
   if request({kind='item_inspect',item_id=id})then self.inspect_inflight=id;self.inspect_since=frame;self.inspect_due=frame+30 end
  end
 end
 return self
end
return M
