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
