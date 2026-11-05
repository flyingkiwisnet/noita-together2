-- Production guard/bridge with injected deferred engine removal. No saves,
-- app/service/journal state or actual Noita process is used by this test.
local repo=assert(os.getenv('NT2_ITEM_REPO'));local raw=dofile
local baseline=os.getenv('NT2_ITEM_DEFERRED_BASELINE')=='1'
dofile=function(path)
 if path:find('item_catalogue_data.lua',1,true)then return raw(repo..'/packages/contracts/g3-items/catalogue.lua')end
 if baseline and (path:find('/item_local.lua',1,true)or path:find('/item_bridge.lua',1,true))then return raw(repo..'/testing/current/'..path:gsub('^mods/','mod/'))end
 return raw(repo..'/'..path:gsub('^mods/','mod/'))
end
local root='mods/noita_together2/files/g3/'
local json=dofile(root..'bank_json.lua');local checks=0
local function check(v,message)assert(v,message);checks=checks+1 end
local function equal(a,b,message)check(a==b,message or (tostring(a)..' ~= '..tostring(b)))end
local serial=0
local function case(mode,blocked)
 local t={frame=1,entity=10,alive=true,killCalls=0,ready=true,parts={ItemComponent=11,ItemActionComponent=12,SimplePhysicsComponent=13},marker=14,markerValue=nil,parent=0,globals={},sent={},logs={}}
 local e={EntityGetIsAlive=function(id)return id==1 or id==99 or id==t.entity and t.alive end,
 EntityGetFirstComponentIncludingDisabled=function(...)
  local id,kind,tag=...;assert(select('#',...)==2 or type(tag)=='string','Optional string cannot be explicit nil')
  if id==t.entity then return t.parts[kind]end
 end,
 EntityGetComponentIncludingDisabled=function(id,kind)return id==t.entity and kind=='VariableStorageComponent'and t.markerValue and {t.marker}or {}end,
 EntityGetParent=function()return t.parent end,EntityGetRootEntity=function(id)return t.parent~=0 and t.parent or id end,
 EntityGetTransform=function()return 0,0 end,EntityHasTag=function(id,tag)return id==t.entity and tag=='card_action'end,
 EntityGetWithTag=function(tag)return tag=='nt2_item_inlet'and {1}or {}end,
 EntityGetInRadiusWithTag=function(_,_,_,tag)return t.alive and tag=='card_action'and {t.entity}or {}end,
 EntityGetAllChildren=function()return {}end,EntityGetName=function()return ''end,
 ComponentGetValue2=function(id,key)
  if id==11 and key=='uses_remaining'then return -1 end
  if id==12 and key=='action_id'then return 'LIGHT_BULLET'end
  if id==t.marker then return key=='name'and 'nt2_item_original'or key=='value_string'and t.markerValue end
 end,
 EntityAddComponent2=function(_,kind,v)assert(kind=='VariableStorageComponent');t.markerValue=v.value_string;return t.marker end,
 EntityRemoveComponent=function()t.markerValue=nil end,
 GlobalsGetValue=function(key,fallback)return t.globals[key]or fallback end,GlobalsSetValue=function(key,value)t.globals[key]=value end,
 GameGetFrameNum=function()return t.frame end,
 EntityKill=function(id)assert(id==t.entity);t.killCalls=t.killCalls+1;if mode=='throw'then t.alive=false;error('deferred engine throw')end;if mode=='sync'then t.alive=false end end,
 print=function(line)t.logs[#t.logs+1]=line end}
 local function identity()serial=serial+1;return string.format('00000000-0000-4000-8000-%012d',serial)end
 local bridge=dofile(root..'item_bridge.lua').new(e,identity,function(body)t.sent[#t.sent+1]=assert(json.decode(body));return true end,function()return t.ready and {}or nil end,function()return false end,function()end)
 bridge:bind('live-scope');local captured=assert(bridge.guard:capture(1))
 local op={id=captured.id,digest=string.rep('a',64),direction='deposit',stage='offer'}
 bridge.effects[op.id]=captured.item;bridge:receive({enabled=true,fresh=true,effect=op})
 t.bridge=bridge
 function t:tick(frame)self.frame=frame;self.bridge:tick(frame,99)end
 function t:results()local result={};for _,p in ipairs(self.sent)do if p.kind=='item_result'then result[#result+1]=p end end;return result end
 t:tick(1);equal(t.sent[1].kind,'item_prepare','Actual bridge emits prepare')
 op.attempt=t.sent[1].attempt;op.stage='permit';if blocked then t.parent=99 end;t:tick(2)
 t.bridge=bridge;t.op=op;t.record=captured;t.e=e
 return t
end
local delayed=case('deferred')
equal(#delayed:results(),0,'No uncertain receipt while a successful removal request is still pending')
equal(delayed.record.state,'removing','Live volatile removal observation')
check(delayed.globals['nt2_item_'..delayed.op.id]:find(':uncertain$'),'Durable uncertainty precedes engine call')
for f=3,6 do delayed:tick(f);equal(#delayed:results(),0,'No early result under duplicate permits')end
delayed.alive=false;delayed:tick(7);delayed:tick(8)
equal(#delayed:results(),1);check(delayed:results()[1].applied,'Same-live absent original reports applied')
equal(delayed.killCalls,1,'Deferred removal is never called twice')
for f=9,20 do delayed:tick(f)end;equal(#delayed:results(),1,'Duplicate permits/results are copied once')
check(delayed.logs[1]:find('state=removing',1,true)and delayed.logs[2]:find('reason=removal_observed',1,true),'Bounded exact-attempt transition diagnostics')
local gap=case('deferred');gap.alive=false;gap.bridge:observe(3)
equal(#gap:results(),0,'Copy-gap observation does not publish a result')
equal(gap.record.state,'applied_session','Validated Playing owner can observe existing removal through short copy gap')
gap:tick(4);gap:tick(5);equal(#gap:results(),1);check(gap:results()[1].applied);equal(gap.killCalls,1)
for _,mode in ipairs({'sync','throw','stuck'})do
 local t=case(mode);for f=3,130 do t:tick(f)end
 equal(#t:results(),1,mode..' has one terminal result');equal(t:results()[1].applied,mode=='sync',mode..' preserves ambiguity')
 t.alive=false;for f=131,135 do t:tick(f)end;equal(#t:results(),1,'Late disappearance never upgrades '..mode);equal(t.killCalls,1)
end
for _,fault in ipairs({'parent','marker','component','authority','scope','cold','attempt','digest','direction','footer','evidence','pause','frame-reset'})do
 local t=case('deferred');local op=t.op
 if fault=='parent'then t.parent=99
 elseif fault=='marker'then t.markerValue='other-owner'
 elseif fault=='component'then t.parts.SimplePhysicsComponent=77
 elseif fault=='authority'then t.ready=false
 elseif fault=='scope'then t.bridge:bind('other-scope')
 elseif fault=='cold'then
  local cold=dofile(root..'item_bridge.lua').new(t.e,function()error('Must retain original identity')end,function()error('Cold guard cannot report completion')end,function()return {}end,function()return false end,function()end)
  cold:bind('live-scope');local r=assert(cold.guard:capture(1));check(not cold.guard:offer(op,r.item,'cold',false),'Persisted uncertain original cannot be prepared cold');t.bridge=cold
 elseif fault=='attempt'then op.attempt='another-attempt'
 elseif fault=='digest'then op.digest=string.rep('b',64)
 elseif fault=='direction'then op.direction='claim'
 elseif fault=='footer'then t.bridge:receive({enabled=true,fresh=true})
 elseif fault=='evidence'then t.globals['nt2_item_'..op.id]='changed'
 elseif fault=='pause'then t.bridge:pause()
 elseif fault=='frame-reset'then t:tick(0)end
 t:tick(3);t.ready=true;t.parent=0;t.parts.SimplePhysicsComponent=13;t.markerValue='live-scope:'..op.id;t.alive=false
 for f=4,10 do t:tick(f)end
 for _,p in ipairs(t:results())do check(not p.applied,'Cannot promote after '..fault)end
 equal(t.killCalls,1,'Never re-kill after '..fault)
end
-- An unresolved original does not consume a guard-wide effect slot.
local t=case('stuck');for f=3,125 do t:tick(f)end
local first=t.record;local count=#t:results();t.alive=true;t.entity=20;t.markerValue=nil
local nextOriginal=assert(t.bridge.guard:capture(1));check(nextOriginal.id~=first.id,'Next original gets independent identity')
local nextOp={id=nextOriginal.id,digest=string.rep('c',64),direction='deposit',stage='offer'}
t.bridge.effects[nextOp.id]=nextOriginal.item;t.bridge:receive({enabled=true,fresh=true,effect=nextOp});t:tick(126)
nextOp.attempt=t.sent[#t.sent].attempt;nextOp.stage='permit';t:tick(127);t.alive=false;t:tick(128);t:tick(129)
equal(#t:results(),count+1,'Another original progresses after timeout');check(t:results()[count+1].applied);equal(t.killCalls,2,'One call for each independent original')
local display_stale=case('deferred');display_stale.bridge.view.fresh=false;display_stale.alive=false;display_stale:tick(3);display_stale:tick(4)
equal(#display_stale:results(),1,'Display invalidation does not revoke a validated live removal attempt');check(display_stale:results()[1].applied);equal(display_stale.killCalls,1)
-- A live issued permit waiting for its unchanged physical source remains reusable.
local blocked=case('deferred',true);equal(blocked.killCalls,0);equal(blocked.record.state,'prepared')
for f=3,20 do blocked:tick(f)end;equal(blocked.killCalls,0);equal(#blocked:results(),0)
equal(#blocked.logs,1,'One bounded waiting transition, no frame spam');check(blocked.logs[1]:find('source_unavailable',1,true))
blocked.parent=0;blocked:tick(21);equal(blocked.killCalls,1);equal(blocked.record.state,'removing')
-- A different offered sibling cannot revoke an already issued removal permit.
blocked.bridge:receive({enabled=true,effects={blocked.op,{id='sibling',stage='offer',direction='deposit'}}})
blocked.alive=false;blocked:tick(22);blocked:tick(23);equal(#blocked:results(),1);check(blocked:results()[1].applied);equal(blocked.killCalls,1)
check(blocked.logs[2]:find('source_revalidated',1,true),'Exact original source can resume its issued attempt')
print('ITEM_DEFERRED_REMOVAL_CHECKS|'..checks..'|engineFactsInjected=true|actualNoitaTrace=false')
