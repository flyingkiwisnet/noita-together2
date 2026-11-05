local d1_ok,D1=pcall(dofile,'mods/noita_together2/files/g3/d1_hook_api.lua')
local nt2_ok,NT2_C1=pcall(dofile,'mods/noita_together2/files/g3/c1_hook_api.lua')
if not nt2_ok then NT2_C1=nil end
-- Adapted from legacy orb pickup; forced curse/fake-heart semantics retained.
local _item_pickup = item_pickup

local _EntityLoad = EntityLoad

function item_pickup(entity_item, entity_who_picked, name)
    if NT2_C1 and NT2_C1.duplicate and NT2_C1:duplicate(entity_item)then return end
    if not EntityGetIsAlive(entity_item)then return end
    local original,why
    if NT2_C1 and NT2_C1:can_pickup(entity_who_picked) and GameHasFlagRun("NT_sync_orbs") and not EntityHasTag(entity_item,"fake_orb") then
        local c=EntityGetFirstComponentIncludingDisabled(entity_item,"OrbComponent")
        if c then original,why=NT2_C1:capture("orb",{orbId=ComponentGetValue2(c,"orb_id")},entity_item)end
        if why=='duplicate'then return end
    end
    local orb=EntityGetFirstComponentIncludingDisabled(entity_item,'OrbComponent')
    local observation=d1_ok and orb and D1.begin('orb',entity_who_picked,entity_item,original and original.eventId,ComponentGetValue2(orb,'orb_id'))
    _item_pickup(entity_item, entity_who_picked, name)
    if observation then D1.commit(observation)end
    if original then NT2_C1:submit(original)end
end

--hook EntityLoad to inject fake_orb tag onto spawned heart
function EntityLoad(filename, pos_x, pos_y)
    --in "no curse" mode, replace fake orb's cursed hearts with real hearts (anti griefing option)
    --but the originator's (orb has no fake_orb tag) cursed heart remains real
    if GameHasFlagRun("NT_sync_orbs_no_curse") and filename == "data/entities/items/pickup/heart_evil.xml" then
        --not a huge fan of doing it this way but other options are uglier
        local orbs = EntityGetInRadiusWithTag(pos_x, pos_y, 1, "fake_orb") --should be at exactly same x,y position
        if #orbs > 0 then --probably never 0 or 1
            --replace with a normal heart; will get fake_orb tag down below
            filename = "data/entities/items/pickup/heart.xml"
        end
    end

    local entity_id = _EntityLoad(filename, pos_x, pos_y)
    if filename == "data/entities/items/pickup/heart.xml" then
        EntityAddTag(entity_id,"fake_orb")
        --nt print_error("inject fake_orb tag into orb heart OK")
    end
    return entity_id
end
