-- Shared component snapshot bookkeeping seam. The engine/third-party lock adapter
-- MUST supply may_restore; direct Noita component integration remains unapproved.
local M={}
local kinds={controls=true,inventory=true,wand=true,cape=true,hat=true,amulet=true,gem=true}
function M.new(api,validate)
 assert(type(api.may_restore)=='function')
 local self={next=0,tokens={},resources={}}
 function self:acquire(owner,descriptors)
  if not validate.id(owner) or not validate.array(descriptors,7) or #descriptors==0 or self.next>=4294967295 then return nil end
  local count=0;for _ in pairs(self.tokens) do count=count+1 end;if count>=64 then return nil end
  local keys={};local prepared={}
  for _,d in ipairs(descriptors) do
   if type(d)~='table' or not kinds[d.kind] or not validate.integer(d.entity,1,2147483647) or not validate.integer(d.component,1,2147483647) or type(d.blocked)~='boolean' or not api.alive(d) then return nil end
   local key=d.entity..':'..d.component..':'..d.kind;if keys[key] then return nil end;keys[key]=true
   local current=self.resources[key]
   if current and current.blocked~=d.blocked then return nil end
   if not current then local snapshot={entity=d.entity,component=d.component,kind=d.kind,blocked=d.blocked};local original=api.read(snapshot);if type(original)~='boolean' then return nil end;current={descriptor=snapshot,original=original,blocked=d.blocked,owners={},pending=false} end
   prepared[#prepared+1]={key=key,resource=current}
  end
  self.next=self.next+1;local token=self.next;self.tokens[token]={owner=owner,keys={}}
  for _,entry in ipairs(prepared) do
   local r=entry.resource;self.resources[entry.key]=r;r.owners[token]=true;self.tokens[token].keys[#self.tokens[token].keys+1]=entry.key
   if api.read(r.descriptor)~=r.blocked then local ok,value=pcall(api.write,r.descriptor,r.blocked);if not ok or value~=true then return token,false end end
  end
  return token,true
 end
 function self:release(token)
  local lease=self.tokens[token];if not lease then return false end
  local complete=true
  for _,key in ipairs(lease.keys) do
   local r=self.resources[key]
   if r then
    r.owners[token]=nil
    if next(r.owners)==nil then
     if not api.alive(r.descriptor) or api.read(r.descriptor)~=r.blocked then self.resources[key]=nil
     elseif api.may_restore(r.descriptor,r.original,lease.owner) then
      local ok,value=pcall(api.write,r.descriptor,r.original);if ok and value==true then self.resources[key]=nil else r.pending=true;complete=false end
     else r.pending=true;complete=false end
    end
   end
  end
  if complete then self.tokens[token]=nil end;return complete
 end
 return self
end
return M
