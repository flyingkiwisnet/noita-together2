local repo=assert(os.getenv('NT2_ITEM_REPO'))
local paths={
'files/c2/local.lua','files/c2/world.lua','files/c2/hook_api.lua','files/c2/storage.lua','files/c2/hourglass.lua','files/c2/append/boss_arena.lua','files/c2/append/sampo_append.lua','files/c2/append/snowcastle_cavern.lua','files/c2/scripts/hourglass_events.lua','files/c2/scripts/sampo/sampo_aura.lua','files/c2/scripts/sampo/sampo_aura_attractor_fix.lua','files/g3/bank_native.lua','init-g3-seed-loot.lua'}
for _,p in ipairs(paths)do assert(loadfile(repo..'/mod/noita_together2/'..p))end
print('PASS '..#paths..' production Lua syntax checks')
local repo=assert(os.getenv('NT2_ITEM_REPO'));NT2_C1_TEST_ENGINE_ONLY=true
local fixture=dofile(repo..'/apps/desktop/tools/c1-local-test.lua');NT2_C1_TEST_ENGINE_ONLY=nil
local checks=0;local function eq(a,b)assert(a==b,tostring(a)..' ~= '..tostring(b));checks=checks+1 end
local function engine()
 local e,c,entities,g,f=fixture.engine();e.SessionNumbersGetValue=function()return '0'end
 e.EntityGetClosestWithTag=function()return 0 end;e.EntitySetComponentIsEnabled=function(_,part,value)c[part].enabled=value end;e.ComponentGetIsEnabled=function(part)return c[part].enabled~=false end
 e.ComponentHasTag=function(part,tag)return c[part]._tags==tag or c[part].component_tags and c[part].component_tags[tag]or false end
 e.ComponentAddTag=function(part,tag)c[part].component_tags=c[part].component_tags or{};c[part].component_tags[tag]=true end
 e.ComponentRemoveTag=function(part,tag)if c[part].component_tags then c[part].component_tags[tag]=nil end end
 e.EntitySetComponentsWithTagEnabled=function()end;e.PhysicsSetStatic=function()end;e.GamePlaySound=function()end;e.GameScreenshake=function(value)e.shake=value end
 e.GameTriggerMusicFadeOutAndDequeueAll=function()e.music=(e.music or 0)+1 end;e.GameTriggerMusicEvent=function()end
 e.EntityCreateNew=function()return e.EntityLoad('empty',0,0)end
 e.CellFactory_GetType=function(material)return 4 end;e.EntityGetInRadiusWithTag=function(_,_,_,tag)return e.EntityGetWithTag(tag)end
 e.EntityAddChild=function(parent,id)entities[id].parent=parent end;e.EntityGetParent=function(id)return entities[id].parent or 0 end
 local load=e.EntityLoad;e.loads={};e.EntityLoad=function(path,x,y)local id=load(path,x,y);e.loads[#e.loads+1]={path=path,x=x,y=y,id=id};if path:find('hourglass_effect_trigger',1,true)then e.EntityAddTag(id,'hourglass_entity');e.EntityAddComponent2(id,'MaterialAreaCheckerComponent',{})end;return id end
 e.EntityIngestMaterial=function(_,material,count)e.ingested=count end
 e.GameGetWorldStateEntity=function()return 2 end
 e.ComponentObjectGetValue2=function(part,object,key)return (c[part][object]or{})[key]or 1 end
 e.ComponentObjectSetValue2=function(part,object,key,value)c[part][object]=c[part][object]or{};c[part][object][key]=value end
 return e,c,entities,g,f
end
local e,c,entities,g,f=engine();local json=e.dofile('mods/noita_together2/files/g3/bank_json.lua');local store=e.dofile('mods/noita_together2/files/c2/storage.lua')
local world=e.dofile('mods/noita_together2/files/c2/world.lua');eq(world.near(199,0,0,0),true);eq(world.near(200,0,0,0),false);eq(world.near(201,0,0,0),false)
for _,v in ipairs({{-1,0},{0,0},{13,13},{14,13},{32,13},{33,33},{34,33}})do eq(world.name_index(v[1]),v[2])end
local effects=e.dofile('mods/noita_together2/files/c2/hourglass.lua').new(e)
local total=0;for _ in pairs(effects.entries)do total=total+1 end;eq(total,7)
eq(effects:register('speed','water',function()end),false);eq(effects:register('custom','water','not a function'),false);eq(effects:register('custom','water',nil,true),true);eq(effects:apply('custom',1,'Peer'),'skipped')
e.EntityAddComponent2(1,'IngestionComponent',{ingestion_size=100})
for _,name in ipairs({'ambrosia','berserk','charm','confusion','speed','levitation','speed_and_levitation'})do
 local state=effects:apply(name,1,'Peer');eq(state,'applied')
 if name=='berserk'or name=='speed'or name=='levitation'or name=='speed_and_levitation'then eq(c[e.EntityGetFirstComponent(e.last_effect,'GameEffectComponent')].frames,7200)end
end
eq(e.ingested,300)
local next_id=0;local sent={};local function uid()next_id=next_id+1;return string.format('%08d-1111-4111-8111-111111111111',next_id)end
local module=e.dofile('mods/noita_together2/files/c2/local.lua');local function create()local owner=module.new(e,uid,function(body)sent[#sent+1]=assert(json.decode(body));return true end,function()end);owner.context=string.rep('01',16);return owner end
local owner=create();local view={world=string.rep('a',64),principal='self',worldToken='world',progression=0,initialized=true,snapshot={sequence='1',rules={revision=1,rules={NT_sync_steve=true}},completions={}}}
do
 local unbound=create();local before=#sent;unbound:tick(false,1,1)
 eq(#sent,before) -- Unknown mode/world must not occupy the shared completion lane.
 local bound={world=view.world,principal='self',worldToken='world',initialized=true,snapshot=view.snapshot}
 eq(unbound:receive(bound),true);unbound:tick(false,1,2)
 eq(sent[#sent].kind,'c2_completion');eq(sent[#sent].state.progression,0)
end
eq(owner:receive(view),true);owner:tick(true,1,100)
local event={eventId='11111111-1111-4111-8111-111111111111',sequence='1',recipientProgression=0,sourceName='Peer',payload={steve={progression=0}}}
view.event=event;owner:receive(view);owner:tick(true,1,100);eq(sent[#sent].apply.eventId,event.eventId);eq(g.TEMPLE_SPAWN_GUARDIAN,nil)
view.permit='p';owner:receive(view);owner:tick(true,1,100);eq(g.TEMPLE_SPAWN_GUARDIAN,'1');eq(e.shake,150);eq(sent[#sent].receipt.state,'applied')
owner:tick(true,1,100);eq(sent[#sent].receipt.state,'applied')
-- Save restores an applied receipt without replay; an issued boundary is uncertain.
local xml=dofile(repo..'/apps/desktop/tools/noita-globals-xml.lua');g.nt2_rejoin_local_world_v1='nt2h1:1:61';g.after='preserved'
local loaded=assert(xml.restore(xml.serialize(g)));for k in pairs(g)do g[k]=nil end;for k,v in pairs(loaded)do g[k]=v end
local cold=create();cold:receive(view);cold:tick(true,1,101);eq(sent[#sent].receipt.state,'applied');eq(g.after,'preserved');eq(g.nt2_rejoin_local_world_v1,'nt2h1:1:61')
store.write(e,'receipt',{world=view.world,eventId=event.eventId,sequence='1',state='applying',permit='p'})
local ambiguous=create();ambiguous:receive(view);ambiguous:tick(true,1,102);eq(sent[#sent].receipt.state,'uncertain')
-- Missing actor and current receiver disable never acquire an effect permit.
view.event={eventId=uid(),sequence='2',recipientProgression=0,sourceName='Peer',payload={steve={progression=0}}};view.permit=nil
entities[1].tags.player_unit=nil;local before=#sent;ambiguous:receive(view);ambiguous:tick(true,nil,103);eq(#sent,before);entities[1].tags.player_unit=true
view.snapshot.rules.rules.NT_sync_steve=false;view.permit='next';ambiguous:receive(view);ambiguous:tick(true,1,104);eq(sent[#sent].receipt.state,'skipped');eq(sent[#sent].receipt.reason,'sharing_disabled')
-- Actual competing callbacks share one immutable bound activation, no source effect.
view.event=nil;view.permit=nil;view.snapshot.rules.rules.NT_sync_steve=true;ambiguous:receive(view);e.frame=105;ambiguous:tick(true,1,105)
local anchor=e.EntityCreateNew();store.mark(e,anchor,'nt2_c2_hourglass_group',{world=view.world,context=ambiguous.context,progression=0,activationId=uid(),claimed=false})
e.dofile('mods/noita_together2/files/c2/scripts/hourglass_events.lua')
local first=e.CreateHourglassEntity(0,0,'speed','magic_liquid_movement_faster',anchor);local second=e.CreateHourglassEntity(0,0,'berserk','magic_liquid_berserk',anchor)
e.GetUpdatedEntityID=function()return first end;e.material_area_checker_success();local queue=store.read(e,'outgoing').queue;eq(#queue,1);eq(queue[1].payload.hourglass.effectId,'speed');eq(queue[1].senderDisposition,'applied')
e.GetUpdatedEntityID=function()return second end;e.material_area_checker_success();eq(#store.read(e,'outgoing').queue,1);eq(e.EntityGetIsAlive(second),false)
for key,value in pairs(g)do if key:match('^nt2_c2_')then eq(value:find('["&<>%z\1-\31]'),nil)end end
for _,parts in pairs(entities)do for _,part in ipairs(parts.parts.VariableStorageComponent or{})do eq(c[part].value_string:find('["&<>%z\1-\31]'),nil)end end
print('PASS '..checks..' C2 injected-engine production hook/effect/owner/save checks')
-- Restricted weak-ambrosia callbacks: stacking, foreign changes, duplicate cleanup,
-- serialized Continue markers and actor/component replacement.
do
 local e,c,entities,g=engine();local restricted=dofile(repo..'/apps/desktop/tools/c1-restricted-context.lua')
 local h=restricted(e,function(path)local file=assert(io.open(repo..'/'..path:gsub('^mods/','mod/'),'rb'));local body=file:read('*a');file:close();return body end)
 local function effect()local id=e.EntityLoad('effect',0,0);e.EntityAddChild(1,id);return id end
 local a,b=effect(),effect();local updated=a;h.GetUpdatedEntityID=function()return updated end
 local function run(name)h.dofile('mods/noita_together2/files/c2/effects/weak_ambrosia_'..name..'.lua')end
 eq(h.require,nil);eq(h.ffi,nil);run('start');eq(c[11].damage_multipliers.melee,.5);run('start');eq(c[11].damage_multipliers.melee,.5)
 updated=b;run('start');eq(c[11].damage_multipliers.melee,.25)
 c[11].damage_multipliers.melee=c[11].damage_multipliers.melee*.8
 updated=a;run('end');eq(c[11].damage_multipliers.melee,.4);run('end');eq(c[11].damage_multipliers.melee,.4)
 updated=b;run('end');eq(c[11].damage_multipliers.melee,.8);eq(c[11].damage_multipliers.holy,nil)
 local replacement=effect();updated=replacement;run('start');eq(c[11].damage_multipliers.melee,.4)
 entities[replacement].parent=2;run('end');eq(c[11].damage_multipliers.melee,.4)
 entities[replacement].parent=1;entities[1].parts.DamageModelComponent={e.EntityAddComponent2(2,'DamageModelComponent',{})};run('end');eq(c[11].damage_multipliers.melee,.4)
end
-- Arena factory ownership, deferred assets, one battle and no boss resurrection.
do
 local e,c,entities,g,flags=engine();local store=e.dofile('mods/noita_together2/files/c2/storage.lua');store.write(e,'mode',{managed=true})
 local vanilla=0;e.spawn_items=function()vanilla=vanilla+1 end;e.dofile('mods/noita_together2/files/c2/append/boss_arena.lua');e.spawn_items(100,200);eq(vanilla,0);eq(#e.loads,5)
 e.spawn_items(100,200);eq(#e.loads,5)
 local saved={world=string.rep('b',64),progression=0};local count=0;local arena=e.dofile('mods/noita_together2/files/c2/world.lua').new(e,function()count=count+1 end)
 eq(arena:bind(saved,uid,'context'),true);local boss=arena:owned(saved,'boss');local reference=arena:owned(saved,'reference');eq(boss~=nil,true);eq(reference~=nil,true)
 entities[reference].alive=false;eq(arena:battle(saved,1),false);eq(e.music,nil);entities[reference].alive=true
 eq(arena:battle(saved,1),true);eq(e.music,1);eq(#e.loads,9);eq(e.loads[6].x,100-235);eq(e.loads[6].y,200-73);eq(e.loads[9].x,100+257);eq(e.loads[9].y,200+262)
 eq(arena:battle(saved,1),true);eq(#e.loads,9);entities[boss].alive=false;eq(arena:battle(saved,1),true);eq(#e.loads,9)
end
print('PASS '..checks..' total C2 focused production checks (injected engine)')
-- Aura uses the actual two-frame script decisions and only its owned entities.
do
 local e,c,entities,g,flags=engine();local store=e.dofile('mods/noita_together2/files/c2/storage.lua');store.write(e,'mode',{managed=true})
 e.spawn_items=function()error('managed factory')end;e.dofile('mods/noita_together2/files/c2/append/boss_arena.lua');e.spawn_items(0,0)
 local saved={world=string.rep('c',64),progression=0};local arena=e.dofile('mods/noita_together2/files/c2/world.lua').new(e,function()store.write(e,'state',saved)end);arena:bind(saved,uid,'ctx');arena:scope(saved,{encounterId='encounter'},'ctx')
 local aura=arena:owned(saved,'aura');local disabled=arena:owned(saved,'disabled');local emitter=e.EntityAddComponent2(aura,'ParticleEmitterComponent',{attractor_force=-.4,area_circle_radius=190,velocity_always_away_from_center=1});e.EntityAddComponent2(aura,'LuaComponent',{_tags='nt_aura_script'})
 store.write(e,'grant',{playing=false,frame=100,player=1,world=saved.world,context='ctx',progression=0,encounterId='encounter',arenaToken=saved.arenaToken,stage='unlocked',rulesRevision=1})
 e.GetUpdatedEntityID=function()return aura end;local function run()e.dofile('mods/noita_together2/files/c2/scripts/sampo/sampo_aura.lua')end
 run();eq(c[emitter].area_circle_radius,190)
 local grant=store.read(e,'grant');grant.playing=true;store.write(e,'grant',grant)
 run();eq(c[emitter].area_circle_radius,184);eq(c[emitter].velocity_always_away_from_center,1.1)
 for i=1,33 do run()end
 eq(e.EntityGetIsAlive(aura),false);eq(e.EntityGetIsAlive(disabled),false);eq(store.read(e,'state').realSampo,true)
 local sampo=arena:owned(saved,'sampo');eq(sampo~=nil,true);local n=#e.loads;run();eq(#e.loads,n)
 store.write(e,'tokens',{world=saved.world,context='ctx',ids={uid()}})
 local vanilla=0;e.item_pickup=function()vanilla=vanilla+1 end;e.dofile('mods/noita_together2/files/c2/append/sampo_append.lua');e.orb_count=33;e.item_pickup(sampo,1,'sampo');eq(vanilla,0);eq(flags.NT_got_sampo,true);eq(store.read(e,'outgoing').queue[1].payload.pickup.orbCount,33)
 e.item_pickup(sampo,1,'sampo');eq(#store.read(e,'outgoing').queue,1)
end
print('PASS '..checks..' total C2 focused production checks (injected engine)')
-- Captured winning progression is immutable even if local NG+ advances later.
do
 local e,c,entities,g=engine();e.NT2_C2_ENABLED=true;local ordinal='7';e.SessionNumbersGetValue=function()return ordinal end
 local owner=e.dofile('mods/noita_together2/files/g3/c1_local.lua').new(e,uid,function()return true end,e.dofile('mods/noita_together2/files/g3/c1_content.lua').load(e))
 owner.scope=string.rep('d',64);owner.context='ctx';owner.ready=true;owner.playing=true;owner.revision=1
 local original=owner:capture('death',{state='won',cause='normal'});eq(original.payload.death.progression,7);ordinal='8';eq(original.payload.death.progression,7)
end
-- Corrupt ownership/receipts cannot be interpreted as fresh gameplay.
do
 local e,c,entities,g=engine();local module=e.dofile('mods/noita_together2/files/c2/local.lua');local json=e.dofile('mods/noita_together2/files/g3/bank_json.lua')
 g.nt2_c2_state='nt2h1:2:zzzz';local sent=0;local owner=module.new(e,uid,function()sent=sent+1;return true end)
 owner:receive({world=string.rep('e',64),progression=0,initialized=true,snapshot={rules={revision=1,rules={NT_sync_steve=true}}}});owner:tick(true,1,100);eq(owner.corrupt,true);eq(sent,0);eq(g.nt2_c2_state,'nt2h1:2:zzzz')
 local effect=e.EntityLoad('effect',0,0);e.EntityAddChild(1,effect);e.EntityAddComponent2(effect,'VariableStorageComponent',{name='nt2_c2_ambrosia',value_string='broken'})
 e.GetUpdatedEntityID=function()return effect end;e.dofile('mods/noita_together2/files/c2/effects/weak_ambrosia_start.lua');eq(c[11].damage_multipliers,nil)
end
local output=os.getenv('NT2_C2_TEST_OUTPUT')
if output then
 local xml=dofile(repo..'/apps/desktop/tools/noita-globals-xml.lua');local codec=e.dofile('mods/noita_together2/files/rejoin/local_world.lua')
 g.nt2_rejoin_local_world_v1=assert(codec.encode_saved('actual-association<&"'));g.nt2_c1_authority=assert(json.encode_saved({name='C1 adjacent <&"'}));g.before='before';g.after='after'
 local f=assert(io.open(output..'/globals.xml','wb'));f:write(xml.serialize(g));f:close()
 local rows={'<Entity>'};local count=0
 for _,entity in pairs(entities)do for _,part in ipairs(entity.parts.VariableStorageComponent or{})do rows[#rows+1]='<VariableStorageComponent name="'..c[part].name..'" value_string="'..c[part].value_string..'"/>';count=count+1 end end
 rows[#rows+1]='</Entity>';f=assert(io.open(output..'/components.xml','wb'));f:write(table.concat(rows));f:close();eq(count>0,true)
 f=assert(io.open(output..'/expected.json','wb'));f:write(assert(json.encode({globals=g,components=count},{bytes=131072,nodes=16384})));f:close()
end
print('PASS '..checks..' total C2 focused production checks (injected engine)')
-- The composed injected engine must preserve ordinary motion and optional-arg
-- arity outside explicit exact-distance actions.
do
 local e,c,entities=engine();local original=e.EntityGetFirstComponentIncludingDisabled
 e.EntityGetFirstComponentIncludingDisabled=function(...)assert(select('#',...)==2 or type(select(3,...))=='string');return original(...)end
 local oldenv=os.getenv('NT2_C3_REPO');assert(oldenv,'Set NT2_C3_REPO for composed adapter regression')
 local adapter=dofile(repo..'/apps/desktop/tools/lifecycle-lc1-c2.lua').new(e,{frame=100},{current=function()return 1 end},function()end,function()end,repo)
 eq(e.EntityGetFirstComponentIncludingDisabled(1,'DamageModelComponent'),11)
 e.EntityLoad('mods/noita_together2/files/c2/entities/sampo/sampo_aura.xml',0,80);adapter:tick({c2={ready=false,playing=false,scope='fixture'}});eq(e.GlobalsGetValue('nt2_c2_fixture_engine',''):sub(1,6),'nt2h1:')
 e.EntitySetTransform(1,100,20);entities[1].x=101;eq(e.EntityGetTransform(1),101)
 local ingestion=e.EntityGetFirstComponentIncludingDisabled(1,'IngestionComponent');eq(e.ComponentGetValue2(ingestion,'ingestion_size'),0);eq(e.dofile('mods/noita_together2/files/c2/hourglass.lua').new(e):apply('confusion',1,'Peer'),'applied');eq(e.ComponentGetValue2(ingestion,'ingestion_size'),0)
end
print('PASS '..checks..' total C2 focused production and adapter checks (injected engine)')

-- Saved ownership follows persistent tokens when Noita remaps entity IDs.
do
 local e,c,entities,g=engine();local store=e.dofile('mods/noita_together2/files/c2/storage.lua');store.write(e,'mode',{managed=true});e.spawn_items=function()end;e.dofile('mods/noita_together2/files/c2/append/boss_arena.lua');e.spawn_items(0,0)
 local saved={world=string.rep('f',64),progression=0};local arena=e.dofile('mods/noita_together2/files/c2/world.lua').new(e,function()end);arena:bind(saved,uid,'old');local token=saved.arenaToken;local old=saved.group;local row=store.component(e,old,'nt2_c2_arena')
 local replacement=e.EntityCreateNew();e.EntityAddTag(replacement,'nt2_c2_arena_group');store.mark(e,replacement,'nt2_c2_arena',row);e.EntityKill(old)
 eq(arena:bind(saved,uid,'new'),true);eq(saved.group,replacement);eq(saved.arenaToken,token);eq(arena:owned(saved,'boss')~=nil,true)
 local effect=e.EntityLoad('ambrosia',0,0);e.EntityAddChild(1,effect);e.EntityAddComponent2(effect,'VariableStorageComponent',{name='nt2_c2_ambrosia_key',value_string=uid()});e.GetUpdatedEntityID=function()return effect end
 e.dofile('mods/noita_together2/files/c2/effects/weak_ambrosia_start.lua');eq(c[11].damage_multipliers.melee,.5)
 local actor=e.EntityLoad('restored actor',0,0);local values={};for k,v in pairs(c[11].damage_multipliers)do values[k]=v end
 local damage=e.EntityAddComponent2(actor,'DamageModelComponent',{damage_multipliers=values,component_tags=c[11].component_tags});entities[effect].parent=actor
 e.dofile('mods/noita_together2/files/c2/effects/weak_ambrosia_end.lua');eq(c[damage].damage_multipliers.melee,1);e.dofile('mods/noita_together2/files/c2/effects/weak_ambrosia_end.lua');eq(c[damage].damage_multipliers.melee,1);eq(next(c[damage].component_tags),nil)
end
print('PASS '..checks..' total C2 focused production and adapter checks (injected engine)')
-- Explicit far-distance tests walk, including the return to the ordinary pose.
do
 local e,c,entities=engine();entities[1].x=1000;local plan={kind='distance',x=201};local json=e.dofile('mods/noita_together2/files/g3/bank_json.lua')
 local state={frame=100};local adapter=dofile(repo..'/apps/desktop/tools/lifecycle-lc1-c2.lua').new(e,state,{current=function()return 1 end},function(key,consume)if key=='c2-action'and plan then local value=json.encode(plan);if consume then plan=nil end;return value end end,function()end,repo)
 local owner={ready=true,playing=true,scope='fixture',view={}};local x=1000
 for i=1,60 do state.frame=state.frame+1;adapter:tick({c2=owner});local next_x=e.EntityGetTransform(1);eq(math.abs(next_x-x)<=16,true);x=next_x end
 eq(x,201);plan={kind='position_free'}
 for i=1,60 do state.frame=state.frame+1;adapter:tick({c2=owner});local next_x=e.EntityGetTransform(1);eq(math.abs(next_x-x)<=16,true);x=next_x end
 eq(x,1000)
end
print('PASS '..checks..' total C2 focused production and adapter checks (injected engine)')
-- Steve uses the actor's own position and never replays anger observed off.
do
 local e,c,entities,g=engine();local json=e.dofile('mods/noita_together2/files/g3/bank_json.lua');local calls={};local sent={}
 local owner=e.dofile('mods/noita_together2/files/c2/local.lua').new(e,uid,function(body)sent[#sent+1]=assert(json.decode(body));return true end)
 local view={world=string.rep('9',64),principal='self',worldToken='world',progression=0,initialized=true,snapshot={sequence='1',rules={revision=1,rules={NT_sync_steve=false}},completions={}}};owner.context='ctx';owner:receive(view);owner:tick(true,1,100)
 g.TEMPLE_SPAWN_GUARDIAN='1';owner:tick(true,1,100);eq(e.dofile('mods/noita_together2/files/c2/storage.lua').read(e,'outgoing'),nil)
 view.snapshot.rules.rules.NT_sync_steve=true;owner:receive(view);owner:tick(true,1,100);eq(e.dofile('mods/noita_together2/files/c2/storage.lua').read(e,'outgoing'),nil)
 g.TEMPLE_SPAWN_GUARDIAN='0';local exit=e.EntityLoad('exit',300,100);e.EntityGetClosestWithTag=function(x,y,tag)eq(x,10);eq(y,20);eq(tag,'workshop_exit');return exit end
 e.GameTextGet=function(key,name)calls[#calls+1]={key,name};return key end
 local state=owner:steve(1,{sourceName='Named <&" sender'});eq(state,'applied');local loaded=e.loads[#e.loads];eq(loaded.path,'data/entities/misc/spawn_necromancer_shop.xml');eq(loaded.x,180);eq(loaded.y,130);eq(calls[#calls-1][1],'$noitatogether_angered_gods_title');eq(calls[#calls-1][2],'Named <&" sender');eq(calls[#calls][1],'$noitatogether_angered_gods_subtitle')
 local count=#e.loads;eq(owner:steve(1,{sourceName='repeat'}),'skipped');eq(#e.loads,count)
end
print('PASS '..checks..' total C2 focused production and adapter checks (injected engine)')
-- Exercise the production bank adapter's C2 identity supplier, not a test UUID.
do
 local e=engine();e.NT2_C1_ENABLED=false;e.NT2_C2_ENABLED=true;local raw=e.dofile;local capture;local nonce=string.rep('f',32)
 e.dofile=function(path)
  if path=='mods/noita_together2/files/c2/local.lua'then return{new=function(_,identity)capture=identity;return{}end}end
  if path:find('gold_wallet.lua',1,true)then return{new=function()return{snapshot=function()end}end}end
  if path:find('gold_local.lua',1,true)or path:find('bank_ui.lua',1,true)or path:find('item_bridge.lua',1,true)then return{new=function()return{}end}end
  return raw(path)
 end
 local module=e.dofile('mods/noita_together2/files/g3/bank_native.lua')
 module.new({nt2_g3_bank_view=function()end,nt2_g3_bank_publish=function()end},{},{new=function()return{}end},e,{}, {},function()return nonce end)
 for _,value in ipairs({string.rep('0',32),string.rep('f',32),'123456789abcdef0123456789abcdef0'})do nonce=value;local id=capture();eq(#id,36);eq(id:sub(15,15),'4');eq(id:sub(20,20):match('^[89ab]$')~=nil,true)end
end
print('PASS '..checks..' total C2 focused production and adapter checks (injected engine)')

-- Recovery metadata stays ready across bank/C1/C2 mailbox rotation only while
-- the native authority is current; an actual gap invalidates cached readiness.
do
 local e=engine();e.NT2_C1_ENABLED=false;e.NT2_C2_ENABLED=false
 local raw=e.dofile
 e.dofile=function(path)
  if path:find('gold_wallet.lua',1,true)then return{new=function()return{snapshot=function()end}end}end
  if path:find('item_bridge.lua',1,true)then error('item profile absent in this focused adapter fixture')end
  if path:find('gold_local.lua',1,true)or path:find('bank_ui.lua',1,true)then return{new=function()return{}end}end
  return raw(path)
 end
 local body='{}';local status=2;local context={};for i=0,15 do context[i]=0 end
 local dll={nt2_g3_bank_view=function()return #body end,nt2_g3_bank_publish=function()return 1 end,nt2_g3_status=function()return status end}
 local ffi={new=function()return{}end,string=function()return body end}
 local bank=e.dofile('mods/noita_together2/files/g3/bank_native.lua').new(dll,context,ffi,e,{}, {},uid)
 e.GameGetWorldStateEntity=function()return 123 end;e.nt2_c1_permitted=function()return false end
 local function policy()return{ready=false,receive=function(self,v)self.ready=v.initialized;return true end,tick=function()end}end
 bank.c1=policy();bank.c2=policy()
 body='{"c1":{"initialized":true}}';bank:tick(false,1);eq(bank.c1.ready,true)
 body='{"c2":{"world":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","initialized":true}}';bank:tick(false,1);eq(bank.c1.ready,true);eq(bank.c2.ready,true)
 body='{}';bank:tick(false,1);eq(bank.c1.ready,true);eq(bank.c2.ready,true)
 status=0;bank:tick(false,1);eq(bank.c1.ready,false);eq(bank.c2.ready,false)
 status=2;bank:tick(false,1);eq(bank.c1.ready,false);eq(bank.c2.ready,false)
 body='{"c1":{"initialized":true}}';bank:tick(false,1);eq(bank.c1.ready,true);eq(bank.c2.ready,false)
 body='{"c2":{"initialized":true}}';bank:tick(false,1);eq(bank.c1.ready,true);eq(bank.c2.ready,true)
end
print('PASS '..checks..' total C2 focused production and adapter checks (injected engine)')

-- Explicit offline transfers only this saved encounter to local progression.
-- Execute the production factory, aura, pickup and handoff across real raw XML.
do
 for _,phase in ipairs({'locked','transitioning','picked_waiting','battle_done'})do
  local e,c,entities,g,flags=engine();local store=e.dofile('mods/noita_together2/files/c2/storage.lua')
  local codec=e.dofile('mods/noita_together2/files/rejoin/local_world.lua');g.nt2_rejoin_local_world_v1=assert(codec.encode_saved('owned association '..phase));store.write(e,'mode',{managed=true})
  local vanilla=0;e.spawn_items=function()vanilla=vanilla+1 end;e.dofile('mods/noita_together2/files/c2/append/boss_arena.lua');e.spawn_items(0,0)
  local saved={world=string.rep('d',64),progression=0,association=g.nt2_rejoin_local_world_v1};local module=e.dofile('mods/noita_together2/files/c2/world.lua')
  local arena=module.new(e,function()store.write(e,'state',saved)end);eq(arena:bind(saved,uid,'ctx'),true);arena:scope(saved,{encounterId='offline encounter'},'ctx')
  local aura=arena:owned(saved,'aura');e.EntityAddComponent2(aura,'ParticleEmitterComponent',{attractor_force=-.4,area_circle_radius=190,velocity_always_away_from_center=1});e.EntityAddComponent2(aura,'LuaComponent',{_tags='nt_aura_script'})
  local grant={playing=true,frame=100,player=1,world=saved.world,context='ctx',progression=0,encounterId='offline encounter',arenaToken=saved.arenaToken,stage=phase=='locked'and 'locked'or 'unlocked',rulesRevision=1}
  store.write(e,'grant',grant);e.GetUpdatedEntityID=function()return aura end
  if phase~='locked'then e.dofile('mods/noita_together2/files/c2/scripts/sampo/sampo_aura.lua')end
  if phase=='picked_waiting'or phase=='battle_done'then
   for i=1,33 do e.dofile('mods/noita_together2/files/c2/scripts/sampo/sampo_aura.lua')end
   store.write(e,'tokens',{world=saved.world,context='ctx',ids={uid()}});e.item_pickup=function()vanilla=vanilla+1 end;e.dofile('mods/noita_together2/files/c2/append/sampo_append.lua');e.item_pickup(arena:owned(saved,'sampo'),1,'sampo');saved=store.read(e,'state');eq(saved.localPickup,true)
   if phase=='battle_done'then eq(arena:battle(saved,1),true);entities[arena:owned(saved,'boss')].alive=false end
  end
  local before=#e.loads;eq(module.offline(e,1),false);eq(#e.loads,before)
  -- A terminal/connection hold alone cannot become an opt-out.
  grant.playing=false;store.write(e,'grant',grant);eq(module.offline(e,1),false);eq(#e.loads,before)
  grant.playing=true;store.write(e,'grant',grant);g.nt2_rejoin_local_world_v1_offline='1'
  eq(e.dofile('mods/noita_together2/files/c2/hook_api.lua').current(1),nil)
  local association=g.nt2_rejoin_local_world_v1;g.nt2_rejoin_local_world_v1='wrong scope';eq(module.offline(e,1),false);eq(#e.loads,before);g.nt2_rejoin_local_world_v1=association
  local ordinal=e.SessionNumbersGetValue;e.SessionNumbersGetValue=function()return '1'end;eq(module.offline(e,1),false);eq(#e.loads,before);e.SessionNumbersGetValue=ordinal
  eq(module.offline(e,1),true);saved=store.read(e,'state');local sampo=arena:owned(saved,'sampo');eq(sampo~=nil,true);eq(arena:owned(saved,'aura'),nil);eq(arena:owned(saved,'disabled'),nil)
  if not saved.localPickup then
   e.item_pickup=function()vanilla=vanilla+1 end;e.dofile('mods/noita_together2/files/c2/append/sampo_append.lua');eq(e.item_pickup(sampo,1,'sampo'),true)
  end
  saved=store.read(e,'state');eq(saved.battle.done,true);eq(saved.localPickup,true);eq(e.music,1)
  local loads=#e.loads;local outgoing=store.read(e,'outgoing');local queue=outgoing and #outgoing.queue or 0
  eq(module.offline(e,1),true);e.spawn_items(0,0);eq(vanilla,0);eq(#e.loads,loads)
  local xml=dofile(repo..'/apps/desktop/tools/noita-globals-xml.lua');local restored=assert(xml.restore(xml.serialize(g)));for k in pairs(g)do g[k]=nil end;for k,v in pairs(restored)do g[k]=v end
  eq(e.dofile('mods/noita_together2/files/c2/world.lua').offline(e,1),true);eq(#e.loads,loads);eq(e.music,1);eq(store.read(e,'grant').playing,nil)
  eq((store.read(e,'outgoing')or{queue={}}).queue and #(store.read(e,'outgoing')or{queue={}}).queue,queue)
  local rocks,sampos,bosses=0,0,0;for _,row in ipairs(e.loads)do if row.path:find('loose_lava',1,true)then rocks=rocks+1 elseif row.path=='data/entities/animals/boss_centipede/sampo.xml'then sampos=sampos+1 elseif row.path=='data/entities/animals/boss_centipede/boss_centipede.xml'then bosses=bosses+1 end end
  eq(rocks,4);eq(sampos,1);eq(bosses,1)
 end
end
print('PASS '..checks..' total C2 focused production and adapter checks (injected engine)')
