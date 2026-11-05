-- Local preferences, draft, follow and dedup; none of these confer authority.
local M={}
function M.new(text,notice)
 local self={run=nil,peers={},hud={},hud_versions={},follow={},show_players=false,show_chat=true,show_ghosts=true,opacity=1,draft='',composer=false,pending=nil,feedback='',chat_seen={},chat_order={}}
 function self:roster(run,peers)
  if run~=self.run then self.follow={};self.chat_seen={};self.chat_order={};self.hud={};self.hud_versions={};self.pending=nil;self.draft='';self.composer=false;self.feedback='';self.run=run end
  local next={};for _,peer in ipairs(peers)do if peer.status==3 or peer.status=='disconnected'then self.hud[peer.actor]=nil end;next[peer.actor]=peer end
  for actor in pairs(self.peers)do if not next[actor] then self.follow[actor]=nil;self.hud[actor]=nil;self.hud_versions[actor]=nil end end;self.peers=next
 end
 function self:rows()
  local rows={};for actor,peer in pairs(self.peers)do rows[#rows+1]={actor=actor,name=peer.name,status=peer.status,hud=self.hud[actor]} end
  table.sort(rows,function(a,b)local x,y=a.name:lower(),b.name:lower();return x==y and a.actor<b.actor or x<y end);return rows
 end
 function self:toggle_follow(actor)
  if not self.peers[actor] then return false end
  self.follow[actor]=not self.follow[actor] or nil
  notice((self.follow[actor] and 'Following ' or 'Stopped following ')..self.peers[actor].name);return true
 end
 function self:chat(id,name,message)
  if self.chat_seen[id] then return false end
  if self.show_chat and notice(name..': '..message,'chat')==false then return false end
  self.chat_seen[id]=true;self.chat_order[#self.chat_order+1]=id
  if #self.chat_order>256 then self.chat_seen[table.remove(self.chat_order,1)]=nil end
  return true
 end
 function self:edit(value)
  if self.pending then return false end
  local n=text.length(value);if not n then self.feedback='Maximum 99 characters / 396 UTF-8 bytes; no control characters';return false end
  if self.draft~=value then self.draft=value;self.feedback=''end;return true
 end
 function self:cancel() if self.pending then return false end;self.draft='';self.composer=false;self.feedback='';return true end
 function self:submit(send,x,y,playing)
  if not playing or self.pending then return false end
  if not text.message(self.draft) then self.feedback='Enter a message (maximum 99 characters)';return false end
  local id,reason=send(self.draft,x,y-10)
  if not id then self.feedback=reason or 'Message unavailable';return false end
  self.pending=id;self.feedback='Sending message…';return true
 end
 function self:outcome(id,status,reason)
  if id~=self.pending then return false end
  if status=='accepted' then self.pending=nil;self.draft='';self.composer=false;self.feedback='Message placed';notice('Message placed')
  elseif status=='refused' then self.pending=nil;self.feedback=reason or 'Message refused'
  else self.feedback='Waiting for the original message result…' end
  return true
 end
 return self
end
function M.radar(x,y,px,py,width,height)
 if math.abs(x-px)<=250 and math.abs(y-py)<=150 then return nil end
 local dx,dy=x-px,y-py;local distance=math.sqrt(dx*dx+dy*dy);if distance==0 then return nil end
 return math.max(30,math.min(width-30,width/2-30+dx/distance*300)),math.max(15,math.min(height-30,height/2+dy/distance*170)),math.floor(distance/10)
end
return M
