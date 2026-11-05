-- Bounded observational source extension. No globals/save writes, seed command,
-- mode inference or authority. Callback order is evidence, not a classifier.
local M={}
function M.new()
 local count,updates,emitted,dropped,print_errors=0,0,0,0,0
 local buffer={}
 local function value(fn,fallback)
  local ok,v=pcall(fn);if not ok then return fallback end
  if type(v)=='number' and v==v and v~=math.huge and v~=-math.huge then return tostring(v):sub(1,80) end
  if type(v)=='string' and #v<=20 and v:match('^%d+$') then return v end
  return fallback -- Never copy arbitrary strings/errors into candidate diagnostics.
 end
 local function size(key) return value(function() return #GlobalsGetValue(key,'') end,'?') end
 local observer={}
 function observer:record(event,native,ffi)
  if count>=64 then dropped=dropped+1;return end;count=count+1
  local fields={'NT2_STARTUP_ENTRY|v=2','seq='..count,'event='..event,'scope_origin_clock=unavailable','frame='..value(GameGetFrameNum,'?'),'clock='..value(GameGetRealWorldTimeSinceStarted,'?'),'world='..value(GameGetWorldStateEntity,'?'),'actual_seed='..value(function() return StatsGetValue('world_seed') end,'?'),'saved_marker_bytes='..size('nt2_rejoin_local_world_v1'),'loot_bytes='..size('nt2_seed_loot_policy_v1')}
  if native and ffi then local ok=pcall(function() local d=ffi.new('uint8_t[256]');if native.nt2_g3_startup_diagnostic(d,256)==256 then local w=ffi.cast('uint32_t*',d);fields[#fields+1]='original_result='..w[2];fields[#fields+1]='reason='..w[3];fields[#fields+1]='loader_ms='..w[36];fields[#fields+1]='total_ms='..w[37];fields[#fields+1]='claim_ms='..(w[39]==1 and tostring(w[6]) or 'unknown');local function hex(offset,n) local t={} for i=offset,offset+n-1 do t[#t+1]=string.format('%02x',tonumber(d[i])) end return table.concat(t) end;fields[#fields+1]='world_id='..hex(64,16);fields[#fields+1]='bundle_digest='..hex(80,32);fields[#fields+1]='config_digest='..hex(112,32) end end);if not ok then fields[#fields+1]='native_record=unavailable' end end
  buffer[#buffer+1]=table.concat(fields,'|')
 end
 function observer:replay()
  if emitted+#buffer+1>2048 then return end
  for _,line in ipairs(buffer) do emitted=emitted+1;if not pcall(print,line) then print_errors=print_errors+1 end end
  emitted=emitted+1;pcall(print,'NT2_STARTUP_ENTRY_COVERAGE|v=2|last='..count..'|dropped='..dropped..'|print_errors='..print_errors..'|emitted='..emitted)
 end
 function observer:update(native,ffi) updates=updates+1;if updates==1 or updates==60 or updates==650 then self:record('preupdate'..updates,native,ffi);self:replay() end end
 return observer
end
return M
