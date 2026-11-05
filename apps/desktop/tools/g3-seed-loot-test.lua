-- Original Lua runtime, injected local APIs; no engine/terrain acceptance claim.
local repo=assert(os.getenv('NT2_SEED_LOOT_REPO'))
local function load_owned(path,e)
    local f=assert(loadfile(repo..'/mod/noita_together2/'..path));setfenv(f,e);return f()
end
local hook_count=0
local function hooks(recorded)
    local e=setmetatable({},{__index=_G});e._G=e
    local calls,seeds={},{}
    e.GlobalsGetValue=function() return recorded end
    e.ModTextFileGetContent=function() return 'saved' end
    e.GameGetWorldStateEntity=function() return 123 end
    e.StatsGetValue=function() return '424242' end
    e.StatsGlobalGetValue=function() return '3' end;e.GameGetFrameNum=function() return 40 end
    e.SetWorldSeed=function() error('loot cannot change terrain seed') end
    local original=function(x,y) seeds[#seeds+1]={x,y};return 'rng' end;e.SetRandomSeed=original
    e.Random=function(min) return min+1 end
    e.dofile=function(path) return load_owned(path:sub(#'mods/noita_together2/'+1),e) end
    local enabled=recorded=='shared:1:424242'
    local functions={biome_scripts={'spawn_wands','spawn_potions','spawn_heart'},generate_shop_item={'generate_shop_item'},gun_procedural={'generate_gun'},perk_reroll={'item_pickup'},co_op_mail={'spawn_all_shopitems','spawn_all_perks'}}
    for name,names in pairs(functions) do
        for _,fn in ipairs(names) do
            local fail=false;local captured
            e[fn]=function(...) captured={n=select('#',...),...};e.SetRandomSeed(10,20);if fail then error('injected base failure') end;return 42,nil,'tail' end
            load_owned('files/loot/'..name..'.lua',e)
            local a,b,c=e[fn](100,200,'extra',true)
            assert(a==42 and b==nil and c=='tail','base return tuple preserved')
            local coordinate=fn=='spawn_wands' or fn=='spawn_potions' or fn=='spawn_heart' or fn=='generate_shop_item' or fn=='spawn_all_shopitems'
            assert(captured[1]==(enabled and coordinate and 99 or 100));assert(captured[2]==(enabled and coordinate and 199 or 200));assert(captured[3]=='extra' and captured[4]==true)
            if not coordinate then assert(seeds[#seeds][1]==(enabled and 50 or 10)) end
            assert(e.SetRandomSeed==original,'temporary wrapper restored after return')
            fail=true;assert(not pcall(e[fn],100,200));assert(e.SetRandomSeed==original,'temporary wrapper restored after error')
            hook_count=hook_count+1
        end
    end
    for _,name in ipairs({'chest_random','chest_random_super'}) do
        local captured
        e.drop_random_reward=function(x,y,id,rx,ry,reseed)
            captured={x,y,id,rx,ry,reseed};if reseed then e.SetRandomSeed(x,y) end;return 21,nil,'chest'
        end
        load_owned('files/loot/'..name..'.lua',e)
        for _,reseed in ipairs({true,false}) do
            local a,b,c=e.drop_random_reward(100,200,7,8,9,reseed)
            assert(a==21 and b==nil and c=='chest');assert(captured[1]==100 and captured[2]==200 and captured[3]==7 and captured[4]==8 and captured[5]==9)
            assert(captured[6]==(not enabled and reseed));if enabled then assert(seeds[#seeds][1]==40 and seeds[#seeds][2]==307,'base reseed cannot undo chosen inputs') end
        end
        hook_count=hook_count+1
    end
    local entity_path,tag;e.EntityLoad=function(path,x,y) entity_path=path;assert(x==100 and y==200);return 9 end;e.EntityAddTag=function(id,value) assert(id==9);tag=value end
    load_owned('files/loot/preset_wands_random.lua',e);e.spawn_trapwand(100,200)
    assert(entity_path=='data/entities/items/wands/level_01/wand_002.xml' and tag=='trap_wand');assert(seeds[#seeds][1]==(enabled and 140 or 100));hook_count=hook_count+1
    load_owned('files/loot/butcher_randomseed.lua',e);local permanent=e.SetRandomSeed;assert(permanent(10,20)=='rng');assert(seeds[#seeds][1]==(enabled and 47 or 10) and seeds[#seeds][2]==(enabled and 23 or 20));assert(e.SetRandomSeed==permanent);hook_count=hook_count+1
    local mappings={};e.ModLuaFileAppend=function(base,append) mappings[#mappings+1]={base,append} end
    load_owned('files/loot/register.lua',e);assert(#mappings==12);assert(mappings[12][1]=='data/scripts/biomes/snowcastle_hourglass_chamber.lua')
end
for _,value in ipairs({'shared:1:424242','shared:0:424242','individual:0','','shared:1:invalid','shared:1:0','shared:1:4294967296'}) do hooks(value) end

local ffi=require('ffi');local original_load=ffi.load
local entry_count=0
local function entry(startup,policy,actual,social_policy,prior_record,silent_early,world_at_magic)
    entry_count=entry_count+1
    dofile(repo..'/apps/desktop/tools/mod-protection-stubs.lua')
    local e=setmetatable({},{__index=_G});e._G=e
    local generated=false;local forced,accepted,loaded,reports,closed=0,0,0,0,0;local playing=false;local recorded=prior_record or ''
    local world_ready=false;local virtual='new:shared:1:999';local writes=0;local forbidden=0;local init_phase=true
    e.print=function(line) assert(not line:find('event=protection_failed',1,true),'fixture must supply protection APIs') end
    e.EntityGetName=function() return '' end;e.GameGetAllInventoryItems=function() return {} end
    e.ModLuaFileAppend=function() end
    e.GameGetWorldStateEntity=function() return world_ready and 123 or 0 end
    e.GlobalsSetValue=function(key,value) if not world_ready then forbidden=forbidden+1;if silent_early then return end;error('WorldState does not exist') end;assert(key=='nt2_seed_loot_policy_v1');writes=writes+1;recorded=value end
    e.GlobalsGetValue=function(_,fallback) if not world_ready then forbidden=forbidden+1;if silent_early then return fallback end;error('WorldState does not exist') end;return recorded~='' and recorded or fallback end
    e.ModTextFileSetContent=function(path,value) assert(init_phase and path=='mods/noita_together2/files/loot/run-policy.txt');virtual=value end
    e.ModTextFileGetContent=function(path) assert(path=='mods/noita_together2/files/loot/run-policy.txt');return virtual end
    e.SetWorldSeed=function(seed) assert(not generated and seed==424242);forced=forced+1 end
    e.StatsGetValue=function() return actual and tostring(actual) or '?' end
    e.ModGetActiveModIDs=function() return {'noita_together2'} end;e.GameIsBetaBuild=function() return nil end
    e.dofile=function(path)
        if path:find('/native_owner.lua',1,true) or path:find('/social_native.lua',1,true) then return {new=function() return {tick=function() end,close=function() end} end} end
        if path:find('/files/g3/',1,true) then return {} end
        return load_owned(path:sub(#'mods/noita_together2/'+1),e)
    end
    local native={}
    native.nt2_g3_startup_prepare=function(out,c) if startup==1 then out[0]=policy==2 and 0 or 424242;ffi.fill(c,16,1) end;return startup end
    native.nt2_g3_prepare=native.nt2_g3_startup_prepare
    native.nt2_g3_world_policy=function() return policy end
    native.nt2_g3_seed_known=function(_,value,size) assert(value==tostring(actual) and size==#value);accepted=accepted+1;return accepted>=3 and 1 or 0 end
    native.nt2_g3_world_loaded=function() loaded=loaded+1;return loaded>=3 and 1 or 0 end
    native.nt2_g3_report=function(_,body) reports=reports+1;assert(body:find(actual and tostring(actual) or 'null',1,true));return 1 end
    native.nt2_g3_status=function() return playing and 2 or 1 end
    native.nt2_g3_close=function() closed=closed+1;return 1 end
    for _,name in ipairs({'association','saved_lookup','offline_choice','resume_decision','resume'}) do native['nt2_g3_'..name]=function() return 0 end end
    for _,name in ipairs({'nt2_g3_heartbeat_diagnostic','nt2_g3_move','nt2_g3_peers','nt2_g3_social_lease','nt2_g3_social_next','nt2_g3_social_terminal'}) do native[name]=function() return 0 end end
    local social=setmetatable({nt2_g3_world_policy=function() return social_policy or policy end},{__index=native})
    ffi.load=function(path) return path:find('social',1,true) and social or native end
    local override=os.getenv('NT2_LOOT01_ENTRY_OVERRIDE')
    if override then local f=assert(loadfile(override));setfenv(f,e);f() else load_owned('init-g3-seed-loot.lua',e) end
    e.OnModPreInit();ffi.load=original_load
    local bound=startup==1 and (policy==1 or policy==2 or policy==5) and (not social_policy or social_policy==policy)
    assert(forced==(bound and policy~=2 and 1 or 0),'only explicitly shared policy forces seed before generation')
    assert(writes==0 and recorded==(prior_record or ''),'PreInit cannot touch world globals')
    assert(forbidden==0,'entry attempted Globals before WorldState exists')
    local function separate_hook()
        local hook=setmetatable({},{__index=e});hook._G=hook
        hook.dofile=function(path) return load_owned(path:sub(#'mods/noita_together2/'+1),hook) end
        return load_owned('files/loot/policy.lua',hook)
    end
    local function separate_biome(expected)
        local hook=setmetatable({},{__index=e});hook._G=hook
        local x,y,rng
        hook.SetRandomSeed=function(a,b) rng={a,b} end
        hook.Random=function(min) return min+1 end;hook.StatsGlobalGetValue=function() return '3' end
        hook.spawn_potions=function(a,b) x,y=a,b;return 21,nil,'tail' end
        hook.spawn_wands=function() end;hook.spawn_heart=function() end
        hook.dofile=function(path) return load_owned(path:sub(#'mods/noita_together2/'+1),hook) end
        load_owned('files/loot/biome_scripts.lua',hook)
        local a,b,c=hook.spawn_potions(100,200);assert(a==21 and b==nil and c=='tail')
        assert(x==(expected and 99 or 100) and y==(expected and 199 or 200),'separate biome hook must use staged/saved policy before player exists')
        assert((rng~=nil)==expected)
    end
    local expected_early=bound and policy==5 and actual==424242
    assert(separate_hook().enabled()==expected_early,'early separate Lua context must see staged known policy')
    separate_biome(expected_early)
    init_phase=false;world_ready=world_at_magic==true;e.OnMagicNumbersAndWorldSeedInitialized()
    assert(forbidden==0 and (world_ready or writes==0),'Magic callback may also precede WorldState')
    -- First affected biome/item context after WorldState construction and before player spawn.
    world_ready=true;assert(separate_hook().enabled()==(expected_early or not bound and prior_record=='shared:1:424242' and actual==424242))
    separate_biome(expected_early or not bound and prior_record=='shared:1:424242' and actual==424242)
    generated=true;e.OnPlayerSpawned(99)
    for i=1,12 do NT2_FIXTURE.frame=i;e.OnWorldPreUpdate() end
    if not bound then assert(accepted==0 and loaded==0 and recorded==(prior_record or '') and not NT2_FIXTURE.components[1].enabled);return end
    assert(recorded==(policy==2 and 'individual:0' or 'shared:'..(policy==5 and '1' or '0')..':424242'))
    if not actual or policy~=2 and actual~=424242 then assert(accepted==0 and loaded==0 and not NT2_FIXTURE.components[1].enabled);return end
    assert(accepted==3 and loaded==3 and reports>0 and not NT2_FIXTURE.components[1].enabled)
    playing=true;e.OnWorldPreUpdate();assert(NT2_FIXTURE.components[1].enabled)
    playing=false;e.OnWorldPreUpdate();assert(not NT2_FIXTURE.components[1].enabled,'loss of authority reholds')
    -- A changed report cannot release this original individual world even if status remains playing.
    actual=type(actual)=="string" and "1" or actual+1;playing=true;e.OnWorldPreUpdate();assert(not NT2_FIXTURE.components[1].enabled)
end
entry(1,1,424242);entry(1,5,424242);entry(1,2,777);entry(1,2,888);entry(1,2,"18446744073709551615");entry(1,2,nil);entry(1,1,999);entry(1,0,424242);entry(1,3,424242);entry(1,5,424242,1);entry(-2,2,777,nil,'shared:1:424242');entry(0,5,777)
entry(-2,5,424242,nil,'shared:1:424242');entry(-2,1,424242,nil,'shared:0:424242');entry(0,5,424242,nil,'shared:1:424242');entry(1,1,424242,nil,'shared:1:424242');entry(1,2,777,nil,'shared:1:424242')
entry(1,5,424242,nil,nil,true);entry(0,5,424242,nil,'shared:1:424242',true);entry(1,5,424242,nil,nil,false,true)
ffi.load=original_load
print('PASS seed/loot: '..hook_count..' local hook cases; '..entry_count..' entry lifecycle cases; registration, disabled/individual/invalid policy, preserved tuples/arguments, RNG error restoration/chest reseed; actual entry early Globals refusal/silent refusal/separate biome contexts/shared/individual/unknown/wrong/policy agreement/Continue/hold/release/change. Engine facts injected.')
