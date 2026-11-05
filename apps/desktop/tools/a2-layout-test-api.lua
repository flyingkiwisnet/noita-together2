-- Minimal injected layout API for tests of retained facts/input, not pixels or
-- scrolling. The focused player-list recording test supplies its own recorder.
return function(e)
 e.GUI_OPTION=e.GUI_OPTION or {};e.GUI_OPTION.Layout_NextSameLine=1
 for _,name in ipairs({'GuiOptionsAddForNextWidget','GuiBeginScrollContainer','GuiEndScrollContainer','GuiLayoutBeginVertical','GuiLayoutEnd','GuiLayoutAddVerticalSpacing'})do
  if not e[name]then e[name]=function()end end
 end
end
