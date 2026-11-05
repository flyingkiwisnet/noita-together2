-- Real selected Lua entry and local-world consumer on the original Lua runtime.
-- Engine/native/canonical facts are injected; this never executes Noita.
local repo=assert(os.getenv('NT2_ST1_REPO'))
local case=assert(os.getenv('NT2_ST1_CASE'))
local raw=dofile;raw(repo..'/apps/desktop/tools/mod-protection-stubs.lua')
local s=NT2_FIXTURE;local ffi=require('ffi')
local prepares,seed_calls,resumes,lookups,writes,closed,consumed=0,0,0,0,0,0,0
local bootstrap='saved';local clicked;local labels={};local record='{"local_world":"11111111-1111-1111-1111-111111111111"}'
local codec=raw(repo..'/mod/noita_together2/files/rejoin/local_world.lua')
local encoded=assert(codec.encode_saved(record))
local key='nt2_rejoin_local_world_v1';local globals={nt2_seed_loot_policy_v1='shared:1:424242'}
local continue_case=case:find('continue',1,true) or case=='offline-then-late' or case=='foreign' or case:find('offline-marker',1,true) or case=='malformed' or case=='partial' or case=='seed-mismatch' or case=='ordinary' or case=='saved-before-start'
local associated=case:find('continue',1,true) or case=='offline-then-late' or case=='foreign' or case:find('offline-marker',1,true) or case=='partial' or case=='seed-mismatch'
local function metadata()
 if associated then
  local after=case=='continue-late150' and 150 or case=='continue-late500' and 500 or case=='offline-then-late' and 300 or 0
  if s.frame<after then return {} end
  return {[key]=encoded,[key..'_seed']=case=='partial' and s.frame<400 and '' or '424242',[key..'_loot']='shared:1:424242',[key..'_offline']=case:find('offline-marker',1,true) and '1' or '0'}
 elseif case=='malformed' then return {[key]='not a complete marker'} end
 return {}
