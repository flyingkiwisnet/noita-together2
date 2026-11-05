-- Exact nine owner callback timelines; native results remain explicit hypotheses.
local repo=assert(os.getenv('NT2_ST1_REPO'));local raw=dofile
local timeline=raw(assert(os.getenv('NT2_ST1_TIMELINE')))
local mode=assert(os.getenv('NT2_ST1_HYPOTHESIS'));local continued=timeline.case=='cold-continue'
raw(repo..'/apps/desktop/tools/mod-protection-stubs.lua');local s=NT2_FIXTURE;local ffi=require('ffi')
local world,seed_calls,prepares,writes=0,0,0,0
local globals={nt2_seed_loot_policy_v1='individual:0'}
local bootstrap='new:shared:1:'..timeline.prior_seed
GameGetWorldStateEntity=function() return world end
StatsGetValue=function() return timeline.seed end
GlobalsGetValue=function(k,f) if world==0 then return f end;return globals[k] or f end
GlobalsSetValue=function(k,v) assert(world>0,'Unresolved world0 metadata write');writes=writes+1;globals[k]=v end
ModTextFileGetContent=function() return bootstrap end
ModTextFileSetContent=function(_,v) bootstrap=v end
ModLuaFileAppend=function() end
EntityGetName=function() return '' end
ModGetActiveModIDs=function() return {'noita_together2'} end
GameIsBetaBuild=function() return false end
SetWorldSeed=function(value) assert(not continued and mode=='ready' and value==tonumber(timeline.seed));seed_calls=seed_calls+1 end
local clicked=false;GuiButton=function(_,id) return clicked and id==9021 end
local native={};local context=string.rep('A',16)
native.nt2_g3_startup_prepare=function(out,token)
 prepares=prepares+1;if continued then assert(mode=='preinit-refused');return -2 end
 if mode=='throw' then error('explicit injected hypothesis; not measured native failure') end
 if mode~='ready' then return mode=='pending' and -3 or 0 end
 out[0]=tonumber(timeline.seed);ffi.copy(token,context,16);return 1
end
native.nt2_g3_prepare=function(out,token) out[0]=tonumber(timeline.seed);ffi.copy(token,context,16);return 1 end
native.nt2_g3_world_policy=function() return 1 end
native.nt2_g3_seed_known=function() return 1 end
native.nt2_g3_world_loaded=function() return 1 end
native.nt2_g3_report=function() return 1 end
native.nt2_g3_status=function() return 0 end -- no Start/authority in owner tests
for _,name in ipairs({'heartbeat_diagnostic','close','move','peers','social_lease','social_next','social_terminal','association','offline_choice','resume','resume_decision'}) do native['nt2_g3_'..name]=function() return 0 end end
native.nt2_g3_saved_lookup=function() return 4 end
ffi.load=function() return native end
dofile=function(path)
 if path:find('/g3/native_owner.lua',1,true) or path:find('/g3/social_native.lua',1,true) then return {new=function() return {tick=function() end,close=function() end} end} end
 return raw(repo..'/'..path:gsub('^mods/','mod/'))
end
raw(repo..'/mod/noita_together2/init-g3-seed-loot.lua')
if not continued or mode=='preinit-refused' then OnModPreInit() end
local owner='owner-a';local marker='{"version":1,"local_world":"11111111-1111-4111-8111-111111111111","bundle_digest":"'..string.rep('a',64)..'","actual_seed":"'..timeline.seed..'","loot":"shared:0:'..timeline.seed..'"}'
local waiting=dofile('mods/noita_together2/files/startup/waiting_candidate.lua').new({
 current_owner=function() return continued and nil or owner end,
 snapshot=function(id,value) if mode~='ready' or s.frame<timeline.spawn_frame+90 then return nil end;return {kind='fresh-waiting',owner=id,seed=value,loot='shared:0:'..value,record=marker} end,
 park=function(record,value,loot) if record==marker and value==timeline.seed and loot=='shared:0:'..value then return 'await-canonical-waiting' end end,
 choose_offline=function() owner=nil;return true end,
})
for _,event in ipairs(timeline.events) do
 world=event.world;s.frame=event.frame
 if event.event=='seed_boundary' then OnMagicNumbersAndWorldSeedInitialized()
 elseif event.event=='world_initialized' then OnWorldInitialized();waiting:world_initialized()
 elseif event.event=='spawn_before_marker' then OnPlayerSpawned(99);waiting:spawn()
 elseif event.event=='preupdate' then OnWorldPreUpdate();waiting:tick()
 elseif event.event=='postupdate' then OnWorldPostUpdate() end
 if continued then assert(globals.nt2_seed_loot_policy_v1=='individual:0','Early update overwrote restored loot from stale bootstrap') end
end
for frame=1,650 do
 s.frame=timeline.spawn_frame+frame-1;OnWorldPreUpdate();waiting:tick();OnWorldPostUpdate()
 assert(not s.components[1].enabled,'Unprotected intermediate frame '..frame)
 assert(s.enabled[5] and s.enabled[6] and not s.enabled[7],'Inventory component state changed')
end
if continued then assert(seed_calls==0 and prepares==(mode=='preinit-refused' and 1 or 0) and writes==0) end
if mode=='ready' then
 assert(waiting.state=='saved-waiting' and globals.nt2_waiting_intent_v1,'Source prototype failed to persist waiting marker before Start')
 assert(not marker:find('proof',1,true) and not marker:find('refresh',1,true))
 -- Restored saved waiting intent parks only; no fresh preparation or authority.
 local restored=dofile('mods/noita_together2/files/startup/waiting_candidate.lua').new({current_owner=function() return nil end,snapshot=function() error('Continue requested a fresh bundle') end,park=function(record,value,loot) assert(record==marker and value==timeline.seed);return 'await-canonical-waiting' end,choose_offline=function() return true end})
 restored:world_initialized();restored:spawn();assert(restored.state=='parked');restored:offline();assert(restored.state=='offline');restored:tick();assert(restored.state=='offline')
end
clicked=true;OnWorldPostUpdate();clicked=false;assert(s.components[1].enabled,'Explicit offline escape failed')
print('ST1_EVIDENCE_PASS id='..timeline.id..' hypothesis='..mode..' original_release='..timeline.observed_release..' corrected_held_frames=650 seed_calls='..seed_calls..' candidate_waiting='..waiting.state..' actual_native_reason_unknown=true')
