local d1_ok,D1=pcall(dofile,'mods/noita_together2/files/g3/d1_hook_api.lua')
local nt2_ok,NT2_C1=pcall(dofile,'mods/noita_together2/files/g3/c1_hook_api.lua')
if not nt2_ok then NT2_C1=nil end
-- Adapted from legacy Noita Together heart pickup; local modifiers/effects retained.
local _item_pickup = item_pickup

local function shared_heart(entity_item, entity_who_picked, name)
    local damagemodels = EntityGetComponent(entity_who_picked, "DamageModelComponent")
    local variablestorages = EntityGetComponent(entity_who_picked, "VariableStorageComponent")

    local max_hp_old = 0
    local max_hp = 0
    local multiplier = tonumber( GlobalsGetValue( "HEARTS_MORE_EXTRA_HP_MULTIPLIER", "1" ) )
    local min_hp_to_add = 0.16 * multiplier

    local x, y = EntityGetTransform(entity_item)
    local playercount = NT2_C1:roster_count()
    if (damagemodels ~= nil) then
        for i, damagemodel in ipairs(damagemodels) do
            max_hp = tonumber(ComponentGetValue(damagemodel, "max_hp"))
            max_hp_old = max_hp
            if (GameHasFlagRun("NT_sync_hearts")) then
                local expected_max_hp = min_hp_to_add
                if(playercount > 0) then
                    expected_max_hp = (1 / (playercount)) * multiplier
                end
                max_hp = max_hp + math.max(min_hp_to_add , expected_max_hp)
            else
                max_hp = max_hp + 1 * multiplier
            end
            local max_hp_cap = tonumber(ComponentGetValue(damagemodel, "max_hp_cap"))
            if max_hp_cap > 0 then
                max_hp = math.min(max_hp, max_hp_cap)
            end

            -- if( hp > max_hp ) then hp = max_hp end
            ComponentSetValue(damagemodel, "max_hp", max_hp)
        end
    end

    EntityLoad("data/entities/particles/image_emitters/heart_effect.xml", x, y - 12)
    EntityLoad("data/entities/particles/heart_out.xml", x, y - 8)
    local description = GameTextGet("$logdesc_heart", tostring(math.floor(max_hp * 25)))
    if max_hp == max_hp_old then
        description = GameTextGet("$logdesc_heart_blocked", tostring(math.floor(max_hp * 25)))
    end

    GamePrintImportant("$log_heart", description)
    if playercount == 0 then
        description = GameTextGet("$noitatogether_heart_blocked_playercount")
        print_error(description)
        GamePrint(description)
    end
    GameTriggerMusicCue("item")
    EntityKill(entity_item)
end

function item_pickup(entity_item, entity_who_picked, name)
    if NT2_C1 and NT2_C1.duplicate and NT2_C1:duplicate(entity_item)then return end
    if not EntityGetIsAlive(entity_item)then return end
    if (NT2_C1 and NT2_C1:can_pickup(entity_who_picked) and GameHasFlagRun("NT_sync_hearts")) then
        local original,why
        if not EntityHasTag(entity_item,"fake_orb") then
            original,why=NT2_C1:capture("heart",{},entity_item)
        if why=='duplicate'then return end
        end
        local observation=d1_ok and D1.begin('heart',entity_who_picked,entity_item,original and original.eventId)
        shared_heart(entity_item, entity_who_picked, name)
        if observation then D1.commit(observation)end
        if original then NT2_C1:submit(original) end
    else
        local observation=d1_ok and D1.begin('heart',entity_who_picked,entity_item)
        _item_pickup(entity_item, entity_who_picked, name)
        if observation then D1.commit(observation)end
    end
end
