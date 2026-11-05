// UI projection only. Actual Start still validates the same durable identities
// under the room lock; neither presence nor this projection grants authority.
export interface WaitingFacts {
 consented:boolean;desktopLive:boolean;reviewLive:boolean;terminal?:string;
 attachment:boolean;gameLive:boolean;scopeMatches:boolean;reportPresent:boolean;
 reportFailure?:string|undefined;loaded:boolean;protected:boolean;running?:boolean;excluded?:boolean;
}
export function waitingReadiness(f:WaitingFacts):{ready:boolean;readiness:string;readinessReason:string}{
 const result=(readiness:string,readinessReason:string,ready=false)=>({ready,readiness,readinessReason});
 if(f.excluded)return result('excluded','host_excluded');
 if(f.terminal)return result(f.terminal==='expired'?'expired':'cancelled','new_game_required');
 if(!f.consented)return result('configuration_changed','review_current_configuration');
 if(!f.reviewLive&&!f.running)return result('expired','review_expired');
 if(!f.desktopLive)return result('offline','desktop_offline');
 if(!f.attachment)return result('waiting_noita','actual_game_required');
 if(!f.scopeMatches)return result('restart_required','game_scope_changed');
 if(!f.gameLive)return result('offline','game_offline');
 if(!f.reportPresent)return result('checking_report','actual_report_required');
 if(!f.loaded)return result('loading_world','protected_load_required');
 if(f.reportFailure)return result('checking_report',f.reportFailure);
 if(!f.protected)return f.running?result('playing',''):result('restart_required','protected_world_required');
 return result('ready','',true);
}
