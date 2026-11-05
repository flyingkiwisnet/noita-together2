-- Actual Rust frame encoder -> actual Lua dispatcher; no pipe/service mock.
local repo=assert(os.getenv('NT2_A3_REPO'));local raw=dofile;dofile=function(path)return raw(repo..'/'..path:gsub('^mods/','mod/'))end
local root='mods/noita_together2/files/g3/';local module=dofile(root..'social_dispatch.lua');local catalogue=dofile(root..'social_catalogue.lua');local checks=0;local count=0;local applied=0
local function check(v,s)assert(v,s);checks=checks+1 end
local gate={context=string.rep('\1',16),generation=1,owner='current',now=20,until_time=21,playing=true}
local dispatch=module.new(function()return gate end,function(commands)applied=applied+#commands;return true end,function()end)
for line in io.lines(repo..'/spikes/noita-entry/.build/a3-client-20261022-01/wire.frames')do
 local kind,index,hex=line:match('^(%a+) (%d+) (%x+)$');index=tonumber(index);local bytes=hex:gsub('%x%x',function(x)return string.char(tonumber(x,16))end);local batch=module.decode(bytes);check(batch and #batch.commands==1,'Rust native frame decoded');local c=batch.commands[1]
 if kind=='emote'then check(c.tag==8 and c.id==index and c.active and c.duration==catalogue.entries[catalogue.ids[index+1]].frames and c.age_ms==1300,'emote exact code/duration/relative queued age')
 elseif kind=='cosmetic'then check(c.tag==9 and c.robe==math.floor(index/8)and c.hat==(index%2==1)and c.amulet==(math.floor(index/2)%2==1)and c.gem==(math.floor(index/4)%2==1),'six robes/all flag bits')
 else check(c.tag==15 and c.status==index and #c.id==32 and #c.event==36,'bounded original outcome')end
 check(dispatch:receive(bytes),'current frame applied');local prior=applied;local ok,reason=dispatch:receive(bytes);check(not ok and reason=='replay'and applied==prior,'same native sequence never reapplied');check(not module.decode(bytes..'x'),'tail rejected');check(not module.decode(bytes:sub(1,-2)),'truncation rejected');count=count+1
end
check(count==71 and applied==71,'complete encoder/decoder catalogue');gate.playing=false;check(not dispatch:receive('invalid'),'protected authority never applies packet')
local ffi=require('ffi');local copies={};local available=true
local outgoing=dofile(root..'a2_native_input.lua').new({nt2_g3_startup_nonce=function(buffer,size)ffi.fill(buffer,size,1);return 1 end,nt2_g3_social_publish=function(_,buffer,size)if not available then return 0 end;copies[#copies+1]=ffi.string(buffer,size);return 1 end},{},ffi)
local identity=outgoing:identity();check(identity==string.rep('01',16),'real CSPRNG-copy identity encoding')
for _,id in ipairs(catalogue.ids)do for _,active in ipairs({true,false})do check(outgoing:social('emote',identity,{emoteId=id,active=active}),'real outgoing emote copy');check(copies[#copies]=='{"kind":"emote","id":"'..identity..'","emote":{"emoteId":"'..id..'","active":'..tostring(active)..'}}','actual Lua JSON matches Rust intent fixture shape')end end
for _,robe in ipairs(catalogue.robes)do check(outgoing:social('cosmetic',identity,{robeId=robe,hat=true,amulet=false,gem=true}),'real outgoing cosmetic copy');check(copies[#copies]=='{"kind":"cosmetic","id":"'..identity..'","cosmetic":{"robeId":"'..robe..'","hat":true,"amulet":false,"gem":true}}','actual flags/robe JSON shape')end
available=false;local before=#copies;check(outgoing:social('emote',identity,{emoteId='wave',active=false}),'occupied native slot retains one explicit original');check(outgoing.diagnostics.emote_unavailable==1 and #copies==before,'bounded outgoing diagnostics distinguish waiting from copying');available=true;outgoing:flush();outgoing:flush();check(#copies==before+1,'deferred wire original copied once')
print('PASS A3 wire '..checks..' Rust-to-Lua frame assertions; no actual Noita rendering')
