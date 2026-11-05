-- SE1-C settings-scope entry candidate; uninstalled, preserves accepted potion code.
-- Engine scope/context acceptance and Root composition review remain required.
dofile("mods/noita_together2/files/loot/register.lua")
ModLuaFileAppend("data/scripts/biomes/temple_altar.lua","mods/noita_together2/files/g3/bank_mail.lua")
ModLuaFileAppend("data/scripts/biomes/boss_arena.lua","mods/noita_together2/files/g3/bank_mail.lua")
local c1_enabled=ModDoesFileExist and ModDoesFileExist("mods/noita_together2/files/c1/register.lua")or false
if c1_enabled then dofile("mods/noita_together2/files/c1/register.lua")end
local c2_enabled=ModDoesFileExist and ModDoesFileExist("mods/noita_together2/files/c2/register.lua")or false
if c2_enabled then dofile("mods/noita_together2/files/c2/register.lua")end
local race_enabled=ModDoesFileExist and ModDoesFileExist("mods/noita_together2/files/race/register.lua")or false
if race_enabled then dofile("mods/noita_together2/files/race/register.lua")end
local social_preferences=dofile("mods/noita_together2/files/g3/social_preferences.lua")
local logging_enabled=social_preferences.get(_G,"NT_ENABLE_LOGGER")
local social_visual=dofile("mods/noita_together2/files/g3/social_visual.lua")
if logging_enabled then ModMagicNumbersFileAdd("mods/noita_together2/files/logging.xml")end
if ModIsEnabled and ModIsEnabled("mnee")then ModLuaFileAppend("mods/mnee/bindings.lua","mods/noita_together2/files/g3/mnee_bindings.lua")end
dofile_once("data/scripts/lib/utilities.lua")
local bridge,context,guard,seed,accepted,loaded,player,hold,gui
local gameplay, social_gameplay
local player_api=dofile("mods/noita_together2/files/g3/game_api.lua")
local composer_lock=dofile("mods/noita_together2/files/g3/a2_input_lock.lua")
local hold_owner={}
local recovery, recovering=nil,false
local recovery_display=dofile("mods/noita_together2/files/rejoin/local_world.lua")
local lobby_display=dofile("mods/noita_together2/files/rejoin/lobby_status.lua")
local race_deadline_display=lobby_display.race_status()
local offline=false
local entry_observer=logging_enabled and dofile("mods/noita_together2/files/startup/entry_observer.lua").new()or {record=function()end,replay=function()end,update=function()end}
local entry_scope=dofile("mods/noita_together2/files/startup/settings_scope.lua")
local entry_started,entry_activity,entry_receipt,entry_kind=false,false,nil,"ambiguous"
local fresh_failed=false
local actual_seed_matches=false
local policy,actual_seed
local loot_policy=dofile("mods/noita_together2/files/loot/policy.lua")
local testing_loadout=dofile("mods/noita_together2/files/startup/testing_loadout.lua").new(_G)
local inspection=dofile("mods/noita_together2/files/inspection.lua")
local report_sequence,report_body,report_content,next_scan=0,nil,nil,nil
local report_acknowledged=false
local report_complete,last_scan_clock=false,nil
local report_sample_content,report_sample_at=nil,nil
-- Collection runs only on actual engine callbacks. Keep ample headroom under
-- the existing6s evidence freshness, independent of rendering frame rate.
local function scan_clock()
    local ok,now=pcall(GameGetRealWorldTimeSinceStarted)
    if not ok or type(now)~="number" or now~=now or now<0 or now==math.huge or (last_scan_clock and now<last_scan_clock) then return nil end
    last_scan_clock=now;return now
end
local function schedule_scan(acknowledged)
    local now=scan_clock();next_scan=now and now+(acknowledged and 1 or 0.05) or nil
end
local function scan_due()
    local now=scan_clock();return now and (not next_scan or now>=next_scan)
end
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
        if not logging_enabled then return end
        if seen[phase] or samples>=8 or type(p)~="number" or p<=0 then return end
        seen[phase]=true;samples=samples+1
        -- Diagnostic failure must never escape into the entry's authority path.
        local ok=pcall(snapshot,phase,p)
        if not ok and type(_G.print)=="function" then pcall(_G.print,"NT2_POTION|v=1|phase="..phase.."|diagnostic_unavailable=1") end
    end
    function m:frame(waiting)
        if not logging_enabled then return end
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
local function log(event,detail)if logging_enabled then print("NT2_G2|event="..event.."|frame="..tostring(GameGetFrameNum()).."|detail="..tostring(detail))end end
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
        local c=EntityGetFirstComponentIncludingDisabled(hold.player,"ControlsComponent")
        local d=EntityGetFirstComponentIncludingDisabled(hold.player,"DamageModelComponent")
        local hp=d and ComponentGetValue2(d,"hp")
        -- Restore only the component actually captured; never revive dead controls.
        if c and c==hold.controls and hold.enabled~=nil and not (type(hp)=="number" and hp<0) then ComponentSetValue2(c,"enabled",hold.enabled) end
        if d and d==hold.damage and hold.wait~=nil then ComponentSetValue2(d,"wait_for_kill_flag_on_death",hold.wait) end
        for _,entry in ipairs(hold.inventory or {}) do EntitySetComponentIsEnabled(hold.player,entry.id,entry.enabled) end
        if hold.poly then EntityRemoveTag(hold.player,"polymorphable_NOT") end
        if hold.effect and EntityGetIsAlive(hold.effect) and (not EntityGetParent or EntityGetParent(hold.effect)==hold.player) then
            -- EntityKill may leave a child visible until the frame ends. Retire
            -- our ownership tag first so same-frame protection cannot adopt it.
            EntityRemoveTag(hold.effect,"nt2_g3_owned_hold");EntityKill(hold.effect)
        end
        if hold.recovery then
            for _,id in ipairs(EntityGetComponentIncludingDisabled(hold.player,"VariableStorageComponent")or {})do if id==hold.recovery and ComponentGetValue2(id,"name")=="nt2_g3_recovery"then EntityRemoveComponent(hold.player,id)end end
        end
    end
    for _,entry in pairs(hold.potions or {}) do restore_potion(entry) end
    hold=nil
