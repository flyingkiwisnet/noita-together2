-- Candidate process-local engine append registry. NEVER execute this registry.
-- Availability/reset lifetime in settings is an explicit engine acceptance item.
-- No ModSetting/Globals/filesystem persistence, credentials or arbitrary paths.
local M={};local root='mods/noita_together2/files/startup/'
local registry=root..'scope_registry.lua'
local tags={new=root..'scope_new.lua',restart=root..'scope_restart.lua',spent=root..'scope_spent.lua',invalid=root..'scope_invalid.lua'}
local traces={new=tags.new,restart=tags.restart,unknown=tags.invalid,menu=root..'scope_menu.lua',runtime=root..'scope_runtime.lua'}
local function trace(kind)
 if type(ModLuaFileGetAppends)~='function' or type(ModLuaFileAppend)~='function' then return end
 local ok,list=pcall(ModLuaFileGetAppends,root..'scope_trace.lua')
 if ok and type(list)=='table' and #list<16 then pcall(ModLuaFileAppend,root..'scope_trace.lua',traces[kind] or traces.unknown) end
end
function M.observations()
 local output={};if type(ModLuaFileGetAppends)~='function' then return output end
 local ok,list=pcall(ModLuaFileGetAppends,root..'scope_trace.lua');if not ok or type(list)~='table' then return output end
 for i=1,math.min(#list,16) do for kind,path in pairs(traces) do if list[i]==path then output[#output+1]='settings_'..kind end end end
 return output
end
local function read()
 if type(ModLuaFileGetAppends)~='function' then return nil end
 local ok,list=pcall(ModLuaFileGetAppends,registry)
 if not ok or type(list)~='table' or #list>4 then return nil end
 for k,v in pairs(list) do if type(k)~='number' or k<1 or k>#list or k~=math.floor(k) or (v~=tags.new and v~=tags.restart and v~=tags.spent and v~=tags.invalid) then return nil end end
 return list
end
local function append(tag)
 if type(ModLuaFileAppend)~='function' then return false end
 return pcall(ModLuaFileAppend,registry,tag)
end
local function signature(list) return list and table.concat(list,'|') or 'unavailable' end
function M.snapshot() return signature(read()) end
function M.produce(kind)
 trace(kind)
 local list=read();if not list then return 'unavailable' end
 if kind=='menu' or kind=='runtime' then
  -- Noise cannot leave a pending fresh event usable after an unrelated update.
  if #list==1 and list[1]==tags.new then append(tags.invalid);return 'invalidated' end
  return 'ignored'
 end
 if kind~='new' and kind~='restart' then
  if #list<4 then append(tags.invalid) end;return 'unknown'
 end
 if #list~=0 then
  if #list<4 and list[#list]~=tags.invalid then append(tags.invalid) end
  return 'ambiguous' -- includes duplicate NEW_GAME or stale/consumed entry
 end
 if not append(tags[kind]) then return 'unavailable' end
 local after=read();return after and #after==1 and after[1]==tags[kind] and 'recorded' or 'unavailable'
end
function M.consume()
 local list=read()
 if not list then return 'unavailable',M.snapshot() end
 if #list==0 then return 'missing',M.snapshot() end
 if #list==2 and list[2]==tags.spent then return 'spent',M.snapshot() end
 if #list~=1 or (list[1]~=tags.new and list[1]~=tags.restart) then return 'ambiguous',M.snapshot() end
 if not append(tags.spent) then return 'unavailable',M.snapshot() end
 local after=read();if not after or #after~=2 or after[1]~=list[1] or after[2]~=tags.spent then return 'ambiguous',M.snapshot() end
 return list[1]==tags.new and 'fresh' or 'restored',signature(after)
end
return M
