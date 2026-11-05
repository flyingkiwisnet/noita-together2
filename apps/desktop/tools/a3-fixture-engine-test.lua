-- Validate the composition's injected component surface against the actual
-- production visual adapter before Root starts any TLS/DB composition.
local repo=assert(os.getenv('NT2_A3_REPO'));local raw=dofile
raw(repo..'/apps/desktop/tools/mod-protection-stubs.lua');local state=NT2_FIXTURE;state.components[2].hp=4;state.components[6]={}
local ghosts={};local serial=1000;local inputs,outputs={},{};local checks=0
local function check(v,s)assert(v,s);checks=checks+1 end
local alive=EntityGetIsAlive;EntityGetIsAlive=function(id)return ghosts[id]~=nil or alive(id)end
EntityLoad=function(path,x,y)serial=serial+1;ghosts[serial]={x=x,y=y,ghost=path:find('/ghost.xml',1,true)~=nil};return serial end
local kill=EntityKill;EntityKill=function(id)if ghosts[id]then ghosts[id]=nil else kill(id)end end
EntityGetName=function()return ''end;EntityGetParent=function()return 0 end;EntityGetRootEntity=function(id)return id end
EntityGetTransform=function(id)local en=ghosts[id];return en and en.x or 20,en and en.y or 30,0 end;EntitySetTransform=function()end;GuiImageButton=function()return false end;GuiDestroy=function()end
local function input(k,remove)local v=inputs[k];if remove then inputs[k]=nil end;return v end;local function mark(k,v)outputs[k]=v end
local forms=raw(repo..'/apps/desktop/tools/lifecycle-lc1-polymorph.lua').new(_G,state,input,mark)
local helper=raw(repo..'/apps/desktop/tools/lifecycle-lc1-a3.lua').new(_G,state,forms,ghosts,input,mark)
dofile=function(path)local previous=os.getenv('NT2_A3_SOCIAL_LOCAL_FILE');if previous and previous~=''and path=='mods/noita_together2/files/g3/social_local.lua'then return raw(previous)end;local oldnative=os.getenv('NT2_A3_SOCIAL_NATIVE_FILE');if oldnative and oldnative~=''and path=='mods/noita_together2/files/g3/social_native.lua'then return raw(oldnative)end;return raw(repo..'/'..path:gsub('^mods/','mod/'))end
local v=dofile('mods/noita_together2/files/g3/social_visual.lua').new(_G)
for _,id in ipairs(dofile('mods/noita_together2/files/g3/social_catalogue.lua').ids)do check(v:start(forms:current(),id,true,0),'fixture supports real local visual '..id);v:stop(forms:current())end
local ghost=EntityLoad('mods/noita_together2/files/g3/ghost.xml',100,200);check(v:colour(ghost,'blue',{hat=true,amulet=true,gem=true},false,false),'fixture per-actor ghost colour');check(v:start(ghost,'wave',false,60),'fixture actual ghost emote adapter')
local body=EntityGetFirstComponentIncludingDisabled(99,'CharacterPlatformingComponent');check(ComponentGetValue2(body,'run_velocity')==287,'fixture actual modified local motion restored')
v:start(99,'wave',true,0);inputs['a2-form']='199:poly';forms:apply();v:stop(99);check(v:start(199,'heart',true,0),'fixture current replacement component membership');v:stop(199);local nextbody=EntityGetFirstComponentIncludingDisabled(199,'CharacterPlatformingComponent');check(ComponentGetValue2(nextbody,'run_velocity')==287,'fixture replacement motion restored')
inputs['a3-flags']='secret_hat,secret_amulet_gem';check(HasFlagPersistent('secret_hat')and not HasFlagPersistent('secret_amulet'),'actual flag observations separated');inputs['a3-menu']='held';check(InputIsKeyDown(20)and not InputIsKeyDown(4),'only injected Q is held')
helper:snapshot({visual=v,peers={},states={},rendered={},menu={},robe='red'});check(outputs['a3-view.json']:find('"engineFactsInjected":true',1,true)and outputs['a3-view.json']:find('"serviceMocked":false',1,true),'snapshot labels evidence honestly');v:clear()
-- Actual social_native -> controls/menu -> fixture GuiImageButton -> copy-only
-- publisher. Only native lease/copy observations are injected, no service reply
-- or direct selection callback is substituted. Reproduce the Root lease gaps.
local ffi=require('ffi');local lease=true;local attempts={};local context=ffi.new('uint8_t[16]');ffi.fill(context,16,1)
GameGetRealWorldTimeSinceStarted=function()return state.frame/60 end
local dll={nt2_g3_social_terminal=function()return 0 end,nt2_g3_social_next=function()return 0 end,
 nt2_g3_startup_nonce=function(buffer,size)ffi.fill(buffer,size,(state.frame%254)+1);return 1 end,
 nt2_g3_social_publish=function(_,buffer,size)local body=ffi.string(buffer,size);attempts[#attempts+1]=body;local result=lease and 1 or 0;helper:observe_social_copy(result,body);return result end,
 nt2_g3_social_lease=function(_,header)if not lease then return 0 end;ffi.fill(header,64,0);ffi.copy(header+8,context,16);header[24]=1;header[28]=75;return 64 end}
local native=dofile('mods/noita_together2/files/g3/social_native.lua').new(dll,context,ffi,_G,dofile('mods/noita_together2/files/g3/validation.lua'),dofile('mods/noita_together2/files/g3/social_game_api.lua'),dofile('mods/noita_together2/files/g3/hud_poi_local.lua'),dofile('mods/noita_together2/files/g3/social_dispatch.lua'))
local function tick(ready,playing)lease=ready;state.frame=state.frame+1;native:tick(playing~=false,forms:current(),playing~=false)end
local function actions()local n=0;for _,body in ipairs(attempts)do if body:find('"kind":"emote"',1,true)or body:find('"kind":"cosmetic"',1,true)then n=n+1 end end;return n end
local function last_action(kind)for i=#attempts,1,-1 do if attempts[i]:find('"kind":"'..kind..'"',1,true)then return attempts[i]end end end
inputs['a3-menu']=nil;tick(true)
local initial_cosmetic=last_action('cosmetic');check(actions()==1 and initial_cosmetic:find('"robeId":"purple"',1,true)and initial_cosmetic:find('"hat":true',1,true)and initial_cosmetic:find('"amulet":false',1,true)and initial_cosmetic:find('"gem":true',1,true),'first eligible cosmetic includes persistent flags already present before Start')
-- Focused adapter outcome only; actual successful service evidence is Root's
-- TLS/SQL composition. Resolve the automatic publication before manual input.
native.a3:outcome(native.a3.pending_cosmetic.identity,1,'','')
inputs['a3-menu']='held';tick(true);check(native.a3.menu.open,'actual native path opens held-Q menu')
for _=1,20 do tick(false);check(native.a3.menu.open and not native.a3.menu.locked,'copy gaps retain held menu');tick(true)end
lease=false;local before=actions();inputs['a3-select']='color_red';tick(false);local deferred_colour=native.a3.pending_cosmetic.identity;check(not inputs['a3-select']and native.a3.robe=='red'and actions()==before,'one colour intent waits without publishing during copy gap');tick(true);check(native.a3.pending_cosmetic.identity==deferred_colour and actions()==before+1 and last_action('cosmetic'):find('"robeId":"red"',1,true),'same physical colour choice copies once when permission returns without another click')
helper:snapshot(native.a3);local copied=native.a3.pending_cosmetic.identity
check(outputs['a3-view.json']:find('"lastCosmeticOriginal":"'..copied..'"',1,true),'fixture observes exact copied colour identity before its outcome')
-- Inject a refusal only to test real rollback/no-replay and passive identity
-- observation. Successful TLS/SQL outcomes belong to Root's combined run.
native.a3:outcome(copied,2,'Social selection rate limit','');helper:snapshot(native.a3)
check(native.a3.robe=='purple'and native.a3.pending_cosmetic==nil and outputs['a3-view.json']:find('"lastCosmeticOriginal":"'..copied..'"',1,true),'refusal rolls back optimistic robe while copied identity remains observable')
before=actions();for _=1,5 do tick(true);helper:snapshot(native.a3)end
check(actions()==before and native.a3.robe=='purple','later callbacks never auto-replay a refused colour original')
before=actions();inputs['a3-select']='wave';tick(false);local deferred_wave=native.a3.local_emote.identity;check(not native.a3.local_emote.started and actions()==before,'one emote intent waits without publishing or first-frame animation');tick(true);check(native.a3.local_emote.identity==deferred_wave and native.a3.local_emote.started,'same physical emote selection progresses on next update without another click');tick(false);check(native.a3.local_emote.started,'selection survives following short copy gap')
local wave_original=native.a3.local_emote.identity;for _=1,5 do tick(true);helper:snapshot(native.a3)end
check(actions()==before+1 and native.a3.local_emote.identity==wave_original and outputs['a3-view.json']:find('"lastOriginal":"'..wave_original..'"',1,true),'one consumed wave click produces one copied original across held-Q callbacks')
-- Real cleanup spans a Q press. Holding the blocked key must stay locked even
-- after Stop copies; an observed release and a fresh press are required.
inputs['a3-menu']=nil;tick(true);native.a3:cancel(true);before=actions();inputs['a3-menu']='held';tick(false)
check(native.a3.pending_stop and native.a3.menu.locked and not native.a3.menu.open,'Q pressed during pending cleanup correctly locks the protected menu')
tick(true);check(not native.a3.pending_stop and native.a3.menu.locked and not native.a3.menu.open and actions()==before+1,'Stop copies once but held Q remains locked until release')
inputs['a3-select']='heart';tick(true);check(inputs['a3-select']=='heart'and not native.a3.local_emote and actions()==before+1,'locked physical selection is unconsumed and cannot create an original')
inputs['a3-menu']=nil;inputs['a3-select']=nil;tick(true);tick(true);check(not native.a3.menu.locked and not native.a3.menu.open,'real released callbacks clear the cleanup key lock')
inputs['a3-menu']='held';tick(true);check(native.a3.menu.open,'fresh Q press reopens after cleanup without weakening protection')
inputs['a3-select']='heart';tick(false);helper:snapshot(native.a3)
local heart_original=native.a3.local_emote and native.a3.local_emote.identity
check(not inputs['a3-select']and heart_original and actions()==before+1,'one heart intent retained across copy gap without native publish')
tick(true);helper:snapshot(native.a3)
check(native.a3.local_emote.identity==heart_original and actions()==before+2,'same heart original copies when permission returns without reclicking')
for _=1,4 do tick(true);helper:snapshot(native.a3)end
check(actions()==before+2 and native.a3.local_emote.identity==heart_original,'later held callbacks preserve the copied original without replay')
state.components[1].enabled=false;tick(false,false);check(not native.a3.menu.open and not native.a3.local_emote and state.components[1].enabled==false,'actual protection closes menu/cancels animation without releasing foreign controls');native:close()
check(not outputs['a3-first-ghost-loss.json'],'ghost-loss capture stays quiet until explicitly armed')
v:colour(ghost,'blue',{hat=false,amulet=true,gem=false},false,false)
local completed={id='heart',event='11111111-1111-8111-8111-111111111111',revision=1,duration=101,age_ms=5000,at=state.frame/60,active=false}
local observed={visual=v,peers={peer={status=1,ghost=string.rep('b',32)}},states={peer={emote=completed}},rendered={peer={entity=ghost}},menu={},robe='purple'}
helper:snapshot(observed);check(outputs['a3-view.json']:find('"duration":101',1,true)and outputs['a3-view.json']:find('"elapsed":300',1,true)and not v.active[ghost],'passive finite history distinguishes already-completed canonical delivery without creating a visual')
for revision=2,32 do completed.revision=revision;helper:snapshot(observed)end;local _,history_count=outputs['a3-view.json']:gsub('"duration":101','');check(history_count==16,'canonical/render transition history is bounded to16 facts')
inputs['a3-watch-retained']='armed';helper:observe_peers(0);EntityKill(ghost)
local first_loss=outputs['a3-first-ghost-loss.json'];check(first_loss and first_loss:find('"result":0',1,true)and first_loss:find('"robe":"blue"',1,true)and first_loss:find('a3%-fixture%-engine%-test.lua')and not EntityGetIsAlive(ghost),'armed capture records actual removal caller, peer result and previous presentation without preventing removal')
local other=EntityLoad('mods/noita_together2/files/g3/ghost.xml',0,0);EntityKill(other);check(outputs['a3-first-ghost-loss.json']==first_loss,'first-loss evidence is bounded to one record and never overwritten by cleanup')
-- A bounded one-slot native copy surface. ACK frees one opportunity at each
-- callback, with a due changed HUD sample. No HTTP/service reply is fabricated.
-- This reproduces automatic HUD capture monopolizing explicit post-form input.
inputs['a3-watch-retained']=nil;inputs['a3-menu']=nil;inputs['a2-form']='299:normal';forms:apply();state.components[1].enabled=true
local boot,occupied=true,false;local copies={};local queue_dll={}
for key,value in pairs(dll)do queue_dll[key]=value end
queue_dll.nt2_g3_social_publish=function(_,buffer,size)local body=ffi.string(buffer,size);if not boot and occupied then return 0 end;if not boot then occupied=true;copies[#copies+1]=body end;return 1 end
local queued=dofile('mods/noita_together2/files/g3/social_native.lua').new(queue_dll,context,ffi,_G,dofile('mods/noita_together2/files/g3/validation.lua'),dofile('mods/noita_together2/files/g3/social_game_api.lua'),dofile('mods/noita_together2/files/g3/hud_poi_local.lua'),dofile('mods/noita_together2/files/g3/social_dispatch.lua'))
lease=true;state.frame=state.frame+1;queued:tick(true,forms:current(),true);queued.a3:outcome(queued.a3.pending_cosmetic.identity,1,'','');boot=false
for attempt=1,12 do
 occupied=false;state.components[2].hp=4+attempt/1000;state.frame=state.frame+120;inputs['a3-menu']='held';inputs['a3-select']='wave';queued:tick(true,forms:current(),true)
 if queued.a3.local_emote then break end
end
check(queued.a3.local_emote and copies[1]:find('"kind":"emote"',1,true),'explicit post-form emote wins a free copy opportunity before a due changed HUD sample')
inputs['a3-menu']=nil;inputs['a3-select']=nil;occupied=false;state.components[2].hp=5;state.frame=state.frame+120;queued:tick(true,forms:current(),true)
check(copies[#copies]:find('"kind":"hud"',1,true),'HUD still publishes at the next opportunity after explicit input')
queued.a3:cancel(true);occupied=false;state.components[2].hp=6;state.frame=state.frame+120;queued:tick(true,forms:current(),true)
check(not queued.a3.pending_stop and copies[#copies]:find('"kind":"emote"',1,true)and copies[#copies]:find('"active":false',1,true),'owned Stop cleanup also wins a free copy opportunity before HUD refresh');queued:close()
local lease_header=ffi.new('uint8_t[64]');lease_header[28]=51;helper:observe_social_lease(64,lease_header)
for _=1,20 do helper:observe_social_copy(0,'{"kind":"emote","id":"'..string.rep('a',32)..'","active":true,"notRecorded":"private-payload"}')end
helper:snapshot(nil);local copy_snapshot=outputs['a3-view.json'];local _,refusal_rows=copy_snapshot:gsub('"kind":"emote"','')
check(refusal_rows==16 and copy_snapshot:find('"remainingMs":51',1,true)and copy_snapshot:find('"refused":20',1,true)and not copy_snapshot:find('private%-payload'),'copy refusal evidence records actual results/lease, is bounded and excludes raw payload')
local poi_model=queued.renderer.model;poi_model:edit('fixture POI');local result=poi_model:submit(function()return nil,'Optional message lane is busy; keep the draft and try again'end,0,0,true);helper:observe_poi_submit(poi_model,result);helper:snapshot(nil)
check(outputs['a3-view.json']:find('"copied":false',1,true)and outputs['a3-view.json']:find('"poiSubmits":1',1,true),'POI observation distinguishes actual model busy/no-copy outcome')
local poi_id=string.rep('a',32);result=poi_model:submit(function()return poi_id end,0,0,true);helper:observe_poi_submit(poi_model,result);helper:observe_social_copy(1,'{"kind":"poi","id":"'..poi_id..'","text":"not-recorded"}')
for _=1,20 do helper:observe_social_copy(1,'{"kind":"hud"}')end;helper:snapshot(nil)
check(outputs['a3-view.json']:find('"lastPoiCopied":{"frame":'..state.frame..',"id":"'..poi_id..'"}',1,true)and outputs['a3-view.json']:find('"pending":"'..poi_id..'"',1,true)and not outputs['a3-view.json']:find('not%-recorded'),'copied POI identity remains visible after trace rotation, without recording its payload')
-- Actual native movement owner and A3 visual presenter share the same engine
-- entities. Inject read statuses only; no successful service reply is claimed.
inputs['a3-menu']=nil;inputs['a3-select']=nil
local packet=ffi.new('uint8_t[864]');ffi.copy(packet,'NG3R',4);packet[4]=2;packet[5]=1;packet[6]=1;ffi.copy(packet+8,context,16);packet[24]=200;packet[28]=1
ffi.fill(packet+32,16,10);ffi.fill(packet+544,16,10);ffi.copy(packet+560,'NTG3',4);packet[564]=1;packet[566]=1;ffi.copy(packet+568,context,16)
local peer_hash=string.rep('0a',16);local read_result,native_status,pose_sequence=864,2,0
local function word(at,value)local bytes=ffi.new('uint32_t[1]',value);ffi.copy(packet+at,bytes,4)end
word(588,220000);word(592,8000)
local read_bridge={nt2_g3_move=function()return 1 end,nt2_g3_status=function()return native_status end,nt2_g3_peers=function(_,out)if read_result==864 then ffi.copy(out,packet,864)end;return read_result end}
local playback=dofile('mods/noita_together2/files/g3/native_owner.lua').new(read_bridge,context,ffi,dofile('mods/noita_together2/files/g3/game_api.lua'),dofile('mods/noita_together2/files/g3/validation.lua'),dofile('mods/noita_together2/files/g3/movement.lua'),dofile('mods/noita_together2/files/g3/ghosts.lua'))
a3_ghosts=function()return playback:entities()end
local presentation=dofile('mods/noita_together2/files/g3/social_local.lua').new(_G,function()return nil end,{model={},lock={close=function()end}})
presentation:roster('read-gap','local',{{actor='peer',ghost=peer_hash,status=1}})
local loop_event='55555555-5555-8555-8555-555555555555';local started_at=GameGetRealWorldTimeSinceStarted()
check(presentation:cosmetic({actor='peer',ghost=peer_hash,event=loop_event,sequence=1,revision=1,robe='blue',hat=true,amulet=false,gem=true},started_at),'read-gap peer actual current cosmetic accepted')
check(presentation:emote({actor='peer',ghost=peer_hash,event=loop_event,sequence=1,revision=1,id='wave',active=true,duration=0,age_ms=31000},started_at),'read-gap aged loop accepted')
local function step(result,playing,frames,fresh)
 read_result=result;state.frame=state.frame+(frames or 1);if fresh then pose_sequence=pose_sequence+1;word(584,pose_sequence)end
 playback:tick(forms:current(),state.frame,playing~=false);presentation:tick(playing~=false,forms:current(),false)
 return playback:entities()[peer_hash]
end
local original_entity=step(864,true,1,true);local original_holder=presentation.visual.active[original_entity].holder
local colours_before=outputs['a3-view.json'];helper:snapshot(presentation);colours_before=outputs['a3-view.json']:match('"particles":(%d+)')
check(step(0,true)==original_entity and presentation.visual.active[original_entity].holder==original_holder,'one unavailable read preserves actual ghost and same accepted loop holder')
check(step(864,true,1,true)==original_entity and presentation.visual.active[original_entity].holder==original_holder,'fresh read after gap preserves holder and original')
helper:snapshot(presentation);check(outputs['a3-view.json']:match('"particles":(%d+)')==colours_before,'copy gap never replays colour particles')
check(step(0,true,121)==nil,'unavailable reads never refresh unchanged receiver-clock two-second pose staleness')
local recreated=step(864,true,1,true);check(recreated and recreated~=original_entity and presentation.states.peer.emote.event==loop_event and presentation.states.peer.emote.at==started_at,'legitimate recreation keeps canonical original and relative-age anchor')
local heart_event='66666666-6666-8666-8666-666666666666';check(presentation:emote({actor='peer',ghost=peer_hash,event=heart_event,sequence=2,revision=2,id='heart',active=true,duration=101,age_ms=1000},GameGetRealWorldTimeSinceStarted()),'finite heart accepted at existing elapsed age')
recreated=step(864,true,1,true);local heart=presentation.visual.active[recreated];check(heart and heart.id=='heart','eligible aged finite heart starts on actual current entity')
local heart_holder=heart.holder;step(0,true);check(presentation.visual.active[recreated].holder==heart_holder,'finite visual also keeps its holder through one missed read')
check(step(-1,true)==nil,'known denial removes actual ghost/effect immediately')
check(step(0,true,120)==nil,'availability after denial cannot resurrect a ghost')
local after_finite=step(864,true,1,true);check(after_finite and not presentation.visual.active[after_finite]and presentation.visual.colours[after_finite].robe=='blue','fresh valid recreation restores current robe but never replays completed finite animation')
helper:snapshot(presentation);check(outputs['a3-view.json']:match('"particles":(%d+)')==colours_before,'legitimate recreation never replays old colour burst')
native_status=1;check(step(0,true)==nil,'independent real protection status cannot retain playback on missing read');native_status=2
step(864,true,1,true);packet[8]=2;check(step(864,true)==nil,'changed copied scope rejects immediately');packet[8]=1;check(step(0,true)==nil,'invalid copied batch stays rejected across availability');step(864,true,1,true)
packet[7]=1;check(step(864,true)==nil,'malformed copied batch cannot retain entities');packet[7]=0;check(step(0,true)==nil,'malformed rejection cannot become availability');step(864,true,1,true)
packet[6]=0;packet[28]=0;ffi.fill(packet+32,512,0);check(step(864,true)==nil,'fresh valid empty roster immediately removes prior actor')
packet[6]=1;packet[28]=1;ffi.fill(packet+32,16,10);step(864,true,1,true);check(step(864,false)==nil,'local protection still clears actual presentation')
step(864,true,1,true);native_status=0;check(step(0,true)==nil,'authority expiry or terminal native status never becomes a read gap');native_status=2
step(864,true,1,true);read_bridge.nt2_g3_status=nil;check(step(0,true)==nil,'missing independent status export fails closed');read_bridge.nt2_g3_status=function()return 2 end
step(864,true,1,true);check(step(-2,true)==nil,'unknown native read status fails closed');presentation:close();playback:close()
print('PASS A3 composition engine surface '..checks..' actual-visual-adapter assertions; no service/DB run')
