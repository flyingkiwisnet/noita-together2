-- Production registration and actual installed-game source; injected engine APIs.
local repo=assert(os.getenv('NT2_RACE_REPO'));local game=repo..'/spikes/noita-entry/.build/race-goal-catalogue-20261105/game-reference/'
local checks=0;local function check(ok,label)assert(ok,label);checks=checks+1 end
local function read(path)local f=assert(io.open(path,'rb'));local s=f:read('*a');f:close();return s end
local ending_path='data/entities/animals/boss_centipede/ending/sampo_start_ending_sequence.lua'
local function register(altered)
 local e=setmetatable({},{__index=_G});e._G=e;local texts,hooks={},{}
 e.dofile=function(path)local f=assert(loadfile(repo..'/'..path:gsub('^mods/','mod/')));setfenv(f,e);return f()end
 e.ModTextFileGetContent=function(path)local value=texts[path]or read(game..path);return path==altered and(value..'\n-- changed')or value end
 e.ModTextFileSetContent=function(path,value)texts[path]=value end;e.ModLuaFileAppend=function(base,hook)hooks[#hooks+1]={base,hook}end
 local detectors=e.dofile('mods/noita_together2/files/race/register.lua')
 return detectors,texts,hooks
end
local detectors,texts,hooks=register();check(detectors.classic and detectors.map,'known source detectors supported');check(#hooks==11,'only fixed ten final boss scripts and orb hook')
for _,path in ipairs({ending_path,'data/scripts/biome_map.lua','data/biome_impl/biome_map.png'})do
 local d,t=register(path);check(path==ending_path and not d.classic and not t[ending_path]or path~=ending_path and not d.map,'changed source refuses detector '..path)
end
local function ending(orbs,underground,distance,greed,mountain,progression)
 local calls={};local function record(value)calls[#calls+1]=value end
 local e=setmetatable({},{__index=_G});e._G=e
 e.dofile=function(path)if path=='mods/noita_together2/files/race/hook_api.lua'then return{terminal=function(kind,classic)record(classic and'race_classic'or'race_other')end}end end;e.dofile_once=function()end;e.ORB_COUNT_IN_WORLD=11
 e.do_newgame_plus=function()record('ngplus')end;e.GetUpdatedEntityID=function()return 1 end
 e.EntityGetTransform=function(id)if id==2 then return distance or 0,0 end;return 0,0 end
 e.EntityGetWithTag=function(tag)if tag=='ending_sampo_spot_underground'then return underground and{2}or{}elseif tag=='ending_sampo_spot_mountain'then return mountain and{3}or{}elseif tag=='player_unit'then return{4}end;return{}end
 e.StatsGetValue=function()return'1'end;e.GameGetOrbCountThisRun=function()return orbs end;e.SessionNumbersGetValue=function()return tostring(progression or 0)end
 e.GameHasFlagRun=function(key)return key=='greed_curse'and greed or false end
 e.AddFlagPersistent=record;e.GameAddFlagRun=record;e.GameOnCompleted=function()record('vanilla_completed')end
 e.EntityLoad=function(path)record(path);return 50 end;e.EntityKill=function()record('entity_kill')end;e.EntityGetClosestWithTag=function()return 0 end
 e.GameClearOrbsFoundThisRun=function()record('clear_orbs')end;e.GameDoEnding2=function()record('ending2')end
 e.GameGetWorldStateEntity=function()return 90 end;e.EntityGetFirstComponent=function()return nil end
 e.GamePlaySound=function()end;e.EntityAddChild=function()end;e.ComponentSetValue=function()end;e.print=function()end
 local f=assert(loadstring(texts[ending_path],ending_path));setfenv(f,e);f()
 local positions={};for i,value in ipairs(calls)do positions[value]=i end;return positions
end
local gold='data/entities/animals/boss_centipede/ending/gold_effect.xml'
for _,greed in ipairs({false,true})do local p=ending(0,true,0,greed,false);check(p.race_classic and p.race_classic<p[gold]and p[gold]<p.entity_kill,'Classic recorded immediately before real gold-effect branch');check(p.vanilla_completed and p.race_other>p.vanilla_completed and p.race_other<p.race_classic,'generic completion remains only an upgradeable candidate')end
for _,case in ipairs({{11,true,0,false,false},{33,true,0,false,false},{0,false,0,false,true},{5,false,0,false,true},{0,true,32,false,false}})do local p=ending(unpack(case));check(not p.race_classic and not p[gold],'nongold/other/NG+ source branches excluded')end
local p=ending(0,true,31.999,false,false);check(p.race_classic~=nil,'strict underground Manhattan boundary retained')
local other=ending(5,false,0,false,true);check(other.race_other and other.race_other<other.clear_orbs and other.clear_orbs<other.ngplus,'unsupported NG+ is an Other ending before orb reset, not continued Race tracking')
print('Race production source registration PASS '..checks..' assertions; injected engine APIs, no Noita launch.')
