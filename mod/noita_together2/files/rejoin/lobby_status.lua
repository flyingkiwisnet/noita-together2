-- Current app membership presentation only. No saved state and no play decision.
local M={}
function M.read(bridge,ffi)
 if not bridge or not ffi or not bridge.nt2_g3_lobby_status then return nil,bridge and'unavailable'or'no_app'end
 local ok,value,contact=pcall(function()
  local out=ffi.new('uint8_t[256]')
  if bridge.nt2_g3_lobby_status(out,256)~=256 or ffi.string(out,4)~='NLD1' then return nil end
  local words=ffi.cast('uint32_t*',out)
  local code,size=tonumber(words[2]),tonumber(words[8])
  if tonumber(words[1])==1 and code==0 then return nil,'connected'end
  if tonumber(words[1])~=1 or code<1 or code>5 or size<1 or size>192 then return nil end
  local name=ffi.string(out+64,size)
  if name:find('[%z\1-\31\127]') then return nil end
  return {code=code,name=name},'connected'
 end)
 return ok and value or nil,ok and(contact or'unavailable')or'unavailable'
end
function M.compatibility(value,race_result)
 if race_result then return nil end -- confirmed matching terminal outcome takes presentation priority
 if type(value)=='table'and value.code==5 and type(value.name)=='string'and#value.name>0 and#value.name<=192 and not value.name:find('[%z\1-\31\127]')then return {title='NT2 versions do not match',detail=value.name}end
end
function M.message(value,contact)
 if M.compatibility(value)then return value.name end
 if not value and contact=='no_app'then return 'No NT2 app connection. Open the NT2 app.'end
 if not value and contact=='unavailable'then return 'NT2 connection unavailable or not validated. Check connection and matching app/mod in the NT2 app.'end
 if type(value)~='table' or type(value.name)~='string' or #value.name==0 or #value.name>192 or value.name:find('[%z\1-\31\127]') then return 'No lobby connection. Open the NT2 app and join a lobby.' end
 local code=value.code
 if code==3 then return 'Reconnecting to lobby '..value.name..'; waiting for connection.' end
 local suffix=code==1 and 'checking seed and mods.' or code==2 and 'waiting for host Start.' or code==4 and 'host Start confirmed; checking world recovery.'
 if not suffix then return M.message(nil) end
 return 'Connected to lobby '..value.name..'; '..suffix
end
-- Volatile presentation only; never grants play or chooses offline mode.
function M.race_status()
 local self={}
 local function key(record,seed)
  if type(record)~='string'or type(seed)~='string'then return nil end
  local world=record:match('"local_world"%s*:%s*"([%x%-]+)"');local actual=record:match('"actual_seed"%s*:%s*"(%d+)"')
  if not world or #world~=36 or actual~=seed then return nil end
  return record,world
 end
 function self:update(view,record,seed,lobby)
  local binding,world=key(record,seed)
  if binding~=self.binding then self.binding=binding;self.result=nil;self.scope=nil;self.retired=nil;self.blocked=nil end
  if not binding then return end
  -- A new active lobby after terminal projection retirement cannot inherit it.
  if lobby and self.result and(self.retired and lobby.code~=3 or lobby.code==1 or lobby.code==2)then self.result=nil;self.blocked=true end
  if self.blocked or type(view)~='table'then return end
  local snapshot=view.snapshot
  if view.localWorld~=world or type(view.world)~='string'or type(snapshot)~='table'or type(snapshot.runKey)~='string'then self.result=nil;return end
  local scope=view.world..':'..snapshot.runKey
  if scope~=self.scope then self.result=nil;self.retired=nil;self.scope=scope end
  if snapshot.status~='ended'or snapshot.endReason~='race_deadline'or snapshot.collectionState~='available'then self.result=nil;return end
  local result={title='Race time limit reached',detail='',footer='Race ended. View results in the NT2 app.'}
  local own=view.deadlineResult
  if type(own)=='table'then
   local count,total,rank=own.completedGoals,own.goalTotal,own.rank
   if type(count)=='number'and count%1==0 and type(total)=='number'and total%1==0 and count>=0 and count<=total and total<=16 then result.detail='Goals completed: '..count..' / '..total end
   if type(rank)=='number'and rank%1==0 and rank>0 and rank<=32 then result.detail=result.detail..(result.detail~=''and' - 'or'')..'Final place: #'..rank end
  end
  self.result=result
 end
 function self:message(record,seed,phase,reason,lobby)
  local binding=key(record,seed)
  if not binding or binding~=self.binding or phase~='terminal'or lobby or(reason and reason~='Rejoin is unavailable for this participation')then return nil end
  if self.result then self.retired=true end
  return self.result
 end
 return self
end
return M