end
GameGetWorldStateEntity=function() return case=='continue-late500' and s.frame<4 and 0 or 42 end
GlobalsGetValue=function(k,f) local current=metadata();return current[k] or globals[k] or f end
GlobalsSetValue=function(k,v) writes=writes+1;globals[k]=v end
StatsGetValue=function() return (case=='seed-mismatch' or case=='offline-marker-mismatch') and '999' or '424242' end
ModGetActiveModIDs=function() return {'noita_together2'} end
GameIsBetaBuild=function() return false end
ModLuaFileAppend=function() end
ModTextFileGetContent=function() return bootstrap end
ModTextFileSetContent=function(_,v) bootstrap=v end
SetWorldSeed=function(v) assert(not continue_case,'Continue reseeded');assert(v==424242);seed_calls=seed_calls+1 end
EntityGetName=function() return '' end
GuiText=function(_,_,_,v) labels[#labels+1]=v end
GuiButton=function(_,id) return clicked==id end
local native={}
local token=string.rep('A',16)
local function current(ctx) return ctx and ffi.string(ctx,16)==token end
native.nt2_g3_startup_prepare=function(out,ctx)
 prepares=prepares+1
 if case=='throw' then error('injected startup exception') end
 if case=='lost-reply' then consumed=consumed+1;return 0 end
 local status=({refused=0,minus2=-2,minus3=-3,['app-late']=0})[case]
 if status then return status end
 out[0]=424242;ffi.copy(ctx,token,16);return 1
end
native.nt2_g3_prepare=function(out,ctx) if case=='social-refused' then return 0 end;out[0]=424242;ffi.copy(ctx,token,16);return 1 end
native.nt2_g3_world_policy=function() return case=='policy-refused' and 0 or 5 end
native.nt2_g3_seed_known=function(ctx) return current(ctx) and 1 or 0 end
native.nt2_g3_world_loaded=function(ctx) return current(ctx) and 1 or 0 end
native.nt2_g3_report=function() return 1 end
native.nt2_g3_status=function(ctx) return current(ctx) and (((case=='canonical-release' or (case=='continue-release' or case=='continue-social-cold')) and s.frame>=300 and s.frame<400) or case=='retained-world' and s.frame>=100) and 2 or 0 end
if case=='continue-social-cold' then native.nt2_g3_status=function(ctx) return current(ctx) and (s.frame<300 and 1 or s.frame<400 and 2 or 0) or 0 end end -- Real held status is1, not disconnected0.
native.nt2_g3_saved_lookup=function() lookups=lookups+1;return case=='foreign' and 0 or case=='continue-unknown' and 4 or (case=='continue-intent' or (case=='continue-release' or case=='continue-social-cold')) and 2 or 1 end
native.nt2_g3_resume_decision=function() return 3 end
native.nt2_g3_resume=function(_,_,_,ctx) resumes=resumes+1;if (case=='continue-release' or case=='continue-social-cold') then ffi.copy(ctx,token,16);return 1 end;return -3 end
native.nt2_g3_close=function() closed=closed+1;return 1 end
for _,name in ipairs({'heartbeat_diagnostic','move','peers','social_lease','social_next','social_terminal','association','offline_choice'}) do native['nt2_g3_'..name]=function() return 0 end end
local social_cold_calls=0;local cold_social=setmetatable({nt2_g3_resume=function(...) social_cold_calls=social_cold_calls+1;if social_cold_calls<4 then return 0 end;return native.nt2_g3_resume(...) end},{__index=native})
ffi.load=function(path)
 if case=='native-unavailable' or case=='app-late' and s.frame<200 then error('injected loader unavailable') end
 if case=='continue-social-cold' and path:find('social',1,true) then return cold_social end
 return native
end
dofile=function(path)
 if path:find('/g3/native_owner.lua',1,true) or path:find('/g3/social_native.lua',1,true) then return {new=function() return {tick=function() end,close=function() end} end} end
 return raw(repo..'/'..path:gsub('^mods/','mod/'))
end
local entry=os.getenv('NT2_ST1_ENTRY') or repo..'/mod/noita_together2/init-g3-seed-loot.lua'
raw(entry)
if not continue_case then OnModPreInit();OnMagicNumbersAndWorldSeedInitialized() end
if case=='saved-before-start' then s.components[1].enabled=false;s.components[2].wait_for_kill_flag_on_death=true;s.components[3]={name='nt2_g3_recovery',value_string='101---'};s.poly=true;s.effect=true end
OnWorldInitialized();OnPlayerSpawned(99)
local held,free=0,0;local expected_inventory={s.enabled[5],s.enabled[6],s.enabled[7]}
local function assert_frame()
 local allowed=case=='offline-marker' or (case=='explicit-offline' or case=='offline-then-late' or case=='continue-offline') and s.frame>=200 or (case=='canonical-release' or (case=='continue-release' or case=='continue-social-cold')) and s.frame>=300 and s.frame<400 or case=='retained-world' and s.frame>=100 and s.frame<250
 -- Cold secondary readiness can shift the matching inspection sample by a
 -- frame. Require real release within the canonical window, never outside it.
 if case=='continue-social-cold' then allowed=s.frame>=300 and s.frame<400 and s.components[1].enabled end
 assert(s.components[1].enabled==not not allowed,'protection gap case='..case..' frame='..s.frame)
 if allowed then free=free+1 else held=held+1;assert(s.components[2].wait_for_kill_flag_on_death and s.poly and s.effect,'incomplete hold') end
 for i,id in ipairs({5,6,7}) do assert(s.enabled[id]==expected_inventory[i],'inventory component changed') end
end
for frame=1,650 do
 s.frame=frame
 if case=='retained-world' and frame==250 then OnModPreInit();OnWorldInitialized();OnPlayerSpawned(99) end
 if (case=='explicit-offline' or case=='offline-then-late' or case=='continue-offline') and frame==200 then clicked=case=='continue-offline' and 9142 or 9021 end
 OnWorldPreUpdate();OnWorldPostUpdate();clicked=nil
 assert_frame()
 if case=='continue-early' and frame==200 then clicked=9141;OnWorldPostUpdate();clicked=nil end
end
assert(continue_case and seed_calls==0 and prepares==0 or not continue_case,'Continue consumed a fresh grant')
if case~='canonical-release' and case~='policy-refused' then assert(seed_calls<=1) end
if case=='lost-reply' then assert(consumed==1 and prepares==1,'uncertain consumption replayed') end
if case=='continue-early' then assert(resumes>0) end
if case=='continue-offline' then assert(writes==1 and globals[key..'_offline']=='1' and resumes==0) end
if case=='ordinary' or case=='malformed' or case=='seed-mismatch' then assert(writes==0 and prepares==0 and seed_calls==0 and resumes==0) end
if case=='explicit-offline' then local before=lookups;OnWorldInitialized();s.frame=700;OnWorldPreUpdate();assert(lookups==before) end
assert(#labels>0 or case=='foreign' or case=='offline-marker','no actionable UI')
if case=='continue-social-cold' then assert(social_cold_calls>=4 and free>0 and free<=100,'Secondary loading delay/recovery: calls='..social_cold_calls..' free='..free) end
print('ST1_PASS case='..case..' frames=650 held='..held..' free='..free..' prepare='..prepares..' seed='..seed_calls..' consumed='..consumed..' synthetic_engine=true')
