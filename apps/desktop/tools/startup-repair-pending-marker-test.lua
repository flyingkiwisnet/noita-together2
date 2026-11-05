-- Actual saved-world module; injected engine/authority only. No Noita launch.
local root=assert(os.getenv('NT2_ST1_REPO'));local M=dofile(root..'/mod/noita_together2/files/rejoin/local_world.lua')
local world=0;local values={};local writes={};local held,released,retired=0,0,0
GameGetWorldStateEntity=function() return world end;StatsGetValue=function() return '424242' end
GlobalsGetValue=function(k,f) return values[k] or f end;GlobalsSetValue=function(k,v) assert(world>0);values[k]=v;writes[#writes+1]=k end
SetWorldSeed=function() error('Saved pending marker must not set seed') end
local marker='{"kind":"nt2-pending","version":1,"local_world":"11111111-1111-4111-8111-111111111111","bundle_digest":"'..string.rep('a',64)..'","actual_seed":"424242","loot":"shared:1:424242"}'
local canonical=false;local result='admitted';local last_generation
local adapter={known=function(saved) return saved.record==marker,true,false,false end,hold_owned=function() held=held+1;return {owned=true} end,release_owned=function(h) assert(h.owned);released=released+1 end,request=function(_,g) last_generation=g;return 'local handle' end,poll=function() return result,last_generation end,inspected=function() return canonical end,retire=function() retired=retired+1 end}
local fresh=M.new(adapter);assert(not fresh:waiting(marker,'424242','shared:1:424242'));assert(#writes==0)
world=42;assert(fresh:waiting(marker,'424242','shared:1:424242'));assert(writes[#writes]=='nt2_rejoin_local_world_v1');assert(M.decode_saved(values.nt2_rejoin_local_world_v1)==marker)
local restored=M.new(adapter);assert(restored:loaded());assert(held==1 and released==0 and restored.state=='reconnecting');restored:tick();assert(restored.state=='inspecting' and released==0)
result='released';restored:tick();assert(released==0,'Pending marker/recovery response must not release without current canonical inspection')
canonical=true;restored:tick();assert(released==1);restored:lost();assert(held==2);restored:offline();assert(retired==1 and values.nt2_rejoin_local_world_v1_offline=='1');assert(not M.new(adapter):loaded())
assert(not M.new(adapter):waiting(marker,'424242','shared:1:424242'),'Offline tombstone must prevent late pending save')
print('PENDING_MARKER_PASS loaded_only=true complete_record_last=true no_reseed=true same_hold_owner=true explicit_offline=true injected_engine=true')
