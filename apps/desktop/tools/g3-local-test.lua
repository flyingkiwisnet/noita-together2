local root=assert(os.getenv("NT2_G3_MODULES"))
local raw_dofile=dofile
dofile=function(path)local relative=path:match('^mods/noita_together2/(.+)$');return raw_dofile(relative and root:gsub('/files/g3$','')..'/'..relative or path)end
local function load(name) return dofile(root.."/"..name..".lua") end
local validate=load("validation")
local movement_module,ghost_module,transport_module,lifecycle_module=load("movement"),load("ghosts"),load("transport"),load("lifecycle")
local checks={}
local function check(name,fn) local filter=os.getenv("NT2_G3_FILTER");if filter and not name:find(filter,1,true)then return end;fn();checks[#checks+1]=name;print("PASS "..name) end
local function pose(actor,sequence,x) return {actor=actor,sequence=sequence,x=x or 10,y=20,rotation=0,facing=1,vx=0,vy=0} end
local entities,next_entity,removed={},0,0
local ghost_api={alive=function(id) return entities[id]~=nil end,spawn=function(x,y) next_entity=next_entity+1;entities[next_entity]={x=x,y=y};return next_entity end,
    remove=function(id) entities[id]=nil;removed=removed+1 end,position=function(id,x,y) assert(entities[id]);entities[id].x=x;entities[id].y=y end}
local player={x=5,y=6}
local seconds=0
ghost_api.time=function() return seconds end
local movement=movement_module.new({alive=function(p) return p==player end,time=function() return seconds end,position=function(p) return p.x,p.y,0,1,0,0 end},validate)
local ghosts=ghost_module.new(ghost_api,validate)
local transport=transport_module.new(validate)
local lifecycle=lifecycle_module.new(movement,ghosts,transport,validate)
check("movement waits for release/scope and samples at ten Hz",function()
    movement:reset("world-A")
    assert(not movement:sample("world-A","actor",player,0,false))
    assert(not movement:sample("world-B","actor",player,0,true))
    assert(movement:sample("world-A","actor",player,0,true).sequence==1)
    for frame=1,5 do assert(not movement:sample("world-A","actor",player,frame,true)) end
    seconds=0.05;assert(not movement:sample("world-A","actor",player,6,true))
    seconds=0.1;assert(movement:sample("world-A","actor",player,6,true).sequence==2)
    seconds=0.2;player.x=0/0;assert(not movement:sample("world-A","actor",player,12,true));player.x=5
end)
check("ghosts reject outsiders wrong worlds malformed batches and stale sequence",function()
    assert(not lifecycle:start("world-A",{"actor"},false))
    assert(lifecycle:start("world-A",{"actor"},true))
    assert(not lifecycle:receive("world-B",{pose("actor",1)}))
    assert(not lifecycle:receive("world-A",{pose("outsider",1)}))
    assert(not lifecycle:receive("world-A",{[2]=pose("actor",1)}))
    assert(not lifecycle:receive("world-A",{17}))
    local bad=pose("actor",1);bad.x=math.huge;assert(not lifecycle:receive("world-A",{bad}))
    assert(lifecycle:receive("world-A",{pose("actor",2)}));lifecycle:tick(0)
    assert(ghosts.count==1)
    assert(lifecycle:receive("world-A",{pose("actor",1,99)}));lifecycle:tick(1)
    assert(ghosts.ghosts.actor.tx==10)
    seconds=2.21;lifecycle:tick(122);assert(ghosts.count==0)
    movement.sequence=77;assert(lifecycle:start("world-A",{"actor"},true));assert(movement.sequence==77)
end)
check("lossy movement coalesces and control queue has independent bounded budget",function()
    transport:reset("world-A")
    for i=1,32 do assert(transport:pose("world-A",pose("actor"..i,1))) end
    assert(not transport:pose("world-A",pose("actor33",1)))
    assert(transport:pose("world-A",pose("actor1",2,77)))
    local controls=0
    for i=1,32 do assert(transport:control("world-A",function() controls=controls+1 end)) end
    assert(not transport:control("world-A",function() error("overflow") end))
    local positions=0
    assert(transport:drain("world-A",function() positions=positions+1 end)==6)
    assert(controls==2 and positions==4 and transport.pose_count==28)
    transport:reset("world-B");assert(transport:drain("world-A",function() error("stale") end)==0)
end)
check("expired or lost ghost entities retain a bounded scope sequence fence",function()
    assert(ghosts:bind("world-sequence",{"peer"}));seconds=0
    assert(ghosts:receive("world-sequence",pose("peer",1),0))
    seconds=3;ghosts:tick(180,true);assert(ghosts.count==0)
    assert(not ghosts:receive("world-sequence",pose("peer",1),180))
    assert(ghosts:receive("world-sequence",pose("peer",2),180))
    entities[ghosts.ghosts.peer.entity]=nil;ghosts:tick(181,true)
    assert(not ghosts:receive("world-sequence",pose("peer",2),181))
    assert(ghosts:receive("world-sequence",pose("peer",3),181))
    assert(ghosts:bind("world-sequence",{"peer"}))
    assert(not ghosts:bind("world-sequence",{"replacement"}))
    assert(not ghosts:receive("world-sequence",pose("peer",3),10))
    assert(ghosts:bind("new-authenticated-context",{"peer"}))
    assert(ghosts:receive("new-authenticated-context",pose("peer",1),182))
    assert(not ghosts:receive("world-sequence",pose("peer",4),182))
    local count=0;for _ in pairs(ghosts.sequences) do count=count+1 end;assert(count==1)
    ghosts:clear();assert(next(ghosts.sequences)==nil)
end)
check("local handler faults are contained and one hundred scope replacements clean ghosts",function()
    transport:reset("world-A");transport:control("world-A",function() error("injected handler fault") end)
    transport:drain("world-A",function() end);assert(transport.errors==1)
    for i=1,100 do
        assert(lifecycle:start("world-"..i,{"actor"},true))
        assert(lifecycle:receive("world-"..i,{pose("actor",1)}));lifecycle:tick(i)
        assert(ghosts.count==1)
        lifecycle:stop();assert(ghosts.count==0 and transport.pose_count==0 and transport.control_count==0)
    end
    assert(next(entities)==nil)
end)
check("frequent low actor IDs cannot starve other pending ghosts",function()
    transport:reset("world-A")
    for i=1,32 do assert(transport:pose("world-A",pose("actor"..i,1))) end
    local seen={}
    for pass=1,8 do
        transport:drain("world-A",function(p) seen[p.actor]=true end)
        for i=1,4 do transport:pose("world-A",pose("actor"..i,pass+1)) end
    end
    local total=0;for _ in pairs(seen) do total=total+1 end;assert(total==32)
end)
check("real engine adapters use only a fixed ghost asset and bounded transforms",function()
    local path
    EntityGetIsAlive=function() return true end
    EntityLoad=function(p) path=p;return 1 end
    EntityKill=function() end
    EntitySetTransform=function(_,x,y) assert(x==10 and y==20) end
    local api=load("game_api").ghosts();api.spawn(10,20);api.position(1,10,20,0,1)
    assert(path=="mods/noita_together2/files/g3/ghost.xml")
end)
local gold=load("gold_local")
local function operation(id,direction) return {scope="world-A",id=id or "operation-1",digest=string.rep("a",64),amount=10,direction=direction or "deposit"} end
local function bank(fault)
    local balance,writes,effects,records=100,0,0,{}
    local api={balance=function() return balance end,set_balance=function(v) effects=effects+1;balance=v;if fault=="effect" then error("after mutation") end end}
    local journal={write=function(record) writes=writes+1;if (fault=="intent" and writes==1) or (fault=="receipt" and writes==2) then return false end
        local copy={};for k,v in pairs(record) do copy[k]=v end;records[#records+1]=copy;return true end}
    local guard=gold.new(api,journal,validate);assert(guard:bind("world-A",{}))
    return guard,function() return balance,effects,records end
end
check("gold session duplicate and conflicting operations never apply twice",function()
    local guard,state=bank();local op=operation()
    assert(guard:apply(op,false)=="rejected")
    assert(guard:apply(op,true)=="applied_session")
    assert(guard:apply(op,true)=="applied_session")
    local balance,effects=state();assert(balance==90 and effects==1)
    op.digest=string.rep("b",64);assert(guard:apply(op,true)=="conflict")
    assert(guard:apply(operation("withdraw-1","withdraw"),true)=="applied_session")
    balance,effects=state();assert(balance==100 and effects==2)
end)
check("intent write effect and receipt faults remain blocked or uncertain without replay",function()
    for _,fault in ipairs({"intent","effect","receipt"}) do
        local guard,state=bank(fault);local result=guard:apply(operation(),true)
        assert(result==(fault=="intent" and "blocked_no_effect" or "uncertain"))
        for i=1,10 do assert(guard:apply(operation(),true)==result) end
        local balance,effects=state();assert(effects==(fault=="intent" and 0 or 1))
        assert(balance==(fault=="intent" and 100 or 90))
    end
end)
check("restored evidence never infers loaded-wallet effect from app acknowledgement",function()
    for _,phase in ipairs({"intent","applied_session"}) do for _,balance in ipairs({90,100}) do
        local effects=0
        local guard=gold.new({balance=function() return balance end,set_balance=function() effects=effects+1 end},{write=function() error("no repeat") end},validate)
        assert(guard:bind("world-A",{{id="operation-1",digest=string.rep("a",64),scope="world-A",state=phase}}))
        assert(guard:apply(operation(),true)=="uncertain" and effects==0)
    end end
end)
check("receipt storage exhausts safely rather than evicting unresolved operations",function()
    local guard=gold.new({balance=function() return 10000 end,set_balance=function() end},{write=function() return true end},validate)
    local restored={};for i=1,256 do restored[i]={id="op"..i,digest=string.rep("a",64),scope="world-A"} end
    assert(guard:bind("world-A",restored));assert(guard:apply(operation(),true)=="receipt_limit")
    restored[257]=restored[1];assert(not guard:bind("world-A",restored))
end)
check("persistence probe has explicit opt-in stages and refuses replay",function()
    local source=assert(os.getenv("NT2_G3_PROBE"));local init_path=source:gsub("files/probe.lua","init.lua");assert(loadfile(init_path))
    local module=dofile(source)
    for _,direction in ipairs({"deposit","withdraw"}) do for _,mode in ipairs({"intent_only","effect_only","complete"}) do
        local wallet,receipt,global=20,"",""
        local probe=module.new({balance=function() return wallet end,set_balance=function(_,v) wallet=v end,
            receipt=function() return receipt end,set_receipt=function(_,v) receipt=v end,global=function() return global end,
            set_global=function(v) global=v end,seed=function() return "424242" end,log=function() end})
        local after=direction=="deposit" and 19 or 21
        probe.player=1;assert(probe:step(mode,direction));assert(wallet==(mode=="intent_only" and 20 or after))
        assert(receipt:find(mode=="complete" and "phase=applied" or "phase=intent",1,true))
        assert(not probe:step(mode,direction));assert(wallet==(mode=="intent_only" and 20 or after))
    end end
end)
check("social hooks retain twenty emotes six robes and owned input lifecycle",function()
    local module=load("social_local")
    local catalogue=module.catalogue();local emotes,colors=0,0
    local source=assert(io.open(assert(os.getenv("NT2_G3_SOCIAL_CATALOGUE")),"rb"));local proposal=source:read("*a");source:close()
    local canonical_count=0
    for id,frames in proposal:gmatch('"id"%s*:%s*"([%w_]+)"%s*,%s*"durationFrames"%s*:%s*(%d+)') do
        canonical_count=canonical_count+1;assert(catalogue.emotes[id]==tonumber(frames))
    end
    assert(canonical_count==20)
    local locks={external=true};local next_lock=0;local started,stopped=0,0;local interrupt=false;local alive=true
    local api={acquire_input=function(owner) assert(owner=="nt2_g3_emote");next_lock=next_lock+1;locks[next_lock]=true;return next_lock end,
        release_input=function(token) assert(type(token)=="number" and locks[token]);locks[token]=nil end,
        start_emote=function(actor,id) assert(catalogue.emotes[id]~=nil);started=started+1 end,
        stop_emote=function(actor) stopped=stopped+1 end,set_robe=function(actor,id) end,
        interrupted=function() return interrupt end,alive=function() return alive end}
    local social=module.new(api,validate);assert(social:bind("world-雪","local",{"local","peer"}))
    for id in pairs(catalogue.emotes) do emotes=emotes+1;assert(social:emote("world-雪","local",emotes,id,true,0)) end
    assert(emotes==20 and started==20 and locks.external)
    assert(social:bind("world-雪","local",{"local","peer"}));assert(not social:emote("world-雪","local",20,"wave",true,0))
    assert(not social:emote("world-雪","outsider",21,"wave",true,0))
    assert(not social:emote("world-雪","local",21,"../../script",true,0))
    for _,id in ipairs(catalogue.robes) do colors=colors+1;assert(social:robe("world-雪","peer",id)) end
    assert(colors==6 and catalogue.default_robe=="purple");assert(not social:robe("world-雪","peer","file.xml"))
    assert(social:emote("world-雪","local",21,"wave",true,0));interrupt=true
    social:tick(15,true);assert(social.lock);social:tick(16,true);assert(not social.lock and locks.external)
    interrupt=false;assert(social:emote("world-雪","peer",1,"heart",true,0));assert(not social.lock)
    social:tick(102,true);assert(social.active.peer);social:tick(103,true);assert(not social.active.peer)
    assert(social:emote("world-雪","local",22,"wave",true,104));alive=false;social:tick(105,true);assert(not social.lock)
    alive=true;assert(social:emote("world-雪","local",23,"wave",true,106))
    assert(social:bind("next-world","local",{"local","peer"}));assert(not social.lock and locks.external)
    assert(social:emote("next-world","local",1,"wave",true,0));social:tick(0,false);assert(not social.lock and not social.scope)
    assert(locks.external);for token in pairs(locks) do assert(token=="external") end
    assert(social:bind("fault-world","local",{"local"}))
    api.start_emote=function() error("injected partial render fault") end
    assert(not social:emote("fault-world","local",1,"wave",true,0));assert(not social.lock and not social.active["local"])
    api.acquire_input=function() error("injected input owner fault") end
    assert(not social:emote("fault-world","local",2,"wave",true,0));assert(not social.lock and locks.external)
    api.start_emote=function() end
    api.acquire_input=function() next_lock=next_lock+1;locks[next_lock]=true;return next_lock end
    assert(social:emote("fault-world","local",3,"wave",true,0));local retained=social.lock
    api.release_input=function() error("injected release fault") end
    assert(not social:bind("replacement","local",{"local"}));assert(social.lock==retained and locks[retained] and locks.external)
    api.release_input=function(token) assert(locks[token]);locks[token]=nil end
    assert(social:bind("replacement","local",{"local"}));assert(not social.lock and locks.external)
end)
check("native batch decoder gates protected play malformed frames pause replay and cleanup",function()
    local ffi=require("ffi")
    local module=load("native_owner")
    local context=ffi.new("uint8_t[16]");ffi.fill(context,16,1)
    local batch=ffi.new("uint8_t[864]");local sends=0
    local function encoded(sequence)
        ffi.fill(batch,864,0);ffi.copy(batch,"NG3R",4);batch[4]=2;batch[5]=1;batch[6]=1
        ffi.copy(batch+8,context,16);batch[24]=200;batch[28]=1;ffi.fill(batch+32,16,2);ffi.fill(batch+544,16,2)
        ffi.copy(batch+560,"NTG3",4);batch[564]=1;batch[566]=1;ffi.copy(batch+568,context,16)
        batch[584]=sequence;batch[588]=232;batch[589]=3 -- x=1.0
    end
    local bridge={nt2_g3_move=function() sends=sends+1;return 1 end,
        nt2_g3_peers=function(_,out,size) assert(size==864);ffi.copy(out,batch,864);return 864 end}
    local api={movement=function() return {alive=function(p) return p==player end,time=function() return seconds end,
        position=function() return 5,6,0,1,0,0 end} end,ghosts=function() return ghost_api end}
    local before=next_entity;seconds=0
    local owner=module.new(bridge,context,ffi,api,validate,movement_module,ghost_module)
    encoded(1);owner:tick(player,0,false);assert(sends==0 and next_entity==before)
    owner:tick(player,0,true);assert(sends==1 and next_entity==before+1)
    owner:tick(player,1,false);assert(not entities[before+1])
    owner:tick(player,2,true);assert(next_entity==before+1) -- old pose cannot recreate
    encoded(2);owner:tick(player,3,true);assert(next_entity==before+2)
    batch[623]=1;owner:tick(player,4,true);assert(next_entity==before+2 and not entities[before+2])
    encoded(3);batch[568]=9;owner:tick(player,5,true);assert(next_entity==before+2)
    encoded(3);owner:tick(player,6,true);assert(next_entity==before+3)
    batch[6]=0;owner:tick(player,7,true);assert(entities[before+3]) -- no new samples, current roster remains
    batch[28]=0;ffi.fill(batch+32,512,0);owner:tick(player,8,true);assert(not entities[before+3]) -- confirmed departure
    encoded(4);owner:tick(player,9,true);assert(next_entity==before+4)
    batch[24]=0;owner:tick(player,10,true);assert(not entities[before+4]) -- expired native authority
    encoded(4);owner:tick(player,11,true);assert(next_entity==before+4) -- consumed sequence cannot resurrect
    owner:close();assert(not entities[before+4]);assert(sends==1)
    local entry=assert(loadfile(root.."/../../init-g3.lua"));assert(type(entry)=="function")
end)
check("Retained HUD replay holds and private POI snapshot completion never resurrect dismissal",function()
    local module=load("hud_poi_local");local shown_hud,shown_pois={},{};seconds=0
    local api={time=function() return seconds end,clear_hud=function() shown_hud={} end,clear_pois=function() shown_pois={} end,
        hud=function(actor,hp,max,sampo) shown_hud[actor]={hp=hp,max=max,sampo=sampo} end,remove_hud=function(actor) shown_hud[actor]=nil end,
        poi=function(sequence,actor,text,x,y) shown_pois[sequence]={actor=actor,text=text,x=x,y=y} end,remove_poi=function(sequence) shown_pois[sequence]=nil end}
    local local_state=module.new(api,validate);assert(local_state:bind("scope",1,{"peer"}))
    assert(local_state:receive_hud("scope",1,"peer",1,1000,2000,true,2));assert(shown_hud.peer.sampo)
    assert(not local_state:receive_hud("scope",1,"outsider",1,1,2,false,2))
    seconds=3600;local_state:tick();assert(shown_hud.peer and shown_hud.peer.hp==1000)
    assert(not local_state:receive_hud("scope",1,"peer",1,1,2,false,4))
    local_state:pause();assert(local_state:bind("scope",1,{"peer"}));assert(not local_state:receive_hud("scope",1,"peer",1,1,2,false,4))
    assert(not local_state:receive_hud("scope",1,"peer",2,4294967295,4294967295,false,4))
    assert(not local_state:receive_hud("scope",1,"peer",2,0,0,false,4))
    assert(local_state:receive_hud("scope",1,"peer",2,3000,2000,false,4));assert(shown_hud.peer.hp==3000 and shown_hud.peer.max==2000)
    assert(local_state:receive_hud("scope",1,"peer",3,0,2000,false,4));assert(shown_hud.peer.hp==0)
    assert(local_state:receive_hud("scope",1,"peer",4294967295,2147483647,2147483647,false,4))
    local marker={sequence="1",actor="peer",text="literal 雪 <img onerror=attack>",x=10,y=20}
    assert(local_state:begin_snapshot("scope",1,"2"));assert(local_state:page("scope",1,"2","1",{marker}));assert(next(shown_pois)==nil)
    assert(local_state:page("scope",1,"2","2",{}));assert(shown_pois["1"].text==marker.text)
    assert(local_state:dismiss("1"));assert(not shown_pois["1"])
    -- Lost dismissal acknowledgement; ordinary refresh contains the marker again.
    assert(local_state:begin_snapshot("scope",1,"2"));assert(local_state:page("scope",1,"2","2",{marker}));assert(not shown_pois["1"])
    assert(local_state:dismiss("1"));assert(local_state.dismiss_count==1)
    -- Service high can grow: capture33, finish prefix33, get34 on ordinary refresh.
    local first={};for i=2,32 do first[#first+1]={sequence=tostring(i),actor="peer",text="marker",x=i,y=i} end
    assert(local_state:begin_snapshot("scope",1,"33"));assert(local_state:page("scope",1,"33","32",first))
    local tail={{sequence="33",actor="peer",text="old tail",x=33,y=33},{sequence="34",actor="peer",text="new tail",x=34,y=34}}
    assert(local_state:page("scope",1,"34","34",tail));assert(local_state.playing and shown_hud.peer and shown_pois["33"] and not shown_pois["34"])
    assert(local_state:begin_snapshot("scope",1,"34"));assert(local_state:page("scope",1,"34","34",tail));assert(shown_pois["34"])
    assert(local_state:begin_snapshot("scope",1,"18446744073709551615"))
    assert(local_state:page("scope",1,"18446744073709551615","18446744073709551615",{}));assert(next(shown_pois)==nil)
    assert(local_state:begin_snapshot("scope",1,"2"));assert(not local_state:page("scope",1,"2","0",{}));assert(not local_state.playing)
    assert(local_state:bind("scope",1,{"peer"}));assert(not local_state:bind("scope",2,{"peer"}));assert(next(shown_hud)==nil)
    local_state:close();assert(not local_state.scope and next(shown_pois)==nil)
end)
check("fixed Noita social adapter bounded entities literal names GUI paging cleanup",function()
    local clock=0;local entities={};local created=0;local texts={};local images={};local clicked={};local destroyed=0
    local engine={GameGetRealWorldTimeSinceStarted=function() return clock end,
        EntityLoad=function(path,x,y) assert(path=="mods/noita_together2/files/g3/poi.xml");created=created+1;entities[created]={x=x,y=y};return created end,
        EntityGetIsAlive=function(id) return entities[id]~=nil end,EntityKill=function(id) entities[id]=nil end,
        EntityGetFirstComponentIncludingDisabled=function(id) return id end,ComponentSetValue2=function(id,key,value) entities[id][key]=value end,
        GuiCreate=function() return 55 end,GuiDestroy=function(gui) assert(gui==55);destroyed=destroyed+1 end,
        GuiStartFrame=function() texts={};images={} end,
        GuiButton=function(gui,id,x,y,label) if label=="Next" and x>=200 and clicked.NextPoi then clicked.NextPoi=nil;return true end;if clicked[label] then clicked[label]=nil;return true end;return false end,
        GuiText=function(gui,x,y,text) texts[#texts+1]=text end,
        GuiImage=function(gui,id,x,y,path,alpha,sx) assert(path:find("mods/noita_together2/files/g3/ui/",1,true)==1);images[#images+1]={path=path,sx=sx} end}
    local adapter=load("social_game_api").new(engine,validate)
    assert(adapter:set_peer("peer","$unsafe 雪 <img>",100));assert(not adapter:set_peer("peer","bad\nname"))
    for i=1,20 do adapter.api.poi(tostring(i),"peer","literal <script>",i,i) end
    adapter.api.hud("peer",3000,2000,true);clicked.Players=true;clicked.Messages=true;adapter:tick(true)
    assert(created==8);assert(table.concat(texts,"|"):find(" $unsafe 雪 <img>",1,true));assert(table.concat(texts,"|"):find("HP 3.0 / 2.0",1,true));assert(table.concat(texts,"|"):find("10m",1,true))
    for _,image in ipairs(images) do if image.path:find("hpbar_full",1,true) then assert(image.sx==1) end end
    clicked.NextPoi=true;adapter:tick(true);assert(created==16 and adapter.poi_offset==8)
    adapter:tick(true);assert(created==20)
    local dismissed;adapter:set_dismiss(function(sequence) dismissed=sequence;adapter.api.remove_poi(sequence) end)
    local sequence,entity=next(adapter.entities);entities[entity]=nil;adapter:tick(true);assert(dismissed==sequence and not adapter.pois[sequence])
    adapter:tick(false);assert(next(entities)==nil and not adapter.gui and destroyed==1)
end)
check("social input M-Nee precedence native Q bounded chords and legacy controls coexist",function()
    local controls=load("social_controls");local native_calls=0
    assert(controls.menu_down({ModIsEnabled=function() return true end,is_binding_down=function(group,key) assert(group=="nt2_g3_emotes" and key=="emote_menu");return true end,
        InputIsKeyDown=function() native_calls=native_calls+1;return true end},1));assert(native_calls==0)
    assert(not controls.menu_down({ModIsEnabled=function() return true end,InputIsKeyDown=function() error("M-Nee takes precedence") end},1))
    assert(controls.menu_down({InputIsKeyDown=function(code) return code==20 end},1))
    assert(not controls.menu_down({InputIsKeyDown=function() return true end,ModSettingGet=function() return "key_code" end},1))
    assert(controls.menu_down({ModSettingGet=function() return "key_code,20,mouse_code,1,joystick_code,2" end,InputIsKeyDown=function() return true end,
        InputIsMouseButtonDown=function() return true end,InputIsJoystickButtonDown=function(joystick,code) return joystick==0 and code==2 end},1))
    assert(controls.menu_down({EntityGetFirstComponentIncludingDisabled=function() return 5 end,ComponentGetValue2=function(comp,key) return key:find("mButtonDown",1,true)==1 end},1))
end)
check("twenty fixed emote menu assets six robes ghost-only sprites timing replay and faults",function()
    local visible=true;local components={};local next_component=20;local allocations=0;local chosen=1;local publish_fault=false;local published={};local robe_click=nil
    local engine={GameGetRealWorldTimeSinceStarted=function() return 0 end,EntityGetIsAlive=function(id) return id==1 end,EntityGetFirstComponentIncludingDisabled=function() return 10 end,
        ComponentGetValue2=function() return visible end,ComponentSetValue2=function(comp,key,value) assert(comp==10 and key=="visible");visible=value end,
        EntityAddComponent2=function(entity,kind,fields) assert(entity==1 and kind=="SpriteComponent");assert(fields.image_file:find("mods/noita_together2/files/g3/emotes/",1,true)==1);next_component=next_component+1;components[next_component]=fields;allocations=allocations+1;return next_component end,
        EntityRemoveComponent=function(entity,comp) components[comp]=nil end,
        GuiCreate=function() return 3 end,GuiDestroy=function() end,GuiStartFrame=function() end,
        GuiImageButton=function(gui,id,x,y,label,path) assert(path:find("mods/noita_together2/files/g3/emotes/icons/",1,true)==1);local asset=assert(io.open(os.getenv("NT2_G3_MODULES").."/emotes/icons/"..path:match("([^/]+)$"),"rb"));asset:close();return id==chosen end,
        GuiButton=function(gui,id,x,y,label) return label==robe_click end}
    local lease={scope="scope",generation=1,owner="verified-owner",until_time=10,playing=true}
    local menu=load("social_menu").new(engine,function(kind,id) if publish_fault then error("injected publisher fault") end;published[#published+1]={kind=kind,id=id};return true end,function() return lease end)
    assert(not menu:bind_ghost("original_player",1,false));assert(menu:bind_ghost("owned_ghost",1,true));assert(not menu:emote("owned_ghost","../../unknown",true,0,1))
    local sequence=0
    for i=1,20 do chosen=i;menu:draw(true,true);menu:draw(false,true);local id=published[#published].id;sequence=sequence+1;assert(menu:emote("owned_ghost",id,true,0,sequence));assert(not visible)
        local old_allocations=allocations;sequence=sequence+1;assert(menu:emote("owned_ghost",id,true,10,sequence));assert(allocations==old_allocations);menu:stop("owned_ghost");assert(visible);assert(not menu:emote("owned_ghost",id,true,11,sequence))
    end
    for _,robe in ipairs({"red","orange","yellow","green","blue","purple"}) do robe_click=robe;chosen=1;menu:draw(true,true);assert(menu.robe==robe);menu:draw(false,true);sequence=sequence+1;assert(menu:emote("owned_ghost","wave",true,0,sequence));menu:stop("owned_ghost") end
    robe_click=nil;publish_fault=true;menu:draw(true,true);menu:draw(false,true);assert(menu.selected=="wave")
    publish_fault=false;sequence=sequence+1;assert(menu:emote("owned_ghost","heart",true,0,sequence));menu:tick(102,true,false);assert(not visible);menu:tick(103,true,false);assert(visible)
    menu:tick(104,false,false);assert(menu:bind_ghost("owned_ghost",1,true));assert(not menu:emote("owned_ghost","heart",true,105,sequence))
    assert(not menu:emote("owned_ghost","heart",true,0/0,sequence+1));assert(not menu:emote("owned_ghost","heart",true,1.5,sequence+1))
    lease.until_time=0;assert(not menu:emote("owned_ghost","wave",true,106,sequence+1));lease.until_time=10;lease.generation=2
    assert(menu:bind_ghost("owned_ghost",1,true));assert(menu:emote("owned_ghost","wave",true,107,1))
    menu:close();assert(not menu.gui and next(components)==nil)
end)
check("historical marker admission never adds live HUD authority",function()
    local api={time=function() return 0 end,clear_hud=function() end,clear_pois=function() end,hud=function() end,remove_hud=function() end,poi=function() end,remove_poi=function() end}
    local state=load("hud_poi_local").new(api,validate);assert(state:bind("scope",1,{"live"}))
    assert(state:admit_marker_actor("scope",1,"historical"));assert(not state:receive_hud("scope",1,"historical",1,1,2,false,1))
    assert(state:begin_snapshot("scope",1,"1"));assert(state:page("scope",1,"1","1",{{sequence="1",actor="historical",text="departed message",x=0,y=0}}))
    assert(state:dismiss("1"));state:pause();assert(state:bind("scope",1,{"live"}));assert(state:begin_snapshot("scope",1,"1"));assert(state:page("scope",1,"1","1",{{sequence="1",actor="historical",text="departed message",x=0,y=0}}));assert(not state.markers["1"])
    state:close();assert(next(state.marker_roster)==nil)
end)
check("bounded social frames validate before callbacks and retain hold fences",function()
 local dispatch=load("social_dispatch")
 local function u(n,count) local s="";for _=1,count do s=s..string.char(n%256);n=math.floor(n/256) end;return s end
 local context=string.rep("\1",16)
 local function frame(sequence,body,count) return "NSF1\1\0\0\0"..context..u(1,4)..u(sequence,4)..u(#body,4)..u(count,2).."\0\0"..body end
 local function str(s) return u(#s,2)..s end
 local hud="\1"..str("peer")..str("雪")..u(1,4)..u(3000,4)..u(2000,4).."\1"
 local raw=frame(1,hud,1);local batch=assert(dispatch.decode(raw));assert(batch.commands[1].name=="雪" and batch.commands[1].sequence==1)
 assert(not dispatch.decode(raw:sub(1,-2)));assert(not dispatch.decode(raw.."x"));assert(not dispatch.decode(frame(2,"\2"..str("\192\128"),1)))
 assert(not dispatch.decode(frame(2,"\2"..str("\237\160\128"),1)));assert(not dispatch.decode(frame(2,"\2"..str("\194\128"),1)))
 local lease={context=context,generation=1,owner="native-owner",now=100,until_time=101,playing=true};local applied=0;local paused=0
 local d=dispatch.new(function() return lease end,function(commands,now) applied=applied+1;assert(now==100 and #commands==1);return true end,function() paused=paused+1 end)
 assert(d:receive(raw));assert(not d:receive(raw));lease.playing=false;assert(not d:receive(raw));lease.playing=true;assert(not d:receive(raw));assert(applied==1 and paused>=2)
 assert(d:receive(frame(2,"\5",1)));lease.until_time=100;assert(not d:receive(frame(3,"\5",1)));assert(applied==2)
 lease.until_time=101;lease.generation=2;assert(not d:receive(frame(3,"\5",1)))
end)
check("shared component leases preserve snapshots co-owners foreign writes and blocked restoration",function()
 local entry=assert(os.getenv("NT2_G3_MODULES")):gsub('/files/g3$','/init-g3-social.lua');assert(loadfile(entry)) -- Syntax only; never execute Noita entry here.
 local values={[1]=true,[2]=false};local allowed=true;local exists=true
 local api={alive=function() return exists end,read=function(d) return values[d.component] end,write=function(d,v) values[d.component]=v;return true end,may_restore=function() return allowed end}
 local leases=load("component_leases").new(api,load("validation"));local d={{entity=7,component=1,kind='controls',blocked=false},{entity=7,component=2,kind='inventory',blocked=false}}
 local a,ok=leases:acquire('emote',d);assert(a and ok and not values[1]);local b=assert(leases:acquire('other-mod',d));assert(leases:release(a) and not values[1]);allowed=false;assert(not leases:release(b) and not values[1]);allowed=true;assert(leases:release(b) and values[1] and not values[2])
 local c=assert(leases:acquire('emote',{{entity=7,component=1,kind='wand',blocked=false}}));values[1]=true;assert(leases:release(c) and values[1])
 local descriptor={entity=7,component=1,kind='cape',blocked=false};local captured=assert(leases:acquire('emote',{descriptor}));descriptor.component=2;assert(leases:release(captured) and values[1] and not values[2])
 local e=assert(leases:acquire('emote',{{entity=7,component=1,kind='hat',blocked=false}}));exists=false;assert(leases:release(e));assert(not leases:release(999));assert(not leases:acquire('emote',{{entity=7,component=1,kind='arbitrary',blocked=false}}))
end)
check("actual social entry initialization lease frame hold and callback fault",function()
 dofile(assert(os.getenv("NT2_G3_MODULES")):gsub('/mod/noita_together2/files/g3$','/apps/desktop/tools/g3-social-entry-test.lua'))
end)
local file=assert(io.open(assert(os.getenv("NT2_G3_RESULT")),"wb"))
file:write('{"passed":true,"count":'..#checks..',"scope":"original installed LuaJIT with injected game APIs; no actual Noita or network authentication acceptance","actualNoita":false,"checks":[')
for i,name in ipairs(checks) do if i>1 then file:write(',') end;file:write('"'..name..'"') end
file:write(']}');file:close()


