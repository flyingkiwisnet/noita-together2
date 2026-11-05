// Internal canonical policy. External CR1 field names remain candidate/reviewable.
export interface AdmissionFacts {
 terminalReason?:string;ownerReason?:string; identityValid:boolean; ownerActive:boolean;
 reportValid:boolean; protected:boolean; entered:boolean; generation:number;
 now:number; transportUntil:number; reviewUntil:number;
}
export interface AdmissionDecision {
 authorized:boolean; playing:boolean; terminal:boolean; reason:string;
 remainingMs:number; initialReviewRemainingMs:number;
}
export function decideWorldAdmission(f:AdmissionFacts):AdmissionDecision{
 const review=Number.isFinite(f.reviewUntil-f.now)?Math.max(0,Math.min(3600000,f.reviewUntil-f.now)):0;
 const hold=(reason:string,terminal=false):AdmissionDecision=>({authorized:false,playing:false,terminal,reason,remainingMs:0,initialReviewRemainingMs:review});
 if(f.terminalReason)return hold(f.terminalReason,true);
 if(!f.identityValid)return hold('owner_binding_changed');
 if(!f.ownerActive)return hold(f.ownerReason??'inspection_required');
 if(!f.reportValid)return hold('report_invalid');
 let until=f.transportUntil;
 if(f.generation===1&&!f.entered){if(review<=0)return hold('initial_review_expired');until=Math.min(until,f.reviewUntil);}
 const remainingMs=Math.max(0,Math.min(30000,until-f.now));
 if(!remainingMs)return hold('lease_expired');
 if(!f.protected&&!f.entered)return hold('entry_required');
 return {authorized:true,playing:!f.protected&&f.entered,terminal:false,reason:'',remainingMs,initialReviewRemainingMs:review};
}
