-- Selected production entry + real saved-world module; engine/service facts are
-- injected. No claim of real engine save persistence or canonical authorization.
local repo=assert(os.getenv('NT2_ST1_REPO'));local case=assert(os.getenv('NT2_WAITING_CASE'));local raw=dofile
raw(repo..'/apps/desktop/tools/mod-protection-stubs.lua');local s=NT2_FIXTURE;local ffi=require('ffi')
local world,continued,proof,start,offline=0,false,false,false,false
local globals={};local writes={};local seed_calls,prepares,resumes=0,0,0;local current_seed='424242'
local marker='{"kind":"nt2-waiting","version":1,"local_world":"11111111-1111-4111-8111-111111111111","bundle_digest":"'..string.rep('a',64)..'","actual_seed":"424242","loot":"shared:0:424242"}'
GameGetWorldStateEntity=function() return world end;StatsGetValue=function() return current_seed end
GlobalsGetValue=function(k,f) if world==0 then return f end;return globals[k] or f end
GlobalsSetValue=function(k,v) assert(world>0,'world-zero write');writes[#writes+1]=k;globals[k]=v end
local bootstrap='';ModTextFileGetContent=function() return bootstrap end;ModTextFileSetContent=function(_,v) bootstrap=v end
ModLuaFileAppend=function() end;EntityGetName=function() return '' end;ModGetActiveModIDs=function() return {'noita_together2'} end;GameIsBetaBuild=function() return false end
SetWorldSeed=function(v) assert(not continued and v==424242);seed_calls=seed_calls+1 end
local clicked=false;GuiButton=function(_,id) return clicked and id==9142 end
local native={};local first=string.rep('A',16);local second=string.rep('B',16)
native.nt2_g3_startup_prepare=function(out,context) assert(not continued,'Continue attempted fresh preparation');prepares=prepares+1;out[0]=424242;ffi.copy(context,first,16);return 1 end
native.nt2_g3_prepare=native.nt2_g3_startup_prepare
native.nt2_g3_world_policy=function() return 1 end;native.nt2_g3_seed_known=function() return 1 end;native.nt2_g3_world_loaded=function() return 1 end
native.nt2_g3_report=function() return 1 end;native.nt2_g3_status=function() return start and 2 or 1 end
native.nt2_g3_association=function(context,out,capacity) assert(capacity==1024);if not proof then return 0 end;ffi.copy(out,marker,#marker);return #marker end
native.nt2_g3_saved_lookup=function(record,size) assert(ffi.string(record,size)==marker);return case=='missing-proof' and 4 or 2 end
native.nt2_g3_resume=function(id,seed,size,context) assert(continued and ffi.string(seed,size)=='424242');resumes=resumes+1;ffi.copy(context,second,16);return 1 end
native.nt2_g3_resume_decision=function() return 1 end;native.nt2_g3_offline_choice=function() offline=true;return 1 end
for _,name in ipairs({'heartbeat_diagnostic','close','move','peers','social_lease','social_next','social_terminal'}) do native['nt2_g3_'..name]=function() return 0 end end
ffi.load=function() return native end
dofile=function(path) if path:find('/g3/native_owner.lua',1,true) or path:find('/g3/social_native.lua',1,true) then return {new=function() return {tick=function() end,close=function() end} end} end;return raw(repo..'/'..path:gsub('^mods/','mod/')) end
local function load(fresh)
 raw(repo..'/mod/noita_together2/init-g3-seed-loot.lua');if fresh then OnModPreInit() end
 OnMagicNumbersAndWorldSeedInitialized();world=1;OnWorldInitialized();OnPlayerSpawned(99)
end
load(true)
for frame=1,120 do s.frame=frame;OnWorldPreUpdate();OnWorldPostUpdate();assert(not s.components[1].enabled);assert(not globals.nt2_rejoin_local_world_v1,'Marker advertised before protected proof') end
if case~='quit-before-enrollment' then proof=true end
for frame=121,650 do s.frame=frame;OnWorldPreUpdate();OnWorldPostUpdate();assert(not s.components[1].enabled);assert(s.enabled[5] and s.enabled[6] and not s.enabled[7]) end
if proof then assert(globals.nt2_rejoin_local_world_v1 and writes[#writes]=='nt2_rejoin_local_world_v1','Waiting record not committed last') else assert(not globals.nt2_rejoin_local_world_v1) end
assert(seed_calls==1 and prepares==2 and resumes==0)
continued=true;world=0;if case=='wrong-seed' then current_seed='424243' end;load(false)
for frame=651,950 do s.frame=frame;OnWorldPreUpdate();OnWorldPostUpdate();assert(not s.components[1].enabled,'Continue released before canonical Start');assert(seed_calls==1 and prepares==2) end
if case=='ready-start' then
 assert(resumes>=2,'Primary/social did not share saved-world contact');start=true
 for frame=951,960 do s.frame=frame;OnWorldPreUpdate();OnWorldPostUpdate() end
 assert(s.components[1].enabled,'Existing canonical status did not release owned hold')
elseif case=='offline' then clicked=true;OnWorldPostUpdate();clicked=false;assert(offline and s.components[1].enabled);assert(globals.nt2_rejoin_local_world_v1_offline=='1')
else assert(resumes==0,'Unverified/mismatched waiting world reached recovery') end
assert(seed_calls==1 and prepares==2,'Saved waiting path reconsumed/reseeded');assert(s.enabled[5] and s.enabled[6] and not s.enabled[7],'Inventory flags changed')
print('PASS selected waiting bridge '..case..': protected before proof/Start; same saved seed and loadout; no fresh grant; synthetic engine/service facts')


