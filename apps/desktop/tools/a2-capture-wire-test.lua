-- Production capture and FFI encoder; engine observations and native copy result
-- are injected. This is not actual Noita capture or network evidence.
local repo=assert(os.getenv('NT2_A2_REPO'));local raw=dofile
dofile=function(path)return raw(repo..'/'..path:gsub('^mods/','mod/'))end
local ffi=require('ffi');local bodies={};local clock=38;local alive=true;local copy_ready=true
local input=dofile('mods/noita_together2/files/g3/a2_native_input.lua').new({nt2_g3_social_publish=function(_,bytes,size)if not copy_ready then return 0 end;bodies[#bodies+1]=ffi.string(bytes,size);return 1 end,nt2_g3_startup_nonce=function(bytes,size)ffi.fill(bytes,size,1);return 1 end},{},ffi)
local capture=dofile('mods/noita_together2/files/g3/a2_capture.lua')
local engine={GameGetRealWorldTimeSinceStarted=function()return clock end,EntityGetIsAlive=function(id)return id==165 and alive end,
 EntityHasTag=function()return false end,EntityGetFirstComponentIncludingDisabled=function(id,kind)assert(id==165 and kind=='DamageModelComponent');return 987 end,
 ComponentGetValue2=function(_,field)return field=='hp'and 3.9999997615814 or field=='max_hp'and 4 end,
 EntityGetTransform=function()return 227,-79.17 end,BiomeMapGetName=function()return '$biome_coalmine'end,
 EntityGetAllChildren=function(id)if id==165 then return {169}end end,EntityGetFilename=function()return ''end}
local publisher=capture.publisher(engine,function(value)return input:hud(value)end)
publisher:tick(165,true)
assert(#bodies==1 and bodies[1]:find('"hpKnown":true',1,true)and bodies[1]:find('"hpMilli":4000',1,true)and bodies[1]:find('"biome":"$biome_coalmine"',1,true),'live production capture must encode known information')
clock=38.5;publisher:tick(165,true);assert(#bodies==1,'unchanged capture throttled')
clock=39;publisher:tick(165,true);assert(#bodies==2,'unchanged live sample refreshed')
alive=false;clock=40;publisher:tick(165,true);assert(#bodies==3 and bodies[3]:find('"hpKnown":false',1,true)and not bodies[3]:find('"biome"',1,true),'unavailable capture publishes explicit unknown sample')
clock=41;publisher:tick(165,false);assert(#bodies==3,'protection suppresses publication')
assert(publisher.diagnostics.samples==4 and publisher.diagnostics.known==3 and publisher.diagnostics.unknown==1 and publisher.diagnostics.biome==3 and publisher.diagnostics.accepted==3,'capture counters distinguish sampled, throttled and unavailable facts')
copy_ready=false;assert(not input:hud({}),'occupied native slot refuses the replaceable HUD sample')
local queued=input:poi('literal private message',1,2);assert(queued and #bodies==3 and not input:dismiss('128'),'one explicit immutable intent can wait; another cannot replace it')
assert(not input:poi('different draft',3,4)and input.diagnostics.hud_unavailable==1 and input.diagnostics.poi_unavailable==1 and input.diagnostics.dismiss_unavailable==0 and input.diagnostics.native_error==0,'bounded local queue distinguishes a blocked gesture from an attempted native copy')
copy_ready=true;input:flush();assert(#bodies==4 and bodies[4]:find('literal private message',1,true)and not bodies[4]:find('different draft',1,true)and input.diagnostics.poi_accepted==1,'slot drain copies the same original explicit intent')
assert(input:poi('literal private message',1,2)==queued and #bodies==4,'repeated original does not make another native copy')
local output=assert(io.open(assert(os.getenv('NT2_A2_CAPTURE_OUTPUT')),'wb'));output:write(bodies[1]);output:close()
print('PASS production capture -> FFI input JSON; known, unchanged refresh, unknown and protected (engine/native observations injected)')
