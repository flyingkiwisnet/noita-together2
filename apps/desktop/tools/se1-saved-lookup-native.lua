local ffi=require('ffi')
ffi.cdef[[void __stdcall Sleep(unsigned long); unsigned long long __stdcall GetTickCount64();
int nt2_g3_saved_lookup(const uint8_t*,uint32_t);int nt2_g3_resume(const uint8_t*,const uint8_t*,uint32_t,uint8_t*);
int nt2_g3_status(const uint8_t*);int nt2_g3_close(const uint8_t*);int nt2_g3_report(const uint8_t*,const uint8_t*,uint32_t,uint64_t);]]
ffi.cdef[[void* __stdcall CreateFileA(const char*,unsigned long,unsigned long,void*,unsigned long,unsigned long,void*);int __stdcall WriteFile(void*,const void*,unsigned long,unsigned long*,void*);int __stdcall ReadFile(void*,void*,unsigned long,unsigned long*,void*);int __stdcall CloseHandle(void*);int __stdcall SetNamedPipeHandleState(void*,unsigned long*,unsigned long*,unsigned long*);]]
local dir=assert(os.getenv('NT2_OWNER_DIR'));local win=ffi.load('kernel32')
local function read(name)local f=io.open(dir..'/'..name,'rb');if not f then return end;local s=f:read('*a');f:close();return s end
local function mark(name,s)local f=assert(io.open(dir..'/'..name,'wb'));f:write(s or 'fixture');f:close()end
local until_ms=tonumber(win.GetTickCount64())+12000
if os.getenv('NT2_OWNER_OLD')=='1' then
 mark('old-ready');while not read('old-exit') and tonumber(win.GetTickCount64())<until_ms do win.Sleep(5)end
 assert(read('old-exit'),'old fixture timed out');return
end
local dll=ffi.load(assert(os.getenv('NT2_OWNER_DLLS'))..'/nt2_g3_transport.dll')
local marker=assert(read('marker.json'));local report=assert(read('report.json'));local context=ffi.new('uint8_t[16]')
local id=assert(marker:match('"local_world"%s*:%s*"([%x%-]+)"')):gsub('%-','');assert(#id==32)
local association=ffi.new('uint8_t[16]');for i=0,15 do association[i]=tonumber(id:sub(i*2+1,i*2+2),16)end
local seed=assert(marker:match('"actual_seed"%s*:%s*"(%d+)"'));local old_context
for cycle=1,(os.getenv('NT2_OWNER_TURNOVER')=='1' and 2 or 1) do
 local known=false;local resumed=false;local probed=false;context=ffi.new('uint8_t[16]');until_ms=tonumber(win.GetTickCount64())+12000
 while tonumber(win.GetTickCount64())<until_ms do
 if not known then local lookup=dll.nt2_g3_saved_lookup(marker,#marker);if lookup==2 then known=true;mark('lookup-known')else assert(lookup==0 or lookup==4,'unexpected lookup '..lookup)end end
 if known and not resumed then local r=dll.nt2_g3_resume(association,seed,#seed,context);if r==1 then resumed=true;mark('native-collection')end end
 if resumed then dll.nt2_g3_report(context,report,#report,1)end
 assert(dll.nt2_g3_status(context)~=2,'released without canonical Start')
 if cycle==2 and resumed and not probed and read('probe-old-context') then
  local frame=ffi.new('uint8_t[?]',32+#marker);local w=ffi.cast('uint32_t*',frame);w[0]=0x3247544e;w[1]=8;w[2]=14;w[3]=#marker;ffi.copy(frame+16,old_context,16);ffi.copy(frame+32,marker,#marker)
  local pipe=win.CreateFileA(assert(read('pipe-name.txt')),0xc0000000,0,nil,3,0,nil);assert(pipe~=ffi.cast('void*',-1))
  local mode=ffi.new('unsigned long[1]',3);assert(win.SetNamedPipeHandleState(pipe,mode,nil,nil)~=0)
  local n=ffi.new('unsigned long[1]');local reply=ffi.new('uint8_t[48]');assert(win.WriteFile(pipe,frame,32+#marker,n,nil)~=0 and n[0]==32+#marker);local read_ok=false;local end_ms=tonumber(win.GetTickCount64())+1000;repeat read_ok=win.ReadFile(pipe,reply,48,n,nil)~=0;if not read_ok then win.Sleep(1)end until read_ok or tonumber(win.GetTickCount64())>=end_ms;win.CloseHandle(pipe);assert(read_ok and n[0]==48);assert(ffi.cast('uint32_t*',reply)[2]==0,'old context lookup replaced live owner');mark('old-lookup-refused');probed=true
 end
 if read(cycle==1 and 'recovery-complete' or 'turnover-recovery-complete') then
  assert(known and resumed);mark(cycle==1 and 'protected-after-recovery' or 'protected-after-turnover')
  if cycle==1 and os.getenv('NT2_OWNER_TURNOVER')=='1' then
   old_context=ffi.string(context,16);dll.nt2_g3_close(context);win.Sleep(100)
   -- Force the real op14 with the still-loaded broker context to reach Rust
   -- before Core polls. This exposes the previously context-free selector key.
   local lookup_deadline=tonumber(win.GetTickCount64())+2000;local pending_lookup
   repeat pending_lookup=dll.nt2_g3_saved_lookup(marker,#marker);if pending_lookup~=4 then win.Sleep(10)end until pending_lookup==4 or tonumber(win.GetTickCount64())>=lookup_deadline
   assert(pending_lookup==4,'new lookup did not reach mailbox context reset');mark('turnover-ready')
   while not read('turnover-go') and tonumber(win.GetTickCount64())<until_ms do win.Sleep(5)end
   assert(read('turnover-go'));break
  else dll.nt2_g3_close(context);print('PASS real saved lookup -> native report -> recovered, held before Start');return end
 end
 win.Sleep(5)
 end
 assert(cycle==1 and os.getenv('NT2_OWNER_TURNOVER')=='1','bounded saved lookup/recovery fixture timed out')
end
