-- Actual settings entry/framework callback routing; engine APIs injected.
local repo=assert(os.getenv('NT2_A3_REPO'));local raw=dofile;local checks=0
local function check(v,s)assert(v,s);checks=checks+1 end
local settings,held,labels,scopes={}, {}, {}, {};local clicks=false;local updated,framework_count,framework_draw=0,0,0
MOD_SETTING_SCOPE_RUNTIME=1;MOD_SETTING_SCOPE_RUNTIME_RESTART=2;MOD_SETTING_SCOPE_NEW_GAME=3;MOD_SETTING_SCOPE_ONLY_SET_DEFAULT=4
ModIsEnabled=function(id)return id=='noita_together2'end
ModSettingGet=function(id)return settings[id]end;ModSettingSet=function(id,v)settings[id]=v end;ModSettingSetNextValue=function(id,v)settings[id]=v end
InputIsKeyDown=function(id)return held[id]or false end;InputIsMouseButtonDown=function()return false end;InputIsJoystickButtonDown=function()return false end
GuiButton=function(_,_,_,_,label)labels[#labels+1]=label;local c=clicks;clicks=false;return c end;GuiText=function(_,_,_,label)labels[#labels+1]=label end;GuiTooltip=function()end
dofile=function(path)
 if path=='data/scripts/lib/mod_settings.lua'then
  mod_settings_update=function(id,entries)check(id=='noita_together2'and #entries==8,'real framework receives eight NT2 settings');updated=updated+1 end
  mod_settings_gui_count=function()framework_count=framework_count+1;return 7 end
  mod_settings_gui=function()framework_draw=framework_draw+1 end;return
 elseif path:find('/startup/settings_scope.lua',1,true)then return {produce=function(kind)scopes[#scopes+1]=kind end}end
 return raw(repo..'/'..path:gsub('^mods/','mod/'))
end
raw(repo..'/mod/noita_together2/settings.lua');check(#mod_settings==8,'one eight-setting catalogue')
local p=dofile('mods/noita_together2/files/g3/social_preferences.lua');for _,s in ipairs(mod_settings)do check(s.value_default==p.defaults[s.id],'stored default '..s.id);check(s.scope==(s.id=='NT_ENABLE_LOGGER'and 2 or 1),'runtime/restart scope '..s.id)end
for scope,kind in ipairs({'runtime','restart','new','menu'})do ModSettingsUpdate(scope);check(scopes[#scopes]==kind,'entry scope routing retained')end
check(updated==4 and ModSettingsGuiCount()==8 and framework_count==1,'framework update/count callbacks retained');ModSettingsGui(1,false);check(labels[#labels]:find('Q',1,true),'human-readable current binding')
clicks=true;ModSettingsGui(1,false);ModSettingsGui(1,false);held[4]=true;ModSettingsGui(1,false);held[4]=nil;ModSettingsGui(1,false);check(settings['noita_together2.NT_EMOTE_BINDING']=='key_code,4,mouse_code,joystick_code','visible rebinder commits released native chord')
local prior=settings['noita_together2.NT_EMOTE_BINDING'];clicks=true;ModSettingsGui(1,false);ModSettingsGui(1,false);held[41]=true;ModSettingsGui(1,false);held[41]=nil;check(settings['noita_together2.NT_EMOTE_BINDING']==prior,'Escape preserves prior valid binding')
ModIsEnabled=function()return true end;ModSettingsGui(1,false);check(table.concat(labels,'\n'):find('M%-Nee owns'),'M-Nee priority explained in real settings UI');InputIsKeyDown=nil;ModSettingsGui(1,false);check(table.concat(labels,'\n'):find('Native binding unavailable',1,true),'unavailable integration has visible feedback');check(framework_draw>5,'existing framework GUI remains reachable')
print('PASS A3 settings '..checks..' assertions; actual settings entry, injected framework/input')
