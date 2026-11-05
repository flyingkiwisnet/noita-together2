-- Engine-test boundary: Noita writes Globals attribute bytes verbatim. Never
-- escape values on the engine's behalf or skip malformed rows during restore.
local M={}
function M.serialize(globals)
 local keys={};for key in pairs(globals)do keys[#keys+1]=key end;table.sort(keys)
 local rows={'<WorldState><Globals>'}
 for _,key in ipairs(keys)do rows[#rows+1]='<E key="'..key..'" value="'..globals[key]..'"/>'end
 rows[#rows+1]='</Globals></WorldState>';return table.concat(rows)
end
function M.restore(xml)
 local body=xml:match('^<WorldState><Globals>(.*)</Globals></WorldState>$');if not body then return nil end
 local result={};local offset=1
 while offset<=#body do
  local row=body:sub(offset);local key,value,after=row:match('^<E key="([^"&<>%z\1-\31]+)" value="([^"&<>%z\1-\31]*)"/>()')
  if not after or result[key]~=nil then return nil end
  result[key]=value;offset=offset+after-1
 end
 return result
end
return M
