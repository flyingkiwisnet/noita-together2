-- Production capture with injected engine facts; not actual-Noita/save evidence.
local repo=assert(os.getenv('NT2_RACE_REPO'));local checks=0
local function check(v,m)assert(v,m);checks=checks+1 end
local function fixture()
 local e=setmetatable({},{__index=_G});e._G=e;local globals,entities,parts={},{},{};local frame=10;local serial=100;local sent={}
 local function component(id,kind,v)serial=serial+1;parts[serial]=v;local list=entities[id].parts[kind]or{};entities[id].parts[kind]=list;list[#list+1]=serial;return serial end
 local function entity(id,file,tags)entities[id]={alive=true,file=file,parts={},tags=tags or{}}end
 entity(1,'player.xml',{player_unit=true});component(1,'GameStatsComponent',{is_player=true});component(1,'ControlsComponent',{enabled=true});local damage=component(1,'DamageModelComponent',{hp=4,max_hp=20});local wallet=component(1,'WalletComponent',{money=10000})
 e.EntityGetIsAlive=function(id)return entities[id]and entities[id].alive or false end
 e.EntityGetRootEntity=function(id)return id end;e.EntityGetFilename=function(id)return entities[id]and entities[id].file end
 e.EntityHasTag=function(id,k)return entities[id]and entities[id].tags[k]or false end
 e.EntityGetWithTag=function(k)return k=='player_unit'and{1}or{}end
 e.EntityGetComponentIncludingDisabled=function(id,k)return entities[id]and entities[id].parts[k]or{}end
 e.EntityGetFirstComponentIncludingDisabled=function(id,k)return(e.EntityGetComponentIncludingDisabled(id,k)or{})[1]end
 e.EntityAddComponent2=component;e.ComponentGetValue2=function(c,k)return parts[c][k]end;e.ComponentSetValue2=function(c,k,v)parts[c][k]=v end
 e.GlobalsGetValue=function(k,f)return globals[k]or f end;e.GlobalsSetValue=function(k,v)globals[k]=v end
 e.GameGetFrameNum=function()return frame end;e.SessionNumbersGetValue=function()return e.progression or'0'end
 e.EntityGetTransform=function()return e.x or 0,100 end;e.EntityGetAllChildren=function()return{}end
 e.BiomeMapGetName=function()return e.biome or'$biome_liquidcave'end;e.BiomeMapGetSize=function()return 70 end;e.GetParallelWorldPosition=function(x)return math.floor((x+17920)/35840)end
 e.GameHasFlagRun=function()return false end
 e.dofile=function(path)local f=assert(loadfile(repo..'/'..path:gsub('^mods/','mod/')));setfenv(f,e);return f()end
 local json=e.dofile('mods/noita_together2/files/g3/bank_json.lua');local store=e.dofile('mods/noita_together2/files/race/storage.lua');local api=e.dofile('mods/noita_together2/files/race/hook_api.lua');local n=0
 local main=e.dofile('mods/noita_together2/files/race/local.lua').new(e,function()n=n+1;return string.format('00000000-0000-4000-8000-%012d',n)end,function(body)sent[#sent+1]=json.decode(body);return true end)
 local view={world=string.rep('a',64),status='racing',initialized=true,acceptedSequence=0,mapVerified=true}
 local function tick(playing)main:tick(playing~=false,1,view,string.rep('b',32))end
 local function q()return store.read(e,'outgoing')end
 tick();return e,api,main,tick,q,sent,entity,component,function()frame=frame+1 end,view,parts,damage,wallet,store
end
do
 local e,api,main,tick,q,sent,entity,component,step,view=fixture()
 step();tick();check(q().queue[1].observations[1].value=='500','HP uses max_hp times25');check(q().queue[1].observations[2].value=='10000','wallet UI units')
 check(q().queue[1].observations[3].biomeId=='ancient_laboratory','raw liquidcave identity')
 local first=sent[#sent].commit;tick();check(sent[#sent].commit.eventId==first.eventId,'copy ack does not discard original')
 view.acceptedSequence=1;view.accepted={q().queue[1].eventId};tick();check(#q().queue==0,'only canonical acceptance retires original')
 for _,id in ipairs({0,128,256,11,139,267})do entity(id+1000,'orb.xml');component(id+1000,'OrbComponent',{orb_id=id});local o=api.orb_begin(id+1000,1);check(o and o.orbId==id,'full orb identity '..id);api.orb_commit(o);check(api.orb_begin(id+1000,1)==nil,'duplicate orb callback')end
 for _,id in ipairs({13,141,269,127,255,268})do entity(id+2000,'orb.xml');component(id+2000,'OrbComponent',{orb_id=id});check(api.orb_begin(id+2000,1)==nil,'unknown orb rejected '..id)end
 entity(2,'data/entities/animals/boss_dragon.xml');api.boss(2);api.boss(2);entity(3,'data/entities/animals/boss_dragon/minion.xml');api.boss(3)
 step();tick();local count=0;for _,o in ipairs(q().queue[1].observations)do if o.kind=='boss'then count=count+1 end end;check(count==1,'exact final boss type once')
end
do
 local e,api,main,tick,q,sent,entity,component,step,view,parts,damage,wallet=fixture();step();tick();view.acceptedSequence=1;view.accepted={q().queue[1].eventId};tick()
 parts[damage].max_hp=24;parts[wallet].money=15000;parts[damage].hp=-0.1
 api.terminal('ending',false);api.terminal('ending',true);api.terminal('dead',false)
 check(#q().queue==0,'terminal remains draft until same-frame callbacks finish')
 entity(2,'data/entities/animals/boss_pit/boss_pit.xml');api.boss(2);step();tick(false)
 local original=q().queue[1];check(original.terminal=='ending','ending terminal wins same-frame death')
 local facts={};for _,o in ipairs(original.observations)do facts[o.kind]=o end
 check(facts.max_hp.value=='600'and facts.gold.value=='15000','final numeric facts before terminal');check(facts.ending.ending=='classic','verified Classic upgrades generic candidate');check(facts.boss.bossId=='wand_connoisseur','same-frame boss retained')
 api.observe({{kind='orb',orbId=4}});step();tick(false);check(#q().queue==1 and q().closed,'terminal immutable, no new capture')
 check(sent[#sent].commit.eventId==original.eventId,'terminal retries while protected')
end
do
 local e,api,main,tick,q,sent,entity,component,step,view,parts,damage,wallet,store=fixture();step();tick();view.acceptedSequence=1;view.accepted={q().queue[1].eventId};tick();local sample=e.dofile('mods/noita_together2/files/race/sample.lua')
 e.x=35840*3;local loc=sample.locations(e,1,true);check(loc[2].direction=='east'and loc[2].index==3,'directional parallel level')
 e.x=-35840;loc=sample.locations(e,1,true);check(loc[2].direction=='west'and loc[2].index==1,'west separate');e.x=17920;check(#sample.locations(e,1,true)==1,'wall margin excluded');e.x=35840;check(#sample.locations(e,1,false)==1,'unknown map never credited')
 parts[damage].max_hp=1;parts[wallet].money=0;api.observe(sample.facts(e,1,false));check(#q().staged==0,'decrease never resets highwater')
 component(1,'VariableStorageComponent',{name='nt2_g3_recovery'});api.terminal('dead',false);check(q().terminal=='none','native hold refuses terminal');store.write(e,'authority',{});api.boss(1);check(#q().staged==0,'stale/offline grant excluded')
end
do
 local e,api,main,tick,q,sent,entity,component,step,view,parts,damage,wallet,store=fixture();step();tick();view.acceptedSequence=1;view.accepted={q().queue[1].eventId};tick()
 for i=0,28 do api.observe({{kind='orb',orbId=i}})end
 parts[damage].max_hp=30;parts[wallet].money=25000;api.terminal('ending',true);check(#q().staged==32,'reserved terminal facts fit full ordinary batch');step();tick(false);check(q().closed and #q().queue[1].observations==32,'bounded complete terminal batch')
 -- An older save must allocate above accepted sequence, without recreating success.
 store.write(e,'outgoing',{world=view.world,nextSequence=1,queue={},staged={},known={},terminal='none'});view.acceptedSequence=9;tick();api.observe({{kind='orb',orbId=0}});step();tick();check(q().queue[1].sequence==10,'Continue synchronizes accepted sequence')
end
do
 local e,api,main,tick,q,sent,entity,component,step,view,parts,damage,wallet,store=fixture();step();tick();local original=q().queue[1]
 view.acceptedSequence=9;view.accepted={};tick();check(q().queue[1].eventId==original.eventId,'sequence highwater alone never discards uncertain original')
 view.accepted={original.eventId};view.reservedSequence=12;tick();api.observe({{kind='orb',orbId=0}});step();tick();check(q().queue[1].sequence==13,'older save allocates above durable unacknowledged app sequence')
end
do
 local e,api,main,tick,q,sent,entity,component,step,view,parts,damage,wallet=fixture();step();tick();view.accepted={q().queue[1].eventId};view.acceptedSequence=1;tick()
 local has=e.EntityHasTag;e.EntityHasTag=function(id,tag)return id==1 and tag=='polymorphed'or has(id,tag)end
 e.biome='$biome_crypt';parts[damage].max_hp=999;parts[wallet].money=999999;tick();local sampler=e.dofile('mods/noita_together2/files/race/sample.lua')
 check(#sampler.facts(e,1,false)==0,'transformed HP/wallet remain unknown');api.observe(sampler.locations(e,1,true));check(q().staged[1].biomeId=='temple_of_the_art','form biome tracks own actor')
 e.GetUpdatedEntityID=function()return 1 end;e.dofile('mods/noita_together2/files/race/player_death.lua');e.death();check(q().terminal=='none','form expiry/removal is not player death')
 e.damage_received(1,'fatal injected engine message',0,true);check(q().terminal=='dead','documented fatal message seals own form death');main:post_update();check(q().closed and #q().queue==1,'post-update seals once without another engine frame')
 check(sent[#sent].commit.terminal=='dead'and sent[#sent].commit.observations[1].kind=='biome','final form batch keeps location and excludes numeric form facts')
end
do
 local e,api,main,tick,q,sent,entity,component,step,view,parts,damage,wallet,store=fixture()
 view.needsClassic=true;tick();check(not main.ready and e.GlobalsGetValue('nt2_race_detector_failure','')~='','selected unavailable detector blocks readiness')
 view.needsClassic=false;tick();check(main.ready,'unselected unavailable detector leaves other goals ready')
 e.progression='1';tick();check(not main.ready,'NG+ never advertises Race readiness')
end
do
 local e=fixture();local sha=e.dofile('mods/noita_together2/files/race/sha256.lua')
 check(sha('')=='e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855','SHA empty vector')
 check(sha('abc')=='ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad','SHA abc vector')
 local catalogue=e.dofile('mods/noita_together2/files/race/catalogue.lua')
 for path,pin in pairs(catalogue.sourceHashes)do local f=assert(io.open(repo..'/spikes/noita-entry/.build/race-goal-catalogue-20261105/game-reference/'..path,'rb'));local bytes=f:read('*a');f:close();check(sha(bytes)==pin,'actual extracted source pin '..path)end
end
print('Race production capture PASS '..checks..' assertions; injected engine facts only.')
