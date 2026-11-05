-- Execute the four reused production auxiliary scripts under Lua51. Engine
-- components/time are injected; this does not claim real rendered particles.
local repo=assert(os.getenv('NT2_A3_REPO'));local checks=0
local function check(v,s)assert(v,s);checks=checks+1 end
local function run(name,times,age)
 local values={[1]={mTimesExecuted=times},[2]={is_emitting=false},[3]={is_emitting=false},[4]={alpha=0},[5]={name='nt2_a3_age_frames',value_int=age or 0}}
 for n=6,10 do values[n]={radius=0}end;local x,y=10,20
 local env=setmetatable({GetUpdatedEntityID=function()return 99 end,EntityGetTransform=function()return x,y end,EntitySetTransform=function(_,a,b)x=a;y=b end,
  EntityGetFirstComponentIncludingDisabled=function(_,kind)return ({LuaComponent=1,ParticleEmitterComponent=2,SpriteParticleEmitterComponent=3,SpriteComponent=4})[kind]end,
  EntityGetComponentIncludingDisabled=function(_,kind)return kind=='LightComponent'and {6,7,8,9,10}or kind=='VariableStorageComponent'and {5}or {}end,
  ComponentGetValue2=function(c,k)assert(values[c],'fixed script component');return values[c][k]end,
  ComponentSetValue2=function(c,k,v)assert(values[c],'fixed script component');values[c][k]=v;checks=checks+1 end},{__index=_G})
 local f=assert(loadfile(repo..'/mod/noita_together2/files/g3/emotes/effects/'..name..'_entity.lua'));setfenv(f,env);f();return values,x,y
end
local v=run('heart',17);check(not v[2].is_emitting,'heart before emission boundary');v=run('heart',18);check(v[2].is_emitting and v[3].is_emitting,'heart exact legacy emission boundary');v=run('heart',0,54);check(v[2].is_emitting,'late heart seeks phase')
for _,t in ipairs({0,15,16,22,49,50,56,57})do local a=run('thumbs_up',t);local alpha=math.min(0.7,math.max(0,(t-15)*0.1));if t>=50 then alpha=math.max(0,0.7-(t-49)*0.1)end;check(math.abs(a[4].alpha-alpha)<0.00001,'thumb alpha phase '..t)end
v=run('thumbs_up',0,60);check(math.abs(v[4].alpha-0.5)<0.00001,'late thumbs seeks without fresh fade-in')
for _,t in ipairs({0,68,69,82,83,120})do v=run('aggressive_point',t);check(v[2].is_emitting==(t>68 and t<83),'point phase '..t)end
v=run('aggressive_point',0,90);check(not v[2].is_emitting,'cold point does not replay completed burst')
for _,t in ipairs({0,53,54,153,154,163,193,325,326,335,375})do v=run('praise_the_sun',t);if t>=153 then check(v[2].is_emitting==(t<=325),'sun emitter phase '..t);check(v[2].emission_interval_min_frames>=5 and v[2].emission_interval_max_frames<=13,'sun bounded emitter interval')end;if t>325 then check(v[4].alpha>=0 and v[4].alpha<=0.1,'sun fade bound')end end
v=run('praise_the_sun',0,335);check(not v[2].is_emitting and v[4].alpha==0,'late sun does not replay completed effect')
print('PASS A3 auxiliary effects '..checks..' production-script assertions; injected components/frame age')
