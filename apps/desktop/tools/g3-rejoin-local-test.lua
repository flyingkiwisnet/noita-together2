local repo=assert(os.getenv('NT2_REJOIN_REPO'))
local M=dofile(repo..'/mod/noita_together2/files/rejoin/local_world.lua')
local ready=false;local globals={};local writes=0;local actual='424242';local inventory={'original wand','original potion'};local position={100,200};local loot='shared:1:424242'
GameGetWorldStateEntity=function() return ready and 42 or 0 end
GlobalsGetValue=function(k,f) assert(ready,'WorldState missing');return globals[k] or f end
GlobalsSetValue=function(k,v) assert(ready,'WorldState missing');globals[k]=v;writes=writes+1 end
StatsGetValue=function(k) assert(k=='world_seed');return actual end
SetWorldSeed=function() error('Continue must never reseed') end
local function adapter(known,intent)
 local a={calls=0,holds=0,retired=0,result=nil,reply_generation=0,other_input_block=true,owned_effect=false,inspected_current=false}
 a.known=function(saved) return known and saved.record=='non-secret association' and saved.loot==loot,intent end
 a.hold_owned=function() a.holds=a.holds+1;a.owned_effect=true;return {owned=true} end
 a.release_owned=function(effect) assert(effect.owned);a.owned_effect=false;assert(a.other_input_block,'do not release another mod restriction') end
 a.request=function(record,generation) assert(record.seed==actual);a.calls=a.calls+1;a.reply_generation=generation;return 'opaque local request handle' end
 a.poll=function() return a.result,a.reply_generation,a.reason end
 a.inspected=function() return a.inspected_current end
 a.retire=function() a.retired=a.retired+1 end
 return a
