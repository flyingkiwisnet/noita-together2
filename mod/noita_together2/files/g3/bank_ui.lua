-- Bank presentation/input is separate from the wallet effect handler.
local M={}
function M.new(send,identity)
 local self={amount='1',status='Bank unavailable.',view={},pending=nil,open=false}
 function self:receive(view)
  self.view=view;local submission=view.submission
  if submission and submission.id==self.pending then
   if submission.status=='accepted'then self.pending=nil;self.amount='1'
   elseif submission.status=='refused'then self.pending=nil;self.status=view.status or 'Gold transfer refused.'
   elseif submission.status=='unresolved'then self.status='Result unresolved. The original is retained; do not repeat it.'end
  end
  if not self.pending and view.status and view.status~=''then self.status=view.status end
 end
 function self:submit(direction,wallet)
  if self.pending then self.status='Waiting for the original operation.';return false end
  local amount=tonumber(self.amount)
  if not self.amount:match('^%d+$')or not amount or amount<1 or amount>1000000 or amount~=math.floor(amount)then self.status='Enter an amount from 1 to 1000000.';return false end
  if self.view.balance==nil then self.status='Bank unavailable. Waiting for a confirmed balance.';return false end
  if not self.view.enabled then self.status='Send Gold is disabled.';return false end
  if direction=='deposit'and (wallet==nil or wallet<amount)then self.status='Not enough gold in your wallet.';return false end
  if direction=='withdraw'and (self.view.balance==nil or self.view.balance<amount)then self.status='Not enough gold in the bank.';return false end
  local id=identity();if not id then self.status='Bank controls are unavailable.';return false end
  local accepted=send('{"kind":"submit","id":"'..id..'","direction":"'..direction..'","amount":'..amount..'}')
  if accepted~=true then self.status='Bank controls are busy. Try again.';return false end
  self.pending=id;self.status='Submitting gold operation.';return true
 end
 function self:draw(e,gui,id,button,wallet,width,height)
  local x,y=math.max(8,width/2-120),self.items and math.max(8,height/2-165)or math.max(80,height/2-50)
  -- Match the legacy bank: larger Z is deeper, opaque panel10/content9.
  -- Reassert BOTH shared and one-widget depth at every draw boundary, since
  -- the shared button callback also draws a tooltip. Hover state must neither
  -- select the panel's layer nor leak into the next label/input/button.
  local function layer(z)
   if e.GuiZSet then e.GuiZSet(gui,z)end
   if e.GuiZSetForNextWidget then e.GuiZSetForNextWidget(gui,z)end
  end
  if e.GuiImageNinePiece then layer(10);e.GuiImageNinePiece(gui,id(),x-5,y-5,240,self.items and 326 or 100,1,'mods/noita_together2/files/g3/bank_outer.png')end
  layer(9);e.GuiText(gui,x,y,'BANK   Gold: '..(self.view.balance~=nil and tostring(self.view.balance)or 'unavailable')..(self.view.fresh and ''or ' (stale)'))
  layer(9);e.GuiText(gui,x,y+12,'Wallet: '..(wallet~=nil and tostring(wallet)or 'unavailable')..'   Queued: '..tostring(self.view.queued or 0))
  layer(9);e.GuiText(gui,x,y+25,'Amount')
  -- Reserve the amount ID while pending so the later grid/actions stay stable.
  local amount_id=id()
  if e.GuiTextInput and not self.pending then layer(9);local v=e.GuiTextInput(gui,amount_id,x+42,y+23,self.amount,120,10,'0123456789');if type(v)=='string'then self.amount=v end end
  layer(9);if button(x,y+40,'TAKE')then self:submit('withdraw',wallet)end
  layer(9);if button(x+70,y+40,'DEPOSIT')then self:submit('deposit',wallet)end
  layer(9);e.GuiText(gui,x,y+56,self.status)
  if self.view.refresh_status then layer(9);e.GuiText(gui,x,y+67,self.view.refresh_status)end
  if self.items then dofile('mods/noita_together2/files/g3/item_presenter.lua').draw(e,gui,id,self.items,x,y+80,layer)end
  layer(9);if button(x,y+(self.items and 307 or 83),'Close')then self.open=false end
  layer(0) -- Restore the shared renderer's frame-default depth after tooltips.
 end
 return self
end
return M
