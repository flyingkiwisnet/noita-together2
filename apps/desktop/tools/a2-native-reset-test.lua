-- Real private wrappers/broker/Windows pipe; admission responses are injected.
local ffi=require('ffi');ffi.cdef[[
void __stdcall Sleep(unsigned long);unsigned long __stdcall GetTickCount(void);
int nt2_g3_resume(const uint8_t*,const uint8_t*,uint32_t,uint8_t*);
int nt2_g3_status(const uint8_t*);int nt2_g3_world_loaded(const uint8_t*);
int nt2_g3_report(const uint8_t*,const uint8_t*,uint32_t,uint64_t);
int nt2_g3_social_lease(const uint8_t*,uint8_t*,uint32_t);
int nt2_g3_social_next(const uint8_t*,uint8_t*,uint32_t);
]]
local root=assert(os.getenv('NT2_A2_NATIVE_DIR'))
local primary=ffi.load(root..'/nt2_g3_transport.dll');local social=ffi.load(root..'/nt2_g3_social.dll')
local association=ffi.new('uint8_t[16]');ffi.fill(association,16,1)
local context=ffi.new('uint8_t[16]');local peer=ffi.new('uint8_t[16]')
local lease=ffi.new('uint8_t[64]');local frame=ffi.new('uint8_t[32768]')
local began=tonumber(ffi.C.GetTickCount());local joined=false;local first=nil;local second=nil
local function u32(p,n)return tonumber(ffi.cast('uint32_t*',p+n)[0])end
while tonumber(ffi.C.GetTickCount())-began<10000 do
 local admitted=primary.nt2_g3_resume(association,'424242',6,context)
 if admitted==1 then
  if not joined and social.nt2_g3_resume(association,'424242',6,peer)==1 then assert(ffi.string(peer,16)==ffi.string(context,16));joined=true end
  primary.nt2_g3_world_loaded(context);primary.nt2_g3_report(context,'{}',2,1)
  if social.nt2_g3_social_lease(context,lease,64)==64 then
   local size=social.nt2_g3_social_next(context,frame,32768)
   if size>0 then
    assert(ffi.string(frame,5)=='NSF1\1');assert(ffi.string(frame+8,16)==ffi.string(context,16))
    local generation,sequence=u32(frame,24),u32(frame,28)
    if generation==1 then assert(not first,'Duplicate first frame');first=sequence
    elseif generation==2 then assert(first and sequence>first,'Generation reused old ACK identity');second=sequence
    else error('Unexpected generation') end
   end
  end
 end
 if second then ffi.C.Sleep(100);print('PASS real broker reset: same context; generation1 frame '..first..'; generation2 frame '..second);return end
 ffi.C.Sleep(1)
end
error('Bounded real-broker reset timeout; first='..tostring(first)..'; second='..tostring(second))
