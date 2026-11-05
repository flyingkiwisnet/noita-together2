-- Native keyboard/mouse/controller0 chords and optional M-Nee ownership.
local M={};local modes={key_code=true,mouse_code=true,joystick_code=true}
local function valid(mode,n)return n and n==math.floor(n)and (mode=='key_code'and n>=4 and n<=512 or mode=='mouse_code'and n>=1 and n<=7 or mode=='joystick_code'and n>=0 and n<=56)end
function M.parse(binding)
 if type(binding)~='string'or #binding>128 or binding:find(',,',1,true)or binding:sub(-1)==','then return nil end
 local out={key_code={},mouse_code={},joystick_code={}};local mode=nil;local count=0;local seen={}
 for token in binding:gmatch('[^,]+')do
  if modes[token]then if seen[token]then return nil end;seen[token]=true;mode=token
  else local n=tonumber(token);if not mode or not valid(mode,n)or tostring(n)~=token or seen[mode..token]then return nil end;seen[mode..token]=true;count=count+1;if count>8 then return nil end;out[mode][#out[mode]+1]=n end
 end
 return count>0 and out or nil
end
function M.label(binding,keys)
 local chord=M.parse(binding);if not chord then return 'Invalid binding (rebind to repair)'end
 local result={};for _,mode in ipairs({'key_code','mouse_code','joystick_code'})do for _,code in ipairs(chord[mode])do
  local name;for key,n in pairs(keys[mode])do if n==code and (not name or #key<#name or #key==#name and key<name)then name=key end end
  name=name or tostring(code);if mode=='joystick_code'then name='Controller 0 '..name elseif mode=='mouse_code'then name=name:gsub('MOUSE_','Mouse ')end;result[#result+1]=name
 end end;return table.concat(result,' + ')
end
function M.serialize(chord)
 local out={};for _,mode in ipairs({'key_code','mouse_code','joystick_code'})do out[#out+1]=mode;table.sort(chord[mode]);for _,n in ipairs(chord[mode])do out[#out+1]=tostring(n)end end
 local value=table.concat(out,',');return M.parse(value)and value or nil
end
local function pressed(e,mode,n)
 local fn=mode=='key_code'and e.InputIsKeyDown or mode=='mouse_code'and e.InputIsMouseButtonDown or e.InputIsJoystickButtonDown
 if type(fn)~='function'then return false end
 local ok,v;if mode=='joystick_code'then ok,v=pcall(fn,0,n)else ok,v=pcall(fn,n)end;return ok and v==true
end
function M.new(e)
 local self={mnee_loaded=false}
 function self:down(binding)
  if e.ModIsEnabled and e.ModIsEnabled('mnee')then
   if not self.mnee_loaded then self.mnee_loaded=true;pcall(e.dofile_once or dofile_once,'mods/mnee/lib.lua')end
   if type(e.is_binding_down)~='function'then return false,'M-Nee binding unavailable; check that M-Nee loads before NT2'end
   local ok,v=pcall(e.is_binding_down,'nt2_g3_emotes','emote_menu');return ok and v==true,not ok and 'M-Nee binding unavailable'or nil
  end
  if type(e.InputIsKeyDown)~='function'then return false,'Native input unavailable; enable supported M-Nee or update Noita'end
  local chord=M.parse(binding);if not chord then return false,'Invalid emote binding; rebind in NT2 settings'end
  for mode,values in pairs(chord)do for _,n in ipairs(values)do if not pressed(e,mode,n)then return false end end end;return true
 end
 return self
end
function M.menu_down(e)return M.new(e or _G):down((e or _G).ModSettingGet and (e or _G).ModSettingGet('noita_together2.NT_EMOTE_BINDING')or 'key_code,20,mouse_code,joystick_code')end
function M.rebinder(e,keys)
 local self={listening=false,armed=false,seen=false,chord=nil}
 function self:start()self.listening=true;self.armed=false;self.seen=false;self.chord={key_code={},mouse_code={},joystick_code={}}end
 function self:cancel()self.listening=false;self.chord=nil;return nil,'Binding unchanged'end
 function self:poll()
  if not self.listening then return end
  if self.armed and pressed(e,'key_code',41)then return self:cancel()end
  local down={};local any=false
  for mode,values in pairs(keys)do local seen={};for _,n in pairs(values)do if not seen[n]and pressed(e,mode,n)then seen[n]=true;any=true;down[#down+1]={mode,n}end end end
  if not self.armed then if not any then self.armed=true end;return end
  for _,v in ipairs(down)do local found=false;for _,n in ipairs(self.chord[v[1]])do if n==v[2]then found=true end end;if not found then self.chord[v[1]][#self.chord[v[1]]+1]=v[2]end end
  if any then self.seen=true elseif self.seen then local value=M.serialize(self.chord);self.listening=false;self.chord=nil;return value,value and 'Binding updated'or 'Invalid/empty chord; binding unchanged'end
 end
 return self
end
return M
