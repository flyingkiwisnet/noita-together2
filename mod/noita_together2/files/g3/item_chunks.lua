-- Bounded payload copying on the existing optional bank mailbox. Chunk data is
-- never executable text, item authority or persistence evidence.
local M={MAX_BYTES=131072,CHUNK_BYTES=2304,MAX_INFLIGHT=4}
local alphabet='ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/'
local indexes={};for i=1,#alphabet do indexes[alphabet:sub(i,i)]=i-1 end
function M.base64(s)
 local out={}
 for i=1,#s,3 do local a,b,c=s:byte(i,i+2);local n=a*65536+(b or 0)*256+(c or 0);out[#out+1]=alphabet:sub(math.floor(n/262144)%64+1,math.floor(n/262144)%64+1)..alphabet:sub(math.floor(n/4096)%64+1,math.floor(n/4096)%64+1)..(b and alphabet:sub(math.floor(n/64)%64+1,math.floor(n/64)%64+1)or '=')..(c and alphabet:sub(n%64+1,n%64+1)or '=')end
 return table.concat(out)
end
function M.unbase64(s)
 if type(s)~='string'or #s%4~=0 or #s>3072 or s:find('[^A-Za-z0-9+/=]')then return nil end
 local out={}
 for i=1,#s,4 do local a,b,c,d=s:sub(i,i),s:sub(i+1,i+1),s:sub(i+2,i+2),s:sub(i+3,i+3);if not indexes[a]or not indexes[b]or c=='='and d~='='or (c=='='or d=='=')and i+3~=#s or c~='='and not indexes[c]or d~='='and not indexes[d]then return nil end
  local n=indexes[a]*262144+indexes[b]*4096+(indexes[c]or 0)*64+(indexes[d]or 0);out[#out+1]=string.char(math.floor(n/65536));if c~='='then out[#out+1]=string.char(math.floor(n/256)%256)end;if d~='='then out[#out+1]=string.char(n%256)end
 end
 local result=table.concat(out);if M.base64(result)~=s then return nil end;return result
end
function M.sender(id,channel,body)
 if type(id)~='string'or #id==0 or #id>80 or id:find('[^%w_%-]')or type(channel)~='string'or #channel>24 or channel:find('[^a-z_]')or type(body)~='string'or #body==0 or #body>M.MAX_BYTES then return nil end
 local self={id=id,channel=channel,body=body,offset=0}
 function self:chunk()if self.offset>=#self.body then return nil end;return {kind='item_chunk',id=self.id,channel=self.channel,offset=self.offset,total=#self.body,data=M.base64(self.body:sub(self.offset+1,self.offset+M.CHUNK_BYTES))}end
 function self:copied(chunk)if chunk.id~=self.id or chunk.channel~=self.channel or chunk.offset~=self.offset then return false end;self.offset=self.offset+math.min(M.CHUNK_BYTES,#self.body-self.offset);return true end
 return self
end
function M.receiver()
 local self={entries={},count=0,completed={}}
 function self:reset()self.entries={};self.count=0;self.completed={}end
 function self:accept(chunk)
  if type(chunk)~='table'or type(chunk.id)~='string'or #chunk.id==0 or #chunk.id>80 or chunk.id:find('[^%w_%-]')or type(chunk.channel)~='string'or #chunk.channel>24 or chunk.channel:find('[^a-z_]')or type(chunk.total)~='number'or chunk.total%1~=0 or chunk.total<1 or chunk.total>M.MAX_BYTES or type(chunk.offset)~='number'or chunk.offset%1~=0 or chunk.offset<0 or chunk.offset>=chunk.total then return nil,'invalid'end
  local bytes=M.unbase64(chunk.data);if not bytes or #bytes==0 or #bytes>chunk.total-chunk.offset then return nil,'invalid'end
  local key=chunk.channel..':'..chunk.id;local entry=self.entries[key]
  if not entry then
   if chunk.offset~=0 then return nil,'busy'end
   -- Retain a bounded replay window, but completed copies cannot reserve all
   -- assembly slots forever. Never evict an incomplete descriptor.
   while self.count>=M.MAX_INFLIGHT and #self.completed>0 do local old=table.remove(self.completed,1);if self.entries[old]and self.entries[old].complete then self.entries[old]=nil;self.count=self.count-1 end end
   if self.count>=M.MAX_INFLIGHT then return nil,'busy'end
   entry={total=chunk.total,offset=0,parts={},chunks={}};self.entries[key]=entry;self.count=self.count+1
  end
  if entry.total~=chunk.total then return nil,'conflict'end
  if chunk.offset<entry.offset then if entry.chunks[chunk.offset]~=bytes then return nil,'conflict'end;return entry.complete,'replay'end
  if chunk.offset~=entry.offset then return nil,'gap'end
  entry.chunks[chunk.offset]=bytes;entry.parts[#entry.parts+1]=bytes;entry.offset=entry.offset+#bytes
  if entry.offset==entry.total then entry.complete=table.concat(entry.parts);self.completed[#self.completed+1]=key;return entry.complete,'complete'end
  return nil,'pending'
 end
 function self:forget(id,channel)local key=channel..':'..id;if self.entries[key]then self.entries[key]=nil;self.count=self.count-1 end;for i=#self.completed,1,-1 do if self.completed[i]==key then table.remove(self.completed,i)end end end
 return self
end
return M
