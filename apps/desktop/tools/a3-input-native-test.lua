-- Real wrappers/broker and production bounded input. Admission is injected by
-- the Rust pipe fixture; no service, Noita, real save or installed app is used.
local ffi=require('ffi');ffi.cdef[[void __stdcall Sleep(unsigned long);unsigned long __stdcall GetTickCount(void);
int nt2_g3_debug_publish(const uint8_t*,const uint8_t*,uint32_t);int nt2_g3_debug_mode(const uint8_t*);
int nt2_g3_resume(const uint8_t*,const uint8_t*,uint32_t,uint8_t*);int nt2_g3_world_loaded(const uint8_t*);
int nt2_g3_report(const uint8_t*,const uint8_t*,uint32_t,uint64_t);int nt2_g3_social_publish(const uint8_t*,const uint8_t*,uint32_t);
int nt2_g3_startup_nonce(uint8_t*,uint32_t);int nt2_g3_social_lease(const uint8_t*,uint8_t*,uint32_t);int nt2_g3_social_next(const uint8_t*,uint8_t*,uint32_t);]]
local repo=assert(os.getenv('NT2_A3_REPO'));local dir=assert(os.getenv('NT2_A3_NATIVE_DIR'));local raw=dofile
dofile=function(p)return raw(repo..'/'..p:gsub('^mods/','mod/'))end
local primary=ffi.load(dir..'/nt2_g3_transport.dll');local social=ffi.load(dir..'/nt2_g3_social.dll')
local a=ffi.new('uint8_t[16]');ffi.fill(a,16,1);local c=ffi.new('uint8_t[16]');local peer=ffi.new('uint8_t[16]')
local header=ffi.new('uint8_t[64]');local frame=ffi.new('uint8_t[32768]');local began=tonumber(ffi.C.GetTickCount())
local debug_good='{"revision":1,"enabled":true,"logger":true,"frame":0,"time_ms":0,"peer_copies":0,"peer_unavailable":0,"peer_denied":0,"roster":0,"poses":0,"social_frames":0,"hud_samples":0,"hud_copied":0,"hud_received":0,"hud_refused":0,"poi_copied":0,"poi_refused":0,"social_errors":0,"movement_errors":0,"first_loss":null,"recent":[],"truncated":false}'
local debug_last=-1000;local off_seen=false;local on_seen=false
local outgoing,poi,emote;local stage=0;local hud_at=0;local callback_stalled=false;local pauses=0
while tonumber(ffi.C.GetTickCount())-began<8500 do
 if primary.nt2_g3_resume(a,'424242',6,c)==1 then
  if not outgoing and social.nt2_g3_resume(a,'424242',6,peer)==1 then outgoing=dofile('mods/noita_together2/files/g3/a2_native_input.lua').new(social,c,ffi)end
  primary.nt2_g3_world_loaded(c);primary.nt2_g3_report(c,'{}',2,1)
  if outgoing then
   local elapsed=tonumber(ffi.C.GetTickCount())-began
   if elapsed-debug_last>=100 then
    debug_last=elapsed;local mode=primary.nt2_g3_debug_mode(c);if elapsed>4500 and elapsed<5900 and mode==0 then off_seen=true end;if elapsed>6200 and mode==1 then on_seen=true end
    local value=elapsed>3000 and elapsed<3800 and '{"token":"MUST_NOT_PERSIST"}'or debug_good
    assert(primary.nt2_g3_debug_publish(c,value,#value)==1)
    assert(primary.nt2_g3_debug_publish(c,string.rep('x',4097),4097)==0,'oversized footer refused locally')
   end
   if social.nt2_g3_social_lease(c,header,64)==64 then
    local n=social.nt2_g3_social_next(c,frame,32768);if n>40 and frame[40]==5 then pauses=pauses+1 end
   end
   if stage==0 and outgoing:hud({hp=4,max_hp=8})then
    -- This guarantees occupancy before the one physical placement. No ACK-gap retry.
    poi=assert(outgoing:poi('single click under HUD',500,1200));stage=1
    assert(outgoing.diagnostics.poi_accepted==0,'explicit placement must initially wait behind actual HUD')
   end
   outgoing:flush()
   if stage==1 and outgoing.diagnostics.poi_accepted==1 then
    emote=assert(outgoing:identity());assert(outgoing:social('emote',emote,{emoteId='wave',active=true}));stage=2
    assert(outgoing.diagnostics.emote_accepted==0,'one emote click waits behind actual POI original')
   end
   if stage==2 and outgoing.diagnostics.emote_accepted==1 and not callback_stalled then callback_stalled=true;ffi.C.Sleep(300)end
   local elapsed=tonumber(ffi.C.GetTickCount())-began
   if elapsed>=hud_at then outgoing:hud({hp=4,max_hp=8});hud_at=elapsed+500 end
   if stage==2 and elapsed>6500 and outgoing.diagnostics.hud_accepted>=3 then
    assert(outgoing.diagnostics.poi_accepted==1 and outgoing.diagnostics.emote_accepted==1,'no duplicate native copies')
    assert(outgoing.diagnostics.hud_accepted>=2,'healthy HUD resumes after explicit actions')
    assert(off_seen and on_seen,'debug off/on must propagate through the real broker reply; off='..tostring(off_seen)..'; on='..tostring(on_seen))
    print('PASS one POI and one emote click copied once through actual occupied native slot;300ms callback stall; HUD '..outgoing.diagnostics.hud_accepted..'; pause frames '..pauses);return
   end
  end
 end
 ffi.C.Sleep(5)
end
error('bounded actual-native input test timed out at stage '..stage)

