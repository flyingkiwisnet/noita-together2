-- Isolated Lua5.1; all engine facts are injected, never starts Noita.
local repo=assert(os.getenv('NT2_ST1_REPO'));local out=assert(os.getenv('NT2_ST1_OUT'))
local source=assert(os.getenv('NT2_ST1_SOURCE'));local build='nt2-st1-v2-20261017-r1'
local passed=0
local function scenario(name,options)
 local o=options or {};local frame,world,mark,writes,prints=10000,o.unready and 0 or 42,nil,0,0
 local lines={};local e=setmetatable({},{__index=_G});e._G=e
 e.dofile=function() return {build=o.badbuild and 'wrong' or build,source=source,run=string.rep('a',32),label=o.label or 'cold-fresh',marker_write=o.write~=false} end
 e.GameGetFrameNum=function() return frame end;e.GameGetRealWorldTimeSinceStarted=function() return frame/60 end
 e.GameGetWorldStateEntity=function() return world end;e.StatsGetValue=function() return o.numericseed and 424242 or '18446744073709551615' end
 e.GameIsDailyRun=nil;e.GameIsDailyRunOrDailyPracticeRun=function() error('private-error-must-not-export') end;e.GameIsModeFullyDeterministic=function() return '' end
 e.GlobalsGetValue=function(key,fallback) if key=='nt2_st1_disposable_marker_v1' then return mark or fallback end;return 'private-association-body' end
 e.GlobalsSetValue=function(key,value) assert(key=='nt2_st1_disposable_marker_v1');writes=writes+1;mark=value;if o.nonNil then return true end end
 e.EntityGetFirstComponentIncludingDisabled=function(_,kind) assert(kind=='ControlsComponent' or kind=='WorldStateComponent');return 1 end
 e.ComponentGetValue2=function(_,field) if field=='time_total' then return frame/60 end;return false end
 e.ModMagicNumbersFileAdd=function() end
 for _,key in ipairs({'SetWorldSeed','ComponentSetValue2','EntitySetTransform','EntityLoad','EntityKill'}) do e[key]=function() error('gameplay mutation') end end
 e.print=function(line)
  prints=prints+1;assert(#line<2048 and not line:find('private',1,true))
  if o.throwprint and frame<10550 then error('silent logger') end
  if frame>=(o.delay or 10000) then lines[#lines+1]=line end
 end
 local f=assert(loadfile(repo..'/apps/desktop/tools/startup-diagnostic/mod/nt2_st1_probe/init.lua'));setfenv(f,e);f()
 e.OnModPreInit();e.OnModInit();e.OnModPostInit();e.OnMagicNumbersAndWorldSeedInitialized();e.OnWorldInitialized();e.OnPlayerSpawned(99)
 for i=1,700 do frame=10000+i;if o.reset and i==500 then lines={} end;if o.overflow then world=42+i end;e.OnWorldPreUpdate();e.OnWorldPostUpdate() end
 local joined=table.concat(lines,'\n');local expected=not o.badbuild and not o.unready and o.label~='cold-continue' and o.write~=false
 assert(writes==(expected and 1 or 0),'fresh permission '..name)
 assert(joined:find('|daily=unsupported|daily_practice=error|deterministic=empty|',1,true),'distinct statuses')
 assert(joined:find('|seed='..(o.numericseed and 'number_not_string' or 's18446744073709551615')..'|',1,true),'exact seed/status')
 if not o.overflow then
  assert(joined:find('|seq=1|event=script_loaded|frame=n10000.000000|clock=n166.666667|',1,true),'original early event lost '..name)
  for _,event in ipairs({'preinit','init','postinit','seed_boundary','world_initialized','spawn_before_marker'}) do assert(joined:find('|event='..event..'|',1,true),event) end
  if expected then assert(joined:find('|event=probe_marker_written|',1,true),'nil/non-nil marker success') end
  assert(joined:find('|update=650',1,true),'last replay missing')
  local before=writes;world=42;mark=nil;frame=50000;e.OnWorldInitialized();e.OnPlayerSpawned(99)
  for i=1,650 do frame=50000+i;e.OnWorldPreUpdate();e.OnWorldPostUpdate() end
  assert(writes==before,'permission reused on retained world')
 else
  assert(joined:match('|dropped=[1-9]'),'overflow not explicit');assert(prints<=8192,'transport unbounded')
 end
 if o.retainedfixture then local log=assert(io.open(out..'/'..o.retainedfixture,'wb'));log:write(table.concat(lines,'\n')..'\n');log:close() end
 if o.fixture then local log=assert(io.open(out..'/'..o.fixture,'wb'));log:write(joined..'\n');log:close() end
 passed=passed+1;print('PASS '..name..' lines='..#lines..' markerWrites='..writes)
end
scenario('delayed-silent',{delay=10550,fixture='delayed.log'})
scenario('throwing-logger',{throwprint=true})
scenario('log-reset-at500',{reset=true,fixture='reset.log'})
scenario('continue-never-writes',{label='cold-continue',write=true,fixture='continue.log',retainedfixture='retained.log'})
scenario('nil-return-marker-success',{})
scenario('nonnil-return-marker-success',{nonNil=true})
scenario('first-spawn-unready',{unready=true})
scenario('bad-build-no-permission',{badbuild=true})
scenario('numeric-seed-is-not-exact',{numericseed=true})
scenario('bounded-overflow',{overflow=true,fixture='overflow.log'})
print('PASS '..passed..' isolated Lua scenarios; engine facts injected; no Noita launch')
