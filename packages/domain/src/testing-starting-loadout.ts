import {createRequire} from 'node:module';
const require=createRequire(import.meta.url);
export const TESTING_STARTING_LOADOUT=require('../../contracts/g3/testing-starting-loadout.json') as {revision:string;settingKey:string;values:string[];default:boolean;label:string;pickups:{tinkerPerk:string;tinkerCount:number;noMoreShufflePerk:string;noMoreShuffleCount:number;randomPerks:number;randomPerkExclusions:string[];randomWands:number;wandSpacingPixels:number;wandIds:string[]}};
export function testingStartingLoadout(config:{settings?:{key?:string;value?:string}[]}){return config.settings?.some(s=>s.key===TESTING_STARTING_LOADOUT.settingKey&&s.value==='true')??false;}
export function testingLoadoutCompatibility<T extends {settingKeys:string[]}>(compatibility:T,enabled:boolean):T{
 return {...compatibility,settingKeys:[...compatibility.settingKeys.filter(k=>k!==TESTING_STARTING_LOADOUT.settingKey),...(enabled?[TESTING_STARTING_LOADOUT.settingKey]:[])]};
}
