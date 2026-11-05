-- Original Lua runtime; all engine facts injected. Does not launch Noita.
local repo=assert(os.getenv('NT2_ST1_REPO'))
local function scenario(label,write,not_ready)
 local raw_frame,world,writes,mark=10000,not_ready and 0 or 42,0,nil
 local lines={};local e=setmetatable({},{__index=_G});e._G=e
 e.ModMagicNumbersFileAdd=function(path) assert(path=='mods/nt2_st1_probe/logging.xml') end
 e.dofile=function(path) assert(path=='mods/nt2_st1_probe/case.lua');return {run=string.rep('a',32),label=label,marker_write=write} end
 e.GameGetFrameNum=function() return raw_frame end
 e.GameGetRealWorldTimeSinceStarted=function() return raw_frame/60 end
 e.GameGetWorldStateEntity=function() return world end
 e.StatsGetValue=function() return '424242' end
 e.GlobalsGetValue=function(key,fallback)
  if key=='nt2_st1_disposable_marker_v1' then return mark or fallback end
  if key=='nt2_rejoin_local_world_v1' then return 'do-not-export-association-body' end
  error('Unexpected global read')
 end
 e.GlobalsSetValue=function(key,value) assert(key=='nt2_st1_disposable_marker_v1' and value=='st1-disposable-a');writes=writes+1;mark=value end
 e.EntityGetFirstComponentIncludingDisabled=function(_,kind) assert(kind=='ControlsComponent');return 1 end
 e.ComponentGetValue2=function(_,key) assert(key=='enabled');return false end
 for _,name in ipairs({'SetWorldSeed','ComponentSetValue2','EntitySetTransform','EntityLoad','EntityKill'}) do e[name]=function() error('Diagnostic altered gameplay') end end
 e.print=function(line) assert(#line<2048 and not line:find('do-not-export-association-body',1,true));lines[#lines+1]=line end
 local f=assert(loadfile(repo..'/apps/desktop/tools/startup-diagnostic/mod/nt2_st1_probe/init.lua'));setfenv(f,e);f()
 e.OnModPreInit();e.OnWorldInitialized();e.OnPlayerSpawned(99)
 local fresh=write and (label=='cold-fresh' or label=='fresh-after-save')
 assert(writes==(fresh and not not_ready and 1 or 0))
 for i=1,700 do raw_frame=10000+i;if i==150 then mark='st1-disposable-a' end;e.OnWorldPreUpdate();e.OnWorldPostUpdate() end
 local found=false;for _,line in ipairs(lines) do if line:find('frame=10150|relative_update=150',1,true) and line:find('marker=st1-disposable-a',1,true) then found=true end end
 assert(found,'Large raw frame suppressed world-relative observation')
 local before=writes;world=42;mark=nil;raw_frame=50000;e.OnWorldInitialized();e.OnPlayerSpawned(99)
 for i=1,650 do raw_frame=50000+i;if i==500 then mark='st1-disposable-a' end;e.OnWorldPreUpdate();e.OnWorldPostUpdate() end
 assert(writes==before,'Retained spawn inherited fresh marker permission')
 found=false;for _,line in ipairs(lines) do if line:find('frame=50500|relative_update=500',1,true) and line:find('marker=st1-disposable-a',1,true) then found=true end end
 assert(found,'Retained world-relative observation did not reset')
 print('PASS diagnostic label='..label..' configured_write='..tostring(write)..' first_world_ready='..tostring(not not_ready)..' marker_writes='..writes..' raw_frame10000/50000 restore150/500 observed; retained permission not reused')
end
scenario('cold-continue',false,false)
scenario('cold-continue',true,false) -- Defensive: even a misconfigured Continue cannot write.
scenario('cold-fresh',true,false)
scenario('fresh-after-save',true,false)
scenario('cold-fresh',true,true) -- No permission migration when first spawn was unready.
