local ffi=require('ffi');ffi.cdef[[int nt2_g3_startup_nonce(uint8_t*,uint32_t);]]
local path=assert(os.getenv('NT2_SE1_DLLS'));local seen={}
for _,name in ipairs({'nt2_g3_transport.dll','nt2_g3_social.dll'}) do
 local lib=ffi.load(path..'/'..name);assert(lib.nt2_g3_startup_nonce(nil,16)==0)
 local b=ffi.new('uint8_t[32]');ffi.fill(b,32,123);assert(lib.nt2_g3_startup_nonce(b,15)==0 and ffi.string(b,32)==string.rep('{',32))
 for i=1,32 do assert(lib.nt2_g3_startup_nonce(b,16)==1);local value=ffi.string(b,16);assert(value~=string.rep('\0',16) and not seen[value]);assert(ffi.string(b+16,16)==string.rep('{',16));seen[value]=true end
end
print('SE1_NONCE_PASS64 correlations unique/nonzero; invalid capacities unchanged; no broker/pipe/startup claim invoked')
