local ffi=require('ffi');ffi.cdef[[
void __stdcall Sleep(unsigned long);
unsigned long __stdcall GetTickCount(void);
int nt2_g3_startup_prepare(uint32_t*,uint8_t*);
int nt2_g3_resume(const uint8_t*,const uint8_t*,uint32_t,uint8_t*);
int nt2_g3_status(const uint8_t*);int nt2_g3_close(const uint8_t*);
typedef struct {uint32_t version,size;void* prepare;void* control;void* status;void* report;void* move;void* copy;void* diagnostic;int (*metrics)(uint32_t*,uint32_t);void* resume;} RejoinApi;
const RejoinApi* nt2_g3_private_api(void);
]]
local root=assert(os.getenv('NT2_REJOIN_NATIVE'))
local primary=ffi.load(root..'/nt2_g3_transport.dll');local social=ffi.load(root..'/nt2_g3_social.dll')
local private_broker=ffi.load(root..'/nt2_g3_ipc.dll');assert(primary.nt2_g3_private_api().version==2)
local seed=ffi.new('uint32_t[1]');local original=ffi.new('uint8_t[16]')
local startup=primary.nt2_g3_startup_prepare(seed,original);print('PRIVATE_SAVED_STARTUP '..tonumber(startup))
assert(startup==-2,'global saved-world New Game guard retained')
local association=ffi.new('uint8_t[16]');ffi.fill(association,16,1)
local fresh=ffi.new('uint8_t[16]');local actual='424242'
assert(primary.nt2_g3_resume(association,actual,#actual,fresh)==-3)
local pending_other=ffi.new('uint8_t[16]');ffi.fill(pending_other,16,9)
assert(primary.nt2_g3_resume(pending_other,actual,#actual,ffi.new('uint8_t[16]'))==0,'another association cannot steal pending owner')
local function admitted(c,assoc)
 for i=1,1000 do local result=primary.nt2_g3_resume(assoc,actual,#actual,c);if result~=-3 then return result end;ffi.C.Sleep(1) end;error('private admission timeout')
end
assert(admitted(fresh,association)==1);assert(ffi.string(fresh,16)~=ffi.string(original,16));assert(primary.nt2_g3_status(fresh)==1,'admission is held until inspection')
local peer=ffi.new('uint8_t[16]');assert(social.nt2_g3_resume(association,actual,#actual,peer)==1 and ffi.string(peer,16)==ffi.string(fresh,16))
local second=ffi.new('uint8_t[16]');assert(primary.nt2_g3_resume(association,actual,#actual,second)==0,'healthy second primary copy refused')
assert(primary.nt2_g3_close(original)==0 and primary.nt2_g3_status(original)==0,'old token fenced')
local a,b=primary.nt2_g3_private_api(),social.nt2_g3_private_api();assert(a==b and a.version==2)
local metrics=ffi.new('uint32_t[9]');assert(a.metrics(metrics,9)==1 and metrics[0]>0)
local started=tonumber(ffi.C.GetTickCount());for i=1,2000 do assert(primary.nt2_g3_resume(association,actual,#actual,fresh)==1);assert(primary.nt2_g3_status(fresh)==1) end
local elapsed=tonumber(ffi.C.GetTickCount())-started;assert(elapsed<250,'recurring callbacks must not wait on network')
print('PRIVATE_REJOIN_POLL_MS '..elapsed)
ffi.C.Sleep(2100);assert(primary.nt2_g3_status(fresh)==0,'expired native authority holds')
local replacement=ffi.new('uint8_t[16]');assert(admitted(replacement,association)==1 and ffi.string(replacement,16)~=ffi.string(fresh,16))
assert(primary.nt2_g3_close(fresh)==0,'delayed old close cannot retire new generation');assert(primary.nt2_g3_status(replacement)==1)
assert(primary.nt2_g3_close(replacement)==1)
local denied=ffi.new('uint8_t[16]');local foreign=ffi.new('uint8_t[16]');ffi.fill(foreign,16,9);assert(admitted(denied,foreign)==0);assert(primary.nt2_g3_status(denied)==0)
local again=ffi.new('uint8_t[16]');assert(primary.nt2_g3_startup_prepare(seed,again)==-2,'no consumed New Game rearming')
print('PASS real private broker/both wrappers; saved guard; fresh generation; one worker; held admission; competing primary/old close/foreign association refused;2000 nonblocking polls; no engine/world mutation')
