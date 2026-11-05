-- Production hooks/collector; labeled engine facts, no actual Noita/save evidence.
local repo=assert(os.getenv('NT2_D1_REPO'));local checks=0
local function check(v,m)assert(v,m);checks=checks+1 end
local function fixture()
 local e=setmetatable({},{__index=_G});e._G=e;local globals,parts,entities={}, {}, {};local serial=100;local frame,progression=5,0;local sent={};local vanilla=0
 local function component(id,kind,value)serial=serial+1;parts[serial]=value;local list=entities[id].parts[kind]or{};entities[id].parts[kind]=list;list[#list+1]=serial;return serial end
 local function entity(id,tags)entities[id]={alive=true,tags=tags or{},parts={}}end
 entity(1,{player_unit=true});component(1,'GameStatsComponent',{is_player=true});component(1,'ControlsComponent',{enabled=true});component(1,'DamageModelComponent',{hp=4,max_hp=8,max_hp_cap=0})
 e.EntityGetIsAlive=function(id)return entities[id]and entities[id].alive or false end
 e.EntityGetRootEntity=function(id)return id end
 e.EntityHasTag=function(id,tag)return entities[id]and entities[id].tags[tag]or false end
 e.EntityGetWithTag=function(tag)return tag=='player_unit'and{1}or{}end
 e.EntityGetFirstComponentIncludingDisabled=function(id,kind)return entities[id]and(entities[id].parts[kind]or{})[1]end
 e.EntityGetComponentIncludingDisabled=function(id,kind)return entities[id]and entities[id].parts[kind]or{}end
 e.EntityGetComponent=e.EntityGetComponentIncludingDisabled
 e.EntityAddComponent2=function(id,kind,value)if e.fail_marker then return 0 end;return component(id,kind,value)end
 e.ComponentGetValue2=function(id,key)return parts[id][key]end;e.ComponentGetValue=e.ComponentGetValue2
 e.ComponentSetValue2=function(id,key,v)parts[id][key]=v end;e.ComponentSetValue=e.ComponentSetValue2
 e.GlobalsGetValue=function(k,f)return globals[k]or f end;e.GlobalsSetValue=function(k,v)globals[k]=v end
 e.GameGetFrameNum=function()return frame end;e.SessionNumbersGetValue=function()return tostring(progression)end
 e.GameHasFlagRun=function(k)return e.sharing and(k=='NT_sync_hearts'or k=='NT_sync_orbs')or false end
 e.GameTextGet=function(key)return key end
 e.EntityKill=function(id)entities[id].alive=false end;e.EntityGetTransform=function()return 0,0 end
 for _,k in ipairs({'GamePrint','GamePrintImportant','GameTriggerMusicCue','EntityLoad','print_error'})do e[k]=function()end end
 local load=e.dofile
 e.dofile=function(path)
  if path:find('c1_hook_api.lua',1,true)then return {duplicate=function()return false end,can_pickup=function()return true end,roster_count=function()return 1 end,capture=function()return {eventId='11111111-1111-4111-8111-111111111111'}end,submit=function()end}end
  local f=assert(loadfile(repo..'/'..path:gsub('^mods/','mod/')));setfenv(f,e);return f()
 end
 local id=0;local function identity()id=id+1;return string.format('00000000-0000-4000-8000-%012d',id)end
 local json=e.dofile('mods/noita_together2/files/g3/bank_json.lua');local api=e.dofile('mods/noita_together2/files/g3/d1_hook_api.lua');local main=e.dofile('mods/noita_together2/files/g3/d1_local.lua').new(e,identity,function(v)sent[#sent+1]=json.decode(v);return true end)
 local function tick(accepted,playing)main:tick(playing~=false,1,{world=string.rep('a',64)},string.rep('b',32),accepted)end
 local function rows()return json.decode_saved(globals.nt2_d1_outgoing).queue end
 tick()
 return e,api,main,tick,rows,function()return sent end,function(n,tags,orb)entity(n,tags);if orb then component(n,'OrbComponent',{orb_id=orb})end end,function()vanilla=vanilla+1 end,function()return vanilla end,globals,function(v)progression=v end
end
for _,hook in ipairs({'heart_append.lua','heart_better_append.lua','orb_pickup.lua'})do
 for _,sharing in ipairs({false,true})do
  local e,api,main,tick,rows,sent,add,base,count,g=fixture();e.sharing=sharing;add(2,{},hook=='orb_pickup.lua'and 128 or nil)
  e.item_pickup=function(id,player)base();if hook~='orb_pickup.lua'then e.EntityKill(id)end end
  e.dofile('mods/noita_together2/files/c1/append/'..hook);e.item_pickup(2,1,'pickup')
  check(#rows()==1 and rows()[1].applied,'actual pickup observed ON/OFF '..hook)
  check(rows()[1].observation.kind==(hook=='orb_pickup.lua'and'orb'or'heart'),'fixed pickup kind')
  if sharing then check(rows()[1].observation.eventId=='11111111-1111-4111-8111-111111111111','same C1 original links statistics')else check(count()==1,'sharing OFF preserves vanilla callback')end
  if hook=='orb_pickup.lua'then check(rows()[1].observation.orbId=='128','full parallel orb identity retained')end
  tick();local original=sent()[1].observation.eventId;tick();check(sent()[2].observation.eventId==original,'retry keeps immutable original')
  tick({original});check(#rows()==0,'accepted observation removed')
  add(3,{fake_orb=true},128);e.item_pickup(3,1,'fake');check(#rows()==0,'fake/forced heart/orb excluded')
 end
end
do
 local e,api,main,tick,rows,sent,add,base,count,g=fixture();add(2);local original=api.begin('heart',1,2);check(original~=nil,'capture before mutation');tick();check(#sent()==0 and not rows()[1].applied,'uncertain outcome never reported or mutated')
 api.commit(original);local saved=g.nt2_d1_outgoing;local restored=e.dofile('mods/noita_together2/files/g3/d1_local.lua').new(e,function()return nil end,function(v)local json=e.dofile('mods/noita_together2/files/g3/bank_json.lua');check(json.decode(v).observation.eventId==original.eventId,'Continue retains same original');return true end)
 restored:tick(true,1,{world=string.rep('a',64)},string.rep('c',32),{});check(g.nt2_d1_outgoing~=nil and saved~=nil,'observed-only saved queue retained')
 add(4);e.fail_marker=true;check(api.begin('heart',1,4)==nil,'failed dedup marker never confirms capture');e.fail_marker=false
 add(5);tick(nil,false);check(api.begin('heart',1,5)==nil,'protected/offline observations refused')
 g.nt2_d1_outgoing='malformed';tick();check(g.nt2_d1_outgoing=='malformed','corrupt queue is preserved')
end
do
 local e,api,main,tick,rows,sent,add,base,count,g=fixture();local deaths=0;e.death=function()deaths=deaths+1 end;local current=2;e.GetUpdatedEntityID=function()return current end;e.dofile('mods/noita_together2/files/c1/append/steve_death.lua')
 add(2);e.death(0,'local',1,true);e.death(0,'duplicate',1,true);current=3;add(3);e.death(0,'second legitimate kill',1,true);check(#rows()==2,'two distinct actual Steve kills; duplicate callback once');check(deaths==3,'vanilla death callback retained')
 current=4;add(4);e.death(0,'other player',99,true);check(#rows()==2,'unattributed/other-player kills excluded')
 g.TEMPLE_SPAWN_GUARDIAN='1';tick();check(#rows()==2,'anger state is not a Steve death')
end
do
 local e,api,main,tick,rows,sent,add,base,count,g=fixture()
 for id=2,65 do add(id);api.commit(api.begin('heart',1,id));tick()end
 check(#rows()==64,'optional queue has a fixed64 observation bound');add(66);check(api.begin('heart',1,66)==nil and #rows()==64,'overflow never grows queue');check(tonumber(g.nt2_d1_dropped)>0,'overflow has bounded diagnostics')
 local json=e.dofile('mods/noita_together2/files/g3/bank_json.lua');local a=json.decode_saved(g.nt2_d1_authority);a.frame=0;g.nt2_d1_authority=json.encode_saved(a);check(api.begin('heart',1,66)==nil,'stale frame grant cannot publish')
end
do
 local e,api,main,tick,rows,sent,add=fixture();add(2);add(3);e.EntityGetFirstComponentIncludingDisabled=(function(old)return function(id,kind)if id==3 and kind=='ProjectileComponent'then return 9999 end;return old(id,kind)end end)(e.EntityGetFirstComponentIncludingDisabled)
 e.ComponentGetValue2=(function(old)return function(id,key)if id==9999 and key=='mWhoShot'then return 1 end;return old(id,key)end end)(e.ComponentGetValue2)
 api.steve(2,3);check(#rows()==1,'projectile responsible attribution resolves local shooter')
end
do
 local e,api,main,tick,rows,sent,add=fixture();add(2);local controls=e.EntityGetFirstComponentIncludingDisabled(1,'ControlsComponent');e.ComponentSetValue2(controls,'enabled',false);check(api.begin('heart',1,2)==nil,'unowned disabled controls remain protected')
 e.EntityAddComponent2(1,'VariableStorageComponent',{name='nt2_a2_composer_lock',value_bool=true});api.steve(2,1);check(#rows()==1,'actual locally attributed kill while typing still counts')
 add(3);e.EntityAddComponent2(1,'VariableStorageComponent',{name='nt2_g3_recovery'});check(api.begin('heart',1,3)==nil,'native protection wins over composer lease')
end
print('PASS D1 production hook/local collector '..checks..' assertions; injected engine facts only')
