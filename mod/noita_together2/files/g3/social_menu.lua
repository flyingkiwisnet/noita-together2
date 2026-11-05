-- Centered legacy hold/select menu; presentation and transport live elsewhere.
local M={};local catalogue=dofile('mods/noita_together2/files/g3/social_catalogue.lua')
function M.new(e,controls,callbacks)
 local self={gui=nil,open=false,locked=false,last_reason=nil}
 local function sound(name,player)if e.GamePlaySound and player then local x,y=e.EntityGetTransform(player);e.GamePlaySound('data/audio/Desktop/ui.bank','ui/inventory_'..name,x,y)end end
 function self:hide(player)
  if self.open then self.open=false;sound('close',player)end
  if self.gui then e.GuiDestroy(self.gui);self.gui=nil end
 end
 function self:tick(player,playing,enabled,binding,blocked)
  local down,reason=controls:down(binding)
  if reason and reason~=self.last_reason then self.last_reason=reason;if callbacks.feedback then callbacks.feedback(reason)elseif callbacks.notice then callbacks.notice(reason)end end
  if not down then self.locked=false end
  if not playing or not enabled or blocked then self:hide(player);if down then self.locked=true end;return down end
  if not down or self.locked then self:hide(player);return down end
  if not self.open then if callbacks.before_open then callbacks.before_open()end;self.open=true;sound('open',player)end
  if not self.gui then self.gui=e.GuiCreate()end;local gui=self.gui;e.GuiStartFrame(gui)
  if e.GuiOptionsAdd then e.GuiOptionsAdd(gui,e.GUI_OPTION and e.GUI_OPTION.NoPositionTween or 6)end
  local width,height=e.GuiGetScreenDimensions(gui);local start_x=width/2-100;local start_y=height/2+20
  for index,id in ipairs(catalogue.ids)do
   local x=start_x+((index-1)%10)*20;local y=start_y+math.floor((index-1)/10)*20;local spec=catalogue.entries[id]
   if e.GuiZSetForNextWidget then e.GuiZSetForNextWidget(gui,8)end
   if e.GuiImageButton(gui,index,x,y,'',spec.icon)then
    if callbacks.select(id)then self.locked=true;self:hide(player);return down end
   end
   if e.GuiTooltip then e.GuiTooltip(gui,spec.name,'')end
  end
  for index,robe in ipairs(catalogue.robes)do
   if e.GuiZSetForNextWidget then e.GuiZSetForNextWidget(gui,8)end
   if e.GuiImageButton(gui,100+index,width/2-30+(index-1)*10,start_y+40,'',catalogue.root..'icons/color_'..robe..'.png')then callbacks.robe(robe)end
   if e.GuiTooltip then e.GuiTooltip(gui,robe:sub(1,1):upper()..robe:sub(2)..' Robes','')end
  end
  return down
 end
 function self:close(player)self:hide(player);self.locked=true end
 return self
end
return M
