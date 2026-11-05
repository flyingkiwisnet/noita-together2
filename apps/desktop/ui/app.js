function uxConnectionMessage(value){return ({'Service TLS/connection unavailable':'Cannot reach or verify the service connection. Check the server address and network, then retry. A version mismatch has not been confirmed.','Service Info unavailable':'Service information is unavailable. Check the connection and retry.','Unsupported candidate contract':'The service response could not be validated. Check the server address and matching NT2 release; see Compatibility details.','Service/contract mismatch':'App and service protocols do not match. Use the matching NT2 app/service release, then reconnect; see Compatibility details.'})[value]??value;}
function seedDescription(config){
    const testing=' \u00b7 Testing mode '+((config?.settings??[]).some(s=>s.key==='NT_testing_mode'&&s.value==='true')?'on':'off');
    const policy=config?.seedPolicy??'shared-fixed';
    if(policy==='individual-random')return 'Separate random worlds \u00b7 actual seeds shown \u00b7 Randomize loot off'+testing;
    const loot=config?.seedPolicy!==undefined&&config.randomizeLoot===true;
    return (policy==='shared-random'?'Shared random seed ':'Fixed shared seed ')+(config?.seed??0)+' \u00b7 Randomize loot '+(loot?'on':'off')+testing;
}
function recoveryDescription(r){
    if(r?.phase==='Ready'){
        if(r.shared_rules?.bootstrap===true)return 'Host Start confirmed - initializing shared player rules; waiting for game confirmation.';
        if(r.host_started===true)return 'Host Start confirmed · entering play.';
        if(r.awaiting_host_start===true)return 'Waiting for host Start · world protected.';
        return 'Checking seed and mods · world protected.';
    }
    if(r?.phase==='Playing')return 'Playing with current host permission.';
    if(r?.phase==='Offline')return 'This world is continuing offline.';
    if(r?.phase==='Ended')return r.terminal_reason==='player_death_end'?'This shared run ended after a player died. The host must prepare a fresh run; Join it before New Game. Continue cannot resume this run.':`${r.terminal_reason??'Shared participation ended'}. Shared play is held.`;
    if(r?.shared_rules?.bootstrap===true)return 'Host Start confirmed - initializing shared player rules; waiting for game confirmation.';
    if(r?.waiting_for_game)return r.host_started===true?'Host Start confirmed · waiting for game updates before resuming.':(r.app_connected?'App connected · ':'')+'Waiting for game updates; world protected.';
    if(r?.phase==='Checking')return 'Checking seed and mods · world protected.';
    if(r?.phase==='Reconnecting')return "Reconnecting to this world's run · world protected.";
    if(r?.phase==='Loading')return 'Loading this world · world protected.';
    if(r?.phase==='Choice')return 'Choose Rejoin for this saved world or continue offline.';
    if(r?.phase==='WaitingForNoita')return 'Waiting for Noita; Continue keeps its saved world.';
    if(r?.current_world&&r.held)return r.reason?`Waiting to revalidate this world: ${r.reason}. Shared play is held.`:'Reconnecting this saved world; shared play is held pending validation.';
    if(r?.intent_retained)return r.fresh_setup_active?'Previous saved-world recovery is remembered; setting up the selected lobby.':'Recovery requested for a previous saved world; no current protected world is confirmed.';
    if(r?.can_rejoin)return 'Known saved world: rejoin its previous run or leave the previous run.';
    return r?.terminal_reason??'';
}
function setupDescription(setup,rejoin){
    if(['Ready','Playing','Checking','Reconnecting','Loading'].includes(rejoin?.phase))return recoveryDescription(rejoin);
    const stages={initializing_shared_rules:'Host Start confirmed - initializing shared player rules; waiting for game confirmation.',continue_world:'Saved world selected · open Noita and choose Continue.',choose_lobby:'Select a lobby and choose Join.',preparing:'Joining and preparing your next New Game…',waiting_noita:'Waiting for Noita · run Noita and choose New Game. Continue keeps its saved seed.',loading_world:'Loading protected world…',checking_report:'Checking seed and mods · world protected.',waiting_game:'App connected · waiting for game updates; world protected.',checking_permission:'Checking the current run · world protected.',entering_play:'Host Start confirmed · entering play.',ready:'Checking the current run · world protected.',playing:'Playing with host authorization.'};
    return stages[setup?.stage]??setup?.stage??'';
}
function auth1Providers(v,selectedOriginMatches=true){
    const labels={twitch:'Twitch',google:'Google',local:'Local account'};
    if(!selectedOriginMatches)return Object.entries(labels).map(([id,label])=>({id,label,available:false,reason:'Check this service first'}));
    if(v.mock)return [{id:'website',label:'Website: development mock',available:true},{id:'twitch',label:'Twitch: development mock',available:true},...['google','local'].map(id=>({id,label:labels[id],available:false,reason:'Unavailable in this development mock'}))];
    const providers=v.authentication?.discovered?v.authentication.providers??[]:[];
    return Object.entries(labels).map(([id,label])=>{const p=providers.find(p=>p.id===id);return {id,label,available:p?.available===true,reason:p?.reason??(v.authentication?.options_error||'Check service sign-in options')};});
}
function auth1AccountDescription(v){
    if(!v.signed_in)return '';
    if(v.principal?.kind==='guest')return 'Guest session';
    const a=v.authentication?.account,e=a?.entitlements;
    if(!a||!e)return v.mock?'Development mock account':(v.authentication?.account_error?'Current account permissions unavailable. Refresh or sign in again.':'Checking current account permissions...');
    const role={'user':'User','power-user':'Power User','admin':'Admin','dev':'Dev'}[a.role]??'Unknown role';
    return `${a.displayName} · ${role} · ${a.provider} · Lobby capacity ${e.capacityLimit} · Custom lobby names ${e.customLobbyName?'available':'unavailable'} · Testing mode ${e.testingMode?'available':'unavailable'}${e.manageUsers?' · User administration on service website':''}`;
}
const el=id=>document.getElementById(id);
const invoke=window.__TAURI__.core.invoke;
let current=null,queue=Promise.resolve(),queued=0;
const selections=new Map();
let sessionScope=null,roomScope=null,draftVersion=0,gamePathDirty=false;
let candidateBootstrapped=false,serverDirty=false;
let loginBusy=false,loginMessage='',uiIssued=0,uiRendered=0;
const composing=new Set();
function text(node,value){
    value=String(value??'');if(node.textContent===value)return;
    const selection=window.getSelection?.();
    const anchor=selection?.anchorNode,focus=selection?.focusNode;
    const anchorLocal=anchor?.parentNode===node,focusLocal=focus?.parentNode===node;
    const start=selection?.anchorOffset,end=selection?.focusOffset;
    node.textContent=value;
    if((anchorLocal||focusLocal)&&node.firstChild)selection.setBaseAndExtent(anchorLocal?node.firstChild:anchor,anchorLocal?Math.min(start,value.length):start,focusLocal?node.firstChild:focus,focusLocal?Math.min(end,value.length):end);
}
// Retain nodes and their native focus/selection/details state across unchanged polls.
function reconcile(parent,items,key,create,update){
    const nodes=new Map([...parent.children].map(n=>[n.dataset.key,n]));
    let position=parent.firstElementChild;
    for(const [index,item] of items.entries()){
        const id=String(key(item,index));let node=nodes.get(id);
        if(!node){node=create(item);node.dataset.key=id;}
        nodes.delete(id);update(node,item);
        if(node!==position)parent.insertBefore(node,position);
        position=node.nextElementSibling;
    }
    for(const node of nodes.values())node.remove();
}
function scopeOf(v){return JSON.stringify([v.server_origin??v.website??'',v.service_id??'',v.service_epoch??'',v.desktop_id??null,v.session_epoch??'',v.account_id??null]);}
function playerRow(){
    const label=document.createElement('tr'),checkbox=document.createElement('input'),span=document.createElement('button');
    span.type='button';span.className='player-name';span.onclick=()=>{const member=current?.room?.members?.find(m=>m.desktopId===label.dataset.key);if(member)uxOpenPlayer(member,span);};
    checkbox.type='checkbox';checkbox.onchange=()=>{selections.set(label.dataset.key,checkbox.checked);if(current)render(current);};
    const details=document.createElement('details'),summary=document.createElement('summary'),seed=document.createElement('p'),mods=document.createElement('ol'),diagnostics=document.createElement('small');
    text(summary,'Game inspection');details.append(summary,seed,mods,diagnostics);
    const name=document.createElement('td');name.append(checkbox,span,details);label.append(name);for(let i=0;i<5;i++){const cell=document.createElement('td');if(i===4)cell.dataset.status='true';label.append(cell);}label.nt2Player={checkbox,span,details};return label;
}
function render(v){
  if(typeof betaRender==='function')betaRender(v);
  const issue=v.compatibility_issue,selected=v.room?v.server_origin:(el('server').value||v.server_origin),show=issue&&issue.origin===selected?.replace(/\/$/,'');el('compatibility-alert').hidden=!show;
  if(show){text(el('compatibility-message'),issue.message);text(el('compatibility-action'),issue.action);text(el('compatibility-details'),JSON.stringify({component:issue.component,code:issue.code,...issue.details},null,2));}

  window.NT2_RACE_UI?.render(v);
    let g3Panel=el('g3-originals');
    if(v.g3?.enabled){
        if(!g3Panel){
            g3Panel=document.createElement('section');g3Panel.id='g3-originals';
            const heading=document.createElement('h2');text(heading,'G3 transfer status');
            const status=document.createElement('p');status.dataset.role='status';
            const diagnostics=document.createElement('p');diagnostics.dataset.role='diagnostics';
            const records=document.createElement('div');records.dataset.role='records';
            const playerDetails=document.createElement('details'),playerSummary=document.createElement('summary'),playerCounters=document.createElement('textarea'),copyPlayer=document.createElement('button');
            text(playerSummary,'Player information diagnostics');playerCounters.dataset.role='player-information';playerCounters.readOnly=true;playerCounters.rows=8;playerCounters.maxLength=16384;playerCounters.setAttribute('aria-label','Player information diagnostic counters');
            text(copyPlayer,'Copy player information diagnostics');copyPlayer.type='button';copyPlayer.onclick=()=>{playerCounters.focus();playerCounters.select();document.execCommand('copy');};
            playerDetails.append(playerSummary,playerCounters,copyPlayer);
            const history=document.createElement('details'),historySummary=document.createElement('summary'),bankStatus=document.createElement('p');text(historySummary,'Transfer diagnostics and retained history');bankStatus.dataset.role='bank-status';history.append(historySummary,diagnostics,playerDetails,records);g3Panel.append(heading,status,bankStatus,history);document.body.append(g3Panel);
        }
        text(g3Panel.querySelector('[data-role="status"]'),v.g3.native_playing?'Gameplay connected':'Waiting for verified gameplay');
        const movement=v.g3.movement??{};
        const playerCounters=g3Panel.querySelector('[data-role="player-information"]');
        if(!playerCounters.matches(':focus'))playerCounters.value=JSON.stringify({player_information:v.g3.player_information??null,optional_stream:v.g3.optional_stream??null},null,2).slice(0,16384);
        const controlFailure=v.g3.control?.failure??{},movementFailure=v.g3.movement_failure??{},failureContext=v.g3.control?.failure_context??{};
        const failureSummary=f=>`${f.count??0} (${String(f.stage??'none').slice(0,32)} / ${String(f.category??'none').slice(0,32)})`;
        text(g3Panel.querySelector('[data-role="diagnostics"]'),`Movement app totals - accepted sends: ${movement.publications??0} | accepted replies: ${movement.receives??0} | calls attempted send/peers: ${movement.publication_attempts??0}/${movement.peer_attempts??0} | missed: ${movement.missed??0} | longest reply gap in app history: ${movement.max_receive_gap_ms??0} ms | validated phase: ${v.g3.control?.last_validated_phase??'none'} | local entry: ${Boolean(v.g3.control?.entered_locally)} | control failures: ${failureSummary(controlFailure)} | movement failures: ${failureSummary(movementFailure)} | first movement failure: ${failureSummary(v.g3.movement_first_failure??{})} | first lost movement guard: ${v.g3.movement_first_authority_loss??'none'} | last control failure phase: ${failureContext.phase??'none'} | entered: ${Boolean(failureContext.entered_locally)} | revisions local/service: ${failureContext.local_revision??0}/${failureContext.service_revision??0}`);
        text(g3Panel.querySelector('[data-role="bank-status"]'),[v.g3.bank?.status,v.g3.bank?.items?.status,v.g3.c1?.status].filter(Boolean).join(' · '));
        reconcile(g3Panel.querySelector('[data-role="records"]'),v.g3.operations??[],o=>o.operation_id,()=>{
            const row=document.createElement('p'),label=document.createElement('span'),button=document.createElement('button');
            text(button,'Check original status');button.type='button';row.append(label,button);return row;
        },(row,o)=>{
            text(row.firstChild,`${o.direction} ${o.asset==='item'?'item':o.amount+' gold'} · ${o.operation_id} · ${o.state}${o.server_status?' · service: '+o.server_status:''} · ${o.current_scope?'current world':'retained history'} · lobby ${o.room_id??'unknown'} · ${o.restored?'loaded-world outcome unverified':o.state==='applied_session'?'applied in this session; saved-world persistence unproven':'local outcome retained'} `);
            row.lastChild.disabled=!v.signed_in;
            row.lastChild.onclick=()=>call('g3_original_status',{operationId:o.operation_id});
        });
        g3Panel.hidden=false;
    }else if(g3Panel)g3Panel.hidden=true;
    const nextSession=scopeOf(v),nextRoom=JSON.stringify([nextSession,v.room?.roomId??null]);
    if(nextSession!==sessionScope||nextRoom!==roomScope){
        selections.clear();el('chat-text').value='';draftVersion++;el('password').value='';
        for(const node of [...el('chat-messages').children])node.remove();el('chat-messages').scrollTop=0;
        if(sessionScope!==null&&nextSession!==sessionScope)el('room').value='';
        sessionScope=nextSession;roomScope=nextRoom;
    }
    current=v;
    const candidate=v.protocol_revision==='g2-draft-5';
    if(candidate&&!candidateBootstrapped){
        if(!serverDirty&&!composing.has('server')&&document.activeElement!==el('server'))el('server').value=v.suggested_server??'';
        el('service').value='';candidateBootstrapped=true;
    }
    el('service-label').hidden=candidate;
    if(candidate)text(el('configure-game'),'Verify installation for this session');
    for(const id of ['candidate-signin','candidate-policy','load-game'])if(el(id))el(id).hidden=!candidate;
    for(const id of ['pairing-help','legacy-persona','approve'])if(el(id))el(id).hidden=candidate;
    if(el('guest-login'))el('guest-login').disabled=loginBusy||v.signed_in||v.auth_pending;
    if(el('provider'))el('provider').disabled=loginBusy||v.signed_in||v.auth_pending;
    if(el('cancel-signin')){el('cancel-signin').hidden=!candidate||!v.auth_pending;el('cancel-signin').disabled=loginBusy;}
    if(el('load-game'))el('load-game').disabled=!v.game_path||!meCandidate(v)||!['waiting','ended','staging'].includes(v.room?.state)||v.native_armed||(v.preparation&&v.preparation.state!=='issued');
    if(el('game-connection'))text(el('game-connection'),candidate?(v.installation_state??'')+' · '+(v.native_connection?.status??'No game contact yet')+(v.social?.requested?' · '+v.social.state:'')+(v.inspection_status?' · Inspection: '+v.inspection_status.stage+(v.inspection_status.current?' (current)':' (no current accepted game)')+' · local revision '+(v.inspection_status.local_observation_revision??'none')+' / service accepted '+v.inspection_status.service_observation_revision+(v.inspection_status.failure?' · '+v.inspection_status.failure:''):''): '');
    text(el('start'),candidate?'Host: Start inspected players':'Host: prepare selected players');
    if(el('debug-setting')){const d=v.debug_diagnostics;el('debug-setting').hidden=typeof d?.enabled!=='boolean';el('debug-diagnostics').checked=d?.enabled===true;text(el('debug-status'),d?.write_failed?'Diagnostic storage unavailable':typeof d?.enabled==='boolean'?(d.enabled?'Local debug recording enabled':'Local debug recording disabled'):'');}
    if(el('installation-diagnostics-panel')){el('installation-diagnostics-panel').hidden=!candidate||!v.installation_diagnostics;const fingerprints=(v.installation_diagnostics??'').slice(0,2048);if(el('installation-diagnostics').value!==fingerprints)el('installation-diagnostics').value=fingerprints;}
    if(!gamePathDirty&&!composing.has('game-path')&&document.activeElement!==el('game-path'))el('game-path').value=v.game_path??'';
    const me=v.room?.members?.find(m=>m.desktopId===v.desktop_id),host=Boolean(v.account_id&&v.principal?.kind!=='guest'&&v.room?.hostAccountId===v.account_id);
    text(el('session-state'),v.signed_in?(v.principal?.kind==='guest'?'Guest · ':'Signed in · ')+(v.principal?.displayName??v.account_id):candidate?(v.auth_state||'Choose account sign-in or guest'):v.activation_code?'Pairing code: '+v.activation_code+' · awaiting approval':'Not paired');
    text(el('attempt'),v.activation_attempt?'Attempt ID: '+v.activation_attempt:'');
    el('website').disabled=!v.website;el('approve').disabled=!v.activation_code||!v.mock;
    el('login').disabled=loginBusy||v.signed_in||Boolean(v.activation_code)||v.auth_pending;el('logout').disabled=!v.profile_selected;
    el('server').disabled=loginBusy||v.signed_in||Boolean(v.activation_code)||v.auth_pending;el('service').disabled=el('server').disabled;
    renderLogin(v);
    el('fixture').disabled=!v.signed_in||!v.mock||(candidate&&v.principal?.kind==='guest');el('join').disabled=!v.signed_in;el('refresh').disabled=!v.signed_in;
    el('first-rooms').disabled=!v.signed_in||!v.room_cursor;el('next-rooms').disabled=!v.signed_in||!v.next_room_id;
    text(el('connection'),v.status_error?'Connection: '+v.status_error:'');
    el('ready').disabled=!me||!me.present||!(candidate?['waiting','ended','staging']:['waiting','ended','suspended']).includes(v.room?.state);el('ready').checked=Boolean(me?.ready);
    el('start').disabled=!host||!(candidate?['staging']:['waiting','ended','suspended']).includes(v.room?.state);
    if(v.setup?.enabled){
        el('ready-label').hidden=true;el('load-game').hidden=true;el('leave-lobby').hidden=false;
        el('leave-lobby').disabled=!v.room&&!v.setup.intent_retained;
        text(el('start'),'Start everyone included');
        const included=(v.room?.members??[]).filter(m=>selections.get(m.desktopId)??m.included??true);
        el('start').disabled=!host||v.room?.state!=='staging'||!included.length||included.some(m=>!m.ready||m.readiness!=='ready'||m.consentedConfigRevision!==v.room.configRevision||!v.inspections?.some(r=>r.desktopId===m.desktopId));
        text(el('setup-stage'),v.setup.error?(v.setup.intent_retained?`Needs attention: ${v.setup.error}${v.setup.blocked?'':' · Setup will retry automatically.'}`:v.setup.error):setupDescription(v.setup,v.rejoin));
        el('game-path').hidden=!v.setup.installation_required;el('configure-game').hidden=!v.setup.installation_required;
        if(v.setup.installation_required)text(el('configure-game'),'Use and remember this Noita folder');
    }else{el('ready-label').hidden=false;el('leave-lobby').hidden=true;text(el('setup-stage'),'');el('game-path').hidden=false;el('configure-game').hidden=false;}
    text(el('rejoin-state'),recoveryDescription(v.rejoin));
    text(el('continue-offline'),v.rejoin?.current_world?'Continue this world offline':'Leave previous run');
    el('rejoin-previous').hidden=!v.rejoin?.can_rejoin;el('continue-offline').hidden=!v.rejoin?.can_offline;
    el('end').disabled=!host||!['preparing','loading','running','suspended','staging'].includes(v.room?.state);
    el('cancel').disabled=candidate?!v.auth_pending&&!v.preparation:!v.activation_code&&(!me&&!host||!['preparing','loading'].includes(v.room?.state));
    text(el('room-state'),v.room?'Room: '+v.room.state+' · '+v.room.name:'No room selected');
    text(el('configuration'),v.room?.config?seedDescription(v.room.config)+' · '+v.room.config.mode+' · configuration '+v.room.configRevision+' · '+JSON.stringify(v.room.config.settings??[]):'');
    reconcile(el('servers'),v.history??[],s=>s,()=>document.createElement('option'),(o,s)=>{if(o.value!==s)o.value=s;});
    const connection=v.connection;const connectionText=connection?`${connection.state}${connection.reason?`: ${connection.reason}`:''}${connection.protection_confirmed===true?' · Protection confirmed':connection.protection_confirmed===false?' · World reports unprotected':''}`:v.handoff;
    text(el('handoff'),connectionText+(v.seed===null?'':` · seed ${v.seed}`));text(el('limits'),v.limits+' · '+v.protocol_revision);
    const selectedRoom=el('room').value;
    const activeIds=new Set((v.room?.members??[]).map(m=>m.desktopId));for(const id of selections.keys())if(!activeIds.has(id))selections.delete(id);
    const rooms=new Map((v.rooms??[]).map(r=>[r.roomId,r]));if(v.room)rooms.set(v.room.roomId,v.room);
    // A discovery-page update must not silently choose a different room for the user.
    if(selectedRoom&&!rooms.has(selectedRoom)){
        const selectedOption=[...el('room').children].find(o=>o.value===selectedRoom);
        if(selectedOption)rooms.set(selectedRoom,{roomId:selectedRoom,retainedLabel:selectedOption.textContent});
    }
    reconcile(el('room'),[...rooms.values()],r=>r.roomId,()=>document.createElement('option'),(o,r)=>{if(o.value!==r.roomId)o.value=r.roomId;text(o,r.retainedLabel??(r.name+' · '+r.state+(r.passwordRequired?' · password':'')));});
    if(rooms.has(selectedRoom))el('room').value=selectedRoom;
    reconcile(el('players'),v.room?.members??[],m=>m.desktopId,playerRow,(label,m)=>{
        const {checkbox,span,details}=label.nt2Player;checkbox.disabled=!host;checkbox.checked=selections.get(m.desktopId)??m.included??true;
        const name=m.principal?.displayName??'Player';
        checkbox.ariaLabel='Select '+name+' to play';
        text(span,name);uxReadinessCells(v,m).forEach((model,i)=>uxStatusCell(label.children[i+1],model,'readiness-'+encodeURIComponent(m.desktopId)+'-'+i));
        const report=v.inspections?.find(r=>r.desktopId===m.desktopId);
        const expected=v.room?.config?.seed,actual=report?.actualSeed;
        const known=actual!==null&&actual!==undefined;
        const seedState=!known?'Unknown':v.room?.config?.seedPolicy==='individual-random'?'Independent world':String(actual)===String(expected)?'Correct':'Wrong';
        text(details.children[1],report?`Actual seed ${known?actual:'unknown'} · ${seedState} · ${report.loaded?'loaded':'loading'} · ${report.protected?'protected waiting':'not protected'} · ${report.inventoryComplete?'captured inventory':'inventory unavailable'} · ${report.compatibility??'compatibility unknown'}`:'Actual seed unknown · awaiting game inspection');
        reconcile(details.children[2],report?.activeMods??[],(_,i)=>i,()=>document.createElement('li'),(li,id)=>text(li,id));
        text(details.children[3],`Account ${m.accountId??''} · desktop ${m.desktopId}${report?' · observation '+report.observationRevision+' · captured '+report.capturedAt+' · beta '+(report.beta===true?'true':report.beta===false?'false':'unknown'):''}`);
    });
    if(typeof uxRenderChat==='function')uxRenderChat(v);
    if(typeof renderUx==='function')renderUx(v);
}
function call(command,input){
    if(command==='g1_status'&&queued)return;
    const priority=command==='g1_logout'||input?.action==='cancel'||input?.action==='preview_lobby';if(!priority&&queued>=16){el('error').textContent='Client command queue is full; try again';return Promise.resolve(false);}
    queued++;
    const run=async()=>{const serial=++uiIssued;try{const value=await invoke(command,input?{input}:{});if(serial>=uiRendered){uiRendered=serial;render(value);if(command!=='g1_status')el('error').textContent='';}return true;}catch(e){if(serial>=uiRendered){uiRendered=serial;el('error').textContent=String(e).includes('ACCOUNT_ALREADY_JOINED')?'This account already has a world in this lobby. Continue that world on its existing PC, or deliberately leave its entry before joining elsewhere.':String(e);}return false;}finally{queued--;}};
    if(priority)return run();queue=queue.then(run);return queue;
}
function action(action,extra={}){const scoped=['refresh_lobby','recovery_list','recovery_original','recovery_pending','review_recovery','configure_lobby','set_lobby_policy','set_lobby_access','moderate_player','player_details','room_bans','cancel_lobby','end_lobby','delete_lobby','retry_lobby_action','get_player_rules','set_player_rules','get_world_rules','set_world_rules'].includes(action);return call('g1_action',{action,...(scoped?{room:current?.room?.roomId}:{}),...extra});}
function meCandidate(v){return v.room?.members?.find(m=>m.desktopId===v.desktop_id);}
function loginInput(){return {server:el('server').value,service_id:current?.protocol_revision==='g2-draft-5'?'':el('service').value,remember:el('remember').checked};}
function renderLogin(v){
    text(el('login-feedback'),loginBusy?loginMessage:(uxConnectionMessage(v.login_error)||loginMessage||(v.auth_pending?v.auth_state:'')));
    const mismatch=v.identity_mismatch;
    const matches=mismatch&&el('server').value.trim().replace(/\/$/,'')===mismatch.origin;
    el('identity-recovery').hidden=!matches;
    if(matches)text(el('identity-description'),`At ${mismatch.origin}, saved service: ${mismatch.expected}. Discovered service: ${mismatch.discovered}.`);
    for(const id of ['trust-signin','trust-guest'])el(id).disabled=loginBusy||v.auth_pending||v.signed_in||!matches;
    const candidate=v.protocol_revision==='g2-draft-5';
    if(el('account-summary'))text(el('account-summary'),auth1AccountDescription(v));
    if(el('change-password')){el('change-password').hidden=!candidate||!v.authentication?.password_change_available;el('change-password').disabled=loginBusy||!v.signed_in;}
}
async function doLogin(extra={},recovery){
    if(loginBusy)return false;
    const input={...loginInput(),...(current?.protocol_revision==='g2-draft-5'?{provider:el('provider').value}:{}),...extra};
    if(recovery)input.recovery={...recovery};
    loginBusy=true;loginMessage=recovery?'Rechecking this service before fresh sign-in…':(extra.discovery_only?'Checking service sign-in options…':'Checking service and signing in…');
    if(current)render(current);
    try{
        const ok=await call('g1_login',input);
        if(!ok){loginMessage=el('error').textContent;try{await call('g1_status');}catch{}}
        else loginMessage='';
        return ok;
    }finally{loginBusy=false;if(current)render(current);}
}
el('login').onclick=()=>doLogin();
if(el('cancel-signin'))el('cancel-signin').onclick=()=>action('cancel_sign_in');
if(el('provider'))el('provider').onchange=()=>{if(current)renderLogin(current);};
if(el('change-password'))el('change-password').onclick=()=>action('open_password_change');
if(el('guest-login'))el('guest-login').onclick=()=>doLogin({guest_name:el('guest-name').value});
el('trust-signin').onclick=()=>doLogin({},current?.identity_mismatch);
el('trust-guest').onclick=()=>doLogin({guest_name:el('guest-name').value},current?.identity_mismatch);
if(el('load-game'))el('load-game').onclick=()=>action('load_game');
el('website').onclick=()=>action('open_website');el('approve').onclick=()=>action('approve_mock',{subject:el('persona').value});el('fixture').onclick=()=>action('create_fixture',current?.protocol_revision==='g2-draft-5'?{allow_guests:el('allow-guests').checked}:{});
el('join').onclick=async()=>{const room=el('invite').value.trim()||el('room').value,password=el('password').value;
    const ok=await action('join',{room,password:password||undefined});if(ok){if(el('password').value===password)el('password').value='';}else{try{await call('g1_status');}catch{}}return ok;};
