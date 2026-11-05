-- Production copy adapter, injected DLL occupancy. Native/pipe is tested separately.
local repo=assert(os.getenv('NT2_A3_REPO'));local ffi=require('ffi');local raw=dofile
dofile=function(p)local prior=os.getenv('NT2_A3_INPUT_FILE');if prior and prior~=''and p=='mods/noita_together2/files/g3/a2_native_input.lua'then return raw(prior)end;return raw(repo..'/'..p:gsub('^mods/','mod/'))end
local calls,copies,nonce=0,{},0;local available=false;local checks=0
local function check(v,s)assert(v,s);checks=checks+1 end
local input=dofile('mods/noita_together2/files/g3/a2_native_input.lua').new({
 nt2_g3_startup_nonce=function(b,n)nonce=nonce+1;ffi.fill(b,n,nonce);return 1 end,
 nt2_g3_social_publish=function(_,b,n)calls=calls+1;if not available then return 0 end;copies[#copies+1]=ffi.string(b,n);return 1 end
},{},ffi)
local sample={hp=4,max_hp=8};local id=input:poi('one click',123,456)
check(id and #id==32,'a single explicit click waits behind an occupied HUD slot')
for _=1,600 do
 input:flush();check(not input:hud(sample),'HUD cannot take the reserved explicit slot')
 check(not input:social('emote',string.rep('a',32),{emoteId='wave',active=true}),'no growing second explicit queue')
end
check(nonce==1 and #copies==0,'waiting neither remints an ID nor claims a native copy')
available=true;input:flush();input:flush()
check(#copies==1 and copies[1]:find('"id":"'..id..'"',1,true),'eventually copies exactly the original identity once')
check(input:hud(sample),'ordinary HUD resumes after explicit copy')
available=false;local emote=input:identity();check(input:social('emote',emote,{emoteId='heart',active=true}),'one emote click waits')
input:pause();available=true;input:set_ready(true);input:flush();check(#copies==2,'unsent protected Start is discarded')
input:set_ready(false);check(input:social('emote',emote,{emoteId='heart',active=false}),'Stop can wait through a lease gap')
input:flush();check(#copies==2,'queued action cannot bypass missing copy permission')
input:set_ready(true);input:flush();check(#copies==3 and copies[3]:find('"active":false',1,true),'same Stop copies after permission returns')
input:set_ready(false);check(input:poi('old scope',0,0),'one scoped intent retained');input:close();input:set_ready(true);input:flush();check(#copies==3,'scope close discards unsent old intent')
check(not input:poi(string.rep('x',4096),0,0),'oversize message never enters queue')
available=false;local start=input:identity();local stop=input:identity()
check(input:social('emote',start,{emoteId='heart',active=true}),'finite Start may wait')
check(input:social('emote',stop,{emoteId='heart',active=false}),'completion replaces only a known unsent Start')
available=true;input:flush();check(#copies==4 and copies[4]:find('"id":"'..stop..'"',1,true)and copies[4]:find('"active":false',1,true),'completed unsent animation is never replayed late')
local accepted,copied=input:social('emote',stop,{emoteId='heart',active=false});check(accepted and copied and #copies==4,'copied Stop receipt prevents a duplicate retry copy')
print('PASS A3 input contention '..checks..' assertions;600 occupied callbacks, one stable original; DLL occupancy injected')
