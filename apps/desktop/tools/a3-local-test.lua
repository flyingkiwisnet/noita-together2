-- Production A3 adapters; injected Noita APIs, no actual engine acceptance.
local repo=assert(os.getenv('NT2_A3_REPO'));local raw=dofile
dofile=function(path)local previous=os.getenv('NT2_A3_SOCIAL_VISUAL_FILE');if previous and previous~=''and path=='mods/noita_together2/files/g3/social_visual.lua'then return raw(previous)end;return raw(repo..'/'..path:gsub('^mods/','mod/'))end
local root='mods/noita_together2/files/g3/';local cat=dofile(root..'social_catalogue.lua');local control=dofile(root..'social_controls.lua');local pref=dofile(root..'social_preferences.lua');local checks=0
local api_errors={};local function api_error(s)api_errors[#api_errors+1]=s;error(s)end
local function check(v,s)assert(v,s..(#api_errors>0 and '; strict API: '..api_errors[1]or ''));checks=checks+1 end
check(#cat.ids==20 and #cat.robes==6,'complete catalogue')
for _,bad in ipairs({'','key_code,mouse_code,joystick_code','key_code,20,,mouse_code','key_code,no','key_code,20,key_code,21','mouse_code,0','joystick_code,57'})do check(not control.parse(bad),'invalid binding never always on')end
local held={key_code={},mouse_code={},joystick_code={}};local settings={};local sounds={};local entities,components={},{};local serial=1000;local loaded={};local frame,now=0,0;local click=nil;local rendered={};local gui=0
local e={InputIsKeyDown=function(n)return held.key_code[n]or false end,InputIsMouseButtonDown=function(n)return held.mouse_code[n]or false end,InputIsJoystickButtonDown=function(index,n)check(index==0,'controller0');return held.joystick_code[n]or false end,ModSettingGet=function(k)return settings[k]end,ModSettingSet=function(k,v)settings[k]=v end,ModSettingSetNextValue=function()end}
local c=control.new(e);held.key_code[20]=true;check(c:down(pref.defaults.NT_EMOTE_BINDING),'default Q');check(not c:down('key_code,20,mouse_code,2,joystick_code,23'),'entire chord required');held.mouse_code[2]=true;held.joystick_code[23]=true;check(c:down('key_code,20,mouse_code,2,joystick_code,23'),'mixed chord')
e.ModIsEnabled=function()return true end;local loads=0;e.dofile_once=function(path)loads=loads+1;check(path=='mods/mnee/lib.lua','real M-Nee library');e.is_binding_down=function(namespace,id)check(namespace=='nt2_g3_emotes'and id=='emote_menu','NT2-only M-Nee registration');return true end end
check(c:down('invalid'),'supported M-Nee owns binding');c:down('invalid');check(loads==1,'M-Nee loaded once');e.is_binding_down=nil;check(not c:down(pref.defaults.NT_EMOTE_BINDING),'unavailable M-Nee does not silently fall back');e.ModIsEnabled=function()return false end
held={key_code={},mouse_code={},joystick_code={}};local rebind=control.rebinder(e,{key_code={Q=20,ESC=41},mouse_code={R=2},joystick_code={A=23}});rebind:start();rebind:poll();held.key_code[20]=true;rebind:poll();held.mouse_code[2]=true;rebind:poll();held.key_code[20]=nil;held.mouse_code[2]=nil;local binding=rebind:poll();check(binding=='key_code,20,mouse_code,2,joystick_code','released chord commits once')
rebind:start();rebind:poll();held.key_code[41]=true;check(rebind:poll()==nil and not rebind.listening,'escape keeps prior binding');held.key_code[41]=nil
local flags={no_progress_flags_perk=true};e.GameHasFlagRun=function(k)return flags[k]or false end;e.GameAddFlagRun=function(k)flags[k]=true end;e.GameRemoveFlagRun=function(k)flags[k]=nil end;settings['noita_together2.NT_NO_STAT_PROGRESS']=true;pref.progress(e);settings['noita_together2.NT_NO_STAT_PROGRESS']=false;pref.progress(e);check(flags.no_progress_flags_perk and not flags.no_progress_flags_action,'progress restores only owned flags')
local function entity(id,name,parent)entities[id]={name=name,parent=parent or 0,alive=true,components={}}end
local function add(id,kind,v)serial=serial+1;v=v or {};components[serial]=v;v.owner=id;v.kind=kind;local a=entities[id].components[kind]or {};entities[id].components[kind]=a;a[#a+1]=serial;return serial end
local missing_component=nil
local function first(id,kind,...)
 if type(id)~='number'or id<=0 or type(kind)~='string'then return api_error('invalid entity/component lookup arguments')end
 if select('#',...)>1 or select('#',...)==1 and type((...))~='string'then return api_error('optional component tag must be omitted or a string')end
 local tag=(...);local en=entities[id];if en and en.alive then for _,v in ipairs(en.components[kind]or {})do if not tag or tag==''or (components[v]._tags or ''):find(tag,1,true)then return v end end end;return missing_component
end
local function body(id)entity(id,'player');add(id,'HotspotComponent',{_tags='cape_root',sprite_hotspot_name='cape'});add(id,'ControlsComponent',{enabled=true,mMouseDelta=0});add(id,'DamageModelComponent',{hp=4});add(id,'CharacterPlatformingComponent',{run_velocity=287,mouse_look=false});add(id,'SpriteComponent',{_tags='lukki_disable',visible=true,rect_animation='run',image_file='original-body'});for _,tag in ipairs({'player_hat2','player_amulet','player_amulet_gem'})do add(id,'SpriteComponent',{_tags=tag,visible=true,rect_animation='run'})end end
body(99);entity(101,'arm_r',99);add(101,'SpriteComponent',{_tags='with_item',visible=true,image_file='original-arm'});entity(102,'cape',99);add(102,'VerletPhysicsComponent',{cloth_color=123,cloth_color_edge=456});entity(103,'wand',99);add(103,'SpriteComponent',{visible=false});add(99,'Inventory2Component',{mActualActiveItem=103});body(399)
e.EntityGetIsAlive=function(id)return entities[id]and entities[id].alive or false end;e.EntityGetComponentIncludingDisabled=function(id,kind,...)if select('#',...)>0 and type((...))~='string'then return api_error('optional component-list tag must be omitted or a string')end;return entities[id]and entities[id].alive and entities[id].components[kind]or nil end;e.EntityGetFirstComponentIncludingDisabled=first
local function component(id,key)if type(id)~='number'or id<=0 or not components[id]or key~=nil and type(key)~='string'then return api_error('invalid or missing component id/field')end;return components[id]end
e.ComponentGetValue2=function(id,key)local value=component(id,key);if key=='mMouseDelta'then return value[key],0 end;return value[key]end
e.ComponentSetValue2=function(id,key,value)local item=component(id,key);assert(e.EntityGetIsAlive(item.owner),'stale mutation');check(key~='hp'and key~='enabled','no heal or input-lock release');item[key]=value end
e.ComponentGetIsEnabled=function(id)component(id);return true end;e.EntityAddComponent2=add;e.EntityRemoveComponent=function(id,c)check(components[c].owner==id,'component actual owner');for _,list in pairs(entities[id].components)do for i=#list,1,-1 do if list[i]==c then table.remove(list,i)end end end;components[c]=nil end
local refreshed={};e.EntityRefreshSprite=function(id,c)local v=component(c);check(v.owner==id and v.kind=='SpriteComponent','refresh uses actual owned sprite');refreshed[c]=v.image_file end
e.ComponentAddTag=function(c,tag)components[c]._tags=(components[c]._tags or '')..','..tag end;e.ComponentHasTag=function(c,tag)return components[c]and (','..(components[c]._tags or '')..','):find(','..tag..',',1,true)~=nil or false end;e.ComponentRemoveTag=function(c,tag)local out={};for t in (components[c]._tags or ''):gmatch('[^,]+')do if t~=tag then out[#out+1]=t end end;components[c]._tags=table.concat(out,',')end
e.EntityGetAllChildren=function(id)local a={};for child,en in pairs(entities)do if en.alive and en.parent==id then a[#a+1]=child end end;return #a>0 and a or nil end;e.EntityGetName=function(id)return entities[id].name end;e.EntityGetParent=function(id)return entities[id]and entities[id].parent or 0 end;e.EntityRemoveFromParent=function(id)entities[id].parent=0 end;e.EntityAddChild=function(id,child)if entities[child].name=='cape'then check(first(id,'HotspotComponent','cape_root')~=nil,'cape attaches to actual named sprite hotspot, never root/feet')end;entities[child].parent=id end
e.EntityGetTransform=function()return 20,30 end;e.EntityLoad=function(path)serial=serial+1;local id=serial;entity(id,path:find('cape.xml',1,true)and 'cape'or path);if path:find('holder',1,true)then add(id,'HotspotComponent',{_tags='cape_root',sprite_hotspot_name='cape'})end;if path:find('holder_local.xml',1,true)then entities[id].tags={nt2_a3_local_holder=true}end;loaded[#loaded+1]=path;if path:find('cape.xml',1,true)then add(id,'VerletPhysicsComponent',{cloth_color=123,cloth_color_edge=456})end;return id end;e.EntityKill=function(id)entities[id].alive=false end;e.EntityAddTag=function(id,tag)entities[id].tags=entities[id].tags or {};entities[id].tags[tag]=true end;e.EntityHasTag=function(id,tag)return entities[id]and entities[id].tags and entities[id].tags[tag]or false end;e.HasFlagPersistent=function()return true end
e.GameGetFrameNum=function()return frame end;e.GameGetRealWorldTimeSinceStarted=function()return now end;e.GamePlaySound=function(_,sound)sounds[#sounds+1]=sound end;e.GuiCreate=function()gui=gui+1;return gui end;e.GuiDestroy=function()end;e.GuiStartFrame=function()rendered={}end;e.GuiOptionsAdd=function()end;e.GuiGetScreenDimensions=function()return 640,360 end;e.GuiTooltip=function()end;e.GuiImageButton=function(_,_,x,y,_,path)rendered[#rendered+1]={x=x,y=y,path=path};if click and path:match('/([^/]+)%.png$')==click then click=nil;return true end;return false end;e.GlobalsGetValue=function(_,fallback)return fallback end;e.GlobalsSetValue=function()end;e.a3_ghosts=function()return {['aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa']=399}end
local visual=dofile(root..'social_visual.lua').new(e);local motion=first(99,'CharacterPlatformingComponent');local bs=first(99,'SpriteComponent','lukki_disable')
for _,id in ipairs(cat.ids)do check(visual:start(99,id,true,0),'local visual '..id);check(components[motion].run_velocity==0 and not components[bs].visible,'owned motion/body');visual:stop(99);check(components[motion].run_velocity==287 and components[motion].mouse_look==false and components[bs].visible and entities[102].parent==99,'restore actual modified stats, sprites and cape');check(components[first(103,'SpriteComponent')].visible==false,'hidden item remains hidden')end
check(not visual:start(99,'unknown',true),'unknown fails safe');components[first(99,'DamageModelComponent')].hp=-1;check(not visual:start(99,'wave',true),'dead cannot emote');components[first(99,'DamageModelComponent')].hp=4
visual:start(99,'wave',true);components[motion].run_velocity=321;components[first(99,'ControlsComponent')].enabled=false;visual:stop(99);check(components[motion].run_velocity==321 and not components[first(99,'ControlsComponent')].enabled,'foreign field/input changes preserved');components[motion].run_velocity=287;components[first(99,'ControlsComponent')].enabled=true
for _,robe in ipairs(cat.robes)do check(visual:colour(99,robe,{hat=true,amulet=true,gem=true},true,true),'robe '..robe)end;local count=#loaded;visual:colour(99,'purple',{hat=true,amulet=true,gem=true},true,true);check(#loaded==count,'duplicate colour has no particle replay');visual:clear_colour(99);check(components[bs].image_file=='original-body'and components[first(101,'SpriteComponent')].image_file=='original-arm','ordinary art restored')
local sent={};local r={model={composer=false,show_players=false},lock={close=function()end}};local sequence=0
local a=dofile(root..'social_local.lua').new(e,function(kind,value,id)sequence=sequence+1;id=id or string.format('%032x',sequence);sent[#sent+1]={kind=kind,value=value,id=id};return id end,r)
a:roster('run','me',{{actor='me',ghost='bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb',status=1},{actor='peer',ghost='aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',status=1}})
local function tick(n,playing)for _=1,n or 1 do frame=frame+1;now=now+1/60;a:tick(playing~=false,99,playing~=false)end end
tick();held.key_code[20]=true;tick(3);check(#rendered==26 and #sounds==1,'two centered rows plus six colours; sound once');check(rendered[1].x==220 and rendered[1].y==200 and rendered[11].x==220 and rendered[11].y==220,'legacy centered two-row layout')
click='heart';tick();check(a.local_emote and not a.local_emote.started and #sounds==2,'selection starts on following update and closes once');tick();check(a.local_emote.started,'actual local visual starts next update');held.key_code[20]=nil;tick(100);check(a.local_emote~=nil,'heart not stopped before exact boundary');tick(2);check(not a.local_emote,'heart completes at catalogue+2 boundary');check(a.pending_stop and a.pending_stop.emoteId=='heart','completion queues owned stop');tick();check(sent[#sent].kind=='emote'and sent[#sent].value.active==false,'completion sends/coalesces stop')
local event='11111111-1111-8111-8111-111111111111';check(a:emote({actor='peer',ghost='aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',revision=1,event=event,sequence=1,id='wave',active=true,duration=0,age_ms=31000},now),'retained loop accepted');tick(700);check(a.visual.active[399]~=nil,'loop lasts beyond10seconds without new originals');local holder=a.visual.active[399].holder
a:emote({actor='peer',ghost='aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',revision=1,event=event,sequence=1,id='wave',active=true,duration=0,age_ms=32000},now);tick();check(a.visual.active[399].holder==holder,'same accepted original never restarts')
a:emote({actor='peer',ghost='aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',revision=2,event='33333333-3333-8333-8333-333333333333',sequence=2,id='wave',active=false,duration=0,age_ms=0},now);tick();check(not a.visual.active[399],'new stop clears remote visual');check(not a:emote({actor='peer',ghost='aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',revision=1,event=event,sequence=1,id='wave',active=true,duration=0,age_ms=1},now),'old start cannot overwrite newer stop')
check(a:emote({actor='peer',ghost='aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',revision=3,event=event,sequence=3,id='heart',active=false,duration=101,age_ms=5000},now),'completed timed cold state accepted');tick();check(not a.visual.active[399],'completed cold timed effect never replays')
held.key_code[20]=true;click='wave';tick();tick();check(a.local_emote~=nil,'new intentional local selection');tick(1,false);check(not a.local_emote and components[motion].run_velocity==287,'protection cancels owned emote and restores real motion');a:close()
components[bs].visible=false;check(not visual:start(99,'wave',true),'hidden alternate body does not acquire a new animation');components[bs].visible=true
components[bs].alpha=0.25;visual:start(99,'heart',true);local effect=visual.active[99].effect;local sprite=visual.active[99].sprite;check(components[sprite].alpha==0.25,'emote keeps actual body transparency');entities[effect]={alive=true,parent=0,name='foreign recycled ID',components={}};visual:stop(99);check(entities[effect].alive,'recycled unparented effect ID is not killed');components[bs].alpha=1
visual:start(99,'wave',true);local oldmotion=motion;components[oldmotion]=nil;entities[99].components.CharacterPlatformingComponent={};motion=add(99,'CharacterPlatformingComponent',{run_velocity=412,mouse_look=true});visual:stop(99);check(components[motion].run_velocity==412,'replaced motion component never receives stale restoration')
visual:colour(399,'blue',{hat=true,amulet=false,gem=false},false,false);visual:start(399,'wave',false,0);local stable=visual.active[399].holder;check(visual.active[399].overlays.hat~=nil,'unlocked remote emote overlay');visual:colour(399,'red',{hat=false,amulet=true,gem=true},false,false);check(visual.active[399].holder==stable and visual.active[399].overlays.hat==nil and visual.active[399].overlays.amulet and visual.active[399].overlays.gem,'cosmetic changes update overlays without restarting');visual:clear()
visual:colour(399,'blue',{hat=true,amulet=true,gem=true},false,false);visual:start(399,'heart',false,0);local ghostbody=first(399,'SpriteComponent');components[ghostbody].alpha=0.25;visual:opacity(399,30);check(components[visual.active[399].sprite].alpha==0.25 and components[visual.active[399].overlays.hat].alpha==0.25,'runtime ghost opacity reaches emote and cosmetic overlays');components[ghostbody].alpha=0;visual:opacity(399,60);check(not visual.active[399].effect,'zero ghost opacity suppresses owned auxiliary effect');components[ghostbody].alpha=1;visual:opacity(399,80);check(visual.active[399].effect~=nil,'visible current effect restores using elapsed phase');visual:clear()
local cape_parent=entities[102].parent;visual:start(99,'wave',true);entities[102].parent=399;visual:stop(99);check(entities[102].parent==399,'foreign cape reparent is preserved');entities[102].parent=cape_parent
-- A fresh local instance exercises actual disable/re-enable and denial cleanup.
held.key_code[20]=nil;a=dofile(root..'social_local.lua').new(e,function(kind,value,id)sequence=sequence+1;id=id or string.format('%032x',sequence);sent[#sent+1]={kind=kind,value=value,id=id};return id end,r);a:roster('run','me',{{actor='me',ghost='bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb',status=1},{actor='peer',ghost='aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',status=1}});tick()
local event2='22222222-2222-8222-8222-222222222222';a:emote({actor='peer',ghost='aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',revision=1,event=event2,sequence=1,id='wave',active=true,duration=0,age_ms=0},now);a:cosmetic({actor='peer',ghost='aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',revision=1,event=event2,sequence=1,robe='blue',hat=false,amulet=true,gem=true},now);tick();check(a.visual.active[399]and a.visual.colours[399].robe=='blue','current remote state rendered');check(not a:cosmetic({actor='peer',ghost='aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',revision=2,event=event2,sequence=1,robe='red',hat=false,amulet=true,gem=true},now),'same accepted cosmetic original cannot substitute its robe at a higher revision');settings['noita_together2.NT_SHOW_EMOTES']=false;tick();check(not a.visual.active[399]and not a.visual.colours[399],'disable restores ordinary remote presentation');settings['noita_together2.NT_SHOW_EMOTES']=true;tick();check(a.visual.active[399]and a.visual.colours[399].robe=='blue','reenable reapplies current eligible state')
held.key_code[20]=true;click='wave';tick();tick();local firstidentity=a.local_emote.identity;held.key_code[20]=nil;tick();held.key_code[20]=true;click='wave';tick();tick();check(a.local_emote and a.local_emote.identity~=firstidentity,'intentional same-emote selection restarts while NT2 owns hidden body');local identity=a.local_emote.identity;a:outcome(identity,2,'Social selection rate limit','');check(not a.local_emote and a.pending_stop,'refused replacement still queues cleanup for last accepted remote emote');held.key_code[20]=nil;tick();check(sent[#sent].value.active==false,'denial cleanup is sent through same publisher')
held.key_code[20]=true;click='heart';tick();tick();held.key_code[20]=nil;tick(104);check(not a.local_emote,'timed local completes');settings['noita_together2.NT_SHOW_EMOTES']=false;tick();settings['noita_together2.NT_SHOW_EMOTES']=true;tick();check(not a.local_emote,'display toggle never resumes completed own animation');a:close()
-- Cosmetic outcomes are injected adapter facts here, never integration proof.
-- One unresolved automatic/manual publication keeps rollback on confirmed state.
do
 local oldget,oldset=e.GlobalsGetValue,e.GlobalsSetValue
 for _,statuses in ipairs({{2,2},{1,2},{2,1},{1,1}})do
  local saved='purple';e.GlobalsGetValue=function(_,fallback)return saved or fallback end;e.GlobalsSetValue=function(_,value)saved=value end
  local published={};local c=dofile(root..'social_local.lua').new(e,function(kind,value,id)sequence=sequence+1;id=id or string.format('%032x',sequence);if kind=='cosmetic'then published[#published+1]={id=id,value=value}end;return id end,r)
  local function step(icon)held.key_code[20]=icon and true or nil;click=icon;frame=frame+1;now=now+1/60;c:tick(true,99,true)end
  step();local initial=c.pending_cosmetic.identity;step('color_red');check(#published==1 and c.robe=='purple','automatic colour waits for its real result before a manual copy');c:outcome(initial,1,'','')
  step('color_red');local red=c.pending_cosmetic.identity;check(c.robe=='red'and saved=='purple','optimistic red is not persisted as accepted');step('color_blue');check(#published==2 and c.pending_cosmetic.identity==red,'second quick colour has clear busy feedback and cannot copy over unresolved red')
  c:outcome(red,0,'','');step('color_blue');check(#published==2 and c.pending_cosmetic.identity==red,'uncertain colour retains its exact original and prevents a competing action');c:outcome(string.rep('f',32),2,'foreign','');check(c.robe=='red','foreign/reordered outcomes cannot roll back current selection')
  c:outcome(red,statuses[1],'injected refusal','');local confirmed=statuses[1]==1 and 'red'or 'purple';check(c.robe==confirmed and saved==confirmed,'first colour settles on actual accepted baseline')
  step('color_blue');local blue=c.pending_cosmetic.identity;check(blue~=red and #published==3,'a later explicit colour is a new action after definitive outcome');c:outcome(red,2,'late old refusal','');check(c.robe=='blue','late old refusal cannot overwrite the new pending preview');c:outcome(blue,statuses[2],'injected refusal','');confirmed=statuses[2]==1 and 'blue'or confirmed;check(c.robe==confirmed and saved==confirmed,'two refused colours and accepted/refused combinations retain only confirmed colour')
  step();step();check(#published==3,'settled refusals do not auto-replay');step('color_green');local retired=c.pending_cosmetic.identity;c:close();c:outcome(retired,1,'','');check(c.robe==confirmed and not c.pending_cosmetic and saved==confirmed,'close/owner turnover discards unconfirmed preview and ignores late acceptance')
 end
 local saved='purple';e.GlobalsGetValue=function()return saved end;e.GlobalsSetValue=function(_,value)saved=value end;local count=0;local c=dofile(root..'social_local.lua').new(e,function()count=count+1;return string.format('%032x',count)end,r)
 held.key_code[20]=nil;frame=frame+1;now=now+1/60;c:tick(true,99,true);c:outcome(c.pending_cosmetic.identity,2,'automatic refused','');frame=frame+1;now=now+1/60;c:tick(true,99,true);check(count==1 and c.robe=='purple'and saved=='purple','refused initial/flag publication cannot silently accept or replay a new colour');c:close()
 e.GlobalsGetValue,e.GlobalsSetValue=oldget,oldset;held.key_code[20]=nil
end
-- Cold Noita-Lua state: discard every in-memory lease, retain serialized fields,
-- and remap component IDs as a loader can do. Only tagged original fields may
-- be restored; the persisted holder keeps the original cape in the save graph.
local visual_module=dofile(root..'social_visual.lua');local cold=visual_module.new(e);components[motion].run_velocity=412;components[motion].mouse_look=true
cold:colour(99,'red',{hat=true,amulet=true,gem=true},true,false);check(cold:start(99,'wave',true,0),'cold fixture begins real owned animation');local saved_holder=cold.active[99].holder;check(entities[102].parent==saved_holder,'original cape is retained beneath saved local holder');cold=nil
local remap,nextcomponents={},{};for id,v in pairs(components)do serial=serial+1;remap[id]=serial;nextcomponents[serial]=v end;for _,en in pairs(entities)do for _,list in pairs(en.components)do for i,cid in ipairs(list)do list[i]=assert(remap[cid])end end end;components=nextcomponents;motion=first(99,'CharacterPlatformingComponent');bs=first(99,'SpriteComponent','lukki_disable')
check(components[motion].run_velocity==0 and not components[bs].visible,'saved presentation values survive injected load');check(visual_module.recover(e,99),'cold marker recovery accepts remapped components');check(components[motion].run_velocity==412 and components[motion].mouse_look==true and components[bs].visible and components[bs].image_file=='original-body','cold restore keeps actual modified originals');check(entities[102].parent==99 and not entities[saved_holder].alive,'cold recovery restores owned cape and removes stale emote holder');check(components[first(103,'SpriteComponent')].visible==false,'cold restore preserves originally hidden held item');check(components[first(99,'ControlsComponent')].enabled,'presentation recovery never writes control authority')
check(visual_module.recover(e,99),'cold recovery idempotent');cold=visual_module.new(e);cold:start(99,'wave',true,0);components[motion].run_velocity=999;components[first(99,'ControlsComponent')].enabled=false;cold=nil;visual_module.recover(e,99);check(components[motion].run_velocity==999 and not components[first(99,'ControlsComponent')].enabled,'cold recovery preserves foreign stat/control changes');components[first(99,'ControlsComponent')].enabled=true
-- The real optional tag is omitted, not explicitly nil. Missing first-component
-- results include documented nil and defensive zero; neither is a component.
for _,zero in ipairs({false,true})do
 missing_component=zero and 0 or nil
 local sparse=visual_module.new(e);local id=zero and 899 or 799
 entity(id,'sparse-ghost');entity(id+1,'cape',id)
 check(sparse:colour(id,'blue',{hat=false,amulet=false,gem=false},false,false),'missing body/cape component colour path stays safe')
 local opacity_ok=pcall(sparse.opacity,sparse,id,120);check(opacity_ok,'missing body opacity must stay safe without issuing a component query');check(not sparse:start(id,'heart',false,120),'missing body cannot start a visual');sparse:clear()
 entity(id+2,'sparse-local');add(id+2,'SpriteComponent',{_tags='lukki_disable',visible=true,rect_animation='run',image_file='ordinary'});entity(id+3,'cape',id+2)
 check(sparse:colour(id+2,'red',{hat=false,amulet=false,gem=false},true,false),'local cape without Verlet component stays safe')
 sparse:opacity(id+2,0);check(not sparse:start(id+2,'wave',true,0),'missing controls/damage is unavailable, never component0');sparse:clear()
 body(id+4);check(sparse:start(id+4,'wave',true,0),'absent optional inventory/cape does not block a compatible local emote');sparse:stop(id+4)
 local load=e.EntityLoad;e.EntityLoad=function()return zero and 0 or nil end
 check(not sparse:start(id+4,'wave',true,0),'failed holder load cannot reach component APIs');e.EntityLoad=load;sparse:clear()
end
missing_component=nil
-- Remote geometry/lifecycle regression. Asset origins/alpha silhouettes are
-- checked independently against the actual archive by check-a3-ghost-geometry.
-- These are injected API facts, not proof of Noita cloth simulation.
do
 local v=visual_module.new(e);local id=1400;local transforms={[id]={213,457,0,-1,1}}
 local get_transform=e.EntityGetTransform;local set_transform=e.EntitySetTransform
 e.EntityGetTransform=function(target)local t=transforms[target]or {20,30,0,1,1};return unpack(t)end
 e.EntitySetTransform=function()error('presentation must not move or turn the actor')end
 local f=assert(io.open(repo..'/mod/noita_together2/files/g3/ghost.xml','r'));local xml=f:read('*a');f:close()
 local image=assert(xml:match('image_file="([^"]+)"'));local ox=tonumber(xml:match('offset_x="([^"]+)"'));local oy=tonumber(xml:match('offset_y="([^"]+)"'))
 local function ghost()
  entity(id,'remote');add(id,'HotspotComponent',{_tags='cape_root',sprite_hotspot_name='cape'})
  local b=add(id,'SpriteComponent',{image_file=image,offset_x=ox,offset_y=oy,alpha=0.35,visible=true,rect_animation='stand'});e.EntityRefreshSprite(id,b);return b
 end
 local b=ghost();local local_body=first(99,'SpriteComponent','lukki_disable');local original_local=components[local_body].image_file;local local_cape=entities[102].parent
 for _,facing in ipairs({-1,1})do
  transforms[id][4]=facing
  for _,robe in ipairs(cat.robes)do
   check(v:colour(id,robe,{hat=false,amulet=false,gem=false},false,false),'remote robe '..robe)
   check(components[b].offset_x==ox and components[b].offset_y==oy and components[b].alpha==0.35,'colour keeps sprite origin and ghost opacity')
   local cape=v.colours[id].cape;check(e.EntityGetIsAlive(cape)and entities[cape].parent==id,'ordinary owned cape has actor hotspot parent')
   check(refreshed[b]==components[b].image_file,'changed body image refreshed before attachment')
   for _,emote in ipairs(cat.ids)do
    check(v:start(id,emote,false,0),'remote emote starts for robe/facing')
    local active=v.active[id];check(entities[cape].parent==(cat.entries[emote].cape and active.holder or id),'cape uses current custom hotspot or ordinary actor')
    check(refreshed[active.sprite]==components[active.sprite].image_file,'new emote sprite is immediately refreshed')
    v:stop(id);check(entities[cape].parent==id and e.EntityGetIsAlive(cape),'emote stop returns live cape before deleting holder')
    check(components[b].visible and transforms[id][1]==213 and transforms[id][2]==457 and transforms[id][4]==facing,'rendering never changes actor coordinates/facing')
   end
  end
 end
 v:colour(id,'blue',{hat=false,amulet=false,gem=false},false,false);v:start(id,'wave',false,0)
 local holder=v.active[id].holder;v:colour(id,'red',{hat=false,amulet=false,gem=false},false,false)
 check(v.active[id].holder==holder and refreshed[v.active[id].sprite]==components[v.active[id].sprite].image_file,'colour during emote refreshes variant without restart')
 local dead=v.colours[id].cape;e.EntityKill(dead);v:colour(id,'red',{hat=false,amulet=false,gem=false},false,false)
 local replacement=v.colours[id].cape;check(replacement~=dead and entities[replacement].parent==holder and v.active[id].holder==holder,'identical colour recreates lost owned cape under active hotspot')
 v:stop(id);check(entities[replacement].parent==id,'recreated dance cape returns to actor')
 e.EntityKill(replacement);v:colour(id,'red',{hat=false,amulet=false,gem=false},false,false);local fresh=v.colours[id].cape
 check(fresh~=replacement and entities[fresh].parent==id,'identical ordinary colour recreates missing cape')
 entity(1499,'foreign-parent');add(1499,'HotspotComponent',{_tags='cape_root',sprite_hotspot_name='cape'});e.EntityRemoveFromParent(fresh);e.EntityAddChild(1499,fresh)
 v:colour(id,'red',{hat=false,amulet=false,gem=false},false,false);local ours=v.colours[id].cape
 check(ours~=fresh and e.EntityGetIsAlive(fresh)and entities[fresh].parent==1499,'foreign reparent is preserved while actor gets its own replacement')
 v:clear_colour(id);check(not e.EntityGetIsAlive(ours)and e.EntityGetIsAlive(fresh),'cleanup removes only owned attached cape')
 check(components[b].image_file==image and refreshed[b]==image and components[b].offset_x==ox and components[b].offset_y==oy,'clear restores ordinary purple art/cache/origin')
 e.EntityKill(id);b=ghost();v:colour(id,'green',{hat=false,amulet=false,gem=false},false,false);check(e.EntityGetIsAlive(v.colours[id].cape),'recreated ghost receives new owned cape');v:clear()
 check(components[local_body].image_file==original_local and entities[102].parent==local_cape,'remote correction leaves local art and cape unchanged')
 e.EntityGetTransform=get_transform;e.EntitySetTransform=set_transform
end
check(#api_errors==0,'all actual API calls have valid optional argument types and positive existing component IDs, including swallowed pcall failures')
print('PASS A3 '..checks..' focused production-adapter assertions; strict optional arguments/missing components; injected engine/input/native facts')
