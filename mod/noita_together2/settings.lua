-- Local preferences plus the existing framework settings-scope routing.
dofile("data/scripts/lib/mod_settings.lua")
local preferences=dofile('mods/noita_together2/files/g3/social_preferences.lua')
mod_settings={
 {id='NT_HINTS',ui_name='Show optional hints',value_default=true,scope=MOD_SETTING_SCOPE_RUNTIME},
 {id='NT_NO_STAT_PROGRESS',ui_name='Disable vanilla discovery progress',ui_description='Local perks, spells and enemies only; does not control server statistics.',value_default=false,scope=MOD_SETTING_SCOPE_RUNTIME},
 {id='NT_FOLLOW_DEFAULT',ui_name='Track newly appearing players by default',ui_description='Adds new players to radar/follow. Manual choices are retained; does not hide ghosts.',value_default=false,scope=MOD_SETTING_SCOPE_RUNTIME},
 {id='NT_GHOST_OPACITY',ui_name='Player ghost opacity',value_default=1,value_min=0,value_max=1,value_display_multiplier=100,scope=MOD_SETTING_SCOPE_RUNTIME},
 {id='NT_ENABLE_LOGGER',ui_name='Enable local debug logger (restart required)',ui_description='Applied at restart. Bounded diagnostics stay local and are never automatically uploaded.',value_default=true,scope=MOD_SETTING_SCOPE_RUNTIME_RESTART},
 {id='NT_SHOW_WUOTE_FRIDGE',ui_name='Use fridge icon for bank',ui_description='Show the legacy fridge icon for the gold bank.',value_default=false,scope=MOD_SETTING_SCOPE_RUNTIME},
 {id='NT_SHOW_EMOTES',ui_name='Show emotes and robe colours',ui_description='Enables the emote menu and local/remote presentation; disabling restores owned effects.',value_default=true,scope=MOD_SETTING_SCOPE_RUNTIME},
 {id='NT_EMOTE_BINDING',value_default='key_code,20,mouse_code,joystick_code',hidden=true,scope=MOD_SETTING_SCOPE_RUNTIME}
}
local ok,scope=pcall(dofile,'mods/noita_together2/files/startup/settings_scope.lua')
mod_settings_version=2
function ModSettingsUpdate(init_scope)
 if mod_settings_update then mod_settings_update("noita_together2",mod_settings,init_scope)end
 if not ok then return end -- No cross-context facility means no fresh permission.
 local available,enabled=pcall(function() return ModIsEnabled('noita_together2') end)
 if not available or enabled~=true then scope.produce('unknown');return end
 local kind='unknown'
 if init_scope==MOD_SETTING_SCOPE_NEW_GAME then kind='new'
 elseif init_scope==MOD_SETTING_SCOPE_RUNTIME_RESTART then kind='restart'
 elseif init_scope==MOD_SETTING_SCOPE_RUNTIME then kind='runtime'
 elseif init_scope==MOD_SETTING_SCOPE_ONLY_SET_DEFAULT then kind='menu' end
 scope.produce(kind)
end
function ModSettingsGuiCount()return mod_settings_gui_count('noita_together2',mod_settings)+1 end
local status='';local controls=dofile('mods/noita_together2/files/g3/social_controls.lua');local keys=dofile('mods/noita_together2/files/g3/social_keys.lua');local listener=controls.rebinder(_G,keys)
function ModSettingsGui(gui,in_main_menu)
 mod_settings_gui('noita_together2',mod_settings,gui,in_main_menu)
 local value,reason=listener:poll();if value then preferences.set(_G,'NT_EMOTE_BINDING',value)end;if reason then status=reason end
 local binding=preferences.get(_G,'NT_EMOTE_BINDING');local supported=type(InputIsKeyDown)=='function'
 local label=(listener.listening and 'Listening: release chord to save; Esc cancels'or 'Current emote binding: '..controls.label(binding,keys))
 if supported then
  if GuiButton(gui,100001,0,0,label)then if listener.listening then listener:cancel();status='Binding unchanged'else listener:start();status=''end end
  if GuiTooltip then GuiTooltip(gui,'Keyboard + mouse + controller 0 chords; previous binding is kept on cancel or invalid input.','')end
 else GuiText(gui,0,0,'Native binding unavailable; use supported M-Nee or update Noita')end
 if ModIsEnabled and ModIsEnabled('mnee')then GuiText(gui,0,0,'M-Nee owns the emote binding; configure NT2 Emote Menu there. Load M-Nee before NT2.')end
 if status~=''then GuiText(gui,0,0,status)end
end
