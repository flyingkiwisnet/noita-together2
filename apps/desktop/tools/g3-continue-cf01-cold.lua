local ffi=require('ffi')
ffi.cdef[[void __stdcall Sleep(unsigned long);unsigned long __stdcall GetTickCount();int nt2_g3_saved_lookup(const uint8_t*,uint32_t);]]
local image=ffi.load(assert(os.getenv('NT2_REJOIN_NATIVE'))..'/nt2_g3_transport.dll')
local record='{"local_world":"11111111-1111-1111-1111-111111111111"}'
local until_at=tonumber(ffi.C.GetTickCount())+1500;local value=4;local maximum=0
-- First native call is the saved metadata lookup: no resume/startup/warmup.
repeat local at=tonumber(ffi.C.GetTickCount());value=tonumber(image.nt2_g3_saved_lookup(record,#record));maximum=math.max(maximum,tonumber(ffi.C.GetTickCount())-at);ffi.C.Sleep(10) until value~=4 or tonumber(ffi.C.GetTickCount())>=until_at
if os.getenv('NT2_CF_BASELINE')=='1' then assert(value==4);print('CF01_NATIVE_NEGATIVE cold_lookup_stuck=4 no_prepare=true') else assert(value==1,'cold lookup did not receive authenticated local reply');assert(maximum<80,'cache export blocked');print('CF01_NATIVE_PASS cold_lookup=1 no_prepare=true maximum_ms='..maximum) end
