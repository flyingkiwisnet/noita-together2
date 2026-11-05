local repo=assert(os.getenv('NT2_ITEM_REPO'))
NT2_C1_TEST_ENGINE_ONLY=true;local fixture=dofile(repo..'/apps/desktop/tools/c1-local-test.lua');NT2_C1_TEST_ENGINE_ONLY=nil
local restricted=dofile(repo..'/apps/desktop/tools/c1-restricted-context.lua')
local checks=0
local function eq(a,b)assert(a==b,tostring(a)..' ~= '..tostring(b));checks=checks+1 end
local function context(parent)
 return restricted(parent,function(path)
  if path:find('c1_catalogue_data.lua',1,true)then path='packages/contracts/g3-c1/catalogue.lua'else path=path:gsub('^mods/','mod/')end
  local f=assert(io.open(repo..'/'..path,'rb'));local body=f:read('*a');f:close();return body
 end)
end
local function setup()
 local e,c,entities,g,flags=fixture.engine();local sent={};local uid=0
 e.ComponentGetIsEnabled=function(id)return c[id].enabled~=false end
 local json=e.dofile('mods/noita_together2/files/g3/bank_json.lua')
 local content=e.dofile('mods/noita_together2/files/g3/c1_content.lua').load(e)
 local owner=e.dofile('mods/noita_together2/files/g3/c1_local.lua').new(e,function()uid=uid+1;return string.format('%08d-1111-4111-8111-111111111111',uid)end,function(body)sent[#sent+1]=assert(json.decode(body));return true end,content)
 owner.context=string.rep('01',16)
 local rules=fixture.copy(fixture.rules);rules.NT_death_penalty='weak_respawn'
 local view={world=string.rep('a',64),initialized=true,memberCount=2,snapshot={revision=1,rules=rules}}
 owner:receive(view);owner:tick(true,1,100)
 return e,c,entities,g,flags,owner,sent,json,view
end
local root='mods/noita_together2/files/c1/'
-- Real append + real API in a state with no require/FFI/NT2_C1 main global.
for _,kind in ipairs({'heart','heart_better','perk','orb'})do
 local e,c,entities,g,flags,owner,sent,json,view=setup();local h=context(e);eq(h.require,nil);eq(h.NT2_C1,nil)
 local vanilla=0;h.item_pickup=function(item,player)vanilla=vanilla+1;e.EntityKill(item)end
 h.GetKeyPerkComponents=function()return{perk_id='EXTRA_MONEY'}end;h.IsBannedPerkId=function()return false end
 if kind=='orb'then e.EntityAddComponent2(2,'OrbComponent',{orb_id=0})end
 h.dofile(root..'append/'..({heart='heart_append.lua',heart_better='heart_better_append.lua',perk='perk_pickup.lua',orb='orb_pickup.lua'})[kind])
 h.item_pickup(2,1,'pickup');local q=assert(json.decode_saved(g.nt2_c1_outgoing));eq(#q.queue,1);eq(q.queue[1].senderDisposition,'applied')
 local id=q.queue[1].eventId;eq(#id,36);eq(c[11].hp,2)
 if kind=='heart'then eq(c[11].max_hp,4.5)elseif kind=='heart_better'then eq(c[11].max_hp,5)else eq(vanilla,1)end
 h.item_pickup(2,1,'repeat');eq(#assert(json.decode_saved(g.nt2_c1_outgoing)).queue,1);eq(vanilla,kind=='perk'and 1 or kind=='orb'and 1 or 0)
 e.frame=101;owner:tick(true,1,101);eq(sent[1].publish.eventId,id)
 if kind=='heart'or kind=='heart_better'then
  local peer,pc,_,_,_,po,_,_,pv=setup();pv.event={eventId=id,sequence='1',payload={heart={}},sourceName='Peer',capturedRules=view.snapshot.rules};pv.permit=true
  po:receive(pv);po:tick(true,1,101);eq(pc[11].hp,2.5);eq(pc[11].max_hp,4.5);po:tick(true,1,102);eq(pc[11].hp,2.5)
 end
end
-- Missing helper, main grant, expired frame, context/world mismatch, protected
-- Controls and native recovery markers all preserve ordinary local pickup.
for _,mode in ipairs({'helper','engine','grant','stale','context','world','controls','input','recovery','tokens'})do
 for _,kind in ipairs({'heart','heart_better','perk','orb'})do
  local e,c,entities,g,flags,owner,sent,json=setup();local h=context(e);local vanilla=0;local shared_notice=0
  h.GamePrint=function(text)if text=='$noitatogether_teamperk_was_shared'then shared_notice=shared_notice+1 end end
  h.item_pickup=function()vanilla=vanilla+1 end;h.GetKeyPerkComponents=function()return{perk_id='EXTRA_MONEY'}end;h.IsBannedPerkId=function()return false end
  if mode=='helper'then local real=h.dofile;h.dofile=function(path)if path:find('c1_hook_api',1,true)then error('Helper unavailable')end;return real(path)end
  elseif mode=='engine'then h.GlobalsGetValue=function()error('Globals temporarily unavailable')end
  elseif mode=='grant'then g.nt2_c1_authority=''
  elseif mode=='stale'then e.frame=103
  elseif mode=='controls'then c[13].enabled=false
  elseif mode=='input'then c[13].enabled=false;h.ComponentGetIsEnabled=function()return true end
  elseif mode=='recovery'then e.EntityAddComponent2(1,'VariableStorageComponent',{name='nt2_g3_recovery'})
  else local t=assert(json.decode_saved(g.nt2_c1_tokens));if mode=='context'then t.context=string.rep('02',16)elseif mode=='world'then t.world=string.rep('b',64)else t.ids={}end;g.nt2_c1_tokens=assert(json.encode_saved(t))end
  h.dofile(root..'append/'..({heart='heart_append.lua',heart_better='heart_better_append.lua',perk='perk_pickup.lua',orb='orb_pickup.lua'})[kind]);h.item_pickup(2,1,'fallback')
  -- An empty ID pool still permits the retained shared-heart local arithmetic.
  eq(vanilla,mode=='tokens'and(kind=='heart'or kind=='heart_better')and 0 or 1);eq(g.nt2_c1_outgoing,nil)
  if kind=='perk'then eq(shared_notice,0)end
 end
end
-- The main-context token is consumed only once even across separate item states.
do
 local e,c,entities,g,flags,owner,sent,json=setup();local first=context(e).dofile('mods/noita_together2/files/g3/c1_hook_api.lua');local second=context(e).dofile('mods/noita_together2/files/g3/c1_hook_api.lua')
 local a=assert(first:capture('heart',{}));first:submit(a);local b=assert(second:capture('heart',{}));second:submit(b);eq(a.eventId==b.eventId,false)
 local tokens=assert(json.decode_saved(g.nt2_c1_tokens));eq(#tokens.ids,30)
 -- A stale context cannot import its queue into the current main owner.
 local stale=assert(json.decode_saved(g.nt2_c1_outgoing));stale.context=string.rep('02',16);g.nt2_c1_outgoing=assert(json.encode_saved(stale));e.frame=101;owner:tick(true,1,101);eq(#sent,0)
end
-- Collision callback executes the real API in a distinct biome context.
do
 local e,c,entities,g,flags,owner,sent,json=setup();local h=context(e);h.GetUpdatedEntityID=function()return 2 end
 h.dofile(root..'scripts/checkpoint.lua');h.collision_trigger(1);local cp=assert(json.decode_saved(g.nt2_c1_checkpoint));eq(cp.world,owner.scope);eq(cp.x,'10')
 g.nt2_c1_checkpoint='';c[13].enabled=false;h.collision_trigger(1);eq(g.nt2_c1_checkpoint,'')
 local unavailable=context(e);unavailable.dofile_once=function()end;local read=unavailable.dofile;unavailable.dofile=function(path)if path:find('c1_hook_api',1,true)then error('Helper unavailable')end;return read(path)end
 unavailable.dofile(root..'scripts/checkpoint.lua');unavailable.collision_trigger(1);eq(g.nt2_c1_checkpoint,'')
end
-- Context rollover burns unused IDs but retains immutable pending originals;
-- cold Continue restores those originals as uncertain, never replays gameplay.
do
 local e,c,entities,g,flags,owner,sent,json,view=setup();local api=context(e).dofile('mods/noita_together2/files/g3/c1_hook_api.lua')
 local original=assert(api:capture('heart',{}));api:submit(original);e.frame=101;owner:tick(true,1,101)
 local old=assert(json.decode_saved(g.nt2_c1_tokens));local unused=old.ids[1];eq(#owner.queue,1)
 owner.context=string.rep('02',16);e.frame=102;owner:tick(true,1,102);local fresh=assert(json.decode_saved(g.nt2_c1_tokens));eq(fresh.context,owner.context);eq(fresh.ids[1]==unused,false)
 eq(owner.queue[1].eventId,original.eventId);local next_original=assert(api:capture('heart',{}));api:submit(next_original)
 eq(#assert(json.decode_saved(g.nt2_c1_outgoing)).queue,2)
 local uid=1000;local module=e.dofile('mods/noita_together2/files/g3/c1_local.lua');local content=e.dofile('mods/noita_together2/files/g3/c1_content.lua').load(e)
 local cold=module.new(e,function()uid=uid+1;return string.format('%08d-1111-4111-8111-111111111111',uid)end,function()return true end,content);cold.context=string.rep('03',16);cold:receive(view)
 eq(g.nt2_c1_tokens,'');eq(#cold.queue,2);eq(cold.queue[1].eventId,original.eventId);eq(cold.queue[1].senderDisposition,'uncertain')
 cold:tick(false,1,103);eq(g.nt2_c1_tokens,'');eq(c[11].hp,2);eq(c[11].max_hp,4)
 e.frame=104;cold:tick(true,1,104);eq(assert(json.decode_saved(g.nt2_c1_tokens)).ids[1]==unused,false);eq(#cold.queue,2)
end
-- Actual chosen fungal mapping stays local and goes through the same API.
for _,available in ipairs({true,false})do
 local e,c,entities,g,flags,owner,sent,json=setup();local h=context(e);local converted=0
 h.GameGetWorldStateEntity=function()return 1 end;e.EntityAddComponent2(1,'WorldStateComponent',{EVERYTHING_TO_GOLD=false})
 h.SetRandomSeed=function()end;h.rand=function()return 0 end;h.random_create=function()return{}end
 h.materials_from={{materials={'water','blood'},name_material='water'}};h.materials_to={{material='oil'}}
 h.pick_random_from_table_weighted=function(_,t)return t[1]end;h.get_held_item_material=function()return 0 end
 local types={water=1,blood=2,oil=3};h.CellFactory_GetType=function(k)return types[k]end;h.CellFactory_GetName=function(id)for k,v in pairs(types)do if v==id then return k end end end
 h.CellFactory_GetUIName=tostring;h.GameTextGetTranslatedOrNot=function(v)return v end;h.ConvertMaterialEverywhere=function()converted=converted+1 end
 h.GameCreateParticle=function()end;h.EntityRemoveIngestionStatusEffect=function()end;h.GameTriggerMusicFadeOutAndDequeueAll=function()end;h.GameTriggerMusicEvent=function()end
 h.EntityAddChild=function()end;h.EntityCreateNew=function(name)return e.EntityLoad(name,0,0)end;h.log_messages={'test'};h.random_from_array=function(t)return t[1]end
 if not available then local real=h.dofile;h.dofile=function(path)if path:find('c1_hook_api',1,true)then error('Helper unavailable')end;return real(path)end end
 h.dofile(root..'append/fungal_shift.lua');h.fungal_shift(1,0,0,false);eq(converted,2)
 local queue=json.decode_saved(g.nt2_c1_outgoing or'');eq(queue and #queue.queue or 0,available and 1 or 0)
 h.fungal_shift(1,0,0,false);eq(converted,2)
end
-- Status-effect states use no sharing helper; latched Heartache mode survives
-- a flag change between the actual start/end scripts in restricted contexts.
for _,alternate in ipairs({true,false})do
 local e,c,entities,g,flags=setup();local storage=e.EntityAddComponent2(2,'VariableStorageComponent',{name='effect_hearty',value_float=0,value_bool=false})
 e.EntityAddComponent2(2,'UIIconComponent',{name='$status_hearty'});local h=context(e);h.GetUpdatedEntityID=function()return 2 end;h.dofile_once=function()end
 h.edit_component=function(player,kind,callback)for _,id in ipairs(e.EntityGetComponent(player,kind)or{})do callback(id,{})end end
 flags.NT_alt_heartache=alternate;h.dofile(root..'append/status_effects/hearty_start.lua');eq(c[11].hp,1);eq(c[storage].value_bool,alternate)
 flags.NT_alt_heartache=not alternate;h.EntityGetRootEntity=function()return 1 end;h.dofile(root..'append/status_effects/hearty_end.lua');eq(c[11].hp,2);eq(c[11].max_hp,4)
end
print('PASS '..checks..' restricted C1 hook/API checks; no require/FFI and actual appended scripts')
