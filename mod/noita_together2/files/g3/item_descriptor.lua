-- Local consumer of the server-owned item descriptor/catalogue. No entity,
-- script or sprite path from a received descriptor is executed or loaded.
local M={MAX_BYTES=131072,MAX_DECK=1024,MAX_PERMANENT=256,MAX_MATERIALS=1024}
local float_max=3.4028234663852886e38
local function finite(v)return type(v)=='number'and v==v and v>-math.huge and v<math.huge end
local function integer(v,lo,hi)return finite(v)and v%1==0 and v>=lo and v<=hi end
local function array(t,max)
 if type(t)~='table'or getmetatable(t)then return nil end;local n=0
 for k in pairs(t)do n=n+1;if not integer(k,1,max)or n>max then return nil end end
 for i=1,n do if t[i]==nil then return nil end end;return n
end
local function fields(t,allowed)
 if type(t)~='table'or getmetatable(t)then return false end
 local keys={};for _,key in ipairs(allowed)do keys[key]=true end
 for key in pairs(t)do if not keys[key]then return false end end;return true
end
local function utf8(s,max)
 if type(s)~='string'or #s>max or s:find('%z')then return false end
 local i=1
 while i<=#s do local a=s:byte(i);local n,v,min
  if a<128 then n=1;v=a;min=0 elseif a>=194 and a<=223 then n=2;v=a-192;min=128 elseif a>=224 and a<=239 then n=3;v=a-224;min=2048 elseif a>=240 and a<=244 then n=4;v=a-240;min=65536 else return false end
  for j=1,n-1 do local b=s:byte(i+j);if not b or b<128 or b>191 then return false end;v=v*64+b-128 end
  if v<min or v>1114111 or v>=55296 and v<=57343 then return false end;i=i+n
 end;return true
end
local function decimal(s,lo,hi,whole)
 if type(s)~='string'or #s==0 or #s>64 or not s:match('^-?%d+%.?%d*[eE]?[+%-]?%d*$')then return nil end
 local n=tonumber(s);if not finite(n)or n<lo or n>hi or whole and (not s:match('^-?%d+$')or n%1~=0)then return nil end;return n
