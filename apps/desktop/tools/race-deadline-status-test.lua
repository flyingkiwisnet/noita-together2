-- Production Rust metadata -> main protected-status model and local_world draw.
-- Canonical inputs/GUI are injected. No authority, network or engine outcome.
local repo=assert(os.getenv('NT2_REJOIN_REPO'));local raw=dofile
local json=raw(repo..'/mod/noita_together2/files/g3/bank_json.lua')
local display=raw(repo..'/mod/noita_together2/files/rejoin/lobby_status.lua')
local recovery=raw(repo..'/mod/noita_together2/files/rejoin/local_world.lua')
local marker='{"kind":"nt2-world","version":1,"local_world":"11111111-1111-4111-8111-111111111111","bundle_digest":"'..string.rep('a',64)..'","actual_seed":"42","loot":"shared:0:42"}'
local saved={record=marker,seed='42',loot='shared:0:42'};local checks=0;local frames={}
local function check(v,message)assert(v,message);checks=checks+1 end
local f=assert(io.open(assert(os.getenv('NT2_RACE_VIEW_INPUT')),'rb'));for line in f:lines()do local name,body=line:match('^([^|]+)|(.+)$');check(#body<=8192,'unchanged mailbox bound');frames[name]=assert(json.decode(body)).race end;f:close()
local function result(model,phase,reason,lobby)return model:message(marker,'42',phase or'terminal',reason,lobby)end
for _,name in ipairs({'outage','hostEnd','otherworld','unbound'})do
 local model=display.race_status();model:update(frames[name],marker,'42');check(not result(model),name..' must not claim a deadline')
end
local model=display.race_status();model:update(frames.deadline,marker,'42');local r=assert(result(model))
check(display.compatibility({code=5,name='Old versions differ'},r)==nil,'retired compatibility notice cannot override confirmed matching Race terminal result')
check(r.title=='Race time limit reached','prominent understandable deadline heading')
check(r.detail:find('Goals completed: 3 / 8',1,true)and r.detail:find('Final place: #2',1,true),'own canonical goals/place preserved')
check(r.footer:find('Race ended',1,true)and r.footer:find('results',1,true),'explains ended Race and result location')
model:update(nil,marker,'42');check(result(model).title==r.title,'actor/lobby projection retirement retains confirmed deadline')
for _,phase in ipairs({'lookup','choice','reconnecting','inspecting','playing','offline'})do check(not result(model,phase),'never override '..phase..' status')end
check(not result(model,'terminal','Storage failure'),'specific recovery failure is not suppressed')
local reconnect={code=3,name='Original lobby'};model:update(nil,marker,'42',reconnect);check(not result(model,'terminal',nil,reconnect),'genuine lobby reconnection error remains visible');check(display.message(reconnect):find('Reconnecting',1,true),'normal connection message retained')
check(result(model).title==r.title,'confirmed result can return after connection message clears')
model:update(nil,marker,'42',{code=2,name='New lobby'});check(not result(model),'different lobby projection retires old result')
model:update(frames.deadline,marker,'42');check(not result(model),'stale old metadata cannot rehydrate after new lobby')
local world=display.race_status();world:update(frames.deadline,marker,'42');check(result(world)~=nil,'fresh matching world accepts result')
world:update(nil,marker:gsub('11111111','22222222'),'42');check(not result(world),'different saved world clears retained result')
local seed=display.race_status();seed:update(frames.deadline,marker,'99');check(not seed:message(marker,'99','terminal'),'physical seed mismatch cannot claim result')
local host=display.race_status();host:update(frames.deadline,marker,'42');host:update(frames.hostEnd,marker,'42');check(not result(host),'host End cannot inherit old deadline label')
local run=display.race_status();run:update(frames.deadline,marker,'42');local next_run=assert(json.decode(assert(json.encode({race=frames.outage})))).race;next_run.snapshot.runKey='new-run';run:update(next_run,marker,'42');check(not result(run),'new run does not inherit terminal result')
local partial=assert(json.decode(assert(json.encode({race=frames.deadline})))).race;partial.deadlineResult.rank=0
local available=display.race_status();available:update(partial,marker,'42');check(result(available).detail=='Goals completed: 3 / 8','unknown canonical place is not invented')
partial.deadlineResult=nil;available:update(partial,marker,'42');check(result(available).title=='Race time limit reached'and result(available).detail=='','confirmed deadline remains clear while own details unavailable')
-- Actual main recovery draw, same explicit offline action/hold ownership.
local owner_model=display.race_status();owner_model:update(frames.deadline,marker,'42');local rows,buttons={},{};local click=false;local retired,released=0,0
GuiText=function(_,_,_,text)rows[#rows+1]=text end
GuiButton=function(_,_,_,_,text)buttons[#buttons+1]=text;return click and text=='Continue offline'end
GameGetWorldStateEntity=function()return 1 end;GlobalsSetValue=function()end
local owner=recovery.new({race_status=function(record,phase,reason)return owner_model:message(record.record,'42',phase,reason)end,retire=function()retired=retired+1 end,release_owned=function(_,offline)check(offline==true,'offline release stays explicit');released=released+1 end})
owner.state='terminal';owner.reason='Rejoin is unavailable for this participation';owner.record=saved;owner.hold={};owner:draw(1)
check(rows[1]:find('Race time limit reached',1,true)and rows[1]:find('world protected',1,true),'existing main protected area shows deadline')
check(rows[2]==r.detail and buttons[1]=='Continue offline','canonical result and explicit offline button remain')
check(owner.state=='terminal'and retired==0 and released==0,'presentation never changes protection or chooses offline')
click=true;owner:draw(1);check(owner.state=='offline'and retired==1 and released==1,'explicit offline click preserves existing retirement semantics')
check(loadfile(repo..'/mod/noita_together2/init-g3-seed-loot.lua')~=nil,'main hook syntax')
print('Race deadline protected status PASS '..checks..' assertions; synthetic canonical/GUI facts only.')
