local repo=assert(os.getenv('NT2_ITEM_REPO'));local output=assert(os.getenv('NT2_C1_SAVE_OUTPUT'))
NT2_C1_TEST_ENGINE_ONLY=true;local fixture=dofile(repo..'/apps/desktop/tools/c1-local-test.lua');NT2_C1_TEST_ENGINE_ONLY=nil
local xml=dofile(repo..'/apps/desktop/tools/noita-globals-xml.lua');local restricted=dofile(repo..'/apps/desktop/tools/c1-restricted-context.lua')
local checks=0;local function eq(a,b)assert(a==b,tostring(a)..' ~= '..tostring(b));checks=checks+1 end
local function write(name,value)local f=assert(io.open(output..'/'..name,'wb'));f:write(value);f:close()end
local e,c,entities,g,flags=fixture.engine();local json=e.dofile('mods/noita_together2/files/g3/bank_json.lua');local codec=e.dofile('mods/noita_together2/files/rejoin/local_world.lua')
local special='quote " apostrophe \' amp & less < greater > newline\n\r\t utf8 '..string.char(195,169)
local expected={text=special,world=string.rep('a',64),context=string.rep('01',16),queue={{eventId='11111111-1111-4111-8111-111111111111',senderDisposition='uncertain',payload={fungal={fromMaterials={'water','blood'},toMaterial='oil'}}}}}
local adjacent=assert(codec.encode_saved(assert(json.encode({local_world='11111111-1111-4111-8111-111111111111',scope=special}))))
local all={before='preserve-before',nt2_rejoin_local_world_v1=adjacent,nt2_rejoin_local_world_v1_seed='424242',nt2_rejoin_local_world_v1_loot='shared:1:424242',after='preserve-after'}
for _,key in ipairs({'authority','tokens','outgoing','checkpoint','effect_receipt','own_death','life'})do
 local saved=assert(json.encode_saved(expected));eq(saved:find('["&<>%z\1-\31]'),nil);all['nt2_c1_'..key]=saved
 eq(assert(json.decode_saved(saved)).text,special);eq(json.decode_saved(assert(json.encode(expected))),nil)
end
local bytes=xml.serialize(all);write('all-globals.xml',bytes);write('expected.json',assert(json.encode({values=all,payload=expected})))
local restored=assert(xml.restore(bytes));for k,v in pairs(all)do eq(restored[k],v)end
eq(codec.decode_saved(restored.nt2_rejoin_local_world_v1),codec.decode_saved(adjacent))
local broken=xml.serialize({before='ok',nt2_c1_authority=assert(json.encode(expected)),nt2_rejoin_local_world_v1=adjacent,after='ok'});write('raw-json-broken.xml',broken);eq(xml.restore(broken),nil)
for _,size in ipairs({1,1024,8192,131072})do local raw=string.rep('a',size);eq(codec.decode_saved(assert(codec.encode_saved(raw,size)),size),raw);eq(codec.encode_saved(raw..'x',size),nil)end
eq(codec.encode_saved(string.rep('a',1025)),nil);eq(codec.encode_saved('x',131073),nil);eq(codec.decode_saved('nt2h1:01:61'),nil);eq(codec.decode_saved('nt2h1:1:6A'),nil);eq(codec.decode_saved('nt2h1:2:61'),nil)
local big={value=string.rep('<&"',4096)};local saved=assert(json.encode_saved(big,{bytes=131072}));eq(json.decode_saved(saved),nil);eq(assert(json.decode_saved(saved,{bytes=131072})).value,big.value)
-- Real main/restricted producers -> raw XML -> fresh Lua owner. Existing local
-- HP remains, original ID survives uncertain restore, no second local effect.
local content=e.dofile('mods/noita_together2/files/g3/c1_content.lua').load(e);local module=e.dofile('mods/noita_together2/files/g3/c1_local.lua');local uid=0;local sent={}
local function create()return module.new(e,function()uid=uid+1;return string.format('%08d-1111-4111-8111-111111111111',uid)end,function(body)sent[#sent+1]=assert(json.decode(body));return true end,content)end
local view={world=string.rep('a',64),initialized=true,memberCount=2,snapshot={revision=1,rules=fixture.copy(fixture.rules)}}
local owner=create();owner.context=string.rep('01',16);owner:receive(view);owner:tick(true,1,100)
local h=restricted(e,function(path)if path:find('c1_catalogue_data.lua',1,true)then path='packages/contracts/g3-c1/catalogue.lua'else path=path:gsub('^mods/','mod/')end;local f=assert(io.open(repo..'/'..path,'rb'));local b=f:read('*a');f:close();return b end)
h.item_pickup=function()error('Expected real shared collector hook')end;h.dofile('mods/noita_together2/files/c1/append/heart_append.lua');h.item_pickup(2,1,'heart')
eq(c[11].max_hp,4.5);eq(c[11].hp,2);local original=assert(json.decode_saved(g.nt2_c1_outgoing)).queue[1];eq(original.senderDisposition,'applied')
g.nt2_rejoin_local_world_v1=adjacent;g.nt2_rejoin_local_world_v1_seed='424242';g.nt2_rejoin_local_world_v1_loot='shared:1:424242'
write('production-globals.xml',xml.serialize(g));local loaded=assert(xml.restore(xml.serialize(g)));for k in pairs(g)do g[k]=nil end;for k,v in pairs(loaded)do g[k]=v end
local cold=create();cold.context=string.rep('02',16);cold:receive(view);eq(cold.queue[1].eventId,original.eventId);eq(cold.queue[1].senderDisposition,'uncertain');eq(g.nt2_c1_tokens,'')
e.frame=101;cold:tick(false,1,101);eq(#sent,0);e.frame=102;cold:tick(true,1,102);eq(sent[1].publish.eventId,original.eventId);eq(sent[1].publish.senderDisposition,'uncertain');eq(c[11].max_hp,4.5);eq(c[11].hp,2);eq(g.nt2_rejoin_local_world_v1,adjacent)
for _,parts in pairs(entities)do for _,id in ipairs(parts.parts.VariableStorageComponent or{})do if c[id].value_string then eq(c[id].value_string:find('["&<>%z\1-\31]'),nil)end end end
print('PASS '..checks..' C1 raw-XML saved Globals/uncertain-original checks')
