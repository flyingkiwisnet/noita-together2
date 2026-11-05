local repo=assert(os.getenv('NT2_ST1_REPO'));local raw=dofile
local function scenario(case)
 local values={};local world,owner,snapshot_ready,writes=0,'a',false,0;local actual='424242';local fail=case=='partial-write'
 GameGetWorldStateEntity=function() return world end
 StatsGetValue=function() return actual end
 GlobalsGetValue=function(k,f) return values[k] or f end
 GlobalsSetValue=function(k,v) if fail and k=='nt2_waiting_intent_v1' then error('injected final marker write failure') end;writes=writes+1;values[k]=v end
 dofile=function(path) return raw(repo..'/'..path:gsub('^mods/','mod/')) end
 SetWorldSeed=function() error('Waiting intent cannot reseed') end
 local record='{"version":1,"local_world":"11111111-1111-4111-8111-111111111111"}'
 local parks=0
 local adapter={current_owner=function() return owner end,snapshot=function() if not snapshot_ready then return nil end;return {kind='fresh-waiting',owner='a',seed=case=='wrong-seed' and '999' or actual,loot=case=='wrong-loot' and 'shared:0:999' or 'shared:0:'..actual,record=record} end,park=function(r,value,loot) parks=parks+1;if case~='forged' and r==record and value==actual and loot=='shared:0:'..actual then return 'await-canonical-waiting' end end,choose_offline=function() owner=nil;return true end}
 local M=dofile('mods/noita_together2/files/startup/waiting_candidate.lua');local m=M.new(adapter)
 m:tick();assert(writes==0 and parks==0);world=1;m:world_initialized();m:spawn();assert(writes==0)
 if case=='stale' then owner='b' end;snapshot_ready=true
 local ok=pcall(m.tick,m)
 if case=='partial-write' then
  assert(not ok and not values.nt2_waiting_intent_v1,'Incomplete marker became visible');local other=M.new(adapter);snapshot_ready=false;other:spawn();assert(parks==0);snapshot_ready=true;fail=false;m:tick();assert(m.state=='saved-waiting')
 elseif case=='wrong-seed' or case=='wrong-loot' or case=='stale' then assert(ok and writes==0 and parks==0)
 else
  assert(ok and m.state=='saved-waiting');local next_load=M.new(adapter);snapshot_ready=false;next_load:spawn();assert(next_load.state==(case=='forged' and 'unresolved' or 'parked'))
  next_load:offline();assert(next_load.state=='offline');local prior=writes;snapshot_ready=true;next_load:tick();assert(writes==prior,'Late snapshot after offline wrote metadata')
 end
 print('PASS waiting candidate '..case..': no seed/play authority, bound positive-hook writes and parked original only')
end
for _,case in ipairs({'stale','wrong-seed','wrong-loot','partial-write','restore-offline','forged'}) do scenario(case) end
