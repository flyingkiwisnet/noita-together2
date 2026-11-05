-- Nightmare-style pickups from the proven complete fresh-entry transaction.
-- No saved/config/client flag classifies a world as New Game or grants authority.
local M={}
local root='mods/noita_together2/files/startup/'
local bootstrap=root..'testing-loadout.txt'
local key='nt2_testing_starting_loadout_v1'
local function hex(s)return(s:gsub('.',function(c)return string.format('%02x',c:byte())end))end
local function uuid(s)
 if type(s)~='string'or #s~=36 or s:sub(9,9)~='-'or s:sub(14,14)~='-'or s:sub(19,19)~='-'or s:sub(24,24)~='-'then return nil end
 local h=s:gsub('-','');if #h~=32 or not h:match('^[0-9a-f]+$')or h==string.rep('0',32)then return nil end
 return(h:gsub('..',function(v)return string.char(tonumber(v,16))end))
end
function M.new(e)
 local json=e.dofile('mods/noita_together2/files/g3/bank_json.lua')
 local actor=e.dofile('mods/noita_together2/files/g3/game_api.lua')
 local self={}
 local function fresh()
  local ok,list=pcall(e.ModLuaFileGetAppends,root..'scope_registry.lua')
  return ok and type(list)=='table'and #list==2 and list[1]==root..'scope_new.lua'and list[2]==root..'scope_spent.lua'
 end
 function self:saved()
  e.ModTextFileSetContent(bootstrap,'saved');assert(e.ModTextFileGetContent(bootstrap)=='saved','Testing loadout bootstrap reset failed')
 end
 function self:stage(complete)
  assert(fresh(),'Testing loadout needs the consumed fresh-entry registry')
  local snapshot=assert(json.decode(complete.snapshot,{bytes=16384,nodes=2048,depth=16}),'Invalid complete testing configuration')
  assert(snapshot.version==1 and type(snapshot.scope)=='table'and type(snapshot.config)=='table'and uuid(snapshot.local_world)==complete.world and snapshot.bundle_digest==hex(complete.digest)and snapshot.scope.config_digest==hex(complete.config_digest)and snapshot.generation==complete.generation and snapshot.seed==complete.seed and snapshot.policy==complete.policy,'Testing configuration differs from complete entry')
  local enabled=false;local seen=false
  for _,setting in ipairs(snapshot.config.settings or{})do
   if setting.key=='NT_testing_mode'then assert(not seen and(setting.value=='true'or setting.value=='false'),'Invalid frozen Testing mode');seen=true;enabled=setting.value=='true'end
  end
  local staged={world=snapshot.local_world,digest=snapshot.scope.config_digest,enabled=enabled}
  local encoded=assert(json.encode_saved(staged));e.ModTextFileSetContent(bootstrap,encoded);assert(e.ModTextFileGetContent(bootstrap)==encoded,'Testing loadout staging failed')
 end
 local function persist(state)e.GlobalsSetValue(key,assert(json.encode_saved(state)))end
 local function catalogue()
  local c=assert(json.decode(e.ModTextFileGetContent(root..'testing-starting-loadout.json')),'Testing catalogue unavailable')
  assert(c.revision=='nt2-testing-starting-loadout-1'and c.settingKey=='NT_testing_mode'and c.default==false,'Testing catalogue revision differs')
  local p=c.pickups;assert(p and p.tinkerPerk=='EDIT_WANDS_EVERYWHERE'and p.tinkerCount==1 and p.noMoreShufflePerk=='NO_MORE_SHUFFLE'and p.noMoreShuffleCount==1 and p.randomPerks==3 and p.randomWands==3 and p.wandSpacingPixels==44 and #p.wandIds==6 and #p.randomPerkExclusions==2 and p.randomPerkExclusions[1]==p.tinkerPerk and p.randomPerkExclusions[2]==p.noMoreShufflePerk,'Testing pickup catalogue differs')
  local expected={wand_level_02=true,wand_level_02_better=true,wand_level_03=true,wand_unshuffle_01=true,wand_unshuffle_02=true,wand_unshuffle_03=true};for _,id in ipairs(p.wandIds)do assert(expected[id],'Unknown testing wand');expected[id]=nil end
  return p
 end
 -- The helper keeps its usual SetRandomSeed/Random API and loot coordinate
 -- wrappers, but its own draws use procedural RNG and never reseed global RNG.
 local function isolated_rng(x,y,fn)
  local seed,random,randomf=e.SetRandomSeed,e.Random,e.Randomf;local sx,sy,n=x,y,0
  e.SetRandomSeed=function(a,b)sx,sy,n=a,b,0 end
  e.Random=function(a,b)n=n+1;if a==nil then return e.ProceduralRandomf(sx+n,sy,0,1)end;return e.ProceduralRandomi(sx+n,sy,b and a or 0,b or a)end
  e.Randomf=function(a,b)n=n+1;return e.ProceduralRandomf(sx+n,sy,b and a or 0,b or a or 1)end
  local ok,err=pcall(fn);e.SetRandomSeed,e.Random,e.Randomf=seed,random,randomf;if not ok then error(err,0)end
 end
 function self:spawn(player)
  if not fresh()then return 'saved' end
  local staged=json.decode_saved(e.ModTextFileGetContent(bootstrap));if not staged or staged.enabled~=true then return 'disabled'end
  local raw=e.GlobalsGetValue(key,'');local old=raw~=''and json.decode_saved(raw)or nil
  if raw~=''and not old then return 'corrupt'end
  if old and old.world==staged.world then return old.digest==staged.digest and old.state or 'conflict'end
  local id,state=actor.resolve_player(e,player);if id~=player or not state or not state.ready or state.transformed then return 'pending'end
  local ok,world=pcall(e.GameGetWorldStateEntity);if not ok or type(world)~='number'or world<=0 then return 'pending'end
  local x,y=e.EntityGetTransform(player);if type(x)~='number'or type(y)~='number'or x~=x or y~=y or math.abs(x)==math.huge or math.abs(y)==math.huge then return 'pending'end
  local saved={world=staged.world,digest=staged.digest,state='issuing',step='catalogue'};persist(saved)
  local function step(name)saved.step=name;persist(saved)end
  local function own(entity,kind,slot)
   assert(type(entity)=='number'and entity>0,'Starting pickup spawn failed')
   e.EntityAddTag(entity,'nt2_testing_starting_pickup')
   e.EntityAddComponent2(entity,'VariableStorageComponent',{name='nt2_testing_starting_pickup',value_string=assert(json.encode_saved({world=saved.world,kind=kind,slot=slot}))})
  end
  local success=pcall(function()
   local p=catalogue();step('perk_api');e.dofile_once('data/scripts/perks/perk.lua')
   local spawn=e.perk_spawn;assert(type(spawn)=='function'and type(e.perk_spawn_many)=='function'and type(e.perk_spawn_with_name)=='function','Perk pickup helpers unavailable')
   local count=0;local phase='random_perk'
   e.perk_spawn=function(px,py,perk,keep)
    if phase=='random_perk'then assert(perk~=p.tinkerPerk and perk~=p.noMoreShufflePerk,'Fixed perks must be separate');if count>=p.randomPerks then return nil end end
    assert(keep==true,'Nightmare starting perks must retain the others');local entity=spawn(px,py,perk,true);count=count+1;own(entity,phase,count);return entity
   end
   local good,err=pcall(function()isolated_rng(x,y,function()
    step('random_perks');local loot=e.dofile('mods/noita_together2/files/loot/policy.lua')
    local function perks()e.SetRandomSeed(x-90,y);e.perk_spawn_many(x-90,y-24,true,p.randomPerkExclusions)end
    if loot.enabled()then loot.frame_rng(perks)else perks()end
    assert(count==3,'Expected three random starting perks')
    step('tinker');phase='tinker';count=0;e.perk_spawn_with_name(x+30,y-24,p.tinkerPerk,true);assert(count==1,'Expected one separate Tinker')
    step('no_more_shuffle');phase='no_more_shuffle';count=0;e.perk_spawn_with_name(x+60,y-24,p.noMoreShufflePerk,true);assert(count==1,'Expected one separate No More Shuffle')
    e.SetRandomSeed(x,y-48)
    for i=1,p.randomWands do step('wand_'..i);local opt=e.Random(1,#p.wandIds);assert(type(opt)=='number'and opt%1==0 and opt>=1 and opt<=#p.wandIds,'Invalid random wand index');local entity=e.EntityLoad('data/entities/items/'..p.wandIds[opt]..'.xml',x-44+(i-1)*p.wandSpacingPixels,y-48);own(entity,'wand',i);e.EntityAddTag(entity,'nightmare_starting_wand')end
   end)end)
   e.perk_spawn=spawn;if not good then error(err,0)end
  end)
  saved.state=success and 'done'or 'uncertain';persist(saved)
  if e.print then e.print('NT2_TEST_LOADOUT|state='..saved.state..'|step='..saved.step)end
  return saved.state
 end
 return self
end
return M
