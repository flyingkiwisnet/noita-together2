local nt2_ok,NT2_C1=pcall(dofile,'mods/noita_together2/files/g3/c1_hook_api.lua')
if not nt2_ok then NT2_C1=nil end
-- Adapted from legacy Noita Together fungal_shift.lua; selection/RNG order retained.
function fungal_shift( entity, x, y, debug_no_limits )
	local parent = EntityGetParent( entity )
	if parent ~= 0 then
		entity = parent
	end

	local frame = GameGetFrameNum()
	local last_frame = tonumber( GlobalsGetValue( "fungal_shift_last_frame", "-1000000" ) )
	if frame < last_frame + 60*60*5 and not debug_no_limits then
		return -- long cooldown
	end

	local comp_worldstate = EntityGetFirstComponent( GameGetWorldStateEntity(), "WorldStateComponent" )
	if( comp_worldstate ~= nil and ComponentGetValue2( comp_worldstate, "EVERYTHING_TO_GOLD" ) ) then
		return -- do nothing in case everything is gold
	end

	local iter = tonumber( GlobalsGetValue( "fungal_shift_iteration", "0" ) )
	GlobalsSetValue( "fungal_shift_iteration", tostring(iter+1) )
	if iter > 20 and not debug_no_limits then
		return
	end

	SetRandomSeed( 89346, 42345+iter )

	local converted_any = false

	local rnd = random_create(9123,58925+iter ) -- TODO: store for next change
	local from = pick_random_from_table_weighted( rnd, materials_from )
	local to = pick_random_from_table_weighted( rnd, materials_to )
	local held_material = get_held_item_material( entity )
	local from_material_name = ""

	-- if a potion is equipped, randomly use main material from potion as one of the materials
	if held_material > 0 and random_nexti( rnd, 1, 100 ) <= 75 then
		if random_nexti( rnd, 1, 100 ) <= 50 then
			from = {}
			from.materials = { CellFactory_GetName(held_material) }
		else
			to = {}
			to.material = CellFactory_GetName(held_material)
		end
	end

    local original
    if NT2_C1 and NT2_C1:can_pickup(entity) and GameHasFlagRun("NT_sync_shift") then
        original=NT2_C1:capture("fungal",{fromMaterials=from.materials,toMaterial=to.material,displayMaterial=from.name_material})
    end

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
		-- remove tripping effect
		EntityRemoveIngestionStatusEffect( entity, "TRIP" );

		-- audio
		GameTriggerMusicFadeOutAndDequeueAll( 5.0 )
		GameTriggerMusicEvent( "music/oneshot/tripping_balls_01", false, x, y )

		-- particle fx
		local eye = EntityLoad( "data/entities/particles/treble_eye.xml", x,y-10 )
		if eye ~= 0 then
			EntityAddChild( entity, eye )
		end

		-- log
		local log_msg = ""
		if from_material_name ~= "" then
			log_msg = GameTextGet( "$logdesc_reality_mutation", from_material_name )
			GamePrint( log_msg )
		end
		GamePrintImportant( random_from_array( log_messages ), log_msg, "data/ui_gfx/decorations/3piece_fungal_shift.png" )
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

		if add_icon then
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
    if original then NT2_C1:submit(original)end
end