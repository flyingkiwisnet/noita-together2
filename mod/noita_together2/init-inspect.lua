ModMagicNumbersFileAdd("mods/noita_together2/files/logging.xml")
dofile_once("data/scripts/lib/utilities.lua")
local bridge,context,guard,seed,accepted,loaded,player,hold,gui
local tried,offline=false,false
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
    hold.recovery=EntityAddComponent2(p,"VariableStorageComponent",{name="nt2_g2_recovery",value_string=flags})
    if not hold.recovery or hold.recovery==0 then error("Protection recovery marker unavailable") end
    ComponentSetValue2(controls,"enabled",false)
    ComponentSetValue2(damage,"wait_for_kill_flag_on_death",true)
    for _,entry in ipairs(hold.inventory) do EntitySetComponentIsEnabled(p,entry.id,false) end
    if hold.poly then EntityAddTag(p,"polymorphable_NOT") end
    hold.effect=LoadGameEffectEntityTo(p,"data/entities/misc/effect_protection_all.xml")
    if not hold.effect or hold.effect==0 then error("Protection effect unavailable") end
    EntityAddTag(hold.effect,"nt2_g2_owned_hold")
    local effect=EntityGetFirstComponentIncludingDisabled(hold.effect,"GameEffectComponent")
    if not effect then error("Protection effect component unavailable") end
    ComponentSetValue2(effect,"frames",-1)
end
local function close()
    if context and bridge then bridge.nt2_g2_close(context) end
    if guard then require("ffi").gc(guard,nil);guard=nil end
end
function OnModPreInit()
    if tried then return end;tried=true
    local ok,err=pcall(function()
        local ffi=require("ffi")
        ffi.cdef[[int nt2_g2i_prepare(uint32_t*,uint8_t*); int nt2_g2i_seed_accepted(const uint8_t*,uint32_t); int nt2_g2i_world_loaded(const uint8_t*); int nt2_g2i_status(const uint8_t*); int nt2_g2i_close(const uint8_t*); int nt2_g2i_report(const uint8_t*,const uint8_t*,uint32_t,uint64_t);]]
        local native=ffi.load("mods/noita_together2/files/nt2_g2_inspect_bridge.dll")
        bridge={nt2_g2_prepare=native.nt2_g2i_prepare,nt2_g2_seed_accepted=native.nt2_g2i_seed_accepted,nt2_g2_world_loaded=native.nt2_g2i_world_loaded,nt2_g2_status=native.nt2_g2i_status,nt2_g2_close=native.nt2_g2i_close,nt2_g2i_report=native.nt2_g2i_report}
        context=ffi.new("uint8_t[16]");local result=ffi.new("uint32_t[1]")
        local status=bridge.nt2_g2_prepare(result,context);log("prepare",status)
        if status==1 then
            -- The finalizer captures this world's immutable token, never the mutable global context.
            local token=ffi.new("uint8_t[16]");ffi.copy(token,context,16);local native_close=bridge.nt2_g2_close
            guard=ffi.gc(token,function(t) native_close(t) end)
            seed=tonumber(result[0]);SetWorldSeed(seed);log("seed_requested",seed)
        else context=nil end
    end)
    if not ok then close();log("bridge_error",tostring(err):sub(1,240)) end
end
function OnMagicNumbersAndWorldSeedInitialized()
    if not seed then return end
    local actual=tonumber(StatsGetValue("world_seed"))
    accepted=actual==seed and bridge.nt2_g2_seed_accepted(context,actual)==1
    log("seed_accepted",tostring(accepted)..";expected="..seed..";actual="..tostring(actual))
    -- Keep the bound world available for a wrong-seed diagnostic; never treat it as accepted.
end
local function observe()
    local report=inspection.collect(player~=nil,hold~=nil)
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
    local acknowledged=context and bridge and bridge.nt2_g2i_report(context,body,#body,report_sequence)==1
    if acknowledged then report_content=content end
    log("inspection",tostring(acknowledged)..";sequence="..report_sequence..";complete="..tostring(report.complete))
    return acknowledged and report.complete and report.actualSeed==string.format("%.0f",seed) and not changed and not release_blocked
end
function OnPlayerSpawned(p)
    -- A saved waiting world can be continued solo without inheriting our temporary controls/effect.
    for _,component in ipairs(EntityGetComponentIncludingDisabled(p,"VariableStorageComponent") or {}) do
        if ComponentGetValue2(component,"name")=="nt2_g2_recovery" then
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
            for _,child in ipairs(EntityGetAllChildren(p) or {}) do if EntityHasTag(child,"nt2_g2_owned_hold") then EntityKill(child) end end
            EntityRemoveComponent(p,component);log("saved_hold_restored","continue")
        end
    end
    if not seed or loaded then return end
    player=p
    local ok,err=pcall(protect,p)
    if not ok then close();log("protection_failed",err);return end
    if accepted then loaded=bridge.nt2_g2_world_loaded(context)==1 end
    log("world_loaded",tostring(loaded)..";seed="..seed)
    observe();next_scan=GameGetFrameNum()+300
end
function OnWorldPreUpdate()
    if not player or offline then return end
    local status=accepted and loaded and context and bridge.nt2_g2_status(context) or 0
    -- Recollect on the game thread immediately before release. Pause cannot fabricate a rescan.
    if status==2 and hold then
        if not observe() or bridge.nt2_g2_status(context)~=2 then status=1 end
    elseif GameGetFrameNum()>=next_scan then
        observe();next_scan=GameGetFrameNum()+300
        status=accepted and loaded and context and bridge.nt2_g2_status(context) or 0
        if release_blocked then status=1 end
    end
    if status==2 then
        if hold then restore();released=true;log("released",seed) end
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
