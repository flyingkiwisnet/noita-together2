-- Narrow legacy item/wand presenter. Fixed catalogue assets only; literal text.
local M={}
local art='mods/noita_together2/files/g3/ui/item_bank/'
local wand_icons={'icon_gun_shuffle-1.png','icon_gun_actions_per_round-1.png','icon_fire_rate_wait-1.png','icon_reload_time-1.png','icon_mana_max-1.png','icon_mana_charge_speed-1.png','icon_gun_capacity-1.png','icon_spread_degrees-1.png'}
local spell_icons={'LIGHT_BULLET_TRIGGER','POLYMORPH_FIELD','BLOODLUST','BURST_4','MATERIAL_WATER','ALPHA','X_RAY','TORCH'}
local function label(e,key,fallback)if type(key)=='string'and e.GameTextGetTranslatedOrNot then local ok,text=pcall(e.GameTextGetTranslatedOrNot,key);if ok and type(text)=='string'and text~=''and text~=key then return text end end;return fallback or key or 'Unknown'end
function M.icon(catalogue,row)
 local id=row.iconId;local entry=catalogue.sprites[id]or catalogue.spells[id]or catalogue.objects[id]
 if entry then return entry.sprite or entry.path end
 if row.kind=='container'then return row.iconId=='powder'and 'data/ui_gfx/items/material_pouch.png'or 'data/ui_gfx/items/potion.png'end
 return 'data/ui_gfx/inventory/item_bg.png'
end
-- Legacy get_wand_sprite resolves a fixed sprite XML to its image for GuiImage.
-- The received item supplies only a catalogue ID; XML-derived paths must also
-- be in the fixed local catalogue. Cache on the bank instance across frames.
function M.gui_sprite(e,catalogue,row,cache)
 local path=M.icon(catalogue,row);if cache[path]then return cache[path]end
 local result=path
 if path:sub(-4)=='.xml'and e.ModTextFileGetContent then local ok,xml=pcall(e.ModTextFileGetContent,path)
  if ok and type(xml)=='string'and #xml<65536 then local png=xml:match('filename="([^"]+)"');if png and catalogue.sprite_ids[png]then result=png end end
 end
 cache[path]=result;return result
