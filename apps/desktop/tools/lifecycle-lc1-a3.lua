-- Noita API/input observations only. Real production adapters, native mailbox,
-- scheduler and authenticated service decide all social/authority outcomes.
local M={}
function M.new(e,state,forms,ghosts,input,mark)
 local api={};for _,k in ipairs({'EntityGetIsAlive','EntityGetFirstComponentIncludingDisabled','EntityGetComponentIncludingDisabled','EntityGetAllChildren','EntityGetParent','EntityGetRootEntity','EntityHasTag','EntityAddTag','EntityRemoveTag','EntityAddComponent2','EntityRemoveComponent','EntityLoad','EntityKill','EntityGetName','EntityGetTransform','EntitySetTransform','EntitySetComponentIsEnabled','ComponentGetIsEnabled','ComponentGetValue2','ComponentSetValue2','GuiImageButton'})do api[k]=e[k]end
 local entities,owned,subjects,tags={}, {}, {}, {};local serial=600000;local sounds,particles,loads=0,0,0;local last_original,last_cosmetic='',''
 local json;local last_ghosts,peer_observation={},{};local first_ghost_loss=false
 local emote_history,history_seen={},{}
 local copy_evidence={leaseCalls=0,leaseReady=0,publish={},trace={}}
 local function current()return forms and forms:current()or 99 end
 local function alive(id)if entities[id]then return entities[id].alive end;return api.EntityGetIsAlive(id)end
 local function add(id,kind,value)
  assert(alive(id),'A3 injected component requires live subject');serial=serial+1;state.components[serial]=value;owned[serial]={entity=id,kind=kind};return serial
 end
 local function visual(name,parent)
  serial=serial+1;local id=serial;entities[id]={name=name,parent=parent or 0,alive=true,tags={},x=0,y=0};return id
 end
 local function ensure(id)
  if subjects[id]or not alive(id)or (id~=current()and not ghosts[id])then return end;subjects[id]=true
  add(id,'SpriteComponent',{_tags=id==current()and 'lukki_disable'or 'nt2_ghost',visible=true,alpha=1,image_file='injected-original-body',rect_animation='run'})
  if id==current()then add(id,'CharacterPlatformingComponent',{run_velocity=287,mouse_look=false});state.components[6].mActualActiveItem=202 end
  local arm=visual('arm_r',id);add(arm,'SpriteComponent',{_tags='with_item',visible=true,image_file='injected-original-arm'})
  local cape=visual('cape',id);add(cape,'VerletPhysicsComponent',{cloth_color=123,cloth_color_edge=456})
  if id==current()then for _,tag in ipairs({'player_hat2','player_amulet','player_amulet_gem'})do add(id,'SpriteComponent',{_tags=tag,visible=true,alpha=1,rect_animation='stand'})end end
 end
 local function list(id,kind,tag)
  ensure(id);local out={};for c,owner in pairs(owned)do if owner.entity==id and owner.kind==kind and state.components[c]and (not tag or (state.components[c]._tags or ''):find(tag,1,true))then out[#out+1]=c end end;table.sort(out);return out
 end
 e.EntityGetIsAlive=alive
 e.EntityGetFirstComponentIncludingDisabled=function(id,kind,tag)local values=list(id,kind,tag);if #values>0 then return values[1]end;if entities[id]then return nil end;return api.EntityGetFirstComponentIncludingDisabled(id,kind,tag)end
 e.EntityGetComponentIncludingDisabled=function(id,kind,tag)
  local result=entities[id]and {}or api.EntityGetComponentIncludingDisabled(id,kind,tag)or {};local out={};for _,v in ipairs(result)do out[#out+1]=v end;for _,v in ipairs(list(id,kind,tag))do out[#out+1]=v end;return out
 end
 e.EntityAddComponent2=function(id,kind,v)if entities[id]or kind=='SpriteComponent'or kind=='VariableStorageComponent'and type(v.name)=='string'and v.name:find('^nt2_a3_restore_')then return add(id,kind,v)end;return api.EntityAddComponent2(id,kind,v)end
 e.ComponentAddTag=function(c,tag)assert(state.components[c]);local v=state.components[c];v._tags=(v._tags and v._tags..','or '')..tag end
 e.ComponentRemoveTag=function(c,tag)assert(state.components[c]);local out={};for t in (state.components[c]._tags or ''):gmatch('[^,]+')do if t~=tag then out[#out+1]=t end end;state.components[c]._tags=table.concat(out,',')end
 e.ComponentHasTag=function(c,tag)return state.components[c]and (','..(state.components[c]._tags or '')..','):find(','..tag..',',1,true)~=nil or false end
 e.EntityRemoveComponent=function(id,c)if owned[c]then assert(owned[c].entity==id,'A3 stale component owner');state.components[c]=nil;owned[c]=nil else api.EntityRemoveComponent(id,c)end end
 e.ComponentGetValue2=function(c,k)if owned[c]then assert(state.components[c],'A3 stale read');return state.components[c][k]end;return api.ComponentGetValue2(c,k)end
 e.ComponentSetValue2=function(c,k,v)if owned[c]then assert(alive(owned[c].entity)and state.components[c],'A3 stale visual mutation');state.components[c][k]=v else api.ComponentSetValue2(c,k,v)end end
 e.ComponentGetIsEnabled=function(c)if owned[c]then return state.enabled[c]~=false end;return api.ComponentGetIsEnabled(c)end
 e.EntitySetComponentIsEnabled=function(id,c,v)if owned[c]then assert(owned[c].entity==id);state.enabled[c]=v else api.EntitySetComponentIsEnabled(id,c,v)end end
 e.EntityGetAllChildren=function(id)
  ensure(id);local out={};if not entities[id]then for _,v in ipairs(api.EntityGetAllChildren(id)or {})do out[#out+1]=v end end
  for v,en in pairs(entities)do if en.alive and en.parent==id then out[#out+1]=v end end;table.sort(out);return out
 end
 e.EntityGetName=function(id)return entities[id]and entities[id].name or api.EntityGetName(id)end
 e.EntityGetParent=function(id)return entities[id]and entities[id].parent or api.EntityGetParent(id)end
 e.EntityGetRootEntity=function(id)if entities[id]then local p=id;while entities[p]and entities[p].parent~=0 do p=entities[p].parent end;return p end;return api.EntityGetRootEntity(id)end
 e.EntityAddChild=function(parent,id)assert(entities[id]and alive(parent));entities[id].parent=parent end
 e.EntityRemoveFromParent=function(id)assert(entities[id]);entities[id].parent=0 end
 e.EntityHasTag=function(id,tag)if entities[id]then return entities[id].tags[tag]or false end;if tag:find('^nt2_a3_')then return tags[id]and tags[id][tag]or false end;return api.EntityHasTag(id,tag)end
 e.EntityAddTag=function(id,tag)if entities[id]then entities[id].tags[tag]=true elseif tag:find('^nt2_a3_')then tags[id]=tags[id]or {};tags[id][tag]=true else api.EntityAddTag(id,tag)end end
 e.EntityRemoveTag=function(id,tag)if entities[id]then entities[id].tags[tag]=nil elseif tag:find('^nt2_a3_')then if tags[id]then tags[id][tag]=nil end else api.EntityRemoveTag(id,tag)end end
 e.EntityLoad=function(path,x,y)
  if not path:find('mods/noita_together2/files/g3/emotes/',1,true)and path~='data/entities/verlet_chains/cape/cape.xml'then return api.EntityLoad(path,x,y)end
  loads=loads+1;local id=visual(path:find('cape.xml',1,true)and 'cape'or path);entities[id].x=x;entities[id].y=y
  if path:find('/holder_local.xml',1,true)then entities[id].tags.nt2_a3_local_holder=true end
  if path:find('color_swap_particles',1,true)then particles=particles+1 end
  if path:find('cape.xml',1,true)then add(id,'VerletPhysicsComponent',{cloth_color=123,cloth_color_edge=456})end;return id
 end
 e.EntityKill=function(id)
  if ghosts[id]and ghosts[id].ghost and not first_ghost_loss and input('a3-watch-retained',false)then
   first_ghost_loss=true
   mark('a3-first-ghost-loss.json',json({engineFactsInjected=true,serviceMocked=false,frame=state.frame,entity=id,controls=state.components[1].enabled,nativePeers=peer_observation,previousPresentation=last_ghosts[id],caller=debug.traceback('',2):sub(1,2048)}))
  end
  if entities[id]then entities[id].alive=false;for child,en in pairs(entities)do if en.parent==id and en.alive then e.EntityKill(child)end end else api.EntityKill(id)end
 end
 e.EntityGetTransform=function(id)if entities[id]then return entities[id].x,entities[id].y,0 end;return api.EntityGetTransform(id)end
 e.EntitySetTransform=function(id,x,y)if entities[id]then entities[id].x=x;entities[id].y=y else api.EntitySetTransform(id,x,y)end end
 e.ModSettingGet=function(key)local value=input('a3-setting-'..key:gsub('^noita_together2%.',''),false);if value=='true'then return true elseif value=='false'then return false end;return value and (tonumber(value)or value)or nil end
 e.ModSettingSet=function(key,value)mark('a3-setting-'..key:gsub('^noita_together2%.',''),tostring(value))end;e.ModSettingSetNextValue=function()end
 e.InputIsKeyDown=function(code)return code==20 and input('a3-menu',false)~=nil end;e.InputIsMouseButtonDown=function()return false end;e.InputIsJoystickButtonDown=function()return false end
 e.HasFlagPersistent=function(flag)return (','..(input('a3-flags',false)or '')..','):find(','..flag..',',1,true)~=nil end
 e.GamePlaySound=function()sounds=sounds+1 end
 -- The ordinary fixture supplies the actual GUI call surface used by both
 -- presenters. These are injected viewport/options facts, not selection results.
 e.GUI_OPTION={NoPositionTween=6,Layout_NextSameLine=1};e.GuiGetScreenDimensions=function()return 640,360 end
 e.GuiOptionsAdd=function(_,option)assert(option==6,'fixed immediate-position GUI option')end;e.GuiTooltip=e.GuiTooltip or function()end
 e.GuiImageButton=function(gui,id,x,y,text,path)
  local token=path:match('/([^/]+)%.png$');if token and input('a3-select',false)==token then input('a3-select',true);copy_evidence.lastSelection={frame=state.frame,token=token,lease=copy_evidence.lastLease,copied=false,completed=false};mark('a3-selected',token);return true end;return api.GuiImageButton(gui,id,x,y,text,path)
 end
 json=function(v)
  if type(v)=='table'then local keys,out={},{};for k in pairs(v)do keys[#keys+1]=k end;table.sort(keys);for _,k in ipairs(keys)do out[#out+1]=json(tostring(k))..':'..json(v[k])end;return '{'..table.concat(out,',')..'}'
  elseif type(v)=='string'then return '"'..v:gsub('[%z\1-\31\\"]',function(c)return string.format('\\u%04x',c:byte())end)..'"'
  elseif type(v)=='boolean'or type(v)=='number'then return tostring(v)end;return 'null'
 end
 local self={}
 function self:observe_social_lease(result,header)
  copy_evidence.leaseCalls=math.min(2147483647,copy_evidence.leaseCalls+1)
  local value={result=tonumber(result),frame=state.frame}
  if result==64 then value.remainingMs=tonumber(header[28])+tonumber(header[29])*256+tonumber(header[30])*65536+tonumber(header[31])*16777216;copy_evidence.leaseReady=math.min(2147483647,copy_evidence.leaseReady+1)end
  copy_evidence.lastLease=value
 end
 function self:observe_social_copy(result,body)
  local kind=body:match('"kind":"([a-z]+)"');if kind~='hud'and kind~='emote'and kind~='cosmetic'and kind~='poi'and kind~='dismiss'then kind='other'end
  local counts=copy_evidence.publish[kind]or {accepted=0,refused=0};copy_evidence.publish[kind]=counts;local key=result==1 and 'accepted'or 'refused';counts[key]=math.min(2147483647,counts[key]+1)
  local id=body:match('"id":"([0-9a-f]+)"');if not id or #id~=32 then id=nil end
  local selection=copy_evidence.lastSelection
  if selection and selection.frame==state.frame and ((kind=='emote'and body:find('"active":true',1,true)and body:find('"emoteId":"'..selection.token..'"',1,true))or(kind=='cosmetic'and selection.token=='color_'..(body:match('"robeId":"([a-z]+)"')or '')))then
   selection.copyResult=tonumber(result);if result==1 then selection.copied=true;selection.copiedIdentity=id end
  end
  if kind=='poi'and result==1 and id then copy_evidence.lastPoiCopied={id=id,frame=state.frame}end
  local trace=copy_evidence.trace;if #trace>=16 then table.remove(trace,1)end
  trace[#trace+1]={frame=state.frame,subject=current(),kind=kind,result=tonumber(result),id=id,active=kind=='emote'and body:match('"active":(%a+)')or nil,lease=copy_evidence.lastLease}
 end
 function self:observe_poi_submit(model,result)
  copy_evidence.poiSubmits=math.min(2147483647,(copy_evidence.poiSubmits or 0)+1)
  copy_evidence.lastPoiSubmit={count=copy_evidence.poiSubmits,frame=state.frame,copied=result==true,pending=model.pending,feedback=(model.feedback or ''):sub(1,396),draftBytes=#(model.draft or '')}
 end
 function self:observe_peers(result,reply)
  local previous=peer_observation;peer_observation={frame=state.frame,result=tonumber(result)}
  if result==864 then
   peer_observation.leaseMs=tonumber(reply[24])+tonumber(reply[25])*256+tonumber(reply[26])*65536+tonumber(reply[27])*16777216
   peer_observation.rosterCount=tonumber(reply[28]);peer_observation.poseCount=tonumber(reply[6]);peer_observation.version=tonumber(reply[4]);peer_observation.allowed=tonumber(reply[5])
  elseif previous.result==864 then peer_observation.lastAccepted=previous
  else peer_observation.lastAccepted=previous.lastAccepted end
 end
 function self:snapshot(a)
  if copy_evidence.lastSelection then copy_evidence.lastSelection.completed=true end
  ensure(current());local active={};if a then for id,v in pairs(a.visual.active)do active[tostring(id)]={id=v.id,holder=v.holder}end end
  last_ghosts={};local peers={}
  if a then for actor,p in pairs(a.peers)do
   local rendered=a.rendered[actor];local en=rendered and rendered.entity;local v=en and a.visual.active[en];local c=en and a.visual.colours[en];local canonical=a.states[actor]or {};local emote=canonical.emote;local cosmetic=canonical.cosmetic
   local age=emote and math.floor((emote.age_ms+math.max(0,e.GameGetRealWorldTimeSinceStarted()-emote.at)*1000)*60/1000)or 0
   peers[actor]={status=p.status,ghost=p.ghost,entity=en,emote=v and v.id or '',holder=v and v.holder or 0,event=emote and emote.event or '',revision=emote and emote.revision or 0,canonicalEmote=emote and emote.id or '',canonicalActive=emote and emote.active or false,durationFrames=emote and emote.duration or 0,elapsedFrames=age,robe=c and c.robe or '',canonicalRobe=cosmetic and cosmetic.robe or '',canonicalCosmeticRevision=cosmetic and cosmetic.revision or 0,hat=c and c.hat or false,amulet=c and c.amulet or false,gem=c and c.gem or false}
   if en then last_ghosts[en]={actor=actor,robe=peers[actor].robe,emote=peers[actor].emote,holder=peers[actor].holder,status=p.status,ghost=p.ghost}end
   if emote then
    local signature=emote.event..':'..emote.revision..':'..tostring(emote.active)..':'..peers[actor].emote
    if history_seen[actor]~=signature then
     history_seen[actor]=signature;if #emote_history>=16 then table.remove(emote_history,1)end
     emote_history[#emote_history+1]={actor=actor,event=emote.event,id=emote.id,duration=emote.duration,elapsed=age,active=emote.active,rendered=peers[actor].emote,frame=state.frame,entity=en or 0,status=p.status}
    end
   end
  end end
  for actor in pairs(history_seen)do if not peers[actor]then history_seen[actor]=nil end end
  local motion=list(current(),'CharacterPlatformingComponent')[1];local b=list(current(),'SpriteComponent','lukki_disable')[1];local current_emote=a and a.local_emote
  if current_emote then last_original=current_emote.identity end
  if a and a.pending_cosmetic and a.pending_cosmetic.manual then last_cosmetic=a.pending_cosmetic.identity end
  local live_ghosts={};for id,g in pairs(ghosts)do if g.ghost then live_ghosts[tostring(id)]={x=g.x,y=g.y,alive=alive(id)}end end
  mark('a3-view.json',json({enabled=a and a.enabled or false,liveGhosts=live_ghosts,engineFactsInjected=true,serviceMocked=false,frame=state.frame,subject=current(),controls=state.components[1].enabled,motion=motion and state.components[motion].run_velocity or -1,bodyVisible=b and state.components[b].visible or false,localEmote=current_emote and current_emote.id or '',localOriginal=current_emote and current_emote.identity or '',lastOriginal=last_original,lastCosmeticOriginal=last_cosmetic,localEvent=current_emote and current_emote.event or '',pendingStop=a and a.pending_stop~=nil or false,robe=a and a.robe or '',menu=a and a.menu.open or false,menuLocked=a and a.menu.locked or false,copyReady=a and a.social_ready or false,menuReason=a and a.menu.last_reason or '',sounds=sounds,particles=particles,loads=loads,active=active,peers=peers,emoteHistory=emote_history,copyEvidence=copy_evidence}))
 end
 return self
end
return M
