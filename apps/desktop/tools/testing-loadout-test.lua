local repo=assert(os.getenv('NT2_ITEM_REPO'));local output=assert(os.getenv('NT2_TESTING_LOADOUT_CONFIG_OUTPUT'))
NT2_C1_TEST_ENGINE_ONLY=true;local fixture=dofile(repo..'/apps/desktop/tools/c1-local-test.lua');NT2_C1_TEST_ENGINE_ONLY=nil
local checks=0;local function eq(a,b)assert(a==b,tostring(a)..' ~= '..tostring(b));checks=checks+1 end
local function read(path)local f=assert(io.open(path,'rb'));local s=f:read('*a');f:close();return s end
local root='mods/noita_together2/files/startup/';local key='nt2_testing_starting_loadout_v1'
local function setup(loot_on,banned)
 local e,c,entities,g,flags=fixture.engine();local raw=e.dofile;local text={};local registry={root..'scope_new.lua',root..'scope_spent.lua'};local loads={};local rng=0;local picks=0;local procedural={}
 e.GameGetWorldStateEntity=function()return 2 end;e.StatsGetValue=function()return '424242'end
 e.ModTextFileSetContent=function(path,s)text[path]=s end
 e.ModTextFileGetContent=function(path)
  if path==root..'testing-starting-loadout.json'then return read(repo..'/packages/contracts/g3/testing-starting-loadout.json')end
  if path=='mods/noita_together2/files/loot/run-policy.txt'then return loot_on and 'entry-new:shared:1:424242'or 'entry-new:shared:0:424242'end
  return text[path]or''
 end
 e.ModLuaFileGetAppends=function()return registry end
 e.SetRandomSeed=function()rng=rng+1 end;e.Random=function()rng=rng+1;return 1 end;e.Randomf=function()rng=rng+1;return .5 end
 e.ProceduralRandomi=function(x,y,a,b)procedural[#procedural+1]={x=x,y=y};return a+math.floor(math.abs(x+y))%(b-a+1)end
 e.ProceduralRandomf=function(x,y,a,b)return a+(b-a)*.5 end
 e.perk_list={};for _,id in ipairs({'EDIT_WANDS_EVERYWHERE','NO_MORE_SHUFFLE','GLASS_CANNON','EXTRA_HP','SPEED','LOW_GRAVITY','EXTRA_MONEY'})do e.perk_list[#e.perk_list+1]={id=id,perk_icon='icon',ui_name=id,ui_description=id}end
 e.get_perk_with_id=function(list,id)for _,row in ipairs(list)do if row.id==id then return row end end end
 e.perk_get_spawn_order=function(ignore)local found={};for _,row in ipairs(e.perk_list)do local skip=false;for _,id in ipairs(ignore)do if row.id==id then skip=true end end;if not skip then found[#found+1]=row.id end end;return found end
 e.create_all_player_perks=function()end;e.perk_pickup=function()picks=picks+1 end
 local many_calls=0;local tinker_calls=0
 e.perk_spawn_many=function(x,y,keep,ignore)
  many_calls=many_calls+1;eq(keep,true);eq(ignore[1],'EDIT_WANDS_EVERYWHERE');eq(ignore[2],'NO_MORE_SHUFFLE');eq(#ignore,2);local pool=e.perk_get_spawn_order(ignore);e.SetRandomSeed(x,y)
  for i=1,3 do local n=e.Random(1,#pool);e.perk_spawn(x+(i-1)*20,y,table.remove(pool,n),keep)end
 end
 e.perk_spawn_with_name=function(x,y,id,keep)tinker_calls=tinker_calls+1;eq(id=='EDIT_WANDS_EVERYWHERE'or id=='NO_MORE_SHUFFLE',true);return e.perk_spawn(x,y,id,keep)end
 local loaded=false;e.dofile_once=function(path)if path=='data/scripts/perks/perk.lua'then if not loaded then loaded=true;raw('mods/noita_together2/files/c1/append/perk.lua')end else return raw(path)end end
 local original_load=e.EntityLoad;e.EntityLoad=function(path,x,y)local id=original_load(path,x,y);loads[#loads+1]={path=path,x=x,y=y,id=id};return id end
 local json=raw('mods/noita_together2/files/g3/bank_json.lua');local module=raw(root..'testing_loadout.lua').new(e)
 local function complete(snapshot)
  local d=assert(json.decode(snapshot,{bytes=16384,nodes=2048,depth=16}));local function bytes(h)return(h:gsub('-',''):gsub('..',function(v)return string.char(tonumber(v,16))end))end
  return{snapshot=snapshot,world=bytes(d.local_world),digest=bytes(d.bundle_digest),config_digest=bytes(d.scope.config_digest),seed=d.seed,policy=d.policy,generation=d.generation}
 end
 local old_seed,old_random,old_randomf=e.SetRandomSeed,e.Random,e.Randomf
 if banned then flags.NT_ban_perks=true end
 c[13].enabled=false
 local normal={e.EntityLoad('normal_wand_a',10,20),e.EntityLoad('normal_wand_b',10,20),e.EntityLoad('normal_potion',10,20)}
 return{e=e,c=c,entities=entities,g=g,flags=flags,json=json,module=module,loads=loads,complete=complete,text=text,registry=function(value)registry=value end,normal=normal,rng=function()return rng end,picks=function()return picks end,procedural=procedural,originals={old_seed,old_random,old_randomf},counts=function()return many_calls,tinker_calls end}
end
local enabled=read(output..'/enabled-snapshot.json');local disabled=read(output..'/disabled-snapshot.json')
-- Actual Rust Config -> frozen Prepared snapshot -> production mod consumer.
for _,variant in ipairs({'disabled','absent','on','randomized','banned'})do
 local f=setup(variant=='randomized',variant=='banned');local snapshot=variant=='disabled'and disabled or enabled
 if variant=='absent'then local d=assert(f.json.decode(enabled,{bytes=16384,nodes=2048}));d.config.settings=nil;snapshot=assert(f.json.encode(d))end
 f.module:saved();f.module:stage(f.complete(snapshot));local result=f.module:spawn(1)
 if variant=='disabled'or variant=='absent'then eq(result,'disabled');eq(#f.loads,3);eq(f.rng(),0);eq(f.g[key],nil)
 else
  eq(result,'done');eq(#f.loads,11);eq(f.c[13].enabled,false);eq(f.picks(),0);eq(f.rng(),0)
  local m,t=f.counts();eq(m,1);eq(t,2);local perk_count,wand_count,tinker_count,shuffle_count=0,0,0,0;local slots={}
  for i=4,#f.loads do local row=f.loads[i];local entity=f.entities[row.id];local metadata
   for _,part in ipairs(entity.parts.VariableStorageComponent or{})do local data=f.c[part];if data.name=='nt2_testing_starting_pickup'then metadata=assert(f.json.decode_saved(data.value_string));eq(metadata.world,f.json.decode(enabled,{bytes=16384,nodes=2048}).local_world)end end
   eq(metadata~=nil,true);if metadata.kind=='wand'then wand_count=wand_count+1;slots[metadata.slot]=row.x;eq(entity.tags.nightmare_starting_wand,true);eq(entity.parts.LuaComponent,nil);eq(row.y,-28)
   else perk_count=perk_count+1;local keep=false;local perk
    for _,part in ipairs(entity.parts.VariableStorageComponent or{})do local data=f.c[part];if data.name=='perk_id'then perk=data.value_string elseif data.name=='perk_dont_remove_others'then keep=data.value_bool=='1'end end
    eq(keep,true);if perk=='EDIT_WANDS_EVERYWHERE'then tinker_count=tinker_count+1;eq(metadata.kind,'tinker');eq(row.x,40);eq(row.y,-4)elseif perk=='NO_MORE_SHUFFLE'then shuffle_count=shuffle_count+1;eq(metadata.kind,'no_more_shuffle');eq(row.x,70);eq(row.y,-4)else eq(metadata.kind,'random_perk')end
    if variant=='banned'then eq(perk=='GLASS_CANNON',false)end
   end
  end
  eq(perk_count,5);eq(wand_count,3);eq(tinker_count,1);eq(shuffle_count,1);eq(slots[2]-slots[1],44);eq(slots[3]-slots[2],44)
  for _,id in ipairs(f.normal)do eq(f.e.EntityGetIsAlive(id),true)end
  eq(f.e.SetRandomSeed,f.originals[1]);eq(f.e.Random,f.originals[2]);eq(f.e.Randomf,f.originals[3])
  if variant=='randomized'then eq(f.procedural[1].x,21)else eq(f.procedural[1].x,-79)end
  for i=1,4 do eq(f.module:spawn(1),'done')end;eq(#f.loads,11)
  local xml=dofile(repo..'/apps/desktop/tools/noita-globals-xml.lua');local restored=assert(xml.restore(xml.serialize(f.g)));for k in pairs(f.g)do f.g[k]=nil end;for k,v in pairs(restored)do f.g[k]=v end
  local cold=f.e.dofile(root..'testing_loadout.lua').new(f.e);eq(cold:spawn(1),'done');eq(#f.loads,11)
  cold:saved();f.registry({root..'scope_restart.lua',root..'scope_spent.lua'});eq(cold:spawn(1),'saved');eq(#f.loads,11)
  if variant=='on'then
   local file=assert(io.open(output..'/globals.xml','wb'));file:write(xml.serialize(f.g));file:close()
   local rows={'<Entity>'};for _,entity in pairs(f.entities)do for _,part in ipairs(entity.parts.VariableStorageComponent or{})do local data=f.c[part];if data.name=='nt2_testing_starting_pickup'then rows[#rows+1]='<VariableStorageComponent name="'..data.name..'" value_string="'..data.value_string..'"/>'end end end;rows[#rows+1]='</Entity>';file=assert(io.open(output..'/components.xml','wb'));file:write(table.concat(rows));file:close()
  end
 end
end
-- Missing, transformed and dead actors park unissued work; reconnect/respawn do
-- not repeat completed work, and a different frozen new world gets its own set.
do
 local f=setup();f.module:saved();f.module:stage(f.complete(enabled));f.entities[1].tags.polymorphed=true;eq(f.module:spawn(1),'pending');eq(#f.loads,3);f.entities[1].tags.polymorphed=nil
 f.c[11].hp=-1;eq(f.module:spawn(1),'pending');f.c[11].hp=2;eq(f.module:spawn(1),'done');eq(f.module:spawn(1),'done');eq(#f.loads,11)
 f.registry({root..'scope_restart.lua',root..'scope_spent.lua'});eq(pcall(function()f.module:stage(f.complete(enabled))end),false);eq(#f.loads,11)
 f.registry({root..'scope_new.lua',root..'scope_spent.lua'});local next_world=f.json.decode(enabled,{bytes=16384,nodes=2048});next_world.local_world='22222222-2222-4222-8222-222222222222';f.module:saved();f.module:stage(f.complete(assert(f.json.encode(next_world))));eq(f.module:spawn(1),'done');eq(#f.loads,19)
end
-- A partly completed issue is saved as uncertain and never retries any step.
for _,fail_at in ipairs({1,2,3,4,5,6,7,8})do
 local f=setup();local original=f.e.EntityLoad;local n=0;f.e.EntityLoad=function(...)n=n+1;if n==fail_at then error('injected engine load failure')end;return original(...)end
 f.module:saved();f.module:stage(f.complete(enabled));eq(f.module:spawn(1),'uncertain');local loads=#f.loads;eq(f.module:spawn(1),'uncertain');eq(#f.loads,loads);eq(f.e.SetRandomSeed,f.originals[1]);eq(f.e.Random,f.originals[2]);eq(f.e.Randomf,f.originals[3])
 local cold=f.e.dofile(root..'testing_loadout.lua').new(f.e);eq(cold:spawn(1),'uncertain');eq(#f.loads,loads)
end
-- Corrupt saved state and mismatched complete bundle never spawn fresh extras.
do
 local f=setup();f.module:saved();f.module:stage(f.complete(enabled));f.g[key]='nt2h1:2:zzzz';eq(f.module:spawn(1),'corrupt');eq(#f.loads,3)
 local complete=f.complete(enabled);complete.world=string.rep('x',16);eq(pcall(function()f.module:stage(complete)end),false);eq(#f.loads,3)
end
print('PASS '..checks..' testing loadout production helper/call/save checks; injected engine facts')
