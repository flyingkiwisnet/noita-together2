-- Runs the real x86 native encoder in original installed LuaJIT; no Noita callbacks.
local ffi=require("ffi")
ffi.cdef[[int NT2G3EncodePose(uint8_t*,uint32_t,const uint8_t*,uint32_t,int32_t,int32_t,int32_t,int32_t,uint32_t);]]
local native=ffi.load(assert(os.getenv("NT2_G3_NATIVE_DLL")))
local output=ffi.new("uint8_t[64]"); local context=ffi.new("uint8_t[16]");ffi.fill(context,16,1)
assert(native.NT2G3EncodePose(output,64,context,1,-1250,2000,0,-10,1)==64)
local bytes=ffi.string(output,64)
assert(bytes:sub(1,8)=="NTG3\1\0\1\0" and bytes:byte(45)==1)
assert(native.NT2G3EncodePose(output,63,context,1,0,0,0,0,0)==0)
assert(native.NT2G3EncodePose(output,64,context,0,0,0,0,0,0)==0)
assert(native.NT2G3EncodePose(output,64,context,1,0,0,0,0,2)==0)
ffi.fill(context,16,0);assert(native.NT2G3EncodePose(output,64,context,1,0,0,0,0,0)==0)
local file=assert(io.open(assert(os.getenv("NT2_G3_NATIVE_FRAME")),"wb"));file:write(bytes);file:close()
print("PASS real x86 G3 encoder bounds and deterministic frame; no game or authenticated transport")