end
local a=adapter(true,false);local m=M.new(a)
assert(not m:associate('non-secret association',actual,loot,true));assert(not m:loaded());assert(writes==0)
ready=true;assert(not m:associate('non-secret association',actual,loot,false));assert(m:associate('non-secret association',actual,loot,true))
local saved={};for k,v in pairs(globals) do saved[k]=v end
local continued=M.new(a);assert(continued:loaded());assert(continued.state=='choice' and a.calls==0 and a.owned_effect)
assert(continued:loaded() and a.holds==1);assert(continued:rejoin());assert(continued:rejoin() and a.calls==1)
a.result='released';continued:tick();assert(continued.state=='reconnecting' and a.owned_effect)
a.result='admitted';continued:tick();assert(continued.state=='inspecting')
a.result='released';continued:tick();assert(a.owned_effect);a.inspected_current=true;continued:tick();assert(continued.state=='playing' and not a.owned_effect)
local old_generation,old_pending=continued.generation,continued.pending;continued:lost();assert(a.calls==1 and a.owned_effect and continued.generation==old_generation and continued.pending==old_pending)
a.result='lost';continued:tick();assert(a.calls==1 and continued.pending==old_pending and a.owned_effect)
a.reply_generation=old_generation-1;a.result='released';continued:tick();assert(a.owned_effect)
a.reply_generation=old_generation;a.result='admitted';continued:tick();a.result='released';continued:tick();assert(continued.state=='playing' and not a.owned_effect and a.calls==1)
continued:offline();assert(a.retired==1 and not a.owned_effect);continued:tick();assert(a.calls==1 and globals.nt2_rejoin_local_world_v1_offline=='1');assert(not M.new(a):loaded())
globals=saved;local first=adapter(true,true);local app_first=M.new(first);assert(app_first:loaded() and app_first.state=='reconnecting' and first.calls==1);assert(app_first:loaded() and first.calls==1)
assert(not M.new(adapter(false,false)):loaded());actual='999';assert(not M.new(adapter(true,false)):loaded());actual='424242'
-- Terminal replies stop shared retries but must not choose offline for the user.
for _,phase in ipairs({'reconnecting','inspecting'}) do
 globals={};for k,v in pairs(saved) do globals[k]=v end
 local terminal=adapter(true,false);local world=M.new(terminal);assert(world:loaded());assert(world:rejoin())
 if phase=='inspecting' then terminal.result='admitted';world:tick();assert(world.state=='inspecting') end
 local before=writes;local old=world.generation;terminal.reason='Host ended the run';terminal.result='terminal';world:tick()
 assert(world.state=='terminal' and terminal.owned_effect and terminal.retired==0 and writes==before)
 assert(globals.nt2_rejoin_local_world_v1_offline=='0' and world.generation>old and world.pending==nil)
 assert(not world:rejoin());assert(world:loaded() and world.state=='terminal');assert(not world:associate('non-secret association',actual,loot,true));assert(writes==before)
 terminal.reply_generation=old;terminal.result='released';world:tick();assert(terminal.owned_effect and terminal.calls==1)
 local buttons,labels={},{};local choose=false
 GuiText=function(_,_,_,label) labels[#labels+1]=label end
 GuiButton=function(_,id,_,_,label) buttons[#buttons+1]=label;return choose and id==9142 end
 world:draw({});assert(#buttons==1 and buttons[1]=='Continue offline' and labels[1]:find('Host ended the run',1,true))
 assert(terminal.retired==0 and terminal.owned_effect and writes==before)
 choose=true;world:draw({});assert(world.state=='offline' and not terminal.owned_effect and terminal.other_input_block)
 assert(terminal.retired==1 and writes==before+1 and globals.nt2_rejoin_local_world_v1_offline=='1')
 world:offline();assert(terminal.retired==1 and writes==before+1);assert(not M.new(terminal):loaded())
 assert(not M.new(terminal):associate('late association with new local uuid',actual,loot,true));assert(writes==before+1)
end
globals={};assert(not M.new(adapter(true,false)):loaded())
assert(inventory[1]=='original wand' and inventory[2]=='original potion' and position[1]==100 and position[2]==200 and loot=='shared:1:424242')
print('PASS rejoin lifecycle plus terminal protected/no automatic retirement or writes; visible reason/offline-only action; explicit offline single retirement/release; late metadata refusal; original inventory/position/loot/other-mod restriction preserved')
-- Async lookup must hold only until Rust answers; foreign saves stay ordinary offline.
globals={};for k,v in pairs(saved) do globals[k]=v end
local lookup=adapter(true,false);local answered=false;lookup.known=function() if not answered then return nil end;return false end
local checking=M.new(lookup);local before=writes;assert(checking:loaded() and checking.state=='lookup' and lookup.owned_effect);checking:tick();assert(lookup.owned_effect)
answered=true;checking:tick();assert(checking.state=='offline' and not lookup.owned_effect and lookup.retired==0 and writes==before)
-- A trusted explicit app opt-out is distinct from canonical terminal End.
globals={};for k,v in pairs(saved) do globals[k]=v end
local app=adapter(true,false);local selected=false;app.known=function() return true,false,false,selected end
local world=M.new(app);assert(world:loaded());world:rejoin();app.result='terminal';world:tick();assert(world.state=='terminal' and app.owned_effect)
world:tick();assert(app.owned_effect and app.retired==0);selected=true;world:tick();assert(world.state=='offline' and not app.owned_effect and app.retired==1 and globals.nt2_rejoin_local_world_v1_offline=='1');world:tick();assert(app.retired==1)
print('PASS asynchronous unknown lookup/no retirement; terminal remains held until trusted explicit app offline signal; single owned restoration')
-- Exact shared death reason uses the existing protected selector without a grant.
for _,entry in ipairs({'loaded','lookup','watching','poll'})do
 globals={};for k,v in pairs(saved)do globals[k]=v end
 local a=adapter(true,false);local complete=false;local finishes=0
 a.known=function()if entry=='lookup'and not complete then return nil end;return true,false,complete,false,complete and 'player_death_end'or nil end
 a.finish=function(reason)assert(reason=='player_death_end');finishes=finishes+1 end
 local m=M.new(a)
 if entry=='loaded'then complete=true;assert(m:loaded())
 elseif entry=='watching'then assert(m:waiting('{"kind":"nt2-world"}',actual,loot));complete=true;m:tick()
 else assert(m:loaded());if entry=='poll'then assert(m:rejoin());a.result='terminal';a.reason='player_death_end'else complete=true end;m:tick()end
 assert(m.state=='terminal'and finishes==1 and a.owned_effect and a.retired==0)
end
print('PASS shared death finish from loaded/lookup/watching/poll; no grant or automatic offline')
