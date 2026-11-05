-- Candidate complete local transaction reader. It does not classify a world or
-- apply a seed; its caller must have a separately proven fresh-entry signal.
local M={};local bytes=16384;local header=192
function M.request(native,ffi,context,budget)
 local out=ffi.new('uint8_t[?]',bytes)
 local result=tonumber(native.nt2_g3_startup_bundle(out,bytes,context,budget or 6000))
 if result~=1 then return nil,'startup-transaction-failed',result end
 local word=ffi.cast('uint32_t*',out)
 local kind=tonumber(word[44]);if kind~=0 and kind~=1 then return nil,'unknown-startup-source-kind',result end
 if word[0]~=0x3247544e or word[1]~=9 or word[2]~=16 or word[3]~=1 or word[4]~=0 or (word[6]~=1 and word[6]~=2 and word[6]~=5) or word[7]==0 or word[8]==0 or word[8]>12000 or word[9]==0 or word[9]>bytes-header or ffi.string(out+48,16)~=ffi.string(context,16) or (kind==0 and ffi.string(out+64,16)~=ffi.string(context,16)) then return nil,'malformed-startup-bundle',result end
 local diagnostic=ffi.new('uint8_t[256]');if native.nt2_g3_startup_diagnostic(diagnostic,256)~=256 then return nil,'startup-timing-absent',result end
 local timing=ffi.cast('uint32_t*',diagnostic);if timing[0]~=0x3154534e or timing[1]~=1 or timing[2]~=1 or timing[3]~=0 or timing[37]>6000 or ffi.string(diagnostic+48,16)~=ffi.string(context,16) then return nil,'startup-timing-differs',result end
 return {kind=kind==1 and 'provisional' or 'canonical',seed=tonumber(word[5]),policy=tonumber(word[6]),context=context,world=ffi.string(out+64,16),digest=ffi.string(out+80,32),config_digest=ffi.string(out+112,32),generation=tonumber(word[7]),elapsed_ms=tonumber(timing[37]),snapshot=ffi.string(out+header,word[9]),raw=out},nil,result
end
return M
