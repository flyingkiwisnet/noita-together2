-- Actual candidate DLL and OS pipe; game/service authority remains fixture facts.
local ffi=require("ffi")
ffi.cdef[[
void __stdcall Sleep(unsigned long);
int nt2_g3_prepare(uint32_t*,uint8_t*);
int nt2_g3_seed_accepted(const uint8_t*,uint32_t);
int nt2_g3_world_loaded(const uint8_t*);
int nt2_g3_status(const uint8_t*);
int nt2_g3_report(const uint8_t*,const uint8_t*,uint32_t,uint64_t);
int nt2_g3_move(const uint8_t*,uint32_t,int32_t,int32_t,int32_t,int32_t,uint32_t);
int nt2_g3_peers(const uint8_t*,uint8_t*,uint32_t);
int nt2_g3_close(const uint8_t*);
]]
local dll=ffi.load(assert(os.getenv("NT2_G3_TRANSPORT_DLL")))
local dir=assert(os.getenv("NT2_G3_TRANSPORT_DIR"))
local function mark(name) local f=assert(io.open(dir.."/"..name,"wb"));f:write("fixture");f:close() end
local function exists(name) local f=io.open(dir.."/"..name,"rb");if f then f:close();return true end end
local function wait(fn) for i=1,800 do if fn() then return end;ffi.C.Sleep(10) end;error("fixture timeout") end
local context=ffi.new("uint8_t[16]");local seed=ffi.new("uint32_t[1]")
if os.getenv("NT2_G3_SAVE_REFUSAL")=="1" then
    assert(dll.nt2_g3_prepare(seed,context)==-2);mark("saved-refused");print("PASS production saved-world refusal in owned fixture tree");return
end
assert(dll.nt2_g3_prepare(seed,context)==1 and seed[0]==424242)
assert(dll.nt2_g3_seed_accepted(context,424241)==0)
assert(dll.nt2_g3_seed_accepted(context,424242)==1)
assert(dll.nt2_g3_world_loaded(context)==1)
assert(dll.nt2_g3_move(context,1,0,0,0,0,0)==0) -- Protected, before Start.
local function report(protected,sequence)
    local body='{"actualSeed":"424242","mods":["noita_together2"],"complete":true,"error":null,"beta":null,"loaded":true,"protected":'..tostring(protected)..',"modVersion":"nt2-g3-dev-1","bridgeVersion":"g3-transport-1"}'
    wait(function() return dll.nt2_g3_report(context,ffi.cast("const uint8_t*",body),#body,sequence)==1 end)
end
report(true,1);mark("protected");wait(function() return exists("start") end)
report(false,2);assert(dll.nt2_g3_status(context)==2);mark("playing")
wait(function() return exists("authorized") end)
local foreign=ffi.new("uint8_t[16]");ffi.fill(foreign,16,9)
assert(dll.nt2_g3_move(foreign,1,0,0,0,0,0)==0)
assert(dll.nt2_g3_close(foreign)==0)
assert(dll.nt2_g3_move(context,1,-1250,2000,0,-10,1)==1)
assert(dll.nt2_g3_move(context,1,-1250,2000,0,-10,1)==0);mark("pose")
local reply=ffi.new("uint8_t[864]")
wait(function() return dll.nt2_g3_peers(context,reply,864)==864 end)
assert(ffi.string(reply,4)=="NG3R" and reply[4]==2 and reply[5]==1 and reply[6]==0)
assert(dll.nt2_g3_peers(context,reply,351)==0);mark("peers")
wait(function() return exists("pause") end)
-- No game callbacks during this wait: native heartbeat must observe the hold.
wait(function() return dll.nt2_g3_status(context)~=2 end)
assert(dll.nt2_g3_move(context,2,0,0,0,0,0)==0)
assert(dll.nt2_g3_peers(context,reply,864)==0)
assert(dll.nt2_g3_close(context)==1);assert(dll.nt2_g3_status(context)==0);mark("closed")
print("PASS actual G3 DLL protected/Start/context/pose/poll/pause/cleanup")
