-- Fixture-only data transport. Read once before injected game-thread callbacks.
-- Fixed bounded records, hex literal text; no network text is evaluated as Lua.
local f=assert(io.open(assert(os.getenv('NT2_G3_SOCIAL_INPUT')),'rb'))
local data=f:read(4097);f:close();assert(#data<=4096)
local binary_file=assert(io.open(assert(os.getenv('NT2_G3_SOCIAL_FRAME')),'rb'))
local binary=binary_file:read(32769);binary_file:close();assert(#binary<=32768)
local next_file=assert(io.open(assert(os.getenv('NT2_G3_SOCIAL_NEXT_FRAME')),'rb'))
local next_binary=next_file:read(32769);next_file:close();assert(#next_binary<=32768)
local rows={}
for line in data:gmatch('[^\n]+') do
 local row={};for field in (line..'\t'):gmatch('(.-)\t') do row[#row+1]=field end;rows[#rows+1]=row
end
assert(#rows==3 and rows[1][1]=='bind' and rows[2][1]=='hud' and rows[3][1]=='poi')
local scope,generation,actor=rows[1][2],tonumber(rows[1][3]),rows[1][4]
local root=assert(os.getenv('NT2_G3_MODULES'))
local validation=dofile(root..'/validation.lua')
local module=dofile(root..'/hud_poi_local.lua')
local seconds=0;local entities={};local entity_count=0;local drawn={};local bar_scale=nil
local engine={GameGetRealWorldTimeSinceStarted=function() return seconds end,
 EntityLoad=function(path,x,y) assert(path=='mods/noita_together2/files/g3/poi.xml');entity_count=entity_count+1;entities[entity_count]={x=x,y=y};return entity_count end,
 EntityGetIsAlive=function(id) return entities[id]~=nil end,EntityKill=function(id) entities[id]=nil end,
 EntityGetFirstComponentIncludingDisabled=function(id,kind) assert(kind=='ItemComponent');return id end,
 ComponentSetValue2=function(id,key,value) entities[id][key]=value end,
 GuiCreate=function() return 1 end,GuiDestroy=function() end,GuiStartFrame=function() drawn={} end,
 GuiButton=function(gui,id,x,y,label) return false end,
 GuiText=function(gui,x,y,text) drawn[#drawn+1]=text end,
 GuiImage=function(gui,id,x,y,path,alpha,sx) assert(path:match('^mods/noita_together2/files/g3/ui/'));if path:find('hpbar_full',1,true) then bar_scale=sx end end}
local renderer=dofile(root..'/social_game_api.lua').new(engine,validation)
local name_hex=assert(rows[1][5]);assert(#name_hex<=792 and #name_hex%2==0 and not name_hex:find('[^a-f0-9]'))
local display_name=name_hex:gsub('..',function(pair) return string.char(tonumber(pair,16)) end)
assert(renderer:set_peer(actor,display_name,tonumber(rows[1][6])));renderer.show_players=true;renderer.show_pois=true
local state=module.new(renderer.api,validation);renderer:set_dismiss(function(sequence) return state:dismiss(sequence) end);assert(state:bind(scope,generation,{actor}))
local h=rows[2];assert(#h==6)
local p=rows[3];assert(#p==8 and #p[8]<=792 and #p[8]%2==0 and not p[8]:find('[^a-f0-9]'))
local text=p[8]:gsub('..',function(pair) return string.char(tonumber(pair,16)) end)
local marker={sequence=p[4],actor=p[5],name=display_name,x=tonumber(p[6]),y=tonumber(p[7]),text=text}
local lease={context=scope:gsub('..',function(pair) return string.char(tonumber(pair,16)) end),generation=generation,owner='injected-native-owner',now=0,until_time=1,playing=true}
local dispatch=dofile(root..'/social_dispatch.lua').new(function() return lease end,function(commands,now)
 assert(state:bind(scope,generation,{actor}))
 for _,c in ipairs(commands) do
  if c.tag==1 then assert(renderer:set_peer(c.actor,c.name,tonumber(rows[1][6])));assert(state:receive_hud(scope,generation,c.actor,c.sequence,c.hp,c.max_hp,c.sampo))
  elseif c.tag==2 then assert(state:remove_live(scope,generation,c.actor));renderer.peers[c.actor]=nil
  elseif c.tag==3 then assert(state:begin_snapshot(scope,generation,c.high))
  elseif c.tag==4 then for _,m in ipairs(c.markers) do assert(state:admit_marker_actor(scope,generation,m.actor)) end;assert(state:page(scope,generation,c.high,c.next,c.markers))
  else error('fixture command') end
 end;return true
end,function() state:pause() end)
assert(dispatch:receive(binary));assert(not dispatch:receive(binary))
assert(renderer.hud[actor].hp==3000 and renderer.hud[actor].max==2000 and renderer.hud[actor].sampo)
renderer:tick(true);assert(renderer.pois[p[4]].text=='literal 雪 <img onerror=literal>' and bar_scale==1 and entity_count==1)
assert(table.concat(drawn,'|'):find('HP 3.0 / 2.0',1,true));assert(entities[1].ui_description==' literal 雪 <img onerror=literal>')
assert(table.concat(drawn,'|'):find(' '..display_name,1,true))
assert(entities[1].item_name==' '..display_name)
assert(dispatch:receive(next_binary));renderer:tick(true);assert(not renderer.hud[actor] and not renderer.peers[actor] and entities[2].item_name==' '..display_name and not entities[1])
assert(not dispatch:receive(binary));assert(not state:receive_hud(scope,generation,actor,tonumber(h[3]),1,2,false,1))
-- Actual adapter observes local pickup/removal before any network acknowledgement.
engine.EntityKill(2);renderer:tick(true);assert(not renderer.pois[p[4]] and state.dismissed[p[4]])
state:pause();assert(state:bind(scope,generation,{actor}))
assert(state:begin_snapshot(scope,generation,p[2]));assert(state:page(scope,generation,p[2],p[3],{marker}));assert(not renderer.pois[p[4]])
seconds=3;state:tick();assert(not renderer.hud[actor])
assert(not state:begin_snapshot(scope,generation+1,p[2]));assert(not state.playing and not next(renderer.pois))
state:close();renderer:tick(false);assert(not renderer.gui and not next(entities));print('PASS Core HTTPS trusted Noita GUI/entity adapter HP POI pickup dismissal hold retained-state generation cleanup')
