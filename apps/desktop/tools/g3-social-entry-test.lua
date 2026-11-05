-- Executes actual unapproved ENTRY initialization/callbacks. All engine/DLL facts injected.
local root=assert(os.getenv('NT2_G3_MODULES'));local ffi=require('ffi')
local function u(n,count) local s='';for _=1,count do s=s..string.char(n%256);n=math.floor(n/256) end;return s end
local function str(s) return u(#s,2)..s end
local ctx=string.rep('\1',16);local generation=7;local sequence=1;local playing=true;local fault=false;local closes=0
local function frame()
 local body='\7\1'..str('peer')..'\1'..str('peer')..str('literal 雪')..u(sequence,4)..u(3000,4)..u(2000,4)..'\1'..u(500,2)
 return 'NSF1\1\0\0\0'..ctx..u(generation,4)..u(sequence,4)..u(#body,4)..u(2,2)..'\0\0'..body
end
local next_frame=frame();local native={}
native.nt2_g3_prepare=function(out,context) out[0]=424242;ffi.copy(context,ctx,16);return 1 end
native.nt2_g3_social_lease=function(context,out,capacity)
 if fault then error('injected lease callback fault') end;if not playing then return 0 end
 local header='NSR1\1\1\0\0'..ctx..u(generation,4)..u(75,4)..u(0,4)..u(0,4)..u(1,8)..string.rep('\0',16);assert(#header==64);ffi.copy(out,header,64);return 64
end
native.nt2_g3_social_next=function(context,out,capacity) if not next_frame then return 0 end;local value=next_frame;next_frame=nil;ffi.copy(out,value,#value);return #value end
native.nt2_g3_close=function() closes=closes+1;return 1 end
local terminal_frame
native.nt2_g3_social_terminal=function(context,out) if not terminal_frame then return 0 end;local value=terminal_frame;terminal_frame=nil;ffi.copy(out,value,#value);return #value end
native.nt2_g3_status=function() return 2 end
for _,name in ipairs({'nt2_g3_seed_accepted','nt2_g3_world_loaded','nt2_g3_report','nt2_g3_move','nt2_g3_peers'}) do native[name]=function() return 1 end end
local e={};setmetatable(e,{__index=_G});e._G=e
e.ModMagicNumbersFileAdd=function() end;e.dofile_once=function() end;e.SetWorldSeed=function() end;e.GameGetFrameNum=function() return 1 end;e.GameGetRealWorldTimeSinceStarted=function() return 0 end
e.GuiCreate=function() return 1 end;e.GuiDestroy=function() end;e.GuiStartFrame=function() end;e.GuiText=function() end;e.GuiButton=function() return false end;e.GuiImage=function() end
e.EntityGetIsAlive=function() return true end
e.dofile=function(path)
 if path=='mods/noita_together2/files/inspection.lua' then return {collect=function() return {actualSeed='424242',complete=true} end,content=function() return 'same' end,encode=function() return '{}',true end} end
 if path=='mods/noita_together2/files/g3/native_owner.lua' then return {new=function() return {tick=function() end,close=function() end} end} end -- Reviewed ghost path tested separately.
 return dofile(root..'/'..assert(path:match('/files/g3/(.+)$')))
end
local entry=root:gsub('/files/g3$','/init-g3-social.lua');local chunk=assert(loadfile(entry));setfenv(chunk,e);chunk()
local original_load=ffi.load;ffi.load=function(path) assert(path=='mods/noita_together2/files/nt2_g3_social.dll');return native end
e.OnModPreInit();ffi.load=original_load
local function visit(fn,target,value,seen)
 seen=seen or {};if seen[fn] then return end;seen[fn]=true
 for i=1,40 do local name,old=debug.getupvalue(fn,i);if not name then break end;if name==target then debug.setupvalue(fn,i,value) elseif type(old)=='function' then visit(old,target,value,seen) end end
end
-- Inject loaded/accepted engine observations; bridge always comes from ACTUAL OnModPreInit.
visit(e.OnWorldPreUpdate,'player',7);visit(e.OnWorldPreUpdate,'accepted',true);visit(e.OnWorldPreUpdate,'loaded',true)
e.OnWorldPreUpdate()
local social
for i=1,40 do local name,value=debug.getupvalue(e.OnWorldPreUpdate,i);if name=='social_gameplay' then social=value end end
assert(social and social.renderer.hud.peer and social.renderer.hud.peer.hp==3000,'Actual entry must deliver native HUD callback')
playing=false;e.OnWorldPreUpdate();assert(not next(social.renderer.hud),'No authority clears owned HUD')
playing=true;sequence=2;next_frame=frame();e.OnWorldPreUpdate();assert(social.renderer.hud.peer)
fault=true;e.OnWorldPreUpdate();assert(not next(social.renderer.hud) and closes>=1,'Entry fault must close owned state')
-- A separate injected world reinitializes the ACTUAL entry, then receives control-only End.
ctx=string.rep('\2',16);generation=8;sequence=1;fault=false;playing=true;next_frame=frame()
chunk=assert(loadfile(entry));setfenv(chunk,e);chunk();ffi.load=function() return native end;e.OnModPreInit();ffi.load=original_load
visit(e.OnWorldPreUpdate,'player',7);visit(e.OnWorldPreUpdate,'accepted',true);visit(e.OnWorldPreUpdate,'loaded',true);e.OnWorldPreUpdate()
for i=1,40 do local name,value=debug.getupvalue(e.OnWorldPreUpdate,i);if name=='social_gameplay' then social=value end end
assert(social.renderer.hud.peer)
terminal_frame='NSF1\1\0\0\0'..ctx..u(generation,4)..u(2,4)..u(1,4)..u(1,2)..'\0\0\6';playing=false;e.OnWorldPreUpdate()
assert(social.ended and not next(social.renderer.hud),'Actual entry terminal clears and retires')
playing=true;sequence=3;next_frame=frame();e.OnWorldPreUpdate();assert(social.ended and not next(social.renderer.hud),'Terminal never accepts renewed fake gameplay lease')
print('PASS actual social entry initializes DLL bridge, delivers HUD, holds, resumes, contains fault and permanently retires on End')
