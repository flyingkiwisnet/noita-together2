-- Actual packaged entry/inspection/protection/movement/capture/renderer.
-- Engine/native facts are injected; no service, actual Noita or save evidence.
local repo=assert(os.getenv('NT2_A2_REPO'));local raw=dofile;local ffi=require('ffi');local output=print;local logs={};local logger_off=ModSettingGet and ModSettingGet('noita_together2.NT_ENABLE_LOGGER')==false;local logger_registered=0
print=function(s)logs[#logs+1]=s end
local entities,components,owners={}, {}, {};local serial=10000;local frame,time,status=0,0,2;local closed,moves,bodies=0,{},{};local checks=0;local protection_writes=0
local function check(v,message)assert(v,message);checks=checks+1 end
local function add_component(id,kind,v)serial=serial+1;components[serial]=v or {};owners[serial]=id;local list=entities[id].components[kind]or {};entities[id].components[kind]=list;list[#list+1]=serial;return serial end
local function form(id,poly,control,damage,hp)
 entities[id]={alive=true,parent=0,x=id,y=id+10,tags={[poly and 'polymorphed' or 'player_unit']=true},components={}}
 add_component(id,'GameStatsComponent',{is_player=true});if control~=false then add_component(id,'ControlsComponent',{enabled=control~=nil and control or true})end
 if damage~=false then add_component(id,'DamageModelComponent',{hp=hp or 4,max_hp=8,wait_for_kill_flag_on_death=false})end
end
function EntityGetIsAlive(id)return entities[id]and entities[id].alive or false end
function EntityGetWithTag(tag)local ids={};for id,e in pairs(entities)do if e.alive and e.tags[tag]then ids[#ids+1]=id end end;table.sort(ids);return ids end
function EntityGetFirstComponentIncludingDisabled(id,kind)if not EntityGetIsAlive(id)then return nil end;return (entities[id].components[kind]or {})[1]end
function EntityGetComponentIncludingDisabled(id,kind)return EntityGetIsAlive(id)and entities[id].components[kind]or {}end
function ComponentGetValue2(id,key)assert(components[id],'invalid component read');return components[id][key]end
function ComponentSetValue2(id,key,v)assert(components[id]and EntityGetIsAlive(owners[id]),'stale component write');assert(key~='hp'and key~='kill_now','must not heal or force death');if key=='wait_for_kill_flag_on_death'and v==true then protection_writes=protection_writes+1 end;components[id][key]=v end
function EntityAddComponent2(id,kind,v)return add_component(id,kind,v)end
function EntityRemoveComponent(id,c)check(owners[c]==id,'remove marker only from actual component owner');for _,list in pairs(entities[id].components)do for i=#list,1,-1 do if list[i]==c then table.remove(list,i)end end end;components[c]=nil;owners[c]=nil end
function EntityGetParent(id)return entities[id]and entities[id].parent or 0 end
function EntityGetRootEntity(id)local e=entities[id];return e and e.parent~=0 and EntityGetRootEntity(e.parent)or id end
function EntityGetTransform(id)assert(EntityGetIsAlive(id),'stale transform read');local e=entities[id];return e.x,e.y,0 end
function EntitySetTransform(id,x,y)assert(EntityGetIsAlive(id),'stale transform write');entities[id].x=x;entities[id].y=y end
function EntityGetAllChildren(id)local ids={};for child,e in pairs(entities)do if e.alive and e.parent==id then ids[#ids+1]=child end end;return ids end
function EntityGetName()return ''end
function EntityHasTag(id,tag)return entities[id]and entities[id].tags[tag]or false end
function EntityAddTag(id,tag)entities[id].tags[tag]=true end;function EntityRemoveTag(id,tag)entities[id].tags[tag]=nil end
function IsPlayer(id)local c=EntityGetFirstComponentIncludingDisabled(id,'GameStatsComponent');return c and components[c].is_player==true or false end
function LoadGameEffectEntityTo(id)serial=serial+1;local effect=serial;entities[effect]={alive=true,parent=id,tags={},components={}};add_component(effect,'GameEffectComponent',{frames=1});return effect end
function EntityKill(id)entities[id].alive=false end
function EntitySetComponentIsEnabled(id,c,value)check(owners[c]==id,'owned component entity');components[c].enabled=value end
function ComponentGetIsEnabled(c)return components[c].enabled end
function GameGetFrameNum()return frame end;function GameGetRealWorldTimeSinceStarted()return time end
function GameGetWorldStateEntity()return 1 end;function StatsGetValue()return '424242'end;function ModGetActiveModIDs()return {'noita_together2'}end
function GameIsBetaBuild()return false end;function BiomeMapGetName()return '$biome_coalmine'end
local globals={};function GlobalsGetValue(k,f)return globals[k]or f end;function GlobalsSetValue(k,v)globals[k]=v end
function ModMagicNumbersFileAdd()logger_registered=logger_registered+1 end;function ModLuaFileAppend()end;function ModLuaFileGetAppends()return {}end;function dofile_once()end
function SetWorldSeed()error('form change must never set seed')end
local gui=0;function GuiCreate()gui=gui+1;return gui end;function GuiDestroy()end;function GuiStartFrame()end;function GuiText()end;function GuiButton()return false end;function GuiImage()end
function GuiGetScreenDimensions()return 640,240 end;function GuiOptionsAdd()end;GUI_OPTION={NoPositionTween=6}
dofile=function(path)
 if path=='mods/noita_together2/files/g3/c1_catalogue_data.lua'then return raw(repo..'/packages/contracts/g3-c1/catalogue.lua')end
 if path=='mods/noita_together2/files/g3/item_catalogue_data.lua'then return raw(repo..'/packages/contracts/g3-items/catalogue.lua')end
 return raw(repo..'/'..path:gsub('^mods/','mod/'))
end
raw(os.getenv('NT2_POLY_ENTRY_FILE')or repo..'/mod/noita_together2/init-g3-seed-loot.lua')
local function visit(fn,key,value,set,seen)
 seen=seen or {};if seen[fn]then return nil end;seen[fn]=true
 for i=1,70 do local name,old=debug.getupvalue(fn,i);if not name then break end;if name==key then if set then debug.setupvalue(fn,i,value)end;return old or false end end
 for i=1,70 do local name,old=debug.getupvalue(fn,i);if not name then break end;if type(old)=='function'then local found=visit(old,key,value,set,seen);if found~=nil then return found end end end
end
local ctx=ffi.new('uint8_t[16]');ffi.fill(ctx,16,1)
local bridge={_libraries={},nt2_g3_status=function()return status end,nt2_g3_close=function()closed=closed+1 end,
 nt2_g3_association=function()return 0 end,nt2_g3_report=function()return 1 end,nt2_g3_seed_known=function()return 1 end,nt2_g3_world_loaded=function()return 1 end,
 nt2_g3_heartbeat_diagnostic=function()return status==2 and 0 or 7 end,
 nt2_g3_move=function(_,seq,x,y)moves[#moves+1]={sequence=seq,x=x,y=y};return 1 end,nt2_g3_peers=function()return 0 end,
 nt2_g3_social_terminal=function()return 0 end,nt2_g3_social_lease=function()return 0 end,
 nt2_g3_social_publish=function(_,body,size)bodies[#bodies+1]=ffi.string(body,size);return 1 end}
bridge.nt2_g3_offline_choice=function()return 1 end
local native_calls=0;for k,v in pairs(bridge)do if type(v)=='function'then local f=v;bridge[k]=function(...)native_calls=native_calls+1;return f(...)end end end
form(99,false,true,true);form(66,true,true,true);components[EntityGetFirstComponentIncludingDisabled(66,'GameStatsComponent')].is_player=false
for k,v in pairs({player=99,context=ctx,bridge=bridge,seed=424242,actual_seed='424242',actual_seed_matches=true,accepted=true,loaded=true,released=true})do check(visit(OnWorldPreUpdate,k,v,true)~=nil,'bind actual entry '..k)end
local pending_destroy={};local killed=0
if os.getenv('NT2_POLY_CASE')=='continue-deferred-effect'then
 function EntityKill(id)pending_destroy[id]=true;killed=killed+1 end
end
local function step(n)for _=1,n or 1 do frame=frame+6;time=time+0.11;OnWorldPreUpdate();OnWorldPostUpdate();for id in pairs(pending_destroy)do entities[id].alive=false;for _,list in pairs(entities[id].components)do for _,c in ipairs(list)do components[c]=nil;owners[c]=nil end end end;pending_destroy={}end end
local function current()return visit(OnWorldPreUpdate,'player')end
local function control(id)return components[assert(EntityGetFirstComponentIncludingDisabled(id,'ControlsComponent'))]end
local function damage(id)return components[assert(EntityGetFirstComponentIncludingDisabled(id,'DamageModelComponent'))]end
local function changed(id,poly,ctrl,dmg,hp)local old=current();if old then entities[old].alive=false end;form(id,poly,ctrl,dmg,hp)end
step(2);check(#moves>0 and current()==99,'normal subject publishes movement');local before=#moves;local sequence=moves[#moves].sequence
local case=os.getenv('NT2_POLY_CASE')
if case=='continue-deferred-effect'then
 status=1;step(2)
 local owner=visit(OnWorldPreUpdate,'recovery');local adapter=visit(owner.tick,'adapter');owner.record={record='injected saved association',seed='424242',loot='shared:0:424242'};owner.hold=adapter.hold_owned();owner.state='inspecting';owner.generation=7;owner.pending={context=ctx};owner.chosen=true
 adapter.poll=function()return status~=2 and 'lost' or owner.state=='reconnecting' and 'admitted' or 'released',7 end
 check(visit(OnWorldPreUpdate,'c1_enabled',true,true)~=nil,'enable actual C1 release gate');NT2_C1={ready=false,context=string.rep('01',16)}
 local held=visit(OnWorldPreUpdate,'hold');local original_effect=held.effect;check(held.enabled==true and held.wait==false,'original control/death flags retained')
 status=2
 if os.getenv('NT2_CONTINUE_EFFECT_ONLY')~='1'then
  step(3);check(owner.state=='inspecting' and not control(99).enabled and killed==0,'Continue never releases before C1 metadata is ready');check(visit(OnWorldPreUpdate,'hold').effect==original_effect and closed==0,'C1 initialization keeps the existing effect and conversation')
  NT2_C1.ready=true;NT2_C1.context=string.rep('02',16);step(2);check(killed==0 and owner.state=='inspecting','wrong C1 context cannot release')
 end
 NT2_C1.ready=true
 NT2_C1.context=string.rep('01',16)
 -- Revoke immediately after the recovery callback restores its owned hold.
 -- Noita still exposes the killed child for the rest of this callback/frame.
 local inspected=adapter.inspected;adapter.inspected=function(...)local ok=inspected(...);if ok then local release=adapter.release_owned;adapter.release_owned=function(...)release(...);status=1;adapter.release_owned=release end end;return ok end
 step();held=visit(OnWorldPreUpdate,'hold');check(held and held.effect~=original_effect and not control(99).enabled,'same-frame revoke creates a new effect instead of adopting pending destruction');check(entities[original_effect].tags.nt2_g3_owned_hold==nil,'retired effect loses ownership before deferred kill')
 step(2);check(closed==0 and visit(OnWorldPreUpdate,'hold').complete,'deferred destruction does not close recovery or poison later frames')
 -- A stale owned child without a GameEffectComponent must also be skipped.
 local broken=held.effect;local c=EntityGetFirstComponentIncludingDisabled(broken,'GameEffectComponent');entities[broken].components.GameEffectComponent={};components[c]=nil;owners[c]=nil
 step();check(visit(OnWorldPreUpdate,'hold').effect~=broken and closed==0,'missing component is repaired without re-adopting a stale owned child')
 adapter.inspected=inspected;status=2;step(3);check(owner.state=='playing' and control(99).enabled and not damage(99).wait_for_kill_flag_on_death,'eligible Continue restores exact original flags after C1 readiness')
 check(not entities[99].tags.polymorphable_NOT and closed==0,'own polymorph flag restored and native identity retained')
 print=output;print('PASS deferred Continue production entry '..checks..' assertions; actual entry/local_world/C1 gate, deferred engine destruction, injected native facts');return
end
if case=='bank-lock'then
 local social=visit(OnWorldPreUpdate,'social_gameplay');local renderer=social.renderer
 local observer=raw(repo..'/apps/desktop/tools/lifecycle-lc1-permission-observer.lua').new()
 local function observe()
  local l=renderer.lock.lease;local owned=l and components[l.marker]and components[l.marker].name=='nt2_a2_composer_lock'and components[l.marker].value_bool==true
  local protected=false;for _,id in ipairs(EntityGetComponentIncludingDisabled(99,'VariableStorageComponent'))do if components[id].name=='nt2_g3_recovery'then protected=true end end
  return observer:sample({frame=frame,controls=control(99).enabled,lease_owned=owned==true,native_status=status,native_status_frame=frame,protected=protected},true)
 end
 check(observe().permitted,'observer starts from actual healthy entry state')
 renderer:set_bank(social.bank);social.bank.ui.open=true;renderer.model.bank_open=true
 check(renderer.lock:open(99,true),'real bank window acquires its input lease')
 local marker=renderer.lock.lease.marker;local writes=protection_writes;local moving=#moves
 step(3)
 check(not control(99).enabled and renderer.lock.lease and components[marker]~=nil,'entry recovery must preserve the current bank-owned input lease across frames')
 check(protection_writes==writes and not damage(99).wait_for_kill_flag_on_death,'bank typing never acquires native protection')
 check(#moves>moving,'current native movement publication keeps progressing during bank typing')
 local typing=observe();check(typing.typing and typing.permitted and typing.holds==0,'actual live bank lock preserves permission without hiding a hold')
 social.bank.ui.open=false;step(1);check(control(99).enabled and not renderer.lock.lease,'closing bank restores the owned input flags')
 observe()
 social.bank.ui.open=true;renderer.model.bank_open=true;check(renderer.lock:open(99,true),'bank lease before real native permission loss')
 check(observe().typing,'actual bank is typing immediately before injected permission loss')
 status=1;step(2);check(not control(99).enabled and damage(99).wait_for_kill_flag_on_death and not renderer.lock.lease,'real permission loss closes UI ownership before native protection')
 local loss=observe();check(loss.lost and not loss.permitted and loss.holds==1,'genuine entry protection during bank input increments the unchanged healthy-peer hold history')
 status=2;step(3);check(control(99).enabled and not damage(99).wait_for_kill_flag_on_death and not social.bank.ui.open,'current authority restores real pre-bank control originals without reopening bank')
 check(observe().holds==1,'entry recovery never resets the recorded genuine hold')
 print=output;print('PASS bank-lock production entry '..checks..' assertions; actual entry/renderer/lock, injected engine/native facts');return
end
if case=='offline-outage'or case=='offline-terminal'or case=='recovery-components'then
 changed(199,true,true,true);status=1;step(2)
 local owner=visit(OnWorldPreUpdate,'recovery');local adapter=visit(owner.tick,'adapter')
 owner.record={record='injected saved association',seed='424242',loot='shared:0:424242'};owner.hold=adapter.hold_owned()
 if case=='recovery-components'then
  owner.state='inspecting';owner.generation=7;local admitted=false;adapter.poll=function()if status~=2 then return 'lost',7 end;if not admitted then admitted=true;return 'admitted',7 end;return 'released',7 end
  local c=EntityGetFirstComponentIncludingDisabled(199,'ControlsComponent');local d=EntityGetFirstComponentIncludingDisabled(199,'DamageModelComponent')
  entities[199].components.ControlsComponent={};entities[199].components.DamageModelComponent={};components[c]=nil;components[d]=nil;owners[c]=nil;owners[d]=nil
  step(2);check(closed==0,'actual local_world record path never writes removed held components')
  add_component(199,'ControlsComponent',{enabled=true});add_component(199,'DamageModelComponent',{hp=4,max_hp=8,wait_for_kill_flag_on_death=false})
  step(2);check(not control(199).enabled and damage(199).wait_for_kill_flag_on_death,'actual recovery early-return path captures newly appearing components')
  visit(OnWorldPreUpdate,'loaded',false,true);changed(299,false,true,true);OnPlayerSpawned(299);step(2)
  check(current()==299 and visit(OnWorldPreUpdate,'hold').player==299 and not control(299).enabled,'replacement spawn callback transfers actual recovery hold while native loaded is pending')
  damage(299).hp=0;visit(OnWorldPreUpdate,'loaded',true,true);status=2;step(3);check(owner.state=='playing'and control(299).enabled and not damage(299).wait_for_kill_flag_on_death,'actual local_world release restores replacement component originals after permission at valid zero HP')
 else
  owner.state=case=='offline-terminal'and 'terminal'or 'reconnecting';owner:offline()
  check(control(199).enabled and not damage(199).wait_for_kill_flag_on_death and not visit(OnWorldPreUpdate,'hold'),'actual local_world offline choice restores held transformed body without online permission')
  local calls=native_calls;changed(299,true,true,true);OnPlayerSpawned(299);step(2);changed(399,false,true,true);OnPlayerSpawned(399);step(2)
  check(control(299).enabled and control(399).enabled and not visit(OnWorldPreUpdate,'hold'),'offline normal/poly/return does not acquire another hold')
  check(native_calls==calls,'offline form transitions make no new native calls')
 end
 check(not table.concat(logs,'\n'):find('protection_failed',1,true),'actual recovery/offline path has no stale protection error loop')
 print=output;print('PASS '..case..' production-entry/local_world '..checks..' assertions; injected native/engine facts');return
end
if os.getenv('NT2_POLY_CASE')=='stale-protection'then
 changed(199,true,true,true);status=1;step(5);check(closed==0 and current()==199 and not control(199).enabled,'stale original must protect current form without closing/recreating owner')
 check(not table.concat(logs,'\n'):find('protection_failed',1,true),'stale entity must not enter per-frame protection error loop')
 print=output;print('PASS stale protection production-entry regression '..checks..' assertions; injected native/engine facts');return
end
local prior_protection=protection_writes;changed(199,true,true,true);step(7);check(current()==199 and #moves>before and moves[#moves].x==199000 and moves[#moves].sequence>sequence,'new local polymorph replaces removed original without resetting movement fence')
check(protection_writes==prior_protection,'healthy ready form with current world permission never acquires a transient hold')
check(control(199).enabled and closed==0 and control(66).enabled,'healthy transformed form keeps actual owner and ignores unrelated polymorphed NPC');check(bodies[#bodies]:find('"hpKnown":false',1,true)and bodies[#bodies]:find('"sampoKnown":false',1,true)and bodies[#bodies]:find('"biome"',1,true),'transformed location captured without inventing ordinary HP/inventory')
changed(299,true,true,true);step(2);changed(399,false,true,true);OnPlayerSpawned(399);step(2);check(current()==399 and moves[#moves].x==399000 and control(399).enabled,'sequential forms and ordinary return reacquire even loaded spawn callback')
entities[399].alive=false;before=#moves;step(3);check(not current()and #moves==before and closed==0,'brief no-player gap neither samples stale actor nor closes native owner')
status=1;form(499,false,true,true);step(2);check(current()==499 and not control(499).enabled and damage(499).wait_for_kill_flag_on_death,'new body held during authority outage')
local owner=visit(OnWorldPreUpdate,'recovery');local adapter=visit(owner.tick,'adapter');local token=adapter.hold_owned();check(token~=nil,'actual recovery adapter caches per-world hold ownership')
changed(599,true,true,true);step(3);check(current()==599 and not control(599).enabled and adapter.hold_owned()==token,'held form replacement preserves cached ownership identity')
adapter.release_owned(token);check(not control(599).enabled,'cached token cannot release without current native permission')
status=2;step(3);check(control(599).enabled and not damage(599).wait_for_kill_flag_on_death,'legitimate current permission restores replacement flags without leaked hold')
status=1;changed(699,true,true,false);step(3);check(not control(699).enabled and closed==0,'missing damage component holds current controls without close/error loop')
status=2;step(2);check(not control(699).enabled,'incomplete form cannot inherit online play')
add_component(699,'DamageModelComponent',{hp=4,max_hp=8,wait_for_kill_flag_on_death=false});step(4);check(control(699).enabled and not damage(699).wait_for_kill_flag_on_death,'component completion restores only captured flags after permission')
status=1;step(2);damage(699).hp=-0.1;step(2);check(not damage(699).wait_for_kill_flag_on_death and not control(699).enabled,'genuine transformed nonpositive HP is not wait-locked or resurrected')
status=2;changed(799,false,true,true,0);damage(799).hp=0;step(10);local zero=false;for _,body in ipairs(bodies)do if body:find('"hpMilli":0',1,true)then zero=true end end;check(zero,'valid ordinary zero HP remains observable independently of control eligibility')
changed(899,false,true,true);step(3);local social=visit(OnWorldPreUpdate,'social_gameplay');check(social.renderer.lock:open(899,true),'composer owns original controls before polymorph')
local oldmark;for _,id in ipairs(EntityGetComponentIncludingDisabled(899,'VariableStorageComponent'))do if components[id].name=='nt2_a2_composer_lock'then oldmark=components[id]end end
changed(999,true,true,true);control(999).enabled=false;add_component(999,'VariableStorageComponent',{name=oldmark.name,value_bool=oldmark.value_bool});step(4);check(control(999).enabled and not social.renderer.lock.lease,'inherited composer-disabled flags restore only on current valid form/permission')
status=1;step(2);local flags;for _,id in ipairs(EntityGetComponentIncludingDisabled(999,'VariableStorageComponent'))do if components[id].name=='nt2_g3_recovery'then flags=components[id].value_string end end
changed(1099,false,true,true);control(1099).enabled=false;damage(1099).wait_for_kill_flag_on_death=true;add_component(1099,'VariableStorageComponent',{name='nt2_g3_recovery',value_string=flags});step(3);check(not control(1099).enabled,'inherited held form never releases during outage');status=2;step(4);check(control(1099).enabled and not damage(1099).wait_for_kill_flag_on_death,'inherited marker preserves real original flags on ordinary return')
status=1;changed(1199,true,false,true);step(3);check(current()==1199 and damage(1199).wait_for_kill_flag_on_death and closed==0,'missing controls does not select another actor or close native owner')
add_component(1199,'ControlsComponent',{enabled=true});step(2);check(not control(1199).enabled,'newly available controls remain held during outage');status=2;step(3);check(control(1199).enabled,'legitimate permission restores newly captured control flags')
-- Ambiguous engine identities cannot select the first entry or reset the owner.
form(1299,true,true,true);local api=dofile('mods/noita_together2/files/g3/game_api.lua');check(api.resolve_player(_G,1199)==nil,'ambiguous local identities are not an arbitrary first entity');entities[1299].alive=false
-- Replaced damage while composer owns input must never read a stale component.
local lock=dofile('mods/noita_together2/files/g3/a2_input_lock.lua').new(_G);check(lock:open(1199,true),'composer before component transition');local olddamage=EntityGetFirstComponentIncludingDisabled(1199,'DamageModelComponent');entities[1199].components.DamageModelComponent={};components[olddamage]=nil;owners[olddamage]=nil;lock:close(false)
check(#dofile('mods/noita_together2/files/g3/a2_input_lock.lua').inherited(_G,1199)==1,'missing component keeps owned recovery evidence without false death')
add_component(1199,'DamageModelComponent',{hp=4,max_hp=8,wait_for_kill_flag_on_death=false});status=1;step(3);check(not control(1199).enabled,'missing-component composer marker transfers to owned hold');status=2;step(3);check(control(1199).enabled,'restored current components release only original owned controls')
local all=table.concat(logs,'\n');check(not all:find('protection_failed',1,true)and not all:find('g3_gameplay_failed',1,true)and closed==0,'no stale-entity protection/close/recreate loop')
if logger_off then check(logger_registered==0,'logger opt-out skips magic-number logger registration');check(#logs==0,'entry and movement diagnostics obey captured restart logger opt-out: '..all)end
check(visit(OnWorldPreUpdate,'context')==ctx,'all form transitions preserve native world context')
print=output;print('PASS polymorph packaged entry '..checks..' assertions; current subject, movement/HUD, gaps, held expiry, incomplete/dead forms and owned input/hold; injected facts')
