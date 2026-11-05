-- Read-only canonical results. Hiding the panel never changes capture.
local M={}
local catalogue=dofile('mods/noita_together2/files/race/catalogue.lua')
local labels={reach_max_hp='Maximum HP',accumulate_gold='Gold held',collect_orbs='Distinct orbs',defeat_bosses='Different bosses',defeat_named_bosses='Named bosses',visit_biome='Reach biome',visit_parallel_world='Parallel World',classic_ending='Classic Ending'}
local function literal(v)return ' '..tostring(v or''):gsub('_',' ')end
local function clock(ms)local seconds=math.floor((tonumber(ms)or 0)/1000);return string.format('%02d:%02d',math.floor(seconds/60),seconds%60)end
function M.draw(e,gui,id,view,width,height,left_clearance)
 local snapshot=view and view.snapshot;if type(snapshot)~='table'or type(snapshot.config)~='table'then return end
 local own
 for _,row in ipairs(snapshot.entrants or{})do if row.rowKey==snapshot.selfRow then own=row;break end end
 local duration=tonumber(snapshot.config.durationMs)or 0;local elapsed=tonumber(snapshot.elapsedMs)or 0
 if snapshot.status~='ended'and view.localElapsedAt and e.GameGetRealWorldTimeSinceStarted then
  local now=e.GameGetRealWorldTimeSinceStarted();if type(now)=='number'and now==now and now>=0 and now<=1000000000 then elapsed=elapsed+math.max(0,math.floor(now*1000)-view.localElapsedAt)end
 end
 local timer=duration>0 and('Remaining '..clock(math.max(0,duration-elapsed)))or('Elapsed '..clock(elapsed))
 local lines={}
 local function line(text,indent)lines[#lines+1]={text=text,indent=indent or 0}end
 local progress={};for _,p in ipairs(own and own.progress or{})do progress[p.instanceId]=p end
 for _,goal in ipairs(snapshot.config.goals or{})do
  local p=progress[goal.instanceId]or{};local name=labels[goal.id]or goal.id
  if goal.biomeId then name=name..': '..(catalogue.biomeNames[goal.biomeId]or literal(goal.biomeId))end
  if goal.direction then name=name..':'..literal(goal.direction)..' '..tostring(goal.index)end
  local value=p.value or'0';local target=p.target or goal.target
  line((p.complete and'[x] 'or'[ ] ')..name..(target and(' '..value..' / '..target)or''))
  if goal.bossIds then local credited={};for _,key in ipairs(p.credited or{})do credited[key]=true end;for _,key in ipairs(goal.bossIds)do line((credited[key]and'[x] 'or'[ ] ')..(catalogue.bossNames[key]or literal(key)),6)end end
 end
 line(view.standingsUpdating and'Standings (updating)'or'Standings')
 if view.standingsUnavailable then line('Standings unavailable')elseif view.standingsComplete==false then line('Updating standings...')else
  for _,row in ipairs(view.standings or{})do
   line((row.rank and row.rank>0 and('#'..row.rank..' ')or'')..tostring(row.completedGoals or 0)..'/'..tostring(view.goalTotal or#(snapshot.config.goals or{}))..' '..tostring(row.displayName or'Player')..' '..tostring(row.status or''))
  end
 end
 local title='Race - '..timer..' - '..tostring(own and own.status or snapshot.status or'waiting')
 local function dimensions(text)local w,h;if e.GuiGetTextDimensions then w,h=e.GuiGetTextDimensions(gui,text)end;return w or #text*5,h or 9 end
 local function trim(text)local i=#text;while i>0 and text:byte(i)>=128 and text:byte(i)<192 do i=i-1 end;return text:sub(1,i-1)end
 local max_width=math.min(180,math.max(40,width-(left_clearance or 0)-14));local panel_width=dimensions(title)+4;local _,line_height=dimensions('Race');line_height=math.max(9,line_height+1)
 for _,row in ipairs(lines)do panel_width=math.max(panel_width,dimensions(row.text)+row.indent+4)end
 panel_width=math.min(max_width,panel_width);local x=math.max(5,width-panel_width-6);local y=64
 local function text(tx,ty,label,available)
  local shown=label;while dimensions(shown)>available and #shown>3 do shown=trim(shown);end
  if shown~=label then while dimensions(shown..'...')>available and #shown>0 do shown=trim(shown)end;shown=shown..'...'end
  if e.GuiColorSetForNextWidget then e.GuiColorSetForNextWidget(gui,1,1,1,1)end
  e.GuiText(gui,tx,ty,shown);if label=='Standings'and e.GuiTooltip then e.GuiTooltip(gui,'Canonical standings','Equal goal counts and times share a rank.')end;if shown~=label and e.GuiTooltip then e.GuiTooltip(gui,label,'')end
 end
 text(x,y,title,panel_width)
 local content_height=#lines*line_height+4;local body_y=y+line_height+2;local body_height=math.min(content_height,126,math.max(12,height-body_y-22))
 -- The final scroll arguments are gamepad focus/margins, not background flags.
 -- Apply the documented engine option only to the container; labels stay opaque.
 if e.GuiOptionsAddForNextWidget and e.GUI_OPTION and e.GUI_OPTION.DrawSemiTransparent then e.GuiOptionsAddForNextWidget(gui,e.GUI_OPTION.DrawSemiTransparent)end
 if e.GuiColorSetForNextWidget then e.GuiColorSetForNextWidget(gui,1,1,1,0.22)end
 e.GuiBeginScrollContainer(gui,id(),x,body_y,panel_width,body_height,false,1,1)
 for i,row in ipairs(lines)do text(row.indent+2,(i-1)*line_height+2,row.text,panel_width-row.indent-8)end
 e.GuiEndScrollContainer(gui)
 return {x=x,y=y,width=panel_width,height=body_height,contentHeight=content_height,backgroundAlpha=0.22}
end
return M