end
local function encode(n)assert(finite(n),'Nonfinite item field');return string.format('%.17g',n)end
M.number=decimal;M.encode_number=encode;M.array=array;M.utf8=utf8
function M.new(catalogue)
 assert(type(catalogue)=='table'and type(catalogue.revision)=='string','Fixed local item catalogue required')
 local self={catalogue=catalogue}
 local function spell(v)
  if not fields(v,{'actionId','usesRemaining'})or type(v.actionId)~='string'or not catalogue.spells[v.actionId]then return nil,'Unsupported spell'end
  local uses=decimal(v.usesRemaining,-1,2147483647,true);if uses==nil then return nil,'Invalid spell uses'end
  return {actionId=v.actionId,uses=uses,catalogue=catalogue.spells[v.actionId]}
 end
 local function cards(values,max)
  if not array(values,max)then return nil,'Invalid wand cards'end;local result,seen,previous={}, {}, -1
  for _,card in ipairs(values)do
   if not fields(card,{'slot','spell'})or not integer(card.slot,0,2147483647)or card.slot<=previous or seen[card.slot]then return nil,'Invalid wand card order'end
   local s,error=spell(card.spell);if not s then return nil,error end;previous=card.slot;seen[card.slot]=true;s.slot=card.slot;result[#result+1]=s
  end;return result
 end
 function self:resolve(item)
  if not fields(item,{'catalogueRevision','spell','wand','container','object'})or item.catalogueRevision~=catalogue.revision then return nil,'Incompatible item catalogue'end
  local count=0;for _,key in ipairs({'spell','wand','container','object'})do if item[key]~=nil then count=count+1 end end;if count~=1 then return nil,'Invalid item category'end
  if item.spell then local plan,error=spell(item.spell);if not plan then return nil,error end;plan.kind='spell';return plan end
  if item.wand then
   local w=item.wand;if not fields(w,{'named','uiName','spriteId','gripX','gripY','tipX','tipY','manaMax','manaChargeSpeed','reloadTime','actionsPerRound','deckCapacity','shuffleDeckWhenEmpty','spreadDegrees','speedMultiplier','fireRateWait','deck','permanent'})or type(w.named)~='boolean'or type(w.shuffleDeckWhenEmpty)~='boolean'or not utf8(w.uiName,256)or type(w.spriteId)~='string'or not catalogue.sprites[w.spriteId]then return nil,'Unsupported wand appearance'end
   local plan={kind='wand',named=w.named,name=w.uiName,shuffle=w.shuffleDeckWhenEmpty,sprite=catalogue.sprites[w.spriteId],stats={}}
   for _,key in ipairs({'gripX','gripY','tipX','tipY','manaMax','manaChargeSpeed','reloadTime','actionsPerRound','deckCapacity','spreadDegrees','speedMultiplier','fireRateWait'})do
    local whole=key=='reloadTime'or key=='actionsPerRound'or key=='deckCapacity'
    local n=decimal(w[key],whole and -2147483648 or -float_max,whole and 2147483647 or float_max,whole);if n==nil then return nil,'Invalid wand stat: '..key end;plan.stats[key]=n
   end
   local error;plan.deck,error=cards(w.deck,M.MAX_DECK);if not plan.deck then return nil,error end;plan.permanent,error=cards(w.permanent,M.MAX_PERMANENT);if not plan.permanent then return nil,error end
   if plan.stats.deckCapacity+#plan.permanent>2147483647 then return nil,'Wand capacity overflow'end
   return plan
  end
  if item.container then
   local c=item.container;if not fields(c,{'containerType','color','contents'})or c.containerType~='potion'and c.containerType~='powder'or not array(c.contents,M.MAX_MATERIALS)then return nil,'Invalid container'end
   local color=decimal(c.color,0,4294967295,true);if not color then return nil,'Invalid container colour'end
   local plan={kind=c.containerType,color=color,contents={}};local seen={}
   for _,part in ipairs(c.contents)do
    if not fields(part,{'materialId','amount'})or type(part.materialId)~='string'or not catalogue.materials[part.materialId]or seen[part.materialId]then return nil,'Unsupported or duplicate material'end
    local amount=decimal(part.amount,0,1.7976931348623157e308);if not amount or amount<=0 then return nil,'Invalid material quantity'end;seen[part.materialId]=true;plan.contents[#plan.contents+1]={name=part.materialId,amount=amount,catalogue=catalogue.materials[part.materialId]}
   end;return plan
  end
  local o=item.object;if not fields(o,{'objectId'})or type(o.objectId)~='string'or not catalogue.objects[o.objectId]then return nil,'Unsupported object'end
  if catalogue.objects[o.objectId].available==false then return nil,'Object unavailable in this game build'end
  return {kind='object',object=catalogue.objects[o.objectId]}
 end
 local function encode_spell(raw)return {actionId=raw.actionId,usesRemaining=encode(raw.usesRemaining)}end
 function self:capture(api,e,entity)
  local item={catalogueRevision=catalogue.revision}
  if e.EntityHasTag(entity,'wand')then
   local raw=api:wand(entity);local w=raw.stats;local sprite_id=catalogue.sprite_ids[w.sprite];assert(sprite_id,'Unsupported wand sprite')
   item.wand={named=w.named,uiName=w.uiName,spriteId=sprite_id,shuffleDeckWhenEmpty=w.shuffleDeckWhenEmpty,deck={},permanent={}}
   for _,key in ipairs({'gripX','gripY','tipX','tipY','manaMax','manaChargeSpeed','reloadTime','actionsPerRound','deckCapacity','spreadDegrees','speedMultiplier','fireRateWait'})do item.wand[key]=encode(w[key])end
   for _,list in ipairs({'deck','permanent'})do for _,card in ipairs(raw[list])do item.wand[list][#item.wand[list]+1]={slot=card.slotX,spell=encode_spell(card)}end end
  elseif e.EntityHasTag(entity,'card_action')then item.spell=encode_spell(api:spell(entity))
  elseif e.EntityGetFirstComponentIncludingDisabled(entity,'MaterialInventoryComponent')then
   local raw=api:container(entity);item.container={containerType=raw.kind,color=encode(raw.colour),contents={}}
   for _,part in ipairs(raw.contents)do item.container.contents[#item.container.contents+1]={materialId=part.material,amount=encode(part.amount)}end
  else local path=e.EntityGetFilename(entity);local object_id=catalogue.object_ids[path];assert(object_id,'Unsupported object');item.object={objectId=object_id}end
  local plan,error=self:resolve(item);assert(plan,error);return item,plan
 end
 return self
end
return M
