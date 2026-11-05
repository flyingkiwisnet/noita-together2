-- Literal UTF-8, shared by the local composer and native capture codec.
local M={}
function M.length(s)
 if type(s)~='string' or #s>396 then return nil end
 local i,count=1,0
 while i<=#s do
  local a=s:byte(i);local n,cp,min
  if a<128 then n=1;cp=a;min=0
  elseif a>=194 and a<=223 then n=2;cp=a-192;min=128
  elseif a>=224 and a<=239 then n=3;cp=a-224;min=2048
  elseif a>=240 and a<=244 then n=4;cp=a-240;min=65536 else return nil end
  for j=1,n-1 do local b=s:byte(i+j);if not b or b<128 or b>191 then return nil end;cp=cp*64+b-128 end
  if cp<min or cp>1114111 or cp>=55296 and cp<=57343 or cp<32 or cp>=127 and cp<=159 then return nil end
  count=count+1;if count>99 then return nil end;i=i+n
 end
 return count
end
function M.message(s) local n=M.length(s);return n and n>0 and s:find('[^ ]')~=nil end
function M.backspace(s)
 if not M.length(s) or #s==0 then return '' end
 local i=#s;while i>1 and s:byte(i)>=128 and s:byte(i)<192 do i=i-1 end;return s:sub(1,i-1)
end
return M
