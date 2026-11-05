-- Actual Rust native-page serialization -> production Lua merge/panel.
-- GUI measurements and canonical input snapshots are injected, not engine pixels.
local repo=assert(os.getenv('NT2_RACE_REPO'));local raw=dofile
_G.dofile=function(path)return raw(repo..'/'..path:gsub('^mods/','mod/'))end
local json=dofile('mods/noita_together2/files/g3/bank_json.lua')
local native=dofile('mods/noita_together2/files/g3/bank_native.lua')
local panel=dofile('mods/noita_together2/files/race/panel.lua');local checks=0
local function check(v,message)assert(v,message);checks=checks+1 end
local e={GameGetRealWorldTimeSinceStarted=function()return 2 end};local receivers,counts,old,paced={},{},{},{}
local function render(view,width,height)
 local labels,colors,containers={},{},{};local next_alpha=1
 e.GuiGetTextDimensions=function(_,label)return #label*3,9 end
 e.GuiColorSetForNextWidget=function(_,r,g,b,a)next_alpha=a end
 e.GuiText=function(_,x,y,label)labels[#labels+1]=label;colors[#colors+1]=next_alpha;next_alpha=1 end
 e.GuiBeginScrollContainer=function(_,_,x,y,w,h)containers[#containers+1]={x=x,y=y,w=w,h=h,alpha=next_alpha};next_alpha=1 end
 e.GuiEndScrollContainer=function()end;e.GuiTooltip=function()end
 local result=panel.draw(e,1,function()return 100 end,view,width,height)
 check(result.y>=64,'Race below native health/gold clearance')
 check(result.x+result.width<=width-5 and result.width<=180,'right panel stays bounded in viewport')
 check(#containers==1 and containers[1].alpha==0.22,'more transparent background')
 for _,alpha in ipairs(colors)do check(alpha==1,'text remains opaque')end
 return table.concat(labels,'|'),result
end
local file=assert(io.open(assert(os.getenv('NT2_RACE_VIEW_INPUT')),'rb'))
for line in file:lines()do
 local label,body=line:match('^([^|]+)|(.+)$');check(#body<=8192,'existing native packet bound')
 if label:match('^paced%-')then paced[label]=body else
 local account,phase=label:match('^(row%-%a+)%-(%a+)$');local key=account;receivers[key]=receivers[key]or{}
 local receiver=receivers[key];local frame=assert(json.decode(body));native.receive_race(receiver,e,frame.race)
 counts[label]=(counts[label]or 0)+1;if phase=='initial'then old[key]=body end
 local view=receiver.race_view
 check(view.standingsComplete and #view.standings==2,'small roster appears on every useful native page')
 if counts[label]==8 then
  check(view.standingsComplete and #view.standings==2,'both canonical accounts reach each Lua consumer')
  check(view.standings[1].rowKey=='row-a'and view.standings[2].rowKey=='row-b','duplicate display names retain independent row keys')
  local own=view.snapshot.entrants[1];check(own.rowKey==account and #own.progress==1,'local goal detail retained separately')
  if phase=='initial'then
   check(view.standings[1].rank==1 and view.standings[2].rank==1,'canonical zero-progress tie is preserved')
  else
   check(view.standings[1].rank==1 and view.standings[1].completedGoals==1 and view.standings[1].finalCountAtMs=='1234','canonical remote count/rank/time updates preserved')
   check(view.standings[2].rank==2 and view.standings[2].completedGoals==0,'canonical second entrant updates preserved')
   check(own.progress[1].value==(account=='row-a'and'500'or'100'),'remote progress cannot overwrite local goal detail')
   native.receive_race(receiver,e,assert(json.decode(old[key])).race);check(receiver.race_view.snapshot.revision=='8','late old revision cannot roll back standings')
  end
  local labels,result=render(view,427,240);check(labels:find('Same Name',1,true),'standings are drawn by production panel');check(result.height<100,'short Race content does not reserve a tall panel')
 end
 end
end
file:close()
-- Ordinary200ms native pacing,2s snapshot refresh. Drop one page and deliver
-- duplicates/out-of-order old frames; old complete display remains coherent.
local paced_receiver={};local function receive(label)return native.receive_race(paced_receiver,e,assert(json.decode(assert(paced[label]))).race)end
receive('paced-initial-1');receive('paced-initial-0')
check(paced_receiver.race_view.standingsRevision=='7'and#paced_receiver.race_view.standings==6,'initial roster complete out of order')
for tick=0,9 do
 if tick==0 or tick==2 then receive('paced-update-0') -- page1 at200ms is dropped
 elseif tick==3 then receive('paced-update-1')
 elseif tick==4 then receive('paced-initial-1')end -- delayed older revision
 local view=paced_receiver.race_view
 if tick<3 then
  check(view.standingsComplete and view.standingsUpdating and view.standingsRevision=='7','retain last COMPLETE roster while new revision incomplete')
  for i,row in ipairs(view.standings)do check(row.rank==1 and row.completedGoals==0,'old roster never mixes new ranks/counts')end
 else
  check(view.standingsRevision=='8'and not view.standingsUpdating,'new complete revision eventually replaces old at600ms')
  check(view.standings[1].rowKey=='row-5'and view.standings[1].rank==1,'higher canonical rank arriving later displays first')
  for i=2,6 do check(view.standings[i].rowKey=='row-'..(i-2)and view.standings[i].rank==2,'stable canonical ties retain source order')end
 end
 local labels=render(view,427,240);check(not labels:find('Updating standings...',1,true),'incomplete refresh does not blank a coherent visible roster')
end
-- New world/run and explicit no-results must not retain the prior scope.
local transition=assert(json.decode(paced['paced-update-0'])).race;transition.snapshot.runKey='next-run';native.receive_race(paced_receiver,e,transition)
check(#paced_receiver.race_view.standings==0 and not paced_receiver.race_view.standingsComplete,'run transition clears old roster immediately')
local second=assert(json.decode(paced['paced-update-1'])).race;second.snapshot.runKey='next-run';native.receive_race(paced_receiver,e,second)
check(#paced_receiver.race_view.standings==6,'new run can establish a fresh complete roster')
transition.world=string.rep('c',64);native.receive_race(paced_receiver,e,transition)
check(#paced_receiver.race_view.standings==0 and not paced_receiver.race_view.standingsComplete,'world transition clears old roster immediately')
native.receive_race(paced_receiver,e,{world=transition.world,snapshot={collectionState='deleted'}})
check(not paced_receiver.race_standings and not paced_receiver.race_parts and#paced_receiver.race_view.standings==0,'explicit no-results clears display and assembly')
-- All eight pages merge without duplicates; a smaller next revision retires rows.
local large={};local original=assert(json.decode(old['row-a'])).race
for page=7,0,-1 do
 local value=assert(json.decode(old['row-a'])).race;value.snapshot.revision='20';value.standingsTotal=32;value.standingsOffset=page*4;value.standings={}
 for i=1,4 do local n=page*4+i;value.standings[i]={rowKey=tostring(n),rank=n,completedGoals=0,status='racing',displayName='Player'..n}end
 native.receive_race(large,e,value)
 if page>0 then check(not large.race_view.standingsComplete and #large.race_view.standings==0,'partial roster never mixes pages')end
end
check(#large.race_view.standings==32,'all32 canonical standings summaries merge')
for i,row in ipairs(large.race_view.standings)do check(row.rowKey==tostring(i)and row.rank==i,'canonical order/ranks survive out-of-order pages')end
original.snapshot.revision='21';native.receive_race(large,e,original);check(#large.race_view.standings==2,'new revision removes departed/previous rows')
-- Long goals/roster must scroll within the game viewport rather than cover HUD.
local view=receivers['row-a'].race_view;view.snapshot.config.goals={};view.snapshot.entrants[1].progress={};view.standings={};view.goalTotal=16
for i=1,16 do view.snapshot.config.goals[i]={instanceId=tostring(i),id='defeat_named_bosses',bossIds={'high_alchemist','wand_connoisseur'}}end
for i=1,32 do view.standings[i]={rowKey=tostring(i),displayName=string.rep('Player',40),completedGoals=i%16,rank=i,status='racing'}end
for _,size in ipairs({{427,240},{320,180},{640,360}})do local _,result=render(view,size[1],size[2]);check(result.height<result.contentHeight and result.height<=126,'long content uses bounded scrolling');check(result.y+12+result.height<size[2],'scroll panel leaves bottom clearance')end
print('Race native/Lua standings and compact overlay PASS '..checks..' assertions; synthetic canonical/GUI facts, no Noita launch.')
