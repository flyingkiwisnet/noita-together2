-- Real shipped Lua, injected API lifecycle. No actual game or save writes.
local repo=assert(os.getenv('NT2_SEED_LOOT_REPO'))
local path='mods/noita_together2/files/loot/run-policy.txt'
local virtual,world,recorded,seed,write_count='saved',0,'','424242',0
local function context()
 local e=setmetatable({},{__index=_G});e._G=e
 e.ModTextFileGetContent=function(p) assert(p==path);return virtual end
 e.ModTextFileSetContent=function(p,v) assert(p==path);virtual=v end
 e.GameGetWorldStateEntity=function() return world end
 e.GlobalsGetValue=function(_,fallback) assert(world>0,'WorldState missing');return recorded~='' and recorded or fallback end
 e.GlobalsSetValue=function(_,v) assert(world>0,'WorldState missing');recorded=v;write_count=write_count+1 end
 e.StatsGetValue=function() return seed end
 local f=assert(loadfile(repo..'/mod/noita_together2/files/loot/policy.lua'));setfenv(f,e);return f(),e
end
local a=context();local b=context()
a.stage('shared:1:424242');assert(a.enabled() and b.enabled() and write_count==0)
world=1;assert(b.enabled() and recorded=='shared:1:424242' and write_count==1)
assert(a.enabled() and write_count==1,'separate reader must not repeat an unchanged world write')
-- No callback resets a module-local cache; the reused context must observe new bootstrap/world.
world=0;a.stage('shared:0:424242');assert(not b.enabled())
world=2;recorded='shared:1:424242';assert(not b.enabled() and recorded=='shared:0:424242')
world=0;a.stage('individual:0');assert(not b.enabled())
world=3;recorded='shared:1:424242';assert(not b.enabled() and recorded=='individual:0')
world=0;a.stage('');assert(not b.enabled())
world=4;recorded='shared:1:424242';assert(not b.enabled() and recorded=='')
local previous_writes=write_count;a.saved();recorded='shared:1:424242';assert(b.enabled() and write_count==previous_writes)
seed='999';assert(not b.enabled() and recorded=='shared:1:424242','saved other-world policy must not leak')
seed='424242';recorded='shared:0:424242';assert(not b.enabled() and write_count==previous_writes)
recorded='individual:0';assert(not b.enabled() and write_count==previous_writes)
for _,value in ipairs({'new:shared:1:0','new:shared:1:0424242','new:shared:1:4294967296','new:shared:1:424242\n','return true','saved:shared:1:424242'}) do
 virtual=value;recorded='shared:1:424242';assert(not b.enabled() and write_count==previous_writes)
end
a.stage('shared:0:0');assert(not b.enabled() and recorded=='shared:0:0','legacy zero/off policy retained')
local c,e=context();a.stage('shared:1:424242');recorded=''
e.GlobalsSetValue=function() end -- Native API can log/refuse without raising a Lua exception.
assert(not c.enabled() and recorded=='','failed world write must fail readback, not inherit stale globals')
world=0;assert(not pcall(e.GlobalsGetValue,'nt2_seed_loot_policy_v1',''))
assert(a.enabled(),'known staged early policy does not require world globals')
print('PASS LOOT01 lifecycle: early separate contexts, first valid world persistence, reused-context replacement, off/individual/refusal, saved/offline preservation, mismatched world, malformed inert bootstrap, legacy0/off, silent write refusal/readback. Engine facts injected.')
