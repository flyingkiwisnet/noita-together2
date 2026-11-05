window.addEventListener('DOMContentLoaded',()=>setTimeout(async()=>{
 const checks=[];const check=(name,ok)=>{if(!ok)throw Error(name);checks.push(name);};
 try{
  const ipc=window.__TAURI__.core.invoke;
  render(await ipc('g1_status'));check('candidate controls visible and copied-code help hidden',!el('candidate-signin').hidden&&el('pairing-help').hidden);
  const restarted=current.recent_servers.length>0;
  check('fresh candidate or persisted restart has correct address choice',el('server').value===(restarted?'http://127.0.0.1:4355':''));
  check('internal service identity is not routine input',el('service').closest('label').hidden&&el('service').value==='');
  check('installation verification label is truthful',el('configure-game').textContent==='Verify installation for this session');
  check('unverified installation blocks Load game',!current.game_path&&el('load-game').disabled);
  check('installation and native contact diagnostics are visible',el('game-connection').textContent.includes(current.installation_state)&&el('game-connection').textContent.includes(current.native_connection.status));
  const beforeDiagnostics=current;
  const fingerprintView={...current,installation_diagnostics:'NT2 candidate installation fingerprints\nnoita.exe: '+('a'.repeat(64))+'\n'};
  render(fingerprintView);const fingerprintBox=el('installation-diagnostics');fingerprintBox.focus();fingerprintBox.setSelectionRange(5,20);render(fingerprintView);
  check('diagnostic text and selection survive status rendering',!el('installation-diagnostics-panel').hidden&&fingerprintBox.selectionStart===5&&fingerprintBox.selectionEnd===20);
  el('copy-installation-diagnostics').onclick();check('diagnostics copy offers visible feedback',fingerprintBox.selectionStart===0&&fingerprintBox.selectionEnd===fingerprintBox.value.length&&el('copy-diagnostics-status').textContent.length>0);
  render({...fingerprintView,room:{members:[{desktopId:'beta-desktop',accountId:'beta-account'}]},inspections:[{desktopId:'beta-desktop',actualSeed:'424242',activeMods:['noita_together2'],beta:true}]});
  check('actual beta report stays visible',el('players').textContent.includes('beta true'));
  render(beforeDiagnostics);
  el('server').value='http://127.0.0.1:4355';el('server').dispatchEvent(new Event('input'));el('server').focus();el('server').setSelectionRange(7,12);render(await ipc('g1_status'));
  check('connection polling preserves address focus and draft',document.activeElement===el('server')&&el('server').selectionStart===7&&el('server').value==='http://127.0.0.1:4355');
  el('guest-name').value='WebView guest';
  await el('guest-login').onclick();check('actual IPC and Core guest session visibly rendered',current.signed_in&&current.principal.kind==='guest'&&el('session-state').textContent==='Guest · WebView guest');
  check('discovered identity and recent selection are pinned',current.service_id==='nt2-client-browser-candidate'&&current.recent_servers[0].origin==='http://127.0.0.1:4355'&&el('servers').children[0].value==='http://127.0.0.1:4355');
  check('guest hosting unavailable',el('fixture').disabled&&el('start').disabled);
  const scope=current.desktop_id;el('invite').value='unfinished room choice';await el('invite').dispatchEvent(new Event('compositionstart'));render(await ipc('g1_status'));check('poll retains unfinished input and session',el('invite').value==='unfinished room choice'&&current.desktop_id===scope);
  let denied=false;try{await ipc('g1_action',{input:{action:'arbitrary_operation'}});}catch{denied=true;}check('actual restricted IPC refuses unknown operation',denied);
  await el('logout').onclick();check('actual logout clears session visibly',!current.signed_in&&el('session-state').textContent==='Signed out locally');
  el('provider').value='twitch';await el('login').onclick();check('address-only labelled automatic sign-in has visible pending feedback',current.auth_pending&&el('session-state').textContent.includes('labelled mock account')&&!el('cancel').disabled);
  await el('cancel').onclick();check('actual cancellation clears pending return',!current.auth_pending&&el('session-state').textContent.includes('cancelled'));
  await el('logout').onclick();
  location.href=location.origin+'/?nt2_probe='+encodeURIComponent(JSON.stringify({passed:true,restarted,checks,scope:'actual WebView2, production UI, restricted IPC, candidate Core and routed service; address-only guest and sign-in cancellation, no Noita or HTTPS evidence'}));
 }catch(error){location.href=location.origin+'/?nt2_probe='+encodeURIComponent(JSON.stringify({passed:false,checks,error:String(error)}));}
},100));
