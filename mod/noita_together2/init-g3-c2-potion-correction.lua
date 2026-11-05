-- UNAPPROVED separate potion correction candidate; pre-init seed acquisition precedes generation.
ModMagicNumbersFileAdd("mods/noita_together2/files/logging.xml")
dofile_once("data/scripts/lib/utilities.lua")
local bridge,context,guard,seed,accepted,loaded,player,hold,gui
local gameplay, social_gameplay
local tried,offline=false,false
local actual_seed_matches=false
local inspection=dofile("mods/noita_together2/files/inspection.lua")
local report_sequence,report_body,report_content,next_scan=0,nil,nil,0
local released,release_blocked=false,false
local inventory_types={"InventoryGuiComponent","Inventory2Component","ItemPickUpperComponent"}
-- NT2_POTION_DIAGNOSTIC_BEGIN
-- Inlined only into the separate diagnostic entry. Every engine call is a getter.
local potion_trace=(function()
    local m={}
    local seen,tracked,tracked_set={}, {}, {}
    local samples,spawn_frame,subject=0,nil,nil
    local calls,errors,budget_hit=0,0,false
    local function read(name,...)
        if calls>=512 then budget_hit=true;return nil end
        calls=calls+1
        local fn=_G[name]
        if type(fn)~="function" then errors=errors+1;return nil end
        local ok,a,b=pcall(fn,...)
        if not ok then errors=errors+1;return nil end
        return a,b
    end
    local function scalar(value)
        if value==true then return "1" end
        if value==false then return "0" end
        if type(value)=="number" and value==value and math.abs(value)<=1000000000 then return string.format("%.2f",value) end
        return "?" -- no arbitrary strings, names, paths or error text in output
    end
    local function id(value)
        if type(value)=="number" and value==math.floor(value) and value>=0 and value<=4294967295 then return string.format("%.0f",value) end
        return "?"
    end
    local function component(entity,kind)
        local c=read("EntityGetFirstComponentIncludingDisabled",entity,kind)
        if type(c)=="number" and c>0 then return c end
    end
    local function enabled(c) return c and scalar(read("ComponentGetIsEnabled",c)) or "-" end
    local function value(c,field) return c and scalar(read("ComponentGetValue2",c,field)) or "-" end
    local function tags(entity)
        local out={}
        -- Fixed whitelist, in this order; never log arbitrary mod tag strings.
        for _,tag in ipairs({"potion","wand","item_pickup","item_physics","hittable","teleportable_NOT"}) do out[#out+1]=scalar(read("EntityHasTag",entity,tag)) end
        return table.concat(out,"")
    end
    local function remember(entity)
        if not tracked_set[entity] and #tracked<4 and read("EntityHasTag",entity,"potion")==true then tracked[#tracked+1]=entity;tracked_set[entity]=true end
    end
    local function snapshot(phase,p)
        calls,errors,budget_hit=0,0,false
        local frame=read("GameGetFrameNum")
        if not spawn_frame then spawn_frame=frame end
        subject=p
        local controls=component(p,"ControlsComponent")
        local inventory=component(p,"Inventory2Component")
        local gui_component=component(p,"InventoryGuiComponent")
        local pickup=component(p,"ItemPickUpperComponent")
        local quick,children,child_rows=nil,nil,{}
        local player_children=read("EntityGetAllChildren",p)
        if type(player_children)=="table" then
            for i=1,math.min(#player_children,16) do
                local child=player_children[i]
                if read("EntityGetName",child)=="inventory_quick" then quick=child;break end
            end
        end
        if quick then children=read("EntityGetAllChildren",quick) end
        if type(children)=="table" then
            for i=1,math.min(#children,12) do
                local child=children[i];remember(child)
                local item=component(child,"ItemComponent")
                child_rows[#child_rows+1]=id(child)..","..id(read("EntityGetParent",child))..","..id(read("EntityGetRootEntity",child))..","..tags(child)..","..enabled(item)
            end
        end
        local potion_rows={}
        for _,entity in ipairs(tracked) do
            local alive=read("EntityGetIsAlive",entity)
            if alive==true then
                local item=component(entity,"ItemComponent")
                local material=component(entity,"MaterialInventoryComponent")
                local physics=component(entity,"PhysicsBodyComponent")
                local physics2=component(entity,"PhysicsBody2Component")
                local damage=component(entity,"DamageModelComponent")
                local potion=component(entity,"PotionComponent")
                local x,y=read("EntityGetTransform",entity)
                local slot_x,slot_y
                if item then slot_x,slot_y=read("ComponentGetValue2",item,"inventory_slot") end
                potion_rows[#potion_rows+1]=table.concat({id(entity),scalar(alive),id(read("EntityGetParent",entity)),id(read("EntityGetRootEntity",entity)),scalar(x),scalar(y),tags(entity),enabled(item),value(item,"drinkable"),value(item,"is_pickable"),value(item,"has_been_picked_by_player"),scalar(slot_x),scalar(slot_y),enabled(material),id(read("GetMaterialInventoryMainMaterial",entity)),enabled(physics),enabled(physics2),value(physics2,"mInitialized"),enabled(damage),enabled(potion)},",")
            else potion_rows[#potion_rows+1]=id(entity)..","..scalar(alive) end
        end
        local line="NT2_POTION|v=1|phase="..phase.."|frame="..id(frame).."|player="..id(p).."|controls="..value(controls,"enabled").."|inventory="..enabled(inventory).."|initialized="..value(inventory,"mInitialized").."|gui="..enabled(gui_component).."|pickup="..enabled(pickup).."|quick="..id(quick).."|child_count="..(type(children)=="table" and id(#children) or "?").."|children=["..table.concat(child_rows,";").."]|tracked=["..table.concat(potion_rows,";").."]|calls="..calls.."|errors="..errors.."|budget="..(budget_hit and "1" or "0")
        line=line.."|player_child_count="..(type(player_children)=="table" and id(#player_children) or "?").."|limits=16,12,4"
        if #line>4070 then line=line:sub(1,4070).."|output_truncated=1" end
        if type(_G.print)=="function" then pcall(_G.print,line) end
    end
    function m:sample(phase,p)
        if seen[phase] or samples>=8 or type(p)~="number" or p<=0 then return end
        seen[phase]=true;samples=samples+1
        -- Diagnostic failure must never escape into the entry's authority path.
        local ok=pcall(snapshot,phase,p)
        if not ok and type(_G.print)=="function" then pcall(_G.print,"NT2_POTION|v=1|phase="..phase.."|diagnostic_unavailable=1") end
    end
    function m:frame(waiting)
        if not subject or samples>=8 or seen.waiting_after_60_frames then return end
        if not seen.first_post_update then self:sample("first_post_update",subject);return end
        if not waiting or type(spawn_frame)~="number" then return end
        -- Only this cheap documented frame getter runs between the two samples.
        local fn=_G.GameGetFrameNum
        if type(fn)~="function" then return end
        local ok,frame=pcall(fn)
        if ok and type(frame)=="number" and frame>=spawn_frame+60 then self:sample("waiting_after_60_frames",subject) end
    end
    return m
end)()
-- NT2_POTION_DIAGNOSTIC_END
local function log(event,detail) print("NT2_G2|event="..event.."|frame="..tostring(GameGetFrameNum()).."|detail="..tostring(detail)) end
-- Legacy waiting approach: inventory keeps its engine initialization/update.
-- Only owned drinkable flags are changed; item-local markers survive a held save.
local potion_marker="nt2_g3_drinkable_recovery_v1"
local function potion_items(p)
    local result={}
    local children=EntityGetAllChildren(p) or {}
    for i=1,math.min(#children,16) do
        local quick=children[i]
        if EntityGetName(quick)=="inventory_quick" then
            local items=EntityGetAllChildren(quick) or {}
            for j=1,math.min(#items,12) do
                local item=items[j]
                if EntityGetIsAlive(item) and EntityHasTag(item,"potion") then
                    local component=EntityGetFirstComponentIncludingDisabled(item,"ItemComponent")
                    if component then result[#result+1]={entity=item,id=component} end
                end
            end
        end
    end
    return result
end
local function restore_potion(entry)
    if not EntityGetIsAlive(entry.entity) then return end
    -- Never write a stale/replaced ItemComponent.
    if EntityGetFirstComponentIncludingDisabled(entry.entity,"ItemComponent")==entry.id then
        ComponentSetValue2(entry.id,"drinkable",entry.drinkable)
    end
    if entry.marker then EntityRemoveComponent(entry.entity,entry.marker) end
end
local function recover_potions(p)
    for _,entry in ipairs(potion_items(p)) do
        for _,marker in ipairs(EntityGetComponentIncludingDisabled(entry.entity,"VariableStorageComponent") or {}) do
            if ComponentGetValue2(marker,"name")==potion_marker then
                local value=ComponentGetValue2(marker,"value_string")
                if value=="1" or value=="0" then
                    entry.drinkable=value=="1";entry.marker=marker;restore_potion(entry)
                end
            end
        end
    end
end
local function protect_potions()
    for _,entry in ipairs(potion_items(hold.player)) do
        local original=hold.potions[entry.entity]
        if not original then
            if hold.potion_count>=12 then error("Protected potion bound exceeded") end
            entry.drinkable=ComponentGetValue2(entry.id,"drinkable")
            if type(entry.drinkable)~="boolean" then error("Potion drinkable flag unavailable") end
            if entry.drinkable then
                entry.marker=EntityAddComponent2(entry.entity,"VariableStorageComponent",{name=potion_marker,value_string="1"})
                if not entry.marker or entry.marker==0 then error("Potion recovery marker unavailable") end
            end
            -- Publish only after the changed flag has durable recovery data.
            -- A failed creation must retry without mutating/caching the flag.
            hold.potions[entry.entity]=entry
            hold.potion_count=hold.potion_count+1
            original=entry
        end
        if original.id==entry.id then ComponentSetValue2(entry.id,"drinkable",false) end
    end
end
local function restore()
    if not hold then return end
    if EntityGetIsAlive(hold.player) then
        ComponentSetValue2(hold.controls,"enabled",hold.enabled)
        ComponentSetValue2(hold.damage,"wait_for_kill_flag_on_death",hold.wait)
        for _,entry in ipairs(hold.inventory or {}) do EntitySetComponentIsEnabled(hold.player,entry.id,entry.enabled) end
        if hold.poly then EntityRemoveTag(hold.player,"polymorphable_NOT") end
        if hold.effect and EntityGetIsAlive(hold.effect) then EntityKill(hold.effect) end
        if hold.recovery then EntityRemoveComponent(hold.player,hold.recovery) end
    end
    for _,entry in pairs(hold.potions or {}) do restore_potion(entry) end
    hold=nil
end
local function protect(p)
    if hold then protect_potions();return end
    local controls=EntityGetFirstComponentIncludingDisabled(p,"ControlsComponent")
    local damage=EntityGetFirstComponentIncludingDisabled(p,"DamageModelComponent")
    if not controls or not damage then error("Player protection components unavailable") end
    local x,y=EntityGetTransform(p)
    -- Capture before mutation, so failure can preserve a recoverable safe state.
    hold={player=p,controls=controls,damage=damage,enabled=ComponentGetValue2(controls,"enabled"),wait=ComponentGetValue2(damage,"wait_for_kill_flag_on_death"),poly=not EntityHasTag(p,"polymorphable_NOT"),x=x,y=y}
    local flags=(hold.enabled and "1" or "0")..(hold.wait and "1" or "0")..(hold.poly and "1" or "0")
    -- Retain the old six-character recovery format; inventory was not mutated.
    hold.inventory={};hold.potions={};hold.potion_count=0;flags=flags.."---"
    hold.recovery=EntityAddComponent2(p,"VariableStorageComponent",{name="nt2_g3_recovery",value_string=flags})
    if not hold.recovery or hold.recovery==0 then error("Protection recovery marker unavailable") end
    ComponentSetValue2(controls,"enabled",false)
    ComponentSetValue2(damage,"wait_for_kill_flag_on_death",true)
    protect_potions()
    if hold.poly then EntityAddTag(p,"polymorphable_NOT") end
    hold.effect=LoadGameEffectEntityTo(p,"data/entities/misc/effect_protection_all.xml")
    if not hold.effect or hold.effect==0 then error("Protection effect unavailable") end
    EntityAddTag(hold.effect,"nt2_g3_owned_hold")
    local effect=EntityGetFirstComponentIncludingDisabled(hold.effect,"GameEffectComponent")
    if not effect then error("Protection effect component unavailable") end
    ComponentSetValue2(effect,"frames",-1)
end
local function close()
    if social_gameplay then social_gameplay:close();social_gameplay=nil end
    if gameplay then gameplay:close();gameplay=nil end
    if context and bridge then bridge.nt2_g3_close(context) end
    if guard then require("ffi").gc(guard,nil);guard=nil end
end
function OnModPreInit()
    if tried then return end;tried=true
    local ok,err=pcall(function()
        local ffi=require("ffi")
        ffi.cdef[[int nt2_g3_startup_prepare(uint32_t*,uint8_t*); int nt2_g3_social_terminal(const uint8_t*,uint8_t*,uint32_t); int nt2_g3_social_lease(const uint8_t*,uint8_t*,uint32_t); int nt2_g3_social_next(const uint8_t*,uint8_t*,uint32_t); int nt2_g3_move(const uint8_t*,uint32_t,int32_t,int32_t,int32_t,int32_t,uint32_t); int nt2_g3_peers(const uint8_t*,uint8_t*,uint32_t); int nt2_g3_prepare(uint32_t*,uint8_t*); int nt2_g3_seed_accepted(const uint8_t*,uint32_t); int nt2_g3_world_loaded(const uint8_t*); int nt2_g3_status(const uint8_t*); int nt2_g3_heartbeat_diagnostic(const uint8_t*); int nt2_g3_close(const uint8_t*); int nt2_g3_report(const uint8_t*,const uint8_t*,uint32_t,uint64_t);]]
        local native=ffi.load("mods/noita_together2/files/nt2_g3_transport.dll")
        local social=ffi.load("mods/noita_together2/files/nt2_g3_social.dll")
        bridge={nt2_g3_prepare=native.nt2_g3_startup_prepare,nt2_g3_seed_accepted=native.nt2_g3_seed_accepted,nt2_g3_world_loaded=native.nt2_g3_world_loaded,nt2_g3_status=native.nt2_g3_status,nt2_g3_heartbeat_diagnostic=native.nt2_g3_heartbeat_diagnostic,nt2_g3_close=native.nt2_g3_close,nt2_g3_report=native.nt2_g3_report,nt2_g3_move=native.nt2_g3_move,nt2_g3_peers=native.nt2_g3_peers,nt2_g3_social_lease=social.nt2_g3_social_lease,nt2_g3_social_next=social.nt2_g3_social_next,nt2_g3_social_terminal=social.nt2_g3_social_terminal}
        -- Function pointers alone do not retain ffi.load namespaces. Keep both
        -- wrapper images alive for every callback using this bound bridge.
        bridge._libraries={native,social}
        context=ffi.new("uint8_t[16]");local result=ffi.new("uint32_t[1]")
        local status=bridge.nt2_g3_prepare(result,context);log("prepare",status)
        if status==1 then
            -- The finalizer captures this world's immutable token, never the mutable global context.
            local token=ffi.new("uint8_t[16]");ffi.copy(token,context,16);local native_close=bridge.nt2_g3_close
            guard=ffi.gc(token,function(t) native.nt2_g3_close(t) end)
            local shared_seed=ffi.new("uint32_t[1]");local shared_context=ffi.new("uint8_t[16]")
            assert(social.nt2_g3_prepare(shared_seed,shared_context)==1 and tonumber(shared_seed[0])==tonumber(result[0]) and ffi.string(shared_context,16)==ffi.string(context,16),"Shared primary/social world refused")
            seed=tonumber(result[0]);SetWorldSeed(seed);log("seed_requested",seed)
        else context=nil end
    end)
    if not ok then close();log("bridge_error",tostring(err):sub(1,240)) end
end
function OnMagicNumbersAndWorldSeedInitialized()
    if not seed then return end
    local actual=tonumber(StatsGetValue("world_seed"))
    actual_seed_matches=actual==seed
    accepted=actual_seed_matches and bridge.nt2_g3_seed_accepted(context,actual)==1
    log("seed_accepted",tostring(accepted)..";expected="..seed..";actual="..tostring(actual))
    -- Keep the bound world available for a wrong-seed diagnostic; never treat it as accepted.
end
local function observe()
    local report=inspection.collect(player~=nil,hold~=nil)
    report.modVersion="nt2-g3-c2-dev-1"
    report.bridgeVersion="g3-c2-ipc-1"
    local content=inspection.content(report)
    local changed=report_content~=nil and content~=report_content
    if changed and released then release_blocked=true end
    if changed and not hold then
        local ok,err=pcall(protect,player)
        if not ok then close();log("protection_failed",err);return false end
        report.protected=true
    end
    local body,within_limits=inspection.encode(report)
    if not within_limits then report.complete=false end
    if body~=report_body then
        if report_sequence>=4294967295 then close();log("report_failed","sequence_limit");return false end
        report_sequence=report_sequence+1;report_body=body
    end
    local acknowledged=context and bridge and bridge.nt2_g3_report(context,body,#body,report_sequence)==1
    if acknowledged then report_content=content end
    log("inspection",tostring(acknowledged)..";sequence="..report_sequence..";complete="..tostring(report.complete))
    return acknowledged and report.complete and report.actualSeed==string.format("%.0f",seed) and not changed and not release_blocked
end
function OnPlayerSpawned(p)
    potion_trace:sample("spawn_before",p) -- NT2_POTION_DIAGNOSTIC_HOOK
    -- A saved waiting world can be continued solo without inheriting our temporary controls/effect.
    for _,component in ipairs(EntityGetComponentIncludingDisabled(p,"VariableStorageComponent") or {}) do
        if ComponentGetValue2(component,"name")=="nt2_g3_recovery" then
            local flags=ComponentGetValue2(component,"value_string")
            local controls=EntityGetFirstComponentIncludingDisabled(p,"ControlsComponent")
            local damage=EntityGetFirstComponentIncludingDisabled(p,"DamageModelComponent")
            if controls then ComponentSetValue2(controls,"enabled",flags:sub(1,1)=="1") end
            if damage then ComponentSetValue2(damage,"wait_for_kill_flag_on_death",flags:sub(2,2)=="1") end
            for index,kind in ipairs(inventory_types) do
                local value=flags:sub(index+3,index+3);local component=EntityGetFirstComponentIncludingDisabled(p,kind)
                if component and (value=="0" or value=="1") then EntitySetComponentIsEnabled(p,component,value=="1") end
            end
            if flags:sub(3,3)=="1" then EntityRemoveTag(p,"polymorphable_NOT") end
            for _,child in ipairs(EntityGetAllChildren(p) or {}) do if EntityHasTag(child,"nt2_g3_owned_hold") then EntityKill(child) end end
            EntityRemoveComponent(p,component);log("saved_hold_restored","continue")
        end
    end
    recover_potions(p)
    if not seed or loaded then potion_trace:sample("unbound_after_recovery",p) end -- NT2_POTION_DIAGNOSTIC_HOOK
    if not seed or loaded then return end
    player=p
    local ok,err=pcall(protect,p)
    if not ok then close();log("protection_failed",err);return end
    potion_trace:sample("spawn_after_protect",p) -- NT2_POTION_DIAGNOSTIC_HOOK
    if accepted then loaded=bridge.nt2_g3_world_loaded(context)==1 end
    log("world_loaded",tostring(loaded)..";seed="..seed)
    local acknowledged=observe();next_scan=GameGetFrameNum()+(acknowledged and 300 or 1)
end
function OnWorldPreUpdate()
    if not player or offline then return end
    -- Poll the same immutable control originals; frame callbacks never wait.
    if actual_seed_matches and not accepted then accepted=bridge.nt2_g3_seed_accepted(context,seed)==1 end
    if accepted and not loaded then loaded=bridge.nt2_g3_world_loaded(context)==1 end
    local status=accepted and loaded and context and bridge.nt2_g3_status(context) or 0
    -- Recollect on the game thread immediately before release. Pause cannot fabricate a rescan.
    if status==2 and hold then
        if not observe() or bridge.nt2_g3_status(context)~=2 then status=1 end
    elseif GameGetFrameNum()>=next_scan then
        local acknowledged=observe();next_scan=GameGetFrameNum()+(acknowledged and 300 or 1)
        status=accepted and loaded and context and bridge.nt2_g3_status(context) or 0
        if release_blocked then status=1 end
    end
    if status==2 then
        if hold then restore();released=true;log("released",seed);observe() end
        potion_trace:sample("start_restored",player) -- NT2_POTION_DIAGNOSTIC_HOOK
    else
        local ok,err=pcall(protect,player)
        if not ok then close();log("protection_failed",err) end
        if released and hold then potion_trace:sample("reprotected",player) end -- NT2_POTION_DIAGNOSTIC_HOOK
        if hold then
            ComponentSetValue2(hold.controls,"enabled",false)
            EntitySetTransform(player,hold.x,hold.y)
            local velocity=EntityGetFirstComponentIncludingDisabled(player,"VelocityComponent")
            if velocity then ComponentSetValue2(velocity,"mVelocity",0,0) end
        end
    end
end
local protected_update=OnWorldPreUpdate
function OnWorldPreUpdate()
    protected_update()
    if not context or not bridge or not player or offline then if gameplay then gameplay:close();gameplay=nil end;return end
    if not gameplay then
        gameplay=dofile("mods/noita_together2/files/g3/native_owner.lua").new(bridge,context,require("ffi"),
            dofile("mods/noita_together2/files/g3/game_api.lua"),dofile("mods/noita_together2/files/g3/validation.lua"),
            dofile("mods/noita_together2/files/g3/movement.lua"),dofile("mods/noita_together2/files/g3/ghosts.lua"))
    end
    local ok,err=pcall(gameplay.tick,gameplay,player,GameGetFrameNum(),not hold and accepted and loaded and bridge.nt2_g3_status(context)==2)
    if not ok then
        local owned=gameplay;gameplay=nil;pcall(owned.close,owned)
        close();release_blocked=true;log("g3_gameplay_failed",err)
    end
end
function OnWorldPostUpdate()
    potion_trace:frame(hold~=nil) -- NT2_POTION_DIAGNOSTIC_HOOK
    if not hold or offline then return end
    if not gui then gui=GuiCreate() end
    GuiStartFrame(gui)
    GuiText(gui,10,35,"NT2: protected while waiting for host inspection / Start")
    GuiText(gui,10,47,"Use the normal menu to leave. A replacement world needs a new run.")
    if GuiButton(gui,9021,10,62,"Leave NT2 and continue this world offline") then
        -- Explicit user opt-out only; server loses this selected game's lease.
        close();restore();offline=true;log("offline_opt_out",seed)
        potion_trace:sample("offline_restored",player) -- NT2_POTION_DIAGNOSTIC_HOOK
    end
end


-- UNAPPROVED separate source entry. Never replace installed/reviewed init-g3.lua.
local seed_ghost_update=OnWorldPreUpdate
function OnWorldPreUpdate()
    seed_ghost_update()
    if not context or not bridge or not player or offline then
        if social_gameplay then social_gameplay:close();social_gameplay=nil end
        return
    end
    if not social_gameplay then
        social_gameplay=dofile("mods/noita_together2/files/g3/social_native.lua").new(bridge,context,require("ffi"),_G,
            dofile("mods/noita_together2/files/g3/validation.lua"),dofile("mods/noita_together2/files/g3/social_game_api.lua"),
            dofile("mods/noita_together2/files/g3/hud_poi_local.lua"),dofile("mods/noita_together2/files/g3/social_dispatch.lua"))
    end
    local ok,err=pcall(social_gameplay.tick,social_gameplay,not hold and accepted and loaded and bridge.nt2_g3_status(context)==2)
    if not ok then close();release_blocked=true;log("g3_social_failed",err) end
end
