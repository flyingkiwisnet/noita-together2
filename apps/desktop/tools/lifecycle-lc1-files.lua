-- A3 private fixture files only. Win32 readers share deletion/writes; completed
-- snapshots publish by atomic rename. Retry only sharing/lock/access failures.
local ffi=require('ffi')
ffi.cdef[[
void* __stdcall CreateFileA(const char*,unsigned long,unsigned long,void*,unsigned long,unsigned long,void*);
int __stdcall ReadFile(void*,void*,unsigned long,unsigned long*,void*);
int __stdcall WriteFile(void*,const void*,unsigned long,unsigned long*,void*);
int __stdcall GetFileSizeEx(void*,int64_t*);
int __stdcall DeleteFileA(const char*);
int __stdcall MoveFileExA(const char*,const char*,unsigned long);
int __stdcall CloseHandle(void*);
unsigned long __stdcall GetLastError();
unsigned long __stdcall GetTickCount();
void __stdcall Sleep(unsigned long);
]]
local M={};local invalid=ffi.cast('void*',-1)
local function retry(operation)
 local until_time=tonumber(ffi.C.GetTickCount())+250
 while true do local value,code=operation();if value~=nil then return value end
  if (code~=5 and code~=32 and code~=33)or tonumber(ffi.C.GetTickCount())>=until_time then error('Private fixture file I/O error '..tostring(code))end
  ffi.C.Sleep(2)
 end
end
function M.new(dir)
 local serial=0;local self={}
 local function path(name)assert(type(name)=='string'and #name<=128 and name:match('^[%w_.%-]+$')and name~='.'and name~='..','Private signal basename required');return dir..'/'..name end
 local function remove(file)return retry(function()if ffi.C.DeleteFileA(file)~=0 then return true end;local code=tonumber(ffi.C.GetLastError());if code==2 or code==3 then return true end;return nil,code end)end
 function self:mark(name,value)
  local target=path(name);serial=serial+1;local temporary=path('.lc1-'..serial..'.tmp');local bytes=tostring(value or 'fixture');assert(#bytes<=65536,'Private output size bound: '..name..' bytes='..#bytes)
  retry(function()
   local h=ffi.C.CreateFileA(temporary,0x40000000,7,nil,2,0,nil);if h==invalid then return nil,tonumber(ffi.C.GetLastError())end
   local count=ffi.new('unsigned long[1]');local ok=ffi.C.WriteFile(h,bytes,#bytes,count,nil);local code=tonumber(ffi.C.GetLastError());ffi.C.CloseHandle(h)
   if ok==0 then return nil,code end;assert(tonumber(count[0])==#bytes,'Private output incomplete');return true
  end)
  local ok,reason=pcall(retry,function()if ffi.C.MoveFileExA(temporary,target,1)~=0 then return true end;return nil,tonumber(ffi.C.GetLastError())end)
  if not ok then pcall(remove,temporary);error(reason)end
 end
 function self:input(name,consume)
  local file=path(name);local value=retry(function()
   local h=ffi.C.CreateFileA(file,0x80000000,7,nil,3,0,nil);if h==invalid then local code=tonumber(ffi.C.GetLastError());if code==2 or code==3 then return false end;return nil,code end
   local size=ffi.new('int64_t[1]');if ffi.C.GetFileSizeEx(h,size)==0 then local code=tonumber(ffi.C.GetLastError());ffi.C.CloseHandle(h);return nil,code end
   local length=tonumber(size[0]);if length<0 or length>65536 then ffi.C.CloseHandle(h);error('Private signal size bound')end
   local bytes=ffi.new('uint8_t[?]',math.max(1,length));local count=ffi.new('unsigned long[1]');local ok=ffi.C.ReadFile(h,bytes,length,count,nil);local code=tonumber(ffi.C.GetLastError());ffi.C.CloseHandle(h)
   if ok==0 then return nil,code end;assert(tonumber(count[0])==length,'Private signal changed while reading');return ffi.string(bytes,length)
  end)
  if value==false then return nil end;if consume then remove(file)end;return value
 end
 function self:exists(name)return self:input(name,false)~=nil end
 return self
end
return M
