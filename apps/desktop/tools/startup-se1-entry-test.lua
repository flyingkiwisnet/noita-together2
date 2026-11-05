-- Production settings+init+bundle+loot+saved consumer; engine/canonical facts injected.
local repo=assert(os.getenv('NT2_SE1_REPO'));local out=assert(os.getenv('NT2_SE1_OUT'));local raw=dofile
local total=0
local function scenario(name,o)
 local filter=os.getenv('NT2_SE1_FILTER');if filter and filter~='' and not name:find(filter,1,true) then return end
 o=o or {};raw(repo..'/apps/desktop/tools/mod-protection-stubs.lua');local s=NT2_FIXTURE
 local ffi=require('ffi');local registry={};local virtual={['mods/noita_together2/files/loot/run-policy.txt']='new:shared:1:999'}
 local world,claims,nonces,seeds,resumes,closes,mode=0,0,0,0,0,{},nil
 local token=string.rep('A',16);local saved={};local printlines={};local reports={};local report_attempts={};local actual='424242';local status=1;local clicked=false;local engine_clock=0
 local function getappends(path) local r={} for i,v in ipairs(registry[path] or {}) do r[i]=v end;return r end
 ModLuaFileGetAppends=getappends
 ModLuaFileAppend=function(path,from) registry[path]=registry[path] or {};table.insert(registry[path],from) end
 ModTextFileGetContent=function(p) return virtual[p] or '' end;ModTextFileSetContent=function(p,v) virtual[p]=v end
 GameGetWorldStateEntity=function() return world end;StatsGetValue=function() return actual end
 -- Elapsed engine time is independent of GameGetFrameNum, which the saved-entry
 -- and replacement scenarios deliberately reset/reorder.
 GameGetRealWorldTimeSinceStarted=function() return engine_clock end
 GlobalsGetValue=function(k,f) return world>0 and saved[k] or f end;GlobalsSetValue=function(k,v) assert(world>0);saved[k]=v end
 EntityGetName=function() return '' end;ModGetActiveModIDs=function() return (o.report_samples_changed and s.frame>0 or o.release_mod_change and s.frame>=200) and {'noita_together2','changed_mod'} or {'noita_together2'} end;GameIsBetaBuild=function() return false end
 SetWorldSeed=function(value) assert(mode=='new' and value==424242,'non-fresh reseed');seeds=seeds+1 end
 GuiButton=function(_,id) return clicked and id==9021 end
 local original_print=print;print=function(line) assert(not tostring(line):find('credential',1,true));printlines[#printlines+1]=line end
 local native={}
 native.nt2_g3_startup_nonce=function(c,n) assert(n==16);nonces=nonces+1;ffi.fill(c,16,nonces+40);token=string.rep(string.char(nonces+64),16);return 1 end
 native.nt2_g3_startup_bundle=function(b,n,c,budget)
  claims=claims+1;assert(mode=='new','Continue fresh claim');assert(budget>=100 and budget<=6000)
  if o.fail then return 0 end
  ffi.fill(b,n);ffi.copy(c,token,16);local w=ffi.cast('uint32_t*',b)
  w[0]=0x3247544e;w[1]=9;w[2]=16;w[3]=1;w[5]=o.policy==2 and 0 or 424242;w[6]=o.policy or 5;w[7]=1;w[8]=1000;w[9]=2
  w[44]=o.provisional and 1 or 0
  ffi.copy(b+48,token,16);ffi.copy(b+64,o.provisional and string.rep('B',16) or token,16);ffi.copy(b+80,string.rep('d',32),32);ffi.copy(b+112,string.rep('c',32),32);ffi.copy(b+192,'{}',2);return 1
 end
 native.nt2_g3_startup_diagnostic=function(b,n) ffi.fill(b,n);local w=ffi.cast('uint32_t*',b);w[0]=0x3154534e;w[1]=1;w[2]=1;w[37]=10;ffi.copy(b+48,token,16);return 256 end
 native.nt2_g3_seed_known=function(c) return ffi.string(c,16)==token and 1 or 0 end
 native.nt2_g3_world_loaded=function(c) return ffi.string(c,16)==token and 1 or 0 end
 native.nt2_g3_status=function(c) return ffi.string(c,16)==token and status or 0 end
 native.nt2_g3_report=function(_,body,n,sequence) sequence=tonumber(sequence);reports[#reports+1]={sequence=sequence,body=type(body)=='string' and body or ffi.string(body,n),frame=s.frame};report_attempts[sequence]=(report_attempts[sequence] or 0)+1;if o.async_report then return report_attempts[sequence]>=3 and 1 or 0 end;return (o.report_samples or o.report_samples_changed) and #reports<4 and 0 or 1 end
 native.nt2_g3_close=function(c) closes[#closes+1]=ffi.string(c,16);return 1 end
 native.nt2_g3_resume=function(_,_,_,c) resumes=resumes+1;ffi.copy(c,token,16);return 1 end
 native.nt2_g3_saved_lookup=function() if o.fresh_intent and s.frame>=300 then return o.fresh_intent=='terminal' and 3 or o.fresh_intent=='pending' and 4 or o.fresh_intent=='unconfirmed' and 0 or o.fresh_intent=='terminal-then-offline' and (s.frame<350 and 3 or 5) or 5 end;return 2 end;native.nt2_g3_world_policy=function() return 5 end;native.nt2_g3_resume_decision=function() return 3 end
 for _,fn in ipairs({'heartbeat_diagnostic','move','peers','social_lease','social_next','social_terminal','association','offline_choice'}) do native['nt2_g3_'..fn]=function() return 0 end end
  if o.fresh_intent then native.nt2_g3_association=function(c,b,n)
  assert(ffi.string(c,16)==token,'metadata requested for wrong live context');local marker='{"kind":"nt2-world","version":1,"local_world":"11111111-1111-4111-8111-111111111111","bundle_digest":"'..string.rep('a',64)..'","actual_seed":"424242","loot":"shared:1:424242"}'
  assert(#marker<n);ffi.copy(b,marker,#marker);return #marker
 end end
 ffi.load=function() if o.lateimage and s.frame==0 then error('loader unavailable') end;return native end
 local set_component=ComponentSetValue2
 ComponentSetValue2=function(id,key,value)
  if o.async_report and s.frame>=200 and id==1 and key=='enabled' and value then
   local last=assert(reports[#reports]);assert(report_attempts[last.sequence]>=3,'release preceded asynchronous report acknowledgement')
   if o.release_mod_change then assert(last.body:find('changed_mod',1,true),'release reused different pre-scan content') end
   assert(not o.invalid_release_seed,'wrong actual seed released')
  end
  return set_component(id,key,value)
 end
 dofile=function(path)
  if path:find('/g3/native_owner.lua',1,true) or path:find('/g3/social_native.lua',1,true) then return {new=function() return {tick=function() end,close=function() end} end} end
  return raw(repo..'/'..path:gsub('^mods/','mod/'))
 end
 -- Settings and init have different _G and separate dofile/module instances.
 local env=setmetatable({_G=false},{__index=_G});env._G=env
 env.ModIsEnabled=function(id) assert(id=='noita_together2');return not o.disabled end
 if o.noappend then env.ModLuaFileAppend=function() error('settings append unavailable') end end
 env.ModSettingSet=function() error('No persisted startup flag allowed') end
 env.dofile=function(path)
  if path=='data/scripts/lib/utilities.lua' then return end
  local p=path=='data/scripts/lib/mod_settings.lua' and out..'/mod_settings.lua' or repo..'/'..path:gsub('^mods/','mod/')
  local f=assert(loadfile(p));setfenv(f,env);return f()
 end
 env.dofile('mods/noita_together2/settings.lua')
 local scopes={new=env.MOD_SETTING_SCOPE_NEW_GAME,restart=env.MOD_SETTING_SCOPE_RUNTIME_RESTART,menu=env.MOD_SETTING_SCOPE_ONLY_SET_DEFAULT,runtime=env.MOD_SETTING_SCOPE_RUNTIME}
 local function produce(kind) mode=kind;env.ModSettingsUpdate(scopes[kind]) end
 if o.menu then produce('menu');produce('runtime') end
 if not o.missing then produce(o.continue and 'restart' or 'new') end
 if o.restart_after_new then produce('restart') end
 if o.duplicate then produce('new') end
 if o.noise then produce('runtime') end
 raw(repo..'/mod/noita_together2/init-g3-seed-loot.lua')
 if o.frame then s.frame=o.frame end
 OnModPreInit();if o.duppreinit then OnModPreInit() end;OnModInit();OnModPostInit()
 OnMagicNumbersAndWorldSeedInitialized();world=42
 if o.continue then
  saved.nt2_seed_loot_policy_v1='shared:1:424242'
  -- Delayed NT2 restore is partial companion metadata, not an unrelated save
  -- with all NT2 association fields absent (which correctly restores offline).
  if o.delayed then saved.nt2_rejoin_local_world_v1_seed='424242' end
  local codec=dofile('mods/noita_together2/files/rejoin/local_world.lua')
  local metadata='{"local_world":"11111111-1111-4111-8111-111111111111"}'
  if not o.delayed then saved.nt2_rejoin_local_world_v1=assert(codec.encode_saved(metadata));saved.nt2_rejoin_local_world_v1_seed='424242';saved.nt2_rejoin_local_world_v1_loot='shared:1:424242' end
  o.metadata=metadata
 end
 local applied=not (o.continue or o.missing or o.duplicate or o.noise or o.fail or o.noappend or o.lateimage or o.restart_after_new or o.disabled)
 assert(seeds==(applied and o.policy~=2 and 1 or 0),name..' early seed count '..seeds)
 if o.earlyworldinit then OnWorldInitialized() end
 OnWorldPreUpdate();OnPlayerSpawned(99);OnWorldInitialized();OnWorldInitialized() -- duplicates/order cannot reset valid entry
 for frame=1,(o.fps and o.fps*36 or 500) do s.frame=frame;engine_clock=engine_clock+1/(o.fps or 60)
  if o.delayed and frame==150 then local codec=dofile('mods/noita_together2/files/rejoin/local_world.lua');saved.nt2_rejoin_local_world_v1=assert(codec.encode_saved(o.metadata));saved.nt2_rejoin_local_world_v1_seed='424242';saved.nt2_rejoin_local_world_v1_loot='shared:1:424242' end
  if frame==200 and not o.hold then status=2 end
  if frame==300 and o.fresh_intent then status=0 end
  if frame==200 and o.invalid_release_seed then actual='999' end
  if o.offline and frame==100 then clicked=true end
  if not(o.async_pause and frame>=200 and frame<=400) then OnWorldPreUpdate();OnWorldPostUpdate() end;clicked=false
  if frame<200 and not (o.offline and frame>=100) then assert(not s.components[1].enabled,name..' protection gap '..frame) end
  assert(s.enabled[5] and s.enabled[6] and not s.enabled[7],'inventory component mutated')
 end
 if o.continue then assert(claims==0 and nonces==0 and seeds==0 and resumes>0,'saved recovery took fresh lane');assert(saved.nt2_seed_loot_policy_v1=='shared:1:424242','saved loot overwritten') end
 if applied and not o.hold and not o.invalid_release_seed and not o.fresh_intent then assert(s.components[1].enabled,'canonical fresh release missing') end
 if o.invalid_release_seed then assert(not s.components[1].enabled,'changed actual seed released') end
 if o.fail or o.lateimage then assert(nonces<=1 and not s.components[1].enabled,'failed claim retried or released') end
 if o.hold and not o.offline and not o.fresh_intent then assert(not s.components[1].enabled,'uncertain world timer released') end
 if o.offline then assert(s.components[1].enabled,'explicit offline did not restore') end
  if o.fresh_intent then
  assert(saved.nt2_rejoin_local_world_v1,'fresh production caller did not persist marker')
  local offline=o.fresh_intent=='offline' or o.fresh_intent=='terminal-then-offline';assert(s.components[1].enabled==offline,'fresh scoped intent did not restore/protect actual Controls')
  assert(saved.nt2_rejoin_local_world_v1_offline==(offline and '1' or '0'),'fresh offline marker differs')
  assert(resumes==0,'fresh intent path created a restored/rejoin conversation')
  assert(#closes==(offline and 1 or 0),'fresh intent closed unchosen world or repeated retirement')
 end
 if o.report_samples then assert(#reports>=5,'inspection did not refresh');for i=2,4 do assert(reports[i].sequence==reports[1].sequence and reports[i].body==reports[1].body,'unacknowledged inspection original changed') end;assert(reports[#reports].sequence>reports[1].sequence and reports[#reports].body==reports[1].body,'unchanged actual inspection never produced a fresh sample') end
 if o.report_samples_changed then assert(#reports>=5);for i=2,4 do assert(reports[i].sequence==reports[1].sequence and reports[i].body==reports[1].body,'changed engine content rewrote an unacknowledged original') end;assert(reports[#reports].sequence>reports[1].sequence and reports[#reports].body~=reports[1].body,'acknowledged original prevented fresh changed engine collection') end
 if o.async_report then if not o.invalid_release_seed then assert(s.components[1].enabled,'asynchronous acknowledgement never reached eligible entry') end;assert(#reports<100,'asynchronous acknowledgement created an endless release-report loop');for i=2,#reports do if reports[i].sequence==reports[i-1].sequence then assert(reports[i].body==reports[i-1].body,'asynchronous retry rewrote original bytes') end end end
 if o.fps then local largest=0;for i=2,#reports do if reports[i-1].frame>=200 then largest=math.max(largest,(reports[i].frame-reports[i-1].frame)/o.fps) end end;assert(largest<=1.1,name..' actual callback report gap '..largest..'s lacks headroom under unchanged6s observation validity');original_print('SE1_TIMING fps='..o.fps..' largest_callback_report_gap_s='..largest..' reports='..#reports..' callbacks='..o.fps*36);end
 local before=claims
 if o.reload then raw(repo..'/mod/noita_together2/init-g3-seed-loot.lua');OnModPreInit();OnMagicNumbersAndWorldSeedInitialized();OnPlayerSpawned(99);assert(claims==before,'script reload replayed scope') end
 if o.replacement then
  local old=token;registry={};produce('new');world=0;s.frame=0;OnModPreInit();OnMagicNumbersAndWorldSeedInitialized();world=42;saved={};OnWorldPreUpdate();OnPlayerSpawned(99);OnWorldInitialized();assert(seeds==2 and nonces==2 and token~=old,'replacement entry not distinct')
  assert(closes[#closes]==old,'old owner not closed with immutable token')
  collectgarbage('collect');assert(closes[#closes]==old,'old finalizer closed replacement context')
 end
 if o.replayed then produce('new');OnModPreInit();OnPlayerSpawned(99);assert(claims==before and seeds==1 and nonces==1,'replayed scope rearmed fresh') end
 if o.retainedmissing then
  registry={};world=43;saved.nt2_seed_loot_policy_v1='individual:0';OnWorldPreUpdate();assert(saved.nt2_seed_loot_policy_v1=='individual:0' and claims==before,'stale context wrote loaded loot')
 end
 if o.fail or o.lateimage then local before_nonce=nonces;OnModPreInit();OnPlayerSpawned(99);assert(claims==before and nonces==before_nonce,'late duplicate escaped original failure') end
 if applied then local joined=table.concat(printlines,'\n');assert(joined:find('|event=settings_new|',1,true) and joined:find('|event=claim|',1,true),'buffered scope/claim observations missing') end
 print=original_print;total=total+1;print('SE1_PASS '..name..' claims='..claims..' nonce='..nonces..' seeds='..seeds..' resume='..resumes..' injected_engine=true')
end
scenario('fresh-with-old-save',{menu=true,duppreinit=true,frame=17777})
scenario('fresh-intent-app-offline',{fresh_intent='offline'})
scenario('fresh-intent-held-app-offline',{fresh_intent='offline',hold=true})
scenario('fresh-intent-host-terminal',{fresh_intent='terminal'})
scenario('fresh-intent-held-exclusion',{fresh_intent='terminal',hold=true})
scenario('fresh-intent-terminal-then-app-offline',{fresh_intent='terminal-then-offline'})
scenario('fresh-intent-pending-protects',{fresh_intent='pending'})
scenario('fresh-intent-unconfirmed-protects',{fresh_intent='unconfirmed'})
scenario('foreground-clock30fps',{fps=30})
scenario('foreground-clock60fps',{fps=60})
scenario('provisional-distinct-world-context',{provisional=true})
scenario('same-inspection-fresh-sample-after-reliable-ack',{hold=true,report_samples=true})
scenario('changed-inspection-retains-unacknowledged-original',{hold=true,report_samples_changed=true})
scenario('asynchronous-every-sequence-ack-release',{async_report=true})
scenario('asynchronous-Continue-ack-release',{async_report=true,continue=true})
scenario('asynchronous-release-changed-content',{async_report=true,release_mod_change=true})
scenario('asynchronous-release-wrong-seed-protected',{async_report=true,invalid_release_seed=true})
scenario('asynchronous-release-fresh-scan-after-pause',{async_report=true,async_pause=true})
scenario('fresh-individual-preserves-engine-seed',{policy=2})
scenario('fresh-shared-loot-disabled',{policy=1})
scenario('world-initialized-before-update',{earlyworldinit=true})
scenario('Continue-delayed-marker',{continue=true,delayed=true,frame=22222})
scenario('duplicate-scope',{duplicate=true,hold=true})
scenario('abandoned-new-then-restart',{restart_after_new=true,hold=true})
scenario('menu-runtime-only',{menu=true,missing=true,hold=true})
scenario('runtime-after-new',{noise=true,hold=true})
scenario('missing-scope',{missing=true,hold=true})
scenario('unsupported-settings-append',{noappend=true,hold=true})
scenario('disabled-mod-settings-cannot-arm',{disabled=true,hold=true})
scenario('failed-complete-claim',{fail=true})
scenario('late-image-no-retry',{lateimage=true})
scenario('uncertain-held-past500',{hold=true})
scenario('explicit-offline',{offline=true,hold=true})
scenario('consumed-script-reload',{reload=true})
scenario('replacement-registry-reset',{replacement=true})
scenario('replacement-with-stale-registry-held',{replayed=true})
scenario('retained-missing-scope-no-stale-loot',{retainedmissing=true})
print('SE1_TOTAL '..total..' passed; real shipped settings framework; engine/native/canonical facts injected')
