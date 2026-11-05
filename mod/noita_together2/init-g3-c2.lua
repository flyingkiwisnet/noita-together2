-- UNAPPROVED C2 shared-broker entry; pre-init seed acquisition precedes generation.
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
local function log(event,detail) print("NT2_G2|event="..event.."|frame="..tostring(GameGetFrameNum()).."|detail="..tostring(detail)) end
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
    hold=nil
end
local function protect(p)
    if hold then return end
    local controls=EntityGetFirstComponentIncludingDisabled(p,"ControlsComponent")
    local damage=EntityGetFirstComponentIncludingDisabled(p,"DamageModelComponent")
    if not controls or not damage then error("Player protection components unavailable") end
    local x,y=EntityGetTransform(p)
    -- Capture before mutation, so failure can preserve a recoverable safe state.
    hold={player=p,controls=controls,damage=damage,enabled=ComponentGetValue2(controls,"enabled"),wait=ComponentGetValue2(damage,"wait_for_kill_flag_on_death"),poly=not EntityHasTag(p,"polymorphable_NOT"),x=x,y=y}
    local flags=(hold.enabled and "1" or "0")..(hold.wait and "1" or "0")..(hold.poly and "1" or "0")
    hold.inventory={}
    for _,kind in ipairs(inventory_types) do
        local component=EntityGetFirstComponentIncludingDisabled(p,kind)
        if component then local enabled=ComponentGetIsEnabled(component);hold.inventory[#hold.inventory+1]={id=component,enabled=enabled};flags=flags..(enabled and "1" or "0") else flags=flags.."-" end
    end
    hold.recovery=EntityAddComponent2(p,"VariableStorageComponent",{name="nt2_g3_recovery",value_string=flags})
    if not hold.recovery or hold.recovery==0 then error("Protection recovery marker unavailable") end
    ComponentSetValue2(controls,"enabled",false)
    ComponentSetValue2(damage,"wait_for_kill_flag_on_death",true)
    for _,entry in ipairs(hold.inventory) do EntitySetComponentIsEnabled(p,entry.id,false) end
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
    if not seed or loaded then return end
    player=p
    local ok,err=pcall(protect,p)
    if not ok then close();log("protection_failed",err);return end
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
    else
        local ok,err=pcall(protect,player)
        if not ok then close();log("protection_failed",err) end
        if hold then
            ComponentSetValue2(hold.controls,"enabled",false)
            for _,entry in ipairs(hold.inventory) do EntitySetComponentIsEnabled(player,entry.id,false) end
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
    if not hold or offline then return end
    if not gui then gui=GuiCreate() end
    GuiStartFrame(gui)
    GuiText(gui,10,35,"NT2: protected while waiting for host inspection / Start")
    GuiText(gui,10,47,"Use the normal menu to leave. A replacement world needs a new run.")
    if GuiButton(gui,9021,10,62,"Leave NT2 and continue this world offline") then
        -- Explicit user opt-out only; server loses this selected game's lease.
        close();restore();offline=true;log("offline_opt_out",seed)
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
