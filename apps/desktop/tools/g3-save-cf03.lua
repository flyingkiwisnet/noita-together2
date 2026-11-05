-- Original Lua, injected engine facts, actual unescaped XML attribute writer.
local repo=assert(os.getenv('NT2_CF03_REPO'));local dir=assert(os.getenv('NT2_CF03_DIR'))
local M=dofile(os.getenv('NT2_CF03_MODULE') or repo..'/mod/noita_together2/files/rejoin/local_world.lua')
local function read(name)local f=assert(io.open(dir..'/'..name,'rb'));local s=f:read('*a');f:close();return s end
local function write(name,s)local f=assert(io.open(dir..'/'..name,'wb'));f:write(s);f:close() end
local key='nt2_rejoin_local_world_v1';local record=read('record.txt');local globals={}
GameGetWorldStateEntity=function() return 42 end;StatsGetValue=function() return '424242' end
GlobalsGetValue=function(k,f) return globals[k] or f end;GlobalsSetValue=function(k,v) globals[k]=v end
SetWorldSeed=function()error('save/Continue must not reseed')end
local held,requests=false,0
local adapter={known=function(saved)assert(saved.record==record,'decoded bytes changed');return true end,hold_owned=function()held=true;return{}end,release_owned=function()held=false end,request=function(saved)assert(saved.record==record);requests=requests+1;return{}end,poll=function()return nil end,inspected=function()return false end,retire=function()error('no implicit offline')end}
if os.getenv('NT2_CF03_PHASE')=='save' or os.getenv('NT2_CF03_PHASE')=='old-save' then
 local m=M.new(adapter);assert(m:associate(record,'424242','shared:1:424242',true));if os.getenv('NT2_CF03_PHASE')=='old-save' then assert(globals[key]==record) else assert(M.decode_saved(globals[key])==record) end
 local xml='<WorldState><Globals>'
 for _,k in ipairs({key,key..'_seed',key..'_loot',key..'_offline'}) do xml=xml..'<E key="'..k..'" value="'..globals[k]..'"/>' end
 write(os.getenv('NT2_CF03_PHASE')=='old-save' and 'old-saved.xml' or 'saved.xml',xml..'</Globals></WorldState>');write('encoded.txt',globals[key]);print('CF03_SAVED decoded_bytes='..#record..' encoded_bytes='..#globals[key])
else
 globals[key]=read('restored.txt');globals[key..'_seed']='424242';globals[key..'_loot']='shared:1:424242';globals[key..'_offline']='0'
 local m=M.new(adapter);assert(m:loaded() and m.state=='choice' and held and requests==0);assert(m:rejoin() and requests==1 and held);assert(m:rejoin() and requests==1)
 assert(M.encode_saved(string.rep('x',1025))==nil)
 for _,bad in ipairs({'','raw JSON','nt2h2:1:78','nt2h1:0:','nt2h1:1:7','nt2h1:2:78','nt2h1:1:7878','nt2h1:01:78','nt2h1:1:gg','nt2h1:1:00','nt2h1:1025:'..string.rep('78',1025)}) do assert(M.decode_saved(bad)==nil,'malformed accepted: '..bad:sub(1,20)) end
 local encoded=M.encode_saved(record);assert(M.decode_saved(encoded:sub(1,-3))==nil,'even truncation accepted')
 print('CF03_LOADED exact_utf8=true choice_held=true single_consent=true max_and_malformed_bounds=true no_reseed=true')
end
