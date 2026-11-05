// Presentation only. Canonical readiness and host Start remain authoritative.
const value=(kind,label,detail)=>({kind,label,detail});
const activeHelp=new WeakMap();
const action='Update the app and NT2 mod to the matching release, then restart Noita. See the app for local installation details.';
export function readinessPresentation({member:m,report,room,info,included=true,now=Date.now()},previous){
 const context=JSON.stringify([info.serviceId,info.serviceEpoch,room.roomId,room.runId,room.runEpoch,room.configRevision,m.desktopId,info.compatibility?.modVersion,info.compatibility?.bridgeVersion]);
 const world=report?JSON.stringify([report.attachmentId,report.worldToken,report.gameInstanceId]):previous?.world;
 const same=previous?.context===context&&(!report||previous.world===world);
 let failure=same?previous.failure:null;
 const expected=info.compatibility??{},reason=m.readinessReason??'',stage=m.readiness??'';
 const presence=Date.parse(report?.presenceUntil),review=Date.parse(report?.validUntil);
 const stale=!!report&&(Number.isFinite(presence)&&presence<=now||room.state==='staging'&&Number.isFinite(review)&&review<=now);
 const observed=!!report&&report.desktopId===m.desktopId&&!!report.attachmentId&&!!report.worldToken&&!!report.gameInstanceId&&Number.isInteger(report.observationRevision)&&report.observationRevision>0&&Number.isFinite(Date.parse(report.capturedAt));
 const fresh=observed&&Number.isFinite(presence)&&presence>now&&(room.state!=='staging'||Number.isFinite(review)&&review>now);
 const versions=observed&&report.compatibilityProvenance==='game-thread-client-observation'&&!!report.modVersion&&!!report.bridgeVersion&&!!expected.modVersion&&!!expected.bridgeVersion;
 const mismatch=versions&&(report.modVersion!==expected.modVersion||report.bridgeVersion!==expected.bridgeVersion);
 if(mismatch)failure=`Running NT2 mod: ${report.modVersion}; required: ${expected.modVersion}. Running native runtime: ${report.bridgeVersion}; required: ${expected.bridgeVersion}. ${action}`;
 else if(reason==='observed_version_mismatch')failure=`The running NT2 mod or native runtime does not match the service's accepted release. ${action}`;
 else if(versions&&fresh&&m.attached===true)failure=null;
 const diagnostics=reason?'\nDiagnostic reason: '+reason:'';
 const app=m.present===true?value('ok','Connected','The service currently sees this player’s desktop app.'):m.present===false?value('warning','Offline','The service has no current desktop presence. Reconnect the app; this does not prove that Noita is closed.'):value('unknown','Not checked','Desktop presence has not been reported.');
 const paused=stage==='paused'||reason==='paused';
 let noita=paused?value('warning','Paused','The current readiness report explicitly says the game is paused. This is not an app disconnect.'):stale?value('warning','Stale updates','A game report was observed, but its presence or protected review has expired. Return to Noita and check the app; process state is unknown.'):m.attached===true?m.loaded===true?value('ok','Observed','The service has current game contact and reports the world loaded.'):value('warning','Loading','Game contact is present; the world is still loading.'):value('unknown','Not observed','No current game contact is confirmed. Join this lobby in the app, then start Noita and choose New Game. If a world is already open, check the app.');
 const version=failure?value('error','Mismatch',failure):versions&&fresh&&m.attached===true?value('ok','Verified',`The observed NT2 mod (${report.modVersion}) and native runtime (${report.bridgeVersion}) match the service. Optional gameplay mod differences are advisory; see Mods.`):value('unknown','Not checked',stale?'The last compatibility observation is stale. A current matching game observation is required.':'No current scoped NT2 mod/runtime compatibility observation is available. Ready or connected alone does not verify versions.');
 const individual=room.config?.seedPolicy==='individual-random',known=report?.seedStatus==='known'&&typeof report.actualSeed==='string',expectedSeed=room.config?.seed;
 const differs=!individual&&(known&&expectedSeed!==undefined&&report.actualSeed!==String(expectedSeed)||reason==='seed_mismatch');
 const seed=individual?value('unknown','N/A','Individual random worlds use independent seeds; matching the host seed is not required.'):differs?value('error','Mismatch','The observed world seed differs from this lobby’s shared seed. Use the app’s protected new-world handoff; do not reuse a different world.'):known&&fresh&&m.attached===true&&expectedSeed!==undefined?value('ok','Matched','The actual world seed matches the lobby’s resolved shared seed.'):value('unknown','Not checked','No current actual shared-seed observation is available.');
 let status;
 if(room.state==='ended'||['ended','finished','dead','dnf'].includes(stage))status=value('unknown',({finished:'Finished',dead:'Died',dnf:'DNF'})[stage]??'Ended','This participation has ended. See the lobby’s statistics for accepted results.');
 else if(!included||m.included===false||stage==='excluded')status=value('unknown','Excluded','This player is excluded from the host’s current Start selection.');
 else if(failure)status=value('error','Versions differ',failure);
 else if(differs)status=value('error','Seed differs',seed.detail);
 else if(['inventory_incomplete','collector_failed'].includes(reason)||report?.collectorFailure==='protection_failed')status=value('error','Inspection failed','The game could not complete a required protected-world inspection. Check the app’s details, return to Noita and retry or restart as directed.');
 else if(['restart_required','expired','cancelled','configuration_changed'].includes(stage))status=value('error',stage==='configuration_changed'?'Review changes':'Restart required','Review the changed or expired preparation in the app, then use a new protected world.');
 else if(paused)status=value('warning','Paused',noita.detail);
 else if(stale)status=value('warning','Stale updates',noita.detail);
 else if(stage==='playing')status=value('ok','Playing','The service authorizes this participant’s current world to play.');
 else if(stage==='reconnecting'||stage==='offline'||reason==='game_offline')status=value('warning','Reconnecting','The app or game is reconnecting. Keep the same world and check the app; a paused game is not assumed closed.');
 else if(stage==='ready'||m.ready)status=value('warning','Waiting for host','This participant is ready under the existing service rules. The host must review and Start.');
 else if(stage==='loading_world')status=value('warning','Loading','The protected game world is loading.');
 else if(stage==='checking_report')status=value('warning','Checking','The service is checking the current protected world report.');
 else status=value('warning',m.present===false?'App offline':'Waiting for Noita','Use the app to join this lobby, then start Noita and choose New Game.');
 status.detail+=diagnostics;
 return {context,world,failure,cells:{app,noita,versions:version,seed,status}};
}

