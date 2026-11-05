-- Production recorder; engine time/mode/copy outcomes are labelled injected.
local repo=assert(os.getenv('NT2_DEBUG_REPO'));local raw=dofile
dofile=function(path)return raw(repo..'/'..path:gsub('^mods/','mod/'))end
local clock=0;GameGetRealWorldTimeSinceStarted=function()return clock end
local logger=true;ModSettingGet=function(key)if key=='noita_together2.NT_ENABLE_LOGGER'then return logger end end
local api=dofile('mods/noita_together2/files/g3/game_api.lua');local enabled=true;local reports={};local checks=0
local function check(v,s)assert(v,s);checks=checks+1 end
local bridge={nt2_g3_debug_mode=function()return enabled and 1 or 0 end,nt2_g3_debug_publish=function(_,bytes,size)check(type(bytes)=='string'and #bytes==size and size<=4096,'bounded copy');reports[#reports+1]=bytes;return 1 end}
local function observe(d,frame,playing,copied,roster,poses,ghosts)clock=frame/60;d:observe(frame,playing,copied,roster,poses,ghosts)end
local d=api.debug(bridge,{},nil)
observe(d,0,true,864,1,1,1);observe(d,60,true,864,1,0,0)
check(d.first_loss and d.first_loss.frame==60 and d.first_loss.kind==1,'first missing presentation retained despite eligible roster')
observe(d,120,true,864,1,1,1);check(d.first_loss.frame==60,'recovery cannot replace first loss');check(reports[#reports]:find('"kind":2',1,true),'recovery event exported')
for i=3,30 do observe(d,i*60,true,864,1,1,1)end
check(#d.recent==8 and d.truncated,'bounded recent context');check(d.first_loss.frame==60,'first event survives ring rotation');check(#d.before_loss==1 and d.before_loss[1].frame==0 and d.recovery.frame==120 and #d.after_loss==4,'sticky before/after/recovery survive coalescing and healthy report rotation')
d:social({frames=10,received=3,invalid=1,log_errors=0,token='DO_NOT_EXPORT'},{samples=5},{hud_accepted=4,hud_unavailable=1,poi_accepted=2,poi_unavailable=3,native_error=1})
d:error(7);observe(d,1860,true,864,1,1,1)
check(not reports[#reports]:find('DO_NOT_EXPORT',1,true),'fixed social allowlist');check(reports[#reports]:find('"hud_copied":4',1,true),'HUD counters copied');check(reports[#reports]:find('"social_errors":3',1,true),'fixed error count')
enabled=false;observe(d,1920,true,864,1,0,0)
check(reports[#reports]:find('"enabled":false',1,true)and reports[#reports]:find('"first_loss":null',1,true)and reports[#reports]:find('"recent":[]',1,true),'off only control metadata')
local count=#reports;observe(d,1980,true,864,1,0,0);check(#reports==count,'off no new repeated gameplay records')
enabled=true;observe(d,2040,true,864,1,1,1);check(reports[#reports]:find('"enabled":true',1,true),'reenable without source edit')
local normal=api.debug(bridge,{},nil);normal:canonical(0);observe(normal,0,true,864,1,1,1);observe(normal,60,true,864,0,0,0)
check(not normal.first_loss and normal.recent[#normal.recent].kind==3,'ordinary roster departure distinguished')
local hold=api.debug(bridge,{},nil);observe(hold,0,true,864,1,1,1);observe(hold,60,false,-1,1,1,0)
check(not hold.first_loss and hold.recent[#hold.recent].kind==4,'protection distinguished')
logger=false;local opted=dofile('mods/noita_together2/files/g3/game_api.lua').debug(bridge,{},nil)
observe(opted,0,true,864,1,1,1);observe(opted,60,true,864,1,0,0)
check(not opted.first_loss and #opted.recent==0 and reports[#reports]:find('"logger":false',1,true),'logger preference stricter mod opt out')
local missing=api.debug({},nil,nil);check(pcall(observe,missing,0,true,864,1,1,1),'missing exports fail harmlessly')
local failed=api.debug({nt2_g3_debug_mode=function()return 1 end,nt2_g3_debug_publish=function()error('unwritable injected')end},nil,nil)
check(pcall(observe,failed,0,true,864,1,1,1),'copy failure cannot fail gameplay callback')
print('PASS automatic mod diagnostics '..checks..' checks; production recorder; injected clock/copy/mode; actualNoita=false')
