local root='mods/noita_together2/files/race/'
local catalogue=dofile(root..'catalogue.lua')
local verified={};local ok,sha=pcall(dofile,root..'sha256.lua')
for path,pin in pairs(catalogue.sourceHashes)do
 local read,bytes=pcall(ModTextFileGetContent,path)
 verified[path]=ok and read and type(bytes)=='string'and sha(bytes)==pin or false
end
local ending='data/entities/animals/boss_centipede/ending/sampo_start_ending_sequence.lua'
if verified[ending]then
 local source=ModTextFileGetContent(ending)
 local needle='EntityLoad( "data/entities/animals/boss_centipede/ending/gold_effect.xml", x, y )'
 local at=source:find(needle,1,true)
 if at and not source:find(needle,at+1,true)then
  -- Candidate first, Classic only at the verified normal gold-effect branch.
  local prefix="\nlocal nt2_race_api=dofile('mods/noita_together2/files/race/hook_api.lua')\n"
  source=source:sub(1,at-1).."nt2_race_api.terminal('ending',true)\n"..source:sub(at)
  source=source:gsub('GameOnCompleted%(%)',"GameOnCompleted()\n\tnt2_race_api.terminal('ending',false)",1)
  source=source:gsub('GameClearOrbsFoundThisRun%(%)',"nt2_race_api.terminal('ending',false)\n\t\tGameClearOrbsFoundThisRun()",1)
  ModTextFileSetContent(ending,prefix..source)
 else verified[ending]=false end
end
ModLuaFileAppend('data/scripts/items/orb_pickup.lua',root..'append/orb.lua')
for script in pairs(catalogue.bossHooks)do ModLuaFileAppend(script,root..'append/boss.lua')end
-- Fixed local capability facts; never infer support merely from a goal label.
NT2_RACE_DETECTORS={classic=verified[ending],map=verified['data/scripts/biome_map.lua']and verified['data/biome_impl/biome_map.png']}
return NT2_RACE_DETECTORS
