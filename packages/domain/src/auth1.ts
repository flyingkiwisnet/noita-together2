export const AUTH1_ROLES=['user','power-user','admin','dev'] as const;
export type AccountRole=typeof AUTH1_ROLES[number];
export const roleRank=(role:string)=>AUTH1_ROLES.indexOf(role as AccountRole);
export function maySetRole(actor:string,target:string,next:string){return roleRank(actor)===3&&roleRank(next)>=0||actor==='admin'&&['user','power-user'].includes(target)&&['user','power-user'].includes(next);}
export function mayResetLocal(actor:string,target:string){return actor==='dev'||actor==='admin'&&['user','power-user'].includes(target);}
export function normalizeUsername(input:unknown){if(typeof input!=='string')throw Error('Invalid username');const value=input.normalize('NFKC').trim().toLowerCase();if(!/^[a-z0-9][a-z0-9._-]{2,31}$/.test(value))throw Error('Invalid username');return value;}
export function auth1Limits(standard=8,power=16){if(!Number.isInteger(standard)||!Number.isInteger(power)||standard<1||standard>power||power>32)throw Error('AUTH1 capacity limits require 1 <= standard <= power <= 32');return {standard,power};}
export function auth1Entitlements(role:string,name:string,testing:boolean,limits=auth1Limits()){
 let stem=name;while(new TextEncoder().encode(stem).length>70)stem=stem.slice(0,-1);const rank=roleRank(role),generatedLobbyName=(stem+"'s lobby");return {customLobbyName:rank>=1,capacityLimit:rank>=1?limits.power:limits.standard,standardCapacityLimit:limits.standard,powerCapacityLimit:limits.power,generatedLobbyName,testingMode:rank===3&&testing,manageUsers:rank>=2,manageAllRoles:rank===3};
}
