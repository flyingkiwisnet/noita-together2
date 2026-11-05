-- Installed API documentation specifies camera defaults; use player coordinates.
local repo=assert(os.getenv('NT2_A2_REPO'));local raw=dofile
dofile=function(path)return raw(repo..'/'..path:gsub('^mods/','mod/'))end
local capture=dofile('mods/noita_together2/files/g3/a2_capture.lua')
local observed='not called';local value='$biome_coalmine';local position=true
local e={EntityGetIsAlive=function()return true end,EntityHasTag=function()return false end,
 EntityGetTransform=function()if position then return 500,1200 end end,
 BiomeMapGetName=function(x,y)assert(x==500 and y==1200,'query biome at observed player position, not camera defaults');observed=value;return value end}
assert(capture.sample(e,99).biome=='$biome_coalmine' and observed==value)
value='_EMPTY_';assert(capture.sample(e,99).biome=='_EMPTY_','observed empty biome replaces old name with unknown, no invented Mountain')
value=nil;assert(capture.sample(e,99).biome==nil,'missing biome remains unknown')
value='$biome_holymountain';assert(capture.sample(e,99).biome==value,'real named biome retained')
position=false;observed='not called';assert(capture.sample(e,99).biome==nil and observed=='not called','unavailable player position must not sample camera')
e.BiomeMapGetName=function()error('unavailable')end;position=true;assert(capture.sample(e,99).biome==nil)
print('PASS production biome capture uses observed player coordinates; empty/missing/failed observations stay unknown; no engine evidence claimed')