el('room').onchange=()=>{};
el('invite').onchange=()=>{};
el('leave-lobby').onclick=()=>action('leave_lobby');
el('first-rooms').onclick=()=>action('first_rooms');el('next-rooms').onclick=()=>action('next_rooms');
el('server').oninput=()=>{serverDirty=true;loginMessage='';if(current)renderLogin(current);if(current?.protocol_revision==='g2-draft-5')return;const match=current?.recent_servers?.find(s=>s.origin===el('server').value);if(match)el('service').value=match.service_id;};
el('ready').onchange=()=>action('ready',{ready:el('ready').checked});
el('chat-send').onclick=async()=>{if(composing.has('chat-text'))return;const draft=el('chat-text').value,version=draftVersion,scope=roomScope;if(await action('chat_send',{text:draft})){if(scope===roomScope&&version===draftVersion&&el('chat-text').value===draft&&!composing.has('chat-text')){el('chat-text').value='';draftVersion++;}}};
el('start').onclick=()=>{const selected=[],excluded=[];for(const m of current?.room?.members??[])((selections.get(m.desktopId)??m.included??true)?selected:excluded).push(m.desktopId);
    if(current?.setup?.enabled){const participants=selected.map(id=>{const r=current.inspections?.find(r=>r.desktopId===id);return {desktopId:id,attachmentId:r?.attachmentId,worldToken:r?.worldToken,observationRevision:r?.observationRevision,contentDigest:r?.contentDigest};});const start_capture=Object.fromEntries(['server_origin','service_id','service_epoch','session_epoch','desktop_id','account_id','room','inspections'].map(k=>[k,current[k]]));return action('start',{participants,excluded,config_revision:current.room.configRevision,start_capture});}
    else return action('start',{selected,excluded});};
