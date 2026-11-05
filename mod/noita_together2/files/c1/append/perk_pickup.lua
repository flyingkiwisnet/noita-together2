local nt2_ok,NT2_C1=pcall(dofile,'mods/noita_together2/files/g3/c1_hook_api.lua')
if not nt2_ok then NT2_C1=nil end
-- Adapted from legacy perk pickup/nullification no-share hooks.
local _item_pickup = item_pickup

function item_pickup(entity_item, entity_who_picked, item_name)
    if NT2_C1 and NT2_C1.duplicate and NT2_C1:duplicate(entity_item)then return end
    local list = dofile("mods/noita_together2/files/g3/c1_gameplay.lua").team_perks
    local key_perk_components = GetKeyPerkComponents(entity_item)
    local perk_id = key_perk_components.perk_id
    local blocked = key_perk_components.nt_dont_share
    local banned_perk = IsBannedPerkId(perk_id)

    local original,why
    if NT2_C1 and NT2_C1:can_pickup(entity_who_picked) and not blocked and not banned_perk then
        if GameHasFlagRun("NT_sync_perks") or GameHasFlagRun("NT_team_perks") and list[perk_id] then
            original,why=NT2_C1:capture("perk",{perkId=perk_id},entity_item)
        if why=='duplicate'then return end
            if original then GamePrint("$noitatogether_teamperk_was_shared")end
        end
    end
    _item_pickup(entity_item, entity_who_picked, item_name)
    if original then NT2_C1:submit(original)end
end
