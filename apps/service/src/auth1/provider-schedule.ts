// Half-hour checks leave room for a bounded 120-family cohort, network timeouts
// and retries before an hourly validation lease. This is not a capacity claim.
export const PROVIDER_SCHEDULE={batch:128,concurrency:2,checkMs:1800000,marginMs:900000,retryMs:30000,maxRetryMs:300000} as const;
export const providerCheckAt=(now:number,until:number)=>Math.max(now,Math.min(now+PROVIDER_SCHEDULE.checkMs,until-PROVIDER_SCHEDULE.marginMs));
export const providerRetryAt=(now:number,failures:number)=>now+Math.min(PROVIDER_SCHEDULE.maxRetryMs,PROVIDER_SCHEDULE.retryMs*2**Math.min(failures-1,4));
