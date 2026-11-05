local repo=assert(os.getenv('NT2_ITEM_REPO'))
local output=assert(os.getenv('NT2_HOURGLASS_OUTPUT'))
local function read(path)local f=assert(io.open(path,'rb'));local v=f:read('*a');f:close();return v end
local function write(path,value)local f=assert(io.open(path,'wb'));f:write(value);f:close()end
NT2_C1_TEST_ENGINE_ONLY=true
local fixture=dofile(repo..'/apps/desktop/tools/c1-local-test.lua')
NT2_C1_TEST_ENGINE_ONLY=nil
local checks=0
local function eq(a,b)assert(a==b,tostring(a)..' ~= '..tostring(b));checks=checks+1 end
local installed_csv=nil
local function engine()
 local e,c,entities,g=fixture.engine();local virtual={['data/translations/common.csv']=installed_csv or read(assert(os.getenv('NT2_HOURGLASS_COMMON')))}
 e.ModTextFileGetContent=function(path)return virtual[path]or read(repo..'/'..path:gsub('^mods/','mod/'))end
 e.ModTextFileSetContent=function(path,value)virtual[path]=value end
 e.ModLuaFileAppend=function()end
 if not installed_csv then
  e.dofile('mods/noita_together2/files/c1/register.lua')
  e.dofile('mods/noita_together2/files/c2/register.lua')
  local once=virtual['data/translations/common.csv']
  e.dofile('mods/noita_together2/files/c2/register.lua');eq(virtual['data/translations/common.csv'],once)
  installed_csv=once
 end
 return e,c,entities,g,virtual
