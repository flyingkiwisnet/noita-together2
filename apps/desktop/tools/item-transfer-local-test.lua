-- Production local capture/descriptor/presentation code; Noita component and GUI
-- observations below are injected. This does not establish loaded-save safety.
local repo=assert(os.getenv('NT2_ITEM_REPO'));local raw=dofile
dofile=function(path)if path:find('item_catalogue_data.lua',1,true)then return raw(repo..'/packages/contracts/g3-items/catalogue.lua')end;return raw(repo..'/'..path:gsub('^mods/','mod/'))end
local root='mods/noita_together2/files/g3/'
local checks=0;local function check(value,why)assert(value,why);checks=checks+1 end
local function equal(a,b,why)check(a==b,why or (tostring(a)..' ~= '..tostring(b)))end
local gate=raw(repo..'/apps/desktop/tools/lifecycle-lc1-b1.lua').item_copy_gate();local fault={kind='item_chunk',channel='deposit',id='next'}
local hit=raw(repo..'/apps/desktop/tools/lifecycle-lc1-b1.lua').item_cell_hit;local pixel={x=228,y=160,label='   '};check(not hit(pixel,200,160,'   '),'A dimmed earlier cell cannot consume another cell pixel gesture');check(hit(pixel,228,160,'   '));check(not hit(pixel,228,160,'Close'))
check(not gate(fault,{kind='item_chunk',channel='inventory',id='inventory-1'}),'Inventory cannot bind or be blocked by deposit fault')
check(gate(fault,{kind='item_chunk',channel='deposit',id='original-a'}),'First deposit binds the private fault')
check(not gate(fault,{kind='item_chunk',channel='deposit',id='original-b'}),'Other deposit original progresses')
check(not gate(fault,{kind='item_chunk',channel='inventory',id='original-a'}),'Matching ID on another channel still progresses')
check(gate(fault,{kind='item_chunk',channel='deposit',id='original-a'}),'Only original retries remain blocked')
check(not gate(nil,{kind='item_chunk',channel='deposit',id='original-a'}),'Disarming restores deposit traffic')
local result_fault={kind='item_result',id='claim-a'};check(not gate(result_fault,{kind='item_result',id='claim-b'}),'Unrelated result progresses');check(gate(result_fault,{kind='item_result',id='claim-a'}),'Ambiguous result fault matches exact claim');check(not gate(result_fault,{kind='item_copied',id='claim-a'}),'Output copy acknowledgements progress')
local catalogue={revision='nt2-items-base-2',spells={LIGHT_BULLET={name='Spark Bolt',category=0},BOMB={name='Bomb',category=0}},sprites={test_wand={path='data/items_gfx/wands/wand_0000.png'}},sprite_ids={['data/items_gfx/wands/wand_0000.png']='test_wand'},materials={water={name='Water'},oil={name='Oil'}},objects={disc={path='data/entities/items/pickup/runestones/runestone_disc.xml',name='Runestone of edges'}},object_ids={['data/entities/items/pickup/runestones/runestone_disc.xml']='disc'}}
local parts={
 [101]={mana_max=10000,mana_charge_speed=35.5,ui_name='$item_wand',sprite_file='data/items_gfx/wands/wand_0000.png',gun_config={reload_time=-10,actions_per_round=2,deck_capacity=121,shuffle_deck_when_empty=false},gunaction_config={spread_degrees=-4.5,speed_multiplier=1.25,fire_rate_wait=-2.25}},
 [102]={always_use_item_name_in_ui=true,item_name='Literal "wand" 雪',ui_sprite=''},[103]={offset_x=-3.25,offset_y=5},[104]={offset={21.5,-5}},[105]={},
 [211]={uses_remaining=-1,permanently_attached=true,inventory_slot={0,0}},[212]={action_id='LIGHT_BULLET'},
 [221]={uses_remaining=2,permanently_attached=false,inventory_slot={3,0}},[222]={action_id='BOMB'},
 [231]={uses_remaining=0,permanently_attached=false,inventory_slot={0,0}},[232]={action_id='LIGHT_BULLET'},
 [301]={count_per_material_type={[1]=0,[2]=450.25,[3]=200}},[302]={item_name='$item_potion'},[303]={},[401]={},[402]={},
}
local entities={
 [1]={x=0,y=0,parts={},tags={}},[10]={x=23,y=0,parts={AbilityComponent=101,ItemComponent=102,SpriteComponent=103,HotspotComponent=104,SimplePhysicsComponent=105},tags={wand=true},children={22,21,23}},
 [21]={parent=10,parts={ItemComponent=211,ItemActionComponent=212},tags={card_action=true}},[22]={parent=10,parts={ItemComponent=221,ItemActionComponent=222},tags={card_action=true}},[23]={parent=10,parts={ItemComponent=231,ItemActionComponent=232},tags={card_action=true}},
 [30]={x=1,y=2,parts={MaterialInventoryComponent=301,ItemComponent=302,PhysicsBodyComponent=303},tags={item_physics=true},path='data/entities/items/pickup/potion.xml'},
 [40]={x=1,y=1,parts={PhysicsBodyComponent=401,ItemComponent=402},tags={item_physics=true},path=catalogue.objects.disc.path},
}
local writes=0
local e={EntityGetIsAlive=function(id)return entities[id]~=nil end,EntityHasTag=function(id,tag)return entities[id].tags[tag]==true end,
 EntityGetParent=function(id)return entities[id].parent or 0 end,EntityGetRootEntity=function(id)return entities[id].parent or id end,EntityGetTransform=function(id)return entities[id].x or 0,entities[id].y or 0 end,
 EntityGetFirstComponentIncludingDisabled=function(...)
  local n=select('#',...);local id,kind,tag=...
  assert(n==2 or (n==3 and type(tag)=='string'),'param 3 was not a string')
  if n==3 then assert(kind=='HotspotComponent'and tag=='shoot_pos','Exact tagged hotspot lookup')end
  return entities[id].parts[kind]
 end,EntityGetAllChildren=function(id)return entities[id].children end,EntityGetFilename=function(id)return entities[id].path end,
 ComponentGetValue2=function(id,key)local value=parts[id][key];if type(value)=='table'and key~='count_per_material_type'then return unpack(value)end;return value end,
 ComponentObjectGetValue2=function(id,object,key)return parts[id][object][key]end,CellFactory_GetName=function(id)return ({[0]='air',[1]='water',[2]='oil'})[id]end,CellFactory_GetType=function(name)return ({air=0,water=1,oil=2})[name]end,
 GameGetPotionColorUint=function()return 4294967295 end,EntityGetInRadiusWithTag=function(_,_,_,tag)local ids={};for id,entity in pairs(entities)do if entity.tags[tag]then ids[#ids+1]=id end end;return ids end,
 EntityKill=function()writes=writes+1 end,ComponentSetValue2=function()writes=writes+1 end}
check(not pcall(e.EntityGetFirstComponentIncludingDisabled,10,'ItemComponent',nil),'Engine stub rejects explicit nil optional tag')
local api=dofile(root..'item_game_api.lua').new(e)
local descriptors=dofile(root..'item_descriptor.lua');local codec=descriptors.new(catalogue)
local wand,plan=codec:capture(api,e,10);equal(wand.wand.deckCapacity,'120');equal(wand.wand.deck[1].slot,0);equal(wand.wand.deck[1].spell.usesRemaining,'0');equal(wand.wand.deck[2].slot,3);equal(wand.wand.permanent[1].spell.usesRemaining,'-1');equal(plan.stats.fireRateWait,-2.25);equal(plan.stats.spreadDegrees,-4.5);equal(plan.name,'Literal "wand" 雪');equal(plan.stats.manaMax,10000)
for _,uses in ipairs({-1,0,9})do parts[231].uses_remaining=uses;local item=codec:capture(api,e,23);equal(item.spell.usesRemaining,tostring(uses));end;parts[231].uses_remaining=0
local container=codec:capture(api,e,30);equal(#container.container.contents,2);equal(container.container.contents[1].materialId,'oil');equal(container.container.contents[2].materialId,'water');equal(container.container.contents[2].amount,'450.25');equal(container.container.color,'4294967295')
parts[301].count_per_material_type={};parts[302].item_name='$item_powder_stash_3';local empty=codec:capture(api,e,30);equal(empty.container.containerType,'powder');equal(#empty.container.contents,0)
parts[301].count_per_material_type={[2]=1.125};parts[302].item_name='$item_potion'
local object=codec:capture(api,e,40);equal(object.object.objectId,'disc');equal(codec:resolve(object).object.path,catalogue.objects.disc.path)
check(api:dropped(10,1),'Dropped world wand accepted');entities[10].x=24;check(not api:dropped(10,1),'Exact24 boundary refused');entities[10].x=23;entities[10].parent=99;check(not api:dropped(10,1),'Held/parented wand refused');entities[10].parent=nil
local snapshot=api:dropped(10,1);entities[10].parts.ItemComponent=999;check(not api:revalidate(snapshot),'Replaced item component refused');entities[10].parts.ItemComponent=102;check(api:revalidate(snapshot),'Current exact components retained')
entities[10].parts.AbilityComponent=998;check(not api:revalidate(snapshot),'Replaced ability identity rejected even before matching-content read');entities[10].parts.AbilityComponent=101
local found=api:candidates(1);local seen={};for _,id in ipairs(found)do check(not seen[id],'Overlapping inlet tags deduplicated');seen[id]=true end
for _,mutate in ipairs({function()parts[222].action_id='UNKNOWN'end,function()parts[101].gunaction_config.spread_degrees=0/0 end,function()parts[101].sprite_file='../../untrusted.xml'end,function()parts[221].inventory_slot={0,0}end,function()parts[221].inventory_slot={3,1}end})do
 mutate();check(not pcall(codec.capture,codec,api,e,10),'Invalid whole wand refused before mutation');parts[222].action_id='BOMB';parts[101].gunaction_config.spread_degrees=-4.5;parts[101].sprite_file='data/items_gfx/wands/wand_0000.png';parts[221].inventory_slot={3,0}
end
local bads={{catalogueRevision=catalogue.revision,object={objectId='../../bad.xml'}},{catalogueRevision=catalogue.revision,spell={actionId='LIGHT_BULLET',usesRemaining='-2'}},{catalogueRevision=catalogue.revision,container={containerType='potion',color='4294967295',contents={{materialId='water',amount='0'}}}},{catalogueRevision=catalogue.revision,container={containerType='powder',color='1',contents={{materialId='water',amount='1'},{materialId='water',amount='2'}}}}}
for _,bad in ipairs(bads)do check(not codec:resolve(bad),'Malformed/unsupported descriptor refused')end
equal(writes,0,'Capture/validation never removes or mutates an original')
local model_module=dofile(root..'item_bank_model.lua');local model=model_module.new();model:bind('scope')
for _,size in ipairs({0,1,39,40,41,80})do local rows={};for i=1,size do rows[i]={id=string.format('%04d',i),kind='spell',name='Bolt',category=0,revision=1}end;check(model:replace('scope',1,rows));local page,pages,total=model:visible();equal(pages,math.max(1,math.ceil(size/40)));equal(total,size);equal(#page,math.min(40,size));model:move(10);equal(model.page,pages-1);model:move(-10);equal(model.page,0)end
local rows={{id='one',kind='spell',name='Spark [Bolt] 100%',category=0},{id='two',kind='object',name='Stone',search_text='Runestone .^%'},{id='wand-a',kind='wand',name='A',search_text='Wand Bomb',stats={shuffleDeckWhenEmpty=false,actionsPerRound=2,fireRateWait=-10,reloadTime=3}},{id='wand-b',kind='wand',name='B',search_text='Wand Spark Bolt',stats={shuffleDeckWhenEmpty=true,actionsPerRound=1,fireRateWait=-20,reloadTime=1}}}
check(model:replace('scope',2,rows));for _,query in ipairs({'[BOLT]','100%','.^%'})do check(model:edit(query));local _,_,total=model:visible();equal(total,1,'Literal case-insensitive search across kinds')end
model:edit('');model:set_tab('wands');local page=model:visible();equal(page[1].id,'wand-a','Fixed criterion priority: no shuffle first');model:toggle('wands',2,true);page=model:visible();equal(page[1].id,'wand-b','Right click solos one wand criterion');for i=1,8 do equal(model.wand_filters[i],i==2)end
model:set_tab('spells');model:toggle('spells',2,true);local _,_,total=model:visible();equal(total,0);model:toggle('spells',1,false);_,_,total=model:visible();equal(total,1)
model:set_tab('all');model:removed('one',3);check(model:replace('scope',3,rows));page=model:visible();local removed=false;for _,row in ipairs(page)do if row.id=='one'then removed=row.removed end end;check(removed,'Late available row cannot resurrect claimed original');check(not model:replace('scope',2,rows),'Old bank revision refused');model:bind('next-world');equal(#model.rows,0)
local json=dofile(root..'bank_json.lua');local chunks=dofile(root..'item_chunks.lua')
local presenter=dofile(root..'item_presenter.lua');equal(presenter.icon(catalogue,{kind='container',iconId='powder'}),'data/ui_gfx/items/material_pouch.png');local pouch={catalogueRevision=catalogue.revision,container={containerType='powder',color='1',contents={{materialId='water',amount='1500'}}}};check(table.concat(presenter.lines(e,codec,pouch),'\n'):find('100%%'),'Powder percentage retains legacy1500-unit display basis')
catalogue.sprites.fixed_xml={path='data/items_gfx/fixed.xml'};local reads=0;local images={};local sprite_engine={ModTextFileGetContent=function()reads=reads+1;return '<Sprite filename="data/items_gfx/wands/wand_0000.png" />'end};equal(presenter.gui_sprite(sprite_engine,catalogue,{iconId='fixed_xml'},images),'data/items_gfx/wands/wand_0000.png');presenter.gui_sprite(sprite_engine,catalogue,{iconId='fixed_xml'},images);equal(reads,1,'Fixed XML image resolution cached');sprite_engine.ModTextFileGetContent=function()return '<Sprite filename="../../bad.png" />'end;equal(presenter.gui_sprite(sprite_engine,catalogue,{iconId='fixed_xml'},{}),'data/items_gfx/fixed.xml','Untrusted XML-derived paths cannot enter the GUI')
for _,name in ipairs({'item_bridge.lua','item_local.lua','item_construct.lua','item_presenter.lua'})do check(loadfile(repo..'/mod/noita_together2/files/g3/'..name),'Production module syntax '..name)end
local real=dofile(root..'item_catalogue.lua');local object_count,available=0,0;for _,entry in pairs(real.objects)do object_count=object_count+1;if entry.available then available=available+1 end end;equal(object_count,43);equal(available,40)
local encoded=assert(json.encode(wand));check(encoded:find('"deck":%['));check(json.decode(encoded,{bytes=131072,nodes=32768,depth=16}));check(not json.decode('{"x":null,"x":1}'),'Duplicate null key rejected');equal(#json.decode('[1,null,3]'),3,'Array null is retained for descriptor rejection')
local payload=string.rep('雪%[]',2000);local sender=assert(chunks.sender('copy','deposit',payload));local receiver=chunks.receiver();local complete
local stream=chunks.receiver();for i=1,20 do local c=assert(chunks.sender('stream-'..i,'players','[]')):chunk();local value,state=stream:accept(c);equal(state,'complete','Completed replay copies cannot block the fifth transfer');equal(value,'[]');check(stream.count<=4 and #stream.completed<=4);local _,again=stream:accept(c);equal(again,'replay')end
local partials=chunks.receiver();for i=1,4 do local _,state=partials:accept({id='partial-'..i,channel='detail',offset=0,total=2,data='eA=='});equal(state,'pending')end;local _,busy=partials:accept({id='fifth',channel='detail',offset=0,total=2,data='eA=='});equal(busy,'busy','Incomplete descriptors cannot be evicted');local done=partials:accept({id='partial-1',channel='detail',offset=1,total=2,data='eQ=='});equal(done,'xy');local _,admitted=partials:accept({id='fifth',channel='detail',offset=0,total=2,data='eA=='});equal(admitted,'pending','Completed slot becomes reusable');check(partials.entries['detail:partial-2']~=nil);equal(partials.count,4)
while sender:chunk()do local c=sender:chunk();local result,state=receiver:accept(c);check(state=='pending'or state=='complete');local again,replay=receiver:accept(c);equal(replay,'replay');if result then complete=result;equal(again,result)end;check(sender:copied(c))end;equal(complete,payload)
check(not chunks.unbase64('Zh=='),'Noncanonical base64 pad bits refused');check(not chunks.unbase64('===='));local bad={id='gap',channel='deposit',offset=1,total=2,data='eA=='};local _,state=receiver:accept(bad);equal(state,'busy')
-- Mutation fixture exercises the production constructor/readback and guard.
local next_id,next_part=1000,10000;local globals={};local killed=0
local function entity(path,x,y,wand_type)
 next_id=next_id+1;local id=next_id;local row={x=x,y=y,path=path,parts={},tags=wand_type and {wand=true}or {item_physics=true},children={}};entities[id]=row
 local function add(kind,value)next_part=next_part+1;parts[next_part]=value;row.parts[kind]=next_part;return next_part end
 add('ItemComponent',{is_pickable=true,always_use_item_name_in_ui=false,item_name=path:find('powder',1,true)and '$item_powder_stash_3'or '',ui_sprite='',uses_remaining=-1,inventory_slot={0,0},permanently_attached=false});add('SimplePhysicsComponent',{})
 if wand_type then add('AbilityComponent',{gun_config={},gunaction_config={}});add('SpriteComponent',{});add('HotspotComponent',{})
 elseif path:find('potion',1,true)or path:find('powder',1,true)then add('MaterialInventoryComponent',{count_per_material_type={[2]=999}})end
 return id
end
e.EntityLoad=function(path,x,y)if path:find('poof_',1,true)then return entity(path,x,y,false)end;return entity(path,x,y,path:find('item_wand.xml',1,true)~=nil)end
e.CreateItemActionEntity=function(action,x,y)local id=entity('action',x or 0,y or 0,false);local row=entities[id];row.tags={card_action=true};next_part=next_part+1;row.parts.ItemActionComponent=next_part;parts[next_part]={action_id=action};return id end
e.ComponentSetValue2=function(c,key,...)local value={...};if #value==1 then parts[c][key]=value[1]else parts[c][key]=value end;writes=writes+1 end
e.ComponentObjectSetValue2=function(c,object,key,value)parts[c][object][key]=value;writes=writes+1 end
e.EntityAddChild=function(parent,child)entities[child].parent=parent;table.insert(entities[parent].children,child)end
e.EntitySetComponentsWithTagEnabled=function()end;e.EntityRefreshSprite=function()end
e.RemoveMaterialInventoryMaterial=function(id)parts[entities[id].parts.MaterialInventoryComponent].count_per_material_type={}end
e.AddMaterialInventoryMaterial=function(id,name,amount)local c=parts[entities[id].parts.MaterialInventoryComponent];c.count_per_material_type[e.CellFactory_GetType(name)+1]=amount end
e.GlobalsGetValue=function(key,fallback)return globals[key]or fallback end;e.GlobalsSetValue=function(key,value)globals[key]=value end
e.EntityKill=function(id)killed=killed+1;entities[id]=nil end
local constructor=dofile(root..'item_construct.lua').new(e,codec)
for _,item in ipairs({wand,container,empty,object,{catalogueRevision=catalogue.revision,spell={actionId='BOMB',usesRemaining='0'}}})do
 local job=assert(constructor:start(item,0,0));equal(job:step(false),'deferred');check(not job.entity,'No construction under protection')
 local result=job:step(true);while result=='constructing'do result=job:step(true)end;equal(result,'applied_session','Every supported kind roundtrips')
 local again=codec:capture(api,e,job.entity);equal(json.encode(again),json.encode(item),'Descriptor readback keeps charges,slots,mixtures and name');local id=job.entity;equal(job:step(true),'applied_session');equal(job.entity,id,'Duplicate job callback does not spawn')
end
-- The server-approved base wand roots were absent from the old sprite namespace.
-- Capture uses ui_sprite first, otherwise ability.sprite_file, as legacy does.
local actual_codec=descriptors.new(real);local actual_constructor=dofile(root..'item_construct.lua').new(e,actual_codec)
local roots={
 {'data/items_gfx/handgun.xml','data/items_gfx/handgun.png'},
 {'data/items_gfx/bomb_wand.xml','data/items_gfx/bomb_wand.png'},
 {'data/items_gfx/flute.xml','data/items_gfx/flute.png'},
 {'data/items_gfx/kantele.xml','data/items_gfx/kantele.png'},
 {'data/items_gfx/machinegun.xml','data/items_gfx/machinegun.png'},
 {'data/entities/items/wands/experimental/experimental_wand_1_sprite.xml','data/entities/items/wands/experimental/experimental_wand_1.png'},
 {'data/entities/items/wands/experimental/experimental_wand_2_sprite.xml','data/entities/items/wands/experimental/experimental_wand_2.png'}}
local saved_ui,saved_ability=parts[102].ui_sprite,parts[101].sprite_file
for _,pair in ipairs(roots)do for _,path in ipairs(pair)do
 check(real.sprite_ids[path]~=nil,'Reviewed base wand and fixed image are registered')
 parts[102].ui_sprite='';parts[101].sprite_file=path;local before_writes=writes
 local source=actual_codec:capture(api,e,10);equal(writes,before_writes,'Capture never changes the original');equal(source.catalogueRevision,real.revision);equal(source.wand.spriteId,real.sprite_ids[path])
 local job=assert(actual_constructor:start(source,1,1));local state=job:step(true);while state=='constructing'do state=job:step(true)end;equal(state,'applied_session')
 local recaptured=actual_codec:capture(api,e,job.entity);equal(json.encode(recaptured),json.encode(source),'Base wand capture/construct/recapture is faithful')
 local ability=entities[job.entity].parts.AbilityComponent;local item=entities[job.entity].parts.ItemComponent;local sprite=entities[job.entity].parts.SpriteComponent
 equal(parts[ability].sprite_file,path);equal(parts[sprite].image_file,path);equal(parts[item].ui_sprite,'','Template empty UI sprite falls back to reconstructed fixed ability sprite')
 end
 local xml=pair[1];local png=pair[2];local cache={};local local_engine={ModTextFileGetContent=function(path)equal(path,xml);return '<Sprite filename="'..png..'" />'end}
 equal(presenter.gui_sprite(local_engine,real,{iconId=real.sprite_ids[xml]},cache),png,'Only registered fixed XML image dependency reaches GuiImage')
end
parts[102].ui_sprite='data/items_gfx/handgun.png';parts[101].sprite_file='mods/untrusted/ability.xml';equal(actual_codec:capture(api,e,10).wand.spriteId,real.sprite_ids['data/items_gfx/handgun.png'],'Explicit registered UI sprite has legacy precedence')
parts[102].ui_sprite='mods/untrusted/ui.xml';parts[101].sprite_file='data/items_gfx/handgun.xml';check(not pcall(actual_codec.capture,actual_codec,api,e,10),'An unknown UI sprite cannot be laundered through a known ability fallback');equal(api:sprite_diagnostic(10),'sprite_ui_mod_path')
parts[102].ui_sprite='';parts[101].sprite_file='data/items_gfx/not_registered.xml';check(not pcall(actual_codec.capture,actual_codec,api,e,10));equal(api:sprite_diagnostic(10),'sprite_ability_base_xml')
parts[102].ui_sprite='../../PRIVATE_PATH.xml';equal(api:sprite_diagnostic(10),'sprite_ui_other','Diagnostic category contains no arbitrary path')
parts[102].ui_sprite=saved_ui;parts[101].sprite_file=saved_ability
globals.PERK_NO_MORE_SHUFFLE_WANDS='1';local shuffled=json.decode(json.encode(wand),{bytes=131072,nodes=32768,depth=16});shuffled.wand.shuffleDeckWhenEmpty=true;local override=assert(constructor:start(shuffled,0,0));equal(override:step(true),'applied_session');equal(parts[entities[override.entity].parts.AbilityComponent].gun_config.shuffle_deck_when_empty,false);globals.PERK_NO_MORE_SHUFFLE_WANDS='0'
local original_create=e.CreateItemActionEntity;e.CreateItemActionEntity=function()error('injected creation failure')end;local failed=assert(constructor:start(wand,0,0));equal(failed:step(true),'uncertain');check(not e.EntityGetIsAlive(failed.entity));local before=killed;equal(failed:step(true),'uncertain');equal(killed,before,'Partial failure never retries');e.CreateItemActionEntity=original_create
local components={}
e.EntityGetComponentIncludingDisabled=function(id,kind)local result={};for c,value in pairs(components)do if value.entity==id and value.kind==kind then result[#result+1]=c end end;return result end
e.EntityAddComponent2=function(id,kind,value)next_part=next_part+1;parts[next_part]=value;components[next_part]={entity=id,kind=kind};return next_part end
e.EntityRemoveComponent=function(id,c)components[c]=nil;parts[c]=nil end
local seq=0;local function identity()seq=seq+1;return string.format('00000000-0000-4000-8000-%012d',seq)end
-- Actual bridge inventory snapshot under strict optional-string semantics.
entities[50]={parts={Inventory2Component=501},tags={},children={51}}
entities[51]={parts={},tags={},children={10},name='inventory_quick'};parts[501]={mActualActiveItem=10};parts[102].inventory_slot={2,0};entities[10].parent=51
local previous_name,previous_inlets=e.EntityGetName,e.EntityGetWithTag
e.EntityGetName=function(id)return entities[id].name or ''end;e.EntityGetWithTag=function()return {}end
local inventory_bridge=dofile(root..'item_bridge.lua').new(e,identity,function()return true end,function()return {}end,function()return false end,function()end)
inventory_bridge:bind('strict-inventory');inventory_bridge.view={enabled=true,fresh=true};inventory_bridge:tick(30,50)
local inventory=assert(json.decode(inventory_bridge.inventory_body,{bytes=131072,nodes=32768,depth=16}))
equal(#inventory.wands,1,'Bridge emits one real production wand snapshot');equal(inventory.heldSlot,'2');check(inventory.wands[1].wand and not inventory.wands[1].unavailableReason,'Strict engine calls preserve supported wand snapshot');equal(inventory.wands[1].wand.deckCapacity,'120')
e.EntityGetName=previous_name;e.EntityGetWithTag=previous_inlets;entities[10].parent=nil;parts[102].inventory_slot=nil
-- Dropped potion capture/prepare keeps the exact original until authorization.
local previous_candidates=e.EntityGetInRadiusWithTag
e.EntityGetInRadiusWithTag=function(_,_,_,tag)return tag=='item_physics'and {30}or {}end
local potion_guard=dofile(root..'item_local.lua').new(e,codec,api,identity);potion_guard:bind('strict-potion')
local potion=assert(potion_guard:capture(1));equal(potion.snapshot.entity,30);equal(potion.item.container.contents[1].amount,'1.125')
local potion_op={id=potion.id,digest=string.rep('b',64),direction='deposit'}
check(potion_guard:offer(potion_op,potion.item,'prepare-only',false),'Strict engine permits exact dropped potion preparation');equal(potion.state,'prepared');check(entities[30]~=nil,'Prepared potion is retained before permit')
potion_guard:bind('deleted');e.EntityGetInRadiusWithTag=previous_candidates
local local_guard=dofile(root..'item_local.lua').new(e,codec,api,identity);local_guard:bind('scope');local captured=assert(local_guard:capture(1));check(entities[captured.snapshot.entity]);local offer={id=captured.id,digest=string.rep('a',64),direction='deposit'};check(local_guard:offer(offer,captured.item,'attempt',false));offer.attempt='attempt';entities[captured.snapshot.entity].parent=88;check(not local_guard:permit(offer,false,nil,nil,true),'Held original is not killed');entities[captured.snapshot.entity].parent=nil;local applied=assert(local_guard:permit(offer,false,nil,nil,true));equal(applied.state,'applied_session');check(not entities[captured.snapshot.entity]);before=killed;local_guard:permit(offer,false,nil,nil,true);equal(killed,before,'Duplicate permit cannot kill twice')
local cold=dofile(root..'item_local.lua').new(e,codec,api,identity);cold:bind('scope');check(not cold:offer(offer,captured.item,'next',false),'Cold effect evidence cannot replay')
local cleanup=dofile(root..'item_local.lua').new(e,codec,api,identity);cleanup:bind('scope');local untouched=assert(cleanup:capture(1));local original_entity=untouched.snapshot.entity;local original_marker=untouched.marker;cleanup:bind('deleted');check(entities[original_entity]~=nil,'Deletion metadata cleanup keeps unconsumed original');check(components[original_marker]==nil,'Positive lobby-deleted reset removes only the unmutated inlet marker')
-- Exercise the concrete optional bridge/presenter with the real fixed catalogue.
local requests={};e.GameGetFrameNum=function()return 1 end
local bridge=dofile(root..'item_bridge.lua').new(e,identity,function(body)requests[#requests+1]=assert(json.decode(body));return true end,function()return nil end,function()return false end,function()end)
bridge:bind('gui-scope');bridge.view={enabled=true,fresh=true};local icons,slots=0,0
e.GuiButton=function()return false,false end;e.GuiTextInput=function(_,_,_,_,value)return value end;e.GuiText=function()end;e.GuiImage=function(_,_,_,_,path)check(type(path)=='string'and not path:find('..',1,true),'Fixed sprite path');icons=icons+1 end;e.GuiGetPreviousWidgetInfo=function()return false,false,false end
local rows={};for i=1,40 do rows[i]={itemId=identity(),revision='9007199254740993',state='available',kind='spell',iconId='LIGHT_BULLET',usesRemaining='-1',displayName='Spark Bolt',senderName='Player',claimAllowed=true}end
bridge.bank={revision='9007199254740993',items=rows,page=0,pageCount=1};dofile(root..'item_presenter.lua').draw(e,1,function()slots=slots+1;return slots end,bridge,0,0,function()end)
bridge.bank={revision='9007199254740993',items=rows,page=0,pageCount=1,query=requests[1].query};presenter.draw(e,1,function()slots=slots+1;return slots end,bridge,0,0,function()end)
equal(icons,120,'Unmatched query has empty slots, then forty fixed item icons');check(requests[1].kind=='item_query');equal(#requests[1].query.spellTypes,8);equal(#requests[1].query.wandSort,4);check(requests[1].query.useFilters);check(bridge:claim(rows[1]));equal(requests[#requests].kind,'item_claim');equal(requests[#requests].revision,'9007199254740993','Revision never rounded through Lua numbers')
local claim_id=requests[#requests].id;rows[1].claimAllowed=false;rows[1].state='claimed'
local sent_before_busy=#requests;bridge.query_needed=false;bridge:receive({enabled=true,fresh=true,submission={id=claim_id,status='unresolved'}});check(bridge.claimed[rows[1].itemId]~=nil and not bridge.query_needed,'Unresolved busy original keeps its exact claim suppression');equal(#requests,sent_before_busy,'Busy result cannot create a replacement claim')
bridge.query_needed=false;bridge:receive({enabled=true,fresh=true,submission={id='unrelated',status='refused'}});check(bridge.claimed[rows[1].itemId]~=nil,'Unrelated refusal cannot clear a pending claim')
bridge:receive({enabled=true,fresh=true,submission={id=claim_id,status='refused'}});check(bridge.claimed[rows[1].itemId]==nil and bridge.query_needed,'Exact refused claim requests a confirmed refresh');check(bridge.claim_ids[rows[1].itemId]==nil)
check(not bridge.bank.items[1].claimAllowed,'Refusal alone cannot turn cached optimistic state into availability')
bridge.query_needed=false;bridge:receive({enabled=true,fresh=true,submission={id=claim_id,status='refused'}});check(not bridge.query_needed,'Repeated refusal footer does not flood queries')
local fresh_row={};for key,value in pairs(rows[1])do fresh_row[key]=value end;fresh_row.claimAllowed=true;fresh_row.state='available'
local refreshed=json.encode({revision='9007199254740993',items={fresh_row},page=0,pageCount=1,query=requests[1].query});bridge:receive({enabled=true,fresh=true,chunk=chunks.sender(identity(),'bank',refreshed):chunk()});check(bridge.bank.items[1].claimAllowed,'Confirmed same-revision row becomes usable after known refusal')
-- Start the exact engine injection used by the combined driver and capture a
-- A refused native copy or stale optional view never consumes a visible row.
local click_row={itemId=identity(),revision='1',state='available',kind='spell',iconId='LIGHT_BULLET',usesRemaining='-1',claimAllowed=true}
local claim_calls,copy_accepted=0,false
local click_items={catalogue=real,codec=codec,view={enabled=true,fresh=true},bank={revision='1',items={click_row},page=0,pageCount=1,query=requests[1].query},query=function()return true end,claim=function()claim_calls=claim_calls+1;return copy_accepted end}
local function draw_claim()presenter.draw(e,1,function()slots=slots+1;return slots end,click_items,0,0,function()end)end
local old_button=e.GuiImageButton;e.GuiImageButton=function(_,_,x,y,_,path)return x==0 and y==55 and path:find('/slot.png',1,true)~=nil end
-- Presenter creates its own button adapter from the engine, just like the game.
draw_claim();equal(claim_calls,1);equal(click_row.state,'available','Refused native copy keeps row available');check(click_row.claimAllowed)
click_items.view.fresh=false;draw_claim();equal(claim_calls,2,'Stale display still submits an individually validated claim');equal(click_row.state,'available')
click_items.view.fresh=true;click_items.view.enabled=false;draw_claim();equal(claim_calls,2,'Disabled item view cannot submit a claim')
click_items.view.enabled=true;copy_accepted=true;draw_claim();equal(claim_calls,3);equal(click_row.state,'claimed','Only an accepted claim dims the row');check(not click_row.claimAllowed);e.GuiImageButton=old_button
local amount_plan='10';local amount_env={GuiTextInput=function(_,_,_,_,value)return value end}
raw(repo..'/apps/desktop/tools/lifecycle-lc1-b1.lua').new(amount_env,{},nil,function(key,consume)if key=='b1-amount'then local value=amount_plan;if consume then amount_plan=nil end;return value end end,function()end)
equal(amount_env.GuiTextInput(1,2,200,109,'',180,32),'','Search widget cannot consume a gold amount gesture');equal(amount_plan,'10')
equal(amount_env.GuiTextInput(1,3,242,38,'1',120,10,'0123456789'),'10','Actual gold amount widget receives its gesture');check(amount_plan==nil);equal(amount_env.GuiTextInput(1,3,242,38,'10',120,10,'0123456789'),'10','Amount gesture consumed once')
-- Concurrent issued removals can overlap for many engine frames.
local delayed_sources={};local saved_kill=e.EntityKill;local saved_candidates=e.EntityGetInRadiusWithTag
local overlap=dofile(root..'item_local.lua').new(e,codec,api,identity);overlap:bind('overlap')
local ops,records={},{};local source_ids={}
for i=1,3 do source_ids[i]=e.CreateItemActionEntity('LIGHT_BULLET',1,1)end
e.EntityGetInRadiusWithTag=function(_,_,_,tag)return tag=='card_action'and source_ids or {}end
e.EntityKill=function(id)delayed_sources[id]=(delayed_sources[id]or 0)+1 end
for i=1,3 do local r=assert(overlap:capture(1));records[i]=r;local op={id=r.id,digest=string.rep(tostring(i),64),direction='deposit',attempt=string.format('%032d',i),stage='permit'};ops[i]=op;check(overlap:offer(op,r.item,op.attempt,false));check(not overlap:permit(op,false,nil,nil,true,i));equal(r.state,'removing')end
for i=1,3 do equal(records[i].state,'removing','Starting sibling cannot revoke another issued removal');equal(delayed_sources[records[i].snapshot.entity],1)end
for frame=4,10 do overlap:observe_removals(frame,ops,true);for i=1,3 do equal(records[i].state,'removing')end end
-- Removing one actual permit still revokes only that original.
overlap:observe_removals(11,{ops[1],ops[3]},true);equal(records[2].state,'uncertain');equal(records[1].state,'removing');equal(records[3].state,'removing')
for _,r in ipairs(records)do entities[r.snapshot.entity]=nil end
overlap:observe_removals(12,ops,true);equal(records[1].state,'applied_session');equal(records[3].state,'applied_session');equal(records[2].state,'uncertain','A restored permit cannot upgrade revoked uncertainty')
for i=1,3 do overlap:permit(ops[i],false,nil,nil,true,13);equal(delayed_sources[records[i].snapshot.entity],1,'One engine call per overlapping original')end
e.EntityKill=saved_kill;e.EntityGetInRadiusWithTag=saved_candidates
-- representative original. This validates its setup before occupying SQL.
entities[99]={x=0,y=0,parts={},tags={},children={}}
local pending=json.encode({catalogueRevision=real.revision,spell={actionId='LIGHT_BULLET',usesRemaining='-1'}});local marked={}
local fixture_state={frame=1}
local fixture=raw(repo..'/apps/desktop/tools/lifecycle-lc1-items.lua').new(e,fixture_state,function(key,consume)if key=='items-spawn'then local value=pending;if consume then pending=nil end;return value end end,function(key,value)marked[key]=value end,hit)
fixture:snapshot({items={uploads={},guard={status=''},view={}}});local snapshot=assert(json.decode(marked['items-view.json'],{bytes=262144,nodes=65536,depth=20}));equal(#snapshot.aliveOriginals,1);equal(snapshot.originals,1);check(snapshot.engineFactsInjected and snapshot.savePersistenceProven==false)
local ix,iy=e.EntityGetTransform(800001);entities[99].x=100;local moved_x,moved_y=e.EntityGetTransform(800001);equal(moved_x,ix,'Synthetic temple inlet stays in the world while player moves');equal(moved_y,iy);entities[99].x=0
-- Repeated native footer views must not generate one ACK per frame or skip the
--30-frame game cadence. Permits below are labelled synthetic handshake inputs.
local copied_packets={};local loop_bridge=dofile(root..'item_bridge.lua').new(e,identity,function(body)copied_packets[#copied_packets+1]=assert(json.decode(body));return true end,function()return {}end,function()return false end,function()end);loop_bridge:bind('loop-scope')
local footer={enabled=true,fresh=true,chunk=chunks.sender('players-progress','players','[]'):chunk()}
for frame=1,90 do loop_bridge:receive(footer);fixture_state.frame=frame;loop_bridge:tick(frame,99);fixture:snapshot({items=loop_bridge})end
local ack_count,deposit_count=0,0;for _,p in ipairs(copied_packets)do if p.kind=='item_copied'then ack_count=ack_count+1 elseif p.kind=='item_chunk'and p.channel=='deposit'then deposit_count=deposit_count+1 end end;equal(ack_count,1,'One queued ACK for repeated unchanged footer');check(deposit_count>0,'Capture/upload progresses while the copied footer repeats');equal(loop_bridge.guard.count,1,'One original capture at the actual30-frame cadence')
local original;for _,r in pairs(loop_bridge.guard.records)do original=r end;local op={id=original.id,digest=string.rep('a',64),direction='deposit',stage='offer'};footer.chunk=chunks.sender(original.id,'effect_item',original.body):chunk();footer.effect=op
for frame=91,94 do loop_bridge:receive(footer);fixture_state.frame=frame;loop_bridge:tick(frame,99);fixture:snapshot({items=loop_bridge})end
op.stage='permit';op.attempt=loop_bridge.guard.records[original.id].attempt
for frame=95,150 do loop_bridge:receive(footer);fixture_state.frame=frame;loop_bridge:tick(frame,99);fixture:snapshot({items=loop_bridge})end
local result_count=0;for _,p in ipairs(copied_packets)do if p.kind=='item_result'and p.id==original.id then result_count=result_count+1;check(p.applied,'Synthetic permitted removal observed once')end end;equal(result_count,1,'Repeated permit footer does not requeue an already-copied result')
-- Native output-copy lease gaps may skip exact modulo30 frames. Keep the
-- existing30-frame budget, but run once on the first eligible callback after due.
local cadence=dofile(root..'item_bridge.lua').new(e,identity,function()return true end,function()return {}end,function()return false end,function()end)
cadence:bind('cadence');cadence:receive({enabled=true,fresh=true});local scans={};local at=0
cadence.guard.capture=function()scans[#scans+1]=at;return nil end
for _,frame in ipairs({1,2,29,31,33,61,300,301})do at=frame;cadence:tick(frame,99)end
equal(#scans,4,'Missed cadence has one bounded due capture, no catch-up flood');for i,frame in ipairs({1,31,61,300})do equal(scans[i],frame,'Capture on first eligible callback at/after due')end
-- Group selection is capped by live originals, not only queued descriptor copies.
local group=dofile(root..'item_bridge.lua').new(e,identity,function()return true end,function()return {}end,function()return false end,function()end)
group:bind('group');group:receive({enabled=true,fresh=true});local captures=0;local live=0
local old_inlets=e.EntityGetWithTag;e.EntityGetWithTag=function(tag)if tag=='nt2_item_inlet'then return {1,2}else return old_inlets(tag)end end
group.guard.active_count=function()return live end
group.guard.capture=function(_,inlet)captures=captures+1;live=live+1;return {id='group-'..captures,body='{}'}end
group:tick(1,99);equal(captures,4,'Four inlet originals selected in one cadence');equal(#group.uploads,4,'Bounded descriptor queue')
group.uploads={};group:tick(31,99);equal(captures,4,'Copies accepted by app still count as live originals')
live=2;group:tick(61,99);equal(captures,6,'Completed/refused siblings release only their own slots');local deposits=0;for _,copy in ipairs(group.uploads)do if copy.channel=='deposit'then deposits=deposits+1 end end;equal(deposits,2)
e.EntityGetWithTag=old_inlets
local active_guard=dofile(root..'item_local.lua').new(e,codec,api,identity);active_guard:bind('active-cap')
for i,state in ipairs({'captured','prepared','removing','constructing','uncertain','refused','applied_session'})do active_guard.records[i]={state=state}end
equal(active_guard:active_count(),4,'Only live local originals consume active group slots')
-- Confirmed query pages are cached locally, fenced on scope and never mixed.
local cached=dofile(root..'item_bridge.lua').new(e,identity,function()return true end,function()return nil end,function()return false end,function()end);cached:bind('cache-a')
local qa,qb={search='a'},{search='b'}
local function page(query,revision,id)local value={query=query,revision=revision,items={},page=0,pageCount=1};cached:receive({enabled=true,fresh=true,chunk=chunks.sender(id,'bank',assert(json.encode(value))):chunk()})end
check(cached:query(qa));page(qa,'9','page-a');equal(cached.bank.revision,'9')
check(cached:query(qb));check(cached.bank==nil,'No mismatched query as current');page(qb,'10','page-b');equal(cached.bank.revision,'10')
page(qa,'11','page-a-late');equal(cached.bank.revision,'10','Late different query does not replace current page')
check(cached:query(qa));equal(cached.bank.revision,'11','Open cached query renders immediately');page(qa,'8','page-a-old');equal(cached.bank.revision,'11','Old page cannot replace a newer confirmed query')
cached:bind('cache-b');check(cached.bank==nil and next(cached.banks)==nil,'Scope change clears all query cache')
print('ITEM_TRANSFER_LOCAL_CHECKS|'..checks..'|engineFactsInjected=true|loadedSaveProof=false')
