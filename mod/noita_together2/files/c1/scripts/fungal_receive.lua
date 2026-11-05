-- Adapted from legacy utils.lua/DoFungalShift; no reroll or rebroadcast.
return function(e, player, from, to)
    local entity = player
    local x, y = EntityGetTransform(player)
	local frame = GameGetFrameNum()
	local last_frame = tonumber( GlobalsGetValue( "fungal_shift_last_frame", "-1000000" ) )
	if frame < last_frame + 60*60*5 then
		return 'skipped','fungal_cooldown' -- long cooldown
	end

	local comp_worldstate = EntityGetFirstComponent( GameGetWorldStateEntity(), "WorldStateComponent" )
	if( comp_worldstate ~= nil and ComponentGetValue2( comp_worldstate, "EVERYTHING_TO_GOLD" ) ) then
		return 'skipped','gold_world' -- do nothing in case everything is gold
	end

	local iter = tonumber( GlobalsGetValue( "fungal_shift_iteration", "0" ) )
	GlobalsSetValue( "fungal_shift_iteration", tostring(iter+1) )
	if iter > 20 then
		return 'skipped','fungal_limit'
	end

	SetRandomSeed( 89346, 42345+iter )

	local converted_any = false
	local from_material_name = ""

	-- apply effects
	for i,it in ipairs(from.materials) do
		local from_material = CellFactory_GetType( it )
		local to_material = CellFactory_GetType( to.material )
		from_material_name = string.upper( GameTextGetTranslatedOrNot( CellFactory_GetUIName( from_material ) ) )
		if from.name_material then
			from_material_name = string.upper( GameTextGetTranslatedOrNot( CellFactory_GetUIName( CellFactory_GetType( from.name_material ) ) ) )
		end

		-- convert
		if from_material ~= to_material then
			print(CellFactory_GetUIName(from_material) .. " -> " .. CellFactory_GetUIName(to_material))
			ConvertMaterialEverywhere( from_material, to_material )
			converted_any = true

			-- shoot particles of new material
			GameCreateParticle( CellFactory_GetName(from_material), x-10, y-10, 20, rand(-100,100), rand(-100,-30), true, true )
			GameCreateParticle( CellFactory_GetName(from_material), x+10, y-10, 20, rand(-100,100), rand(-100,-30), true, true )
		end
	end

	-- fx
	if converted_any then
		-- audio
		GameTriggerMusicFadeOutAndDequeueAll( 5.0 )
		GameTriggerMusicEvent( "music/oneshot/tripping_balls_01", false, x, y )

		-- particle fx
		local eye = EntityLoad( "data/entities/particles/treble_eye.xml", x,y-10 )
		if eye ~= 0 and entity ~= 0 then
			EntityAddChild( entity, eye )
		end

		-- log
		local log_msg = ""
		if from_material_name ~= "" then
			log_msg = GameTextGet( "$logdesc_reality_mutation", from_material_name )
			GamePrint( log_msg )
		end
		--GamePrintImportant( random_from_array( log_messages ), log_msg, "data/ui_gfx/decorations/3piece_fungal_shift.png" ) do we really want to annoy everyone with this?
		GlobalsSetValue( "fungal_shift_last_frame", tostring(frame) )

		-- add ui icon
		local add_icon = true
		local children = EntityGetAllChildren(entity)
		if children ~= nil then
			for i,it in ipairs(children) do
				if ( EntityGetName(it) == "fungal_shift_ui_icon" ) then
					add_icon = false
					break
				end
			end
		end

		if add_icon and entity ~= 0 then
			local icon_entity = EntityCreateNew( "fungal_shift_ui_icon" )
			EntityAddComponent( icon_entity, "UIIconComponent", 
			{ 
				name = "$status_reality_mutation",
				description = "$statusdesc_reality_mutation",
				icon_sprite_file = "data/ui_gfx/status_indicators/fungal_shift.png"
			})
			EntityAddChild( entity, icon_entity )
		end
	end
    return converted_any and 'applied' or 'skipped', converted_any and 'effect_applied' or 'fungal_noop'
end
