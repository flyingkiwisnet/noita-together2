-- Injected engine facts only; capture, native transport, journal and service are real.
local M={}
function M.new(e,state,forms,input,mark,files)
 local repo=assert(os.getenv('NT2_C3_REPO'));local restricted=assert(loadfile(repo..'/apps/desktop/tools/c1-restricted-context.lua'))()
 local json=e.dofile('mods/noita_together2/files/g3/bank_json.lua');local catalogue=e.dofile('mods/noita_together2/files/race/catalogue.lua')
 local function read(path)local f=assert(io.open(path,'rb'));local value=f:read('*a');f:close();return value end
 local changed={};local original_text=e.ModTextFileGetContent
 e.ModTextFileGetContent=function(path)if changed[path]then return changed[path]end;if catalogue.sourceHashes[path]then return read(repo..'/spikes/noita-entry/.build/race-goal-catalogue-20261105/game-reference/'..path)end;local value=original_text(path);if path=='data/translations/common.csv'and value==''then return',en\n'end;return value end
 local set_text=e.ModTextFileSetContent;e.ModTextFileSetContent=function(path,value)if catalogue.sourceHashes[path]then changed[path]=value;return end;return set_text(path,value)end
 local function hook()return restricted(e,function(path)return changed[path]or read(files..'/'..path)end)end
 local function player()return forms and forms:current()or 99 end
 local rows,parts={},{};local serial=980000
 local first=e.EntityGetFirstComponentIncludingDisabled;local components=e.EntityGetComponentIncludingDisabled;local add=e.EntityAddComponent2;local alive=e.EntityGetIsAlive;local kill=e.EntityKill;local filename=e.EntityGetFilename;local get=e.ComponentGetValue2
 e.EntityGetFirstComponentIncludingDisabled=function(id,kind)if rows[id]then return(rows[id].parts[kind]or{})[1]end;return first(id,kind)end
 e.EntityGetComponentIncludingDisabled=function(id,kind)if rows[id]then return rows[id].parts[kind]or{}end;return components(id,kind)end
 e.EntityAddComponent2=function(id,kind,value)if rows[id]then serial=serial+1;parts[serial]=value;rows[id].parts[kind]=rows[id].parts[kind]or{};table.insert(rows[id].parts[kind],serial);return serial end;if id==player()and kind=='LuaComponent'then return 989999 end;return add(id,kind,value)end
 e.ComponentGetValue2=function(id,key)if parts[id]then return parts[id][key]end;return get(id,key)end
 e.EntityGetIsAlive=function(id)return rows[id]~=nil or alive(id)end;e.EntityGetFilename=function(id)return rows[id]and rows[id].file or filename(id)end
 e.EntityKill=function(id)if rows[id]then rows[id]=nil;return end;return kill(id)end
 local has=e.EntityHasTag;e.EntityHasTag=function(id,tag)if rows[id]then return false end;return has(id,tag)end
 local biome=e.BiomeMapGetName;e.BiomeMapGetName=function(...)return input('race-biome',false)or biome(...)end
 e.BiomeMapGetSize=function()return 70 end;e.GetParallelWorldPosition=function(x)return math.floor((x+17920)/35840)end
 local function entity(path)serial=serial+1;local id=serial;rows[id]={file=path,parts={}};return id end
 local actions,last=0,nil;local self={}
 function self:tick(bank)
  if not bank or not bank.race then return end
  local raw=input('race-input',true)
  if raw then local request=assert(json.decode(raw));assert(request.id and request.id~=last);last=request.id;actions=actions+1
   local h=hook();local api=h.dofile('mods/noita_together2/files/race/hook_api.lua')
   if request.kind=='max_hp'then state.components[2].max_hp=request.value/25;api.observe(h.dofile('mods/noita_together2/files/race/sample.lua').facts(h,player(),false))
   elseif request.kind=='gold'then local wallet=e.EntityGetFirstComponentIncludingDisabled(player(),'WalletComponent');e.ComponentSetValue2(wallet,'money',request.value);api.observe(h.dofile('mods/noita_together2/files/race/sample.lua').facts(h,player(),false))
   elseif request.kind=='orb'then
    local id=entity('data/entities/items/orbs/orb.xml');e.EntityAddComponent2(id,'OrbComponent',{orb_id=request.orbId});h.item_pickup=function(item)e.EntityKill(item)end
    h.dofile('mods/noita_together2/files/c1/append/orb_pickup.lua');h.dofile('mods/noita_together2/files/race/append/orb.lua');h.item_pickup(id,player(),'Injected physical orb')
   elseif request.kind=='boss'then
    local path;for file,key in pairs(catalogue.bosses)do if key==request.bossId then path=file end end
    local id=entity(assert(path));h.GetUpdatedEntityID=function()return id end;h.death=function()e.EntityKill(id)end;h.dofile('mods/noita_together2/files/race/append/boss.lua');h.death(0,'Injected final defeat',0,true)
   elseif request.kind=='biome'then mark('race-biome',request.biome);api.observe(h.dofile('mods/noita_together2/files/race/sample.lua').locations(h,player(),true))
   elseif request.kind=='parallel'then local x=35840*request.index*(request.direction=='east'and 1 or -1);mark('a2-x',tostring(x));mark('a2-y','100');e.EntitySetTransform(player(),x,100);api.observe(h.dofile('mods/noita_together2/files/race/sample.lua').locations(h,player(),true))
   elseif request.kind=='death'or request.kind=='finish_death'then
    if request.value then state.components[2].max_hp=request.value/25 end
    state.components[2].hp=-1;h.GetUpdatedEntityID=player;h.dofile('mods/noita_together2/files/race/player_death.lua');h.death();e.GameTriggerGameOver()
   elseif request.kind=='ending'then
    -- Labeled branch fact. Actual ending-script branch execution has its own source test.
    api.terminal('ending',request.classic==true)
   else error('Unknown fixed Race engine fixture input')end
   mark('race-last-input.json',assert(json.encode({id=request.id,kind=request.kind,actions=actions})))
  end
  local store=e.dofile('mods/noita_together2/files/race/storage.lua');local q=store.read(e,'outgoing')
  mark('race-view.json',assert(json.encode({ready=bank.race.ready,view=bank.race_view or{},queued=q and#q.queue or 0,closed=q and q.closed or false,terminal=q and q.terminal or'none',actions=actions,hp=tostring(state.components[2].hp),max=tostring(state.components[2].max_hp),wait=state.components[2].wait_for_kill_flag_on_death==true,gameOvers=e.game_overs or 0,injectedEngine=true,actualNoita=false})))
 end
 return self
end
return M
