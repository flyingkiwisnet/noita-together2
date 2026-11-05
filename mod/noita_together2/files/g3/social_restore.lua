-- Saved presentation leases only. The closed field list cannot heal, enable
-- controls or grant play authority. Component tags survive ID remapping.
local M={};local prefix='nt2_a3_restore_';local serial=0
local fields={SpriteComponent={visible=true,rect_animation=true,image_file=true},CharacterPlatformingComponent={run_velocity=true,mouse_look=true},VerletPhysicsComponent={cloth_color=true,cloth_color_edge=true}}
local function read(e,key,...)local f=e[key];if type(f)~='function'then return nil end;local ok,a=pcall(f,...);if ok then return a end end
local function member(e,id,c,kind)if read(e,'EntityGetIsAlive',id)~=true then return false end;for _,v in ipairs(read(e,'EntityGetComponentIncludingDisabled',id,kind)or {})do if v==c then return true end end;return false end
local function encode(v)
 if type(v)=='boolean'then return v and 'B1'or 'B0'end
 if type(v)=='number'and v==v and math.abs(v)<1e30 then return 'N'..string.format('%.17g',v)end
 if type(v)=='string'and #v<=512 then return 'S'..v:gsub('.',function(c)return string.format('%02x',c:byte())end)end
end
local function decode(v)
 if v=='B1'then return true elseif v=='B0'then return false end
 if v:sub(1,1)=='N'then local n=tonumber(v:sub(2));if n and n==n and math.abs(n)<1e30 then return n end
 elseif v:sub(1,1)=='S'then local hex=v:sub(2);if #hex<=1024 and #hex%2==0 and not hex:find('[^0-9a-f]')then return hex:gsub('%x%x',function(s)return string.char(tonumber(s,16))end)end end
end
local function payload(kind,key,original,ours)local a,b=encode(original),encode(ours);if fields[kind]and fields[kind][key]and a and b then return '1|'..kind..'|'..key..'|'..a..'|'..b end end
function M.supported(e)return type(e.ComponentAddTag)=='function'and type(e.ComponentRemoveTag)=='function'and type(e.ComponentHasTag)=='function'end
function M.record(e,s)
 if not M.supported(e)then return false end;local value=payload(s.kind,s.key,s.original,s.ours);if not value then return false end
 local count=0;for _,c in ipairs(read(e,'EntityGetComponentIncludingDisabled',s.entity,'VariableStorageComponent')or {})do local name=read(e,'ComponentGetValue2',c,'name');if type(name)=='string'and name:sub(1,#prefix)==prefix then count=count+1 end end;if count>=32 then return false end
 serial=serial+1;s.tag=prefix..tostring(s):gsub('[^%w]','')..'_'..serial
 s.marker=read(e,'EntityAddComponent2',s.entity,'VariableStorageComponent',{name=s.tag,value_string=value});if not s.marker or s.marker==0 then return false end
 read(e,'ComponentAddTag',s.component,s.tag);if read(e,'ComponentHasTag',s.component,s.tag)~=true then read(e,'EntityRemoveComponent',s.entity,s.marker);s.marker=nil;return false end
 read(e,'EntityAddTag',s.entity,'nt2_a3_restore_fields');return true
end
function M.update(e,s,value)
 if not member(e,s.entity,s.marker,'VariableStorageComponent')or read(e,'ComponentGetValue2',s.marker,'name')~=s.tag or read(e,'ComponentHasTag',s.component,s.tag)~=true then return false end
 local data=payload(s.kind,s.key,s.original,value);if not data then return false end;read(e,'ComponentSetValue2',s.marker,'value_string',data);return true
end
function M.finish(e,s)
 if member(e,s.entity,s.component,s.kind)and read(e,'ComponentHasTag',s.component,s.tag)==true then read(e,'ComponentRemoveTag',s.component,s.tag)end
 if member(e,s.entity,s.marker,'VariableStorageComponent')and read(e,'ComponentGetValue2',s.marker,'name')==s.tag then read(e,'EntityRemoveComponent',s.entity,s.marker)end
end
function M.recover(e,player)
 if not M.supported(e)or read(e,'EntityGetIsAlive',player)~=true then return false end
 local ids,seen,holders={}, {}, {};local function visit(id,depth)
  if seen[id]or #ids>=256 or depth>4 or read(e,'EntityGetIsAlive',id)~=true then return end;seen[id]=true;ids[#ids+1]=id
  if read(e,'EntityHasTag',id,'nt2_a3_local_holder')==true and read(e,'EntityGetParent',id)==player then holders[#holders+1]=id end
  for _,child in ipairs(read(e,'EntityGetAllChildren',id)or {})do if #ids>=256 then break end;visit(child,depth+1)end
 end;visit(player,0)
 for _,id in ipairs(read(e,'EntityGetWithTag','nt2_a3_restore_fields')or {})do if #ids>=256 then break end;if read(e,'EntityGetRootEntity',id)==player then visit(id,0)end end
 local count=0;for _,id in ipairs(ids)do
  local markers={};for _,v in ipairs(read(e,'EntityGetComponentIncludingDisabled',id,'VariableStorageComponent')or {})do markers[#markers+1]=v end
  for index,marker in ipairs(markers)do if index>128 then break end
   local tag=read(e,'ComponentGetValue2',marker,'name');local data=read(e,'ComponentGetValue2',marker,'value_string')
   if type(tag)=='string'and tag:match('^nt2_a3_restore_[%w_]+$')and type(data)=='string'and #data<=2200 and count<128 then
    local kind,key,a,b=data:match('^1|(%w+)|([%w_]+)|([^|]*)|([^|]*)$');local original,ours=a and decode(a),b and decode(b)
    if kind and fields[kind]and fields[kind][key]and original~=nil and ours~=nil and type(original)==type(ours)then
     local matches={};for _,c in ipairs(read(e,'EntityGetComponentIncludingDisabled',id,kind)or {})do if read(e,'ComponentHasTag',c,tag)==true then matches[#matches+1]=c end end
     if #matches==1 then local c=matches[1];if read(e,'ComponentGetValue2',c,key)==ours then read(e,'ComponentSetValue2',c,key,original)end;read(e,'ComponentRemoveTag',c,tag);read(e,'EntityRemoveComponent',id,marker);count=count+1 end
    end
   end
  end
 end
 for _,holder in ipairs(holders)do
  for _,child in ipairs(read(e,'EntityGetAllChildren',holder)or {})do if read(e,'EntityGetName',child)=='cape'and read(e,'EntityGetParent',child)==holder then read(e,'EntityRemoveFromParent',child);read(e,'EntityAddChild',player,child)end end
  read(e,'EntityKill',holder)
 end
 return true
end
return M
