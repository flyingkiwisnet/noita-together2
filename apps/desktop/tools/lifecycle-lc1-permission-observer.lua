-- Fixture observation only. An owned typing lock changes Controls.enabled,
-- without revoking native permission or adding any protection effects.
local M={}
function M.new()
 local self={previous=nil,holds=0,input_locks=0}
 function self:sample(value,played)
  local typing=not value.controls and value.lease_owned==true and value.protected~=true
   and value.native_status==2 and value.native_status_frame==value.frame
  local permitted=value.protected~=true and (value.controls==true or typing)
  local previous=self.previous
  local lost=played==true and previous~=nil and previous.permitted and not permitted
  local input_started=typing and (not previous or not previous.typing)
  if lost then self.holds=self.holds+1 end
  if input_started then self.input_locks=self.input_locks+1 end
  local changed=not previous or previous.permitted~=permitted or previous.typing~=typing
  self.previous={permitted=permitted,typing=typing}
  return {permitted=permitted,typing=typing,lost=lost,changed=changed,holds=self.holds,input_locks=self.input_locks}
 end
 return self
end
return M