end
local e,_,_,_,virtual=engine()
write(output..'/assembled-common.csv',virtual['data/translations/common.csv'])
if os.getenv('NT2_HOURGLASS_PHASE')=='assemble'then print('PASS production C1/C2 translation assembly and idempotence');return end
local json=e.dofile('mods/noita_together2/files/g3/bank_json.lua')
local tables=assert(json.decode(read(output..'/lookup.json'),{bytes=131072,nodes=16384,depth=16}))
local legacy=tables.english
local function format(value,args)return(value:gsub('\\n','\n'):gsub('%$(%d)',function(n)return tostring(args[tonumber(n)+1]or '')end))end
local expected={
 ambrosia={'boost_title','ambrosia_subtitle','protection'},
 berserk={'boost_title','berserk_subtitle','damage'},
 charm={'boon_title',false,false},
 confusion={'fungus_title','fungus_subtitle_yes',false},
 speed={'boost_title','speed_subtitle','speed'},
 levitation={'boost_title','levitatium_subtitle','levitatium'},
 speed_and_levitation={'boost_title','hastium_subtitle','hastium'},
}
local prefix='noitatogether_hourglass_'
local name='Peer $0 100% <&",'
local function api(e,language,failure)
 local notices,icons={},{}
 e.GameTextGet=function(key,...)
  if failure=='error'then error('injected text API failure')end
  if failure=='empty'then return ''end
  if failure=='key'then return key end
  if failure=='bare'then return key:sub(2)end
  if failure=='nil'then return nil end
  local value=(tables.languages[language]or{})[key:sub(2)]
  return value and format(value,{...})or ''
 end
 e.GamePrintImportant=function(title,subtitle)
  assert(type(title)=='string'and title:find('%S')and not title:find('^%$noitatogether_'))
  assert(type(subtitle)=='string'and not subtitle:find('^%$noitatogether_'))
  notices[#notices+1]={title,subtitle}
 end
 local add=e.EntityAddComponent2
 e.EntityAddComponent2=function(id,kind,values)if kind=='UIIconComponent'then icons[#icons+1]=values end;return add(id,kind,values)end
 e.EntityAddChild=function()end;e.EntityIngestMaterial=function()end;e.CellFactory_GetType=function()return 1 end
 return notices,icons
end
local modes={};for language in pairs(tables.languages)do modes[#modes+1]={language}end
for _,failure in ipairs({'empty','key','bare','nil','error'})do modes[#modes+1]={'en',failure}end
modes[#modes+1]={'unknown-language'}
for _,mode in ipairs(modes)do
 for id,row in pairs(expected)do
  local e,c=engine();local notices,icons=api(e,mode[1],mode[2])
  local registry=e.dofile('mods/noita_together2/files/c2/hourglass.lua').new(e)
  eq(registry:apply(id,1,name),'applied');eq(#notices,1)
  eq(notices[1][1],format(legacy[prefix..row[1]],{name}))
  eq(notices[1][2],row[2]and legacy[prefix..row[2]]or '')
  if row[3]then
   eq(#icons,1);eq(icons[1].name,legacy[prefix..'buff_'..row[3]..'_name'])
   eq(icons[1].description,format(legacy[prefix..'buff_'..row[3]..'_desc'],{name}))
   eq(icons[1].is_perk,false)
  else eq(#icons,0)end
  if id~='charm'and id~='confusion'and id~='ambrosia'then eq(c[e.EntityGetFirstComponent(e.last_effect,'GameEffectComponent')].frames,7200)end
 end
 -- The no-player fungus branch has its own legacy subtitle.
 local e=engine();local notices=api(e,mode[1],mode[2]);local registry=e.dofile('mods/noita_together2/files/c2/hourglass.lua').new(e)
 eq(registry:apply('confusion',nil,name),'unissued');eq(notices[1][2],legacy[prefix..'fungus_subtitle_no'])
 -- Every other C2 row shares the loader/fallback, including Steve and Sampo.
 local module=e.dofile('mods/noita_together2/files/c2/hourglass.lua')
 for key,value in pairs(legacy)do eq(module.text(e,'$'..key,name,'Salt','13'),format(value,{name,'Salt','13'}))end
end
-- Preserve a future nonempty localized cell while filling only absent locales.
do
 local e,_,_,_,virtual=engine();local module=e.dofile('mods/noita_together2/files/c2/hourglass.lua')
 local path='mods/noita_together2/files/c2/translations.csv'
 virtual[path]=e.ModTextFileGetContent(path):gsub('noitatogether_hourglass_boost_title,%$0 boosted the team,', 'noitatogether_hourglass_boost_title,$0 boosted the team,RU localized $0',1)
 -- The injected localized field replaces the first empty Russian cell.
 module.install_translations(e)
 write(output..'/localized-common.csv',virtual['data/translations/common.csv'])
 local calls=0;e.GameTextGet=function(_,sender)calls=calls+1;return 'RU localized '..sender end
 eq(module.text(e,'$noitatogether_hourglass_boost_title',name),'RU localized '..name);eq(calls,1)
 e.GameTextGet=function()return ''end
 eq(module.text(e,'$noitatogether_hourglass_boost_title',nil),'Another player boosted the team')
end
-- Real sender callback: one notice, immutable operation, no source effect.
for id in pairs(expected)do
 local e,c,entities=engine();local notices,icons=api(e,'en')
 e.SessionNumbersGetValue=function()return '0'end;e.EntitySetComponentIsEnabled=function(_,part,value)c[part].enabled=value end
 e.EntitySetComponentsWithTagEnabled=function()end
 e.EntityGetInRadiusWithTag=function(_,_,_,tag)return e.EntityGetWithTag(tag)end
 local load=e.EntityLoad
 e.EntityLoad=function(path,x,y)local id=load(path,x,y);if path:find('hourglass_effect_trigger',1,true)then e.EntityAddTag(id,'hourglass_entity');e.EntityAddComponent2(id,'MaterialAreaCheckerComponent',{})end;return id end
 local store=e.dofile('mods/noita_together2/files/c2/storage.lua')
 local registry=e.dofile('mods/noita_together2/files/c2/hourglass.lua').new(e)
 local world=string.rep('a',64);local context='test-context';local material=registry.entries[id].material
 store.write(e,'grant',{world=world,context=context,playing=true,frame=100,player=1,progression=0,rulesRevision=1,hourglass={[id]=material}})
 local anchor=e.EntityLoad('anchor',0,0)
 store.mark(e,anchor,'nt2_c2_hourglass_group',{world=world,context=context,progression=0,activationId='11111111-1111-4111-8111-111111111111'})
 e.dofile('mods/noita_together2/files/c2/scripts/hourglass_events.lua')
 local trigger=e.CreateHourglassEntity(0,0,id,material,anchor);e.GetUpdatedEntityID=function()return trigger end
 e.material_area_checker_success();eq(#notices,1);eq(notices[1][1],legacy[prefix..'gave_boon']);eq(notices[1][2],'');eq(#icons,0)
 local operation=store.read(e,'outgoing').queue;eq(#operation,1);eq(operation[1].payload.hourglass.effectId,id);eq(operation[1].senderDisposition,'applied')
 e.material_area_checker_success();eq(#notices,1);eq(#store.read(e,'outgoing').queue,1)
end
print('PASS '..checks..' production translation/message checks; all seven effects, sender, both fungus subtitles, icons, locales and fallback; injected engine calls')
