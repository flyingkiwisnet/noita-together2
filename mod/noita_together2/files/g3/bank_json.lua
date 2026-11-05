-- Bounded local mailbox JSON only; never executes received text.
local M={}
local arrays=setmetatable({},{__mode='k'})
M.null={}
function M.array(t)arrays[t]=true;return t end
function M.decode(s,limits)
 limits=limits or {};if type(s)~='string'or #s>(limits.bytes or 8192)then return nil end
 local i,nodes=1,0
 local function ws()local _,j=s:find('^[ \r\n\t]*',i);i=(j or i-1)+1 end
 local function string_value()
  assert(s:sub(i,i)=='"');i=i+1;local out={}
  while i<=#s do local c=s:sub(i,i);i=i+1;if c=='"'then return table.concat(out)end
   if c=='\\'then local q=s:sub(i,i);i=i+1;local escapes={['"']='"',['\\']='\\',['/']='/',b='\b',f='\f',n='\n',r='\r',t='\t'}
    if q=='u'then local h=s:sub(i,i+3);assert(h:match('^%x%x%x%x$'));i=i+4;local v=tonumber(h,16);assert(v<0xd800 or v>0xdfff);c=v<128 and string.char(v)or v<2048 and string.char(192+math.floor(v/64),128+v%64)or string.char(224+math.floor(v/4096),128+math.floor(v/64)%64,128+v%64)
    else c=assert(escapes[q])end
   else assert(c:byte()>=32)end;out[#out+1]=c
  end;error('unterminated string')
 end
 local value
 value=function(depth)
  nodes=nodes+1;assert(depth<=(limits.depth or 12)and nodes<=(limits.nodes or 512));ws();local c=s:sub(i,i)
  if c=='"'then return string_value()end
  if c=='{'or c=='['then local object=c=='{';local result,seen={},{};if not object then M.array(result)end;i=i+1;ws();local closing=object and '}'or ']';if s:sub(i,i)==closing then i=i+1;return result end
   repeat local key;if object then key=string_value();assert(not seen[key]);seen[key]=true;ws();assert(s:sub(i,i)==':');i=i+1 end
    local v=value(depth+1);if object then if v~=M.null then result[key]=v end else result[#result+1]=v end;ws();c=s:sub(i,i);i=i+1;if c==closing then return result end;assert(c==',');ws()
   until false
  end
  for _,word in ipairs({'true','false','null'})do if s:sub(i,i+#word-1)==word then i=i+#word;if word=='true'then return true elseif word=='false'then return false else return M.null end end end
  local raw=s:match('^-?%d+',i);assert(raw);i=i+#raw;local v=tonumber(raw);assert(v and math.abs(v)<=9007199254740991);return v
 end
 local ok,result=pcall(function()local v=value(0);ws();assert(i>#s);return v end);if ok then return result end
end
function M.encode(value,limits)
 limits=limits or {};local nodes,bytes,stack=0,0,{}
 local function quote(s)
  return '"'..s:gsub('[%z\1-\31\\"]',function(c)local escapes={['"']='\\"',['\\']='\\\\',['\n']='\\n',['\r']='\\r',['\t']='\\t'};return escapes[c]or string.format('\\u%04x',c:byte())end)..'"'
 end
 local function encode(v,depth,key)
  nodes=nodes+1;assert(nodes<=(limits.nodes or 16384)and depth<=(limits.depth or 16));local result
  if v==M.null or v==nil then result='null'
  elseif type(v)=='string'then result=quote(v)
  elseif type(v)=='boolean'then result=tostring(v)
  elseif type(v)=='number'then assert(v==v and v%1==0 and math.abs(v)<=9007199254740991);result=string.format('%.0f',v)
  else
   assert(type(v)=='table'and not getmetatable(v)and not stack[v]);stack[v]=true
   local array=arrays[v]or key=='deck'or key=='permanent'or key=='contents'or key=='slots'or key=='spellTypes'or key=='wandSort'
   local keys,count={},0;for k in pairs(v)do count=count+1;keys[#keys+1]=k;if type(k)=='number'then array=true end end
   local parts={}
   if array then for _,k in ipairs(keys)do assert(type(k)=='number'and k%1==0 and k>=1 and k<=count)end;for i=1,count do assert(v[i]~=nil);parts[i]=encode(v[i],depth+1)end;result='['..table.concat(parts,',')..']'
   else for _,k in ipairs(keys)do assert(type(k)=='string')end;table.sort(keys);for _,k in ipairs(keys)do parts[#parts+1]=quote(k)..':'..encode(v[k],depth+1,k)end;result='{'..table.concat(parts,',')..'}'end
   stack[v]=nil
  end
  bytes=bytes+#result;assert(bytes<=(limits.bytes or 131072)*17);return result
 end
 local ok,result=pcall(encode,value,0);if ok and #result<=(limits.bytes or 131072)then return result end
end
-- Structured Globals are persisted as raw XML attributes by Noita. Reuse the
-- saved-world codec, with the existing JSON byte bounds, instead of raw JSON.
-- Native/mailbox transport continues to use encode/decode above.
local saved_codec
local function codec()
 if not saved_codec then saved_codec=dofile('mods/noita_together2/files/rejoin/local_world.lua')end
 return saved_codec
end
function M.encode_saved(value,limits)
 local raw=M.encode(value,limits);if not raw then return nil end
 return codec().encode_saved(raw,limits and limits.bytes or 131072)
end
function M.decode_saved(value,limits)
 local raw=codec().decode_saved(value,limits and limits.bytes or 8192)
 return raw and M.decode(raw,limits)or nil
end
return M
