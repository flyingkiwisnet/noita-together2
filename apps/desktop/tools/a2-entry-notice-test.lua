-- Actual entry callback wiring; Noita getters and pending renderer injected.
local repo=assert(os.getenv('NT2_A2_REPO'));local raw=dofile
dofile=function(path)return raw(repo..'/'..path:gsub('^mods/','mod/'))end
raw(repo..'/apps/desktop/tools/mod-protection-stubs.lua')
ModLuaFileAppend=function()end;ModLuaFileGetAppends=function()return {}end
local entry=os.getenv('NT2_A2_ENTRY_FILE');if not entry or entry==''then entry=repo..'/mod/noita_together2/init-g3-seed-loot.lua'end;raw(entry)
local calls=0;local found=false
for index=1,30 do local name=debug.getupvalue(OnWorldPostUpdate,index);if not name then break end;if name=='social_gameplay'then debug.setupvalue(OnWorldPostUpdate,index,{post_update=function()calls=calls+1 end});found=true;break end end
assert(found,'actual entry must flush notices in post-update even when not held')
OnWorldPostUpdate();assert(calls==1,'actual post-update flush skipped or duplicated')
print('PASS production OnWorldPostUpdate reaches notice delivery before protected-UI early return; engine facts injected')
