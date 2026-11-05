-- Real private wrapper ABI only; no prepare, authority, app, game or pipe run.
local ffi=require('ffi');ffi.cdef[[int nt2_g3_peers(const uint8_t*,uint8_t*,uint32_t);int nt2_g3_status(const uint8_t*);void Sleep(unsigned long);]]
local image=ffi.load(assert(os.getenv('NT2_A3_PEER_READ_DLL')));local context=ffi.new('uint8_t[16]');ffi.fill(context,16,1);local out=ffi.new('uint8_t[864]')
for _=1,20 do
 assert(image.nt2_g3_peers(context,out,864)==-1,'Missing broker or unbound scope must never report eligible availability')
 assert(image.nt2_g3_status(context)==0,'Read smoke cannot mint playing authority');ffi.C.Sleep(5)
end
print('PASS real private wrapper40 fail-closed peer/status assertions; no prepared scope/pipe/game/service')
