-- Actual Win32 private-file transport. No game, service or gameplay outcomes.
local repo=assert(os.getenv('NT2_A3_REPO'));local dir=assert(os.getenv('NT2_A3_FILE_DIR'));local raw=dofile;local ffi=require('ffi')
local clock=raw(repo..'/apps/desktop/tools/lifecycle-lc1-clock.lua').new()
local files=raw(repo..'/apps/desktop/tools/lifecycle-lc1-files.lua').new(dir)
local checks=0;local function check(v,s)assert(v,s);checks=checks+1 end
check(not files:exists('a3-menu'),'missing held signal');files:mark('a3-menu','held');check(files:input('a3-menu',false)=='held','complete held signal')
local path=dir..'/a3-menu';local handle=ffi.C.CreateFileA(path,0x80000000,7,nil,3,0,nil);check(handle~=ffi.cast('void*',-1),'shared reader opens');check(ffi.C.DeleteFileA(path)~=0,'shared reader permits simultaneous deletion');check(ffi.C.CloseHandle(handle)~=0,'owned reader closes');check(not files:exists('a3-menu'),'one logical release')
for i=1,30 do local value='color_red:'..i;files:mark('a3-select',value);check(files:input('a3-select',false)==value,'atomic replacement preserves complete value');check(files:input('a3-select',true)==value and not files:exists('a3-select'),'consume removes one logical selection')end
files:mark('a3-view.json','{"frame":1}');files:mark('a3-view.json','{"frame":2}');check(files:input('a3-view.json',true)=='{"frame":2}','atomic snapshot replacement')
check(not pcall(function()files:mark('../outside','bad')end),'nonlocal basename rejected');check(not pcall(function()files:mark('missing/child','bad')end),'nested signal rejected')
clock.close();print('PASS A3 private Win32 files '..checks..' assertions; complete publication, shared deletion, one consumption and path errors')
