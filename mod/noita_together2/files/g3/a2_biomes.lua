-- Fixed legacy biome identifiers; values never select a remote asset.
local M={
 ["$biome_coalmine"]="$biome_coalmine",
 ["$biome_coalmine_alt"]="$biome_coalmine_alt",
 ["$biome_excavationsite"]="$biome_excavationsite",
 ["$biome_fungicave"]="$biome_fungicave",
 ["$biome_rainforest"]="$biome_rainforest",
 ["$biome_snowcave"]="$biome_snowcave",
 ["$biome_snowcastle"]="$biome_snowcastle",
 ["$biome_vault"]="$biome_vault",
 ["$biome_crypt"]="$biome_crypt",
 ["$biome_holymountain"]="$biome_holymountain",
 ["$biome_boss_victoryroom"]="$biome_boss_victoryroom",
 ["$biome_boss_arena"]="$biome_boss_arena",
 ["$biome_desert"]="$biome_desert",
 ["$biome_dragoncave"]="$biome_dragoncave",
 ["$biome_gold"]="$biome_gold",
 ["$biome_lake"]="$biome_lake",
 ["$biome_sandcave"]="$biome_sandcave",
 ["$biome_tower"]="$biome_tower",
 ["$biome_vault_frozen"]="$biome_vault_frozen",
 ["$biome_clouds"]="$biome_clouds",
 ["$biome_liquidcave"]="$biome_liquidcave",
 ["$biome_secret_lab"]="$biome_secret_lab",
 ["$biome_orbroom"]="$biome_orbroom",
 ["$biome_wizardcave"]="$biome_wizardcave",
 ["$biome_rainforest_dark"]="$biome_rainforest_dark",
 ["$biome_mestari_secret"]="$biome_mestari_secret",
 ["$biome_ghost_secret"]="$biome_ghost_secret",
 ["$biome_winter_caves"]="$biome_winter_caves",
 ["$biome_the_end"]="$biome_the_end",
 ["$biome_the_end_sky"]="$biome_the_end_sky",
 ["$biome_wandcave"]="$biome_wandcave",
 ["$biome_winter"]="$biome_winter",
 ["$biome_fun"]="$biome_fun",
 ["$biome_robobase"]="$biome_robobase",
}

-- Fixed legacy biome_sprites map (Soler91, MIT; ui/biomes/LICENSE).
-- Mountain is accepted only from the existing exact starting-biome signal.
M.sprites={
 ["$nt2_starting_mountain"]="mountain.png",
 ["$biome_coalmine"]="coalmine.png",
 ["$biome_coalmine_alt"]="coalmine_alt.png",
 ["$biome_excavationsite"]="excavationsite.png",
 ["$biome_fungicave"]="fungicave.png",
 ["$biome_rainforest"]="rainforest.png",
 ["$biome_snowcave"]="snowcave.png",
 ["$biome_snowcastle"]="snowcastle.png",
 ["$biome_vault"]="vault.png",
 ["$biome_crypt"]="crypt.png",
 ["$biome_holymountain"]="holymountain.png",
 ["$biome_boss_victoryroom"]="the_work.png",
 ["$biome_boss_arena"]="laboratory.png",
 ["$biome_desert"]="desert.png",
 ["$biome_dragoncave"]="dragoncave.png",
 ["$biome_gold"]="the_gold.png",
 ["$biome_lake"]="lake.png",
 ["$biome_sandcave"]="sandcave.png",
 ["$biome_tower"]="tower.png",
 ["$biome_vault_frozen"]="vault_frozen.png",
 ["$biome_clouds"]="cloudscape.png",
 ["$biome_liquidcave"]="ancient_laboratory.png",
 ["$biome_secret_lab"]="alchemistboss.png",
 ["$biome_orbroom"]="orbroom.png",
 ["$biome_wizardcave"]="wizardcave.png",
 ["$biome_rainforest_dark"]="lukki.png",
 ["$biome_mestari_secret"]="wizardboss.png",
 ["$biome_ghost_secret"]="snowy_boss.png",
 ["$biome_winter_caves"]="snowy_chasm.png",
 ["$biome_the_end"]="hell_work.png",
 ["$biome_the_end_sky"]="sky_work.png",
 ["$biome_wandcave"]="wandcave.png",
 ["$biome_winter"]="winter.png",
 ["$biome_fun"]="fun.png",
 ["$biome_robobase"]="robobase.png",
}
function M.icon(key)return M.sprites[key]end

-- The vanilla entrance/tutorial biome is a precise location signal; no other
-- unnamed mountain/surface biome is inferred from coordinates or entry intent.
function M.starting(e,x,y)
 if type(e.DebugBiomeMapGetFilename)~='function'then return false end
 local ok,name=pcall(e.DebugBiomeMapGetFilename,x,y)
 return ok and name=='data/biome/mountain_left_entrance.xml'
end
function M.register(e)
 if type(e.ModTextFileGetContent)~='function'or type(e.ModTextFileSetContent)~='function'then return false end
 local file='data/translations/common.csv';local body=e.ModTextFileGetContent(file)
 if type(body)~='string'then return false end
 if not body:find('nt2_starting_mountain,',1,true)then
  e.ModTextFileSetContent(file,body..'\nnt2_starting_mountain,Mountain,Гора,Montanha,Montaña,Berg,Montagne,Montagna,Góra,山,山,산,,Starting Mountain only,\n')
 end;return true
end
return M
