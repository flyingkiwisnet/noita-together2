local ffi=require('ffi');ffi.cdef[[int nt2_g3_startup_bundle(uint8_t*,uint32_t,uint8_t*,uint32_t);int nt2_g3_startup_diagnostic(uint8_t*,uint32_t);int nt2_g3_close(const uint8_t*);int nt2_g3_status(const uint8_t*);void Sleep(unsigned long);uint64_t GetTickCount64();]]
local root=assert(os.getenv('NT2_STARTUP_DLLS'));local primary=ffi.load(root..'/nt2_g3_transport.dll');local social=ffi.load(root..'/nt2_g3_social.dll')
local context=ffi.new('uint8_t[16]');local loader_delay=tonumber(os.getenv('NT2_STARTUP_LOADER_DELAY') or '0');local win=ffi.load('kernel32')
if loader_delay>0 then local began=tonumber(win.GetTickCount64());assert(primary.nt2_g3_status(context)==0 and social.nt2_g3_status(context)==0);assert(tonumber(win.GetTickCount64())-began<100,'First cached frame exports blocked behind loader') end
local bundle=ffi.new('uint8_t[16384]');local result=primary.nt2_g3_startup_bundle(bundle,16384,context,tonumber(os.getenv('NT2_STARTUP_BUDGET')))
local consumer=dofile(assert(os.getenv('NT2_STARTUP_REPO'))..'/mod/noita_together2/files/startup/bundle.lua')
local expect=os.getenv('NT2_STARTUP_EXPECT')=='1';assert((result==1)==expect,'Unexpected complete startup result '..result)
local word=ffi.cast('uint32_t*',bundle);local diagnostic=ffi.new('uint8_t[256]');assert(primary.nt2_g3_startup_diagnostic(diagnostic,256)==256);local d=ffi.cast('uint32_t*',diagnostic)
if loader_delay>0 then
 assert(d[0]==0x3154534e and d[1]==1)
 if expect then assert(d[36]>=loader_delay-100 and d[37]<6000,'Real loader stage timing absent')
 else
  assert(d[3]==14 and d[37]<=tonumber(os.getenv('NT2_STARTUP_BUDGET'))+100 and d[39]==0,'Loader expiry: reason='..d[3]..' elapsed_ms='..d[37]..' claim_known='..d[39])
  local original=ffi.string(diagnostic,256);win.Sleep(loader_delay+100)
  assert(primary.nt2_g3_startup_bundle(bundle,16384,context,300)==0,'Late loader completion silently created a fresh claim')
  assert(primary.nt2_g3_startup_diagnostic(diagnostic,256)==256 and ffi.string(diagnostic,256)==original)
 end
end
if expect then
 assert(word[0]==0x3247544e and word[1]==9 and word[3]==1 and word[5]==424242 and word[6]==5 and word[9]>0)
 local json=ffi.string(bundle+192,word[9]);assert(json:find('"config":',1,true) and json:find('"scope":',1,true) and json:find('"bundle_digest":',1,true));assert(not json:find('proof',1,true))
 local parsed=assert(consumer.request(primary,ffi,context,6000));assert(parsed.seed==424242 and parsed.policy==5 and parsed.snapshot==json)
 local other=ffi.new('uint8_t[16]');local second=ffi.new('uint8_t[16384]');local remaining=6000-tonumber(d[37]);assert(remaining>=100);assert(social.nt2_g3_startup_bundle(second,16384,other,remaining)==1);assert(ffi.string(other,16)==ffi.string(context,16));assert(ffi.string(second+64,96)==ffi.string(bundle+64,96));assert(ffi.string(second+192,word[9])==json)
end
local latched=ffi.string(diagnostic,256);primary.nt2_g3_close(context);assert(primary.nt2_g3_startup_diagnostic(diagnostic,256)==256 and ffi.string(diagnostic,256)==latched,'Close lost original diagnostic')
local retained=os.getenv('NT2_STARTUP_RETAINED')
if retained and retained~='' then
 local original_context=ffi.string(context,16);local original_identity=ffi.string(bundle+64,96)
 local zero=ffi.new('uint8_t[16]');assert(primary.nt2_g3_startup_bundle(bundle,16384,zero,200)==0,'Closed attempt silently restarted')
 assert(primary.nt2_g3_startup_bundle(bundle,16384,context,200)==0,'Old context silently restarted')
 local done=assert(io.open(retained..'/first.done','wb'));done:write('closed');done:close()
 local ready
 for _=1,500 do ready=io.open(retained..'/new.ready','rb');if ready then ready:close();break end;win.Sleep(10) end
 assert(ready,'Fixture did not inject next distinct world')
 local next_context=ffi.new('uint8_t[16]');ffi.fill(next_context,16,67)
 assert(primary.nt2_g3_startup_bundle(bundle,16384,next_context,6000)==1,'Explicit injected new-world entry failed')
 assert(ffi.string(next_context,16)~=original_context and ffi.string(bundle+64,96)~=original_identity,'Previous world result leaked into new attempt')
 assert(primary.nt2_g3_close(context)==0,'Old close touched new attempt')
 local other=ffi.new('uint8_t[16]');local second=ffi.new('uint8_t[16384]');assert(social.nt2_g3_startup_bundle(second,16384,other,6000)==1)
 assert(ffi.string(other,16)==ffi.string(next_context,16) and ffi.string(second+64,96)==ffi.string(bundle+64,96))
 primary.nt2_g3_close(next_context)
end
print('STARTUP_REAL_PASS result='..result..' reason='..d[3]..' elapsed_ms='..d[37]..' loader_ms='..d[36]..' claim_ms='..d[6]..' attempts='..d[8]..' policy='..word[6]..'; engine callbacks not executed')
