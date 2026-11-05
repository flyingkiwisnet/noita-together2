-- Deterministic receiver-clock playback, injected engine only.
local root=assert(os.getenv('NT2_G3_MODULES'))
local validate=dofile(root..'/validation.lua');local module=dofile(root..'/ghosts.lua')
local checks={}
local function check(name,fn) fn();checks[#checks+1]=name;print('PASS '..name) end
local function fixture()
 local now=0;local entities={};local next_id=0;local created=0;local removed=0
 local api={time=function() return now end,alive=function(id) return entities[id]~=nil end,
 spawn=function(x,y) next_id=next_id+1;created=created+1;entities[next_id]={x=x,y=y};return next_id end,
 remove=function(id) removed=removed+1;entities[id]=nil end,
 position=function(id,x,y) assert(entities[id]);entities[id].x=x;entities[id].y=y end}
 local g=module.new(api,validate);assert(g:bind('world',{'peer'}))
 local f={g=g,entities=entities,set=function(t) now=t end,created=function() return created end,removed=function() return removed end}
 function f:receive(seq,x,t) now=t;return g:receive('world',{actor='peer',sequence=seq,x=x,y=0,vx=100,vy=0,rotation=0,facing=1},math.floor(t*60)) end
 function f:tick(t) now=t;g:tick(math.floor(t*60),true);local ghost=g.ghosts.peer;return ghost and entities[ghost.entity].x end
 return f
end
check('time playback matches across30Hz60Hz and slow frames',function()
 for _,hz in ipairs({10,30,60}) do
  local f=fixture();local next_sample=0
  for frame=0,math.floor(hz*0.8) do local t=frame/hz
   while next_sample<=8 and next_sample/10<=t+0.00000001 do assert(f:receive(next_sample+1,next_sample*10,next_sample/10));next_sample=next_sample+1 end
   f:tick(t)
  end
  local x=f:tick(0.8);assert(math.abs(x-62)<0.00001, tostring(hz)..' '..x)
  assert(#f.g.ghosts.peer.samples<=module.MAX_SAMPLES)
 end
end)
check('bursty arrivals coalesce and duplicate/out-of-order refuse',function()
 local f=fixture();assert(f:receive(1,0,0));assert(f:receive(2,10,0.1));assert(f:receive(3,20,0.2));assert(f:receive(4,25,0.2));assert(f:receive(5,30,0.2))
 assert(#f.g.ghosts.peer.samples==3);assert(not f:receive(5,99,0.2));assert(not f:receive(3,99,0.2));assert(math.abs(f:tick(0.33)-20)<0.00001)
end)
check('ordinary gap holds one entity then resumes with bounded playback',function()
 local f=fixture();assert(f:receive(1,0,0));assert(f:receive(2,10,0.1));assert(f:tick(0.4)==10);assert(f:tick(0.8)==10)
 assert(f:receive(3,30,0.9));assert(f:tick(0.9)==10);assert(math.abs(f:tick(0.99)-20)<0.00001);assert(f:tick(1.09)==30)
 assert(f.created()==1 and f.removed()==0)
end)
check('teleport resets without terrain sweep and buffer remains bounded',function()
 local f=fixture();assert(f:receive(1,0,0));assert(f:receive(2,1000,0.1));assert(f:tick(0.1)==1000)
 for i=3,100 do assert(f:receive(i,1000+i,0.1+i*0.001)) end
 assert(#f.g.ghosts.peer.samples<=module.MAX_SAMPLES)
end)
check('buffer underrun resumes continuously without a catch-up jump',function()
 local f=fixture();assert(f:receive(1,0,0));assert(f:receive(2,10,0.1));assert(f:tick(0.34)==10)
 assert(f:receive(3,35,0.35));assert(math.abs(f:tick(0.35)-10)<0.00001)
 assert(math.abs(f:tick(0.44)-22.5)<0.00001);assert(math.abs(f:tick(0.54)-35)<0.00001);assert(f.created()==1)
end)
check('expired data scope replacement terminal and replay fences retire',function()
 local f=fixture();assert(f:receive(1,0,0));assert(f:tick(2.01)==nil);assert(not f:receive(1,1,2.02));assert(f:receive(2,2,2.03))
 f.g:tick(123,false);assert(f.g.count==0);assert(f.g:bind('other',{'peer'}));assert(not f.g:receive('world',{actor='peer',sequence=3,x=3,y=0,vx=0,vy=0,rotation=0,facing=1},123))
end)
check('complete actor roster removal differs from harmless sample gaps',function()
 local f=fixture();assert(f:receive(1,0,0));assert(f:tick(0.4)==0);assert(f.created()==1)
 assert(f.g:reconcile('world',{}));assert(f.g.count==0 and f.removed()==1)
 assert(not f:receive(2,10,0.5));assert(f.g:reconcile('world',{'peer'}));assert(not f:receive(1,10,0.5));assert(f:receive(2,10,0.5))
 assert(not f.g:reconcile('other',{'peer'}));assert(not f.g:reconcile('world',{'peer','peer'}))
 f:tick(0.6);assert(f:tick(0.59)==nil)
end)
check('jittered delivery produces bounded continuous render steps',function()
 local f=fixture();local arrivals={0,0.08,0.21,0.29,0.43,0.5,0.62,0.71,0.83};local next_sample=1;local previous;local max_step=0
 for frame=0,60 do local t=frame/60
  while arrivals[next_sample] and arrivals[next_sample]<=t do local at=arrivals[next_sample];assert(f:receive(next_sample,at*100,at));next_sample=next_sample+1 end
  local x=f:tick(t);if previous then max_step=math.max(max_step,math.abs(x-previous)) end;previous=x
 end
 assert(max_step<2, 'jitter render step '..max_step);assert(f.created()==1)
 print('MEASURE delayMs=180 samplesMax=8 render60Hz maxStep='..max_step)
end)
print('A1_PLAYBACK_RESULT passed='..#checks..' actualNoita=false')