end
function M.lines(e,codec,item)
 local plan,reason=codec:resolve(item);if not plan then return {reason or 'Unavailable item'}end
 local c=codec.catalogue;local lines={}
 local function spell(entry)return label(e,entry.catalogue.nameKey,entry.actionId)..' ['..(entry.uses==-1 and 'unlimited'or tostring(entry.uses))..']'end
 if plan.kind=='spell'then lines[1]=spell(plan)
 elseif plan.kind=='wand'then
  lines[1]=plan.named and plan.name or label(e,plan.name,'Wand');local s=plan.stats
  for _,pair in ipairs({{'Shuffle',plan.shuffle and 'Yes'or 'No'},{'Casts',s.actionsPerRound},{'Cast delay',s.fireRateWait},{'Recharge',s.reloadTime},{'Max mana',s.manaMax},{'Mana recharge',s.manaChargeSpeed},{'Capacity',s.deckCapacity},{'Spread',s.spreadDegrees},{'Speed',s.speedMultiplier}})do lines[#lines+1]=pair[1]..': '..tostring(pair[2])end
  for _,card in ipairs(plan.permanent)do lines[#lines+1]='Always cast '..card.slot..': '..spell(card)end
  for _,card in ipairs(plan.deck)do lines[#lines+1]='Slot '..card.slot..': '..spell(card)end
 elseif plan.kind=='potion'or plan.kind=='powder'then
  lines[1]=plan.kind=='powder'and 'Powder pouch'or 'Potion';local total=0;for _,part in ipairs(plan.contents)do total=total+part.amount end;local divisor=plan.kind=='powder'and 15 or 10
  lines[#lines+1]=string.format('Contents: %.4g (%.4g%%)',total,total/divisor)
  for _,part in ipairs(plan.contents)do lines[#lines+1]=label(e,part.catalogue.nameKey,part.name)..': '..string.format('%.8g (%.4g%%)',part.amount,part.amount/divisor)end
  if #plan.contents==0 then lines[#lines+1]='Empty'end
 else lines[1]=label(e,plan.object.nameKey,plan.object.id or 'Object')end
 return lines
end
function M.draw(e,gui,id,items,x,y,layer)
 local json=dofile('mods/noita_together2/files/g3/bank_json.lua');local model=items.model
 if not model then model=dofile('mods/noita_together2/files/g3/item_bank_model.lua').new();items.model=model end
 local function query()
  local types,sort={},{};for i=1,8 do if model.spell_filters[i]then types[#types+1]=i-1 end;if model.wand_filters[i]then sort[#sort+1]=model.criteria and model.criteria[i].key or dofile('mods/noita_together2/files/g3/item_bank_model.lua').criteria[i].key end end
  return {tab=model.tab,search=model.search,spellTypes=json.array(types),wandSort=json.array(sort),sort=model.grouped and 'name'or 'default',page=model.page,useFilters=true}
 end
 local function button(px,py,value)local clicked,right=e.GuiButton(gui,id(),px,py,value);return clicked,right end
 local function decoration()if e.GuiOptionsAddForNextWidget and e.GUI_OPTION and e.GUI_OPTION.NonInteractive then e.GuiOptionsAddForNextWidget(gui,e.GUI_OPTION.NonInteractive)end end
 local changed=false
 for i,tab in ipairs({'all','wands','spells','items'})do layer(9);local clicked;if e.GuiImageButton then clicked=e.GuiImageButton(gui,id(),x+(i-1)*54,y,tab:upper(),art..tab..'.png')else clicked=button(x+(i-1)*54,y,tab:upper())end;if clicked then model:set_tab(tab);changed=true end end
 if e.GuiTextInput then layer(9);local value=e.GuiTextInput(gui,id(),x,y+14,model.search,180,32);if type(value)=='string'and value~=model.search then model:edit(value);changed=true end end
 layer(9);if button(x+187,y+14,'Sort')then model:sort();changed=true end
 if model.tab=='spells'or model.tab=='wands'then local module=dofile('mods/noita_together2/files/g3/item_bank_model.lua');for i=1,8 do
  local filters=model.tab=='spells'and model.spell_filters or model.wand_filters;local name=model.tab=='spells'and module.categories[i]or module.criteria[i].label
  local px,py=x+((i-1)%4)*57,y+29+math.floor((i-1)/4)*11
  layer(9);local left,right=button(px,py,(filters[i]and '+'or '-')..name:sub(1,7));if left or right then model:toggle(model.tab,i,right);changed=true end
  local icon=model.tab=='wands'and art..wand_icons[i]or items.catalogue.spells[spell_icons[i]]and items.catalogue.spells[spell_icons[i]].sprite
  if icon and e.GuiImage then layer(9);e.GuiImage(gui,id(),px+45,py,icon,filters[i]and 1 or 0.4,0.5,0.5)end
  if e.GuiTooltip then layer(8);e.GuiTooltip(gui,name,'Left click: toggle. Right click: solo.')end
 end end
 local q=query();local key=json.encode(q)
 if changed or items.query_needed or items.query_key~=key then if items:query(q)then items.query_key=key;items.query_needed=false;items.loading=true end end
 local bank=items.bank
 if bank and json.encode(bank.query)==key then items.loading=false;model.page=bank.page or 0 end
 local rows=bank and json.encode(bank.query)==key and bank.items or {};local hovered=nil
 for cell=1,40 do local row=rows[cell];local px,py=x+((cell-1)%8)*28,y+55+math.floor((cell-1)/8)*25
  -- Stable cell IDs across empty/populated refreshes. Larger Z is deeper:
  -- outer panel10, slot9, action8, icon7, uses6, tooltip5.
  local background_id,icon_id,action_id=id(),id(),id()
  layer(9);decoration();if e.GuiImage then e.GuiImage(gui,background_id,px,py,art..'slot.png',1,1,1)end
  if row then
   local available=items.view.enabled==true and row.claimAllowed==true and row.state=='available'
   layer(8);local clicked=e.GuiImageButton and e.GuiImageButton(gui,action_id,px,py,'',art..'slot.png')or false
   local over=false;if e.GuiGetPreviousWidgetInfo then local _,_,h=e.GuiGetPreviousWidgetInfo(gui);over=h end
   if clicked then if items.cancel_inspect then items:cancel_inspect()end;if available and items:claim(row)then row.claimAllowed=false;row.state='claimed'end end
   if over then hovered=row;if not clicked and items.inspect_id~=row.itemId then if items:inspect(row.itemId)then items.inspect_id=row.itemId end end end
   if e.GuiTooltip then local detail=items.detail;local lines={};if detail and detail.summary and detail.summary.itemId==row.itemId and detail.item then lines=M.lines(e,items.codec,detail.item)end;lines[#lines+1]='From: '..(row.senderName or 'Player');if not available then lines[#lines+1]='Unavailable / claimed / disabled'end;layer(5);e.GuiTooltip(gui,' '..label(e,row.nameKey,row.displayName),table.concat(lines,'\n'))end
   layer(7);local detail=items.detail;local flask=detail and detail.summary and detail.summary.itemId==row.itemId and detail.item and detail.item.container
   if flask and flask.containerType=='potion'and e.GuiColorSetForNextWidget then local color=tonumber(flask.color);if color then e.GuiColorSetForNextWidget(gui,(color%256)/193,(math.floor(color/256)%256)/193.5,(math.floor(color/65536)%256)/193,1)end end
   if e.GuiImage then
    items.gui_sprites=items.gui_sprites or {};local path=M.gui_sprite(e,items.catalogue,row,items.gui_sprites);local w,h=20,20
    if e.GuiGetImageDimensions then local ok,a,b=pcall(e.GuiGetImageDimensions,gui,path,1);if ok and type(a)=='number'and type(b)=='number'and a>0 and b>0 and a<4096 and b<4096 then w,h=a,b end end
    local scale=math.min(1,16/w,16/h);decoration();e.GuiImage(gui,icon_id,px+2+(16-w*scale)/2,py+2+(16-h*scale)/2,path,available and 1 or 0.35,scale,scale)
   end

   if row.usesRemaining and row.usesRemaining~='-1'then layer(6);decoration();e.GuiText(gui,px+2,py+10,row.usesRemaining)end
  end
 end
 if not hovered then items.inspect_id=nil;if items.cancel_inspect then items:cancel_inspect()end end
 layer(9);local pages=bank and bank.pageCount or 1;local page=model.page
 for _,b in ipairs({{0,'<<',-10,'back10.png'},{30,'<',-1,'back.png'},{150,'>',1,'forward.png'},{180,'>>',10,'forward10.png'}})do local clicked;if e.GuiImageButton then clicked=e.GuiImageButton(gui,id(),x+b[1],y+183,'',art..b[4])else clicked=button(x+b[1],y+183,b[2])end;if clicked then model.page=math.max(0,math.min(pages-1,page+b[3]));items.query_needed=true end end
 layer(9);e.GuiText(gui,x+65,y+185,tostring(page+1)..' / '..tostring(pages))
 layer(9);e.GuiText(gui,x,y+198,items.loading and 'Loading bank...'or (items.status or items.view.status or ''))
 if items.view.refresh_status then layer(9);e.GuiText(gui,x,y+209,items.view.refresh_status)end
 layer(0)
end
return M
