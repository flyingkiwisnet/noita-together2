-- Bounded XML-safe C2 local ownership data. No native, disk or network calls.
local json=dofile('mods/noita_together2/files/g3/bank_json.lua')
local M={}
function M.read(e,key)local raw=e.GlobalsGetValue('nt2_c2_'..key,'');if raw==''then return nil end;local value=json.decode_saved(raw,{bytes=131072,nodes=16384});return value,value==nil and 'invalid_saved_encoding'or nil end
function M.write(e,key,value)e.GlobalsSetValue('nt2_c2_'..key,assert(json.encode_saved(value,{bytes=131072,nodes=16384})))end
function M.component(e,entity,name)
 for _,c in ipairs(e.EntityGetComponentIncludingDisabled(entity,'VariableStorageComponent')or{})do if e.ComponentGetValue2(c,'name')==name then return json.decode_saved(e.ComponentGetValue2(c,'value_string')),c end end
end
function M.mark(e,entity,name,value)
 local _,c=M.component(e,entity,name);local encoded=assert(json.encode_saved(value))
 if c then e.ComponentSetValue2(c,'value_string',encoded)else c=e.EntityAddComponent2(entity,'VariableStorageComponent',{name=name,value_string=encoded})end
 return c
end
function M.progression(e)
 if type(e.SessionNumbersGetValue)~='function'then return nil end
 local ok,value=pcall(e.SessionNumbersGetValue,'NEW_GAME_PLUS_COUNT');if not ok then return nil end
 if type(value)=='number'then if value%1==0 and value>=0 and value<=4294967295 then return value end;return nil end
 if type(value)~='string'or not value:match('^%d+$')then return nil end
 local n=tonumber(value);return n and n<=4294967295 and tostring(n)==value and n or nil
end
return M
