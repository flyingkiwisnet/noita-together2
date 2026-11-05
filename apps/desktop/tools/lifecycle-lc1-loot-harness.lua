-- Fixture assertions over the actual mapped policy/hooks, never authority.
local M={}
local root='mods/noita_together2/files/startup/'
local function module(files,parent,path)
 local env=setmetatable({}, {__index=parent});env._G=env
 env.dofile=function(p)local relative=assert(p:match('^mods/noita_together2/(.+)$'));local chunk=assert(loadfile((parent.NT2_LC1_MOD_ROOT or files..'/mods/noita_together2')..'/'..relative));setfenv(chunk,env);return chunk()end
 return env.dofile(path),env
end
function M.hook(files,parent,enabled)
 local observed,random_calls={},0
 local _,env=module(files,parent,'mods/noita_together2/files/loot/policy.lua')
 env.spawn_wands=function(x,y)observed={x,y};return 'base-wand-result'end
 env.spawn_potions=env.spawn_wands;env.spawn_heart=env.spawn_wands
 env.SetRandomSeed=function(x,y)assert(x==parent.GameGetFrameNum() and y==69427);random_calls=random_calls+1 end
 env.Random=function(min,max)assert(min==0 and max>0);return 1 end
 env.StatsGlobalGetValue=function(key)assert(key=='death_count');return '7'end
 env.dofile('mods/noita_together2/files/loot/biome_scripts.lua')
 assert(env.spawn_wands(100,200)=='base-wand-result')
 assert(observed[1]==(enabled and 99 or 100) and observed[2]==(enabled and 199 or 200),'separate-context loot coordinates differ')
 assert(random_calls==(enabled and 1 or 0),'separate-context loot RNG differs')
end
function M.fresh(files,parent,recorded,enabled)
 assert(parent.ModTextFileGetContent('mods/noita_together2/files/loot/run-policy.txt')=='entry-new:'..recorded,'current-entry broker loot staging differs')
 local list=parent.ModLuaFileGetAppends(root..'scope_registry.lua')
 assert(#list==2 and list[1]==root..'scope_new.lua' and list[2]==root..'scope_spent.lua','fresh loot requires the consumed new/spent registry')
 local policy=module(files,parent,'mods/noita_together2/files/loot/policy.lua')
 assert(policy.initialize()==recorded,'current-entry loot policy differs')
 M.hook(files,parent,enabled)
end
function M.saved(files,parent,recorded)
 assert(parent.ModTextFileGetContent('mods/noita_together2/files/loot/run-policy.txt')=='saved','Continue must reset the inert bootstrap')
 local list=parent.ModLuaFileGetAppends(root..'scope_registry.lua')
 assert(#list==2 and list[1]==root..'scope_restart.lua' and list[2]==root..'scope_spent.lua','Continue requires consumed restored scope')
 local policy=module(files,parent,'mods/noita_together2/files/loot/policy.lua')
 assert(policy.initialize()==recorded,'Continue must read only its restored loot policy')
 M.hook(files,parent,recorded:match('^shared:1:')~=nil)
end
function M.regressions(files)
 local cases=0
 for _,recorded in ipairs({'shared:1:424242','shared:0:424242','individual:0'})do
  local ready,bootstrap,init,writes=false,'saved',true,0
  local saved={};local registry={}
  local parent=setmetatable({}, {__index=_G})
  parent.NT2_LC1_MOD_ROOT=files..'/mod/noita_together2'
  parent.GameGetFrameNum=function()return 17 end
  parent.StatsGetValue=function()return '424242'end
  parent.GameGetWorldStateEntity=function()return ready and 123 or 0 end
  parent.GlobalsGetValue=function(k,fallback)assert(ready,'early context accessed Globals');return saved[k] or fallback end
  parent.GlobalsSetValue=function(k,v)assert(ready,'early context wrote Globals');saved[k]=v;writes=writes+1 end
  parent.ModTextFileSetContent=function(path,value)assert(init and path=='mods/noita_together2/files/loot/run-policy.txt');bootstrap=value end
  parent.ModTextFileGetContent=function()return bootstrap end
  parent.ModLuaFileGetAppends=function(path)return registry[path] or {}end
  parent.ModLuaFileAppend=function(path,value)registry[path]=registry[path] or {};table.insert(registry[path],value)end
  local scope=module(files,parent,root..'settings_scope.lua')
  local policy=module(files,parent,'mods/noita_together2/files/loot/policy.lua')
  assert(scope.produce('new')=='recorded');assert(scope.consume()=='fresh');policy.stage(recorded,true)
  M.fresh(files,parent,recorded,recorded:match('^shared:1:')~=nil);assert(writes==0)
  ready=true;M.fresh(files,parent,recorded,recorded:match('^shared:1:')~=nil);assert(saved.nt2_seed_loot_policy_v1==recorded and writes==1);cases=cases+2
  local invalid={{},{root..'scope_new.lua'},{root..'scope_restart.lua',root..'scope_spent.lua'},{root..'scope_new.lua',root..'scope_new.lua'},{root..'scope_new.lua',root..'scope_spent.lua',root..'scope_invalid.lua'}}
  for _,list in ipairs(invalid)do
   registry[root..'scope_registry.lua']=list;ready=false;assert(policy.initialize()=='' and writes==1)
   ready=true;saved.nt2_seed_loot_policy_v1='shared:0:424242';assert(policy.initialize()=='shared:0:424242' and writes==1,'stale fresh registry overwrote Continue');M.hook(files,parent,false);cases=cases+2
  end
  parent.ModLuaFileGetAppends=function()error('injected unavailable registry')end
  assert(policy.initialize()=='shared:0:424242' and writes==1);cases=cases+1
  parent.ModLuaFileGetAppends=function(path)return registry[path] or {}end
  registry={};assert(scope.produce('restart')=='recorded');assert(scope.consume()=='restored');policy.saved();init=false
  M.saved(files,parent,'shared:0:424242');assert(writes==1);cases=cases+1
 end
 print('LC1_LOOT_HARNESS_CHECKS|'..cases)
end
return M
