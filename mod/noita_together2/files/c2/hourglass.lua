-- Adapted from legacy Noita Together hourglass_events.lua (MIT, Soler91).
-- Trusted local registry only. Remote values select a closed registered ID.
local M={}
local translations='mods/noita_together2/files/c2/translations.csv'
local languages={'en','ru','pt-br','es-es','de','fr-fr','it','pl','zh-cn','jp','ko'}
local function lines(value)
 local out={}
 for line in ((value or ''):gsub('\r\n','\n'):gsub('\r','\n')..'\n'):gmatch('(.-)\n')do
  if line:find('%S')then out[#out+1]=line end
 end
 return out
end
-- Noita/legacy CSV uses escaped \n inside a field, never physical multiline text.
local function fields(line)
 local out,value,quoted={},'',false;local i=1
 while i<=#line do
  local c=line:sub(i,i)
  if c=='"'then
   if quoted and line:sub(i+1,i+1)=='"'then value=value..'"';i=i+1 else quoted=not quoted end
  elseif c==','and not quoted then out[#out+1]=value;value=''
  else value=value..c end
  i=i+1
 end
 assert(not quoted,'Invalid C2 translation CSV');out[#out+1]=value;return out
end
local function encode(row)
 local out={};for i,value in ipairs(row)do
  if value:find('[,"\r\n]')then value='"'..value:gsub('"','""')..'"'end;out[i]=value
 end;return table.concat(out,',')
end
local function source(e)
 local rows={};if type(e.ModTextFileGetContent)=='function'then
  for _,line in ipairs(lines(e.ModTextFileGetContent(translations)))do local row=fields(line);rows[row[1]]=row end
 end;return rows
end
function M.install_translations(e)
 local file='data/translations/common.csv';local base=lines(e.ModTextFileGetContent(file));assert(base[1],'Translation header missing')
 local header=fields(base[1]);local columns={};for i,name in ipairs(header)do columns[name]=i end
 assert(columns.en,'English translation column missing')
 local rows=source(e);local out={}
 -- Remove blank separators (including the earlier C1 append seam) and replace
 -- only our own keys, so repeated registration never creates shadowing rows.
 for _,line in ipairs(base)do local key=line:sub(1,1)=='"'and fields(line)[1]or line:match('^([^,]*)');if not rows[key]then out[#out+1]=line end end
 local keys={};for key in pairs(rows)do keys[#keys+1]=key end;table.sort(keys)
 for _,key in ipairs(keys)do
  local row=rows[key];local value={};for i=1,#header do value[i]=''end;value[1]=key
  for i,language in ipairs(languages)do local column=columns[language];if column then value[column]=row[i+1]and row[i+1]~=''and row[i+1]or row[2] end end
  out[#out+1]=encode(value)
 end
 e.ModTextFileSetContent(file,table.concat(out,'\n')..'\n')
end
function M.text(e,key,...)
 local args={...};for i=1,select('#',...)do if args[i]==nil then args[i]='Another player'end end
 local ok,value=pcall(e.GameTextGet,key,unpack(args))
 if ok and type(value)=='string'and value:find('%S')and value~=key and value~=key:gsub('^%$','')then return value end
 -- Keep required notices visible even when another mod or the engine cannot
 -- resolve a key. Format the legacy English template once, with literal names.
 local row=source(e)[key:gsub('^%$','')];local fallback=row and row[2]or key:gsub('^%$','')
 return (fallback:gsub('\\n','\n'):gsub('%$(%d)',function(n)return tostring(args[tonumber(n)+1]or '')end))
end
function M.new(e)
 local self={entries={}}
 function self:register(effect,material,handler,force)
  if type(effect)~='string'or #effect>64 or not effect:match('^[a-z][a-z0-9_]*$')or type(material)~='string'or not material:match('^[a-z][a-z0-9_]*$')then return false end
  if self.entries[effect]~=nil and not force then return false end
  if handler~=nil and type(handler)~='function'or handler==nil and not force then return false end
  self.entries[effect]={material=material,handler=handler};return true
 end
 function self:apply(effect,player,sender,data)
  local entry=self.entries[effect]
  if not entry or type(entry.handler)~='function'then return 'skipped','unsupported_content'end
  entry.handler(player,sender,data);return player and 'applied'or 'unissued',player and 'effect_applied'or 'actor_missing'
 end
    assert(self:register("ambrosia", "magic_liquid_protection_all", function(player, triggered_by_name, data)
        local x, y = (function()if player and e.EntityGetIsAlive(player)then return e.EntityGetTransform(player)end;return e.GameGetCameraPos()end)()
        local effect_entity = e.EntityLoad("mods/noita_together2/files/c2/effects/weak_ambrosia.xml", x, y)
        if data and data.eventId then e.EntityAddComponent2(effect_entity,'VariableStorageComponent',{name='nt2_c2_ambrosia_key',value_string=data.eventId})end
        e.EntityAddChild(player, effect_entity)
        e.EntityAddComponent2(effect_entity, "UIIconComponent", {
            icon_sprite_file = "mods/noita_together2/files/c2/ui_gfx/status_indicators/nt_hourglass_protection_half.png",
            name = M.text(e,"$noitatogether_hourglass_buff_protection_name"),
            description = M.text(e,"$noitatogether_hourglass_buff_protection_desc", triggered_by_name),
            display_above_head = true,
            display_in_hud = true,
            is_perk = false
        })
        e.GamePrintImportant(M.text(e,"$noitatogether_hourglass_boost_title", triggered_by_name), M.text(e, "$noitatogether_hourglass_ambrosia_subtitle"))
    end))

    assert(self:register("berserk", "magic_liquid_berserk", function(player, triggered_by_name, data)
        local effect_entity = e.LoadGameEffectEntityTo(player, "data/entities/misc/effect_damage_multiplier.xml")
        local effect_comp = e.EntityGetFirstComponent(effect_entity, "GameEffectComponent")
        e.ComponentSetValue2(effect_comp, "frames", 60*120)
        e.EntityAddComponent2(effect_entity, "UIIconComponent", {
            icon_sprite_file = "data/ui_gfx/status_indicators/berserk.png",
            name = M.text(e,"$noitatogether_hourglass_buff_damage_name"),
            description = M.text(e,"$noitatogether_hourglass_buff_damage_desc", triggered_by_name),
            display_above_head = true,
            display_in_hud = true,
            is_perk = false
        })
        e.GamePrintImportant(M.text(e,"$noitatogether_hourglass_boost_title", triggered_by_name), M.text(e, "$noitatogether_hourglass_berserk_subtitle"))
    end))

    assert(self:register("charm", "magic_liquid_charm", function(player, triggered_by_name, data)
        local x, y = (function()if player and e.EntityGetIsAlive(player)then return e.EntityGetTransform(player)end;return e.GameGetCameraPos()end)()
        e.EntityLoad("data/entities/projectiles/deck/regeneration_field_long.xml", x, y)
        e.GamePrintImportant(M.text(e,"$noitatogether_hourglass_boon_title", triggered_by_name), "")
    end))

    assert(self:register("confusion", "material_confusion", function(player, triggered_by_name, data)
        local fungi = e.CellFactory_GetType("fungi")
        if (player ~= nil) then
            e.EntityIngestMaterial( player, fungi, 300 )
            local stomach = e.EntityGetFirstComponent(player, "IngestionComponent")
            if (stomach ~= nil) then
                local ingestion_size = e.ComponentGetValue2(stomach, "ingestion_size")
                e.ComponentSetValue2(stomach, "ingestion_size", math.max(0, ingestion_size - 300))
            end
            e.GamePrintImportant(M.text(e,"$noitatogether_hourglass_fungus_title", triggered_by_name), M.text(e, "$noitatogether_hourglass_fungus_subtitle_yes"))
        else
            e.GamePrintImportant(M.text(e,"$noitatogether_hourglass_fungus_title", triggered_by_name), M.text(e, "$noitatogether_hourglass_fungus_subtitle_no"))
        end
    end))

    --accel boosts your speed (only)
    assert(self:register("speed", "magic_liquid_movement_faster", function(player, triggered_by_name, data)
        local effect_entity = e.LoadGameEffectEntityTo(player, "data/entities/misc/effect_movement_faster_2x.xml")
        local effect_comp = e.EntityGetFirstComponent(effect_entity, "GameEffectComponent")
        e.ComponentSetValue2(effect_comp, "frames", 60*120)
        e.EntityAddComponent2(effect_entity, "UIIconComponent", {
            icon_sprite_file = "data/ui_gfx/status_indicators/movement_faster.png",
            name = M.text(e,"$noitatogether_hourglass_buff_speed_name"),
            description = M.text(e,"$noitatogether_hourglass_buff_speed_desc", triggered_by_name),
            display_above_head = true,
            display_in_hud = true,
            is_perk = false
        })
        e.GamePrintImportant(M.text(e,"$noitatogether_hourglass_boost_title", triggered_by_name), M.text(e, "$noitatogether_hourglass_speed_subtitle"))
    end))

    --levitatium boosts your levitation (only)
    assert(self:register("levitation", "magic_liquid_faster_levitation", function(player, triggered_by_name, data)
        local effect_entity = e.LoadGameEffectEntityTo(player, "data/entities/misc/effect_faster_levitation.xml")
        local effect_comp = e.EntityGetFirstComponent(effect_entity, "GameEffectComponent")
        e.ComponentSetValue2(effect_comp, "frames", 60*120)
        e.EntityAddComponent2(effect_entity, "UIIconComponent", {
            icon_sprite_file = "data/ui_gfx/status_indicators/faster_levitation.png",
            name = M.text(e,"$noitatogether_hourglass_buff_levitatium_name"),
            description = M.text(e,"$noitatogether_hourglass_buff_levitatium_desc", triggered_by_name),
            display_above_head = true,
            display_in_hud = true,
            is_perk = false
        })
        e.GamePrintImportant(M.text(e,"$noitatogether_hourglass_boost_title", triggered_by_name), M.text(e, "$noitatogether_hourglass_levitatium_subtitle"))
    end))

    --hastium boosts both levitation and speed
    assert(self:register("speed_and_levitation", "magic_liquid_faster_levitation_and_movement", function(player, triggered_by_name, data)
        local effect_entity = e.LoadGameEffectEntityTo(player, "mods/noita_together2/files/c2/effects/hourglass_hastium.xml")
        local effect_comp = e.EntityGetFirstComponent(effect_entity, "GameEffectComponent")
        e.ComponentSetValue2(effect_comp, "frames", 60*120)
        e.EntityAddComponent2(effect_entity, "UIIconComponent", {
            icon_sprite_file = "mods/noita_together2/files/c2/ui_gfx/status_indicators/nt_hourglass_speed_levitation.png",
            name = M.text(e,"$noitatogether_hourglass_buff_hastium_name"),
            description = M.text(e,"$noitatogether_hourglass_buff_hastium_desc", triggered_by_name),
            display_above_head = true,
            display_in_hud = true,
            is_perk = false
        })
        e.GamePrintImportant(M.text(e,"$noitatogether_hourglass_boost_title", triggered_by_name), M.text(e, "$noitatogether_hourglass_hastium_subtitle"))
    end))
 return self
end
return M