export function createReadinessCell(document,column,id,{status=false}={}){
 const cell=document.createElement('td'),button=document.createElement('button'),symbol=document.createElement('span'),label=document.createElement('span'),help=document.createElement('span');
 cell.className='readiness-value'+(status?' is-status':'');button.type='button';button.className='readiness-trigger';symbol.className='readiness-symbol';symbol.setAttribute('aria-hidden','true');label.className='readiness-label';help.id=id;help.className='readiness-help';help.setAttribute('role','tooltip');help.hidden=true;button.setAttribute('aria-describedby',id);button.setAttribute('aria-expanded','false');button.append(symbol,label);cell.append(button,help);
 let pinned=false;
 const hide=()=>{pinned=false;button.setAttribute('aria-expanded','false');help.hidden=true;if(activeHelp.get(document)===hide)activeHelp.delete(document);};
 const show=()=>{const old=activeHelp.get(document);if(old&&old!==hide)old();help.hidden=false;activeHelp.set(document,hide);};
 button.addEventListener('mouseenter',show);button.addEventListener('focus',show);
 button.addEventListener('mouseleave',()=>{if(!pinned&&document.activeElement!==button)hide();});button.addEventListener('blur',hide);
 button.addEventListener('click',()=>{pinned=!pinned;button.setAttribute('aria-expanded',String(pinned));if(pinned)show();else hide();});
 button.addEventListener('keydown',event=>{if(event.key==='Escape'){pinned=false;button.setAttribute('aria-expanded','false');hide();}});
 return {cell,button,help,update(v){cell.dataset.kind=v.kind;symbol.textContent=({ok:'✓',warning:'⚠',error:'✕',unknown:'—'})[v.kind];label.textContent=v.label;button.setAttribute('aria-label',column+': '+v.label+'. Details');help.textContent=column+' — '+v.label+'\n'+v.detail;}};
}
