-- Original Lua + actual entry; engine/save facts and native mailbox injected.
local repo=assert(os.getenv('NT2_CF_REPO'));local entry=assert(os.getenv('NT2_CF_ENTRY'));local baseline=os.getenv('NT2_CF_BASELINE')=='1'
local raw=dofile;raw(repo..'/apps/desktop/tools/mod-protection-stubs.lua');local s=NT2_FIXTURE
local ffi=require('ffi');local oldload=ffi.load
local ready=false;local globals_ready=false;local stats_ready=false;local writes,seeds,lookups,prepares,requests=0,0,0,0,0
local globals={nt2_rejoin_local_world_v1='{"local_world":"11111111-1111-1111-1111-111111111111"}',nt2_rejoin_local_world_v1_seed='424242',nt2_rejoin_local_world_v1_loot='shared:1:424242',nt2_rejoin_local_world_v1_offline='0',nt2_seed_loot_policy_v1='shared:1:424242'}
local save_codec=raw(repo..'/mod/noita_together2/files/rejoin/local_world.lua')
if os.getenv('NT2_CF03_RESTORED') then
 local f=assert(io.open(os.getenv('NT2_CF03_RESTORED'),'rb'));globals.nt2_rejoin_local_world_v1=f:read('*a');f:close()
else globals.nt2_rejoin_local_world_v1=assert(save_codec.encode_saved(globals.nt2_rejoin_local_world_v1)) end
local case=os.getenv('NT2_CF_CASE') or 'known';local known=tonumber(os.getenv('NT2_CF_KNOWN') or '1');
if case=='absent' then globals={} end
if case=='retired' then globals.nt2_rejoin_local_world_v1_offline='1' end
if case=='malformed' then globals.nt2_rejoin_local_world_v1=string.rep('x',1025) end
if case=='waiting' then s.components[1].enabled=false;s.components[2].wait_for_kill_flag_on_death=true;s.components[3]={name='nt2_g3_recovery',value_string='101---'};s.poly=true;s.effect=true end
EntityGetName=function() return '' end
local unused=1;local bootstrap='new:shared:1:999';local labels={};local clicked=nil
ModLuaFileAppend=function() end
GameGetWorldStateEntity=function() return ready and 42 or 0 end
GlobalsGetValue=function(k,f) assert(globals_ready,'Globals not loaded yet');if os.getenv('NT2_CF_CASE')=='very-late' and s.frame<150 then return f end;return globals[k] or f end
GlobalsSetValue=function(k,v) assert(globals_ready);writes=writes+1;globals[k]=v end
StatsGetValue=function() assert(stats_ready,'Stats not loaded yet');return s.frame<10 and '0' or '424242' end
SetWorldSeed=function() seeds=seeds+1;error('Continue must never reseed') end
ModTextFileGetContent=function() return bootstrap end
ModTextFileSetContent=function() error('mod-init-only operation on Continue') end
GuiText=function(_,_,_,text) labels[#labels+1]=text end
GuiButton=function(_,id) return clicked==id end
local native={}
native.nt2_g3_startup_prepare=function() prepares=prepares+1;error('Continue must not call new-world preparation') end
native.nt2_g3_saved_lookup=function(record,size) assert(size<=1024 and record:sub(1,1)=='{','lookup must receive decoded JSON');lookups=lookups+1;return known end
native.nt2_g3_resume_decision=function() return 3 end
local request_contexts={}
native.nt2_g3_resume=function(_,_,_,ctx) requests=requests+1;request_contexts[tostring(ctx)]=true;return -3 end
for _,name in ipairs({'prepare','world_policy','seed_accepted','seed_known','world_loaded','status','heartbeat_diagnostic','close','report','move','peers','social_lease','social_next','social_terminal','association','offline_choice'}) do native['nt2_g3_'..name]=function() return 0 end end
ffi.load=function() return native end
dofile=function(path) return raw(repo..'/'..path:gsub('^mods/','mod/')) end
raw(entry)
-- Continue path never manually calls OnModPreInit or the generation callback.
if OnWorldInitialized then OnWorldInitialized() end
OnPlayerSpawned(99)
for i=1,3 do s.frame=i;OnWorldPreUpdate();OnWorldPostUpdate() end
ready=true
for i=4,5 do s.frame=i;OnWorldPreUpdate();OnWorldPostUpdate() end
globals_ready=true;stats_ready=true
for i=6,170 do s.frame=i;OnWorldPreUpdate();OnWorldPostUpdate() end
if baseline then assert(lookups==0 and s.components[1].enabled and #labels==0,'baseline should silently discard Continue');print('CF01_NEGATIVE missing_preinit=true later_valid_metadata=true silent_offline=true');return end
if case=='absent' or case=='retired' or case=='malformed' or known==0 then
 assert(s.components[1].enabled and writes==0 and prepares==0 and seeds==0 and requests==0,'ordinary/foreign/retired save altered')
 print('CF01_ENTRY_PASS case='..case..' known='..known..' ordinary_offline=true no_retirement=true');return
end
if known==5 then assert(s.components[1].enabled and writes==1 and globals.nt2_rejoin_local_world_v1_offline=='1');print('CF01_ENTRY_PASS trusted_explicit_offline=true');return end
if known==3 then assert(not s.components[1].enabled and writes==0 and requests==0 and table.concat(labels,'|'):find('unavailable',1,true));print('CF01_ENTRY_PASS terminal_held=true no_implicit_offline=true');return end
if known==2 then local count=0;for _ in pairs(request_contexts) do count=count+1 end;assert(count==1);assert(not s.components[1].enabled and requests>0 and seeds==0 and prepares==0 and writes==0);print('CF01_ENTRY_PASS recorded_app_intent=true single_request=true');return end
assert(lookups>0 and s.components[1].enabled==false and table.concat(labels,'|'):find('saved world',1,true),'actual entry did not surface held choice')
assert(writes==0 and seeds==0 and prepares==0,'Continue changed bootstrap/save/seed or consumed New Game')
clicked=9141;s.frame=171;OnWorldPostUpdate();clicked=nil;OnWorldPreUpdate();assert(requests==1,'one deliberate original request')
if case=='stale' then
 OnWorldInitialized();OnPlayerSpawned(99);for i=172,175 do s.frame=i;OnWorldPreUpdate();OnWorldPostUpdate() end;assert(not s.components[1].enabled and requests==1);clicked=9141;OnWorldPostUpdate();clicked=nil;OnWorldPreUpdate();assert(requests==2,'previous chosen state/context leaked into next load')
end
print('CF01_ENTRY_PASS no_preinit=true late_world_globals_stats=true held_choice=true single_rejoin=true save_writes=0 reseeds=0 new_game_prepare=0')
