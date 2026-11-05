-- Actual entry/inspection callbacks over the real injected private native bridge.
-- Engine protection components and game APIs are injected; never executes Noita.
return function(dll,context,root)
 local ffi=require('ffi');local frame=0;local values={enabled=false,wait_for_kill_flag_on_death=true}
 local body,sequence;local bridge={}
 for _,name in ipairs({'nt2_g3_status','nt2_g3_close','nt2_g3_seed_accepted','nt2_g3_world_loaded'}) do bridge[name]=dll[name] end
 bridge.nt2_g3_report=function(c,b,n,s) body=ffi.string(b,n);sequence=s;return dll.nt2_g3_report(c,b,n,s) end
 local e={};setmetatable(e,{__index=_G});e._G=e
 e.ModMagicNumbersFileAdd=function() end;e.dofile_once=function() end
 e.GameGetFrameNum=function() return frame end;e.StatsGetValue=function() return '424242' end
 e.ModGetActiveModIDs=function() return {'noita_together2'} end;e.GameIsBetaBuild=function() return nil end
 e.EntityGetIsAlive=function() return true end;e.EntityGetTransform=function() return 0,0 end
 e.EntityGetFirstComponentIncludingDisabled=function(_,kind) if kind=='ControlsComponent' then return 1 elseif kind=='DamageModelComponent' then return 2 elseif kind=='GameEffectComponent' then return 3 end end
 e.ComponentGetValue2=function(_,key) return values[key] end;e.ComponentSetValue2=function(_,key,value) values[key]=value end
 e.EntityHasTag=function() return true end;e.EntityAddComponent2=function() return 4 end;e.LoadGameEffectEntityTo=function() return 5 end
 for _,name in ipairs({'EntityRemoveComponent','EntityKill','EntityRemoveTag','EntityAddTag','EntitySetTransform','EntitySetComponentIsEnabled'}) do e[name]=function() end end
 e.dofile=function(path)
  if path:find('/native_owner.lua',1,true) or path:find('/social_native.lua',1,true) then return {new=function() return {tick=function() end,close=function() end} end} end
  local target=path:find('/inspection.lua',1,true) and root:gsub('/files/g3$','/files/inspection.lua') or root..'/'..assert(path:match('/files/g3/(.+)$'))
  local chunk=assert(loadfile(target));setfenv(chunk,e);return chunk()
 end
 local path=root:gsub('/files/g3$','/init-g3-social.lua');local chunk=assert(loadfile(path));setfenv(chunk,e);chunk()
 local function set(fn,target,value,seen)
  seen=seen or {};if seen[fn] then return end;seen[fn]=true
  for i=1,60 do local name,old=debug.getupvalue(fn,i);if not name then break end
   if name==target then debug.setupvalue(fn,i,value) elseif type(old)=='function' then set(old,target,value,seen) end
  end
 end
 for name,value in pairs({bridge=bridge,context=context,seed=424242,accepted=true,loaded=true,player=7,
  hold={player=7,controls=1,damage=2,enabled=true,wait=false,inventory={},x=0,y=0}}) do set(e.OnWorldPreUpdate,name,value) end
 local self={holds=0,ticks=0}
 function self:tick()
  local before=values.enabled;frame=frame+1;e.OnWorldPreUpdate();self.ticks=self.ticks+1
  if before and not values.enabled then self.holds=self.holds+1 end
 end
 function self:playing() return values.enabled and dll.nt2_g3_status(context)==2 end
 function self:acknowledged(playing)
  if not body or not body:find('"protected":'..tostring(not playing),1,true) then return false end
  return dll.nt2_g3_report(context,ffi.cast('const uint8_t*',body),#body,sequence)==1
 end
 return self
end
