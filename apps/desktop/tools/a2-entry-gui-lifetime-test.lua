-- Actual packaged entry + social adapter/renderer. Native/gameplay facts and
-- the protected pre-update seam are injected; not real Noita/authority evidence.
local repo=assert(os.getenv('NT2_A2_REPO'));local raw=dofile;local ffi=require('ffi');local old_print=print
raw(repo..'/apps/desktop/tools/mod-protection-stubs.lua')
local logs={};print=function(s)logs[#logs+1]=s end
ModLuaFileAppend=function()end;ModLuaFileGetAppends=function()return {}end
local time=20;GameGetRealWorldTimeSinceStarted=function()return time end
local creates,destroys,closes=0,0,0;local options,draw={},{};local fail_draw=false
GUI_OPTION={NoPositionTween=6}
GuiCreate=function()creates=creates+1;return creates end;GuiDestroy=function()destroys=destroys+1 end
GuiStartFrame=function(gui)options[gui]=nil end;GuiOptionsAdd=function(gui,value)options[gui]=value end
GuiGetScreenDimensions=function()return 640,240 end;GuiTooltip=function()end;GuiImage=function()end
GuiText=function()end;GuiButton=function(_,_,_,y,label)draw[#draw+1]={y=y,label=label};return false end
GuiImageButton=function(gui,_,x,y,_,path)
 if fail_draw then error('injected optional GUI draw failure')end
 assert(y==0 and options[gui]==6,'packaged entry toolbar must retain legacy top-row immediate position')
 draw[#draw+1]={gui=gui,y=y,path=path};return false
end
NT2_FIXTURE.components[2].hp=4;NT2_FIXTURE.components[2].max_hp=8
local game={tick=function()end,close=function()end,positions=function()return {}end,entities=function()return {}end,set_visibility=function()end}
dofile=function(path)
 if path=='mods/noita_together2/files/g3/native_owner.lua'then return {new=function()return game end}end
 return raw(repo..'/'..path:gsub('^mods/','mod/'))
end
local entry=os.getenv('NT2_A2_ENTRY_FILE')or repo..'/mod/noita_together2/init-g3-seed-loot.lua';raw(entry)
local function set(fn,target,value,seen)
 seen=seen or {};if seen[fn]then return false end;seen[fn]=true;local found=false
 for i=1,60 do local name,old=debug.getupvalue(fn,i);if not name then break end
  if name==target then debug.setupvalue(fn,i,value);found=true elseif type(old)=='function'then found=set(old,target,value,seen)or found end
 end;return found
end
local checks=0;local function check(v,message)assert(v,message);checks=checks+1 end
check(set(OnWorldPreUpdate,'protected_update',function()end),'inject only protected native/inspection callback seam')
local ctx=ffi.new('uint8_t[16]');ffi.fill(ctx,16,1)
local bridge={nt2_g3_status=function()return 2 end,nt2_g3_close=function()closes=closes+1 end,
 nt2_g3_social_terminal=function()return 0 end,nt2_g3_social_publish=function()return 1 end,
 nt2_g3_social_lease=function(_,header)ffi.fill(header,64,0);ffi.fill(header+8,16,1);header[24]=1;header[28]=75;return 64 end,
 nt2_g3_social_next=function()return 0 end}
for key,value in pairs({context=ctx,bridge=bridge,player=99,accepted=true,loaded=true,gameplay=game})do check(set(OnWorldPreUpdate,key,value),'bind injected '..key)end
OnWorldPreUpdate();check(creates==1 and #draw==5,'actual entry creates one social GUI')
OnWorldPreUpdate();check(creates==1 and destroys==0,'ordinary frames retain same GUI')
fail_draw=true;OnWorldPreUpdate();check(destroys==1 and closes==0,'optional draw error closes GUI without closing native gameplay authority')
check(table.concat(logs,'\n'):find('g3_social_failed',1,true),'actual entry logs optional failure for full-logger diagnosis')
fail_draw=false;OnWorldPreUpdate();check(creates==2,'actual entry recreates GUI after optional error')
NT2_FIXTURE.components[1].enabled=false;check(set(OnWorldPreUpdate,'hold',{}),'inject already-protected state')
draw={};OnWorldPreUpdate();OnWorldPostUpdate();local icons=0;for _,w in ipairs(draw)do if w.path then icons=icons+1;check(w.y==0,'held toolbar stays at same fixed position')elseif w.label and w.label:find('Leave NT2',1,true)then check(w.y>=12,'existing protected action remains below top-row toolbar')end end
check(icons==4 and creates==3,'separate held-status GUI does not replace or transform toolbar GUI')
check(NT2_FIXTURE.components[1].enabled==false and closes==0,'GUI work cannot release protected controls or retire native owner')
set(OnWorldPreUpdate,'hold',nil);NT2_FIXTURE.components[1].enabled=true;OnWorldPreUpdate();check(creates==3,'return from held fact retains social GUI')
print=old_print;print('PASS packaged entry GUI lifetime '..checks..' assertions; optional fault/recreate and held overlay; injected engine/native facts')
