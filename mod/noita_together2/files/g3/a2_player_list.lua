-- Adapted from Noita Together core ui.lua draw_player_info/draw_player_list,
-- Copyright (c) 2022 Soler91, MIT; full notice: ui/biomes/LICENSE.
-- Presentation only: roster/HUD retention, current poses and follow belong to
-- the caller. No legacy transport, global PlayerList or inventory preview.
local M={}
local root='mods/noita_together2/files/g3/'
local biomes=dofile(root..'a2_biomes.lua')
local function number(v)return type(v)=='number'and v==v and v>-math.huge and v<math.huge end
function M.draw(e,gui,id,rows,facts,follow,translated,preview)
 local function layer(z)
  if e.GuiZSet then e.GuiZSet(gui,z)end
  if e.GuiZSetForNextWidget then e.GuiZSetForNextWidget(gui,z)end
 end
 local function same_line()e.GuiOptionsAddForNextWidget(gui,e.GUI_OPTION.Layout_NextSameLine)end
 layer(10);e.GuiBeginScrollContainer(gui,id(),5,50,100,150,false,1,1)
 e.GuiLayoutBeginVertical(gui,0,0)
 for _,row in ipairs(rows)do
  local h,depth=facts(row);local hp,max=h and h.hp,h and h.max
  local known=number(hp)and hp>=0 and number(max)and max>0
  if h and h.sampo==true then same_line();layer(9);e.GuiImage(gui,id(),88,0,root..'ui/sampo.png',0.5,1,1)end
  local icon=h and biomes.icon(h.biome)
  if icon then same_line();layer(9);e.GuiImage(gui,id(),80,0,root..'ui/biomes/'..icon,0.8,1,1)end
  -- Keep the current literal-name prefix so a name starting with '$' cannot
  -- select a translation key. The rest follows the legacy row geometry.
  layer(10);local caption=' '..row.name..(type(row.race_count)=='number'and(' ['..row.race_count..']')or'');local clicked,right=e.GuiButton(gui,id(),0,0,caption);if clicked and row.actor then follow(row.actor)end
  local hovered=false;if e.GuiGetPreviousWidgetInfo then local _,_,h=e.GuiGetPreviousWidgetInfo(gui);hovered=h end
  local previewed=preview and row.actor and preview(row,right==true,hovered==true)
  local state=({[1]='playing',[2]='waiting',[3]='disconnected',[4]='respawning'})[row.status]or row.status or 'unknown'
  if row.race_status then state=state..(row.race_connection_stale and' (last known)'or'')..' / Race '..row.race_status end
  local tooltip=(known and string.format('HP %.1f / %.1f',hp/1000*25,max/1000*25)or 'HP unknown')
   ..'\nLocation: '..translated(h and h.biome or '')
   ..'\nDepth: '..(number(depth)and string.format('%.0fm',depth/10)or 'unknown')
   ..'\nStatus: '..state..((not h or h.sampo==nil)and '\nSampo unknown'or '')..(row.readonly and '\nLobby member; gameplay details unavailable before Start.'or preview and '\nRight click: toggle wand preview.'or '')
  if e.GuiTooltip and not previewed then layer(8);e.GuiTooltip(gui,' '..row.name,tooltip)end
  if known then
   local fraction=math.max(0,math.min(1,hp/max))
   if fraction>0 then same_line();layer(9);e.GuiImage(gui,id(),0,0,root..'ui/hpbar_full.png',1,fraction,1)end
   layer(10);e.GuiImage(gui,id(),0,0,root..'ui/hpbar_empty.png',1,1,1)
  end
  e.GuiLayoutAddVerticalSpacing(gui,2)
 end
 e.GuiLayoutEnd(gui);layer(10);e.GuiEndScrollContainer(gui)
 layer(0) -- Do not leak list/tooltip depth into radar, composer or bank.
end
return M
