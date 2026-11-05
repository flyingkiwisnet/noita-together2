export const GOLD_LIMIT=1000000;
export const BALANCE_LIMIT=4294967295;
export const GOLD_CAPABILITY='core.gold';
export const GOLD_SETTING='NT_send_gold';
export const GOLD_BANK_PAGE=8;
export const GOLD_NOTICE_MS=10000;
export function sendGoldEnabled(config:{mode?:string;settings?:{key?:string;value?:string}[]}){
 const setting=config.settings?.find(s=>s.key===GOLD_SETTING);
 return config.mode==='core'&&(!setting||setting.value==='true');
}
export const G3_EVENT_CATALOG=[{id:'player.death',capability:'core.progress',mode:'core',target:'self',maxCount:1},{id:'player.win',capability:'core.progress',mode:'core',target:'self',maxCount:1}] as const;
export function receiptTransition(direction:string,state:string,outcome:string){
 if(state==='completed'||state==='cancelled')return undefined;
 if(direction==='deposit'&&outcome==='escrowed'||direction==='withdraw'&&outcome==='applied')return 'completed';
 if(outcome==='uncertain'||direction==='withdraw'&&outcome==='not_applied')return 'uncertain';
 if(direction==='deposit'&&outcome==='not_applied')return 'cancelled';
 return undefined;
}