end
local function protect(p)
    local subject=player_api.player_state(_G,p,true)
    if not subject then return false end -- No controllable entity is not a close.
    if subject.dead then
        -- C1 owns this genuine death. Keep an existing connection hold while
        -- its authorized own respawn completes; C1 never releases that hold.
        if c1_enabled and NT2_C1 and NT2_C1:owns_death(p)then return hold~=nil end
        if hold and hold.player==p then restore() end
        return false -- Do not wait-lock, heal or resurrect genuine negative HP.
    end
    if not hold then
        if social_gameplay then social_gameplay:before_protect(p) end
        local x,y=EntityGetTransform(p)
        hold={player=p,x=x,y=y,inventory={},potions={},potion_count=0,poly=not EntityHasTag(p,"polymorphable_NOT")}
        -- A form may inherit an owned hold/composer marker. Preserve original
        -- flags, without briefly enabling controls on an unauthorized new form.
        hold.inherited={}
        for _,id in ipairs(EntityGetComponentIncludingDisabled(p,"VariableStorageComponent")or {})do
            if ComponentGetValue2(id,"name")=="nt2_g3_recovery"then
                local flags=ComponentGetValue2(id,"value_string")
                if type(flags)=="string"and flags:match("^[01%-][01%-][01]...$")then
                    if flags:sub(1,1)~="-"then hold.enabled=flags:sub(1,1)=="1"end
                    if flags:sub(2,2)~="-"then hold.wait=flags:sub(2,2)=="1"end
                    hold.poly=flags:sub(3,3)=="1";hold.recovery=hold.recovery or id;hold.inherited[#hold.inherited+1]=id
                end
            end
        end
        local composed=composer_lock.inherited(_G,p)
        if #composed>0 then hold.enabled=true;for _,id in ipairs(composed)do hold.inherited[#hold.inherited+1]=id end end
    end
    -- Incomplete forms stay held until their real components become available.
    if not subject.controls and hold.controls then hold.controls=nil;hold.enabled=nil end
    if not subject.damage and hold.damage then hold.damage=nil;hold.wait=nil end
    if subject.controls then
        if hold.controls~=subject.controls then
            if hold.controls or hold.enabled==nil then hold.enabled=ComponentGetValue2(subject.controls,"enabled")end
            hold.controls=subject.controls
        end
        ComponentSetValue2(hold.controls,"enabled",false)
    end
    if subject.damage then
        if hold.damage~=subject.damage then
            if hold.damage or hold.wait==nil then hold.wait=ComponentGetValue2(subject.damage,"wait_for_kill_flag_on_death")end
            hold.damage=subject.damage
        end
        ComponentSetValue2(hold.damage,"wait_for_kill_flag_on_death",true)
    end
    local flags=(hold.enabled==nil and "-"or hold.enabled and "1"or "0")..(hold.wait==nil and "-"or hold.wait and "1"or "0")..(hold.poly and "1"or "0").."---"
    if not hold.recovery then
        hold.recovery=EntityAddComponent2(p,"VariableStorageComponent",{name="nt2_g3_recovery",value_string=flags})
        if not hold.recovery or hold.recovery==0 then error("Protection recovery marker unavailable")end
    else ComponentSetValue2(hold.recovery,"value_string",flags)end
    if hold.inherited then for _,id in ipairs(hold.inherited)do if id~=hold.recovery then EntityRemoveComponent(p,id)end end;hold.inherited=nil end
    protect_potions()
    if hold.poly then EntityAddTag(p,"polymorphable_NOT")end
    local function effect_component(id)
        if not id or id==0 or not EntityGetIsAlive(id) or not EntityHasTag(id,"nt2_g3_owned_hold")
            or EntityGetParent and EntityGetParent(id)~=p then return nil end
        local c=EntityGetFirstComponentIncludingDisabled(id,"GameEffectComponent")
        if type(c)=="number" and c>0 then return c end
    end
    local effect=effect_component(hold.effect)
    if not effect then
        hold.effect=nil
        for _,id in ipairs(EntityGetAllChildren(p)or {})do if effect_component(id)then hold.effect=id;break end end
        hold.effect=hold.effect or LoadGameEffectEntityTo(p,"data/entities/misc/effect_protection_all.xml")
        if not hold.effect or hold.effect==0 then error("Protection effect unavailable")end
        EntityAddTag(hold.effect,"nt2_g3_owned_hold")
    end
    effect=effect_component(hold.effect)
    if not effect then error("Protection effect component unavailable")end
    ComponentSetValue2(effect,"frames",-1)
    hold.complete=subject.ready;return hold.complete
end
local function close()
    if social_gameplay then social_gameplay:close();social_gameplay=nil end
    if gameplay then gameplay:close();gameplay=nil end
    if context and bridge then bridge.nt2_g3_close(context) end
    if guard then require("ffi").gc(guard,nil);guard=nil end
end
-- Continue does not require the fresh-world mod-init callback. Binding images
-- is separate from preparation, ModTextFile writes and SetWorldSeed.
local bridge_retry_at=nil
local function ensure_bridge()
    if bridge then return true end
    if bridge_retry_at and GameGetFrameNum()<bridge_retry_at then return false end
    bridge_retry_at=GameGetFrameNum()+30
    local ok,err=pcall(function()
        local ffi=require("ffi")
        ffi.cdef[[int nt2_g3_startup_nonce(uint8_t*,uint32_t);]]
        ffi.cdef[[int nt2_g3_resume(const uint8_t*,const uint8_t*,uint32_t,uint8_t*); int nt2_g3_association(const uint8_t*,uint8_t*,uint32_t); int nt2_g3_saved_lookup(const uint8_t*,uint32_t); int nt2_g3_offline_choice(); int nt2_g3_resume_decision(const uint8_t*); int nt2_g3_seed_known(const uint8_t*,const uint8_t*,uint32_t); uint32_t nt2_g3_world_policy(const uint8_t*); int nt2_g3_startup_prepare(uint32_t*,uint8_t*); int nt2_g3_startup_bundle(uint8_t*,uint32_t,uint8_t*,uint32_t); int nt2_g3_startup_diagnostic(uint8_t*,uint32_t); int nt2_g3_social_terminal(const uint8_t*,uint8_t*,uint32_t); int nt2_g3_social_lease(const uint8_t*,uint8_t*,uint32_t); int nt2_g3_social_next(const uint8_t*,uint8_t*,uint32_t); int nt2_g3_move(const uint8_t*,uint32_t,int32_t,int32_t,int32_t,int32_t,uint32_t); int nt2_g3_peers(const uint8_t*,uint8_t*,uint32_t); int nt2_g3_prepare(uint32_t*,uint8_t*); int nt2_g3_seed_accepted(const uint8_t*,uint32_t); int nt2_g3_world_loaded(const uint8_t*); int nt2_g3_status(const uint8_t*); int nt2_g3_display_status(const uint8_t*); int nt2_g3_heartbeat_diagnostic(const uint8_t*); int nt2_g3_close(const uint8_t*); int nt2_g3_report(const uint8_t*,const uint8_t*,uint32_t,uint64_t);]]
        local native=ffi.load("mods/noita_together2/files/nt2_g3_transport.dll")
        local social=ffi.load("mods/noita_together2/files/nt2_g3_social.dll")
        bridge={nt2_g3_prepare=native.nt2_g3_startup_prepare,nt2_g3_world_policy=native.nt2_g3_world_policy,nt2_g3_seed_accepted=native.nt2_g3_seed_accepted,nt2_g3_seed_known=native.nt2_g3_seed_known,nt2_g3_world_loaded=native.nt2_g3_world_loaded,nt2_g3_status=native.nt2_g3_status,nt2_g3_heartbeat_diagnostic=native.nt2_g3_heartbeat_diagnostic,nt2_g3_close=native.nt2_g3_close,nt2_g3_report=native.nt2_g3_report,nt2_g3_move=native.nt2_g3_move,nt2_g3_peers=native.nt2_g3_peers,nt2_g3_social_lease=social.nt2_g3_social_lease,nt2_g3_social_next=social.nt2_g3_social_next,nt2_g3_social_terminal=social.nt2_g3_social_terminal}
        -- Function pointers alone do not retain ffi.load namespaces. Keep both
        -- wrapper images alive for every callback using this bound bridge.
        bridge._libraries={native,social}
        -- Start both fixed image workers together; status never prepares a
        -- world and returns cached/unavailable without waiting for the loader.
        local empty=ffi.new("uint8_t[16]");native.nt2_g3_status(empty);social.nt2_g3_status(empty)
        bridge.nt2_g3_association=native.nt2_g3_association
        bridge.nt2_g3_saved_lookup=native.nt2_g3_saved_lookup
        bridge.nt2_g3_offline_choice=native.nt2_g3_offline_choice
        ffi.cdef[[int nt2_g3_bank_publish(const uint8_t*,const uint8_t*,uint32_t); int nt2_g3_bank_view(const uint8_t*,uint8_t*,uint32_t); int nt2_g3_debug_publish(const uint8_t*,const uint8_t*,uint32_t); int nt2_g3_debug_mode(const uint8_t*); int nt2_g3_social_publish(const uint8_t*,const uint8_t*,uint32_t); int nt2_g3_startup_nonce(uint8_t*,uint32_t);]]
        bridge.nt2_g3_bank_publish=native.nt2_g3_bank_publish
        bridge.nt2_g3_bank_view=native.nt2_g3_bank_view
        bridge.nt2_g3_social_publish=native.nt2_g3_social_publish
        bridge.nt2_g3_debug_publish=native.nt2_g3_debug_publish;bridge.nt2_g3_debug_mode=native.nt2_g3_debug_mode
        bridge.nt2_g3_startup_nonce=native.nt2_g3_startup_nonce
        bridge.nt2_g3_resume=native.nt2_g3_resume
        bridge.nt2_g3_resume_decision=native.nt2_g3_resume_decision
        ffi.cdef[[int nt2_g3_lobby_status(uint8_t*,uint32_t);]]
        bridge.nt2_g3_lobby_status=native.nt2_g3_lobby_status
        bridge.nt2_g3_display_status=native.nt2_g3_display_status
    end)
    if not ok then bridge=nil;log("bridge_error",tostring(err):sub(1,240)) end
    return bridge~=nil
end
-- No engine-mode inference from a missing seed, saved-file guess or frame count.
-- Exact restored metadata may select recovery; otherwise only an explicit user
-- choice leaves this hold. Actual cold/Continue ordering remains to be measured.
-- Actual traces contain world updates before WorldInitialized. Until a complete
-- fresh preparation is confirmed, even those early callbacks must read only the
-- loaded world's saved loot and cannot publish a stale transient bootstrap.
local saved_world,continue_pending=true,false
local startup_reason="Startup mode is not confirmed; checking saved-world metadata."
local function local_loot()
    return saved_world and loot_policy.read_saved() or loot_policy.initialize()
end
local function reset_entry()
    close();hold_owner={};context=nil;seed=nil;policy=nil;accepted=false;loaded=false
    player=nil;hold=nil;recovery=nil;recovering=false;offline=false;actual_seed=nil;actual_seed_matches=false
    released=false;release_blocked=false;report_sequence=0;report_body=nil;report_content=nil;next_scan=nil;report_acknowledged=false;report_complete=false;last_scan_clock=nil;report_sample_content=nil;report_sample_at=nil
    continue_pending=false;saved_world=true;fresh_failed=false;bridge_retry_at=nil
end
function OnWorldInitialized()
    -- V2: first world updates precede this callback. It is NOT an entry reset.
    entry_observer:record("world_initialized");entry_observer:replay()
end
-- Accepted complete transaction selected by the consumed settings-scope candidate.
-- No file/Globals/frame heuristic or persisted setting activates it.
local function apply_validated_fresh_entry()
    saved_world=true
    fresh_failed=true -- cleared only by this one complete early application
    startup_reason="Local startup preparation is pending."
    local ok,err=pcall(function()
        loot_policy.saved() -- Reset the transient bootstrap for this init, including Continue.
        assert(ensure_bridge(),"Native images unavailable")
        local ffi=require("ffi")
        local native,social=bridge._libraries[1],bridge._libraries[2]
        context=ffi.new("uint8_t[16]")
        assert(native.nt2_g3_startup_nonce(context,16)==1,"Entry correlation unavailable")
        entry_observer:record("claim")
        local consumer=dofile("mods/noita_together2/files/startup/bundle.lua")
        local complete,reason,status=consumer.request(native,ffi,context,6000);log("prepare",status)
        if complete then
            -- The finalizer captures this world's immutable token, never the mutable global context.
            local token=ffi.new("uint8_t[16]");ffi.copy(token,context,16);local native_close=bridge.nt2_g3_close
            guard=ffi.gc(token,function(t) native.nt2_g3_close(t) end)
            local remaining=6000-complete.elapsed_ms;assert(remaining>=100,"Complete startup cold budget exhausted before secondary acknowledgment")
            local shared_context=ffi.new("uint8_t[16]");local shared=assert(consumer.request(social,ffi,shared_context,remaining))
            assert(shared.kind==complete.kind and shared.seed==complete.seed and shared.policy==complete.policy and shared.world==complete.world and shared.digest==complete.digest and shared.snapshot==complete.snapshot and ffi.string(shared_context,16)==ffi.string(context,16),"Shared complete primary/social world refused")
            policy=complete.policy;seed=complete.seed
            assert(policy~=2 or seed==0,"Individual policy must not contain a forced shared seed")
            if policy~=2 then SetWorldSeed(seed);log("seed_requested",seed) end
            loot_policy.stage(policy==2 and "individual:0" or "shared:"..(policy==5 and "1" or "0")..":"..string.format("%.0f",seed),true)
            testing_loadout:stage(complete)
            saved_world=false
            startup_reason=nil
            fresh_failed=false
            entry_observer:record("fresh_applied")
        else
            -- A refusal cannot prove Continue or an ordinary offline world.
            -- Keep saved loot untouched until mode/metadata is established.
            saved_world=true
            startup_reason="Startup was not confirmed. For shared play, wait for the app then choose New Game."
            context=nil
            entry_observer:record("fresh_failed")
        end
    end)
    if not ok then close();context=nil;seed=nil;policy=nil;saved_world=true;startup_reason="Startup failed. For shared play, wait for the app then choose New Game.";entry_observer:record("fresh_failed");log("bridge_error",tostring(err):sub(1,240)) end
end
function OnModPreInit()
    local snapshot=entry_scope.snapshot()
    if entry_started and snapshot==entry_receipt and not entry_activity then
        entry_observer:record("duplicate_preinit");return
    end
    reset_entry();local bootstrap_ready=pcall(function()loot_policy.saved();testing_loadout:saved()end) -- init-only inert bootstrap reset, no saved loot write
    entry_observer=dofile("mods/noita_together2/files/startup/entry_observer.lua").new()
    entry_kind,entry_receipt=entry_scope.consume();entry_started=true;entry_activity=false
    for _,event in ipairs(entry_scope.observations()) do entry_observer:record(event) end
    entry_observer:record("preinit");entry_observer:record("scope_"..entry_kind)
    local reasons={restored="Checking this saved world's protected identity.",unavailable="Engine entry handoff API unavailable; world protected.",missing="No current settings entry scope was delivered; world protected.",spent="Entry scope already consumed; world protected.",ambiguous="Entry scope duplicated or ambiguous; world protected."}
    startup_reason=reasons[entry_kind] or "Checking the complete current-entry startup bundle."
    if not bootstrap_ready then fresh_failed=true;startup_reason="Local loot bootstrap reset failed; world protected.";entry_observer:record("fresh_failed");return end
    if entry_kind=="fresh" then apply_validated_fresh_entry() end
end
function OnModInit()
    dofile("mods/noita_together2/files/g3/a2_biomes.lua").register(_G)
    entry_observer:record("init")
end
function OnModPostInit() entry_observer:record("postinit") end
function OnMagicNumbersAndWorldSeedInitialized()
    entry_activity=true
    entry_observer:record("seed_boundary")
    local_loot() -- Publishes Globals only if WorldState is already available.
    if not seed then return end
    local actual=tostring(StatsGetValue("world_seed"))
    actual_seed=actual
    actual_seed_matches=actual:match("^%d+$")~=nil and #actual<=20 and (#actual==1 or actual:sub(1,1)~="0") and (#actual<20 or actual<="18446744073709551615") and (policy==2 or actual==string.format("%.0f",seed))
    accepted=actual_seed_matches and bridge.nt2_g3_seed_known(context,actual,#actual)==1
    if not actual_seed_matches then startup_reason="The shared seed was not applied. Shared play requires a fresh New Game." end
    log("seed_accepted",tostring(accepted)..";expected="..seed..";actual="..tostring(actual))
    -- Keep the bound world available for a wrong-seed diagnostic; never treat it as accepted.
end
local function observe(for_release)
    local collected_at=scan_clock()
    if for_release and (not collected_at or not player_api.ready(_G,player) or hold and not hold.complete)then return false end
    if report_body and not report_acknowledged then
        -- Retry the exact original until acknowledged, without another inventory
        -- scan or a restamped sequence. A release probe continues with a fresh
        -- actual scan in this callback once that same original is acknowledged.
        local acknowledged=context and bridge and bridge.nt2_g3_report(context,report_body,#report_body,report_sequence)==1
        report_acknowledged=acknowledged or false;schedule_scan(acknowledged)
        if acknowledged then report_content=report_sample_content end
        log("inspection",tostring(acknowledged)..";sequence="..report_sequence..";complete="..tostring(report_complete))
        if not acknowledged or not for_release then return false end
    end
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
    -- A fresh immediate pre-release scan may confirm the exact acknowledged
    -- bytes without creating another asynchronous report. Keep the original
    -- collection time; reuse is bounded well inside existing6s freshness.
    local release_match=for_release and report_acknowledged and body==report_body
        and report_sample_at and collected_at-report_sample_at<=2
    if not release_match then
        if report_sequence>=4294967295 then close();log("report_failed","sequence_limit");return false end
        report_sequence=report_sequence+1;report_body=body;report_acknowledged=false
        report_sample_content=content;report_sample_at=collected_at
    end
    report_complete=report.complete
    local acknowledged=release_match or context and bridge and bridge.nt2_g3_report(context,body,#body,report_sequence)==1
    if acknowledged then report_content=content;report_acknowledged=true end
    -- Current Rust/native release is bound to this acknowledged report. A
    -- canonical host review may accept changed advisory content while held.
    if recovery and recovery.record and acknowledged and report.complete and not changed
        and context and bridge.nt2_g3_status(context)==2 then release_blocked=false end
    log("inspection",tostring(acknowledged)..";sequence="..report_sequence..";complete="..tostring(report.complete))
    if release_match then log("release_scan","matched_acknowledged;sequence="..report_sequence)
    else schedule_scan(acknowledged) end
    return acknowledged and report.complete and actual_seed_matches and report.actualSeed==actual_seed and not changed and not release_blocked
end
-- All recovery callbacks poll the one native mailbox. Rust alone authenticates
local function current_display_status()
 if not bridge or not context then return 0 end
 local ok,value=pcall(function() return tonumber(bridge.nt2_g3_display_status(context)) end)
 return ok and value or 0
end
local function current_lobby_status()
 return lobby_display.read(bridge,bridge and require('ffi') or nil)
end
local function current_race_deadline(saved,phase,reason)
 local ok,record,actual=pcall(function()return saved and saved.record or recovery_display.decode_saved(GlobalsGetValue('nt2_rejoin_local_world_v1','')),tostring(StatsGetValue('world_seed'))end)
 if not ok then record=nil;actual=nil end
 local lobby=current_lobby_status()
 if lobby and lobby.code==5 and phase=='terminal'then lobby=nil end
 race_deadline_display:update(social_gameplay and social_gameplay.bank and social_gameplay.bank.race_view,record,actual,lobby)
 return race_deadline_display:message(record,actual,phase,reason,lobby)
end
local function current_compatibility_status()
 local terminal=current_race_deadline(recovery and recovery.record,recovery and recovery.state,recovery and recovery.reason)
 return lobby_display.compatibility(current_lobby_status(),terminal)
end
-- metadata and creates new service ownership. This path never sets world seed.
local function player_rules_ready()
    if GlobalsGetValue('nt2_race_active','0')=='1'then return NT2_RACE and NT2_RACE.ready==true end
    if not c1_enabled then return true end
    if not context or not NT2_C1 or not NT2_C1.ready then return false end
    local bytes={};for i=0,15 do bytes[#bytes+1]=string.format('%02x',tonumber(context[i]))end
    return NT2_C1.context==table.concat(bytes)
end
local function recovery_owner()
    if recovery or not bridge then return recovery end
    local ffi=require("ffi")
    recovery=recovery_display.new({
        display_status=current_display_status,
        lobby_status=current_lobby_status,
        race_status=current_race_deadline,
        compatibility_status=current_compatibility_status,
        known=function(saved)
            local state=tonumber(bridge.nt2_g3_saved_lookup(saved.record,#saved.record))
            -- Existing zero conflates malformed input, missing local proof and
            -- foreign scope. It is not positive ordinary-save identification.
            -- Do not freeze a new native classifier/ABI from that rejection.
            if state~=1 and state~=2 and state~=3 and state~=5 and state~=6 then return nil end
            return true,state==2,state==3 or state==6,state==5,state==6 and "player_death_end" or nil
        end,
        finish=function(reason)
            if reason=="player_death_end" then
                dofile("mods/noita_together2/files/g3/c1_local.lua").finish_run(_G,player,nil,reason)
            end
        end,
        hold_owned=function() protect(player);return hold_owner end,
        release_owned=function(owned,local_offline)
            if owned~=hold_owner then return end
            -- Explicit local opt-out/foreign lookup restores only our hold;
            -- it is independent of online admission and never grants it.
            if local_offline then restore();if c1_enabled and player then dofile('mods/noita_together2/files/g3/c1_local.lua').recover(_G,player)end;offline=true;return end
            if hold and hold.player==player and hold.complete and player_api.ready(_G,player) and player_rules_ready()
                and context and bridge and loaded and accepted and not release_blocked and bridge.nt2_g3_status(context)==2 then restore();released=true end
        end,
        retire=function() bridge.nt2_g3_offline_choice();close();offline=true end,
        request=function(saved,generation)
            close();context=ffi.new("uint8_t[16]")
            local uuid=saved.record:match('"local_world"%s*:%s*"([%x%-]+)"')
            local digits=uuid and uuid:gsub("%-","") or ""
            assert(#digits==32,"Invalid saved association identity")
            local association=ffi.new("uint8_t[16]")
            for i=0,15 do association[i]=tonumber(digits:sub(i*2+1,i*2+2),16) end
            actual_seed=saved.seed;actual_seed_matches=tostring(StatsGetValue("world_seed"))==saved.seed
            accepted=false;loaded=false;released=false;release_blocked=false;recovering=true
            report_sequence=0;report_body=nil;report_content=nil;next_scan=nil;report_acknowledged=false;report_complete=false;last_scan_clock=nil;report_sample_content=nil;report_sample_at=nil
            return {association=association,saved=saved,generation=generation,context=context}
        end,
        poll=function(pending)
            if not pending or pending.context~=context then return nil end
            -- Terminal/offline intent is a separate scoped mailbox fact. Keep
            -- polling it even while a gameplay/collection lease is exhausted.
            local saved_state=tonumber(bridge.nt2_g3_saved_lookup(pending.saved.record,#pending.saved.record))
            if saved_state==3 or saved_state==6 then return "terminal",pending.generation,saved_state==6 and "player_death_end" or nil end
            if saved_state==5 then return "offline",pending.generation end
            local decision=tonumber(bridge.nt2_g3_resume_decision(context))
            if decision==4 then return "terminal",pending.generation end
            if decision==2 then return "offline",pending.generation end
            -- A zero cached lease means protect and wait on this conversation.
            -- It does not authorize closing/replacing a still loaded world.
            if guard and bridge.nt2_g3_status(context)==0 then return "lost",pending.generation end
            local status=bridge.nt2_g3_resume(pending.association,pending.saved.seed,#pending.saved.seed,context)
            if status~=1 then return nil,pending.generation end
            if not guard then
                local social=bridge._libraries[2]
                local other=ffi.new("uint8_t[16]")
                -- The secondary wrapper may still be loading asynchronously.
                -- Retry its cached result on another frame while keeping this
                -- same protected owner; never turn image latency into a grant.
                if social.nt2_g3_resume(pending.association,pending.saved.seed,#pending.saved.seed,other)~=1 then return nil,pending.generation end
                assert(ffi.string(other,16)==ffi.string(context,16),"Shared recovery owner differs")
                local token=ffi.new("uint8_t[16]");ffi.copy(token,context,16)
                local native=bridge._libraries[1]
                guard=ffi.gc(token,function(t) native.nt2_g3_close(t) end)
                policy=tonumber(bridge.nt2_g3_world_policy(context))
                assert(policy==1 or policy==2 or policy==5,"Recovery policy refused")
                seed=policy==2 and 0 or tonumber(actual_seed)
                local expected=policy==2 and "individual:0" or "shared:"..(policy==5 and "1" or "0")..":"..actual_seed
                assert(expected==pending.saved.loot and local_loot()==expected,"Saved loot policy differs")
            end
            accepted=actual_seed_matches and bridge.nt2_g3_seed_known(context,actual_seed,#actual_seed)==1
            loaded=accepted and bridge.nt2_g3_world_loaded(context)==1
            if loaded then
                local status=bridge.nt2_g3_status(context)
                if (status==2 or scan_due()) and observe(status==2) and bridge.nt2_g3_status(context)==2 then
                    return "released",pending.generation
                end
            end
            return "admitted",pending.generation
        end,
        inspected=function(saved) return loaded and accepted and actual_seed==saved.seed and hold and hold.complete and player_api.ready(_G,player) and player_rules_ready() and not release_blocked and bridge.nt2_g3_status(context)==2 end,
    })
    return recovery
end
-- Engine entity identity can change without changing the native world/owner.
local function refresh_player(hint)
    if offline then return end
    if not player and not loaded and not hold then return end
    local current=player_api.resolve_player(_G,hint or player)
    if current~=player then
        if current then
            local was_held=hold~=nil
            if hold then restore()end
            player=current;recover_potions(player)
            -- A new body does not revoke an already current world permission.
            -- Transfer existing protection, or acquire it when permission/body
            -- readiness is unavailable; never invent a release on a form change.
            local permitted=released and loaded and accepted and not release_blocked and context and bridge
                and bridge.nt2_g3_status(context)==2 and(player_api.ready(_G,player)or c1_enabled and NT2_C1 and NT2_C1:owns_death(player))
            if was_held or not permitted then protect(player)end
        else player=nil end
    end
end
function OnPlayerSpawned(p)
    if social_gameplay then social_gameplay.a3:before_hold(p);social_gameplay.a3.visual:clear_colour(p)end
    social_visual.recover(_G,p)
    if offline then return end
    if loaded or player or hold then refresh_player(p);return end
    entry_activity=true
    entry_observer:record("spawn")
    potion_trace:sample("spawn_before",p) -- NT2_POTION_DIAGNOSTIC_HOOK
    -- A saved waiting world can be continued solo without inheriting our temporary controls/effect.
    for _,component in ipairs(EntityGetComponentIncludingDisabled(p,"VariableStorageComponent") or {}) do
        if ComponentGetValue2(component,"name")=="nt2_g3_recovery" then
            local flags=ComponentGetValue2(component,"value_string")
            local controls=EntityGetFirstComponentIncludingDisabled(p,"ControlsComponent")
            local damage=EntityGetFirstComponentIncludingDisabled(p,"DamageModelComponent")
            if controls and flags:sub(1,1)~="-"then ComponentSetValue2(controls,"enabled",flags:sub(1,1)=="1")end
            if damage and flags:sub(2,2)~="-"then ComponentSetValue2(damage,"wait_for_kill_flag_on_death",flags:sub(2,2)=="1")end
            for index,kind in ipairs(inventory_types) do
                local value=flags:sub(index+3,index+3);local component=EntityGetFirstComponentIncludingDisabled(p,kind)
                if component and (value=="0" or value=="1") then EntitySetComponentIsEnabled(p,component,value=="1") end
            end
            if flags:sub(3,3)=="1" then EntityRemoveTag(p,"polymorphable_NOT") end
            for _,child in ipairs(EntityGetAllChildren(p) or {}) do if EntityHasTag(child,"nt2_g3_owned_hold") then EntityKill(child) end end
            EntityRemoveComponent(p,component);log("saved_hold_restored","continue")
        end
    end
    if c1_enabled then dofile('mods/noita_together2/files/g3/c1_local.lua').recover(_G,p)end
    recover_potions(p)
    if not seed or loaded then potion_trace:sample("unbound_after_recovery",p) end -- NT2_POTION_DIAGNOSTIC_HOOK
    if not seed then
        player=p
        saved_world=true;continue_pending=not fresh_failed
        -- Protect the callback/readiness gap. Absent/foreign metadata restores
        -- only this temporary hold; it never retires or rewrites a saved world.
        local ok,err=pcall(protect,p)
        if not ok then log("protection_failed",err) end
        ensure_bridge()
        entry_observer:record("hold");entry_observer:replay()
        return
    end
    if loaded then return end
    player=p
    local ok,err=pcall(protect,p)
    if not ok then close();log("protection_failed",err);return end
    testing_loadout:spawn(p)
    potion_trace:sample("spawn_after_protect",p) -- NT2_POTION_DIAGNOSTIC_HOOK
    entry_observer:record("hold");entry_observer:replay()
    if accepted then loaded=bridge.nt2_g3_world_loaded(context)==1 end
    log("world_loaded",tostring(loaded)..";seed="..seed)
    observe()
end
function OnWorldPreUpdate()
    refresh_player()
    social_preferences.progress(_G)
    entry_activity=true
    if entry_receipt and entry_scope.snapshot()~=entry_receipt then
        -- Registry changes before the next preinit cannot inherit a prior seed,
        -- bootstrap or release. Never consume/apply a fresh scope on a frame.
        local p,h=player,hold;if p then pcall(protect,p);h=hold end
        reset_entry();player=p;hold=h;continue_pending=p~=nil
        entry_kind="ambiguous";entry_receipt=entry_scope.snapshot()
        startup_reason="Entry scope changed outside early initialization; world protected."
        entry_observer:record("scope_changed")
    end
    entry_observer:update(bridge and bridge._libraries[1],bridge and require("ffi") or nil)
    local_loot() -- Also persist before saving worlds with no loot hook invocation.
    if not saved_world and player and not offline then testing_loadout:spawn(player)end
    if offline and c2_enabled and player then dofile('mods/noita_together2/files/c2/world.lua').offline(_G,player)end
    if not player or offline then return end
    -- Recovery can return early below. Revalidate a held body's actual
    -- components first, including components that appeared/replaced in place.
    if hold then
        local ok,err=pcall(protect,player)
        if not ok then close();log("protection_failed",err);return end
    end
    if continue_pending then
        if hold then
            if hold.controls then ComponentSetValue2(hold.controls,"enabled",false)end;EntitySetTransform(player,hold.x,hold.y)
            local velocity=EntityGetFirstComponentIncludingDisabled(player,"VelocityComponent")
            if velocity then ComponentSetValue2(velocity,"mVelocity",0,0) end
        end
        if not ensure_bridge() then return end
        local owner=recovery_owner()
        local ok,found,reason=pcall(owner.loaded,owner)
        if not ok then log("saved_lookup_error","engine metadata not ready");return end
        if found then continue_pending=false
        elseif found==false and (reason=="offline" or reason=="unrelated" and entry_kind=="restored") then
            -- Explicit restored settings scope plus readable WorldState/seed
            -- distinguishes an unrelated save with no NT2 marker. Ambiguous
            -- startup or malformed/partial existing markers remain protected.
            close();restore();if c1_enabled and player then dofile('mods/noita_together2/files/g3/c1_local.lua').recover(_G,player)end;offline=true;continue_pending=false;return
        else return end
    end
    local owner=recovery_owner()
    if owner then
        owner:tick()
        if owner.state=="playing" and released and context and bridge.nt2_g3_status(context)~=2 then owner:lost() end
        if loaded and accepted and (not owner.record or owner.state=="playing") and context then
            local ffi=require("ffi");local record=ffi.new("uint8_t[1024]")
            local size=bridge.nt2_g3_association(context,record,1024)
            if size>0 and size<=1024 then
                local metadata=ffi.string(record,size)
                if metadata:match('"kind"%s*:%s*"nt2%-world"') then owner:waiting(metadata,actual_seed,local_loot())
                elseif released then owner:associate(metadata,actual_seed,local_loot(),true) end
            end
        end
        if owner.state~="playing" and owner.record then
            if hold then
                if hold.controls then ComponentSetValue2(hold.controls,"enabled",false)end;EntitySetTransform(player,hold.x,hold.y)
                local velocity=EntityGetFirstComponentIncludingDisabled(player,"VelocityComponent")
                if velocity then ComponentSetValue2(velocity,"mVelocity",0,0) end
            end
            return
        end
    end
    if not context or not bridge or not seed then return end
    -- Poll the same immutable control originals; frame callbacks never wait.
    if actual_seed_matches and not accepted then accepted=bridge.nt2_g3_seed_known(context,actual_seed,#actual_seed)==1 end
    if accepted and not loaded then loaded=bridge.nt2_g3_world_loaded(context)==1 end
    local status=accepted and loaded and context and bridge.nt2_g3_status(context) or 0
    -- Recollect on the game thread immediately before release. Pause cannot fabricate a rescan.
    if status==2 and hold then
        if not observe(true) or bridge.nt2_g3_status(context)~=2 then status=1 end
    elseif scan_due() then
        observe()
        status=accepted and loaded and context and bridge.nt2_g3_status(context) or 0
        if release_blocked then status=1 end
    end
    local race_wait=GlobalsGetValue('nt2_race_active','0')=='1'and(not NT2_RACE or not NT2_RACE.view or NT2_RACE.view.status~='racing')
    if status==2 and(not player_rules_ready()or race_wait)then
        status=1;startup_reason="Initializing shared player rules."
    end
    if status==2 then
        if hold then restore();released=true;log("released",seed);entry_observer:record("release");entry_observer:replay();observe() end
        composer_lock.recover(_G,player,true,social_gameplay and social_gameplay.renderer.lock.lease)
        potion_trace:sample("start_restored",player) -- NT2_POTION_DIAGNOSTIC_HOOK
    else
        local ok,err=pcall(protect,player)
        if not ok then close();log("protection_failed",err) end
        if released and hold then potion_trace:sample("reprotected",player) end -- NT2_POTION_DIAGNOSTIC_HOOK
        if hold then
            if hold.controls then ComponentSetValue2(hold.controls,"enabled",false)end
            EntitySetTransform(player,hold.x,hold.y)
            local velocity=EntityGetFirstComponentIncludingDisabled(player,"VelocityComponent")
            if velocity then ComponentSetValue2(velocity,"mVelocity",0,0) end
        end
    end
end
local protected_update=OnWorldPreUpdate
function OnWorldPreUpdate()
    protected_update()
    if not context or not bridge or offline then if gameplay then gameplay:close();gameplay=nil end;return end
    if not gameplay then
        gameplay=dofile("mods/noita_together2/files/g3/native_owner.lua").new(bridge,context,require("ffi"),
            dofile("mods/noita_together2/files/g3/game_api.lua"),dofile("mods/noita_together2/files/g3/validation.lua"),
            dofile("mods/noita_together2/files/g3/movement.lua"),dofile("mods/noita_together2/files/g3/ghosts.lua"))
    end
    local ok,err=pcall(gameplay.tick,gameplay,player,GameGetFrameNum(),player_api.ready(_G,player) and not hold and accepted and loaded and bridge.nt2_g3_status(context)==2)
    if not ok then
        local owned=gameplay;gameplay=nil;pcall(owned.close,owned)
        close();release_blocked=true;log("g3_gameplay_failed",err)
    end
end
function OnWorldPostUpdate()
    if social_gameplay then social_gameplay:post_update() end
    potion_trace:frame(hold~=nil) -- NT2_POTION_DIAGNOSTIC_HOOK
    if not hold or offline then return end
    if not gui then gui=GuiCreate() end
    GuiStartFrame(gui)
    local race_result=current_race_deadline(recovery and recovery.record,recovery and recovery.state,recovery and recovery.reason)
    local lobby,contact=current_lobby_status()
    GuiText(gui,10,83,race_result and race_result.footer or lobby_display.message(lobby,contact))
    if recovery and recovery.record and recovery.state~="playing" then recovery:draw(gui);return end
    local compatibility=current_compatibility_status()
    GuiText(gui,10,35,"NT2: "..(compatibility and compatibility.title or startup_reason or recovery_display.message(current_display_status())))
    GuiText(gui,10,47,compatibility and compatibility.detail or"Use the normal menu to leave.")
    if GuiButton(gui,9021,10,62,"Leave NT2 and continue this world offline") then
        -- Explicit user opt-out only; server loses this selected game's lease.
        if recovery and recovery.record then recovery:offline() else GlobalsSetValue('nt2_rejoin_local_world_v1_offline','1');if bridge then bridge.nt2_g3_offline_choice() end;close();restore();if c1_enabled and player then dofile('mods/noita_together2/files/g3/c1_local.lua').recover(_G,player)end;offline=true end
        log("offline_opt_out",seed)
        potion_trace:sample("offline_restored",player) -- NT2_POTION_DIAGNOSTIC_HOOK
    end
end


-- UNAPPROVED separate source entry. Never replace installed/reviewed init-g3.lua.
local seed_ghost_update=OnWorldPreUpdate
function OnWorldPreUpdate()
    seed_ghost_update()
    if not context or not bridge or offline then
        if social_gameplay then social_gameplay:close();social_gameplay=nil end
        return
    end
    if not social_gameplay then
        local social_engine=setmetatable({NT2_C2_ENABLED=c2_enabled,NT2_C1_ENABLED=c1_enabled,nt2_c1_permitted=function()return not offline and not hold and accepted and loaded and context and bridge.nt2_g3_status(context)==2 end,nt2_debug=function()return gameplay and gameplay.debug end,GamePrint=GamePrint,a2_positions=function()return gameplay and gameplay:positions()or {}end,a3_ghosts=function()return gameplay and gameplay:entities()or {}end,a2_ghost_visibility=function(hidden,opacity)if gameplay then gameplay:set_visibility(hidden,opacity)end end,a2_ghost_names=function(names)if gameplay then gameplay:set_names(names)end end},{__index=_G})
        social_gameplay=dofile("mods/noita_together2/files/g3/social_native.lua").new(bridge,context,require("ffi"),social_engine,
            dofile("mods/noita_together2/files/g3/validation.lua"),dofile("mods/noita_together2/files/g3/social_game_api.lua"),
            dofile("mods/noita_together2/files/g3/hud_poi_local.lua"),dofile("mods/noita_together2/files/g3/social_dispatch.lua"))
    end
    local eligible=not hold and accepted and loaded and bridge.nt2_g3_status(context)==2
    local ok,err=pcall(social_gameplay.tick,social_gameplay,eligible and player_api.ready(_G,player),player,eligible and player~=nil)
    if not ok then if gameplay and gameplay.debug then gameplay.debug:error(7)end;local failed=social_gameplay;social_gameplay=nil;pcall(failed.close,failed);log("g3_social_failed",err) end
end