el('cancel').onclick=()=>action('cancel');el('end').onclick=()=>action('end');el('refresh').onclick=()=>action('refresh');el('logout').onclick=()=>call('g1_logout');
setInterval(()=>call('g1_status'),1000);
el('configure-game').onclick=async()=>{const path=el('game-path').value;if(await action('configure_game',{game_path:path})){if(el('game-path').value===path)gamePathDirty=false;}};
el('game-path').oninput=()=>{gamePathDirty=true;};
el('chat-text').oninput=()=>{draftVersion++;};
for(const id of ['game-path','chat-text','invite','password','server','service']){
    el(id).addEventListener('compositionstart',()=>composing.add(id));
    el(id).addEventListener('compositionend',()=>{composing.delete(id);if(id==='chat-text')draftVersion++;});
}
if(window.__NT2_CANDIDATE__){
    el('server').value='';el('service').value='';el('service-label').hidden=true;
    text(el('configure-game'),'Verify installation for this session');call('g1_status');
}

if(el('copy-installation-diagnostics'))el('copy-installation-diagnostics').onclick=()=>{
    const box=el('installation-diagnostics');box.focus();box.select();
    const copied=document.execCommand('copy');
    text(el('copy-diagnostics-status'),copied?'Diagnostic fingerprints copied':'Select and copy the diagnostic text');
};

el('rejoin-previous').onclick=()=>action('rejoin_previous');
el('continue-offline').onclick=()=>action('continue_offline');

if(el('debug-diagnostics'))el('debug-diagnostics').onchange=()=>action('set_debug_diagnostics',{debug_diagnostics:el('debug-diagnostics').checked});

if(typeof initUx==='function')initUx();
