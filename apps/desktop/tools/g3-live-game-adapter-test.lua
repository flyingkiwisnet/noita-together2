-- Actual production adapter/owner modules, documented engine API shapes injected.
-- No Noita, private pipe or service claimed by this focused game-thread test.
local root=assert(os.getenv('NT2_G3_MODULES'));local ffi=require('ffi')
local raw=dofile;dofile=function(path)local prefix='mods/noita_together2/files/g3/';return raw(path:sub(1,#prefix)==prefix and root..'/'..path:sub(#prefix+1)or path)end
local function load(n) return dofile(root..'/'..n..'.lua') end
local clock,x,velocity,alive=0,1.25,0.5,true;local entities={};local next_entity=20
EntityGetIsAlive=function(id) return id==7 and alive or entities[id]~=nil end
GameGetRealWorldTimeSinceStarted=function() return clock end
EntityGetTransform=function(id) assert(id==7);return x,2.5,0,1,1 end
EntityGetFirstComponentIncludingDisabled=function(id,kind) assert(id==7 and kind=='VelocityComponent');return 9 end
ComponentGetValue2=function(id,key) assert(id==9 and key=='mVelocity');return velocity,-0.25 end
EntityLoad=function(path,px,py) assert(path=='mods/noita_together2/files/g3/ghost.xml');next_entity=next_entity+1;entities[next_entity]={x=px,y=py};return next_entity end
EntityKill=function(id) entities[id]=nil end
EntityGetComponentIncludingDisabled=function()return {}end;EntityGetAllChildren=function()return {}end
EntitySetTransform=function(id,px,py,rotation,facing,scale) assert(entities[id] and rotation==0 and facing==1 and scale==1);entities[id]={x=px,y=py} end
local logs={};local saved_print=print;print=function(s) logs[#logs+1]=s;saved_print(s) end
local context=ffi.new('uint8_t[16]');ffi.fill(context,16,1)
local response=ffi.new('uint8_t[864]');ffi.copy(response,'NG3R',4);response[4]=2;response[5]=1;response[6]=1;ffi.copy(response+8,context,16);response[24]=200;response[28]=1
ffi.fill(response+32,16,2);ffi.fill(response+544,16,2);ffi.copy(response+560,'NTG3',4);response[564]=1;response[566]=1;ffi.copy(response+568,context,16)
local function integer(at,n) local bytes=ffi.new('int32_t[1]',n);ffi.copy(response+at,bytes,4) end
integer(584,1);integer(588,3500);integer(592,-1250)
local reject_move,missing_reply=false,false;local calls=0
local bridge={nt2_g3_move=function(c,seq,px,py,vx,vy,facing) calls=calls+1;assert(px==1250 and py==2500 and vx==500 and vy==-250 and facing==0);return reject_move and 0 or 1 end,
 nt2_g3_peers=function(c,out,size) assert(size==864);if missing_reply then return 0 end;ffi.copy(out,response,864);return 864 end}
local owner=load('native_owner').new(bridge,context,ffi,load('game_api'),load('validation'),load('movement'),load('ghosts'))
owner:tick(7,0,true);assert(owner.sampled==1 and owner.movement_accepted==1 and owner.nonempty_batches==1 and next(entities),'Actual API adapter creates first peer from a validated injected batch')
local _,peer=next(entities);assert(peer.x==3.5 and peer.y==-1.25)
clock=0.1;reject_move=true;owner:tick(7,6,true);assert(owner.sampled==2 and owner.movement_accepted==1)
clock='invalid';owner:tick(7,12,true);assert(owner.clock_invalid==1 and not next(entities))
clock=0.2;x=0/0;owner:tick(7,18,true);assert(owner.pose_invalid==1)
x=1.25;alive=false;owner:tick(7,24,true);assert(owner.player_or_sequence==1)
alive=true;missing_reply=true;owner:tick(7,300,true);assert(not next(entities))
owner:tick(7,600,false);assert(logs[#logs]:find('playing=false',1,true) and logs[#logs]:find('clock_invalid=1',1,true) and logs[#logs]:find('pose_invalid=1',1,true))
for _,s in ipairs(logs) do assert(not s:find(string.rep('01',16),1,true) and not s:find('token',1,true)) end
owner:close();assert(not next(entities))
saved_print('PASS production game adapter: first peer, sampling milli-units, native refusal, invalid engine values, unavailable reply, held cleanup and bounded nonsecret diagnostics')

local before=#logs
clock=1;x=1.25;alive=true;missing_reply=false;reject_move=false
local heartbeat=0
bridge.nt2_g3_heartbeat_diagnostic=function() return heartbeat end
local short=load('native_owner').new(bridge,context,ffi,load('game_api'),load('validation'),load('movement'),load('ghosts'))
short:tick(7,1000,true)
heartbeat=6;short:tick(7,1020,false)
heartbeat=0;short:tick(7,1040,true)
assert(#logs==before+1,'No diagnostic may exceed the 60-frame budget')
short:tick(7,1060,true)
assert(#logs==before+2,'Pending transition summary emits at first eligible observation')
local summary=logs[#logs]
assert(summary:find('playing=true',1,true) and summary:find('holds=1',1,true) and summary:find('recoveries=1',1,true)
 and summary:find('hold_reason=6',1,true),'Brief hold and its reason survive recovery')
short:tick(7,1200,true)
assert(logs[#logs]:find('hold_reason=6',1,true),'Success does not erase observed hold reason')
heartbeat=9;short:tick(7,1220,false);heartbeat=0;short:tick(7,1240,true);short:tick(7,1260,true)
assert(logs[#logs]:find('holds=2',1,true) and logs[#logs]:find('recoveries=2',1,true) and logs[#logs]:find('hold_reason=9',1,true))
short:tick(7,1265,false);short:tick(7,1270,true);short:tick(7,1275,false);short:tick(7,1276,true)
assert(short.holds==4 and short.recoveries==4 and short.hold_reason==9,'Every sub-budget transition is counted and ACK does not erase failure')
assert(#logs==before+4,'Multiple brief transitions stay within the log budget')
short.holds=2147483647;short:tick(7,1280,false);assert(short.holds==2147483647,'Counters saturate')
short:close()
local fresh=load('native_owner').new(bridge,context,ffi,load('game_api'),load('validation'),load('movement'),load('ghosts'))
fresh:tick(7,1300,true);assert(fresh.holds==0 and fresh.recoveries==0 and fresh.hold_reason==0,'Fresh context clears diagnostic history')
fresh:close()
for i=before+1,#logs do assert(not logs[i]:find(string.rep('01',16),1,true) and not logs[i]:find('token',1,true) and #logs[i]<600) end
saved_print('PASS brief hold/recovery: sticky reason, bounded cadence, repeated transitions, saturation, fresh-context reset and no private values')
