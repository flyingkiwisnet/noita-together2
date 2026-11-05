local json=dofile('mods/noita_together2/files/g3/bank_json.lua')
local M={}
function M.read(e,key)
 local raw=e.GlobalsGetValue('nt2_race_'..key,'');if raw==''then return nil end
 local value=json.decode_saved(raw,{bytes=131072,nodes=16384});return value,value==nil
end
function M.write(e,key,value)e.GlobalsSetValue('nt2_race_'..key,assert(json.encode_saved(value,{bytes=131072,nodes=16384})))end
function M.active(e)return e.GlobalsGetValue('nt2_race_active','0')=='1'end
function M.drop(e)
 local n=tonumber(e.GlobalsGetValue('nt2_race_dropped','0'))or 0;e.GlobalsSetValue('nt2_race_dropped',tostring(math.min(2147483647,n+1)))
end
function M.marker(e,entity,name,value)
 for _,c in ipairs(e.EntityGetComponentIncludingDisabled(entity,'VariableStorageComponent')or{})do if e.ComponentGetValue2(c,'name')==name then return false end end
 local c=e.EntityAddComponent2(entity,'VariableStorageComponent',{name=name,value_string=value});return c and c~=0
end
return M
