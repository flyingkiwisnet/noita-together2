-- Disposable fixture pacing only. A private timer avoids changing the system
-- timer resolution or treating Sleep17's measured32.7FPS as60FPS.
local ffi=require('ffi')
ffi.cdef[[
int __stdcall QueryPerformanceCounter(int64_t*);
int __stdcall QueryPerformanceFrequency(int64_t*);
void* __stdcall CreateWaitableTimerExA(void*,const char*,uint32_t,uint32_t);
int __stdcall SetWaitableTimer(void*,const int64_t*,int32_t,void*,void*,int);
uint32_t __stdcall WaitForSingleObject(void*,uint32_t);
int __stdcall CloseHandle(void*);
]]
local M={}
function M.new()
 local frequency=ffi.new('int64_t[1]');assert(ffi.C.QueryPerformanceFrequency(frequency)~=0)
 local hz=tonumber(frequency[0]);assert(hz>0)
 local counter,due=ffi.new('int64_t[1]'),ffi.new('int64_t[1]')
 local timer=ffi.C.CreateWaitableTimerExA(nil,nil,2,0x00100002)
 assert(timer~=nil,'Private high-resolution fixture timer unavailable')
 timer=ffi.gc(timer,function(handle) ffi.C.CloseHandle(handle) end)
 local m={}
 function m.time() assert(ffi.C.QueryPerformanceCounter(counter)~=0);return tonumber(counter[0])/hz end
 function m.wait(started,hint)
  assert(hint==17 or hint==33,'Only60/30FPS fixture targets are allowed')
  local remaining=started+1/(hint==17 and 60 or 30)-m.time()
  if remaining<=0 then return end -- Actual callback overrun remains visible.
  due[0]=-math.max(1,math.ceil(remaining*10000000))
  assert(ffi.C.SetWaitableTimer(timer,due,0,nil,nil,0)~=0)
  assert(ffi.C.WaitForSingleObject(timer,1000)==0,'Fixture pacing wait failed')
 end
 function m.close() if timer then local handle=ffi.gc(timer,nil);timer=nil;assert(ffi.C.CloseHandle(handle)~=0) end end
 return m
end
return M
