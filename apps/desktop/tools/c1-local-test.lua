-- Production C1 hooks/handlers with injected Noita calls. No game/save evidence.
local repo=assert(os.getenv('NT2_ITEM_REPO'));local rawload=loadfile
local checks=0
local function eq(a,b)assert(type(a)=='number'and type(b)=='number'and math.abs(a-b)<1e-9 or a==b,tostring(a)..' ~= '..tostring(b));checks=checks+1 end
local function engine()
 local e=setmetatable({},{__index=_G});e._G=e
 local globals,flags,components,entities={},{},{[11]={hp=2,max_hp=4,max_hp_cap=0},[12]={is_player=true},[13]={enabled=true}},{[1]={x=10,y=20,alive=true,tags={player_unit=true},parts={DamageModelComponent={11},GameStatsComponent={12},ControlsComponent={13}}},[2]={x=10,y=20,alive=true,tags={},parts={}}}
 local next_id=20;local function entity(id)return assert(entities[id],'missing injected entity '..tostring(id))end
 function e.dofile(path)local p=path:gsub('^mods/','mod/');if p:find('c1_catalogue_data.lua',1,true)then p='packages/contracts/g3-c1/catalogue.lua'end;local f=assert(rawload(repo..'/'..p));setfenv(f,e);return f()end
 e.dofile_once=e.dofile
 e.GlobalsGetValue=function(k,fallback)return globals[k]or fallback end;e.GlobalsSetValue=function(k,v)
  if k:match('^nt2_c1_')then assert(type(v)=='string'and not v:find('["&<>%z\1-\31]'),'Unsafe persisted C1 Global: '..k)end
  globals[k]=v
 end
 e.GameHasFlagRun=function(k)return flags[k]==true end;e.GameAddFlagRun=function(k)flags[k]=true end;e.GameRemoveFlagRun=function(k)flags[k]=nil end
 e.EntityGetIsAlive=function(id)return entities[id]and entities[id].alive or false end;e.EntityGetRootEntity=function(id)return id end
 e.EntityGetWithTag=function(tag)local out={};for id,v in pairs(entities)do if v.alive and v.tags[tag]then out[#out+1]=id end end;return out end
 e.EntityHasTag=function(id,tag)return entity(id).tags[tag]or false end;e.EntityAddTag=function(id,tag)entity(id).tags[tag]=true end
 e.EntityGetComponentIncludingDisabled=function(id,kind)return entity(id).parts[kind]end;e.EntityGetComponent=e.EntityGetComponentIncludingDisabled
 e.EntityGetFirstComponentIncludingDisabled=function(id,kind)return(entity(id).parts[kind]or{})[1]end;e.EntityGetFirstComponent=e.EntityGetFirstComponentIncludingDisabled
 e.ComponentGetValue2=function(c,k)return assert(components[c])[k]end;e.ComponentGetValue=e.ComponentGetValue2
 e.ComponentSetValue2=function(c,k,v)components[c][k]=v end;e.ComponentSetValue=e.ComponentSetValue2
 e.EntityAddComponent2=function(id,kind,data)next_id=next_id+1;components[next_id]=data;entity(id).parts[kind]=entity(id).parts[kind]or{};table.insert(entity(id).parts[kind],next_id);return next_id end
 e.EntityRemoveComponent=function(id,c)for kind,rows in pairs(entity(id).parts)do for i=#rows,1,-1 do if rows[i]==c then table.remove(rows,i)end end end;components[c]=nil end
 e.EntityAddComponent=e.EntityAddComponent2
 e.EntityGetTransform=function(id)return entity(id).x,entity(id).y end;e.EntitySetTransform=function(id,x,y)entity(id).x=x;entity(id).y=y end
 e.EntityGetAllChildren=function()return{}end;e.EntityGetParent=function()return 1 end
 e.EntityKill=function(id)entity(id).alive=false end;e.EntityLoad=function(path,x,y)next_id=next_id+1;local id=next_id;entities[id]={x=x,y=y,alive=true,tags={},parts={}};if path:find('death_cam',1,true)then entities[id].tags.nt2_c1_death_camera=true end;if path:find('forced_orb',1,true)then e.EntityAddComponent2(id,'OrbComponent',{});e.EntityAddComponent2(id,'ItemComponent',{})end;return id end
 e.print=function()end;e.GameTriggerGameOver=function()e.game_overs=(e.game_overs or 0)+1 end
 e.GameGetFrameNum=function()return e.frame or 100 end;e.GameTextGet=function(key,...)return key end;e.GamePrint=function()end;e.GamePrintImportant=function()end;e.GameTriggerMusicCue=function()end;e.print_error=function()end
 e.ModSettingGet=function()return nil end;e.MagicNumbersGetValue=function(k)return k:find('_X')and '10'or '20'end
 e.GameGetOrbCountThisRun=function()return e.orb_count or 0 end;e.GameGetOrbCollectedThisRun=function(id)return e.orb_owned==id end
 e.GameSetCameraFree=function(value)e.camera_free=value end;e.GameGetCameraPos=function()return 10,20 end;e.GameSetCameraPos=function()end
 e.LoadGameEffectEntityTo=function(player,path)local id=e.EntityLoad(path,0,0);e.EntityAddComponent2(id,'GameEffectComponent',{});e.last_effect=id;return id end
 e.HandleBannedPerksEvent=function()e.bans=(e.bans or 0)+1;return true end
 return e,components,entities,globals,flags
end
local rules={NT_sync_hearts=true,NT_sync_orbs=true,NT_sync_shift=true,NT_team_perks=true,NT_sync_perks=false,NT_alt_heartache=false,NT_ban_perks=false,NT_sync_orbs_no_curse=false,NT_ondeath_kick=false,NT_sync_orb_count=0,NT_death_penalty='end'}
local function copy(t)local o={};for k,v in pairs(t)do o[k]=v end;return o end
if NT2_C1_TEST_ENGINE_ONLY then return {engine=engine,rules=rules,copy=copy}end
-- Actual append callbacks retain local normal/large quirks and vanilla off path.
for _,large in ipairs({false,true})do
 for _,n in ipairs({0,1,2,6,7,100})do
  local e,c,entities,g,f=engine();local original;local captured=0;local vanilla=0
  e.NT2_C1={can_pickup=function()return true end,roster_count=function()return n end,capture=function(_,kind,payload)captured=captured+1;eq(kind,'heart');return{}end,submit=function()end}
  local actual=e.dofile;e.dofile=function(path)if path:find('c1_hook_api',1,true)then return e.NT2_C1 end;return actual(path)end
  e.item_pickup=function()vanilla=vanilla+1 end;f.NT_sync_hearts=true
  e.dofile('mods/noita_together2/files/c1/append/'..(large and 'heart_better_append.lua'or 'heart_append.lua'))
  e.item_pickup(2,1,'heart');eq(c[11].hp,2);eq(c[11].max_hp,4+math.max(large and .32 or .16,n>0 and(large and 2 or 1)/n or 0));eq(captured,1);eq(vanilla,0)
  e.item_pickup(2,1,'heart');eq(captured,1)
 end
end
do
 local e,c,entities,g,f=engine();local captured=0
 e.NT2_C1={can_pickup=function()return true end,roster_count=function()return 2 end,capture=function()captured=captured+1 end}
 local actual=e.dofile;e.dofile=function(path)if path:find('c1_hook_api',1,true)then return e.NT2_C1 end;return actual(path)end
 local vanilla=0;e.item_pickup=function()vanilla=vanilla+1 end;e.dofile('mods/noita_together2/files/c1/append/heart_append.lua')
 e.item_pickup(2,1,'heart');eq(vanilla,1);eq(c[11].max_hp,4)
 f.NT_sync_hearts=true;entities[2].tags.fake_orb=true;e.item_pickup(2,1,'heart');eq(captured,0);eq(c[11].max_hp,4.5)
end
-- Canonical base coverage, receiver skip boundaries and fixed orb relocation.
do
 local e,c,entities=engine();local content=e.dofile('mods/noita_together2/files/g3/c1_content.lua').load(e)
 local count=0;for _ in pairs(content.perks)do count=count+1 end;eq(count,106)
 local effects=e.dofile('mods/noita_together2/files/g3/c1_effects.lua').new(e,content)
 local state=effects:apply(1,{payload={heart={}},sourceName='Peer'},rules,2,100,rules);eq(state,'applied');eq(c[11].hp,2.5);eq(c[11].max_hp,4.5)
 local off=copy(rules);off.NT_sync_hearts=false;state=effects:apply(1,{payload={heart={}},sourceName='Peer'},off,2,100,rules);eq(state,'skipped');eq(c[11].hp,2.5)
 for id in pairs(content.orbs)do
  eq(type(id),'number')
  local status=effects:apply(1,{payload={orb={orbId=id}},sourceName='Peer'},rules,2,100,rules);eq(status,'applied')
 end
 eq(#effects.orb_moves,39);effects:tick(109,1,true);eq(#effects.orb_moves,39);effects:tick(110,1,true);eq(#effects.orb_moves,0)
 local capped=copy(rules);capped.NT_sync_orb_count=1;e.orb_count=1;state=effects:apply(1,{payload={orb={orbId=0}},sourceName='Peer'},capped,2,111,rules);eq(state,'skipped')
 local stacks={};e.perk_list={};e.get_perk_with_id=function(_,id)return content.perks[id]end
 e.perk_spawn=function(x,y,id)e.last_perk=id;return e.EntityLoad('perk',x,y)end;e.perk_pickup=function(perk,player)eq(e.EntityGetFirstComponentIncludingDisabled(perk,'VariableStorageComponent')~=nil,true);stacks[e.last_perk]=(stacks[e.last_perk]or 0)+1 end
 local all=copy(rules);all.NT_sync_perks=true
 for id in pairs(content.perks)do local status=effects:apply(1,{payload={perk={perkId=id}},sourceName='Peer'},all,2,120,rules);eq(status,'applied')end
 for id in pairs(content.perks)do eq(stacks[id],1)end
 state=effects:apply(1,{payload={perk={perkId='EXTRA_MONEY'}},sourceName='Peer'},all,2,121,rules);eq(state,'applied');eq(stacks.EXTRA_MONEY,2)
end
-- Ordered local receipt guard: duplicate, cold ambiguous, missing actor, rule toggles.
do
 local e,c,entities,g,f=engine();local sent={};local uid=0
 local content=e.dofile('mods/noita_together2/files/g3/c1_content.lua').load(e)
 local owner=e.dofile('mods/noita_together2/files/g3/c1_local.lua').new(e,function()uid=uid+1;return string.format('%08d-1111-4111-8111-111111111111',uid)end,function(body)sent[#sent+1]=body;return true end,content)
 local view={world=string.rep('a',64),initialized=true,memberCount=2,snapshot={revision=1,rules=copy(rules)}}
 owner:receive(view);owner:tick(true,1,100)
 view.event={eventId='11111111-1111-4111-8111-111111111111',sequence='1',payload={heart={}},sourceName='Peer',capturedRules=rules};view.permit='permit'
 owner:receive(view);owner:tick(true,1,101);eq(c[11].hp,2.5);owner:tick(true,1,102);eq(c[11].hp,2.5)
 local second=e.dofile('mods/noita_together2/files/g3/c1_local.lua').new(e,function()end,function()return true end,content);second:receive(view);second:tick(true,1,103);eq(c[11].hp,2.5)
 view.event=nil;view.permit=nil;view.snapshot={revision=2,rules=copy(rules)};view.snapshot.rules.NT_ban_perks=true;owner:receive(view);owner:tick(true,1,104);eq(e.bans,1);owner:receive(view);owner:tick(true,1,105);eq(e.bans,1)
 view.snapshot={revision=1,rules=copy(rules)};owner:receive(view);eq(f.NT_ban_perks,true)
 for _,mode in ipairs({'full_respawn','weak_respawn','end'})do
  c[11].hp=-0.1;c[11].max_hp=4;e.frame=200;owner.terminal_state=false;owner.death=nil;owner.last_death=-31
  view.snapshot={revision=owner.revision+1,rules=copy(rules)};view.snapshot.rules.NT_death_penalty=mode;owner:receive(view);owner:tick(true,1,200)
  if mode=='end'then eq(c[11].kill_now,true)else eq(c[11].max_hp,mode=='weak_respawn'and 3.2 or 4);eq(c[11].hp,c[11].max_hp);eq(e.ComponentGetValue2(e.EntityGetFirstComponentIncludingDisabled(e.last_effect,'GameEffectComponent'),'frames'),2400);eq(e.camera_free,true);owner:tick(true,1,800);eq(e.camera_free,false)end
 end
end
-- Execute the retained Heartache scripts, including changing rules at expiry.
for _,alternate in ipairs({false,true})do
 local e,c,entities,g,f=engine();local storage=e.EntityAddComponent2(2,'VariableStorageComponent',{name='effect_hearty',value_float=0,value_bool=false})
 e.EntityAddComponent2(2,'UIIconComponent',{name='$status_hearty'})
 e.GetUpdatedEntityID=function()return 2 end
 e.dofile_once=function()end
 e.edit_component=function(player,kind,callback)for _,id in ipairs(e.EntityGetComponent(player,kind)or{})do callback(id,{})end end
 f.NT_alt_heartache=alternate
 e.dofile('mods/noita_together2/files/c1/append/status_effects/hearty_start.lua');eq(c[11].hp,1);eq(c[11].max_hp,alternate and 4 or 2);eq(c[storage].value_bool,alternate)
 f.NT_alt_heartache=not alternate
 local root=e.EntityGetRootEntity;e.EntityGetRootEntity=function(id)if id==2 then return 1 end;return root(id)end
 e.dofile('mods/noita_together2/files/c1/append/status_effects/hearty_end.lua');eq(c[11].hp,2);eq(c[11].max_hp,4)
end
-- Real remote fungal handler: conversion list, no-op, cooldown, gold, iteration.
do
 local e,c,entities,g=engine();local converted={};local seeded={};local music=0
 e.GameGetWorldStateEntity=function()return 1 end;e.EntityAddComponent2(1,'WorldStateComponent',{EVERYTHING_TO_GOLD=false})
 e.SetRandomSeed=function(a,b)seeded={a,b}end;e.rand=function()return 0 end
 local types={water=1,blood=2,oil=3};e.CellFactory_GetType=function(name)return types[name]or -1 end;e.CellFactory_GetName=function(id)for name,n in pairs(types)do if n==id then return name end end end
 e.CellFactory_GetUIName=function(id)return tostring(id)end;e.GameTextGetTranslatedOrNot=function(v)return v end
 e.ConvertMaterialEverywhere=function(a,b)converted[#converted+1]={a,b}end;e.GameCreateParticle=function()end
 e.GameTriggerMusicFadeOutAndDequeueAll=function()end;e.GameTriggerMusicEvent=function()music=music+1 end
 e.EntityAddChild=function()end;e.EntityCreateNew=function(name)return e.EntityLoad(name,0,0)end;e.EntityGetName=function()return ''end
 local receive=e.dofile('mods/noita_together2/files/c1/scripts/fungal_receive.lua')
 e.frame=18000;local state,reason=receive(e,1,{materials={'water','blood'},name_material='water'},{material='oil'});eq(state,'applied');eq(#converted,2);eq(music,1);eq(g.fungal_shift_iteration,'1');eq(g.fungal_shift_last_frame,'18000');eq(seeded[1],89346);eq(seeded[2],42345)
 state,reason=receive(e,1,{materials={'water'}},{material='oil'});eq(state,'skipped');eq(reason,'fungal_cooldown');eq(#converted,2)
 e.frame=36000;g.fungal_shift_iteration='20';state=receive(e,1,{materials={'water'}},{material='oil'});eq(state,'applied');eq(g.fungal_shift_iteration,'21')
 e.frame=54000;state,reason=receive(e,1,{materials={'water'}},{material='oil'});eq(state,'skipped');eq(reason,'fungal_limit');eq(g.fungal_shift_iteration,'22')
 g.fungal_shift_iteration='0';state,reason=receive(e,1,{materials={'water'}},{material='water'});eq(state,'skipped');eq(reason,'fungal_noop');eq(g.fungal_shift_iteration,'1')
 c[e.EntityGetFirstComponentIncludingDisabled(1,'WorldStateComponent')].EVERYTHING_TO_GOLD=true;state,reason=receive(e,1,{materials={'water'}},{material='oil'});eq(state,'skipped');eq(reason,'gold_world');eq(g.fungal_shift_iteration,'1')
end
-- Polymorph death waits for normal owned body; a missing actor is never death.
do
 local e,c,entities,g=engine();local content=e.dofile('mods/noita_together2/files/g3/c1_content.lua').load(e);local id=0
 local owner=e.dofile('mods/noita_together2/files/g3/c1_local.lua').new(e,function()id=id+1;return string.format('%08d-1111-4111-8111-111111111111',id)end,function()return true end,content)
 local r=copy(rules);r.NT_death_penalty='weak_respawn'
 local view={world=string.rep('b',64),initialized=true,memberCount=2,snapshot={revision=1,rules=r}};owner:receive(view)
 entities[1].tags={};owner:tick(true,1,100);eq(owner.death,nil);eq(c[11].max_hp,4)
 entities[1].tags={polymorphed=true};c[11].hp=-1;owner:tick(true,1,101);eq(owner.death.poly,true);eq(c[11].max_hp,4)
 owner:tick(true,1,103);eq(c[11].max_hp,4)
 entities[1].tags={player_unit=true};owner:tick(false,1,104);eq(c[11].max_hp,4)
 -- Mode remains weak after a newer rule snapshot selects full respawn.
 local full=copy(rules);full.NT_death_penalty='full_respawn';view.snapshot={revision=2,rules=full};owner:receive(view);owner:tick(true,1,105);eq(c[11].max_hp,3.2);eq(c[11].hp,3.2)
 owner:checkpoint(1,50.25,-20.5);local saved=e.dofile('mods/noita_together2/files/g3/bank_json.lua').decode_saved(g.nt2_c1_checkpoint);eq(saved.x,'50.25');eq(saved.y,'-20.5')
 c[11].hp=-1;e.frame=200;owner:tick(true,1,200);eq(entities[1].x,50.25);eq(entities[1].y,-20.5)
end
-- Retained Glass Cannon removal/rebuild helpers preserve unrelated stacks.
do
 local e,c,entities,g,f=engine();local removed=0;local spawned=0
 local glass={id='GLASS_CANNON',ui_name='$perk_glass_cannon',func_remove=function(player)eq(player,1);removed=removed+1 end}
 e.perk_list={glass,{id='EXTRA_MONEY'}};e.get_perk_with_id=function(list,id)for _,p in ipairs(list)do if p.id==id then return p end end end
 e.get_perk_picked_flag_name=function(id)return 'PERK_'..id end
 f.PERK_GLASS_CANNON=true;f.PERK_EXTRA_MONEY=true;g.PERK_GLASS_CANNON_PICKUP_COUNT='2';g.PERK_EXTRA_MONEY_PICKUP_COUNT='3'
 e.perk_spawn_with_name=function()spawned=spawned+1;return e.EntityLoad('perk',0,0)end
 e.GameGetGameEffectCount=function()return 0 end
 local actual=e.dofile;e.dofile=function(path)if path:sub(1,5)=='data/'then return end;return actual(path)end
 e.dofile_once=e.dofile
 e.dofile('mods/noita_together2/files/c1/scripts/ban_perks_events.lua')
 eq(e.HandleBannedPerksEvent(),true);eq(spawned,2);eq(removed,1);eq(g.PERK_GLASS_CANNON_PICKUP_COUNT,'0');eq(f.PERK_GLASS_CANNON,nil);eq(g.PERK_EXTRA_MONEY_PICKUP_COUNT,'3');eq(f.PERK_EXTRA_MONEY,true)
 eq(e.HandleBannedPerksEvent(),true);eq(spawned,2);eq(removed,1)
 entities[1].tags={polymorphed=true};eq(e.HandleBannedPerksEvent(),false);eq(spawned,2)
end
-- Known-unissued polymorph death survives a cold Lua owner; ambiguous HP does not repeat.
do
 local e,c,entities,g=engine();local content=e.dofile('mods/noita_together2/files/g3/c1_content.lua').load(e);local serial=0;local sent={}
 local module=e.dofile('mods/noita_together2/files/g3/c1_local.lua')
 local function create()return module.new(e,function()serial=serial+1;return string.format('%08d-1111-4111-8111-111111111111',serial)end,function(body)sent[#sent+1]=assert(e.dofile('mods/noita_together2/files/g3/bank_json.lua').decode(body));return true end,content)end
 local r=copy(rules);r.NT_death_penalty='weak_respawn';local view={world=string.rep('c',64),initialized=true,memberCount=2,snapshot={revision=1,rules=r}}
 local owner=create();owner:receive(view);entities[1].tags={polymorphed=true};c[11].hp=-1;e.frame=100;owner:tick(true,1,100);eq(#sent,0);eq(#owner.queue,1);eq(owner.queue[1].payload.death.state,'respawning')
 local cold=create();e.frame=101;cold:receive(view);eq(cold.death~=nil,true);entities[1].tags={player_unit=true};e.frame=104;cold:tick(true,1,104);eq(c[11].max_hp,3.2);eq(sent[1].publish.senderDisposition,'applied');eq(sent[1].publish.eventId,owner.death.original.eventId)
 c[11].hp=-1;e.frame=200;local ambiguous=create();ambiguous:receive(view);ambiguous:tick(true,1,200);eq(c[11].max_hp,3.2);eq(c[11].hp,-1);eq(ambiguous.own_uncertain~=nil,true)
 c[11].hp=2;ambiguous:tick(true,1,201);eq(ambiguous.own_uncertain,nil);c[11].hp=-1;e.frame=300;ambiguous:tick(true,1,300);eq(c[11].max_hp,2.56)
end
-- Offline/Continue restores only owned wait/camera leases, after connection holds.
do
 local e,c,entities,g=engine();c[11].wait_for_kill_flag_on_death=true
 e.EntityAddComponent2(1,'VariableStorageComponent',{name='nt2_c1_wait',value_bool=false})
 local hold=e.EntityAddComponent2(1,'VariableStorageComponent',{name='nt2_g3_recovery'})
 local module=e.dofile('mods/noita_together2/files/g3/c1_local.lua');eq(module.recover(e,1),false);eq(c[11].wait_for_kill_flag_on_death,true)
 e.EntityRemoveComponent(1,hold)
 local camera=e.EntityLoad('mods/noita_together2/files/c1/entities/death_cam.xml',0,0);e.EntityAddComponent2(camera,'VariableStorageComponent',{name='nt2_c1_camera_owner',value_string='owned'})
 local other=e.EntityLoad('mods/noita_together2/files/c1/entities/death_cam.xml',0,0);e.EntityAddComponent2(other,'VariableStorageComponent',{name='nt2_c1_camera_owner',value_string='foreign'})
 g.nt2_c1_camera='owned';eq(module.recover(e,1),true);eq(c[11].wait_for_kill_flag_on_death,false);eq(e.EntityGetIsAlive(camera),false);eq(e.EntityGetIsAlive(other),true);eq(e.camera_free,false)
end
-- A local mailbox copy is not a durable app ACK. Save the source and life until that ACK.
do
 local e,c,entities,g=engine();local copied=0;local content=e.dofile('mods/noita_together2/files/g3/c1_content.lua').load(e)
 local owner=e.dofile('mods/noita_together2/files/g3/c1_local.lua').new(e,function()return 'aaaaaaaa-1111-4111-8111-111111111111'end,function()copied=copied+1;return true end,content)
 local view={world=string.rep('b',64),initialized=true,memberCount=2,snapshot={revision=1,rules=copy(rules)}};owner:receive(view);owner:tick(true,1,100)
 local original=owner:capture('heart',{});owner:submit(original);owner:tick(true,1,101);eq(#owner.queue,1);eq(copied,1)
 owner:tick(false,1,102);eq(#owner.queue,1)
 view.localAccepted={original.eventId..':0'};owner:receive(view);owner:tick(true,1,103);eq(#owner.queue,0)
 owner.life={deathEventId=original.eventId,state='playing'};owner:tick(true,1,104);eq(owner.life~=nil,true)
 view.localAccepted={original.eventId..':life'};owner:receive(view);owner:tick(true,1,105);eq(owner.life,nil)
end
-- Restore classification is committed to the same mailbox before the first
-- tick; current-session completions added afterward keep their own disposition.
do
 local e,c,entities,g=engine();local json=e.dofile('mods/noita_together2/files/g3/bank_json.lua');local content=e.dofile('mods/noita_together2/files/g3/c1_content.lua').load(e)
 local module=e.dofile('mods/noita_together2/files/g3/c1_local.lua');local sent={};local serial=0
 local function create()local owner=module.new(e,function()serial=serial+1;return string.format('%08d-1111-4111-8111-111111111111',serial)end,function(body)sent[#sent+1]=assert(json.decode(body));return true end,content);owner.context=string.rep('01',16);return owner end
 local view={world=string.rep('b',64),initialized=true,memberCount=2,snapshot={revision=1,rules=copy(rules)}}
 local first=create();first:receive(view);first:tick(true,1,100);local old=first:capture('heart',{});first:submit(old);eq(assert(json.decode_saved(g.nt2_c1_outgoing)).queue[1].senderDisposition,'applied')
 local restored=create();restored:receive(view);eq(restored.queue[1].senderDisposition,'uncertain');eq(assert(json.decode_saved(g.nt2_c1_outgoing)).queue[1].senderDisposition,'uncertain')
 restored:tick(true,1,101);eq(restored.queue[1].senderDisposition,'uncertain');eq(sent[1].publish.senderDisposition,'uncertain');eq(sent[1].publish.eventId,old.eventId);eq(c[11].hp,2);eq(c[11].max_hp,4)
 local fresh=restored:capture('heart',{});restored:submit(fresh);restored:receive(view);restored:tick(true,1,102);eq(sent[#sent].publish.senderDisposition,'uncertain');eq(restored.queue[2].senderDisposition,'applied');eq(assert(json.decode_saved(g.nt2_c1_outgoing)).queue[2].senderDisposition,'applied')
 view.localAccepted={old.eventId..':0'};restored:receive(view);restored:tick(true,1,103);eq(sent[#sent].publish.eventId,fresh.eventId);eq(sent[#sent].publish.senderDisposition,'applied')
 local same_context=create();same_context:receive(view);eq(assert(json.decode_saved(g.nt2_c1_outgoing)).queue[1].senderDisposition,'uncertain');same_context:tick(true,1,104);eq(sent[#sent].publish.eventId,fresh.eventId);eq(sent[#sent].publish.senderDisposition,'uncertain')
end
-- A component/form return reuses its original wait marker, not a new true snapshot.
do
 local e,c,entities,g=engine();local module=e.dofile('mods/noita_together2/files/g3/c1_local.lua');local content=e.dofile('mods/noita_together2/files/g3/c1_content.lua').load(e)
 local owner=module.new(e,function()return 'aaaaaaaa-1111-4111-8111-111111111111'end,function()return true end,content)
 owner:receive({world=string.rep('b',64),initialized=true,memberCount=2,snapshot={revision=1,rules=copy(rules)}});owner:tick(true,1,100);local marker=owner.wait.marker
 owner.wait=nil;owner:tick(true,1,101);eq(owner.wait.marker,marker);eq(owner.wait.old,false);owner:close();eq(c[11].wait_for_kill_flag_on_death,false)
end
-- Its pre-existing wait lease survives a metadata gap without granting a heal.
do
 local e,c=engine();local content=e.dofile('mods/noita_together2/files/g3/c1_content.lua').load(e)
 local owner=e.dofile('mods/noita_together2/files/g3/c1_local.lua').new(e,function()return 'aaaaaaaa-1111-4111-8111-111111111111'end,function()return true end,content)
 owner:receive({world=string.rep('b',64),initialized=true,memberCount=2,snapshot={revision=1,rules=copy(rules)}});owner:tick(true,1,100)
 owner.ready=false;c[11].hp=-1;c[13].enabled=false;eq(owner:owns_death(1),true);owner:tick(false,1,101);eq(c[11].hp,-1);eq(c[13].enabled,false)
 owner.wait=nil;eq(not owner:owns_death(1),true)
end
-- Protected post-entry completion observes a genuine death but never heals,
-- releases Controls or issues a received effect while held; its genuine own respawn completes.
do
 local e,c,entities,g=engine();local copied={};local content=e.dofile('mods/noita_together2/files/g3/c1_content.lua').load(e)
 local owner=e.dofile('mods/noita_together2/files/g3/c1_local.lua').new(e,function()return 'aaaaaaaa-1111-4111-8111-111111111111'end,function(body,completion)copied[#copied+1]={body=body,completion=completion};return true end,content)
 local r=copy(rules);r.NT_death_penalty='weak_respawn';local view={world=string.rep('b',64),initialized=true,memberCount=2,snapshot={revision=1,rules=r},completion=true}
 owner:receive(view);c[11].hp=-1;c[13].enabled=false;owner:tick(false,1,100);eq(c[11].hp,3.2);eq(c[11].max_hp,3.2);eq(c[13].enabled,false);eq(#copied,1);eq(copied[1].completion,true);eq(owner.death,nil);eq(owner.life.state,'playing')
 local original=owner.queue[1];view.localAccepted={original.eventId..':0'};owner:receive(view);owner:tick(false,1,101);eq(#owner.queue,0);eq(c[11].hp,3.2)
 view.completion=false;owner:receive(view);owner:tick(true,1,102);eq(c[11].hp,3.2);eq(c[11].max_hp,3.2);eq(c[13].enabled,false);eq(owner.life.state,'playing')
end
-- Explicit offline restores connection ownership first, then the C1 wait lease.
do
 local e,c,entities,g=engine();local module=e.dofile('mods/noita_together2/files/g3/c1_local.lua');local content=e.dofile('mods/noita_together2/files/g3/c1_content.lua').load(e)
 local owner=module.new(e,function()return 'aaaaaaaa-1111-4111-8111-111111111111'end,function()return true end,content);owner:receive({world=string.rep('b',64),initialized=true,memberCount=2,snapshot={revision=1,rules=copy(rules)}});owner:tick(true,1,100)
 local connection=e.EntityAddComponent2(1,'VariableStorageComponent',{name='nt2_g3_recovery'});owner:close();eq(c[11].wait_for_kill_flag_on_death,true);eq(module.recover(e,1),false)
 e.EntityRemoveComponent(1,connection);eq(module.recover(e,1),true);eq(c[11].wait_for_kill_flag_on_death,false)
end
-- Cold callback order may expose a player before readable WorldState globals.
do
 local e,c,entities,g=engine();local module=e.dofile('mods/noita_together2/files/g3/c1_local.lua');local content=e.dofile('mods/noita_together2/files/g3/c1_content.lua').load(e);local get=e.GlobalsGetValue;local set=e.GlobalsSetValue
 e.GameGetWorldStateEntity=function()return 0 end;e.GlobalsGetValue=function()error('Injected early WorldState')end;e.GlobalsSetValue=e.GlobalsGetValue
 local owner=module.new(e,function()return nil end,function()error('No early publication')end,content);eq(module.recover(e,1),false);owner:tick(false,1,0);eq(owner.ready,false);eq(c[11].wait_for_kill_flag_on_death,nil)
 e.GlobalsGetValue=get;e.GlobalsSetValue=set;e.GameGetWorldStateEntity=function()return 1 end;eq(module.recover(e,1),true)
end
-- Removing our old camera sprite cannot take over a newer camera lease.
do
 local e,c,entities,g=engine();local content=e.dofile('mods/noita_together2/files/g3/c1_content.lua').load(e);local owner=e.dofile('mods/noita_together2/files/g3/c1_local.lua').new(e,function()return nil end,function()return true end,content)
 local entity=e.EntityLoad('mods/noita_together2/files/c1/entities/death_cam.xml',0,0);e.EntityAddComponent2(entity,'VariableStorageComponent',{name='nt2_c1_camera_owner',value_string='old-owned'});owner.camera={entity=entity,id='old-owned'};g.nt2_c1_camera='new-owner';e.camera_free=true;owner:close_camera();eq(e.EntityGetIsAlive(entity),false);eq(e.camera_free,true);eq(g.nt2_c1_camera,'new-owner')
end
-- A repeated physical callback after a partial local hook failure retains the
-- same original and cannot add HP a second time. Main-issued token is injected.
do
 local e,c,entities,g,f=engine();e.frame=100;f.NT_sync_hearts=true;local json=e.dofile('mods/noita_together2/files/g3/bank_json.lua')
 g.nt2_c1_authority=assert(json.encode_saved({world=string.rep('a',64),revision=1,count=2,frame=100,player=1,playing=true,context=string.rep('01',16)}))
 g.nt2_c1_tokens=assert(json.encode_saved({world=string.rep('a',64),context=string.rep('01',16),ids={'11111111-1111-4111-8111-111111111111'}}))
 e.item_pickup=function()error('Shared path required')end;e.dofile('mods/noita_together2/files/c1/append/heart_append.lua');local spawn=e.EntityLoad;e.EntityLoad=function()error('Injected post-HP particle failure')end
 local ok=pcall(e.item_pickup,2,1,'heart');eq(ok,false);eq(c[11].max_hp,4.5);eq(e.EntityGetIsAlive(2),true)
 local authority=assert(json.decode_saved(g.nt2_c1_authority));authority.playing=false;g.nt2_c1_authority=assert(json.encode_saved(authority));e.EntityLoad=spawn;ok=pcall(e.item_pickup,2,1,'heart');eq(ok,true);eq(c[11].max_hp,4.5);eq(#assert(json.decode_saved(g.nt2_c1_outgoing)).queue,1)
end
-- Winning remains distinct from shared death/end effects.
do
 local e,c=engine();local content=e.dofile('mods/noita_together2/files/g3/c1_content.lua').load(e);local effects=e.dofile('mods/noita_together2/files/g3/c1_effects.lua').new(e,content);effects.terminal=function()error('Remote win ended the run')end
 local status=effects:apply(1,{payload={death={state='won',cause='normal'}},sourceName='Winner'},rules,2,100,rules);eq(status,'skipped');eq(e.game_overs or 0,0);eq(c[11].hp,2)
end
-- Legacy FinishRun parity, canonical protected completion and saved ambiguity.
for _,mode in ipairs({'end','weak_respawn','full_respawn'})do
 local e,c,entities,g=engine();local sent={};local content=e.dofile('mods/noita_together2/files/g3/c1_content.lua').load(e)
 local owner=e.dofile('mods/noita_together2/files/g3/c1_local.lua').new(e,function()return '11111111-1111-4111-8111-111111111111'end,function(v)sent[#sent+1]=v;return true end,content)
 local captured=copy(rules);captured.NT_death_penalty=mode
 local view={world=string.rep('b',64),initialized=true,memberCount=2,snapshot={revision=1,rules=copy(rules)},event={eventId='22222222-1111-4111-8111-111111111111',sequence='1',payload={death={state='terminal',cause='normal'}},sourceName='Peer',capturedRules=captured},permit='permit'}
 owner:receive(view);owner:tick(true,1,100)
 eq(e.game_overs or 0,mode=='full_respawn'and 0 or 1)
 if mode~='full_respawn' then eq(c[11].hp,0);eq(c[11].kill_now,true);eq(#owner.queue,0);owner:tick(true,1,101);eq(e.game_overs,1)end
end
do
 local e,c,entities,g=engine();local content=e.dofile('mods/noita_together2/files/g3/c1_content.lua').load(e)
 local function fresh()return e.dofile('mods/noita_together2/files/g3/c1_local.lua').new(e,function()error('Remote end recaptured death')end,function()return true end,content)end
 local owner=fresh();local module=e.dofile('mods/noita_together2/files/g3/c1_local.lua')
 owner:tick(false,1,99);eq(e.game_overs or 0,0);eq(owner.terminal_state,nil)
 g.nt2_rejoin_local_world_v1='nt2h1:owned-world'
 owner.scope=string.rep('c',64);owner:finish_run(1,'player_death_end');eq(e.game_overs,1);eq(#owner.queue,0)
 eq(module.finish_run(e,1,nil,'player_death_end'),false);eq(e.game_overs,1)
 local json=e.dofile('mods/noita_together2/files/g3/bank_json.lua')
 g.nt2_c1_finished=assert(json.encode_saved({association=g.nt2_rejoin_local_world_v1,state='applying'}))
 eq(module.finish_run(e,1,nil,'player_death_end'),false);eq(e.game_overs,1)
 g.nt2_rejoin_local_world_v1='nt2h1:different-world'
 eq(module.finish_run(e,1,nil,'player_death_end'),true);eq(e.game_overs,2)
 local restored=fresh();local nextview={world=string.rep('d',64),initialized=true,memberCount=2,snapshot={revision=1,rules=copy(rules)}};restored:receive(nextview);eq(restored.terminal_state,false)
end
-- A retained owner can close while a same-process restored WorldState is absent.
-- Preserve saved markers for later recovery; closing must revoke local authority
-- without touching unavailable Globals or old-world entity/component leases.
do
 local e,c,entities,g=engine();local module=e.dofile('mods/noita_together2/files/g3/c1_local.lua');local content=e.dofile('mods/noita_together2/files/g3/c1_content.lua').load(e)
 local owner=module.new(e,function()return nil end,function()error('Close cannot publish')end,content)
 owner.ready=true;owner.playing=true;owner.camera={id='retained-camera',entity=1};owner.wait={player=1,damage=11}
 g.nt2_c1_camera='retained-camera';g.nt2_c1_authority='saved-authority';g.nt2_c1_tokens='saved-tokens'
 e.GameGetWorldStateEntity=function()return 0 end
 e.GlobalsGetValue=function()error('No Globals while WorldState is absent')end;e.GlobalsSetValue=e.GlobalsGetValue
 e.EntityGetIsAlive=function()error('No old-entity cleanup against absent WorldState')end
 owner:close();eq(owner.playing,false);eq(owner.ready,false);eq(owner.camera,nil);eq(owner.wait,nil)
 eq(g.nt2_c1_camera,'retained-camera');eq(g.nt2_c1_authority,'saved-authority');eq(g.nt2_c1_tokens,'saved-tokens')
 e.GameGetWorldStateEntity=function()error('WorldState unavailable')end;owner.ready=true;owner.playing=true;owner:close();eq(owner.ready,false);eq(owner.playing,false)
end
print('PASS '..checks..' C1 production hook/handler checks (injected engine)')
