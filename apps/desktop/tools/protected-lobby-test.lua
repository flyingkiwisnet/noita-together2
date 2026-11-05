local repo=assert(os.getenv('NT2_REJOIN_REPO'))
local display=dofile(repo..'/mod/noita_together2/files/rejoin/lobby_status.lua')
local recovery=dofile(repo..'/mod/noita_together2/files/rejoin/local_world.lua')
local checks=0
local function check(value)assert(value);checks=checks+1 end
for code=1,4 do
 local value=display.message({code=code,name='Test lobby'})
 check(value:find('Test lobby',1,true)~=nil)
 check((value:find('waiting for host Start',1,true)~=nil)==(code==2))
 check((value:find('waiting for connection',1,true)~=nil)==(code==3))
end
for _,value in ipairs({false,{},{code=9,name='old'},{code=1,name='bad\nname'},{code=2,name=string.rep('x',193)}})do
 check(display.message(value)=='No lobby connection. Open the NT2 app and join a lobby.')
end
check(display.message(nil):find('No lobby connection',1,true)~=nil)
check(display.read(nil,nil)==nil)
local rows={}
GuiText=function(_,_,_,text)rows[#rows+1]=text end
GuiButton=function()return false end
for _,code in ipairs({0,1,2,3,4})do
 rows={}
 local owner=recovery.new({display_status=function()return 2 end,lobby_status=function()return code>0 and {code=code,name='current'}or nil end})
 owner.state='inspecting';owner:draw(1)
 check((rows[1]:find('waiting for host Start',1,true)~=nil)==(code==2))
 if code==4 then check(rows[1]:find('host Start confirmed',1,true)~=nil)end
end
check(loadfile(repo..'/mod/noita_together2/init-g3-seed-loot.lua')~=nil)
local diagnostic={code=5,name='App and service versions differ; use matching release; see app details.'}
check(display.compatibility(diagnostic).title=='NT2 versions do not match')
check(display.message(diagnostic):find('App and service',1,true)~=nil)
check(display.compatibility(nil)==nil and display.compatibility({code=2,name='New lobby'})==nil)
check(display.message(nil,'no_app'):find('No NT2 app connection',1,true)~=nil)
check(display.message(nil,'unavailable'):find('not validated',1,true)~=nil)
check(not display.message(nil,'unavailable'):find('versions do not match',1,true))
local state='inspecting';rows={};local protected=recovery.new({compatibility_status=function()return display.compatibility(diagnostic)end})
protected.state=state;protected:draw(1);check(rows[1]:find('NT2 versions do not match',1,true)~=nil);check(protected.state==state)
print('PASS '..checks..' protected lobby messages, saved-run Start suppression, and active entry syntax')
