-- Labelled wallet/outlet observations only. All permissions, journal phases,
-- transport and ledger decisions use production Client/native/mod/Server code.
local M={}
function M.item_cell_hit(plan,x,y,label)
 return type(plan)=='table'and type(plan.x)=='number'and type(plan.y)=='number'and plan.x>=0 and plan.x<=8192 and plan.y>=0 and plan.y<=8192 and plan.x>=x and plan.x<x+20 and plan.y>=y and plan.y<y+20 and label==plan.label
end
-- A private item fault binds only the first matching deposit original, or an
-- explicitly named result. Inventory/other items keep using the same mailbox.
function M.item_copy_gate()
 local signature,target
 return function(plan,message)
  if not plan then signature=nil;target=nil;return false end
  assert(type(plan)=='table'and type(plan.id)=='string'and (plan.kind=='item_chunk'and plan.channel=='deposit'or plan.kind=='item_result'and plan.id~='next'and plan.channel==nil),'scoped item copy fault required')
  local key=plan.kind..':'..(plan.channel or '')..':'..plan.id
  if key~=signature then signature=key;target=plan.id~='next'and plan.id or nil end
  if type(message)~='table'or message.kind~=plan.kind or plan.channel and message.channel~=plan.channel then return false end
  if not target then assert(type(message.id)=='string','original identity required');target=message.id end
  return message.id==target
 end
end
function M.new(e,state,forms,input,mark)
 local mailbox=dofile('mods/noita_together2/files/g3/bank_json.lua');local item_gate=M.item_copy_gate()
 local base={};for _,key in ipairs({'EntityGetFirstComponentIncludingDisabled','ComponentGetValue2','ComponentSetValue2','EntityGetWithTag','EntityGetIsAlive','EntityGetTransform','EntityLoad','GuiImageButton','GuiTextInput'})do base[key]=e[key]end
 local wallet=1000;local writes,pink=0,0;local component=790001;local outlet=790002
 local function current()return forms and forms:current()or 99 end
 e.EntityGetFirstComponentIncludingDisabled=function(id,kind,...)if id==current()and kind=='WalletComponent'then if input('b1-no-wallet',false)then return nil end;return component end;return base.EntityGetFirstComponentIncludingDisabled(id,kind,...)end
 e.ComponentGetValue2=function(id,key,...)if id==component and key=='money'then return wallet end;return base.ComponentGetValue2(id,key,...)end
 e.ComponentSetValue2=function(id,key,value,...)
  if id==component and key=='money'then if input('b1-write-fail',false)then return false end;assert(type(value)=='number'and value==math.floor(value)and value>=0 and value<=2147483647,'invalid injected wallet write');wallet=value;writes=writes+1;return end
  return base.ComponentSetValue2(id,key,value,...)
 end
 e.EntityGetWithTag=function(tag)if tag=='nt2_b1_outlet'then return {outlet}end;return base.EntityGetWithTag and base.EntityGetWithTag(tag)or {}end
 e.EntityGetIsAlive=function(id)if id==outlet then return true end;return base.EntityGetIsAlive(id)end
 e.EntityGetTransform=function(id)if id==outlet then local x,y=base.EntityGetTransform(current());return x+(input('b1-outlet',false)and 0 or 100),y,0 end;return base.EntityGetTransform(id)end
 e.EntityLoad=function(path,x,y)if path=='data/entities/particles/poof_pink.xml'then pink=pink+1;return 790003 end;return base.EntityLoad(path,x,y)end
 e.GuiImageButton=function(gui,id,x,y,text,path)if path:match('/bank.png$')or path:match('/fridge.png$')then if input('a2-click',false)=='Gold bank'then input('a2-click',true);mark('a2-clicked','Gold bank');return true end end;return base.GuiImageButton(gui,id,x,y,text,path)end
 -- Target the actual numeric amount widget. A frame may already have drawn
 -- it when the driver writes, so the later item-search input must not eat it.
 e.GuiTextInput=function(gui,id,x,y,value,width,length,allowed)
  if width==120 and length==10 and allowed=='0123456789'then return input('b1-amount',true)or value end
  return base.GuiTextInput(gui,id,x,y,value,width,length,allowed)
 end
 local function quote(s)return '"'..s:gsub('[%z\1-\31\\"]',function(c)if c=='"'then return '\\"'elseif c=='\\'then return '\\\\'else return string.format('\\u%04x',c:byte())end end)..'"'end
 local function json(v)
  if v==nil then return 'null'elseif type(v)=='boolean'or type(v)=='number'then return tostring(v)elseif type(v)=='string'then return quote(v)end
  local out={};for k,value in pairs(v)do out[#out+1]=quote(tostring(k))..':'..json(value)end;return '{'..table.concat(out,',')..'}'
 end
 local self={item_cell_hit=M.item_cell_hit}
 function self:block_item_copy(body)
  local configured=input('items-copy-block',false);local plan=configured and assert(mailbox.decode(configured))or nil
  return item_gate(plan,mailbox.decode(body))
 end
 function self:snapshot(bank)
  if bank and not self.observed_submit then self.observed_submit=true;local submit=bank.ui.submit;bank.ui.submit=function(ui,direction,wallet)local result=submit(ui,direction,wallet);mark('b1-last-gesture.json',json({direction=direction,accepted=result,pending=ui.pending,status=ui.status}));return result end end
  local injected=input('b1-set-wallet',true);if injected then wallet=assert(tonumber(injected))end
  if bank then mark('b1-view.json',json({frame=state.frame,wallet=wallet,writes=writes,pink=pink,near=input('b1-outlet',false)~=nil,open=bank.ui.open,amount=bank.ui.amount,pending=bank.ui.pending,status=bank.ui.status,view=bank.last,engineInputsInjected=true,savePersistenceProven=false}))end
 end
 return self
end
return M
